# ARIA-M09-01: Zig's @cImport Analysis

**Task ID**: ARIA-M09-01
**Status**: Completed
**Date**: 2026-01-14
**Focus**: Analyze Zig's compile-time C parsing

---

## Executive Summary

Zig's `@cImport` provides seamless C interoperability by parsing C headers at compile time, eliminating the need for binding generators. This analysis examines the implementation, type mapping rules, and macro handling to inform Aria's C interop design.

---

## 1. Overview of Zig's C Interoperability

### 1.1 The Problem with Traditional FFI

| Approach | Tooling Required | Maintenance Burden |
|----------|------------------|-------------------|
| Manual bindings | Manual declaration | High (sync required) |
| Binding generators | bindgen, SWIG, etc. | Medium (regenerate) |
| **Zig's approach** | None | **Low (automatic)** |

### 1.2 Zig's Solution: @cImport

```zig
const c = @cImport({
    @cInclude("stdio.h");
    @cInclude("sqlite3.h");
});

pub fn main() void {
    _ = c.printf("Hello from C!\n");
    var db: ?*c.sqlite3 = null;
    _ = c.sqlite3_open("test.db", &db);
}
```

**Key Insight**: C declarations become Zig types automatically.

---

## 2. How @cImport Works

### 2.1 Architecture

```
┌─────────────────────────────────────────────────────┐
│                    @cImport                          │
├─────────────────────────────────────────────────────┤
│  ┌───────────────┐    ┌──────────────────────────┐  │
│  │ @cInclude     │───►│ C Preprocessor           │  │
│  │ @cDefine      │    │ (libclang or custom)     │  │
│  │ @cUndef       │    └───────────┬──────────────┘  │
│  └───────────────┘                │                  │
│                                   ▼                  │
│                    ┌──────────────────────────────┐  │
│                    │ translate-c                  │  │
│                    │ (C AST → Zig AST)            │  │
│                    └───────────┬──────────────────┘  │
│                                │                     │
│                                ▼                     │
│                    ┌──────────────────────────────┐  │
│                    │ Zig Type System              │  │
│                    │ (structs, functions, etc.)   │  │
│                    └──────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

### 2.2 The @cImport Builtin

**Syntax**:
```zig
const imported = @cImport(expression);
```

**Expression Contents**:
- `@cInclude(path)` - Include C header
- `@cDefine(name, value)` - Define macro
- `@cUndef(name)` - Undefine macro

**Returns**: A struct type containing all C declarations.

### 2.3 Translate-C Under the Hood

`@cImport` is effectively shorthand for:
1. Run C preprocessor on included files
2. Parse C to AST (using libclang or Zig's parser)
3. Translate C AST to Zig AST
4. Import resulting Zig module

```bash
# Command-line equivalent
zig translate-c header.h > header.zig
zig build-exe main.zig
```

---

## 3. Type Mapping Rules

### 3.1 Basic Types

| C Type | Zig Type |
|--------|----------|
| `char` | `u8` or `i8` (platform-dependent) |
| `short` | `c_short` |
| `int` | `c_int` |
| `long` | `c_long` |
| `long long` | `c_longlong` |
| `float` | `f32` |
| `double` | `f64` |
| `void` | `void` or `anyopaque` |
| `_Bool` | `bool` |
| `size_t` | `usize` |

### 3.2 Pointer Types

| C Type | Zig Type |
|--------|----------|
| `T*` | `[*c]T` (C pointer) or `*T` |
| `const T*` | `[*c]const T` |
| `void*` | `*anyopaque` |
| `T**` | `[*c][*c]T` |
| NULL | `null` |

**Important**: `[*c]T` is Zig's "C pointer" type:
- Allows null
- Allows pointer arithmetic
- Interops with C seamlessly

### 3.3 Struct Types

```c
// C
struct Point {
    int x;
    int y;
};
```

```zig
// Zig (translated)
const Point = extern struct {
    x: c_int,
    y: c_int,
};
```

**Properties**:
- `extern struct` ensures C-compatible layout
- Field order preserved
- Padding matches C ABI

### 3.4 Union Types

```c
// C
union Value {
    int i;
    float f;
};
```

```zig
// Zig (translated)
const Value = extern union {
    i: c_int,
    f: f32,
};
```

### 3.5 Enum Types

```c
// C
enum Color { RED, GREEN, BLUE };
```

```zig
// Zig (translated)
const Color = enum(c_int) {
    RED = 0,
    GREEN = 1,
    BLUE = 2,
};
```

### 3.6 Function Types

```c
// C
int add(int a, int b);
typedef void (*callback_t)(int);
```

```zig
// Zig (translated)
extern fn add(a: c_int, b: c_int) c_int;
const callback_t = ?*const fn(c_int) callconv(.C) void;
```

---

## 4. Macro Handling

### 4.1 Object-Like Macros

```c
#define MAX_SIZE 1024
#define PI 3.14159
#define NULL ((void*)0)
```

```zig
// Zig (translated)
pub const MAX_SIZE = 1024;
pub const PI = 3.14159;
pub const NULL = null;
```

### 4.2 Function-Like Macros

```c
#define MAX(a, b) ((a) > (b) ? (a) : (b))
#define ARRAY_SIZE(arr) (sizeof(arr) / sizeof((arr)[0]))
```

```zig
// Zig (translated as inline functions)
pub inline fn MAX(a: anytype, b: anytype) @TypeOf(a, b) {
    return if (a > b) a else b;
}

