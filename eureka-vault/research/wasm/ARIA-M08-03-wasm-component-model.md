# ARIA-M08-03: WebAssembly Component Model Research

**Task ID**: ARIA-M08-03
**Status**: Completed
**Date**: 2026-01-14
**Focus**: Study WASM Component Model, WIT, and interface types

---

## Executive Summary

The WebAssembly Component Model is a major evolution enabling language-agnostic component composition. This research analyzes WIT interface types, WASI 0.2/0.3, and cross-language interop for Aria's WASM strategy.

---

## 1. Overview

### 1.1 The Component Model Vision

```
Before Component Model:
  WASM Module A ←──→ [Custom glue code] ←──→ WASM Module B
  (Only integers and linear memory)

With Component Model:
  WASM Component A ←──→ [Standard interfaces (WIT)] ←──→ WASM Component B
  (Rich types: strings, records, lists, resources)
```

### 1.2 Key Components

| Concept | Description |
|---------|-------------|
| **Component** | Self-contained WASM unit with typed interfaces |
| **WIT** | WebAssembly Interface Types (IDL) |
| **WASI** | Standard system interfaces |
| **World** | Set of imports/exports a component uses |

---

## 2. WIT (WebAssembly Interface Types)

### 2.1 What is WIT?

WIT is an Interface Definition Language for defining component interfaces:
- Declarative (no logic)
- Language-neutral
- Defines contracts between components

### 2.2 Syntax Examples

```wit
// types.wit
package example:types@1.0.0;

// Type definitions
record user {
    id: u64,
    name: string,
    email: string,
}

variant result {
    ok(user),
    err(string),
}

// Interface definition
interface user-service {
    get-user: func(id: u64) -> result;
    create-user: func(name: string, email: string) -> result;
    list-users: func() -> list<user>;
}
```

### 2.3 Supported Types

| Category | Types |
|----------|-------|
| Primitives | `bool`, `s8`-`s64`, `u8`-`u64`, `f32`, `f64`, `char` |
| Compound | `string`, `list<T>`, `tuple<...>`, `option<T>`, `result<T,E>` |
| Named | `record`, `variant`, `enum`, `flags` |
| Resources | `resource` (with own/borrow) |

---

## 3. WASI (WebAssembly System Interface)

### 3.1 Current Status

| Version | Status | Release |
|---------|--------|---------|
| WASI 0.2.0 | **Stable** | January 2024 |
| WASI 0.3 | Expected | First half 2025 |
| WASI 1.0 | Future | TBD |

### 3.2 WASI 0.2 Interfaces

```wit
// Available in WASI 0.2
wasi:cli/stdin
wasi:cli/stdout
wasi:cli/stderr
wasi:filesystem/types
wasi:filesystem/preopens
wasi:http/types
wasi:http/outgoing-handler
wasi:random/random
wasi:clocks/monotonic-clock
wasi:clocks/wall-clock
```

### 3.3 WASI 0.3 (Preview 3)

Expected features:
- **Native async I/O** via Component Model
- Upgraded APIs using async primitives
- Better streaming support

---

## 4. Worlds

### 4.1 What is a World?

A **world** defines the complete interface contract:
- What the component imports (needs from host)
- What the component exports (provides to others)

### 4.2 Example World

```wit
// my-world.wit
package example:app@1.0.0;

world my-app {
    // Imports (dependencies)
    import wasi:filesystem/types@0.2.0;
    import wasi:http/outgoing-handler@0.2.0;
    import example:types/user-service;

    // Exports (capabilities)
    export wasi:http/incoming-handler@0.2.0;
}
```

---

## 5. Runtime Support

### 5.1 Current Runtimes

| Runtime | Component Support | Notes |
|---------|------------------|-------|
| Wasmtime | Full | Reference implementation |
| WAMR | Partial | Embedded focus |
| WasmEdge | Good | Cloud-native |
| wazero | Growing | Pure Go |
| Wasmer | Growing | Multi-language |
| jco | Good | JavaScript host |

### 5.2 Wasmtime Example

