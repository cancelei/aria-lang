//! Basic executor for Aria concurrency.
//!
//! This module provides the initial thread-per-task executor implementation.
//! Future versions will include a work-stealing scheduler as specified in
//! ARIA-PD-009.
//!
//! # Implementation Notes
//!
//! The current implementation uses one OS thread per spawned task.
//! This is simple but not optimal for high task counts. The design
//! documents specify the following targets for a future work-stealing
//! implementation:
//!
//! - Context switch: < 300ns
//! - Task spawn overhead: < 50ns
//! - 1M concurrent tasks: < 1GB memory

use std::thread;

use crate::error::TaskError;
use crate::task::Task;
use crate::RuntimeConfig;

/// Global runtime configuration.
///
/// In the future, this will be replaced by a proper runtime instance.
static CONFIG: std::sync::OnceLock<RuntimeConfig> = std::sync::OnceLock::new();

/// Initialize the runtime with the given configuration.
///
/// This should be called once at program startup. If not called,
/// default configuration is used.
pub fn init(config: RuntimeConfig) {
    let _ = CONFIG.set(config);
}

/// Get the current runtime configuration.
fn get_config() -> &'static RuntimeConfig {
    CONFIG.get_or_init(RuntimeConfig::default)
}

/// Spawn a new task that will run concurrently.
///
/// Returns a `Task<T>` handle that can be used to await the result.
///
/// # Thread-per-task Implementation
///
/// Currently, each spawned task runs in its own OS thread. This is
/// simple and correct but has higher overhead than the planned
/// work-stealing scheduler.
///
/// # Example
///
/// ```rust
/// use aria_runtime::spawn;
///
/// let task = spawn(|| {
///     // Expensive computation
///     42
/// });
///
/// let result = task.join().unwrap();
/// assert_eq!(result, 42);
/// ```
///
/// # Panics
///
/// If the spawned function panics, the panic is caught and converted
/// to a `TaskError::Panicked`. The original panic payload is preserved
/// in the error message when possible.
pub fn spawn<F, T>(f: F) -> Task<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + Clone + 'static,
{
    let (mut task, handle) = Task::new();
    let config = get_config();

    let thread_name = format!("{}-{}", config.thread_name_prefix, task.id().as_u64());

    let mut builder = thread::Builder::new().name(thread_name);

    if let Some(stack_size) = config.stack_size {
        builder = builder.stack_size(stack_size);
    }

    let thread = builder
        .spawn(move || {
            handle.set_running();

            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

            match result {
                Ok(value) => {
                    handle.complete_ok(value);
                }
                Err(panic) => {
                    let message = panic
                        .downcast_ref::<String>()
                        .cloned()
                        .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                        .unwrap_or_else(|| "unknown panic".to_string());

                    handle.complete_err(TaskError::Panicked(message));
                }
            }
        })
        .expect("failed to spawn thread");

    task.set_thread(thread);
    task
}

/// Spawn a blocking task on a separate thread.
///
/// This is used for blocking operations (like synchronous I/O or FFI calls)
/// that shouldn't block the async runtime.
///
/// In the current thread-per-task implementation, this is equivalent to
/// `spawn`. However, when a work-stealing scheduler is implemented,
/// `spawn_blocking` will use a dedicated thread pool to avoid blocking
/// worker threads.
///
/// # Example
///
/// ```rust
/// use aria_runtime::spawn_blocking;
///
/// let task = spawn_blocking(|| {
///     // Blocking I/O operation
///     std::fs::read_to_string("README.md").ok()
/// });
/// ```
pub fn spawn_blocking<F, T>(f: F) -> Task<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + Clone + 'static,
{
    // For now, just delegate to spawn.
    // In the future, this will use a dedicated blocking thread pool.
    spawn(f)
}

/// Run a function on the current thread, blocking until completion.
///
/// This is the entry point for running concurrent code from a
/// synchronous context.
///
/// # Example
///
/// ```rust
/// use aria_runtime::{block_on, spawn};
///
/// block_on(|| {
///     let t1 = spawn(|| 1);
///     let t2 = spawn(|| 2);
///     t1.join().unwrap() + t2.join().unwrap()
/// });
/// ```
pub fn block_on<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    f()
}

/// Yield control to allow other tasks to run.
///
/// In the current thread-per-task implementation, this is a no-op since
/// each task has its own thread. When a work-stealing scheduler is
/// implemented, this will yield to the scheduler.
///
/// # Example
///
/// ```rust
/// use aria_runtime::executor::yield_now;
///
/// for i in 0..1000 {
///     if i % 100 == 0 {
///         yield_now();
///     }
///     // Process item
/// }
/// ```
pub fn yield_now() {
    // In thread-per-task mode, yield to other OS threads
    thread::yield_now();
}

/// Get the number of available CPU cores.
///
/// This is useful for determining the default number of worker threads
/// for a multi-threaded runtime.
pub fn available_parallelism() -> usize {
    thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn test_spawn_simple() {
        let task = spawn(|| 42);
        assert_eq!(task.join().unwrap(), 42);
    }

    #[test]
    fn test_spawn_with_closure_capture() {
        let value = 100;
        let task = spawn(move || value * 2);
        assert_eq!(task.join().unwrap(), 200);
    }

    #[test]
    fn test_spawn_multiple_tasks() {
        let t1 = spawn(|| 1);
        let t2 = spawn(|| 2);
        let t3 = spawn(|| 3);

        assert_eq!(t1.join().unwrap() + t2.join().unwrap() + t3.join().unwrap(), 6);
    }

    #[test]
    fn test_spawn_concurrent_execution() {
        let counter = Arc::new(AtomicUsize::new(0));

        let tasks: Vec<_> = (0..10)
            .map(|_| {
                let counter = Arc::clone(&counter);
                spawn(move || {
                    counter.fetch_add(1, Ordering::SeqCst);
                })
            })
            .collect();

        for task in tasks {
            task.join().unwrap();
        }

        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }

    #[test]
    fn test_spawn_panic_handling() {
        let task = spawn(|| {
            panic!("intentional panic");
        });

        let result: Result<(), TaskError> = task.join();
        assert!(matches!(result, Err(TaskError::Panicked(_))));
    }

    #[test]
    fn test_task_state_progression() {
        let task = spawn(|| {
            thread::sleep(Duration::from_millis(50));
            42
        });

        // Task should eventually complete
        let result = task.join().unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_block_on() {
        let result = block_on(|| {
            let t1 = spawn(|| 10);
            let t2 = spawn(|| 20);
            t1.join().unwrap() + t2.join().unwrap()
        });

        assert_eq!(result, 30);
    }

    #[test]
    fn test_spawn_blocking() {
        let task = spawn_blocking(|| {
            // Simulate blocking I/O
            thread::sleep(Duration::from_millis(10));
            "done"
        });

        assert_eq!(task.join().unwrap(), "done");
    }

    #[test]
    fn test_available_parallelism() {
        let cores = available_parallelism();
        assert!(cores >= 1);
    }

    #[test]
    fn test_yield_now() {
        // Just ensure it doesn't panic
        yield_now();
    }

    #[test]
    fn test_spawn_string_result() {
        let task = spawn(|| "hello".to_string());
        assert_eq!(task.join().unwrap(), "hello");
    }

    #[test]
    fn test_spawn_with_heavy_computation() {
        let task = spawn(|| {
            let mut sum = 0u64;
            for i in 0..10000 {
                sum += i;
            }
            sum
        });

        assert_eq!(task.join().unwrap(), 49995000);
    }
}
