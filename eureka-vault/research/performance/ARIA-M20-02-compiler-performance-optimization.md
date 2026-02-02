# ARIA-M20-02: Compiler Performance Optimization

**Task ID**: ARIA-M20-02
**Status**: Completed
**Date**: 2026-01-14
**Focus**: Research compiler performance optimization techniques

---

## Executive Summary

Fast compilation is crucial for developer experience. This research analyzes techniques used by Rust, Go, Zig, and other compilers to achieve fast build times, informing Aria's compiler architecture.

---

## 1. Overview

### 1.1 Why Compilation Speed Matters

- **Developer productivity**: Slow builds break flow
- **CI/CD costs**: Longer builds = higher costs
- **Iteration speed**: Fast feedback loops essential
- **Language adoption**: Speed affects perception

### 1.2 Compilation Speed Targets

| Speed | Perception | Example |
|-------|------------|---------|
| < 1s | Instant | Go, Zig |
| 1-10s | Fast | Rust incremental |
| 10-60s | Acceptable | Rust clean |
| > 60s | Slow | Large C++ projects |

---

## 2. Rust Compiler Performance (2025)

### 2.1 Recent Improvements

Per 2025 benchmarks:
- **6x faster builds** possible with optimization techniques
- Pre-compiled standard library caching
- Split compilation (parallel stages)
- Smart rebuild (affected paths only)

### 2.2 Techniques Used

| Technique | Speedup |
|-----------|---------|
| Incremental compilation | 2-10x |
| Parallel codegen | 2-4x |
| Pipelined compilation | 1.2-1.5x |
| Thin LTO | Better than fat LTO |

### 2.3 Incremental Compilation

```
First build:
  Parse → HIR → MIR → LLVM → Binary (slow)

Subsequent builds:
  Changed files only → reuse cached artifacts
  Dependency tracking ensures correctness
```

---

## 3. Go's Compilation Speed

### 3.1 Design Decisions

| Decision | Impact |
|----------|--------|
| No header files | Single pass parsing |
| Simple type system | Fast type checking |
| Package-level compilation | Parallel by default |
| Direct machine code | Skip IR for simple cases |

### 3.2 Key Strategies

```go
// Package = compilation unit
// Can compile packages in parallel

// Import path = build dependency
// Clear DAG for parallel builds

// No generics (pre-1.18) meant simpler compilation
// Post-1.18 generics use stenciling (monomorphization)
```

---

## 4. Zig's Approach

### 4.1 Incremental by Design

```zig
// Zig's compiler is built incrementally from the start
// Every compilation reuses previous results

// Self-hosted compiler in Zig
// Dogfooding ensures fast compilation
```

### 4.2 Compilation Model

| Feature | Benefit |
|---------|---------|
| No preprocessor | Simpler parsing |
| Comptime | Compile-time evaluation |
| Lazy compilation | Only compile what's used |
| Async/await in compiler | Parallel I/O |

---

## 5. Optimization Techniques

### 5.1 Frontend Optimization

| Technique | Description |
|-----------|-------------|
| Lazy parsing | Parse on demand |
| Parallel parsing | Multi-file parsing |
| Syntax tree sharing | Incremental reparsing |
| Error recovery | Continue after errors |

### 5.2 Middle-End Optimization

| Technique | Description |
|-----------|-------------|
| Demand-driven | Only analyze what's needed |
| Query-based | Cache intermediate results |
| Parallel type checking | When possible |
| Incremental MIR | Reuse unchanged functions |

### 5.3 Backend Optimization

| Technique | Description |
|-----------|-------------|
| Parallel codegen | Multiple LLVM instances |
| Thin LTO | Parallel link-time opt |
| Cranelift | Faster than LLVM |
| Split DWARF | Faster debug builds |

---

## 6. Optimization Levels

### 6.1 GCC/Clang Model

| Level | Description | Use Case |
|-------|-------------|----------|
| `-O0` | No optimization | Debug |
| `-Og` | Debug-friendly opt | Debug with some speed |
| `-O1` | Basic optimization | Fast compile |
| `-O2` | Standard optimization | Release |
| `-O3` | Aggressive optimization | Performance critical |
| `-Os` | Size optimization | Embedded |

### 6.2 Profile-Guided Optimization

```
1. Build with instrumentation
2. Run with representative workload
3. Rebuild using profile data
4. Compiler knows hot paths

Intel 2025: PGO reports greatly improved for Thin LTO
```

---

## 7. Parallelization Strategies

### 7.1 File-Level Parallelism

```
Simple: Compile each file in parallel
Limitation: Limited by largest file

Parser ─┬─ file1.aria ─┬─ Codegen
        ├─ file2.aria ─┤
        └─ file3.aria ─┘
```

### 7.2 Function-Level Parallelism

```
Better: Compile functions in parallel within a file

File → Parse → Type Check → Per-function codegen (parallel)
```

### 7.3 Pipeline Parallelism

