//! Task types and handles for Aria concurrency.
//!
//! This module provides:
//! - `Task<T>` - A handle to a spawned task with result type T
//! - `JoinHandle<T>` - Low-level handle for awaiting task completion
//! - `TaskGroup` - Structured concurrency scope
//! - `TaskId` - Unique identifier for tasks

use std::any::Any;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle as StdJoinHandle;

use parking_lot::{Condvar, Mutex};

use crate::error::TaskError;

/// Unique identifier for a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(u64);

impl TaskId {
    /// Generate a new unique task ID.
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        TaskId(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Get the raw ID value.
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Task({})", self.0)
    }
}

/// State of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Task is pending execution.
    Pending,
    /// Task is currently running.
    Running,
    /// Task completed successfully.
    Completed,
    /// Task was cancelled.
    Cancelled,
    /// Task panicked.
    Panicked,
}

/// Internal shared state for a task.
struct TaskInner<T> {
    /// Task identifier.
    id: TaskId,
    /// Current state of the task.
    state: Mutex<TaskState>,
    /// The result of the task, set when completed.
    result: Mutex<Option<Result<T, TaskError>>>,
    /// Condition variable for waiting on completion.
    completed: Condvar,
}

impl<T> TaskInner<T> {
    fn new(id: TaskId) -> Self {
        Self {
            id,
            state: Mutex::new(TaskState::Pending),
            result: Mutex::new(None),
            completed: Condvar::new(),
        }
    }

    fn set_running(&self) {
        *self.state.lock() = TaskState::Running;
    }

