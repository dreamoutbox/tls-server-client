//! Network transport module for mutual TLS peer communication and framing.

mod error;
mod framed_stream;
#[cfg(test)]
mod framed_stream_test;
mod mtls_connector;
mod mtls_listener;

pub use error::{MAX_FRAME_SIZE, NetworkError};
pub use framed_stream::FramedStream;
pub use mtls_connector::MtlsConnector;
pub use mtls_listener::MtlsListener;
