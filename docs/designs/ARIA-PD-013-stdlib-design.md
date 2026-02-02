# ARIA-PD-013: Standard Library Design

**Decision ID**: ARIA-PD-013
**Status**: Approved
**Date**: 2026-01-15
**Author**: MATRIX (Product Decision Agent - Iteration 3)
**Research Inputs**:
- ARIA-M16-02: Standard Library Architecture Design (GENESIS)
- ARIA-PD-005: Effect System Design (ORACLE)
- ARIA-PD-006: Concurrency Model (ORACLE)

---

## Executive Summary

This document defines Aria's standard library organization, making concrete decisions about module hierarchy, essential types, async integration, naming conventions, and prelude contents. The design synthesizes GENESIS's research on Rust, Kotlin, Swift, and Go stdlib architectures.

**Final Decisions**:
1. **Module Hierarchy**: Three-tier architecture (core/alloc/std) enabling embedded and WASM targets
2. **Essential Types**: 38 types for v1.0, prioritized by implementation phase
3. **Async Integration**: Effect-based AsyncIterator unified with Iterator
4. **Naming Conventions**: snake_case for functions, PascalCase for types, lowercase for modules
5. **Prelude Contents**: Conservative 42-item prelude with tiered override options

---

## 1. Module Hierarchy Design

### 1.1 Three-Tier Architecture

```
ARIA STANDARD LIBRARY ARCHITECTURE

+================================================================+
|                        aria.std                                  |
|  Requirements: Operating System + Allocator + Runtime            |
|  Contents: File I/O, Networking, Threading, Time, Process        |
+================================================================+
                              |
                              | depends on
                              v
+================================================================+
|                       aria.alloc                                 |
|  Requirements: Global Allocator                                  |
|  Contents: Vec, HashMap, String, Box, Rc, Arc, RefCell          |
+================================================================+
                              |
                              | depends on
                              v
+================================================================+
|                       aria.core                                  |
|  Requirements: None (Freestanding)                               |
|  Contents: Primitives, Option, Result, Iterator, Traits         |
+================================================================+
```

### 1.2 Tier Boundaries

| Tier | Name | Requirements | Import Syntax |
|------|------|--------------|---------------|
| 0 | `aria.core` | None | `import aria.core` |
| 1 | `aria.alloc` | Global allocator | `import aria.alloc` |
| 2 | `aria.std` | OS + allocator | `import aria.std` |

### 1.3 Target Configuration

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
import aria.std.wasm      # WASM-specific APIs
# aria.std.fs, aria.std.thread not available

# Native target (default)
import aria.core
import aria.alloc
import aria.std           # Full functionality
```

---

## 2. Complete Module Organization

### 2.1 aria.core Modules

```
aria.core
 +-- types             # Compiler primitive types
 |   +-- Int8, Int16, Int32, Int64, Int
 |   +-- UInt8, UInt16, UInt32, UInt64, UInt
 |   +-- Float32, Float64, Float
 |   +-- Bool, Char, Never
 |
 +-- option            # Option[T] = Some(T) | None
 |
 +-- result            # Result[T, E] = Ok(T) | Err(E)
 |
 +-- ordering          # Ordering = Less | Equal | Greater
 |
 +-- slice             # Slice[T] borrowed view
 |
 +-- array             # Array[T, const N] fixed-size
 |
 +-- range             # Range[T], RangeInclusive[T], RangeFull
 |
 +-- traits            # Fundamental traits
 |   +-- eq            # Eq trait
 |   +-- ord           # Ord trait (extends Eq)
 |   +-- hash          # Hash trait
 |   +-- clone         # Clone trait
 |   +-- copy          # Copy marker trait
 |   +-- default       # Default trait
 |   +-- debug         # Debug trait (formatting)
 |   +-- display       # Display trait (user-facing)
 |
 +-- marker            # Marker traits
 |   +-- Transfer      # Safe to move between tasks
 |   +-- Sharable      # Safe to share between tasks
 |   +-- Sized         # Has known size at compile time
 |
 +-- iter              # Iterator infrastructure
 |   +-- Iterator      # Core iteration trait
 |   +-- IntoIterator  # Conversion to iterator
 |   +-- DoubleEndedIterator
 |   +-- ExactSizeIterator
 |   +-- FusedIterator
 |
 +-- ops               # Operator traits
 |   +-- Add, Sub, Mul, Div, Rem
 |   +-- Neg, Not
 |   +-- BitAnd, BitOr, BitXor
 |   +-- Shl, Shr
 |   +-- Index, IndexMut
 |   +-- Deref, DerefMut
 |
 +-- convert           # Type conversion
 |   +-- From, Into
 |   +-- TryFrom, TryInto
 |   +-- AsRef, AsMut
 |
 +-- num               # Numeric operations (no allocation)
 |   +-- wrapping arithmetic
 |   +-- saturating arithmetic
 |   +-- checked arithmetic
 |
 +-- ptr               # Raw pointer operations (unsafe)
 |
 +-- mem               # Memory intrinsics
     +-- size_of, align_of
     +-- swap, replace, take
