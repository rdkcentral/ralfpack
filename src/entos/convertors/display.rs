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
use crate::entos::configs::DisplayConfiguration;
use crate::entos::convertors::common::*;

impl FromCapabilities for DisplayConfiguration {
    fn from_capabilities(capabilities: &config_xml::Capabilities) -> Option<Self> {
        let mut config = DisplayConfiguration {
            refresh_rate_hz: None,
            picture_mode: None,
        };

        if let Some(_) = capabilities.find("refresh-rate-60hz") {
            config.refresh_rate_hz = Some(60);
        }

        if let Some(cap) = capabilities.find("game-mode") {
            if let Some(value) = &cap.value {
                match value.to_ascii_lowercase().as_str() {
                    "static" => {
                        config.picture_mode = Some("gameModeStatic".to_string());
                    }
                    "dynamic" => {
                        config.picture_mode = Some("gameModeDynamic".to_string());
                    }
                    _ => {
                        log::warn!("Invalid game-mode value: {}", value);
                    }
                }
            } else {
                log::warn!("Missing 'game-mode' value");
            }
        }

        if config.refresh_rate_hz.is_none() && config.picture_mode.is_none() {
            None
        } else {
            Some(config)
        }
    }
}

impl ToCapabilities for DisplayConfiguration {
    fn to_capabilities(&self) -> Vec<config_xml::Capability> {
        let mut caps = Vec::new();

        if self.refresh_rate_hz == Some(60) {
            caps.push(config_xml::Capability {
                name: "refresh-rate-60hz".to_string(),
                value: None,
            });
        }

        if let Some(picture_mode) = &self.picture_mode {
            match picture_mode.to_ascii_lowercase().as_str() {
                "gamemodestatic" => {
                    caps.push(config_xml::Capability {
                        name: "game-mode".to_string(),
                        value: Some("static".to_string()),
                    });
                }
                "gamemodedynamic" => {
                    caps.push(config_xml::Capability {
                        name: "game-mode".to_string(),
                        value: Some("dynamic".to_string()),
                    });
                }
                _ => {
                    log::warn!("Unrecognised picture mode value: {}", picture_mode);
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
    fn test_display_configuration_serialization() {
        let config = DisplayConfiguration {
            refresh_rate_hz: Some(60),
            picture_mode: Some("gameModeStatic".to_string()),
        };
        let serialized = serde_json::to_string(&config).unwrap();
        let expected = json!({
            "refreshRateHz": 60,
            "pictureMode": "gameModeStatic"
        });
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&serialized).unwrap(),
            expected
        );

        let config = DisplayConfiguration {
            refresh_rate_hz: None,
            picture_mode: Some("gameModeDynamic".to_string()),
        };
        let serialized = serde_json::to_string(&config).unwrap();
        let expected = json!({
            "pictureMode": "gameModeDynamic"
        });
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&serialized).unwrap(),
            expected
        );

        let config = DisplayConfiguration {
            refresh_rate_hz: None,
            picture_mode: None,
        };
        let serialized = serde_json::to_string(&config).unwrap();
        let expected = json!({});
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&serialized).unwrap(),
            expected
        );
    }

    #[test]
    fn test_display_configuration_deserialization() {
        let data = json!({
            "refreshRateHz": 60,
            "pictureMode": "gameModeStatic"
        });
        let deserialized: DisplayConfiguration = serde_json::from_value(data).unwrap();
        assert_eq!(deserialized.refresh_rate_hz, Some(60));
        assert_eq!(deserialized.picture_mode, Some("gameModeStatic".to_string()));

        let data = json!({
            "pictureMode": "gameModeDynamic"
        });
        let deserialized: DisplayConfiguration = serde_json::from_value(data).unwrap();
        assert_eq!(deserialized.refresh_rate_hz, None);
        assert_eq!(deserialized.picture_mode, Some("gameModeDynamic".to_string()));

        let data = json!({
            "refreshRateHz": 60
        });
        let deserialized: DisplayConfiguration = serde_json::from_value(data).unwrap();
        assert_eq!(deserialized.refresh_rate_hz, Some(60));
        assert_eq!(deserialized.picture_mode, None);

        let data = json!({});
        let deserialized: DisplayConfiguration = serde_json::from_value(data).unwrap();
        assert_eq!(deserialized.refresh_rate_hz, None);
        assert_eq!(deserialized.picture_mode, None);
    }

    #[test]
    fn test_display_from_capabilities() {
        let caps = vec![
            config_xml::Capability {
                name: "refresh-rate-60hz".to_string(),
                value: None,
            },
            config_xml::Capability {
                name: "game-mode".to_string(),
                value: Some("static".to_string()),
            },
        ];
        let capabilities = config_xml::Capabilities { capabilities: caps };

        let config = DisplayConfiguration::from_capabilities(&capabilities);
        assert_eq!(
            config,
            Some(DisplayConfiguration {
                refresh_rate_hz: Some(60),
                picture_mode: Some("gameModeStatic".to_string())
            })
        );

        let caps = vec![config_xml::Capability {
            name: "game-mode".to_string(),
            value: Some("dynamic".to_string()),
        }];
        let capabilities = config_xml::Capabilities { capabilities: caps };

        let config = DisplayConfiguration::from_capabilities(&capabilities);
        assert_eq!(
            config,
            Some(DisplayConfiguration {
                refresh_rate_hz: None,
                picture_mode: Some("gameModeDynamic".to_string())
            })
        );

        let caps = vec![config_xml::Capability {
            name: "refresh-rate-60hz".to_string(),
            value: None,
        }];
        let capabilities = config_xml::Capabilities { capabilities: caps };

        let config = DisplayConfiguration::from_capabilities(&capabilities);
        assert_eq!(
            config,
            Some(DisplayConfiguration {
                refresh_rate_hz: Some(60),
                picture_mode: None
            })
        );

        let caps = vec![config_xml::Capability {
            name: "game-mode".to_string(),
            value: Some("invalid".to_string()),
        }];
        let capabilities = config_xml::Capabilities { capabilities: caps };

        let config = DisplayConfiguration::from_capabilities(&capabilities);
        assert_eq!(config, None);

        let caps = vec![config_xml::Capability {
            name: "some-other-cap".to_string(),
            value: Some("value".to_string()),
        }];
        let capabilities = config_xml::Capabilities { capabilities: caps };

        let config = DisplayConfiguration::from_capabilities(&capabilities);
        assert_eq!(config, None);
    }

    #[test]
    fn test_display_to_capabilities() {
        let config = DisplayConfiguration {
            refresh_rate_hz: Some(60),
            picture_mode: Some("gameModeStatic".to_string()),
        };
        let caps = config.to_capabilities();
        assert_eq!(caps.len(), 2);
        assert!(
            caps.iter()
                .any(|cap| cap.name == "refresh-rate-60hz" && cap.value.is_none())
        );
        assert!(
            caps.iter()
                .any(|cap| cap.name == "game-mode" && cap.value.as_deref() == Some("static"))
        );

        let config = DisplayConfiguration {
            refresh_rate_hz: None,
            picture_mode: Some("gameModeDynamic".to_string()),
        };
        let caps = config.to_capabilities();
        assert_eq!(caps.len(), 1);
        assert!(
            caps.iter()
                .any(|cap| cap.name == "game-mode" && cap.value.as_deref() == Some("dynamic"))
        );

        let config = DisplayConfiguration {
            refresh_rate_hz: Some(60),
            picture_mode: None,
        };
        let caps = config.to_capabilities();
        assert_eq!(caps.len(), 1);
        assert!(
            caps.iter()
                .any(|cap| cap.name == "refresh-rate-60hz" && cap.value.is_none())
        );

        let config = DisplayConfiguration {
            refresh_rate_hz: None,
            picture_mode: None,
        };
        let caps = config.to_capabilities();
        assert_eq!(caps.len(), 0);
    }
}
