# ARIA-M05-01: WebAssembly Compilation Strategy

**Task ID**: ARIA-M05-01
**Status**: Completed
**Date**: 2026-01-15
**Focus**: WASM compilation strategy for browser and WASI runtime targets
**Research Agent**: FLUX

---

## Executive Summary

This document presents Aria's comprehensive WebAssembly compilation strategy, addressing target selection, memory management in WASM's linear memory model, JS interoperability, and debug tooling. The strategy is designed to support both browser deployments and server-side WASI runtimes while maintaining Aria's core design principles: ownership inference, contracts, and developer experience.

**Key Recommendations**:
1. **Target WASM 3.0** features including GC proposal for managed objects
2. **Dual-path memory management**: Linear memory for owned values, WASM GC for shared (@shared) types
3. **Component Model adoption** for cross-language interoperability and future-proofing
4. **DWARF-based debugging** with source map fallback for legacy environments

---

## 1. WASM Feature Matrix and Target Selection

### 1.1 Feature Assessment (2025-2026)

| Feature | Browser Support | WASI Support | Aria Relevance | Recommendation |
|---------|-----------------|--------------|----------------|----------------|
| **Core WASM 1.0** | Universal | Universal | Foundation | Target |
| **WASM 3.0 (Sep 2025)** | Chrome/Firefox/Safari | Wasmtime 22+ | New baseline | Target |
| **64-bit Memory** | WASM 3.0 | WASM 3.0 | Large data | Target |
| **GC Proposal** | WASM 3.0 | Wasmtime 21+ | @shared types | Target |
| **Threads** | Chrome/Firefox | Wasmtime | Concurrency | Optional |
| **SIMD** | Stable | Stable | Performance | Target |
| **Exception Handling** | Experimental | Partial | Error handling | Feature-detect |
| **Component Model** | jco (JS host) | Wasmtime/WasmEdge | Interop | Target for 0.2 |
| **Memory Control** | Proposal stage | Proposal | Future | Watch |

### 1.2 Target Profiles

```toml
# aria.toml - WASM target profiles

[profile.wasm-browser]
target = "wasm32-unknown-unknown"
features = ["gc", "simd", "bulk-memory"]
opt-level = "z"          # Aggressive size optimization
panic = "trap"           # Minimal panic handling
debug-info = "dwarf"     # DWARF embedded or split
component-model = false  # Core module for browser

[profile.wasm-component]
target = "wasm32-wasi"
wasi-version = "0.2"
features = ["gc", "simd", "threads"]
component-model = true   # Full component model
wit-world = "wasi:cli/command"

[profile.wasm-edge]
target = "wasm32-wasi"
wasi-version = "0.2"
features = ["gc"]
opt-level = "s"          # Size with some speed
component-model = true
wit-world = "wasi:http/proxy"
```

### 1.3 Feature Detection Strategy

```aria
# Compile-time feature detection
@wasm_feature_gate(:gc)
fn allocate_gc_object(data: T) -> GcRef[T]
  # Uses WASM GC struct allocation
  __wasm_gc_alloc(T, data)
end

@wasm_feature_gate(:simd)
fn vector_add(a: Array[Float], b: Array[Float]) -> Array[Float]
  # Uses SIMD instructions
  __wasm_simd_add(a, b)
end

# Runtime feature detection (rare cases)
fn best_implementation() -> fn(Array[Int]) -> Int
  if wasm_feature?(:simd)
    simd_sum
  else
    scalar_sum
  end
end
```

---

## 2. Memory Management in WASM Linear Memory

### 2.1 The Challenge: Aria Ownership in WASM

Aria's hybrid ownership model (from ARIA-M02-04) must be mapped to WASM's constraints:

| Aria Ownership | WASM Representation | Strategy |
|----------------|---------------------|----------|
| Owned values | Linear memory | Custom allocator |
| References (`ref`) | i32 pointers | Bounds-checked |
| Mutable refs (`mut ref`) | i32 pointers | Single-accessor enforced |
| Shared (`@shared`) | **WASM GC objects** | Host-managed lifetime |
| Weak (`@weak`) | WASM GC weak refs | GC handles cycles |

### 2.2 Linear Memory Layout

