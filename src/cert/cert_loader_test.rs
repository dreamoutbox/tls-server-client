use std::path::Path;
use std::sync::Arc;

use crate::cert::CertLoader;

#[test]
fn test_cert_loader_loads_generated_certs() {
    let certs_path = Path::new("certs/server.crt");
    let key_path = Path::new("certs/server.key");
    let ca_path = Path::new("certs/ca.crt");

    if certs_path.exists() && key_path.exists() && ca_path.exists() {
        let certs = CertLoader::load_certs(certs_path).expect("valid server certs");
        assert!(!certs.is_empty());

        let _key = CertLoader::load_private_key(key_path).expect("valid server key");

        let roots = CertLoader::load_root_cert_store(ca_path).expect("valid root cert store");
        assert!(!roots.is_empty());

        let server_cfg =
            CertLoader::server_config(certs_path, key_path).expect("valid server config");
        assert!(Arc::strong_count(&server_cfg) >= 1);

        let client_cfg = CertLoader::client_config(ca_path).expect("valid client config");
        assert!(Arc::strong_count(&client_cfg) >= 1);
    }
}
