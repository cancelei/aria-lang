# ARIA-PD-009: Runtime System Design

**Decision ID**: ARIA-PD-009
**Status**: Approved
**Date**: 2026-01-15
**Based On**: ARIA-M11-05-green-threads-runtime.md, ARIA-M11-04-colored-functions-analysis.md, ARIA-M11-06-channel-patterns.md
**Decision Agent**: TITAN (Product)

---

## 1. Executive Summary

This document specifies Aria's optional runtime system design, following NEXUS research recommendations for a **stackless-first architecture with optional work-stealing runtime**. The design enables:

- Zero-cost concurrency for embedded/WASM targets (no runtime required)
- Ergonomic concurrent programming for applications (spawn millions of tasks)
- Context switch cost <300ns target
- Per-task memory overhead of 64-256 bytes

### Core Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Default execution** | Stackless state machines | Zero overhead when concurrency unused |
| **Runtime model** | Optional, library-provided | Flexibility for embedded to server |
| **Scheduler algorithm** | Work-stealing with per-core queues | Proven efficient, scales to many cores |
| **Preemption model** | Cooperative with soft preemption | Balance of overhead and fairness |
| **I/O integration** | Pluggable driver interface | Platform-specific optimization |
| **Function coloring** | Caller-provided execution context | Zig-inspired colorblind approach |

---

## 2. Architecture Overview

### 2.1 Layered Runtime Design

```
+-----------------------------------------------------------------------+
|                        Aria Concurrency Stack                          |
+-----------------------------------------------------------------------+
|                                                                        |
|  Layer 4: User Code                                                    |
|  +------------------------------------------------------------------+ |
|  |  spawn { ... }  |  Channel.new()  |  select { ... }               | |
|  +------------------------------------------------------------------+ |
|                                                                        |
|  Layer 3: Runtime API (std.runtime)                                    |
|  +------------------------------------------------------------------+ |
|  |  runtime::spawn()  |  runtime::block_on()  |  runtime::yield()    | |
|  +------------------------------------------------------------------+ |
|                                                                        |
|  Layer 2: Scheduler (pluggable)                                        |
|  +------------------------------------------------------------------+ |
|  |  SingleThreaded  |  WorkStealing  |  Custom Executor              | |
|  +------------------------------------------------------------------+ |
|                                                                        |
|  Layer 1: I/O Driver (platform-specific)                               |
|  +------------------------------------------------------------------+ |
|  |  epoll (Linux)  |  kqueue (macOS)  |  IOCP (Windows)  |  WASI    | |
|  +------------------------------------------------------------------+ |
|                                                                        |
|  Layer 0: Compiler-Generated State Machines                            |
|  +------------------------------------------------------------------+ |
|  |  Stackless coroutines  |  Poll trait  |  Waker interface          | |
|  +------------------------------------------------------------------+ |
|                                                                        |
+-----------------------------------------------------------------------+
```

### 2.2 Execution Modes

| Mode | Runtime | Use Case | Memory Overhead |
|------|---------|----------|-----------------|
| **No Runtime** | None | Embedded, WASM, bare metal | 0 bytes base |
| **Single-threaded** | Minimal | Simple servers, CLI tools | ~4KB base |
| **Multi-threaded** | Full | Applications, services | ~64KB base + 4KB/core |

---

## 3. Runtime API Specification

### 3.1 Core Types

```aria
module std.runtime

/// A handle to a spawned task, allowing await or cancellation
pub type TaskHandle<T> = struct {
    id: TaskId,
    result: Future<Result<T, TaskError>>,
}

/// Task identifier for debugging and tracking
pub type TaskId = u64

/// Error types for task operations
pub enum TaskError {
    Cancelled,
    Panicked(PanicInfo),
    JoinError,
}

/// Runtime configuration
pub struct RuntimeConfig {
    /// Number of worker threads (0 = single-threaded mode)
    worker_threads: usize,
    /// Size of per-worker task queue
    queue_size: usize,
    /// Enable work stealing between workers
    work_stealing: bool,
    /// I/O driver to use
    io_driver: IoDriverKind,
    /// Soft preemption threshold (microseconds, 0 = disabled)
    preempt_threshold_us: u64,
}

impl RuntimeConfig {
    /// Single-threaded runtime (minimal overhead)
    pub fn single_threaded() -> Self {
        RuntimeConfig {
            worker_threads: 0,
            queue_size: 256,
            work_stealing: false,
            io_driver: IoDriverKind::Auto,
            preempt_threshold_us: 0,
        }
    }

    /// Multi-threaded runtime (one worker per CPU core)
    pub fn multi_threaded() -> Self {
        RuntimeConfig {
            worker_threads: num_cpus(),
            queue_size: 256,
            work_stealing: true,
            io_driver: IoDriverKind::Auto,
            preempt_threshold_us: 1000, // 1ms soft preemption
        }
    }
}
```

