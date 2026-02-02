//! Timer wheel for efficient timeout handling.
//!
//! This module implements a hierarchical timing wheel for managing many concurrent
//! timers efficiently. Key properties:
//!
//! - O(1) timer insertion and cancellation
//! - O(1) amortized expiration processing
//! - Efficient for thousands of concurrent timers
//!
//! # Design
//!
//! The timer wheel uses a single-level wheel with configurable tick duration.
//! For most use cases (timeouts in ms-second range), this provides excellent
//! performance. A hierarchical wheel can be added later if needed.
//!
//! # Example
//!
//! ```rust
//! use aria_runtime::timer::{TimerWheel, TimerHandle};
//! use std::time::Duration;
//! use std::sync::Arc;
//!
//! // Create a timer wheel
//! let wheel = Arc::new(TimerWheel::new());
//!
//! // Schedule a timer
//! let handle = wheel.schedule(Duration::from_millis(100), || {
//!     println!("Timer fired!");
//! });
//!
//! // Cancel if needed
//! handle.cancel();
//! ```

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use parking_lot::{Condvar, Mutex};

/// Default tick interval (1ms)
const DEFAULT_TICK_INTERVAL: Duration = Duration::from_millis(1);

/// Default wheel size (1024 slots = ~1 second with 1ms ticks)
const DEFAULT_WHEEL_SIZE: usize = 1024;

/// A timer entry in the wheel.
struct TimerEntry {
    /// When this timer should fire (in ticks from wheel start)
    deadline_ticks: u64,
    /// The callback to execute
    callback: Box<dyn FnOnce() + Send + 'static>,
    /// Unique timer ID
    id: u64,
    /// Cancellation flag (shared with handle)
    cancelled: Arc<AtomicBool>,
}

/// Handle to a scheduled timer.
///
/// Can be used to cancel the timer before it fires.
#[derive(Clone)]
pub struct TimerHandle {
    id: u64,
    cancelled: Arc<AtomicBool>,
}

impl TimerHandle {
    /// Cancel this timer.
    ///
    /// Returns true if the timer was cancelled, false if it already fired.
    pub fn cancel(&self) -> bool {
        !self.cancelled.swap(true, Ordering::AcqRel)
    }

    /// Check if this timer has been cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    /// Get the timer ID.
    pub fn id(&self) -> u64 {
        self.id
    }
}

/// Internal state of the timer wheel.
struct WheelInner {
    /// The wheel slots, each containing timers for that tick
    slots: Vec<Mutex<VecDeque<TimerEntry>>>,
    /// Current tick position in the wheel
    current_tick: AtomicU64,
    /// Next timer ID
    next_id: AtomicU64,
    /// Tick interval
    tick_interval: Duration,
    /// Wheel size
    wheel_size: usize,
    /// Start time of the wheel
    start_time: Instant,
    /// Shutdown flag
    shutdown: AtomicBool,
    /// Condvar for waking the tick thread
    tick_condvar: Condvar,
    /// Mutex for the condvar
    tick_mutex: Mutex<()>,
}

impl WheelInner {
    fn new(tick_interval: Duration, wheel_size: usize) -> Self {
        let slots = (0..wheel_size)
            .map(|_| Mutex::new(VecDeque::new()))
            .collect();

        Self {
            slots,
            current_tick: AtomicU64::new(0),
            next_id: AtomicU64::new(1),
            tick_interval,
            wheel_size,
            start_time: Instant::now(),
            shutdown: AtomicBool::new(false),
            tick_condvar: Condvar::new(),
            tick_mutex: Mutex::new(()),
        }
    }

    fn schedule<F>(&self, delay: Duration, callback: F) -> TimerHandle
    where
        F: FnOnce() + Send + 'static,
    {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let cancelled = Arc::new(AtomicBool::new(false));

        // Calculate deadline in ticks
        let delay_ticks = delay.as_nanos() / self.tick_interval.as_nanos();
        let current = self.current_tick.load(Ordering::Acquire);
        let deadline_ticks = current + delay_ticks as u64;

        // Calculate slot index
        let slot_index = (deadline_ticks as usize) % self.wheel_size;

        let entry = TimerEntry {
            deadline_ticks,
            callback: Box::new(callback),
            id,
            cancelled: Arc::clone(&cancelled),
        };

        // Insert into slot
        self.slots[slot_index].lock().push_back(entry);

        // Wake the tick thread if needed
        self.tick_condvar.notify_one();

        TimerHandle { id, cancelled }
    }

    fn tick(&self) -> Vec<Box<dyn FnOnce() + Send + 'static>> {
        let current = self.current_tick.fetch_add(1, Ordering::AcqRel);
        let slot_index = (current as usize) % self.wheel_size;

        let mut expired = Vec::new();
        let mut slot = self.slots[slot_index].lock();

        // Process all timers in this slot
        let mut remaining = VecDeque::new();

