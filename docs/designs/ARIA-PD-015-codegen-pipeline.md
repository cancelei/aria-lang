# ARIA-PD-015: Code Generation Pipeline Design

**Decision ID**: ARIA-PD-015
**Status**: Approved
**Date**: 2026-01-15
**Author**: FORGE-III (Product Decision Agent)
**Research Inputs**:
- ARIA-M06-02: Code Generation and Optimization Strategies (NOVA)
- ARIA-PD-008: Effect Compilation Pipeline Design (CIPHER)
- ARIA-M07-01: LLVM vs Cranelift Comparison

---

## Decision Summary

Aria's code generation pipeline will implement a **Dual-Backend Architecture** with:

1. **Cranelift as primary backend** for debug builds, REPL, and JIT
2. **LLVM as optional backend** for release builds and maximum optimization
3. **Six-stage optimization pipeline** with effect-aware passes
4. **Tiered JIT compilation** for hot-path optimization
5. **Comprehensive platform support matrix** prioritizing x86_64 and AArch64

**Core Philosophy**: Fast iteration during development (Cranelift), maximum performance for production (LLVM), with effects optimized at both levels.

---

## 1. Backend Selection Decision

### 1.1 Decision Matrix

| Criterion | Cranelift | LLVM | Weight | Winner |
|-----------|-----------|------|--------|--------|
| **Compile Speed (Debug)** | 1.5-3x faster | Baseline | 0.25 | Cranelift |
| **Compile Speed (Release)** | ~Same | Baseline | 0.10 | Tie |
| **Runtime Performance** | ~14% slower | Baseline | 0.20 | LLVM |
| **Platform Support** | 4-5 targets | 20+ targets | 0.10 | LLVM |
| **WASM Support** | Native | Via Emscripten | 0.15 | Cranelift |
| **Integration Complexity** | Pure Rust | C++ FFI | 0.10 | Cranelift |
| **Binary Size** | Smaller | Larger (~500MB) | 0.05 | Cranelift |
| **Effect Compilation** | WasmFX coming | Mature exceptions | 0.05 | Tie |

**Weighted Score**: Cranelift 0.65, LLVM 0.45 for development; LLVM wins for production.

### 1.2 Backend Selection Decision Table

| Build Profile | Backend | Optimization Level | Rationale |
|--------------|---------|-------------------|-----------|
| `dev` | Cranelift | -O0 | Maximum compile speed |
| `dev-fast` | Cranelift | -Og | Debug-friendly perf |
| `test` | Cranelift | -O1 | Fast test iteration |
| `release` | LLVM | -O2 | Production builds |
| `release-perf` | LLVM | -O3 | Performance-critical |
| `release-size` | LLVM | -Os | Embedded/WASM |
| `repl` | Cranelift JIT | -O0 | Instant feedback |

### 1.3 Default Configuration

```toml
# aria.toml - Compiler configuration defaults

[profile.dev]
backend = "cranelift"
opt-level = 0
debug = true
debug-info = "full"
incremental = true
effect-validation = true
lto = false

[profile.dev-fast]
backend = "cranelift"
opt-level = "g"  # Debug-optimized
debug = true
debug-info = "line-tables-only"
incremental = true
effect-validation = true

[profile.test]
backend = "cranelift"
opt-level = 1
debug = true
debug-info = "line-tables-only"
incremental = true
effect-validation = true

[profile.release]
backend = "llvm"
opt-level = 2
debug = false
debug-info = false
lto = "thin"
effect-validation = false  # Compile-time only
panic = "abort"

[profile.release-perf]
backend = "llvm"
opt-level = 3
debug = false
lto = "fat"
codegen-units = 1
panic = "abort"

[profile.release-size]
backend = "llvm"
opt-level = "s"
debug = false
lto = "thin"
panic = "abort"
strip = true
```

### 1.4 Architecture Overview

