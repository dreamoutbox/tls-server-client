//! Certificate and TLS configuration loader.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;

use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::{ClientConfig, RootCertStore, ServerConfig};

use crate::cert::error::CertError;

/// Utility loader for certificates, keys, and rustls configurations.
pub struct CertLoader;

impl CertLoader {
    /// Loads X.509 certificates from a PEM file.
    ///
    /// # Arguments
    /// * `path` - Filesystem path to the certificate PEM file.
    ///
    /// # Errors
    /// Returns `CertError::Io` if reading fails, or `CertError::NoCertificatesFound`
    /// if the file contains no PEM certificates.
    ///
    /// # Example
    /// ```no_run
    /// use std::path::Path;
    /// use tls_server_client::cert::CertLoader;
    ///
    /// let certs = CertLoader::load_certs(Path::new("certs/server.crt"));
    /// ```
    pub fn load_certs(path: &Path) -> Result<Vec<CertificateDer<'static>>, CertError> {
        let file = File::open(path).map_err(|err| CertError::Io {
            path: path.to_path_buf(),
            source: err,
        })?;
        let mut reader = BufReader::new(file);

        let mut certs = Vec::new();
        for item in rustls_pemfile::certs(&mut reader) {
            let cert = item.map_err(|err| CertError::Io {
                path: path.to_path_buf(),
                source: err,
            })?;
            certs.push(cert);
        }

        if certs.is_empty() {
            return Err(CertError::NoCertificatesFound {
                path: path.to_path_buf(),
            });
        }

