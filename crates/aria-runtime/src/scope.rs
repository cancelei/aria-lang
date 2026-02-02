//! Structured concurrency scopes for Aria.
//!
//! This module provides:
//! - `Scope` - A structured concurrency scope that ensures tasks cannot outlive their parent
//! - `CancelToken` - For cooperative cancellation
//! - Error propagation - First error cancels siblings
//!
//! # Design (ARIA-PD-006)
//!
//! Scopes provide structured concurrency guarantees:
//! - Tasks spawned in a scope cannot outlive the scope
//! - When a scope exits, all tasks are awaited
//! - First error cancels all sibling tasks
//! - Cancellation is cooperative via `CancelToken`
//!
//! # Example
//!
//! ```rust
//! use aria_runtime::scope::{Scope, with_scope};
//!
//! let result = with_scope(|scope| {
//!     let h1 = scope.spawn(|| 1 + 1);
//!     let h2 = scope.spawn(|| 2 + 2);
//!     h1.join().unwrap() + h2.join().unwrap()
//! });
//! assert_eq!(result, 6);
//! ```

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

use parking_lot::{Condvar, Mutex};

use crate::error::TaskError;
use crate::task::TaskId;

/// A token for cooperative cancellation.
///
/// Tasks can check this token periodically to see if they should stop.
/// Cancellation in Aria is cooperative - tasks must explicitly check
/// for cancellation at appropriate points.
///
/// # Example
///
/// ```rust
/// use aria_runtime::scope::CancelToken;
///
/// fn long_running_task(cancel: CancelToken) -> Result<i32, &'static str> {
///     for i in 0..1000 {
///         if cancel.is_cancelled() {
///             return Err("cancelled");
///         }
///         // Do work...
///     }
///     Ok(42)
/// }
/// ```
#[derive(Clone)]
pub struct CancelToken {
    cancelled: Arc<AtomicBool>,
}

impl CancelToken {
    /// Create a new cancel token (not cancelled).
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Check if cancellation has been requested.
    ///
    /// Uses `Acquire` ordering to ensure we see all writes that happened
    /// before the cancellation was set.
    #[inline]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    /// Request cancellation.
    ///
    /// Uses `Release` ordering to ensure all prior writes are visible
    /// to threads that subsequently observe the cancellation.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    /// Check for cancellation and return an error if cancelled.
    ///
    /// This is a convenience method for cancellation checkpoints.
    pub fn check(&self) -> Result<(), TaskError> {
        if self.is_cancelled() {
            Err(TaskError::Cancelled)
        } else {
            Ok(())
        }
    }
}

impl Default for CancelToken {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for CancelToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CancelToken")
            .field("cancelled", &self.is_cancelled())
            .finish()
    }
}

/// Internal state for a spawned task in a scope.
struct ScopedTaskInner<T> {
    /// Task identifier.
    id: TaskId,
    /// The result of the task.
    result: Mutex<Option<Result<T, TaskError>>>,
    /// Condition variable for waiting.
    completed: Condvar,
    /// Whether the task has finished.
    finished: AtomicBool,
}

impl<T> ScopedTaskInner<T> {
    fn new() -> Self {
        Self {
            id: TaskId::new(),
            result: Mutex::new(None),
            completed: Condvar::new(),
            finished: AtomicBool::new(false),
        }
    }

    fn complete(&self, result: Result<T, TaskError>) {
        *self.result.lock() = Some(result);
        // Release ensures the result write is visible before finished is set
        self.finished.store(true, Ordering::Release);
        self.completed.notify_all();
    }

    #[inline]
    fn is_finished(&self) -> bool {
        // Acquire ensures we see the result if finished is true
        self.finished.load(Ordering::Acquire)
    }

    fn wait(&self) -> Result<T, TaskError>
    where
        T: Clone,
    {
        let mut result = self.result.lock();
        while result.is_none() {
            self.completed.wait(&mut result);
        }
        result.clone().unwrap()
    }
}

/// A handle to a task spawned within a scope.
pub struct ScopedJoinHandle<T> {
    inner: Arc<ScopedTaskInner<T>>,
}

