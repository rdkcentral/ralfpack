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
use crate::entos::configs::MarketplaceInterceptConfiguration;
use crate::entos::convertors::common::*;

impl FromCapabilities for MarketplaceInterceptConfiguration {
    fn from_capabilities(capabilities: &config_xml::Capabilities) -> Option<Self> {
        // Parse the value as a boolean, log error if no value is provided or invalid value
        if let Some(cap) = capabilities.find("intercept") {
            if let Some(value) = &cap.value {
                let trimmed_value = value.trim().to_lowercase();
                match trimmed_value.as_str() {
                    "true" => {
                        return Some(MarketplaceInterceptConfiguration { enable: Some(true) });
                    }
                    "false" => {
                        return Some(MarketplaceInterceptConfiguration { enable: Some(false) });
                    }
                    _ => {
                        log::error!("intercept capability value must be 'true' or 'false', got: '{}'", value);
                    }
                }
            } else {
                log::error!("intercept capability requires a value (true or false)");
            }
        }

        None
    }
}

impl ToCapabilities for MarketplaceInterceptConfiguration {
    fn to_capabilities(&self) -> Vec<config_xml::Capability> {
        let mut caps = Vec::new();

        if let Some(value) = self.enable {
            caps.push(config_xml::Capability {
                name: "intercept".to_string(),
                value: Some(value.to_string()),
            });
        }

        caps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intercept_configuration_serialization() {
        let config = MarketplaceInterceptConfiguration { enable: Some(true) };
        let serialized = serde_json::to_string(&config).unwrap();
        let expected = serde_json::json!({
            "enable": true
        });
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&serialized).unwrap(),
            expected
        );

        let config = MarketplaceInterceptConfiguration { enable: Some(false) };
        let serialized = serde_json::to_string(&config).unwrap();
        let expected = serde_json::json!({
            "enable": false
        });
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&serialized).unwrap(),
            expected
        );

        let config = MarketplaceInterceptConfiguration { enable: None };
        let serialized = serde_json::to_string(&config).unwrap();
        let expected = serde_json::json!({});
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&serialized).unwrap(),
            expected
        );
    }

    #[test]
    fn test_intercept_from_capability() {
        // Test intercept capability with value "true"
        let capabilities = config_xml::Capabilities {
            capabilities: vec![config_xml::Capability {
                name: "intercept".to_string(),
                value: Some("tRue".to_string()),
            }],
        };

        let intercept_config = MarketplaceInterceptConfiguration::from_capabilities(&capabilities).unwrap();
        assert_eq!(intercept_config.enable, Some(true));

        // Test intercept capability with value "false"
        let capabilities = config_xml::Capabilities {
            capabilities: vec![config_xml::Capability {
                name: "intercept".to_string(),
                value: Some("FalsE".to_string()),
            }],
        };

        let intercept_config = MarketplaceInterceptConfiguration::from_capabilities(&capabilities).unwrap();
        assert_eq!(intercept_config.enable, Some(false));

        // Test intercept capability without value (should log error and not add config)
        let capabilities = config_xml::Capabilities {
            capabilities: vec![config_xml::Capability {
                name: "intercept".to_string(),
                value: None,
            }],
        };

        let intercept_config = MarketplaceInterceptConfiguration::from_capabilities(&capabilities);
        assert!(intercept_config.is_none());

        // Test intercept capability with invalid value (should log error and not add config)
        let capabilities = config_xml::Capabilities {
            capabilities: vec![config_xml::Capability {
                name: "intercept".to_string(),
                value: Some("maybe".to_string()),
            }],
        };

        let intercept_config = MarketplaceInterceptConfiguration::from_capabilities(&capabilities);
        assert!(intercept_config.is_none());

        // Test no intercept capability at all (should be fine, no config added)
        let capabilities = config_xml::Capabilities {
            capabilities: vec![config_xml::Capability {
                name: "wan-lan".to_string(),
                value: None,
            }],
        };

        let intercept_config = MarketplaceInterceptConfiguration::from_capabilities(&capabilities);
        assert!(intercept_config.is_none());
    }

    #[test]
    fn test_intercept_to_capability() {
        let config = MarketplaceInterceptConfiguration { enable: Some(true) };
        let capabilities = config.to_capabilities();
        assert_eq!(capabilities.len(), 1);
        assert_eq!(capabilities[0].name, "intercept");
        assert_eq!(capabilities[0].value.as_ref().unwrap(), "true");

        let config = MarketplaceInterceptConfiguration { enable: Some(false) };
        let capabilities = config.to_capabilities();
        assert_eq!(capabilities.len(), 1);
        assert_eq!(capabilities[0].name, "intercept");
        assert_eq!(capabilities[0].value.as_ref().unwrap(), "false");

        let config = MarketplaceInterceptConfiguration { enable: None };
        let capabilities = config.to_capabilities();
        assert_eq!(capabilities.len(), 0);
    }
}
