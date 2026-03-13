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
use crate::package_config::*;
use entos::convertors::common::{FromCapabilities, ToCapabilities};

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::convert::Into;

/// Some fixed runtime package IDs that are used to generate the dependencies for the package
pub(crate) mod runtime_package_ids {
    /// The base layer package ID, this is the package that provides the base functionality for
    /// all applications and services.
    #[allow(dead_code)]
    pub const BASE_LAYER: &str = "com.sky.baselayer";

    /// The Cobalt runtime package ID, this is the package that provides the Cobalt runtime
    /// for applications.
    pub const COBALT_RUNTIME: &str = "com.sky.cobalt";

    /// The RDK Browser runtime package ID, this is the package that provides the RDK Browser
    /// runtime for web applications.
    pub const RDK_BROWSER_RUNTIME: &str = "com.sky.rdkbrowser";

    /// The Luna runtime package ID, Luna is deprecated but may still exist in some older catalogues.
    pub const LUNA_RUNTIME: &str = "com.sky.luna";
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename = "widget")]
pub struct ConfigXml {
    /// If namespace is present it will be "http://www.bskyb.com/ns/widgets"
    #[allow(dead_code)]
    #[serde(rename = "@xmlns")]
    pub xmlns: Option<String>,

    /// The version of the config.xml schema, this can be "1.0" or "2.0", but version 1.0 has
    /// been deprecated for many years now.
    #[allow(dead_code)]
    #[serde(rename = "@version")]
    schema_version: String,

    /// The name element contains the short, version, and long names of the package.  The short
    /// name is the unique appID and is used as the package ID.
    name: Name,

    /// Icon element, it is optional but pretty much every widget has an icon.  If present this
    /// will be used as the package icon.
    #[serde(skip_serializing_if = "Option::is_none")]
    icon: Option<Icon>,

    /// Content element which contains the entry point for the package, and can contain platform
    /// filter values.  These are deprecated in the OCI format, but retained for backwards
    /// compatibility.
    content: Content,

    #[serde(default = "empty_capabilities")]
    capabilities: Capabilities,

    /// Parental control flag has long since been deprecated, however for backwards compatibility
    /// we still parse it and add as a vendor config if present.
    #[serde(rename = "parentalControl", skip_serializing_if = "Option::is_none")]
    parental_control: Option<bool>,
}

/// Returns an empty capabilities struct, used as a default value for the capabilities field in
/// ConfigXml.
fn empty_capabilities() -> Capabilities {
    Capabilities {
        capabilities: Vec::new(),
    }
}

/// Convert the parsed config.xml into a package config struct.  This is where all logic sits for
/// how to convert existing config.xml capabilities into the new package config format.
impl From<&ConfigXml> for PackageConfig {
    fn from(config_xml: &ConfigXml) -> Self {
        let pkg_config = PackageConfig {
            id: config_xml.name.short.clone(),
            version: config_xml.get_package_version(),
            version_name: Some(config_xml.name.version.clone()),
            name: config_xml.name.long.clone(),
            entry_point: config_xml.content.src.clone(),
            package_type: config_xml.get_package_type().to_string(),
            package_specifier: Some(config_xml.get_package_specifier().to_string()),
            dependencies: config_xml.get_package_dependencies(),
            permissions: config_xml.capabilities.to_permissions(),
            configuration: config_xml.get_configuration(),
        };

        pkg_config
    }
}

impl ConfigXml {
    /// Returns the appId from the config.xml which is used as the package ID.
    #[allow(dead_code)]
    pub fn get_package_id(&self) -> String {
        self.name.short.clone()
    }

    /// Returns the type of the package this is based on the content.src field in the
    /// config.xml and the capabilities field.
    fn get_package_type(&self) -> PackageType {
        let content_type = &self.content.content_type;

        if content_type.starts_with("runtime/") {
            PackageType::Runtime
        } else if content_type.starts_with("application/") {
            if self.capabilities.has_capability("system-app") || self.capabilities.has_capability("daemon-app") {
                PackageType::Service
            } else {
                PackageType::Application
            }
        } else {
            PackageType::Unknown
        }
    }

    fn get_package_specifier(&self) -> PackageSpecifier {
        let content_type = &self.content.content_type;

        if let Some(subtype_str) = content_type.split('/').nth(1) {
            match subtype_str {
                "html" => PackageSpecifier::Html,
                "cobalt" => PackageSpecifier::Cobalt,
                "flutter" => PackageSpecifier::Flutter,
                "luna" => PackageSpecifier::Luna,
                "system" => PackageSpecifier::System,
                _ => PackageSpecifier::Unknown,
            }
        } else {
            PackageSpecifier::Unknown
        }
    }

    /// Gets the dependency id for the package based on the content.type field in the
    /// config.xml.  This is a hardcoded lookup from "application/XXXX" to the current
    /// runtime package ids.
    fn get_package_dependencies(&self) -> HashMap<String, String> {
        let mut deps = HashMap::new();
        match self.content.content_type.as_str() {
            "application/html" => {
                deps.insert(runtime_package_ids::RDK_BROWSER_RUNTIME.to_owned(), "*".to_owned());
            }
            "application/cobalt" => {
                deps.insert(runtime_package_ids::COBALT_RUNTIME.to_owned(), "*".to_owned());
            }
            "application/luna" => {
                deps.insert(runtime_package_ids::LUNA_RUNTIME.to_owned(), "*".to_owned());
            }
            "application/system" => {
                // Disabled base-layer dependency when converting widgets as it's likely existing
                // widgets don't yet work with a base-layer.
                // deps.insert(runtime_package_ids::BASE_LAYER.to_owned(), "*".to_owned());
            }
            "runtime/html" | "runtime/cobalt" | "runtime/luna" | "runtime/flutter" => {
                // Disabled base-layer dependency when converting widgets as it's likely existing
                // runtime widgets don't yet work with a base-layer.
                // deps.insert(runtime_package_ids::BASE_LAYER.to_owned(), "*".to_owned());
            }
            _ => {
                log::warn!("Unknown package type from config.xml, defaulting to 'application/system'");
            }
        }

        deps
    }

