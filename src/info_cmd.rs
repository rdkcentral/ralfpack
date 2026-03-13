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
use crate::package_reader::{PackageSignatureInfo, RalfPackage};
use serde::Serialize;
use std::path::PathBuf;

pub const EXAMPLES: &str = color_print::cstr!(
    "<bold><underline>Examples:</underline></bold>

  # Dump summary information about a RALF package
  ralfpack info <<RALF_PACKAGE>>

"
);

/// The info command just has sub-commands so this struct is just a wrapper
#[derive(clap::Args)]
pub struct InfoArgs {
    #[command(subcommand)]
    command: Option<InfoCommands>,

    /// RALF package path
    ralf_package: Option<PathBuf>,
}

/// Sub-commands for the 'info' command
#[derive(clap::Subcommand)]
enum InfoCommands {
    /// Display the configuration of a package
    #[command(after_help = CONFIG_EXAMPLES)]
    Config(ConfigArgs),

    /// Display signing information about a package
    #[command(after_help = SIGNING_INFO_EXAMPLES)]
    Signing(SigningInfoArgs),
}

/// Options for the 'info summary' sub-command
#[derive(clap::Args)]
struct SummaryArgs {
    /// RALF package path
    ralf_package: PathBuf,
}

/// Options for the 'info config' sub-command
#[derive(clap::Args)]
struct ConfigArgs {
    /// The format to output the configuration in, either 'raw', 'json' or 'configxml', defaults to 'json'.
    #[arg(short, long, default_value = "json")]
    format: String,

    /// RALF package path
    ralf_package: PathBuf,
}

const CONFIG_EXAMPLES: &str = color_print::cstr!(
    "<bold><underline>Examples:</underline></bold>

  # Dump the config JSON file to stdout
  ralfpack info config <<RALF_PACKAGE>>

  # Convert the config JSON to EntOS widget config.xml format, this may be lossy as not all fields
  # have a direct mapping in config.xml
  ralfpack info config --format configxml <<RALF_PACKAGE>>

"
);

/// Options for the 'info signing' sub-command
#[derive(clap::Args)]
struct SigningInfoArgs {
    /// If set, also output the full certificate chain (if present) after the signing certificate
    #[arg(long, alias = "cert-chain", default_value = "false")]
    certificate_chain: bool,

    /// When outputting certificates, if this flag is set then pretty print the certificate(s)
    /// in openssl style text format, otherwise output in raw PEM format
    #[arg(long, default_value = "false")]
    text: bool,

    /// RALF package path
    ralf_package: PathBuf,
}

const SIGNING_INFO_EXAMPLES: &str = color_print::cstr!(
    "<bold><underline>Examples:</underline></bold>

  # Dump the signing certificate in PEM format to stdout
  ralfpack info signing <<RALF_PACKAGE>>

  # Dump the signing certificate and certificate chain (if present) in PEM format to stdout
  ralfpack info signing --cert-chain <<RALF_PACKAGE>>

  # Dump the signing certificate and certificate chain (if present) in openssl style readable text format to stdout
  ralfpack info signing --cert-chain --text <<RALF_PACKAGE>>

"
);

/// Handler for the top level 'info' command, just dispatches to the sub-commands
pub fn display_package_info(args: InfoArgs) -> Result<(), String> {
    match &args.command {
        None => display_package_summary(args.ralf_package),
        Some(InfoCommands::Config(config_args)) => display_package_config(config_args),
        Some(InfoCommands::Signing(signing_info_args)) => display_package_signature(signing_info_args),
    }
}

/// Opens the package and extracts the config from it and then outputs it in the requested format
fn display_package_config(args: &ConfigArgs) -> Result<(), String> {
    // Open the RALF package
    let package = RalfPackage::open(&args.ralf_package)?;

    // Serialize and output the config in the requested format
    match args.format.as_str() {
        "raw" => {
            // Just output the raw config JSON as-is
            let raw_config = package.raw_config()?;
            println!("{}", raw_config);
            Ok(())
        }
        "json" => {
            // Get the package config
            let config = package.config()?;
            let config_json = serde_json::to_string_pretty(&config)
                .map_err(|e| format!("Failed to serialize config to JSON: {}", e))?;
            println!("{}", config_json);
            Ok(())
        }
        "configxml" => {
            // Get the package config
            let config = package.config()?;

            // Convert to config.xml format and serialize to XML
            let mut config_xml_string = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n".to_string();
            let mut serializer = quick_xml::se::Serializer::new(&mut config_xml_string);
            serializer.indent(' ', 4);

            let config_xml = entos::config_xml::ConfigXml::from(&config);
            config_xml
                .serialize(serializer)
                .map_err(|e| format!("Failed to serialize config to XML: {}", e))?;

            println!("{}", config_xml_string);
            Ok(())
        }
        _ => Err("Invalid format specified, must be 'raw', 'json' or 'configxml'".to_string()),
    }
}

