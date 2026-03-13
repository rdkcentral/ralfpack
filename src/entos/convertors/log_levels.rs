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

use crate::package_config::LogLevelsConfiguration;

impl FromCapabilities for LogLevelsConfiguration {
    fn from_capabilities(capabilities: &config_xml::Capabilities) -> Option<Self> {
        static ALLOWED_LOG_LEVELS: [&str; 7] = ["default", "debug", "info", "milestone", "warning", "error", "fatal"];

        for cap in capabilities.capabilities.iter() {
            match cap.name.as_str() {
                "log-levels" => {
                    if let Some(value) = &cap.value {
                        let levels = split_to_set(value, ',');
                        let mut log_levels_set = LogLevelsConfiguration::new();
                        for level in levels.iter() {
                            let lowered_level = level.to_lowercase();
                            if !ALLOWED_LOG_LEVELS.contains(&lowered_level.as_str()) {
                                log::warn!("Invalid log level found in config.xml: {}", level);
                                return None;
                            }

                            log_levels_set.insert(lowered_level);
                        }

                        return Some(log_levels_set);
                    }
                }
                _ => {}
            }
        }

        None
    }
}

impl ToCapabilities for LogLevelsConfiguration {
    fn to_capabilities(&self) -> Vec<config_xml::Capability> {
        let mut caps = Vec::new();

        let value = self.iter().cloned().collect::<Vec<String>>().join(",");
        caps.push(config_xml::Capability {
            name: "log-levels".to_string(),
            value: Some(value),
        });

        caps
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashSet;

    #[test]
    fn test_log_levels_configuration_to_capabilities() {
        let json_snippet = json!(["debug", "info", "warning", "error"]);

        let config = serde_json::from_value::<LogLevelsConfiguration>(json_snippet).unwrap();
        assert_eq!(config.len(), 4);
        assert!(config.contains("debug"));
        assert!(config.contains("info"));
        assert!(config.contains("warning"));
        assert!(config.contains("error"));

        let capabilities = config.to_capabilities();
        assert_eq!(capabilities.len(), 1);
        let loglevels_cap = &capabilities[0];
        assert_eq!(loglevels_cap.name, "log-levels");
        let loglevels_values = split_to_set(loglevels_cap.value.as_ref().unwrap(), ',');

        let expected = HashSet::from([
            "debug".to_string(),
            "info".to_string(),
            "warning".to_string(),
            "error".to_string(),
        ]);
        assert_eq!(loglevels_values, expected);
    }

    #[test]
    fn test_log_levels_configuration_from_capabilities() {
        let capabilities = config_xml::Capabilities {
            capabilities: vec![
                config_xml::Capability {
                    name: "log-levels".to_string(),
                    value: Some("debug, info,\nwarning,   milestone".to_string()),
                },
                config_xml::Capability {
                    name: "unrelated-capability".to_string(),
                    value: Some("ignored".to_string()),
                },
            ],
        };

        let config = LogLevelsConfiguration::from_capabilities(&capabilities).unwrap();
        assert_eq!(config.len(), 4);
        assert!(config.contains("debug"));
        assert!(config.contains("info"));
        assert!(config.contains("warning"));
        assert!(config.contains("milestone"));
    }
}
