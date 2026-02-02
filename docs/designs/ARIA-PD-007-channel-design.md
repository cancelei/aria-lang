# ARIA-PD-007: Channel-Based Communication Design

**Document ID**: ARIA-PD-007
**Type**: Product Design Document (PDD)
**Status**: APPROVED
**Date**: 2026-01-15
**Decision Agent**: VORTEX
**Based On**: ARIA-M11-06-channel-patterns.md (ECHO Research)
**Depends On**: ARIA-PD-002-ownership-decisions.md

---

## 1. Executive Summary

This document defines Aria's channel-based communication system for concurrent and parallel programming. The design integrates ECHO's research recommendations with Aria's ownership model to provide:

- **Type-safe channels** with compile-time verification
- **Capability-annotated transfers** ensuring data-race freedom
- **Flexible buffering strategies** with bounded defaults
- **Select/choice syntax** for multiplexing
- **Zero-cost ownership transfer** through move semantics

**Core Design Decision**: Aria channels use **move semantics by default** for ownership transfer, with `@shared` types enabling multi-consumer patterns. Channel types are parameterized by both element type and direction capabilities.

---

## 2. Channel Type System

### 2.1 Core Channel Types

```ebnf
channel_type      = 'Chan' '<' type_expr [ ',' buffer_spec ] '>'
                  | 'SendChan' '<' type_expr '>'
                  | 'RecvChan' '<' type_expr '>'
                  | 'BroadcastChan' '<' type_expr '>' ;

buffer_spec       = 'Unbuffered'                    (* rendezvous, capacity=0 *)
                  | 'Bounded' '<' integer_lit '>'   (* fixed capacity *)
                  | 'Unbounded' ;                   (* unlimited - discouraged *)
```

### 2.2 Type Definitions

| Type | Description | Cloneable | Send | Receive |
|------|-------------|-----------|------|---------|
| `Chan[T]` | Bidirectional channel handle | No | Yes | Yes |
| `SendChan[T]` | Send-only channel endpoint | Yes (MPSC/MPMC) | Yes | No |
| `RecvChan[T]` | Receive-only channel endpoint | Yes (SPMC/MPMC) | No | Yes |
| `BroadcastChan[T]` | One-to-many broadcast | Publisher: No, Subscriber: Yes | Publisher only | Subscribers only |

### 2.3 Channel Creation Functions

```aria
module std::channel

## Channel creation with explicit buffer strategy

# Bounded channel (RECOMMENDED DEFAULT)
fn bounded[T](capacity: USize) -> (SendChan[T], RecvChan[T])
  requires capacity > 0

# Rendezvous channel (zero capacity, synchronous handoff)
fn rendezvous[T]() -> (SendChan[T], RecvChan[T])

# Unbounded channel (USE WITH CAUTION - memory risk)
@deprecated("Prefer bounded channels for production code")
fn unbounded[T]() -> (SendChan[T], RecvChan[T])

# Broadcast channel (one publisher, many subscribers)
fn broadcast[T](capacity: USize) -> (BroadcastSender[T], fn() -> BroadcastReceiver[T])

# MPSC channel (default pattern - multiple senders, single receiver)
fn mpsc[T](capacity: USize = 32) -> (SendChan[T], RecvChan[T])

# SPSC channel (optimized single-producer single-consumer)
fn spsc[T](capacity: USize = 32) -> (SendChan[T], RecvChan[T])

# MPMC channel (multiple producers, multiple consumers)
fn mpmc[T](capacity: USize = 32) -> (SendChan[T], RecvChan[T])
```

### 2.4 Full Type Signatures

