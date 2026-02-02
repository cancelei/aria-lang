# ARIA-M11-01: Concurrency Model Design for Aria

**Task ID**: ARIA-M11-01
**Status**: Research Complete
**Date**: 2026-01-15
**Agent**: NEXUS (Research)
**Focus**: Comprehensive concurrency model design integrating with Aria's ownership and effect systems

---

## Executive Summary

This research document presents Aria's concurrency model design, synthesizing insights from Go's goroutines, Rust's async/await, Kotlin's structured concurrency, Swift's actors, and Erlang's supervision trees. The recommended approach is a **hybrid model combining effect-inferred async with structured concurrency and Go-style channels**, prioritizing safety and ergonomics over raw performance.

**Key Recommendation**: Aria should adopt an effect-based concurrency model where async operations are tracked through the effect system, enabling synchronous-looking code without function coloring while maintaining Rust-level safety guarantees through ownership integration.

---

## 1. Concurrency Model Comparison

### 1.1 Green Threads (Go) vs Async/Await (Rust) vs Actors (Erlang/Swift)

| Aspect | Go Goroutines | Rust async/await | Erlang Actors | Swift Actors |
|--------|---------------|------------------|---------------|--------------|
| **Mental Model** | Concurrent functions | State machines | Message-passing | Isolated reference types |
| **Scheduling** | M:N (runtime) | M:N (runtime) | Preemptive | Cooperative |
| **Stack Model** | Growable (stackful) | Stackless | Fixed per-process | System stacks |
| **Communication** | Channels | futures/streams | Mailboxes | async method calls |
| **Safety** | Runtime (data races possible) | Compile-time (Send/Sync) | Process isolation | Actor isolation + Sendable |
| **Cancellation** | Context-based | Drop-based | Process kill | Task cancellation |
| **Function Coloring** | No | Yes (async) | No | Partial (isolated methods) |
| **Learning Curve** | Low | High | Medium | Medium |

### 1.2 Strengths Analysis

#### Go Goroutines
- **Simplicity**: No async/await, code looks synchronous
- **Low overhead**: ~2KB per goroutine, millions possible
- **Channels**: First-class communication primitives
- **Work-stealing**: Efficient load balancing
- **Weakness**: No compile-time data race prevention

#### Rust async/await
- **Zero-cost abstraction**: No runtime overhead when not used
- **Compile-time safety**: Send/Sync prevent data races
- **Fine-grained control**: Explicit suspension points
- **Interoperability**: Can mix with sync code
- **Weakness**: Function coloring, complex lifetimes with async

#### Erlang Actors
- **Fault tolerance**: "Let it crash" with supervision trees
- **Process isolation**: Each actor has isolated heap
- **Location transparency**: Same code for local/distributed
- **Hot code reloading**: Update running systems
- **Weakness**: Message-passing overhead, no shared memory

#### Swift Actors
- **Compile-time isolation**: Actor isolation checking
- **Sendable protocol**: Type-level data safety
- **MainActor**: UI thread integration
- **Structured concurrency**: TaskGroup, async let
- **Weakness**: Still has function coloring (async methods)

### 1.3 Recommendation for Aria

**Primary Model**: Effect-based structured concurrency with channels
- Takes Go's simplicity (no function coloring)
- Takes Rust's safety (compile-time checks via ownership)
- Takes Kotlin's structured concurrency (scope-based lifecycle)
- Takes channels from Go (type-safe communication)
- Uses Aria's effect system to track async operations

---

## 2. Interaction with Aria's Ownership Model

### 2.1 The Send/Sync Problem

Rust's `Send` and `Sync` traits determine what can safely cross thread boundaries:
- **Send**: Type can be transferred to another thread
- **Sync**: Type can be shared between threads (T: Sync if &T: Send)

Aria needs equivalent concepts to prevent data races at compile time.

### 2.2 Proposed Aria Model: Transfer and Sharable

```aria
# Built-in traits for thread safety
trait Transfer
  # Type can be safely moved to another task
  # Automatically derived for types with all Transfer fields
end

trait Sharable
  # Type can be safely shared (via reference) between tasks
  # T: Sharable if ref T: Transfer
end

# Ownership keywords interact with concurrency
fn spawn_task[T: Transfer](f: () -> {Async} T) -> Task[T]
  # f must only capture Transfer values
end

fn spawn_shared[T: Sharable](data: ref T, f: (ref T) -> {Async} Unit)
  # data must be Sharable to be accessed from multiple tasks
end
```

### 2.3 Automatic Transfer Derivation

