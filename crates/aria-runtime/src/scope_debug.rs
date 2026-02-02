//! Debugging and visualization tools for structured concurrency scopes.
//!
//! This module provides tools to inspect and visualize the state of
//! concurrent scopes and their tasks, useful for debugging and monitoring.
//!
//! # Example
//!
//! ```rust
//! use aria_runtime::scope_debug::{DebugScope, with_debug_scope};
//!
//! let tree = with_debug_scope("fetch_user_data", |scope| {
//!     let _t1 = scope.spawn_named("fetch_profile", || 1);
//!     let _t2 = scope.spawn_named("fetch_posts", || 2);
//!     42
//! });
//!
//! println!("{}", tree.to_mermaid());
//! ```

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::{Condvar, Mutex};

use crate::error::TaskError;
use crate::scope::CancelToken;
use crate::task::TaskId;

/// State of a task for debugging purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugTaskState {
    /// Task is pending execution.
    Pending,
    /// Task is currently running.
    Running,
    /// Task completed successfully.
    Completed,
    /// Task was cancelled.
    Cancelled,
    /// Task panicked.
    Failed,
}

impl std::fmt::Display for DebugTaskState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DebugTaskState::Pending => write!(f, "pending"),
            DebugTaskState::Running => write!(f, "running"),
            DebugTaskState::Completed => write!(f, "completed"),
            DebugTaskState::Cancelled => write!(f, "cancelled"),
            DebugTaskState::Failed => write!(f, "failed"),
        }
    }
}

/// Information about a task for debugging.
#[derive(Debug, Clone)]
pub struct DebugTaskInfo {
    /// Task identifier.
    pub id: TaskId,
    /// Optional name for the task.
    pub name: Option<String>,
    /// Current state of the task.
    pub state: DebugTaskState,
    /// When the task was spawned.
    pub spawned_at: Instant,
    /// When the task completed (if finished).
    pub completed_at: Option<Instant>,
    /// Duration the task ran for.
    pub duration: Option<Duration>,
}

impl DebugTaskInfo {
    fn new(id: TaskId, name: Option<String>) -> Self {
        Self {
            id,
            name,
            state: DebugTaskState::Pending,
            spawned_at: Instant::now(),
            completed_at: None,
            duration: None,
        }
    }

    fn mark_running(&mut self) {
        self.state = DebugTaskState::Running;
    }

    fn mark_completed(&mut self, success: bool) {
        self.state = if success {
            DebugTaskState::Completed
        } else {
            DebugTaskState::Failed
        };
        self.completed_at = Some(Instant::now());
        self.duration = Some(self.completed_at.unwrap() - self.spawned_at);
    }

    fn mark_cancelled(&mut self) {
        self.state = DebugTaskState::Cancelled;
        self.completed_at = Some(Instant::now());
        self.duration = Some(self.completed_at.unwrap() - self.spawned_at);
    }
}

/// A tree representation of a scope and its tasks.
#[derive(Debug, Clone)]
pub struct ScopeTree {
    /// Name of the scope.
    pub name: String,
    /// Tasks in this scope.
    pub tasks: Vec<DebugTaskInfo>,
    /// Child scopes.
    pub children: Vec<ScopeTree>,
    /// When the scope was created.
    pub created_at: Instant,
    /// When the scope finished.
    pub finished_at: Option<Instant>,
    /// Whether the scope timed out.
    pub timed_out: bool,
    /// Whether the scope was cancelled.
    pub cancelled: bool,
}

