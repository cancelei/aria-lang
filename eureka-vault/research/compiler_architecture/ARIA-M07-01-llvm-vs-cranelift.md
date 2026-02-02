# ARIA-M07-01: LLVM vs Cranelift Comparison

**Task ID**: ARIA-M07-01
**Status**: Completed
**Date**: 2026-01-14
**Focus**: In-depth comparison for Aria's backend needs

---

## Executive Summary

This research compares LLVM and Cranelift as potential backends for Aria, analyzing compilation speed, code quality, ecosystem support, and integration complexity.

---

## 1. Overview

### 1.1 LLVM

- **Created**: 2003 by Chris Lattner
- **Language**: C++
- **Lines of code**: ~20 million
- **Primary use**: Production compilers (Clang, Rust, Swift)

### 1.2 Cranelift

- **Created**: 2016 (as Cretonne)
- **Language**: Rust
- **Lines of code**: ~200K
- **Primary use**: JIT compilation, WebAssembly

---

## 2. Feature Comparison Matrix

| Feature | LLVM | Cranelift |
|---------|------|-----------|
| **Optimization quality** | Excellent | Good |
| **Compile speed (debug)** | Slow | Fast |
| **Compile speed (release)** | Very slow | Moderate |
| **Platform support** | Extensive | Growing |
| **WASM support** | Via Emscripten | Native |
| **JIT support** | MCJIT/ORC | Native |
| **Code size** | Large binary | Smaller |
| **Debugging support** | Excellent | Good |
| **LTO support** | Yes | Limited |
| **Community** | Large | Growing |

---

## 3. Compilation Speed

### 3.1 Benchmarks

| Scenario | LLVM | Cranelift | Difference |
|----------|------|-----------|------------|
| Cranelift self-compile | 37.5s | 29.6s | **20% faster** |
| CPU-seconds | 211s | 125s | **40% less** |
| Debug build (typical) | 1x | 1.5-3x faster | |
| Release build | 1x | ~same | |

### 3.2 Why Cranelift is Faster

1. **Single IR level**: No multiple lowering passes
2. **Simpler algorithms**: Trade-off vs optimization quality
3. **Cache-friendly layout**: Tightly packed IR
4. **No legacy**: Modern design without baggage

### 3.3 Academic Analysis (2024)

According to TU Munich research:
- Cranelift compiles 20-35% faster than LLVM
- But 16x slower than custom single-pass backends
- For JIT scenarios, the difference is more pronounced

---

## 4. Code Quality

### 4.1 Runtime Performance

| Benchmark | LLVM | Cranelift | Notes |
|-----------|------|-----------|-------|
| Compute-intensive | 1.0x | ~1.14x slower | ~14% gap |
| I/O bound | 1.0x | ~1.0x | Negligible |
| Startup time | Slower | Faster | JIT advantage |

### 4.2 Optimization Capabilities

| Optimization | LLVM | Cranelift |
|--------------|------|-----------|
| Constant folding | Yes | Yes |
| Dead code elimination | Yes | Yes |
| Inlining | Aggressive | Basic |
| Loop unrolling | Yes | Limited |
| Vectorization | Yes | Limited |
| LTO | Yes | Limited |
| Profile-guided | Yes | No |

---

## 5. Platform Support

### 5.1 LLVM Targets

```
x86, x86_64, ARM, AArch64, MIPS, PowerPC, RISC-V,
WebAssembly, SystemZ, SPARC, Hexagon, NVPTX (CUDA),
AMDGPU, and many more...
```

### 5.2 Cranelift Targets

```
x86_64, AArch64, RISC-V, s390x (IBM z)
WebAssembly (native support)
```

### 5.3 Summary

- **LLVM**: 20+ targets, mature support
- **Cranelift**: 4-5 targets, growing

---

## 6. Integration

### 6.1 LLVM Integration

**Pros**:
- Well-documented C API (libLLVM)
- Rust bindings (inkwell, llvm-sys)
- Extensive tutorials and examples

**Cons**:
- Large dependency (~500MB debug)
- Complex build system
- Version compatibility issues

### 6.2 Cranelift Integration

**Pros**:
- Pure Rust (easy integration)
- Simple API
- Small footprint

**Cons**:
- Less documentation
- Smaller community
- Still evolving API

---

## 7. Rust Ecosystem Integration

### 7.1 rustc Backend Support

- **LLVM**: Default backend, production quality
- **Cranelift**: Available in nightly (since Oct 2023)

### 7.2 Usage with rustc

```bash
# Use Cranelift for debug builds
RUSTFLAGS="-Zcodegen-backend=cranelift" cargo build

# Results: ~20-40% faster debug builds
```

---

## 8. WebAssembly Support

### 8.1 LLVM + WebAssembly

- Via Emscripten or wasm32 target
- Full optimization pipeline
- Larger output size
- Slower compilation

### 8.2 Cranelift + WebAssembly

- Native WASM support
- Used by Wasmtime, Wasmer
- Fast JIT compilation
- Good for runtime compilation

---

## 9. Decision Framework

### 9.1 Choose LLVM If:

- Maximum runtime performance required
- Need extensive platform support
- Release build quality is critical
- Profile-guided optimization needed
- Mature tooling essential

### 9.2 Choose Cranelift If:

- Fast iteration/debug builds priority
- WebAssembly is primary target
- JIT compilation needed
- Rust-native toolchain preferred
- Smaller binary size desired

### 9.3 Hybrid Approach

Several projects use both:
- **Debug builds**: Cranelift (fast)
- **Release builds**: LLVM (optimized)

This is what Rust nightly supports today.

---

## 10. Recommendations for Aria

### 10.1 Recommended Strategy

**Dual-backend approach**:

```
Aria Source → Aria MIR → Backend Selection
                              ↓
                   ┌──────────┴──────────┐
                   ↓                      ↓
              Cranelift              LLVM
           (Debug builds)      (Release builds)
           (JIT/REPL)          (Production)
           (WASM primary)      (Native primary)
```

### 10.2 Implementation Phases

| Phase | Backend | Focus |
|-------|---------|-------|
| 1 | Cranelift | Initial development, WASM |
| 2 | LLVM | Production optimization |
| 3 | Both | User-selectable |

### 10.3 Rationale

1. **Cranelift first**: Simpler, Rust-native, faster iteration
2. **LLVM later**: When optimization becomes critical
3. **Both available**: Let users choose based on needs

### 10.4 IR Design Implications

Design Aria MIR to be backend-agnostic:

```aria
# Aria MIR should lower to either backend
AriaMIR {
  # Abstract operations that map to both
  operations: [Add, Call, Load, Store, Branch...]

  # Backend-specific lowering
  fn lower_to_cranelift(self) -> CraneliftIR
  fn lower_to_llvm(self) -> LLVMModule
}
```

---

## 11. Key Resources

1. [Cranelift Official Site](https://cranelift.dev/)
2. [Cranelift vs LLVM Comparison](https://github.com/bytecodealliance/wasmtime/blob/main/cranelift/docs/compare-llvm.md)
3. [Cranelift for Rust - LWN](https://lwn.net/Articles/964735/)
4. [A Possible New Backend for Rust](https://jason-williams.co.uk/posts/a-possible-new-backend-for-rust/)
5. [Compiler Performance and LLVM](https://pling.jondgoodwin.com/post/compiler-performance/)

---

## 12. Open Questions

1. Can we abstract backend differences cleanly in Aria MIR?
2. What's the minimum viable LLVM subset for Aria?
3. Should we invest in Cranelift improvements?
4. How do we handle backend-specific optimizations?