        Ok(certs)
    }

    /// Loads a private key (PKCS#8, PKCS#1 RSA, or SEC1 EC) from a PEM file.
    ///
    /// # Arguments
    /// * `path` - Filesystem path to the private key PEM file.
    ///
    /// # Errors
    /// Returns `CertError::Io` if reading fails, or `CertError::NoPrivateKeyFound`
    /// if no supported private key is found.
    ///
    /// # Example
    /// ```no_run
    /// use std::path::Path;
    /// use tls_server_client::cert::CertLoader;
    ///
    /// let key = CertLoader::load_private_key(Path::new("certs/server.key"));
    /// ```
    pub fn load_private_key(path: &Path) -> Result<PrivateKeyDer<'static>, CertError> {
        let file = File::open(path).map_err(|err| CertError::Io {
            path: path.to_path_buf(),
            source: err,
        })?;
        let mut reader = BufReader::new(file);

        let key = rustls_pemfile::private_key(&mut reader)
            .map_err(|err| CertError::Io {
                path: path.to_path_buf(),
                source: err,
            })?
            .ok_or_else(|| CertError::NoPrivateKeyFound {
                path: path.to_path_buf(),
            })?;

        Ok(key)
    }

    /// Loads CA certificates from a PEM file into a rustls `RootCertStore`.
    ///
    /// # Arguments
    /// * `path` - Filesystem path to the CA certificate PEM file.
    ///
    /// # Errors
    /// Returns `CertError::Io` if reading fails or no certificates are found.
    ///
    /// # Example
    /// ```no_run
    /// use std::path::Path;
    /// use tls_server_client::cert::CertLoader;
    ///
    /// let roots = CertLoader::load_root_cert_store(Path::new("certs/ca.crt"));
    /// ```
    pub fn load_root_cert_store(path: &Path) -> Result<RootCertStore, CertError> {
        let certs = Self::load_certs(path)?;
        let mut root_store = RootCertStore::empty();
        for cert in certs {
            // RootCertStore::add ignores unsupported certificates or errors
            let _ = root_store.add(cert);
        }
        Ok(root_store)
    }

    /// Builds a rustls `ServerConfig` from certificate and private key paths.
    ///
    /// # Arguments
    /// * `cert_path` - Path to server certificate PEM file.
    /// * `key_path` - Path to server private key PEM file.
    ///
    /// # Errors
    /// Returns `CertError` if files cannot be read or TLS configuration is invalid.
    ///
    /// # Example
    /// ```no_run
    /// use std::path::Path;
    /// use tls_server_client::cert::CertLoader;
    ///
    /// let config = CertLoader::server_config(
    ///     Path::new("certs/server.crt"),
    ///     Path::new("certs/server.key"),
    /// );
    /// ```
    pub fn server_config(
        cert_path: &Path,
        key_path: &Path,
    ) -> Result<Arc<ServerConfig>, CertError> {
        let certs = Self::load_certs(cert_path)?;
        let key = Self::load_private_key(key_path)?;

        let provider = rustls::crypto::ring::default_provider();
        let config = ServerConfig::builder_with_provider(Arc::new(provider))
            .with_safe_default_protocol_versions()?
            .with_no_client_auth()
            .with_single_cert(certs, key)?;

        Ok(Arc::new(config))
    }

    /// Builds a rustls `ClientConfig` trusting the CA certificate at `ca_path`.
    ///
    /// # Arguments
    /// * `ca_path` - Path to CA certificate PEM file.
    ///
    /// # Errors
    /// Returns `CertError` if CA cannot be read or TLS configuration is invalid.
    ///
    /// # Example
    /// ```no_run
    /// use std::path::Path;
    /// use tls_server_client::cert::CertLoader;
    ///
    /// let config = CertLoader::client_config(Path::new("certs/ca.crt"));
    /// ```
    pub fn client_config(ca_path: &Path) -> Result<Arc<ClientConfig>, CertError> {
        let root_store = Self::load_root_cert_store(ca_path)?;

        let provider = rustls::crypto::ring::default_provider();
        let config = ClientConfig::builder_with_provider(Arc::new(provider))
            .with_safe_default_protocol_versions()?
            .with_root_certificates(root_store)
            .with_no_client_auth();

        Ok(Arc::new(config))
    }

    /// Builds a rustls `ServerConfig` requiring mutual TLS authentication verified against `ca_path`.
    ///
    /// # Arguments
    /// * `cert_path` - Path to server certificate PEM file.
    /// * `key_path` - Path to server private key PEM file.
    /// * `ca_path` - Path to CA certificate PEM file used to authenticate clients.
    ///
    /// # Errors
    /// Returns `CertError` if files cannot be read or TLS configuration is invalid.
    ///
    /// # Example
    /// ```no_run
    /// use std::path::Path;
    /// use tls_server_client::cert::CertLoader;
    ///
    /// let config = CertLoader::mtls_server_config(
    ///     Path::new("certs/node1.crt"),
    ///     Path::new("certs/node1.key"),
    ///     Path::new("certs/ca.crt"),
    /// );
    /// ```
    pub fn mtls_server_config(
        cert_path: &Path,
        key_path: &Path,
        ca_path: &Path,
    ) -> Result<Arc<ServerConfig>, CertError> {
        let certs = Self::load_certs(cert_path)?;
        let key = Self::load_private_key(key_path)?;
        let root_store = Self::load_root_cert_store(ca_path)?;

        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
            Arc::new(root_store),
            provider.clone(),
        )
        .build()?;
        let config = ServerConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()?
            .with_client_cert_verifier(verifier)
            .with_single_cert(certs, key)?;

        Ok(Arc::new(config))
    }

    /// Builds a rustls `ClientConfig` with client certificate for mutual TLS authentication.
    ///
    /// # Arguments
    /// * `cert_path` - Path to client certificate PEM file.
    /// * `key_path` - Path to client private key PEM file.
    /// * `ca_path` - Path to CA certificate PEM file used to authenticate peers.
    ///
    /// # Errors
    /// Returns `CertError` if files cannot be read or TLS configuration is invalid.
    ///
    /// # Example
    /// ```no_run
    /// use std::path::Path;
    /// use tls_server_client::cert::CertLoader;
    ///
    /// let config = CertLoader::mtls_client_config(
    ///     Path::new("certs/node1.crt"),
    ///     Path::new("certs/node1.key"),
    ///     Path::new("certs/ca.crt"),
    /// );
    /// ```
    pub fn mtls_client_config(
        cert_path: &Path,
        key_path: &Path,
        ca_path: &Path,
    ) -> Result<Arc<ClientConfig>, CertError> {
        let certs = Self::load_certs(cert_path)?;
        let key = Self::load_private_key(key_path)?;
        let root_store = Self::load_root_cert_store(ca_path)?;

        let provider = rustls::crypto::ring::default_provider();
        let config = ClientConfig::builder_with_provider(Arc::new(provider))
            .with_safe_default_protocol_versions()?
            .with_root_certificates(root_store)
            .with_client_auth_cert(certs, key)?;

        Ok(Arc::new(config))
    }
}
