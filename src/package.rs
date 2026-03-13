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
use oci_spec::image::*;
use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::fs::File;
use std::io::Seek;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::{fmt, io};

use crate::package_content::PackageContent;
use crate::package_signature::PackageSignature;
use crate::signing_config::SigningConfig;

pub const MEDIA_TYPE_PACKAGE_CONFIG: &str = "application/vnd.rdk.package.config.v1+json";

#[allow(unused)]
pub const MEDIA_TYPE_PACKAGE_CONTENT_TAR: &str = "application/vnd.rdk.package.content.layer.v1.tar";

#[allow(unused)]
pub const MEDIA_TYPE_PACKAGE_CONTENT_TAR_GZIP: &str = "application/vnd.rdk.package.content.layer.v1.tar+gzip";

#[allow(unused)]
pub const MEDIA_TYPE_PACKAGE_CONTENT_TAR_ZSTD: &str = "application/vnd.rdk.package.content.layer.v1.tar+zstd";

#[allow(unused)]
pub const MEDIA_TYPE_PACKAGE_CONTENT_EROFS: &str = "application/vnd.rdk.package.content.layer.v1.erofs+dmverity";

pub const MEDIA_TYPE_PACKAGE_ARTIFACT_TYPE: &str = "application/vnd.rdk.package+type";

/// The default alignment for image layer contents within the package.  This is the page size of
/// most systems, and is also the block size of EROFS images.
const DEFAULT_ALIGNMENT: u64 = 4096;

/// The package output formats we support, the default is Zip.
#[derive(PartialEq, Clone, Debug)]
pub enum PackageOutputFormat {
    Tar = 1,
    Zip = 2,
    Directory = 3,
}

impl fmt::Display for PackageOutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            PackageOutputFormat::Tar => "tar",
            PackageOutputFormat::Zip => "zip",
            PackageOutputFormat::Directory => "dir",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for PackageOutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "tar" => Ok(PackageOutputFormat::Tar),
            "zip" => Ok(PackageOutputFormat::Zip),
            "dir" => Ok(PackageOutputFormat::Directory),
            _ => Err(format!("Invalid package output format: {}", s)),
        }
    }
}

/// This trait is used to represent a blob of data that is part of the package.  It's just a handy
/// wrapper around the data that contains the SHA256 digest and size of the blob.
pub trait PackageBlob: io::Read + io::Seek {
    /// Returns the SHA256 digest of the blob
    fn digest(&self) -> Sha256Digest;

    /// Returns the size of the blob
    fn size(&self) -> u64;

    /// Returns a descriptor that describes the blob.
    fn descriptor(&self) -> oci_spec::image::Descriptor;
}

/// PackageBlobString is a wrapper around a string that implements the Read and PackageBlob traits.
/// This means the digest of the string is calculated and stored in the object, as well as the size,
/// making it easy to write the 'blob' to the package.
///
#[derive(Clone)]
pub struct PackageBlobString {
    /// The media type of the blob
    media_type: MediaType,

    /// Cursor around the string bytes so can implement the Read trait
    cursor: std::io::Cursor<Vec<u8>>,

    /// The size of the string in bytes
    size: usize,

    /// The SHA256 digest of the string
    digest: Sha256Digest,

    /// Optional annotations for the blob
    annotations: HashMap<String, String>,
}

impl PackageBlobString {
    /// Create a new blob from a string with the given media_type.
    pub fn new(media_type: MediaType, data: String) -> PackageBlobString {
        let bytes = data.as_bytes();
        let len = bytes.len();

        // Calculate the SHA256 digest of the string bytes
        let mut hasher = openssl::sha::Sha256::new();
        hasher.update(bytes);
        let sha256 = hasher.finish();
        let digest = Sha256Digest::from_str(hex::encode(sha256).as_str()).expect("Failed to create digest");

        // Create a cursor around the bytes
        let cursor = std::io::Cursor::new(bytes.to_vec());

        PackageBlobString {
            media_type: media_type,
            cursor: cursor,
            size: len,
            digest: digest,
            annotations: HashMap::new(),
        }
    }

    /// Add some annotations to the blob.  This only takes effect the next time the descriptor is
    /// created.
    pub fn add_annotation(&mut self, key: &str, value: &str) {
        self.annotations.insert(key.to_string(), value.to_string());
    }
}

impl io::Read for PackageBlobString {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.cursor.read(buf)
    }
}

impl io::Seek for PackageBlobString {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.cursor.seek(pos)
    }
}

