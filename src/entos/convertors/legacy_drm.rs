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
use crate::entos::configs::LegacyDrmConfiguration;
use crate::entos::convertors::common::*;

impl FromCapabilities for LegacyDrmConfiguration {
    fn from_capabilities(capabilities: &config_xml::Capabilities) -> Option<Self> {
        let mut config = LegacyDrmConfiguration {
            types: Vec::new(),
            storage_size_kb: None,
        };

        if let Some(cap) = capabilities.find("drm-type") {
            if let Some(value) = &cap.value {
                config.types = split_to_vec(value, ',');
            }
        }

        if let Some(cap) = capabilities.find("drm-store") {
            if let Some(value) = &cap.value {
                if let Ok(size) = value.parse::<u32>() {
                    config.storage_size_kb = Some(size);
                } else {
                    log::warn!("Invalid drm-store value: {}", value);
                }
            }
        }

        if config.types.is_empty() && config.storage_size_kb.is_none() {
            None
        } else {
            Some(config)
        }
    }
}

impl ToCapabilities for LegacyDrmConfiguration {
    fn to_capabilities(&self) -> Vec<config_xml::Capability> {
        let mut caps = Vec::new();

        // Weirdly some old widget have a 'drm-type' capability with an empty value, so to match
        // their capabilities we also add the 'drm-type' empty capability
        if self.types.is_empty() {
            caps.push(config_xml::Capability {
                name: "drm-type".to_string(),
                value: None,
            });
        } else {
            caps.push(config_xml::Capability {
                name: "drm-type".to_string(),
                value: Some(vec_to_string(&self.types, ',')),
            });
        }

        if let Some(size) = self.storage_size_kb {
            caps.push(config_xml::Capability {
                name: "drm-store".to_string(),
                value: Some(size.to_string()),
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
    fn test_legacy_drm_configuration_serialization() {
        let config = LegacyDrmConfiguration {
            types: vec!["type1".to_string(), "type2".to_string()],
            storage_size_kb: Some(2048),
        };
        let serialized = serde_json::to_string(&config).unwrap();
        let expected = json!({
            "types": ["type1", "type2"],
            "storageSizeKB": 2048
        });
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&serialized).unwrap(),
            expected
        );

        let config = LegacyDrmConfiguration {
            types: vec![],
            storage_size_kb: None,
        };
        let serialized = serde_json::to_string(&config).unwrap();
        let expected = json!({
            "types": []
        });
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&serialized).unwrap(),
            expected
        );
    }

    #[test]
    fn test_legacy_drm_configuration_deserialization() {
        let data = json!({
            "types": ["typeA", "typeB"],
            "storageSizeKB": 4096
        });
        let deserialized: LegacyDrmConfiguration = serde_json::from_value(data).unwrap();
        assert_eq!(deserialized.types, vec!["typeA".to_string(), "typeB".to_string()]);
        assert_eq!(deserialized.storage_size_kb, Some(4096));

        let data = json!({
            "storageSizeKB": 4096
        });
        let deserialized: LegacyDrmConfiguration = serde_json::from_value(data).unwrap();
        assert_eq!(deserialized.types.len(), 0);
        assert_eq!(deserialized.storage_size_kb, Some(4096));

        let data = json!({
            "types": ["typeA", "typeB"],
        });
        let deserialized: LegacyDrmConfiguration = serde_json::from_value(data).unwrap();
        assert_eq!(deserialized.types, vec!["typeA".to_string(), "typeB".to_string()]);
        assert!(deserialized.storage_size_kb.is_none());

        let data = json!({});
        let deserialized: LegacyDrmConfiguration = serde_json::from_value(data).unwrap();
        assert!(deserialized.types.is_empty());
        assert!(deserialized.storage_size_kb.is_none());
    }