```
WASM Linear Memory Layout for Aria:
┌─────────────────────────────────────────────────┐
│ 0x0000-0x0FFF: Reserved (null checks)           │
├─────────────────────────────────────────────────┤
│ 0x1000-0x?????: Static data segment             │
│   - String literals                             │
│   - Constant arrays                             │
│   - Type metadata                               │
├─────────────────────────────────────────────────┤
│ Stack (grows down from __stack_base)            │
│   - Local variables                             │
│   - Function arguments                          │
│   - Temporary values                            │
├─────────────────────────────────────────────────┤
│ Heap (managed by Aria allocator)                │
│   - Dynamically allocated objects               │
│   - Array buffers                               │
│   - String buffers                              │
└─────────────────────────────────────────────────┘
```

### 2.3 Ownership-Aware Allocation

```aria
# Aria WASM allocator interface
module Aria.Wasm.Allocator

  # Compile-time ownership tracking
  @inline
  fn allocate[T](size: USize) -> own T*
    let ptr = __wasm_memory_alloc(size, align_of(T))
    # Ownership transfer to caller
    ptr as own T*
  end

  @inline
  fn deallocate[T](ptr: own T*)
    # Ownership consumed, memory freed
    __wasm_memory_free(ptr, size_of(T))
  end

  # Reference creation (no allocation)
  @inline
  fn borrow[T](ptr: own T*) -> ref T
    # Creates reference, ownership retained by original
    ptr as ref T
  end

  # For WASM environments without GC
  @wasm_no_gc
  fn manual_arc_increment(ptr: T*)
    # Manual reference counting fallback
    let rc_ptr = ptr - 8  # RC header
    __atomic_add_i32(rc_ptr, 1)
  end
end
```

### 2.4 WASM GC for @shared Types

When WASM GC is available, `@shared` types map directly:

```aria
# Aria source
@shared class Node
  value: Int
  @weak parent: Node?
  children: Array[Node]
end

# Compiles to WIT/WASM GC
# (wasm type)
# (rec
#   (type $Node (struct
#     (field $value i32)
#     (field $parent (ref null $Node))  ; weak via host
#     (field $children (ref $Array_Node))
#   ))
# )
```

**WASM GC Integration**:

```
Aria @shared Object → WASM GC struct
     ↓
Runtime manages:
  - Allocation (struct.new)
  - Reference counting → GC tracing
  - Cycle detection → automatic
  - Weak references → (ref null $T)
```

### 2.5 Memory Safety Guarantees

| Safety Property | Mechanism in WASM |
|-----------------|-------------------|
| Bounds checking | Linear memory bounds + explicit checks |
| Use-after-free | Ownership tracking at compile time |
| Double-free | Single ownership, compile-time |
| Data races | Single-threaded or `@shared` atomic |
| Null safety | `Option` type + explicit null refs |

---

## 3. Component Model vs Core WASM Decision

### 3.1 Comparison Matrix

| Aspect | Core WASM | Component Model |
|--------|-----------|-----------------|
| **Browser support** | Universal | Via jco (JS host) |
| **WASI runtime support** | Limited (Preview 1) | Full (Preview 2+) |
| **Type richness** | i32/i64/f32/f64 only | Strings, records, variants |
| **Interop complexity** | High (manual glue) | Low (auto-generated) |
| **Binary size** | Smaller | Slightly larger |
| **Composability** | None | First-class |
| **Future direction** | Legacy | Standard path |

### 3.2 Aria's Strategy: Dual Output

```
aria build --target wasm-browser
  → Core WASM module (.wasm) + JS glue (aria-bindgen)

aria build --target wasm-component
  → WASM Component (.wasm) + WIT definitions (.wit)
```

### 3.3 WIT Interface Generation

Aria types automatically generate WIT interfaces:

```aria
# Aria source
@wasm_export
struct User
  id: Int
  name: String
  email: String
end

@wasm_export
trait UserService
  fn get_user(id: Int) -> Result[User, String]
  fn create_user(name: String, email: String) -> Result[User, String]
  fn list_users() -> Array[User]
end
```

**Generated WIT**:

```wit
// aria-app.wit (auto-generated)
package aria:app@1.0.0;

record user {
    id: s64,
    name: string,
    email: string,
}

interface user-service {
    get-user: func(id: s64) -> result<user, string>;
    create-user: func(name: string, email: string) -> result<user, string>;
    list-users: func() -> list<user>;
}

world aria-app {
    export user-service;
}
```

