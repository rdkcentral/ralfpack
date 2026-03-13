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

use crate::package::PackageBuilder;
use crate::package_reader::RalfPackage;
use crate::signing_config::{SigningConfig, SigningOptions};
use std::io::Seek;
use std::path::PathBuf;
use tempfile::tempfile;

pub const EXAMPLES: &str = color_print::cstr!("<bold><underline>Examples:</underline></bold>

  # Sign a RALF package using a PKCS12 file containing the private key and optional certificate(s)
  ralfpack sign --pkcs12 signing.p12 --passphrase mypassword <<RALF_PACKAGE>>

  # Sign a RALF package using separate PEM files for the private key, certificate and signing chain
  ralfpack sign --key private.pem --cert certificate.pem --cert-chain chain.pem <<RALF_PACKAGE>>

  # Sign a RALF package using just a private key (no certificates)
  ralfpack sign --key private.pem --passphrase mypassword <<RALF_PACKAGE>>

  # Sign a RALF package where the key password is set as an environment variable called MY_KEY_PASS
  ralfpack sign --key private.pem --passphrase env://MY_KEY_PASS --cert certificate.pem --cert-chain chain.pem <<RALF_PACKAGE>>

");

#[derive(clap::Args)]
pub struct SignArgs {
    /// Shared signing options
    #[command(flatten)]
    signing: SigningOptions,

    /// Output package path
    ralf_package: PathBuf,
}

/// Signs or re-signs a RALF package using the specified signing options.
pub fn sign_package(args: SignArgs) -> Result<(), String> {
    // Open the RALF package
    let package = RalfPackage::open(&args.ralf_package)?;

    // Extract the package contents to a temporary file
    let pkg_content = package.content()?;

    // Extract the package configuration
    let pkg_config = package.raw_config()?;

    // Get any auxiliary files
    let pkg_aux_content = package.auxiliary_content()?;

    // Get the signing configuration from the command line arguments
    let mut signing_config = SigningConfig::from_options(&args.signing)?;

    // Set the signing package id if not already set
    if signing_config.signature_identity().is_none() {
        let parsed_config = package.config()?;
        signing_config.set_signature_identity(parsed_config.id.as_str());
    }

    // Now put it back together, in another temporary file, but signed
    let temp_file = tempfile().map_err(|e| format!("Failed to create temp file: {}", e))?;

    // Create the package using the builder
    let mut package_builder = PackageBuilder::new_to_zip_file(temp_file)
        .config(pkg_config.as_str())
        .content(pkg_content);

    // Add any auxiliary content to the package builder
    for blob in pkg_aux_content {
        package_builder = package_builder.auxiliary_content(Box::new(blob));
    }

    // Set the signing config
    package_builder = package_builder.signing_config(signing_config);

    // Finally build the package into the temporary file
    let mut new_package = package_builder
        .build()
        .map_err(|e| format!("Failed to rebuild package with new signature: {}", e))?;

    // Drop the original package to close any open file handles on it
    drop(package);

    // Rewind the temporary file to the start
    new_package
        .rewind()
        .map_err(|e| format!("Failed to rewind temp file: {}", e))?;

    // Reopen the ralf package for writing and truncate it
    let mut old_package = std::fs::File::create(&args.ralf_package)
        .map_err(|e| format!("Failed to open output package file for writing: {}", e))?;

    // Now copy the contents of the temporary file to the output file
    std::io::copy(&mut new_package, &mut old_package)
        .map_err(|e| format!("Failed to re-write the package file with new signature: {}", e))?;

    println!("Successfully signed {}", args.ralf_package.display());

    Ok(())
}