impl<T> ScopedJoinHandle<T>
where
    T: Clone,
{
    /// Get the task ID.
    pub fn id(&self) -> TaskId {
        self.inner.id
    }

    /// Check if the task has finished.
    pub fn is_finished(&self) -> bool {
        self.inner.is_finished()
    }

    /// Wait for the task to complete and return the result.
    ///
    /// If the task failed with an error, this will return that error.
    pub fn join(self) -> Result<T, TaskError> {
        self.inner.wait()
    }

    /// Wait for the task and return the result wrapped in Result.
    ///
    /// Unlike `join()`, this doesn't propagate the error - it returns
    /// it as part of the Result, allowing individual error handling.
    pub fn join_result(self) -> Result<T, TaskError> {
        self.inner.wait()
    }

    /// Try to get the result without blocking.
    pub fn try_join(&self) -> Option<Result<T, TaskError>>
    where
        T: Clone,
    {
        if self.is_finished() {
            self.inner.result.lock().clone()
        } else {
            None
        }
    }
}

impl<T: Clone> Clone for ScopedJoinHandle<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// A structured concurrency scope.
///
/// `Scope` ensures that all tasks spawned within it complete before
/// the scope exits. This is the implementation of `Async.scope` from
/// the Aria language.
///
/// # Structured Concurrency Guarantees
///
/// 1. **Lifetime**: Tasks cannot outlive the scope
/// 2. **Completion**: All tasks are awaited when scope exits
/// 3. **Error Propagation**: First error cancels siblings (default)
/// 4. **Cancellation**: Cooperative via `CancelToken`
///
/// # Example
///
/// ```rust
/// use aria_runtime::scope::with_scope;
///
/// let result = with_scope(|scope| {
///     let task1 = scope.spawn(|| 10 * 2);
///     let task2 = scope.spawn(|| 5 + 5);
///
///     task1.join().unwrap() + task2.join().unwrap()
/// });
/// assert_eq!(result, 30);
/// ```
pub struct Scope {
    /// Cancel token for this scope.
    cancel_token: CancelToken,
    /// Count of active tasks.
    active_count: Arc<AtomicUsize>,
    /// Condition variable for waiting on all tasks.
    all_completed: Arc<(Mutex<bool>, Condvar)>,
    /// First error encountered (for error propagation).
    first_error: Arc<Mutex<Option<TaskError>>>,
    /// Thread handles for cleanup (only used when use_pool is false).
    threads: Vec<thread::JoinHandle<()>>,
    /// Whether to cancel siblings on first error.
    cancel_on_error: bool,
    /// Whether to use the global thread pool for spawning.
    use_pool: bool,
}

impl Scope {
    /// Create a new scope using the global thread pool.
    ///
    /// This is the recommended way to create scopes as it reuses
    /// threads from the pool rather than spawning new OS threads.
    pub fn new() -> Self {
        Self {
            cancel_token: CancelToken::new(),
            active_count: Arc::new(AtomicUsize::new(0)),
            all_completed: Arc::new((Mutex::new(false), Condvar::new())),
            first_error: Arc::new(Mutex::new(None)),
            threads: Vec::new(),
            cancel_on_error: true,
            use_pool: true, // Default: use thread pool
        }
    }

    /// Create a scope that spawns dedicated OS threads.
    ///
    /// Use this when you need guaranteed thread isolation or
    /// when tasks may block for extended periods.
    pub fn new_threaded() -> Self {
        Self {
            cancel_token: CancelToken::new(),
            active_count: Arc::new(AtomicUsize::new(0)),
            all_completed: Arc::new((Mutex::new(false), Condvar::new())),
            first_error: Arc::new(Mutex::new(None)),
            threads: Vec::new(),
            cancel_on_error: true,
            use_pool: false, // Use dedicated threads
        }
    }

    /// Create a scope that doesn't cancel siblings on error (supervisor-like).
    pub fn new_supervised() -> Self {
        Self {
            cancel_token: CancelToken::new(),
            active_count: Arc::new(AtomicUsize::new(0)),
            all_completed: Arc::new((Mutex::new(false), Condvar::new())),
            first_error: Arc::new(Mutex::new(None)),
            threads: Vec::new(),
            cancel_on_error: false,
            use_pool: true, // Default: use thread pool
        }
    }

    /// Get the cancel token for this scope.
    pub fn cancel_token(&self) -> CancelToken {
        self.cancel_token.clone()
    }

