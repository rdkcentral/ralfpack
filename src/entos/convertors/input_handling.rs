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
use crate::package_config::InputHandlingConfiguration;
use std::collections::HashSet;

impl FromCapabilities for InputHandlingConfiguration {
    fn from_capabilities(capabilities: &config_xml::Capabilities) -> Option<Self> {
        let mut keys_configuration = InputHandlingConfiguration {
            key_intercept: HashSet::new(),
            key_capture: HashSet::new(),
            key_monitor: HashSet::new(),
        };

        for cap in capabilities.capabilities.iter() {
            match cap.name.as_str() {
                "keymapping" => {
                    if let Some(value) = &cap.value {
                        keys_configuration.key_capture = split_to_set(value, ',');
                    }
                }
                "forward-keymapping" => {
                    if let Some(value) = &cap.value {
                        keys_configuration.key_monitor = split_to_set(value, ',');
                    }
                }
                _ => {}
            }
        }

        if keys_configuration.key_monitor.is_empty()
            && keys_configuration.key_intercept.is_empty()
            && keys_configuration.key_capture.is_empty()
        {
            return None;
        }

        Some(keys_configuration)
    }
}

impl ToCapabilities for InputHandlingConfiguration {
    fn to_capabilities(&self) -> Vec<config_xml::Capability> {
        let mut caps = Vec::new();

        if !self.key_capture.is_empty() {
            caps.push(config_xml::Capability {
                name: "keymapping".to_string(),
                value: Some(set_to_string(&self.key_capture, ',')),
            });
        }

        if !self.key_monitor.is_empty() {
            caps.push(config_xml::Capability {
                name: "forward-keymapping".to_string(),
                value: Some(set_to_string(&self.key_monitor, ',')),
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
    fn test_keys_serialization() {
        let config = InputHandlingConfiguration {
            key_capture: HashSet::from(["play".to_string()]),
            key_monitor: HashSet::from(["volumeUp".to_string()]),
            key_intercept: HashSet::from(["volume+".to_string()]),
        };

        let json_snippet = serde_json::to_value(&config).unwrap();
        let expected = json!({
            "keyCapture": ["play"],
            "keyMonitor": ["volumeUp"],
            "keyIntercept": ["volume+"]
        });
        assert_eq!(json_snippet, expected);
    }

    #[test]
    fn test_keys_deserialization() {
        let json_snippet = json!({
            "keyCapture": ["play", "pause", "stop"],
            "keyMonitor": ["volumeUp", "volumeDown"],
            "keyIntercept": ["volume+", "volume-"]
        });

        let config = serde_json::from_value::<InputHandlingConfiguration>(json_snippet).unwrap();
        assert_eq!(config.key_capture.len(), 3);
        assert!(config.key_capture.contains("play"));
        assert!(config.key_capture.contains("pause"));
        assert!(config.key_capture.contains("stop"));
        assert_eq!(config.key_monitor.len(), 2);
        assert!(config.key_monitor.contains("volumeUp"));
        assert!(config.key_monitor.contains("volumeDown"));
        assert_eq!(config.key_intercept.len(), 2);
        assert!(config.key_intercept.contains("volume+"));
        assert!(config.key_intercept.contains("volume-"));

        let json_snippet = json!({});

        let config = serde_json::from_value::<InputHandlingConfiguration>(json_snippet).unwrap();
        assert_eq!(config.key_capture.len(), 0);
        assert_eq!(config.key_monitor.len(), 0);
        assert_eq!(config.key_intercept.len(), 0);
    }

    #[test]
    fn test_keys_configuration_to_capabilities() {
        let json_snippet = json!({
            "keyCapture": ["play", "pause", "stop", "fastForward", "rewind"],
            "keyMonitor": ["volumeUp", "volumeDown", "mute"],
            "keyIntercept": ["volume+", "volume-"]
        });

        let config = serde_json::from_value::<InputHandlingConfiguration>(json_snippet).unwrap();
        assert_eq!(config.key_capture.len(), 5);
        assert!(config.key_capture.contains("play"));
        assert!(config.key_capture.contains("pause"));
        assert!(config.key_capture.contains("stop"));
        assert!(config.key_capture.contains("fastForward"));
        assert!(config.key_capture.contains("rewind"));
        assert_eq!(config.key_monitor.len(), 3);
        assert!(config.key_monitor.contains("volumeUp"));
        assert!(config.key_monitor.contains("volumeDown"));
        assert!(config.key_monitor.contains("mute"));
        assert_eq!(config.key_intercept.len(), 2);
        assert!(config.key_intercept.contains("volume+"));
        assert!(config.key_intercept.contains("volume-"));

        let capabilities = config.to_capabilities();
        assert_eq!(capabilities.len(), 2);
        let keymapping_cap = capabilities.iter().find(|c| c.name == "keymapping").unwrap();
        let forward_keymapping_cap = capabilities.iter().find(|c| c.name == "forward-keymapping").unwrap();
        let keymapping_values = split_to_set(keymapping_cap.value.as_ref().unwrap(), ',');
        let forward_keymapping_values = split_to_set(forward_keymapping_cap.value.as_ref().unwrap(), ',');

        let expected = HashSet::from([
            "play".to_string(),
            "pause".to_string(),
            "stop".to_string(),
            "fastForward".to_string(),
            "rewind".to_string(),
        ]);
        assert_eq!(keymapping_values, expected);

        let expected = HashSet::from(["volumeUp".to_string(), "volumeDown".to_string(), "mute".to_string()]);
        assert_eq!(forward_keymapping_values, expected);
    }

    #[test]
    fn test_keys_configuration_from_capabilities() {
        let capabilities = config_xml::Capabilities {
            capabilities: vec![
                config_xml::Capability {
                    name: "keymapping".to_string(),
                    value: Some("play,pause,stop,fastForward,rewind".to_string()),
                },
                config_xml::Capability {
                    name: "forward-keymapping".to_string(),
                    value: Some("volumeUp,volumeDown,mute".to_string()),
                },
                config_xml::Capability {
                    name: "unrelated-capability".to_string(),
                    value: Some("ignored".to_string()),
                },
            ],
        };

        let config = InputHandlingConfiguration::from_capabilities(&capabilities).unwrap();
        assert_eq!(config.key_capture.len(), 5);
        assert!(config.key_capture.contains("play"));
        assert!(config.key_capture.contains("pause"));
        assert!(config.key_capture.contains("stop"));
        assert!(config.key_capture.contains("fastForward"));
        assert!(config.key_capture.contains("rewind"));
        assert_eq!(config.key_monitor.len(), 3);
        assert!(config.key_monitor.contains("volumeUp"));
        assert!(config.key_monitor.contains("volumeDown"));
        assert!(config.key_monitor.contains("mute"));
        assert_eq!(config.key_intercept.len(), 0);
    }
}
