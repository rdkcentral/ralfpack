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
use crate::entos::configs::AgeRatingConfiguration;
use crate::entos::convertors::common::*;

impl FromCapabilities for AgeRatingConfiguration {
    fn from_capabilities(capabilities: &config_xml::Capabilities) -> Option<Self> {
        if let Some(cap) = capabilities.find("age-rating") {
            if let Some(value) = &cap.value {
                let value = value.trim();
                if let Ok(age) = value.parse::<i32>() {
                    return Some(AgeRatingConfiguration::AgeRating(age));
                } else {
                    log::warn!("'age-rating' capability value is not a valid integer: {}", value);
                }
            } else {
                log::warn!("Missing 'age-rating' capability value");
            }
        }

        None
    }
}

impl ToCapabilities for AgeRatingConfiguration {
    fn to_capabilities(&self) -> Vec<config_xml::Capability> {
        match self {
            AgeRatingConfiguration::AgeRating(age) => Vec::from([config_xml::Capability {
                name: "age-rating".to_string(),
                value: Some(age.to_string()),
            }]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_serialize_age_rating_config() {
        let config = AgeRatingConfiguration::AgeRating(15);
        let json_value = serde_json::to_value(&config).unwrap();
        assert_eq!(json_value, json!(15));
    }

    #[test]
    fn test_deserialize_age_rating_config() {
        let json_value = json!(18);
        let config: AgeRatingConfiguration = serde_json::from_value(json_value).unwrap();
        assert_eq!(config, AgeRatingConfiguration::AgeRating(18));
    }

    #[test]
    fn test_age_rating_from_capabilities() {
        let caps = vec![
            config_xml::Capability {
                name: "age-rating".to_string(),
                value: Some(" 12 ".to_string()),
            },
            config_xml::Capability {
                name: "some-other-cap".to_string(),
                value: Some("value".to_string()),
            },
        ];
        let capabilities = config_xml::Capabilities { capabilities: caps };

        let config = AgeRatingConfiguration::from_capabilities(&capabilities);
        assert_eq!(config, Some(AgeRatingConfiguration::AgeRating(12)));

        let caps = vec![config_xml::Capability {
            name: "some-other-cap".to_string(),
            value: Some("value".to_string()),
        }];
        let capabilities = config_xml::Capabilities { capabilities: caps };

        let config = AgeRatingConfiguration::from_capabilities(&capabilities);
        assert_eq!(config, None);
    }

    #[test]
    fn test_age_rating_to_capabilities() {
        let config = AgeRatingConfiguration::AgeRating(13);
        let caps = config.to_capabilities();
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].name, "age-rating");
        assert_eq!(caps[0].value, Some("13".to_string()));
    }
}
