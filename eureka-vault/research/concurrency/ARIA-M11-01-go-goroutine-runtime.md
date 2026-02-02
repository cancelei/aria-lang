# ARIA-M11-01: Go Goroutine Runtime Study

**Task ID**: ARIA-M11-01
**Status**: Completed
**Date**: 2026-01-14
**Focus**: Deep dive into Go's scheduler and runtime

---

## Executive Summary

Go's goroutine scheduler is a masterclass in lightweight concurrency. This research analyzes the GMP model, work stealing, and runtime implementation for Aria's concurrency design.

---

## 1. Overview

### 1.1 What Makes Go Concurrency Special?

- **Goroutines**: Lightweight threads (~2KB initial stack)
- **Channels**: Type-safe communication primitives
- **No async/await**: Synchronous-looking concurrent code
- **M:N threading**: Many goroutines on few OS threads

### 1.2 Key Numbers

| Metric | Value |
|--------|-------|
| Initial goroutine stack | ~2KB (grows as needed) |
| Goroutines per program | Millions possible |
| Context switch cost | ~100ns (vs ~1μs for threads) |
| Time slice | ~10ms |

---

## 2. GMP Model

### 2.1 Core Components

```
G (Goroutine)
├── Stack (starts 2KB, grows)
├── Program counter
├── State (runnable, running, blocked)
└── Context for scheduling

M (Machine/OS Thread)
├── Runs goroutines
├── System call handling
├── Parked when idle
└── Limited by GOMAXPROCS

P (Processor)
├── Local run queue (256 slots)
├── Resources for execution
├── Connects G to M
└── Count = GOMAXPROCS
```

### 2.2 Visual Model

```
┌─────────────────────────────────────────────────────────────┐
│                        Go Runtime                            │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Global Run Queue                                            │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  G  G  G  G  G  ...                                  │    │
│  └─────────────────────────────────────────────────────┘    │
│            ↑ (checked 1/61 of the time)                      │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │      P0      │  │      P1      │  │      P2      │       │
│  │ ┌──────────┐ │  │ ┌──────────┐ │  │ ┌──────────┐ │       │
│  │ │Local Q   │ │  │ │Local Q   │ │  │ │Local Q   │ │       │
│  │ │G G G G   │ │  │ │G G G     │ │  │ │G G G G G │ │       │
│  │ └──────────┘ │  │ └──────────┘ │  │ └──────────┘ │       │
│  │      ↓       │  │      ↓       │  │      ↓       │       │
│  │      M0      │  │      M1      │  │      M2      │       │
│  │  (OS Thread) │  │  (OS Thread) │  │  (OS Thread) │       │
│  └──────────────┘  └──────────────┘  └──────────────┘       │
│         ↑                                    ↑               │
│         └──────── Work Stealing ─────────────┘               │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## 3. Work Stealing

### 3.1 Algorithm

```
When P's local queue is empty:
1. Check global queue (1/61 probability)
2. Try to steal from another P
   a. Pick random victim P
   b. Steal half of their goroutines
