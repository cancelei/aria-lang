# ARIA-M06-02: MLIR Architecture Analysis

**Task ID**: ARIA-M06-02
**Status**: Completed
**Date**: 2026-01-14
**Focus**: Study MLIR's extensible IR approach

---

## Executive Summary

MLIR (Multi-Level Intermediate Representation) is an extensible compiler infrastructure that supports multiple abstraction levels through its dialect system. This research analyzes its architecture for potential application in Aria's compiler design.

---

## 1. MLIR Overview

### 1.1 What is MLIR?

MLIR is an **open-source compiler infrastructure** developed as an LLVM sub-project:
- Created by Chris Lattner at Google (2018)
- Released as part of LLVM (2019)
- Supports multiple abstraction levels
- Highly extensible through dialects

### 1.2 Key Innovation

Unlike traditional compilers with fixed IR levels, MLIR allows:
- Custom operations, types, and attributes per domain
- Progressive lowering between abstraction levels
- Reusable transformation infrastructure

---

## 2. Dialect System

### 2.1 What is a Dialect?

A dialect is a **namespace for operations, types, and attributes**:

```mlir
// Standard dialect operations
%0 = arith.addi %a, %b : i32
%1 = memref.load %mem[%i] : memref<256xi32>

// Affine dialect (loop analysis)
affine.for %i = 0 to 256 {
  affine.store %val, %mem[%i] : memref<256xi32>
}

// Custom domain dialect
%result = ml.matmul %A, %B : tensor<128x128xf32>
```

### 2.2 Built-in Dialects

| Dialect | Purpose |
|---------|---------|
| `arith` | Arithmetic operations |
| `memref` | Memory references |
| `affine` | Polyhedral analysis |
| `scf` | Structured control flow |
| `func` | Functions |
| `llvm` | LLVM IR representation |
| `gpu` | GPU programming |
| `vector` | Vector operations |

### 2.3 Creating Custom Dialects

```tablegen
// TableGen definition
def MyDialect : Dialect {
  let name = "my";
  let summary = "My custom dialect";
}

def MyOp : Op<MyDialect, "custom_op"> {
  let arguments = (ins AnyType:$input);
  let results = (outs AnyType:$output);
}
```

---

## 3. Multi-Level Representation

### 3.1 Progressive Lowering

```
High-Level DSL
     ↓
Domain-Specific Dialect (e.g., ml.matmul)
     ↓
Mid-Level Dialect (e.g., linalg.matmul)
     ↓
Loop-Level Dialect (e.g., affine.for)
     ↓
Low-Level Dialect (e.g., llvm.*)
     ↓
LLVM IR / Machine Code
```

### 3.2 Benefits

| Benefit | Description |
|---------|-------------|
| Optimization at right level | Apply transforms where most effective |
| Reusable passes | Same pass works across similar dialects |
| Gradual lowering | Preserve semantics longer |
| Domain awareness | High-level ops capture intent |

---

## 4. Transform Dialect (2025)

### 4.1 Overview

The Transform dialect (presented at CGO 2025) enables:
- Fine-grained control of optimizations
- Schedulable transformations
- Composable optimization recipes

### 4.2 Example

```mlir
transform.sequence failures(propagate) {
^bb0(%arg0: !transform.any_op):
  %0 = transform.structured.match ops{["linalg.matmul"]} in %arg0
  %1, %loops = transform.structured.tile %0 [64, 64, 64]
  transform.structured.vectorize %1
}
```

---

## 5. MLIR Architecture

### 5.1 Core Components

