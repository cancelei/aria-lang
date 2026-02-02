# ARIA-PD-010: FFI and Foreign Interoperability Design

**Document Type**: Product Design Document (PDD)
**Status**: APPROVED
**Date**: 2026-01-15
**Author**: RELAY (Product Decision Agent)
**Research Source**: BRIDGE (ARIA-M09-03)

---

## Executive Summary

This document establishes Aria's official Foreign Function Interface (FFI) design based on comprehensive research from Eureka Iteration 3. After reviewing Zig's @cImport, Rust's bindgen and cxx, PyO3's GIL management, and the WebAssembly Component Model, this PDD makes concrete syntax and semantic decisions.

**Final Decision**: Aria adopts a **unified `extern` keyword family** with:
1. Zig-style direct C header import via `extern C from`
2. Explicit ownership annotations at all FFI boundaries
3. Automatic GIL management for Python with manual override
4. Component Model for WASM with WIT generation
5. Safe wrappers by default, explicit `unsafe` for raw access

---

## 1. C Import Strategy

### 1.1 Decision: Zig-Style Direct Import

**DECIDED**: Aria uses `extern C from` for direct C header parsing at compile time.

```aria
# Basic C header import
extern C from "sqlite3.h"
extern C from "openssl/ssl.h" as ssl

# With preprocessor definitions
extern C from "platform.h" do
  define "_GNU_SOURCE"
  define "MAX_BUFFER", "4096"
end

# Multiple headers under single namespace
extern C from ["header1.h", "header2.h"] as lib
```

**Rationale**:
- No external tooling required (bindgen step eliminated)
- Single source of truth (C header IS the binding)
- Lower maintenance burden
- Developer-friendly experience

### 1.2 Implementation: libclang Backend

**DECIDED**: Use libclang for C header parsing.

| Requirement | Decision |
|-------------|----------|
| Parser Backend | libclang (C11 full support) |
| Distribution | Bundled with Aria compiler |
| Cache Strategy | By file path + mtime + defines |
| C++ Support | Basic (structs, functions only) |

**Rationale**: libclang is battle-tested in Zig and rust-bindgen, provides full C11 support, and handles complex headers reliably.

### 1.3 C Type Mapping (Final)

| C Type | Aria Type | Notes |
|--------|-----------|-------|
| `int`, `short`, `long` | `CInt`, `CShort`, `CLong` | Platform-specific |
| `int32_t`, `uint64_t` | `Int32`, `UInt64` | Fixed-size |
| `float`, `double` | `CFloat`, `CDouble` | IEEE 754 |
| `char` | `CChar` | Platform-specific signedness |
| `T*` | `CPtr[T]` | Nullable pointer |
| `const T*` | `CPtr[T].const` | Const-qualified |
| `T* restrict` | `CPtr[T].restrict` | Restrict-qualified |
| `char*` / `const char*` | `CString` | NUL-terminated |
| `void*` | `CVoidPtr` | Type-erased pointer |
| `struct S` | `extern struct S` | C-compatible layout |
| `union U` | `extern union U` | C-compatible union |
| `enum E` | `extern enum E` | Integer-backed |
| `T (*)(Args)` | `CFn[(Args)] -> T` | Function pointer |
| `T[N]` | `CArray[T, N]` | Fixed-size array |

### 1.4 Macro Handling

| Macro Type | Aria Behavior |
|------------|---------------|
| Object-like constants | Auto-convert to `const` |
| Simple function macros | Auto-convert to `inline fn` |
| Complex macros | Require `@c_macro` wrapper |
| Variadic macros | Manual wrapper required |

```aria
# Automatic constant macro conversion
extern C from "limits.h"
max_int = C.INT_MAX  # Works automatically

# Manual complex macro wrapper
@c_macro
fn offsetof(type: Type, field: Symbol) -> USize
  __builtin_offsetof(type, field)
end
```

---

## 2. FFI Function Syntax and Safety Markers

### 2.1 External Function Declaration

**DECIDED**: Use `extern "ABI"` for explicit function declarations.

