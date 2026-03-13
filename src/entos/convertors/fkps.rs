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
use crate::entos::configs::FkpsConfiguration;
use crate::entos::convertors::common::*;

impl FromCapabilities for FkpsConfiguration {
    fn from_capabilities(capabilities: &config_xml::Capabilities) -> Option<Self> {
        if let Some(cap) = capabilities.find("fkps") {
            if let Some(value) = &cap.value {
                let files = split_to_vec(value, ',');
                if files.is_empty() {
                    log::warn!("'fkps' capability value is empty");
                } else {
                    return Some(FkpsConfiguration { files });
                }
            } else {
                log::warn!("Missing 'fkps' capability value");
            }
        }

        None
    }
}

impl ToCapabilities for FkpsConfiguration {
    fn to_capabilities(&self) -> Vec<config_xml::Capability> {
        let mut caps = Vec::new();

        if !self.files.is_empty() {
            caps.push(config_xml::Capability {
                name: "fkps".to_string(),
                value: Some(vec_to_string(&self.files, ',')),
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
    fn test_fpks_config_serialization() {
        let config = FkpsConfiguration {
            files: vec!["file1.fkps".to_string(), "file2.fkps".to_string()],
        };
        let serialized = serde_json::to_string(&config).unwrap();
        let expected = json!({
            "files": ["file1.fkps", "file2.fkps"]
        });
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&serialized).unwrap(),
            expected
        );

        let deserialized: FkpsConfiguration = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.files, config.files);
    }

    #[test]
    fn test_fkps_config_deserialization() {
        let json_data = json!({
            "files": ["file1.fkps", "file2.fkps"]
        });
        let deserialized: FkpsConfiguration = serde_json::from_value(json_data).unwrap();
        assert_eq!(
            deserialized.files,
            vec!["file1.fkps".to_string(), "file2.fkps".to_string()]
        );

        let empty_json = json!({
            "files": []
        });
        let deserialized_empty: FkpsConfiguration = serde_json::from_value(empty_json).unwrap();
        assert!(deserialized_empty.files.is_empty());

        let empty_json = json!({});
        let deserialized_empty: FkpsConfiguration = serde_json::from_value(empty_json).unwrap();
        assert!(deserialized_empty.files.is_empty());
    }

    #[test]
    fn test_fkps_config_to_capabilities() {
        let config = FkpsConfiguration {
            files: vec!["file1.fkps".to_string(), "file2.fkps".to_string()],
        };
        let caps = config.to_capabilities();
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].name, "fkps");
        assert_eq!(caps[0].value.as_deref(), Some("file1.fkps,file2.fkps"));

        let empty_config = FkpsConfiguration { files: vec![] };
        let empty_caps = empty_config.to_capabilities();
        assert!(empty_caps.is_empty());
    }

    #[test]
    fn test_fkps_config_from_capabilities() {
        let caps = config_xml::Capabilities {
            capabilities: vec![config_xml::Capability {
                name: "fkps".to_string(),
                value: Some("file1.fkps,file2.fkps".to_string()),
            }],
        };
        let config = FkpsConfiguration::from_capabilities(&caps).unwrap();
        assert_eq!(config.files, vec!["file1.fkps".to_string(), "file2.fkps".to_string()]);

        let caps_empty = config_xml::Capabilities {
            capabilities: vec![config_xml::Capability {
                name: "fkps".to_string(),
                value: Some("".to_string()),
            }],
        };
        let config_empty = FkpsConfiguration::from_capabilities(&caps_empty);
        assert!(config_empty.is_none());

        let caps_missing = config_xml::Capabilities {
            capabilities: vec![config_xml::Capability {
                name: "fkps".to_string(),
                value: None,
            }],
        };
        let config_missing = FkpsConfiguration::from_capabilities(&caps_missing);
        assert!(config_missing.is_none());
    }
}