    #[test]
    fn test_legacy_drm_from_capabilities() {
        // Test with both drm-type and drm-store
        let capabilities = config_xml::Capabilities {
            capabilities: vec![
                config_xml::Capability {
                    name: "drm-type".to_string(),
                    value: Some("type1, type2".to_string()),
                },
                config_xml::Capability {
                    name: "drm-store".to_string(),
                    value: Some("2048".to_string()),
                },
            ],
        };
        let drm_config = LegacyDrmConfiguration::from_capabilities(&capabilities).unwrap();
        assert_eq!(drm_config.types, vec!["type1".to_string(), "type2".to_string()]);
        assert_eq!(drm_config.storage_size_kb, Some(2048));

        // Test with only drm-type
        let capabilities = config_xml::Capabilities {
            capabilities: vec![config_xml::Capability {
                name: "drm-type".to_string(),
                value: Some("typeA, typeB".to_string()),
            }],
        };
        let drm_config = LegacyDrmConfiguration::from_capabilities(&capabilities).unwrap();
        assert_eq!(drm_config.types, vec!["typeA".to_string(), "typeB".to_string()]);
        assert!(drm_config.storage_size_kb.is_none());

        // Test with only drm-store
        let capabilities = config_xml::Capabilities {
            capabilities: vec![config_xml::Capability {
                name: "drm-store".to_string(),
                value: Some("4096".to_string()),
            }],
        };
        let drm_config = LegacyDrmConfiguration::from_capabilities(&capabilities).unwrap();
        assert!(drm_config.types.is_empty());
        assert_eq!(drm_config.storage_size_kb, Some(4096));

        // Test with invalid drm-store
        let capabilities = config_xml::Capabilities {
            capabilities: vec![config_xml::Capability {
                name: "drm-store".to_string(),
                value: Some("invalid".to_string()),
            }],
        };
        let drm_config = LegacyDrmConfiguration::from_capabilities(&capabilities);
        assert!(drm_config.is_none());

        // Test with no relevant capabilities
        let capabilities = config_xml::Capabilities {
            capabilities: vec![config_xml::Capability {
                name: "unrelated-capability".to_string(),
                value: Some("ignored".to_string()),
            }],
        };
        let drm_config = LegacyDrmConfiguration::from_capabilities(&capabilities);
        assert!(drm_config.is_none());
    }

    #[test]
    fn test_legacy_drm_to_capabilities() {
        let config = LegacyDrmConfiguration {
            types: vec!["type1".to_string(), "type2".to_string()],
            storage_size_kb: Some(2048),
        };
        let capabilities = config.to_capabilities();
        assert_eq!(capabilities.len(), 2);
        let drm_type_cap = capabilities.iter().find(|c| c.name == "drm-type").unwrap();
        assert_eq!(drm_type_cap.value.as_ref().unwrap(), "type1,type2");
        let drm_store_cap = capabilities.iter().find(|c| c.name == "drm-store").unwrap();
        assert_eq!(drm_store_cap.value.as_ref().unwrap(), "2048");

        let config = LegacyDrmConfiguration {
            types: vec![],
            storage_size_kb: Some(4096),
        };
        let capabilities = config.to_capabilities();
        assert_eq!(capabilities.len(), 2);
        assert_eq!(capabilities[0].name, "drm-type");
        assert_eq!(capabilities[0].value, None);
        assert_eq!(capabilities[1].name, "drm-store");
        assert_eq!(capabilities[1].value.as_ref().unwrap(), "4096");

        let config = LegacyDrmConfiguration {
            types: vec!["typeA".to_string()],
            storage_size_kb: None,
        };
        let capabilities = config.to_capabilities();
        assert_eq!(capabilities.len(), 1);
        let drm_type_cap = &capabilities[0];
        assert_eq!(drm_type_cap.name, "drm-type");
        assert_eq!(drm_type_cap.value.as_ref().unwrap(), "typeA");

        let config = LegacyDrmConfiguration {
            types: vec![],
            storage_size_kb: None,
        };
        let capabilities = config.to_capabilities();
        assert_eq!(capabilities.len(), 1);
        assert_eq!(capabilities[0].name, "drm-type");
        assert_eq!(capabilities[0].value, None);
    }
}