    /// Spawn a task within this scope.
    ///
    /// The task will be cancelled if the scope is cancelled or
    /// if another task fails (when cancel_on_error is true).
    ///
    /// By default, tasks are spawned on the global thread pool for efficiency.
    /// Use `Scope::new_threaded()` if you need dedicated OS threads.
    pub fn spawn<F, T>(&mut self, f: F) -> ScopedJoinHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + Clone + 'static,
    {
        self.active_count.fetch_add(1, Ordering::AcqRel);

        let inner = Arc::new(ScopedTaskInner::new());
        let inner_clone = Arc::clone(&inner);

        let active_count = Arc::clone(&self.active_count);
        let all_completed = Arc::clone(&self.all_completed);
        let first_error = Arc::clone(&self.first_error);
        let cancel_token = self.cancel_token.clone();
        let cancel_on_error = self.cancel_on_error;

        // The task closure - same for both pool and thread spawning
        let task = move || {
            // Run the task and catch panics
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                // Check for early cancellation
                if cancel_token.is_cancelled() {
                    return Err(TaskError::Cancelled);
                }
                Ok(f())
            }));

            // Convert panic to error
            let result = match result {
                Ok(Ok(value)) => Ok(value),
                Ok(Err(e)) => Err(e),
                Err(panic) => {
                    let msg = panic
                        .downcast_ref::<String>()
                        .cloned()
                        .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                        .unwrap_or_else(|| "unknown panic".to_string());
                    Err(TaskError::Panicked(msg))
                }
            };

            // Handle error propagation
            if let Err(ref e) = result {
                if cancel_on_error && !matches!(e, TaskError::Cancelled) {
                    // Store first error and cancel other tasks
                    let mut first = first_error.lock();
                    if first.is_none() {
                        *first = Some(e.clone());
                        cancel_token.cancel();
                    }
                }
            }

            // Complete the task
            inner_clone.complete(result);

            // Decrement active count and notify if all done
            let remaining = active_count.fetch_sub(1, Ordering::AcqRel) - 1;
            if remaining == 0 {
                let (lock, cvar) = &*all_completed;
                *lock.lock() = true;
                cvar.notify_all();
            }
        };

        if self.use_pool {
            // Use the global thread pool for efficient task execution
            let _ = crate::pool::global_pool().spawn(task);
        } else {
            // Spawn a dedicated OS thread
            let handle = thread::spawn(task);
            self.threads.push(handle);
        }

        ScopedJoinHandle { inner }
    }

    /// Spawn a task that receives the cancel token.
    ///
    /// This allows the task to cooperatively check for cancellation.
    pub fn spawn_with_cancel<F, T>(&mut self, f: F) -> ScopedJoinHandle<T>
    where
        F: FnOnce(CancelToken) -> T + Send + 'static,
        T: Send + Clone + 'static,
    {
        let cancel_token = self.cancel_token.clone();
        self.spawn(move || f(cancel_token))
    }

    /// Wait for all tasks to complete.
    pub fn join_all(&mut self) {
        // Wait for all tasks to finish
        let (lock, cvar) = &*self.all_completed;
        let mut completed = lock.lock();
        while !*completed && self.active_count.load(Ordering::Acquire) > 0 {
            cvar.wait(&mut completed);
        }

        // Join all threads
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
    }

    /// Get the number of active tasks.
    #[inline]
    pub fn active_count(&self) -> usize {
        // Relaxed is fine for informational queries
        self.active_count.load(Ordering::Relaxed)
    }

    /// Check if all tasks have completed.
    #[inline]
    pub fn is_complete(&self) -> bool {
        self.active_count.load(Ordering::Acquire) == 0
    }

    /// Cancel all tasks in this scope.
    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }

    /// Get the first error if any task failed.
    pub fn first_error(&self) -> Option<TaskError> {
        self.first_error.lock().clone()
    }
}

impl Default for Scope {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        // Ensure all tasks complete before scope is dropped
        self.join_all();
    }
}