    /// Tries to determine the semantic version of the package from the config.xml.
    /// config.xml files don't use semantic versioning, instead their version field is free-form,
    /// so we guess what the semver should be.
    ///
    /// The way we guess it split the string on '.' and then for each part try to read a number,
    /// if we find a part that is not a number then skip and move to the next part.
    pub fn get_package_version(&self) -> String {
        let parts: Vec<&str> = self.name.version.split('.').collect();
        let mut semver_parts = Vec::new();

        for part in parts {
            // Find the last character that is a digit and trim the string to that point
            let mut trimmed_part = part.trim_start();
            let mut last_digit_index = -1;
            for (i, c) in trimmed_part.chars().enumerate() {
                if c.is_numeric() {
                    last_digit_index = i as i32;
                } else {
                    break;
                }
            }

            // If the part contained no digits, skip it
            if last_digit_index == -1 {
                continue;
            }

            trimmed_part = &trimmed_part[..=(last_digit_index as usize)];
            if let Ok(num) = trimmed_part.parse::<u32>() {
                semver_parts.push(num.to_string());
            } else {
                // Not a number, skip this part
                continue;
            }
        }

        // If we have no parts then return a default version
        if semver_parts.is_empty() {
            return "0.0.0".to_string();
        }
        // If we have more than 4 parts then truncate to 4
        if semver_parts.len() > 4 {
            semver_parts.truncate(4);
        }

        // Join the parts together with '.' and return
        let ver = semver_parts.join(".");
        ver.to_string()
    }