```

### 2.2 aria.alloc Modules

```
aria.alloc
 +-- vec               # Vec[T] dynamic array
 |   +-- Vec[T]
 |   +-- IntoIter[T]
 |   +-- Drain[T]
 |
 +-- string            # String types
 |   +-- String        # Owned UTF-8 string
 |   +-- FromUtf8Error
 |
 +-- boxed             # Heap allocation
 |   +-- Box[T]
 |
 +-- rc                # Reference counting (single-threaded)
 |   +-- Rc[T]
 |   +-- Weak[T]
 |
 +-- sync              # Atomic reference counting
 |   +-- Arc[T]
 |   +-- Weak[T] (atomic)
 |
 +-- cell              # Interior mutability
 |   +-- Cell[T]       # Copy-based
 |   +-- RefCell[T]    # Runtime borrow checking
 |   +-- OnceCell[T]   # Single initialization
 |
 +-- collections       # Collection types
 |   +-- hash_map      # HashMap[K: Hash + Eq, V]
 |   +-- hash_set      # HashSet[T: Hash + Eq]
 |   +-- btree_map     # BTreeMap[K: Ord, V]
 |   +-- btree_set     # BTreeSet[T: Ord]
 |   +-- deque         # Deque[T]
 |   +-- binary_heap   # BinaryHeap[T: Ord]
 |   +-- linked_list   # LinkedList[T] (rare use)
 |
 +-- fmt               # String formatting
     +-- Formatter
     +-- Arguments
     +-- format()
