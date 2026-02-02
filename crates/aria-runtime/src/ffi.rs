//! FFI interface for aria-runtime
//!
//! This module provides C-callable functions that can be called from
//! compiled Aria code. These functions bridge the gap between the
//! Cranelift-generated machine code and the Rust runtime.
//!
//! # Memory Safety
//!
//! All functions in this module are `unsafe` and use raw pointers.
//! The caller (generated code) is responsible for ensuring:
//! - Pointers are valid
//! - Memory is properly aligned
//! - Lifetimes are respected

use std::ffi::c_void;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use crate::executor::spawn;
use crate::task::Task;

/// Global counter for task IDs
static TASK_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Type alias for task function pointer
/// Takes captured data pointer, returns result as i64
pub type TaskFn = unsafe extern "C" fn(*mut c_void) -> i64;

/// Wrapper to hold both function pointer and captures together
/// This makes the whole package Send-safe
struct TaskPackage {
    func: TaskFn,
    captures: *mut c_void,
}

// SAFETY: The FFI contract guarantees:
// - func is a valid function pointer that can be called from any thread
// - captures is a pointer to data that is safe to access from any thread
unsafe impl Send for TaskPackage {}

impl TaskPackage {
    fn run(self) -> i64 {
        unsafe { (self.func)(self.captures) }
    }
}

/// Opaque task handle for FFI
#[repr(C)]
pub struct AriaTaskHandle {
    id: u64,
    // Pointer to Task stored on heap
    handle_ptr: *mut c_void,
}

/// Storage for pending task handles
/// Using a Mutex for thread-safe access
static TASK_HANDLES: Mutex<Option<std::collections::HashMap<u64, Task<i64>>>> = Mutex::new(None);

fn with_task_handles<F, R>(f: F) -> R
where
    F: FnOnce(&mut std::collections::HashMap<u64, Task<i64>>) -> R,
{
    let mut guard = TASK_HANDLES.lock().unwrap();
    let handles = guard.get_or_insert_with(std::collections::HashMap::new);
    f(handles)
}

/// Spawn a new async task.
///
/// # Arguments
/// * `func` - Function pointer to execute
/// * `captures` - Pointer to captured variables (owned by task after call)
///
/// # Returns
/// Task ID that can be used with `aria_async_await`
///
/// # Safety
/// - `func` must be a valid function pointer
/// - `captures` must be valid for the lifetime of the task
#[no_mangle]
pub unsafe extern "C" fn aria_async_spawn(
    func: TaskFn,
    captures: *mut c_void,
) -> u64 {
    let task_id = TASK_ID_COUNTER.fetch_add(1, Ordering::SeqCst);

    // Package the function and captures together
    let package = TaskPackage { func, captures };

    // Spawn the task
    let task = spawn(move || package.run());

    // Store the task for later await
    with_task_handles(|handles| {
        handles.insert(task_id, task);
    });

    task_id
}

/// Await a task's completion and get its result.
///
/// # Arguments
/// * `task_id` - Task ID returned from `aria_async_spawn`
///
/// # Returns
/// The task's result value, or 0 if task not found
///
/// # Safety
/// Must only be called once per task_id
#[no_mangle]
pub unsafe extern "C" fn aria_async_await(task_id: u64) -> i64 {
    let task = with_task_handles(|handles| handles.remove(&task_id));
    if let Some(task) = task {
        task.join().unwrap_or(0)
    } else {
        // Task not found or already awaited
        0
    }
}

/// Yield control to the scheduler.
///
/// This is a cooperative yield point that allows other tasks to run.
/// In thread-per-task mode, this is effectively a no-op or thread yield.
#[no_mangle]
pub extern "C" fn aria_async_yield() {
    std::thread::yield_now();
}

/// Check if a task has completed without blocking.
///
/// # Arguments
/// * `task_id` - Task ID to check
///
/// # Returns
/// 1 if completed, 0 if still running, -1 if not found
#[no_mangle]
pub unsafe extern "C" fn aria_async_poll(task_id: u64) -> i64 {
    with_task_handles(|handles| {
        if let Some(task) = handles.get(&task_id) {
            if task.is_finished() {
                1
            } else {
                0
            }
        } else {
            -1
        }
    })
}

/// Simple spawn that takes a function pointer with no captures.
///
/// This is a simpler interface for functions that don't need closures.
#[no_mangle]
pub unsafe extern "C" fn aria_spawn_simple(func: extern "C" fn() -> i64) -> u64 {
    let task_id = TASK_ID_COUNTER.fetch_add(1, Ordering::SeqCst);

    let task = spawn(move || func());
    with_task_handles(|handles| {
        handles.insert(task_id, task);
    });

    task_id
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe extern "C" fn test_task(_: *mut c_void) -> i64 {
        42
    }

    #[test]
    fn test_spawn_and_await() {
        unsafe {
            let task_id = aria_async_spawn(test_task, std::ptr::null_mut());
            assert!(task_id > 0);

            let result = aria_async_await(task_id);
            assert_eq!(result, 42);
        }
    }

    #[test]
    fn test_yield() {
        // Should not panic
        aria_async_yield();
    }

    extern "C" fn simple_task() -> i64 {
        100
    }

    #[test]
    fn test_spawn_simple() {
        unsafe {
            let task_id = aria_spawn_simple(simple_task);
            let result = aria_async_await(task_id);
            assert_eq!(result, 100);
        }
    }
}