### 3.4 WASI World Definitions

```aria
# Aria world declaration
@wasm_world
world AriaHttpHandler
  # Standard WASI imports
  import wasi.http.types
  import wasi.http.outgoing_handler
  import wasi.logging

  # Custom imports (from host)
  import config.get_setting as fn(key: String) -> String?

  # Exports (Aria implements)
  export wasi.http.incoming_handler
end

@wasm_component(world: AriaHttpHandler)
module MyHttpApp
  fn handle_request(request: IncomingRequest) -> Response
    let path = request.path
    match path
      "/" => Response.ok("Hello from Aria!")
      "/api/users" => handle_users_api(request)
      _ => Response.not_found()
    end
  end
end
```

---

## 4. JavaScript Interop Design

### 4.1 Interop Architecture (Browser Target)

```
┌─────────────────────────────────────────────────────────┐
│                    JavaScript Runtime                    │
│  ┌─────────────────────────────────────────────────┐    │
│  │              aria-runtime.js                     │    │
│  │  - Memory management helpers                     │    │
│  │  - Type conversion (string, array, objects)     │    │
│  │  - Event loop integration                        │    │
│  │  - DOM API wrappers                              │    │
│  └───────────────────────┬─────────────────────────┘    │
│                          │                               │
│  ┌───────────────────────▼─────────────────────────┐    │
│  │              my-app.wasm                         │    │
│  │  - Aria compiled code                            │    │
│  │  - aria-stdlib (WASM portion)                   │    │
│  │  - Application logic                             │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

### 4.2 Type Marshalling Rules

| Aria Type | JS Type | Direction | Copy/Reference |
|-----------|---------|-----------|----------------|
| `Int` (i64) | `BigInt` | Both | Value |
| `Int32` | `number` | Both | Value |
| `Float` | `number` | Both | Value |
| `Bool` | `boolean` | Both | Value |
| `String` | `string` | Both | Copy |
| `Array[UInt8]` | `Uint8Array` | Both | **Zero-copy view** |
| `Array[T]` | `Array` | Both | Copy |
| `Option[T]` | `T \| undefined` | Both | Wrapped |
| `Result[T,E]` | throw/return | Export | Exception |
| `JsValue` | `any` | Import | Opaque handle |

### 4.3 Zero-Copy Strategies

```aria
# Zero-copy view into WASM memory (read-only)
@wasm_export
fn get_binary_data() -> ArrayView[UInt8]
  let data = load_from_cache()
  data.as_view()  # JS sees Uint8Array view into WASM memory
end

# Zero-copy for TypedArrays from JS
@wasm_import(js: "processBinaryData")
extern fn process_binary(data: ArrayView[UInt8]) -> Int

# Explicit copy for safety
@wasm_export
fn get_data_copy() -> Array[UInt8]
  let data = load_from_cache()
  data.clone()  # Explicit copy to JS heap
end
```

### 4.4 DOM and Web API Access

```aria
# Import Web APIs
@wasm_import(web_sys: true)
module Web
  extern struct Document
  extern struct Element
  extern struct Event

  extern fn window() -> Window
  extern fn document() -> Document

  impl Document
    extern fn query_selector(self, selector: String) -> Element?
    extern fn create_element(self, tag: String) -> Element
  end

  impl Element
    extern fn set_inner_html(mut self, html: String)
    extern fn add_event_listener(self, event: String, handler: fn(Event))
    extern fn get_attribute(self, name: String) -> String?
    extern fn set_attribute(mut self, name: String, value: String)
  end
end

# Usage
fn setup_ui()
  let doc = Web.document()
  if let Some(app) = doc.query_selector("#app")
    app.set_inner_html("<h1>Hello from Aria!</h1>")
    app.add_event_listener("click") { |event|
      Web.console_log("Clicked!")
    }
  end
end
```

### 4.5 Async/Promise Integration

```aria
# JS Promise to Aria async
@wasm_import(js: "fetch")
extern fn js_fetch(url: String) -> Promise[Response]

# Aria async function using JS promise
async fn fetch_data(url: String) -> {Async, IO} Result[String, Error]
  let response = js_fetch(url).await?
  let text = response.text().await?
  Ok(text)
end

# Export async function as Promise to JS
@wasm_export
async fn process_url(url: String) -> String
  let data = fetch_data(url).await?
  process(data)