```

### 2.3 aria.std Modules

```
aria.std
 +-- prelude           # Auto-imported items
 |
 +-- io                # I/O traits and utilities
 |   +-- Read          # trait Read
 |   +-- Write         # trait Write
 |   +-- Seek          # trait Seek
 |   +-- BufRead       # trait BufRead
 |   +-- BufReader[R]
 |   +-- BufWriter[W]
 |   +-- Cursor[T]     # In-memory I/O
 |   +-- stdin, stdout, stderr
 |   +-- Error, ErrorKind
 |
 +-- fs                # File system
 |   +-- File
 |   +-- OpenOptions
 |   +-- Metadata
 |   +-- DirEntry
 |   +-- ReadDir
 |   +-- read_to_string(path)
 |   +-- write(path, contents)
 |   +-- create_dir(path)
 |   +-- remove_file(path)
 |
 +-- path              # Path manipulation
 |   +-- Path          # Borrowed path slice
 |   +-- PathBuf       # Owned path
 |   +-- Component
 |
 +-- net               # Networking
 |   +-- tcp
 |   |   +-- TcpListener
 |   |   +-- TcpStream
 |   +-- udp
 |   |   +-- UdpSocket
 |   +-- addr
 |       +-- IpAddr = V4(Ipv4Addr) | V6(Ipv6Addr)
 |       +-- SocketAddr
 |       +-- ToSocketAddrs
 |
 +-- thread            # Threading
 |   +-- spawn(f)      # Spawn OS thread
 |   +-- current()     # Current thread handle
 |   +-- sleep(duration)
 |   +-- yield_now()
 |   +-- Thread
 |   +-- JoinHandle[T]
 |   +-- ThreadId
 |
 +-- sync              # Synchronization primitives
 |   +-- Mutex[T]      # Mutual exclusion
 |   +-- RwLock[T]     # Reader-writer lock
 |   +-- Barrier       # Thread barrier
 |   +-- Condvar       # Condition variable
 |   +-- Once          # One-time initialization
 |   +-- OnceLock[T]   # Thread-safe OnceCell
 |
 +-- channel           # Message passing
 |   +-- channel[T]()  # Unbounded channel
 |   +-- bounded[T](n) # Bounded channel
 |   +-- Sender[T]
 |   +-- Receiver[T]
 |   +-- SendError[T]
 |   +-- RecvError
 |
 +-- time              # Time utilities
 |   +-- Duration      # Time span (also in core)
 |   +-- Instant       # Monotonic clock
 |   +-- SystemTime    # Wall clock
 |   +-- UNIX_EPOCH
 |
 +-- process           # Process management
 |   +-- Command
 |   +-- Child
 |   +-- ExitStatus
 |   +-- exit(code)
 |   +-- abort()
 |   +-- id()
 |
 +-- env               # Environment
 |   +-- args()        # Command-line arguments
 |   +-- vars()        # Environment variables
 |   +-- var(key)      # Get env var
 |   +-- set_var(k, v) # Set env var
 |   +-- current_dir()
 |   +-- set_current_dir(path)
 |
 +-- ffi               # Foreign function interface
 |   +-- CString       # Null-terminated C string
 |   +-- CStr          # Borrowed C string
 |   +-- c_void
 |   +-- c_char, c_int, c_long, etc.
 |
 +-- os                # OS-specific functionality
     +-- linux         # Linux-specific APIs
     +-- macos         # macOS-specific APIs
     +-- windows       # Windows-specific APIs
```

### 2.4 aria.async Module

```
aria.async             # Effect-based async runtime
 +-- task              # Task management
 |   +-- Task[T]       # Async task handle
 |   +-- spawn(f)      # Spawn async task
 |   +-- spawn_blocking(f)
 |   +-- yield_now()
 |
 +-- future            # Future type (effect-based)
 |   +-- Future[T]     # Represents pending computation
 |   +-- Poll[T]       # Poll result = Ready(T) | Pending
 |
 +-- stream            # Async iteration
 |   +-- AsyncIterator # Core trait
 |   +-- StreamExt     # Extension methods
 |   +-- from_fn(f)    # Create from async closure
 |   +-- from_iter(i)  # Create from iterator
 |   +-- interval(d)   # Periodic stream
 |   +-- once(v)       # Single-item stream
 |   +-- empty()       # Empty stream
 |
 +-- select            # Concurrency combinators
 |   +-- select!       # Race multiple futures
 |   +-- join!         # Await all futures
 |
 +-- runtime           # Runtime configuration
     +-- Runtime
     +-- Builder
     +-- current()
```

### 2.5 Extended Modules (Post-1.0)

```
aria.encoding          # Serialization (v1.1+)
 +-- json              # JSON parsing/generation
 +-- base64            # Base64 encoding
 +-- hex               # Hexadecimal encoding
 +-- utf8              # UTF-8 utilities

aria.text              # Text processing (v1.1+)
 +-- regex             # Regular expressions
 +-- template          # String templates

aria.math              # Mathematics (v1.1+)
 +-- rand              # Random number generation
 +-- num               # Extended numeric operations

aria.test              # Testing utilities (v1.0)
 +-- assert            # Assertion macros
 +-- property          # Property-based testing
 +-- mock              # Mocking utilities
