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

use openssl::pkcs12::Pkcs12;
use openssl::pkey::{PKey, Private};
use openssl::stack::Stack;
use openssl::x509::X509;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

/// Command line options for signing a package.
/// These options are used for the 'create', 'convert' and 'sign' commands.
/// You can use a populated SigningConfig object to create a SigningConfig struct which is then used
/// to sign the package.
#[derive(clap::Args, Debug)]
pub struct SigningOptions {
    /// The path to PKCS12 (.p12) file containing the certificate(s) and private key for signing.
    #[arg(long, conflicts_with = "key")]
    pub pkcs12: Option<PathBuf>,

    /// Path to the RSA PEM key file used for signing the package.
    #[arg(long, conflicts_with = "pkcs12")]
    pub key: Option<PathBuf>,

    /// The passphrase for the key file, if '-' then the passphrase is read from stdin. If no
    /// passphrase is provided then you may be prompted for it if the key is encrypted.
    /// Passphrase may also be set as an environment variable by specifying 'env://[ENV_VAR_NAME]'
    #[arg(long)]
    pub passphrase: Option<String>,

    /// Path to the X.509 certificate in PEM format to include in the OCI Signature.
    #[arg(long, alias = "cert")]
    pub certificate: Option<PathBuf>,

    /// Path to a PEM file containing one or more X.509 certificates to include in the OCI Signature
    /// to build the certificate chain for verifying the signing certificate. This optional argument
    /// can be specified multiple times to include multiple discrete X.509 certificates.
    /// These certificates are included in the package signature.
    #[arg(long, alias = "cert-chain", action = clap::ArgAction::Append)]
    pub certificate_chain: Vec<PathBuf>,

    /// Manually set the .critical.docker-reference field in the Signature to the given value.
    /// By default, this is set to the package id (e.g. com.example.myapp).
    #[arg(long)]
    pub signature_identity: Option<String>,

    /// Skip the check on the expiry of the signing certificate (and optional chain).  By default,
    /// the tool checks that the signing certificate(s) have at least 3 years before expiry.
    #[arg(long, default_value_t = false)]
    pub skip_certificate_expiry_check: bool,
}

/// Structure to hold the signing configuration for a package.
pub struct SigningConfig {
    /// The parsed private key to use for signing the package.
    key: PKey<Private>,

    /// The optional X509 certificate containing the public side of the private key.  This is added
    /// to the package and used for verification.
    certificate: Option<X509>,

    /// The optional additional X.509 certificates to be included in the package.  These are used
    /// for verification of the signing certificate against the CA certificate on the device.
    certificate_chain: Option<Stack<X509>>,

    /// The optional identity to include in the signature, if not set then the package id is used.
    signature_identity: Option<String>,
}

/// Loads a file into memory with a limit on the size of the file.
fn read_file_with_limit<P: AsRef<Path>>(file_path: P, limit: usize) -> std::io::Result<Vec<u8>> {
    let file = std::fs::File::open(file_path)?;
    let mut buffer = Vec::with_capacity(limit);
    file.take(limit as u64).read_to_end(&mut buffer)?;
    Ok(buffer)
}

fn passphrase_prompt() -> String {
    let mut passphrase = String::new();
    println!("Enter passphrase: ");
    io::stdin().read_line(&mut passphrase).expect("Failed to read line");
    passphrase.trim().to_string()
}

/// Callback function used to prompt the user for a passphrase when reading a PEM or PKCS12 file.
fn passphrase_callback_prompt(returned_passphrase: &mut [u8]) -> Result<usize, openssl::error::ErrorStack> {
    let passphrase = passphrase_prompt();
    let passphrase_len = passphrase.len();
    returned_passphrase.copy_from_slice(passphrase.as_bytes());
    Ok(passphrase_len)
}