// Complex macros may not translate
// ARRAY_SIZE typically requires manual handling
```

### 4.3 Limitations

| Macro Type | Translation Support |
|------------|---------------------|
| Constants | Full |
| Simple function macros | Good |
| Complex macros | Limited |
| Stringification (#) | No |
| Token pasting (##) | Limited |
| Variadic macros | Partial |

---

## 5. Implementation Details

### 5.1 LibClang Dependency

Historically, Zig used libclang for C parsing:
- Full C11 support
- Accurate preprocessing
- Type information extraction

**Drawback**: Large dependency (~100MB+)

### 5.2 Moving to Build System

There's an accepted proposal to move `@cImport` to the build system:

```zig
// Future: explicit step in build.zig
const c_lib = b.addTranslateC(.{
    .source_file = .{ .path = "header.h" },
    .include_paths = &.{"/usr/include"},
});
```

**Benefits**:
- Remove libclang from compiler binary
- Explicit caching and invalidation
- Better build system integration

### 5.3 Single Instance Recommendation

```zig
// Recommended: single @cImport for entire project
const c = @cImport({
    @cInclude("header1.h");
    @cInclude("header2.h");
});

// NOT recommended: multiple @cImport (type incompatibility)
const c1 = @cImport(@cInclude("header1.h"));
const c2 = @cImport(@cInclude("header1.h"));
// c1.SomeType != c2.SomeType (different types!)
```

---

## 6. Comparison with Other Approaches

### 6.1 Rust (bindgen)

```rust
// rust-bindgen generates Rust code
// build.rs
bindgen::Builder::default()
    .header("wrapper.h")
    .generate()
    .write_to_file("bindings.rs");

// main.rs
include!("bindings.rs");
```

**Differences from Zig**:
- Separate build step
- Generated code checked in (often)
- More manual configuration
- Better macro handling options

### 6.2 Nim (importc)

```nim
proc printf(format: cstring): cint {.importc, varargs, header: "<stdio.h>".}
```

**Differences**:
- Manual declaration per function
- Pragma-based approach
- No automatic translation

### 6.3 D (extern(C))

```d
extern(C) int printf(const char* format, ...);
```

**Differences**:
- Manual declaration
- C ABI compatibility built into language
- No automatic import

---

## 7. Best Practices

### 7.1 Using @cImport Effectively

```zig
// 1. Single import point
const c = @cImport({
    // 2. Define platform macros if needed
    @cDefine("_GNU_SOURCE", {});

    // 3. Include in dependency order
    @cInclude("sys/types.h");
    @cInclude("my_library.h");
});

// 4. Create Zig-friendly wrappers
pub const Database = struct {
    handle: *c.sqlite3,

    pub fn open(path: [:0]const u8) !Database {
        var db: ?*c.sqlite3 = null;
        const result = c.sqlite3_open(path, &db);
        if (result != c.SQLITE_OK) return error.OpenFailed;
        return Database{ .handle = db.? };
    }

    pub fn close(self: *Database) void {
        _ = c.sqlite3_close(self.handle);
    }
};
```

### 7.2 Handling Translation Failures

```zig
// Some C code won't translate - provide manual declaration
extern fn problematic_c_function(arg: c_int) c_int;