```rust
use wasmtime::component::*;

// Load and instantiate component
let engine = Engine::default();
let component = Component::from_file(&engine, "my-component.wasm")?;
let linker = Linker::new(&engine);

// Bind WASI interfaces
wasmtime_wasi::add_to_linker(&mut linker)?;

// Instantiate
let instance = linker.instantiate(&mut store, &component)?;

// Call exported function
let result = instance.call("process", &mut store, &input)?;
```

---

## 6. Component Composition

### 6.1 Why It Matters

```
Traditional linking:
  Rust app → Rust library (same language)

Component composition:
  Rust component → WIT interface ← Python component
  (Language-agnostic)
```

### 6.2 Composition Example

```bash
# Compose components
wasm-tools compose \
    --definitions ./wit \
    --component ./core.wasm \
    --component ./plugin.wasm \
    -o ./composed.wasm
```

---

## 7. Recommendations for Aria

### 7.1 WIT Generation

```aria
# Aria could generate WIT from types

@wasm_export
struct User {
    id: Int,
    name: String,
    email: String,
}

@wasm_export
trait UserService {
    fn get_user(id: Int) -> Result[User, String]
    fn create_user(name: String, email: String) -> Result[User, String]
    fn list_users() -> Array[User]
}

# Generates:
# record user { id: u64, name: string, email: string }
# interface user-service { ... }
```

### 7.2 World Definition

```aria
# Aria world declaration
@wasm_world
world MyApp {
    # Imports
    import wasi.filesystem
    import wasi.http

    # Exports
    export http_handler: HTTPHandler
}

@wasm_component(world: MyApp)
module MyAppComponent {
    fn http_handler(request: Request) -> Response
        # Implementation
    end
}
```

### 7.3 Type Mapping

| Aria Type | WIT Type |
|-----------|----------|
| `Int` | `s64` |
| `UInt` | `u64` |
| `Float` | `f64` |
| `Bool` | `bool` |
| `String` | `string` |
| `Array[T]` | `list<T>` |
| `Option[T]` | `option<T>` |
| `Result[T, E]` | `result<T, E>` |
| `struct` | `record` |
| `enum` | `variant` |

### 7.4 Resource Handling

```aria
# Aria resources for WASM
@wasm_resource
struct FileHandle {
    # Opaque handle to host resource
}

impl FileHandle {
    @wasm_constructor
    fn open(path: String) -> Result[FileHandle, Error]

    @wasm_method
    fn read(self, bytes: Int) -> Array[UInt8]

    @wasm_method
    fn close(self)
}
```

### 7.5 Async Support (WASI 0.3)

```aria
# Prepare for WASI 0.3 async
@wasm_async
fn fetch_data(url: String) -> {Async, IO} Response
    # Will compile to Component Model async
    HTTP.get(url)
end
```

### 7.6 Build Integration

```bash
# Aria WASM component build
aria build \
    --target wasm-component \
    --world wasi:http/proxy@0.2.0 \
    --wit-dir ./wit \
    -o my-component.wasm

# Outputs:
# - my-component.wasm (component binary)
# - my-component.wit (generated interface)
```

---

## 8. Migration Path

### 8.1 Current (Core WASM)

```aria
# Today: compile to core WASM module
aria build --target wasm32
```

### 8.2 Near-term (WASI 0.2)

```aria
# Near-term: WASI 0.2 components
aria build --target wasm-component --wasi 0.2
```

### 8.3 Future (WASI 0.3+)

```aria
# Future: full async component model
aria build --target wasm-component --wasi 0.3
```

---

## 9. Key Resources

1. [Component Model Introduction](https://component-model.bytecodealliance.org/)
2. [WASI.dev](https://wasi.dev/)
3. [WASI Interfaces](https://wasi.dev/interfaces)
4. [Component Model Status 2025](https://eunomia.dev/blog/2025/02/16/wasi-and-the-webassembly-component-model-current-status/)
5. [Fermyon: WASI and Component Model](https://www.fermyon.com/blog/webassembly-wasi-and-the-component-model)

---

## 10. Open Questions

1. How do Aria's effects map to Component Model async?
2. Should Aria generate WIT automatically or require explicit definition?
3. What's the migration strategy for existing core WASM code?
4. How do we handle component versioning?
