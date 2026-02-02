//! Thread pool for Aria runtime.
//!
//! Provides a work-stealing thread pool optimized for structured concurrency.
//! Tasks are submitted to the pool and executed by worker threads.
//!
//! # Design
//!
//! - Fixed number of worker threads (defaults to available parallelism)
//! - Global task queue with local work-stealing
//! - Graceful shutdown on drop
//! - Integration with `Scope` for structured concurrency
//!
//! # Example
//!
//! ```rust
//! use aria_runtime::pool::ThreadPool;
//!
//! let pool = ThreadPool::new();
//! let handle = pool.spawn(|| 1 + 1);
//! assert_eq!(handle.join().unwrap(), 2);
//! ```

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle as StdJoinHandle};

use crossbeam_deque::{Injector, Stealer, Worker};
use parking_lot::{Condvar, Mutex};

use crate::error::TaskError;
use crate::task::TaskId;

/// A task to be executed by the thread pool.
type BoxedTask = Box<dyn FnOnce() + Send + 'static>;

/// Internal state for a pooled task result.
struct TaskResult<T> {
    result: Mutex<Option<Result<T, TaskError>>>,
    completed: Condvar,
}

impl<T> TaskResult<T> {
    fn new() -> Self {
        Self {
            result: Mutex::new(None),
            completed: Condvar::new(),
        }
    }

    fn complete(&self, result: Result<T, TaskError>) {
        *self.result.lock() = Some(result);
        self.completed.notify_all();
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

    fn is_complete(&self) -> bool {
        self.result.lock().is_some()
    }
}

/// Handle for awaiting a pooled task's completion.
pub struct PooledJoinHandle<T> {
    id: TaskId,
    result: Arc<TaskResult<T>>,
}

impl<T> PooledJoinHandle<T>
where
    T: Clone,
{
    /// Get the task ID.
    pub fn id(&self) -> TaskId {
        self.id
    }

    /// Check if the task has completed.
    pub fn is_complete(&self) -> bool {
        self.result.is_complete()
    }

    /// Wait for the task to complete and return the result.
    pub fn join(self) -> Result<T, TaskError> {
        self.result.wait()
    }

    /// Try to get the result without blocking.
    pub fn try_join(&self) -> Option<Result<T, TaskError>>
    where
        T: Clone,
    {
        let result = self.result.result.lock();
        result.clone()
    }
}

impl<T: Clone> Clone for PooledJoinHandle<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            result: Arc::clone(&self.result),
        }
    }
}

/// Shared state for the thread pool.
struct PoolInner {
    /// Global task queue (for external submissions).
    global_queue: Injector<BoxedTask>,
    /// Stealers for each worker's local queue.
    stealers: Vec<Stealer<BoxedTask>>,
    /// Number of active tasks.
    active_tasks: AtomicUsize,
    /// Shutdown flag.
    shutdown: AtomicBool,
    /// Condition variable for workers waiting for tasks.
    task_available: Condvar,
    /// Mutex for the condition variable.
    task_mutex: Mutex<()>,
    /// Number of workers.
    num_workers: usize,
}

impl PoolInner {
    /// Find and steal a task from another worker or the global queue.
    fn find_task(&self, local: &Worker<BoxedTask>, worker_id: usize) -> Option<BoxedTask> {
        // Try local queue first
        if let Some(task) = local.pop() {
            return Some(task);
        }

        // Try global queue
        loop {
            match self.global_queue.steal() {
                crossbeam_deque::Steal::Success(task) => return Some(task),
                crossbeam_deque::Steal::Empty => break,
                crossbeam_deque::Steal::Retry => continue,
            }
        }

        // Try stealing from other workers
        let start = worker_id;
        for i in 0..self.stealers.len() {
            let idx = (start + i + 1) % self.stealers.len();
            if idx == worker_id {
                continue;
            }
            loop {
                match self.stealers[idx].steal() {
                    crossbeam_deque::Steal::Success(task) => return Some(task),
                    crossbeam_deque::Steal::Empty => break,
                    crossbeam_deque::Steal::Retry => continue,
                }
            }
        }

        None
    }

    /// Notify workers that a task is available.
    fn notify_task_available(&self) {
        self.task_available.notify_one();
    }

    /// Notify all workers (for shutdown).
    fn notify_all(&self) {
        self.task_available.notify_all();
    }
}