impl PackageBlob for PackageBlobString {
    fn digest(&self) -> Sha256Digest {
        self.digest.clone()
    }

    fn size(&self) -> u64 {
        self.size as u64
    }

    fn descriptor(&self) -> Descriptor {
        DescriptorBuilder::default()
            .media_type(self.media_type.clone())
            .digest(Digest::from(self.digest.clone()))
            .size(self.size as u64)
            .annotations(self.annotations.clone())
            .build()
            .expect("Failed to create data descriptor")
    }
}

///
/// Internal enum use to write files and blobs into the final package.  It allows for writing blobs
/// to either a directory or a tarball.
///
enum PackageWriter {
    Tar { tar: tar::Builder<File> },
    Zip { zip: zip::ZipWriter<File> },
}

impl PackageWriter {
    fn append_file<P: AsRef<Path>, R: io::Read>(&mut self, path: P, data: &mut R, size: u64) -> std::io::Result<()> {
        match self {
            PackageWriter::Tar { tar } => Self::_append_file_to_tar(tar, path, data, size, 0),
            PackageWriter::Zip { zip } => Self::_append_file_to_zip(zip, path, data, size, 0, true),
        }
    }

    #[allow(unused)]
    fn append_file_aligned<P: AsRef<Path>, R: io::Read>(
        &mut self,
        path: P,
        data: &mut R,
        size: u64,
        alignment: u64,
    ) -> std::io::Result<()> {
        match self {
            PackageWriter::Tar { tar } => Self::_append_file_to_tar(tar, path, data, size, alignment),
            PackageWriter::Zip { zip } => Self::_append_file_to_zip(zip, path, data, size, alignment, true),
        }
    }

    /// Adds a file to the package without any compression and with the desired alignment.  This
    /// only has an effect on zip archives, as tar files are not compressed by default.
    fn append_file_aligned_uncompr<P: AsRef<Path>, R: io::Read>(
        &mut self,
        path: P,
        data: &mut R,
        size: u64,
        alignment: u64,
    ) -> std::io::Result<()> {
        match self {
            PackageWriter::Tar { tar } => Self::_append_file_to_tar(tar, path, data, size, alignment),
            PackageWriter::Zip { zip } => Self::_append_file_to_zip(zip, path, data, size, alignment, false),
        }
    }

    fn append_blob<B: PackageBlob>(&mut self, blob: &mut B) -> std::io::Result<()> {
        let path = format!("blobs/sha256/{}", blob.digest().digest());
        let size = blob.size();
        blob.rewind()?;
        self.append_file(path, blob, size)
    }

    fn append_boxed_blob(&mut self, blob: &mut Box<dyn PackageBlob>) -> std::io::Result<()> {
        let path = format!("blobs/sha256/{}", blob.digest().digest());
        let size = blob.size();
        let mut blob = blob.as_mut();
        blob.rewind()?;
        self.append_file(path, &mut blob, size)
    }

    #[allow(unused)]
    fn append_blob_aligned<B: PackageBlob>(&mut self, blob: &mut B, alignment: u64) -> std::io::Result<()> {
        let path = format!("blobs/sha256/{}", blob.digest().digest());
        let size = blob.size();
        blob.rewind()?;
        self.append_file_aligned(path, blob, size, alignment)
    }

    /// Appends a blob to the package with the desired alignment, but without any compression.  This
    /// only has an effect on zip archives, as tar files don't individually compress files.
    fn append_blob_aligned_uncompr<B: PackageBlob>(&mut self, blob: &mut B, alignment: u64) -> std::io::Result<()> {
        let path = format!("blobs/sha256/{}", blob.digest().digest());
        let size = blob.size();
        blob.rewind()?;
        self.append_file_aligned_uncompr(path, blob, size, alignment)
    }

    /// Appends a blob to the package without any compression.  This only has an effect on zip
    /// archives, as tar files don't individually compress files.
    #[allow(unused)]
    fn append_blob_uncompr<B: PackageBlob>(&mut self, blob: &mut B) -> std::io::Result<()> {
        let path = format!("blobs/sha256/{}", blob.digest().digest());
        let size = blob.size();
        blob.rewind()?;
        self.append_file_aligned_uncompr(path, blob, size, 0)
    }