```aria
## SendChan - Send-only channel endpoint
struct SendChan[T]
  # Internal implementation details hidden

  derive(Clone)  # Cloneable for MPSC/MPMC patterns
end

impl SendChan[T]
  ## Send a value, transferring ownership
  ## Blocks until space available or channel closed
  fn send(self, value: T) -> Result[Unit, SendError[T]]
    requires T: Send  # Type must be safe to send across threads

  ## Try to send without blocking
  fn try_send(self, value: T) -> Result[Unit, TrySendError[T]]

  ## Send with timeout
  fn send_timeout(self, value: T, timeout: Duration) -> Result[Unit, SendTimeoutError[T]]

  ## Close the sending side
  fn close(self)

  ## Check if channel is closed
  fn closed?(self) -> Bool

  ## Check available capacity
  fn capacity(self) -> USize
end

## RecvChan - Receive-only channel endpoint
struct RecvChan[T]
  derive(Clone)  # Cloneable for SPMC/MPMC patterns
end

impl RecvChan[T]
  ## Receive a value, taking ownership
  ## Blocks until value available or channel closed
  fn recv(self) -> Result[T, RecvError]

  ## Try to receive without blocking
  fn try_recv(self) -> Result[T, TryRecvError]

  ## Receive with timeout
  fn recv_timeout(self, timeout: Duration) -> Result[T, RecvTimeoutError]

  ## Iterate over received values (consumes until closed)
  fn iter(self) -> ChannelIterator[T]
end

## Make RecvChan iterable
impl RecvChan[T]: IntoIterator[T]
  fn into_iter(self) -> ChannelIterator[T]
    self.iter()
  end
end
```

---

## 3. Ownership Transfer Semantics

### 3.1 Core Principle: Move by Default

**Decision**: Channel sends **consume ownership** of the sent value. This matches Aria's Tier 1 ownership inference and provides zero-cost transfers.

```aria
# Value is MOVED into channel - original binding invalidated
let data = create_large_data()
tx.send(data)
# data is no longer valid here - compile error if used
```

### 3.2 Sendable Type Constraint

Only types marked with the `Send` capability can be transferred through channels:

```aria
trait Send
  # Marker trait - no methods required
  # Indicates type is safe to transfer between threads
end

# Primitive types are automatically Send
impl Send for Int, UInt, Float, Bool, Char, String, Bytes

# Arrays/collections of Send types are Send
impl[T: Send] Send for Array[T]
impl[K: Send, V: Send] Send for Map[K, V]

# @shared types are Send (atomic reference counting)
impl[T] Send for @shared T
```

### 3.3 Ownership Transfer Rules

| Scenario | Behavior | Rationale |
|----------|----------|-----------|
| `tx.send(value)` | Value moved to channel | Zero-cost, single ownership |
| `tx.send(copy value)` | Deep copy sent | Sender retains original |
| `tx.send(borrow value)` | **COMPILE ERROR** | Cannot send borrowed reference |
| `rx.recv()` | Receiver takes ownership | Clean ownership transfer |

### 3.4 Integration with @shared Types

For scenarios requiring shared access across multiple consumers:

```aria
# @shared types enable multi-reference patterns
@shared class SharedState
  data: Array[Int]

  fn update(value: Int)
    data.push(value)
  end
end

fn multi_consumer_pattern()
  let (tx, rx) = mpmc[SharedState](100)
  let state = SharedState.new([])

  # Clone @shared reference for multiple sends
  for _ in 0..10
    tx.send(state.clone())  # Reference count increment, not deep copy
  end
end
```

### 3.5 Consume Syntax for Explicit Transfer

For clarity in complex scenarios, use explicit `consume` keyword:

```aria
fn transfer_ownership(tx: SendChan[Resource], resource: Resource)
  # Explicit consume makes ownership transfer visible
  tx.send(consume resource)
  # resource is now invalid
end
```

---

## 4. Buffer Strategy Specification

### 4.1 Strategy Comparison

| Strategy | Capacity | Blocking Behavior | Use Case |
|----------|----------|-------------------|----------|
| **Rendezvous** | 0 | Both block until handoff | Synchronization points |
| **Bounded** | N | Sender blocks when full | Production systems (DEFAULT) |
| **Unbounded** | Unlimited | Never blocks sender | Testing only, memory risk |

### 4.2 Buffer Strategy Decision Matrix

```
Question: What buffer strategy should I use?
│
├── Need synchronization point?
│   └── YES → rendezvous[T]()
│
├── Production code?
│   └── YES → bounded[T](capacity)
│       ├── Low latency needed? → capacity: 1-10
│       ├── High throughput? → capacity: 100-1000
│       └── Streaming batches? → capacity: batch_size
│
└── Development/testing?
    └── unbounded[T]() (with caution)
```

### 4.3 Default Buffer Sizes