### 3.2 Primary API Functions

```aria
module std.runtime

/// Initialize and run the runtime, executing the given future to completion.
/// This is the entry point for concurrent Aria programs.
///
/// # Example
/// ```aria
/// fn main() {
///     runtime::block_on(async {
///         let handle = runtime::spawn(async { compute() })
///         handle.await
///     })
/// }
/// ```
pub fn block_on<T>(future: impl Future<Output = T>) -> T

/// Spawn a new task onto the runtime.
/// Returns a handle that can be awaited or cancelled.
///
/// # Example
/// ```aria
/// let handle = runtime::spawn(async {
///     fetch_data().await
/// })
/// let result = handle.await?
/// ```
pub fn spawn<T: Send>(future: impl Future<Output = T> + Send) -> TaskHandle<T>

/// Spawn a task that is !Send (single-threaded runtime only).
/// Useful for thread-local state or non-Send types.
pub fn spawn_local<T>(future: impl Future<Output = T>) -> TaskHandle<T>

/// Yield control to the scheduler, allowing other tasks to run.
/// Use this in CPU-intensive loops to maintain fairness.
///
/// # Example
/// ```aria
/// for i in 0..1_000_000 {
///     if i % 1000 == 0 {
///         runtime::yield_now().await
///     }
///     process(i)
/// }
/// ```
pub fn yield_now() -> impl Future<Output = ()>

/// Get the current task's ID (for debugging/logging).
pub fn current_task_id() -> Option<TaskId>

/// Check if the current task has been cancelled.
/// Call this periodically in long-running tasks.
pub fn is_cancelled() -> bool

/// Sleep for the specified duration.
pub fn sleep(duration: Duration) -> impl Future<Output = ()>

/// Create a timeout wrapper around a future.
pub fn timeout<T>(duration: Duration, future: impl Future<Output = T>)
    -> impl Future<Output = Result<T, TimeoutError>>
```

### 3.3 Runtime Builder API

```aria
module std.runtime

/// Builder for custom runtime configuration
pub struct RuntimeBuilder {
    config: RuntimeConfig,
}

impl RuntimeBuilder {
    pub fn new() -> Self

    /// Set number of worker threads (0 = current thread only)
    pub fn worker_threads(mut self, n: usize) -> Self

    /// Set the I/O driver
    pub fn io_driver(mut self, driver: IoDriverKind) -> Self

    /// Enable/disable work stealing
    pub fn enable_work_stealing(mut self, enable: bool) -> Self

    /// Set soft preemption threshold
    pub fn preempt_threshold(mut self, duration: Duration) -> Self

    /// Set per-worker queue capacity
    pub fn queue_capacity(mut self, size: usize) -> Self

    /// Build and return the runtime
    pub fn build(self) -> Result<Runtime, RuntimeError>
}

/// The runtime handle for manual control
pub struct Runtime {
    // ... internal fields
}

impl Runtime {
    /// Run a future to completion on this runtime
    pub fn block_on<T>(&self, future: impl Future<Output = T>) -> T

    /// Spawn a task on this runtime
    pub fn spawn<T: Send>(&self, future: impl Future<Output = T> + Send)
        -> TaskHandle<T>

    /// Gracefully shut down the runtime
    pub fn shutdown(self) -> Result<(), ShutdownError>

    /// Shut down immediately, cancelling all tasks
    pub fn shutdown_now(self)
}

// Convenience function for quick runtime creation
pub fn new_runtime() -> Runtime {
    RuntimeBuilder::new().build().unwrap()
}
```

### 3.4 Task Group API (Structured Concurrency)

```aria
module std.runtime

