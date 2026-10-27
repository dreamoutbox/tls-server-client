//! TLS echo server binary.
//!
//! Listens on a TCP port, negotiates TLS with connecting clients, and echoes
//! back received payloads.

use std::net::SocketAddr;
use std::path::Path;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;
use tracing::Instrument;

use tls_server_client::cert::CertLoader;

/// Handles an individual incoming TCP connection by completing TLS handshake and echoing bytes.
///
/// # Arguments
/// * `stream` - The raw incoming TCP stream.
/// * `peer_addr` - Socket address of the connected peer.
/// * `acceptor` - TLS acceptor configured with server certificates.
///
/// # Errors
/// Returns an error if TLS handshake fails or reading/writing encounters an I/O error.
///
/// # Example
/// ```no_run
/// # use std::net::SocketAddr;
/// # use tokio::net::TcpStream;
/// # use tokio_rustls::TlsAcceptor;
/// # async fn example(stream: TcpStream, peer_addr: SocketAddr, acceptor: TlsAcceptor) {
/// #     let _ = tls_server_client_bin_example::handle_connection(stream, peer_addr, acceptor).await;
/// # }
/// ```
async fn handle_connection(
    stream: TcpStream,
    peer_addr: SocketAddr,
    acceptor: TlsAcceptor,
) -> anyhow::Result<()> {
    let mut tls_stream = acceptor.accept(stream).await?;
    tracing::info!(%peer_addr, "TLS handshake completed");

    let mut buf = vec![0u8; 4096];
    loop {
        let n = tls_stream.read(&mut buf).await?;
        if n == 0 {
            tracing::info!(%peer_addr, "connection closed by peer");
            break;
        }

        tracing::debug!(%peer_addr, bytes_read = n, "received payload, echoing back");
        tls_stream.write_all(&buf[..n]).await?;
        tls_stream.flush().await?;
    }

    Ok(())
}

/// Binds to a local address, loads certificates, and accepts TLS connections.
///
/// # Arguments
/// * `bind_addr` - The host:port address string to bind the TCP listener to.
/// * `cert_path` - Path to the server certificate PEM file.
/// * `key_path` - Path to the server private key PEM file.
///
/// # Errors
/// Returns an error if certificate loading fails or TCP binding fails.
///
/// # Example
/// ```no_run
/// # use std::path::Path;
/// # async fn example() {
/// #     let _ = tls_server_client_bin_example::run_server("127.0.0.1:8443", Path::new("certs/server.crt"), Path::new("certs/server.key")).await;
/// # }
/// ```
async fn run_server(bind_addr: &str, cert_path: &Path, key_path: &Path) -> anyhow::Result<()> {
    let server_config = CertLoader::server_config(cert_path, key_path)?;
    let acceptor = TlsAcceptor::from(server_config);

    let listener = TcpListener::bind(bind_addr).await?;
    tracing::info!(%bind_addr, "TLS server listening");

    loop {
        let (stream, peer_addr) = listener.accept().await?;
        // Clone cheap Arc handle to ServerConfig for spawned connection task
        let conn_acceptor = acceptor.clone();
        let span = tracing::info_span!("connection", %peer_addr);

        tokio::spawn(
            async move {
                if let Err(err) = handle_connection(stream, peer_addr, conn_acceptor).await {
                    tracing::warn!(%peer_addr, error = %err, "connection terminated with error");
                }
            }
            .instrument(span),
        );
    }
}

/// Entrypoint for the TLS server binary.
///
/// # Arguments
/// None.
///
/// # Errors
/// Propagates any fatal errors during configuration or server execution.
///
/// # Example
/// ```bash
/// cargo run --bin server
/// ```
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let bind_addr = std::env::var("TLS_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8443".to_string());
    let cert_path =
        std::env::var("TLS_CERT_PATH").unwrap_or_else(|_| "certs/server.crt".to_string());
    let key_path = std::env::var("TLS_KEY_PATH").unwrap_or_else(|_| "certs/server.key".to_string());

    run_server(&bind_addr, Path::new(&cert_path), Path::new(&key_path)).await
}
