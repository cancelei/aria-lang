# ARIA-PD-006: Concurrency Model Design Decision

**Document Type**: Product Design Document (PDD)
**Status**: APPROVED
**Date**: 2026-01-15
**Author**: APEX (Product Decision Agent)
**Reviewers**: FLUX, NEXUS, BOLT

---

## Executive Summary

This document establishes Aria's official concurrency model based on comprehensive research from Eureka Iteration 2. After reviewing analysis of Go goroutines, Rust async/await, Kotlin coroutines, Swift actors, Erlang processes, and Zig's colorblind approach, this PDD makes concrete decisions on syntax, semantics, and implementation requirements.

**Final Decision**: Aria adopts a **hybrid effect-based concurrency model** combining:
1. Effect-inferred async operations (no function coloring)
2. Structured concurrency with scope-based task management
3. Go-style channels as first-class primitives
4. Optional runtime (compile-time selectable)
5. Rust-style ownership for thread safety (Transfer/Sharable traits)

---

## 1. Concurrency Model Architecture

### 1.1 Design Principles

| Principle | Decision | Rationale |
|-----------|----------|-----------|
| Function Coloring | **No explicit async/await keywords** | Effect inference eliminates viral propagation |
| Task Safety | **Compile-time data race prevention** | Transfer/Sharable traits like Rust's Send/Sync |
| Task Lifecycle | **Structured concurrency by default** | Tasks cannot outlive parent scope |
| Communication | **Channels as primary primitive** | CSP model proven at scale |
| Runtime | **Optional, compile-time selectable** | Zero-cost for embedded, full features for applications |

### 1.2 High-Level Architecture

```
+------------------------------------------------------------------+
|                    Aria Concurrency Model                         |
+------------------------------------------------------------------+
|                                                                    |
|  +------------------+  +------------------+  +------------------+  |
|  |   Effect System  |  |  Task Scheduler  |  |  Channel System  |  |
|  |  (Compile-time)  |  |    (Runtime)     |  |    (Runtime)     |  |
|  +--------+---------+  +--------+---------+  +--------+---------+  |
|           |                     |                     |            |
|           v                     v                     v            |
|  +------------------+  +------------------+  +------------------+  |
|  | Transfer/Sharable|  | Work-Stealing    |  | Bounded/Unbounded|  |
|  | Trait Checking   |  | + Local Queues   |  | MPSC/MPMC/Bcast  |  |
|  +------------------+  +------------------+  +------------------+  |
|                                                                    |
+------------------------------------------------------------------+
|                    Target Platforms                                |
+------------------------------------------------------------------+
|  [Native: Multi-threaded]  [WASM: Single-threaded]  [Embedded]    |
+------------------------------------------------------------------+
```

---

## 2. Syntax Decisions

### 2.1 Task Spawning: `spawn`

**Decision**: Use `spawn` keyword for creating concurrent tasks.

```aria
# Basic spawn - returns Task[T] handle
task = spawn expensive_computation()

# Block spawn with do...end
task = spawn do
  step1()
  step2()
  step3()
end

# Named spawn for debugging/tracing
task = spawn(name: "data-processor") do
  process_data()
end

# Spawn with explicit data transfer
data = create_data()
task = spawn do
  process(move data)  # data explicitly moved into task
end

# Spawn onto specific executor (when using runtime)
task = spawn(on: Executor.io) do
  read_file(path)
end
```

**Semantics**:
- `spawn` creates a new lightweight task
- Returns `Task[T]` handle for awaiting or cancellation
- Captured values are moved by default (ownership transfer)
- Task begins execution immediately (not lazy)

### 2.2 Awaiting Results: `.await` Method

**Decision**: Use method-style `.await` for consistency with Aria's method chaining.

```aria
# Basic await
result = task.await

# Await with timeout
result = task.await(timeout: 5.seconds)

# Await multiple tasks (tuple destructuring)
(a, b, c) = (task1, task2, task3).await_all

# Await first to complete (racing)
result = [task1, task2, task3].await_first

# Await with explicit result handling
match task.await_result
  Ok(value) => process(value)
  Err(error) => handle_error(error)
end
```

**Important**: Due to effect inference, explicit `.await` is often unnecessary in sequential code:

```aria
# User writes this (no explicit await needed):
fn fetch_user_posts(user_id: Int)
  user = fetch_user(user_id)      # Suspends if async
  posts = fetch_posts(user.id)    # Suspends if async
  render(user, posts)
end

# Compiler infers {Async, IO} effect and generates state machine
```