    fn complete(&self, result: Result<T, TaskError>) {
        let new_state = match &result {
            Ok(_) => TaskState::Completed,
            Err(TaskError::Cancelled) => TaskState::Cancelled,
            Err(TaskError::Panicked(_)) => TaskState::Panicked,
            Err(_) => TaskState::Completed, // Other errors still count as "completed"
        };

        *self.state.lock() = new_state;
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

    #[allow(dead_code)]
    fn try_take(&self) -> Option<Result<T, TaskError>> {
        self.result.lock().take()
    }

    fn state(&self) -> TaskState {
        *self.state.lock()
    }

    fn is_finished(&self) -> bool {
        matches!(
            self.state(),
            TaskState::Completed | TaskState::Cancelled | TaskState::Panicked
        )
    }
}

/// A handle to a spawned task.
///
/// `Task<T>` represents a concurrent computation that will eventually
/// produce a value of type `T`. It provides methods for:
/// - Waiting for the result (`.await` in Aria syntax)
/// - Checking completion status
/// - Getting the task ID for debugging
///
/// # Example (Aria syntax)
///
/// ```aria
/// task = spawn expensive_computation()
/// result = task.await  // Block until complete
/// ```
///
/// # Structured Concurrency
///
/// Tasks created via `TaskGroup` are guaranteed to complete before
/// the group scope exits, ensuring structured concurrency.
pub struct Task<T> {
    inner: Arc<TaskInner<T>>,
    /// The thread handle, if this task owns the thread.
    thread: Option<StdJoinHandle<()>>,
}

impl<T> Task<T>
where
    T: Send + 'static,
{
    /// Create a new task in pending state.
    pub(crate) fn new() -> (Self, TaskHandle<T>) {
        let id = TaskId::new();
        let inner = Arc::new(TaskInner::new(id));

        let task = Task {
            inner: Arc::clone(&inner),
            thread: None,
        };

        let handle = TaskHandle { inner };

        (task, handle)
    }

    /// Set the thread handle for this task.
    pub(crate) fn set_thread(&mut self, thread: StdJoinHandle<()>) {
        self.thread = Some(thread);
    }

    /// Get the task's unique identifier.
    pub fn id(&self) -> TaskId {
        self.inner.id
    }

    /// Get the current state of the task.
    pub fn state(&self) -> TaskState {
        self.inner.state()
    }

    /// Check if the task has finished (completed, cancelled, or panicked).
    pub fn is_finished(&self) -> bool {
        self.inner.is_finished()
    }

    /// Wait for the task to complete and return the result.
    ///
    /// This is equivalent to `.await` in Aria syntax.
    pub fn join(mut self) -> Result<T, TaskError>
    where
        T: Clone,
    {
        // First, wait for the thread to finish
        if let Some(thread) = self.thread.take() {
            thread.join().map_err(|e| {
                TaskError::Panicked(
                    e.downcast_ref::<String>()
                        .cloned()
                        .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                        .unwrap_or_else(|| "unknown panic".to_string()),
                )
            })?;
        }

        // Then get the result
        self.inner.wait()
    }

    /// Try to get the result without blocking.
    ///
    /// Returns `None` if the task hasn't completed yet.
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

/// Internal handle used by the executor to complete a task.
pub(crate) struct TaskHandle<T> {
    inner: Arc<TaskInner<T>>,
}

impl<T> TaskHandle<T> {
    /// Mark the task as running.
    pub fn set_running(&self) {
        self.inner.set_running();
    }

    /// Complete the task with a result.
    pub fn complete(&self, result: Result<T, TaskError>) {
        self.inner.complete(result);
    }

    /// Complete the task with a successful value.
    pub fn complete_ok(&self, value: T) {
        self.complete(Ok(value));
    }

    /// Complete the task with an error.
    pub fn complete_err(&self, error: TaskError) {
        self.complete(Err(error));
    }
}

/// A low-level handle for awaiting task completion.
///
/// Unlike `Task<T>`, `JoinHandle<T>` doesn't own the thread and
/// is primarily used for internal task management.
pub struct JoinHandle<T> {
    inner: Arc<TaskInner<T>>,
}

impl<T> JoinHandle<T>
where
    T: Clone,
{
    /// Create a join handle from a task.
    pub fn from_task(task: &Task<T>) -> Self
    where
        T: Send + 'static,
    {
        JoinHandle {
            inner: Arc::clone(&task.inner),
        }
    }

    /// Get the task ID.
    pub fn id(&self) -> TaskId {
        self.inner.id
    }

    /// Check if the task has finished.
    pub fn is_finished(&self) -> bool {
        self.inner.is_finished()
    }

    /// Wait for the task to complete.
    pub fn join(&self) -> Result<T, TaskError> {
        self.inner.wait()
    }
}

impl<T: Clone> Clone for JoinHandle<T> {
    fn clone(&self) -> Self {
        JoinHandle {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// A group of tasks for structured concurrency.
///
/// `TaskGroup` ensures that all spawned tasks complete before
/// the group is dropped, implementing structured concurrency
/// as specified in ARIA-PD-006.
///
/// # Example (Aria syntax)
///
/// ```aria
/// with Async.scope |scope|
///   profile = scope.spawn fetch_profile(user_id)
///   posts = scope.spawn fetch_posts(user_id)
///
///   UserData(
///     profile: profile.await,
///     posts: posts.await
///   )
/// end
/// # All tasks guaranteed complete here
/// ```
pub struct TaskGroup {
    /// Tasks in this group.
    tasks: Vec<Box<dyn Any + Send>>,
    /// Count of active (non-completed) tasks.
    active_count: Arc<AtomicUsize>,
    /// Condition variable for waiting on all tasks.
    all_completed: Arc<(Mutex<bool>, Condvar)>,
}

impl TaskGroup {
    /// Create a new empty task group.
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            active_count: Arc::new(AtomicUsize::new(0)),
            all_completed: Arc::new((Mutex::new(false), Condvar::new())),
        }
    }

    /// Spawn a task within this group.
    ///
    /// The task is guaranteed to complete before the group scope exits.
    pub fn spawn<F, T>(&mut self, f: F) -> JoinHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + Clone + 'static,
    {
        self.active_count.fetch_add(1, Ordering::SeqCst);
        let active_count = Arc::clone(&self.active_count);
        let all_completed = Arc::clone(&self.all_completed);

        // Wrap the function to track completion
        let wrapped = move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

            // Decrement active count and notify if all done
            let remaining = active_count.fetch_sub(1, Ordering::SeqCst) - 1;
            if remaining == 0 {
                let (lock, cvar) = &*all_completed;
                *lock.lock() = true;
                cvar.notify_all();
            }

            match result {
                Ok(value) => value,
                Err(panic) => {
                    // Re-panic to let the task handle it
                    std::panic::resume_unwind(panic);
                }
            }
        };

        let task = crate::executor::spawn(wrapped);
        let handle = JoinHandle::from_task(&task);
        self.tasks.push(Box::new(task));
        handle
    }

    /// Wait for all tasks in the group to complete.
    pub fn join_all(&self) {
        let (lock, cvar) = &*self.all_completed;
        let mut completed = lock.lock();
        while !*completed && self.active_count.load(Ordering::SeqCst) > 0 {
            cvar.wait(&mut completed);
        }
    }

    /// Get the number of active tasks.
    pub fn active_count(&self) -> usize {
        self.active_count.load(Ordering::SeqCst)
    }

    /// Check if all tasks have completed.
    pub fn is_complete(&self) -> bool {
        self.active_count.load(Ordering::SeqCst) == 0
    }
}

impl Default for TaskGroup {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TaskGroup {
    fn drop(&mut self) {
        // Ensure all tasks complete before the group is dropped
        // This enforces structured concurrency
        self.join_all();
    }
}

/// Execute a function with a task group scope.
///
/// This is the primary API for structured concurrency, ensuring
/// all spawned tasks complete before the scope exits.
///
/// # Example
///
/// ```rust
/// use aria_runtime::task::with_task_group;
///
/// let results = with_task_group(|group| {
///     let h1 = group.spawn(|| 1 + 1);
///     let h2 = group.spawn(|| 2 + 2);
///     (h1.join().unwrap(), h2.join().unwrap())
/// });
/// assert_eq!(results, (2, 4));
/// ```
pub fn with_task_group<F, R>(f: F) -> R
where
    F: FnOnce(&mut TaskGroup) -> R,
{
    let mut group = TaskGroup::new();
    let result = f(&mut group);
    group.join_all();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_task_id_unique() {
        let id1 = TaskId::new();
        let id2 = TaskId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_task_id_display() {
        let id = TaskId(42);
        assert_eq!(format!("{}", id), "Task(42)");
    }

    #[test]
    fn test_task_state_transitions() {
        let inner: TaskInner<i32> = TaskInner::new(TaskId::new());

        assert_eq!(inner.state(), TaskState::Pending);

        inner.set_running();
        assert_eq!(inner.state(), TaskState::Running);

        inner.complete(Ok(42));
        assert_eq!(inner.state(), TaskState::Completed);
        assert!(inner.is_finished());
    }

    #[test]
    fn test_task_cancelled_state() {
        let inner: TaskInner<i32> = TaskInner::new(TaskId::new());
        inner.complete(Err(TaskError::Cancelled));
        assert_eq!(inner.state(), TaskState::Cancelled);
    }

    #[test]
    fn test_task_group_basic() {
        let results = with_task_group(|group| {
            let h1 = group.spawn(|| 10);
            let h2 = group.spawn(|| 20);
            (h1.join().unwrap(), h2.join().unwrap())
        });

        assert_eq!(results, (10, 20));
    }

    #[test]
    fn test_task_group_structured_concurrency() {
        use std::sync::atomic::AtomicBool;

        let completed = Arc::new(AtomicBool::new(false));
        let completed_clone = Arc::clone(&completed);

        with_task_group(|group| {
            group.spawn(move || {
                thread::sleep(Duration::from_millis(50));
                completed_clone.store(true, Ordering::SeqCst);
            });
        });

        // Task must be complete when with_task_group returns
        assert!(completed.load(Ordering::SeqCst));
    }

    #[test]
    fn test_join_handle_clone() {
        let (task, _handle) = Task::<i32>::new();
        let join = JoinHandle::from_task(&task);
        let join_clone = join.clone();

        assert_eq!(join.id(), join_clone.id());
    }
}