```aria
# Transfer derived automatically for value types
struct Point(x: Float, y: Float)  # Transfer derived

# Non-Transfer types (contain non-Transfer fields)
struct Connection
  socket: RawSocket  # RawSocket is not Transfer
end

# Explicit opt-in for types with interior mutability
@unsafe_transfer  # Developer asserts thread safety
struct AtomicCounter
  value: AtomicInt
end

# @shared types are always Transfer (reference counted)
@shared class SharedState
  data: String
end
```

### 2.4 Integration with Ownership Tiers

```aria
# Tier 1: Inferred Transfer (80% of code)
fn parallel_process(items: Array[Item]) -> {Async} Array[Result]
  # Compiler infers:
  # - items is Transfer (Array[Item] where Item: Transfer)
  # - Move semantics apply (items moved into async context)
  items.par_map |item| process(item) end
end

# Tier 2: Explicit annotations for shared data
fn with_shared_state[life L](state: ref[L] SharedState) -> {Async}
  # state is borrowed, must be Sharable
  # L lifetime tracks scope boundary
end

# Tier 3: @shared for complex sharing patterns
@shared class TaskPool
  tasks: Array[Task[Unit]]

  fn spawn_into(self, f: () -> {Async} Unit)
    self.tasks.push(spawn f)
  end
end
```

### 2.5 Move Semantics in Concurrent Context

```aria
# Values are moved into tasks by default
fn example()
  data = create_data()  # owned

  spawn do
    # data is MOVED here, no longer accessible outside
    process(data)
  end

  # ERROR: data was moved into spawned task
  # print(data)
end

# Explicit copy when needed
fn example_with_copy()
  data = create_data()

  spawn do
    process(copy data)  # explicit copy
  end

  print(data)  # OK: we still have our copy
end

# Shared access via @shared
fn example_with_shared()
  data = @shared SharedData.new()

  spawn do
    data.update()  # OK: @shared handles synchronization
  end

  data.read()  # OK: can access from multiple places
end
```

---

## 3. Structured Concurrency Design

### 3.1 Core Principles

Based on Kotlin's coroutine model and the "Notes on Structured Concurrency" paper:

1. **Scoped Lifetime**: Tasks cannot outlive their parent scope
2. **Cancellation Propagation**: Cancelling parent cancels all children
3. **Error Propagation**: Child failure propagates to parent
4. **Resource Safety**: Resources automatically cleaned up on scope exit

### 3.2 Aria Scope Syntax

```aria
# Basic scope - waits for all children
fn fetch_user_data(user_id: Int) -> {Async} UserData
  with Async.scope |scope|
    # All spawned tasks bound to this scope
    profile = scope.spawn fetch_profile(user_id)
    posts = scope.spawn fetch_posts(user_id)
    friends = scope.spawn fetch_friends(user_id)

    # Wait for all and construct result
    UserData(
      profile: profile.await,
      posts: posts.await,
      friends: friends.await
    )
  end
  # Scope automatically waits for all children before exiting
end
```

### 3.3 Supervisor Scope (Erlang-inspired)

```aria
# Supervisor scope - children fail independently
fn resilient_fetch(urls: Array[String]) -> {Async} Array[Result[Response, Error]]
  with Async.supervisor |scope|
    tasks = urls.map |url|
      scope.spawn fetch(url)
    end

    # Each task completes independently
    # Failures don't cancel siblings
    tasks.map |t| t.await_result end
  end
end

# With restart strategies (Erlang OTP inspired)
with Async.supervisor(strategy: :one_for_one, max_restarts: 3) |scope|
  scope.spawn(restart: :permanent) worker1()
  scope.spawn(restart: :transient) worker2()  # Only restart on abnormal exit
  scope.spawn(restart: :temporary) worker3()   # Never restart
end
```

### 3.4 Cancellation Model

```aria
# Cancellation via scope
fn long_operation() -> {Async, Cancel} Result
  for item in items
    Cancel.check()  # Cancellation checkpoint
    process(item)
  end
end

# Timeout wrapper
fn with_timeout() -> {Async} Result[Data, TimeoutError]
  with Async.timeout(5.seconds)
    long_operation()
  end
end

# Manual cancellation
fn controllable_operation()
  cancel_token = CancelToken.new()

  task = spawn with cancel_token
    long_operation()
  end

  # Later...
  cancel_token.cancel()  # Signals cancellation
  task.await  # Returns Err(Cancelled)
end
```

### 3.5 Nursery Pattern (Trio-inspired)