impl ScopeTree {
    /// Create a new scope tree.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            tasks: Vec::new(),
            children: Vec::new(),
            created_at: Instant::now(),
            finished_at: None,
            timed_out: false,
            cancelled: false,
        }
    }

    /// Get the total number of tasks (including in child scopes).
    pub fn total_tasks(&self) -> usize {
        self.tasks.len() + self.children.iter().map(|c| c.total_tasks()).sum::<usize>()
    }

    /// Get the number of completed tasks.
    pub fn completed_tasks(&self) -> usize {
        self.tasks
            .iter()
            .filter(|t| t.state == DebugTaskState::Completed)
            .count()
            + self
                .children
                .iter()
                .map(|c| c.completed_tasks())
                .sum::<usize>()
    }

    /// Get the number of failed tasks.
    pub fn failed_tasks(&self) -> usize {
        self.tasks
            .iter()
            .filter(|t| t.state == DebugTaskState::Failed)
            .count()
            + self.children.iter().map(|c| c.failed_tasks()).sum::<usize>()
    }

    /// Get the number of cancelled tasks.
    pub fn cancelled_tasks(&self) -> usize {
        self.tasks
            .iter()
            .filter(|t| t.state == DebugTaskState::Cancelled)
            .count()
            + self
                .children
                .iter()
                .map(|c| c.cancelled_tasks())
                .sum::<usize>()
    }

    /// Export to Mermaid flowchart format.
    ///
    /// # Example Output
    ///
    /// ```text
    /// graph TD
    ///     scope1[fetch_user_data]
    ///     scope1 --> t1[fetch_profile: completed]
    ///     scope1 --> t2[fetch_posts: running]
    /// ```
    pub fn to_mermaid(&self) -> String {
        let mut output = String::from("graph TD\n");
        self.write_mermaid(&mut output, "scope", &mut 0);
        output
    }

    fn write_mermaid(&self, output: &mut String, prefix: &str, counter: &mut usize) {
        let scope_id = format!("{}{}", prefix, counter);
        *counter += 1;

        // Scope node
        let scope_status = if self.timed_out {
            " [TIMEOUT]"
        } else if self.cancelled {
            " [CANCELLED]"
        } else {
            ""
        };
        output.push_str(&format!("    {}[{}{}]\n", scope_id, self.name, scope_status));

        // Task nodes
        for task in &self.tasks {
            let task_id = format!("t{}", task.id.as_u64());
            let task_name = task.name.as_deref().unwrap_or("task");
            let duration_str = task
                .duration
                .map(|d| format!(" ({:.1}ms)", d.as_secs_f64() * 1000.0))
                .unwrap_or_default();

            output.push_str(&format!(
                "    {} --> {}[{}: {}{}]\n",
                scope_id, task_id, task_name, task.state, duration_str
            ));
        }

        // Child scopes
        for child in &self.children {
            let child_id = format!("{}{}", prefix, counter);
            output.push_str(&format!("    {} --> {}\n", scope_id, child_id));
            child.write_mermaid(output, prefix, counter);
        }
    }

    /// Export to DOT (Graphviz) format.
    pub fn to_dot(&self) -> String {
        let mut output = String::from("digraph scope_tree {\n");
        output.push_str("    rankdir=TB;\n");
        output.push_str("    node [shape=box];\n");
        self.write_dot(&mut output, "scope", &mut 0);
        output.push_str("}\n");
        output
    }

    fn write_dot(&self, output: &mut String, prefix: &str, counter: &mut usize) {
        let scope_id = format!("{}{}", prefix, counter);
        *counter += 1;

        // Scope node with styling
        let color = if self.timed_out {
            "orange"
        } else if self.cancelled {
            "red"
        } else {
            "lightblue"
        };
        output.push_str(&format!(
            "    {} [label=\"{}\" style=filled fillcolor={}];\n",
            scope_id, self.name, color
        ));

        // Task nodes
        for task in &self.tasks {
            let task_id = format!("t{}", task.id.as_u64());
            let task_name = task.name.as_deref().unwrap_or("task");
            let color = match task.state {
                DebugTaskState::Completed => "lightgreen",
                DebugTaskState::Running => "yellow",
                DebugTaskState::Cancelled => "orange",
                DebugTaskState::Failed => "red",
                DebugTaskState::Pending => "white",
            };
            output.push_str(&format!(
                "    {} [label=\"{}: {}\" style=filled fillcolor={}];\n",
                task_id, task_name, task.state, color
            ));
            output.push_str(&format!("    {} -> {};\n", scope_id, task_id));
        }

        // Child scopes
        for child in &self.children {
            let child_id = format!("{}{}", prefix, counter);
            output.push_str(&format!("    {} -> {};\n", scope_id, child_id));
            child.write_dot(output, prefix, counter);
        }
    }

    /// Export to a simple text tree format.
    pub fn to_text_tree(&self) -> String {
        let mut output = String::new();
        self.write_text_tree(&mut output, "", true);
        output
    }

    fn write_text_tree(&self, output: &mut String, prefix: &str, is_last: bool) {
        let connector = if is_last { "└── " } else { "├── " };
        let status = if self.timed_out {
            " [TIMEOUT]"
        } else if self.cancelled {
            " [CANCELLED]"
        } else {
            ""
        };
        output.push_str(&format!("{}{}{}{}\n", prefix, connector, self.name, status));

        let child_prefix = format!("{}{}   ", prefix, if is_last { " " } else { "│" });

        let total_items = self.tasks.len() + self.children.len();
        for (i, task) in self.tasks.iter().enumerate() {
            let is_task_last = i == total_items - 1;
            let task_connector = if is_task_last { "└── " } else { "├── " };
            let task_name = task.name.as_deref().unwrap_or("task");
            let duration_str = task
                .duration
                .map(|d| format!(" ({:.1}ms)", d.as_secs_f64() * 1000.0))
                .unwrap_or_default();
            output.push_str(&format!(
                "{}{}{}: {}{}\n",
                child_prefix, task_connector, task_name, task.state, duration_str
            ));
        }

        for (i, child) in self.children.iter().enumerate() {
            let is_child_last = i == self.children.len() - 1 && self.tasks.is_empty();
            child.write_text_tree(output, &child_prefix, is_child_last);
        }
    }
}

