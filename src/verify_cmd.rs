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

use crate::package_reader::RalfPackage;
use std::path::PathBuf;
use std::str::FromStr;

pub const EXAMPLES: &str = color_print::cstr!(
    "<bold><underline>Examples:</underline></bold>

  # Verify a RALF package against a certificate authority bundle
  ralfpack verify --ca-roots ca-bundle.pem <<RALF_PACKAGE>>

  # Verify a RALF package against a certificate authority bundle ignoring certificate expiration
  ralfpack verify --ca-roots ca-bundle.pem --no-check-time <<RALF_PACKAGE>>

  # Verify a RALF package against a local certificate, its (optional) chain, along with a CA bundle
  ralfpack verify --cert cert.pem --cert-chain chain.pem --ca-roots ca-bundle.pem <<RALF_PACKAGE>>

  # Verify a RALF package using a public key
  ralfpack verify --key public.pem <<RALF_PACKAGE>>

"
);

#[derive(clap::Args)]
pub struct VerifyArgs {
    /// Path to a PEM file containing one or more trusted CA root certificates.  Use this option
    /// if the package signature contains a signing certificate and (optionally) a certificate chain
    /// to build a chain to one of the given CA roots.  If the package signature does not contain a
    /// signing certificate, this option must be used in conjunction with --certificate (or --key).
    #[arg(long)]
    ca_roots: Option<PathBuf>,

    /// Path to a PEM file containing a trusted public key.  Use this option if the package
    /// signature does not contain a signing certificate, or if you want to verify the signature
    /// using a specific public key.
    #[arg(long)]
    key: Option<PathBuf>,

    /// Path to a PEM file containing a public certificate, which will be verified along with
    /// the (optional) certificate chain and given CA roots.  If the signature in the package
    /// already contains the signing certificate, this option is not required, but if given, it
    /// will override any certificate in the package signature.
    #[arg(long, alias = "cert")]
    certificate: Option<PathBuf>,

    /// Path to a PEM file containing an optional certificate chain, which will be used to build
    /// a certificate chain from the given certificate to one of the given CA roots. If the signature
    /// in the package already contains a certificate chain, this option is not required, but if
    /// given, it will override any certificate chain in the package signature.
    #[arg(long, alias = "cert-chain")]
    certificate_chain: Option<PathBuf>,

    /// Do not check the validity period of certificates when verifying, this includes any CA
    /// roots, the signing certificate and any certificates in the chain.
    /// This can be useful when verifying packages signed with certificates that have expired,
    /// but you still want to verify the signature and the certificate chain.
    #[arg(long, default_value_t = false)]
    no_check_time: bool,

    /// Input package path
    ralf_package: PathBuf,
}

///
/// Verifies the signature of a RALF package using the specified verification options.
pub fn verify_package(args: VerifyArgs) -> Result<(), String> {
    // Check that at least one of --ca-roots or --key is provided
    if args.ca_roots.is_none() && args.key.is_none() {
        return Err("Either --ca-roots or --key must be specified for verification".to_string());
    }
    if args.key.is_some() && (args.ca_roots.is_some() || args.certificate.is_some() || args.certificate_chain.is_some())
    {
        return Err("Only one of --key or --ca-roots (or --certificate or --certificate-chain) can be specified for verification".to_string());
    }

    // Open the RALF package
    let package = RalfPackage::open(&args.ralf_package)?;

    // Get the signing info
    let mut signing_info = package.signature()?;

    // The public key to use for the verification
    let public_key;

    // If key is not provided then we need to either get the certificate from the package or have
    // it provided on the command line, then we need to verify that certificate against the CA roots
    if args.key.is_some() {
        // Parse the public key in a form we can use
        public_key = parse_public_key(args.key.as_ref().unwrap())?;
    } else {
        // If a certificate or certificate chain was provided on the command line then use those
        // instead of the ones in the package signature
        if let Some(cert_file) = args.certificate {
            signing_info.certificate = Some(parse_certificate(cert_file)?);
        }
        if let Some(chain_file) = args.certificate_chain {
            signing_info.certificate_chain = Some(parse_certificate_chain(chain_file)?);
        }

        // We must have a signing certificate now
        if signing_info.certificate.is_none() {
            return Err("No signing certificate found in package signature, and none provided on command line".to_string());
        }
        let cert = signing_info.certificate.as_ref().unwrap();

        // Get the CA roots, we must have been given them on the command line
        let ca_store = parse_ca_roots(args.ca_roots.unwrap(), args.no_check_time)?;

        // Verify the certificate chain against the CA roots
        verify_certificate_chain(
            &cert,
            &signing_info.certificate_chain,
            &ca_store
        )?;

        // It all checks out so extract the public key from the signing certificate
        public_key = cert.public_key()
            .map_err(|e| format!("Failed to extract public key from signing certificate: {}", e))?;
    }

    // Verify the signed blob using the public key
    verify_blob(&signing_info.signed_blob, &signing_info.signature, public_key)?;

    // The signature has verified the JSON blob, so we now need to check what was in the JSON blob
    // and make sure it matches the package we have.
    let image_digest = parse_and_verify_signed_blob(&signing_info.signed_blob)?;
    if image_digest != package.content_manifest_digest() {
        return Err(format!(
            "Package manifest digest does not match signed manifest digest: package='{}' signed='{}'",
            package.content_manifest_digest(),
            image_digest
        ));
    }

    // Final step is to verify all the blobs in the content manifest are present in the package
    // and that their digests match.
    package.verify_all_content_blobs()?;

    println!("Package signature verification succeeded");

    Ok(())
}