### 2.3 Structured Concurrency: `with Async.scope`

**Decision**: Use scope blocks for structured concurrency with automatic cleanup.

```aria
# Basic scope - waits for all spawned tasks
fn fetch_user_data(user_id: Int) -> UserData
  with Async.scope |scope|
    profile = scope.spawn fetch_profile(user_id)
    posts = scope.spawn fetch_posts(user_id)
    friends = scope.spawn fetch_friends(user_id)

    UserData(
      profile: profile.await,
      posts: posts.await,
      friends: friends.await
    )
  end
  # Scope automatically waits for ALL children before exiting
  # If any child fails, siblings are cancelled
end
```

### 2.4 Supervisor Scope (Erlang-inspired)

**Decision**: Provide supervisor scope for fault-tolerant task groups.

```aria
# Supervisor scope - children fail independently
fn resilient_fetch(urls: Array[String]) -> Array[Result[Response, Error]]
  with Async.supervisor |scope|
    tasks = urls.map |url|
      scope.spawn fetch(url)
    end

    # Each task completes independently
    # Failures don't cancel siblings
    tasks.map |t| t.await_result end
  end
end

# With restart strategies
with Async.supervisor(strategy: :one_for_one, max_restarts: 3) |scope|
  scope.spawn(restart: :permanent) worker1()
  scope.spawn(restart: :transient) worker2()  # Only restart on abnormal exit
  scope.spawn(restart: :temporary) worker3()  # Never restart
end
```

### 2.5 Channels

**Decision**: Type-safe channels with Go-inspired syntax.

```aria
# Create typed channel
ch = Channel[Int].new()             # Unbuffered (rendezvous)
ch = Channel[String].new(cap: 10)   # Buffered

# Send and receive
ch.send(42)           # Blocks if full/no receiver
value = ch.recv()     # Blocks if empty
value = <-ch          # Shorthand for recv

# Non-blocking variants
ch.try_send(42)       # Returns Result[Unit, SendError]
ch.try_recv()         # Returns Option[T]

# Close channel
ch.close()

# Iterate over channel (until closed)
for value in ch
  process(value)
end
```

### 2.6 Select Statement

**Decision**: Go-inspired select with Aria syntax.

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
    msg = <-inbox =>
      if msg.quit?
        break
      end
      handle(msg)
    <-shutdown => break
  end
end
```

### 2.7 Parallel Iterators

**Decision**: Rayon-inspired parallel iterators via `.par` method.

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

## 3. Colored Functions Decision

### 3.1 Decision: Effect-Inferred Async (No Function Coloring)

**Aria does NOT use explicit `async`/`await` keywords for function definitions.**

Instead, the effect system tracks async operations:

```aria
# User writes synchronous-looking code:
fn main()
  user = fetch_user(123)       # fetch_user has {Async, IO} effect
  posts = fetch_posts(user.id) # fetch_posts has {Async, IO} effect
  render(user, posts)          # render is pure
end

# Compiler infers: fn main() -> {Async, IO}
# and generates appropriate state machine code
```

### 3.2 Rationale

| Problem with Colored Functions | Aria's Solution |
|-------------------------------|-----------------|
| Viral propagation (call async = become async) | Effect inference propagates automatically |
| Library ecosystem split (sync vs async) | Single library works for both |
| Forced refactoring when adding async call | Effect change is automatic |
| Mental overhead of two function types | One mental model |

### 3.3 Effect Declaration (When Needed)

For documentation or API contracts, effects can be explicitly declared:

```aria
# Explicit effect annotation (optional but good for documentation)
fn fetch_data(url: String) -> {Async, IO} Response
  http.get(url)
end

# Pure function (no effects)
fn calculate(x: Int) -> Int  # Implicitly pure
  x * 2 + 1
end

# Effect polymorphic function
fn transform[E](items: Array[Item], f: (Item) -> {E} Result) -> {E} Array[Result]
  items.map(f)
end
```

### 3.4 Bridging Sync and Async Contexts

When explicit control is needed:

```aria
# Block on async from sync context (entry point only)
fn sync_main()
  with Async.block_on
    async_work()
  end
end

# Spawn blocking work from async context
fn async_context() -> {Async}
  # Run blocking FFI call without blocking the async runtime
  result = spawn_blocking do
    blocking_ffi_call()
  end.await
