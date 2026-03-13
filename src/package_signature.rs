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

use serde_json::json;
use std::time::SystemTime;

use crate::build_info;
use crate::package::{PackageBlob, PackageBlobString};
use crate::signing_config::SigningConfig;

pub const MEDIA_TYPE_COSIGN_SIGNATURE: &str = "application/vnd.dev.cosign.simplesigning.v1+json";
pub const ANNOTATION_COSIGN_SIGNATURE: &str = "dev.cosignproject.cosign/signature";
pub const ANNOTATION_COSIGN_CERTIFICATE: &str = "dev.sigstore.cosign/certificate";
pub const ANNOTATION_COSIGN_CHAIN: &str = "dev.sigstore.cosign/chain";

/// Represents the signature of a package.  It contains 3 pieces of information that need to be
/// written to the package:
///     1. The signature manifest - this is a JSON string that contains the config and signed blob.
///     2. The signed blob - this is a JSON string that references the manifest of the content and
///        is signed by the private key.
///     3. The config blob - this is not actually used, but is included in the package for
///        compatibility with the cosign tooling.
///
pub struct PackageSignature {
    /// The signature manifest - this is a JSON string that contains the config and signed blob.
    manifest: PackageBlobString,

    /// The signed data blob - this is a JSON string that references the manifest of the content
    data: PackageBlobString,

    /// The config blob - this is not actually used, but is included in the package for cosign compatibility
    config: PackageBlobString,
}

impl PackageSignature {
    /// Creates a new signature object.  This is not used yet, but will be used to create the
    /// signature manifest and signed blob.
    pub fn new(image_manifest_digest: &oci_spec::image::Digest, config: &SigningConfig) -> PackageSignature {
        // Create the JSON blob that we're going to sign, the important thing is that it contains
        // the digest of the image manifest.
        let data_json = json!({
            "critical": {
                "identity": {
                    "docker-reference": config.signature_identity().unwrap_or(String::new()),
                },
                "image": {
                    "docker-manifest-digest": image_manifest_digest.to_string(),
                },
                "type": "cosign container image signature"
            },
            "optional": {
                "creator": format!("{} v{}", build_info::PKG_NAME, build_info::PKG_VERSION),
                "timestamp": SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(),
            }
        });

        // Convert the JSON to a string
        let data_json_str = data_json.to_string();

        // Now we need to sign the JSON blob with the private key
        let signature = config
            .sign_bytes(data_json_str.as_bytes())
            .expect("Failed to sign data blob");

        // Wrap the string as a blob
        let mut data_blob = PackageBlobString::new(
            oci_spec::image::MediaType::Other(MEDIA_TYPE_COSIGN_SIGNATURE.to_string()),
            data_json_str,
        );

        // Create the annotations for the data descriptor
        data_blob.add_annotation(
            ANNOTATION_COSIGN_SIGNATURE,
            &openssl::base64::encode_block(signature.as_slice()),
        );

        // Get the optional PEM encoded signing certificate from the signing config
        let certs = config.certificate();
        if certs.is_some() {
            data_blob.add_annotation(ANNOTATION_COSIGN_CERTIFICATE, &certs.unwrap());
        }

        // Get the optional PEM encoded certificate chain from the signing config
        let cert_chain = config.certificate_chain();
        if cert_chain.is_some() {
            data_blob.add_annotation(ANNOTATION_COSIGN_CHAIN, &cert_chain.unwrap());
        }

        // Create the boilerplate config (we don't use, but cosign tool generates it, so we follow suit)
        let config_json = json!({
            "architecture": "",
            "created": "0001-01-01T00:00:00Z",
            "history": [
                {
                    "created": "0001-01-01T00:00:00Z"
                }
            ],
            "os": "",
            "rootfs": {
                "type": "layers",
                "diff_ids": [
                    oci_spec::image::Digest::from(data_blob.digest())
                ]
            },
            "config": {}
        });

        // Wrap the string as a blob
        let config_blob = PackageBlobString::new(oci_spec::image::MediaType::ImageConfig, config_json.to_string());

        // Create the signature manifest
        let manifest = oci_spec::image::ImageManifestBuilder::default()
            .schema_version(oci_spec::image::SCHEMA_VERSION)
            .media_type(oci_spec::image::MediaType::ImageManifest)
            .config(config_blob.descriptor())
            .layers(vec![data_blob.descriptor()])
            .build()
            .expect("Failed to create signature manifest");

        // Convert the signature manifest to a JSON string, this is what gets written to the package
        let manifest_str = manifest
            .to_string_pretty()
            .expect("Failed to convert signature manifest to string");

        // Convert the manifest JSON to a blob
        let mut manifest_blob = PackageBlobString::new(oci_spec::image::MediaType::ImageManifest, manifest_str);

        // The ref.name annotation on the manifest descriptor, for the signature this takes the
        // form of "sha256-<digest>.sig", where <digest> is the SHA256 hash of the content manifest.
        manifest_blob.add_annotation(
            oci_spec::image::ANNOTATION_REF_NAME,
            &format!("sha256-{}.sig", image_manifest_digest.digest()),
        );

        // Store the 3 blobs, the config and data blobs, as well as the JSON blob that stores the
        // manifest.
        PackageSignature {
            manifest: manifest_blob,
            data: data_blob,
            config: config_blob,
        }
    }

    /// Returns a descriptor that references the manifest blob for the signature.  This contains the
    /// SHA256 hash of the manifest, the size, and the media type along with any annotations.
    pub fn manifest_descriptor(&self) -> oci_spec::image::Descriptor {
        self.manifest.descriptor()
    }

    /// Returns a Read object that contains the data blob for the signature.
    pub fn data_blob(&self) -> PackageBlobString {
        self.data.clone()
    }

    /// Returns a Read object that contains the config blob for the signature.
    pub fn config_blob(&self) -> PackageBlobString {
        self.config.clone()
    }

    /// Returns a Read object that contains the config blob for the signature.
    pub fn manifest_blob(&self) -> PackageBlobString {
        self.manifest.clone()
    }
}
