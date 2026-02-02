//! Aria Runtime - Concurrency primitives for the Aria programming language
//!
//! This crate provides the core concurrency abstractions for Aria:
//! - `Task<T>` - A handle to a spawned concurrent task
//! - `JoinHandle<T>` - For awaiting task completion
//! - `TaskGroup` - Structured concurrency scope (legacy)
//! - `Scope` - Structured concurrency with cancellation (Async.scope)
//! - `AsyncEffectHandler` - Bridge from Async effect to runtime
//! - `Channel<T>` - Typed channels for inter-task communication
//! - `CancelToken` - Cooperative cancellation
//!
//! # Design Goals
//!
//! Based on ARIA-PD-006 and ARIA-PD-009:
//! - Structured concurrency: tasks cannot outlive their parent scope
//! - Target context switch: < 300ns
//! - Optional work-stealing runtime (future enhancement)
//! - Thread-per-task as initial simple implementation
//!
//! # Effect System Integration
//!
//! The `async_handler` module provides the bridge between the Async effect
//! (defined in aria-effects) and the actual runtime implementation. When
//! compiled code performs async operations, it calls into the handler methods
//! which delegate to the underlying executor.
//!
//! # Low-Level Runtime FFI
//!
//! The `runtime_ffi` module provides C-compatible FFI functions for compiled
//! Aria programs, including memory management, string operations, arrays,
//! hashmaps, I/O, and panic handling.

pub mod async_handler;
pub mod channel;
pub mod error;
pub mod executor;
pub mod ffi;
pub mod pool;
pub mod scope;
pub mod scope_debug;
pub mod task;
pub mod timer;
pub mod runtime_ffi;

pub use async_handler::{
    run_async, with_async_context, current_async_context,
    AsyncContext, AsyncEffectHandler, AsyncTaskId,
};
pub use channel::{Channel, Sender, Receiver, ChannelError, unbuffered, buffered};
pub use error::{RuntimeError, TaskError};
pub use executor::{block_on, spawn, spawn_blocking};
pub use pool::{ThreadPool, PooledJoinHandle, global_pool, pool_spawn};
pub use scope::{
    CancelToken, Scope, ScopedJoinHandle,
    with_scope, with_scope_result, with_supervised_scope,
    with_timeout_scope, with_timeout_scope_partial,
};
pub use scope_debug::{
    DebugJoinHandle, DebugScope, DebugTaskInfo, DebugTaskState, ScopeTree,
    with_debug_scope, with_debug_scope_result,
};
pub use task::{JoinHandle, Task, TaskGroup, TaskId, TaskState};
pub use timer::{
    global_timer, schedule_at, schedule_timer,
    TimerHandle, TimerWheel, TimeoutResult,
};

/// Configuration for the runtime.
///
/// Currently supports basic thread-per-task execution.
/// Future versions will support work-stealing scheduler.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Name prefix for spawned threads
    pub thread_name_prefix: String,
    /// Stack size for spawned threads (bytes)
    pub stack_size: Option<usize>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            thread_name_prefix: "aria-task".to_string(),
            stack_size: None,
        }
    }
}

impl RuntimeConfig {
    /// Create a new runtime configuration with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the thread name prefix.
    pub fn with_thread_name_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.thread_name_prefix = prefix.into();
        self
    }

    /// Set the stack size for spawned threads.
    pub fn with_stack_size(mut self, size: usize) -> Self {
        self.stack_size = Some(size);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_config_default() {
        let config = RuntimeConfig::default();
        assert_eq!(config.thread_name_prefix, "aria-task");
        assert!(config.stack_size.is_none());
    }

    #[test]
    fn test_runtime_config_builder() {
        let config = RuntimeConfig::new()
            .with_thread_name_prefix("custom")
            .with_stack_size(1024 * 1024);

        assert_eq!(config.thread_name_prefix, "custom");
        assert_eq!(config.stack_size, Some(1024 * 1024));
    }
}