        while let Some(entry) = slot.pop_front() {
            if entry.cancelled.load(Ordering::Acquire) {
                // Timer was cancelled, skip it
                continue;
            }

            if entry.deadline_ticks <= current {
                // Timer has expired
                expired.push(entry.callback);
            } else {
                // Timer hasn't expired yet (wrapped around the wheel)
                remaining.push_back(entry);
            }
        }

        // Put back the remaining timers
        *slot = remaining;

        expired
    }
}

/// A hierarchical timing wheel for efficient timer management.
///
/// The wheel provides O(1) timer insertion and cancellation, and efficiently
/// handles thousands of concurrent timers.
pub struct TimerWheel {
    inner: Arc<WheelInner>,
    tick_thread: Mutex<Option<JoinHandle<()>>>,
}

impl TimerWheel {
    /// Create a new timer wheel with default settings.
    ///
    /// Uses 1ms tick interval and 1024 slots.
    pub fn new() -> Self {
        Self::with_config(DEFAULT_TICK_INTERVAL, DEFAULT_WHEEL_SIZE)
    }

    /// Create a timer wheel with custom configuration.
    pub fn with_config(tick_interval: Duration, wheel_size: usize) -> Self {
        let inner = Arc::new(WheelInner::new(tick_interval, wheel_size));

        Self {
            inner,
            tick_thread: Mutex::new(None),
        }
    }

    /// Start the timer wheel's tick thread.
    ///
    /// This must be called before scheduling timers if you want them to fire.
    /// For testing, you can manually call `advance()` instead.
    pub fn start(&self) {
        let mut thread_guard = self.tick_thread.lock();
        if thread_guard.is_some() {
            return; // Already started
        }

        let inner = Arc::clone(&self.inner);
        let handle = thread::Builder::new()
            .name("aria-timer".to_string())
            .spawn(move || {
                let mut next_tick = Instant::now() + inner.tick_interval;

                while !inner.shutdown.load(Ordering::Acquire) {
                    // Wait until next tick
                    let now = Instant::now();
                    if now < next_tick {
                        let wait_time = next_tick - now;
                        let mut guard = inner.tick_mutex.lock();
                        let _ = inner.tick_condvar.wait_for(&mut guard, wait_time);
                    }

                    if inner.shutdown.load(Ordering::Acquire) {
                        break;
                    }

                    // Process expired timers
                    let expired = inner.tick();
                    for callback in expired {
                        // Execute callbacks (could be moved to thread pool)
                        callback();
                    }

                    next_tick += inner.tick_interval;
                }
            })
            .expect("Failed to spawn timer thread");

        *thread_guard = Some(handle);
    }

    /// Stop the timer wheel.
    pub fn stop(&self) {
        self.inner.shutdown.store(true, Ordering::Release);
        self.inner.tick_condvar.notify_one();

        if let Some(handle) = self.tick_thread.lock().take() {
            let _ = handle.join();
        }
    }

    /// Schedule a timer to fire after the given delay.
    pub fn schedule<F>(&self, delay: Duration, callback: F) -> TimerHandle
    where
        F: FnOnce() + Send + 'static,
    {
        self.inner.schedule(delay, callback)
    }

    /// Schedule a timer with a deadline (absolute time).
    pub fn schedule_at<F>(&self, deadline: Instant, callback: F) -> TimerHandle
    where
        F: FnOnce() + Send + 'static,
    {
        let now = Instant::now();
        let delay = if deadline > now {
            deadline - now
        } else {
            Duration::ZERO
        };
        self.schedule(delay, callback)
    }

    /// Manually advance the wheel by one tick (for testing).
    ///
    /// Returns the callbacks that expired.
    pub fn advance(&self) -> Vec<Box<dyn FnOnce() + Send + 'static>> {
        self.inner.tick()
    }

    /// Get the current tick count.
    pub fn current_tick(&self) -> u64 {
        self.inner.current_tick.load(Ordering::Acquire)
    }

    /// Get the tick interval.
    pub fn tick_interval(&self) -> Duration {
        self.inner.tick_interval
    }
}

impl Default for TimerWheel {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TimerWheel {
    fn drop(&mut self) {
        self.stop();
    }
}

// ============================================================================
// Global Timer Wheel
// ============================================================================

use std::sync::OnceLock;

static GLOBAL_WHEEL: OnceLock<TimerWheel> = OnceLock::new();

/// Get the global timer wheel.
///
/// Creates and starts the wheel on first access.
pub fn global_timer() -> &'static TimerWheel {
    GLOBAL_WHEEL.get_or_init(|| {
        let wheel = TimerWheel::new();
        wheel.start();
        wheel
    })
}

/// Schedule a timer on the global wheel.
pub fn schedule_timer<F>(delay: Duration, callback: F) -> TimerHandle
where
    F: FnOnce() + Send + 'static,
{
    global_timer().schedule(delay, callback)
}

