//! TLS echo client binary.
//!
//! Connects to the TLS echo server, verifies server certificate against CA,
//! sends a test payload, and prints the echoed response.

use std::path::Path;

use rustls_pki_types::ServerName;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

use tls_server_client::cert::CertLoader;

/// Connects to a TLS echo server, sends a test payload, and verifies the echo response.
///
/// # Arguments
/// * `server_addr` - Target TCP address (host:port) to connect to.
/// * `server_name` - Expected DNS name or IP for TLS certificate validation.
/// * `ca_path` - Path to the CA certificate PEM file used to verify the server.
/// * `payload` - Message payload to send to the server.
///
/// # Errors
/// Returns an error if connection, TLS handshake, or reading/writing fails,
/// or if the echoed payload does not match the sent payload.
///
/// # Example
/// ```no_run
/// # use std::path::Path;
/// # async fn example() {
/// #     let _ = tls_server_client_bin_example::send_and_receive(
/// #         "127.0.0.1:8443",
/// #         "localhost",
/// #         Path::new("certs/ca.crt"),
/// #         "Hello, TLS!",
/// #     ).await;
/// # }
/// ```
async fn send_and_receive(
    server_addr: &str,
    server_name: &str,
    ca_path: &Path,
    payload: &str,
) -> anyhow::Result<String> {
    let client_config = CertLoader::client_config(ca_path)?;
    let connector = TlsConnector::from(client_config);

    let domain = ServerName::try_from(server_name.to_string())
        .map_err(|_| anyhow::anyhow!("invalid server name: {server_name}"))?;

    tracing::info!(%server_addr, %server_name, "connecting to TLS server");
    let stream = TcpStream::connect(server_addr).await?;
    let mut tls_stream = connector.connect(domain, stream).await?;
    tracing::info!("TLS handshake completed successfully");

    tracing::debug!(payload_len = payload.len(), "sending payload to server");
    tls_stream.write_all(payload.as_bytes()).await?;
    tls_stream.flush().await?;

    let mut response_buf = vec![0u8; payload.len()];
    tls_stream.read_exact(&mut response_buf).await?;

    let echoed = String::from_utf8(response_buf)
        .map_err(|err| anyhow::anyhow!("response is not valid utf-8: {err}"))?;

    if echoed != payload {
        anyhow::bail!("echo mismatch: expected '{payload}', received '{echoed}'");
    }

    Ok(echoed)
}

/// Entrypoint for the TLS client binary.
///
/// # Arguments
/// None.
///
/// # Errors
/// Propagates any fatal errors during TLS connection, handshake, or I/O.
///
/// # Example
/// ```bash
/// cargo run --bin client
/// ```
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let server_addr =
        std::env::var("TLS_SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:8443".to_string());
    let server_name =
        std::env::var("TLS_SERVER_NAME").unwrap_or_else(|_| "localhost".to_string());
    let ca_path =
        std::env::var("TLS_CA_PATH").unwrap_or_else(|_| "certs/ca.crt".to_string());
    let payload =
        std::env::var("TLS_PAYLOAD").unwrap_or_else(|_| "Hello, Secure Rust TLS World!".to_string());

    let echoed = send_and_receive(&server_addr, &server_name, Path::new(&ca_path), &payload).await?;

    tracing::info!(%echoed, "received verified echo response");
    println!("{echoed}");

    Ok(())
}
