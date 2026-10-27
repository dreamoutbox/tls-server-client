#[cfg(test)]
mod tests {
    use crate::config::node_config::NodeConfig;
    use crate::config::peer_config::PeerConfig;
    use std::path::Path;

    #[test]
    fn test_valid_node_config_validation() {
        let config = NodeConfig {
            self_id: 1,
            listen_addr: "127.0.0.1:8443".parse().expect("valid address"),
            cert_path: "certs/node1.crt".into(),
            key_path: "certs/node1.key".into(),
            ca_path: "certs/ca.crt".into(),
            peers: vec![PeerConfig {
                id: 2,
                addr: "127.0.0.1:8444".parse().expect("valid address"),
            }],
        };

        assert!(config.validate(Path::new("test.toml")).is_ok());
    }

    #[test]
    fn test_invalid_node_config_zero_id() {
        let config = NodeConfig {
            self_id: 0,
            listen_addr: "127.0.0.1:8443".parse().expect("valid address"),
            cert_path: "certs/node1.crt".into(),
            key_path: "certs/node1.key".into(),
            ca_path: "certs/ca.crt".into(),
            peers: vec![],
        };

        assert!(config.validate(Path::new("test.toml")).is_err());
    }

    #[test]
    fn test_invalid_node_config_self_in_peers() {
        let config = NodeConfig {
            self_id: 1,
            listen_addr: "127.0.0.1:8443".parse().expect("valid address"),
            cert_path: "certs/node1.crt".into(),
            key_path: "certs/node1.key".into(),
            ca_path: "certs/ca.crt".into(),
            peers: vec![PeerConfig {
                id: 1,
                addr: "127.0.0.1:8443".parse().expect("valid address"),
            }],
        };

        assert!(config.validate(Path::new("test.toml")).is_err());
    }
}
