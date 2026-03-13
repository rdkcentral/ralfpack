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

use crate::package_config::MemoryConfiguration;

impl FromCapabilities for MemoryConfiguration {
    fn from_capabilities(capabilities: &config_xml::Capabilities) -> Option<Self> {
        let mut memory_quotas = MemoryConfiguration {
            sys_memory: None,
            gpu_memory: None,
        };

        for cap in capabilities.capabilities.iter() {
            if cap.name == "sys-memory-limit" && cap.value.is_some() {
                memory_quotas.sys_memory = cap.value.clone();
            } else if cap.name == "gpu-memory-limit" && cap.value.is_some() {
                memory_quotas.gpu_memory = cap.value.clone();
            }
        }

        if memory_quotas.sys_memory.is_none() && memory_quotas.gpu_memory.is_none() {
            // No quotas set, return None
            return None;
        }

        Some(memory_quotas)
    }
}

impl ToCapabilities for MemoryConfiguration {
    fn to_capabilities(&self) -> Vec<config_xml::Capability> {
        let mut caps = Vec::new();

        if let Some(sys_memory) = &self.sys_memory {
            caps.push(config_xml::Capability {
                name: "sys-memory-limit".to_string(),
                value: Some(sys_memory.clone()),
            });
        }

        if let Some(gpu_memory) = &self.gpu_memory {
            caps.push(config_xml::Capability {
                name: "gpu-memory-limit".to_string(),
                value: Some(gpu_memory.clone()),
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
    fn test_memory_configuration_to_capabilities() {
        let json_snippet = json!({
            "system": "256M",
            "gpu": "256K"
        });

        let config = serde_json::from_value::<MemoryConfiguration>(json_snippet).unwrap();
        assert_eq!(config.gpu_memory, Some("256K".to_string()));
        assert_eq!(config.sys_memory, Some("256M".to_string()));

        let capabilities = config.to_capabilities();
        assert_eq!(capabilities.len(), 2);

        let gpu_memory_cap = capabilities.iter().find(|c| c.name == "gpu-memory-limit").unwrap();
        assert_eq!(gpu_memory_cap.name, "gpu-memory-limit");
        assert_eq!(gpu_memory_cap.value.as_ref().unwrap(), "256K");

        let sys_memory_cap = capabilities.iter().find(|c| c.name == "sys-memory-limit").unwrap();
        assert_eq!(sys_memory_cap.name, "sys-memory-limit");
        assert_eq!(sys_memory_cap.value.as_ref().unwrap(), "256M");
    }

    #[test]
    fn test_memory_configuration_from_capabilities() {
        let capabilities = config_xml::Capabilities {
            capabilities: vec![
                config_xml::Capability {
                    name: "gpu-memory-limit".to_string(),
                    value: Some("512K".to_string()),
                },
                config_xml::Capability {
                    name: "sys-memory-limit".to_string(),
                    value: Some("512m".to_string()),
                },
                config_xml::Capability {
                    name: "unrelated-capability".to_string(),
                    value: Some("ignored".to_string()),
                },
            ],
        };

        let config = MemoryConfiguration::from_capabilities(&capabilities).unwrap();
        assert_eq!(config.gpu_memory, Some("512K".to_string()));
        assert_eq!(config.sys_memory, Some("512m".to_string()));
    }
}