```aria
# Single function declaration
extern "C" fn puts(s: CString) -> CInt

# Block declaration
extern "C"
  fn malloc(size: USize) -> CVoidPtr
  fn free(ptr: CVoidPtr)
  fn memcpy(dest: CVoidPtr, src: CVoidPtr.const, n: USize) -> CVoidPtr
end

# Alternative ABI (future)
extern "stdcall" fn windows_api(x: CInt) -> CInt  # Windows calling convention
extern "fastcall" fn optimized(x: CInt) -> CInt   # Register-based
```

### 2.2 Calling C from Aria

**DECIDED**: C functions accessible via `C.` namespace prefix.

```aria
extern C from "stdio.h"

fn main
  C.puts("Hello from Aria!")

  buffer: CArray[CChar, 256]
  C.snprintf(buffer.ptr, 256, "Value: %d", 42)
end
```

### 2.3 Exporting Aria Functions to C

**DECIDED**: Use `@export` annotation with ABI specification.

```aria
# Export function with C ABI
@export(abi: "C", name: "aria_process")
fn process_data(data: CPtr[UInt8], len: USize) -> CInt
  # Aria code here
  0
end

# Generate C header declaration:
# int aria_process(uint8_t* data, size_t len);
```

### 2.4 Safety Markers

**DECIDED**: Three safety levels with explicit markers.

| Level | Marker | Behavior |
|-------|--------|----------|
| Safe | (default) | Auto-generated wrappers, bounds checking |
| Unsafe | `unsafe` block | Direct FFI, no runtime checks |
| Raw | `@raw_ffi` | No checks, no ownership tracking |

```aria
# Level 1: Safe wrappers (default)
extern C from "sqlite3.h"
db = Sqlite.open("test.db")!  # Safe wrapper with Result

# Level 2: Unsafe block for direct calls
unsafe
  ptr = C.malloc(1024)
  C.memset(ptr, 0, 1024)
  C.free(ptr)
end

# Level 3: Raw FFI (expert only)
@raw_ffi
fn dangerous_interop(ptr: RawPtr) -> RawPtr
  C.dangerous_operation(ptr)
end
```

### 2.5 No-Panic Enforcement

**DECIDED**: `@no_panic` annotation prevents panics crossing FFI boundaries.

```aria
# Callback safe for C to call
@no_panic
@export(abi: "C")
fn aria_callback(data: CVoidPtr) -> CInt
  catch_panic do
    process(data)
    0  # Success
  rescue e
    log_error(e)
    -1  # Error code
  end
end
```

---

## 3. Memory Ownership Across FFI Boundaries

### 3.1 Ownership Annotations

**DECIDED**: Four ownership annotations for FFI boundaries.

| Annotation | Meaning | Aria Responsibility |
|------------|---------|---------------------|
| `@owned` | Aria owns returned memory | Must free (or wrap in RAII) |
| `@borrowed` | Foreign owns memory | Must not free |
| `@transfer` | Ownership moves to foreign | Must not use after call |
| `@owned(free_with: fn)` | Owned with specific cleanup | Must call specified function |

```aria
extern C
  # Aria owns - must free with free()
  @owned
  fn malloc(size: USize) -> CVoidPtr

  # C owns - don't free
  @borrowed
  fn getenv(name: CString) -> CString

  # Ownership transfers to C
  fn free(@transfer ptr: CVoidPtr)

  # Owned with specific cleanup
  @owned(free_with: sqlite3_free)
  fn sqlite3_mprintf(fmt: CString, ...) -> CString
end
```

### 3.2 Automatic RAII Wrapper Generation

**DECIDED**: Compiler generates RAII wrappers for common patterns.

```aria
# Given C API:
extern C from "sqlite3.h"

# Aria auto-generates:
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

  fn finalize
    C.sqlite3_close(@handle) unless @handle.null?
  end
end
```

### 3.3 Pattern Recognition for Wrapper Generation

| C Pattern | Aria Wrapper Pattern |
|-----------|---------------------|
| `T* xxx_create()` / `xxx_destroy(T*)` | RAII class with finalizer |
| `int xxx_init(T**)` / `xxx_deinit(T*)` | `fn new() -> Result[T, E]` |
| `int xxx_open(...)` / `xxx_close(...)` | RAII class with open/close |
| `T* xxx_get_ref()` | `fn get_ref() -> &T` (borrowed) |

