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
use crate::entos::configs::PreLaunchConfiguration;
use crate::entos::convertors::common::*;

impl FromCapabilities for PreLaunchConfiguration {
    fn from_capabilities(capabilities: &config_xml::Capabilities) -> Option<Self> {
        if let Some(cap) = capabilities.find("pre-launch") {
            if let Some(value) = &cap.value {
                return match value.to_ascii_lowercase().as_str() {
                    "allowed" => Some(PreLaunchConfiguration::Allowed),
                    "recent" => Some(PreLaunchConfiguration::Recent),
                    "never" => Some(PreLaunchConfiguration::Never),
                    _ => {
                        log::warn!("Invalid 'pre-launch' value: {}", value);
                        None
                    }
                };
            }
        }

        None
    }
}

impl ToCapabilities for PreLaunchConfiguration {
    fn to_capabilities(&self) -> Vec<config_xml::Capability> {
        Vec::from([config_xml::Capability {
            name: "pre-launch".to_string(),
            value: Some(
                match self {
                    PreLaunchConfiguration::Never => "never",
                    PreLaunchConfiguration::Recent => "recent",
                    PreLaunchConfiguration::Allowed => "allowed",
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
    fn test_serialize_pre_launch() {
        let config = PreLaunchConfiguration::Allowed;
        let json_value = serde_json::to_value(&config).unwrap();
        assert_eq!(json_value, json!("allowed"));

        let config = PreLaunchConfiguration::Recent;
        let json_value = serde_json::to_value(&config).unwrap();
        assert_eq!(json_value, json!("recent"));

        let config = PreLaunchConfiguration::Never;
        let json_value = serde_json::to_value(&config).unwrap();
        assert_eq!(json_value, json!("never"));
    }

    #[test]
    fn test_deserialize_pre_launch() {
        let json_value = json!("allowed");
        let config: PreLaunchConfiguration = serde_json::from_value(json_value).unwrap();
        assert_eq!(config, PreLaunchConfiguration::Allowed);

        let json_value = json!("reCent");
        let config: PreLaunchConfiguration = serde_json::from_value(json_value).unwrap();
        assert_eq!(config, PreLaunchConfiguration::Recent);

        let json_value = json!("neveR");
        let config: PreLaunchConfiguration = serde_json::from_value(json_value).unwrap();
        assert_eq!(config, PreLaunchConfiguration::Never);

        let json_value = json!("invalid");
        let result: Result<PreLaunchConfiguration, _> = serde_json::from_value(json_value);
        assert!(result.is_err());
    }

    #[test]
    fn test_pre_launch_from_capabilities() {
        let caps = vec![
            config_xml::Capability {
                name: "pre-launch".to_string(),
                value: Some("allowed".to_string()),
            },
            config_xml::Capability {
                name: "some-other-cap".to_string(),
                value: Some("value".to_string()),
            },
        ];
        let capabilities = config_xml::Capabilities { capabilities: caps };

        let config = PreLaunchConfiguration::from_capabilities(&capabilities);
        assert_eq!(config, Some(PreLaunchConfiguration::Allowed));

        let caps = vec![config_xml::Capability {
            name: "pre-launch".to_string(),
            value: Some("reCent".to_string()),
        }];
        let capabilities = config_xml::Capabilities { capabilities: caps };

        let config = PreLaunchConfiguration::from_capabilities(&capabilities);
        assert_eq!(config, Some(PreLaunchConfiguration::Recent));

        let caps = vec![config_xml::Capability {
            name: "pre-launch".to_string(),
            value: Some("never".to_string()),
        }];
        let capabilities = config_xml::Capabilities { capabilities: caps };

        let config = PreLaunchConfiguration::from_capabilities(&capabilities);
        assert_eq!(config, Some(PreLaunchConfiguration::Never));

        let caps = vec![config_xml::Capability {
            name: "pre-launch".to_string(),
            value: Some("invalid".to_string()),
        }];
        let capabilities = config_xml::Capabilities { capabilities: caps };

        let config = PreLaunchConfiguration::from_capabilities(&capabilities);
        assert_eq!(config, None);

        let caps = vec![config_xml::Capability {
            name: "some-other-cap".to_string(),
            value: Some("value".to_string()),
        }];
        let capabilities = config_xml::Capabilities { capabilities: caps };

        let config = PreLaunchConfiguration::from_capabilities(&capabilities);
        assert_eq!(config, None);
    }

    #[test]
    fn test_pre_launch_to_capabilities() {
        let config = PreLaunchConfiguration::Allowed;
        let caps = config.to_capabilities();
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].name, "pre-launch");
        assert_eq!(caps[0].value.as_deref(), Some("allowed"));

        let config = PreLaunchConfiguration::Recent;
        let caps = config.to_capabilities();
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].name, "pre-launch");
        assert_eq!(caps[0].value.as_deref(), Some("recent"));

        let config = PreLaunchConfiguration::Never;
        let caps = config.to_capabilities();
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].name, "pre-launch");
        assert_eq!(caps[0].value.as_deref(), Some("never"));
    }
}
