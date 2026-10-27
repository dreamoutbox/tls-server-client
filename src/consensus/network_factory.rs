//! OpenRaft network factory for building peer network clients.

use std::net::SocketAddr;

use openraft::BasicNode;
use openraft::network::RaftNetworkFactory;

use crate::consensus::network_client::NetworkClient;
use crate::consensus::type_config::TypeConfig;
use crate::network::MtlsConnector;

/// Factory for creating [`NetworkClient`] instances for cluster peers.
#[derive(Clone)]
pub struct NetworkFactory {
    connector: MtlsConnector,
}

impl NetworkFactory {
    /// Creates a new network factory.
    ///
    /// # Arguments
    /// * `connector` - Mutual TLS connector used by clients to establish connections.
    ///
    /// # Example
    /// ```rust,no_run
    /// use std::sync::Arc;
    /// use rustls::ClientConfig;
    /// use tls_server_client::network::MtlsConnector;
    /// use tls_server_client::consensus::NetworkFactory;
    ///
    /// # fn doc_example(connector: MtlsConnector) {
    /// let factory = NetworkFactory::new(connector);
    /// # }
    /// ```
    pub fn new(connector: MtlsConnector) -> Self {
        Self { connector }
    }
}

impl RaftNetworkFactory<TypeConfig> for NetworkFactory {
    type Network = NetworkClient;

    async fn new_client(&mut self, target: u64, node: &BasicNode) -> Self::Network {
        let target_addr: SocketAddr = node.addr.parse().expect("valid peer address in BasicNode");
        NetworkClient::new(target, target_addr, self.connector.clone())
    }
}
