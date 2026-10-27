//! OpenRaft type configuration for the swarm cluster POC.

use std::io::Cursor;

use crate::consensus::task_command::TaskCommand;
use crate::consensus::task_response::TaskResponse;

openraft::declare_raft_types!(
    /// Type configuration defining the consensus types used by this Raft node.
    pub TypeConfig:
        D = TaskCommand,
        R = TaskResponse,
        NodeId = u64,
        Node = openraft::BasicNode,
        Entry = openraft::Entry<TypeConfig>,
        SnapshotData = Cursor<Vec<u8>>,
        AsyncRuntime = openraft::TokioRuntime
);