/// Processes a PKCS12 file and returns a SigningConfig object.  The PKCS12 file must contain a
/// private key, otherwise an error is returned.  The PKCS12 file may also contain a certificate and
/// a certificate chain.
fn process_pkcs12_file<P: AsRef<Path>>(file_path: P, passphrase: &str) -> Result<SigningConfig, String> {
    let result = read_file_with_limit(file_path, 1024 * 1024);
    if result.is_err() {
        return Err(format!("Error reading PKCS12 file: {:?}", result.unwrap_err()));
    }

    let result = Pkcs12::from_der(result.unwrap().as_slice());
    if result.is_err() {
        return Err(format!("Error parsing PKCS12 file: {:?}", result.err()));
    }

    let pkcs12 = result.unwrap();
    let mut result = pkcs12.parse2(passphrase);
    if result.is_err() {
        // If no passphrase was provided, then prompt the user for it and try again
        if passphrase.is_empty() {
            let passphrase = passphrase_prompt();
            result = pkcs12.parse2(&passphrase);
        }

        if result.is_err() {
            return Err(format!("Error parsing PKCS12 file: {:?}", result.err()));
        }
    }

    let parsed_pkcs12 = result.unwrap();
    if parsed_pkcs12.pkey.is_none() {
        return Err("No private key found in PKCS12 file.".to_string());
    }

    Ok(SigningConfig {
        key: parsed_pkcs12.pkey.unwrap(),
        certificate: parsed_pkcs12.cert,
        certificate_chain: parsed_pkcs12.ca,
        signature_identity: None,
    })
}

/// Processes a PEM private key file and returns the object.  If no passphrase is provided, then
/// the user may be prompted to enter one on the command line.
fn process_pem_key_file<P: AsRef<Path>>(file_path: P, passphrase: &str) -> Result<PKey<Private>, String> {
    let result = read_file_with_limit(&file_path, 1024 * 1024);
    if result.is_err() {
        return Err(format!(
            "Error reading file '{}': {}",
            &file_path.as_ref().display(),
            result.unwrap_err()
        ));
    }

    if !passphrase.is_empty() {
        // Try and read the file using the supplied passphrase
        let result =
            PKey::private_key_from_pem_passphrase(result.unwrap().as_slice(), passphrase.as_bytes()).map_err(|e| {
                format!(
                    "Error reading private key file '{}': {}",
                    file_path.as_ref().display(),
                    e
                )
            })?;

        Ok(result)
    } else {
        // Try and read the file without a passphrase
        let result = PKey::private_key_from_pem_callback(result.unwrap().as_slice(), passphrase_callback_prompt)
            .map_err(|e| {
                format!(
                    "Error reading private key file '{}': {}",
                    file_path.as_ref().display(),
                    e
                )
            })?;

        Ok(result)
    }
}

fn process_pem_certificate_file<P: AsRef<Path>>(file_path: P) -> Result<X509, String> {
    let result = read_file_with_limit(&file_path, 1024 * 1024);
    if result.is_err() {
        log::error!(
            "Error reading PEM certificate file '{}': {}",
            file_path.as_ref().display(),
            result.unwrap_err()
        );
        std::process::exit(1);
    }

    let result = X509::from_pem(result.unwrap().as_slice());
    if result.is_err() {
        log::error!("Error reading certificate file: {}", result.unwrap_err());
        std::process::exit(1);
    }

    Ok(result.unwrap())
}

fn process_pem_certificate_stack_file<P: AsRef<Path>>(file_path: P) -> Result<Stack<X509>, String> {
    let result = read_file_with_limit(&file_path, 1024 * 1024);
    if result.is_err() {
        log::error!(
            "Error reading certificate stack file '{}': {}",
            file_path.as_ref().display(),
            result.unwrap_err()
        );
        std::process::exit(1);
    }

    let result = X509::stack_from_pem(result.unwrap().as_slice());
    if result.is_err() {
        log::error!("Error reading certificate chain file: {}", result.unwrap_err());
        std::process::exit(1);
    }

    let x509_vec = result.unwrap();

    let x509_stack = Stack::new();
    if x509_stack.is_err() {
        log::error!("Error creating X509 stack: {:?}", x509_stack.err());
        std::process::exit(1);
    }

    let mut x509_stack = x509_stack.unwrap();
    for x509 in x509_vec {
        let result = x509_stack.push(x509);
        if result.is_err() {
            log::error!("Error pushing X509 certificate to stack: {}", result.unwrap_err());
            std::process::exit(1);
        }
    }

    Ok(x509_stack)
}

fn check_signing_cert_expiry(signing_config: &SigningConfig) -> bool {
    let min_expiry = openssl::asn1::Asn1Time::days_from_now(365 * 3);
    if min_expiry.is_err() {
        log::error!("Error getting minimum not after date: {:?}", min_expiry.err());
        return false;
    }

    let min_expiry = min_expiry.unwrap();

    if let Some(cert) = &signing_config.certificate {
        let expiry = cert.not_after();
        if expiry < min_expiry {
            log::error!(
                "Signing certificate expires in less than 3 years - expires on {}",
                expiry
            );
            return false;
        }
    }

    if let Some(chain) = &signing_config.certificate_chain {
        for cert in chain.iter() {
            let expiry = cert.not_after();
            if expiry < min_expiry {
                log::error!(
                    "Certificate in chain expires in less than 3 years - expires on {}",
                    expiry
                );
                return false;
            }
        }
    }

    true
}

