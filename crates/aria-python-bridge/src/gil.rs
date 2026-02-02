//! GIL (Global Interpreter Lock) Management
//!
//! This module provides abstractions for managing Python's GIL.
//! Based on ARIA-M10 Python Interop milestone.
//!
//! ## Background
//!
//! Python's GIL is a mutex that protects access to Python objects,
//! preventing multiple native threads from executing Python bytecode
//! at once. For Aria-Python interop, we need to:
//!
//! 1. Acquire the GIL before calling Python code
//! 2. Release it during long Aria computations
//! 3. Manage nested GIL acquisitions properly
//!
//! ## Strategies
//!
//! - **GilGuard**: RAII guard for GIL acquisition
//! - **GilPool**: Pool for managing Python objects
//! - **GilState**: Thread-local GIL state tracking

use std::cell::Cell;
use std::fmt;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::error::{PyBridgeError, PyBridgeResult};

// ============================================================================
// GIL State Tracking
// ============================================================================

/// Global flag indicating if Python is initialized
static PYTHON_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Counter for GIL acquisition depth (for debugging)
static GIL_ACQUIRE_COUNT: AtomicU64 = AtomicU64::new(0);

thread_local! {
    /// Thread-local GIL hold count
    static GIL_DEPTH: Cell<usize> = const { Cell::new(0) };

    /// Thread-local flag for whether we own the GIL
    static GIL_HELD: Cell<bool> = const { Cell::new(false) };
}

/// State of the GIL for the current thread
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GilState {
    /// GIL is not held by this thread
    NotHeld,
    /// GIL is held by this thread
    Held,
    /// GIL was already held when we entered (nested)
    NestedHeld,
}

impl GilState {
    /// Get the current GIL state for this thread
    pub fn current() -> Self {
        GIL_HELD.with(|held| {
            if held.get() {
                GilState::Held
            } else {
                GilState::NotHeld
            }
        })
    }

    /// Check if the GIL is currently held
    pub fn is_held() -> bool {
        GIL_HELD.with(|held| held.get())
    }

    /// Get the current nesting depth
    pub fn depth() -> usize {
        GIL_DEPTH.with(|depth| depth.get())
    }
}

impl fmt::Display for GilState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GilState::NotHeld => write!(f, "NotHeld"),
            GilState::Held => write!(f, "Held"),
            GilState::NestedHeld => write!(f, "NestedHeld"),
        }
    }
}

// ============================================================================
// Python Initialization (Stub)
// ============================================================================

/// Initialize the Python interpreter (stub).
///
/// In a real implementation, this would call `Py_Initialize()`.
pub fn initialize_python() -> PyBridgeResult<()> {
    if PYTHON_INITIALIZED.load(Ordering::Acquire) {
        return Ok(());
    }

    // Stub: Mark as initialized
    PYTHON_INITIALIZED.store(true, Ordering::Release);

    Ok(())
}

/// Check if Python is initialized
pub fn is_python_initialized() -> bool {
    PYTHON_INITIALIZED.load(Ordering::Acquire)
}

/// Finalize the Python interpreter (stub).
///
/// In a real implementation, this would call `Py_Finalize()`.
pub fn finalize_python() -> PyBridgeResult<()> {
    if !PYTHON_INITIALIZED.load(Ordering::Acquire) {
        return Ok(());
    }

    // Stub: Mark as not initialized
    PYTHON_INITIALIZED.store(false, Ordering::Release);

    Ok(())
}

// ============================================================================
// GilGuard - RAII GIL Acquisition
// ============================================================================

/// RAII guard for GIL acquisition.
///
/// Acquires the GIL when created, releases when dropped.
///
/// # Example
///
/// ```ignore
/// let _gil = GilGuard::acquire()?;
/// // GIL is held here
/// call_python_function()?;
/// // GIL is released when _gil is dropped
/// ```
#[derive(Debug)]
pub struct GilGuard {
    /// State when we acquired the GIL
    previous_state: GilState,
    /// Marker to prevent Send/Sync
    _marker: PhantomData<*mut ()>,
}

impl GilGuard {
    /// Acquire the GIL.
    ///
    /// If the GIL is already held by this thread, this increments
    /// the nesting depth but doesn't actually re-acquire.
    pub fn acquire() -> PyBridgeResult<Self> {
        if !is_python_initialized() {
            return Err(PyBridgeError::InterpreterNotInitialized);
        }

        let previous_state = GilState::current();

        // Increment depth
        GIL_DEPTH.with(|depth| {
            depth.set(depth.get() + 1);
        });

        // Mark as held
        GIL_HELD.with(|held| {
            held.set(true);
        });

        // Track acquisition count (for debugging)
        GIL_ACQUIRE_COUNT.fetch_add(1, Ordering::Relaxed);

        Ok(Self {
            previous_state,
            _marker: PhantomData,
        })
    }