```aria
module std::channel::defaults

const DEFAULT_MPSC_CAPACITY: USize = 32
const DEFAULT_SPSC_CAPACITY: USize = 64     # Higher for optimized path
const DEFAULT_MPMC_CAPACITY: USize = 32
const DEFAULT_BROADCAST_CAPACITY: USize = 16
```

### 4.4 Backpressure Handling

```aria
fn producer(tx: SendChan[Data])
  for item in data_stream
    match tx.try_send(item)
      Ok(()) => continue
      Err(TrySendError::Full(item)) =>
        # Backpressure: channel full
        # Option 1: Block
        tx.send(item)?
        # Option 2: Drop
        log.warn("Dropping item due to backpressure")
        # Option 3: Buffer locally
        local_buffer.push(item)
      Err(TrySendError::Disconnected(item)) =>
        # Channel closed, stop producing
        break
    end
  end
end
```

---

## 5. Select/Choice Syntax

### 5.1 Select Expression Syntax

```ebnf
select_expr       = 'select' newline
                    { select_arm }
                    [ default_arm ]
                    'end' ;

select_arm        = recv_arm | send_arm | timeout_arm ;

recv_arm          = pattern '<-' expression [ 'if' expression ] '=>'
                    ( expression | block ) ;

send_arm          = expression '<-' expression [ 'if' expression ] '=>'
                    ( expression | block ) ;

timeout_arm       = 'after' expression '=>' ( expression | block ) ;

default_arm       = 'default' '=>' ( expression | block ) ;
```

### 5.2 Select Semantics

| Behavior | Description |
|----------|-------------|
| **Blocking** | Without `default`, blocks until one arm ready |
| **Non-blocking** | With `default`, immediately falls through if none ready |
| **Fair selection** | Multiple ready arms selected randomly |
| **Timeout** | `after` arm triggers after duration if nothing else ready |
| **Guards** | `if` clause enables conditional arm participation |

### 5.3 Select Examples

```aria
## Basic select - blocking until one ready
fn multiplexer(ch1: RecvChan[Int], ch2: RecvChan[String]) -> Message
  select
    n <- ch1 => Message::Number(n)
    s <- ch2 => Message::Text(s)
  end
end

## Non-blocking select with default
fn try_receive[T](rx: RecvChan[T]) -> Option[T]
  select
    value <- rx => Some(value)
    default => None
  end
end

## Select with timeout
fn receive_with_timeout[T](rx: RecvChan[T], timeout: Duration) -> Result[T, TimeoutError]
  select
    value <- rx => Ok(value)
    after timeout => Err(TimeoutError)
  end
end

## Select with send operations
fn router(in_ch: RecvChan[Task], out1: SendChan[Task], out2: SendChan[Task])
  for task in in_ch
    select
      out1 <- task if task.priority == :high => ()
      out2 <- task => ()
    end
  end
end

## Complex select with guards
fn load_balancer(
  work: RecvChan[Job],
  workers: Array[SendChan[Job]],
  results: RecvChan[Result]
)
  let mut pending = 0
  let max_pending = 100

  loop
    select
      # Accept new work if under capacity
      job <- work if pending < max_pending =>
        workers[job.hash % workers.len].send(job)?
        pending += 1

      # Collect results
      result <- results =>
        pending -= 1
        process_result(result)

      # Periodic health check
      after 5.seconds =>
        log.info("Pending: #{pending}")
    end
  end
end
```

### 5.4 Select on Multiple Channels of Same Type

```aria
## Select across array of channels
fn fan_in[T](channels: Array[RecvChan[T]]) -> RecvChan[T]
  let (tx, rx) = mpsc[T](channels.len * 10)

  spawn {
    loop
      # Dynamic select across all channels
      select_any channels { |ch|
        value <- ch => tx.send(value)?
      }
    end
  }

  rx
end

## Alternative: selectv! macro for dynamic channel sets
fn aggregate[T](channels: Array[RecvChan[T]], output: SendChan[T])
  loop
    selectv! channels, output { |rx, tx|
      value <- rx => tx.send(value)?
      default => break
    }
  end
end
```

---

## 6. Channel Cardinality Types

### 6.1 SPSC (Single Producer, Single Consumer)

