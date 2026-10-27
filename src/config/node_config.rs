//! Local node configuration.

use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::config::error::ConfigError;
use crate::config::peer_config::PeerConfig;

/// Configuration defining the local node's identity, certificates, and peer connections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeConfig {
    /// Unique identifier for this local node.
    pub self_id: u64,
    /// Local address to bind the listener to.
    pub listen_addr: SocketAddr,
    /// Path to the node's TLS certificate.
    pub cert_path: PathBuf,
    /// Path to the node's TLS private key.
    pub key_path: PathBuf,
    /// Path to the shared CA certificate for validating peers.
    pub ca_path: PathBuf,
    /// List of peer nodes in the cluster.
    #[serde(default)]
    pub peers: Vec<PeerConfig>,
}

impl NodeConfig {
    /// Reads, deserializes, and validates a [`NodeConfig`] from a TOML file on disk.
    ///
    /// # Arguments
    /// * `path` - Filesystem path pointing to the TOML configuration file.
    ///
    /// # Errors
    /// Returns [`ConfigError::Io`] if file read fails, [`ConfigError::Deserialize`]
    /// if TOML parsing fails, or [`ConfigError::Validation`] if validation rules are violated.
    ///
    /// # Example
    /// ```rust,no_run
    /// use tls_server_client::config::NodeConfig;
    ///
    /// let config = NodeConfig::from_file("config/node1.toml").unwrap();
    /// println!("Node ID: {}", config.self_id);
    /// ```
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path_ref = path.as_ref();
        let content = fs::read_to_string(path_ref).map_err(|source| ConfigError::Io {
            path: path_ref.to_path_buf(),
            source,
        })?;

        let config: Self = toml::from_str(&content).map_err(|source| ConfigError::Deserialize {
            path: path_ref.to_path_buf(),
            source,
        })?;

        config.validate(path_ref)?;
        Ok(config)
    }

    /// Validates the configuration consistency.
    ///
    /// # Arguments
    /// * `path` - Source path of the configuration for error reporting.
    ///
    /// # Errors
    /// Returns [`ConfigError::Validation`] if node ID is zero or if a peer has the same ID as self.
    ///
    /// # Example
    /// ```rust
    /// use std::path::Path;
    /// use tls_server_client::config::NodeConfig;
    ///
    /// let config = NodeConfig {
    ///     self_id: 1,
    ///     listen_addr: "127.0.0.1:8443".parse().unwrap(),
    ///     cert_path: "certs/node1.crt".into(),
    ///     key_path: "certs/node1.key".into(),
    ///     ca_path: "certs/ca.crt".into(),
    ///     peers: vec![],
    /// };
    /// assert!(config.validate(Path::new("test.toml")).is_ok());
    /// ```
    pub fn validate(&self, path: &Path) -> Result<(), ConfigError> {
        if self.self_id == 0 {
            return Err(ConfigError::Validation {
                path: path.to_path_buf(),
                message: "self_id must be greater than 0".to_string(),
            });
        }

        for peer in &self.peers {
            if peer.id == self.self_id {
                return Err(ConfigError::Validation {
                    path: path.to_path_buf(),
                    message: format!("peer list cannot contain self_id ({})", self.self_id),
                });
            }
        }

        Ok(())
    }
}