/// Execute a function with a structured concurrency scope.
///
/// This is the primary API for structured concurrency in Aria,
/// implementing `with Async.scope |scope| ... end`.
///
/// # Guarantees
///
/// - All spawned tasks will complete before this function returns
/// - First error cancels all sibling tasks
/// - Result is the value returned by the closure
///
/// # Example
///
/// ```rust
/// use aria_runtime::scope::with_scope;
///
/// let result = with_scope(|scope| {
///     let h1 = scope.spawn(|| 1 + 1);
///     let h2 = scope.spawn(|| 2 + 2);
///     h1.join().unwrap() + h2.join().unwrap()
/// });
/// assert_eq!(result, 6);
/// ```
pub fn with_scope<F, R>(f: F) -> R
where
    F: FnOnce(&mut Scope) -> R,
{
    let mut scope = Scope::new();
    let result = f(&mut scope);
    scope.join_all();
    result
}

/// Execute with a scope, returning Result if any task failed.
///
/// Unlike `with_scope`, this propagates the first error from any task.
pub fn with_scope_result<F, T>(f: F) -> Result<T, TaskError>
where
    F: FnOnce(&mut Scope) -> T,
{
    let mut scope = Scope::new();
    let result = f(&mut scope);
    scope.join_all();

    if let Some(err) = scope.first_error() {
        Err(err)
    } else {
        Ok(result)
    }
}

/// Execute with a supervised scope (errors don't cancel siblings).
///
/// This implements `with Async.supervisor |scope| ... end`.
pub fn with_supervised_scope<F, R>(f: F) -> R
where
    F: FnOnce(&mut Scope) -> R,
{
    let mut scope = Scope::new_supervised();
    let result = f(&mut scope);
    scope.join_all();
    result
}

/// Execute a function with a timeout scope.
///
/// The scope will automatically cancel all tasks if the timeout expires
/// before all tasks complete. This implements `with Async.timeout(duration) |scope| ... end`.
///
/// # Returns
///
/// - `Ok(value)` if the closure completes within the timeout
/// - `Err(TaskError::Timeout(duration))` if the timeout expires
///
/// # Example
///
/// ```rust
/// use aria_runtime::scope::with_timeout_scope;
/// use aria_runtime::error::TaskError;
/// use std::time::Duration;
///
/// // Fast operation completes successfully
/// let result = with_timeout_scope(Duration::from_secs(1), |scope| {
///     let h = scope.spawn(|| 42);
///     h.join().unwrap()
/// });
/// assert!(result.is_ok());
///
/// // Slow operation times out
/// let result = with_timeout_scope(Duration::from_millis(10), |scope| {
///     scope.spawn(|| {
///         std::thread::sleep(Duration::from_secs(10));
///         42
///     });
/// });
/// assert!(matches!(result, Err(TaskError::Timeout(_))));
/// ```
pub fn with_timeout_scope<F, R>(timeout: std::time::Duration, f: F) -> Result<R, TaskError>
where
    F: FnOnce(&mut Scope) -> R,
{
    use std::sync::atomic::{AtomicBool, Ordering};

    let mut scope = Scope::new();
    let cancel_token = scope.cancel_token();
    let timeout_triggered = Arc::new(AtomicBool::new(false));
    let timeout_triggered_clone = Arc::clone(&timeout_triggered);

    // Spawn a timeout task that will cancel the scope
    let timeout_handle = std::thread::spawn(move || {
        std::thread::sleep(timeout);
        timeout_triggered_clone.store(true, Ordering::Release);
        cancel_token.cancel();
    });

    let result = f(&mut scope);

    // Wait for all tasks to complete (or be cancelled by timeout)
    scope.join_all();

    // Check if the timeout was what triggered the cancellation
    let timed_out = timeout_triggered.load(Ordering::Acquire);

    // Clean up timeout thread
    drop(timeout_handle);

    if timed_out {
        Err(TaskError::Timeout(timeout))
    } else {
        Ok(result)
    }
}

