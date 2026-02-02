# ARIA-M16-02: Standard Library Architecture Design

**Task ID**: ARIA-M16-02
**Status**: Completed
**Date**: 2026-01-15
**Agent**: GENESIS (Research - Eureka Iteration 3)
**Focus**: Comprehensive stdlib architecture design for Aria

---

## Executive Summary

This document presents Aria's standard library architecture, synthesizing insights from Rust's three-tier design (core/alloc/std), Kotlin's stdlib/coroutines separation, Swift's Foundation evolution, Go's batteries-included philosophy, and async runtime patterns from Tokio. The recommended design is a **modular, effect-aware, three-tier standard library** that prioritizes ergonomics while enabling embedded and WASM targets.

**Key Recommendations**:
1. Three-tier architecture: `aria.core` (no runtime) -> `aria.alloc` (heap required) -> `aria.std` (OS required)
2. Effect-integrated collections with async-aware iterators
3. Prelude auto-imports for common types and functions
4. Stability annotations for API evolution

---

## 1. Research Synthesis: What We Learned

### 1.1 Rust's Three-Tier Design

Rust's stdlib is structured into three crates with clear dependency boundaries:

| Tier | Name | Requirements | Contents |
|------|------|--------------|----------|
| 0 | `core` | None | Option, Result, iterators, traits, primitives |
| 1 | `alloc` | Global allocator | Vec, String, Box, Rc, Arc |
| 2 | `std` | OS + allocator | File I/O, networking, threads, time |

**Key Insight**: The `#![no_std]` attribute enables embedded development by opting out of `std` while retaining `core` functionality. HashMap/HashSet are in `std` (not `alloc`) because they require OS-provided randomness for hash collision protection.