    /// Finishes writing the package.  This is only required for tar and zip packages, for directory
    /// packages this is a no-op.
    ///
    /// After this call the PackageWriter should not be used again.
    fn finish(self) -> std::io::Result<File> {
        match self {
            PackageWriter::Tar { tar } => tar.into_inner(),
            PackageWriter::Zip { zip } => zip
                .finish()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)),
        }
    }

    fn _append_file_to_dir<P: AsRef<Path>, R: io::Read>(path: P, data: &mut R) -> std::io::Result<()> {
        let mut file = File::create_new(path)?;
        std::io::copy(data, &mut file)?;
        Ok(())
    }

    /// Simply appends a regular file to the tar archive. However, this also takes an alignment
    /// argument, which if greater than 0 then the function will try and align the start of data
    /// with the given alignment.  However, all tar entries must start on 512 byte boundary, so
    /// the alignment will be rounded up to the nearest 512 factor.
    ///
    /// Alignment is useful for EROFS images, as means we can create a loopback mount of the tar
    /// archive with offset. It also means the code can mmap the archived file directly.
    ///
    /// For alignment to work we prepend a pax comment header to the file if required.
    fn _append_file_to_tar<P: AsRef<Path>, R: io::Read>(
        tar: &mut tar::Builder<File>,
        path: P,
        data: &mut R,
        size: u64,
        alignment: u64,
    ) -> std::io::Result<()> {
        if alignment > 0 {
            let cur_pos = tar.get_ref().seek(io::SeekFrom::Current(0))?;
            assert_eq!(cur_pos % 512, 0);

            // Round the alignment up to the nearest 512 bytes
            let alignment = if alignment % 512 == 0 {
                alignment
            } else {
                ((alignment / 512) + 1) * 512
            };

            // FIXME: Making an assumption that the pax header is exactly 512 bytes, however this
            // is only the case if the path is less than 256 characters (and can be divided
            // cleanly into 155 byte prefix).
            let expected_pos = cur_pos + 512;
            if expected_pos % alignment != 0 {
                let mut padding = alignment - (expected_pos % alignment);
                assert_eq!(padding % 512, 0);

                // The minimum padding we can add is 1024 bytes; 512 for the header and a minimum of
                // 512 for the contents.
                if padding < 1024 {
                    padding += alignment;
                }

                // Calculate the comment length, which is the padding size minus 512 bytes for the
                // PAX header and further reduced by 32 bytes for the "<size> comment=" string
                let comment_len = padding - 512 - 32;
                let comment = " ".repeat(comment_len as usize);

                // We need to add a pax header to align the file to a 4k block size
                let headers = vec![("comment", comment.as_bytes())];
                tar.append_pax_extensions(headers)?;

                // FIXME: Remove this debugging check
                let cur_pos = tar.get_ref().seek(io::SeekFrom::Current(0))?;
                assert_eq!((cur_pos + 512) % 4096, 0);
            }
        }

        // Can now add the actual header and data
        let mut header = tar::Header::new_gnu();
        header.set_path(path)?;
        header.set_size(size as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append(&header, data)?;
        Ok(())
    }

    /// Simply appends a regular file to the zip archive. However, this also takes an alignment
    /// argument, that if set, then it will align the entry data to the given byte boundary.
    /// This is useful for EROFS images, as means we can create a loopback  mount of the zip archive
    /// with offset.
    ///
    /// Also takes a compress argument, which if false means the file is stored without compression.
    fn _append_file_to_zip<P: AsRef<Path>, R: io::Read>(
        zip: &mut zip::ZipWriter<File>,
        path: P,
        data: &mut R,
        size: u64,
        alignment: u64,
        compress: bool,
    ) -> std::io::Result<()> {
        // Set the options for the file, including compression and alignment
        let mut options = zip::write::SimpleFileOptions::default();
        if compress == false {
            options = options.compression_method(zip::CompressionMethod::Stored);
        }
        if alignment > 0 {
            let alignment = std::cmp::min(alignment, 61440);
            options = options.with_alignment(alignment as u16);
        }

        // Convert the path to a str and ensure it's relative
        let path = path.as_ref().to_path_buf();
        assert!(path.is_relative());

        // Start the file write
        zip.start_file(path.to_str().unwrap(), options)?;
        let written = std::io::copy(data, zip)?;
        assert_eq!(written, size);

        Ok(())
    }
}

/// Stores the content and config for the package, as well as a manifest for those parts
struct PackageContentAndConfig {
    /// The image manifest - this is a JSON string that contains the config and image layer blob.
    manifest: PackageBlobString,

    /// The config blob - this is the serialised JSON string from PackageConfig object.
    config: PackageBlobString,

