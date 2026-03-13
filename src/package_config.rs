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

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::io::Read;

/// Privileges that an application or service can request.
pub(crate) mod permissions {
    /// The application is requesting to be the home/launcher app. An app
    /// with this privilege will be started when the device boots and will
    /// be the app that is shown when the user presses the home button.
    ///
    /// This is typically combined with the `DisplayOverlays` and `Compositor`
    /// privileges to allow the app to draw the home screen and control
    /// UI on the device.
    pub const HOME_APP_PERMISSION: &str = "urn:rdk:permission:home-app";

    /// The app or service requires access to the network.
    pub const INTERNET_PERMISSION: &str = "urn:rdk:permission:internet";

    /// The app or service requires access to the firebolt API. This is just
    /// used to request access to firebolt in general, additional firebolt
    /// specific privileges may be required to access specific firebolt APIs.
    pub const FIREBOLT_PERMISSION: &str = "urn:rdk:permission:firebolt";

    /// The app or service requires access to the thunder API. This is just
    /// used to request access to thunder in general, additional thunder
    /// specific privileges may be required to access specific thunder APIs.
    pub const THUNDER_PERMISSION: &str = "urn:rdk:permission:thunder";

    /// The app or service requires access to the Rialto API for A/V playback.
    pub const RIALTO_PERMISSION: &str = "urn:rdk:permission:rialto";

    /// The app is requesting access to any connected game controller devices.
    pub const GAME_CONTROLLER_PERMISSION: &str = "urn:rdk:permission:game-controller";

    /// The app or service is requesting access to the time shift buffer.
    pub const TIME_SHIFT_BUFFER_PERMISSION: &str = "urn:rdk:permission:timeshift-buffer";

    /// The app or service is requesting access to be able to read data from
    /// attached external storage devices. External storage devices are
    /// typically USB memory sticks.
    pub const READ_EXTERNAL_STORAGE_PERMISSION: &str = "urn:rdk:permission:external-storage:read";

    /// The app or service is requesting access to be able to read and write data
    /// from/to attached external storage devices. External storage devices
    /// are typically USB memory sticks.
    pub const WRITE_EXTERNAL_STORAGE_PERMISSION: &str = "urn:rdk:permission:external-storage:write";

    /// The app or service is requesting privilege to display overlays on the
    /// screen. Overlays are typically popups or other UI elements that are
    /// drawn on top of the normal application UI.
    pub const OVERLAY_PERMISSION: &str = "urn:rdk:permission:display-overlay";

    /// The application is requesting access to the composition API. This allows
    /// the app to control the layout and composition of the screen for all apps.
    /// In this sense, it acts like a basic window manager and is typically used
    /// in conjunction with the `HomeApp` privilege to allow the app to act
    /// like a launcher or desktop app.
    pub const COMPOSITOR_PERMISSION: &str = "urn:rdk:permission:compositor";
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageType {
    Base,
    Application,
    Service,
    Runtime,
    /// Represents an unknown or unclassified package type.
    /// Good for handling cases where the content type doesn't match known patterns.
    Unknown,
}

impl Display for PackageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            PackageType::Application => "application",
            PackageType::Base => "base",
            PackageType::Service => "service",
            PackageType::Runtime => "runtime",
            PackageType::Unknown => "unknown",
        };
        write!(f, "{}", s)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageSpecifier {
    Html,
    Cobalt,
    Flutter,
    System,
    Luna,
    Unknown,
}

