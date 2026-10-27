//! Length-prefixed framing over asynchronous byte streams.

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::network::error::{MAX_FRAME_SIZE, NetworkError};

/// A length-prefixed framed stream wrapper over an asynchronous stream.
///
/// Each frame begins with a 4-byte big-endian unsigned integer indicating the length
/// of the subsequent payload in bytes.
#[derive(Debug)]
pub struct FramedStream<S> {
    inner: S,
}

impl<S> FramedStream<S> {
    /// Wraps an asynchronous stream with length-prefixed framing.
    ///
    /// # Arguments
    /// * `inner` - The underlying async byte stream.
    ///
    /// # Example
    /// ```rust
    /// use tokio::io::duplex;
    /// use tls_server_client::network::FramedStream;
    ///
    /// let (client, _server) = duplex(64);
    /// let framed = FramedStream::new(client);
    /// ```
    pub fn new(inner: S) -> Self {
        Self { inner }
    }

    /// Consumes the wrapper and returns the underlying stream.
    ///
    /// # Example
    /// ```rust
    /// use tokio::io::duplex;
    /// use tls_server_client::network::FramedStream;
    ///
    /// let (client, _server) = duplex(64);
    /// let framed = FramedStream::new(client);
    /// let _inner = framed.into_inner();
    /// ```
    pub fn into_inner(self) -> S {
        self.inner
    }

    /// Returns a mutable reference to the underlying stream.
    pub fn get_mut(&mut self) -> &mut S {
        &mut self.inner
    }
}

impl<S> FramedStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    /// Writes a length-prefixed frame to the stream and flushes.
    ///
    /// # Arguments
    /// * `payload` - Bytes to transmit as the frame payload.
    ///
    /// # Errors
    /// Returns [`NetworkError::FrameTooLarge`] if `payload.len()` exceeds [`MAX_FRAME_SIZE`],
    /// or [`NetworkError::Io`] if transmission fails.
    ///
    /// # Example
    /// ```rust
    /// # use tokio::io::duplex;
    /// # use tls_server_client::network::FramedStream;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let (mut client, mut server) = duplex(1024);
    /// let mut framed = FramedStream::new(&mut client);
    /// framed.write_frame(b"hello").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn write_frame(&mut self, payload: &[u8]) -> Result<(), NetworkError> {
        let len = u32::try_from(payload.len()).map_err(|_| NetworkError::FrameTooLarge {
            size: u32::MAX,
            max: MAX_FRAME_SIZE,
        })?;

        if len > MAX_FRAME_SIZE {
            return Err(NetworkError::FrameTooLarge {
                size: len,
                max: MAX_FRAME_SIZE,
            });
        }

        self.inner.write_all(&len.to_be_bytes()).await?;
        self.inner.write_all(payload).await?;
        self.inner.flush().await?;
        Ok(())
    }

    /// Reads the next length-prefixed frame from the stream.
    ///
    /// # Returns
    /// * `Ok(Some(Vec<u8>))` containing the payload if a full frame was read.
    /// * `Ok(None)` if the peer closed the connection gracefully before any bytes were sent.
    ///
    /// # Errors
    /// Returns [`NetworkError::PeerDisconnected`] if the stream terminated mid-frame,
    /// [`NetworkError::FrameTooLarge`] if the frame header exceeds [`MAX_FRAME_SIZE`],
    /// or [`NetworkError::Io`] if read operations fail.
    ///
    /// # Example
    /// ```rust
    /// # use tokio::io::duplex;
    /// # use tls_server_client::network::FramedStream;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let (mut client, mut server) = duplex(1024);
    /// let mut framed_client = FramedStream::new(&mut client);
    /// framed_client.write_frame(b"hello").await?;
    ///
    /// let mut framed_server = FramedStream::new(&mut server);
    /// let frame = framed_server.read_frame().await?.unwrap();
    /// assert_eq!(&frame, b"hello");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn read_frame(&mut self) -> Result<Option<Vec<u8>>, NetworkError> {
        let mut len_buf = [0u8; 4];
        let bytes_read = match self.inner.read_exact(&mut len_buf).await {
            Ok(n) => n,
            Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(err) => return Err(NetworkError::Io(err)),
        };

        if bytes_read == 0 {
            return Ok(None);
        }

        let len = u32::from_be_bytes(len_buf);
        if len > MAX_FRAME_SIZE {
            return Err(NetworkError::FrameTooLarge {
                size: len,
                max: MAX_FRAME_SIZE,
            });
        }

        let usize_len = usize::try_from(len).map_err(|_| NetworkError::FrameTooLarge {
            size: len,
            max: MAX_FRAME_SIZE,
        })?;

        let mut payload = vec![0u8; usize_len];
        match self.inner.read_exact(&mut payload).await {
            Ok(_) => Ok(Some(payload)),
            Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                Err(NetworkError::PeerDisconnected)
            }
            Err(err) => Err(NetworkError::Io(err)),
        }
    }
}