```aria
## Optimized SPSC channel - highest throughput
fn spsc_example()
  let (tx, rx) = spsc[Int](1000)

  # tx and rx are NOT cloneable - enforces single producer/consumer
  spawn { producer(tx) }
  spawn { consumer(rx) }
end

# Compile error: SPSC channels are not cloneable
fn spsc_error()
  let (tx, rx) = spsc[Int](100)
  let tx2 = tx.clone()  # ERROR: SendChan[Int, SPSC] does not implement Clone
end
```

### 6.2 MPSC (Multiple Producer, Single Consumer)

```aria
## MPSC - most common pattern
fn mpsc_example()
  let (tx, rx) = mpsc[Log](1000)

  # Sender is cloneable
  for i in 0..10
    let tx_clone = tx.clone()
    spawn { worker(i, tx_clone) }
  end

  # Single consumer
  spawn { logger(rx) }
end
```

### 6.3 MPMC (Multiple Producer, Multiple Consumer)

```aria
## MPMC - work stealing pattern
fn work_pool()
  let (tx, rx) = mpmc[Task](100)

  # Multiple producers
  for source in task_sources
    let tx_clone = tx.clone()
    spawn { source.produce(tx_clone) }
  end

  # Multiple consumers (workers)
  for _ in 0..num_workers
    let rx_clone = rx.clone()
    spawn { worker(rx_clone) }
  end
end
```

### 6.4 Broadcast (One-to-Many)

```aria
## Broadcast channel - all receivers get all messages
fn event_broadcast()
  let (publisher, subscribe) = broadcast[Event](100)

  # Create subscribers
  let sub1 = subscribe()
  let sub2 = subscribe()
  let sub3 = subscribe()

  # Subscribers receive independently
  spawn { ui_handler(sub1) }
  spawn { logger(sub2) }
  spawn { analytics(sub3) }

  # Publisher sends to all
  for event in event_stream
    publisher.send(event)?
  end
end
```

---

## 7. Error Handling

### 7.1 Error Types

```aria
enum SendError[T]
  Disconnected(T)     # Channel closed, returns unsent value
end

enum TrySendError[T]
  Full(T)             # Channel full, returns unsent value
  Disconnected(T)     # Channel closed, returns unsent value
end

enum SendTimeoutError[T]
  Timeout(T)          # Timeout elapsed, returns unsent value
  Disconnected(T)     # Channel closed, returns unsent value
end

enum RecvError
  Disconnected        # Channel closed, no more values
end

enum TryRecvError
  Empty               # No value available (non-blocking)
  Disconnected        # Channel closed
end

enum RecvTimeoutError
  Timeout             # Timeout elapsed
  Disconnected        # Channel closed
end
```

### 7.2 Error Handling Patterns

```aria
## Pattern 1: Propagate errors
fn reliable_send[T](tx: SendChan[T], value: T) -> Result[Unit, SendError[T]]
  tx.send(value)?
  Ok(())
end

## Pattern 2: Retry on full
fn send_with_retry[T](tx: SendChan[T], value: T, max_retries: Int) -> Bool
  let mut attempts = 0
  let mut current = value

  while attempts < max_retries
    match tx.try_send(current)
      Ok(()) => return true
      Err(TrySendError::Full(v)) =>
        current = v
        attempts += 1
        sleep(10.ms * attempts)  # Exponential backoff
      Err(TrySendError::Disconnected(_)) =>
        return false
    end
  end

  false
end

## Pattern 3: Graceful shutdown
fn graceful_consumer[T](rx: RecvChan[T], process: fn(T))
  for value in rx
    process(value)
  end
  # Loop exits when channel closed and empty
  log.info("Consumer shutdown complete")
end
```

---

## 8. Integration with Ownership Model

### 8.1 Tier 1: Inferred Channel Usage (80%)

Most channel code requires no ownership annotations:

```aria
## Simple producer-consumer - fully inferred
fn simple_pipeline()
  let (tx, rx) = mpsc[String](100)

  spawn {
    for line in File.lines("input.txt")
      tx.send(line)  # Ownership transferred - inferred
    end
    tx.close()
  }

  for line in rx
    print(line)  # Ownership received - inferred
  end
end

## Channel in struct - owned field, no annotation needed
struct Worker
  inbox: RecvChan[Task]
  outbox: SendChan[Result]
end

impl Worker
  fn run(self)
    for task in self.inbox
      let result = process(task)
      self.outbox.send(result)
    end
  end
end
```