/// A scope for structured concurrency - all spawned tasks
/// are awaited before the scope exits.
pub struct TaskGroup<T> {
    handles: Vec<TaskHandle<T>>,
}

impl TaskGroup<T> {
    /// Spawn a task within this group
    pub fn spawn(&mut self, future: impl Future<Output = T> + Send)

    /// Wait for all tasks to complete, returning results
    pub fn join_all(self) -> Vec<Result<T, TaskError>>

    /// Cancel all tasks in the group
    pub fn cancel_all(&mut self)
}

/// Create a task group scope
/// All tasks must complete before the scope exits.
///
/// # Example
/// ```aria
/// let results = runtime::task_group(|group| {
///     for url in urls {
///         group.spawn(async { fetch(url).await })
///     }
/// }).await
/// ```
pub fn task_group<T, F>(f: F) -> impl Future<Output = Vec<Result<T, TaskError>>>
where
    F: FnOnce(&mut TaskGroup<T>),
    T: Send
```

---

## 4. Scheduler Design

### 4.1 Work-Stealing Algorithm

The scheduler implements a work-stealing algorithm based on the Arora-Blumofe-Plaxton deque design, optimized for Aria's stackless coroutines.

#### 4.1.1 Data Structures

```aria
/// Per-worker data structure
struct Worker {
    /// Worker ID (0..num_workers)
    id: WorkerId,

    /// Local run queue (lock-free deque)
    /// Workers push/pop from bottom, thieves steal from top
    local_queue: WorkStealingDeque<Task>,

    /// LIFO slot for immediate execution
    /// Avoids queue overhead for sequential spawns
    lifo_slot: AtomicOption<Task>,

    /// Random number generator for victim selection
    rng: XorShift64,

    /// Thread parker for idle waiting
    parker: Parker,

    /// Statistics for debugging
    stats: WorkerStats,
}

/// Global scheduler state
struct Scheduler {
    /// All workers
    workers: Vec<Worker>,

    /// Global queue for overflow and initial distribution
    global_queue: MpmcQueue<Task>,

    /// Shared I/O driver
    io_driver: IoDriver,

    /// Shutdown signal
    shutdown: AtomicBool,

    /// Number of parked workers
    parked_count: AtomicU32,
}
```

#### 4.1.2 Scheduling Algorithm

```
WORKER MAIN LOOP:
1. Check LIFO slot
   - If task present: pop and execute

2. Check local queue
   - Pop from bottom (LIFO for cache locality)
   - If task found: execute

3. Check global queue (1/61 probability for fairness)
   - Steal up to batch_size tasks
   - Execute first, push rest to local queue

4. Poll I/O driver (non-blocking)
   - Check for completed I/O operations
   - Wake associated tasks

5. Attempt work stealing
   - Select random victim worker
   - Steal half of victim's local queue
   - If successful: execute first task

6. If no work found:
   - Increment parked count
   - Park thread with timeout (1ms)
   - On wake: decrement parked count, goto 1

7. Check shutdown flag
   - If set: exit loop

STEALING PROTOCOL:
- Thief steals from TOP of victim's deque (FIFO)
- This steals older, larger tasks (likely to spawn more work)
- Steal half to amortize stealing overhead
- Use compare-and-swap for lock-free operation
```

#### 4.1.3 Implementation Details

```aria
impl Scheduler {
    /// Called by worker to get next task
    fn next_task(&self, worker: &mut Worker) -> Option<Task> {
        // 1. LIFO slot (fastest path)
        if let Some(task) = worker.lifo_slot.take() {
            return Some(task)
        }

        // 2. Local queue
        if let Some(task) = worker.local_queue.pop() {
            return Some(task)
        }

        // 3. Global queue (probabilistic for fairness)
        if worker.rng.next() % 61 == 0 {
            if let Some(task) = self.global_queue.pop() {
                return Some(task)
            }
        }

        // 4. I/O completions
        self.io_driver.poll_nonblocking(|waker| {
            // Wake tasks directly to LIFO slot
            waker.wake()
        })

        if let Some(task) = worker.lifo_slot.take() {
            return Some(task)
        }

        // 5. Work stealing
        self.steal_work(worker)
    }