```

---

## 3. Essential Types for v1.0

### 3.1 Type Catalog by Priority

#### Phase 1: Bootstrap (Must Have Day 1)

| Type | Module | Description |
|------|--------|-------------|
| `Int`, `Float`, `Bool`, `Char` | core.types | Primitive types |
| `String` | alloc.string | UTF-8 growable string |
| `Str` | core.types | Borrowed string slice |
| `Option[T]` | core.option | Optional value |
| `Result[T, E]` | core.result | Error handling |
| `Vec[T]` | alloc.vec | Dynamic array |
| `Array[T, N]` | core.array | Fixed-size array |
| `Slice[T]` | core.slice | Borrowed view |
| `Iterator` | core.iter | Iteration protocol |
| `Eq`, `Ord`, `Clone`, `Debug` | core.traits | Fundamental traits |

#### Phase 2: Usability (Release Candidate)

| Type | Module | Description |
|------|--------|-------------|
| `HashMap[K, V]` | alloc.collections | Hash table |
| `HashSet[T]` | alloc.collections | Unique elements |
| `Box[T]` | alloc.boxed | Heap allocation |
| `Rc[T]`, `Arc[T]` | alloc.rc/sync | Reference counting |
| `RefCell[T]` | alloc.cell | Runtime borrow check |
| `File` | std.fs | File handle |
| `Read`, `Write` | std.io | I/O traits |
| `Duration` | core.time | Time span |
| `Instant` | std.time | Monotonic clock |
| `Mutex[T]` | std.sync | Mutual exclusion |

#### Phase 3: Complete (v1.0 Release)

| Type | Module | Description |
|------|--------|-------------|
| `BTreeMap[K, V]` | alloc.collections | Ordered map |
| `BTreeSet[T]` | alloc.collections | Ordered set |
| `Deque[T]` | alloc.collections | Double-ended queue |
| `TcpStream`, `TcpListener` | std.net | Networking |
| `Channel[T]` | std.channel | Message passing |
| `Thread`, `JoinHandle` | std.thread | Threading |
| `Path`, `PathBuf` | std.path | Path manipulation |
| `Command` | std.process | Process spawning |

#### Phase 4: Polish (v1.0+)

| Type | Module | Description |
|------|--------|-------------|
| `BinaryHeap[T]` | alloc.collections | Priority queue |
| `LinkedList[T]` | alloc.collections | Linked list |
| `Barrier`, `Condvar` | std.sync | Advanced sync |
| `UdpSocket` | std.net | UDP networking |

### 3.2 Type Count Summary

| Tier | Types | Traits | Total |
|------|-------|--------|-------|
| core | 15 | 18 | 33 |
| alloc | 12 | 3 | 15 |
| std | 25 | 5 | 30 |
| **Total v1.0** | **52** | **26** | **78** |

---

## 4. Async Integration Design

### 4.1 Effect-Based Async Model

Aria's async is built on the effect system (ARIA-PD-005), avoiding the "function coloring" problem.

```aria
# Effects for async operations
effect Async
  fn spawn[T: Transfer](f: () -> T) -> Task[T]
  fn yield_now() -> Unit
  fn sleep(duration: Duration) -> Unit
end

# Functions using Async effect
fn fetch_url(url: String) -> !Async Result[Response, Error]
  let response = http_get(url)  # Implicitly async
  response
end

# Effect inference - no explicit annotation needed
fn process_urls(urls: Vec[String]) -> Vec[Response]
  urls.map(|url| fetch_url(url)).collect()
  # Inferred: !Async !IO
end
```

### 4.2 AsyncIterator Trait

```aria
# Core async iteration trait (in aria.async.stream)
trait AsyncIterator[Item]
  # Low-level poll (for runtime implementors)
  fn poll_next(pin ref self, cx: ref Context) -> Poll[Option[Item]]

  # Size hint for optimization
  fn size_hint(self) -> (Int, Option[Int]) = (0, None)
end

# Poll result type
type Poll[T] = Ready(T) | Pending

