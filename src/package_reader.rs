//
// If not stated otherwise in this file or this component's LICENSE file the
// following copyright and licenses apply:
//
// Copyright 2025 Comcast Cable Communications Management, LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

use crate::package::*;
use crate::package_config::PackageConfig;
use crate::package_content::PackageContent;
use crate::package_signature::*;
use std::cell::RefCell;
use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Write};
use tempfile::tempfile;

pub struct RalfPackage {
    /// The zip reader for the package
    reader: RefCell<zip::read::ZipArchive<File>>,

    /// The parsed image manifest from the package
    image_manifest: oci_spec::image::ImageManifest,

    /// The digest of the image manifest
    image_manifest_digest: oci_spec::image::Digest,

    /// The optional signature manifest from the package
    signature_manifest: Option<oci_spec::image::ImageManifest>,

    /// The digest of the signature manifest, if present
    signature_manifest_digest: Option<oci_spec::image::Digest>,

    /// A list of all files legally referenced by the OCI index
    referenced_files: Vec<String>,
}

/// Information about a package signature
pub struct PackageSignatureInfo {
    /// The raw signature data, decoded from base64 annotation
    #[allow(unused)]
    pub signature: Vec<u8>,

    /// The actual signed data blob, which is JSON of the format described in
    /// https://github.com/containers/image/blob/main/docs/containers-signature.5.md.  No validation
    /// is done on this data, it is just returned as-is.
    pub signed_blob: Vec<u8>,

    /// The signing certificate, if present
    pub certificate: Option<openssl::x509::X509>,

    /// The signing certificate chain if present
    pub certificate_chain: Option<openssl::stack::Stack<openssl::x509::X509>>,
}

