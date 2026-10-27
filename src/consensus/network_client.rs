//! OpenRaft network client implementation over mutual TLS framed streams.

use std::net::SocketAddr;

use openraft::BasicNode;
use openraft::error::{InstallSnapshotError, NetworkError, RPCError, RaftError};
use openraft::network::RPCOption;
use openraft::network::RaftNetwork;
use openraft::raft::{
    AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest, InstallSnapshotResponse,
    VoteRequest, VoteResponse,
};

use crate::consensus::rpc_message::{RaftRpcRequest, RaftRpcResponse};
use crate::consensus::type_config::TypeConfig;
use crate::network::MtlsConnector;

/// Network client that transmits Raft RPCs to a specific remote peer over an mTLS stream.
pub struct NetworkClient {
    target_id: u64,
    target_addr: SocketAddr,
    connector: MtlsConnector,
}

impl NetworkClient {
    /// Creates a new network client targeting a specific peer.
    ///
    /// # Arguments
    /// * `target_id` - Identifier of the target peer.
    /// * `target_addr` - Network socket address of the target peer.
    /// * `connector` - Mutual TLS connector for dialing peers.
    ///
    /// # Example
    /// ```rust,no_run
    /// use std::net::SocketAddr;
    /// use std::sync::Arc;
    /// use rustls::ClientConfig;
    /// use tls_server_client::network::MtlsConnector;
    /// use tls_server_client::consensus::NetworkClient;
    ///
    /// # fn doc_example(connector: MtlsConnector, addr: SocketAddr) {
    /// let client = NetworkClient::new(2, addr, connector);
    /// # }
    /// ```
    pub fn new(target_id: u64, target_addr: SocketAddr, connector: MtlsConnector) -> Self {
        Self {
            target_id,
            target_addr,
            connector,
        }
    }

    /// Internal helper to send a Raft RPC request and receive the corresponding response.
    async fn send_rpc(&self, req: RaftRpcRequest) -> Result<RaftRpcResponse, NetworkError> {
        let mut stream = self
            .connector
            .connect(self.target_id, self.target_addr)
            .await
            .map_err(|e| NetworkError::new(&e))?;

        let req_bytes = serde_json::to_vec(&req).map_err(|e| NetworkError::new(&e))?;
        stream
            .write_frame(&req_bytes)
            .await
            .map_err(|e| NetworkError::new(&e))?;

        let resp_bytes = stream
            .read_frame()
            .await
            .map_err(|e| NetworkError::new(&e))?
            .ok_or_else(|| {
                NetworkError::new(&std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "peer closed connection without responding",
                ))
            })?;

        let resp: RaftRpcResponse =
            serde_json::from_slice(&resp_bytes).map_err(|e| NetworkError::new(&e))?;
        Ok(resp)
    }
}

impl RaftNetwork<TypeConfig> for NetworkClient {
    async fn append_entries(
        &mut self,
        rpc: AppendEntriesRequest<TypeConfig>,
        _option: RPCOption,
    ) -> Result<AppendEntriesResponse<u64>, RPCError<u64, BasicNode, RaftError<u64>>> {
        match self.send_rpc(RaftRpcRequest::AppendEntries(rpc)).await {
            Ok(RaftRpcResponse::AppendEntries(resp)) => Ok(resp),
            Ok(RaftRpcResponse::Error(msg)) => Err(RPCError::Network(NetworkError::new(
                &std::io::Error::other(msg),
            ))),
            Ok(_) => Err(RPCError::Network(NetworkError::new(
                &std::io::Error::other("invalid RPC response variant for AppendEntries"),
            ))),
            Err(err) => Err(RPCError::Network(err)),
        }
    }

    async fn install_snapshot(
        &mut self,
        rpc: InstallSnapshotRequest<TypeConfig>,
        _option: RPCOption,
    ) -> Result<
        InstallSnapshotResponse<u64>,
        RPCError<u64, BasicNode, RaftError<u64, InstallSnapshotError>>,
    > {
        match self.send_rpc(RaftRpcRequest::InstallSnapshot(rpc)).await {
            Ok(RaftRpcResponse::InstallSnapshot(resp)) => Ok(resp),
            Ok(RaftRpcResponse::Error(msg)) => Err(RPCError::Network(NetworkError::new(
                &std::io::Error::other(msg),
            ))),
            Ok(_) => Err(RPCError::Network(NetworkError::new(
                &std::io::Error::other("invalid RPC response variant for InstallSnapshot"),
            ))),
            Err(err) => Err(RPCError::Network(err)),
        }
    }

    async fn vote(
        &mut self,
        rpc: VoteRequest<u64>,
        _option: RPCOption,
    ) -> Result<VoteResponse<u64>, RPCError<u64, BasicNode, RaftError<u64>>> {
        match self.send_rpc(RaftRpcRequest::Vote(rpc)).await {
            Ok(RaftRpcResponse::Vote(resp)) => Ok(resp),
            Ok(RaftRpcResponse::Error(msg)) => Err(RPCError::Network(NetworkError::new(
                &std::io::Error::other(msg),
            ))),
            Ok(_) => Err(RPCError::Network(NetworkError::new(
                &std::io::Error::other("invalid RPC response variant for Vote"),
            ))),
            Err(err) => Err(RPCError::Network(err)),
        }
    }
}