impl Display for PackageSpecifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            PackageSpecifier::Html => "html",
            PackageSpecifier::Cobalt => "cobalt",
            PackageSpecifier::Flutter => "flutter",
            PackageSpecifier::Luna => "luna",
            PackageSpecifier::System => "system",
            PackageSpecifier::Unknown => "unknown",
        };
        write!(f, "{}", s)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationLifecycleConfiguration {
    /// Optional supported non-active lifecycle states for the application
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub supported_non_active_states: Vec<LifecycleState>,

    /// The optional maximum system memory allowed for the app when suspended.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_suspended_system_memory: Option<String>,

    /// The optional maximum time to reduce the system memory usage to its maxSuspendedSystemMemory
    /// after entering suspended state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_time_to_suspend_memory_state: Option<f32>,

    /// Optional start-up timeout value.  If the app or service hasn't notified
    /// the system it is running before the timeout it may be killed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub startup_timeout: Option<f32>,

    /// Optional watchdog timeout value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub watchdog_interval: Option<f32>,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DialConfiguration {
    /// The optional DIAL name(s) to advertise for the app.  If not specified then the appId will
    /// be used as the DIAL name.
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub app_names: HashSet<String>,

    /// Optional list of CORS domains that are allowed to access the DIAL service.
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub cors_domains: HashSet<String>,

    /// Set to true if an origin header is required for all DIAL requests.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub origin_header_required: bool,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LifecycleState {
    Paused,
    Suspended,
    Hibernated,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InputHandlingConfiguration {
    #[serde(skip_serializing_if = "HashSet::is_empty", default)]
    pub key_intercept: HashSet<String>,

    #[serde(skip_serializing_if = "HashSet::is_empty", default)]
    pub key_capture: HashSet<String>,

    #[serde(skip_serializing_if = "HashSet::is_empty", default)]
    pub key_monitor: HashSet<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum NetworkProtocol {
    Tcp,
    Udp,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum NetworkType {
    Public,
    Exported,
    Imported,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkServiceConfiguration {
    /// The optional name of the service, this is intended to be a human-readable name for
    /// the service, however currently it's not populated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// The protocol that the service uses, either TCP or UDP.
    pub protocol: NetworkProtocol,

    /// The port that the service listens on or connects to.
    pub port: u16,

    /// Type of the network service
    #[serde(rename = "type")]
    pub network_type: NetworkType,
}

#[derive(Serialize, Deserialize)]
pub struct MemoryConfiguration {
    /// The amount of memory, in bytes, that the app has requested.
    /// If the app has not requested any memory then this will return None.
    #[serde(rename = "system", skip_serializing_if = "Option::is_none")]
    pub sys_memory: Option<String>,

    /// The amount of GPU memory, in bytes, that the app has requested.
    /// If the app has not requested any memory then this will return None.
    #[serde(rename = "gpu", skip_serializing_if = "Option::is_none")]
    pub gpu_memory: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageConfiguration {
    /// The amount of persistent storage, in bytes, that the app has requested.
    /// If the app has not requested any storage then this will return None.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_local_storage: Option<String>,

    /// The appId (represented as package id) that the storage is associated with.
    /// This is used to identify the app that the storage is associated with.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shared_storage_app_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowConfiguration {
    /// The display size requested by the app, e.g. 1080, 720. Note that this typically only
    /// controls the virtual display resolution for the app, the wayland output display size.
    /// The actual final composition and / or HDMI display size is controlled by the system.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtual_display_size: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Platform {
    pub architecture: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,
}

pub type LogLevelsConfiguration = HashSet<String>;
pub type NetworkConfiguration = Vec<NetworkServiceConfiguration>;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Configuration {
    #[serde(rename = "urn:rdk:config:log-levels", skip_serializing_if = "Option::is_none")]
    pub log_levels: Option<LogLevelsConfiguration>,

    #[serde(
        rename = "urn:rdk:config:application-lifecycle",
        skip_serializing_if = "Option::is_none"
    )]
    pub lifecycle: Option<ApplicationLifecycleConfiguration>,

    #[serde(rename = "urn:rdk:config:dial", skip_serializing_if = "Option::is_none")]
    pub dial: Option<DialConfiguration>,

    #[serde(rename = "urn:rdk:config:platform", skip_serializing_if = "Option::is_none")]
    pub platform: Option<Platform>,

    #[serde(rename = "urn:rdk:config:input-handling", skip_serializing_if = "Option::is_none")]
    pub input_handling: Option<InputHandlingConfiguration>,

    #[serde(
        rename = "urn:rdk:config:network",
        skip_serializing_if = "NetworkConfiguration::is_empty",
        default
    )]
    pub network: NetworkConfiguration,

    #[serde(rename = "urn:rdk:config:memory", skip_serializing_if = "Option::is_none")]
    pub memory: Option<MemoryConfiguration>,

    #[serde(rename = "urn:rdk:config:storage", skip_serializing_if = "Option::is_none")]
    pub storage: Option<StorageConfiguration>,

    #[serde(rename = "urn:rdk:config:window", skip_serializing_if = "Option::is_none")]
    pub window: Option<WindowConfiguration>,

    #[serde(flatten)]
    pub vendor_config: HashMap<String, serde_json::Value>,
}

impl Configuration {
    pub fn new() -> Self {
        Configuration {
            log_levels: None,
            lifecycle: None,
            dial: None,
            platform: None,
            input_handling: None,
            network: Vec::new(),
            memory: None,
            storage: None,
            window: None,
            vendor_config: HashMap::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.log_levels.is_none()
            && self.dial.is_none()
            && self.platform.is_none()
            && self.input_handling.is_none()
            && self.network.is_empty()
            && self.memory.is_none()
            && self.storage.is_none()
            && self.window.is_none()
            && self.vendor_config.is_empty()
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageConfig {
    pub id: String,

    pub version: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    pub package_type: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_specifier: Option<String>,

    pub entry_point: String,

    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub dependencies: HashMap<String, String>,

    #[serde(skip_serializing_if = "HashSet::is_empty", default)]
    pub permissions: HashSet<String>,

    #[serde(skip_serializing_if = "Configuration::is_empty")]
    pub configuration: Configuration,
}

impl PackageConfig {
    /// Helper function to schema check the supplied JSON string against the package config schema.
    pub fn validate_str(config_str: &str) -> bool {
        // Parse the config JSON
        let config = serde_json::from_str(config_str);
        if config.is_err() {
            eprintln!("Package config JSON is not valid");
            return false;
        }

        Self::validate_json(&config.unwrap())
    }

    /// Helper function to schema validate the package config JSON.
    #[allow(dead_code)]
    pub fn validate_reader<R: Read>(config_file: R) -> bool {
        // Parse the config JSON
        let config = serde_json::from_reader(config_file);
        if config.is_err() {
            eprintln!("Package config JSON is not valid");
            return false;
        }

        Self::validate_json(&config.unwrap())
    }

    /// Performs the actual schema validation of the parsed JSON object.
    pub fn validate_json(config_json: &serde_json::Value) -> bool {
        // Load the schema from resources and parse it
        let schema_str = include_str!("schemas/package_config_schema.json");
        let schema_json: serde_json::Value =
            serde_json::from_str(schema_str).expect("JSON schema was not well-formatted");

        // Create the schema validator
        let validator = jsonschema::validator_for(&schema_json).expect("Failed to create schema validator");

        // Stores the first 12 errors for printing
        let mut errors: Vec<jsonschema::ValidationError> = vec![];

        // Validate the config JSON against the schema
        for error in validator.iter_errors(&config_json) {
            if errors.len() < 12 {
                errors.push(error.to_owned());
            }
        }

        if errors.is_empty() {
            true
        } else {
            eprintln!("Package config JSON file failed schema validation");
            for error in &errors {
                eprintln!("---------------");
                eprintln!("Error: {}", error);
                eprintln!("Location: {}", error.instance_path);
            }

            false
        }
    }

    #[allow(unused)]
    pub fn id(&self) -> String {
        self.id.clone()
    }

    #[allow(unused)]
    pub fn version(&self) -> String {
        self.version.clone()
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_basic_schema_validation() {
        let config_json = include_str!("../testdata/package_config.json");

        let result = PackageConfig::validate_str(config_json);

        println!("Result: {}", result);

        assert!(result, "Package config JSON failed schema validation");
    }

    #[test]
    fn test_deserialisation() {
        let config_json = include_str!("../testdata/package_config.json");
        let config: PackageConfig =
            serde_json::from_str(config_json).expect("Failed to read / parse package config file");

        assert_eq!(config.id, "com.sky.myapp");
        assert_eq!(config.version, "1.2.3");
        assert_eq!(config.version_name.unwrap(), "1.2.3-beta");
    }

    #[test]
    fn test_serialisation() {
        let config = PackageConfig {
            id: "com.sky.myapp".to_string(),
            version: "1.2.3".to_string(),
            version_name: Some("1.2.3-beta".to_string()),
            name: Some("My App".to_string()),
            package_type: "application".to_string(),
            package_specifier: Some("html".to_string()),
            entry_point: "web/index.html".to_string(),
            dependencies: HashMap::new(),
            permissions: HashSet::new(),
            configuration: Configuration {
                log_levels: None,
                lifecycle: None,
                dial: None,
                platform: None,
                input_handling: None,
                network: vec![],
                memory: None,
                storage: None,
                window: None,
                vendor_config: HashMap::new(),
            },
        };

        let config_json = serde_json::to_string(&config).expect("Failed to serialise package config");
        let result = PackageConfig::validate_str(&config_json);

        // let config_json_pretty = serde_json::to_string_pretty(&config).expect("Failed to serialize package config");

        assert!(result, "Package config JSON failed schema validation");
    }
}