end
# JS sees: processUrl(url: string): Promise<string>
```

---

## 5. Debug Info and Source Maps Strategy

### 5.1 Debug Information Approaches

| Approach | Use Case | Tooling | Size Impact |
|----------|----------|---------|-------------|
| **DWARF embedded** | Development | Chrome DevTools, lldb | Large |
| **DWARF split** | Release debug | Separate .dwarf file | None on .wasm |
| **Source maps** | Legacy browsers | Browser DevTools | Moderate |
| **None** | Production | N/A | Minimal |

### 5.2 DWARF Generation

```aria
# Build with debug info
aria build --target wasm-browser --debug=dwarf

# Split DWARF for smaller binaries
aria build --target wasm-browser --debug=dwarf-split
# Outputs: app.wasm (small) + app.wasm.dwarf (separate)
```

**DWARF sections embedded**:
- `.debug_info` - Type and function metadata
- `.debug_line` - Instruction to source mapping
- `.debug_abbrev` - Abbreviation tables
- `.debug_str` - String table
- `.debug_ranges` - Address ranges

### 5.3 Source Map Generation (Fallback)

For environments without DWARF support:

```bash
# Generate source map
aria build --target wasm-browser --source-map

# Outputs:
# - app.wasm
# - app.wasm.map (source map v3)
```

Source map structure:
```json
{
  "version": 3,
  "file": "app.wasm",
  "sources": ["src/main.aria", "src/lib.aria"],
  "sourcesContent": [...],
  "names": ["main", "process", "User", ...],
  "mappings": "AAAA,SAASA,..."
}
```

### 5.4 Debug Experience Goals

```
Developer Debug Flow:
1. Set breakpoint in Aria source (VSCode/DevTools)
2. Execution stops at WASM instruction
3. DWARF maps instruction → Aria source line
4. Variable inspection shows Aria types (not i32/pointers)
5. Stack trace shows Aria function names
6. Step through Aria code (not WASM instructions)
```

### 5.5 Debug Build Configuration

```toml
# aria.toml
[profile.wasm-debug]
target = "wasm32-unknown-unknown"
opt-level = 0           # No optimization
debug = "dwarf"         # Full DWARF
contracts = "runtime"   # Enable contract checks
assertions = true       # Enable assert!() checks
panic = "unwind"        # Detailed panic info
name-section = true     # Preserve function names

[profile.wasm-release]
target = "wasm32-unknown-unknown"
opt-level = "z"
debug = "none"          # Strip all debug
contracts = "off"       # No runtime checks
assertions = false
panic = "trap"
lto = true
```

---

## 6. Implementation Phases

### Phase 1: Core WASM Foundation (Q1 2026)

**Goals**: Basic compilation to core WASM

| Task | Description | Deliverable |
|------|-------------|-------------|
| WASM codegen basics | IR → WASM module | `aria build --target wasm32` |
| Linear memory allocator | Simple bump allocator | Stack + heap management |
| Basic JS interop | Number and string types | `@wasm_export` / `@wasm_import` |
| Debug info (names) | Function name preservation | Readable stack traces |

**Key Decisions**:
- Use **Cranelift** for WASM codegen (faster iteration)
- Target **WASM 1.0** for maximum compatibility
- Manual memory management (no GC yet)

### Phase 2: WASM 3.0 Features (Q2 2026)

**Goals**: Leverage modern WASM features

| Task | Description | Deliverable |
|------|-------------|-------------|
| WASM GC integration | @shared types via GC | Automatic cycle handling |
| 64-bit memory | Large array support | i64 address space |
| SIMD optimizations | Vector operations | `@wasm_simd` intrinsics |
| DWARF debug info | Full source debugging | Chrome DevTools integration |

**Key Decisions**:
- Feature detection for graceful degradation
- WASM GC for `@shared`, linear memory for owned

### Phase 3: Component Model & WASI (Q3 2026)

**Goals**: Cross-language interop and server-side

| Task | Description | Deliverable |
|------|-------------|-------------|
| WIT generation | Auto-generate from Aria types | `.wit` file output |
| Component output | Full component binary | `--target wasm-component` |
| WASI 0.2 support | HTTP, filesystem, clock | WASI imports/exports |
| World definitions | Aria world syntax | `@wasm_world` attribute |

**Key Decisions**:
- WIT generation automatic for `@wasm_export` types
- Support both core WASM and component output

### Phase 4: Optimization & Polish (Q4 2026)

**Goals**: Production-ready WASM

| Task | Description | Deliverable |
|------|-------------|-------------|
| wasm-opt integration | Post-processing | Smaller binaries |
| Tree shaking | DCE for WASM | Minimal output |
| Streaming compilation | Large module support | Fast browser load |
| Source maps | Legacy fallback | Broad debug support |
| Performance tuning | Benchmarking | Competitive performance |

---

## 7. Code Generator Selection

### 7.1 Options Analysis

| Generator | Pros | Cons | WASM GC | Aria Fit |
|-----------|------|------|---------|----------|
| **Cranelift** | Fast compile, Rust-native | ~14% slower code | Via Wasmtime | Good for dev |
| **LLVM** | Best optimization | Slow compile | Experimental | Good for release |
| **Binaryen** | WASM-specific opts, GC support | Limited | **Yes** | Good for WASM GC |
| **Custom** | Full control | High effort | Possible | Long-term option |

### 7.2 Recommended Strategy

```
Aria Source
    ↓
