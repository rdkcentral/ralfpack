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

use crate::package_config::StorageConfiguration;

impl FromCapabilities for StorageConfiguration {
    /// Populates the storage quotas for the package as well as the shared storage app ID if
    /// the app is a child app.
    fn from_capabilities(capabilities: &config_xml::Capabilities) -> Option<Self> {
        // Iterate through the capabilities to find storage-related ones
        let mut storage_quota: Option<String> = None;
        let mut parent_id = None;
        for cap in capabilities.capabilities.iter() {
            if cap.name == "storage" {
                // This is a legacy capability, it means give the app the default storage quota.
                // OCI format doesn't have an equivalent default config, so we give it a value
                // based on what the default is traditionally set to - 12MB
                if storage_quota.is_none() {
                    storage_quota = Some("12M".to_string());
                }
            } else if cap.name == "private-storage" {
                // The capability private-storage should just contain an integer value that
                // represents the storage quota in megabytes.
                if let Some(value) = &cap.value {
                    // If the value is not empty, parse it as an u64
                    if let Ok(parsed_value) = value.parse::<u64>() {
                        storage_quota = Some(format!("{}M", parsed_value));
                    } else {
                        log::warn!("Invalid private-storage value: {}", value);
                    }
                }
            } else if cap.name == "child-app" {
                // The capability child-app is used to indicate that the app can use the storage
                // of its parent app.
                parent_id = cap.value.clone();
            }
        }

        if storage_quota.is_none() && parent_id.is_none() {
            // No quotas set, return None
            return None;
        }

        // If the app is a child app, it should not have its own storage quota, so set it to None.
        if parent_id.is_some() {
            storage_quota = None;
        }

        Some(StorageConfiguration {
            max_local_storage: storage_quota,
            shared_storage_app_id: parent_id,
        })
    }
}

impl ToCapabilities for StorageConfiguration {
    /// Converts the storage setting to a set of capability
    fn to_capabilities(&self) -> Vec<config_xml::Capability> {
        let mut caps = Vec::new();

        // If the app has a shared storage app ID, it is a child app, so we add the child-app
        // capability with the parent app ID as the value.
        if let Some(parent_id) = &self.shared_storage_app_id {
            caps.push(config_xml::Capability {
                name: "child-app".to_string(),
                value: Some(parent_id.clone()),
            });
        }

        // If the app has a max local storage quota, we add the private-storage capability
        if let Some(quota) = &self.max_local_storage {
            // Extract the numeric part of the quota (assuming it's in the format "12M", "100M", etc.)
            if let Some(num_part) = quota.strip_suffix('M') {
                if let Ok(num) = num_part.parse::<u64>() {
                    caps.push(config_xml::Capability {
                        name: "private-storage".to_string(),
                        value: Some(num.to_string()),
                    });
                }
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
    fn test_storage_config_from_capabilities() {
        // Test case with storage capability
        let capabilities = config_xml::Capabilities {
            capabilities: vec![config_xml::Capability {
                name: "storage".to_string(),
                value: None,
            }],
        };
        let storage_config = StorageConfiguration::from_capabilities(&capabilities).unwrap();
        assert_eq!(storage_config.max_local_storage, Some("12M".to_string()));
        assert_eq!(storage_config.shared_storage_app_id, None);

        // Test case with private-storage capability
        let capabilities = config_xml::Capabilities {
            capabilities: vec![config_xml::Capability {
                name: "private-storage".to_string(),
                value: Some("50".to_string()),
            }],
        };
        let storage_config = StorageConfiguration::from_capabilities(&capabilities).unwrap();
        assert_eq!(storage_config.max_local_storage, Some("50M".to_string()));
        assert_eq!(storage_config.shared_storage_app_id, None);

        // Test case with child-app capability
        let capabilities = config_xml::Capabilities {
            capabilities: vec![config_xml::Capability {
                name: "child-app".to_string(),
                value: Some("parent.app.id".to_string()),
            }],
        };
        let storage_config = StorageConfiguration::from_capabilities(&capabilities).unwrap();
        assert_eq!(storage_config.max_local_storage, None);
        assert_eq!(storage_config.shared_storage_app_id, Some("parent.app.id".to_string()));

        // Test case with both private-storage and child-app capabilities
        let capabilities = config_xml::Capabilities {
            capabilities: vec![
                config_xml::Capability {
                    name: "private-storage".to_string(),
                    value: Some("100".to_string()),
                },
                config_xml::Capability {
                    name: "child-app".to_string(),
                    value: Some("parent.app.id".to_string()),
                },
            ],
        };
        let storage_config = StorageConfiguration::from_capabilities(&capabilities).unwrap();
        assert_eq!(storage_config.max_local_storage, None);
        assert_eq!(storage_config.shared_storage_app_id, Some("parent.app.id".to_string()));

        // Test case with no relevant capabilities
        let capabilities = config_xml::Capabilities { capabilities: vec![] };
        let storage_config = StorageConfiguration::from_capabilities(&capabilities);
        assert!(storage_config.is_none());
    }

    #[test]
    fn test_storage_config_to_capabilities() {
        let json_snippet = json!({
            "maxLocalStorage": "32M",
            "sharedStorageAppId": "some.parent.app"
        });

        let config: StorageConfiguration = serde_json::from_value(json_snippet).unwrap();
        assert_eq!(config.max_local_storage, Some("32M".to_string()));
        assert_eq!(config.shared_storage_app_id, Some("some.parent.app".to_string()));

        let capabilities = config.to_capabilities();
        assert_eq!(capabilities.len(), 2);
        assert_eq!(capabilities[0].name, "child-app");
        assert_eq!(capabilities[0].value, Some("some.parent.app".to_string()));
        assert_eq!(capabilities[1].name, "private-storage");
        assert_eq!(capabilities[1].value, Some("32".to_string()));
    }
}
