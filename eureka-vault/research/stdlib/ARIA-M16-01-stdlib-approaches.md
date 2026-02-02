# ARIA-M16-01: Standard Library Approaches Survey

**Task ID**: ARIA-M16-01
**Status**: Completed
**Date**: 2026-01-14
**Focus**: Survey stdlib design philosophies across languages

---

## Executive Summary

Standard libraries range from minimal (Go, Zig) to comprehensive (Python, Rust). This research analyzes design philosophies, organization strategies, and trade-offs to inform Aria's standard library design.

---

## 1. Overview

### 1.1 Stdlib Design Philosophies

| Philosophy | Description | Examples |
|------------|-------------|----------|
| Batteries included | Comprehensive stdlib | Python, Java |
| Minimal core | Small stdlib, rich ecosystem | Go, Zig |
| Hybrid | Core + extended stdlib | Rust, Swift |
| Modular | Opt-in stdlib components | Deno, Zig |

### 1.2 Trade-offs

| Batteries Included | Minimal Core |
|-------------------|--------------|
| ✓ Consistency | ✓ Fast compilation |
| ✓ No version conflicts | ✓ Small binaries |
| ✓ Single source of truth | ✓ Flexibility |
| ✗ Large binaries | ✗ Fragmentation |
| ✗ Slow evolution | ✗ Decision fatigue |

---

## 2. Python: Batteries Included

### 2.1 Philosophy

"Batteries included" — rich stdlib for common tasks:
- 200+ modules
- Web servers, email, XML, databases, compression
- Often "good enough" without external packages

### 2.2 Organization

```
python stdlib/
├── Built-in types (int, str, list, dict)
├── Built-in functions (print, len, map)
├── Data structures (collections, heapq, bisect)
├── File/IO (io, os, pathlib, shutil)
├── Text (string, re, textwrap)
├── Data formats (json, csv, xml, html)
├── Networking (http, urllib, socket)
├── Concurrency (threading, multiprocessing, asyncio)
├── Testing (unittest, doctest)
├── Debugging (pdb, traceback)
└── ... many more
```

### 2.3 Lessons

| Lesson | Detail |
|--------|--------|
| Slow updates | urllib vs requests, asyncio evolution |
| Compatibility burden | Legacy code depends on old APIs |
| Discovery problem | Hard to find right module |
| Quality variance | Some modules better maintained |

---

## 3. Go: Minimal but Complete

### 3.1 Philosophy

"Less is exponentially more" — small language, focused stdlib:
- ~150 packages
- High quality, consistent style
- Stable APIs

### 3.2 Organization

```
go stdlib/
├── fmt, io, os (core I/O)
├── strings, bytes, unicode (text)
├── encoding/json, encoding/xml (formats)
├── net/http (web server in stdlib!)
├── sync, context (concurrency)
├── testing (built-in testing)
├── reflect (reflection)
└── runtime (GC, scheduler)
```

### 3.3 Notable Decisions

| Decision | Rationale |
|----------|-----------|
| HTTP server included | Most Go programs are servers |
| No generics stdlib (pre-1.18) | Delayed until language support |
| Single testing framework | Consistency > choice |
| `context` for cancellation | Standard pattern |

---

## 4. Rust: Core + Alloc + Std

### 4.1 Three-Layer Design

```
┌─────────────────────────────────────┐
│              std                     │  ← Requires OS
│  (threads, networking, filesystem)   │
├─────────────────────────────────────┤
│              alloc                   │  ← Requires allocator
│  (Vec, String, Box, Rc, Arc)         │
├─────────────────────────────────────┤
│              core                    │  ← No requirements
│  (Option, Result, iterators, traits) │
└─────────────────────────────────────┘
```

### 4.2 Benefits

- Embedded support (`#![no_std]`)
- Clear dependency boundaries
- Minimal binary when needed

### 4.3 Stdlib vs Ecosystem

| Stdlib | Ecosystem |
|--------|-----------|
| `std::collections::HashMap` | `hashbrown` (faster) |
| `std::sync::Mutex` | `parking_lot` (faster) |
| (none) | `regex`, `serde`, `tokio` |

---

## 5. Zig: Comptime-Powered Stdlib

### 5.1 Philosophy

Small stdlib, powerful compile-time features:
- Stdlib uses same features as user code
- No hidden magic
- Cross-compilation friendly

### 5.2 Unique Features

```zig
// Stdlib uses comptime
pub fn sort(comptime T: type, items: []T, lessThan: fn(T, T) bool) void {
    // Generic at compile time, specialized at runtime
}

// Cross-platform abstractions
const os = @import("std").os;
// Same code works on all platforms
```

---

## 6. Swift: Foundation + Swift Stdlib

### 6.1 Two-Layer Model

```
┌─────────────────────────────────────┐
│           Foundation                 │  ← Objective-C heritage
│  (NSString, NSArray, networking)     │
├─────────────────────────────────────┤
│         Swift Stdlib                 │  ← Pure Swift
│  (Int, String, Array, protocols)     │
└─────────────────────────────────────┘
```

### 6.2 Migration Story

- Bridging between layers
- Gradual migration to pure Swift
- Protocol-oriented design