```
                           Aria Source Code
                                  |
                                  v
                          +---------------+
                          |    Parser     |
                          +---------------+
                                  |
                                  v
                          +---------------+
                          |      HIR      |
                          +---------------+
                                  |
                                  v
                          +---------------+
                          |  Type Check   |
                          +---------------+
                                  |
                                  v
                          +---------------+
                          |   Aria MIR    |
                          +---------------+
                                  |
                    +-------------+-------------+
                    |                           |
                    v                           v
           +----------------+          +----------------+
           |   Cranelift    |          |      LLVM      |
           |    Backend     |          |    Backend     |
           +----------------+          +----------------+
                    |                           |
                    v                           v
           +----------------+          +----------------+
           |  Cranelift IR  |          |    LLVM IR     |
           +----------------+          +----------------+
                    |                           |
                    v                           v
           +----------------+          +----------------+
           | Native Code /  |          | Native Code    |
           | WASM / JIT     |          |                |
           +----------------+          +----------------+
```

---

## 2. Optimization Pipeline Stages

### 2.1 Pipeline Overview

The optimization pipeline consists of six major stages, each with specific passes:

```
Stage 1: Early Optimization (Frontend IR)
   |
Stage 2: Effect Classification & Optimization
   |
Stage 3: MIR Optimization
   |
Stage 4: Backend-Specific Lowering
   |
Stage 5: Backend Optimization
   |
Stage 6: Code Generation
```

### 2.2 Stage 1: Early Optimization (HIR Level)

| Pass | Description | Enabled |
|------|-------------|---------|
| **Constant Folding** | Evaluate compile-time constants | Always |
| **Dead Code Elimination (Basic)** | Remove unreachable code | Always |
| **Inlining (Trivial)** | Inline tiny functions (<8 instructions) | Always |
| **Type Erasure** | Monomorphize generics | Always |
| **Effect Row Simplification** | Simplify effect type constraints | Always |

### 2.3 Stage 2: Effect Classification & Optimization

| Pass | Description | Enabled |
|------|-------------|---------|
| **Effect Classification** | Classify handlers: tail-resumptive vs general | Always |
| **Pure Region Analysis** | Identify effect-free regions | Always |
| **Handler Inlining** | Inline known handlers at perform sites | -O1+ |
| **Evidence Propagation** | Convert dynamic to static evidence lookups | -O1+ |
| **Tail-Resumptive Optimization** | Convert tail-resumptive to direct calls | Always |
| **Effect Fusion** | Combine adjacent effect operations | -O2+ |
| **Open Floating** | Float evidence adjustments upward | -O2+ |

### 2.4 Stage 3: MIR Optimization

| Pass | Description | Enabled |
|------|-------------|---------|
| **SSA Construction** | Convert to SSA form | Always |
| **Constant Propagation** | Propagate known values | -O1+ |
| **Dead Code Elimination (Effect-Aware)** | Remove dead code respecting effects | -O1+ |
| **Common Subexpression Elimination** | Eliminate redundant computations | -O2+ |
| **Loop Invariant Code Motion** | Hoist invariants out of loops | -O2+ |
| **Alias Analysis** | Pointer analysis for optimization | -O2+ |
| **Async State Machine Generation** | Convert simple async to state machines | -O1+ |
| **Fiber Allocation Optimization** | Reduce fiber allocations | -O2+ |

### 2.5 Stage 4: Backend-Specific Lowering

#### 2.5.1 Cranelift Lowering

| Pass | Description |
|------|-------------|
| **MIR to Cranelift IR** | Convert Aria MIR to Cranelift IR |
| **Evidence Register Allocation** | Assign evidence to callee-saved registers |
| **Effect Exception Setup** | Setup exception handlers for effects |
| **Fiber Runtime Glue** | Generate fiber switch code |

#### 2.5.2 LLVM Lowering

| Pass | Description |
|------|-------------|
| **MIR to LLVM IR** | Convert Aria MIR to LLVM IR |
| **Intrinsic Mapping** | Map Aria intrinsics to LLVM |
| **Exception Model Setup** | Configure LLVM exception handling |
| **Metadata Emission** | Emit debug and effect metadata |

