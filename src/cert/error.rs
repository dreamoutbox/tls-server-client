//! Error types for certificate and key operations.

use std::io;
use std::path::PathBuf;

/// Errors that can occur during certificate and key loading.
#[derive(thiserror::Error, Debug)]
pub enum CertError {
    /// Failed to read or access certificate/key file.
    #[error("failed to read file '{path}': {source}")]
    Io {
        /// File path that caused the error.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },
    /// No valid PEM certificates found in the provided file.
    #[error("no certificates found in '{path}'")]
    NoCertificatesFound {
        /// Certificate file path.
        path: PathBuf,
    },
    /// No valid private key found in the provided file.
    #[error("no private key found in '{path}'")]
    NoPrivateKeyFound {
        /// Private key file path.
        path: PathBuf,
    },
    /// Underlying TLS protocol or configuration error.
    #[error("tls configuration error: {0}")]
    Tls(#[from] rustls::Error),
    /// Client certificate verifier builder error.
    #[error("client verifier error: {0}")]
    Verifier(#[from] rustls::server::VerifierBuilderError),
}