/// Utility struct and impl to provide a Write instance that just discards all data written to it.
/// This is used when we want to read a blob and just verify its digest without actually storing
/// the data anywhere.
struct DevNull;
impl Write for DevNull {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // Pretend to accept all bytes
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl RalfPackage {
    /// Opens a RALF package from the specified path.
    pub fn open(path: &std::path::Path) -> Result<Self, String> {
        // Check if the file exists
        if !path.is_file() {
            return Err(format!(
                "RALF package file does not exist or is not a file: {}",
                path.display()
            ));
        }

        // Open the zip file (we're assuming it's a zip file for now)
        let file = File::open(path).map_err(|e| format!("Failed to open RALF package: {}", e))?;
        let mut archive =
            zip::read::ZipArchive::new(file).map_err(|e| format!("Failed to read RALF package: {}", e))?;

        // Read and check the oci-image-layout file
        Self::check_oci_layout(&mut archive)?;

        // Read the index file and process its contents
        let index_json_file = archive
            .by_name("index.json")
            .map_err(|_| "RALF package is missing index.json file".to_string())?;
        let index = oci_spec::image::ImageIndex::from_reader(index_json_file)
            .map_err(|e| format!("Failed to parse index.json file: {}", e))?;
        if index.schema_version() != 2 {
            return Err("Invalid schemaVersion in index.json file".to_string());
        }

        // The two manifests we care about and their digests
        let mut image_manifest: Option<oci_spec::image::ImageManifest> = None;
        let mut signature_manifest: Option<oci_spec::image::ImageManifest> = None;
        let mut image_manifest_digest: Option<oci_spec::image::Digest> = None;
        let mut signature_manifest_digest: Option<oci_spec::image::Digest> = None;

        let mut referenced_files = vec!["oci-layout".to_string(), "index.json".to_string()];

        // Store the parsed manifests from the index
        for descriptor in index.manifests() {
            let manifest_path = format!(
                "blobs/{}/{}",
                descriptor.digest().algorithm(),
                descriptor.digest().digest()
            );
            referenced_files.push(manifest_path);

            if *descriptor.media_type() != oci_spec::image::MediaType::ImageManifest {
                continue;
            }

            let mut blob_data = Vec::new();
            let blob_writer = std::io::Cursor::new(&mut blob_data);
            Self::read_blob(&mut archive, descriptor, blob_writer)?;

            let blob_reader = std::io::Cursor::new(&blob_data);
            let manifest = oci_spec::image::ImageManifest::from_reader(blob_reader)
                .map_err(|e| format!("Failed to parse image manifest blob: {}", e))?;

            if manifest.media_type() != &Some(oci_spec::image::MediaType::ImageManifest) {
                continue;
            }
            if manifest.schema_version() != 2 {
                return Err("Invalid schemaVersion in image manifest".to_string());
            }

            let config_path = format!(
                "blobs/{}/{}",
                manifest.config().digest().algorithm(),
                manifest.config().digest().digest()
            );
            referenced_files.push(config_path);

            for layer in manifest.layers() {
                let layer_path = format!("blobs/{}/{}", layer.digest().algorithm(), layer.digest().digest());
                referenced_files.push(layer_path);
            }

            log::debug!(
                "Found image manifest with config media type with digest {}",
                descriptor.digest()
            );

            // Check if the manifest is a package content manifest by looking at its config media type
            if Self::is_package_manifest(&manifest) {
                if image_manifest.is_some() {
                    return Err("Multiple package manifests found in RALF package".to_string());
                }
                log::debug!(
                    "Found content manifest with config media type '{}' and digest {}",
                    manifest.config().media_type(),
                    descriptor.digest()
                );
                image_manifest = Some(manifest);
                image_manifest_digest = Some(descriptor.digest().clone());
            } else if Self::is_signature_manifest(&manifest) {
                if signature_manifest.is_some() {
                    return Err("Multiple signature manifests found in RALF package".to_string());
                }
                log::debug!(
                    "Found signature manifest with config media type '{}' and digest {}",
                    manifest.config().media_type(),
                    descriptor.digest()
                );
                signature_manifest = Some(manifest);
                signature_manifest_digest = Some(descriptor.digest().clone());
            } else {
                log::warn!(
                    "Ignoring unknown image manifest with config media type '{}'",
                    manifest.config().media_type()
                );
            }
        }

        // We should have found at least one manifest
        if image_manifest.is_none() {
            return Err("No package image manifests found in RALF package".to_string());
        }

        Ok(Self {
            reader: RefCell::new(archive),
            image_manifest: image_manifest.unwrap(),
            image_manifest_digest: image_manifest_digest.unwrap(),
            signature_manifest,
            signature_manifest_digest,
            referenced_files,
        })
    }

    /// Gets the parsed config of the RALF package
    pub fn config(&self) -> Result<PackageConfig, String> {
        let manifest = &self.image_manifest;
        let config_blob = self.read_blob_to_vec(manifest.config())?;
        let package_config: PackageConfig =
            serde_json::from_slice(&config_blob).map_err(|e| format!("Failed to parse package config JSON: {}", e))?;

        log::debug!("Found package config blob with digest {}", manifest.config().digest());

        Ok(package_config)
    }

    /// Gets the raw config string of the RALF package
    pub fn raw_config(&self) -> Result<String, String> {
        let manifest = &self.image_manifest;
        let config_blob = self.read_blob_to_vec(manifest.config())?;
        let config_str =
            String::from_utf8(config_blob).map_err(|e| format!("Failed to convert package config to string: {}", e))?;

        log::debug!("Found package config blob with digest {}", manifest.config().digest());

        Ok(config_str)
    }

    /// Gets the package content of the RALF package
    pub fn content(&self) -> Result<PackageContent, String> {
        let manifest = &self.image_manifest;

        // Iterate through the image layers to find the package content
        for layer in manifest.layers() {
            if !Self::is_package_content_media_type(&layer.media_type()) {
                continue;
            }

            log::debug!("Found package content blob with digest {}", layer.digest());

            // Create a PackageContent from the blob of data in the layer
            let package_content = self.create_content_blob(layer)?;

            // We only support one package content layer, so return it now
            return Ok(package_content);
        }

        Err("No package config found in RALF package".to_string())
    }

    /// Gets any auxiliary data (image layers) of the RALF package, if present.  It is not an
    /// error if no auxiliary content is found, in which case an empty vector is returned.
    pub fn auxiliary_content(&self) -> Result<Vec<PackageContent>, String> {
        let manifest = &self.image_manifest;
        let mut aux_contents: Vec<PackageContent> = Vec::new();

        for layer in manifest.layers() {
            // Skip over the main package content types
            if Self::is_package_content_media_type(&layer.media_type()) {
                continue;
            }

            log::debug!(
                "Found package auxiliary content blob with media type '{}' and digest {}",
                layer.media_type(),
                layer.digest()
            );

            // Create a PackageContent from the blob of data in the layer
            let aux_content = self.create_content_blob(layer)?;

            // Add to the vector of aux contents
            aux_contents.push(aux_content);
        }

        Ok(aux_contents)
    }

    /// Gets the signature data of the RALF package, if present
    pub fn signature(&self) -> Result<PackageSignatureInfo, String> {
        if self.signature_manifest.is_none() {
            return Err("Package is not signed".to_string());
        }

        let manifest = self.signature_manifest.as_ref().unwrap();

        // Look for the cosign signature layer
        let signature_media_type = oci_spec::image::MediaType::Other(MEDIA_TYPE_COSIGN_SIGNATURE.to_string());
        for layer in manifest.layers() {
            if *layer.media_type() != signature_media_type {
                continue;
            }

            log::debug!("Found package signature blob with digest {}", layer.digest());

            // Sanity check the size of the signed blob, it should not be empty and should not be
            // too large (as it should just contain a small JSON document)
            if layer.size() == 0 || layer.size() > (64 * 1024) {
                return Err(format!("Invalid size {} for cosign signature layer", layer.size()));
            }

            let signature: Vec<u8>;
            let mut cert = None;
            let mut cert_chain = None;

            // All the info we need should be in the annotations of the layer
            if let Some(annotations) = layer.annotations() {
                // Get the actual signature bytes from the annotation
                if let Some(sig) = annotations.get(ANNOTATION_COSIGN_SIGNATURE) {
                    signature = openssl::base64::decode_block(sig)
                        .map_err(|e| format!("Failed to decode base64 signature annotation: {}", e))?;
                } else {
                    return Err("Cosign signature layer is missing signature annotation".to_string());
                }

                // Get the optional public key certificate in PEM format
                if let Some(cert_pem) = annotations.get(ANNOTATION_COSIGN_CERTIFICATE) {
                    let cert_x509 = openssl::x509::X509::from_pem(cert_pem.as_bytes())
                        .map_err(|e| format!("Failed to parse certificate: {}", e))?;
                    cert = Some(cert_x509);
                }

                // Get the optional certificate chain in PEM format
                if let Some(cert_chain_pem) = annotations.get(ANNOTATION_COSIGN_CHAIN) {
                    let cert_chain_x509 = openssl::x509::X509::stack_from_pem(cert_chain_pem.as_bytes())
                        .map_err(|e| format!("Failed to parse certificate chain: {}", e))?;

                    let mut cert_stack_x509 = openssl::stack::Stack::new()
                        .map_err(|e| format!("Failed to create certificate stack: {}", e))?;
                    for cert in cert_chain_x509 {
                        cert_stack_x509
                            .push(cert)
                            .map_err(|e| format!("Failed to add certificate to stack: {}", e))?;
                    }

                    cert_chain = Some(cert_stack_x509);
                }
            } else {
                return Err("Cosign signature layer is missing annotations".to_string());
            }

            // Next get the actual signed data blob, which is JSON of the format described in
            // https://github.com/containers/image/blob/main/docs/containers-signature.5.md
            // We just store the unverified signature data for now, verification is done elsewhere.
            let signed_blob = self
                .read_blob_to_vec(layer)
                .map_err(|e| format!("Failed to read signed blob: {}", e))?;

            // We have the signature and optional certs, plus the actual thing that is signed
            return Ok(PackageSignatureInfo {
                signature: signature,
                signed_blob: signed_blob,
                certificate: cert,
                certificate_chain: cert_chain,
            });
        }

        Err("No cosign signature layer found in signature manifest".to_string())
    }

    /// Returns the image manifest digest for the package content
    pub fn content_manifest_digest(&self) -> oci_spec::image::Digest {
        self.image_manifest_digest.clone()
    }

    /// Returns the signature manifest digest if present
    #[allow(unused)]
    pub fn signature_manifest_digest(&self) -> Option<oci_spec::image::Digest> {
        self.signature_manifest_digest.clone()
    }

    /// This function looks at config and image layer blobs and verifies that:
    /// 1) The blob exists in the archive.
    /// 2) It's size matches the expected size in the descriptor.
    /// 3) Its sha256 digest matches the expected digest in the descriptor.
    ///
    /// Nb: the manifest itself has had already had it's digest checked when it was read from the
    /// index in the open() function.
    pub fn verify_all_content_blobs(&self) -> Result<(), String> {
        let manifest = &self.image_manifest;
        let mut archive = self.reader.borrow_mut();
        let mut dev_null = DevNull;

        // Verify the config blob
        Self::read_blob(&mut archive, manifest.config(), &mut dev_null)?;
        log::debug!(
            "Verified config blob with size {} and digest {}",
            manifest.config().size(),
            manifest.config().digest()
        );

        // Verify each layer blob
        for layer in manifest.layers() {
            Self::read_blob(&mut archive, layer, &mut dev_null)?;
            log::debug!(
                "Verified image layer blob with size {} and digest {}",
                layer.size(),
                layer.digest()
            );
        }

        Ok(())
    }

    /// Returns true if the OCI manifest refers to a package image content
    fn is_package_manifest(manifest: &oci_spec::image::ImageManifest) -> bool {
        // The media type we expect for the package config
        let config_media_type = oci_spec::image::MediaType::Other(MEDIA_TYPE_PACKAGE_CONFIG.to_string());

        manifest.config().media_type() == &config_media_type
    }

    /// Returns true if the manifest refers to a signature
    fn is_signature_manifest(manifest: &oci_spec::image::ImageManifest) -> bool {
        // The media type we expect for the cosign signature layer
        let signature_media_type = oci_spec::image::MediaType::Other(MEDIA_TYPE_COSIGN_SIGNATURE.to_string());

        for layer in manifest.layers() {
            if *layer.media_type() == signature_media_type {
                return true;
            }
        }

        false
    }

    /// Returns true if the media type is one of the supported package content types
    fn is_package_content_media_type(media_type: &oci_spec::image::MediaType) -> bool {
        match media_type {
            oci_spec::image::MediaType::Other(s) => match s.as_str() {
                MEDIA_TYPE_PACKAGE_CONTENT_TAR
                | MEDIA_TYPE_PACKAGE_CONTENT_TAR_GZIP
                | MEDIA_TYPE_PACKAGE_CONTENT_TAR_ZSTD
                | MEDIA_TYPE_PACKAGE_CONTENT_EROFS => true,
                _ => false,
            },
            _ => false,
        }
    }

    /// Creates a content blob from the specified media type and file
    fn create_content_blob(&self, descriptor: &oci_spec::image::Descriptor) -> Result<PackageContent, String> {
        // Create a temporary file for storing the package content blob
        let mut temp_file = tempfile().map_err(|e| format!("Failed to create temp file: {}", e))?;

        // Copy the content into a temporary file and then wrap it in a PackageContent
        self.read_blob_to_file(descriptor, &mut temp_file)?;
        let mut package_content = PackageContent::new(descriptor.media_type(), temp_file)
            .map_err(|e| format!("Failed to read package content from blob: {}", e))?;

        // Add any annotations from the layer to the package content
        if let Some(annotations) = descriptor.annotations() {
            for (key, value) in annotations {
                package_content.add_annotation(key, value);
            }
        }

        // Sanity check the newly created descriptor matches the original descriptor
        if &package_content.descriptor() != descriptor {
            println!("Original descriptor: {:?}", descriptor);
            println!("New descriptor: {:?}", package_content.descriptor());

            return Err("Package content descriptor mismatch".to_string());
        }

        Ok(package_content)
    }

    /// Checks that the oci-layout file exists and is valid
    fn check_oci_layout(archive: &mut zip::read::ZipArchive<File>) -> Result<(), String> {
        let oci_layout_file = archive
            .by_name("oci-layout")
            .map_err(|_| "RALF package is missing oci-layout file".to_string())?;

        let oci_layout = oci_spec::image::OciLayout::from_reader(oci_layout_file)
            .map_err(|e| format!("Failed to parse oci-layout file: {}", e))?;

        if oci_layout.image_layout_version() != "1.0.0" {
            return Err("Invalid imageLayoutVersion in oci-layout file".to_string());
        }

        Ok(())
    }

    /// Helper function to read from a reader and write to a writer while calculating the sha256 hash
    /// of the data being copied.  Returns the total number of bytes copied and the sha256 hash.
    /// This is used when copying blobs from the zip archive to temporary files.
    pub fn copy_and_sha256<R: Read, W: Write>(
        mut reader: R,
        mut writer: W,
        max_size: u64,
    ) -> Result<(u64, [u8; 32]), String> {
        let mut hasher = openssl::sha::Sha256::new();
        let mut buf = [0u8; 8192];
        let mut total = 0u64;

        loop {
            let n = reader
                .read(&mut buf)
                .map_err(|e| format!("Failed to read from blob: {}", e))?;
            if n == 0 {
                break;
            }

            total += n as u64;
            if total > max_size {
                return Err(format!("Blob size exceeds expected size of {}", max_size));
            }

            hasher.update(&buf[..n]);
            writer
                .write_all(&buf[..n])
                .map_err(|e| format!("Failed to write blob to file: {}", e))?;
        }

        let digest = hasher.finish();
        Ok((total, digest))
    }

    /// Generic function to read a blob and copy it to a Write instance, while also calculating
    /// its sha256 hash to verify it matches the expected digest.
    fn read_blob<W: Write>(
        archive: &mut zip::ZipArchive<File>,
        descriptor: &oci_spec::image::Descriptor,
        writer: W,
    ) -> Result<(), String> {
        // Currently only support sha256 digests
        if descriptor.digest().algorithm() != &oci_spec::image::DigestAlgorithm::Sha256 {
            return Err(format!(
                "Unsupported digest algorithm '{}', only 'sha256' is supported",
                descriptor.digest().algorithm()
            ));
        }

        // Get the path to the blob file in the zip archive
        let blob_path = format!(
            "blobs/{}/{}",
            descriptor.digest().algorithm(),
            descriptor.digest().digest()
        );

        // Open the blob file in the zip archive for reading
        let blob_file = archive
            .by_name(&blob_path)
            .map_err(|_| format!("Failed to find blob file: {}", blob_path))?;

        // Read from the blob file, write to the provided writer, and calculate sha256 hash and size
        // as it goes
        let (size, sha256) = Self::copy_and_sha256(blob_file, writer, descriptor.size())?;

        // Check the size and sha256 hash match the expected values
        if size != descriptor.size() {
            return Err(format!(
                "Blob size mismatch for {}: expected {}, got {}",
                blob_path,
                descriptor.size(),
                size
            ));
        }

        let expected_sha256 = hex::decode(descriptor.digest().digest())
            .map_err(|e| format!("Failed to decode expected sha256 digest: {}", e))?;
        if sha256.to_vec() != expected_sha256 {
            return Err(format!(
                "Blob sha256 mismatch for {}: expected {:x?}, got {:x?}",
                blob_path, expected_sha256, sha256
            ));
        }

        // Everything matches
        Ok(())
    }

    /// Reads a blob from the archive given its descriptor
    fn read_blob_to_vec(&self, descriptor: &oci_spec::image::Descriptor) -> Result<Vec<u8>, String> {
        let mut blob_data = Vec::new();
        let mut blob_writer = std::io::Cursor::new(&mut blob_data);

        let mut archive = self.reader.borrow_mut();
        Self::read_blob(&mut archive, descriptor, &mut blob_writer)?;

        Ok(blob_data)
    }

    /// Reads a blob from the archive given its descriptor and writes it to the specified file
    fn read_blob_to_file(&self, descriptor: &oci_spec::image::Descriptor, file: &mut File) -> Result<(), String> {
        let mut archive = self.reader.borrow_mut();
        Self::read_blob(&mut archive, descriptor, file)
    }

    /// Returns a list of files present in the archive that are not legally
    /// referenced by the OCI index or manifests.
    pub fn find_unreferenced_files(&self) -> Result<Vec<String>, String> {
        let mut unreferenced = Vec::new();
        let mut archive = self.reader.borrow_mut();

        let referenced_files: HashSet<&str> = self.referenced_files.iter().map(|name| name.as_str()).collect();

        for i in 0..archive.len() {
            let file = archive
                .by_index(i)
                .map_err(|e| format!("Failed to read archive entry: {}", e))?;

            // We only care about files, not directories
            if file.is_dir() {
                continue;
            }

            let name = file.name().to_string();

            if !referenced_files.contains(name.as_str()) {
                unreferenced.push(name);
            }
        }

        Ok(unreferenced)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::str::FromStr;
    use tempfile::NamedTempFile;
    use zip::write::FileOptions;

    #[test]
    fn test_find_unreferenced_files() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let mut zip = zip::ZipWriter::new(&mut temp_file);
        let options: FileOptions<()> = FileOptions::default().compression_method(zip::CompressionMethod::Stored);

        // Write the mandatory oci-layout file and index.json
        zip.start_file("oci-layout", options).unwrap();
        zip.write_all(b"{\"imageLayoutVersion\": \"1.0.0\"}").unwrap();

        zip.start_file("index.json", options).unwrap();
        let index_json = r#"{
            "schemaVersion": 2,
            "manifests": []
        }"#;
        zip.write_all(index_json.as_bytes()).unwrap();

        // Inject an unreferenced rogue file (the vulnerability simulation)
        zip.start_file("malicious_file.sh", options).unwrap();
        zip.write_all(b"echo 'malicious'").unwrap();

        zip.finish().unwrap();

        // We open the archive, ignoring any missing manifest errors by manually parsing the files
        let file = File::open(temp_file.path()).unwrap();
        let archive = zip::read::ZipArchive::new(file).unwrap();

        let package = RalfPackage {
            reader: RefCell::new(archive),
            image_manifest: oci_spec::image::ImageManifest::from_reader(std::io::Cursor::new(b"{\"schemaVersion\": 2, \"config\": {\"mediaType\": \"application/vnd.oci.image.config.v1+json\", \"digest\": \"sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855\", \"size\": 0}, \"layers\": []}")).unwrap(),
            image_manifest_digest: oci_spec::image::Digest::from_str("sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855").unwrap(),
            signature_manifest: None,
            signature_manifest_digest: None,
            referenced_files: vec!["oci-layout".to_string(), "index.json".to_string()],
        };

        let unreferenced = package.find_unreferenced_files().unwrap();
        assert_eq!(unreferenced.len(), 1);
        assert_eq!(unreferenced[0], "malicious_file.sh");
    }
}