    /// Steal work from another worker
    fn steal_work(&self, thief: &mut Worker) -> Option<Task> {
        let num_workers = self.workers.len()

        // Try each worker once in random order
        for _ in 0..num_workers {
            let victim_id = thief.rng.next() % num_workers
            if victim_id == thief.id { continue }

            let victim = &self.workers[victim_id]

            // Steal half of victim's queue
            if let Some(tasks) = victim.local_queue.steal_half() {
                let (first, rest) = tasks.split_first()

                // Push rest to our local queue
                for task in rest {
                    thief.local_queue.push(task)
                }

                return Some(first)
            }
        }

        // Try global queue as fallback
        self.global_queue.pop()
    }
}
```

### 4.2 Per-Core Queue Design

```
MEMORY LAYOUT:

+-------------------+    +-------------------+    +-------------------+
|     Worker 0      |    |     Worker 1      |    |     Worker N      |
| (pinned to core 0)|    | (pinned to core 1)|    | (pinned to core N)|
+-------------------+    +-------------------+    +-------------------+
|                   |    |                   |    |                   |
| +---------------+ |    | +---------------+ |    | +---------------+ |
| | LIFO Slot    | |    | | LIFO Slot    | |    | | LIFO Slot    | |
| | (1 task)     | |    | | (1 task)     | |    | | (1 task)     | |
| +---------------+ |    | +---------------+ |    | +---------------+ |
|                   |    |                   |    |                   |
| +---------------+ |    | +---------------+ |    | +---------------+ |
| | Local Deque  | |    | | Local Deque  | |    | | Local Deque  | |
| | [256 tasks]  | |    | | [256 tasks]  | |    | | [256 tasks]  | |
| |              | |    | |              | |    | |              | |
| | Bottom: push | |    | | Bottom: push | |    | | Bottom: push | |
| |        pop   | |    | |        pop   | |    | |        pop   | |
| |              | |    | |              | |    | |              | |
| | Top: steal   | |    | | Top: steal   | |    | | Top: steal   | |
| +---------------+ |    | +---------------+ |    | +---------------+ |
|                   |    |                   |    |                   |
+-------------------+    +-------------------+    +-------------------+
          |                      |                      |
          +----------------------+----------------------+
                                 |
                    +------------------------+
                    |    Global Queue        |
                    |    (overflow/init)     |
                    |    [1024 tasks]        |
                    +------------------------+
```

### 4.3 Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| Context switch (same core) | <100ns | Benchmark: switch between two tasks |
| Context switch (cross core) | <300ns | Benchmark: task migration |
| Task spawn overhead | <50ns | Benchmark: spawn + immediate poll |
| Work steal latency | <500ns | Benchmark: successful steal |
| I/O poll latency | <1us | Benchmark: epoll_wait round trip |
| Scheduler throughput | >10M tasks/sec | Benchmark: spawn-complete cycle |

### 4.4 Soft Preemption

```aria
/// Soft preemption check inserted by compiler at yield points
#[inline(always)]
fn check_preemption(worker: &Worker) -> bool {
    // Check if we've exceeded time budget
    if worker.time_since_last_yield() > PREEMPT_THRESHOLD {
        // Yield to scheduler
        worker.stats.soft_preemptions += 1
        return true
    }
    false
}

// Compiler inserts at:
// - Loop back-edges
// - Function call boundaries (configurable)
// - Before blocking operations

// Example transformation:
//
// Source:
//   for i in 0..n {
//       compute(i)
//   }
//
// Compiled:
//   for i in 0..n {
//       compute(i)
//       if check_preemption(worker) {
//           yield().await
//       }
//   }
```

---

## 5. No-Runtime Mode Specification

### 5.1 Design Goals

For embedded, WASM, and bare-metal targets, Aria supports execution without any runtime:

| Requirement | Solution |
|-------------|----------|
| No heap allocation | State machines on stack |
| No threads | Single-threaded execution |
| No OS dependencies | Polling-based I/O |
| Predictable memory | Compile-time known sizes |
| No hidden code | Everything explicit |

### 5.2 Stackless Execution Model

```aria
// Without runtime, async functions compile to pollable state machines
async fn fetch_sensor_data(pin: GpioPin) -> SensorData {
    let raw = pin.read_async().await
    SensorData::from_raw(raw)
}

