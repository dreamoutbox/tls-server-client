//! Configuration module for node identities and peer topologies.

mod error;
mod node_config;
#[cfg(test)]
mod node_config_test;
mod peer_config;

pub use error::ConfigError;
pub use node_config::NodeConfig;
pub use peer_config::PeerConfig;