**Reference**: [Rust Standard Library](https://doc.rust-lang.org/std/), [Effective Rust no_std](https://www.lurklurk.org/effective-rust/no-std.html)

### 1.2 Kotlin's Stdlib/Coroutines Separation

Kotlin separates built-in language support from library functionality:

| Component | Package | Contents |
|-----------|---------|----------|
| Built-in | `kotlin.coroutines` | Continuation, suspend keyword, suspendCoroutine |
| Library | `kotlinx.coroutines` | launch, async, Deferred, Flow, Dispatchers |
| Stdlib | `kotlin.collections` | List, Map, Set, Sequence |

**Key Insight**: The built-in support is minimal (compiler primitives), while the library provides rich functionality. This allows the library to evolve independently of the language.

**Reference**: [Kotlin Stdlib](https://kotlinlang.org/api/core/kotlin-stdlib/), [kotlinx.coroutines](https://kotlinlang.org/api/kotlinx.coroutines/)

### 1.3 Swift's Foundation Evolution

Swift's approach has evolved significantly:

| Layer | Contents | Status |
|-------|----------|--------|
| Swift Stdlib | Int, String, Array, protocols | Pure Swift |
| Foundation | NSString, networking, serialization | Objective-C heritage, being rewritten |
| FoundationEssentials | Minimal subset, no i18n | New in Swift 6 |
| FoundationInternationalization | Dates, localization | Separate package |

**Key Insight**: The rewrite provides an opportunity to separate concerns. `FoundationEssentials` targets binary size-sensitive applications. Cross-platform consistency now prioritized.

**Reference**: [Swift Core Libraries](https://www.swift.org/documentation/core-libraries/), [Future of Foundation](https://www.swift.org/blog/future-of-foundation/)

### 1.4 Go's Batteries-Included Philosophy

Go takes a different approach:

- **~150 packages** covering networking, encoding, testing
- **net/http included**: Most Go programs are servers
- **Single implementation**: No ecosystem fragmentation
- **Stability**: APIs rarely change

**Key Insight**: Go's philosophy reduces dependency hell but makes evolution slower. Post-generics (Go 1.22+), pattern routing in `http.ServeMux` reduced framework dependence.

**Reference**: [Go Ecosystem 2025](https://blog.jetbrains.com/go/2025/11/10/go-language-trends-ecosystem-2025/)

### 1.5 Async Runtime Integration Patterns

From Tokio and the Rust async ecosystem:

| Pattern | Description | Aria Relevance |
|---------|-------------|----------------|
| Stream trait | Async iterator (`poll_next`) | AsyncIterator integration |
| StreamExt | Extension methods (next, map, filter) | Effect-aware collections |
| Runtime abstraction | Executor-agnostic code | Effect handlers |
| One-shot continuations | Efficient for common patterns | Effect handler implementation |

**Key Insight**: The `Stream` trait (future `AsyncIterator` in std) provides the async equivalent of `Iterator`. Tokio moves stream utilities to separate crate due to stabilization timing.

**Reference**: [Tokio Streams](https://tokio.rs/tokio/tutorial/streams), [AsyncIterator RFC](https://rust-lang.github.io/rfcs/2996-async-iterator.html)

---

## 2. Aria Standard Library Architecture

### 2.1 Three-Tier Design

```
+=====================================================+
|                    aria.std                          |
|  Requires: OS, Allocator, Runtime                   |
|  Contents: File I/O, Networking, Threads, Time      |
+=====================================================+
                         |
                         | depends on
                         v
+=====================================================+
|                   aria.alloc                         |
|  Requires: Allocator                                |
|  Contents: Vec, HashMap, String, Box, Rc, Arc      |
+=====================================================+
                         |
                         | depends on
                         v
+=====================================================+
|                   aria.core                          |
|  Requires: Nothing (freestanding)                   |
|  Contents: Primitives, Option, Result, Traits       |
+=====================================================+
```

### 2.2 Tier Definitions

#### Tier 0: aria.core (Freestanding)

```aria
# aria.core - No runtime dependencies

# Primitive types (compiler built-ins)
type Int8, Int16, Int32, Int64, Int     # Signed integers
type UInt8, UInt16, UInt32, UInt64, UInt # Unsigned integers
type Float32, Float64, Float            # Floating point
type Bool                                # Boolean
type Char                                # Unicode scalar value
type Never                               # Bottom type (no values)

# Core algebraic types
type Option[T] = Some(T) | None
type Result[T, E] = Ok(T) | Err(E)

# Fixed-size array (stack allocated)
type Array[T, const N: Int]             # [T; N] in Rust terms

# Slice (borrowed view)
type Slice[T]                           # &[T] in Rust terms

# Fundamental traits
trait Eq { fn eq(self, other: Self) -> Bool }
trait Ord: Eq { fn cmp(self, other: Self) -> Ordering }
trait Hash { fn hash(self, hasher: ref Hasher) }
trait Clone { fn clone(self) -> Self }
trait Copy: Clone { }                   # Bitwise copy
trait Default { fn default() -> Self }
trait Debug { fn debug_fmt(self, fmt: ref Formatter) }
trait Display { fn display_fmt(self, fmt: ref Formatter) }

# Iteration
trait Iterator[Item] {
  fn next(ref self) -> Option[Item]
  # Extension methods provided by default impls
}

trait IntoIterator[Item] {
  type Iter: Iterator[Item]
  fn into_iter(self) -> Iter
}

# Concurrency safety (like Rust's Send/Sync)
trait Transfer { }     # Can be moved between tasks
trait Sharable { }     # Can be shared between tasks (&T: Transfer)

# Operator traits
trait Add[Rhs = Self] { type Output; fn add(self, rhs: Rhs) -> Output }
trait Sub[Rhs = Self] { type Output; fn sub(self, rhs: Rhs) -> Output }
# ... other operators

# Range types
type Range[T] = Range { start: T, end: T }
type RangeInclusive[T] = RangeInclusive { start: T, end: T }
type RangeFull
```

#### Tier 1: aria.alloc (Heap Required)

```aria
# aria.alloc - Requires global allocator

# Dynamic collections
type Vec[T]                             # Growable array
type HashMap[K: Hash + Eq, V]           # Hash map
type HashSet[T: Hash + Eq]              # Hash set
type BTreeMap[K: Ord, V]                # Ordered map
type BTreeSet[T: Ord]                   # Ordered set
type Deque[T]                           # Double-ended queue
type LinkedList[T]                      # Doubly-linked list

# String types
type String                             # UTF-8 growable string
type Str                                # Borrowed string slice (&str)

# Smart pointers
type Box[T]                             # Owned heap allocation
type Rc[T]                              # Reference counted (single-threaded)
type Arc[T]                             # Atomic reference counted (multi-threaded)
type Weak[T]                            # Weak reference for Rc/Arc

# Interior mutability
type RefCell[T]                         # Runtime borrow checking
type Cell[T]                            # Copy-based interior mutability
type Mutex[T]                           # Thread-safe interior mutability
type RwLock[T]                          # Reader-writer lock

# Formatting
type Formatter                          # String formatting state
fn format(template: Str, args: ...) -> String
```

#### Tier 2: aria.std (OS Required)

```aria
# aria.std - Requires operating system

# I/O traits and types
trait Read {
  fn read(ref self, buf: ref [UInt8]) -> {IO} Result[Int, IOError]
}

trait Write {
  fn write(ref self, buf: Slice[UInt8]) -> {IO} Result[Int, IOError]
  fn flush(ref self) -> {IO} Result[(), IOError]
}

trait Seek {
  fn seek(ref self, pos: SeekFrom) -> {IO} Result[Int64, IOError]
}

type BufReader[R: Read]                 # Buffered reader
type BufWriter[W: Write]                # Buffered writer

# File system
module aria.std.fs {
  type File
  fn open(path: Str) -> {IO} Result[File, IOError]
  fn create(path: Str) -> {IO} Result[File, IOError]
  fn read_to_string(path: Str) -> {IO} Result[String, IOError]
  fn write_all(path: Str, contents: Slice[UInt8]) -> {IO} Result[(), IOError]
  fn metadata(path: Str) -> {IO} Result[Metadata, IOError]
}

# Networking
module aria.std.net {
  type TcpListener
  type TcpStream
  type UdpSocket
  type IpAddr = V4(Ipv4Addr) | V6(Ipv6Addr)
  type SocketAddr
}

# Threading
module aria.std.thread {
  fn spawn[T: Transfer](f: () -> T) -> {Async} JoinHandle[T]
  fn current() -> Thread
  fn sleep(duration: Duration) -> {IO}
  fn yield_now() -> {IO}
}

# Synchronization (beyond aria.alloc)
module aria.std.sync {
  type Barrier
  type Condvar
  type Once
}

# Channels
module aria.std.channel {
  fn channel[T: Transfer]() -> (Sender[T], Receiver[T])
  fn bounded[T: Transfer](cap: Int) -> (Sender[T], Receiver[T])
  type Sender[T]
  type Receiver[T]
}

# Time
module aria.std.time {
  type Instant                          # Monotonic clock
  type SystemTime                       # Wall clock
  type Duration                         # Time span
}

# Process
module aria.std.process {
  type Command
  type Child
  fn exit(code: Int) -> Never
  fn abort() -> Never
}

# Environment
module aria.std.env {
  fn args() -> Iterator[String]
  fn var(key: Str) -> Option[String]
  fn set_var(key: Str, value: Str)
  fn current_dir() -> {IO} Result[PathBuf, IOError]
}
```

### 2.3 Target Configuration

```aria
# Embedded target (no_std equivalent)
@target(:embedded)
import aria.core          # Always available
# aria.alloc requires custom allocator
# aria.std not available

# WASM target
@target(:wasm)
import aria.core          # Available
import aria.alloc         # Available (WASM has heap)
import aria.std.wasm {    # WASM-specific I/O
  fetch, console, dom
}
# aria.std.fs, aria.std.thread not available

# Native target (default)
import aria.core
import aria.alloc
import aria.std           # Full functionality
```

---

## 3. Module Organization Design

### 3.1 Complete Module Hierarchy

```
aria
 +-- core
 |   +-- types           # Primitive types
 |   +-- option          # Option[T] type
 |   +-- result          # Result[T, E] type
 |   +-- traits          # Fundamental traits
 |   +-- iter            # Iterator trait and combinators
 |   +-- ops             # Operator traits
 |   +-- cmp             # Comparison traits (Eq, Ord)
 |   +-- convert         # From, Into, TryFrom, TryInto
 |   +-- marker          # Marker traits (Copy, Transfer, Sharable)
 |   +-- num             # Numeric traits and operations
 |   +-- slice           # Slice operations
 |   +-- array           # Fixed-size array operations
 |   +-- ptr             # Raw pointer operations (unsafe)
 |   +-- mem             # Memory operations (size_of, align_of)
 |   +-- fmt             # Formatting traits (Debug, Display)
 |
 +-- alloc
 |   +-- vec             # Vec[T] dynamic array
 |   +-- string          # String type
 |   +-- boxed           # Box[T] smart pointer
 |   +-- rc              # Rc[T], Weak[T]
 |   +-- sync            # Arc[T]
 |   +-- collections
 |   |   +-- hash_map    # HashMap[K, V]
 |   |   +-- hash_set    # HashSet[T]
 |   |   +-- btree_map   # BTreeMap[K, V]
 |   |   +-- btree_set   # BTreeSet[T]
 |   |   +-- deque       # Deque[T]
 |   |   +-- linked_list # LinkedList[T]
 |   |   +-- binary_heap # BinaryHeap[T]
 |   +-- fmt             # Formatting implementation
 |
 +-- std
 |   +-- prelude         # Auto-imported items
 |   +-- io              # I/O traits and utilities
 |   |   +-- read        # Read trait
 |   |   +-- write       # Write trait
 |   |   +-- buf         # Buffered I/O
 |   |   +-- cursor      # In-memory I/O
 |   +-- fs              # File system operations
 |   +-- net             # Networking
 |   |   +-- tcp         # TCP
 |   |   +-- udp         # UDP
 |   |   +-- addr        # Addresses
 |   +-- thread          # Threading
 |   +-- sync            # Synchronization
 |   +-- channel         # Message passing
 |   +-- time            # Time utilities
 |   +-- process         # Process management
 |   +-- env             # Environment
 |   +-- path            # Path manipulation
 |   +-- ffi             # Foreign function interface
 |   +-- os              # OS-specific functionality
 |   |   +-- linux
 |   |   +-- macos
 |   |   +-- windows
 |   |   +-- wasm
 |
 +-- async               # Async runtime (effects-based)
 |   +-- task            # Task spawning
 |   +-- runtime         # Runtime configuration
 |   +-- stream          # AsyncIterator
 |   +-- future          # Future type
 |   +-- select          # Select macro/function
 |
 +-- encoding            # Serialization
 |   +-- json            # JSON
 |   +-- base64          # Base64
 |   +-- utf8            # UTF-8 utilities
 |
 +-- text                # Text processing
 |   +-- regex           # Regular expressions
 |   +-- template        # String templates
 |
 +-- math                # Mathematics
 |   +-- rand            # Random number generation
 |   +-- num             # Extended numeric operations
 |
 +-- test                # Testing utilities
     +-- assert          # Assertions
     +-- property        # Property-based testing
     +-- mock            # Mocking utilities
```

### 3.2 Prelude Contents

The prelude is automatically imported into every Aria file:

```aria
# aria.std.prelude - Auto-imported

# Re-exports from aria.core
pub use aria.core.types { Int, Float, Bool, Char, String, Never }
pub use aria.core.option { Option, Some, None }
pub use aria.core.result { Result, Ok, Err }
pub use aria.core.traits {
  Eq, Ord, Hash, Clone, Copy, Default, Debug, Display,
  Transfer, Sharable
}
pub use aria.core.iter { Iterator, IntoIterator }
pub use aria.core.convert { From, Into }

# Re-exports from aria.alloc
pub use aria.alloc.vec { Vec }
pub use aria.alloc.string { String }
pub use aria.alloc.collections { HashMap, HashSet }
pub use aria.alloc.boxed { Box }
pub use aria.alloc.rc { Rc, Arc }

# Common functions
pub fn print(msg: impl Display) -> {IO}
pub fn println(msg: impl Display) -> {IO}
pub fn dbg[T: Debug](value: T) -> T  # Debug print and return

# Common macros/functions
pub fn todo(msg: Str = "not yet implemented") -> Never
pub fn unreachable(msg: Str = "entered unreachable code") -> Never
pub fn panic(msg: Str) -> Never

# Async primitives (effect-based)
pub fn spawn[T: Transfer](f: () -> {Async} T) -> Task[T]
```

### 3.3 Explicit Prelude Override

```aria
# Disable default prelude
#![no_prelude]

# Or selective prelude
#![prelude(minimal)]  # Only core types
#![prelude(alloc)]    # Core + alloc types
#![prelude(full)]     # Full prelude (default)

# Custom prelude extension
module my_app.prelude
  pub use aria.std.prelude { * }
  pub use my_lib.common { MyType, my_function }
end
```

---

## 4. Core vs Std Separation: Decision Matrix

### 4.1 Where Types Belong

| Type/Functionality | Tier | Rationale |
|--------------------|------|-----------|
| Int, Float, Bool | core | Compiler primitives |
| Option, Result | core | No allocation needed |
| Iterator trait | core | Works on slices/arrays |
| Vec, String | alloc | Requires heap allocation |
| HashMap, HashSet | alloc (not std!) | Can use deterministic hash in alloc |
| BTreeMap, BTreeSet | alloc | Heap-only, no randomness needed |
| File, TcpStream | std | Requires OS |
| Thread, Mutex | std | Requires OS |
| Duration | core | Just data, no syscalls |
| Instant, SystemTime | std | Requires OS clock |
| Channel | std | Requires runtime |

### 4.2 HashMap in alloc vs std

**Decision**: Place HashMap in `aria.alloc` with configurable hasher.

```aria
# In aria.alloc.collections.hash_map

# Default hasher (deterministic, DoS-vulnerable)
type DefaultHasher = SipHash13  # Deterministic

# Secure hasher (requires OS randomness)
type RandomizedHasher = SipHash13WithRandomSeed

# HashMap type
type HashMap[K: Hash + Eq, V, H: Hasher = DefaultHasher]

# Usage in alloc (embedded-safe)
let map: HashMap[String, Int] = HashMap.new()  # Uses DefaultHasher

# Usage in std (secure by default)
import aria.std.collections { SecureHashMap }
# SecureHashMap = HashMap[K, V, RandomizedHasher]
```

**Rationale**: Unlike Rust, we make the deterministic hasher the default in `alloc` to enable embedded usage. `aria.std` re-exports `SecureHashMap` with randomized hashing for security-sensitive applications.

### 4.3 Comparison with Rust

| Type | Rust Location | Aria Location | Difference |
|------|---------------|---------------|------------|
| HashMap | std | alloc | Aria uses deterministic default |
| HashSet | std | alloc | Same |
| Duration | core | core | Same |
| Instant | std | std | Same |
| Mutex | std | std | Same |
| RefCell | core | alloc | Aria puts in alloc (needs some allocation for checks) |

---

## 5. Async-Aware Collection Design

### 5.1 The AsyncIterator Trait

```aria
# In aria.async.stream

# Core async iteration trait
trait AsyncIterator[Item] {
  # Poll for next item (low-level)
  fn poll_next(pin ref self, cx: ref Context) -> Poll[Option[Item]]

  # Size hint for optimization
  fn size_hint(self) -> (Int, Option[Int]) = (0, None)
}

# Poll type (from core)
type Poll[T] = Ready(T) | Pending

# High-level extension trait (automatically available)
trait AsyncIteratorExt[Item]: AsyncIterator[Item] {
  # Async next (sugar for poll_next)
  async fn next(ref self) -> Option[Item]

  # Combinators (all async)
  fn map[B](self, f: Item -> B) -> Map[Self, F]
  fn filter(self, pred: Item -> Bool) -> Filter[Self, P]
  fn filter_map[B](self, f: Item -> Option[B]) -> FilterMap[Self, F]
  fn flat_map[B, I: AsyncIterator[B]](self, f: Item -> I) -> FlatMap[Self, F]
  fn take(self, n: Int) -> Take[Self]
  fn skip(self, n: Int) -> Skip[Self]
  fn enumerate(self) -> Enumerate[Self]
  fn zip[Other: AsyncIterator](self, other: Other) -> Zip[Self, Other]

  # Terminal operations
  async fn collect[C: FromAsyncIterator[Item]](self) -> C
  async fn fold[B](self, init: B, f: (B, Item) -> B) -> B
  async fn for_each(self, f: Item -> ()) -> ()
  async fn count(self) -> Int
  async fn any(self, pred: Item -> Bool) -> Bool
  async fn all(self, pred: Item -> Bool) -> Bool
  async fn find(self, pred: Item -> Bool) -> Option[Item]

  # Concurrency control
  fn buffer(self, size: Int) -> Buffered[Self]
  fn buffer_unordered(self, size: Int) -> BufferUnordered[Self]

  # Conversion
  fn into_sync(self) -> SyncIterator[Self] where Item: Transfer
}
```

### 5.2 Async Collection Methods

```aria
# Vec async extension methods
impl[T] Vec[T] {
  # Parallel iteration (returns AsyncIterator)
  fn par(self) -> ParallelIterator[T] where T: Transfer {
    ParallelIterator.new(self)
  }

  # Async iteration over owned vec
  fn into_async_iter(self) -> VecAsyncIterator[T] {
    VecAsyncIterator.new(self)
  }
}

# Usage example
async fn process_items(items: Vec[Item]) -> Vec[Result] {
  items
    .par()
    .map(async |item| process(item).await)
    .buffer_unordered(10)  # Process up to 10 concurrently
    .collect()
    .await
}
```

### 5.3 Stream Creation Utilities

```aria
# In aria.async.stream

# Create stream from async function
fn from_fn[T](f: async () -> Option[T]) -> FromFn[T]

# Create stream from iterator of futures
fn from_iter[I: IntoIterator[Item = impl Future[Output = T]], T](iter: I) -> FromIter[I]

# Create stream that yields forever
fn repeat[T: Clone](value: T) -> Repeat[T]

# Create stream from channel receiver
fn from_receiver[T](rx: Receiver[T]) -> ReceiverStream[T]

# Interval stream (yields at intervals)
fn interval(period: Duration) -> Interval

# Once stream (yields single value)
fn once[T](value: T) -> Once[T]

# Empty stream
fn empty[T]() -> Empty[T]

# Pending stream (never yields)
fn pending[T]() -> Pending[T]
```

### 5.4 Effect Integration with Async Collections

```aria
# Effect types for async operations
effect Async {
  fn spawn[T: Transfer](f: () -> T) -> Task[T]
  fn yield_now() -> ()
  fn sleep(duration: Duration) -> ()
}

# AsyncIterator combinators track effects
fn map[A, B, E](stream: impl AsyncIterator[A], f: A -> {E} B)
  -> impl AsyncIterator[B] + {E}

# Example: Effect inference through stream operations
fn process_urls(urls: Vec[String]) -> {Async, IO} Vec[Response] {
  urls
    .into_async_iter()
    .map(|url| http.get(url))  # IO effect inferred
    .buffer_unordered(5)
    .collect()
}
# Inferred signature: (Vec[String]) -> {Async, IO} Vec[Response]
```

### 5.5 Sync/Async Bridging

```aria
# Convert sync iterator to async
trait IntoAsyncIterator {
  type Item
  type AsyncIter: AsyncIterator[Item]
  fn into_async_iter(self) -> AsyncIter
}

# All sync iterators can become async
impl[I: Iterator] IntoAsyncIterator for I {
  type Item = I::Item
  type AsyncIter = SyncToAsync[I]

  fn into_async_iter(self) -> SyncToAsync[I] {
    SyncToAsync.new(self)
  }
}

# Convert async iterator to blocking sync (in std only)
impl[A: AsyncIterator] A {
  fn blocking_iter(self) -> BlockingIterator[A] where A::Item: Transfer {
    BlockingIterator.new(self)
  }
}

# Usage
fn example() {
  # Sync to async
  let async_iter = [1, 2, 3].iter().into_async_iter()

  # Async to sync (blocking)
  let sync_iter = async_stream.blocking_iter()
  for item in sync_iter {
    # Blocks until item available
  }
}
```

---

## 6. Essential Types Catalog

### 6.1 Type Categories by Necessity

#### Must Have (Day 1)

| Type | Tier | Description |
|------|------|-------------|
| Int, Float, Bool | core | Primitives |
| String, Str | alloc/core | Text |
| Option[T] | core | Optional values |
| Result[T, E] | core | Error handling |
| Vec[T] | alloc | Dynamic array |
| HashMap[K, V] | alloc | Hash table |
| Iterator | core | Iteration protocol |
| Future[T] | core | Async computation |

#### Should Have (Release 1.0)

| Type | Tier | Description |
|------|------|-------------|
| HashSet[T] | alloc | Unique elements |
| BTreeMap[K, V] | alloc | Ordered map |
| Deque[T] | alloc | Double-ended queue |
| Box[T] | alloc | Heap allocation |
| Rc[T], Arc[T] | alloc | Reference counting |
| File | std | File I/O |
| TcpStream | std | Networking |
| Channel[T] | std | Message passing |
| Duration, Instant | core/std | Time |
| Regex | text | Pattern matching |
| Json | encoding | Data interchange |

#### Nice to Have (Post 1.0)

| Type | Tier | Description |
|------|------|-------------|
| BinaryHeap[T] | alloc | Priority queue |
| LinkedList[T] | alloc | Linked list |
| PathBuf | std | Path manipulation |
| Command | std | Process spawning |
| Barrier, Condvar | std | Advanced sync |

### 6.2 Minimal Bootstrap Set

For a minimal Aria implementation (embedded/bootstrap):

```aria
# Minimal core (32 types/traits)
aria.core.types     { Int, UInt, Float, Bool, Char, Never }
aria.core.option    { Option, Some, None }
aria.core.result    { Result, Ok, Err }
aria.core.slice     { Slice }
aria.core.array     { Array }
aria.core.traits    { Eq, Ord, Clone, Copy, Debug }
aria.core.iter      { Iterator, IntoIterator }
aria.core.ops       { Add, Sub, Mul, Div, Index }

# Minimal alloc (6 types)
aria.alloc.vec      { Vec }
aria.alloc.string   { String }
aria.alloc.boxed    { Box }
aria.alloc.rc       { Rc }
aria.alloc.collections { HashMap }
```

This set of ~40 types enables most Aria programs.

### 6.3 Type Implementation Priority

```
Phase 1 (Bootstrap): core primitives, Option, Result, Vec, String
Phase 2 (Usability): HashMap, Iterator combinators, basic I/O
Phase 3 (Complete):  Full collections, networking, async
Phase 4 (Polish):    Advanced types, optimization, edge cases
```

---

## 7. Stability and Versioning

### 7.1 Stability Annotations

```aria
# Stability attributes for API evolution

@stable                    # Committed API, breaking changes = major version
@unstable("nightly")       # Experimental, may change
@deprecated(
  since: "0.5.0",
  note: "Use `new_function` instead",
  remove_in: "1.0.0"
)

# Feature gates for unstable features
#![feature(async_closures)]
#![feature(specialization)]

# Example usage
@stable
fn sort[T: Ord](arr: ref Vec[T]) -> ()

@unstable("nightly")
fn sort_by_cached_key[T, K: Ord](arr: ref Vec[T], f: T -> K) -> ()

@deprecated(since: "0.3.0", note: "Use `sort` instead")
fn old_sort[T: Ord](arr: ref Vec[T]) -> ()
```

### 7.2 Stability Tiers

| Tier | Meaning | Guarantee |
|------|---------|-----------|
| **Stable** | Production ready | No breaking changes in minor versions |
| **Unstable** | Experimental | May change or be removed |
| **Internal** | Implementation detail | No stability guarantee |
| **Deprecated** | Scheduled for removal | Warning on use |

### 7.3 Edition System

```aria
# Like Rust editions, Aria can evolve syntax without breaking old code

# aria.toml
[package]
name = "my-project"
edition = "2026"

# Edition-specific prelude changes
edition 2026 {
  prelude includes AsyncIterator
  String is implicitly convertible to &str
}

edition 2028 {
  # Future changes...
}
```

---

## 8. Design Decisions Summary

### 8.1 Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Tier structure** | 3 tiers (core/alloc/std) | Enables embedded and WASM |
| **HashMap location** | alloc (not std) | Deterministic default enables embedded |
| **Async model** | Effect-based | No function coloring |
| **Prelude scope** | Conservative (40 items) | Avoid namespace pollution |
| **Iterator/Stream** | Unified via effects | AsyncIterator = Iterator + Async effect |
| **Stability** | Rust-style annotations | Proven model for evolution |

### 8.2 Deferred Decisions

| Topic | Deferral Reason | Revisit When |
|-------|-----------------|--------------|
| HTTP server in stdlib | Community preference unclear | After ecosystem forms |
| Database abstractions | Too opinionated | External crate ecosystem |
| GUI/Graphics | Platform-specific | Separate project |
| Distributed computing | Complex design space | Post 1.0 |

### 8.3 Comparison with Inspiration Languages

| Aspect | Rust | Kotlin | Swift | Go | Aria |
|--------|------|--------|-------|-----|------|
| Tiers | 3 | 2 | 2+ | 1 | 3 |
| HashMap in minimal | No | N/A | N/A | N/A | Yes |
| Async in stdlib | Minimal | Stdlib+lib | Stdlib | Runtime | Effect-based |
| Prelude size | ~50 | ~30 | ~20 | N/A | ~40 |
| Effect tracking | No | No | No | No | **Yes** |

---

## 9. Implementation Roadmap

### Phase 1: Core Bootstrap (Weeks 1-4)

```
- [ ] Primitive types (Int, Float, Bool, Char)
- [ ] Option[T] and Result[T, E]
- [ ] Basic traits (Eq, Ord, Clone, Debug)
- [ ] Iterator trait and basic combinators
- [ ] Slice and Array types
```

### Phase 2: Alloc Foundation (Weeks 5-8)

```
- [ ] Vec[T] implementation
- [ ] String type with UTF-8
- [ ] Box[T] heap allocation
- [ ] HashMap[K, V] with deterministic hasher
- [ ] Formatting infrastructure
```

### Phase 3: Std Basics (Weeks 9-12)

```
- [ ] File I/O (Read, Write, File)
- [ ] Basic networking (TcpStream, TcpListener)
- [ ] Threading (spawn, join, sleep)
- [ ] Channels (bounded, unbounded)
- [ ] Time (Duration, Instant)
```

### Phase 4: Async Integration (Weeks 13-16)

```
- [ ] Future[T] type
- [ ] AsyncIterator trait
- [ ] Stream combinators
- [ ] Runtime effect handlers
- [ ] Parallel iterator support
```

### Phase 5: Polish (Weeks 17-20)

```
- [ ] Additional collections (BTreeMap, Deque)
- [ ] Extended I/O (buffered, seek)
- [ ] Error handling refinements
- [ ] Documentation
- [ ] Performance optimization
```

---

## 10. Open Questions for Future Research

1. **HTTP in stdlib?**: Should Aria include HTTP client/server like Go, or defer to ecosystem?

2. **Crypto primitives**: Should hashing (SHA-256) be in stdlib or external?

3. **Regex implementation**: Built-in or via binding to RE2/PCRE?

4. **JSON parser**: Hand-written or generated? Include in core stdlib?

5. **SIMD abstractions**: How to expose platform SIMD portably?

6. **GPU/compute**: Should stdlib provide GPU abstraction layer?

---

## 11. References

### Primary Sources

1. [Rust Standard Library](https://doc.rust-lang.org/std/)
2. [Effective Rust no_std](https://www.lurklurk.org/effective-rust/no-std.html)
3. [Rust liballoc RFC](https://rust-lang.github.io/rfcs/2480-liballoc.html)
4. [Kotlin Stdlib](https://kotlinlang.org/api/core/kotlin-stdlib/)
5. [kotlinx.coroutines](https://kotlinlang.org/api/kotlinx.coroutines/)
6. [Swift Core Libraries](https://www.swift.org/documentation/core-libraries/)
7. [Future of Foundation](https://www.swift.org/blog/future-of-foundation/)
8. [Go Ecosystem 2025](https://blog.jetbrains.com/go/2025/11/10/go-language-trends-ecosystem-2025/)
9. [Tokio Streams](https://tokio.rs/tokio/tutorial/streams)
10. [AsyncIterator RFC](https://rust-lang.github.io/rfcs/2996-async-iterator.html)
11. [State of Async Rust](https://corrode.dev/blog/async/)

### Internal Research

- ARIA-M16-01: Standard Library Approaches Survey
- ARIA-M11-01: Concurrency Model Design
- ARIA-M11-03: Rust Async Runtimes Comparison
- ARIA-M03-01: Algebraic Effects Survey
- ARIA-M15-01: Module Systems Comparison

---

## 12. Document Metadata

| Field | Value |
|-------|-------|
| Task ID | ARIA-M16-02 |
| Status | Completed |
| Author | GENESIS (Research Agent) |
| Date | 2026-01-15 |
| Iteration | Eureka 3 |
| Dependencies | ARIA-M16-01, ARIA-M11-01, ARIA-M15-01 |
| Enables | ARIA-M16-03 (Collections Design), ARIA-M16-04 (I/O Design) |
| Review Required | ARCHITECT, FORGE |
