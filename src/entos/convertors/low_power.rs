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
use crate::entos::configs::LowPowerTerminateConfiguration;
use crate::entos::convertors::common::*;

impl FromCapabilities for LowPowerTerminateConfiguration {
    fn from_capabilities(capabilities: &config_xml::Capabilities) -> Option<Self> {
        if let Some(_) = capabilities.find("no-low-power-mode") {
            return Some(LowPowerTerminateConfiguration::LowPowerTerminate(true));
        }

        None
    }
}

impl ToCapabilities for LowPowerTerminateConfiguration {
    fn to_capabilities(&self) -> Vec<config_xml::Capability> {
        match self {
            LowPowerTerminateConfiguration::LowPowerTerminate(v) => {
                if *v == true {
                    Vec::from([config_xml::Capability {
                        name: "no-low-power-mode".to_string(),
                        value: None,
                    }])
                } else {
                    Vec::new()
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_serialize_low_power_config() {
        let config = LowPowerTerminateConfiguration::LowPowerTerminate(true);
        let json_value = serde_json::to_value(&config).unwrap();
        assert_eq!(json_value, json!(true));
    }

    #[test]
    fn test_deserialize_low_power_config() {
        let json_value = json!(true);
        let config: LowPowerTerminateConfiguration = serde_json::from_value(json_value).unwrap();
        assert_eq!(config, LowPowerTerminateConfiguration::LowPowerTerminate(true));

        let json_value = json!(false);
        let config: LowPowerTerminateConfiguration = serde_json::from_value(json_value).unwrap();
        assert_eq!(config, LowPowerTerminateConfiguration::LowPowerTerminate(false));
    }

    #[test]
    fn test_low_power_from_capabilities() {
        let caps = vec![
            config_xml::Capability {
                name: "no-low-power-mode".to_string(),
                value: None,
            },
            config_xml::Capability {
                name: "some-other-cap".to_string(),
                value: Some("value".to_string()),
            },
        ];
        let capabilities = config_xml::Capabilities { capabilities: caps };

        let config = LowPowerTerminateConfiguration::from_capabilities(&capabilities);
        assert_eq!(config, Some(LowPowerTerminateConfiguration::LowPowerTerminate(true)));

        let caps = vec![config_xml::Capability {
            name: "some-other-cap".to_string(),
            value: Some("value".to_string()),
        }];
        let capabilities = config_xml::Capabilities { capabilities: caps };

        let config = LowPowerTerminateConfiguration::from_capabilities(&capabilities);
        assert_eq!(config, None);
    }

    #[test]
    fn test_low_power_to_capabilities() {
        let config = LowPowerTerminateConfiguration::LowPowerTerminate(true);
        let caps = config.to_capabilities();
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].name, "no-low-power-mode");
        assert_eq!(caps[0].value, None);

        let config = LowPowerTerminateConfiguration::LowPowerTerminate(false);
        let caps = config.to_capabilities();
        assert_eq!(caps.len(), 0);
    }
}
