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
use crate::entos::configs::AgePolicyConfiguration;
use crate::entos::convertors::common::*;

impl FromCapabilities for AgePolicyConfiguration {
    fn from_capabilities(capabilities: &config_xml::Capabilities) -> Option<Self> {
        if let Some(cap) = capabilities.find("age-policy") {
            if let Some(value) = &cap.value {
                return Some(AgePolicyConfiguration::AgePolicy(value.to_string()));
            }
        }

        None
    }
}

impl ToCapabilities for AgePolicyConfiguration {
    fn to_capabilities(&self) -> Vec<config_xml::Capability> {
        match self {
            AgePolicyConfiguration::AgePolicy(s) => Vec::from([config_xml::Capability {
                name: "age-policy".to_string(),
                value: Some(s.to_string()),
            }]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_serialize_age_policy_config() {
        let config = AgePolicyConfiguration::AgePolicy(String::from("foo"));
        let json_value = serde_json::to_value(&config).unwrap();
        assert_eq!(json_value, json!("foo"));
    }

    #[test]
    fn test_deserialize_age_policy_config() {
        let json_value = json!("bar");
        let config: AgePolicyConfiguration = serde_json::from_value(json_value).unwrap();
        assert_eq!(config, AgePolicyConfiguration::AgePolicy("bar".to_string()));
    }

    #[test]
    fn test_age_policy_from_capabilities() {
        let caps = vec![
            config_xml::Capability {
                name: "age-policy".to_string(),
                value: Some("foo".to_string()),
            },
            config_xml::Capability {
                name: "some-other-cap".to_string(),
                value: Some("value".to_string()),
            },
        ];
        let capabilities = config_xml::Capabilities { capabilities: caps };

        let config = AgePolicyConfiguration::from_capabilities(&capabilities);
        assert_eq!(config, Some(AgePolicyConfiguration::AgePolicy("foo".to_string())));

        let caps = vec![config_xml::Capability {
            name: "some-other-cap".to_string(),
            value: Some("value".to_string()),
        }];
        let capabilities = config_xml::Capabilities { capabilities: caps };

        let config = AgePolicyConfiguration::from_capabilities(&capabilities);
        assert_eq!(config, None);
    }

    #[test]
    fn test_age_policy_to_capabilities() {
        let config = AgePolicyConfiguration::AgePolicy("123:moo:terry".to_string());
        let caps = config.to_capabilities();
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].name, "age-policy");
        assert_eq!(caps[0].value, Some("123:moo:terry".to_string()));
    }
}
