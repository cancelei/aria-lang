# ARIA-M08-02: TinyGo WASM Size Optimization Study

**Task ID**: ARIA-M08-02
**Status**: Completed
**Date**: 2026-01-14
**Focus**: Analyze TinyGo's WASM size reduction techniques

---

## Executive Summary

TinyGo produces WASM binaries ~3x smaller than standard Go. This research analyzes its dead code elimination, runtime minimization, and optimization strategies for Aria's WASM backend design.

---

## 1. Overview

### 1.1 TinyGo's Purpose

TinyGo is a Go compiler for "small places":
- Microcontrollers
- WebAssembly
- Command-line tools
- Embedded systems

### 1.2 Size Comparison

| Compiler | Hello World WASM | Typical App |
|----------|------------------|-------------|
| Go | ~2MB | 5-10MB |
| TinyGo | ~100KB | 500KB-2MB |
| **Reduction** | **~20x** | **~3-5x** |

---

## 2. Optimization Levels

### 2.1 TinyGo Options

| Flag | Description | Use Case |
|------|-------------|----------|
| `-opt=0` | No optimization | Debugging |
| `-opt=1` | Basic optimization | Development |
| `-opt=2` | Full optimization | Performance |
| `-opt=s` | Size-optimized | Balance |
| `-opt=z` | **Aggressive size** (default) | WASM |

### 2.2 -opt=z Details

```
-opt=z is like -opt=s but more aggressive:
- Reduces inliner threshold significantly
- Prioritizes size over speed
- Default for TinyGo
- Best for code-size-sensitive targets
```

---

## 3. Dead Code Elimination

### 3.1 LLVM-Based DCE

```
TinyGo compilation pipeline:

Go Source
    ↓
TinyGo Frontend (SSA)
    ↓
LLVM IR
    ↓
LLVM Optimization (including DCE)
    ↓
WASM Binary
```

### 3.2 What Gets Eliminated

- Unused functions
- Unreachable code paths
- Unused type metadata
- Dead imports

### 3.3 Comparison with Standard Go

```go
// Standard Go includes entire reflect package
// Even if only reflect.TypeOf used

// TinyGo eliminates unused reflection machinery
// Only includes what's actually called
```

---

## 4. Runtime Minimization

### 4.1 Panic Handling

| Option | Size Impact | Behavior |
|--------|-------------|----------|
| Default | Larger | Full panic message |
| `-panic=trap` | **Smaller** | Just trap instruction |
| `-panic=print` | Medium | Print then trap |

```bash
# Significant size reduction
tinygo build -target=wasm -panic=trap -o app.wasm
```

### 4.2 Scheduler Options

| Option | Description |
|--------|-------------|
| `-scheduler=none` | No goroutine support |
| `-scheduler=coroutines` | Cooperative scheduling |
| `-scheduler=tasks` | Full scheduler |

### 4.3 Garbage Collector

```bash
# Options for WASM
-gc=none       # No GC (manual or no heap)
-gc=leaking    # Never frees (short-lived programs)
-gc=conservative # Default, reasonable size
-gc=boehm      # Experimental, faster but larger
```

---

## 5. wasm-opt Tool

### 5.1 Post-Processing

```bash
# After TinyGo compilation, further optimize:
wasm-opt -Oz -o optimized.wasm app.wasm

# Options:
# -O   Standard optimization
# -Os  Size optimization
# -Oz  Aggressive size optimization
```

### 5.2 wasm-opt Optimizations

- Dead code elimination
- Constant propagation
- Expression precomputation
- Vacuum (removes nops)
- Duplicate function merging

### 5.3 Combined Pipeline

```bash
# Full optimization pipeline
tinygo build -target=wasm -opt=z -no-debug -panic=trap -o app.wasm main.go
wasm-opt -Oz -o app.opt.wasm app.wasm

# Result: Minimal WASM binary
```

---

## 6. Package Avoidance

### 6.1 Heavy Packages

| Package | Size Impact | Alternative |
|---------|-------------|-------------|
| `fmt` | ~100KB+ | Direct string ops |
| `encoding/json` | ~200KB+ | Manual or tinygo/json |
| `reflect` | Large | Avoid or minimize |
| `regexp` | Large | Simple string matching |

