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

use crate::entos;
use crate::package::{PackageBuilder, PackageOutputFormat};
use crate::package_content::PackageContentFormat;
use crate::signing_config::{SigningConfig, SigningOptions};
use std::path::PathBuf;

pub const EXAMPLES: &str = color_print::cstr!("<bold><underline>Examples:</underline></bold>

  # Convert an EntOS Widget to an unsigned RALF package
  ralfpack convert --widget some.wgt <<RALF_PACKAGE>>

  # Convert an EntOS Widget to an signed RALF package using the specified PKCS12 file containing the private key and optional certificate(s)
  ralfpack convert --widget some.wgt --pkcs12 signing.p12 <<RALF_PACKAGE>>

  # Convert an EntOS Widget to a signed RALF package using separate PEM files for the private key, certificate and signing chain
  ralfpack convert --widget some.wgt --key private.pem --cert certificate.pem --cert-chain chain.pem <<RALF_PACKAGE>>

  # Convert an EntOS Widget to an unsigned RALF package, formatting content as EROFS image within the package
  ralfpack convert --widget some.wgt --image-format erofs <<RALF_PACKAGE>>

  # Convert an EntOS Widget to an unsigned RALF package with the given semantic version
  ralfpack convert --widget some.wgt --widget-version 1.2.3 <<RALF_PACKAGE>>

");

#[derive(clap::Args)]
pub struct ConvertArgs {
    /// Path to the widget file to convert
    #[arg(long)]
    widget: PathBuf,

    /// The semantic version of the widget file when converting to a RALF package. If not specified
    /// the tool will attempt to guess the semantic version from the version string in the config.xml
    #[arg(long)]
    widget_version: Option<String>,

    /// Optional argument to append a suffix to the `versionName` field in the package config, for
    /// example if the widget version is `1.2.3` and the suffix is `-beta1` then the resulting
    /// `versionName` in the package config will be `1.2.3-beta1`.
    #[arg(long)]
    version_name_suffix: Option<String>,

    /// The format to use for the package content image. The tool supports tar with optional
    /// compression (gzip or zstd) and EROFS images.  By default, the tool will use tar for small
    /// packages and EROFS for larger packages.
    /// Possible values are: 'tar', 'tar.gz', 'tar.zst', 'erofs' (alias for 'erofs.lz4'), 'erofs.lz4',
    /// 'erofs.zstd' & 'erofs.nocmpr' (uncompressed).
    #[arg(long)]
    image_format: Option<PackageContentFormat>,

    /// Hidden option to set the output package format, currently only 'zip' is supported.
    #[arg(long, default_value = "zip", hide = true)]
    package_format: String,

    /// Shared signing options
    #[command(flatten)]
    signing: SigningOptions,

    /// If not specified then the output package will contain a copy of the original config.xml
    /// from the widget. If this flag is set then the config.xml will be omitted from the package.
    /// The config.xml is not required in a RALF package, it is provided for backwards compatibility
    /// with EntOS apps that expect it to be present at runtime.
    #[arg(long)]
    remove_configxml: bool,

    /// Output package path
    ralf_package: PathBuf,
}

/// Converts an EntOS widget file to a RALF package.
pub fn convert_widget(args: ConvertArgs) -> Result<(), String> {
    // Get the package format, expect only zip at the moment
    let pkg_format;
    match args.package_format.to_lowercase().as_str() {
        "zip" => {
            pkg_format = PackageOutputFormat::Zip;
        }
        "tar" => {
            pkg_format = PackageOutputFormat::Tar;
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

    // If signing options are provided then create the signing config
    let mut signing_config: Option<SigningConfig> = None;
    if args.signing.pkcs12.is_some() || args.signing.key.is_some() {
        let signing_config_ = SigningConfig::from_options(&args.signing)?;
        signing_config = Some(signing_config_);
    }

    // Open the widget file
    let widget = entos::widget::Widget::open(&args.widget)?;

    // Check if the image format is specified, if not we need to guess it based on the content
    // in the widget
    let image_format: PackageContentFormat;
    if args.image_format.is_none() {
        let widget_size = widget.uncompressed_size()?;
        if widget_size > (1 * 1024 * 1024) {
            image_format = PackageContentFormat::ErofsLz4;
        } else {
            image_format = PackageContentFormat::Tar;
        }
    } else {
        image_format = args.image_format.unwrap();
    }

    // Create the package content extracted from the widget
    let pkg_content = widget.package_content(&image_format, args.remove_configxml)?;

    // Create the package configuration file from the widget's config.xml and the specified
    // version (or the version extracted from the config.xml if not specified)
    let pkg_config = widget.package_config(&args.widget_version, &args.version_name_suffix)?;

    // Check if the widget has app secrets and if so need to add the file as "auxiliary"
    // content to the package
    let pkg_app_secrets = widget.package_app_secrets()?;

    // Create the package using the builder
    let mut package_builder: PackageBuilder;
    match pkg_format {
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
    if let Some(secrets) = pkg_app_secrets {
        package_builder = package_builder.auxiliary_content(Box::new(secrets));
    }

    // Set the package id as the ref_name, which is then set as an annotation in the package
    package_builder = package_builder.ref_name(widget.app_id().as_str());

    // If signing then add the signing config (private key, certs, etc)
    if let Some(mut signing_config) = signing_config {
        // Set the signing identifier to the package id if not already set
        if signing_config.signature_identity().is_none() {
            signing_config.set_signature_identity(widget.app_id().as_str());
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