### 2.6 Stage 5: Backend Optimization

#### Cranelift Optimization Levels

| Level | Passes |
|-------|--------|
| -O0 | Minimal: register allocation only |
| -Og | Debug-friendly: basic block ordering, simple peepholes |
| -O1 | Standard: inlining, constant folding, dead code elimination |
| -O2 | Full: all Cranelift optimizations |

#### LLVM Optimization Levels

| Level | Passes |
|-------|--------|
| -O0 | No optimization (for debugging) |
| -O1 | Basic: mem2reg, SROA, simplifycfg |
| -O2 | Standard: full pipeline without aggressive opts |
| -O3 | Aggressive: loop vectorization, aggressive inlining |
| -Os | Size: minimize code size |
| -Oz | Minimum size: maximum size reduction |

### 2.7 Stage 6: Code Generation

| Output | Backend | Description |
|--------|---------|-------------|
| Native executable | Both | Platform-specific binary |
| Object files | Both | For linking |
| WASM | Cranelift | WebAssembly module |
| JIT code | Cranelift | In-memory executable code |

---

## 3. Effect Optimization Passes

### 3.1 Pass Details

#### 3.1.1 Effect Classification Pass

**Purpose**: Classify all effect handlers to determine optimization strategy.

**Algorithm**:
```rust
/// Classify effect handlers for optimization
pub fn classify_handlers(mir: &MirModule) -> HandlerClassification {
    let mut classification = HandlerClassification::new();

    for handler in mir.handlers() {
        let is_tail_resumptive = handler.operations.iter()
            .all(|op| is_tail_resumptive_operation(op));

        classification.insert(
            handler.id,
            if is_tail_resumptive {
                EffectClassification::TailResumptive
            } else {
                EffectClassification::General
            }
        );
    }

    classification
}
```

**Decision Table**:

| Handler Pattern | Classification | Optimization |
|-----------------|----------------|--------------|
| Single `resume` in tail position | TailResumptive | Direct call |
| Multiple `resume` calls | General | Fiber/CPS |
| `resume` not in tail position | General | Fiber/CPS |
| No `resume` (exception-like) | TailResumptive | Zero-cost |

#### 3.1.2 Handler Inlining Pass

**Purpose**: Inline known handlers at effect perform sites.

**Inlining Criteria**:

| Criterion | Threshold | Action |
|-----------|-----------|--------|
| Size (tail-resumptive) | <= 16 instructions | Always inline |
| Size (tail-resumptive, hot) | <= 64 instructions | Inline |
| Size (general) | <= 32 instructions | Consider inline |
| Call frequency: Hot | - | +30% threshold bonus |
| Effect polymorphic | - | -20% threshold penalty |
| Recursive | - | Never inline |

**Performance Impact**:

| Pattern | Before Inlining | After Inlining | Improvement |
|---------|-----------------|----------------|-------------|
| State.get | ~15 ns (vtable) | ~2 ns (direct) | 7.5x |
| State.set | ~18 ns | ~3 ns | 6x |
| Reader.ask | ~12 ns | ~1 ns | 12x |

#### 3.1.3 Evidence Propagation Pass

**Purpose**: Convert dynamic evidence lookups to static offsets.

**Before**:
```mir
%handler = effect.lookup_dynamic @State, %ev
%result = effect.perform @State.get, [], %handler
```

**After**:
```mir
%result = effect.perform @State.get, [], %ev[STATIC_OFFSET_0]
```

#### 3.1.4 Tail-Resumptive Optimization Pass

**Purpose**: Eliminate continuation capture for tail-resumptive handlers.

**Performance Impact**:

| Handler Type | Without Optimization | With Optimization | Improvement |
|--------------|---------------------|-------------------|-------------|
| Tail-resumptive | ~200 ns | ~2 ns | **100x** |
| Exception (no throw) | ~5 ns | ~0 ns | **Zero-cost** |
| Async (ready) | ~50 ns | ~10 ns | 5x |