/// Schedule a timer with a deadline on the global wheel.
pub fn schedule_at<F>(deadline: Instant, callback: F) -> TimerHandle
where
    F: FnOnce() + Send + 'static,
{
    global_timer().schedule_at(deadline, callback)
}

// ============================================================================
// Timeout Utilities
// ============================================================================

/// Result of a timeout operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeoutResult<T> {
    /// The operation completed within the timeout.
    Ok(T),
    /// The operation timed out.
    TimedOut,
}

impl<T> TimeoutResult<T> {
    /// Returns true if the operation completed.
    pub fn is_ok(&self) -> bool {
        matches!(self, TimeoutResult::Ok(_))
    }

    /// Returns true if the operation timed out.
    pub fn is_timed_out(&self) -> bool {
        matches!(self, TimeoutResult::TimedOut)
    }

    /// Unwrap the result, panicking if timed out.
    pub fn unwrap(self) -> T {
        match self {
            TimeoutResult::Ok(v) => v,
            TimeoutResult::TimedOut => panic!("operation timed out"),
        }
    }

    /// Unwrap the result or return a default.
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            TimeoutResult::Ok(v) => v,
            TimeoutResult::TimedOut => default,
        }
    }

    /// Map the successful value.
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> TimeoutResult<U> {
        match self {
            TimeoutResult::Ok(v) => TimeoutResult::Ok(f(v)),
            TimeoutResult::TimedOut => TimeoutResult::TimedOut,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn test_timer_handle_cancel() {
        let wheel = TimerWheel::new();

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        let handle = wheel.schedule(Duration::from_millis(10), move || {
            counter_clone.fetch_add(1, Ordering::Relaxed);
        });

        assert!(!handle.is_cancelled());
        assert!(handle.cancel());
        assert!(handle.is_cancelled());

        // Advance the wheel past the timer
        for _ in 0..20 {
            wheel.advance();
        }

        // Timer should not have fired
        assert_eq!(counter.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_timer_fires() {
        let wheel = TimerWheel::with_config(Duration::from_millis(1), 64);

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        wheel.schedule(Duration::from_millis(5), move || {
            counter_clone.fetch_add(1, Ordering::Relaxed);
        });

        // Advance past the deadline
        for _ in 0..10 {
            let expired = wheel.advance();
            for cb in expired {
                cb();
            }
        }

        assert_eq!(counter.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_multiple_timers() {
        let wheel = TimerWheel::with_config(Duration::from_millis(1), 64);

        let counter = Arc::new(AtomicUsize::new(0));

        // Schedule 10 timers at different delays
        for i in 0..10 {
            let counter_clone = Arc::clone(&counter);
            wheel.schedule(Duration::from_millis(i * 2), move || {
                counter_clone.fetch_add(1, Ordering::Relaxed);
            });
        }

        // Advance the wheel
        for _ in 0..30 {
            let expired = wheel.advance();
            for cb in expired {
                cb();
            }
        }

        assert_eq!(counter.load(Ordering::Relaxed), 10);
    }

    #[test]
    fn test_timer_with_thread() {
        let wheel = Arc::new(TimerWheel::new());
        wheel.start();

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        wheel.schedule(Duration::from_millis(10), move || {
            counter_clone.fetch_add(1, Ordering::Relaxed);
        });

        // Wait for timer to fire
        thread::sleep(Duration::from_millis(50));

        assert_eq!(counter.load(Ordering::Relaxed), 1);

        wheel.stop();
    }

    #[test]
    fn test_schedule_at() {
        let wheel = TimerWheel::new();

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        let deadline = Instant::now() + Duration::from_millis(5);
        wheel.schedule_at(deadline, move || {
            counter_clone.fetch_add(1, Ordering::Relaxed);
        });

        // Advance the wheel
        for _ in 0..10 {
            let expired = wheel.advance();
            for cb in expired {
                cb();
            }
        }

        assert_eq!(counter.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_timeout_result() {
        let ok: TimeoutResult<i32> = TimeoutResult::Ok(42);
        assert!(ok.is_ok());
        assert!(!ok.is_timed_out());
        assert_eq!(ok.unwrap(), 42);

        let timed_out: TimeoutResult<i32> = TimeoutResult::TimedOut;
        assert!(!timed_out.is_ok());
        assert!(timed_out.is_timed_out());
        assert_eq!(timed_out.unwrap_or(0), 0);
    }

    #[test]
    fn test_timeout_result_map() {
        let ok: TimeoutResult<i32> = TimeoutResult::Ok(21);
        let doubled = ok.map(|x| x * 2);
        assert_eq!(doubled.unwrap(), 42);

        let timed_out: TimeoutResult<i32> = TimeoutResult::TimedOut;
        let mapped = timed_out.map(|x| x * 2);
        assert!(mapped.is_timed_out());
    }
}