/// A work-stealing thread pool.
///
/// The pool maintains a fixed number of worker threads that execute
/// submitted tasks. Tasks can be spawned from any thread and will
/// be executed by an available worker.
///
/// # Shutdown
///
/// When the pool is dropped, it signals all workers to shut down
/// and waits for them to complete their current tasks.
pub struct ThreadPool {
    inner: Arc<PoolInner>,
    workers: Mutex<Vec<StdJoinHandle<()>>>,
}

impl ThreadPool {
    /// Create a new thread pool with the default number of workers.
    ///
    /// The default is `std::thread::available_parallelism()`.
    pub fn new() -> Self {
        let num_workers = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        Self::with_workers(num_workers)
    }

    /// Create a thread pool with a specific number of workers.
    pub fn with_workers(num_workers: usize) -> Self {
        assert!(num_workers > 0, "thread pool must have at least 1 worker");

        // Create local queues for each worker
        let local_queues: Vec<Worker<BoxedTask>> =
            (0..num_workers).map(|_| Worker::new_fifo()).collect();

        // Create stealers from local queues
        let stealers: Vec<Stealer<BoxedTask>> =
            local_queues.iter().map(|w| w.stealer()).collect();

        let inner = Arc::new(PoolInner {
            global_queue: Injector::new(),
            stealers,
            active_tasks: AtomicUsize::new(0),
            shutdown: AtomicBool::new(false),
            task_available: Condvar::new(),
            task_mutex: Mutex::new(()),
            num_workers,
        });

        // Spawn worker threads
        let mut workers = Vec::with_capacity(num_workers);
        for (worker_id, local_queue) in local_queues.into_iter().enumerate() {
            let inner = Arc::clone(&inner);
            let worker = thread::Builder::new()
                .name(format!("aria-pool-{}", worker_id))
                .spawn(move || {
                    worker_loop(inner, local_queue, worker_id);
                })
                .expect("failed to spawn worker thread");
            workers.push(worker);
        }

        Self {
            inner,
            workers: Mutex::new(workers),
        }
    }

    /// Spawn a task on the thread pool.
    ///
    /// Returns a handle that can be used to await the task's completion.
    pub fn spawn<F, T>(&self, f: F) -> PooledJoinHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + Clone + 'static,
    {
        let id = TaskId::new();
        let result = Arc::new(TaskResult::new());
        let result_clone = Arc::clone(&result);

        // Wrap the task to capture the result
        let task: BoxedTask = Box::new(move || {
            let task_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
            let task_result = match task_result {
                Ok(value) => Ok(value),
                Err(panic) => {
                    let msg = panic
                        .downcast_ref::<String>()
                        .cloned()
                        .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                        .unwrap_or_else(|| "unknown panic".to_string());
                    Err(TaskError::Panicked(msg))
                }
            };
            result_clone.complete(task_result);
        });

        // Submit to global queue
        self.inner.active_tasks.fetch_add(1, Ordering::AcqRel);
        self.inner.global_queue.push(task);
        self.inner.notify_task_available();

        PooledJoinHandle { id, result }
    }

    /// Get the number of worker threads.
    pub fn num_workers(&self) -> usize {
        self.inner.num_workers
    }

    /// Get the number of active (not yet completed) tasks.
    pub fn active_tasks(&self) -> usize {
        self.inner.active_tasks.load(Ordering::Relaxed)
    }

    /// Check if the pool is shutting down.
    pub fn is_shutdown(&self) -> bool {
        self.inner.shutdown.load(Ordering::Acquire)
    }
}

impl Default for ThreadPool {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        // Signal shutdown
        self.inner.shutdown.store(true, Ordering::Release);
        self.inner.notify_all();

        // Wait for all workers to finish
        let mut workers = self.workers.lock();
        for worker in workers.drain(..) {
            let _ = worker.join();
        }
    }
}

/// Worker thread loop.
fn worker_loop(inner: Arc<PoolInner>, local: Worker<BoxedTask>, worker_id: usize) {
    loop {
        // Try to find a task
        if let Some(task) = inner.find_task(&local, worker_id) {
            // Execute the task
            task();
            inner.active_tasks.fetch_sub(1, Ordering::AcqRel);
            continue;
        }

        // No task found, check for shutdown
        if inner.shutdown.load(Ordering::Acquire) {
            break;
        }

        // Wait for a task or shutdown
        let mut guard = inner.task_mutex.lock();
        // Double-check after acquiring lock
        if inner.shutdown.load(Ordering::Acquire) {
            break;
        }
        // Check again if there's a task before waiting - if found, drop lock and retry
        if let Some(task) = inner.find_task(&local, worker_id) {
            drop(guard);
            task();
            inner.active_tasks.fetch_sub(1, Ordering::AcqRel);
            continue;
        }
        // Wait with timeout to periodically check for tasks
        inner.task_available.wait_for(&mut guard, std::time::Duration::from_millis(1));
    }
}