#### 3.1.5 Effect-Aware Dead Code Elimination

**Purpose**: Remove dead code while respecting effect observability.

**Effect Classification for DCE**:

| Effect Type | Classification | DCE Behavior |
|-------------|---------------|--------------|
| IO | Observable | Never eliminate |
| State.get | Pure read | Eliminate if unused |
| State.set | Observable | Never eliminate |
| Reader.ask | Pure | Eliminate if unused |
| Exception.raise | Observable | Never eliminate |
| Async.await | Observable | Never eliminate |
| Log.* | Observable | Never eliminate |
| Random.* | Observable | Never eliminate |

#### 3.1.6 Effect Fusion Pass

**Purpose**: Combine adjacent effect operations.

**Before**:
```mir
%ev1 = effect.install %h1, %ev0, @State
%ev2 = effect.install %h2, %ev1, @Reader
```

**After**:
```mir
%ev2 = effect.install.multi [%h1, %h2], %ev0, [@State, @Reader]
```

#### 3.1.7 Open Floating Pass

**Purpose**: Float evidence adjustments upward in call tree.

**Expected Improvement**: 2.5x for effect-heavy code, reduces evidence operations by 60-80%.

### 3.2 Async State Machine Generation

**Purpose**: Convert simple async patterns to Rust-style state machines.

**Decision Criteria**:

| Criterion | State Machine | Fiber Fallback |
|-----------|--------------|----------------|
| All awaits statically known | Yes | No |
| Recursive async calls | No | Yes |
| Local state size | < 4KB | >= 4KB |
| Multi-shot continuations | No | Yes |

---

## 4. Debug vs Release Compilation Flags

### 4.1 Debug Build Configuration

| Flag | Default | Description |
|------|---------|-------------|
| `--debug` | true | Enable debug info |
| `--debug-info` | full | Debug info level: none, line-tables-only, full |
| `--assertions` | true | Enable runtime assertions |
| `--overflow-checks` | true | Check integer overflow |
| `--effect-validation` | true | Runtime effect handler validation |
| `--incremental` | true | Enable incremental compilation |
| `--backend` | cranelift | Code generation backend |
| `--opt-level` | 0 | Optimization level |

### 4.2 Release Build Configuration

| Flag | Default | Description |
|------|---------|-------------|
| `--debug` | false | Disable debug info |
| `--debug-info` | none | No debug info |
| `--assertions` | false | Disable assertions |
| `--overflow-checks` | false | No overflow checks |
| `--effect-validation` | false | No runtime validation |
| `--incremental` | false | Full rebuild for optimization |
| `--backend` | llvm | LLVM for better optimization |
| `--opt-level` | 2 | Standard optimization |
| `--lto` | thin | Link-time optimization |
| `--panic` | abort | Panic strategy |

### 4.3 Debug-Specific Features

| Feature | Debug | Release | Rationale |
|---------|-------|---------|-----------|
| Stack traces | Full | Optional | Development feedback |
| Effect tracing | Available | Off | Performance overhead |
| Fiber debugging | Enabled | Disabled | Runtime overhead |
| Evidence validation | On | Off | Compile-time sufficient |
| Handler source mapping | Full | None | Debug experience |

### 4.4 Conditional Compilation

```aria
@[cfg(debug)]
fn debug_assert(cond: Bool, msg: String): Pure
  if not cond then
    panic(msg)
  end
end

@[cfg(release)]
fn debug_assert(cond: Bool, msg: String): Pure
  // Compiled to no-op
end
```

---

## 5. JIT Compilation Strategy

### 5.1 JIT Use Cases

| Use Case | Priority | Benefit |
|----------|----------|---------|
| REPL | High | Instant feedback, interactive development |
| Hot-path optimization | Medium | Runtime specialization |
| Effect handler specialization | Medium | Runtime devirtualization |
| Dynamic code loading | Low | Plugin system |

