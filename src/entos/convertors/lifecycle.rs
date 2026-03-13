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

use crate::package_config::{ApplicationLifecycleConfiguration, LifecycleState};

impl FromCapabilities for ApplicationLifecycleConfiguration {
    fn from_capabilities(capabilities: &config_xml::Capabilities) -> Option<Self> {
        let mut lifecycle = ApplicationLifecycleConfiguration {
            supported_non_active_states: Vec::new(),
            max_suspended_system_memory: None,
            max_time_to_suspend_memory_state: None,
            startup_timeout: None,
            watchdog_interval: None,
        };

        lifecycle.supported_non_active_states.push(LifecycleState::Paused);

        for cap in capabilities.capabilities.iter() {
            match cap.name.as_str() {
                "start-timeout-sec" => {
                    if let Some(value) = &cap.value {
                        if let Ok(parsed_value) = value.parse::<f32>() {
                            lifecycle.startup_timeout = Some(parsed_value);
                        } else {
                            log::warn!("Invalid start-timeout-sec value: {}", value);
                        }
                    }
                }
                "watchdog-sec" => {
                    if let Some(value) = &cap.value {
                        if let Ok(parsed_value) = value.parse::<f32>() {
                            lifecycle.watchdog_interval = Some(parsed_value);
                        } else {
                            log::warn!("Invalid start-timeout-sec value: {}", value);
                        }
                    }
                }
                "suspend-mode" => {
                    lifecycle.supported_non_active_states.push(LifecycleState::Suspended);
                }
                "hibernate-mode" => {
                    lifecycle.supported_non_active_states.push(LifecycleState::Hibernated);
                }
                _ => {}
            }
        }

        if lifecycle.supported_non_active_states.is_empty()
            && lifecycle.max_suspended_system_memory.is_none()
            && lifecycle.max_time_to_suspend_memory_state.is_none()
            && lifecycle.startup_timeout.is_none()
            && lifecycle.watchdog_interval.is_none()
        {
            // No lifecycle settings found, return None
            return None;
        }

        Some(lifecycle)
    }
}