/// Execute a function with a timeout scope, returning the result even on timeout.
///
/// Unlike `with_timeout_scope`, this returns a tuple of the result and
/// whether a timeout occurred, allowing you to use partial results.
///
/// # Example
///
/// ```rust
/// use aria_runtime::scope::with_timeout_scope_partial;
/// use std::time::Duration;
/// use std::sync::atomic::{AtomicI32, Ordering};
/// use std::sync::Arc;
///
/// let counter = Arc::new(AtomicI32::new(0));
/// let counter_clone = Arc::clone(&counter);
///
/// let (result, timed_out) = with_timeout_scope_partial(
///     Duration::from_millis(50),
///     |scope| {
///         scope.spawn_with_cancel({
///             let counter = Arc::clone(&counter_clone);
///             move |cancel| {
///                 for i in 0..100 {
///                     if cancel.is_cancelled() { break; }
///                     std::thread::sleep(Duration::from_millis(10));
///                     counter.fetch_add(1, Ordering::Relaxed);
///                 }
///             }
///         });
///         "done"
///     }
/// );
///
/// assert!(timed_out);
/// // Some iterations completed before timeout
/// assert!(counter.load(Ordering::Relaxed) > 0);
/// assert!(counter.load(Ordering::Relaxed) < 100);
/// ```
pub fn with_timeout_scope_partial<F, R>(
    timeout: std::time::Duration,
    f: F,
) -> (R, bool)
where
    F: FnOnce(&mut Scope) -> R,
{
    use std::sync::atomic::{AtomicBool, Ordering};

    let mut scope = Scope::new();
    let cancel_token = scope.cancel_token();
    let timeout_triggered = Arc::new(AtomicBool::new(false));
    let timeout_triggered_clone = Arc::clone(&timeout_triggered);

    // Spawn a timeout task
    let timeout_handle = std::thread::spawn(move || {
        std::thread::sleep(timeout);
        timeout_triggered_clone.store(true, Ordering::Release);
        cancel_token.cancel();
    });

    let result = f(&mut scope);

    // Wait for all tasks
    scope.join_all();

    // Check if timeout triggered
    let timed_out = timeout_triggered.load(Ordering::Acquire);
    drop(timeout_handle);

    (result, timed_out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicI32;
    use std::time::Duration;

    #[test]
    fn test_cancel_token_new() {
        let token = CancelToken::new();
        assert!(!token.is_cancelled());
    }

    #[test]
    fn test_cancel_token_cancel() {
        let token = CancelToken::new();
        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn test_cancel_token_check() {
        let token = CancelToken::new();
        assert!(token.check().is_ok());

        token.cancel();
        assert!(token.check().is_err());
    }

    #[test]
    fn test_cancel_token_clone() {
        let token1 = CancelToken::new();
        let token2 = token1.clone();

        token1.cancel();
        assert!(token2.is_cancelled());
    }

    #[test]
    fn test_scope_basic() {
        let result = with_scope(|scope| {
            let h1 = scope.spawn(|| 10);
            let h2 = scope.spawn(|| 20);
            h1.join().unwrap() + h2.join().unwrap()
        });
        assert_eq!(result, 30);
    }

    #[test]
    fn test_scope_structured_concurrency() {
        let completed = Arc::new(AtomicBool::new(false));
        let completed_clone = Arc::clone(&completed);

        with_scope(|scope| {
            scope.spawn(move || {
                thread::sleep(Duration::from_millis(50));
                completed_clone.store(true, Ordering::SeqCst);
            });
        });

        // Task must be complete when with_scope returns
        assert!(completed.load(Ordering::SeqCst));
    }

    #[test]
    fn test_scope_cancel_on_error() {
        let counter = Arc::new(AtomicI32::new(0));
        let counter_clone = Arc::clone(&counter);

        let result = with_scope_result(|scope| {
            // This task will fail
            scope.spawn(|| -> i32 { panic!("intentional failure") });

            // This task checks for cancellation
            scope.spawn_with_cancel(move |cancel| {
                for _ in 0..10 {
                    if cancel.is_cancelled() {
                        return;
                    }
                    thread::sleep(Duration::from_millis(10));
                    counter_clone.fetch_add(1, Ordering::SeqCst);
                }
            });

            42
        });

        // First task should have failed
        assert!(result.is_err());

        // Second task should have been cancelled (counter < 10)
        let final_count = counter.load(Ordering::SeqCst);
        assert!(final_count < 10, "Expected < 10, got {}", final_count);
    }

    #[test]
    fn test_supervised_scope_no_cancel_on_error() {
        let counter = Arc::new(AtomicI32::new(0));
        let counter_clone = Arc::clone(&counter);

        with_supervised_scope(|scope| {
            // This task will fail
            scope.spawn(|| -> i32 { panic!("intentional failure") });

            // This task should complete since we're supervised
            scope.spawn(move || {
                for _ in 0..5 {
                    thread::sleep(Duration::from_millis(10));
                    counter_clone.fetch_add(1, Ordering::SeqCst);
                }
            });
        });

        // Second task should have completed fully
        let final_count = counter.load(Ordering::SeqCst);
        assert_eq!(final_count, 5);
    }

    #[test]
    fn test_scope_spawn_with_cancel() {
        let result = with_scope(|scope| {
            let h = scope.spawn_with_cancel(|cancel| {
                if cancel.is_cancelled() {
                    -1
                } else {
                    42
                }
            });
            h.join().unwrap()
        });
        assert_eq!(result, 42);
    }

    #[test]
    fn test_scope_many_tasks() {
        let sum = with_scope(|scope| {
            let handles: Vec<_> = (0..100).map(|i| scope.spawn(move || i)).collect();

            handles.into_iter().map(|h| h.join().unwrap()).sum::<i32>()
        });
        assert_eq!(sum, 4950); // Sum of 0..100
    }

    #[test]
    fn test_scope_nested() {
        let result = with_scope(|outer| {
            let h1 = outer.spawn(|| {
                with_scope(|inner| {
                    let h = inner.spawn(|| 10);
                    h.join().unwrap()
                })
            });

            let h2 = outer.spawn(|| 20);

            h1.join().unwrap() + h2.join().unwrap()
        });
        assert_eq!(result, 30);
    }

    #[test]
    fn test_scoped_join_handle_clone() {
        with_scope(|scope| {
            let h1 = scope.spawn(|| 42);
            let h2 = h1.clone();

            assert_eq!(h1.id(), h2.id());
        });
    }

    #[test]
    fn test_scope_active_count() {
        let mut scope = Scope::new();
        assert_eq!(scope.active_count(), 0);

        let _h1 = scope.spawn(|| {
            thread::sleep(Duration::from_millis(50));
            1
        });
        let _h2 = scope.spawn(|| {
            thread::sleep(Duration::from_millis(50));
            2
        });

        // Give threads time to start
        thread::sleep(Duration::from_millis(10));
        assert!(scope.active_count() <= 2);

        scope.join_all();
        assert_eq!(scope.active_count(), 0);
    }

    #[test]
    fn test_timeout_scope_completes_in_time() {
        let result = with_timeout_scope(Duration::from_secs(1), |scope| {
            let h = scope.spawn(|| 42);
            h.join().unwrap()
        });
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_timeout_scope_times_out() {
        let result = with_timeout_scope(Duration::from_millis(20), |scope| {
            scope.spawn_with_cancel(|cancel| {
                // Long-running task that checks cancellation
                for _ in 0..100 {
                    if cancel.is_cancelled() {
                        return;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
            });
            "done"
        });

        assert!(matches!(result, Err(TaskError::Timeout(_))));
    }

    #[test]
    fn test_timeout_scope_partial() {
        let counter = Arc::new(AtomicI32::new(0));
        let counter_clone = Arc::clone(&counter);

        let (result, timed_out) = with_timeout_scope_partial(
            Duration::from_millis(50),
            |scope| {
                scope.spawn_with_cancel({
                    let counter = counter_clone;
                    move |cancel| {
                        for _ in 0..100 {
                            if cancel.is_cancelled() {
                                break;
                            }
                            thread::sleep(Duration::from_millis(10));
                            counter.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                });
                "partial"
            },
        );

        assert_eq!(result, "partial");
        assert!(timed_out);
        // Some iterations completed before timeout
        let final_count = counter.load(Ordering::Relaxed);
        assert!(final_count > 0, "Expected some iterations, got 0");
        assert!(final_count < 100, "Expected timeout before completion, got {}", final_count);
    }

    #[test]
    fn test_timeout_scope_no_timeout() {
        let (result, timed_out) = with_timeout_scope_partial(
            Duration::from_secs(1),
            |scope| {
                let h = scope.spawn(|| 123);
                h.join().unwrap()
            },
        );

        assert_eq!(result, 123);
        assert!(!timed_out);
    }
}
