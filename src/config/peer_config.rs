//! Peer node configuration.

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// Configuration entry representing a remote peer in the cluster.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerConfig {
    /// Unique identifier for the peer node.
    pub id: u64,
    /// Network address the peer is reachable at.
    pub addr: SocketAddr,
}

impl PeerConfig {
    /// Creates a new peer configuration instance.
    ///
    /// # Arguments
    /// * `id` - Unique identifier for the peer node.
    /// * `addr` - Network socket address for the peer.
    ///
    /// # Example
    /// ```rust
    /// use std::net::SocketAddr;
    /// use tls_server_client::config::PeerConfig;
    ///
    /// let peer = PeerConfig::new(2, "127.0.0.1:8444".parse().unwrap());
    /// assert_eq!(peer.id, 2);
    /// ```
    #[must_use]
    pub fn new(id: u64, addr: SocketAddr) -> Self {
        Self { id, addr }
    }
}
