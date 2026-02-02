# ARIA-M08-01: Rust wasm-bindgen Analysis

**Task ID**: ARIA-M08-01
**Status**: Completed
**Date**: 2026-01-14
**Focus**: Analyze Rust's JS interop approach for WebAssembly

---

## Executive Summary

wasm-bindgen is Rust's primary tool for high-level JavaScript interoperability in WebAssembly. This research analyzes its architecture, type marshalling, and performance characteristics for Aria's WASM backend design.

---

## 1. Overview

### 1.1 What is wasm-bindgen?

wasm-bindgen facilitates high-level interactions between WebAssembly and JavaScript:
- Generates JavaScript glue code
- Handles type conversions
- Enables DOM access from Rust/WASM
- Future-compatible with Component Model

### 1.2 Ecosystem Tools

| Tool | Purpose |
|------|---------|
| `wasm-bindgen` | Core bindings library |
| `wasm-pack` | Build tool and bundler |
| `js-sys` | JavaScript standard library bindings |
| `web-sys` | Web API bindings |
| `gloo` | Higher-level browser APIs |

---

## 2. How wasm-bindgen Works

### 2.1 Architecture

```
Rust Code + #[wasm_bindgen] attributes
            ↓
    wasm-bindgen macro expansion
            ↓
    Rust → WASM compilation
            ↓
    wasm-bindgen CLI processing
            ↓
    Generated JS glue + .wasm file
            ↓
    JavaScript can import/call
```

### 2.2 Basic Usage

```rust
use wasm_bindgen::prelude::*;

// Export Rust function to JS
#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

// Import JS function into Rust
#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// Export struct to JS
#[wasm_bindgen]
pub struct User {
    name: String,
    age: u32,
}

#[wasm_bindgen]
impl User {
    #[wasm_bindgen(constructor)]
    pub fn new(name: String, age: u32) -> User {
        User { name, age }
    }

    pub fn greet(&self) -> String {
        format!("I'm {} and I'm {}", self.name, self.age)
    }
}
```

---

## 3. Type Marshalling

### 3.1 Supported Types

| Rust Type | JS Type | Conversion |
|-----------|---------|------------|
| `i32`, `u32`, `f32`, `f64` | `number` | Direct |
| `i64`, `u64` | `BigInt` | Direct |
| `bool` | `boolean` | Direct |
| `String` | `string` | Copy |
| `&str` | `string` | Copy |
| `Vec<T>` | `Array` | Copy |
| `Option<T>` | `T \| undefined` | Wrapped |
| `Result<T, E>` | throw/return | Exception |
| `JsValue` | any | Opaque |

### 3.2 Memory Considerations

**Copying (default)**:
```rust
// String is copied from Rust to JS heap
#[wasm_bindgen]
pub fn get_string() -> String {
    "hello".to_string()  // Copied to JS
}
```

**Zero-copy (TypedArrays)**:
```rust
// Return a view into WASM memory
#[wasm_bindgen]
pub fn get_array() -> js_sys::Uint8Array {
    let data: Vec<u8> = vec![1, 2, 3];
    // Creates view, not copy
    unsafe { js_sys::Uint8Array::view(&data) }
}
```

---

## 4. Performance Characteristics

### 4.1 Overhead Analysis (2025)

| Operation | Overhead | Notes |
|-----------|----------|-------|
| Numeric pass | ~0 | Direct, no conversion |
| String pass (small) | ~1-5μs | Copy overhead |
| String pass (large) | O(n) | Linear with size |
| Array pass | O(n) | Must copy unless view |
| Object creation | ~10-50μs | Wrapper allocation |

### 4.2 Benchmark Results

```
Raw WASM export: 1.0x (baseline)
wasm-bindgen:    1.1-1.5x (small overhead)
JavaScript:      0.3-0.8x (slower for compute)
```

### 4.3 Optimization Tips

1. **Batch operations**: Minimize boundary crossings
2. **Use TypedArrays**: Zero-copy for large arrays
3. **Avoid frequent small calls**: Amortize overhead
4. **Pre-allocate**: Reuse buffers when possible

---

## 5. DOM Access

### 5.1 web-sys Example