# High-level extension trait (auto-implemented)
trait AsyncIteratorExt[Item]: AsyncIterator[Item]
  # Async next (sugar)
  async fn next(ref self) -> Option[Item]

  # Combinators
  fn map[B](self, f: Item -> B) -> Map[Self, F]
  fn filter(self, pred: Item -> Bool) -> Filter[Self, P]
  fn filter_map[B](self, f: Item -> Option[B]) -> FilterMap[Self, F]
  fn flat_map[B, I: AsyncIterator[B]](self, f: Item -> I) -> FlatMap[Self, F]
  fn take(self, n: Int) -> Take[Self]
  fn skip(self, n: Int) -> Skip[Self]
  fn enumerate(self) -> Enumerate[Self]
  fn zip[Other: AsyncIterator](self, other: Other) -> Zip[Self, Other]

  # Concurrency control
  fn buffer(self, size: Int) -> Buffered[Self]
  fn buffer_unordered(self, size: Int) -> BufferUnordered[Self]

  # Terminal operations
  async fn collect[C: FromAsyncIterator[Item]](self) -> C
  async fn fold[B](self, init: B, f: (B, Item) -> B) -> B
  async fn for_each(self, f: Item -> Unit) -> Unit
  async fn count(self) -> Int
  async fn any(self, pred: Item -> Bool) -> Bool
  async fn all(self, pred: Item -> Bool) -> Bool
  async fn find(self, pred: Item -> Bool) -> Option[Item]
end
```

### 4.3 Unified Iterator/AsyncIterator Pattern

```aria
# Sync iteration (Iterator)
for item in items
  process(item)
end

# Async iteration (AsyncIterator) - same syntax!
for item in async_stream
  process(item)  # Automatically awaits
end

# Effect inference handles the difference
fn process_items[I: IntoIterator[Item = T]](items: I) -> Vec[Result]
  items.iter().map(|item| process(item)).collect()
  # If process() is async, entire function becomes async
end
```

### 4.4 Sync/Async Bridging

```aria
# Convert sync iterator to async
impl[I: Iterator] IntoAsyncIterator for I
  type AsyncIter = SyncToAsync[I]
  fn into_async_iter(self) -> SyncToAsync[I]
end

# Convert async to blocking sync (std only)
fn blocking_iter[A: AsyncIterator](stream: A) -> BlockingIterator[A]
  where A.Item: Transfer

# Usage
let sync_iter = [1, 2, 3].iter()
let async_iter = sync_iter.into_async_iter()

let blocking = async_stream.blocking_iter()
for item in blocking  # Blocks until available
  println(item)
end
```

### 4.5 Parallel Iteration

```aria
# Vec parallel extension
impl[T] Vec[T]
  fn par(self) -> ParallelIterator[T] where T: Transfer
end

# Usage with buffered concurrency
async fn process_items(items: Vec[Item]) -> Vec[Result]
  items
    .par()
    .map(async |item| process(item))
    .buffer_unordered(10)  # Process up to 10 concurrently
    .collect()
end
```

---

## 5. Naming Conventions

### 5.1 Identifier Conventions

| Element | Convention | Examples |
|---------|------------|----------|
| **Types** | PascalCase | `HashMap`, `TcpStream`, `Option` |
| **Traits** | PascalCase | `Iterator`, `Clone`, `Display` |
| **Functions** | snake_case | `read_to_string`, `spawn_blocking` |
| **Methods** | snake_case | `iter.map()`, `vec.push()` |
| **Modules** | lowercase | `aria.std.io`, `aria.alloc.vec` |
| **Constants** | SCREAMING_SNAKE | `MAX_VALUE`, `UNIX_EPOCH` |
| **Type Parameters** | Single uppercase | `T`, `K`, `V`, `E` |
| **Effect Parameters** | Uppercase word | `Async`, `IO`, `State` |
| **Lifetimes** | Lowercase prefix `'` | `'a`, `'static` |

### 5.2 Naming Patterns

#### Constructor Functions

```aria
# Type::new() for default construction
let vec = Vec.new()
let map = HashMap.new()

# Type::with_*() for configured construction
let vec = Vec.with_capacity(100)
let map = HashMap.with_hasher(custom_hasher)

# Type::from_*() for conversion
let string = String.from_utf8(bytes)
let vec = Vec.from_iter(iter)
```

#### Conversion Methods

```aria
# as_*() for cheap reference conversion (borrowed)
let slice = string.as_bytes()
let str_ref = string.as_str()

# to_*() for expensive conversion (cloned/allocated)
let owned = slice.to_vec()
let string = bytes.to_string()

# into_*() for consuming conversion (owned)
let bytes = string.into_bytes()
let vec = array.into_vec()
```

