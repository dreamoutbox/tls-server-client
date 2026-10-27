//! In-memory Raft storage implementation for the swarm cluster POC.

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::io::Cursor;
use std::ops::RangeBounds;
use std::sync::Arc;

use openraft::storage::{LogState, RaftLogReader, RaftSnapshotBuilder, RaftStorage, Snapshot};
use openraft::{
    EntryPayload, LogId, OptionalSend, RaftLogId, SnapshotMeta, StorageError, StoredMembership,
    Vote,
};
use tokio::sync::RwLock;
use tracing::info;

use crate::consensus::task_response::TaskResponse;
use crate::consensus::type_config::TypeConfig;

/// Internal shared mutable state for in-memory Raft storage.
#[derive(Debug, Default)]
struct StoreInner {
    /// Last persisted vote.
    vote: Option<Vote<u64>>,
    /// In-memory log entries indexed by log index.
    log: BTreeMap<u64, openraft::Entry<TypeConfig>>,
    /// Last purged log ID.
    last_purged_log_id: Option<LogId<u64>>,
    /// Last applied log ID in the state machine.
    last_applied_log_id: Option<LogId<u64>>,
    /// Last applied cluster membership.
    last_membership: StoredMembership<u64, openraft::BasicNode>,
    /// Last snapshot data in bytes.
    snapshot_data: Option<Vec<u8>>,
    /// Metadata of the current snapshot.
    snapshot_meta: Option<SnapshotMeta<u64, openraft::BasicNode>>,
}

/// In-memory implementation of [`RaftStorage`] for the local cluster POC.
///
/// Clones share the underlying [`Arc<RwLock<StoreInner>>`] state so that log readers
/// and writers always observe synchronized log entries and committed state.
#[derive(Debug, Clone, Default)]
pub struct InMemoryStore {
    self_id: u64,
    inner: Arc<RwLock<StoreInner>>,
}

impl InMemoryStore {
    /// Creates a new in-memory store instance for the given node.
    ///
    /// # Arguments
    /// * `self_id` - Identifier for the local node.
    ///
    /// # Example
    /// ```rust
    /// use tls_server_client::consensus::InMemoryStore;
    ///
    /// let store = InMemoryStore::new(1);
    /// ```
    #[must_use]
    pub fn new(self_id: u64) -> Self {
        Self {
            self_id,
            inner: Arc::new(RwLock::new(StoreInner::default())),
        }
    }
}

impl RaftLogReader<TypeConfig> for InMemoryStore {
    async fn try_get_log_entries<RB: RangeBounds<u64> + Clone + Debug + OptionalSend>(
        &mut self,
        range: RB,
    ) -> Result<Vec<openraft::Entry<TypeConfig>>, StorageError<u64>> {
        let inner = self.inner.read().await;
        let entries = inner
            .log
            .range(range)
            .map(|(_, entry)| entry.clone())
            .collect();
        Ok(entries)
    }
}

impl RaftSnapshotBuilder<TypeConfig> for InMemoryStore {
    async fn build_snapshot(&mut self) -> Result<Snapshot<TypeConfig>, StorageError<u64>> {
        let inner = self.inner.read().await;
        let meta = inner.snapshot_meta.clone().unwrap_or_default();
        let data = inner.snapshot_data.clone().unwrap_or_default();
        Ok(Snapshot {
            meta,
            snapshot: Box::new(Cursor::new(data)),
        })
    }
}

impl RaftStorage<TypeConfig> for InMemoryStore {
    type LogReader = Self;
    type SnapshotBuilder = Self;

    async fn save_vote(&mut self, vote: &Vote<u64>) -> Result<(), StorageError<u64>> {
        let mut inner = self.inner.write().await;
        inner.vote = Some(*vote);
        Ok(())
    }

    async fn read_vote(&mut self) -> Result<Option<Vote<u64>>, StorageError<u64>> {
        let inner = self.inner.read().await;
        Ok(inner.vote)
    }

    async fn get_log_state(&mut self) -> Result<LogState<TypeConfig>, StorageError<u64>> {
        let inner = self.inner.read().await;
        let last_log_id = inner.log.values().last().map(|e| *e.get_log_id());
        Ok(LogState {
            last_purged_log_id: inner.last_purged_log_id,
            last_log_id,
        })
    }

    async fn get_log_reader(&mut self) -> Self::LogReader {
        self.clone()
    }

