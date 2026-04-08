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

use crate::package::{PackageBuilder, PackageOutputFormat};
use crate::package_config::PackageConfig;
use crate::package_content::{PackageContentBuilder, PackageContentFormat};
use crate::signing_config::{SigningConfig, SigningOptions};
use crate::utils;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

pub const EXAMPLES: &str = color_print::cstr!("<bold><underline>Examples:</underline></bold>

  # Create an unsigned RALF package from a config JSON file and content directory
  ralfpack create --config somecfg.json --content somedir/ <<RALF_PACKAGE>>

  # Create a signed RALF package from a config JSON file and content archive
  ralfpack create --config somecfg.json --content somecontent.tar.gz --pkcs12 signing.p12 <<RALF_PACKAGE>>

  # Create a signed RALF package from a config JSON file and content directory using separate PEM files for signing
  ralfpack create --config somecfg.json --content somedir/ --key private.pem --cert certificate.pem --cert-chain chain.pem <<RALF_PACKAGE>>

  # Create an unsigned RALF package from a config JSON file and content directory formatting content as EROFS image within the package
  ralfpack create --config somecfg.json --content somedir/ --image-format erofs <<RALF_PACKAGE>>

");

#[derive(clap::Args)]
pub struct CreateArgs {
    /// The path to a directory or archive containing the content to be packaged.
    #[arg(short = 'i', long)]
    content: PathBuf,

    /// The path to a JSON file containing the package configuration.
    #[arg(short, long)]
    config: PathBuf,

    /// Disabled the JSON schema check on the configuration file.
    #[arg(long, default_value_t = false)]
    no_schema_check: bool,

    /// The format to use for the package content image. The tool supports tar with optional
    /// compression (gzip or zstd) and EROFS images.  By default, the tool will use tar for small
    /// packages and EROFS for larger packages.
    /// Possible values are: 'tar', 'tar.gz', 'tar.zst', 'erofs' (alias for 'erofs.lz4'), 'erofs.lz4',
    /// 'erofs.zstd' & 'erofs.nocmpr' (uncompressed).
    #[arg(long)]
    image_format: Option<PackageContentFormat>,

    /// TODO: Include extra key=value annotations in the package.
    #[arg(long)]
    annotations: Option<String>,

    /// TODO: Include an auxiliary metadata file in the package.
    #[arg(long)]
    auxiliary_content: Option<String>,

    /// Hidden option to set the output package format, currently only 'zip' is supported.
    #[arg(long, default_value = "zip", hide = true)]
    package_format: String,

    /// Shared signing options
    #[command(flatten)]
    signing: SigningOptions,

    /// Output package path
    ralf_package: PathBuf,
}

/// Simple helper that guesses the desired content format based on the archive format of the
/// package and size of the file or directory content passed in.
///
/// The thinking here is that large packages should use EROFS as it's more efficient for large
/// packages to be directly mounted and used, while smaller packages can be tarballs that are
/// quickly extracted to RAM and used.
///
/// Also, if the package format is already compressed (zip or tar.gz/tar.zst) then the content
/// format should not be compressed.
///
fn _desired_content_format<P: AsRef<Path>>(
    package_format: &PackageOutputFormat,
    content_path: P,
) -> Result<PackageContentFormat, String> {
    // If the path doesn't exist then we cannot determine the size
    if !content_path.as_ref().exists() {
        return Err(format!("Content path {:?} does not exist", content_path.as_ref()));
    }

    // If it's a file then we need to check if it's an archive and get the extracted size
    let total_size: u64;
    if content_path.as_ref().is_file() {
        // It's a file, so assume it's an archive and get the extracted size
        match utils::archive_extracted_size(&content_path) {
            Ok(size) => total_size = size,
            Err(e) => {
                return Err(format!(
                    "Failed to determine uncompressed size of {:?}: {}",
                    content_path.as_ref(),
                    e
                ));
            }
        }
    } else {
        // It's a directory, so get the size of the directory
        match fs_extra::dir::get_size(&content_path) {
            Ok(size) => total_size = size,
            Err(e) => {
                return Err(format!("Failed to get size of {:?}: {}", content_path.as_ref(), e));
            }
        }
    }

    // If the total size is greater than 1MB, then we use EROFS, otherwise we use tar
    if total_size > (1 * 1024 * 1024) {
        Ok(PackageContentFormat::ErofsLz4)
    } else {
        match package_format {
            PackageOutputFormat::Zip => {
                // Zip is compressed, so use uncompressed tar for the image as will be compressed in the zip
                Ok(PackageContentFormat::Tar)
            }
            PackageOutputFormat::Tar | PackageOutputFormat::Directory => {
                // Plain tar is uncompressed, so use uncompressed tar for the image
                Ok(PackageContentFormat::TarGz)
            }
        }
    }
}

/// Creates a new signed or unsigned RALF package based on the provided arguments.
pub fn create_package(args: CreateArgs) -> Result<(), String> {
    // Check that both the config and content paths exist
    if !args.config.exists() {
        return Err(format!("The specified config file does not exist: {:?}", args.config));
    }
    if !args.content.exists() {
        return Err(format!("The specified content path does not exist: {:?}", args.content));
    }

    // If signing options are provided then create the signing config
    let mut signing_config: Option<SigningConfig> = None;
    if args.signing.pkcs12.is_some() || args.signing.key.is_some() {
        let signing_config_ = SigningConfig::from_options(&args.signing)?;
        signing_config = Some(signing_config_);
    }

    // For now default to always creating a zip package
    let package_format;
    match args.package_format.to_lowercase().as_str() {
        "zip" => {
            package_format = PackageOutputFormat::Zip;
        }
        "tar" => {
            package_format = PackageOutputFormat::Tar;
        }
        "dir" | "directory" => {
            return Err("The 'directory' package format is not currently fully supported".to_string());
        }
        _ => {
            return Err(format!(
                "Invalid package format '{}', must be 'zip'",
                args.package_format
            ));
        }
    }

    // Check if the image format is specified, if not we need to guess it based on the content size
    let image_format: PackageContentFormat;
    if args.image_format.is_none() {
        image_format = _desired_content_format(&package_format, &args.content)?;
    } else {
        image_format = args.image_format.unwrap();
    }

    // If creating a package from a config file and content directory, then we need to read the
    // config file and convert it to a JSON object
    let mut config_file = File::open(&args.config).map_err(|e| format!("Failed to open config file: {}", e))?;

    // Read the entire file into a string
    let mut pkg_config = String::new();
    config_file
        .read_to_string(&mut pkg_config)
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    // Check the config against the JSON schema
    if !args.no_schema_check {
        if !PackageConfig::validate_str(&pkg_config) {
            return Err("The provided config file is not valid according to the JSON schema".to_string());
        }
    }

    // Do a basic deserialization to ensure it's valid JSON and has some of the required fields
    let parsed_config: PackageConfig =
        serde_json::from_str(&pkg_config).map_err(|e| format!("Failed to parse config file: {}", e))?;

    // Create the object that will populate a temporary file in the format requested with the
    // content
    let mut content_builder = PackageContentBuilder::new(&image_format);

    // Limit the max size of the content to 512MB, most devices impose a lower limit than this on
    // packages, so this is just a sanity check
    content_builder.set_size_limit(512 * 1024 * 1024);

    // Limit the number of files in the package, again the device has internal limits when extracting
    // and using packages, so this is just a sanity check
    content_builder.set_entry_limit(32 * 1024);

    // On MacOS exclude certain paths that MacOS adds by default to directories and archives
    if cfg!(target_os = "macos") {
        content_builder.exclude_file(".DS_Store");
        content_builder.exclude_file("__MACOSX");
        content_builder.exclude_file("__MACOSX/*");
    }

    // Add the actual content to the package from the directory or archive specified
    if args.content.is_dir() {
        // Read the directory and copy all the files to the package content
        content_builder
            .append_dir_contents(&args.content)
            .map_err(|e| format!("Failed to create content from directory: {}", e))?;
    } else if args.content.is_file() {
        // Read the contents of the archive file and copy it to the package content
        content_builder
            .append_archive_contents(&args.content)
            .map_err(|e| format!("Failed to create content from archive: {}", e))?;
    } else {
        return Err(format!(
            "The specified content path is neither a file nor a directory: {:?}",
            args.content
        ));
    }

    // Create the package content from the builder
    let pkg_content = content_builder
        .build()
        .map_err(|e| format!("Failed to build package content: {}", e))?;

    // Create the package using the builder
    let mut package_builder: PackageBuilder;
    match package_format {
        PackageOutputFormat::Zip => {
            package_builder = PackageBuilder::new_to_zip(&args.ralf_package);
        }
        PackageOutputFormat::Tar => {
            package_builder = PackageBuilder::new_to_tar(&args.ralf_package);
        }
        PackageOutputFormat::Directory => {
            package_builder = PackageBuilder::new_to_dir(&args.ralf_package);
        }
    }

    // Add the config and content to the package builder
    package_builder = package_builder.config(&pkg_config).content(pkg_content);

    // Add any auxiliary content to the package builder
    // for blob in pkg_aux_content {
    //    package_builder = package_builder.auxiliary_content(blob);
    //}

    // Set the package id as the ref_name, which is then set as an annotation in the package
    package_builder = package_builder.ref_name(parsed_config.id.as_str());

    // If signing then add the signing config (private key, certs, etc)
    if let Some(mut signing_config) = signing_config {
        // Set the signing identifier to the package id if not already set
        if signing_config.signature_identity().is_none() {
            signing_config.set_signature_identity(parsed_config.id.as_str());
        }

        // Set the signing config
        package_builder = package_builder.signing_config(signing_config);
    }

    // Finally build the package
    package_builder
        .build()
        .map_err(|e| format!("Failed to build package: {}", e))?;

    println!("Created {}", args.ralf_package.display());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_desired_content_format_invalid_archive() {
        // Create a temporary file with invalid archive content (no magic bytes)
        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
        temp_file.write_all(b"AAAA").unwrap();

        let package_format = PackageOutputFormat::Zip;
        let result = _desired_content_format(&package_format, temp_file.path());

        assert!(
            result.is_err(),
            "Expected an error for invalid archive format, but got OK"
        );
        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("Failed to determine uncompressed size of"),
            "Error message did not match expected pattern: {}",
            err_msg
        );
        assert!(
            err_msg.contains("Unknown archive format"),
            "Error message did not match expected pattern: {}",
            err_msg
        );
    }
}