    /// The actual content of the package, typically a tarball or an EROFS image file.
    /// OCI format supports multiple layers, however for now we only support a single content
    /// blob for the package.
    data: PackageContent,
}

impl PackageContentAndConfig {
    /// Creates a new signature object.  This is not used yet, but will be used to create the
    /// signature manifest and signed blob.
    fn new(
        config: String,
        content: PackageContent,
        aux_content: &Vec<Box<dyn PackageBlob>>,
        ref_name: &Option<String>,
    ) -> PackageContentAndConfig {
        // Wrap the string as a blob
        let mut config_blob = PackageBlobString::new(MediaType::Other(MEDIA_TYPE_PACKAGE_CONFIG.to_string()), config);

        // And annotate it with the title
        config_blob.add_annotation(oci_spec::image::ANNOTATION_TITLE, "package-config.json");

        // Add the descriptors for the image layers, the first is always the content blob
        let mut layers = vec![content.descriptor()];
        for aux in aux_content {
            layers.push(aux.descriptor());
        }

        // Build the manifest for the content and config
        let manifest = oci_spec::image::ImageManifestBuilder::default()
            .schema_version(SCHEMA_VERSION)
            .media_type(MediaType::ImageManifest)
            .artifact_type(MediaType::Other(MEDIA_TYPE_PACKAGE_ARTIFACT_TYPE.to_string()))
            .config(config_blob.descriptor())
            .layers(layers)
            .build()
            .expect("Failed to create image manifest");

        // Convert the signature manifest to a JSON string, this is what gets written to the package
        let manifest_str = manifest
            .to_string_pretty()
            .expect("Failed to convert content manifest to string");

        // Convert the manifest JSON to a blob
        let mut manifest_blob = PackageBlobString::new(MediaType::ImageManifest, manifest_str);

        // If a ref_name was supplied then add as an annotation of the manifest
        if let Some(ref_name) = ref_name {
            manifest_blob.add_annotation(oci_spec::image::ANNOTATION_REF_NAME, &ref_name);
        }

        // Store the 3 blobs, the config and data blobs, as well as the JSON blob that stores the
        // manifest.
        PackageContentAndConfig {
            manifest: manifest_blob,
            config: config_blob,
            data: content,
        }
    }

    /// Returns a descriptor that references the manifest blob for the signature.  This contains the
    /// SHA256 hash of the manifest, the size, and the media type along with any annotations.
    fn manifest_descriptor(&self) -> oci_spec::image::Descriptor {
        self.manifest.descriptor()
    }

    /// Returns the blob that contains the first entry in the image layer list, i.e. the actual
    /// content of the package.
    fn image_blob(&mut self) -> PackageContent {
        self.data.clone()
    }

    /// Returns the blob that contains the JSON config for the package.
    fn config_blob(&self) -> PackageBlobString {
        self.config.clone()
    }

    /// Returns the manifest blob for the content and config.
    fn manifest_blob(&self) -> PackageBlobString {
        self.manifest.clone()
    }
}

pub trait ReadAndSeek: io::Read + io::Seek {}
impl<T: io::Read + io::Seek> ReadAndSeek for T {}

pub struct PackageBuilder {
    /// The Writer object to write the package content to.  This can be a directory or a tarball.
    output: PackageWriter,

    /// Optional refName for the package.  This is used to set the ref.name annotation
    ref_name: Option<String>,

    /// The signing configuration for the package - if None then the package will not be signed.
    signing_config: Option<SigningConfig>,

    /// The package configuration JSON string.  This is a required field.
    config: Option<String>,

    /// The package content.  This is a required field.
    content: Option<PackageContent>,

    /// Any additional auxiliary blobs to add to the package.
    aux_content: Vec<Box<dyn PackageBlob>>,

    /// Any additional raw files to add to the package. These files aren't added to the blob directory
    /// and aren't referenced in the manifest, but are simply added to the package.
    raw_files: HashMap<PathBuf, Box<dyn ReadAndSeek>>,
}