```aria
# Nursery ensures all tasks complete before continuing
fn parallel_work() -> {Async}
  with Async.nursery |nursery|
    for item in items
      nursery.start_soon process(item)
    end
  end
  # All tasks guaranteed complete here
end

# Nursery with first-result semantics
fn race_fetch(urls: Array[String]) -> {Async} Response
  with Async.nursery(cancel_on_first: true) |nursery|
    for url in urls
      nursery.start_soon fetch(url)
    end
  end
  # Returns first successful result, cancels others
end
```

---

## 4. Proposed Syntax for spawn/await/channels

### 4.1 spawn Syntax

```aria
# Basic spawn - returns Task handle
task = spawn expensive_computation()

# Block spawn
task = spawn do
  step1()
  step2()
  step3()
end

# Named spawn (for debugging/tracing)
task = spawn(name: "data-processor") process_data()

# Spawn with explicit transfer
data = create_data()
task = spawn do
  # data is moved into task
  process(move data)
end

# Spawn onto specific executor
task = spawn(on: Executor.io) do
  read_file(path)
end
```

### 4.2 await Syntax

```aria
# Method-style await (preferred for chaining)
result = task.await

# Await with timeout
result = task.await(timeout: 5.seconds)

# Await multiple tasks
(a, b, c) = (task1, task2, task3).await_all

# Await first to complete (racing)
result = [task1, task2, task3].await_first

# Await with result handling
match task.await_result
  Ok(value) => process(value)
  Err(e) => handle_error(e)
end
```

### 4.3 Channel Syntax

```aria
# Create typed channel
ch = Channel[Int].new()           # Unbuffered
ch = Channel[String].new(cap: 10) # Buffered

# Send and receive
ch.send(42)           # Blocks if full/no receiver
value = ch.recv()     # Blocks if empty
value = <-ch          # Shorthand for recv

# Non-blocking variants
ch.try_send(42)       # Returns Result
ch.try_recv()         # Returns Option

# Close channel
ch.close()

# Iterate over channel (until closed)
for value in ch
  process(value)
end
```

### 4.4 Select Syntax (Go-inspired)

```aria
# Basic select
select
  msg = <-inbox    => handle_message(msg)
  <-timer.tick     => heartbeat()
  response_ch.send(data) => log("sent")
  default          => idle()
end

# Select with timeout
select(timeout: 1.second)
  msg = <-inbox => handle(msg)
  timeout       => handle_timeout()
end

# Select in loop
loop
  select
    msg = <-inbox    =>
      if msg.quit?
        break
      end
      handle(msg)
    <-shutdown       => break
  end
end
```

### 4.5 Parallel Iterators (Rayon-inspired)

```aria
# Parallel map
results = items.par.map |item| process(item) end

# Parallel filter
evens = numbers.par.filter |n| n.even? end

# Parallel reduce
sum = numbers.par.reduce(0) |acc, n| acc + n end

# Chained parallel operations
result = data
  .par
  .filter |x| x.valid? end
  .map |x| transform(x) end
  .collect
```

---

## 5. Effect System Integration

### 5.1 Async as an Effect

```aria
# Async is tracked as an effect
fn fetch_data(url: String) -> {Async, IO} Response
  http.get(url)
end

# Effect inference eliminates explicit annotation
fn process_urls(urls: Array[String])
  # Compiler infers: -> {Async, IO} Array[Response]
  urls.map |url| fetch_data(url) end.await_all
end

# Pure functions have no Async effect
fn calculate(x: Int) -> Int  # No effect annotation needed
  x * 2 + 1
end
```

### 5.2 Hiding Async from Users

```aria
# User writes synchronous-looking code
fn main()
  user = fetch_user(123)
  posts = fetch_posts(user.id)
  render(user, posts)
end

# Compiler infers effects and generates async code
# Equivalent to:
fn main() -> {Async, IO}
  user = fetch_user(123).await
  posts = fetch_posts(user.id).await
  render(user, posts)
end
```

### 5.3 Effect Handlers for Runtime

```aria
# Default async handler (work-stealing)
with Async.runtime(workers: CPU.count)
  main()
end

# Single-threaded handler (for WASM/scripts)
with Async.single_threaded
  main()
end

# Custom handler
handler MyRuntime for Async
  fn spawn(f) = custom_spawn(f)
  fn sleep(d) = custom_sleep(d)
end

with MyRuntime.handler
  main()
end
```

---

## 6. Runtime Architecture

### 6.1 Recommended Architecture

