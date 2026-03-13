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
use crate::entos::convertors::common::*;

use crate::package_config::WindowConfiguration;

impl FromCapabilities for WindowConfiguration {
    fn from_capabilities(capabilities: &config_xml::Capabilities) -> Option<Self> {
        if let Some(capability) = capabilities.find("virtual-resolution") {
            if let Some(value) = &capability.value {
                if let Ok(parsed_value) = value.trim().parse::<u32>() {
                    if parsed_value < 480 || parsed_value > 2160 {
                        log::warn!("Invalid virtual-resolution value: {}", value);
                    } else {
                        return Some(WindowConfiguration {
                            virtual_display_size: Some(parsed_value),
                        });
                    }
                } else {
                    log::warn!("Invalid virtual-resolution value: {}", value);
                }
            }
        }

        None
    }
}

impl ToCapabilities for WindowConfiguration {
    fn to_capabilities(&self) -> Vec<config_xml::Capability> {
        let mut caps = Vec::new();

        if let Some(virtual_display_size) = self.virtual_display_size {
            caps.push(config_xml::Capability {
                name: "virtual-resolution".to_string(),
                value: Some(virtual_display_size.to_string()),
            });
        }

        caps
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_window_configuration_serialization() {
        let config = WindowConfiguration {
            virtual_display_size: Some(1080),
        };
        let serialized = serde_json::to_string(&config).unwrap();
        let expected = json!({
            "virtualDisplaySize": 1080
        });
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&serialized).unwrap(),
            expected
        );
    }

    #[test]
    fn test_window_configuration_deserialization() {
        let data = json!({
            "virtualDisplaySize": 720
        });
        let deserialized: WindowConfiguration = serde_json::from_value(data).unwrap();
        assert_eq!(deserialized.virtual_display_size, Some(720));

        let data = json!({});
        let deserialized: WindowConfiguration = serde_json::from_value(data).unwrap();
        assert!(deserialized.virtual_display_size.is_none());
    }

    #[test]
    fn test_from_capabilities() {
        // Test with valid virtual-resolution
        let capabilities = config_xml::Capabilities {
            capabilities: vec![config_xml::Capability {
                name: "virtual-resolution".to_string(),
                value: Some("1080".to_string()),
            }],
        };
        let window_config = WindowConfiguration::from_capabilities(&capabilities).unwrap();
        assert_eq!(window_config.virtual_display_size, Some(1080));

        // Test with invalid virtual-resolution (too low)
        let capabilities = config_xml::Capabilities {
            capabilities: vec![config_xml::Capability {
                name: "virtual-resolution".to_string(),
                value: Some("100".to_string()),
            }],
        };
        let window_config = WindowConfiguration::from_capabilities(&capabilities);
        assert!(window_config.is_none());

        // Test with invalid virtual-resolution (too high)
        let capabilities = config_xml::Capabilities {
            capabilities: vec![config_xml::Capability {
                name: "virtual-resolution".to_string(),
                value: Some("3000".to_string()),
            }],
        };
        let window_config = WindowConfiguration::from_capabilities(&capabilities);
        assert!(window_config.is_none());
    }

    #[test]
    fn test_to_capabilities() {
        let window_config = WindowConfiguration {
            virtual_display_size: Some(720),
        };
        let capabilities = window_config.to_capabilities();
        assert_eq!(capabilities.len(), 1);
        assert_eq!(capabilities[0].name, "virtual-resolution");
        assert_eq!(capabilities[0].value.as_deref(), Some("720"));
    }
}