### 3.4 Lifetime Tracking

**DECIDED**: Borrowed references from FFI have explicit lifetime bounds.

```aria
extern C
  @borrowed
  fn get_static_string() -> CString  # 'static lifetime inferred

  @borrowed(lifetime: db)
  fn get_error(db: CPtr[Database]) -> CString  # Tied to db lifetime
end

# Usage
fn safe_error_handling(db: Database)
  error = db.get_error()  # Lifetime tied to db
  println(error)
  # error becomes invalid when db is dropped
end
```

---

## 4. Python Interoperability

### 4.1 Python Import Syntax

**DECIDED**: Use `extern Python from` for Python module imports.

```aria
# Import Python modules
extern Python from numpy as np
extern Python from pandas as pd
extern Python from torch
extern Python from my_module.utils as utils
```

### 4.2 GIL Management Strategy

**DECIDED**: Automatic GIL management by default with manual override.

| Scenario | GIL State | Mode |
|----------|-----------|------|
| Accessing Python objects | Held | Automatic |
| Pure Aria computation | Released | Automatic |
| Parallel Aria code | Released | Automatic |
| Python callback execution | Held | Automatic |
| Explicit control needed | Configurable | Manual |

```aria
# Automatic mode (default) - recommended
fn analyze_data(data: PyArray[Float64]) -> Float64
  # GIL acquired automatically when accessing np
  mean = np.mean(data)

  # GIL released for pure Aria computation
  result = mean * 2.0 + compute_offset()

  result
end

# Explicit GIL control (advanced)
fn parallel_compute(py_data: PyObject) -> Array[Float]
  # Extract data while holding GIL
  data = Python.with_gil |py|
    py_data.to_array(Float)
  end

  # Parallel compute without GIL
  data.parallel_map |x| x * 2.0 end
end

# Release GIL for long computation
fn long_computation(data: PyArray[Float64]) -> Float64
  Python.without_gil do
    heavy_aria_computation(data.to_aria_view)
  end
end
```

### 4.3 Python Type Conversion

| Aria Type | Python Type | Strategy | Copy? |
|-----------|-------------|----------|-------|
| `Int` | `int` | Direct | No |
| `Float64` | `float` | Direct | No |
| `Bool` | `bool` | Direct | No |
| `String` | `str` | Convert | Yes |
| `Array[Float64]` | `numpy.ndarray` | View | No |
| `Array[T]` | `list` | Convert | Yes |
| `Option[T]` | `T \| None` | Mapped | No |
| `Result[T, E]` | value/exception | Mapped | No |
| `Dict[K, V]` | `dict` | Convert | Yes |
| `struct` | class instance | Convert | Yes |

### 4.4 Zero-Copy Array Interop

**DECIDED**: Use `PyArrayView` for zero-copy access.

```aria
# Zero-copy view (large data)
fn process_view(data: PyArrayView[Float64]) -> Float64
  # data is a view into Python memory
  # Compiler enforces: cannot outlive Python object
  data.sum
end

# Safe copy (data must persist)
fn store_data(data: PyArray[Float64]) -> Array[Float64]
  data.to_aria_array  # Explicit copy
end
```

### 4.5 Exception Handling

**DECIDED**: Python exceptions become `Result`, Aria errors become Python exceptions.

```aria
# Python -> Aria: exceptions become Results
fn safe_import(module: PyModule) -> Result[PyObject, PyError]
  try_py do
    module.call("risky_function", [1, 2, 3])
  end
end

# Aria -> Python: errors become exceptions
@pyfunction
fn aria_divide(x: Int, y: Int) -> Int raises AriaError
  if y == 0
    raise AriaError.new("Division by zero")
  end
  x / y
end
# In Python: raises AriaError as Python exception
```

### 4.6 Exporting Aria to Python

**DECIDED**: Use `@pyfunction`, `@pyclass`, `@pymodule` annotations.