```
Aria Async Runtime
├── Scheduler (work-stealing)
│   ├── Global queue (overflow/injection)
│   ├── Worker threads (ARIA_WORKERS or CPU count)
│   │   └── Local run queue (lock-free, 256 slots)
│   └── Work stealing (random victim, steal half)
├── I/O Driver
│   ├── Linux: io_uring (preferred) / epoll
│   ├── macOS: kqueue
│   └── Windows: IOCP
├── Timer Driver
│   └── Hierarchical timer wheel
├── Blocking Pool
│   └── Thread pool for blocking FFI calls
└── Channel Implementation
    ├── Unbuffered: rendezvous semantics
    └── Buffered: lock-free ring buffer
```

### 6.2 Task Representation

```aria
# Internal task structure (simplified)
struct Task[T]
  state: TaskState        # Pending, Running, Completed, Cancelled
  result: Option[T]       # Result when completed
  waker: Option[Waker]    # For wake-up notification
  scope: Option[Scope]    # Parent scope reference

  # For structured concurrency
  children: Array[Task[Any]]
  cancel_token: CancelToken
end

enum TaskState
  Pending
  Running
  Completed(Result)
  Cancelled
end
```

### 6.3 WASM Target Considerations

```aria
# WASM runtime (single-threaded)
# - No true parallelism (until threads proposal)
# - Event loop integration
# - Promise interop

@target(:wasm)
with Async.wasm_runtime
  # Uses JavaScript event loop
  # spawn creates Promise-backed tasks
  main()
end
```

---

## 7. Safety Guarantees

### 7.1 Compile-Time Guarantees