end
```

---

## 4. Thread Safety: Transfer and Sharable Traits

### 4.1 Decision: Rust-Inspired Compile-Time Safety

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
```

### 4.2 Automatic Derivation

```aria
# Transfer derived automatically for value types
struct Point(x: Float, y: Float)  # Transfer derived

# Non-Transfer types (contain non-Transfer fields)
struct Connection
  socket: RawSocket  # RawSocket is not Transfer
end

# Explicit opt-in for types with safe interior mutability
@unsafe_transfer  # Developer asserts thread safety
struct AtomicCounter
  value: AtomicInt
end

# @shared types are always Transfer (reference counted)
@shared class SharedState
  data: String
end
```

### 4.3 Spawn Constraints

```aria
# spawn requires captured values to be Transfer
fn spawn_task[T: Transfer](f: () -> {Async} T) -> Task[T]

# Compiler error for non-Transfer captures
fn bad_example()
  conn = establish_connection()  # Connection is not Transfer

  spawn do
    conn.send(data)  # ERROR: cannot capture non-Transfer value
  end
end
```

### 4.4 Error Messages

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

## 5. Runtime Requirements

### 5.1 Decision: Optional Runtime (Compile-Time Selectable)

Aria supports three runtime modes:

| Mode | Use Case | Memory | Features |
|------|----------|--------|----------|
| **No Runtime** | Embedded, single-threaded | Minimal | Blocking I/O only |
| **Single-Threaded** | Scripts, WASM | Low | Cooperative async |
| **Multi-Threaded** | Applications, servers | Standard | Full concurrency |

### 5.2 Runtime Selection

```aria
# Default: No runtime (blocking)
fn main()
  data = read_file("config.txt")  # Blocks
end

# Single-threaded async runtime
@[runtime: "aria.async.single"]
fn main()
  data = read_file("config.txt")  # Async, single-threaded
end

# Multi-threaded runtime (default for servers)
@[runtime: "aria.async.multi"]
fn main()
  spawn worker1()
  spawn worker2()
end

# Custom runtime configuration
with Async.runtime(workers: CPU.count, io_threads: 4)
  server_main()
end
```

### 5.3 Runtime Architecture (Multi-Threaded)

```
Aria Async Runtime
+-- Scheduler (work-stealing)
|   +-- Global queue (overflow/injection)
|   +-- Worker threads (ARIA_WORKERS or CPU count)
|   |   +-- Local run queue (lock-free, 256 slots)
|   +-- Work stealing (random victim, steal half)
+-- I/O Driver
|   +-- Linux: io_uring (preferred) / epoll
|   +-- macOS: kqueue
|   +-- Windows: IOCP
+-- Timer Driver
|   +-- Hierarchical timer wheel
+-- Blocking Pool
    +-- Thread pool for blocking FFI calls
```

### 5.4 WASM Target

```aria
# WASM uses single-threaded event loop integration
@target(:wasm)
with Async.wasm_runtime
  # Uses JavaScript event loop
  # spawn creates Promise-backed tasks
  browser_app()
end
```

---

## 6. Structured Concurrency Primitives

### 6.1 Core Scope Types

| Scope | Behavior | Use Case |
|-------|----------|----------|
| `Async.scope` | Wait for all, cancel all on error | Default concurrent work |
| `Async.supervisor` | Children fail independently | Fault-tolerant services |
| `Async.nursery` | Trio-style, explicit start_soon | Dynamic task spawning |
| `Async.timeout` | Cancel all after duration | Time-bounded operations |

### 6.2 Cancellation Model

**Decision**: Cooperative cancellation with effect integration.

```aria
# Cancellation as an effect
effect Cancel
  fn check() -> Unit    # Throws if cancelled
  fn is_cancelled() -> Bool
end

# Long-running work with cancellation support
fn long_operation() -> {Async, Cancel} Result
  for item in items
    Cancel.check()  # Cancellation checkpoint
    process(item)
  end
end

# Timeout wrapper (auto-cancels on timeout)
fn with_timeout() -> Result[Data, TimeoutError]
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

### 6.3 Error Propagation

```aria
# Default: First error cancels siblings and propagates
with Async.scope |scope|
  a = scope.spawn may_fail_a()
  b = scope.spawn may_fail_b()  # Cancelled if a fails

  # Exception from a or b propagates here