impl ToCapabilities for ApplicationLifecycleConfiguration {
    fn to_capabilities(&self) -> Vec<config_xml::Capability> {
        let mut caps = Vec::new();

        if let Some(timeout) = self.startup_timeout {
            caps.push(config_xml::Capability {
                name: "start-timeout-sec".to_string(),
                value: Some(timeout.to_string()),
            });
        }

        if let Some(interval) = self.watchdog_interval {
            caps.push(config_xml::Capability {
                name: "watchdog-sec".to_string(),
                value: Some(interval.to_string()),
            });
        }

        if self.supported_non_active_states.contains(&LifecycleState::Suspended) {
            caps.push(config_xml::Capability {
                name: "suspend-mode".to_string(),
                value: None,
            });
        }

        if self.supported_non_active_states.contains(&LifecycleState::Hibernated) {
            caps.push(config_xml::Capability {
                name: "hibernate-mode".to_string(),
                value: None,
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
    fn test_application_lifecycle_config_from_capabilities() {
        // Test with empty capabilities
        let empty_caps = config_xml::Capabilities {
            capabilities: Vec::new(),
        };
        let config = ApplicationLifecycleConfiguration::from_capabilities(&empty_caps).unwrap();
        assert_eq!(config.startup_timeout, None);
        assert_eq!(config.watchdog_interval, None);
        assert_eq!(config.max_suspended_system_memory, None);
        assert_eq!(config.max_time_to_suspend_memory_state, None);
        assert_eq!(config.supported_non_active_states.len(), 1);
        assert!(config.supported_non_active_states.contains(&LifecycleState::Paused));

        // Test with start timeout only
        let timeout_caps = config_xml::Capabilities {
            capabilities: vec![config_xml::Capability {
                name: "start-timeout-sec".to_string(),
                value: Some("30.5".to_string()),
            }],
        };
        let config = ApplicationLifecycleConfiguration::from_capabilities(&timeout_caps).unwrap();
        assert_eq!(config.startup_timeout, Some(30.5));
        assert_eq!(config.watchdog_interval, None);
        assert_eq!(config.supported_non_active_states.len(), 1);
        assert!(config.supported_non_active_states.contains(&LifecycleState::Paused));

        // Test with watchdog interval only
        let watchdog_caps = config_xml::Capabilities {
            capabilities: vec![config_xml::Capability {
                name: "watchdog-sec".to_string(),
                value: Some("60.0".to_string()),
            }],
        };
        let config = ApplicationLifecycleConfiguration::from_capabilities(&watchdog_caps).unwrap();
        assert_eq!(config.watchdog_interval, Some(60.0));
        assert_eq!(config.startup_timeout, None);
        assert_eq!(config.supported_non_active_states.len(), 1);
        assert!(config.supported_non_active_states.contains(&LifecycleState::Paused));

        // Test with suspend mode
        let suspend_caps = config_xml::Capabilities {
            capabilities: vec![config_xml::Capability {
                name: "suspend-mode".to_string(),
                value: Some("true".to_string()),
            }],
        };
        let config = ApplicationLifecycleConfiguration::from_capabilities(&suspend_caps).unwrap();
        assert_eq!(config.supported_non_active_states.len(), 2);
        assert!(config.supported_non_active_states.contains(&LifecycleState::Paused));
        assert!(config.supported_non_active_states.contains(&LifecycleState::Suspended));

        // Test with hibernate mode
        let hibernate_caps = config_xml::Capabilities {
            capabilities: vec![config_xml::Capability {
                name: "hibernate-mode".to_string(),
                value: Some("true".to_string()),
            }],
        };
        let config = ApplicationLifecycleConfiguration::from_capabilities(&hibernate_caps).unwrap();
        assert_eq!(config.supported_non_active_states.len(), 2);
        assert!(config.supported_non_active_states.contains(&LifecycleState::Paused));
        assert!(config.supported_non_active_states.contains(&LifecycleState::Hibernated));

        // Test with all capabilities
        let full_caps = config_xml::Capabilities {
            capabilities: vec![
                config_xml::Capability {
                    name: "start-timeout-sec".to_string(),
                    value: Some("45.0".to_string()),
                },
                config_xml::Capability {
                    name: "watchdog-sec".to_string(),
                    value: Some("120.5".to_string()),
                },
                config_xml::Capability {
                    name: "suspend-mode".to_string(),
                    value: Some("true".to_string()),
                },
                config_xml::Capability {
                    name: "hibernate-mode".to_string(),
                    value: Some("true".to_string()),
                },
                config_xml::Capability {
                    name: "unrelated-capability".to_string(),
                    value: Some("ignored".to_string()),
                },
            ],
        };
        let config = ApplicationLifecycleConfiguration::from_capabilities(&full_caps).unwrap();
        assert_eq!(config.startup_timeout, Some(45.0));
        assert_eq!(config.watchdog_interval, Some(120.5));
        assert_eq!(config.supported_non_active_states.len(), 3);
        assert!(config.supported_non_active_states.contains(&LifecycleState::Paused));
        assert!(config.supported_non_active_states.contains(&LifecycleState::Suspended));
        assert!(config.supported_non_active_states.contains(&LifecycleState::Hibernated));

        // Test with invalid timeout value
        let invalid_timeout_caps = config_xml::Capabilities {
            capabilities: vec![config_xml::Capability {
                name: "start-timeout-sec".to_string(),
                value: Some("invalid".to_string()),
            }],
        };
        let config = ApplicationLifecycleConfiguration::from_capabilities(&invalid_timeout_caps).unwrap();
        assert_eq!(config.startup_timeout, None);
        assert_eq!(config.supported_non_active_states.len(), 1);
        assert!(config.supported_non_active_states.contains(&LifecycleState::Paused));

        // Test with invalid watchdog value
        let invalid_watchdog_caps = config_xml::Capabilities {
            capabilities: vec![config_xml::Capability {
                name: "watchdog-sec".to_string(),
                value: Some("not_a_number".to_string()),
            }],
        };
        let config = ApplicationLifecycleConfiguration::from_capabilities(&invalid_watchdog_caps).unwrap();
        assert_eq!(config.watchdog_interval, None);
        assert_eq!(config.supported_non_active_states.len(), 1);
        assert!(config.supported_non_active_states.contains(&LifecycleState::Paused));

        // Test with empty values
        let empty_value_caps = config_xml::Capabilities {
            capabilities: vec![config_xml::Capability {
                name: "start-timeout-sec".to_string(),
                value: None,
            }],
        };
        let config = ApplicationLifecycleConfiguration::from_capabilities(&empty_value_caps).unwrap();
        assert_eq!(config.startup_timeout, None);
        assert_eq!(config.supported_non_active_states.len(), 1);
        assert!(config.supported_non_active_states.contains(&LifecycleState::Paused));
    }

    #[test]
    fn test_application_lifecycle_config_to_capabilities() {
        // Test with minimal config (only paused state)
        let config = ApplicationLifecycleConfiguration {
            supported_non_active_states: vec![LifecycleState::Paused],
            max_suspended_system_memory: None,
            max_time_to_suspend_memory_state: None,
            startup_timeout: None,
            watchdog_interval: None,
        };
        let capabilities = config.to_capabilities();
        assert_eq!(capabilities.len(), 0);

        // Test with startup timeout
        let config = ApplicationLifecycleConfiguration {
            supported_non_active_states: vec![LifecycleState::Paused],
            max_suspended_system_memory: None,
            max_time_to_suspend_memory_state: None,
            startup_timeout: Some(30.5),
            watchdog_interval: None,
        };
        let capabilities = config.to_capabilities();
        assert_eq!(capabilities.len(), 1);
        assert_eq!(capabilities[0].name, "start-timeout-sec");
        assert_eq!(capabilities[0].value.as_ref().unwrap(), "30.5");

        // Test with watchdog interval
        let config = ApplicationLifecycleConfiguration {
            supported_non_active_states: vec![LifecycleState::Paused],
            max_suspended_system_memory: None,
            max_time_to_suspend_memory_state: None,
            startup_timeout: None,
            watchdog_interval: Some(60.0),
        };
        let capabilities = config.to_capabilities();
        assert_eq!(capabilities.len(), 1);
        assert_eq!(capabilities[0].name, "watchdog-sec");
        assert_eq!(capabilities[0].value.as_ref().unwrap(), "60");

        // Test with suspended state
        let config = ApplicationLifecycleConfiguration {
            supported_non_active_states: vec![LifecycleState::Paused, LifecycleState::Suspended],
            max_suspended_system_memory: None,
            max_time_to_suspend_memory_state: None,
            startup_timeout: None,
            watchdog_interval: None,
        };
        let capabilities = config.to_capabilities();
        assert_eq!(capabilities.len(), 1);
        assert_eq!(capabilities[0].name, "suspend-mode");
        assert_eq!(capabilities[0].value, None);

        // Test with hibernated state
        let config = ApplicationLifecycleConfiguration {
            supported_non_active_states: vec![LifecycleState::Paused, LifecycleState::Hibernated],
            max_suspended_system_memory: None,
            max_time_to_suspend_memory_state: None,
            startup_timeout: None,
            watchdog_interval: None,
        };
        let capabilities = config.to_capabilities();
        assert_eq!(capabilities.len(), 1);
        assert_eq!(capabilities[0].name, "hibernate-mode");
        assert_eq!(capabilities[0].value, None);

        // Test with all capabilities
        let config = ApplicationLifecycleConfiguration {
            supported_non_active_states: vec![
                LifecycleState::Paused,
                LifecycleState::Suspended,
                LifecycleState::Hibernated,
            ],
            max_suspended_system_memory: None,
            max_time_to_suspend_memory_state: None,
            startup_timeout: Some(45.0),
            watchdog_interval: Some(120.5),
        };
        let capabilities = config.to_capabilities();
        assert_eq!(capabilities.len(), 4);

        let mut cap_names: Vec<String> = capabilities.iter().map(|c| c.name.clone()).collect();
        cap_names.sort();
        assert_eq!(
            cap_names,
            vec!["hibernate-mode", "start-timeout-sec", "suspend-mode", "watchdog-sec"]
        );

        // Verify specific values
        let start_timeout_cap = capabilities.iter().find(|c| c.name == "start-timeout-sec").unwrap();
        assert_eq!(start_timeout_cap.value.as_ref().unwrap(), "45");

        let watchdog_cap = capabilities.iter().find(|c| c.name == "watchdog-sec").unwrap();
        assert_eq!(watchdog_cap.value.as_ref().unwrap(), "120.5");

        let suspend_cap = capabilities.iter().find(|c| c.name == "suspend-mode").unwrap();
        assert_eq!(suspend_cap.value, None);

        let hibernate_cap = capabilities.iter().find(|c| c.name == "hibernate-mode").unwrap();
        assert_eq!(hibernate_cap.value, None);
    }

    #[test]
    fn test_application_lifecycle_config_roundtrip() {
        // Create original capabilities
        let original_caps = config_xml::Capabilities {
            capabilities: vec![
                config_xml::Capability {
                    name: "start-timeout-sec".to_string(),
                    value: Some("30.0".to_string()),
                },
                config_xml::Capability {
                    name: "watchdog-sec".to_string(),
                    value: Some("60.5".to_string()),
                },
                config_xml::Capability {
                    name: "suspend-mode".to_string(),
                    value: None,
                },
                config_xml::Capability {
                    name: "hibernate-mode".to_string(),
                    value: None,
                },
            ],
        };

        // Convert to config and back
        let config = ApplicationLifecycleConfiguration::from_capabilities(&original_caps).unwrap();
        let converted_caps = config.to_capabilities();

        // Verify roundtrip conversion
        assert_eq!(converted_caps.len(), 4);

        let start_timeout_cap = converted_caps.iter().find(|c| c.name == "start-timeout-sec").unwrap();
        assert_eq!(start_timeout_cap.value.as_ref().unwrap(), "30");

        let watchdog_cap = converted_caps.iter().find(|c| c.name == "watchdog-sec").unwrap();
        assert_eq!(watchdog_cap.value.as_ref().unwrap(), "60.5");

        let suspend_cap = converted_caps.iter().find(|c| c.name == "suspend-mode").unwrap();
        assert_eq!(suspend_cap.value, None);

        let hibernate_cap = converted_caps.iter().find(|c| c.name == "hibernate-mode").unwrap();
        assert_eq!(hibernate_cap.value, None);

        // Convert back again to verify consistency
        let config2 = ApplicationLifecycleConfiguration::from_capabilities(&config_xml::Capabilities {
            capabilities: converted_caps,
        })
        .unwrap();

        assert_eq!(config.startup_timeout, config2.startup_timeout);
        assert_eq!(config.watchdog_interval, config2.watchdog_interval);
        assert_eq!(
            config.supported_non_active_states.len(),
            config2.supported_non_active_states.len()
        );
        for state in &config.supported_non_active_states {
            assert!(config2.supported_non_active_states.contains(state));
        }
    }

    #[test]
    fn test_deserialize_lifecycle_config() {
        let json_snippet = json!({
            "supportedNonActiveStates": ["paused", "suspended"],
            "maxSuspendedSystemMemory": "256M",
            "maxTimeToSuspendMemoryState": 300.5,
            "startupTimeout": 30.0,
            "watchdogInterval": 60
        });

        let config = serde_json::from_value::<ApplicationLifecycleConfiguration>(json_snippet).unwrap();
        assert_eq!(config.supported_non_active_states.len(), 2);
        assert!(config.supported_non_active_states.contains(&LifecycleState::Paused));
        assert!(config.supported_non_active_states.contains(&LifecycleState::Suspended));
        assert_eq!(config.max_suspended_system_memory.as_deref(), Some("256M"));
        assert_eq!(config.max_time_to_suspend_memory_state, Some(300.5));
        assert_eq!(config.startup_timeout, Some(30.0));
        assert_eq!(config.watchdog_interval, Some(60.0));

        let json_snippet = json!({});

        let config = serde_json::from_value::<ApplicationLifecycleConfiguration>(json_snippet).unwrap();
        assert_eq!(config.supported_non_active_states.len(), 0);
        assert_eq!(config.max_suspended_system_memory, None);
        assert_eq!(config.max_time_to_suspend_memory_state, None);
        assert_eq!(config.startup_timeout, None);
        assert_eq!(config.watchdog_interval, None);
    }
}
