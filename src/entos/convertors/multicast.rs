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
use crate::entos::configs::{MulticastClientSocket, MulticastConfiguration, MulticastServerSocket};
use crate::entos::convertors::common::*;

impl FromCapabilities for MulticastConfiguration {
    fn from_capabilities(capabilities: &config_xml::Capabilities) -> Option<Self> {
        let mut config = MulticastConfiguration {
            forwarding: Vec::new(),
            server_sockets: Vec::new(),
            client_sockets: Vec::new(),
        };

        for cap in capabilities.capabilities.iter() {
            match cap.name.as_str() {
                "multicast-client-socket" => {
                    if let Some(value) = &cap.value {
                        let names = split_to_vec(value, ',');
                        for name in names.iter() {
                            if name.is_empty() {
                                log::warn!("Empty socket name in multicast-client-socket value");
                            } else {
                                config
                                    .client_sockets
                                    .push(MulticastClientSocket { name: name.to_string() });
                            }
                        }
                    }
                }
                "multicast-server-socket" => {
                    if let Some(value) = &cap.value {
                        let sockets = split_to_vec(value, ',');
                        for socket in sockets {
                            // The content of the 'multicast-server-socket' capability is <NAME>:<IP>:<PORT>
                            let parts: Vec<&str> = socket.split(':').collect();
                            if parts.len() != 3 {
                                log::warn!("Invalid multicast-server-socket value: {}", socket);
                                continue;
                            }

                            let socket_name = parts[0].to_string();
                            let ip_str = parts[1].to_string();
                            let port_str = parts[2];

                            if ip_str.parse::<std::net::IpAddr>().is_err() {
                                log::warn!("Invalid IP address in multicast-server-socket value: {}", socket);
                                continue;
                            }
                            if let Ok(port) = port_str.parse::<u16>() {
                                config.server_sockets.push(MulticastServerSocket {
                                    name: socket_name,
                                    address: ip_str,
                                    port,
                                });
                            } else {
                                log::warn!("Invalid entry in multicast-server-socket value: {}", socket);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        if config.forwarding.is_empty() && config.server_sockets.is_empty() && config.client_sockets.is_empty() {
            None
        } else {
            Some(config)
        }
    }
}

impl ToCapabilities for MulticastConfiguration {
    fn to_capabilities(&self) -> Vec<config_xml::Capability> {
        let mut caps = Vec::new();

        if !self.client_sockets.is_empty() {
            let names: Vec<String> = self.client_sockets.iter().map(|s| s.name.clone()).collect();
            let value = names.join(",");
            caps.push(config_xml::Capability {
                name: "multicast-client-socket".to_string(),
                value: Some(value),
            });
        }

        if !self.server_sockets.is_empty() {
            let sockets: Vec<String> = self
                .server_sockets
                .iter()
                .map(|s| format!("{}:{}:{}", s.name, s.address, s.port))
                .collect();
            let value = sockets.join(",");
            caps.push(config_xml::Capability {
                name: "multicast-server-socket".to_string(),
                value: Some(value),
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
    fn test_multicast_config_from_capabilities() {
        // Test with empty capabilities
        let empty_caps = config_xml::Capabilities {
            capabilities: Vec::new(),
        };
        assert!(MulticastConfiguration::from_capabilities(&empty_caps).is_none());

        // Test with client sockets only
        let client_caps = config_xml::Capabilities {
            capabilities: vec![config_xml::Capability {
                name: "multicast-client-socket".to_string(),
                value: Some("socket1,socket2,socket3".to_string()),
            }],
        };
        let config = MulticastConfiguration::from_capabilities(&client_caps).unwrap();
        assert_eq!(config.client_sockets.len(), 3);
        assert_eq!(config.client_sockets[0].name, "socket1");
        assert_eq!(config.client_sockets[1].name, "socket2");
        assert_eq!(config.client_sockets[2].name, "socket3");
        assert!(config.server_sockets.is_empty());
        assert!(config.forwarding.is_empty());

        // Test with server sockets only
        let server_caps = config_xml::Capabilities {
            capabilities: vec![config_xml::Capability {
                name: "multicast-server-socket".to_string(),
                value: Some("srv1:224.1.1.1:5000,srv2:239.255.255.250:1900".to_string()),
            }],
        };
        let config = MulticastConfiguration::from_capabilities(&server_caps).unwrap();
        assert_eq!(config.server_sockets.len(), 2);
        assert_eq!(config.server_sockets[0].name, "srv1");
        assert_eq!(config.server_sockets[0].address, "224.1.1.1");
        assert_eq!(config.server_sockets[0].port, 5000);
        assert_eq!(config.server_sockets[1].name, "srv2");
        assert_eq!(config.server_sockets[1].address, "239.255.255.250");
        assert_eq!(config.server_sockets[1].port, 1900);
        assert!(config.client_sockets.is_empty());
        assert!(config.forwarding.is_empty());

        // Test with mixed capabilities
        let mixed_caps = config_xml::Capabilities {
            capabilities: vec![
                config_xml::Capability {
                    name: "multicast-client-socket".to_string(),
                    value: Some("client1,client2".to_string()),
                },
                config_xml::Capability {
                    name: "multicast-server-socket".to_string(),
                    value: Some("server1:224.0.0.1:8080".to_string()),
                },
                config_xml::Capability {
                    name: "unrelated-capability".to_string(),
                    value: Some("ignored".to_string()),
                },
            ],
        };
        let config = MulticastConfiguration::from_capabilities(&mixed_caps).unwrap();
        assert_eq!(config.client_sockets.len(), 2);
        assert_eq!(config.server_sockets.len(), 1);
        assert!(config.forwarding.is_empty());

        // Test json serialization
        let json_value = serde_json::to_value(config).unwrap();
        assert_eq!(
            json_value,
            json!({
                "serverSockets": [
                    {
                        "port": 8080,
                        "address": "224.0.0.1",
                        "name": "server1"
                    }
                ],
                "clientSockets": [
                    {"name": "client1"},
                    {"name": "client2"}
                ]
            })
        );

        // Test with invalid server socket format
        let invalid_caps = config_xml::Capabilities {
            capabilities: vec![config_xml::Capability {
                name: "multicast-server-socket".to_string(),
                value: Some("invalid:format".to_string()),
            }],
        };
        assert!(MulticastConfiguration::from_capabilities(&invalid_caps).is_none());

        // Test with empty values
        let empty_value_caps = config_xml::Capabilities {
            capabilities: vec![config_xml::Capability {
                name: "multicast-client-socket".to_string(),
                value: Some(",,".to_string()),
            }],
        };
        assert!(MulticastConfiguration::from_capabilities(&empty_value_caps).is_none());
    }

    #[test]
    fn test_multicast_config_to_capabilities() {
        let json_snippet = json!({
            "forwarding": [
                {
                    "address": "224.1.1.2",
                    "port": 1234,
                }
            ],
            "serverSockets": [
                {
                    "port": 5000,
                    "address": "224.1.1.1",
                    "name": "test_server"
                },
                {
                    "port": 8900,
                    "address": "224.1.14.3",
                    "name": "test_server_2"
                }
            ],
            "clientSockets": [
                {
                    "name": "test_client"
                },
                {
                    "name": "test_client2"
                }
            ]
        });

        let config: MulticastConfiguration = serde_json::from_value(json_snippet).unwrap();
        assert_eq!(config.client_sockets.len(), 2);
        assert_eq!(config.server_sockets.len(), 2);
        assert_eq!(config.forwarding.len(), 1);
        assert_eq!(config.client_sockets[0].name, "test_client");
        assert_eq!(config.client_sockets[1].name, "test_client2");
        assert_eq!(config.server_sockets[0].name, "test_server");
        assert_eq!(config.server_sockets[0].address, "224.1.1.1");
        assert_eq!(config.server_sockets[0].port, 5000);
        assert_eq!(config.server_sockets[1].name, "test_server_2");
        assert_eq!(config.server_sockets[1].address, "224.1.14.3");
        assert_eq!(config.server_sockets[1].port, 8900);

        let capabilities = config.to_capabilities();
        assert_eq!(capabilities.len(), 2);
        assert_eq!(capabilities[0].name, "multicast-client-socket");
        assert_eq!(capabilities[0].value.as_ref().unwrap(), "test_client,test_client2");
        assert_eq!(capabilities[1].name, "multicast-server-socket");
        assert_eq!(
            capabilities[1].value.as_ref().unwrap(),
            "test_server:224.1.1.1:5000,test_server_2:224.1.14.3:8900"
        );
    }
}
