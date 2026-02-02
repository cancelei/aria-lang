# ARIA-M09-03: Comprehensive FFI Design for Aria

**Task ID**: ARIA-M09-03
**Status**: Completed
**Date**: 2026-01-15
**Agent**: BRIDGE (Eureka Iteration 3)
**Focus**: FFI and C/Python Interoperability Design

---

## Executive Summary

This document synthesizes research on Foreign Function Interface (FFI) patterns from Zig, Rust, and Python ecosystems to design Aria's approach to interoperability. The goal is to achieve Zig-level seamlessness with Rust-level safety guarantees while maintaining Aria's expressive syntax.

**Key Recommendations**:
1. **C Import**: Adopt Zig-style `@cImport` with libclang backend for direct C header parsing
2. **Memory Safety**: Implement ownership transfer annotations at FFI boundaries
3. **Python Interop**: Use PyO3-inspired GIL management with automatic inference
4. **WASM**: Target Component Model with WIT for cross-language portability

---

## 1. C Import Strategy

### 1.1 Comparison: Zig-style vs Bindgen

| Approach | Tooling | Maintenance | C++ Support | Macro Handling |
|----------|---------|-------------|-------------|----------------|
| **Zig @cImport** | None (built-in) | Low | Limited | Partial |
| **Rust bindgen** | External tool | Medium | Better | Configurable |
| **Manual bindings** | None | High | Full control | Manual |