impl PackageBuilder {
    /// Creates a new PackageBuilder unpopulated with any data. It will create a tarball at the
    /// given location with the contents. At a minimum you need to supply a config and some content.
    #[allow(unused)]
    pub fn new_to_tar<P: AsRef<Path>>(file_name: P) -> PackageBuilder {
        // Create a new file to write the tar into
        let file = File::create(file_name).expect("Failed to create package file");

        // Create a tar builder around the file
        let mut tar = tar::Builder::new(file);

        // Append a pax global header to the tarball to avoid warnings from tar tools
        let mut header = tar::Header::new_ustar();
        let data_as_bytes: &[u8] = "45 comment=RDK Application Layer Format v1.0\n".as_bytes();
        header.set_size(data_as_bytes.len() as u64);
        header.set_entry_type(tar::EntryType::XGlobalHeader);
        header.set_cksum();
        tar.append(&header, data_as_bytes)
            .expect("Failed to add RALF global package header to tarball");

        // Return a builder writing the contents to the tar file
        PackageBuilder {
            output: PackageWriter::Tar { tar },
            ref_name: None,
            signing_config: None,
            config: None,
            content: None,
            aux_content: Vec::new(),
            raw_files: HashMap::new(),
        }
    }

    /// Same as new_to_tar, but creates a zip archive instead of a tarball.
    #[allow(unused)]
    pub fn new_to_zip<P: AsRef<Path>>(file_name: P) -> PackageBuilder {
        // Create a new file to write the tar into
        let file = File::create(file_name).expect("Failed to create package file");

        // Create a zip builder around the file
        let mut zip = zip::ZipWriter::new(file);

        // Return a builder writing the contents to the tar file
        PackageBuilder {
            output: PackageWriter::Zip { zip },
            ref_name: None,
            signing_config: None,
            config: None,
            content: None,
            aux_content: Vec::new(),
            raw_files: HashMap::new(),
        }
    }

    /// Same as new_to_zip, but creates a zip archive around an existing file handle instead of
    /// creating a new file.  The exiting file handle must be seekable and writable and will be
    /// truncated to zero length.
    #[allow(unused)]
    pub fn new_to_zip_file(file: File) -> PackageBuilder {
        // Create a zip builder around the file
        let mut zip = zip::ZipWriter::new(file);

        // Return a builder writing the contents to the tar file
        PackageBuilder {
            output: PackageWriter::Zip { zip },
            ref_name: None,
            signing_config: None,
            config: None,
            content: None,
            aux_content: Vec::new(),
            raw_files: HashMap::new(),
        }
    }

    /// Create a builder that writes the package contents to the given target directory.
    #[allow(unused)]
    pub fn new_to_dir<P: AsRef<Path>>(target_dir: P) -> PackageBuilder {
        todo!("Directory output has been removed for now");

        /*
        // Create the target directory if it doesn't exist
        std::fs::create_dir_all(target_dir.as_ref()).expect("Failed to create target directory");

        // Check if the target directory is a valid directory
        if !target_dir.as_ref().is_dir() {
            panic!("Target directory is not a valid directory");
        }

        // Return a builder writing the contents to the directory
        PackageBuilder {
            output: PackageWriter::Dir {
                base_dir: target_dir.as_ref().to_path_buf(),
            },
            ref_name: None,
            signing_config: None,
            config: None,
            content: None,
            aux_content: Vec::new(),
            raw_files: HashMap::new(),
        }
        */
    }

    /// Sets the optional refname for the package. This is used to set the ref.name annotation in
    /// the manifest descriptor.  Typically, this is the "<appId>:<version>" of the package.
    #[allow(unused)]
    pub fn ref_name(mut self, ref_name: &str) -> PackageBuilder {
        self.ref_name = Some(ref_name.to_string());
        self
    }

    /// Sets the signing configuration for the package. This is technically optional, but for
    /// running on a device it is required.
    pub fn signing_config(mut self, config: SigningConfig) -> PackageBuilder {
        self.signing_config = Some(config);
        self
    }

    /// Sets the package configuration for the package. This is a required field.
    /// This converts the config to JSON, calculates the SHA256 hash of the JSON, and writes it as
    /// a blob to the package.
    pub fn config(mut self, config: &str) -> PackageBuilder {
        self.config = Some(config.to_string());
        self
    }

    /// Writes the content to the package. The content is written as a blob into the tarball,
    /// and the SHA256 hash of the blob is written to the package descriptor.
    pub fn content(mut self, content: PackageContent) -> PackageBuilder {
        self.content = Some(content);
        self
    }

    /// Writes addition content blobs to the package. These blobs are added to the image "layers"
    /// with the given media type.
    pub fn auxiliary_content(mut self, content: Box<dyn PackageBlob>) -> PackageBuilder {
        self.aux_content.push(content);
        self
    }