///
/// Helper function to read a public key PEM file and parse it into an OpenSSL PKey
fn parse_public_key(path: &PathBuf) -> Result<openssl::pkey::PKey<openssl::pkey::Public>, String> {
    let key_data = std::fs::read(path).map_err(|e| format!("Failed to read public key file: {}", e))?;

    openssl::pkey::PKey::public_key_from_pem(key_data.as_slice())
        .map_err(|e| format!("Failed to parse public key PEM file: {}", e))
}

///
/// Helper function to read a certificate PEM file and parse it into an OpenSSL X509
fn parse_certificate(path: PathBuf) -> Result<openssl::x509::X509, String> {
    let cert_data = std::fs::read(path).map_err(|e| format!("Failed to read certificate file: {}", e))?;

    openssl::x509::X509::from_pem(cert_data.as_slice())
        .map_err(|e| format!("Failed to parse certificate PEM file: {}", e))
}

///
/// Helper function to read a certificate chain PEM file and parse it into a stack of OpenSSL X509
fn parse_certificate_chain(path: PathBuf) -> Result<openssl::stack::Stack<openssl::x509::X509>, String> {
    let chain_data = std::fs::read(path).map_err(|e| format!("Failed to read certificate chain file: {}", e))?;

    let chain_certs = openssl::x509::X509::stack_from_pem(chain_data.as_slice())
        .map_err(|e| format!("Failed to parse certificate chain PEM file: {}", e))?;

    let mut stack = openssl::stack::Stack::new()
        .map_err(|e| format!("Failed to create certificate stack: {}", e))?;
    for cert in chain_certs {
        stack.push(cert)
            .map_err(|e| format!("Failed to add certificate to stack: {}", e))?;
    }

    Ok(stack)
}

///
/// Helper function to read a CA roots PEM file and parse it into a stack of OpenSSL X509
fn parse_ca_roots(path: PathBuf, no_time_check: bool) -> Result<openssl::x509::store::X509Store, String> {
    let ca_data = std::fs::read(path).map_err(|e| format!("Failed to read CA roots file: {}", e))?;

    let ca_certs = openssl::x509::X509::stack_from_pem(ca_data.as_slice())
        .map_err(|e| format!("Failed to parse CA roots PEM file: {}", e))?;

    let mut store_builder = openssl::x509::store::X509StoreBuilder::new()
        .map_err(|e| format!("Failed to create X509 store builder: {}", e))?;

    if no_time_check {
        store_builder.set_flags(openssl::x509::verify::X509VerifyFlags::NO_CHECK_TIME)
            .map_err(|e| format!("Failed to set NO_CHECK_TIME flag on X509 store: {}", e))?;
    }

    for ca in ca_certs {
        store_builder
            .add_cert(ca)
            .map_err(|e| format!("Failed to add CA root to X509 store: {}", e))?;
    }

    Ok(store_builder.build())
}

///
/// Performs certificate chain verification using the given signing certificate, optional chain,
/// and CA roots.
fn verify_certificate_chain(
    cert: &openssl::x509::X509,
    chain: &Option<openssl::stack::Stack<openssl::x509::X509>>,
    ca_roots: &openssl::x509::store::X509Store,
) -> Result<(), String> {

    // If no certificate chain was provided then use an empty stack
    let empty_stack = openssl::stack::Stack::new()
        .map_err(|e| format!("Failed to create empty certificate stack: {}", e))?;
    let chain_ref = chain.as_ref().unwrap_or(&empty_stack);

    // The X509 store context is used in a bit of a weird way in rust, look at the docs for details
    let mut context = openssl::x509::X509StoreContext::new()
        .map_err(|e| format!("Failed to create X509 store context: {}", e))?;
    let result = context
        .init(&ca_roots, &cert, &chain_ref, |c| c.verify_cert())
        .map_err(|e| format!("Certificate verification failed: {}", e))?;

    // Check the result
    if result == false {
        return Err("Failed to verify the signing certificate against the ca-root certificate(s)".to_string());
    }

    Ok(())
}