```aria
@pymodule(name: "aria_math")
module AriaMath
  @pyclass
  class Vector
    x: Float64
    y: Float64

    @pymethod
    fn magnitude(self) -> Float64
      (self.x * self.x + self.y * self.y).sqrt
    end
  end

  @pyfunction
  fn dot(a: Vector, b: Vector) -> Float64
    a.x * b.x + a.y * b.y
  end
end

# Python usage:
# import aria_math
# v = aria_math.Vector(3.0, 4.0)
# v.magnitude()  # Returns 5.0
```

---

## 5. WASM Import/Export Syntax

### 5.1 WASM Module Export

**DECIDED**: Use `@wasm_export` annotation for functions and `@wasm_module` for modules.

```aria
@wasm_module(name: "aria_math", version: "1.0.0")
module Math
  struct Point
    x: Float64
    y: Float64
  end

  @wasm_export
  fn distance(a: Point, b: Point) -> Float64
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt
  end

  @wasm_export
  fn scale(p: Point, factor: Float64) -> Point
    Point.new(x: p.x * factor, y: p.y * factor)
  end
end
```

### 5.2 WIT Generation

**DECIDED**: Compiler auto-generates WIT from Aria module definitions.

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

### 5.3 WASM Component Import

**DECIDED**: Use `extern wasm from` for importing WASM components.

```aria
# Import external WASM component
extern wasm from "crypto.wasm" as crypto
extern wasm from "logging.wasm" as log

fn secure_operation(data: Bytes) -> Bytes
  log.info("Starting secure operation")
  result = crypto.sha256(data)
  log.info("Operation complete")
  result
end
```

### 5.4 WASM Type Mapping

| Aria Type | WIT Type | WASM Core |
|-----------|----------|-----------|
| `Int32` | `s32` | `i32` |
| `Int64` | `s64` | `i64` |
| `UInt32` | `u32` | `i32` |
| `UInt64` | `u64` | `i64` |
| `Float32` | `float32` | `f32` |
| `Float64` | `float64` | `f64` |
| `Bool` | `bool` | `i32` |
| `String` | `string` | Linear memory + ptr/len |
| `Array[T]` | `list<T>` | Linear memory + ptr/len |
| `Option[T]` | `option<T>` | Variant encoding |
| `Result[T, E]` | `result<T, E>` | Variant encoding |
| `struct` | `record` | Sequential fields |
| `enum` | `variant` | Tagged union |
| `Bytes` | `list<u8>` | Linear memory |

### 5.5 WASI Integration

**DECIDED**: WASI 0.2 as initial target, with clear path to 0.3 async.

```aria
# WASI filesystem access
extern wasi from "wasi:filesystem/types@0.2.0" as fs

fn read_config() -> Result[String, WasiError]
  handle = fs.open("config.json", fs.OpenFlags.read_only)?
  content = fs.read(handle)?
  fs.close(handle)
  Ok(String.from_utf8(content))
end

# Future: WASI 0.3 async
@wasm_async  # Opt-in for async WASM (WASI 0.3)
fn fetch_data(url: String) async -> Result[Bytes, WasiError]
  wasi.http.fetch(url).await
end
```

### 5.6 Memory Management in WASM

**DECIDED**: Explicit allocator control for WASM memory.

```aria
@wasm_module
module WasmLib
  # Export allocator for host to call
  @wasm_export(name: "alloc")
  fn allocate(size: USize) -> WasmPtr
    Wasm.alloc(size)
  end

  @wasm_export(name: "dealloc")
  fn deallocate(ptr: WasmPtr, size: USize)
    Wasm.dealloc(ptr, size)
  end

  # Use standard Aria memory internally
  @wasm_export
  fn process(data: Array[UInt8]) -> Array[UInt8]
    data.map |b| b ^ 0xFF end
  end
end
```

---

## 6. Unified FFI Architecture

### 6.1 The `extern` Keyword Family

| Syntax | Purpose | Backend |
|--------|---------|---------|
| `extern C from "header.h"` | C header import | libclang |
| `extern C from "header.h" as name` | Namespaced C import | libclang |
| `extern "C" fn name(...)` | C ABI function declaration | - |
| `extern Python from module` | Python module import | cpython |
| `extern wasm from "file.wasm"` | WASM component import | wasmtime |
| `extern wasi from "interface"` | WASI interface import | wasmtime |