impl SigningConfig {
    /// Creates a signing config object from the command line options supplied by the user.
    /// This does all the necessary processing of the files and passphrases.
    /// If there are any errors, then a SigningConfigError is returned.
    pub fn from_options(options: &SigningOptions) -> Result<SigningConfig, String> {
        // Run a sanity check on the command line options, we allow both a PKCS12 file or a PEM key
        // file, but only allow one key.
        if options.pkcs12.is_none() && options.key.is_none() {
            return Err("You must supply a key or a pkcs12 file containing a key.".to_string());
        }

        // Get the passphrase from the command line options, environment variable or prompt, may
        // return an empty string if no passphrase is provided (which is valid for unencrypted keys)
        let passphrase = Self::get_passphrase(options)?;

        // Signing credentials, only private key is required
        let mut key: Option<PKey<Private>> = None;
        let mut cert: Option<X509> = None;
        let mut cert_chain: Option<Stack<X509>> = None;

        // If a PKCS12 file is specified, then use that to create the initial signing config
        if let Some(pkcs12_path) = &options.pkcs12 {
            let p12_signing_config = SigningConfig::from_pcks12_file(pkcs12_path, &passphrase)?;
            key = Some(p12_signing_config.key);
            cert = p12_signing_config.certificate;
            cert_chain = p12_signing_config.certificate_chain;
        }

        // If a PEM key file is specified, then fetch that ... but only if we haven't already got
        // a key from the PKCS12 file
        if let Some(key_path) = &options.key {
            if key.is_some() {
                return Err(
                    "Cannot specify both a PKCS12 file containing a private key and a PEM key file.".to_string(),
                );
            }

            let pem_key = process_pem_key_file(key_path, &passphrase)?;
            key = Some(pem_key);
        }

        // Check that we actually have a private key by this point
        if key.is_none() {
            return Err(
                "No private key found for signing, did you specify PKCS12 file containing a private key?.".to_string(),
            );
        }

        // If a PEM certificate file is specified, then fetch that ... but it's an error if we
        // already have a certificate from the PKCS12 file as well
        if let Some(cert_path) = &options.certificate {
            if cert.is_some() {
                return Err(
                    "Cannot specify both a PKCS12 file containing a signing certificate and a PEM certificate file."
                        .to_string(),
                );
            }
            let pem_cert = process_pem_certificate_file(cert_path)?;
            cert = Some(pem_cert);
        }

        // If one or more PEM certificate chain files are specified, then fetch those and add them
        // to the chain
        if options.certificate_chain.len() > 0 {
            // Creat an empty chain if we don't already have one from the PKCS12 file
            if cert_chain.is_none() {
                let new_chain = Stack::new().map_err(|e| format!("Error creating certificate stack: {}", e))?;
                cert_chain = Some(new_chain);
            }

            // Add the certificates from each specified file to the chain
            for cert_chain_path in &options.certificate_chain {
                let cert_stack = process_pem_certificate_stack_file(cert_chain_path)?;
                for cert_stack_cert in cert_stack.iter() {

                    // Check if we already have this certificate in the chain
                    let mut found = false;
                    if let Some(existing_chain) = &cert_chain {
                        for existing_cert in existing_chain.iter() {
                            if existing_cert == cert_stack_cert {
                                log::warn!(
                                    "Certificate from {} is already in the certificate chain, skipping duplicate",
                                    cert_chain_path.display()
                                );
                                found = true;
                                break;
                            }
                        }
                    }

                    // If not found, then add it to the chain
                    if !found {
                        cert_chain
                            .as_mut()
                            .unwrap()
                            .push(cert_stack_cert.to_owned())
                            .map_err(|e| {
                                format!(
                                    "Error adding certificate(s) from {} to stack: {}",
                                    cert_chain_path.display(),
                                    e
                                )
                            })?;
                    }
                }
            }
        }

        // Finally build up the signing config object from the components we have
        let mut signing_config = SigningConfig {
            key: key.unwrap(), // We must have a key by this point
            certificate: cert,
            certificate_chain: cert_chain,
            signature_identity: None,
        };

        // Check the expiry of the signing certificate(s) unless the user has specified to skip this
        if !options.skip_certificate_expiry_check {
            if !signing_config.check_signing_cert_expiry() {
                return Err("Signing certificate expiry check failed.".to_string());
            }
        }

        // If a signature identity is specified, set it in the signing config
        if let Some(identity) = &options.signature_identity {
            signing_config.signature_identity = Some(identity.clone());
        }

        // Okay, return the signing config
        Ok(signing_config)
    }

