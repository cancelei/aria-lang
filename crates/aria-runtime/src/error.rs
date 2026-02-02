//! Error types for the Aria runtime.

use thiserror::Error;

/// Errors that can occur during runtime operations.
#[derive(Debug, Error)]
pub enum RuntimeError {
    /// The runtime has been shut down.
    #[error("runtime has been shut down")]
    Shutdown,

    /// Failed to spawn a task.
    #[error("failed to spawn task: {0}")]
    SpawnFailed(String),

    /// Task execution error.
    #[error("task error: {0}")]
    TaskError(#[from] TaskError),

    /// Channel operation error.
    #[error("channel error: {0}")]
    Channel(String),
}

/// Errors that can occur during task execution.
#[derive(Debug, Error, Clone)]
pub enum TaskError {
    /// The task was cancelled before completion.
    #[error("task was cancelled")]
    Cancelled,

    /// The task panicked during execution.
    #[error("task panicked: {0}")]
    Panicked(String),

    /// Failed to join the task (thread join error).
    #[error("failed to join task: {0}")]
    JoinError(String),

    /// The task has already been joined.
    #[error("task has already been joined")]
    AlreadyJoined,

    /// A task in the group failed.
    #[error("task group failure: {0}")]
    GroupFailure(String),

    /// The operation timed out.
    #[error("operation timed out after {0:?}")]
    Timeout(std::time::Duration),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = TaskError::Cancelled;
        assert_eq!(err.to_string(), "task was cancelled");

        let err = TaskError::Panicked("something went wrong".to_string());
        assert_eq!(err.to_string(), "task panicked: something went wrong");
    }
}