---

## 7. Deno: URL Imports + Std Library

### 7.1 Modular Stdlib

```typescript
// Import directly from URL
import { serve } from "https://deno.land/std@0.208.0/http/server.ts";
import { parse } from "https://deno.land/std@0.208.0/flags/mod.ts";

// Versioned imports
// Can use different versions in same project
```

### 7.2 Benefits

- No centralized registry required
- Explicit versioning
- Easy to fork/modify

---

## 8. Comparison Matrix

| Aspect | Python | Go | Rust | Zig | Swift | Deno |
|--------|--------|-----|------|-----|-------|------|
| Size | Large | Medium | Medium | Small | Large | Medium |
| Stability | Very stable | Stable | Evolving | Evolving | Stable | Evolving |
| No-std support | No | No | Yes | Yes | No | N/A |
| Quality consistency | Variable | High | High | High | Variable | High |
| Update frequency | Slow | Slow | Regular | Regular | Regular | Fast |

---

## 9. Core Stdlib Categories

### 9.1 Essential (Must Have)

| Category | Examples |
|----------|----------|
| Primitives | Int, Float, Bool, String |
| Collections | Array, Map, Set |
| Option/Result | Maybe, Either equivalents |
| I/O | File, stdin/stdout |
| Text | String manipulation |
| Math | Basic numeric operations |

### 9.2 Common (Should Have)

| Category | Examples |
|----------|----------|
| Concurrency | Channels, spawn, mutex |
| Networking | HTTP client (at minimum) |
| Formats | JSON, possibly more |
| Time | DateTime, Duration |
| Testing | Assertions, property testing |
| Regex | Pattern matching |

### 9.3 Extended (Nice to Have)

| Category | Examples |
|----------|----------|
| Database | Connection pools, queries |
| CLI | Argument parsing |
| Compression | gzip, zlib |
| Crypto | Hashing, encryption |
| Template | String templates |

---

## 10. Recommendations for Aria

### 10.1 Three-Tier Design

```aria
# Tier 1: Core (no runtime, always available)
aria.core {
  Int, Float, Bool, String
  Array, Option, Result
  Iterator, basic traits
}

# Tier 2: Alloc (requires allocator)
aria.alloc {
  Vec, HashMap, HashSet
  String (growable)
  Box, Rc, Arc
}

# Tier 3: Std (requires OS)
aria.std {
  File, IO
  Threading, Channels
  Networking
  Time
}
```

### 10.2 Module Organization

```aria
# Proposed structure
aria.core
  aria.core.types       # Primitives
  aria.core.traits      # Core traits
  aria.core.iter        # Iteration
  aria.core.ops         # Operators

aria.collections
  aria.collections.vec
  aria.collections.map
  aria.collections.set

aria.io
  aria.io.file
  aria.io.stream
  aria.io.buffer

aria.text
  aria.text.string
  aria.text.regex
  aria.text.format

aria.time
  aria.time.instant
  aria.time.duration
  aria.time.date

aria.sync
  aria.sync.mutex
  aria.sync.channel
  aria.sync.atomic

aria.net
  aria.net.http
  aria.net.tcp
  aria.net.tls

aria.encoding
  aria.encoding.json
  aria.encoding.base64

aria.test
  aria.test.assert
  aria.test.property
  aria.test.mock
```

### 10.3 Stability Tiers

```aria
# Stability annotations
@stable           # Committed API
fn array_sort[T: Ord](arr: Array[T]) -> Array[T]

@unstable("nightly")  # Experimental
fn array_sort_by_cached_key[T, K: Ord](arr: Array[T], f: T -> K) -> Array[T]

@deprecated(since: "0.5", use: "array_sort")
fn sort_array[T: Ord](arr: Array[T]) -> Array[T]
```

### 10.4 Effect-Aware Design

```aria
# Stdlib functions declare effects
fn read_file(path: String) -> {IO, Error[IOError]} String

fn print(msg: String) -> {IO}

fn spawn[T](task: () -> T) -> {Async} Future[T]

# Pure functions in core
fn map[A, B](arr: Array[A], f: A -> B) -> Array[B]  # No effects
```

### 10.5 Prelude

```aria
# Automatically imported
prelude {
  # Types
  Int, Float, Bool, String
  Array, Option, Result

  # Traits
  Eq, Ord, Hash, Debug, Clone
  Iterator, Into, From

  # Functions
  print, println, dbg

  # Result/Option methods
  ok, err, some, none
  unwrap, expect, map, and_then
}
```

---

## 11. Key Resources

1. [Python Standard Library](https://docs.python.org/3/library/)
2. [Go Standard Library](https://pkg.go.dev/std)
3. [Rust std Documentation](https://doc.rust-lang.org/std/)
4. [Zig Standard Library](https://ziglang.org/documentation/master/std/)
5. [Swift Standard Library](https://developer.apple.com/documentation/swift/swift-standard-library)

---

## 12. Open Questions

1. Where is the line between stdlib and ecosystem for Aria?
2. Should HTTP server be in stdlib (like Go)?
3. How do we handle platform-specific functionality?
4. What's the versioning/stability policy for stdlib?
