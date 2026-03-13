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

use oci_spec::image::MediaType;
use std::cell::RefCell;
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::Path;

use crate::entos;
use crate::entos::config_xml::ConfigXml;

use crate::package::*;
use crate::package_config::PackageConfig;
use crate::package_content::*;

pub struct Widget {
    /// The zip archive / actual widget file
    archive: RefCell<zip::read::ZipArchive<File>>,

    /// The parsed config.xml file from the widget
    config_xml: ConfigXml,
}

impl Widget {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Widget, String> {
        // Open the ZIP file for reading
        let file = File::open(path).expect("Failed to open widget file");
        let mut archive = zip::read::ZipArchive::new(file).expect("Failed to read widget file");

        // Extract the config.xml file from the widget and convert it
        let config_xml_file = archive
            .by_name("config.xml")
            .map_err(|e| format!("Failed to read config.xml file: {}", e))?;
        let config_xml: ConfigXml = quick_xml::de::from_reader(io::BufReader::new(config_xml_file))
            .map_err(|e| format!("Failed to read / parse config.xml file in widget: {}", e))?;

        Ok(Widget {
            archive: RefCell::new(archive),
            config_xml: config_xml,
        })
    }

    /// Returns an OCI formatted package config JSON string from the widget's config.xml file.
    pub fn package_config(
        &self,
        version: &Option<String>,
        version_name_suffix: &Option<String>,
    ) -> Result<String, String> {
        let mut config = PackageConfig::from(&self.config_xml);

        // Override the version if supplied on the command line
        if let Some(version) = version {
            config.version = version.clone();
        }

        // Append the version name suffix if supplied on the command line
        if let Some(suffix) = version_name_suffix {
            if let Some(version_name) = config.version_name {
                config.version_name = Some(format!("{}{}", version_name, suffix));
            }
        }

        // Convert the config to a JSON string
        let config_json = serde_json::to_string_pretty(&config).expect("Failed to convert config.xml to JSON");

        Ok(config_json)
    }

    /// Returns the package content from the widget, converting it to the specified format.
    pub fn package_content(
        &self,
        format: &PackageContentFormat,
        exclude_configxml: bool,
    ) -> Result<PackageContent, String> {
        let mut content_builder = PackageContentBuilder::new(format);

        if exclude_configxml {
            log::debug!("Excluding config.xml from package content as requested");
            content_builder.exclude_file("config.xml");
        }

        content_builder
            .exclude_file("signature1.xml")
            .exclude_file("cherry.config")
            .exclude_file("manifest.json")
            .exclude_file("appsecrets.json")
            .append_zip(&mut self.archive.borrow_mut())
            .map_err(|e| format!("Failed to import widget content: {}", e))?;

        content_builder
            .build()
            .map_err(|e| format!("Failed to build package content: {}", e))
    }

    /// Returns the appsecrets file from the widget, if it exists.  The returned data is a package
    /// blob which can be directly added to the package manifest as an auxiliary layer.
    pub fn package_app_secrets(&self) -> Result<Option<PackageBlobString>, String> {
        let archive = &mut self.archive.borrow_mut();
        let appsecrets = archive.by_name("appsecrets.json");
        if let Ok(mut appsecrets_file) = appsecrets {
            let mut data = String::new();
            appsecrets_file
                .read_to_string(&mut data)
                .map_err(|e| format!("Failed to read appsecrets.json file: {}", e))?;

            log::info!("Found appsecrets.json in widget, adding to package");

            return Ok(Some(PackageBlobString::new(
                MediaType::Other(entos::media_types::ENTOS_APPSECRETS_MEDIATYPE.to_owned()),
                data,
            )));
        }

        Ok(None)
    }

    /// Returns the total uncompressed size of all files in the widget.
    pub fn uncompressed_size(&self) -> Result<u64, String> {
        let archive = &mut self.archive.borrow_mut();
        let mut total_size: u64 = 0;

        for i in 0..archive.len() {
            let file = archive
                .by_index(i)
                .map_err(|e| format!("Failed to read archive file: {}", e))?;

            total_size += file.size();
        }

        Ok(total_size)
    }

    /// Returns the raw config.xml file from the widget as a string.
    #[allow(unused)]
    pub fn raw_config_xml(&self) -> Result<String, String> {
        let archive = &mut self.archive.borrow_mut();
        let config_xml_file = archive
            .by_name("config.xml")
            .expect("Failed to find config.xml in widget file");
        let mut data = String::new();
        let mut reader = io::BufReader::new(config_xml_file);
        reader
            .read_to_string(&mut data)
            .map_err(|e| format!("Failed to read config.xml file: {}", e))?;

        Ok(data)
    }

    /// Returns the app_id from the widget's config.xml file.
    pub fn app_id(&self) -> String {
        self.config_xml.get_package_id()
    }
}