### 5.2 Tiered Compilation Architecture

```
Tier 0: Interpreter (Optional)
   - Zero compile time
   - Collect profiling data
   - Used for cold/rare code

Tier 1: Cranelift -O0
   - Fast compilation (~1ms per function)
   - Moderate performance
   - Default for most code

Tier 2: Cranelift -O2
   - More optimization (~5ms per function)
   - For hot functions
   - Effect specialization

Tier 3: LLVM -O3 (Optional, Deferred)
   - Maximum optimization (~50ms per function)
   - For hottest paths
   - Background compilation
```

### 5.3 JIT Trigger Thresholds

| Trigger | Threshold | Action |
|---------|-----------|--------|
| Function call count | 1,000 calls | Promote to Tier 2 |
| Effect monomorphic | 95% same handler | Specialize for handler |
| Loop iteration | 10,000 iterations | OSR to Tier 2 |
| Handler type change | Any | Deoptimize to Tier 1 |

### 5.4 JIT Memory Management

```rust
/// JIT code memory configuration
pub struct JITConfig {
    /// Initial code cache size
    pub initial_cache_size: usize,       // Default: 64 MB
    /// Maximum code cache size
    pub max_cache_size: usize,           // Default: 256 MB
    /// Specialization cache limit
    pub specialization_limit: usize,     // Default: 100 per function
    /// GC interval for unused code
    pub gc_interval_seconds: u64,        // Default: 60
    /// Minimum age before GC eligible
    pub gc_min_age_seconds: u64,         // Default: 300
}
```

### 5.5 JIT for Effect Specialization

**Profile-Guided Specialization**:

```rust
/// Effect handler profiling for JIT specialization
pub struct EffectProfile {
    /// Call site identifier
    pub site_id: CallSiteId,
    /// Handler type counts
    pub handler_counts: HashMap<HandlerTypeId, u64>,
    /// Monomorphic threshold (95%)
    pub monomorphic: bool,
}

impl EffectProfile {
    pub fn should_specialize(&self) -> Option<HandlerTypeId> {
        if self.monomorphic {
            self.dominant_handler()
        } else {
            None
        }
    }
}
```

### 5.6 REPL JIT Configuration

| Setting | Value | Rationale |
|---------|-------|-----------|
| Backend | Cranelift | Built-in JIT support |
| Optimization | -O0 | Instant feedback |
| Incremental | Yes | Only recompile changes |
| Debug info | Full | Better error messages |
| Effect validation | On | Catch errors immediately |

---

## 6. Target Platform Support Matrix

### 6.1 Tier 1 Platforms (Full Support)

| Target | Cranelift | LLVM | WASM | JIT | Status |
|--------|-----------|------|------|-----|--------|
| x86_64-linux-gnu | Yes | Yes | N/A | Yes | Production |
| x86_64-apple-darwin | Yes | Yes | N/A | Yes | Production |
| x86_64-pc-windows-msvc | Yes | Yes | N/A | Yes | Production |
| aarch64-linux-gnu | Yes | Yes | N/A | Yes | Production |
| aarch64-apple-darwin | Yes | Yes | N/A | Yes | Production |
| wasm32-unknown-unknown | Yes | Via Emscripten | Native | N/A | Production |

### 6.2 Tier 2 Platforms (Supported)

| Target | Cranelift | LLVM | Status |
|--------|-----------|------|--------|
| aarch64-pc-windows-msvc | Yes | Yes | Beta |
| x86_64-unknown-freebsd | Yes | Yes | Beta |
| x86_64-unknown-netbsd | Yes | Yes | Beta |
| wasm32-wasi | Yes | Yes | Beta |

### 6.3 Tier 3 Platforms (Best Effort)

| Target | Cranelift | LLVM | Status |
|--------|-----------|------|--------|
| riscv64gc-linux-gnu | Experimental | Yes | Alpha |
| arm-linux-gnueabihf | Limited | Yes | Alpha |
| powerpc64le-linux-gnu | No | Yes | Alpha |
| s390x-linux-gnu | No | Yes | Experimental |

