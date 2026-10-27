//! Task execution response returned from state machine application.

use serde::{Deserialize, Serialize};

/// Result returned after executing a replicated [`TaskCommand`] in the state machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskResponse {
    /// Node identifier where the task executed.
    pub node_id: u64,
    /// Process exit code (0 indicates success).
    pub exit_code: i32,
    /// Standard output/stderr produced by the command execution.
    pub output: String,
}

impl TaskResponse {
    /// Creates a new task response.
    ///
    /// # Arguments
    /// * `node_id` - Identifier of the executing node.
    /// * `exit_code` - Status exit code of the executed process.
    /// * `output` - Execution output text.
    ///
    /// # Example
    /// ```rust
    /// use tls_server_client::consensus::TaskResponse;
    ///
    /// let res = TaskResponse::new(1, 0, "hello\n");
    /// assert_eq!(res.exit_code, 0);
    /// ```
    pub fn new(node_id: u64, exit_code: i32, output: impl Into<String>) -> Self {
        Self {
            node_id,
            exit_code,
            output: output.into(),
        }
    }
}