// Or use @extern for linking
const problematic = @extern(*const fn(c_int) c_int, .{ .name = "problematic_c_function" });
```

---

## 8. Recommendations for Aria

### 8.1 Proposed Syntax

```aria
# Direct C header import (Zig-style)
extern C from "sqlite3.h"
extern C from "openssl/ssl.h" as ssl

fn use_sqlite
  # Type-safe, with Aria wrappers
  db = C.sqlite3_open("test.db")
  defer C.sqlite3_close(db)

  # Aria automatically wraps C pointers safely
  result = C.sqlite3_exec(db, "SELECT * FROM users")
end
```

### 8.2 Type Mapping Strategy

| C Type | Aria Type | Notes |
|--------|-----------|-------|
| `int`, `long`, etc. | `CInt`, `CLong` | Platform-specific sizes |
| `T*` | `CPtr[T]` | Nullable C pointer |
| `const T*` | `CPtr[T].const` | Const pointer |
| `char*` | `CString` | Special string handling |
| `void*` | `CVoidPtr` | Opaque pointer |
| `struct T` | `extern struct T` | C-compatible layout |

### 8.3 Safety Wrapper Generation

```aria
# Aria generates safe wrappers automatically
extern C from "sqlite3.h"

# Auto-generated wrapper (internal):
class SqliteDatabase
  @handle: CPtr[sqlite3]

  fn open(path: String) -> Result[SqliteDatabase, SqliteError]
    handle = C.sqlite3_open(path.to_c_string)
    if handle.null? then
      Err(SqliteError.OpenFailed)
    else
      Ok(SqliteDatabase.new(handle: handle))
    end
  end

  fn close
    C.sqlite3_close(@handle) unless @handle.null?
  end
end
```

### 8.4 Implementation Options

| Option | Pros | Cons |
|--------|------|------|
| **Use libclang** | Full C support | Large dependency |
| **Custom parser** | Small, fast | Limited C support |
| **Build step (like Zig future)** | Flexible | Requires build system |
| **Hybrid** | Balance | Complexity |

**Recommendation**: Start with libclang, consider custom parser later.

### 8.5 Macro Handling

```aria
# Simple macros: automatic
extern C from "limits.h"
max_int = C.INT_MAX  # Works

# Complex macros: manual wrapper
@c_macro
fn offsetof(type: Type, field: Symbol) -> USize
  __builtin_offsetof(type, field)
end
```

---

## 9. Key Resources

1. **Zig Documentation** - ziglang.org/documentation
2. **Zig Source** - github.com/ziglang/zig (translate-c)
3. **libclang** - clang.llvm.org/doxygen
4. **Zig C Interop Guide** - zighelp.org/chapter-4
5. **Issue #20630** - Move @cImport to build system proposal

---

## 10. Open Questions

1. How do we handle platform-specific headers safely?
2. What's the caching strategy for translated headers?
3. How do we handle C macros that don't translate?
4. Should unsafe C code be explicitly marked?
5. How do ownership rules apply to C pointers?

---

## Appendix: Complete @cImport Example

```zig
const std = @import("std");

// Single C import point for SQLite
const c = @cImport({
    @cInclude("sqlite3.h");
});

pub const Database = struct {
    db: ?*c.sqlite3,

    pub fn init() Database {
        return .{ .db = null };
    }

    pub fn open(self: *Database, path: [:0]const u8) !void {
        const result = c.sqlite3_open(path.ptr, &self.db);
        if (result != c.SQLITE_OK) {
            return error.SqliteOpenError;
        }
    }

    pub fn exec(self: *Database, sql: [:0]const u8) !void {
        var err_msg: [*c]u8 = null;
        const result = c.sqlite3_exec(
            self.db,
            sql.ptr,
            null,
            null,
            &err_msg,
        );
        if (result != c.SQLITE_OK) {
            if (err_msg) |msg| {
                std.debug.print("SQL error: {s}\n", .{msg});
                c.sqlite3_free(msg);
            }
            return error.SqliteExecError;
        }
    }

    pub fn close(self: *Database) void {
        if (self.db) |db| {
            _ = c.sqlite3_close(db);
            self.db = null;
        }
    }
};

pub fn main() !void {
    var db = Database.init();
    try db.open("test.db");
    defer db.close();

    try db.exec("CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT)");
    try db.exec("INSERT INTO users (name) VALUES ('Alice')");

    std.debug.print("Database operations completed successfully!\n", .{});
}
```
