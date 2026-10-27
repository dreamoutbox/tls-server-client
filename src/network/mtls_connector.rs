//! Mutual TLS connector for dialing remote cluster peers.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use rustls::ClientConfig;
use rustls::pki_types::ServerName;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;
use tracing::{debug, error, info, warn};

use crate::network::error::NetworkError;
use crate::network::framed_stream::FramedStream;

/// Connector responsible for establishing authenticated mTLS connections to peers.
#[derive(Clone)]
pub struct MtlsConnector {
    connector: TlsConnector,
}

impl MtlsConnector {
    /// Creates a new connector using the provided mutual TLS client configuration.
    ///
    /// # Arguments
    /// * `config` - Shared client configuration containing local certs and trusted CA roots.
    ///
    /// # Example
    /// ```rust,no_run
    /// use std::sync::Arc;
    /// use rustls::ClientConfig;
    /// use tls_server_client::network::MtlsConnector;
    ///
    /// # fn doc_example(config: Arc<ClientConfig>) {
    /// let connector = MtlsConnector::new(config);
    /// # }
    /// ```
    pub fn new(config: Arc<ClientConfig>) -> Self {
        Self {
            connector: TlsConnector::from(config),
        }
    }

    /// Attempts a single mTLS connection to a peer node.
    ///
    /// # Arguments
    /// * `peer_id` - Identifier of the target peer (used to verify the peer's certificate CN/SAN).
    /// * `addr` - Network socket address of the target peer.
    ///
    /// # Errors
    /// Returns [`NetworkError::InvalidServerName`] if the peer identity cannot be parsed,
    /// [`NetworkError::Io`] if TCP connection fails, or [`NetworkError::HandshakeFailed`]
    /// if TLS mutual authentication fails.
    pub async fn connect(
        &self,
        peer_id: u64,
        addr: SocketAddr,
    ) -> Result<FramedStream<TlsStream<TcpStream>>, NetworkError> {
        let server_name_str = format!("node{}", peer_id);
        let server_name = ServerName::try_from(server_name_str.clone())
            .map_err(|_| NetworkError::InvalidServerName(server_name_str))?;

        let tcp_stream = TcpStream::connect(addr).await?;
        let tls_stream = self
            .connector
            .connect(server_name, tcp_stream)
            .await
            .map_err(|err| NetworkError::HandshakeFailed {
                peer_id,
                reason: err.to_string(),
            })?;

        Ok(FramedStream::new(tls_stream))
    }

    /// Connects to a peer with exponential backoff until successful or timeout reached.
    ///
    /// TCP connection failures (e.g. peer not yet up) are logged at `debug` level,
    /// while TLS handshake / certificate validation failures are logged as `warn`/`error`.
    ///
    /// # Arguments
    /// * `peer_id` - Target peer identifier.
    /// * `addr` - Target peer socket address.
    /// * `max_attempts` - Maximum number of connection attempts before aborting (0 for unlimited).
    /// * `base_delay` - Initial delay between connection retries.
    /// * `max_delay` - Upper bound on backoff sleep duration.
    ///
    /// # Errors
    /// Returns the last [`NetworkError`] if all attempts fail.
    pub async fn connect_with_backoff(
        &self,
        peer_id: u64,
        addr: SocketAddr,
        max_attempts: usize,
        base_delay: Duration,
        max_delay: Duration,
    ) -> Result<FramedStream<TlsStream<TcpStream>>, NetworkError> {
        let mut delay = base_delay;
        let mut attempt = 0usize;

        loop {
            attempt += 1;
            match self.connect(peer_id, addr).await {
                Ok(stream) => {
                    info!(peer_id, %addr, attempt, "mTLS connection to peer established (peer is up)");
                    return Ok(stream);
                }
                Err(NetworkError::Io(err))
                    if err.kind() == std::io::ErrorKind::ConnectionRefused =>
                {
                    debug!(peer_id, %addr, attempt, "Peer not yet up (connection refused), retrying...");
                }
                Err(NetworkError::HandshakeFailed {
                    peer_id: id,
                    ref reason,
                }) => {
                    warn!(
                        peer_id = id,
                        %addr,
                        attempt,
                        %reason,
                        "mTLS handshake failure with peer (certificate or TLS error)"
                    );
                }
                Err(err) => {
                    error!(peer_id, %addr, attempt, ?err, "Unexpected error connecting to peer");
                    if max_attempts > 0 && attempt >= max_attempts {
                        return Err(err);
                    }
                }
            }

            if max_attempts > 0 && attempt >= max_attempts {
                return Err(NetworkError::HandshakeFailed {
                    peer_id,
                    reason: format!("exceeded {} connection attempts", max_attempts),
                });
            }

            tokio::time::sleep(delay).await;
            delay = std::cmp::min(delay.saturating_mul(2), max_delay);
        }
    }
}
