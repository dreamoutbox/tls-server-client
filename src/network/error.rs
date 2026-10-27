//! Network and transport error definitions.

use thiserror::Error;

/// Maximum allowable frame size (16 MB) to prevent denial of service through large allocations.
pub const MAX_FRAME_SIZE: u32 = 16 * 1024 * 1024;

/// Errors that can occur during network communication, TLS handshakes, or frame parsing.
#[derive(Debug, Error)]
pub enum NetworkError {
    /// Underlying I/O error occurred.
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),

    /// TLS protocol error occurred.
    #[error("tls error: {0}")]
    Tls(#[from] rustls::Error),

    /// A received frame exceeds the maximum permitted frame size.
    #[error("frame size {size} bytes exceeds maximum allowed size of {max} bytes")]
    FrameTooLarge {
        /// Attempted frame size in bytes.
        size: u32,
        /// Maximum permitted frame size.
        max: u32,
    },

    /// The remote peer closed the stream prematurely.
    #[error("peer closed connection unexpectedly")]
    PeerDisconnected,

    /// Handshake with peer failed.
    #[error("mtls handshake failed with peer {peer_id}: {reason}")]
    HandshakeFailed {
        /// Identifier of the target peer.
        peer_id: u64,
        /// Description of the handshake failure.
        reason: String,
    },

    /// Failed to parse server name for TLS verification.
    #[error("invalid server name '{0}'")]
    InvalidServerName(String),
}