    /// Gets the passphrase from the command line options, environment variable or prompts the user
    /// to enter it.  If no passphrase is provided, then an empty string is returned.
    fn get_passphrase(options: &SigningOptions) -> Result<String, String> {
        let mut passphrase: String = String::new();
        if let Some(value) = &options.passphrase {
            if value == "-" {
                // read the passphrase from stdin
                std::io::stdin()
                    .read_line(&mut passphrase)
                    .map_err(|err| format!("Failed to read passphrase from stdin: {}", err))?;
            } else if value.starts_with("env://") {
                // read the passphrase from the specified environment variable
                let env_var_name = &value[6..];
                let env_var_value = std::env::var(env_var_name).map_err(|err| {
                    format!(
                        "Failed to read passphrase from environment variable {}: {}",
                        env_var_name, err
                    )
                })?;
                passphrase = env_var_value;
            } else {
                // use the passphrase provided on the command line
                passphrase = value.clone();
            }

            let trimmed_passphrase = passphrase.trim();
            if trimmed_passphrase.is_empty() {
                return Err("Passphrase cannot be empty.".to_string());
            }

            passphrase = trimmed_passphrase.to_string();
        }

        Ok(passphrase)
    }

    fn from_pcks12_file<P: AsRef<Path>>(file_path: P, passphrase: &str) -> Result<SigningConfig, String> {
        process_pkcs12_file(file_path, passphrase)
    }

    #[allow(dead_code)]
    fn from_files<P: AsRef<Path>>(
        key_path: &P,
        passphrase: &str,
        cert_path: &Option<P>,
        cert_chain_path: &Option<P>,
    ) -> Result<SigningConfig, String> {
        let mut config = SigningConfig {
            key: process_pem_key_file(key_path, passphrase)?,
            certificate: None,
            certificate_chain: None,
            signature_identity: None,
        };

        if let Some(cert_path) = cert_path {
            let cert = process_pem_certificate_file(cert_path)?;
            config.certificate = Some(cert);
        }
        if let Some(cert_chain_path) = cert_chain_path {
            let chain = process_pem_certificate_stack_file(cert_chain_path)?;
            config.certificate_chain = Some(chain);
        }

        Ok(config)
    }

    pub fn check_signing_cert_expiry(&self) -> bool {
        check_signing_cert_expiry(&self)
    }

    /// Signs a byte array using the private key stored within the SigningConfig object. The
    /// signature is returned as a byte vector.
    pub fn sign_bytes(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        let mut signer = openssl::sign::Signer::new(openssl::hash::MessageDigest::sha256(), &self.key)
            .map_err(|err| format!("Failed to create signer: {}", err))?;

        signer.update(data)
            .map_err(|err| format!("Failed to sign data: {}", err))?;

        let signature = signer.sign_to_vec()
            .map_err(|err| format!("Failed to sign data: {}", err))?;

        Ok(signature)
    }

    /// If a certificate was provided, this returns the PEM encoded certificate as a String.
    pub fn certificate(&self) -> Option<String> {
        if self.certificate.is_none() {
            return None;
        }

        let cert = self.certificate.as_ref().unwrap();
        let pem = cert.to_pem().expect("Failed to convert certificate to pem");

        let str = String::from_utf8(pem).expect("Failed to convert pem to string");

        Some(str)
    }

    /// If a certificate chain was provided, this returns the PEM encoded certificate chain as
    /// a String containing all the PEM encoded certificates concatenated together.
    pub fn certificate_chain(&self) -> Option<String> {
        if self.certificate_chain.is_none() || self.certificate_chain.as_ref().unwrap().len() == 0 {
            return None;
        }

        let mut chain = String::new();
        let stack = self.certificate_chain.as_ref().unwrap();
        for cert in stack.iter() {
            let pem = cert.to_pem().expect("Failed to convert certificate to pem");

            let str = String::from_utf8(pem).expect("Failed to convert pem to string");

            chain.push_str(&str);
        }

        Some(chain)
    }

    pub fn signature_identity(&self) -> Option<String> {
        self.signature_identity.clone()
    }

    pub fn set_signature_identity(&mut self, identity: &str) {
        self.signature_identity = Some(identity.to_string());
    }
}
