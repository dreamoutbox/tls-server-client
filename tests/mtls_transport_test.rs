use std::path::Path;
use std::time::Duration;

use tls_server_client::cert::CertLoader;
use tls_server_client::network::{MtlsConnector, MtlsListener};

#[tokio::test]
async fn test_mtls_mutual_connection_and_rejection() {
    let node1_cert = Path::new("certs/node1.crt");
    let node1_key = Path::new("certs/node1.key");
    let node2_cert = Path::new("certs/node2.crt");
    let node2_key = Path::new("certs/node2.key");
    let ca_cert = Path::new("certs/ca.crt");

    if !node1_cert.exists() || !node2_cert.exists() || !ca_cert.exists() {
        eprintln!("Certs not found, skipping integration test");
        return;
    }

    // 1. Start mTLS listener for node 1 on an ephemeral port
    let srv_cfg = CertLoader::mtls_server_config(node1_cert, node1_key, ca_cert)
        .expect("node1 server config");
    let listener = MtlsListener::bind("127.0.0.1:0".parse().unwrap(), srv_cfg)
        .await
        .expect("bind listener");
    let listen_addr = listener.local_addr();

    // Spawn server accept task
    let server_task = tokio::spawn(async move {
        let (mut stream, peer_addr) = listener.accept().await.expect("accept node2");
        let msg = stream
            .read_frame()
            .await
            .expect("read frame")
            .expect("some frame");
        assert_eq!(&msg, b"ping from node2");
        stream
            .write_frame(b"pong from node1")
            .await
            .expect("write pong");
        peer_addr
    });

    // 2. Connect as node 2 presenting node 2's certificate
    let cli_cfg = CertLoader::mtls_client_config(node2_cert, node2_key, ca_cert)
        .expect("node2 client config");
    let connector = MtlsConnector::new(cli_cfg);

    let mut client_stream = connector
        .connect_with_backoff(
            1,
            listen_addr,
            5,
            Duration::from_millis(50),
            Duration::from_millis(500),
        )
        .await
        .expect("connect to node1");

    client_stream
        .write_frame(b"ping from node2")
        .await
        .expect("client write");

    let reply = client_stream
        .read_frame()
        .await
        .expect("client read")
        .expect("reply exists");
    assert_eq!(&reply, b"pong from node1");

    let _peer_addr = server_task.await.expect("server completed");
}

#[tokio::test]
async fn test_mtls_unauthenticated_client_rejected() {
    let node1_cert = Path::new("certs/node1.crt");
    let node1_key = Path::new("certs/node1.key");
    let ca_cert = Path::new("certs/ca.crt");

    if !node1_cert.exists() || !ca_cert.exists() {
        return;
    }

    let srv_cfg = CertLoader::mtls_server_config(node1_cert, node1_key, ca_cert)
        .expect("node1 server config");
    let listener = MtlsListener::bind("127.0.0.1:0".parse().unwrap(), srv_cfg)
        .await
        .expect("bind listener");
    let listen_addr = listener.local_addr();

    // Spawn server accept expecting error
    let server_task = tokio::spawn(async move { listener.accept().await });

    // Client connects WITHOUT client certificate
    let unauth_cli_cfg = CertLoader::client_config(ca_cert).expect("client config");
    let connector = MtlsConnector::new(unauth_cli_cfg);

    let connect_res = connector.connect(1, listen_addr).await;
    if let Ok(mut stream) = connect_res {
        // If connect returned before handshake finished, write_frame will trigger the handshake completion and fail
        let _ = stream.write_frame(b"unauthenticated").await;
    }

    let server_res = server_task.await.expect("server completed");
    assert!(
        server_res.is_err(),
        "Server must reject client without cert"
    );
}