    /// Try to acquire the GIL with a timeout.
    ///
    /// This is a stub that always succeeds immediately.
    /// In a real implementation, this would use `PyGILState_Ensure()`
    /// with timeout handling.
    pub fn try_acquire(_timeout_ms: u64) -> PyBridgeResult<Self> {
        // Stub: Just call acquire
        Self::acquire()
    }

    /// Get the current GIL state
    pub fn state(&self) -> GilState {
        GilState::current()
    }

    /// Check if this is a nested acquisition
    pub fn is_nested(&self) -> bool {
        self.previous_state == GilState::Held
    }

    /// Temporarily release the GIL for long computations.
    ///
    /// Returns a guard that re-acquires the GIL when dropped.
    pub fn allow_threads<F, T>(&self, f: F) -> PyBridgeResult<T>
    where
        F: FnOnce() -> T,
    {
        // Mark GIL as not held during the computation
        GIL_HELD.with(|held| held.set(false));

        // Run the computation
        let result = f();

        // Re-acquire the GIL
        GIL_HELD.with(|held| held.set(true));

        Ok(result)
    }
}

impl Drop for GilGuard {
    fn drop(&mut self) {
        // Decrement depth
        GIL_DEPTH.with(|depth| {
            let new_depth = depth.get().saturating_sub(1);
            depth.set(new_depth);

            // Only release if we're at depth 0
            if new_depth == 0 {
                GIL_HELD.with(|held| {
                    held.set(false);
                });
            }
        });
    }
}

// ============================================================================
// GilPool - Object Pool for GIL-Protected Operations
// ============================================================================

/// Pool for managing Python objects during GIL-protected operations.
///
/// This helps batch object allocations and defer cleanup until
/// a good time (reducing GIL contention).
#[derive(Debug)]
pub struct GilPool {
    /// Guard that holds the GIL
    _guard: GilGuard,
    /// Number of objects allocated in this pool
    object_count: Cell<usize>,
    /// Maximum objects before auto-cleanup
    max_objects: usize,
}

impl GilPool {
    /// Create a new object pool with default settings.
    pub fn new() -> PyBridgeResult<Self> {
        Self::with_capacity(1024)
    }

    /// Create a new object pool with specified max capacity.
    pub fn with_capacity(max_objects: usize) -> PyBridgeResult<Self> {
        Ok(Self {
            _guard: GilGuard::acquire()?,
            object_count: Cell::new(0),
            max_objects,
        })
    }

    /// Track an object allocation.
    ///
    /// Returns the current object count.
    pub fn track_alloc(&self) -> usize {
        let count = self.object_count.get() + 1;
        self.object_count.set(count);
        count
    }

    /// Track an object deallocation.
    pub fn track_dealloc(&self) {
        let count = self.object_count.get();
        if count > 0 {
            self.object_count.set(count - 1);
        }
    }

    /// Get the current object count.
    pub fn object_count(&self) -> usize {
        self.object_count.get()
    }

    /// Check if the pool is full.
    pub fn is_full(&self) -> bool {
        self.object_count.get() >= self.max_objects
    }

    /// Trigger a cleanup cycle (stub).
    ///
    /// In a real implementation, this would run Python's garbage collector.
    pub fn cleanup(&self) {
        // Stub: Just reset count
        self.object_count.set(0);
    }

    /// Run a function with this pool, cleaning up after.
    pub fn scope<F, T>(&self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        let result = f();
        // Auto-cleanup if we're full
        if self.is_full() {
            self.cleanup();
        }
        result
    }
}

// ============================================================================
// GIL-Free Operations
// ============================================================================

/// Marker trait for operations that don't require the GIL.
///
/// This is used to mark functions that work with data that's
/// already copied/extracted from Python objects.
pub trait GilFree {}

/// Run a function without the GIL.
///
/// This is for Aria computations that don't touch Python objects.
pub fn without_gil<F, T>(f: F) -> PyBridgeResult<T>
where
    F: FnOnce() -> T,
{
    if !GilState::is_held() {
        // GIL not held, just run
        return Ok(f());
    }

    // Release GIL, run, re-acquire
    GIL_HELD.with(|held| held.set(false));
    let result = f();
    GIL_HELD.with(|held| held.set(true));

    Ok(result)
}

/// Execute a computation with minimal GIL time.
///
/// This acquires the GIL, extracts data, releases the GIL,
/// performs computation, then re-acquires to store results.
pub fn gil_minimal<I, O, Extract, Compute, Store>(
    extract: Extract,
    compute: Compute,
    store: Store,
) -> PyBridgeResult<O>
where
    Extract: FnOnce() -> PyBridgeResult<I>,
    Compute: FnOnce(I) -> O,
    Store: FnOnce(O) -> PyBridgeResult<O>,
{
    // Phase 1: Acquire GIL and extract data
    let input = {
        let _gil = GilGuard::acquire()?;
        extract()?
    };

    // Phase 2: Compute without GIL
    let output = compute(input);

    // Phase 3: Acquire GIL and store results
    let _gil = GilGuard::acquire()?;
    store(output)
}

