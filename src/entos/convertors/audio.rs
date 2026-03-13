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
use crate::entos::configs::AudioConfiguration;
use crate::entos::convertors::common::*;

impl FromCapabilities for AudioConfiguration {
    fn from_capabilities(capabilities: &config_xml::Capabilities) -> Option<Self> {
        let mut config = AudioConfiguration {
            sound_mode: None,
            sound_scene: None,
            loudness_adjustment: None,
        };

        for cap in capabilities.capabilities.iter() {
            match cap.name.as_str() {
                "sound-mode" => {
                    if let Some(value) = &cap.value {
                        config.sound_mode = Some(value.to_string());
                    }
                }
                "sound-scene" => {
                    if let Some(value) = &cap.value {
                        config.sound_scene = Some(value.to_string());
                    }
                }
                "program-reference-level" => {
                    if let Some(value) = &cap.value {
                        if let Ok(level) = value.parse::<i32>() {
                            config.loudness_adjustment = Some(level);
                        } else {
                            log::warn!("Invalid program-reference-level value: {}", value);
                        }
                    }
                }
                _ => {}
            }
        }

        if config.sound_scene.is_none() && config.sound_mode.is_none() && config.loudness_adjustment.is_none() {
            None
        } else {
            Some(config)
        }
    }
}

impl ToCapabilities for AudioConfiguration {
    fn to_capabilities(&self) -> Vec<config_xml::Capability> {
        let mut caps = Vec::new();

        if let Some(sound_mode) = &self.sound_mode {
            caps.push(config_xml::Capability {
                name: "sound-mode".to_string(),
                value: Some(sound_mode.to_string()),
            });
        }

        if let Some(sound_scene) = &self.sound_scene {
            caps.push(config_xml::Capability {
                name: "sound-scene".to_string(),
                value: Some(sound_scene.to_string()),
            });
        }

        if let Some(loudness_adjustment) = &self.loudness_adjustment {
            caps.push(config_xml::Capability {
                name: "program-reference-level".to_string(),
                value: Some(loudness_adjustment.to_string()),
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
    fn test_serialize_audio_config() {
        let config = AudioConfiguration {
            sound_mode: Some("movie".to_string()),
            sound_scene: Some("cinema".to_string()),
            loudness_adjustment: Some(-10),
        };
        let json_value = serde_json::to_value(&config).unwrap();
        assert_eq!(
            json_value,
            json!({
                "soundMode": "movie",
                "soundScene": "cinema",
                "loudnessAdjustment": -10
            })
        );

        let config = AudioConfiguration {
            sound_mode: None,
            sound_scene: Some("music".to_string()),
            loudness_adjustment: None,
        };
        let json_value = serde_json::to_value(&config).unwrap();
        assert_eq!(
            json_value,
            json!({
                "soundScene": "music"
            })
        );

        let config = AudioConfiguration {
            sound_mode: None,
            sound_scene: None,
            loudness_adjustment: None,
        };
        let json_value = serde_json::to_value(&config).unwrap();
        assert_eq!(json_value, json!({}));
    }

    #[test]
    fn test_deserialize_audio_config() {
        let json_value = json!({
            "soundMode": "game",
            "soundScene": "action",
            "loudnessAdjustment": 5
        });
        let config: AudioConfiguration = serde_json::from_value(json_value).unwrap();
        assert_eq!(config.sound_mode, Some("game".to_string()));
        assert_eq!(config.sound_scene, Some("action".to_string()));
        assert_eq!(config.loudness_adjustment, Some(5));

        let json_value = json!({
            "soundScene": "dialogue"
        });
        let config: AudioConfiguration = serde_json::from_value(json_value).unwrap();
        assert_eq!(config.sound_mode, None);
        assert_eq!(config.sound_scene, Some("dialogue".to_string()));
        assert_eq!(config.loudness_adjustment, None);

        let json_value = json!({});
        let config: AudioConfiguration = serde_json::from_value(json_value).unwrap();
        assert_eq!(config.sound_mode, None);
        assert_eq!(config.sound_scene, None);
        assert_eq!(config.loudness_adjustment, None);
    }

    #[test]
    fn test_audio_from_capabilities() {
        let caps = vec![
            config_xml::Capability {
                name: "sound-mode".to_string(),
                value: Some("movie".to_string()),
            },
            config_xml::Capability {
                name: "sound-scene".to_string(),
                value: Some("cinema".to_string()),
            },
            config_xml::Capability {
                name: "program-reference-level".to_string(),
                value: Some("-10".to_string()),
            },
        ];
        let capabilities = config_xml::Capabilities { capabilities: caps };

        let config = AudioConfiguration::from_capabilities(&capabilities);
        assert_eq!(
            config,
            Some(AudioConfiguration {
                sound_mode: Some("movie".to_string()),
                sound_scene: Some("cinema".to_string()),
                loudness_adjustment: Some(-10),
            })
        );

        let caps = vec![config_xml::Capability {
            name: "sound-scene".to_string(),
            value: Some("music".to_string()),
        }];
        let capabilities = config_xml::Capabilities { capabilities: caps };

        let config = AudioConfiguration::from_capabilities(&capabilities);
        assert_eq!(
            config,
            Some(AudioConfiguration {
                sound_mode: None,
                sound_scene: Some("music".to_string()),
                loudness_adjustment: None,
            })
        );

        let caps = vec![];
        let capabilities = config_xml::Capabilities { capabilities: caps };

        let config = AudioConfiguration::from_capabilities(&capabilities);
        assert_eq!(config, None);
    }

    #[test]
    fn test_audio_to_capabilities() {
        let config = AudioConfiguration {
            sound_mode: Some("game".to_string()),
            sound_scene: Some("action".to_string()),
            loudness_adjustment: Some(5),
        };
        let caps = config.to_capabilities();
        assert_eq!(caps.len(), 3);
        assert!(caps.contains(&config_xml::Capability {
            name: "sound-mode".to_string(),
            value: Some("game".to_string())
        }));
        assert!(caps.contains(&config_xml::Capability {
            name: "sound-scene".to_string(),
            value: Some("action".to_string())
        }));
        assert!(caps.contains(&config_xml::Capability {
            name: "program-reference-level".to_string(),
            value: Some("5".to_string())
        }));

        let config = AudioConfiguration {
            sound_mode: None,
            sound_scene: Some("dialogue".to_string()),
            loudness_adjustment: None,
        };
        let caps = config.to_capabilities();
        assert_eq!(caps.len(), 1);
        assert_eq!(
            caps[0],
            config_xml::Capability {
                name: "sound-scene".to_string(),
                value: Some("dialogue".to_string())
            }
        );

        let config = AudioConfiguration {
            sound_mode: None,
            sound_scene: None,
            loudness_adjustment: None,
        };
        let caps = config.to_capabilities();
        assert!(caps.is_empty());
    }
}
