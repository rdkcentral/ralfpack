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

use clap::Parser;

mod convert_cmd;
mod create_cmd;
mod info_cmd;
mod package;
mod package_config;
mod package_content;
mod package_reader;
mod package_signature;
mod sign_cmd;
mod signing_config;
mod utils;
mod verify_cmd;

mod entos {
    pub mod config_xml;
    mod configs;
    mod convertors;
    mod media_types;
    mod permissions;
    pub mod widget;
}

mod erofs {
    pub mod erofs_image;
    mod erofs_sys;
}

mod dmverity {
    pub mod dmverity_gen;
}

// Include the generated-file as a separate module
pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}
pub use built_info as build_info;

#[derive(clap::Parser)]
#[command(name = "ralfpack")]
#[command(author = "Sky UK")]
#[command(version)]
#[command(about = "A tool for creating and managing packages")]
#[command(max_term_width = 160)]
struct Cli {
    /// The subcommand to run
    #[command(subcommand)]
    command: Commands,

    /// Increase output verbosity, can be used multiple times.
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    verbose: u8,
}

/// The possible sub-commands for the tool
#[derive(clap::Subcommand)]
enum Commands {
    /// Create a new package
    #[command(after_help = create_cmd::EXAMPLES)]
    Create(create_cmd::CreateArgs),

    /// Convert an EntOS widget to RALF package format
    #[command(after_help = convert_cmd::EXAMPLES)]
    Convert(convert_cmd::ConvertArgs),

    /// Sign or resign an existing package
    #[command(after_help = sign_cmd::EXAMPLES)]
    Sign(sign_cmd::SignArgs),

    /// Verify a package signature
    #[command(after_help = verify_cmd::EXAMPLES)]
    Verify(verify_cmd::VerifyArgs),

    /// Display information about a package
    #[command(after_help = info_cmd::EXAMPLES)]
    Info(info_cmd::InfoArgs),
}

fn main() {
    // Parse the CLI arguments
    let cli = Cli::parse();

    // Increase the log level based on the verbosity count
    let mut log_level = log::LevelFilter::Warn;
    for _ in 0..cli.verbose {
        match log_level {
            log::LevelFilter::Error => log_level = log::LevelFilter::Warn,
            log::LevelFilter::Warn => log_level = log::LevelFilter::Info,
            log::LevelFilter::Info => log_level = log::LevelFilter::Debug,
            log::LevelFilter::Debug => log_level = log::LevelFilter::Trace,
            log::LevelFilter::Trace => break,
            _ => {}
        }
    }

    // Create a semi-standard logger using colog with the desired log level
    let mut builder = colog::default_builder();
    builder.filter_level(log_level);
    builder.init();

    // Set the log level
    log::set_max_level(log_level);

    // Match on the subcommand and call the appropriate function
    let result = match cli.command {
        Commands::Create(args) => create_cmd::create_package(args),
        Commands::Sign(args) => sign_cmd::sign_package(args),
        Commands::Verify(args) => verify_cmd::verify_package(args),
        Commands::Convert(args) => convert_cmd::convert_widget(args),
        Commands::Info(args) => info_cmd::display_package_info(args),
    };

    // Check for errors
    if result.is_err() {
        log::error!("Error: {}", result.err().unwrap());
        std::process::exit(libc::EXIT_FAILURE);
    }

    // Successful
    std::process::exit(libc::EXIT_SUCCESS);
}