// Compiles to:
enum FetchSensorDataState {
    Initial { pin: GpioPin },
    WaitingRead { pin: GpioPin, future: GpioReadFuture },
    Complete,
}

impl Future for FetchSensorDataState {
    type Output = SensorData

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<SensorData> {
        match self.state {
            Initial { pin } => {
                let future = pin.read_async()
                self.state = WaitingRead { pin, future }
                self.poll(cx)
            }
            WaitingRead { pin, future } => {
                match future.poll(cx) {
                    Poll::Ready(raw) => {
                        self.state = Complete
                        Poll::Ready(SensorData::from_raw(raw))
                    }
                    Poll::Pending => Poll::Pending
                }
            }
            Complete => panic!("polled after completion")
        }
    }
}
```

### 5.3 Manual Polling API

```aria
module std.future

/// The core Future trait (no runtime required)
pub trait Future {
    type Output

    /// Poll the future, advancing its state machine
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output>
}

/// Poll result
pub enum Poll<T> {
    Ready(T),
    Pending,
}

/// Context provided to poll, containing the waker
pub struct Context<'a> {
    waker: &'a Waker,
}

/// Waker used to signal that a future can make progress
pub struct Waker {
    // ... implementation details
}

/// Create a no-op waker for simple polling loops
pub fn noop_waker() -> Waker

/// Simple executor for no-runtime mode
/// Polls a future to completion in a blocking loop
pub fn block_on_simple<T>(mut future: impl Future<Output = T>) -> T {
    let waker = noop_waker()
    let mut cx = Context::from_waker(&waker)
    let mut future = pin!(future)

    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(result) => return result,
            Poll::Pending => {
                // In no-runtime mode, spin or use platform-specific wait
                core::hint::spin_loop()
            }
        }
    }
}
```

### 5.4 Embedded Example

```aria
#![no_std]
#![no_main]

use aria::future::{Future, Poll, Context, noop_waker, pin}
use aria::hal::{Gpio, Timer}

#[entry]
fn main() -> ! {
    let mut gpio = Gpio::new()
    let mut timer = Timer::new()

    // Create futures without allocating
    let led_blink = async {
        loop {
            gpio.pin(13).set_high()
            timer.delay_ms(500).await
            gpio.pin(13).set_low()
            timer.delay_ms(500).await
        }
    }

    // Manual polling loop (no runtime)
    let waker = noop_waker()
    let mut cx = Context::from_waker(&waker)
    let mut future = pin!(led_blink)

    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Pending => {
                // Check hardware, handle interrupts
                cortex_m::asm::wfi() // Wait for interrupt
            }
            Poll::Ready(_) => unreachable!() // Never completes
        }
    }
}
```

### 5.5 WASM Mode

```aria
// For WASM, use JavaScript's event loop as the executor
#[cfg(target_arch = "wasm32")]
mod wasm {
    use aria::future::Future

    /// Spawn a future on the JavaScript event loop
    #[wasm_bindgen]
    pub fn spawn_local<T>(future: impl Future<Output = T> + 'static) {
        wasm_bindgen_futures::spawn_local(async {
            future.await;
        })
    }

    /// Bridge to JavaScript promises
    pub fn to_promise<T: Into<JsValue>>(
        future: impl Future<Output = T> + 'static
    ) -> js_sys::Promise {
        wasm_bindgen_futures::future_to_promise(async {
            Ok(future.await.into())
        })
    }
}

// WASM example
#[wasm_bindgen]
pub async fn fetch_and_process(url: String) -> Result<JsValue, JsError> {
    let response = fetch(&url).await?
    let data = response.json().await?
    Ok(process(data).into())
}
```

### 5.6 Mode Selection

```aria
// Build configuration determines mode

// Cargo.toml equivalent
[features]
default = ["runtime"]
runtime = ["std"]
std = []
no_std = []

// Usage:
// Full runtime:      aria build --features runtime
// Single-threaded:   aria build --features std
// No runtime:        aria build --features no_std --no-default-features
// WASM:              aria build --target wasm32 --features no_std
```

---

## 6. I/O Driver Integration

### 6.1 Driver Interface

```aria
module std.runtime.io

/// Platform-agnostic I/O driver trait
pub trait IoDriver: Send + Sync {
    /// Register interest in an I/O source
    fn register(&self, source: &impl IoSource, interest: Interest) -> io::Result<()>