    async fn append_to_log<I>(&mut self, entries: I) -> Result<(), StorageError<u64>>
    where
        I: IntoIterator<Item = openraft::Entry<TypeConfig>> + OptionalSend,
    {
        let mut inner = self.inner.write().await;
        for entry in entries {
            inner.log.insert(entry.get_log_id().index, entry);
        }
        Ok(())
    }

    async fn delete_conflict_logs_since(
        &mut self,
        log_id: LogId<u64>,
    ) -> Result<(), StorageError<u64>> {
        let mut inner = self.inner.write().await;
        let _ = inner.log.split_off(&log_id.index);
        Ok(())
    }

    async fn purge_logs_upto(&mut self, log_id: LogId<u64>) -> Result<(), StorageError<u64>> {
        let mut inner = self.inner.write().await;
        inner.last_purged_log_id = Some(log_id);
        let retained = inner.log.split_off(&(log_id.index + 1));
        inner.log = retained;
        Ok(())
    }

    async fn last_applied_state(
        &mut self,
    ) -> Result<
        (
            Option<LogId<u64>>,
            StoredMembership<u64, openraft::BasicNode>,
        ),
        StorageError<u64>,
    > {
        let inner = self.inner.read().await;
        Ok((inner.last_applied_log_id, inner.last_membership.clone()))
    }

    async fn apply_to_state_machine(
        &mut self,
        entries: &[openraft::Entry<TypeConfig>],
    ) -> Result<Vec<TaskResponse>, StorageError<u64>> {
        let mut responses = Vec::with_capacity(entries.len());

        for entry in entries {
            let log_id = *entry.get_log_id();
            {
                let mut inner = self.inner.write().await;
                inner.last_applied_log_id = Some(log_id);
            }

            match entry.payload {
                EntryPayload::Normal(ref cmd) => {
                    info!(
                        node_id = self.self_id,
                        cmd = %cmd.cmd,
                        log_index = log_id.index,
                        "Applying replicated task command to state machine"
                    );

                    let output = tokio::process::Command::new("sh")
                        .arg("-c")
                        .arg(&cmd.cmd)
                        .output()
                        .await;

                    let (exit_code, output_str) = match output {
                        Ok(out) => {
                            let code = out.status.code().unwrap_or(-1);
                            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                            (code, format!("{}{}", stdout, stderr))
                        }
                        Err(err) => (-1, format!("Execution error: {}", err)),
                    };

                    info!(
                        node_id = self.self_id,
                        cmd = %cmd.cmd,
                        exit_code,
                        "Replicated task command completed execution"
                    );

                    responses.push(TaskResponse::new(self.self_id, exit_code, output_str));
                }
                EntryPayload::Membership(ref mem) => {
                    let mut inner = self.inner.write().await;
                    inner.last_membership = StoredMembership::new(Some(log_id), mem.clone());
                    responses.push(TaskResponse::new(self.self_id, 0, "membership updated"));
                }
                EntryPayload::Blank => {
                    responses.push(TaskResponse::new(self.self_id, 0, "blank"));
                }
            }
        }

        Ok(responses)
    }

    async fn get_snapshot_builder(&mut self) -> Self::SnapshotBuilder {
        self.clone()
    }

    async fn begin_receiving_snapshot(
        &mut self,
    ) -> Result<Box<Cursor<Vec<u8>>>, StorageError<u64>> {
        Ok(Box::new(Cursor::new(Vec::new())))
    }

    async fn install_snapshot(
        &mut self,
        meta: &SnapshotMeta<u64, openraft::BasicNode>,
        snapshot: Box<Cursor<Vec<u8>>>,
    ) -> Result<(), StorageError<u64>> {
        let mut inner = self.inner.write().await;
        inner.snapshot_meta = Some(meta.clone());
        inner.snapshot_data = Some(snapshot.into_inner());
        inner.last_applied_log_id = meta.last_log_id;
        inner.last_membership = meta.last_membership.clone();
        Ok(())
    }

    async fn get_current_snapshot(
        &mut self,
    ) -> Result<Option<Snapshot<TypeConfig>>, StorageError<u64>> {
        let inner = self.inner.read().await;
        if let Some(meta) = &inner.snapshot_meta {
            let data = inner.snapshot_data.clone().unwrap_or_default();
            Ok(Some(Snapshot {
                meta: meta.clone(),
                snapshot: Box::new(Cursor::new(data)),
            }))
        } else {
            Ok(None)
        }
    }
}
