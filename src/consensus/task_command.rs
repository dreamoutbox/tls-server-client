//! Replicated task command for Raft log entries.

use serde::{Deserialize, Serialize};

/// Application command payload stored and replicated in the Raft log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskCommand {
    /// Shell command or executable to execute across cluster nodes.
    pub cmd: String,
}

impl TaskCommand {
    /// Creates a new task command.
    ///
    /// # Arguments
    /// * `cmd` - Command string to execute.
    ///
    /// # Example
    /// ```rust
    /// use tls_server_client::consensus::TaskCommand;
    ///
    /// let cmd = TaskCommand::new("echo hello");
    /// assert_eq!(cmd.cmd, "echo hello");
    /// ```
    pub fn new(cmd: impl Into<String>) -> Self {
        Self { cmd: cmd.into() }
    }
}