### 8.2 Tier 2: Explicit Annotations (15%)

Annotations needed for complex lifetime scenarios:

```aria
## Channel reference in function - needs lifetime
fn process_with_channel[life L](
  rx: ref[L] RecvChan[Data],
  processor: fn(Data) -> Result
) -> Array[Result]
  rx.iter().map(processor).collect()
end

## Struct holding channel reference
struct ChannelView[life L, T]
  channel: ref[L] RecvChan[T]
  filter: fn(ref T) -> Bool
end

impl ChannelView[life L, T]
  fn filtered_iter(self) -> impl Iterator[T]
    self.channel.iter().filter(self.filter)
  end
end
```

### 8.3 Tier 3: @shared Channels (5%)

For complex multi-owner patterns:

```aria
## @shared for multiple owners of same logical channel
@shared class ChannelHub[T]
  senders: Array[SendChan[T]]
  receivers: Array[RecvChan[T]]

  fn add_worker() -> (SendChan[T], RecvChan[T])
    let (tx, rx) = mpmc[T](100)
    senders.push(tx.clone())
    receivers.push(rx.clone())
    (tx, rx)
  end
end
```

---

## 9. Deadlock Prevention

### 9.1 Compile-Time Strategies

**Priority Annotations** (Optional, for critical sections):

```aria
## Annotate channel priorities to prevent cyclic waits
fn safe_bidirectional()
  # Higher priority must be acquired first
  let (tx1, rx1): Chan[Int, priority=1] = bounded(10)
  let (tx2, rx2): Chan[Int, priority=2] = bounded(10)

  # Compiler ensures priority=1 operations complete before priority=2
end
```

### 9.2 Runtime Timeout Defaults

```aria
## All blocking operations support timeout
fn deadlock_safe_select()
  select
    value <- rx1 => process1(value)
    value <- rx2 => process2(value)
    after 30.seconds =>
      log.warn("Potential deadlock detected")
      diagnose_channels([rx1, rx2])
  end
end
```

### 9.3 Best Practices

| Practice | Description |
|----------|-------------|
| **Timeout all selects** | Use `after` clause for production code |
| **Close channels explicitly** | Prevents hanging receivers |
| **Use bounded channels** | Backpressure prevents memory exhaustion |
| **Single direction flow** | Prefer pipelines over bidirectional |
| **Select with default** | Non-blocking for responsive systems |

---

## 10. Complete API Reference

### 10.1 Module Structure

```aria
module std::channel
  # Creation functions
  pub fn bounded[T](capacity: USize) -> (SendChan[T], RecvChan[T])
  pub fn rendezvous[T]() -> (SendChan[T], RecvChan[T])
  pub fn unbounded[T]() -> (SendChan[T], RecvChan[T])
  pub fn broadcast[T](capacity: USize) -> (BroadcastSender[T], fn() -> BroadcastReceiver[T])
  pub fn mpsc[T](capacity: USize = 32) -> (SendChan[T], RecvChan[T])
  pub fn spsc[T](capacity: USize = 64) -> (SendChan[T], RecvChan[T])
  pub fn mpmc[T](capacity: USize = 32) -> (SendChan[T], RecvChan[T])

  # Channel types
  pub struct SendChan[T]
  pub struct RecvChan[T]
  pub struct BroadcastSender[T]
  pub struct BroadcastReceiver[T]

  # Error types
  pub enum SendError[T]
  pub enum TrySendError[T]
  pub enum SendTimeoutError[T]
  pub enum RecvError
  pub enum TryRecvError
  pub enum RecvTimeoutError

  # Traits
  pub trait Send
end
```

### 10.2 SendChan Methods

```aria
impl SendChan[T]
  fn send(self, value: T) -> Result[Unit, SendError[T]]
  fn try_send(self, value: T) -> Result[Unit, TrySendError[T]]
  fn send_timeout(self, value: T, timeout: Duration) -> Result[Unit, SendTimeoutError[T]]
  fn close(self)
  fn closed?(self) -> Bool
  fn capacity(self) -> USize
  fn len(self) -> USize
  fn empty?(self) -> Bool
  fn full?(self) -> Bool
end
```