### 6.4 Platform-Specific Decisions

| Platform | Backend Strategy | Effect Strategy | Notes |
|----------|------------------|-----------------|-------|
| x86_64 | Cranelift dev, LLVM release | Fiber/exceptions | Full support |
| AArch64 | Cranelift dev, LLVM release | Fiber/exceptions | Full support |
| WASM | Cranelift only | WasmFX / state machine | Native WASM |
| RISC-V | LLVM only | Fiber only | Limited testing |
| ARM32 | LLVM only | Fiber only | Legacy support |

### 6.5 Feature Matrix by Platform

| Feature | x86_64 | AArch64 | WASM | RISC-V |
|---------|--------|---------|------|--------|
| JIT | Yes | Yes | No | No |
| Exceptions | Hardware | Hardware | WasmFX | Software |
| SIMD | SSE4.2/AVX2 | NEON | WASM SIMD | V |
| Fiber stack | Yes | Yes | Limited | Yes |
| Debug info | Full | Full | Limited | Basic |

---

## 7. Compiler Flag Reference

### 7.1 General Flags

| Flag | Values | Default | Description |
|------|--------|---------|-------------|
| `--backend` | cranelift, llvm | cranelift | Code generation backend |
| `--opt-level` | 0, 1, 2, 3, s, z, g | 0 | Optimization level |
| `--target` | <triple> | host | Target platform |
| `--emit` | exe, lib, obj, asm, ir | exe | Output type |

### 7.2 Debug Flags

| Flag | Values | Default | Description |
|------|--------|---------|-------------|
| `--debug` | true, false | false | Enable debug mode |
| `--debug-info` | none, line-tables-only, full | none | Debug info level |
| `--assertions` | true, false | false | Runtime assertions |
| `--overflow-checks` | true, false | false | Integer overflow checks |
| `--sanitize` | none, address, thread, memory | none | Sanitizer to use |

### 7.3 Effect Flags

| Flag | Values | Default | Description |
|------|--------|---------|-------------|
| `--effect-validation` | true, false | false | Runtime handler validation |
| `--effect-trace` | true, false | false | Effect operation tracing |
| `--effect-inline-threshold` | <int> | 32 | Handler inlining size limit |
| `--effect-specialize` | true, false | true | Enable effect specialization |

### 7.4 Optimization Flags

| Flag | Values | Default | Description |
|------|--------|---------|-------------|
| `--lto` | none, thin, fat | none | Link-time optimization |
| `--codegen-units` | <int> | 16 | Parallel codegen units |
| `--inline-threshold` | <int> | 225 | Function inlining threshold |
| `--unroll-loops` | true, false | true | Loop unrolling |

### 7.5 Output Flags

| Flag | Values | Default | Description |
|------|--------|---------|-------------|
| `--strip` | true, false | false | Strip symbols |
| `--panic` | unwind, abort | unwind | Panic strategy |
| `--relocation-model` | static, pic, pie | pic | Relocation model |

---

## 8. Implementation Phases

### Phase 1: Cranelift Foundation (Weeks 1-4)

- [ ] Complete MIR to Cranelift IR lowering
- [ ] Implement evidence register convention
- [ ] Basic effect perform lowering (tail-resumptive only)
- [ ] Debug info emission
- [ ] Target: x86_64-linux-gnu

**Deliverables**:
- Compile simple Aria programs with effects
- Debug builds functional
- Benchmark: compile 1000 LOC in < 1 second

### Phase 2: Effect Optimization (Weeks 5-8)

- [ ] Effect classification pass
- [ ] Handler inlining pass
- [ ] Evidence propagation pass
- [ ] Tail-resumptive optimization
- [ ] Effect-aware DCE

**Deliverables**:
- State effect at 100M+ ops/sec
- Tail-resumptive handlers at near-native performance
- Benchmark suite passing

