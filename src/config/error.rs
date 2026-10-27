//! Configuration error definitions.

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur when loading or validating node configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Failed to read the configuration file from disk.
    #[error("failed to read configuration file from '{path}': {source}")]
    Io {
        /// File path that failed to read.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Failed to deserialize configuration file content.
    #[error("failed to deserialize configuration from '{path}': {source}")]
    Deserialize {
        /// File path containing invalid configuration.
        path: PathBuf,
        /// Underlying TOML deserialization error.
        #[source]
        source: toml::de::Error,
    },

    /// Validation failed on the parsed configuration values.
    #[error("invalid configuration in '{path}': {message}")]
    Validation {
        /// File path of the invalid configuration.
        path: PathBuf,
        /// Validation failure details.
        message: String,
    },
}