### 10.3 RecvChan Methods

```aria
impl RecvChan[T]
  fn recv(self) -> Result[T, RecvError]
  fn try_recv(self) -> Result[T, TryRecvError]
  fn recv_timeout(self, timeout: Duration) -> Result[T, RecvTimeoutError]
  fn iter(self) -> ChannelIterator[T]
  fn closed?(self) -> Bool
  fn len(self) -> USize
  fn empty?(self) -> Bool
end

impl RecvChan[T]: IntoIterator[T]
impl RecvChan[T]: Iterator[T]  # For direct iteration
```

---

## 11. Example Programs

### 11.1 Worker Pool Pattern

```aria
module examples::worker_pool

import std::channel::{mpsc, bounded}

data Task(id: Int, payload: String)
data Result(task_id: Int, output: String)

fn worker_pool(num_workers: Int, tasks: Array[Task]) -> Array[Result]
  let (task_tx, task_rx) = mpmc[Task](tasks.len)
  let (result_tx, result_rx) = mpsc[Result](tasks.len)

  # Spawn workers
  let workers = (0..num_workers).map { |_|
    let rx = task_rx.clone()
    let tx = result_tx.clone()
    spawn {
      for task in rx
        let output = process_task(task)
        tx.send(Result(task_id: task.id, output:))?
      end
    }
  }

  # Send all tasks
  for task in tasks
    task_tx.send(task)?
  end
  task_tx.close()

  # Collect results
  let results = []
  for _ in 0..tasks.len
    results.push(result_rx.recv()?)
  end

  results
end

fn process_task(task: Task) -> String
  # Simulate work
  sleep(100.ms)
  "Processed: #{task.payload}"
end
```

### 11.2 Pipeline Pattern

```aria
module examples::pipeline

import std::channel::bounded

fn pipeline_example(input: Array[Int]) -> Array[String]
  # Stage 1: Filter
  let (s1_tx, s1_rx) = bounded[Int](100)
  spawn {
    for n in input
      if n > 0
        s1_tx.send(n)?
      end
    end
    s1_tx.close()
  }

  # Stage 2: Transform
  let (s2_tx, s2_rx) = bounded[Int](100)
  spawn {
    for n in s1_rx
      s2_tx.send(n * 2)?
    end
    s2_tx.close()
  }

  # Stage 3: Format
  let (s3_tx, s3_rx) = bounded[String](100)
  spawn {
    for n in s2_rx
      s3_tx.send("Value: #{n}")?
    end
    s3_tx.close()
  }

  # Collect
  s3_rx.iter().collect()
end
```

### 11.3 Fan-Out/Fan-In Pattern

```aria
module examples::fan

import std::channel::{mpsc, broadcast}

fn fan_out_fan_in[T, R](
  items: Array[T],
  num_workers: Int,
  process: fn(T) -> R
) -> Array[R]
  let (broadcast_tx, subscribe) = broadcast[T](items.len)
  let (result_tx, result_rx) = mpsc[R](items.len * num_workers)

  # Fan out to workers
  for _ in 0..num_workers
    let sub = subscribe()
    let tx = result_tx.clone()
    spawn {
      for item in sub
        tx.send(process(item))?
      end
    }
  end

  # Send items
  for item in items
    broadcast_tx.send(item)?
  end
  broadcast_tx.close()

  # Fan in results
  result_rx.iter().take(items.len * num_workers).collect()
end
```

### 11.4 Select-Based Event Loop

```aria
module examples::event_loop

import std::channel::{mpsc, bounded}
import std::time::ticker

enum Event
  UserInput(String)
  Timer
  NetworkData(Bytes)
  Shutdown
end

fn event_loop()
  let (input_tx, input_rx) = mpsc[String](100)
  let (network_tx, network_rx) = mpsc[Bytes](1000)
  let (shutdown_tx, shutdown_rx) = bounded[Unit](1)
  let timer = ticker(1.second)

  # Spawn input handlers
  spawn { read_user_input(input_tx) }
  spawn { read_network(network_tx) }

  # Main event loop
  loop
    select
      line <- input_rx =>
        handle_input(line)

      data <- network_rx =>
        handle_network(data)

      _ <- timer.recv() =>
        handle_timer()

      _ <- shutdown_rx =>
        log.info("Shutting down...")
        break

      after 30.seconds =>
        log.debug("Heartbeat")
    end
  end
end
```