    /// Adds additional raw files to the package.  These files aren't added to the blob directory
    /// and aren't referenced in the manifest, but are simply added to the package.
    #[allow(unused)]
    pub fn append_raw_file<P: AsRef<Path>>(mut self, path: P, reader: Box<dyn ReadAndSeek>) -> PackageBuilder {
        self.raw_files.insert(path.as_ref().to_path_buf(), reader);
        self
    }

    /// Builds the package in the given location.
    pub fn build(mut self) -> io::Result<File> {
        // Check we have a config and content
        if self.config.is_none() || self.content.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "No config and / or content provided",
            ));
        }

        // List of manifests to write to the index.json file
        let mut manifests: Vec<Descriptor> = Vec::new();

        // Take the config and content out of the builder
        let content = self.content.take().unwrap();
        let config = self.config.take().unwrap();

        // Wrap the config and content in a PackageContentAndConfig object
        let mut content_and_config = PackageContentAndConfig::new(config, content, &self.aux_content, &self.ref_name);

        // Create the image manifest for the content and config
        let content_and_config_desc = content_and_config.manifest_descriptor();

        // Get the digest of the content and config manifest, this is the SHA256 hash of the
        // JSON string that contains the manifest.
        let content_and_config_digest = content_and_config_desc.digest();

        // If we have a signing config, then create the signature manifest.  Returns None if signing
        // config was not supplied to the builder
        let mut signature = None;
        if let Some(config) = self.signing_config.as_ref() {
            let signature_ = PackageSignature::new(content_and_config_digest, config);
            let signature_manifest = signature_.manifest_descriptor();

            manifests.push(signature_manifest);
            signature = Some(signature_);
        }

        // Add to the list of manifests to write to the index.json file
        manifests.insert(0, content_and_config_desc);

        // Build the index.json file for the package
        let index_json = oci_spec::image::ImageIndexBuilder::default()
            .schema_version(SCHEMA_VERSION)
            .media_type(MediaType::ImageIndex)
            .manifests(manifests)
            .build()
            .expect("Failed to create image index");

        // First write the oci-layout file to the output
        self._write_oci_layout()?;

        // Then write the index.json file to the output
        self._write_index_json(index_json)?;

        // Now write the config and content blobs to the output
        self.output.append_blob(&mut content_and_config.manifest_blob())?;
        self.output.append_blob(&mut content_and_config.config_blob())?;

        // If the content is compressed, ie a tar.gz, tar.zstd, EROFS image, then we need to add
        // the content without compression
        let mut image_blob = content_and_config.image_blob();
        if image_blob.media_type() == MediaType::Other(MEDIA_TYPE_PACKAGE_CONTENT_TAR.to_string()) {
            self.output.append_blob_aligned(&mut image_blob, DEFAULT_ALIGNMENT)?;
        } else {
            self.output
                .append_blob_aligned_uncompr(&mut image_blob, DEFAULT_ALIGNMENT)?;
        }

        // Write any auxiliary blobs to the output
        for mut aux in self.aux_content.drain(..) {
            self.output.append_boxed_blob(aux.borrow_mut())?;
        }

        // If we have a signature, then write the signature blobs to the output
        if let Some(signature) = signature {
            self.output.append_blob(&mut signature.manifest_blob())?;
            self.output.append_blob(&mut signature.config_blob())?;
            self.output.append_blob(&mut signature.data_blob())?;
        }

        // Write any additional raw files to the output
        for (path, mut reader) in self.raw_files.drain() {
            let size = reader.seek(io::SeekFrom::End(0))?;
            reader.seek(io::SeekFrom::Start(0))?;
            self.output.append_file(path, &mut reader, size)?;
        }

        // Finish the output if it's a tar or zip archive
        self.output.finish()
    }

    /// Writes the boilerplate "oci-layout" file to the output.
    fn _write_oci_layout(&mut self) -> io::Result<()> {
        // Create the oci-layout file
        let oci_layout = OciLayoutBuilder::default()
            .image_layout_version("1.0.0")
            .build()
            .expect("build oci layout");

        let str = oci_layout
            .to_string_pretty()
            .expect("Failed to convert oci-layout to string");

        // Write the oci-layout file to the output
        self.output
            .append_file("oci-layout", &mut str.as_bytes(), str.len() as u64)
    }

    /// Serialises and writes the supplied index.json file to the output.
    fn _write_index_json(&mut self, index_json: oci_spec::image::ImageIndex) -> io::Result<()> {
        let str = index_json
            .to_string_pretty()
            .expect("Failed to convert image index to string");

        self.output
            .append_file("index.json", &mut str.as_bytes(), str.len() as u64)
    }
}