### 6.2 Safety Guarantee Levels

| Level | Marker | Memory Safety | Type Safety | Panic Safety |
|-------|--------|---------------|-------------|--------------|
| Safe | (default) | RAII wrappers | Full checking | catch_panic |
| Unsafe | `unsafe` | Manual | Full checking | Developer responsibility |
| Raw | `@raw_ffi` | None | None | None |

### 6.3 Diagnostic Messages

**DECIDED**: Clear compiler messages for FFI issues.

```
error[E0501]: ownership violation at FFI boundary
  --> src/lib.aria:15:5
   |
15 |   C.free(ptr)
   |   ^^^^^^^^^^^ pointer used after transfer
   |
note: ownership transferred here
  --> src/lib.aria:14:3
   |
14 |   data = process(ptr)  # ptr consumed
   |          ^^^^^^^^^^^^

help: if C.free should receive a copy, use `ptr.clone()`
```

---

## 7. Implementation Requirements

### 7.1 Compiler Requirements

| Component | Requirement |
|-----------|-------------|
| C Parser | Bundle libclang 17+ |
| Python | Link against python3-embed |
| WASM | Integrate wasmtime for component model |
| Cache | Persistent parsed header cache |

### 7.2 Runtime Requirements

| Feature | Requirement |
|---------|-------------|
| C FFI | Direct calls via generated trampolines |
| Python FFI | cpython runtime linkage |
| WASM FFI | wasmtime runtime |
| GIL Management | Automatic tracking state machine |

### 7.3 Standard Library FFI Wrappers

**DECIDED**: Provide safe wrappers for common C libraries.

| Library | Aria Module | Status |
|---------|-------------|--------|
| libc | `std.c` | Core |
| SQLite | `std.sqlite` | Standard |
| OpenSSL | `std.crypto` | Standard |
| zlib | `std.compress` | Standard |
| POSIX | `std.posix` | Unix only |

---

## 8. Migration and Compatibility

### 8.1 C Library Compatibility Matrix

| C Feature | Support Level | Notes |
|-----------|---------------|-------|
| C89 | Full | All features |
| C99 | Full | VLAs via heap allocation |
| C11 | Full | Atomics, generics |
| C17 | Full | Bug fixes only |
| C23 | Partial | New features incremental |
| GCC Extensions | Common | __attribute__, __builtin_* |
| MSVC Extensions | Limited | __declspec, SAL annotations |

### 8.2 Python Version Support

| Python Version | Support |
|----------------|---------|
| 3.9 | Minimum supported |
| 3.10 | Full support |
| 3.11 | Full support |
| 3.12 | Full support |
| 3.13+ | Best effort |

### 8.3 WASM Standards Support

| Standard | Support |
|----------|---------|
| Core WASM 1.0 | Full |
| Core WASM 2.0 | Full |
| Component Model | Full |
| WASI 0.2 | Full |
| WASI 0.3 | Planned |

---

## 9. Complete Examples

### 9.1 Full C Integration Example

```aria
# Complete SQLite wrapper demonstration
extern C from "sqlite3.h"

class Database
  @handle: CPtr[sqlite3]

  fn open(path: String) -> Result[Database, DbError]
    handle: CPtr[sqlite3] = null
    result = C.sqlite3_open(path.to_c_string, &handle)

    if result == C.SQLITE_OK
      Ok(Database.new(handle: handle))
    else
      Err(DbError.new(code: result, message: C.sqlite3_errmsg(handle)))
    end
  end

  fn execute(sql: String) -> Result[Unit, DbError]
    err_msg: CPtr[CChar] = null
    result = C.sqlite3_exec(@handle, sql.to_c_string, null, null, &err_msg)

    if result == C.SQLITE_OK
      Ok(())
    else
      error = DbError.new(code: result, message: CString.from_ptr(err_msg))
      C.sqlite3_free(err_msg)
      Err(error)
    end
  end

  fn finalize
    C.sqlite3_close(@handle) unless @handle.null?
  end
end

fn main
  db = Database.open(":memory:")!
  db.execute("CREATE TABLE users (id INTEGER, name TEXT)")!
  db.execute("INSERT INTO users VALUES (1, 'Alice')")!
  println("Database operations complete")
end
```