### Phase 3: General Effects & Async (Weeks 9-12)

- [ ] Fiber runtime integration
- [ ] General effect lowering (CPS/exceptions)
- [ ] Async state machine generation
- [ ] Effect fusion and open floating

**Deliverables**:
- Full effect system functional
- Async competitive with Rust
- General handlers at 1M+ ops/sec

### Phase 4: LLVM Backend (Weeks 13-16)

- [ ] MIR to LLVM IR lowering
- [ ] LLVM optimization pipeline integration
- [ ] LTO support
- [ ] Release build profile

**Deliverables**:
- LLVM backend functional
- 14% performance improvement over Cranelift
- Release builds production-ready

### Phase 5: JIT Infrastructure (Weeks 17-20)

- [ ] Cranelift JIT module integration
- [ ] REPL JIT compilation
- [ ] Hot-path profiling
- [ ] Tiered compilation

**Deliverables**:
- REPL with instant feedback
- Hot-path optimization functional
- JIT specialization for effects

### Phase 6: Platform Expansion (Weeks 21-24)

- [ ] AArch64 support (both backends)
- [ ] Windows support
- [ ] WASM target
- [ ] Cross-compilation

**Deliverables**:
- Tier 1 platforms fully supported
- WASM builds functional
- Cross-compilation working

---

## 9. Performance Targets

### 9.1 Compilation Speed

| Scenario | Target | Measurement |
|----------|--------|-------------|
| Debug build (1K LOC) | < 0.5 sec | Clean build |
| Debug build (10K LOC) | < 3 sec | Clean build |
| Debug build (100K LOC) | < 30 sec | Clean build |
| Incremental rebuild | < 0.5 sec | Single file change |
| Release build (10K LOC) | < 10 sec | Clean build |
| REPL expression | < 50 ms | Single expression |

### 9.2 Runtime Performance

| Benchmark | Target | Reference |
|-----------|--------|-----------|
| State.get/set loop | 150M ops/sec | Koka: 150M |
| Pure function call | 500M ops/sec | Native baseline |
| Exception (happy path) | 0% overhead | Rust/C++ |
| Exception (throw) | < 1 us | Industry standard |
| Async await (ready) | 50M ops/sec | Rust async |
| General handler | 1M ops/sec | OCaml 5 |

### 9.3 Binary Size

| Profile | Target | Notes |
|---------|--------|-------|
| Debug | No limit | Include all debug info |
| Release | < 2x Rust | Similar functionality |
| Release (stripped) | < 1.5x Rust | Comparable |
| WASM | < 100 KB | Simple programs |

---

## 10. Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Cranelift exceptions insufficient | Low | High | Fiber runtime fallback ready |
| Performance targets missed | Medium | Medium | Iterative profiling and optimization |
| LLVM integration complexity | Medium | Medium | Clean MIR abstraction |
| JIT compilation overhead | Low | Low | Optional, background compilation |
| Platform support gaps | Low | Medium | Prioritize Tier 1 platforms |
| Effect optimization bugs | Medium | High | Extensive test suite, validation passes |
| Memory safety in JIT | Low | High | Conservative code generation |

---

## 11. Dependencies

### 11.1 External Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| Cranelift | 0.110+ | Primary code generation |
| LLVM | 18+ | Release optimization |
| Wasmtime | 20+ | WASM runtime (optional) |

### 11.2 Internal Dependencies

| Component | Status | Blocking |
|-----------|--------|----------|
| ARIA-PD-008 (Effect Compilation) | Approved | Phase 2 |
| ARIA-M06 (IR Design) | In Progress | Phase 1 |
| Fiber Runtime | Planned | Phase 3 |
| REPL Infrastructure | Planned | Phase 5 |

---

## 12. Decision Log