    /// Deregister an I/O source
    fn deregister(&self, source: &impl IoSource) -> io::Result<()>

    /// Poll for I/O events (blocking with timeout)
    fn poll(&self, events: &mut Events, timeout: Option<Duration>) -> io::Result<usize>

    /// Poll for I/O events (non-blocking)
    fn poll_nonblocking(&self, events: &mut Events) -> io::Result<usize> {
        self.poll(events, Some(Duration::ZERO))
    }
}

/// Interest in I/O readiness
pub struct Interest {
    pub readable: bool,
    pub writable: bool,
}

/// I/O source that can be registered with a driver
pub trait IoSource {
    /// Get the raw file descriptor / handle
    fn raw_fd(&self) -> RawFd
}

/// I/O events from polling
pub struct Events {
    events: Vec<Event>,
}

pub struct Event {
    token: Token,
    readable: bool,
    writable: bool,
    error: bool,
}
```

### 6.2 Platform Drivers

```aria
/// Driver selection
pub enum IoDriverKind {
    /// Auto-detect best driver for platform
    Auto,
    /// Linux epoll
    Epoll,
    /// macOS/BSD kqueue
    Kqueue,
    /// Windows I/O Completion Ports
    Iocp,
    /// WASI poll_oneoff
    Wasi,
    /// Polling (fallback, less efficient)
    Poll,
}

// Platform-specific implementations

#[cfg(target_os = "linux")]
mod epoll {
    pub struct EpollDriver {
        epoll_fd: RawFd,
        events: Vec<libc::epoll_event>,
    }

    impl IoDriver for EpollDriver {
        fn poll(&self, events: &mut Events, timeout: Option<Duration>) -> io::Result<usize> {
            let timeout_ms = timeout.map(|d| d.as_millis() as i32).unwrap_or(-1)
            let n = unsafe {
                libc::epoll_wait(
                    self.epoll_fd,
                    self.events.as_mut_ptr(),
                    self.events.len() as i32,
                    timeout_ms
                )
            }
            // ... convert to Events
        }
    }
}

#[cfg(target_os = "macos")]
mod kqueue {
    pub struct KqueueDriver {
        kqueue_fd: RawFd,
        events: Vec<libc::kevent>,
    }
    // ... implementation
}

#[cfg(target_os = "windows")]
mod iocp {
    pub struct IocpDriver {
        handle: HANDLE,
        // ... IOCP specifics
    }
    // ... implementation
}
```

### 6.3 Async I/O Primitives

```aria
module std.io.async

/// Async TCP listener
pub struct TcpListener {
    inner: std::net::TcpListener,
    registered: bool,
}

impl TcpListener {
    pub async fn bind(addr: impl ToSocketAddrs) -> io::Result<Self>

    pub async fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
        loop {
            match self.inner.accept() {
                Ok(result) => return Ok(result),
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    // Wait for readiness
                    self.wait_readable().await?
                }
                Err(e) => return Err(e),
            }
        }
    }

    async fn wait_readable(&self) -> io::Result<()> {
        // Register with I/O driver and wait
        // Implementation depends on runtime context
    }
}

/// Async TCP stream
pub struct TcpStream {
    inner: std::net::TcpStream,
}

impl TcpStream {
    pub async fn connect(addr: impl ToSocketAddrs) -> io::Result<Self>

    pub async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>

    pub async fn write(&mut self, buf: &[u8]) -> io::Result<usize>

    pub async fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()>

    pub async fn write_all(&mut self, buf: &[u8]) -> io::Result<()>
}

/// Async file I/O
pub struct File {
    inner: std::fs::File,
}

impl File {
    pub async fn open(path: impl AsRef<Path>) -> io::Result<Self>

    pub async fn create(path: impl AsRef<Path>) -> io::Result<Self>

    pub async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>

    pub async fn write(&mut self, buf: &[u8]) -> io::Result<usize>
}
```

### 6.4 Timer Wheel

```aria
module std.runtime.timer

/// Hierarchical timing wheel for efficient timer management
/// Based on the Tokio timer wheel design
pub struct TimerWheel {
    /// Millisecond-level wheel (64 slots)
    ms_wheel: [Slot; 64],
    /// Second-level wheel (64 slots)
    sec_wheel: [Slot; 64],
    /// Minute-level wheel (64 slots)
    min_wheel: [Slot; 64],
    /// Current time
    now: Instant,
}

