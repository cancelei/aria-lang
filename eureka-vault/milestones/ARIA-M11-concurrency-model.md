# Milestone M11: Concurrency Model

## Overview

Design Aria's concurrency model with effect-inferred async, goroutine-style lightweight threads, and channels - without the async/await syntax pollution.

## Research Questions

1. How do we hide async/await complexity from users?
2. What's the runtime model - green threads, work stealing?
3. How do channels integrate with the effect system?
4. How do we handle structured concurrency?

## Core Innovation Target

```ruby
# No async/await keywords anywhere
fn fetch_all(urls)
  # Compiler infers IO effect, generates async code
  urls.map { |url| http.get(url) }.await_all
end

# Channels (Go-style)
fn producer_consumer
  ch = Channel.new(Int, capacity: 10)

  spawn { 100.times { |i| ch.send(i) } }

  for value in ch
    process(value)
  end
end

# Select for multiplexing
select
  msg = <-inbox    => handle_message(msg)
  timer.tick       => heartbeat()
  default          => idle()
end
```

## Competitive Analysis Required

| Language | Model | Study Focus |
|----------|-------|-------------|
| Go | Goroutines + channels | Simplicity, runtime |
| Rust | async/await + Tokio | Zero-cost, complexity |
| Kotlin | Coroutines | Structured concurrency |
| Swift | Actors | Actor model |
| Erlang | Processes | Fault tolerance |

## Tasks

### ARIA-M11-01: Study Go's goroutine runtime
- **Description**: Deep dive into Go's scheduler and runtime
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, concurrency, go, runtime
- **Deliverables**:
  - Scheduler design
  - Stack management
  - Channel implementation

### ARIA-M11-02: Analyze Kotlin coroutines
- **Description**: Study Kotlin's structured concurrency
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, concurrency, kotlin, structured
- **Deliverables**:
  - Scope management
  - Cancellation patterns
  - Error propagation

### ARIA-M11-03: Research async runtimes
- **Description**: Compare Tokio, async-std, smol
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, concurrency, rust, runtime
- **Deliverables**:
  - Runtime architectures
  - Work stealing analysis
  - IO integration

### ARIA-M11-04: Study effect-based concurrency
- **Description**: Research effect system approach to async
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, concurrency, effects
- **Deliverables**:
  - Effect handler for async
  - Automatic async inference
  - Code generation patterns

### ARIA-M11-05: Design Aria's concurrency model
- **Description**: Design the overall concurrency approach
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Blocked by**: ARIA-M11-01, ARIA-M11-04
- **Tags**: research, concurrency, design
- **Deliverables**:
  - Runtime architecture
  - spawn/channel syntax
  - Effect integration

### ARIA-M11-06: Design structured concurrency
- **Description**: Design structured concurrency features
- **Dependency**: AGENT_CAPABLE
- **Priority**: normal
- **Blocked by**: ARIA-M11-02, ARIA-M11-05
- **Tags**: research, concurrency, structured, design
- **Deliverables**:
  - Scope management
  - Cancellation semantics
  - Error handling

## Implementation Progress

### Phase 1: Core Concurrency (COMPLETED - Jan 2026)
- [x] Transfer/Sharable traits in aria-types
- [x] Task spawning with `spawn()` and `spawn_blocking()`
- [x] `JoinHandle` and `TaskGroup` for basic structured concurrency
- [x] Async effect handler bridge

### Phase 2: Channels & Select (COMPLETED - Jan 2026)
- [x] `Channel<T>` type in aria-types
- [x] Unbuffered and buffered channels in aria-runtime
- [x] Channel effect with operations (new, send, recv, try_send, try_recv, close)
- [x] Channel send/receive syntax (`<-channel`, `channel <- value`)
- [x] Select statement parsing with receive, send, default arms

### Phase 3: Structured Concurrency (COMPLETED - Jan 2026)
- [x] `Scope` struct for structured concurrency
- [x] `CancelToken` for cooperative cancellation
- [x] `with_scope()` and `with_scope_result()` functions
- [x] `with_supervised_scope()` for fault tolerance
- [x] Error propagation (first error cancels siblings)
- [x] Cancel effect with check/token operations
- [x] Async.scope and Async.supervisor operations

### Phase 4: Runtime Optimization (COMPLETED - Jan 2026)
- [x] Thread pool for scoped tasks (ARIA-CONC-OPT-01)
- [x] Work-stealing scheduler (integrated in thread pool)
- [x] Memory ordering optimization (ARIA-CONC-OPT-02)
- [x] Timeout scopes (ARIA-CONC-ENH-01)
- [x] Scope visualization/debugging tools (ARIA-CONC-ENH-02)
- [x] Performance benchmarks (ARIA-M11-09)
- [x] Timer wheel for efficient timeouts
- [ ] I/O driver integration (deferred to Phase 5)

### Phase 5: WASM Support (PENDING)
- [ ] Single-threaded WASM runtime
- [ ] Cooperative scheduling for WASM

## Success Criteria

- [x] Concurrency model designed
- [x] No visible async/await in user code
- [x] Channels + select working
- [x] Structured concurrency support
- [x] Runtime architecture documented

## Research Outputs

| Document | Description |
|----------|-------------|
| [ARIA-M11-01-concurrency-model.md](../research/concurrency/ARIA-M11-01-concurrency-model.md) | Core concurrency model design |
| [ARIA-M11-01-go-goroutine-runtime.md](../research/concurrency/ARIA-M11-01-go-goroutine-runtime.md) | Go runtime analysis |
| [ARIA-M11-02-kotlin-coroutines.md](../research/concurrency/ARIA-M11-02-kotlin-coroutines.md) | Kotlin coroutines analysis |
| [ARIA-M11-03-rust-async-runtimes.md](../research/concurrency/ARIA-M11-03-rust-async-runtimes.md) | Rust async runtime comparison |
| [ARIA-M11-04-colored-functions-analysis.md](../research/concurrency/ARIA-M11-04-colored-functions-analysis.md) | Colored functions problem analysis |
| [ARIA-M11-05-green-threads-runtime.md](../research/concurrency/ARIA-M11-05-green-threads-runtime.md) | Green threads runtime design |
| [ARIA-M11-06-channel-patterns.md](../research/concurrency/ARIA-M11-06-channel-patterns.md) | Channel implementation patterns |
| [ARIA-M11-07-concurrency-benchmarks.md](../research/concurrency/ARIA-M11-07-concurrency-benchmarks.md) | Concurrency benchmarks |
| [ARIA-M11-08-phase3-implementation-findings.md](../research/concurrency/ARIA-M11-08-phase3-implementation-findings.md) | Phase 3 implementation findings |
| [ARIA-M11-09-benchmark-results.md](../research/concurrency/ARIA-M11-09-benchmark-results.md) | Phase 4 benchmark results |

## Key Resources

1. "Go's Concurrency" - Pike
2. "Structured Concurrency" - Elizarov
3. Tokio documentation
4. "Communicating Sequential Processes" - Hoare
5. Kotlin coroutines guide

## Timeline

Target: Q2-Q3 2026

## Related Milestones

- **Depends on**: M03 (Effect System), M06 (IR)
- **Enables**: Async application development
- **Parallel**: M13 (Error Handling)