| Decision | Options Considered | Chosen | Rationale |
|----------|-------------------|--------|-----------|
| Primary backend | Cranelift, LLVM, Both | Cranelift primary | Faster iteration, pure Rust |
| Release backend | Cranelift only, LLVM only, Optional LLVM | Optional LLVM | Best of both worlds |
| JIT approach | Interpreter, Baseline JIT, Tiered | Tiered | Gradual optimization |
| Effect compilation | CPS only, Exceptions only, Hybrid | Hybrid | Performance + generality |
| WASM strategy | Emscripten, Native Cranelift | Native Cranelift | Better integration |

---

## Appendix A: Cranelift IR Patterns

### A.1 Evidence Register Convention

```clif
; x86_64: r12 for evidence vector
; AArch64: x19 for evidence vector

function %effectful_fn(i64, i64) -> i64 {
    ; v0 = regular arg
    ; v1 = evidence vector pointer

block0(v0: i64, v1: i64):
    ; Evidence stored in callee-saved register
    set_pinned_reg r12, v1

    v2 = call %effect_op(v0)
    return v2
}
```

### A.2 Tail-Resumptive Effect

```clif
function %state_get() -> i64 {
block0:
    v0 = get_pinned_reg r12       ; evidence ptr
    v1 = load.i64 v0+0            ; handler ptr at offset 0
    v2 = load.i64 v1+0            ; vtable ptr
    v3 = load.i64 v2+0            ; get function ptr
    v4 = call_indirect sig0, v3(v1)
    return v4
}
```

---

## Appendix B: LLVM IR Patterns

### B.1 Effect Function with Evidence

```llvm
define i64 @effectful_fn(i64 %arg, ptr %evidence) {
entry:
    %handler = load ptr, ptr %evidence
    %vtable = load ptr, ptr %handler
    %get_fn = load ptr, ptr %vtable
    %result = call i64 %get_fn(ptr %handler)
    ret i64 %result
}
```

### B.2 Exception-Based Effect

```llvm
define i64 @general_effect(ptr %evidence) personality ptr @aria_personality {
entry:
    %result = invoke i64 @effect_perform(ptr %evidence)
        to label %normal unwind label %catch

normal:
    ret i64 %result

catch:
    %lp = landingpad { ptr, i32 }
        catch ptr @ChoiceEffect
    %handler_result = call i64 @dispatch_handler(ptr %evidence, { ptr, i32 } %lp)
    ret i64 %handler_result
}
```

---

## Appendix C: Benchmark Suite

```rust
/// Aria codegen benchmark suite
pub mod benchmarks {
    /// Compile time benchmark
    pub fn bench_compile_time(lines: usize) -> Duration {
        let source = generate_test_source(lines);
        let start = Instant::now();
        compile(&source, Profile::Dev);
        start.elapsed()
    }

    /// State effect performance
    pub fn bench_state_counter(n: u64) -> u64 {
        with_state(0u64, || {
            for _ in 0..n {
                set(get() + 1);
            }
            get()
        })
    }

    /// Exception happy path
    pub fn bench_exceptions_happy(n: u64) -> u64 {
        let mut sum = 0u64;
        for i in 0..n {
            sum += try_catch(|| i, |_| 0);
        }
        sum
    }

    /// Async await (ready)
    pub fn bench_async_ready(n: u64) -> u64 {
        run_async(|| {
            let mut sum = 0;
            for i in 0..n {
                sum += await(Promise::resolved(i));
            }
            sum
        })
    }

    /// General handler (backtracking)
    pub fn bench_choice(n: u64) -> Vec<u64> {
        all_choices(|| {
            let mut sum = 0;
            for _ in 0..n {
                sum += if choose() { 1 } else { 2 };
            }
            sum
        })
    }
}
```

---

**Document Status**: This product decision document is complete and approved for implementation. Implementation should proceed according to the phased approach in Section 8.

---

*Document generated by FORGE-III (Product Decision Agent)*
*Based on research by NOVA (ARIA-M06-02)*
*Building upon ARIA-PD-008 by CIPHER*
*Aria Language Project - Eureka Iteration 2*
*Last updated: 2026-01-15*