/// Internal state for debug task tracking.
struct DebugTaskInner<T> {
    info: Mutex<DebugTaskInfo>,
    result: Mutex<Option<Result<T, TaskError>>>,
    completed: Condvar,
}

impl<T> DebugTaskInner<T> {
    fn new(id: TaskId, name: Option<String>) -> Self {
        Self {
            info: Mutex::new(DebugTaskInfo::new(id, name)),
            result: Mutex::new(None),
            completed: Condvar::new(),
        }
    }

    fn mark_running(&self) {
        self.info.lock().mark_running();
    }

    fn complete(&self, result: Result<T, TaskError>) {
        let success = result.is_ok();
        let is_cancelled = matches!(result, Err(TaskError::Cancelled));

        {
            let mut info = self.info.lock();
            if is_cancelled {
                info.mark_cancelled();
            } else {
                info.mark_completed(success);
            }
        }

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

    fn info(&self) -> DebugTaskInfo {
        self.info.lock().clone()
    }
}

/// Handle for a debug task.
pub struct DebugJoinHandle<T> {
    inner: Arc<DebugTaskInner<T>>,
}

impl<T> DebugJoinHandle<T>
where
    T: Clone,
{
    /// Get the task ID.
    pub fn id(&self) -> TaskId {
        self.inner.info.lock().id
    }

    /// Get the task info.
    pub fn info(&self) -> DebugTaskInfo {
        self.inner.info()
    }

    /// Wait for the task to complete.
    pub fn join(self) -> Result<T, TaskError> {
        self.inner.wait()
    }
}

impl<T: Clone> Clone for DebugJoinHandle<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// A scope with debugging capabilities.
///
/// This is like `Scope` but tracks additional information for
/// visualization and debugging purposes.
pub struct DebugScope {
    /// Name of this scope.
    name: String,
    /// Cancel token.
    cancel_token: CancelToken,
    /// Active task count.
    active_count: Arc<AtomicUsize>,
    /// Completion signaling.
    all_completed: Arc<(Mutex<bool>, Condvar)>,
    /// First error.
    first_error: Arc<Mutex<Option<TaskError>>>,
    /// Task info for visualization.
    task_infos: Arc<Mutex<Vec<DebugTaskInfo>>>,
    /// Whether to cancel on error.
    cancel_on_error: bool,
}

impl DebugScope {
    /// Create a new debug scope with a name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            cancel_token: CancelToken::new(),
            active_count: Arc::new(AtomicUsize::new(0)),
            all_completed: Arc::new((Mutex::new(false), Condvar::new())),
            first_error: Arc::new(Mutex::new(None)),
            task_infos: Arc::new(Mutex::new(Vec::new())),
            cancel_on_error: true,
        }
    }

    /// Get the scope name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the cancel token.
    pub fn cancel_token(&self) -> CancelToken {
        self.cancel_token.clone()
    }

    /// Spawn a named task.
    pub fn spawn_named<F, T>(&mut self, name: impl Into<String>, f: F) -> DebugJoinHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + Clone + 'static,
    {
        let task_name = name.into();
        self.spawn_internal(Some(task_name), f)
    }

    /// Spawn an unnamed task.
    pub fn spawn<F, T>(&mut self, f: F) -> DebugJoinHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + Clone + 'static,
    {
        self.spawn_internal(None, f)
    }

    fn spawn_internal<F, T>(&mut self, name: Option<String>, f: F) -> DebugJoinHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + Clone + 'static,
    {
        self.active_count.fetch_add(1, Ordering::AcqRel);

        let id = TaskId::new();
        let inner = Arc::new(DebugTaskInner::new(id, name.clone()));
        let inner_clone = Arc::clone(&inner);

        let active_count = Arc::clone(&self.active_count);
        let all_completed = Arc::clone(&self.all_completed);
        let first_error = Arc::clone(&self.first_error);
        let task_infos = Arc::clone(&self.task_infos);
        let cancel_token = self.cancel_token.clone();
        let cancel_on_error = self.cancel_on_error;

        // Use the global pool
        let _ = crate::pool::global_pool().spawn(move || {
            inner_clone.mark_running();

            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if cancel_token.is_cancelled() {
                    return Err(TaskError::Cancelled);
                }
                Ok(f())
            }));

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
                    let mut first = first_error.lock();
                    if first.is_none() {
                        *first = Some(e.clone());
                        cancel_token.cancel();
                    }
                }
            }

            inner_clone.complete(result);

            // Store task info for later visualization
            task_infos.lock().push(inner_clone.info());

            // Signal completion
            let remaining = active_count.fetch_sub(1, Ordering::AcqRel) - 1;
            if remaining == 0 {
                let (lock, cvar) = &*all_completed;
                *lock.lock() = true;
                cvar.notify_all();
            }
        });

        DebugJoinHandle { inner }
    }

    /// Wait for all tasks to complete.
    pub fn join_all(&mut self) {
        let (lock, cvar) = &*self.all_completed;
        let mut completed = lock.lock();
        while !*completed && self.active_count.load(Ordering::Acquire) > 0 {
            cvar.wait(&mut completed);
        }
    }

    /// Build a scope tree for visualization.
    pub fn build_tree(&self) -> ScopeTree {
        let mut tree = ScopeTree::new(&self.name);
        tree.tasks = self.task_infos.lock().clone();
        tree.cancelled = self.cancel_token.is_cancelled();
        tree.finished_at = Some(Instant::now());
        tree
    }
}

