//! Async Effect Handler for Aria Runtime
//!
//! This module bridges the Async effect from aria-effects to the actual
//! runtime implementation. It provides the effect operations that the
//! compiled code will call when performing async operations.
//!
//! # Architecture
//!
//! ```text
//! Aria Code (with Async effect)
//!        │
//!        ▼
//! Effect Compilation (aria-codegen)
//!        │
//!        ▼
//! AsyncEffectHandler (this module)
//!        │
//!        ▼
//! aria-runtime executor (spawn, join, yield)
//! ```
//!
//! # Usage
//!
//! The handler is used by compiled Aria code to execute async operations:
//!
//! ```rust
//! use aria_runtime::async_handler::{AsyncEffectHandler, AsyncContext};
//!
//! // Create a context for an async computation
//! let ctx = AsyncContext::new();
//!
//! // Spawn a task (maps to Async.spawn in Aria)
//! let task_id = AsyncEffectHandler::spawn_effect(
//!     &ctx,
//!     Box::new(|| std::sync::Arc::new(42i64) as std::sync::Arc<dyn std::any::Any + Send + Sync>)
//! );
//!
//! // Await a task (maps to .await in Aria)
//! let result = AsyncEffectHandler::await_effect(&ctx, task_id);
//! ```

use std::any::Any;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::executor::{spawn, yield_now};
use crate::task::Task;

/// Unique identifier for a spawned task within an async context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AsyncTaskId(u64);