end

# Supervisor: Errors isolated
with Async.supervisor |scope|
  a = scope.spawn may_fail_a()  # Failure doesn't affect b
  b = scope.spawn may_fail_b()  # Runs regardless of a

  (a.await_result, b.await_result)  # Collect results individually
end
```

---

## 7. Complete Syntax Examples

### 7.1 Web Server Example

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

fn handle_connection(stream: TcpStream)
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

fn fetch_users() -> Array[User]
  # Parallel fetch from multiple sources
  with Async.scope |scope|
    db_users = scope.spawn Database.get_users()
    cache_users = scope.spawn Cache.get_users()

    # Merge results
    (db_users.await ++ cache_users.await).unique
  end
end
```

### 7.2 Producer-Consumer Pattern

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

### 7.3 Structured Concurrency with Error Handling

```aria
fn fetch_user_dashboard(user_id: Int) -> Result[Dashboard, DashboardError]
  with Async.scope |scope|
    # Start all fetches in parallel
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

### 7.4 Select with Multiple Channels

```aria
fn multiplexed_handler(inbox: Channel[Message],
                       commands: Channel[Command],
                       shutdown: Channel[Unit])
  loop
    select
      msg = <-inbox =>
        handle_message(msg)

      cmd = <-commands =>
        match cmd
          Command.Status => report_status()
          Command.Pause => pause_processing()
          Command.Resume => resume_processing()
        end

      <-shutdown =>
        cleanup()
        break

      timeout(30.seconds) =>
        heartbeat()
    end
  end
end
```

---

## 8. Performance Targets

Based on BOLT's benchmark research:

| Metric | Target | Comparison |
|--------|--------|------------|
| Skynet 1M benchmark | < 150 ms | Go: 200-224 ms |
| Context switch | < 300 ns | Goroutines: ~170-200 ns |
| Channel send (bounded) | < 100 ns p50 | Rust mpsc: ~70 ns |
| 1M concurrent tasks | < 1 GB memory | Tokio: ~800 MB |
| Task spawn overhead | < 1 us | Go: ~2 us |

---

## 9. Implementation Roadmap

### Phase 1: Core Primitives (8 weeks)
- [ ] Transfer/Sharable trait system
- [ ] Basic spawn/await
- [ ] Single-threaded runtime
- [ ] Effect inference for Async

### Phase 2: Channels and Select (6 weeks)
- [ ] Channel[T] implementation
- [ ] Select statement
- [ ] Buffered channels
- [ ] Channel iteration

### Phase 3: Structured Concurrency (6 weeks)
- [ ] Async.scope
- [ ] Cancellation propagation
- [ ] Supervisor scope
- [ ] Error propagation

### Phase 4: Runtime Optimization (8 weeks)
- [ ] Work-stealing scheduler
- [ ] I/O driver integration (io_uring/epoll/kqueue)
- [ ] Timer wheel
- [ ] Performance benchmarks

### Phase 5: WASM Support (4 weeks)
- [ ] Single-threaded WASM runtime
- [ ] Promise interop
- [ ] Event loop integration

---

## 10. Open Questions (Deferred to v2.0)

1. **First-class Actors**: Should Aria have built-in actor syntax?
   - Defer: Channels cover most use cases

2. **Distributed Computing**: Should channels work across nodes?
   - Defer: Focus on single-node first

3. **Hot Code Reloading**: Erlang-style updates?
   - Defer: Significant complexity

4. **Debug Tooling**: Async stack traces?
   - Plan: Task tracing in v1.1

---

## 11. Decision Summary

| Aspect | Decision |
|--------|----------|
| **Async Keyword** | None (effect-inferred) |
| **Await Syntax** | `.await` method style |
| **Spawn Keyword** | `spawn` with block support |
| **Function Coloring** | No (effect system handles) |
| **Thread Safety** | Transfer/Sharable traits |
| **Structured Concurrency** | Scope-based, default |
| **Channels** | First-class, typed, bounded default |
| **Select** | Go-inspired with timeout/default |
| **Runtime** | Optional, compile-time selectable |
| **Cancellation** | Cooperative via Cancel effect |

---

**Document Status**: APPROVED
**Effective Date**: 2026-01-15
**Next Review**: After Phase 2 completion

---

*This Product Design Document represents final decisions for Aria's concurrency model. Implementation should follow these specifications. Changes require PDD amendment process.*