#### Query Methods

```aria
# is_*() for boolean queries
option.is_some()
result.is_ok()
string.is_empty()

# has_*() for containment queries
map.has_key(k)
set.has_item(v)
```

#### Mutating Methods

```aria
# verb for mutation (no prefix)
vec.push(item)
vec.pop()
map.insert(k, v)
map.remove(k)

# *_mut suffix for mutable reference return
vec.get_mut(index)
map.entry_mut(key)
```

#### Iterator Methods

```aria
# iter() for shared reference iterator
vec.iter()        # -> Iterator[&T]

# iter_mut() for mutable reference iterator
vec.iter_mut()    # -> Iterator[&mut T]

# into_iter() for owned iterator (consumes)
vec.into_iter()   # -> Iterator[T]

# *_by() for custom comparator
vec.sort_by(|a, b| a.field.cmp(b.field))
iter.max_by_key(|x| x.priority)
```

### 5.3 Module Naming

```aria
# Module path follows directory structure
aria.std.io.read      # aria/std/io/read.aria
aria.alloc.collections.hash_map  # aria/alloc/collections/hash_map.aria

# Abbreviated names for common modules
import aria.std.io { Read, Write }          # Not aria.std.input_output
import aria.std.fs { File }                 # Not aria.std.file_system
import aria.alloc.fmt { format }            # Not aria.alloc.format

# Full names for clarity in specialized modules
import aria.std.sync { Mutex, RwLock }      # Not aria.std.synchronization
import aria.async.stream { AsyncIterator }  # Clear async context
```

### 5.4 Effect Naming

```aria
# Effect names are PascalCase nouns
effect IO         # Input/Output operations
effect Async      # Asynchronous execution
effect State[S]   # Stateful computation
effect Exception[E]  # Exception handling

# Effect operations are snake_case verbs
effect Console
  fn print(msg: String) -> Unit
  fn read_line() -> String
  fn flush() -> Unit
end
```

---

## 6. Prelude Contents

### 6.1 Default Prelude (aria.std.prelude)

The prelude is automatically imported into every Aria source file.

```aria
# === Re-exports from aria.core ===

# Primitive types
pub use aria.core.types { Int, Float, Bool, Char, Never }

# String slice
pub use aria.core.types { Str }

# Algebraic types
pub use aria.core.option { Option, Some, None }
pub use aria.core.result { Result, Ok, Err }
pub use aria.core.ordering { Ordering, Less, Equal, Greater }

# Fundamental traits
pub use aria.core.traits {
  Eq, Ord, Hash,
  Clone, Copy,
  Default,
  Debug, Display
}

# Marker traits
pub use aria.core.marker { Transfer, Sharable, Sized }

# Iteration
pub use aria.core.iter { Iterator, IntoIterator }

# Conversion
pub use aria.core.convert { From, Into }

# === Re-exports from aria.alloc ===

# Collections
pub use aria.alloc.vec { Vec }
pub use aria.alloc.string { String }
pub use aria.alloc.collections.hash_map { HashMap }
pub use aria.alloc.collections.hash_set { HashSet }

# Smart pointers
pub use aria.alloc.boxed { Box }
pub use aria.alloc.rc { Rc }
pub use aria.alloc.sync { Arc }

# === Common functions ===

# I/O
pub fn print(msg: impl Display) -> !IO Unit
pub fn println(msg: impl Display) -> !IO Unit
pub fn eprint(msg: impl Display) -> !IO Unit
pub fn eprintln(msg: impl Display) -> !IO Unit

# Debug
pub fn dbg[T: Debug](value: T) -> T  # Print and return

# Panics
pub fn panic(msg: Str) -> Never
pub fn todo(msg: Str = "not yet implemented") -> Never
pub fn unreachable(msg: Str = "entered unreachable code") -> Never

# Assertions
pub fn assert(condition: Bool, msg: Str = "assertion failed")
pub fn assert_eq[T: Eq + Debug](left: T, right: T)
pub fn assert_ne[T: Eq + Debug](left: T, right: T)

# === Async primitives ===

pub use aria.async.task { spawn }
pub use aria.async.future { Future }
```