```
┌─────────────────────────────────────────────────────────────┐
│                        MLIR Core                             │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   Dialects  │  │   Passes    │  │    Interfaces       │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
│                                                              │
│  ┌─────────────────────────────────────────────────────┐    │
│  │                Pattern Rewriting                     │    │
│  │  ┌──────────────┐  ┌──────────────┐  ┌───────────┐  │    │
│  │  │ Canonicalize │  │   Convert    │  │  Greedy   │  │    │
│  │  └──────────────┘  └──────────────┘  └───────────┘  │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              Infrastructure                          │    │
│  │  ┌─────────┐  ┌─────────┐  ┌──────────┐  ┌───────┐  │    │
│  │  │  Types  │  │  Attrs  │  │ Regions  │  │ Blocks│  │    │
│  │  └─────────┘  └─────────┘  └──────────┘  └───────┘  │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

### 5.2 Key Abstractions

| Concept | Description |
|---------|-------------|
| Operation | Basic unit of computation |
| Region | Nested control flow |
| Block | Basic block in a region |
| Value | SSA value |
| Type | Operation type |
| Attribute | Compile-time constant |

---

## 6. MLIR Ecosystem

### 6.1 Notable Projects

| Project | Description |
|---------|-------------|
| **IREE** | ML model execution |
| **torch-mlir** | PyTorch compilation |
| **ONNX-MLIR** | ONNX model compilation |
| **TPU-MLIR** | Google TPU targeting |
| **Mojo** | Systems language built on MLIR |

### 6.2 Mojo and MLIR

Mojo (Modular Inc.) is built entirely on MLIR:
- Uses MLIR as core IR
- Custom dialects for ownership, effects
- Demonstrates MLIR for general-purpose languages

---

## 7. Comparison with Traditional IRs

### 7.1 LLVM IR vs MLIR

| Aspect | LLVM IR | MLIR |
|--------|---------|------|
| Abstraction | Single, low-level | Multiple levels |
| Extensibility | Limited | Highly extensible |
| Domain support | Generic | Domain-specific dialects |
| Optimization | Fixed pipeline | Composable transforms |
| Learning curve | Moderate | Steep |

### 7.2 Code Size

- MLIR: Growing, ~1M lines
- LLVM: ~20M lines
- Cranelift: ~200K lines

---

## 8. Recommendations for Aria

### 8.1 Should Aria Use MLIR?

**Pros**:
- Reusable infrastructure
- Multi-target support built-in
- Active development and community
- Domain-specific dialect for Aria possible

**Cons**:
- Steep learning curve
- Heavy dependency
- May be overkill for simpler needs
- C++ codebase (if Aria compiler is in Rust)

### 8.2 Recommendation

**Hybrid approach**:
1. Design Aria IR inspired by MLIR concepts
2. Use dialect-like abstraction for effects/ownership
3. Consider MLIR for future backends

### 8.3 Aria Dialect Design (If Using MLIR)

```mlir
// Hypothetical Aria dialect
module {
  aria.func @example(%arg0: !aria.owned<i32>) -> !aria.result<i32, !aria.error> {
    %0 = aria.unwrap %arg0 : !aria.owned<i32> -> i32
    %1 = aria.add %0, %0 : i32
    %2 = aria.wrap_ok %1 : i32 -> !aria.result<i32, !aria.error>
    aria.return %2
  } effects [io, may_throw]
}
```

### 8.4 Effect Dialect Concept

```mlir
// Effect tracking in MLIR-style
effect.region [io] {
  %file = effect.perform "file.read" %path : !io.file
  effect.yield %file
}

effect.handler [io] -> [pure] {
  ^entry(%e: !effect.io):
    %result = effect.handle %e with @io_handler
    effect.resume %result
}
```

---

## 9. Key Resources

1. [MLIR Official Site](https://mlir.llvm.org/)
2. [MLIR Wikipedia](https://en.wikipedia.org/wiki/MLIR_(software))
3. [Transform Dialect Paper (CGO 2025)](https://www.steuwer.info/files/publications/2025/CGO-The-MLIR-Transform-Dialect.pdf)
4. [MLIR Introduction - Stephen Diehl](https://www.stephendiehl.com/posts/mlir_introduction/)
5. [Modular: What about MLIR?](https://www.modular.com/blog/democratizing-ai-compute-part-8-what-about-the-mlir-compiler-infrastructure)

---

## 10. Open Questions

1. Is MLIR's complexity justified for Aria's goals?
2. How do we integrate MLIR with a Rust-based compiler?
3. Can we create a lightweight "Aria IR" inspired by MLIR?
4. What dialects would Aria need (effects, ownership, contracts)?