| Guarantee | Mechanism |
|-----------|-----------|
| No data races | Transfer/Sharable traits (like Rust's Send/Sync) |
| No use-after-move | Ownership tracking in async contexts |
| Structured lifetimes | Scope-bound task lifetimes |
| No forgotten tasks | Structured concurrency enforces await |
| Type-safe channels | Generic Channel[T] with Transfer bound |

### 7.2 Runtime Safety

| Feature | Implementation |
|---------|----------------|
| Deadlock detection | Optional runtime check (debug mode) |
| Stack overflow | Growable stacks (native) / no stacks (WASM) |
| Memory leaks | Scope-based cleanup, @shared ref counting |
| Panic handling | Per-task panic recovery, scope propagation |

### 7.3 Error Messages

```
error[E1001]: cannot spawn task capturing non-Transfer value
 --> src/main.aria:10:5
  |
8 | let conn = establish_connection()
  |     ---- `conn` has type `Connection` which is not Transfer
9 |
10| spawn do
  | ^^^^^ cannot capture `conn` in spawned task
11|   conn.send(data)
  |   ---- `conn` captured here
  |
  = help: `Connection` contains `RawSocket` which cannot be safely transferred
  = help: consider using a channel to communicate with the connection
  = help: or wrap in @shared for reference-counted sharing
```

---

## 8. Comparison with Design Goals

### 8.1 Alignment with Aria's Vision

| Goal | How This Design Achieves It |
|------|----------------------------|
| No function coloring | Effect inference hides async; no `async` keyword needed |
| Safety without complexity | Transfer trait auto-derived; ownership rules extended |
| Go-like simplicity | spawn/channels feel like Go; no explicit futures |
| Rust-like safety | Compile-time data race prevention via ownership |
| Structured concurrency | Scope-based task management prevents leaks |
| Effect integration | Async as first-class effect with handlers |

### 8.2 Trade-offs Accepted

| Trade-off | Benefit | Cost |
|-----------|---------|------|
| Effect tracking overhead | Type-safe async | Slight compile time increase |
| No bare async/await | Cleaner syntax | Less familiar to Rust developers |
| Mandatory Transfer bounds | Data race freedom | Some types need annotation |
| Structured concurrency | No leaked tasks | Can't easily detach tasks |

---

## 9. Implementation Roadmap

### Phase 1: Core Primitives (8 weeks)
1. Transfer/Sharable trait system
2. Basic spawn/await
3. Single-threaded runtime
4. Effect inference for Async

### Phase 2: Channels & Select (6 weeks)
1. Channel[T] implementation
2. Select statement
3. Buffered channels
4. Channel iteration

### Phase 3: Structured Concurrency (6 weeks)
1. Async.scope
2. Cancellation propagation
3. Supervisor scope
4. Error propagation

### Phase 4: Runtime Optimization (8 weeks)
1. Work-stealing scheduler
2. I/O driver integration
3. Timer wheel
4. Performance benchmarks

### Phase 5: WASM Support (4 weeks)
1. Single-threaded WASM runtime
2. Promise interop
3. Event loop integration

---

## 10. Open Questions

1. **Blocking FFI Integration**: How to handle blocking C calls in async context?
   - Recommendation: spawn_blocking executor like Tokio

2. **Actor Support**: Should Aria have first-class actors?
   - Recommendation: Defer to v2.0; channels cover most use cases

3. **Distributed Computing**: Should channels work across nodes?
   - Recommendation: Defer; focus on single-node first

4. **Debug Tooling**: How to debug async code?
   - Recommendation: Task tracing, async stack traces

---

## 11. References

### Research Sources
1. [Go Goroutine Runtime Study](./ARIA-M11-01-go-goroutine-runtime.md) - Internal
2. [Kotlin Coroutines Structured Concurrency](./ARIA-M11-02-kotlin-coroutines.md) - Internal
3. [Rust Async Runtimes Comparison](./ARIA-M11-03-rust-async-runtimes.md) - Internal

### External References
1. [Swift Actor Model Proposal](https://github.com/apple/swift-evolution/blob/main/proposals/0306-actors.md)
2. [Swift 6.2 Approachable Concurrency](https://www.avanderlee.com/concurrency/approachable-concurrency-in-swift-6-2-a-clear-guide/)
3. [Understanding Sendable in Swift](https://gayeugur.medium.com/understanding-sendable-in-swift-concurrency-93f2a42153d8)
4. [Erlang OTP Design Principles](https://www.erlang.org/doc/system/design_principles.html)
5. [Gleam OTP Library](https://github.com/gleam-lang/otp)
6. [Notes on Structured Concurrency](https://vorpus.org/blog/notes-on-structured-concurrency-or-go-statement-considered-harmful/) - Nathaniel J. Smith
7. [Kotlin Coroutines Guide](https://kotlinlang.org/docs/coroutines-guide.html)
8. [Tokio Runtime Design](https://tokio.rs/blog/2019-10-scheduler)

---

## Appendix A: Complete Syntax Examples

### A.1 Web Server Example

```aria
module WebServer

import std::net::{TcpListener, TcpStream}
import std::http::{Request, Response}

fn main()
  listener = TcpListener.bind("0.0.0.0:8080")
  print("Server listening on port 8080")

  # Accept connections concurrently
  for stream in listener.incoming()
    spawn handle_connection(stream)
  end
end

fn handle_connection(stream: TcpStream) -> {Async, IO}
  request = Request.parse(stream)
  response = route(request)
  stream.write(response.serialize())
end

fn route(request: Request) -> Response
  match request.path
    "/" => Response.ok("Welcome!")
    "/users" => fetch_users().to_json_response()
    _ => Response.not_found()
  end
end

fn fetch_users() -> {Async, IO} Array[User]
  # Parallel fetch from multiple sources
  with Async.scope |scope|
    db_users = scope.spawn Database.get_users()
    cache_users = scope.spawn Cache.get_users()

    # Merge results
    (db_users.await ++ cache_users.await).unique
  end
end
```

### A.2 Producer-Consumer Pattern

```aria
fn producer_consumer_example()
  ch = Channel[Int].new(cap: 100)
  done = Channel[Unit].new()

  # Multiple producers
  for i in 1..4
    spawn do
      for j in 1..100
        ch.send(i * 1000 + j)
      end
    end
  end

  # Close after all producers done
  spawn do
    # Wait for producers (simplified)
    sleep(1.second)
    ch.close()
  end

  # Multiple consumers
  for _ in 1..2
    spawn do
      for value in ch
        process(value)
      end
      done.send(())
    end
  end

  # Wait for consumers
  done.recv()
  done.recv()
  print("All done!")
end
```

### A.3 Structured Concurrency with Error Handling

```aria
fn fetch_user_dashboard(user_id: Int) -> {Async} Result[Dashboard, DashboardError]
  with Async.scope |scope|
    # Start all fetches
    profile = scope.spawn fetch_profile(user_id)
    activity = scope.spawn fetch_activity(user_id)
    recommendations = scope.spawn fetch_recommendations(user_id)

    # Handle individual failures gracefully
    profile_result = profile.await_result
    activity_result = activity.await_result
    recommendations_result = recommendations.await_result

    match profile_result
      Err(e) => return Err(DashboardError.profile(e))
      Ok(p) =>
        Dashboard(
          profile: p,
          activity: activity_result.unwrap_or([]),
          recommendations: recommendations_result.unwrap_or([])
        ).ok
    end
  end
end
```

---

**Document Status**: Research Complete
**Next Steps**:
- ARIA-M11-04: Effect-based concurrency prototype
- ARIA-M11-05: Runtime implementation design
**Owner**: NEXUS Research Agent
**Reviewers**: FORGE, GUARDIAN