Aria MIR (backend-agnostic)
    ↓
┌───────────────────────────────────────┐
│       Target-Specific Lowering         │
├───────────────────────────────────────┤
│ WASM (Cranelift)  │ WASM (Binaryen)   │
│ - Core modules    │ - GC features      │
│ - Fast iteration  │ - @shared types    │
│                   │ - Advanced opts    │
└───────────────────┴───────────────────┘
    ↓
wasm-opt post-processing
    ↓
Final .wasm binary
```

**Phase 1**: Cranelift only (simplicity)
**Phase 2+**: Add Binaryen path for GC features

---

## 8. Key Resources

1. [WebAssembly 3.0 Announcement](https://webassembly.org/news/2025-09-17-wasm-3.0/)
2. [WASI and Component Model Status](https://eunomia.dev/blog/2025/02/16/wasi-and-the-webassembly-component-model-current-status/)
3. [Component Model Introduction](https://component-model.bytecodealliance.org/)
4. [wasm-bindgen Guide](https://rustwasm.github.io/docs/wasm-bindgen/)
5. [DWARF for WebAssembly](https://yurydelendik.github.io/webassembly-dwarf/)
6. [WebAssembly Debugging Tool Conventions](https://github.com/WebAssembly/tool-conventions/blob/main/Debugging.md)
7. [Binaryen WASM GC Support](https://github.com/WebAssembly/binaryen/discussions/3886)
8. [WASI.dev](https://wasi.dev/)
9. [Memory Control Proposal](https://github.com/WebAssembly/memory-control/blob/main/proposals/memory-control/Overview.md)
10. [Practical Guide to WASM Memory](https://radu-matei.com/blog/practical-guide-to-wasm-memory/)

---

## 9. Open Questions

1. **Thread support**: How to handle Aria's `spawn` in single-threaded WASM browsers?
   - Current thinking: Cooperative scheduling via generators

2. **Effect system mapping**: How do Aria effects translate to WASI 0.3 async?
   - Investigation needed when WASI 0.3 stabilizes

3. **GC fallback**: What to do when WASM GC unavailable but `@shared` used?
   - Options: Error, manual RC, or require feature flag

4. **Binary size budgets**: What are acceptable size targets?
   - Proposed: <100KB core, <500KB typical app, <2MB large app

5. **Streaming instantiation**: How to optimize initial load time?
   - Investigation: Split modules, lazy loading

---

## 10. Conclusion

Aria's WASM compilation strategy provides a clear path from today's core WASM to tomorrow's Component Model:

1. **Start practical**: Core WASM with Cranelift for fast iteration
2. **Leverage WASM 3.0**: GC proposal aligns perfectly with `@shared` types
3. **Embrace Component Model**: Future of interoperability
4. **Prioritize debugging**: DWARF support for developer experience
5. **Dual-target design**: Browser and WASI from the same source

This strategy maintains Aria's core values - ownership safety, contracts, and developer experience - while targeting the modern WASM ecosystem.

---

**Document Status**: Complete
**Next Steps**: ARIA-M08-04 (WASI patterns study), ARIA-M08-05 (JS interop design)
**Author**: FLUX Research Agent