///
/// Helper function to verify a signed blob using a public key
fn verify_blob(
    signed_blob: &Vec<u8>,
    signature: &Vec<u8>,
    public_key: openssl::pkey::PKey<openssl::pkey::Public>,
) -> Result<(), String> {
    // Create a verifier
    let mut verifier = openssl::sign::Verifier::new(openssl::hash::MessageDigest::sha256(), &public_key)
        .map_err(|e| format!("Failed to create verifier: {}", e))?;

    // Update the verifier with the signed blob
    verifier
        .update(signed_blob)
        .map_err(|e| format!("Failed to update verifier: {}", e))?;

    // Verify the signature
    let result = verifier
        .verify(signature)
        .map_err(|e| format!("Failed to verify signature: {}", e))?;

    if result == false {
        Err("Signature verification failed".to_string())
    } else {
        Ok(())
    }
}

///
/// Helper function to flatten a JSON structure into key/value pairs with slash-separated keys
fn flatten_json<'a>(value: &'a serde_json::Value, prefix: String, out: &mut Vec<(String, &'a serde_json::Value)>) {
    match value {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                let new_prefix = if prefix.is_empty() {
                    format!("/{}", k)
                } else {
                    format!("{}/{}", prefix, k)
                };
                flatten_json(v, new_prefix, out);
            }
        }
        _ => {
            out.push((prefix, value));
        }
    }
}

///
/// JSON parses the signed blob and checks that the JSON strictly matches the requirements in
/// https://github.com/containers/image/blob/main/docs/containers-signature.5.md
fn parse_and_verify_signed_blob(signed_blob: &Vec<u8>) -> Result<oci_spec::image::Digest, String> {
    // Parse the signed blob as JSON
    let parsed: serde_json::Value = serde_json::from_slice(signed_blob.as_slice())
        .map_err(|e| format!("Failed to parse signed blob as JSON: {}", e))?;

    // Walk through the flattened structure to verify it matches the spec
    if !parsed.is_object() {
        return Err("Signed blob is not a JSON object".to_string());
    }

    // Get the "critical" field, it must be present
    let critical = parsed
        .get("critical")
        .ok_or("Missing 'critical' field in signed blob".to_string())?;
    if !critical.is_object() {
        return Err("Invalid 'critical' field in signed blob".to_string());
    }

    // Get the "optional" field, it must be present and an object, but its contents are not important
    let optional = parsed
        .get("optional")
        .ok_or("Missing 'optional' field in signed blob".to_string())?;
    if !optional.is_object() {
        return Err("Invalid 'optional' field in signed blob".to_string());
    }

    // Flatten the critical object structure as easier to identify unexpected fields
    let mut flat: Vec<(String, &serde_json::Value)> = Vec::new();
    flatten_json(&critical, String::new(), &mut flat);

    let mut critical_type: Option<String> = None;
    let mut docker_reference: Option<String> = None;
    let mut manifest_digest: Option<String> = None;

    for (key, value) in &flat {
        if !value.is_string() {
            return Err(format!(
                "Invalid value for field '/critical/{}' in signed blob, must be a string",
                key
            ));
        }

        let value = value
            .as_str()
            .ok_or(format!("Invalid value for field '/critical/{}' in signed blob", key))?;

        match key.as_str() {
            "/type" => {
                critical_type = Some(value.to_string());
            }
            "/identity/docker-reference" => {
                docker_reference = Some(value.to_string());
            }
            "/image/docker-manifest-digest" => {
                manifest_digest = Some(value.to_string());
            }
            _ => return Err(format!("Unexpected field in signed blob: /critical/{}", key)),
        }
    }

    // Check that the required fields were found
    let critical_type = critical_type.ok_or("Missing '/critical/type' field in signed blob".to_string())?;
    let _docker_reference =
        docker_reference.ok_or("Missing '/critical/identity/docker-reference' field in signed blob".to_string())?;
    let manifest_digest =
        manifest_digest.ok_or("Missing '/critical/image/docker-manifest-digest' field in signed blob".to_string())?;

    // Check the type field is correct
    if critical_type != "cosign container image signature" {
        return Err(format!(
            "Invalid value for '/critical/type' field in signed blob: expected 'cosign container image signature', got '{}'",
            critical_type
        ));
    }

    // This is the critical bit, this is the digest of the image manifest that was signed
    let digest = oci_spec::image::Digest::from_str(manifest_digest.as_str())
        .map_err(|e| format!("Invalid manifest digest in signed blob: {}", e))?;
    if digest.algorithm() != &oci_spec::image::DigestAlgorithm::Sha256 {
        return Err(format!(
            "Unsupported manifest digest algorithm in signed blob: {}",
            digest.algorithm()
        ));
    }

    Ok(digest)
}
