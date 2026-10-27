#[cfg(test)]
mod tests {
    use crate::network::error::NetworkError;
    use crate::network::framed_stream::FramedStream;
    use tokio::io::duplex;

    #[tokio::test]
    async fn test_write_and_read_frame() {
        let (client, server) = duplex(1024);
        let mut framed_client = FramedStream::new(client);
        let mut framed_server = FramedStream::new(server);

        let data = b"swarm-poc test payload";
        framed_client.write_frame(data).await.expect("write frame");

        let received = framed_server.read_frame().await.expect("read frame");
        assert_eq!(received, Some(data.to_vec()));
    }

    #[tokio::test]
    async fn test_read_frame_eof() {
        let (client, server) = duplex(64);
        drop(client);
        let mut framed_server = FramedStream::new(server);

        let received = framed_server.read_frame().await.expect("read on closed");
        assert_eq!(received, None);
    }

    #[tokio::test]
    async fn test_read_frame_truncated() {
        let (mut client, server) = duplex(64);
        use tokio::io::AsyncWriteExt;
        // write 4-byte length prefix specifying 10 bytes, but send only 2 bytes
        client.write_all(&10u32.to_be_bytes()).await.unwrap();
        client.write_all(&[1, 2]).await.unwrap();
        drop(client);

        let mut framed_server = FramedStream::new(server);
        let result = framed_server.read_frame().await;
        assert!(matches!(result, Err(NetworkError::PeerDisconnected)));
    }
}
