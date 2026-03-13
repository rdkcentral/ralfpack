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
use std::collections::HashSet;

use crate::package_config::DialConfiguration;

impl FromCapabilities for DialConfiguration {
    fn from_capabilities(capabilities: &config_xml::Capabilities) -> Option<Self> {
        let mut dial_configuration = DialConfiguration {
            app_names: HashSet::new(),
            cors_domains: HashSet::new(),
            origin_header_required: false,
        };

        let mut found_dial_capability = false;
        for cap in capabilities.capabilities.iter() {
            match cap.name.as_str() {
                "dial-app" => {
                    if let Some(value) = &cap.value {
                        dial_configuration.cors_domains = split_to_set(value, ',');
                    }
                    found_dial_capability = true;
                }
                "dial-id" => {
                    if let Some(value) = &cap.value {
                        dial_configuration.app_names = split_to_set(value, ',');
                        found_dial_capability = true;
                    }
                }
                "dial-origin-mandatory" => {
                    dial_configuration.origin_header_required = true;
                    found_dial_capability = true;
                }
                _ => {}
            }
        }

        if found_dial_capability == false {
            // No DIAL settings found, return None
            return None;
        }

        Some(dial_configuration)
    }
}

impl ToCapabilities for DialConfiguration {
    fn to_capabilities(&self) -> Vec<config_xml::Capability> {
        let mut caps = Vec::new();

        // If the DIAL configuration is set then it means the app is DIAL enabled and so we need to
        // add the capabilities even if the sets are empty.
        caps.push(config_xml::Capability {
            name: "dial-app".to_string(),
            value: Some(set_to_string(&self.cors_domains, ',')),
        });

        if !self.app_names.is_empty() {
            caps.push(config_xml::Capability {
                name: "dial-id".to_string(),
                value: Some(set_to_string(&self.app_names, ',')),
            });
        }

        if self.origin_header_required {
            caps.push(config_xml::Capability {
                name: "dial-origin-mandatory".to_string(),
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
    fn test_dial_configuration_serialization() {
        let snippet = json!({
            "appNames": ["YouTube", "Netflix"],
            "corsDomains": ["example.com", "another.com"],
            "originHeaderRequired": true
        });

        let dial_config = serde_json::from_value::<DialConfiguration>(snippet).unwrap();
        assert_eq!(dial_config.app_names.len(), 2);
        assert!(dial_config.app_names.contains("YouTube"));
        assert!(dial_config.app_names.contains("Netflix"));
        assert_eq!(dial_config.cors_domains.len(), 2);
        assert!(dial_config.cors_domains.contains("example.com"));
        assert!(dial_config.cors_domains.contains("another.com"));
        assert_eq!(dial_config.origin_header_required, true);

        let snippet = json!({
            "corsDomains": ["example.com", "another.com"],
            "originHeaderRequired": true
        });

        let dial_config = serde_json::from_value::<DialConfiguration>(snippet).unwrap();
        assert_eq!(dial_config.app_names.len(), 0);
        assert_eq!(dial_config.cors_domains.len(), 2);
        assert!(dial_config.cors_domains.contains("example.com"));
        assert!(dial_config.cors_domains.contains("another.com"));
        assert_eq!(dial_config.origin_header_required, true);

        let snippet = json!({
            "originHeaderRequired": true
        });

        let dial_config = serde_json::from_value::<DialConfiguration>(snippet).unwrap();
        assert_eq!(dial_config.app_names.len(), 0);
        assert_eq!(dial_config.cors_domains.len(), 0);
        assert_eq!(dial_config.origin_header_required, true);

        let snippet = json!({});

        let dial_config = serde_json::from_value::<DialConfiguration>(snippet).unwrap();
        assert_eq!(dial_config.app_names.len(), 0);
        assert_eq!(dial_config.cors_domains.len(), 0);
        assert_eq!(dial_config.origin_header_required, false);
    }

    #[test]
    fn test_dial_configuration_to_capabilities() {
        let dial_config = DialConfiguration {
            app_names: HashSet::new(),
            cors_domains: HashSet::new(),
            origin_header_required: false,
        };

        // Test with empty configuration
        let caps = dial_config.to_capabilities();
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].name, "dial-app");
        assert_eq!(caps[0].value.as_deref(), Some(""));

        // Test with some app names and CORS domains
        let snippet = json!({
            "appNames": ["YouTube", "Netflix"],
            "corsDomains": ["example.com", "another.com"],
            "originHeaderRequired": true
        });

        let dial_config = serde_json::from_value::<DialConfiguration>(snippet).unwrap();

        let caps = dial_config.to_capabilities();
        assert_eq!(caps.len(), 3);

        let mut found_dial_app = false;
        let mut found_dial_id = false;
        let mut found_dial_origin = false;

        for cap in &caps {
            match cap.name.as_str() {
                "dial-app" => {
                    found_dial_app = true;
                    let value = cap.value.as_ref().unwrap();
                    let domains: HashSet<_> = split_to_set(value, ',');
                    assert_eq!(domains.len(), 2);
                    assert!(domains.contains("example.com"));
                    assert!(domains.contains("another.com"));
                }
                "dial-id" => {
                    found_dial_id = true;
                    let value = cap.value.as_ref().unwrap();
                    let apps: HashSet<_> = split_to_set(value, ',');
                    assert_eq!(apps.len(), 2);
                    assert!(apps.contains("YouTube"));
                    assert!(apps.contains("Netflix"));
                }
                "dial-origin-mandatory" => {
                    found_dial_origin = true;
                }
                _ => {}
            }
        }

        assert!(found_dial_app);
        assert!(found_dial_id);
        assert!(found_dial_origin);
    }

    #[test]
    fn test_dial_configuration_from_capabilities() {
        let capabilities = config_xml::Capabilities {
            capabilities: vec![
                config_xml::Capability {
                    name: "dial-app".to_string(),
                    value: Some("example.com,another.com".to_string()),
                },
                config_xml::Capability {
                    name: "dial-id".to_string(),
                    value: Some("YouTube,Netflix".to_string()),
                },
                config_xml::Capability {
                    name: "dial-origin-mandatory".to_string(),
                    value: None,
                },
                config_xml::Capability {
                    name: "unrelated-capability".to_string(),
                    value: Some("ignored".to_string()),
                },
            ],
        };

        let dial_config = DialConfiguration::from_capabilities(&capabilities).unwrap();
        assert_eq!(dial_config.app_names.len(), 2);
        assert!(dial_config.app_names.contains("YouTube"));
        assert!(dial_config.app_names.contains("Netflix"));
        assert_eq!(dial_config.cors_domains.len(), 2);
        assert!(dial_config.cors_domains.contains("example.com"));
        assert!(dial_config.cors_domains.contains("another.com"));
        assert!(dial_config.origin_header_required);
    }
}