impl TimerWheel {
    /// Register a timer
    pub fn register(&mut self, deadline: Instant, waker: Waker) -> TimerHandle

    /// Cancel a timer
    pub fn cancel(&mut self, handle: TimerHandle)

    /// Advance time and fire expired timers
    pub fn advance(&mut self, now: Instant) -> Vec<Waker>

    /// Time until next timer expires
    pub fn next_deadline(&self) -> Option<Duration>
}

// Performance characteristics:
// - Insert: O(1)
// - Cancel: O(1) with handle
// - Advance: O(expired timers)
// - Memory: ~12KB base + 24 bytes per active timer
```

---

## 7. Memory Overhead Targets

### 7.1 Per-Component Costs

| Component | Size | Notes |
|-----------|------|-------|
| **Task state machine** | 64-256 bytes | Depends on captured variables |
| **Task metadata** | 32 bytes | ID, state, waker pointer |
| **Per-worker state** | 4KB | Local queue, stats, RNG |
| **Global queue** | 8KB | 1024-slot MPMC queue |
| **I/O driver** | 32KB | Event buffer, registration table |
| **Timer wheel** | 12KB | Base structure |

### 7.2 Scalability Analysis

| Tasks | Memory (no runtime) | Memory (single-threaded) | Memory (4-core) |
|-------|---------------------|--------------------------|-----------------|
| 1 | 64-256B | 20KB | 48KB |
| 1,000 | 64-256KB | 84KB | 112KB |
| 10,000 | 640KB-2.5MB | 704KB | 732KB |
| 100,000 | 6.4-25MB | 6.4MB | 6.5MB |
| 1,000,000 | 64-256MB | 64MB | 64MB |

### 7.3 Comparison with Other Runtimes

| Runtime | Per-Task Cost | 1M Tasks |
|---------|---------------|----------|
| Go goroutines | ~2.5KB | ~2.5GB |
| Tokio (Rust) | ~100B-1KB | ~100MB-1GB |
| Java Virtual Threads | ~1KB | ~1GB |
| Erlang processes | ~2.6KB | ~2.6GB |
| **Aria (target)** | **64-256B** | **64-256MB** |

---

## 8. Integration with Aria's Effect System

### 8.1 Async as an Effect

```aria
/// The Async effect represents potential suspension
effect Async {
    /// Suspend until the future completes
    fn suspend<T>(future: impl Future<Output = T>) -> T
}

/// Functions with Async effect can suspend
fn fetch_data(url: String) -> Result<Data, Error> with Async {
    let response = perform Async::suspend(http::get(url))
    response.json()
}

/// Effect handler for runtime execution
handler RuntimeHandler for Async {
    fn suspend<T>(future: impl Future<Output = T>) -> T {
        runtime::spawn(future).await
    }
}

/// Effect handler for blocking (no runtime)
handler BlockingHandler for Async {
    fn suspend<T>(future: impl Future<Output = T>) -> T {
        block_on_simple(future)
    }
}
```

### 8.2 Colorblind Execution

```aria
/// Functions are agnostic to execution context
fn read_file(path: Path) -> Result<Vec<u8>, Error> with Async {
    let file = perform Async::suspend(File::open(path))
    let data = perform Async::suspend(file.read_all())
    Ok(data)
}