/// Displays the signing certificate and certificate chain (if present) in either  raw PEM format or
/// pretty printed text format.
fn display_package_signature(args: &SigningInfoArgs) -> Result<(), String> {
    // Open the RALF package
    let package = RalfPackage::open(&args.ralf_package)?;

    // Get the signing info
    let signing_info = package.signature()?;

    // Check there a signing certificate included, it is not mandatory for a package to include one
    // (if not there, then it's assumed the public certificate is known by the verifier)
    if signing_info.certificate.is_none() {
        println!("Package is signed, but no signing certificate info in the package");
        return Ok(());
    }

    // Print the signing certificate
    if let Some(certificate) = &signing_info.certificate {
        let text = print_certificate(certificate, args.text)?;
        println!("{}", text);
    }

    // Print the certificate chain if requested
    if args.certificate_chain {
        if let Some(chain) = &signing_info.certificate_chain {
            for cert in chain {
                let text = print_certificate(cert, args.text)?;
                println!("{}", text);
            }
        }
    }

    Ok(())
}

/// Helper to pretty print a x509 certificate in openssl style text format
fn print_certificate(cert: &openssl::x509::X509Ref, text_fmt: bool) -> Result<String, String> {
    if text_fmt {
        let text = cert
            .to_text()
            .map_err(|e| format!("Failed to convert certificate to text format: {}", e))?;
        let text_str =
            String::from_utf8(text).map_err(|e| format!("Failed to convert certificate text to UTF-8: {}", e))?;
        Ok(text_str)
    } else {
        let pem = cert
            .to_pem()
            .map_err(|e| format!("Failed to convert certificate to PEM format: {}", e))?;
        let pem_str =
            String::from_utf8(pem).map_err(|e| format!("Failed to convert certificate PEM to UTF-8: {}", e))?;
        Ok(pem_str)
    }
}

/// Helper to get the next expiry date of the signing certificate as a string
fn get_next_cert_expiry(signing_info: &PackageSignatureInfo) -> String {
    let invalid_time = openssl::asn1::Asn1Time::from_unix(4917934523);
    if invalid_time.is_err() {
        return "err!".to_string();
    }

    let invalid_time = invalid_time.unwrap();
    let mut closest = invalid_time.as_ref();

    if let Some(cert) = &signing_info.certificate {
        if cert.not_after() < closest {
            closest = cert.not_after().to_owned();
        }
    }

    if let Some(chain) = &signing_info.certificate_chain {
        for cert in chain {
            if cert.not_after() < closest {
                closest = cert.not_after().to_owned();
            }
        }
    }

    if closest == invalid_time {
        return "invalid".to_string();
    }

    format!("{}", closest)
}

/// Displays a brief summary about the package
fn display_package_summary(ralf_package: Option<PathBuf>) -> Result<(), String> {
    // Check that the package path is provided
    let package_path = match ralf_package {
        Some(path) => path,
        None => {
            return Err("No RALF package path provided".to_string());
        }
    };

    // Open the RALF package
    let package = RalfPackage::open(&package_path)?;

    // Get basic info about the package
    let config = package.config()?;
    let signing_info = package.signature();

    // Print summary information
    println!("package: {}", package_path.display());
    println!("id: {}", config.id);
    println!("version: {}", config.version);
    println!("version_name: {}", config.version_name.unwrap_or("-".to_string()));

    println!("signed: {}", if signing_info.is_ok() { "true" } else { "false" });
    if let Ok(signing_info) = signing_info {
        println!("signature:");
        if signing_info.certificate.is_some() {
            println!("  signing_cert: true");
            println!(
                "  signing_cert_chain: {}",
                if signing_info.certificate_chain.is_some() {
                    "true"
                } else {
                    "false"
                }
            );
            println!("  signing_cert_next_expiry: {}", get_next_cert_expiry(&signing_info));
        } else {
            println!("  signing_cert: false");
        }
    }

    println!("config:");
    println!("  entrypoint: {}", config.entry_point);
    println!("  dependencies:");
    if config.dependencies.is_empty() {
        println!("    - None");
    } else {
        for dep in &config.dependencies {
            println!("    - {} ({})", dep.0, dep.1);
        }
    }

    println!();

    Ok(())
}