3. Check network poller
4. If all fail, park the M
```

### 3.2 Local Run Queue Implementation

```go
// Simplified representation
type p struct {
    runqhead uint32           // Head of circular queue
    runqtail uint32           // Tail of circular queue
    runq     [256]guintptr    // Fixed-size circular queue
    runnext  guintptr         // Next G to run (priority)
}
```

**Why 256 slots?**
- Cache-friendly size
- Reduces memory allocation
- Overflow goes to global queue

### 3.3 Stealing Granularity

- Steal **half** of victim's queue
- Minimizes stealing frequency
- Balances load effectively

---

## 4. Scheduling Algorithm

### 4.1 findRunnable() Priority

```
1. runnext (single prioritized G)
2. Local run queue
3. Global run queue (1/61 times)
4. Network poller
5. Work stealing from other Ps
6. Global queue (full check)
7. Network poller (block)
```

### 4.2 Why 1/61 for Global Queue?

- Prime number for better distribution
- Reduces contention on global lock
- Prevents starvation while prioritizing local

### 4.3 Time Slicing (Go 1.24+)

```
- Each G gets ~10ms time slice
- Preemption via async signals (since Go 1.14)
- Cooperative preemption at safe points
- Go 1.24 refines preemption mechanism
```

---

## 5. Stack Management

### 5.1 Growable Stacks

```go
// Goroutine starts with small stack
func newgoroutine() {
    // Initial stack: ~2KB
    // Grows automatically on demand
    // Can grow to 1GB
}
```

### 5.2 Stack Growth Process

```
1. Function prologue checks stack space
2. If insufficient, call runtime.morestack
3. Allocate larger stack (2x)
4. Copy contents to new stack
5. Update pointers
6. Continue execution
```

### 5.3 Stack Shrinking

- Stacks can shrink during GC
- Reduces memory for idle goroutines
- Happens at GC safe points

---

## 6. Channel Implementation

### 6.1 Channel Structure

```go
type hchan struct {
    qcount   uint           // Elements in buffer
    dataqsiz uint           // Buffer capacity
    buf      unsafe.Pointer // Buffer (ring buffer)
    elemsize uint16         // Element size
    closed   uint32         // Closed flag
    sendx    uint           // Send index
    recvx    uint           // Receive index
    recvq    waitq          // Blocked receivers
    sendq    waitq          // Blocked senders
    lock     mutex          // Protects all fields
}
```

### 6.2 Send/Receive Operations

**Unbuffered channel send**:
```
1. If receiver waiting: direct send, wake receiver
2. Else: park sender in sendq
```

**Buffered channel send**:
```
1. If buffer not full: enqueue to buffer
2. Else if receiver waiting: direct send
3. Else: park sender
```

---

## 7. Spinning Threads

### 7.1 Purpose

Avoid frequent thread parking/unparking:
- M with P looking for work = spinning
- M without P looking for P = spinning
- Trade CPU for latency

### 7.2 Spinning Limits

- At most GOMAXPROCS spinning Ms
- Beyond that: park threads
- Balance between latency and CPU

---

## 8. Performance Considerations

### 8.1 Strengths

| Aspect | Benefit |
|--------|---------|
| Low overhead | Millions of goroutines |
| Cache locality | Local queues, wake on same P |
| Balanced load | Work stealing |
| Fast communication | Channels optimized |

### 8.2 Potential Issues

| Issue | Cause | Mitigation |
|-------|-------|------------|
| Work stealing overhead | High GOMAXPROCS | Reduce if not needed |
| Channel contention | Hot channels | Buffer or redesign |
| Stack copying | Deep recursion | Iterative alternatives |

---

## 9. Recommendations for Aria

### 9.1 Concurrency Model

```aria
# Go-inspired but with effects
fn fetch_all(urls: Array[String]) -> {Async} Array[Response]
  urls.map |url|
    spawn { HTTP.get(url) }  # Like goroutines
  end.await_all
end

# Channels
fn producer_consumer() -> {Async}
  ch = Channel.new(Int, capacity: 10)

  spawn do
    100.times |i| ch.send(i) end
    ch.close
  end

  for value in ch
    process(value)
  end
end
```

### 9.2 Runtime Design Options

| Option | Approach | Trade-offs |
|--------|----------|------------|
| Go-like | GMP scheduler | Complex, proven |
| Tokio-like | Work-stealing executor | Rust-native |
| Simple | Thread pool | Less efficient |

### 9.3 Recommendation: Hybrid

```aria
# Aria runtime architecture
AriaRuntime {
  # GMP-inspired for lightweight tasks
  processors: Array[Processor]     # Like P
  workers: Array[Worker]           # Like M
  global_queue: Queue[Task]        # Global run queue

  # Effect integration
  effect_handlers: Map[Effect, Handler]

  # Work stealing
  fn steal_work(from: Processor) -> Option[Task]
}
```

### 9.4 Stack Strategy

```aria
# Options for Aria:
# 1. Segmented stacks (like Go pre-1.4)
# 2. Stackful coroutines (like Go 1.4+)
# 3. Stackless (like Rust async)

# Recommendation: Stackful for Go-like experience
@spawn
fn my_goroutine()
  # Starts with small stack
  # Grows automatically
end
```

### 9.5 Channel Design

```aria
# Typed channels with select
Channel[T] {
  fn send(value: T) -> {Async}
  fn receive() -> {Async} T
  fn close()
}

# Select statement (like Go)
select
  msg = <-inbox    => handle(msg)
  <-timeout(5.sec) => handle_timeout()
  default          => handle_idle()
end
```

---

## 10. Key Resources

1. [Go Scheduler - Melatoni](https://nghiant3223.github.io/2025/04/15/go-scheduler.html)
2. [Go's Work-Stealing Scheduler](https://rakyll.org/scheduler/)
3. [Inside the Go Scheduler (2025)](https://medium.com/@hydrurdgn/inside-the-go-scheduler-a-deep-dive-into-goroutines-m-p-g-preemption-work-stealing-3f4d2c38562f)
4. [Scheduling In Go Part II](https://www.ardanlabs.com/blog/2018/08/scheduling-in-go-part2.html)
5. [The Go Scheduler Deep Dive 2025](https://www.bytesizego.com/blog/go-scheduler-deep-dive-2025)

---

## 11. Open Questions

1. Should Aria use stackful or stackless coroutines?
2. How do effects interact with the scheduler?
3. What's the right default for GOMAXPROCS equivalent?
4. How do we handle blocking FFI calls?