impl Drop for DebugScope {
    fn drop(&mut self) {
        self.join_all();
    }
}

/// Execute a function with a debug scope and return the scope tree.
///
/// # Example
///
/// ```rust
/// use aria_runtime::scope_debug::with_debug_scope;
///
/// let tree = with_debug_scope("my_scope", |scope| {
///     let h1 = scope.spawn_named("task_1", || 10);
///     let h2 = scope.spawn_named("task_2", || 20);
///     h1.join().unwrap() + h2.join().unwrap()
/// });
///
/// println!("{}", tree.to_text_tree());
/// ```
pub fn with_debug_scope<F, R>(name: impl Into<String>, f: F) -> ScopeTree
where
    F: FnOnce(&mut DebugScope) -> R,
{
    let mut scope = DebugScope::new(name);
    let _ = f(&mut scope);
    scope.join_all();
    scope.build_tree()
}

/// Execute a function with a debug scope and return both result and tree.
pub fn with_debug_scope_result<F, R>(name: impl Into<String>, f: F) -> (R, ScopeTree)
where
    F: FnOnce(&mut DebugScope) -> R,
{
    let mut scope = DebugScope::new(name);
    let result = f(&mut scope);
    scope.join_all();
    let tree = scope.build_tree();
    (result, tree)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_debug_scope_basic() {
        let tree = with_debug_scope("test_scope", |scope| {
            let h1 = scope.spawn_named("task_1", || 10);
            let h2 = scope.spawn_named("task_2", || 20);
            h1.join().unwrap() + h2.join().unwrap()
        });

        assert_eq!(tree.name, "test_scope");
        assert_eq!(tree.tasks.len(), 2);
        assert_eq!(tree.completed_tasks(), 2);
    }

    #[test]
    fn test_debug_scope_with_failure() {
        let tree = with_debug_scope("failing_scope", |scope| {
            scope.spawn_named("good_task", || 42);
            scope.spawn_named("bad_task", || -> i32 { panic!("oops") });
        });

        assert_eq!(tree.tasks.len(), 2);
        assert!(tree.failed_tasks() >= 1 || tree.cancelled_tasks() >= 1);
    }

    #[test]
    fn test_scope_tree_to_mermaid() {
        let tree = with_debug_scope("render_test", |scope| {
            scope.spawn_named("fetch", || 1);
            scope.spawn_named("process", || 2);
        });

        let mermaid = tree.to_mermaid();
        assert!(mermaid.contains("graph TD"));
        assert!(mermaid.contains("render_test"));
        assert!(mermaid.contains("fetch"));
        assert!(mermaid.contains("process"));
    }

    #[test]
    fn test_scope_tree_to_dot() {
        let tree = with_debug_scope("dot_test", |scope| {
            scope.spawn_named("task_a", || 1);
        });

        let dot = tree.to_dot();
        assert!(dot.contains("digraph"));
        assert!(dot.contains("dot_test"));
        assert!(dot.contains("task_a"));
    }

    #[test]
    fn test_scope_tree_to_text() {
        let tree = with_debug_scope("text_test", |scope| {
            scope.spawn_named("alpha", || 1);
            scope.spawn_named("beta", || 2);
        });

        let text = tree.to_text_tree();
        assert!(text.contains("text_test"));
        assert!(text.contains("alpha"));
        assert!(text.contains("beta"));
    }

    #[test]
    fn test_task_state_tracking() {
        let tree = with_debug_scope("state_test", |scope| {
            let h = scope.spawn_named("tracked", || {
                thread::sleep(Duration::from_millis(10));
                42
            });
            h.join().unwrap()
        });

        let task = &tree.tasks[0];
        assert_eq!(task.state, DebugTaskState::Completed);
        assert!(task.duration.is_some());
    }

    #[test]
    fn test_debug_scope_result() {
        let (result, tree) = with_debug_scope_result("result_test", |scope| {
            let h = scope.spawn_named("compute", || 100);
            h.join().unwrap()
        });

        assert_eq!(result, 100);
        assert_eq!(tree.tasks.len(), 1);
    }

    #[test]
    fn test_debug_task_info() {
        let tree = with_debug_scope("info_test", |scope| {
            let h = scope.spawn_named("info_task", || 42);
            let info = h.info();
            assert!(info.name.as_deref() == Some("info_task"));
            h.join().unwrap()
        });

        assert_eq!(tree.completed_tasks(), 1);
    }
}