```rust
use wasm_bindgen::prelude::*;
use web_sys::{Document, Element, Window};

#[wasm_bindgen]
pub fn update_dom() -> Result<(), JsValue> {
    let window: Window = web_sys::window().unwrap();
    let document: Document = window.document().unwrap();

    let element: Element = document
        .get_element_by_id("app")
        .unwrap();

    element.set_inner_html("Hello from Rust!");

    Ok(())
}
```

### 5.2 Future: Direct DOM Access

wasm-bindgen is designed for future Web IDL bindings:
- Eventually, no JS shim needed
- Direct WASM → DOM calls
- Potential speedup over JavaScript

---

## 6. Component Model Compatibility

### 6.1 Current State

wasm-bindgen acts as a polyfill for features coming in:
- WebAssembly Component Model
- Interface Types
- Module Linking

### 6.2 Future Migration Path

```
Current: Rust → wasm-bindgen → JS glue → Browser
Future:  Rust → Component Model → Direct Browser
```

---

## 7. Toolchain: wasm-pack

### 7.1 Build Workflow

```bash
# Initialize project
wasm-pack new my-wasm-lib

# Build for npm
wasm-pack build --target web

# Output structure:
pkg/
  ├── my_wasm_lib.js      # JS glue
  ├── my_wasm_lib.d.ts    # TypeScript types
  ├── my_wasm_lib_bg.wasm # WASM binary
  └── package.json
```

### 7.2 Target Modes

| Target | Use Case |
|--------|----------|
| `bundler` | Webpack, Rollup |
| `web` | Native ES modules |
| `nodejs` | Node.js |
| `no-modules` | Script tag |

---

## 8. Recommendations for Aria

### 8.1 JS Interop Design

```aria
# Aria WASM interop syntax (proposed)
@wasm_export
fn greet(name: String) -> String
  "Hello, #{name}!"
end

@wasm_import(js: "console.log")
extern fn console_log(msg: String)

# Use web APIs
@wasm_import(web_sys: "document")
extern fn get_document() -> Document
```

### 8.2 Type Mapping

| Aria Type | WASM | JS |
|-----------|------|-----|
| `Int` | i32/i64 | number/BigInt |
| `Float` | f64 | number |
| `String` | ptr+len | string |
| `Array[T]` | ptr+len | Array/TypedArray |
| `Option[T]` | nullable | T \| undefined |
| `Result[T,E]` | tagged | throw/return |

### 8.3 Zero-Copy Strategy

```aria
# Aria could offer explicit zero-copy views
@wasm_export
fn process_data(data: ArrayView[Float]) -> Float
  # data is a view into JS TypedArray
  # No copy, but lifetime-restricted
  data.sum
end

# vs copying (safer but slower)
@wasm_export
fn process_data_copy(data: Array[Float]) -> Float
  # data is copied from JS
  data.sum
end
```

### 8.4 Component Model Preparation

Design Aria's WASM interface to be forward-compatible:

```aria
# Interface definition (like .wit files)
interface Calculator
  fn add(a: Int, b: Int) -> Int
  fn multiply(a: Int, b: Int) -> Int
end

@wasm_component(interface: Calculator)
module AriaCalculator
  fn add(a: Int, b: Int) -> Int = a + b
  fn multiply(a: Int, b: Int) -> Int = a * b
end
```

---

## 9. Key Resources

1. [wasm-bindgen Guide](https://rustwasm.github.io/docs/wasm-bindgen/)
2. [wasm-bindgen GitHub](https://github.com/rustwasm/wasm-bindgen)
3. [Rust and WebAssembly Book](https://rustwasm.github.io/book/)
4. [MDN: Compiling Rust to WebAssembly](https://developer.mozilla.org/en-US/docs/WebAssembly/Guides/Rust_to_Wasm)
5. [WebAssembly in 2025](https://medium.com/@p.reaboi.frontend/webassembly-in-2025-the-full-story-frontend-web3-limitations-7ee7cf0f9292)

---

## 10. Open Questions

1. Should Aria generate its own JS glue or use wasm-bindgen?
2. How do we handle Aria's effects across the WASM boundary?
3. What's the strategy for DOM framework integration?
4. Should we target Component Model directly?