impl AsyncTaskId {
    /// Generate a new unique task ID.
    fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        AsyncTaskId(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Get the raw ID value.
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

/// Type-erased result from an async task.
///
/// We use `Arc<dyn Any + Send + Sync>` instead of `Box<dyn Any + Send>` because
/// the Task::join() method requires Clone, and Arc provides that while allowing
/// us to type-erase the result.
pub type AsyncResult = Arc<dyn Any + Send + Sync>;

/// Type-erased task closure.
pub type AsyncClosure = Box<dyn FnOnce() -> AsyncResult + Send>;

/// Context for managing async tasks within a computation.
///
/// AsyncContext tracks spawned tasks and provides the runtime context
/// needed for effect handlers to work correctly.
pub struct AsyncContext {
    /// Map of task IDs to their handles (type-erased).
    tasks: Mutex<HashMap<AsyncTaskId, AsyncTaskHandle>>,
}

/// Type-erased handle to an async task.
struct AsyncTaskHandle {
    /// The underlying task handle.
    task: Task<AsyncResult>,
}

impl AsyncContext {
    /// Create a new async context.
    pub fn new() -> Self {
        Self {
            tasks: Mutex::new(HashMap::new()),
        }
    }

    /// Register a task in this context.
    fn register_task(&self, id: AsyncTaskId, task: Task<AsyncResult>) {
        let mut tasks = self.tasks.lock().unwrap();
        tasks.insert(id, AsyncTaskHandle { task });
    }

    /// Get a task by ID, removing it from the context.
    fn take_task(&self, id: AsyncTaskId) -> Option<Task<AsyncResult>> {
        let mut tasks = self.tasks.lock().unwrap();
        tasks.remove(&id).map(|h| h.task)
    }
}

impl Default for AsyncContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Handler for the Async effect operations.
///
/// This struct provides static methods that implement each operation
/// defined in the Async effect from aria-effects:
///
/// - `spawn`: Create a new concurrent task
/// - `await`: Wait for a task to complete
/// - `yield`: Yield control to the scheduler
///
/// These methods are called by compiled Aria code through the effect
/// compilation infrastructure.
pub struct AsyncEffectHandler;

impl AsyncEffectHandler {
    /// Execute the Async.spawn operation.
    ///
    /// Creates a new concurrent task that will execute the given closure.
    /// Returns a task ID that can be used with `await_effect` to wait for
    /// the result.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The async context to register the task in
    /// * `f` - The closure to execute concurrently
    ///
    /// # Returns
    ///
    /// A task ID that can be passed to `await_effect`.
    pub fn spawn_effect(ctx: &AsyncContext, f: AsyncClosure) -> AsyncTaskId {
        let task_id = AsyncTaskId::new();

        // Spawn the task using the runtime
        let task = spawn(move || f());

        // Register in context
        ctx.register_task(task_id, task);

        task_id
    }

    /// Execute the Async.await operation.
    ///
    /// Waits for a spawned task to complete and returns its result.
    /// This operation suspends the current computation until the
    /// awaited task finishes.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The async context containing the task
    /// * `task_id` - The ID of the task to await
    ///
    /// # Returns
    ///
    /// The result of the awaited task, or None if the task was not found.
    ///
    /// # Panics
    ///
    /// Panics if the task panicked. In production, this should be
    /// converted to an exception effect.
    pub fn await_effect(ctx: &AsyncContext, task_id: AsyncTaskId) -> Option<AsyncResult> {
        let task = ctx.take_task(task_id)?;

        match task.join() {
            Ok(result) => Some(result),
            Err(e) => {
                // In production, this would raise an exception effect
                // For now, we panic to surface the error
                panic!("Async task failed: {:?}", e);
            }
        }
    }

    /// Execute the Async.yield operation.
    ///
    /// Yields control to the scheduler, allowing other tasks to run.
    /// This is a cooperative yielding point used to prevent long-running
    /// computations from starving other tasks.
    pub fn yield_effect() {
        yield_now();
    }
}

/// Wrapper for running async code from a synchronous context.
///
/// This is similar to Tokio's `block_on` or Python's `asyncio.run`.
/// It creates an async context and runs the given closure within it.
///
/// # Example
///
/// ```rust
/// use aria_runtime::async_handler::run_async;
///
/// let result = run_async(|ctx| {
///     // Spawn some tasks
///     let id1 = aria_runtime::async_handler::AsyncEffectHandler::spawn_effect(
///         ctx,
///         Box::new(|| std::sync::Arc::new(1i32) as std::sync::Arc<dyn std::any::Any + Send + Sync>)
///     );
///     let id2 = aria_runtime::async_handler::AsyncEffectHandler::spawn_effect(
///         ctx,
///         Box::new(|| std::sync::Arc::new(2i32) as std::sync::Arc<dyn std::any::Any + Send + Sync>)
///     );
///
///     // Await both
///     let r1 = aria_runtime::async_handler::AsyncEffectHandler::await_effect(ctx, id1).unwrap();
///     let r2 = aria_runtime::async_handler::AsyncEffectHandler::await_effect(ctx, id2).unwrap();
///
///     *r1.downcast::<i32>().unwrap() + *r2.downcast::<i32>().unwrap()
/// });
///
/// assert_eq!(result, 3);
/// ```
pub fn run_async<F, T>(f: F) -> T
where
    F: FnOnce(&AsyncContext) -> T,
{
    let ctx = AsyncContext::new();
    f(&ctx)
}

// Thread-local async context for implicit context passing.
//
// This allows compiled code to access the current async context
// without explicit parameter passing (using evidence-passing at
// the compiler level).
thread_local! {
    static CURRENT_CONTEXT: std::cell::RefCell<Option<Arc<AsyncContext>>> = const { std::cell::RefCell::new(None) };
}

/// Run a closure with an async context set as the current context.
///
/// This is used by the effect compilation to make the context available
/// to deeply nested code without explicit parameter passing.
pub fn with_async_context<F, T>(ctx: Arc<AsyncContext>, f: F) -> T
where
    F: FnOnce() -> T,
{
    CURRENT_CONTEXT.with(|c| {
        let old = c.borrow_mut().replace(ctx);
        let result = f();
        *c.borrow_mut() = old;
        result
    })
}

/// Get the current async context, if one is set.
///
/// Returns None if not running within an async context.
pub fn current_async_context() -> Option<Arc<AsyncContext>> {
    CURRENT_CONTEXT.with(|c| c.borrow().clone())
}

/// Convenient functions for use when the context is implicit.
pub mod implicit {
    use super::*;

    /// Spawn a task using the implicit current context.
    ///
    /// # Panics
    ///
    /// Panics if called outside of an async context.
    pub fn spawn(f: AsyncClosure) -> AsyncTaskId {
        let ctx = current_async_context().expect("spawn called outside of async context");
        AsyncEffectHandler::spawn_effect(&ctx, f)
    }

    /// Await a task using the implicit current context.
    ///
    /// # Panics
    ///
    /// Panics if called outside of an async context or if the task panicked.
    pub fn await_task(task_id: AsyncTaskId) -> AsyncResult {
        let ctx = current_async_context().expect("await called outside of async context");
        AsyncEffectHandler::await_effect(&ctx, task_id).expect("task not found")
    }

    /// Yield control to the scheduler.
    pub fn yield_now() {
        AsyncEffectHandler::yield_effect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_async_task_id_unique() {
        let id1 = AsyncTaskId::new();
        let id2 = AsyncTaskId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_spawn_and_await() {
        let ctx = AsyncContext::new();

        let task_id = AsyncEffectHandler::spawn_effect(
            &ctx,
            Box::new(|| Arc::new(42i64) as AsyncResult),
        );

        let result = AsyncEffectHandler::await_effect(&ctx, task_id).unwrap();
        let value = *result.downcast::<i64>().unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn test_spawn_multiple_tasks() {
        let ctx = AsyncContext::new();

        let id1 = AsyncEffectHandler::spawn_effect(
            &ctx,
            Box::new(|| Arc::new(10i64) as AsyncResult),
        );
        let id2 = AsyncEffectHandler::spawn_effect(
            &ctx,
            Box::new(|| Arc::new(20i64) as AsyncResult),
        );
        let id3 = AsyncEffectHandler::spawn_effect(
            &ctx,
            Box::new(|| Arc::new(30i64) as AsyncResult),
        );

        let r1 = *AsyncEffectHandler::await_effect(&ctx, id1)
            .unwrap()
            .downcast::<i64>()
            .unwrap();
        let r2 = *AsyncEffectHandler::await_effect(&ctx, id2)
            .unwrap()
            .downcast::<i64>()
            .unwrap();
        let r3 = *AsyncEffectHandler::await_effect(&ctx, id3)
            .unwrap()
            .downcast::<i64>()
            .unwrap();

        assert_eq!(r1 + r2 + r3, 60);
    }

    #[test]
    fn test_await_nonexistent_task() {
        let ctx = AsyncContext::new();
        let fake_id = AsyncTaskId(9999);
        let result = AsyncEffectHandler::await_effect(&ctx, fake_id);
        assert!(result.is_none());
    }

    #[test]
    fn test_run_async() {
        let result = run_async(|ctx| {
            let id = AsyncEffectHandler::spawn_effect(
                ctx,
                Box::new(|| Arc::new("hello".to_string()) as AsyncResult),
            );
            let r = AsyncEffectHandler::await_effect(ctx, id).unwrap();
            r.downcast::<String>().unwrap()
        });
        assert_eq!(*result, "hello");
    }

    #[test]
    fn test_with_async_context() {
        let ctx = Arc::new(AsyncContext::new());

        let result = with_async_context(Arc::clone(&ctx), || {
            let current = current_async_context();
            assert!(current.is_some());

            // Spawn using implicit context
            implicit::spawn(Box::new(|| Arc::new(100i32) as AsyncResult))
        });

        // Await outside the context scope using the original ctx
        let value = AsyncEffectHandler::await_effect(&ctx, result)
            .unwrap()
            .downcast::<i32>()
            .unwrap();
        assert_eq!(*value, 100);
    }

    #[test]
    fn test_yield_effect() {
        // Just ensure it doesn't panic
        AsyncEffectHandler::yield_effect();
    }

    #[test]
    fn test_current_context_none_outside() {
        let current = current_async_context();
        assert!(current.is_none());
    }

    #[test]
    fn test_spawn_with_captured_value() {
        let ctx = AsyncContext::new();
        let value = 42i64;

        let task_id = AsyncEffectHandler::spawn_effect(
            &ctx,
            Box::new(move || Arc::new(value * 2) as AsyncResult),
        );

        let result = AsyncEffectHandler::await_effect(&ctx, task_id).unwrap();
        let computed = *result.downcast::<i64>().unwrap();
        assert_eq!(computed, 84);
    }

    #[test]
    fn test_concurrent_computation() {
        use std::sync::atomic::{AtomicI32, Ordering};

        let ctx = AsyncContext::new();
        let counter = Arc::new(AtomicI32::new(0));

        let mut task_ids = Vec::new();

        // Spawn 10 tasks that each increment a counter
        for _ in 0..10 {
            let counter_clone = Arc::clone(&counter);
            let id = AsyncEffectHandler::spawn_effect(
                &ctx,
                Box::new(move || {
                    counter_clone.fetch_add(1, Ordering::SeqCst);
                    Arc::new(()) as AsyncResult
                }),
            );
            task_ids.push(id);
        }

        // Await all tasks
        for id in task_ids {
            AsyncEffectHandler::await_effect(&ctx, id);
        }

        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }
}