### 6.2 Example

```go
// Heavy (pulls in fmt)
fmt.Sprintf("Hello, %s!", name)

// Light
"Hello, " + name + "!"
```

---

## 7. 2025 TinyGo Features

### 7.1 Recent Improvements

- HTML size report for analysis
- Improved error messages
- CGo improvements (function-like macros)
- Experimental Boehm GC for WASM

### 7.2 HTML Size Report

```bash
# Generate size report
tinygo build -target=wasm -size=full -o app.wasm main.go

# Shows:
# - Per-function sizes
# - Package breakdown
# - Identifies bloat sources
```

---

## 8. Recommendations for Aria

### 8.1 Optimization Levels

```toml
# aria.toml WASM build profiles
[profile.wasm-dev]
target = "wasm32"
opt-level = 0
debug = true
panic = "unwind"

[profile.wasm-release]
target = "wasm32"
opt-level = "z"    # Aggressive size
debug = false
panic = "trap"     # Minimal panic handling
lto = true         # Link-time optimization

[profile.wasm-speed]
target = "wasm32"
opt-level = 2      # Speed over size
debug = false
```

### 8.2 Dead Code Elimination

```aria
# Aria should aggressively eliminate:

# 1. Unused functions
# 2. Unused trait implementations
# 3. Unreachable pattern branches
# 4. Unused effect handlers
# 5. Dead type metadata

# Build flag
aria build --target wasm32 --tree-shake aggressive
```

### 8.3 Runtime Options

```aria
# Configurable runtime for WASM

@wasm_config(
  gc: :none,           # No GC (stack-only or manual)
  panic: :trap,        # Minimal panic handling
  scheduler: :none,    # No async runtime
  contracts: :strip,   # Remove contract checks
)
module MyWasmApp
  # Minimal runtime footprint
end
```

### 8.4 Size Analysis

```bash
# Aria size report (inspired by TinyGo)
aria build --target wasm32 --size-report

# Output:
# Size Report for app.wasm (127KB)
# ─────────────────────────────────
# Code:     89KB (70%)
#   main.aria:      12KB
#   stdlib/array:    8KB
#   stdlib/string:  15KB
#   ...
# Data:     38KB (30%)
#   String literals: 20KB
#   Type metadata:   18KB
```

### 8.5 Stdlib Subsetting

```aria
# Allow subset imports for WASM
import aria.core.minimal  # Just primitives
import aria.string.basic  # No regex, no formatting

# vs full stdlib
import aria.std  # Everything
```

### 8.6 wasm-opt Integration

```bash
# Aria could auto-run wasm-opt
aria build --target wasm32 --release

# Internally runs:
# 1. Aria compiler → app.wasm
# 2. wasm-opt -Oz → app.opt.wasm
# 3. Output optimized binary
```

---

## 9. Size Budget Guidelines

### 9.1 Target Sizes

| Application Type | Target Size |
|-----------------|-------------|
| Simple function | < 50KB |
| Small app | < 200KB |
| Medium app | < 500KB |
| Full app | < 2MB |

### 9.2 Monitoring

```aria
# Build-time size check
@wasm_size_limit(100_000)  # 100KB max
module MyModule
  # Compilation fails if exceeds limit
end
```

---

## 10. Key Resources

1. [TinyGo Optimizing Binaries](https://tinygo.org/docs/guides/optimizing-binaries/)
2. [Shrink TinyGo WASM by 60%](https://www.fermyon.com/blog/optimizing-tinygo-wasm)
3. [TinyGo GitHub](https://github.com/tinygo-org/tinygo)
4. [Binaryen wasm-opt](https://github.com/WebAssembly/binaryen)
5. [TinyGo WebAssembly Guide](https://tinygo.org/docs/guides/webassembly/)

---

## 11. Open Questions

1. Should Aria have a separate "tiny" stdlib for WASM?
2. How do we balance size vs debugging information?
3. What's the overhead of effects in WASM?
4. Should we support streaming compilation for large modules?