// ============================================================================
// Debug Utilities
// ============================================================================

/// Get statistics about GIL usage (for debugging).
pub fn gil_stats() -> GilStats {
    GilStats {
        total_acquires: GIL_ACQUIRE_COUNT.load(Ordering::Relaxed),
        current_depth: GilState::depth(),
        is_held: GilState::is_held(),
    }
}

/// GIL usage statistics.
#[derive(Debug, Clone)]
pub struct GilStats {
    /// Total number of GIL acquisitions
    pub total_acquires: u64,
    /// Current nesting depth
    pub current_depth: usize,
    /// Whether the GIL is currently held
    pub is_held: bool,
}

impl fmt::Display for GilStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GIL Stats: acquires={}, depth={}, held={}",
            self.total_acquires, self.current_depth, self.is_held
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reset_state() {
        // Reset thread-local state
        GIL_DEPTH.with(|d| d.set(0));
        GIL_HELD.with(|h| h.set(false));
    }

    #[test]
    fn test_python_initialization() {
        // Reset
        PYTHON_INITIALIZED.store(false, Ordering::Release);

        assert!(!is_python_initialized());

        initialize_python().unwrap();
        assert!(is_python_initialized());

        finalize_python().unwrap();
        assert!(!is_python_initialized());
    }

    #[test]
    fn test_gil_guard_basic() {
        reset_state();
        initialize_python().unwrap();

        assert!(!GilState::is_held());
        assert_eq!(GilState::depth(), 0);

        {
            let _gil = GilGuard::acquire().unwrap();
            assert!(GilState::is_held());
            assert_eq!(GilState::depth(), 1);
        }

        assert!(!GilState::is_held());
        assert_eq!(GilState::depth(), 0);
    }

    #[test]
    fn test_gil_guard_nested() {
        reset_state();
        initialize_python().unwrap();

        {
            let gil1 = GilGuard::acquire().unwrap();
            assert_eq!(GilState::depth(), 1);
            assert!(!gil1.is_nested());

            {
                let gil2 = GilGuard::acquire().unwrap();
                assert_eq!(GilState::depth(), 2);
                assert!(gil2.is_nested());

                {
                    let gil3 = GilGuard::acquire().unwrap();
                    assert_eq!(GilState::depth(), 3);
                    assert!(gil3.is_nested());
                }

                assert_eq!(GilState::depth(), 2);
            }

            assert_eq!(GilState::depth(), 1);
        }

        assert_eq!(GilState::depth(), 0);
        assert!(!GilState::is_held());
    }

    #[test]
    fn test_gil_guard_uninitialized() {
        reset_state();
        PYTHON_INITIALIZED.store(false, Ordering::Release);

        let result = GilGuard::acquire();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PyBridgeError::InterpreterNotInitialized
        ));
    }

    #[test]
    fn test_gil_pool() {
        reset_state();
        initialize_python().unwrap();

        let pool = GilPool::with_capacity(10).unwrap();

        assert_eq!(pool.object_count(), 0);
        assert!(!pool.is_full());

        for _ in 0..10 {
            pool.track_alloc();
        }

        assert_eq!(pool.object_count(), 10);
        assert!(pool.is_full());

        pool.cleanup();
        assert_eq!(pool.object_count(), 0);
    }

    #[test]
    fn test_without_gil() {
        reset_state();
        initialize_python().unwrap();

        let result = without_gil(|| 42).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_allow_threads() {
        reset_state();
        initialize_python().unwrap();

        let gil = GilGuard::acquire().unwrap();
        assert!(GilState::is_held());

        let result = gil.allow_threads(|| {
            // During this function, GIL should not be held
            // (In stub, we can't really check this from here)
            42
        }).unwrap();

        assert_eq!(result, 42);
        assert!(GilState::is_held()); // Should be re-held after
    }

    #[test]
    fn test_gil_stats() {
        reset_state();
        initialize_python().unwrap();

        let initial_stats = gil_stats();

        {
            let _gil = GilGuard::acquire().unwrap();
            let held_stats = gil_stats();
            assert!(held_stats.is_held);
            assert_eq!(held_stats.current_depth, 1);
            assert!(held_stats.total_acquires > initial_stats.total_acquires);
        }

        let final_stats = gil_stats();
        assert!(!final_stats.is_held);
        assert_eq!(final_stats.current_depth, 0);
    }

    #[test]
    fn test_gil_minimal_pattern() {
        reset_state();
        initialize_python().unwrap();

        let result = gil_minimal(
            // Extract (with GIL)
            || Ok(vec![1, 2, 3]),
            // Compute (without GIL)
            |data: Vec<i32>| data.iter().sum::<i32>(),
            // Store (with GIL)
            |sum| Ok(sum),
        )
        .unwrap();

        assert_eq!(result, 6);
    }
}