### 6.2 Prelude Item Count

| Category | Items | Purpose |
|----------|-------|---------|
| Primitive types | 5 | Int, Float, Bool, Char, Never |
| Core types | 11 | Option, Result, Ordering, etc. |
| Fundamental traits | 10 | Eq, Ord, Clone, Debug, etc. |
| Marker traits | 3 | Transfer, Sharable, Sized |
| Iteration traits | 2 | Iterator, IntoIterator |
| Conversion traits | 2 | From, Into |
| Collection types | 4 | Vec, String, HashMap, HashSet |
| Smart pointers | 3 | Box, Rc, Arc |
| I/O functions | 4 | print, println, eprint, eprintln |
| Debug/panic functions | 6 | dbg, panic, todo, unreachable, assert* |
| Async primitives | 2 | spawn, Future |
| **Total** | **42** | |

### 6.3 Prelude Tiers

```aria
# === Prelude configuration options ===

# No prelude (explicit imports required)
#![no_prelude]

# Minimal prelude (core only, no allocations)
#![prelude(minimal)]
# Includes: primitives, Option, Result, core traits
# Excludes: Vec, String, HashMap, Box, Rc, Arc

# Alloc prelude (core + alloc, no I/O)
#![prelude(alloc)]
# Includes: minimal + Vec, String, Box, Rc, Arc
# Excludes: print, spawn, I/O functions

# Full prelude (default)
#![prelude(full)]
# Includes: everything listed above

# Custom prelude extension
#![prelude(full)]
use my_project.prelude.* as prelude  # Extend with project types
```

### 6.4 Prelude Override Example

```aria
# myproject/src/prelude.aria
# Custom prelude for a web application

pub use aria.std.prelude.*  # Include standard prelude

# Project-specific re-exports
pub use myproject.models { User, Session, Request, Response }
pub use myproject.error { AppError, AppResult }
pub use myproject.config { Config, env }

# Common external crates
pub use serde { Serialize, Deserialize }
pub use async_http { get, post }
```

```aria
# myproject/src/handlers/users.aria
#![prelude(myproject.prelude)]

# User, Session, Request, Response, AppResult all available
fn handle_login(req: Request) -> !Async AppResult[Response]
  let user = User.find_by_email(req.body.email)?
  let session = Session.create(user)?
  Ok(Response.json(session))
end
```

---

## 7. Key Design Decisions

### 7.1 HashMap in alloc vs std

**Decision**: HashMap placed in `aria.alloc` with configurable hasher.

```aria
# Default: Deterministic hasher (embedded-safe, DoS-vulnerable)
type HashMap[K: Hash + Eq, V] = HashMap[K, V, DefaultHasher]

# Secure: Randomized hasher (requires OS, DoS-resistant)
type SecureHashMap[K: Hash + Eq, V] = HashMap[K, V, RandomizedHasher]
```

**Rationale**: Unlike Rust, we make the deterministic hasher default to enable embedded usage. Security-sensitive applications should use `SecureHashMap` explicitly or configure the default via compiler flag.

### 7.2 RefCell Location

**Decision**: RefCell in `aria.alloc` (not core).

**Rationale**: RefCell requires runtime tracking state which benefits from heap allocation for the borrow state. Unlike Rust's core placement, this acknowledges the practical allocation needs.

### 7.3 Duration Location

**Decision**: Duration in `aria.core` (no allocation needed).

**Rationale**: Duration is just a struct holding nanoseconds - no OS calls needed. This matches Rust and enables time calculations in embedded.

```aria
# In aria.core.time
type Duration
  seconds: Int64
  nanos: UInt32

  fn from_secs(secs: Int64) -> Duration
  fn from_millis(millis: Int64) -> Duration
  fn from_nanos(nanos: Int64) -> Duration
  fn as_secs(self) -> Int64
  fn as_millis(self) -> Int64
  fn as_nanos(self) -> Int64
end
```