/// Caller decides execution model
fn main() {
    // Run with runtime (concurrent)
    runtime::block_on(async {
        let data = read_file("config.toml").await?
        process(data)
    })

    // Or run blocking (no runtime needed)
    let data = handle Async with BlockingHandler {
        read_file("config.toml")
    }
    process(data)
}
```

---

## 9. Implementation Phases

### Phase 1: Core Infrastructure (6 weeks)

| Week | Deliverable |
|------|-------------|
| 1-2 | Future trait, Poll, Waker, Context types |
| 3-4 | Compiler state machine transformation |
| 5-6 | Manual polling API, `block_on_simple` |

**Success Criteria**: Embedded blink example works without runtime

### Phase 2: Single-Threaded Runtime (4 weeks)

| Week | Deliverable |
|------|-------------|
| 1-2 | Task queue, single-threaded executor |
| 3 | I/O driver integration (epoll/kqueue) |
| 4 | Timer wheel, `sleep`, `timeout` |

**Success Criteria**: Async TCP echo server works

### Phase 3: Multi-Threaded Runtime (6 weeks)

| Week | Deliverable |
|------|-------------|
| 1-2 | Work-stealing deque implementation |
| 3-4 | Multi-worker scheduler |
| 5 | Soft preemption, yield points |
| 6 | Performance tuning, benchmarks |

**Success Criteria**: Context switch <300ns, >10M tasks/sec throughput

### Phase 4: Production Features (4 weeks)

| Week | Deliverable |
|------|-------------|
| 1 | Structured concurrency (TaskGroup) |
| 2 | Cancellation propagation |
| 3 | WASM support |
| 4 | Documentation, examples |

**Success Criteria**: All API surface documented and tested

---

## 10. Success Metrics

### 10.1 Performance Targets

| Metric | Target | Acceptable | Method |
|--------|--------|------------|--------|
| Context switch (same core) | <100ns | <150ns | Micro-benchmark |
| Context switch (cross core) | <300ns | <500ns | Micro-benchmark |
| Task spawn + poll | <50ns | <100ns | Micro-benchmark |
| 10K concurrent tasks | <1ms latency P99 | <5ms | Load test |
| 100K concurrent tasks | <10ms latency P99 | <50ms | Load test |
| Memory per task | <256 bytes | <512 bytes | Memory profiling |

### 10.2 Compatibility Targets

| Target | Requirement | Status |
|--------|-------------|--------|
| Linux (x86_64, aarch64) | Full runtime | P0 |
| macOS (x86_64, aarch64) | Full runtime | P0 |
| Windows (x86_64) | Full runtime | P1 |
| WASM32 | No-runtime + JS bridge | P0 |
| Embedded (ARM Cortex-M) | No-runtime | P1 |
| FreeBSD | Full runtime | P2 |

### 10.3 API Stability

| API | Stability Commitment |
|-----|---------------------|
| `Future`, `Poll`, `Waker` | Stable from 1.0 |
| `runtime::spawn`, `block_on` | Stable from 1.0 |
| `TaskGroup` | Stable from 1.0 |
| Scheduler internals | Internal, may change |
| I/O driver trait | Unstable, may change |

---

## 11. Risk Mitigation

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Context switch >300ns | Medium | High | Optimize hot paths, profile continuously |
| Memory overhead too high | Low | High | Aggressive state machine optimization |
| Work stealing contention | Medium | Medium | Tune stealing parameters, add backoff |
| I/O driver bugs | Medium | High | Extensive testing on all platforms |
| WASM integration issues | Low | Medium | Early prototype, continuous testing |

---

## 12. Appendix: API Quick Reference

```aria
// Essential imports
use std.runtime.{spawn, block_on, yield_now, TaskHandle}
use std.runtime.{timeout, sleep}
use std.runtime.{task_group, TaskGroup}
use std.future.{Future, Poll}
use std.channel.{channel, bounded, Sender, Receiver}

// Common patterns

// 1. Basic concurrent execution
fn main() {
    runtime::block_on(async {
        let handle = runtime::spawn(async { compute() })
        let result = handle.await?
        println("Result: {result}")
    })
}

// 2. Parallel execution
fn main() {
    runtime::block_on(async {
        let results = runtime::task_group(|group| {
            for url in urls {
                group.spawn(async { fetch(url).await })
            }
        }).await
    })
}

// 3. Timeout handling
fn main() {
    runtime::block_on(async {
        match runtime::timeout(Duration::seconds(5), fetch(url)).await {
            Ok(data) => process(data),
            Err(TimeoutError) => handle_timeout(),
        }
    })
}

// 4. No-runtime embedded
#![no_std]
fn main() -> ! {
    let future = blink_led()
    std::future::block_on_simple(future)
}
```

---

**Document Status**: Approved
**Next Steps**: ARIA-M11-IMPL-01 - Implement Future trait and state machine transformation
**Owner**: Runtime Team
**Reviewers**: NEXUS Research Agent, TITAN Product Agent

---

*Product Design Document created by TITAN for Aria Language Project - Iteration 2*
