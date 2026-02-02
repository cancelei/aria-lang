# ARIA-M11-05: Green Thread Runtime Research

**Agent**: NEXUS
**Eureka Iteration**: 2
**Date**: 2026-01-15
**Status**: Research Complete

---

## Executive Summary

This document presents comprehensive research on green thread runtime designs to inform Aria's concurrency model. After analyzing Go's goroutines, Tokio (Rust), Java Virtual Threads, Erlang/BEAM, and Zig's approach, the recommendation is for Aria to adopt a **hybrid stackless-first design with optional runtime support**, enabling both zero-cost abstractions for embedded/systems work and ergonomic concurrent programming for application development.

---

## 1. Comparison Table: Runtime Implementations

| Feature | Go Goroutines | Tokio (Rust) | Java Virtual Threads | Erlang/BEAM | Zig |
|---------|---------------|--------------|---------------------|-------------|-----|
| **Model** | Stackful (M:N) | Stackless (M:N) | Stackful continuations | Stackful processes | Stackless (planned) |
| **Initial Memory** | ~2KB stack + 392B descriptor | 16B-1KB per future | Few hundred bytes | ~2.6KB (327 words) | 0 (state machine) |
| **Max Memory/Unit** | ~1GB stack | Future size | Dynamic (heap) | Grows as needed | Compile-time known |
| **Preemption** | Preemptive (signal-based) | Cooperative (.await) | Cooperative (blocking calls) | Preemptive (reductions) | Cooperative |
| **Scheduler** | Work-stealing (GMP) | Work-stealing | Platform thread pool | Per-core run queues | None (caller-provided) |
| **Runtime Required** | Yes (embedded) | Yes (library) | Yes (JVM) | Yes (BEAM VM) | Optional |
| **Function Coloring** | No | Yes (async/sync split) | No | No | No (goal) |
| **Context Switch Cost** | Low (~50ns) | Very low (<50ns) | Stack copy to/from heap | Very low (reductions) | Zero (state machine) |
| **Garbage Collection** | Yes (per-goroutine) | No (ownership) | Yes (JVM GC) | Per-process GC | No |