### 11.5 Request-Response Pattern

```aria
module examples::request_response

import std::channel::{bounded, rendezvous}

data Request(id: Int, payload: String, response_ch: SendChan[Response])
data Response(id: Int, result: String)

fn server(requests: RecvChan[Request])
  for req in requests
    let result = process_request(req.payload)
    req.response_ch.send(Response(id: req.id, result:))?
  end
end

fn client(server_ch: SendChan[Request], payload: String) -> String
  let (tx, rx) = rendezvous[Response]()

  server_ch.send(Request(
    id: generate_id(),
    payload:,
    response_ch: tx
  ))?

  let response = rx.recv()?
  response.result
end
```

---

## 12. Grammar Updates

The following additions to GRAMMAR.md are required:

```ebnf
(* Addition to Section 13: Concurrency *)

channel_type    = 'Chan' '<' type_expr [ ',' buffer_spec ] '>'
                | 'SendChan' '<' type_expr '>'
                | 'RecvChan' '<' type_expr '>'
                | 'BroadcastChan' '<' type_expr '>' ;

buffer_spec     = 'Unbuffered'
                | 'Bounded' '<' integer_lit '>'
                | 'Unbounded' ;

channel_expr    = channel_send | channel_recv ;
channel_send    = expression '.' 'send' '(' expression ')'
                | expression '<-' expression ;
channel_recv    = '<-' expression
                | expression '.' 'recv' '(' ')' ;

select_expr     = 'select' newline
                  { select_arm }
                  [ default_arm ]
                  'end' ;

select_arm      = recv_arm | send_arm | timeout_arm ;
recv_arm        = pattern '<-' expression [ 'if' expression ] '=>'
                  ( expression | block ) ;
send_arm        = expression '<-' expression [ 'if' expression ] '=>'
                  ( expression | block ) ;
timeout_arm     = 'after' expression '=>' ( expression | block ) ;
default_arm     = 'default' '=>' ( expression | block ) ;
```

---

## 13. Implementation Roadmap

### Phase 1: Core Channels (4 weeks)
- MPSC bounded channel implementation
- Basic send/recv operations
- Move semantics integration
- RecvChan iterator support

### Phase 2: Select Statement (3 weeks)
- Select expression parsing
- Blocking select implementation
- Non-blocking (default) support
- Timeout arm support

### Phase 3: Channel Variants (3 weeks)
- SPSC optimized implementation
- MPMC implementation
- Broadcast channels
- Rendezvous channels

### Phase 4: Optimization (2 weeks)
- Lock-free SPSC
- Backoff strategies
- Performance benchmarking
- Memory optimization

---

## 14. Decision Summary

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Default buffer strategy** | Bounded | Memory safety, backpressure |
| **Ownership transfer** | Move by default | Zero-cost, matches Aria ownership |
| **Select syntax** | `pattern <- channel =>` | Clean, Go-inspired |
| **Timeout syntax** | `after duration =>` | Clear semantics |
| **Channel creation** | Function-based (`mpsc[T](n)`) | Type inference friendly |
| **Sendable types** | `Send` trait marker | Compile-time safety |
| **Error handling** | Result types with value return | Enables retry patterns |

---

## 15. References

- ARIA-M11-06: Channel-Based Communication Patterns Research (ECHO)
- ARIA-PD-002: Ownership Model Product Decisions
- Go Channels: https://gobyexample.com/channels
- Rust crossbeam-channel: https://docs.rs/crossbeam-channel
- Kotlin Channels: https://kotlinlang.org/docs/channels.html

---

**Document Status**: APPROVED
**Next Steps**: ARIA-M11-07 - Implement core channel types
**Decision Authority**: VORTEX (Product Agent)
**Review**: ECHO (Research), GUARDIAN (Architecture)
