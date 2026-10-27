//! Mutual TLS listener for accepting authenticated inbound connections from cluster peers.

use std::net::SocketAddr;
use std::sync::Arc;

use rustls::ServerConfig;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;
use tokio_rustls::server::TlsStream;
use tracing::{debug, info, warn};

use crate::network::error::NetworkError;
use crate::network::framed_stream::FramedStream;

/// Listener that requires mutual TLS authentication on all inbound connections.
pub struct MtlsListener {
    listener: TcpListener,
    acceptor: TlsAcceptor,
    local_addr: SocketAddr,
}

impl MtlsListener {
    /// Binds an mTLS listener to the specified address with mutual client authentication.
    ///
    /// # Arguments
    /// * `addr` - Network address to bind the TCP listener to.
    /// * `config` - Server configuration requiring client certificate verification against the cluster CA.
    ///
    /// # Errors
    /// Returns [`NetworkError::Io`] if socket binding fails.
    ///
    /// # Example
    /// ```rust,no_run
    /// use std::sync::Arc;
    /// use rustls::ServerConfig;
    /// use tls_server_client::network::MtlsListener;
    ///
    /// # async fn doc_example(config: Arc<ServerConfig>) -> Result<(), Box<dyn std::error::Error>> {
    /// let listener = MtlsListener::bind("127.0.0.1:8443".parse()?, config).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn bind(addr: SocketAddr, config: Arc<ServerConfig>) -> Result<Self, NetworkError> {
        let listener = TcpListener::bind(addr).await?;
        let local_addr = listener.local_addr()?;
        info!(%local_addr, "mTLS listener bound and waiting for authenticated peers");

        Ok(Self {
            listener,
            acceptor: TlsAcceptor::from(config),
            local_addr,
        })
    }

    /// Returns the local socket address this listener is bound to.
    #[must_use]
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Accepts an incoming connection and performs the mutual TLS handshake.
    ///
    /// If the handshake fails (e.g., untrusted certificate, no certificate presented),
    /// a warning is logged and the error is returned.
    ///
    /// # Errors
    /// Returns [`NetworkError::Io`] on TCP socket error or [`NetworkError::Tls`] if
    /// the TLS handshake or client certificate validation fails.
    pub async fn accept(
        &self,
    ) -> Result<(FramedStream<TlsStream<TcpStream>>, SocketAddr), NetworkError> {
        let (tcp_stream, peer_addr) = self.listener.accept().await?;
        debug!(%peer_addr, "Inbound TCP connection received, initiating mTLS handshake");

        match self.acceptor.accept(tcp_stream).await {
            Ok(tls_stream) => {
                info!(%peer_addr, "mTLS mutual handshake succeeded; peer authenticated");
                Ok((FramedStream::new(tls_stream), peer_addr))
            }
            Err(err) => {
                warn!(
                    %peer_addr,
                    %err,
                    "Inbound mTLS handshake failed (client certificate untrusted or invalid)"
                );
                Err(NetworkError::Io(err))
            }
        }
    }
}