```
Best: Start codegen before type checking finishes

Type Check file1 → Type Check file2 → Type Check file3
     ↓ (as soon as ready)
Codegen file1    → Codegen file2    → Codegen file3
```

---

## 8. Caching Strategies

### 8.1 Build Caching

```
# Hash-based caching
cache_key = hash(source_code, compiler_version, flags)

if cache[cache_key] exists:
    return cached_artifact
else:
    compile and store in cache
```

### 8.2 Distributed Caching

| Tool | Description |
|------|-------------|
| sccache | Shared cache for Rust |
| ccache | C/C++ compiler cache |
| Bazel | Distributed build system |
| Turborepo | Monorepo build cache |

### 8.3 Pre-compiled Headers/Modules

```rust
// Rust: Pre-compiled std
// Ship pre-compiled standard library
// Users only compile their code + deps

// C++20 Modules
// Replace header includes with modules
// Much faster compilation
```

---

## 9. Benchmarking & Profiling

### 9.1 Rust Compiler Profiling

```bash
# Time compilation phases
cargo build --timings

# Detailed profiling
RUSTFLAGS="-Z time-passes" cargo build

# Self-profiling
RUSTFLAGS="-Z self-profile" cargo build
```

### 9.2 Key Metrics

| Metric | Description |
|--------|-------------|
| Wall time | Total elapsed time |
| CPU time | Actual work done |
| Peak memory | Maximum RAM used |
| Parallelism | CPU utilization |

---

## 10. Recommendations for Aria

### 10.1 Compiler Architecture

```aria
# Aria compiler phases (designed for parallelism)
Pipeline {
  # Frontend (parallel by file)
  parse: Parser
  resolve: NameResolver
  typecheck: TypeChecker

  # Middle (parallel by function)
  hir: HIRBuilder
  mir: MIRBuilder
  optimize: Optimizer

  # Backend (parallel by codegen unit)
  codegen: CodeGenerator
  link: Linker
}
```

### 10.2 Incremental Compilation

```aria
# Salsa-style query system
@query
fn type_of(expr: ExprId) -> Type
  # Automatically cached
  # Invalidated when dependencies change
end

# Key invariant:
# Changing function body ONLY invalidates that function
# (Unless signature changes)
```

### 10.3 Build Profiles

```toml
# aria.toml
[profile.dev]
opt-level = 0
debug = true
incremental = true
contracts = "debug"  # Runtime checks

[profile.release]
opt-level = 3
debug = false
incremental = false
contracts = "verify"  # Compile-time verification
lto = "thin"

[profile.check]
# Type check only, no codegen
typecheck = true
codegen = false
```

### 10.4 Dual Backend Strategy

```aria
# Development: Cranelift (fast compile)
# Release: LLVM (optimized output)

fn compile(source: Source, profile: Profile) -> Binary
  if profile.is_dev
    CraneliftBackend.compile(source)  # ~20% faster compile
  else
    LLVMBackend.compile(source)       # ~14% faster runtime
  end
end
```

### 10.5 Compilation Caching

```aria
# Content-addressed caching
CacheKey {
  source_hash: Hash
  compiler_version: Version
  target: Target
  profile: Profile
  dependencies: Array[Hash]
}

fn build_with_cache(source: Source, config: Config) -> Binary
  key = CacheKey.from(source, config)

  if cache.contains(key)
    cache.get(key)
  else
    result = compile(source, config)
    cache.put(key, result)
    result
  end
end
```

### 10.6 Parallel Compilation

```aria
# File-level parallelism
fn compile_project(files: Array[File]) -> Array[Object]
  files.parallel_map |file|
    compile_file(file)
  end
end

# Function-level parallelism (within file)
fn compile_file(file: File) -> Object
  hir = parse_and_typecheck(file)

  functions = hir.functions.parallel_map |func|
    compile_function(func)
  end

  Object.link(functions)
end
```

---

## 11. Performance Targets for Aria

| Scenario | Target |
|----------|--------|
| Incremental (1 file change) | < 1s |
| Clean build (small project) | < 10s |
| Clean build (medium project) | < 60s |
| Type check only | < 5s |
| IDE feedback | < 100ms |

---

## 12. Key Resources

1. [Rust Compiler Performance 2025](https://markaicode.com/rust-compiler-performance-2025/)
2. [Intel Compiler Optimization Report 2025](https://www.intel.com/content/www/us/en/developer/articles/technical/compiler-optimization-report-news-2025.html)
3. [Go Compilation Model](https://go.dev/doc/faq#What_is_the_purpose_of_the_project)
4. [LLVM Optimization Levels](https://llvm.org/docs/Passes.html)
5. [Cranelift vs LLVM](https://bytecodealliance.org/articles/cranelift-production-ready)

---

## 13. Open Questions

1. Should Aria ship pre-compiled stdlib binaries?
2. What's the minimum viable incremental compilation?
3. How do we balance compile speed vs runtime performance?
4. Should we support distributed compilation?