### 7.4 Channel in std vs alloc

**Decision**: Channel in `aria.std` (requires runtime).

**Rationale**: Channels require blocking/waking which needs OS primitives or runtime support. Cannot work in pure freestanding environment.

### 7.5 Prelude Size

**Decision**: Conservative 42-item prelude.

| Language | Prelude Size | Philosophy |
|----------|--------------|------------|
| Rust | ~50 | Comprehensive |
| Kotlin | ~30 | Minimal |
| Swift | ~20 | Very minimal |
| **Aria** | **42** | Balanced |

**Rationale**: Large enough to write useful code without imports, small enough to avoid namespace pollution. Users can extend via custom prelude modules.

---

## 8. Implementation Roadmap

### Phase 1: Core Bootstrap (Weeks 1-4)

```
[x] Primitive types (Int, Float, Bool, Char)
[x] Option[T] and Result[T, E]
[x] Basic traits (Eq, Ord, Clone, Debug)
[x] Iterator trait
[ ] Slice and Array types
[ ] Range types
```

### Phase 2: Alloc Foundation (Weeks 5-8)

```
[ ] Vec[T] with full API
[ ] String with UTF-8 handling
[ ] Box[T] heap allocation
[ ] HashMap[K, V] with deterministic hasher
[ ] HashSet[T]
[ ] Formatting infrastructure
```

### Phase 3: Std Basics (Weeks 9-12)

```
[ ] Read/Write traits
[ ] File I/O (File, OpenOptions)
[ ] Basic path handling
[ ] Threading (spawn, join, sleep)
[ ] Mutex, RwLock
[ ] Duration, Instant
```

### Phase 4: Async Integration (Weeks 13-16)

```
[ ] Future[T] type
[ ] AsyncIterator trait
[ ] Stream combinators
[ ] spawn() for async tasks
[ ] Channel[T] with async support
```

### Phase 5: Completion (Weeks 17-20)

```
[ ] BTreeMap, BTreeSet
[ ] Deque, BinaryHeap
[ ] Networking (TcpStream, TcpListener)
[ ] Process spawning
[ ] Environment access
```

---

## 9. Appendix: Module Path Reference

### 9.1 Quick Reference Table

| Need | Import |
|------|--------|
| Vector | `aria.alloc.vec.Vec` (in prelude) |
| Hash map | `aria.alloc.collections.HashMap` (in prelude) |
| File I/O | `aria.std.fs.{File, read_to_string}` |
| Networking | `aria.std.net.tcp.{TcpStream, TcpListener}` |
| Threading | `aria.std.thread.{spawn, sleep}` |
| Channels | `aria.std.channel.{channel, Sender, Receiver}` |
| Async tasks | `aria.async.task.{spawn, Task}` |
| Async streams | `aria.async.stream.{AsyncIterator, StreamExt}` |
| Time | `aria.std.time.{Instant, SystemTime}` |
| Mutex | `aria.std.sync.Mutex` |
| JSON | `aria.encoding.json.{Json, parse, stringify}` |
| Regex | `aria.text.regex.Regex` |
| Testing | `aria.test.{assert, property}` |

### 9.2 Common Import Patterns

```aria
# Full standard library (typical application)
import aria.std.{fs, io, net, thread, time}
import aria.async.{task, stream}

# Embedded/no_std
import aria.core.{option, result, iter, traits}
import aria.alloc.{vec, string}  # If allocator available

# Web/WASM
import aria.core.*
import aria.alloc.*
import aria.std.wasm.{fetch, console}
import aria.encoding.json
```

---

## 10. Document Metadata

| Field | Value |
|-------|-------|
| Decision ID | ARIA-PD-013 |
| Status | Approved |
| Author | MATRIX (Product Decision Agent) |
| Date | 2026-01-15 |
| Iteration | 3 |
| Research Input | ARIA-M16-02 (GENESIS) |
| Dependencies | ARIA-PD-005 (Effect System), ARIA-PD-006 (Concurrency) |
| Enables | Stdlib Implementation, Compiler Development |
| Review Required | ARCHITECT, FORGE |