### 9.2 Full Python Integration Example

```aria
# Complete NumPy integration demonstration
extern Python from numpy as np
extern Python from scipy.optimize as opt

fn optimize_model(training_data: Array[Float64]) -> Result[Float64, PyError]
  # Convert to NumPy (zero-copy when possible)
  np_data = training_data.to_numpy

  # Define objective function
  objective = |params: PyArray[Float64]|
    predictions = np.dot(np_data, params)
    error = np.mean(np.square(predictions - np_data))
    error.to_float
  end

  # Run optimizer
  initial = np.zeros(training_data.len)
  result = try_py do
    opt.minimize(objective, initial, method: "BFGS")
  end?

  Ok(result.fun.to_float)
end

@pymodule(name: "aria_ml")
module AriaML
  @pyfunction
  fn fast_matrix_multiply(a: PyArray[Float64], b: PyArray[Float64]) -> PyArray[Float64]
    # Pure Aria computation - GIL released automatically
    Python.without_gil do
      result = Array.new(a.rows * b.cols, 0.0)
      for i in 0...a.rows
        for j in 0...b.cols
          for k in 0...a.cols
            result[i * b.cols + j] += a[i, k] * b[k, j]
          end
        end
      end
      result.to_numpy
    end
  end
end
```

### 9.3 Full WASM Integration Example

```aria
# Complete WASM component demonstration
@wasm_module(name: "aria_crypto", version: "1.0.0")
module AriaCrypto
  # Import external crypto primitives
  extern wasm from "ring-crypto.wasm" as ring

  struct HashResult
    algorithm: String
    digest: Bytes
    length: UInt32
  end

  @wasm_export
  fn sha256(data: Bytes) -> HashResult
    digest = ring.sha256(data)
    HashResult.new(
      algorithm: "SHA-256",
      digest: digest,
      length: 32
    )
  end

  @wasm_export
  fn sha512(data: Bytes) -> HashResult
    digest = ring.sha512(data)
    HashResult.new(
      algorithm: "SHA-512",
      digest: digest,
      length: 64
    )
  end

  @wasm_export
  fn verify_signature(
    message: Bytes,
    signature: Bytes,
    public_key: Bytes
  ) -> Bool
    ring.ed25519_verify(message, signature, public_key)
  end
end

# Generated WIT interface:
# package aria:crypto@1.0.0;
#
# interface hashing {
#   record hash-result {
#     algorithm: string,
#     digest: list<u8>,
#     length: u32,
#   }
#
#   sha256: func(data: list<u8>) -> hash-result;
#   sha512: func(data: list<u8>) -> hash-result;
#   verify-signature: func(message: list<u8>, signature: list<u8>, public-key: list<u8>) -> bool;
# }
#
# world aria-crypto {
#   export hashing;
# }
```

---

## 10. Decision Summary

| Decision | Choice | Rationale |
|----------|--------|-----------|
| C Import Style | `extern C from` | Zig-proven, zero tooling |
| C Parser | libclang (bundled) | Full C11, reliable |
| Safety Default | Safe wrappers | Prevent common FFI bugs |
| Unsafe Access | Explicit `unsafe` blocks | Clear intent, auditable |
| Ownership | Four annotations | Complete coverage |
| Python GIL | Automatic + manual override | Easy default, escape hatch |
| Python Types | Zero-copy for arrays | Performance critical |
| WASM Target | Component Model | Future-proof, portable |
| WASI Version | 0.2 now, 0.3 roadmap | Stable + async future |
| WIT | Auto-generated | Single source of truth |

---

## 11. Open Decisions (Deferred)

| Question | Deferred To | Rationale |
|----------|-------------|-----------|
| Full C++ interop | ARIA-PD-011 | Scope management |
| Rust FFI | ARIA-PD-012 | After C stabilizes |
| Swift interop | ARIA-PD-013 | Apple platform needs |
| JVM interop | Future | Lower priority |

---

*Product Design Document approved by RELAY, January 2026*
*Based on BRIDGE research document ARIA-M09-03*