/// Global thread pool instance.
static GLOBAL_POOL: std::sync::OnceLock<ThreadPool> = std::sync::OnceLock::new();

/// Get the global thread pool.
///
/// The pool is lazily initialized on first access.
pub fn global_pool() -> &'static ThreadPool {
    GLOBAL_POOL.get_or_init(ThreadPool::new)
}

/// Spawn a task on the global thread pool.
///
/// This is a convenience function equivalent to `global_pool().spawn(f)`.
pub fn pool_spawn<F, T>(f: F) -> PooledJoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + Clone + 'static,
{
    global_pool().spawn(f)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicI32;
    use std::time::Duration;

    #[test]
    fn test_pool_basic() {
        let pool = ThreadPool::with_workers(2);
        let handle = pool.spawn(|| 42);
        assert_eq!(handle.join().unwrap(), 42);
    }

    #[test]
    fn test_pool_multiple_tasks() {
        let pool = ThreadPool::with_workers(4);
        let handles: Vec<_> = (0..100).map(|i| pool.spawn(move || i * 2)).collect();

        let sum: i32 = handles.into_iter().map(|h| h.join().unwrap()).sum();
        assert_eq!(sum, (0..100).map(|i| i * 2).sum());
    }

    #[test]
    fn test_pool_concurrent_execution() {
        let pool = ThreadPool::with_workers(4);
        let counter = Arc::new(AtomicI32::new(0));

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let counter = Arc::clone(&counter);
                pool.spawn(move || {
                    thread::sleep(Duration::from_millis(10));
                    counter.fetch_add(1, Ordering::Relaxed);
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(counter.load(Ordering::Relaxed), 10);
    }

    #[test]
    fn test_pool_panic_handling() {
        let pool = ThreadPool::with_workers(2);
        let handle = pool.spawn(|| -> i32 { panic!("intentional panic") });

        let result = handle.join();
        assert!(result.is_err());
        if let Err(TaskError::Panicked(msg)) = result {
            assert!(msg.contains("intentional panic"));
        } else {
            panic!("expected Panicked error");
        }
    }

    #[test]
    fn test_pool_shutdown() {
        let pool = ThreadPool::with_workers(2);
        let _handle = pool.spawn(|| {
            thread::sleep(Duration::from_millis(50));
            42
        });

        // Drop pool - should wait for task to complete
        drop(pool);
        // If we get here, shutdown completed successfully
    }

    #[test]
    fn test_pool_num_workers() {
        let pool = ThreadPool::with_workers(8);
        assert_eq!(pool.num_workers(), 8);
    }

    #[test]
    fn test_pooled_join_handle_clone() {
        let pool = ThreadPool::with_workers(2);
        let handle1 = pool.spawn(|| 42);
        let handle2 = handle1.clone();

        assert_eq!(handle1.id(), handle2.id());
        assert_eq!(handle1.join().unwrap(), 42);
    }

    #[test]
    fn test_pool_try_join() {
        let pool = ThreadPool::with_workers(2);
        let handle = pool.spawn(|| {
            thread::sleep(Duration::from_millis(50));
            42
        });

        // Should return None immediately
        assert!(handle.try_join().is_none());

        // Wait and try again
        thread::sleep(Duration::from_millis(100));
        assert_eq!(handle.try_join().unwrap().unwrap(), 42);
    }

    #[test]
    fn test_global_pool() {
        let handle = pool_spawn(|| 123);
        assert_eq!(handle.join().unwrap(), 123);
    }

    #[test]
    fn test_pool_work_stealing() {
        // Spawn many tasks to trigger work stealing
        let pool = ThreadPool::with_workers(4);
        let counter = Arc::new(AtomicI32::new(0));

        let handles: Vec<_> = (0..1000)
            .map(|_| {
                let counter = Arc::clone(&counter);
                pool.spawn(move || {
                    counter.fetch_add(1, Ordering::Relaxed);
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(counter.load(Ordering::Relaxed), 1000);
    }
}