    /// Converts the <capabilities>, <platformFilters> and <icon>  of the config.xml into a configuration struct that can be used
    fn get_configuration(&self) -> Configuration {
        // Convert the capabilities into configurations
        let mut configs = self.capabilities.to_configuration();

        // Added the deprecated platform filters as a vendor config, unfortunately these are still
        // needed for backwards compatibility
        if let Some(platform_filters) = &self.content.platform_filters {
            if let Some(platform_filter_config) = platform_filters.to_vendor_config() {
                configs.vendor_config.insert(
                    entos::configs::ENTOS_PLATFORM_FILTERS_CONFIGURATION.to_owned(),
                    platform_filter_config,
                );
            }
        }

        // Add the deprecated parental control flag if it is set
        if let Some(parental_control) = self.parental_control {
            configs.vendor_config.insert(
                entos::configs::ENTOS_PARENTAL_CONTROL_CONFIGURATION.to_owned(),
                serde_json::Value::Bool(parental_control),
            );
        }

        // Add the icon if specified, icon is not part of the OCI meta-data spec, but it's added
        // as a vendor config.
        if let Some(icon) = &self.icon {
            let icon_config = serde_json::json!([
                {
                    "src": icon.src,
                    "type": icon.icon_type,
                }
            ]);
            configs
                .vendor_config
                .insert(entos::configs::ENTOS_ICONS_CONFIGURATION.to_owned(), icon_config);
        }

        configs
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Name {
    #[serde(rename = "@short")]
    short: String,
    #[serde(rename = "@version")]
    version: String,
    #[serde(rename = "$text")]
    long: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
struct Icon {
    #[serde(rename = "@src")]
    src: String,
    #[serde(rename = "@type", skip_serializing_if = "Option::is_none")]
    icon_type: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Content {
    #[serde(rename = "@src")]
    src: String,

    #[serde(rename = "@type")]
    content_type: String,

    #[allow(dead_code)]
    #[serde(rename = "@platform", skip_serializing_if = "Option::is_none")]
    platform: Option<String>,

    #[serde(rename = "platformFilters")]
    platform_filters: Option<PlatformFilters>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
struct PlatformFilters {
    #[serde(rename = "platformId", skip_serializing_if = "Vec::is_empty", default)]
    platform_ids: Vec<PlatformFilter>,

    #[serde(rename = "platformVariant", skip_serializing_if = "Vec::is_empty", default)]
    platform_variants: Vec<PlatformFilter>,

    #[serde(rename = "proposition", skip_serializing_if = "Vec::is_empty", default)]
    propositions: Vec<PlatformFilter>,

    #[serde(rename = "country", skip_serializing_if = "Vec::is_empty", default)]
    countries: Vec<PlatformFilter>,

    #[serde(rename = "region", skip_serializing_if = "Vec::is_empty", default)]
    regions: Vec<PlatformFilter>,

    #[serde(rename = "subdivision", skip_serializing_if = "Vec::is_empty", default)]
    subdivisions: Vec<PlatformFilter>,

    #[serde(rename = "yoctoVersion", skip_serializing_if = "Vec::is_empty", default)]
    yocto_versions: Vec<PlatformFilter>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
struct PlatformFilter {
    #[serde(rename = "@name")]
    name: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Capability {
    #[serde(rename = "@name")]
    pub name: String,

    #[serde(rename = "$text", default, deserialize_with = "process_capability_value")]
    pub value: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Capabilities {
    #[serde(default, rename = "capability")]
    pub capabilities: Vec<Capability>,
}

/// Function to process / deserialise the value stored in the capability element.  It trims any
/// leading and trailing whitespace from the value and returns it as an `Option<String>`. If there
/// was no value, or it was empty, it returns `None`.
fn process_capability_value<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value: Option<String> = Deserialize::deserialize(deserializer)?;
    if let Some(value) = value {
        let trimmed_value = value.trim();
        if !trimmed_value.is_empty() {
            return Ok(Some(trimmed_value.to_owned()));
        }
    }

    Ok(None)
}

impl Capabilities {
    fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.iter().any(|cap| cap.name == capability)
    }

    pub fn find(&self, name: &str) -> Option<&Capability> {
        self.capabilities.iter().find(|cap| cap.name == name)
    }

    #[allow(dead_code)]
    fn contains(&self, capability: &Capability) -> bool {
        self.capabilities
            .iter()
            .any(|cap| cap.name == capability.name && cap.value == capability.value)
    }

    /// Converts some of the capabilities to a list of privileges allowed for the app.
    fn to_permissions(&self) -> HashSet<String> {
        let mut permission_set: HashSet<String> = HashSet::new();

        for cap in self.capabilities.iter() {
            match cap.name.as_str() {
                "home-app" => {
                    permission_set.insert(permissions::HOME_APP_PERMISSION.to_owned());
                    permission_set.insert(permissions::COMPOSITOR_PERMISSION.to_owned());
                }
                "wan-lan" | "lan-wan" => {
                    permission_set.insert(permissions::INTERNET_PERMISSION.to_owned());
                }
                "local-services-1" => {
                    permission_set.insert(entos::permissions::ENTOS_AS_ACCESS_LEVEL1_PERMISSION.to_owned());
                }
                "local-services-2" => {
                    permission_set.insert(entos::permissions::ENTOS_AS_ACCESS_LEVEL2_PERMISSION.to_owned());
                }
                "local-services-3" => {
                    permission_set.insert(entos::permissions::ENTOS_AS_ACCESS_LEVEL3_PERMISSION.to_owned());
                }
                "local-services-4" => {
                    permission_set.insert(entos::permissions::ENTOS_AS_ACCESS_LEVEL4_PERMISSION.to_owned());
                }
                "local-services-5" => {
                    permission_set.insert(entos::permissions::ENTOS_AS_ACCESS_LEVEL5_PERMISSION.to_owned());
                }
                "post-intents" => {
                    permission_set.insert(entos::permissions::ENTOS_AS_POST_INTENT_PERMISSION.to_owned());
                }
                "firebolt" => {
                    permission_set.insert(permissions::FIREBOLT_PERMISSION.to_owned());
                }
                "thunder" => {
                    permission_set.insert(permissions::THUNDER_PERMISSION.to_owned());
                }
                "rialto" => {
                    permission_set.insert(permissions::RIALTO_PERMISSION.to_owned());
                }
                "game-controller" => {
                    permission_set.insert(permissions::GAME_CONTROLLER_PERMISSION.to_owned());
                }
                "tsb-storage" => {
                    permission_set.insert(permissions::TIME_SHIFT_BUFFER_PERMISSION.to_owned());
                }
                "read-external-storage" => {
                    permission_set.insert(permissions::READ_EXTERNAL_STORAGE_PERMISSION.to_owned());
                }
                "write-external-storage" => {
                    permission_set.insert(permissions::WRITE_EXTERNAL_STORAGE_PERMISSION.to_owned());
                }
                "issue-notifications" => {
                    permission_set.insert(permissions::OVERLAY_PERMISSION.to_owned());
                }
                "compositor-app" => {
                    permission_set.insert(permissions::COMPOSITOR_PERMISSION.to_owned());
                }
                "as-player" => {
                    permission_set.insert(entos::permissions::ENTOS_AS_PLAYER_PERMISSION.to_owned());
                }
                "airplay2" => {
                    permission_set.insert(entos::permissions::ENTOS_AIRPLAY_PERMISSION.to_owned());
                }
                "stb-entitlements" => {
                    permission_set.insert(entos::permissions::ENTOS_ENTITLEMENT_INFO_PERMISSION.to_owned());
                }
                "chromecast" => {
                    permission_set.insert(entos::permissions::ENTOS_CHROMECAST_PERMISSION.to_owned());
                }
                "memory-intensive" => {
                    permission_set.insert(entos::permissions::ENTOS_MEMORY_INTENSIVE_PERMISSION.to_owned());
                }
                "bearer-token-authentication" => {
                    permission_set.insert(entos::permissions::ENTOS_BEARER_TOKEN_AUTHENTICATION_PERMISSION.to_owned());
                }
                "https-mutual-authentication" => {
                    permission_set.insert(entos::permissions::ENTOS_HTTPS_MTLS_AUTHENTICATION_PERMISSION.to_owned());
                }
                _ => {}
            }
        }

        permission_set
    }

    /// Converts some of the capabilities to a list of settings allowed for the app.
    fn to_configuration(&self) -> Configuration {
        let mut configuration = Configuration::new();

        // This configuration is missing in widgets, hardcode it then.
        configuration.platform = Some(Platform {
            architecture: "arm".to_string(),
            variant: Some("v7".to_string()),
            os: None,
        });

        configuration.log_levels = LogLevelsConfiguration::from_capabilities(self);
        configuration.lifecycle = ApplicationLifecycleConfiguration::from_capabilities(self);
        configuration.dial = DialConfiguration::from_capabilities(self);
        configuration.input_handling = InputHandlingConfiguration::from_capabilities(self);
        configuration.network = NetworkConfiguration::from_capabilities(self).unwrap_or_default();
        configuration.memory = MemoryConfiguration::from_capabilities(self);
        configuration.storage = StorageConfiguration::from_capabilities(self);
        configuration.window = WindowConfiguration::from_capabilities(self);

        if let Some(legacy_drm_config) = entos::configs::LegacyDrmConfiguration::from_capabilities(self) {
            if let Ok(value) = serde_json::to_value(legacy_drm_config) {
                configuration
                    .vendor_config
                    .insert(entos::configs::ENTOS_LEGACY_DRM_CONFIGURATION.to_owned(), value);
            }
        }
        if let Some(pin_mgmt_config) = entos::configs::PinManagementConfiguration::from_capabilities(self) {
            if let Ok(value) = serde_json::to_value(pin_mgmt_config) {
                configuration
                    .vendor_config
                    .insert(entos::configs::ENTOS_PIN_MANAGEMENT_CONFIGURATION.to_owned(), value);
            }
        }
        if let Some(pre_launch_config) = entos::configs::PreLaunchConfiguration::from_capabilities(self) {
            if let Ok(value) = serde_json::to_value(pre_launch_config) {
                configuration
                    .vendor_config
                    .insert(entos::configs::ENTOS_PRELAUNCH_CONFIGURATION.to_owned(), value);
            }
        }
        if let Some(display_config) = entos::configs::DisplayConfiguration::from_capabilities(self) {
            if let Ok(value) = serde_json::to_value(display_config) {
                configuration
                    .vendor_config
                    .insert(entos::configs::ENTOS_DISPLAY_CONFIGURATION.to_owned(), value);
            }
        }
        if let Some(audio_config) = entos::configs::AudioConfiguration::from_capabilities(self) {
            if let Ok(value) = serde_json::to_value(audio_config) {
                configuration
                    .vendor_config
                    .insert(entos::configs::ENTOS_AUDIO_CONFIGURATION.to_owned(), value);
            }
        }
        if let Some(multicast_config) = entos::configs::MulticastConfiguration::from_capabilities(self) {
            if let Ok(value) = serde_json::to_value(multicast_config) {
                configuration
                    .vendor_config
                    .insert(entos::configs::ENTOS_MULTICAST_CONFIGURATION.to_owned(), value);
            }
        }
        if let Some(fkps_config) = entos::configs::FkpsConfiguration::from_capabilities(self) {
            if let Ok(value) = serde_json::to_value(fkps_config) {
                configuration
                    .vendor_config
                    .insert(entos::configs::ENTOS_FKPS_CONFIGURATION.to_owned(), value);
            }
        }
        if let Some(mediarite_config) = entos::configs::MediariteConfiguration::from_capabilities(self) {
            if let Ok(value) = serde_json::to_value(mediarite_config) {
                configuration
                    .vendor_config
                    .insert(entos::configs::ENTOS_MEDIARITE_CONFIGURATION.to_owned(), value);
            }
        }
        if let Some(intercept_config) = entos::configs::MarketplaceInterceptConfiguration::from_capabilities(self) {
            if let Ok(value) = serde_json::to_value(intercept_config) {
                configuration.vendor_config.insert(
                    entos::configs::ENTOS_MARKETPLACE_INTERCEPT_CONFIGURATION.to_owned(),
                    value,
                );
            }
        }
        if let Some(partner_id_config) = entos::configs::PartnerIdConfiguration::from_capabilities(self) {
            if let Ok(value) = serde_json::to_value(partner_id_config) {
                configuration
                    .vendor_config
                    .insert(entos::configs::ENTOS_CONTENT_PARTNER_ID_CONFIGURATION.to_owned(), value);
            }
        }
        if let Some(catalogue_id) = entos::configs::CatalogueIdConfiguration::from_capabilities(self) {
            if let Ok(value) = serde_json::to_value(catalogue_id) {
                configuration
                    .vendor_config
                    .insert(entos::configs::ENTOS_CATALOGUE_ID_CONFIGURATION.to_owned(), value);
            }
        }
        if let Some(age_policy) = entos::configs::AgePolicyConfiguration::from_capabilities(self) {
            if let Ok(value) = serde_json::to_value(age_policy) {
                configuration
                    .vendor_config
                    .insert(entos::configs::ENTOS_AGE_POLICY_CONFIGURATION.to_owned(), value);
            }
        }
        if let Some(age_rating) = entos::configs::AgeRatingConfiguration::from_capabilities(self) {
            if let Ok(value) = serde_json::to_value(age_rating) {
                configuration
                    .vendor_config
                    .insert(entos::configs::ENTOS_AGE_RATING_CONFIGURATION.to_owned(), value);
            }
        }
        if let Some(age_rating) = entos::configs::LowPowerTerminateConfiguration::from_capabilities(self) {
            if let Ok(value) = serde_json::to_value(age_rating) {
                configuration.vendor_config.insert(
                    entos::configs::ENTOS_LOW_POWER_TERMINATE_CONFIGURATION.to_owned(),
                    value,
                );
            }
        }

        configuration
    }

    /// Parses a string that is expected to contain one or more numbers, optionally with a memory
    /// multiplier suffix (k, m, g).
    #[allow(unused)]
    fn parse_mem_value(value: &Option<String>) -> Option<u64> {
        if let Some(v) = value {
            if v.is_empty() {
                return None;
            }

            let trimmed = v.trim_ascii();
            let multiplier: u64 = match trimmed.chars().last() {
                Some('k') | Some('K') => 1024,
                Some('m') | Some('M') => 1024 * 1024,
                Some('g') | Some('G') => 1024 * 1024 * 1024,
                _ => 1,
            };

            if multiplier != 1 {
                let t = &trimmed[..trimmed.len() - 1];
                return t.parse::<u64>().ok();
            }

            let bytes = trimmed.parse::<u64>().ok()?;
            return Some(bytes * multiplier);
        }

        None
    }
}

impl PlatformFilters {
    /// Converts the platform filters into a vendor config that can be used in the package config.
    /// This is a workaround for the fact that the platform filters are deprecated in the OCI format,
    /// but we still need to support them for backwards compatibility.
    fn to_vendor_config(&self) -> Option<serde_json::Value> {
        // If no platform filters specified, return None
        if self.platform_ids.is_empty()
            && self.platform_variants.is_empty()
            && self.propositions.is_empty()
            && self.regions.is_empty()
            && self.countries.is_empty()
            && self.subdivisions.is_empty()
            && self.yocto_versions.is_empty()
        {
            return None;
        }

        // Otherwise convert the platform filters into a vendor config / json object
        let mut vendor_config = serde_json::Map::new();

        if !self.platform_ids.is_empty() {
            let ids: Vec<String> = self.platform_ids.iter().map(|p| p.name.clone()).collect();
            vendor_config.insert(
                "platformIds".to_string(),
                serde_json::Value::Array(ids.into_iter().map(serde_json::Value::String).collect()),
            );
        }

        if !self.platform_variants.is_empty() {
            let variants: Vec<String> = self.platform_variants.iter().map(|p| p.name.clone()).collect();
            vendor_config.insert(
                "platformVariants".to_string(),
                serde_json::Value::Array(variants.into_iter().map(serde_json::Value::String).collect()),
            );
        }

        if !self.propositions.is_empty() {
            let propositions: Vec<String> = self.propositions.iter().map(|p| p.name.clone()).collect();
            vendor_config.insert(
                "propositions".to_string(),
                serde_json::Value::Array(propositions.into_iter().map(serde_json::Value::String).collect()),
            );
        }

        let mut countries_: Vec<String> = Vec::new();
        if !self.regions.is_empty() {
            let mut regions: Vec<String> = self.regions.iter().map(|p| p.name.clone()).collect();
            countries_.append(&mut regions);
        }
        if !self.countries.is_empty() {
            let mut countries: Vec<String> = self.countries.iter().map(|c| c.name.clone()).collect();
            countries_.append(&mut countries);
        }

        if !countries_.is_empty() {
            vendor_config.insert(
                "countries".to_string(),
                serde_json::Value::Array(countries_.into_iter().map(serde_json::Value::String).collect()),
            );
        }

        if !self.subdivisions.is_empty() {
            let subdivisions: Vec<String> = self.subdivisions.iter().map(|s| s.name.clone()).collect();
            vendor_config.insert(
                "subdivisions".to_string(),
                serde_json::Value::Array(subdivisions.into_iter().map(serde_json::Value::String).collect()),
            );
        }

        if !self.yocto_versions.is_empty() {
            let subdivisions: Vec<String> = self.yocto_versions.iter().map(|s| s.name.clone()).collect();
            vendor_config.insert(
                "yoctoVersions".to_string(),
                serde_json::Value::Array(subdivisions.into_iter().map(serde_json::Value::String).collect()),
            );
        }

        Some(vendor_config.into())
    }

    fn convert_json_array_to_filter_items(value: &serde_json::Value) -> Vec<PlatformFilter> {
        let mut items = Vec::new();
        if let serde_json::Value::Array(arr) = value {
            for item in arr.iter() {
                if let serde_json::Value::String(s) = item {
                    items.push(PlatformFilter { name: s.clone() });
                }
            }
        }
        items
    }

    /// Converts a vendor config JSON object into a PlatformFilters struct.
    fn from_vendor_config(value: &serde_json::Value) -> Self {
        let mut filters = PlatformFilters {
            platform_ids: Vec::new(),
            platform_variants: Vec::new(),
            propositions: Vec::new(),
            regions: Vec::new(),
            countries: Vec::new(),
            subdivisions: Vec::new(),
            yocto_versions: Vec::new(),
        };

        if let serde_json::Value::Object(map) = value {
            if let Some(platform_ids) = map.get("platformIds") {
                filters.platform_ids = Self::convert_json_array_to_filter_items(platform_ids);
            }
            if let Some(platform_variants) = map.get("platformVariants") {
                filters.platform_variants = Self::convert_json_array_to_filter_items(platform_variants);
            }
            if let Some(propositions) = map.get("propositions") {
                filters.propositions = Self::convert_json_array_to_filter_items(propositions);
            }
            if let Some(countries) = map.get("countries") {
                filters.countries = Self::convert_json_array_to_filter_items(countries);
            }
            if let Some(subdivisions) = map.get("subdivisions") {
                filters.subdivisions = Self::convert_json_array_to_filter_items(subdivisions);
            }
            if let Some(yocto_versions) = map.get("yoctoVersions") {
                filters.yocto_versions = Self::convert_json_array_to_filter_items(yocto_versions);
            }
        }

        filters
    }
}

/// Perform the reverse conversion from a PackageConfig struct back into a ConfigXml struct.  This
/// is not a perfect conversion as some information is lost when converting from config.xml and
/// this is just best effort and focuses on the main capabilities and fields.
impl From<&PackageConfig> for ConfigXml {
    fn from(pkg_config: &PackageConfig) -> Self {
        ConfigXml {
            xmlns: Some("http://www.bskyb.com/ns/widgets".to_string()),
            schema_version: "2.0".to_string(),
            name: Name {
                short: pkg_config.id.clone(),
                long: pkg_config.name.clone(),
                version: pkg_config.version_name.clone().unwrap_or(pkg_config.version.clone()),
            },
            icon: get_icon_config(&pkg_config.configuration),
            content: Content {
                src: pkg_config.entry_point.clone(),
                content_type: get_content_type(&pkg_config),
                platform: None,
                platform_filters: get_platform_filters(&pkg_config.configuration),
            },
            capabilities: get_capabilities(&pkg_config),
            parental_control: get_parental_control(&pkg_config.configuration),
        }
    }
}

/// Helper function to get the content type for the config.xml based on the package type and specifier.
fn get_content_type(pkg_config: &PackageConfig) -> String {
    let package_type = match pkg_config.package_type.to_lowercase().as_str() {
        "application" => "application",
        "service" => "application",
        "runtime" => "runtime",
        _ => "application",
    };

    let package_specifier = pkg_config
        .package_specifier
        .as_deref()
        .unwrap_or("unknown")
        .to_lowercase();
    format!("{}/{}", package_type, package_specifier)
}

/// Helper function to extract the platform filters from the package configuration if they exist.
fn get_platform_filters(config: &Configuration) -> Option<PlatformFilters> {
    if let Some(vendor_config) = config
        .vendor_config
        .get(entos::configs::ENTOS_PLATFORM_FILTERS_CONFIGURATION)
    {
        Some(PlatformFilters::from_vendor_config(vendor_config))
    } else {
        None
    }
}

/// Helper function to extract the icon configuration from the package configuration if it exists.
/// In the JSON format the icons are stored as an array, but in the config.xml we only support a
/// single icon, so we just take the first icon in the array if it exists.
fn get_icon_config(config: &Configuration) -> Option<Icon> {
    if let Some(vendor_config) = config.vendor_config.get(entos::configs::ENTOS_ICONS_CONFIGURATION) {
        if let Ok(mut icons_config) =
            serde_json::from_value::<entos::configs::IconsConfiguration>(vendor_config.clone())
        {
            if !icons_config.is_empty() {
                let first_icon = icons_config.remove(0);
                return Some(Icon {
                    src: first_icon.src,
                    icon_type: first_icon.icon_type,
                });
            } else {
                log::warn!("No icons found in icon configuration");
            }
        } else {
            log::warn!("Failed to parse icon configuration from package config");
        }
    }
    None
}

/// Helper function to extract the parental control setting from the package configuration if it exists.
fn get_parental_control(config: &Configuration) -> Option<bool> {
    if let Some(vendor_config) = config
        .vendor_config
        .get(entos::configs::ENTOS_PARENTAL_CONTROL_CONFIGURATION)
    {
        if let serde_json::Value::Bool(enabled) = vendor_config {
            return Some(*enabled);
        }
    }

    None
}

/// Big helper function to extract the capabilities from the package configuration and convert
/// them into a list of capabilities that can be stored in the config.xml file.
fn get_capabilities(pkg_config: &PackageConfig) -> Capabilities {
    let mut capabilities = Vec::new();

    // Add daemon-app and system-app capabilities based on the package type
    if pkg_config.package_type.to_lowercase() == "service" {
        capabilities.push(Capability {
            name: "daemon-app".to_string(),
            value: None,
        });
        capabilities.push(Capability {
            name: "system-app".to_string(),
            value: None,
        });
    }

    // Create a map of known boolean permissions to capabilities
    let mut pmap = HashMap::new();
    pmap.insert(permissions::INTERNET_PERMISSION, "wan-lan");
    pmap.insert(permissions::INTERNET_PERMISSION, "wan-lan");
    pmap.insert(permissions::HOME_APP_PERMISSION, "home-app");
    pmap.insert(permissions::FIREBOLT_PERMISSION, "firebolt");
    pmap.insert(permissions::THUNDER_PERMISSION, "thunder");
    pmap.insert(permissions::RIALTO_PERMISSION, "rialto");
    pmap.insert(permissions::GAME_CONTROLLER_PERMISSION, "game-controller");
    pmap.insert(permissions::READ_EXTERNAL_STORAGE_PERMISSION, "read-external-storage");
    pmap.insert(permissions::WRITE_EXTERNAL_STORAGE_PERMISSION, "write-external-storage");
    pmap.insert(permissions::OVERLAY_PERMISSION, "issue-notifications");
    pmap.insert(permissions::COMPOSITOR_PERMISSION, "compositor-app");
    pmap.insert(permissions::TIME_SHIFT_BUFFER_PERMISSION, "tsb-storage");
    pmap.insert(
        entos::permissions::ENTOS_AS_ACCESS_LEVEL1_PERMISSION,
        "local-services-1",
    );
    pmap.insert(
        entos::permissions::ENTOS_AS_ACCESS_LEVEL2_PERMISSION,
        "local-services-2",
    );
    pmap.insert(
        entos::permissions::ENTOS_AS_ACCESS_LEVEL3_PERMISSION,
        "local-services-3",
    );
    pmap.insert(
        entos::permissions::ENTOS_AS_ACCESS_LEVEL4_PERMISSION,
        "local-services-4",
    );
    pmap.insert(
        entos::permissions::ENTOS_AS_ACCESS_LEVEL5_PERMISSION,
        "local-services-5",
    );
    pmap.insert(entos::permissions::ENTOS_AS_POST_INTENT_PERMISSION, "post-intents");
    pmap.insert(entos::permissions::ENTOS_AS_PLAYER_PERMISSION, "as-player");
    pmap.insert(entos::permissions::ENTOS_AIRPLAY_PERMISSION, "airplay2");
    pmap.insert(entos::permissions::ENTOS_CHROMECAST_PERMISSION, "chromecast");
    pmap.insert(
        entos::permissions::ENTOS_BEARER_TOKEN_AUTHENTICATION_PERMISSION,
        "bearer-token-authentication",
    );
    pmap.insert(
        entos::permissions::ENTOS_HTTPS_MTLS_AUTHENTICATION_PERMISSION,
        "https-mutual-authentication",
    );
    pmap.insert(
        entos::permissions::ENTOS_ENTITLEMENT_INFO_PERMISSION,
        "stb-entitlements",
    );
    pmap.insert(
        entos::permissions::ENTOS_MEMORY_INTENSIVE_PERMISSION,
        "memory-intensive",
    );

    // Add the easy one to one mapping of permissions to capabilities
    for permission in &pkg_config.permissions {
        if let Some(cap_name) = pmap.get(permission.as_str()) {
            capabilities.push(Capability {
                name: cap_name.to_string(),
                value: None,
            });
        }
    }

    // Now handle the more complex configs that require values or special handling
    if let Some(log_levels) = &pkg_config.configuration.log_levels {
        capabilities.append(&mut log_levels.to_capabilities());
    }
    if let Some(lifecycle) = &pkg_config.configuration.lifecycle {
        capabilities.append(&mut lifecycle.to_capabilities());
    }
    if let Some(dial) = &pkg_config.configuration.dial {
        capabilities.append(&mut dial.to_capabilities());
    }
    if let Some(keys) = &pkg_config.configuration.input_handling {
        capabilities.append(&mut keys.to_capabilities());
    }
    if !pkg_config.configuration.network.is_empty() {
        capabilities.append(&mut pkg_config.configuration.network.to_capabilities());
    }
    if let Some(memory) = &pkg_config.configuration.memory {
        capabilities.append(&mut memory.to_capabilities());
    }
    if let Some(storage) = &pkg_config.configuration.storage {
        capabilities.append(&mut storage.to_capabilities());
    }
    if let Some(window) = &pkg_config.configuration.window {
        capabilities.append(&mut window.to_capabilities());
    }

    // And the EntOS vendor configurations that can be represented as capabilities
    for (key, value) in &pkg_config.configuration.vendor_config {
        match key.as_str() {
            entos::configs::ENTOS_MEDIARITE_CONFIGURATION => {
                if let Ok(mediarite_config) =
                    serde_json::from_value::<entos::configs::MediariteConfiguration>(value.clone())
                {
                    capabilities.append(&mut mediarite_config.to_capabilities());
                }
            }
            entos::configs::ENTOS_PIN_MANAGEMENT_CONFIGURATION => {
                if let Ok(pin_mgmt) =
                    serde_json::from_value::<entos::configs::PinManagementConfiguration>(value.clone())
                {
                    capabilities.append(&mut pin_mgmt.to_capabilities());
                }
            }
            entos::configs::ENTOS_PRELAUNCH_CONFIGURATION => {
                if let Ok(pre_launch) = serde_json::from_value::<entos::configs::PreLaunchConfiguration>(value.clone())
                {
                    capabilities.append(&mut pre_launch.to_capabilities());
                }
            }
            entos::configs::ENTOS_AGE_RATING_CONFIGURATION => {
                if let Ok(age_rating) = serde_json::from_value::<entos::configs::AgeRatingConfiguration>(value.clone())
                {
                    capabilities.append(&mut age_rating.to_capabilities());
                }
            }
            entos::configs::ENTOS_AGE_POLICY_CONFIGURATION => {
                if let Ok(age_policy) = serde_json::from_value::<entos::configs::AgePolicyConfiguration>(value.clone())
                {
                    capabilities.append(&mut age_policy.to_capabilities());
                }
            }
            entos::configs::ENTOS_CONTENT_PARTNER_ID_CONFIGURATION => {
                if let Ok(partner_id) = serde_json::from_value::<entos::configs::PartnerIdConfiguration>(value.clone())
                {
                    capabilities.append(&mut partner_id.to_capabilities());
                }
            }
            entos::configs::ENTOS_CATALOGUE_ID_CONFIGURATION => {
                if let Ok(catalog_id) =
                    serde_json::from_value::<entos::configs::CatalogueIdConfiguration>(value.clone())
                {
                    capabilities.append(&mut catalog_id.to_capabilities());
                }
            }
            entos::configs::ENTOS_FKPS_CONFIGURATION => {
                if let Ok(fkps) = serde_json::from_value::<entos::configs::FkpsConfiguration>(value.clone()) {
                    capabilities.append(&mut fkps.to_capabilities());
                }
            }
            entos::configs::ENTOS_MULTICAST_CONFIGURATION => {
                if let Ok(multicast_config) =
                    serde_json::from_value::<entos::configs::MulticastConfiguration>(value.clone())
                {
                    capabilities.append(&mut multicast_config.to_capabilities());
                }
            }
            entos::configs::ENTOS_LEGACY_DRM_CONFIGURATION => {
                if let Ok(legacy_drm) = serde_json::from_value::<entos::configs::LegacyDrmConfiguration>(value.clone())
                {
                    capabilities.append(&mut legacy_drm.to_capabilities());
                }
            }
            entos::configs::ENTOS_DISPLAY_CONFIGURATION => {
                if let Ok(display) = serde_json::from_value::<entos::configs::DisplayConfiguration>(value.clone()) {
                    capabilities.append(&mut display.to_capabilities());
                }
            }
            entos::configs::ENTOS_AUDIO_CONFIGURATION => {
                if let Ok(audio) = serde_json::from_value::<entos::configs::AudioConfiguration>(value.clone()) {
                    capabilities.append(&mut audio.to_capabilities());
                }
            }
            entos::configs::ENTOS_LOW_POWER_TERMINATE_CONFIGURATION => {
                if let Ok(low_power) =
                    serde_json::from_value::<entos::configs::LowPowerTerminateConfiguration>(value.clone())
                {
                    capabilities.append(&mut low_power.to_capabilities());
                }
            }
            entos::configs::ENTOS_MARKETPLACE_INTERCEPT_CONFIGURATION => {
                if let Ok(intercept) =
                    serde_json::from_value::<entos::configs::MarketplaceInterceptConfiguration>(value.clone())
                {
                    capabilities.append(&mut intercept.to_capabilities());
                }
            }
            _ => {
                log::info!("Unknown vendor config key: {}", key);
            }
        }
    }

    Capabilities { capabilities }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    use serde_json::json;

    /// Basic test for parsing a fully featured config.xml file.
    #[test]
    fn test_basic_config_xml_parsing() {
        let config_xml_str = include_str!("../../testdata/all_config.xml");
        let config_xml: ConfigXml = quick_xml::de::from_str(config_xml_str).unwrap();

        assert_eq!(config_xml.schema_version, "2.0");
        assert_eq!(config_xml.name.short, "com.example.app");
        assert_eq!(config_xml.name.long, Some("ExampleApp".to_string()));
        assert_eq!(config_xml.name.version, "1.2.3-beta");
        assert_eq!(
            config_xml.icon,
            Some(Icon {
                src: "icon.png".to_string(),
                icon_type: Some("image/png".to_string()),
            })
        );
        assert_eq!(config_xml.content.src, "run-me.bin");
        assert_eq!(config_xml.content.content_type, "application/system");

        assert!(config_xml.capabilities.contains(&Capability {
            name: "home-app".to_string(),
            value: None,
        }));
        assert!(config_xml.capabilities.contains(&Capability {
            name: "wan-lan".to_string(),
            value: None,
        }));
        assert!(config_xml.capabilities.contains(&Capability {
            name: "local-services-1".to_string(),
            value: None,
        }));
        assert!(config_xml.capabilities.contains(&Capability {
            name: "firebolt".to_string(),
            value: None,
        }));
        assert!(config_xml.capabilities.contains(&Capability {
            name: "private-storage".to_string(),
            value: Some("255".to_string()),
        }));
        assert!(config_xml.capabilities.contains(&Capability {
            name: "sound-scene".to_string(),
            value: Some("SportsFootball, SportsGolf, SportsCricket".to_string()),
        }));
    }

    /// Basic test to test the conversion of the config.xml into a package config.
    #[test]
    fn test_conversion() {
        let config_xml_str = include_str!("../../testdata/all_config.xml");
        let config_xml: ConfigXml = quick_xml::de::from_str(config_xml_str).unwrap();
        let config_json: PackageConfig = (&config_xml).into();

        assert_eq!(config_json.id, "com.example.app");
        assert_eq!(config_json.version, "1.2.3");
        assert_eq!(config_json.version_name, Some("1.2.3-beta".to_string()));
        assert_eq!(config_json.name, Some("ExampleApp".to_string()));
        assert_eq!(config_json.entry_point, "run-me.bin");
        assert_eq!(config_json.package_type, "application");
        assert_eq!(config_json.package_specifier, Some("system".to_string()));
        //assert_eq!(
        //    config_json.dependencies,
        //    HashMap::from([(runtime_package_ids::BASE_LAYER.to_owned(), "*".to_owned())])
        //);
        assert_eq!(config_json.dependencies, HashMap::new());

        assert!(config_json.permissions.contains(permissions::HOME_APP_PERMISSION));

        let configuration = config_json.configuration;
        assert_eq!(
            configuration.dial,
            Some(DialConfiguration {
                app_names: HashSet::from([
                    "uk.co.bbc.iPlayer".to_owned(),
                    "uk.co.bbc.Sport".to_owned(),
                    "uk.co.bbc.News".to_owned()
                ]),
                cors_domains: HashSet::from([
                    "https://www.youtube.com".to_owned(),
                    "https://*.youtube.com".to_owned(),
                    "package:*".to_owned()
                ]),
                origin_header_required: true,
            })
        );
        assert_eq!(
            configuration.input_handling,
            Some(InputHandlingConfiguration {
                key_monitor: HashSet::from(["volume+".to_owned(), "volume-".to_owned(), "mute".to_owned()]),
                key_capture: HashSet::from(["search".to_owned(), "voice".to_owned(), "YouTube".to_owned()]),
                key_intercept: HashSet::new(),
            })
        );
        assert_eq!(
            configuration.log_levels,
            Some(LogLevelsConfiguration::from([
                "default".to_owned(),
                "milestone".to_owned(),
                "fatal".to_owned(),
            ]))
        );

        assert_eq!(
            configuration.platform,
            Some(Platform {
                architecture: "arm".to_string(),
                variant: Some("v7".to_string()),
                os: None,
            })
        );
    }

    /// Basic test to test the conversion of the config.xml into a package config for a webapp.
    #[test]
    fn test_webapp_conversion() {
        let config_xml_str = include_str!("../../testdata/webapp_config.xml");
        let config_xml: ConfigXml = quick_xml::de::from_str(config_xml_str).unwrap();
        let config_json: PackageConfig = (&config_xml).into();

        assert_eq!(config_json.id, "com.flying.monkeys");
        assert_eq!(config_json.version, "2.14.1.2100");
        assert_eq!(config_json.version_name, Some("2.14.1.2100".to_string()));
        assert_eq!(
            config_json.dependencies,
            HashMap::from([(runtime_package_ids::RDK_BROWSER_RUNTIME.to_owned(), "*".to_owned())])
        );
    }

    /// Basic test to test the conversion of the config.xml from a runtime package.
    #[test]
    fn test_runtime_conversion() {
        let config_xml_str = include_str!("../../testdata/runtime_config.xml");
        let config_xml: ConfigXml = quick_xml::de::from_str(config_xml_str).unwrap();
        let config_json: PackageConfig = (&config_xml).into();

        assert_eq!(config_json.id, "com.sky.rdkbrowser");
        assert_eq!(config_json.version, "99.99.99");
        assert_eq!(config_json.version_name, Some("99.99.99".to_string()));
        // assert_eq!(
        //    config_json.dependencies,
        //    HashMap::from([(runtime_package_ids::BASE_LAYER.to_owned(), "*".to_owned())])
        // );
        assert_eq!(config_json.dependencies, HashMap::new());
    }

    /// Basic test to check the conversion of the platform filters into vendor config.
    #[test]
    fn test_platform_filter_conversion() {
        let config_xml_str = include_str!("../../testdata/all_config.xml");
        let config_xml: ConfigXml = quick_xml::de::from_str(config_xml_str).expect("Failed to parse XML");
        let config_json: PackageConfig = (&config_xml).into();

        let filters = config_json
            .configuration
            .vendor_config
            .get(entos::configs::ENTOS_PLATFORM_FILTERS_CONFIGURATION);
        assert!(filters.is_some(), "Platform filters vendor config not found");

        let filters = filters.unwrap();
        assert_eq!(
            filters,
            &json!({
                "platformIds": [ "32B1", "32B2" ],
                "platformVariants": [ "DTH" ],
                "propositions": [ "SKYQ" ],
                "countries": [ "DEU", "AUT", "GBR" ],
                "subdivisions": [ "GBR-SCT" ],
                "yoctoVersions": [ "KIRKSTONE" ]
            })
        );

        // need to normalise the "region" and "country" fields into just "countries" and sort for comparison
        let mut config_filters = config_xml.content.platform_filters.unwrap();
        config_filters.countries.append(&mut config_filters.regions);
        config_filters.countries.sort();

        let mut converted = PlatformFilters::from_vendor_config(filters);
        converted.countries.sort();

        assert_eq!(config_filters, converted);
    }

    #[test]
    fn test_intercept_capability_conversion() {
        // Test intercept capability with value "true"
        let config_xml_str = r#"<?xml version="1.0" encoding="UTF-8"?>
        <widget xmlns="http://www.bskyb.com/ns/widgets" version="2.0">
            <name short="com.test.app" version="1.0.0">TestApp</name>
            <content src="test.bin" type="application/system"/>
            <capabilities>
                <capability name="intercept">tRue</capability>
            </capabilities>
        </widget>"#;

        let config_xml: ConfigXml = quick_xml::de::from_str(config_xml_str).unwrap();
        let config_json: PackageConfig = (&config_xml).into();

        assert!(
            config_json
                .configuration
                .vendor_config
                .contains_key("urn:entos:config:marketplace-intercept")
        );
        let intercept_config = &config_json.configuration.vendor_config["urn:entos:config:marketplace-intercept"];
        assert_eq!(intercept_config, &json!({"enable": true}));

        // Test intercept capability with value "false"
        let config_xml_str_false = r#"<?xml version="1.0" encoding="UTF-8"?>
        <widget xmlns="http://www.bskyb.com/ns/widgets" version="2.0">
            <name short="com.test.app" version="1.0.0">TestApp</name>
            <content src="test.bin" type="application/system"/>
            <capabilities>
                <capability name="intercept">FalsE</capability>
            </capabilities>
        </widget>"#;

        let config_xml_false: ConfigXml = quick_xml::de::from_str(config_xml_str_false).unwrap();
        let config_json_false: PackageConfig = (&config_xml_false).into();

        assert!(
            config_json_false
                .configuration
                .vendor_config
                .contains_key("urn:entos:config:marketplace-intercept")
        );
        let intercept_config_false =
            &config_json_false.configuration.vendor_config["urn:entos:config:marketplace-intercept"];
        assert_eq!(intercept_config_false, &json!({"enable": false}));

        // Test intercept capability without value (should log error and not add config)
        let config_xml_str_no_value = r#"<?xml version="1.0" encoding="UTF-8"?>
        <widget xmlns="http://www.bskyb.com/ns/widgets" version="2.0">
            <name short="com.test.app" version="1.0.0">TestApp</name>
            <content src="test.bin" type="application/system"/>
            <capabilities>
                <capability name="intercept"/>
            </capabilities>
        </widget>"#;

        let config_xml_no_value: ConfigXml = quick_xml::de::from_str(config_xml_str_no_value).unwrap();
        let config_json_no_value: PackageConfig = (&config_xml_no_value).into();

        // Should NOT contain the intercept config when no value is provided
        assert!(
            !config_json_no_value
                .configuration
                .vendor_config
                .contains_key("urn:entos:config:marketplace-intercept")
        );

        // Test intercept capability with invalid value (should log error and not add config)
        let config_xml_str_invalid = r#"<?xml version="1.0" encoding="UTF-8"?>
        <widget xmlns="http://www.bskyb.com/ns/widgets" version="2.0">
            <name short="com.test.app" version="1.0.0">TestApp</name>
            <content src="test.bin" type="application/system"/>
            <capabilities>
                <capability name="intercept">maybe</capability>
            </capabilities>
        </widget>"#;

        let config_xml_invalid: ConfigXml = quick_xml::de::from_str(config_xml_str_invalid).unwrap();
        let config_json_invalid: PackageConfig = (&config_xml_invalid).into();

        // Should NOT contain the intercept config when invalid value is provided
        assert!(
            !config_json_invalid
                .configuration
                .vendor_config
                .contains_key("urn:entos:config:marketplace-intercept")
        );

        // Test no intercept capability at all (should be fine, no config added)
        let config_xml_str_none = r#"<?xml version="1.0" encoding="UTF-8"?>
        <widget xmlns="http://www.bskyb.com/ns/widgets" version="2.0">
            <name short="com.test.app" version="1.0.0">TestApp</name>
            <content src="test.bin" type="application/system"/>
            <capabilities>
                <capability name="wan-lan"/>
            </capabilities>
        </widget>"#;

        let config_xml_none: ConfigXml = quick_xml::de::from_str(config_xml_str_none).unwrap();
        let config_json_none: PackageConfig = (&config_xml_none).into();

        // Should NOT contain the intercept config when no intercept capability is present
        assert!(
            !config_json_none
                .configuration
                .vendor_config
                .contains_key("urn:entos:config:marketplace-intercept")
        );
    }
}
