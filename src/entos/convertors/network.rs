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
use crate::package_config::{NetworkConfiguration, NetworkProtocol, NetworkServiceConfiguration, NetworkType};

/// Processes a single network widget capability, which is just a list of strings containing
/// port numbers optionally prefixed with a protocol (tcp: or udp:).
fn parse_network_configs(network_type: NetworkType, value: &Option<String>) -> Vec<NetworkServiceConfiguration> {
    if value.is_none() {
        return Vec::new();
    }

    let ports = split_to_vec(value.as_ref().unwrap(), ',');
    if ports.is_empty() {
        return Vec::new();
    }

    let mut configs: Vec<NetworkServiceConfiguration> = Vec::new();

    for port in ports.iter() {
        let mut protocol = NetworkProtocol::Tcp;
        let port_lc = port.to_lowercase();
        let mut port_str = port_lc.as_str();
        if port_str.starts_with("tcp:") {
            protocol = NetworkProtocol::Tcp;
            port_str = &port_str[4..];
        } else if port_str.starts_with("udp:") {
            protocol = NetworkProtocol::Udp;
            port_str = &port_str[4..];
        }

        // Try to parse the port number
        if let Ok(port_num) = port_str.parse::<u16>() {
            let service_config = NetworkServiceConfiguration {
                network_type: network_type.clone(),
                protocol: protocol,
                port: port_num,
                name: None,
            };

            configs.push(service_config);
        } else {
            log::warn!("Invalid port number in network capability: {}", port);
        }
    }

    configs
}

impl FromCapabilities for NetworkConfiguration {
    fn from_capabilities(capabilities: &config_xml::Capabilities) -> Option<Self> {
        let mut network_configs = NetworkConfiguration::new();

        for cap in capabilities.capabilities.iter() {
            match cap.name.as_str() {
                "hole-punch" => {
                    let mut configs = parse_network_configs(NetworkType::Public, &cap.value);
                    network_configs.append(&mut configs);
                }
                "local-socket-server" => {
                    let mut configs = parse_network_configs(NetworkType::Exported, &cap.value);
                    network_configs.append(&mut configs);
                }
                "local-socket-client" => {
                    let mut configs = parse_network_configs(NetworkType::Imported, &cap.value);
                    network_configs.append(&mut configs);
                }
                _ => {}
            }
        }

        Some(network_configs)
    }
}

impl ToCapabilities for NetworkConfiguration {
    fn to_capabilities(&self) -> Vec<config_xml::Capability> {
        let mut caps = Vec::new();

        let mut public_ports: Vec<String> = Vec::new();
        let mut exported_ports: Vec<String> = Vec::new();
        let mut imported_ports: Vec<String> = Vec::new();

        for service in self.iter() {
            let port_str = match service.protocol {
                NetworkProtocol::Tcp => format!("{}", service.port),
                NetworkProtocol::Udp => format!("udp:{}", service.port),
            };

            match service.network_type {
                NetworkType::Public => public_ports.push(port_str),
                NetworkType::Exported => exported_ports.push(port_str),
                NetworkType::Imported => imported_ports.push(port_str),
            }
        }

        if !public_ports.is_empty() {
            caps.push(config_xml::Capability {
                name: "hole-punch".to_string(),
                value: Some(public_ports.join(",")),
            });
        }

        if !exported_ports.is_empty() {
            caps.push(config_xml::Capability {
                name: "local-socket-server".to_string(),
                value: Some(exported_ports.join(",")),
            });
        }

        if !imported_ports.is_empty() {
            caps.push(config_xml::Capability {
                name: "local-socket-client".to_string(),
                value: Some(imported_ports.join(",")),
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
    fn test_network_configuration_to_capabilities() {
        let json_snippet = json!([
          {
            "name": "netflix-mdx",
            "port": 8009,
            "protocol": "tcp",
            "type": "public"
          },
          {
            "name": "foo",
            "port": 2468,
            "protocol": "udp",
            "type": "public"
          },
          {
            "name": "com.example.myapp.service",
            "port": 1234,
            "protocol": "udp",
            "type": "exported"
          },
          {
            "name": "com.example.someotherapp.service",
            "port": 4567,
            "protocol": "tcp",
            "type": "imported"
          }
        ]);

        let config = serde_json::from_value::<NetworkConfiguration>(json_snippet).unwrap();
        assert_eq!(config.len(), 4);
        assert_eq!(config[0].name, Some("netflix-mdx".to_string()));
        assert_eq!(config[0].port, 8009);
        assert_eq!(config[0].protocol, NetworkProtocol::Tcp);
        assert_eq!(config[0].network_type, NetworkType::Public);
        assert_eq!(config[1].name, Some("foo".to_string()));
        assert_eq!(config[1].port, 2468);
        assert_eq!(config[1].protocol, NetworkProtocol::Udp);
        assert_eq!(config[1].network_type, NetworkType::Public);
        assert_eq!(config[2].name, Some("com.example.myapp.service".to_string()));
        assert_eq!(config[2].port, 1234);
        assert_eq!(config[2].protocol, NetworkProtocol::Udp);
        assert_eq!(config[2].network_type, NetworkType::Exported);
        assert_eq!(config[3].name, Some("com.example.someotherapp.service".to_string()));
        assert_eq!(config[3].port, 4567);
        assert_eq!(config[3].protocol, NetworkProtocol::Tcp);
        assert_eq!(config[3].network_type, NetworkType::Imported);

        let capabilities = config.to_capabilities();
        assert_eq!(capabilities.len(), 3);
        assert_eq!(capabilities[0].name, "hole-punch");
        assert_eq!(capabilities[0].value, Some("8009,udp:2468".to_string()));
        assert_eq!(capabilities[1].name, "local-socket-server");
        assert_eq!(capabilities[1].value, Some("udp:1234".to_string()));
        assert_eq!(capabilities[2].name, "local-socket-client");
        assert_eq!(capabilities[2].value, Some("4567".to_string()));
    }

    #[test]
    fn test_network_configuration_from_capabilities() {
        let capabilities = config_xml::Capabilities {
            capabilities: vec![
                config_xml::Capability {
                    name: "hole-punch".to_string(),
                    value: Some("tcp:8009, udp:123,  1234".to_string()),
                },
                config_xml::Capability {
                    name: "unrelated-capability".to_string(),
                    value: Some("ignored".to_string()),
                },
            ],
        };

        let config = NetworkConfiguration::from_capabilities(&capabilities).unwrap();
        assert_eq!(config.len(), 3);
        assert_eq!(config[0].name, None);
        assert_eq!(config[0].port, 8009);
        assert_eq!(config[0].protocol, NetworkProtocol::Tcp);
        assert_eq!(config[0].network_type, NetworkType::Public);
        assert_eq!(config[1].name, None);
        assert_eq!(config[1].port, 123);
        assert_eq!(config[1].protocol, NetworkProtocol::Udp);
        assert_eq!(config[1].network_type, NetworkType::Public);
        assert_eq!(config[2].name, None);
        assert_eq!(config[2].port, 1234);
        assert_eq!(config[2].protocol, NetworkProtocol::Tcp);
        assert_eq!(config[2].network_type, NetworkType::Public);
    }
}
