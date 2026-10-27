//! RPC message schemas for transmitting Raft requests and responses over mTLS streams.

use serde::{Deserialize, Serialize};

use crate::consensus::type_config::TypeConfig;

/// RPC request payload transmitted over the length-prefixed TLS framed stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RaftRpcRequest {
    /// Request for leader vote in an election.
    Vote(openraft::raft::VoteRequest<u64>),
    /// Request to append log entries or send heartbeat.
    AppendEntries(openraft::raft::AppendEntriesRequest<TypeConfig>),
    /// Request to stream snapshot chunk.
    InstallSnapshot(openraft::raft::InstallSnapshotRequest<TypeConfig>),
    /// Request to submit a swarm task command to the cluster.
    Task(crate::consensus::task_command::TaskCommand),
}

/// RPC response payload returned over the length-prefixed TLS framed stream.
#[derive(Debug, Serialize, Deserialize)]
pub enum RaftRpcResponse {
    /// Result of a vote request.
    Vote(openraft::raft::VoteResponse<u64>),
    /// Result of an append entries request.
    AppendEntries(openraft::raft::AppendEntriesResponse<u64>),
    /// Result of an install snapshot request.
    InstallSnapshot(openraft::raft::InstallSnapshotResponse<u64>),
    /// Result of an executed task command.
    Task(crate::consensus::task_response::TaskResponse),
    /// Error indication if processing failed.
    Error(String),
}
