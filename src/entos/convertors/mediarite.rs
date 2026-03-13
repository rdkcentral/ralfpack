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
use crate::entos::configs::MediariteConfiguration;
use crate::entos::convertors::common::*;
use std::collections::HashMap;

impl FromCapabilities for MediariteConfiguration {
    fn from_capabilities(capabilities: &config_xml::Capabilities) -> Option<Self> {
        let mut config = MediariteConfiguration {
            underlay: None,
            access_groups: HashMap::new(),
        };

        for cap in capabilities.capabilities.iter() {
            if cap.name == "mediarite-underlay" {
                config.underlay = Some(true);
            } else if cap.name == "mapi" {
                if let Some(value) = &cap.value {
                    let groups = split_to_vec(value, ';');
                    for group in groups {
                        let parts = split_to_vec(&group, ':');
                        if parts.len() != 2 {
                            log::warn!("Invalid mediarite mapi group: {}", group);
                            continue;
                        }

                        let name = parts[0].trim();
                        let access_str = parts[1].trim();
                        if name.is_empty() || access_str.is_empty() {
                            log::warn!("Invalid mediarite mapi group: {}", group);
                            continue;
                        }

                        let access_names = split_to_vec(access_str, ',');
                        if !access_names.is_empty() {
                            config.access_groups.insert(name.to_string(), access_names);
                        }
                    }
                }
            }
        }

        if config.underlay.is_none() && config.access_groups.is_empty() {
            None
        } else {
            Some(config)
        }
    }
}

impl ToCapabilities for MediariteConfiguration {
    /// Converts the storage setting to a set of capability
    fn to_capabilities(&self) -> Vec<config_xml::Capability> {
        let mut caps = Vec::new();

        if let Some(true) = self.underlay {
            caps.push(config_xml::Capability {
                name: "mediarite-underlay".to_string(),
                value: None,
            });
        }

        if !self.access_groups.is_empty() {
            let mut groups: Vec<String> = Vec::new();
            for (name, access_names) in &self.access_groups {
                if !access_names.is_empty() {
                    let access_str = access_names.join(",");
                    groups.push(format!("{}:{}", name, access_str));
                }
            }

            if !groups.is_empty() {
                let value = groups.join(";");
                caps.push(config_xml::Capability {
                    name: "mapi".to_string(),
                    value: Some(value),
                });
            }
        }

        caps
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_mediarite_config_serialization() {
        let config = MediariteConfiguration {
            underlay: Some(true),
            access_groups: {
                let mut map = HashMap::new();
                map.insert("group1".to_string(), vec!["read".to_string(), "write".to_string()]);
                map.insert("group2".to_string(), vec!["read".to_string()]);
                map
            },
        };
        let serialized = serde_json::to_string(&config).unwrap();
        let expected = json!({
            "underlay": true,
            "accessGroups": {
                "group1": ["read", "write"],
                "group2": ["read"]
            }
        });
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&serialized).unwrap(),
            expected
        );
    }

    #[test]
    fn test_mediarite_config_deserialization() {
        let data = json!({
            "underlay": true,
            "accessGroups": {
                "group1": ["read", "write"],
                "group2": ["read"]
            }
        });
        let deserialized: MediariteConfiguration = serde_json::from_value(data).unwrap();
        assert_eq!(deserialized.underlay, Some(true));
        assert_eq!(deserialized.access_groups.len(), 2);
        assert_eq!(
            deserialized.access_groups.get("group1").unwrap(),
            &vec!["read".to_string(), "write".to_string()]
        );
        assert_eq!(
            deserialized.access_groups.get("group2").unwrap(),
            &vec!["read".to_string()]
        );

        let data = json!({
            "underlay": true
        });
        let deserialized: MediariteConfiguration = serde_json::from_value(data).unwrap();
        assert_eq!(deserialized.underlay, Some(true));
        assert_eq!(deserialized.access_groups.len(), 0);

        let data = json!({
            "accessGroups": {
                "group1": ["read", "write"],
                "group2": ["read"]
            }
        });
        let deserialized: MediariteConfiguration = serde_json::from_value(data).unwrap();
        assert_eq!(deserialized.underlay, None);
        assert_eq!(deserialized.access_groups.len(), 2);
        assert_eq!(
            deserialized.access_groups.get("group1").unwrap(),
            &vec!["read".to_string(), "write".to_string()]
        );
        assert_eq!(
            deserialized.access_groups.get("group2").unwrap(),
            &vec!["read".to_string()]
        );

        let data = json!({});
        let deserialized: MediariteConfiguration = serde_json::from_value(data).unwrap();
        assert!(deserialized.underlay.is_none());
        assert_eq!(deserialized.access_groups.len(), 0);
    }

    #[test]
    fn test_mediarite_config_from_capabilities() {
        let caps = config_xml::Capabilities {
            capabilities: vec![
                config_xml::Capability {
                    name: "mediarite-underlay".to_string(),
                    value: None,
                },
                config_xml::Capability {
                    name: "mapi".to_string(),
                    value: Some("Main:trusted;Foo:default,core".to_string()),
                },
            ],
        };

        let config = MediariteConfiguration::from_capabilities(&caps).unwrap();
        assert_eq!(config.underlay, Some(true));
        assert_eq!(config.access_groups.len(), 2);
        assert_eq!(
            config.access_groups.get("Foo").unwrap(),
            &vec!["default".to_string(), "core".to_string()]
        );
        assert_eq!(config.access_groups.get("Main").unwrap(), &vec!["trusted".to_string()]);
    }

    #[test]
    fn test_mediarite_config_to_capabilities() {
        let json_snippet = json!({
            "underlay": true,
            "accessGroups": {
                "group1": ["read", "write"],
                "group2": ["read"]
            }
        });

        let config = serde_json::from_value::<MediariteConfiguration>(json_snippet).unwrap();
        assert_eq!(config.underlay, Some(true));
        assert_eq!(config.access_groups.len(), 2);
        assert_eq!(
            config.access_groups.get("group1").unwrap(),
            &vec!["read".to_string(), "write".to_string()]
        );
        assert_eq!(config.access_groups.get("group2").unwrap(), &vec!["read".to_string()]);

        let caps = config.to_capabilities();

        assert_eq!(caps.len(), 2);

        let underlay_cap = caps.iter().find(|c| c.name == "mediarite-underlay").unwrap();
        assert_eq!(underlay_cap.name, "mediarite-underlay");
        assert!(underlay_cap.value.is_none());

        let mapi_cap = caps.iter().find(|c| c.name == "mapi").unwrap();
        assert_eq!(mapi_cap.name, "mapi");

        // The order of groups in the value string is not guaranteed (and not important),
        // so we check both possibilities
        let expected_values = vec!["group1:read,write;group2:read", "group2:read;group1:read,write"];
        assert!(expected_values.contains(&mapi_cap.value.as_ref().unwrap().as_str()));
    }
}
