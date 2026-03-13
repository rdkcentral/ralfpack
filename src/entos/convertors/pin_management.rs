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

use crate::entos::config_xml;
use crate::entos::configs::PinManagementConfiguration;
use crate::entos::convertors::common::*;

impl FromCapabilities for PinManagementConfiguration {
    fn from_capabilities(capabilities: &config_xml::Capabilities) -> Option<Self> {
        if let Some(cap) = capabilities.find("pin-management") {
            if let Some(value) = &cap.value {
                return match value.to_ascii_lowercase().as_str() {
                    "readwrite" | "read-write" => Some(PinManagementConfiguration::ReadWrite),
                    "readonly" | "read-only" => Some(PinManagementConfiguration::ReadOnly),
                    "excluded" => Some(PinManagementConfiguration::Excluded),
                    _ => {
                        log::warn!("Invalid pin-management value: {}", value);
                        None
                    }
                };
            }
        }

        None
    }
}

impl ToCapabilities for PinManagementConfiguration {
    fn to_capabilities(&self) -> Vec<config_xml::Capability> {
        Vec::from([config_xml::Capability {
            name: "pin-management".to_string(),
            value: Some(
                match self {
                    PinManagementConfiguration::ReadWrite => "readwrite",
                    PinManagementConfiguration::ReadOnly => "readonly",
                    PinManagementConfiguration::Excluded => "excluded",
                }
                .to_string(),
            ),
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_serialize_pin_management() {
        let config = PinManagementConfiguration::ReadWrite;
        let json_value = serde_json::to_value(&config).unwrap();
        assert_eq!(json_value, json!("readwrite"));

        let config = PinManagementConfiguration::ReadOnly;
        let json_value = serde_json::to_value(&config).unwrap();
        assert_eq!(json_value, json!("readonly"));

        let config = PinManagementConfiguration::Excluded;
        let json_value = serde_json::to_value(&config).unwrap();
        assert_eq!(json_value, json!("excluded"));
    }

    #[test]
    fn test_deserialize_pin_management() {
        let json_value = json!("readwrite");
        let config: PinManagementConfiguration = serde_json::from_value(json_value).unwrap();
        assert_eq!(config, PinManagementConfiguration::ReadWrite);

        let json_value = json!("readonly");
        let config: PinManagementConfiguration = serde_json::from_value(json_value).unwrap();
        assert_eq!(config, PinManagementConfiguration::ReadOnly);

        let json_value = json!("excLuded");
        let config: PinManagementConfiguration = serde_json::from_value(json_value).unwrap();
        assert_eq!(config, PinManagementConfiguration::Excluded);

        let json_value = json!("invalid");
        let result: Result<PinManagementConfiguration, _> = serde_json::from_value(json_value);
        assert!(result.is_err());
    }

    #[test]
    fn test_pin_management_from_capabilities() {
        let caps = vec![
            config_xml::Capability {
                name: "pin-management".to_string(),
                value: Some("readwrite".to_string()),
            },
            config_xml::Capability {
                name: "some-other-cap".to_string(),
                value: Some("value".to_string()),
            },
        ];
        let capabilities = config_xml::Capabilities { capabilities: caps };

        let config = PinManagementConfiguration::from_capabilities(&capabilities);
        assert_eq!(config, Some(PinManagementConfiguration::ReadWrite));

        let caps = vec![config_xml::Capability {
            name: "pin-management".to_string(),
            value: Some("readonly".to_string()),
        }];
        let capabilities = config_xml::Capabilities { capabilities: caps };

        let config = PinManagementConfiguration::from_capabilities(&capabilities);
        assert_eq!(config, Some(PinManagementConfiguration::ReadOnly));

        let caps = vec![config_xml::Capability {
            name: "pin-management".to_string(),
            value: Some("excluded".to_string()),
        }];
        let capabilities = config_xml::Capabilities { capabilities: caps };

        let config = PinManagementConfiguration::from_capabilities(&capabilities);
        assert_eq!(config, Some(PinManagementConfiguration::Excluded));

        let caps = vec![config_xml::Capability {
            name: "pin-management".to_string(),
            value: Some("invalid".to_string()),
        }];
        let capabilities = config_xml::Capabilities { capabilities: caps };

        let config = PinManagementConfiguration::from_capabilities(&capabilities);
        assert_eq!(config, None);

        let caps = vec![config_xml::Capability {
            name: "some-other-cap".to_string(),
            value: Some("value".to_string()),
        }];
        let capabilities = config_xml::Capabilities { capabilities: caps };

        let config = PinManagementConfiguration::from_capabilities(&capabilities);
        assert_eq!(config, None);
    }

    #[test]
    fn test_pin_management_to_capabilities() {
        let config = PinManagementConfiguration::ReadWrite;
        let caps = config.to_capabilities();
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].name, "pin-management");
        assert_eq!(caps[0].value.as_deref(), Some("readwrite"));

        let config = PinManagementConfiguration::ReadOnly;
        let caps = config.to_capabilities();
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].name, "pin-management");
        assert_eq!(caps[0].value.as_deref(), Some("readonly"));

        let config = PinManagementConfiguration::Excluded;
        let caps = config.to_capabilities();
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].name, "pin-management");
        assert_eq!(caps[0].value.as_deref(), Some("excluded"));
    }
}