**Source**: [Zig cImport Guide](https://zig.guide/working-with-c/c-import/), [Bindgen User Guide](https://rust-lang.github.io/rust-bindgen/)

### 1.2 Recommended Approach: Hybrid Zig-Style

Aria should adopt a **Zig-inspired direct import** with additional safety features:

```aria
# Direct C header import (compile-time parsing)
extern C from "sqlite3.h"
extern C from "openssl/ssl.h" as ssl

# With preprocessor control
extern C from "platform.h" do
  define "_GNU_SOURCE"
  define "MAX_BUFFER", "4096"
end
```

**Implementation**:
- Use **libclang** for parsing (full C11 support, ~100MB dependency)
- Cache parsed results by file path + modification time
- Single import namespace per compilation unit (avoid type incompatibility)

### 1.3 Type Mapping: C to Aria

| C Type | Aria Type | Notes |
|--------|-----------|-------|
| `int`, `short`, `long` | `CInt`, `CShort`, `CLong` | Platform-specific sizes |
| `int32_t`, `uint64_t` | `Int32`, `UInt64` | Fixed-size types |
| `float`, `double` | `CFloat`, `CDouble` | IEEE 754 |
| `T*` | `CPtr[T]` | Nullable C pointer |
| `const T*` | `CPtr[T].const` | Const-qualified pointer |
| `char*` | `CString` | Special NUL-terminated handling |
| `void*` | `CVoidPtr` or `Opaque` | Opaque pointer type |
| `struct S` | `extern struct S` | C-compatible layout |
| `union U` | `extern union U` | Tagged or untagged |
| `enum E` | `extern enum E` | Integer-backed enum |
| `T (*)(Args)` | `CFn[Args] -> T` | Function pointer |

### 1.4 Macro Handling Strategy

| Macro Type | Aria Support | Example |
|------------|--------------|---------|
| Object-like constants | Automatic | `#define MAX 100` -> `C.MAX = 100` |
| Simple function macros | Inline functions | `#define MIN(a,b)` -> inline fn |
| Complex macros | Manual wrapper | Requires `@c_macro` annotation |
| Variadic macros | Partial | May need manual intervention |

```aria
# Automatic: simple constant macros
extern C from "limits.h"
max_int = C.INT_MAX  # Works automatically

# Manual: complex macro wrapper
@c_macro
fn offsetof(type: Type, field: Symbol) -> USize
  # Implementation via compiler intrinsic
  __builtin_offsetof(type, field)
end
```

---

## 2. Memory Safety Across FFI Boundaries

### 2.1 The Fundamental Challenge

FFI inherently breaks language safety guarantees. As noted in [Effective Rust Item 34](https://effective-rust.com/ffi.html):
> "Allocate and free memory on the same side of the FFI boundary."

**Cross-Language Memory Issues**:
- Use-after-free when C frees memory Aria expects to use
- Memory leaks when ownership transfer is unclear
- Double-free when both sides try to clean up
- Buffer overflows from C code corrupting Aria memory

### 2.2 Ownership Transfer Annotations

Aria introduces explicit ownership annotations for FFI:

```aria
# Ownership annotations at FFI boundary
extern C
  # Aria owns returned pointer (must free it)
  @owned
  fn malloc(size: USize) -> CVoidPtr

  # C retains ownership (don't free in Aria)
  @borrowed
  fn get_error_message() -> CString

  # Ownership transfers to C (don't use in Aria after call)
  fn free(@transfer ptr: CVoidPtr)

  # C allocates, caller must free with specific function
  @owned(free_with: sqlite3_free)
  fn sqlite3_errmsg(db: CPtr[sqlite3]) -> CString
end
```

### 2.3 RAII Wrappers for C Resources

Aria automatically generates RAII wrappers when possible:

```aria
# From C declaration:
extern C from "sqlite3.h"

# Aria auto-generates (internal):
class SqliteDatabase
  @handle: CPtr[sqlite3]

  fn open(path: String) -> Result[SqliteDatabase, SqliteError]
    handle: CPtr[sqlite3] = null
    result = C.sqlite3_open(path.to_c_string, &handle)
    if result == C.SQLITE_OK
      Ok(SqliteDatabase.new(handle: handle))
    else
      Err(SqliteError.from_code(result))
    end
  end

  # Destructor calls C cleanup
  fn finalize
    C.sqlite3_close(@handle) unless @handle.null?
  end
end
```

### 2.4 Safe Wrapper Generation Rules

| C Pattern | Aria Wrapper | Safety Strategy |
|-----------|--------------|-----------------|
| `T* alloc()` | `fn alloc -> T!` | Return Result, track ownership |
| `void free(T*)` | `fn free(self)` | Move semantics, prevent reuse |
| `int init(T**)` | `fn init -> Result[T, Error]` | Error handling |
| `T* get_ref()` | `fn get_ref -> &T` | Borrow, don't free |
| `void callback(fn)` | `fn callback(cb: Fn)` | Prevent callback escaping |

### 2.5 Preventing Panic Across FFI

Rust's insight applies to Aria: panics must not cross FFI boundaries.

```aria
# Safe FFI function wrapper
@no_panic  # Compiler enforces no panic path
extern fn aria_callback(data: CVoidPtr) -> CInt
  # catch_panic converts panics to error codes
  catch_panic do
    # ... Aria code that might panic
    process_data(data)
    0  # Success
  rescue e
    log_error(e)
    -1  # Error code for C
  end
end
```

---

## 3. Python Interoperability

### 3.1 PyO3-Inspired Design

Based on research from [PyO3 documentation](https://pyo3.rs/) and existing ARIA-M10-01 analysis, Aria adopts:

**Key Principles**:
1. **Automatic GIL management** by default
2. **Zero-copy** for compatible types (NumPy arrays)
3. **Explicit control** when needed for performance

### 3.2 Python Import Syntax

```aria
# Import Python modules
extern Python from numpy as np
extern Python from pandas as pd
extern Python from torch

# Use Python objects with automatic type conversion
fn analyze_data(data: PyArray[Float64]) -> Float64
  # GIL acquired automatically when accessing Python
  mean = np.mean(data)

  # GIL released for pure Aria computation
  result = mean * 2.0 + compute_offset()

  result
end
```

### 3.3 GIL Management Strategy

| Scenario | GIL State | Automatic? |
|----------|-----------|------------|
| Accessing Python objects | Held | Yes |
| Pure Aria computation | Released | Yes |
| Parallel Aria code | Released | Yes |
| Python callback | Acquired | Yes |
| Explicit request | Configurable | No |

```aria
# Automatic (recommended default)
@python_interop(gil: :auto)
fn simple_compute(data: PyArray[Float]) -> Float
  data.sum  # Aria manages GIL automatically
end

# Explicit GIL control (advanced)
fn parallel_compute(py_data: PyObject) -> Array[Float]
  # Extract data (needs GIL)
  data = Python.with_gil |py|
    py_data.to_array(Float)
  end

  # Parallel compute (GIL released)
  data.parallel_map |x| x * 2.0 end
end
```

### 3.4 Type Conversion Matrix

| Aria Type | Python Type | Strategy | Copy? |
|-----------|-------------|----------|-------|
| `Int`, `Float` | `int`, `float` | Direct | No |
| `String` | `str` | Convert | Yes |
| `Array[Float]` | `numpy.ndarray` | View | No |
| `Option[T]` | `T \| None` | Mapped | No |
| `Result[T,E]` | exception/return | Mapped | No |
| `Dict[K,V]` | `dict` | Convert | Yes |
| `Struct` | class instance | Convert | Yes |

### 3.5 Zero-Copy Guidelines

```aria
# Zero-copy view (preferred for large data)
fn process_view(data: PyArrayView[Float64]) -> Float64
  # data is a view into Python memory
  # Must not outlive Python object (enforced by lifetimes)
  data.sum
end

# Safe copy (for data that needs to persist)
fn store_data(data: PyArray[Float64]) -> Array[Float64]
  # Explicit copy - data can outlive Python object
  data.to_aria_array
end
```

### 3.6 Exception Handling

```aria
# Python exceptions become Aria Results
fn safe_python_call(module: PyModule) -> Result[Int, PyError]
  try_py do
    module.call("risky_function", [1, 2, 3])
  end
end

# Aria errors become Python exceptions
@pyfunction
fn aria_function(x: Int) -> Int raises AriaError
  if x < 0
    raise AriaError("Negative not allowed")
  end
  x * 2
end
# In Python: raises AriaError as a Python exception
```

---

## 4. WASM Interface Types Analysis

### 4.1 Component Model Overview

The [WebAssembly Component Model](https://component-model.bytecodealliance.org/) provides:
- **WIT (WebAssembly Interface Types)**: IDL for defining interfaces
- **Canonical ABI**: Standard binary representation for complex types
- **WASI 0.2**: Stable system interfaces (released Jan 2024)

### 4.2 WIT for Aria Modules

Aria can generate WIT definitions for its modules:

```wit
// Auto-generated from Aria module
package aria:math@1.0.0;

interface calculations {
  record point {
    x: float64,
    y: float64,
  }

  distance: func(a: point, b: point) -> float64;
  scale: func(p: point, factor: float64) -> point;
}

world aria-math {
  export calculations;
}
```

```aria
# Aria source
module Math
  struct Point
    x: Float64
    y: Float64
  end

  @wasm_export
  fn distance(a: Point, b: Point) -> Float64
    dx = a.x - b.x
    dy = a.y - b.y
    (dx*dx + dy*dy).sqrt
  end

  @wasm_export
  fn scale(p: Point, factor: Float64) -> Point
    Point.new(x: p.x * factor, y: p.y * factor)
  end
end
```

### 4.3 Cross-Language Composition

The Component Model enables:
- Aria components calling Rust/Go/Python/JS components
- No language-specific FFI required
- Automatic marshalling via Canonical ABI

```aria
# Import external WASM component
extern wasm from "rust-crypto.wasm" as crypto

fn hash_data(data: Bytes) -> Bytes
  # Calls Rust component seamlessly
  crypto.sha256(data)
end
```

### 4.4 WASI 0.3 Roadmap (2025)

Upcoming async support will enable:
- Non-blocking I/O in WASM
- Better Go/Node.js interop patterns
- Native async/await across component boundaries

Aria should prepare for:
```aria
# Future: async WASM interop
@wasm_async
fn fetch_data(url: String) async -> Result[Bytes, WasiError]
  wasi.http.fetch(url).await
end
```

### 4.5 Type Mapping: Aria to WIT

| Aria Type | WIT Type | Notes |
|-----------|----------|-------|
| `Int`, `Int32` | `s32` | Signed 32-bit |
| `UInt64` | `u64` | Unsigned 64-bit |
| `Float64` | `float64` | IEEE 754 double |
| `String` | `string` | UTF-8 encoded |
| `Array[T]` | `list<T>` | Dynamic list |
| `Option[T]` | `option<T>` | Optional value |
| `Result[T,E]` | `result<T,E>` | Success or error |
| `struct` | `record` | Product type |
| `enum` | `variant` | Sum type |
| `Bytes` | `list<u8>` | Byte array |

---

## 5. Unified FFI Design

### 5.1 The `extern` Keyword Family

```aria
# C interop (Zig-style direct import)
extern C from "header.h"
extern C from "lib.h" as lib

# Python interop (PyO3-style)
extern Python from module as alias

# WASM Component interop
extern wasm from "component.wasm" as comp

# Generic foreign function declaration
extern "C" fn external_function(x: CInt) -> CInt
```

### 5.2 Safety Levels

```aria
# Level 1: Safe wrappers (default)
# Aria generates safe wrappers automatically
extern C from "sqlite3.h"
db = Sqlite.open("test.db")  # Safe API

# Level 2: Direct unsafe (explicit)
# For performance-critical code
unsafe
  ptr = C.malloc(1024)
  C.memcpy(ptr, data, size)
  C.free(ptr)
end

# Level 3: Raw FFI (expert only)
@raw_ffi
fn low_level_interop(ptr: RawPtr) -> RawPtr
  # No safety checks, full responsibility on developer
  C.dangerous_operation(ptr)
end
```

### 5.3 Ownership Across All FFI

```aria
# Universal ownership annotations work across all FFI types
extern C
  @owned fn c_alloc() -> CPtr[Data]
  fn c_free(@transfer ptr: CPtr[Data])
end

extern Python
  @borrowed fn py_get_buffer() -> PyBuffer
  @owned fn py_create_array(size: Int) -> PyArray
end

extern wasm
  @owned fn wasm_allocate(size: USize) -> WasmPtr
end
```

---

## 6. Implementation Recommendations

### 6.1 Phase 1: C Interop Foundation

1. Integrate libclang for header parsing
2. Implement basic type mapping (primitives, pointers, structs)
3. Generate safe RAII wrappers
4. Support `extern C from` syntax

**Timeline**: 4-6 weeks

### 6.2 Phase 2: Memory Safety Layer

1. Implement ownership annotations (`@owned`, `@borrowed`, `@transfer`)
2. Add lifetime tracking across FFI boundaries
3. Generate compile-time warnings for unsafe patterns
4. Support `@no_panic` annotation

**Timeline**: 3-4 weeks

### 6.3 Phase 3: Python Integration

1. Design GIL management primitives
2. Implement automatic GIL inference
3. Add zero-copy array support
4. Exception/Result interop

**Timeline**: 4-6 weeks

### 6.4 Phase 4: WASM Component Support

1. WIT code generation from Aria modules
2. Component Model type mapping
3. WASI 0.2 interface implementation
4. Cross-component linking

**Timeline**: 6-8 weeks

---

## 7. Open Questions

### 7.1 C Interop
1. Should libclang be bundled or require system installation?
2. How to handle platform-specific headers cleanly?
3. Strategy for C++ interop (basic support vs. full cxx-style)?

### 7.2 Memory Safety
1. Should unsafe FFI code require explicit `unsafe` blocks?
2. How strict should ownership inference be at boundaries?
3. Approach for handling C code that doesn't follow conventions?

### 7.3 Python
1. Should GIL control be exposed or fully abstracted?
2. How to handle Python async with Aria's effect system?
3. Strategy for calling Aria from Python (reverse direction)?

### 7.4 WASM
1. Priority of WASI 0.2 vs. waiting for 0.3 async?
2. Should Aria require Component Model or support core WASM?
3. How to handle WASM memory limits gracefully?

---

## 8. Key Resources

### C Interop
- [Zig @cImport Documentation](https://ziglang.org/documentation/master/)
- [Zig cImport Guide](https://zig.guide/working-with-c/c-import/)
- [Rust Bindgen User Guide](https://rust-lang.github.io/rust-bindgen/)
- [Effective Rust: FFI Boundaries](https://effective-rust.com/ffi.html)
- [C++/Rust FFI with cxx Bridge](https://markaicode.com/cpp-rust-ffi-cxx-bridge-2025/)

### Memory Safety
- [Inside FFI: How Rust Talks to C](https://medium.com/@theopinionatedev/inside-ffi-how-rust-talks-to-c-without-losing-safety-cfc764195ad8)
- [Cross-Language Memory Management Issues](https://www.zhuohua.me/assets/ESORICS2022-FFIChecker.pdf)
- [The Rustonomicon: FFI](https://doc.rust-lang.org/nomicon/ffi.html)
- [Safe FFI Patterns Guide](https://swenotes.com/2025/09/25/foreign-function-interfaces-ffi-a-practical-guide-for-software-teams/)

### Python Interop
- [PyO3 User Guide](https://pyo3.rs/)
- [PyO3 Memory Management](https://pyo3.rs/v0.20.0/memory)
- [Rust-Python FFI Performance](https://johal.in/rust-python-ffi-with-pyo3-creating-high-speed-extensions-for-performance-critical-apps/)
- [Pre-PEP: Rust for CPython](https://discuss.python.org/t/pre-pep-rust-for-cpython/104906)
- [Efficiently Extending Python with PyO3](https://www.blueshoe.io/blog/python-rust-pyo3/)

### WASM
- [WebAssembly Component Model](https://component-model.bytecodealliance.org/)
- [WIT Reference](https://component-model.bytecodealliance.org/design/wit.html)
- [WASI 0.2 Status](https://eunomia.dev/blog/2025/02/16/wasi-and-the-webassembly-component-model-current-status/)
- [WASI Interfaces](https://wasi.dev/interfaces)
- [Component Model at POPL 2025](https://popl25.sigplan.org/details/waw-2025-papers/4/The-WebAssembly-Component-Model)

---

## 9. Prior Art in Eureka Vault

This document synthesizes findings from:
- **ARIA-M09-01**: Zig @cImport Analysis
- **ARIA-M09-02**: libclang for C Parsing Study
- **ARIA-M10-01**: PyO3 Deep Dive

---

## Appendix A: Complete Aria FFI Example

```aria
# Complete FFI example demonstrating all features

# C library import
extern C from "sqlite3.h"

# Python scientific computing
extern Python from numpy as np
extern Python from scipy.optimize as opt

# WASM component for cryptography
extern wasm from "crypto.wasm" as crypto

# High-level database wrapper (auto-generated RAII)
class Database
  @handle: CPtr[sqlite3]

  fn open(path: String) -> Result[Database, DbError]
    handle: CPtr[sqlite3] = null
    result = C.sqlite3_open(path.to_c_string, &handle)
    if result == C.SQLITE_OK
      Ok(Database.new(handle: handle))
    else
      Err(DbError.from_code(result))
    end
  end

  fn query(sql: String) -> Result[Array[Row], DbError]
    # Safe wrapper around C API
    stmt = prepare_statement(sql)?
    defer finalize_statement(stmt)
    collect_rows(stmt)
  end

  fn finalize
    C.sqlite3_close(@handle) unless @handle.null?
  end
end

# Scientific computation with Python
fn optimize_model(data: Array[Float64]) -> Float64
  # Convert to NumPy (zero-copy if possible)
  np_data = data.to_numpy

  # Call SciPy optimizer (GIL managed automatically)
  result = opt.minimize(objective_fn, np_data)

  result.x.to_aria_float
end

# WASM-portable cryptographic operation
@wasm_export
fn secure_hash(data: Bytes) -> Bytes
  crypto.sha256(data)
end

# Main application
fn main
  # Database operations
  db = Database.open("analytics.db")!
  users = db.query("SELECT * FROM users")!

  # Scientific analysis
  scores = users.map(&.score)
  optimal = optimize_model(scores)

  # Secure result
  hash = secure_hash(optimal.to_bytes)

  println("Optimization complete: #{hash.hex}")
end
```

---

## Appendix B: Decision Summary

| Decision | Choice | Rationale |
|----------|--------|-----------|
| C Import Style | Zig-style direct | Lower maintenance, developer-friendly |
| C Parser Backend | libclang | Full C11 support, proven in Zig/bindgen |
| Ownership Model | Explicit annotations | Clear semantics at FFI boundary |
| Python GIL | Automatic with manual override | Ease of use + performance escape hatch |
| WASM Target | Component Model | Future-proof, language-agnostic |
| Safety Default | Safe wrappers | Prevent common FFI bugs |

---

*Research completed by BRIDGE agent, Eureka Iteration 3, January 2026*