### Sources
- [Go Scheduler Deep Dive](https://www.bytesizego.com/blog/go-scheduler-deep-dive-2025)
- [Tokio Runtime Documentation](https://docs.rs/tokio/latest/tokio/runtime/index.html)
- [Java Virtual Threads Guide](https://rockthejvm.com/articles/the-ultimate-guide-to-java-virtual-threads)
- [Erlang Memory Documentation](https://www.erlang.org/doc/system/memory.html)
- [Zig's New Async I/O](https://kristoff.it/blog/zig-new-async-io/)

---

## 2. Scheduler Algorithm Analysis

### 2.1 Work-Stealing Schedulers

Work-stealing is the dominant approach for efficient parallel task scheduling. The algorithm was formalized by Blumofe and Leiserson and is used by Go, Tokio, Cilk, Java Fork/Join, and .NET Task Parallel Library.

#### Core Algorithm

```
Each processor has a deque (double-ended queue) of tasks:
- Workers push/pop from the BOTTOM (LIFO - cache locality)
- Thieves steal from the TOP (FIFO - larger/older tasks)

When a worker's deque is empty:
1. Select a random victim processor
2. Attempt to steal half of victim's tasks
3. If successful, push stolen tasks to local deque
4. If unsuccessful, try another random victim
5. Repeat until work found or termination detected
```

#### Performance Bounds

The expected execution time with work-stealing is:
```
T_p = T_1/P + O(T_infinity)
```
Where:
- T_1 = serial execution time
- T_infinity = critical path length
- P = number of processors

Space bound: S_1 * P (serial space times processors)

#### Implementation Strategies

| Strategy | Description | Used By |
|----------|-------------|---------|
| **Child Stealing** | Spawned task goes to local queue; parent continues | TBB, .NET TPL, OpenMP |
| **Continuation Stealing** | Parent continuation can be stolen; spawned task runs | Cilk, Go |

Continuation stealing requires compiler support but achieves better cache locality for recursive divide-and-conquer algorithms.

#### Go's GMP Model

Go implements a sophisticated work-stealing scheduler with three entities:

- **G (Goroutine)**: Lightweight user-space thread (~2KB initial stack)
- **M (Machine)**: OS thread that executes goroutines
- **P (Processor)**: Logical processor (GOMAXPROCS), holds run queue

```
┌─────────────────────────────────────────────────────────┐
│                     Global Run Queue                     │
└─────────────────────────────────────────────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        ▼                   ▼                   ▼
   ┌────────┐          ┌────────┐          ┌────────┐
   │   P0   │          │   P1   │          │   P2   │
   │ [G,G,G]│          │ [G,G]  │          │ [G]    │
   └────────┘          └────────┘          └────────┘
        │                   │                   │
        ▼                   ▼                   ▼
   ┌────────┐          ┌────────┐          ┌────────┐
   │   M0   │          │   M1   │          │   M2   │
   └────────┘          └────────┘          └────────┘
```

**Stealing Rules**:
1. Check local run queue
2. Check global run queue (1/61 of the time for fairness)
3. Check network poller
4. Steal from another P's local queue (take half)

#### Tokio's Work-Stealing

Tokio uses a multi-threaded, work-stealing scheduler with:
- One worker thread per CPU core (configurable)
- Each worker has a local run queue
- Work-stealing between workers when idle
- Fairness guarantee: if task count doesn't grow unbounded and no task blocks, all tasks get scheduled fairly

```rust
// Tokio automatically distributes work
#[tokio::main]
async fn main() {
    let handles: Vec<_> = (0..1000)
        .map(|i| tokio::spawn(async move { /* task */ }))
        .collect();

    for handle in handles {
        handle.await.unwrap();
    }
}
```

### 2.2 Fair Scheduling (Erlang/BEAM)

Erlang takes a different approach with **reduction-based scheduling**:

```
Each process gets a "reduction budget" (~2000 reductions)
One reduction = one unit of work (function call, arithmetic, message send)

When budget exhausted:
1. Process is preempted
2. Scheduler picks next process from run queue
3. Preempted process goes to end of queue
```

**Architecture**:
- One scheduler per CPU core
- Each scheduler has its own run queue (no lock contention)
- Migration logic balances load between schedulers

**Advantages**:
- True preemption without OS signals
- Predictable latency
- No process can monopolize CPU
- Fair scheduling in user space (faster than kernel)

**Sources**:
- [Deep Diving Into the Erlang Scheduler](https://blog.appsignal.com/2024/04/23/deep-diving-into-the-erlang-scheduler.html)
- [Erlang Scheduler Details](https://hamidreza-s.github.io/erlang/scheduling/real-time/preemptive/migration/2016/02/09/erlang-scheduler-details.html)

---

## 3. Memory and Performance Characteristics

### 3.1 Memory Overhead Per Unit

| Runtime | Initial Size | Growth Strategy | Maximum Size |
|---------|-------------|-----------------|--------------|
| **Go Goroutine** | 2KB stack + 392B descriptor (~2.4KB) | Doubling (2KB→4KB→8KB...) | ~1GB (64-bit) |
| **Tokio Future** | Size of state machine (varies) | None (fixed at compile time) | Fixed |
| **Java Virtual Thread** | Few hundred bytes | Dynamic (heap-allocated) | Heap limited |
| **Erlang Process** | 327 words (~2.6KB on 64-bit) | GC-managed growth | System memory |
| **Rust Future (boxed)** | 16 bytes + heap | None | Fixed |

### 3.2 Measured Scalability

| Runtime | 100K Units | 1M Units | 10M Units |
|---------|-----------|----------|-----------|
| **Go** | ~240MB | ~2.4GB | ~24GB |
| **Tokio** | ~40-100MB | ~400MB-1GB | Depends on future size |
| **Java VT** | ~100MB | ~1GB | Feasible |
| **Erlang** | ~260MB | ~2.6GB | Proven at scale |

### 3.3 Context Switch Overhead

| Runtime | Switch Cost | Notes |
|---------|-------------|-------|
| **Go** | ~50ns | Signal-based preemption adds overhead |
| **Tokio** | <50ns | State machine transition |
| **Java VT** | Stack copy cost | Heap allocation/deallocation |
| **Erlang** | ~few ns | Reduction counter check |
| **OS Thread** | ~1-10μs | Full context switch |

### 3.4 Stack Management Strategies

#### Segmented Stacks (Historical Go)
- Allocate small stack segments
- Link segments when more space needed
- **Problem**: "Hot split" - constant alloc/dealloc at boundary

#### Contiguous Stacks (Current Go)
- Start with small stack (2KB)
- Copy to larger contiguous region when needed
- **Advantage**: No hot split, simple pointer arithmetic

#### Stackless (Rust/Zig)
- State machine stored in future object
- No runtime stack management
- **Advantage**: Zero overhead, compile-time known size
- **Disadvantage**: Function coloring, complex nested states

#### Heap-Stored Stacks (Java VT)
- Stack frames stored in Java heap
- Copied to carrier thread stack when executing
- **Advantage**: Works with existing JVM GC
- **Disadvantage**: Copy overhead on mount/unmount

**Sources**:
- [Go Stack Size Evolution](https://medium.com/a-journey-with-go/go-how-does-the-goroutine-stack-size-evolve-447fc02085e5)
- [Goroutine Memory Footprint](https://tpaschalis.me/goroutines-size/)
- [Tokio Future Size Discussion](https://github.com/tokio-rs/tokio/discussions/4678)

---

## 4. Preemption Models

### 4.1 Cooperative Preemption

**How it works**: Tasks voluntarily yield control at specific points (I/O, function calls, explicit yield).

**Used by**: Tokio (Rust), JavaScript, Early Go (<1.14)

**Advantages**:
- Simple implementation
- No signal handling complexity
- Predictable yield points
- Lower overhead

**Disadvantages**:
- Long-running CPU tasks can starve others
- Requires programmer discipline
- One bad actor blocks entire thread

### 4.2 Preemptive Scheduling

**How it works**: Runtime forcibly interrupts tasks after time quantum.

**Used by**: Go (1.14+), Erlang, OS threads

#### Go's Signal-Based Preemption (1.14+)

```
sysmon daemon (runs on dedicated M without P):
1. Monitors goroutines using P for >10ms
2. Sends SIGURG signal to thread
3. Signal handler triggers goroutine preemption
4. Goroutine state saved at safe point
5. Scheduler runs next goroutine
```

**Advantages**:
- Prevents starvation
- Improved latency
- No programmer intervention needed

**Disadvantages**:
- Platform-specific signal handling
- Overhead of signal delivery
- Complexity in finding safe points

#### Erlang's Reduction-Based Preemption

```
Every operation costs "reductions":
- Function call: ~1 reduction
- BIF call: varies (send: ~10, spawn: ~100)
- GC: charged to process

Process preempted when reduction count reaches limit (~2000)
```

**Advantages**:
- Deterministic preemption
- No signals needed
- Works identically on all platforms
- Fine-grained fairness

**Disadvantages**:
- Requires control over all operations
- Not applicable to FFI/native code
- Compiler must instrument all paths

### 4.3 Hybrid Approaches

**Go's Approach**: Cooperative preemption via function prologues + asynchronous preemption via signals for tight loops.

**Recommendation for Aria**: Consider reduction-like model for Aria VM with cooperative yield points in native code.

**Sources**:
- [Go 1.14 Preemption](https://medium.com/@hydrurdgn/inside-the-go-scheduler-a-deep-dive-into-goroutines-m-p-g-preemption-work-stealing-3f4d2c38562f)
- [Cooperative vs Preemptive Analysis](https://medium.com/traveloka-engineering/cooperative-vs-preemptive-a-quest-to-maximize-concurrency-power-3b10c5a920fe)

---

## 5. Key Design Questions Answered

### Q1: What's the minimum viable runtime for green threads?

**Minimum Requirements**:
1. **Stack/State Management**: Either segmented stacks, contiguous growable stacks, or state machines
2. **Scheduler**: Round-robin at minimum, work-stealing for multi-core efficiency
3. **Yield Mechanism**: Context save/restore or state machine transition
4. **I/O Integration**: Non-blocking I/O with readiness notification (epoll/kqueue/IOCP)

**Minimal Implementation (~200-500 LOC)**:
- Round-robin scheduler
- Cooperative yield points
- Single-threaded execution
- Platform-specific context switching (assembly required)

**Production Implementation**:
- Work-stealing scheduler
- Multi-threaded execution
- Preemption mechanism
- Integrated I/O reactor
- Timer management

### Q2: How do work-stealing schedulers achieve load balancing?

1. **Randomized Victim Selection**: Prevents contention, distributes stealing
2. **Steal Half**: Takes meaningful work, amortizes stealing overhead
3. **FIFO Stealing**: Steals older/larger tasks (likely to spawn more work)
4. **Local Queue Priority**: Maintains cache locality
5. **Global Queue Fallback**: Handles initial task distribution

### Q3: What's the memory overhead per green thread/goroutine?

| Implementation | Minimum | Typical | Maximum |
|----------------|---------|---------|---------|
| Go Goroutine | ~2.4KB | ~4-8KB | ~1GB |
| Tokio Task | ~16B (boxed) | ~100B-1KB | Compile-time fixed |
| Java VT | ~200B | ~1KB | Heap-limited |
| Erlang Process | ~2.6KB | ~4KB | System memory |

### Q4: How does preemption work in cooperative vs preemptive models?

**Cooperative**: Runtime waits for task to yield (at .await, function calls, I/O)
**Preemptive**: Runtime forcibly suspends task (via signals, reduction counting, or timer interrupts)

Modern systems often use **hybrid approaches**:
- Cooperative for common case (low overhead)
- Preemptive as backstop (prevents starvation)

### Q5: Can we have optional runtime (compile-time choice)?

**Yes - Examples**:

1. **Rust/Tokio**: Language provides async/await syntax; runtime is a library choice (Tokio, async-std, smol, embassy)

2. **Zig (planned)**: New Io interface is caller-provided, enabling:
   - Blocking I/O (no runtime)
   - Thread pool (minimal runtime)
   - Green threads (full runtime)

3. **Approach for Aria**:
   ```
   // No runtime - blocking
   fn main() {
       let data = read_file("data.txt");  // Blocks
   }

   // With runtime - concurrent
   @[runtime: "aria.concurrent"]
   fn main() {
       spawn { read_file("a.txt") }
       spawn { read_file("b.txt") }
   }
   ```

**Sources**:
- [Green Threads Explained in 200 Lines of Rust](https://cfsamson.gitbook.io/green-threads-explained-in-200-lines-of-rust)
- [Zig's Colorblind Async/Await](https://kristoff.it/blog/zig-colorblind-async-await/)
- [The State of Async Rust](https://rust-lang.github.io/async-book/01_getting_started/03_state_of_async_rust.html)

---

## 6. Recommendation for Aria's Runtime Design

### 6.1 Proposed Architecture: Hybrid Stackless-First

```
┌─────────────────────────────────────────────────────────────┐
│                    Aria Concurrency Model                    │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────────┐    ┌─────────────────────────────────┐ │
│  │  Compile-Time   │    │        Optional Runtime          │ │
│  │  (Default)      │    │        (Opt-in)                  │ │
│  ├─────────────────┤    ├─────────────────────────────────┤ │
│  │ Stackless async │    │ Work-stealing scheduler         │ │
│  │ State machines  │    │ Preemptive green threads        │ │
│  │ Zero overhead   │    │ Integrated I/O reactor          │ │
│  │ No coloring*    │    │ Timer wheel                     │ │
│  └─────────────────┘    └─────────────────────────────────┘ │
│                                                              │
└─────────────────────────────────────────────────────────────┘
        │                           │
        ▼                           ▼
   Embedded/Systems            Applications
   No allocator needed         Full concurrency
   Predictable timing          Millions of tasks
```

### 6.2 Core Design Principles

1. **Stackless by Default**
   - Async functions compile to state machines
   - Zero runtime overhead when not using concurrency
   - Works in no_std/embedded contexts

2. **Optional Runtime via Import**
   ```aria
   use aria.runtime.concurrent;  // Enables green thread runtime

   fn main() {
       concurrent.spawn(|| {
           // Runs as green thread
       });
   }
   ```

3. **No Function Coloring** (Zig-inspired)
   - Single function syntax for sync and async
   - Calling context determines execution mode
   - I/O interface provided by caller

4. **Pluggable Executors**
   - Default: single-threaded, cooperative
   - Standard library: multi-threaded work-stealing
   - Custom: user can provide their own

### 6.3 Scheduler Design

**Recommended: Work-Stealing with Reduction Hints**

```aria
// Compiler inserts yield points at:
// - Loop back-edges
// - Function calls
// - I/O operations

// Runtime tracks "work units" (like Erlang reductions)
// Preempts after threshold (e.g., 1000 units or 1ms)
```

**Implementation Layers**:

| Layer | Component | Notes |
|-------|-----------|-------|
| L0 | State machine (compiler) | Stackless coroutine representation |
| L1 | Single-threaded executor | Minimal, for embedded |
| L2 | Multi-threaded work-stealing | Standard library default |
| L3 | Custom executor trait | User-provided implementations |

### 6.4 Memory Model

**Proposed Defaults**:
- Task state: Compile-time known size (stackless)
- Execution stack: Shared among tasks on same thread
- Per-task overhead: ~64-256 bytes (state + metadata)
- No per-task GC (ownership-based like Rust)

### 6.5 Preemption Strategy

**Hybrid Cooperative-Preemptive**:

1. **Cooperative (Default)**: Yield at I/O, channel operations, explicit yield
2. **Soft Preemption**: Compiler-inserted yield checks at loop back-edges
3. **Hard Preemption** (optional): Signal-based for CPU-bound code

```aria
// Developer can opt-in to preemption
@[preemptive]
fn cpu_intensive_loop() {
    for i in 0..1_000_000 {
        // Compiler inserts preemption check
        heavy_computation(i);
    }
}
```

### 6.6 Why This Design?

| Goal | Solution | Rationale |
|------|----------|-----------|
| Zero-cost abstractions | Stackless default | No runtime for non-concurrent code |
| Embedded support | no_std compatible | State machines don't need allocator |
| Ergonomic concurrency | Optional runtime | Spawn millions of tasks when needed |
| No function coloring | Caller-provided I/O | Same function works sync or async |
| Performance | Work-stealing | Proven efficient at scale |
| Fairness | Reduction-style hints | Prevents starvation |

### 6.7 Implementation Roadmap

1. **Phase 1**: Stackless coroutines (compiler)
   - State machine transformation
   - No runtime required
   - Foundation for all async

2. **Phase 2**: Single-threaded executor (stdlib)
   - Basic round-robin scheduler
   - Cooperative yielding
   - I/O reactor integration

3. **Phase 3**: Multi-threaded runtime (stdlib)
   - Work-stealing scheduler
   - Soft preemption via yield points
   - Channel-based communication

4. **Phase 4**: Advanced features
   - Hard preemption (signal-based)
   - Structured concurrency
   - Custom executor trait

---

## 7. References

### Primary Sources

1. [Go's Work-Stealing Scheduler](https://rakyll.org/scheduler/)
2. [Inside the Go Scheduler](https://medium.com/@hydrurdgn/inside-the-go-scheduler-a-deep-dive-into-goroutines-m-p-g-preemption-work-stealing-3f4d2c38562f)
3. [Tokio Runtime Documentation](https://docs.rs/tokio/latest/tokio/runtime/index.html)
4. [Making Tokio Scheduler 10x Faster](https://tokio.rs/blog/2019-10-scheduler)
5. [Project Loom Virtual Threads](https://inside.java/2025/02/22/devoxxbelgium-loom-next/)
6. [Deep Diving Into the Erlang Scheduler](https://blog.appsignal.com/2024/04/23/deep-diving-into-the-erlang-scheduler.html)
7. [Zig's New Async I/O](https://kristoff.it/blog/zig-new-async-io/)
8. [What is Zig's Colorblind Async/Await](https://kristoff.it/blog/zig-colorblind-async-await/)

### Academic References

9. [Scheduling Multithreaded Computations by Work Stealing](https://dl.acm.org/doi/10.1145/324133.324234) - Blumofe & Leiserson
10. [Work Stealing - Wikipedia](https://en.wikipedia.org/wiki/Work_stealing)

### Implementation Guides

11. [Green Threads Explained in 200 Lines of Rust](https://cfsamson.gitbook.io/green-threads-explained-in-200-lines-of-rust)
12. [The State of Async Rust](https://rust-lang.github.io/async-book/01_getting_started/03_state_of_async_rust.html)
13. [no_std async on Embedded](https://ferrous-systems.com/blog/stable-async-on-embedded/)

---

## 8. Appendix: Quick Reference

### Memory Overhead Summary

| Runtime | Per-Unit Cost | 1M Units |
|---------|--------------|----------|
| Go | ~2.5KB | ~2.5GB |
| Tokio | ~100B-1KB | ~100MB-1GB |
| Java VT | ~1KB | ~1GB |
| Erlang | ~2.6KB | ~2.6GB |
| Stackless (ideal) | ~64-256B | ~64-256MB |

### Scheduler Comparison

| Scheduler | Type | Preemption | Best For |
|-----------|------|------------|----------|
| Go GMP | Work-stealing | Signal-based | General purpose |
| Tokio | Work-stealing | Cooperative | I/O-bound async |
| Erlang | Per-core queues | Reduction-based | Fault-tolerant systems |
| Single-threaded | Round-robin | Cooperative | Embedded/simple |

### Aria Design Summary

| Aspect | Choice | Rationale |
|--------|--------|-----------|
| Default model | Stackless state machines | Zero cost when not used |
| Runtime | Optional (compile-time) | Flexibility for different use cases |
| Scheduler | Work-stealing | Proven efficient |
| Preemption | Hybrid (cooperative + soft) | Balance of overhead and fairness |
| Function coloring | None (Zig-inspired) | Better ergonomics |

---

*Document generated by NEXUS research agent for Aria Language Project*
