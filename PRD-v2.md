# Aria Programming Language - Product Requirements Document

**Version:** 2.0 (Iterative Enhancement)
**Status:** Active Development
**Last Updated:** 2026-01-14
**Methodology:** 9x Eureka Discovery Cycles

---

## Executive Summary

Aria is a next-generation programming language designed to combine the expressiveness of Ruby/Python with the performance and safety of Rust/C. This document captures evolving requirements through iterative discovery cycles, preserving multiple implementation options where trade-offs exist.

---

## Document Evolution Log

| Version | Date | Key Changes |
|---------|------|-------------|
| v1.0 | Original | Initial vision from GRAMMAR.md |
| v2.0 | 2026-01-14 | Eureka Iteration 1 - Foundation Review |

---

# EUREKA ITERATION 1: Foundation Review

## Discovery Focus: Core Assumptions Validation

### Original Assumptions (from GRAMMAR.md v0.1.0)

1. **Syntax Model**: Ruby-like with `end` terminators, Python-like expressiveness
2. **Type System**: Hindley-Milner inference + Rust-level safety
3. **Memory Model**: Ownership inference without explicit annotations
4. **Concurrency**: Effect-inferred async with goroutine-style threading
5. **Testing**: Built-in contracts, properties, examples
6. **LLM Integration**: Verified AI-assisted optimization in compilation
7. **Targets**: Native (LLVM/Cranelift), WASM, browser

### Eureka Finding 1.1: Syntax Trade-offs

**Original Assumption**: Ruby `end` keyword syntax provides clarity.

**Analysis**:
- Ruby `end` style: Clear block termination, but verbose
- Python indentation: Clean, but significant whitespace has tooling challenges
- Rust `{}` braces: Universal, familiar to C/Java developers
- Go style: Minimal, but controversial omissions (semicolons, parentheses)

**Options to Preserve**:

| Option | Syntax Style | Pros | Cons |
|--------|--------------|------|------|
| A (Current) | Ruby `end` | Readable, unambiguous | Verbose, unfamiliar to C devs |
| B | Brace-based | Universal, tool-friendly | Less "elegant" feel |
| C | Hybrid | `end` for large blocks, `{}` for lambdas | Inconsistency |
| D | Indentation | Clean, Pythonic | Tooling complexity, paste issues |

**Key Finding**: Current choice (Option A) is valid but may limit adoption from C/Rust communities. Consider Option C for pragmatism.

---

### Eureka Finding 1.2: Type System Scope

**Original Assumption**: Hindley-Milner inference with Rust-level safety.

**Analysis**:
- Full Hindley-Milner has decidability guarantees but limits expressiveness
- Rust uses bidirectional type checking, not pure H-M
- Modern languages (TypeScript, Kotlin) use flow-sensitive typing
- Dependent types for contracts (Idris, Dafny) add power but complexity

**Options to Preserve**:

| Option | Type System | Inference Power | Contract Support | Complexity |
|--------|-------------|-----------------|------------------|------------|
| A | Pure H-M | Full | Limited | Low |
| B (Current) | Extended H-M + Traits | High | Moderate | Medium |
| C | Bidirectional + Flow | High | Good | Medium-High |
| D | Dependent Types | Moderate | Excellent | High |

**Key Finding**: Current approach (B) is reasonable. However, for built-in contracts, Option C or hybrid B+D may be needed.

---

### Eureka Finding 1.3: Memory Management Reality Check

**Original Assumption**: Ownership inference without annotations (like Rust but implicit).

**Critical Analysis**:
- Rust's explicit annotations exist for good reason: disambiguation
- Swift ARC is implicit but has performance overhead (reference counting)
- Vale's region-based approach is novel but unproven at scale
- Lobster (game language) uses lifetime inference successfully but for limited scope

**Options to Preserve**:

| Option | Memory Model | Annotation Burden | Performance | Complexity |
|--------|--------------|-------------------|-------------|------------|
| A | Explicit ownership (Rust) | High | Optimal | High learning curve |
| B (Original) | Inferred ownership | Low | TBD | High compiler complexity |
| C | ARC with escape analysis | Low | Good (not optimal) | Medium |
| D | Region-based | Medium | Good | Novel, risky |
| E | Hybrid: Inferred default, explicit opt-in | Medium | Near-optimal | Best of both |

**Breakthrough Finding**: Option E (Hybrid) appears most viable. Infer ownership for 80% of cases, allow explicit annotations for performance-critical code.

---

### PRD v2 Requirements Update

#### REQ-SYNTAX-001: Block Termination Strategy
**Priority**: High
**Status**: Under Review
**Options**:
- **Recommended**: Option C (Hybrid) - `end` for multi-line blocks, braces for inline/lambdas
- **Alternative**: Option A (Current Ruby-style)

#### REQ-TYPE-001: Type Inference System
**Priority**: Critical
**Status**: Confirmed with Extension
**Requirement**: Extended Hindley-Milner with:
- Bidirectional type checking for better error messages
- Flow-sensitive narrowing (like TypeScript/Kotlin)
- Contract-aware refinement types (for requires/ensures)

#### REQ-MEM-001: Memory Management Strategy
**Priority**: Critical
**Status**: Revised
**Requirement**: Hybrid Ownership Model
- Default: Compiler-inferred ownership (best-effort)
- Explicit: `own`, `ref`, `mut ref` annotations when needed
- Escape hatch: ARC for complex shared state
- Target: 80% code requires zero annotations

---

## Metrics: Iteration 1

- **Assumptions Validated**: 4/7
- **Assumptions Requiring Revision**: 3/7 (Syntax, Type System depth, Memory model)
- **New Options Identified**: 5
- **Breakthrough Candidates**: 1 (Hybrid memory model)

---

# EUREKA ITERATION 2: Concurrency & Effect System Deep Dive

## Discovery Focus: Concurrency Model Feasibility

### Original Assumptions Review

1. **Effect-inferred async**: Compiler detects async operations automatically
2. **Goroutine-style threading**: Green threads managed by runtime
3. **Channels**: Go-style communication

### Eureka Finding 2.1: Effect Inference Complexity

**Analysis of "Effect-Inferred Async"**:

The original assumption that the compiler can "infer" async operations is partially flawed:

- **What works**: Detecting I/O operations, network calls, file operations
- **What's hard**: User-defined effects, callback-based APIs, FFI boundaries
- **What's impossible**: Runtime-determined behavior

**Research from Koka, Eff, Frank languages**:
- Algebraic effects provide a principled foundation
- But require explicit effect annotations in function signatures
- "Effect inference" is possible but produces noisy types

**Options to Preserve**:

| Option | Concurrency Model | Effect Tracking | Ease of Use | Performance |
|--------|-------------------|-----------------|-------------|-------------|
| A | Explicit async/await | None | Moderate | Good |
| B (Current) | Inferred async | Full | High (target) | Unknown |
| C | Algebraic effects | Full, explicit | Low | Excellent |
| D | Colored functions (like Rust) | Implicit | Moderate | Excellent |
| E | Goroutines (Go-style) | None | High | Good |

**Key Finding**: Pure inference (Option B) is too ambitious. Recommend Option D+E hybrid: colored functions with lightweight goroutine-style spawning.

---

### Eureka Finding 2.2: Runtime Requirements

**Critical Question**: Does Aria need a runtime?

| Feature | Requires Runtime | Can Be Compile-Time |
|---------|------------------|---------------------|
| Green threads | Yes | No |
| Garbage collection | Yes | No |
| Ownership tracking | No | Yes |
| Channel implementation | Minimal | Partially |
| Effect handlers | Depends | Depends |

**Trade-off Analysis**:

- **With Runtime**: Easier concurrency, potential GC, larger binaries, slower startup
- **Without Runtime**: Faster startup, smaller binaries, harder concurrency, Rust-like

**Options to Preserve**:

| Option | Runtime | Use Case Fit |
|--------|---------|--------------|
| A | Full runtime (like Go) | Server apps, long-running |
| B | Minimal runtime | CLI tools, mixed workloads |
| C (Current implied) | Optional runtime | Flexible but complex |
| D | No runtime (like Rust) | Systems, embedded, WASM |

**Key Finding**: Option B or C recommended. Runtime should be optional and pay-for-what-you-use.

---

### Eureka Finding 2.3: Select Statement Complexity

The current `select` statement (Go-style) is powerful but:
- Requires runtime support
- Complex to implement correctly (fairness, memory ordering)
- Not compatible with WASM's single-threaded model

**Recommendation**: Make `select` a standard library feature, not a language primitive. Allow target-specific implementations.

---

### PRD v2 Requirements Update

#### REQ-CONC-001: Concurrency Primitives
**Priority**: High
**Status**: Revised
**Requirement**:
- `spawn` for lightweight tasks (green threads with runtime, OS threads without)
- `async/await` for sequential async code
- `Channel<T>` in standard library (not language primitive)
- Effect annotations optional but supported: `fn fetch() -> String !IO`

#### REQ-CONC-002: Runtime Strategy
**Priority**: High
**Status**: New
**Options**:
- **Recommended**: Minimal optional runtime
- **Alternative A**: Full runtime (Go-style)
- **Alternative B**: No runtime (Rust-style)

**Decision Criteria**:
- If primary target is servers → Full runtime
- If primary target is CLI/WASM → Minimal/No runtime
- If targeting both → Optional runtime (more complex)

#### REQ-EFFECT-001: Effect System
**Priority**: Medium
**Status**: New
**Requirement**: Lightweight effect annotations
- Syntax: `fn name() -> T !Effect1, Effect2`
- Built-in effects: `IO`, `Async`, `Unsafe`, `Panic`
- Custom effects: User-definable
- Inference: Best-effort, explicit when ambiguous

---

## Metrics: Iteration 2

- **Assumptions Validated**: 1/3 (Channels)
- **Assumptions Requiring Revision**: 2/3 (Effect inference, Runtime model)
- **New Requirements**: 3
- **Complexity Concerns**: High (effect system + runtime optionality)

---

# EUREKA ITERATION 3: Contract System & Verification

## Discovery Focus: Design by Contract Implementation

### Original Assumptions Review

1. **Built-in contracts**: `requires`, `ensures`, `invariant` as language keywords
2. **Property-based testing**: `forall`, `exists` quantifiers in tests
3. **Examples blocks**: Inline test cases with functions

### Eureka Finding 3.1: Contract Checking Modes

**Key Question**: When are contracts checked?

| Mode | Compile-Time | Runtime | Use Case |
|------|--------------|---------|----------|
| Off | No | No | Production (max performance) |
| Runtime | No | Yes | Development, testing |
| Static | Yes (limited) | No | CI, formal verification |
| Full | Yes | Yes | Safety-critical code |

**Current GRAMMAR.md** doesn't specify this. Critical gap.

**Implementation Options**:

| Option | Default Mode | Configurability | Complexity |
|--------|--------------|-----------------|------------|
| A | Runtime only | Per-build | Low |
| B | Static where possible, runtime fallback | Per-function | Medium |
| C | Full verification (Dafny-style) | Global | High |
| D | Off by default, opt-in | Per-contract | Low |

**Recommendation**: Option B - Static verification where decidable, runtime checks otherwise. User controls via attributes.

---

### Eureka Finding 3.2: Contract Expression Power

**Current Grammar allows**:
```ruby
requires arr.sorted? : "array must be sorted"
ensures forall i: Int where 0 <= i < arr.length, arr[i] == target implies result == Some(i)
```

**Analysis**:
- `arr.sorted?` requires runtime method call (not statically verifiable without theorem prover)
- `forall` quantifiers are undecidable in general
- `old(x)` requires capturing previous state (memory overhead)

**Options for Contract Power vs. Verifiability**:

| Option | Expression Power | Static Verifiability | Runtime Overhead |
|--------|------------------|----------------------|------------------|
| A | Full (current) | Low | High |
| B | Restricted to decidable | High | Low |
| C | Tiered (simple/complex) | Medium | Medium |
| D | SMT-backed (Z3 integration) | High | Compile-time |

**Key Finding**: Option C (Tiered) balances usability and verifiability:
- **Tier 1**: Simple predicates (null checks, bounds, type guards) - fully static
- **Tier 2**: Method calls on immutable data - static with caching
- **Tier 3**: Quantifiers, complex expressions - runtime or SMT-backed

---

### Eureka Finding 3.3: Property Testing Integration

**Insight**: Property blocks and contract blocks have different semantics:
- **Contracts**: Must hold for every call (runtime checked)
- **Properties**: Must hold for sampled inputs (test-time checked)

**Current grammar conflates these**. Should clarify:

```ruby
# Contract (checked on every call)
fn sqrt(x: Float) -> Float
  requires x >= 0.0
  ensures result * result ~= x  # ~= means approximately equal
end

# Property (checked during testing with random inputs)
property "sqrt is inverse of square"
  forall x: Float where x >= 0.0
    sqrt(x * x) ~= x.abs
end
```

---

### PRD v2 Requirements Update

#### REQ-CONTRACT-001: Contract Checking Modes
**Priority**: Critical
**Status**: New
**Requirement**: Support multiple contract enforcement modes:
```ruby
@contracts(:static)  # Compile-time verification where possible
@contracts(:runtime) # Always check at runtime (default for debug)
@contracts(:off)     # No checks (production default)
@contracts(:full)    # Static + runtime (safety-critical)
```

#### REQ-CONTRACT-002: Contract Expression Tiers
**Priority**: High
**Status**: New
**Requirement**: Three-tier contract system:
- **Tier 1 (Static)**: Null checks, bounds, type guards, simple arithmetic
- **Tier 2 (Cached)**: Pure method calls, array access, field access
- **Tier 3 (Dynamic)**: Quantifiers, closures, complex expressions

#### REQ-TEST-001: Property vs Contract Distinction
**Priority**: Medium
**Status**: New
**Requirement**: Clear semantic separation:
- `requires/ensures/invariant` → Runtime/static contracts (every call)
- `property` → Test-time properties (sampled)
- `examples` → Concrete test cases (exact inputs)

---

## Metrics: Iteration 3

- **Assumptions Validated**: 1/3 (Examples blocks)
- **Assumptions Requiring Revision**: 2/3 (Contract checking, Property semantics)
- **New Requirements**: 3
- **Breakthrough**: Tiered contract system for practical verification

---

# EUREKA ITERATION 4: LLM Optimization Pipeline

## Discovery Focus: AI-Assisted Compilation Feasibility

### Original Assumptions Review

1. **LLM in compilation**: AI suggests optimizations during compile time
2. **Verification**: Formal methods ensure LLM suggestions are correct
3. **Determinism**: Same source always produces same output

### Eureka Finding 4.1: LLM Non-Determinism Problem

**Critical Issue**: LLMs are inherently non-deterministic.

Even with:
- Temperature = 0
- Same prompt
- Same model version

Output can vary due to:
- Floating-point rounding in different hardware
- Model updates
- API changes

**Impact on Compiler Design**:
- Non-deterministic builds are unacceptable for production
- Reproducible builds are a hard requirement

**Options to Preserve**:

| Option | LLM Usage | Determinism | Practicality |
|--------|-----------|-------------|--------------|
| A (Current) | Compile-time optimization | Broken | Impractical |
| B | Development-time suggestions | OK (not in build) | Good |
| C | Cached/versioned LLM outputs | Deterministic | Complex |
| D | LLM-trained static analyzer | Deterministic | Good |
| E | Opt-in with hash verification | Conditional | Moderate |

**Key Finding**: Option B or D recommended. LLM should assist developers, not be part of the build pipeline.

---

### Eureka Finding 4.2: Verification Complexity

**Original assumption**: "Formal verification of LLM suggestions"

**Reality Check**:
- Full formal verification (theorem proving) is:
  - Slow (minutes to hours per function)
  - Incomplete (can't verify all properties)
  - Requires specifications (someone must write them)

- Practical alternatives:
  - **Translation validation**: Verify input/output equivalence
  - **Differential testing**: Compare behavior on test suite
  - **Type-checked transformations**: Ensure types preserved

**Options for Verification**:

| Option | Method | Confidence | Speed |
|--------|--------|------------|-------|
| A | Full theorem proving | 100% | Very slow |
| B | SMT-based equivalence | High | Slow |
| C | Differential testing | Medium | Fast |
| D | Type preservation | Medium | Fast |
| E | Human review | Variable | Slow |

**Recommendation**: Option C+D combination. Differential testing with type preservation provides practical confidence.

---

### Eureka Finding 4.3: Practical LLM Integration Points

Where LLMs add value without breaking determinism:

| Integration Point | Value | Determinism Impact |
|-------------------|-------|-------------------|
| IDE suggestions | High | None (not in build) |
| Documentation generation | Medium | None |
| Test generation | High | None (test-time) |
| Error message improvement | Medium | None (development) |
| Code review | High | None |
| Performance profiling hints | High | None |
| **Compile-time optimization** | Medium | **Broken** |

**Key Finding**: LLM integration should be in tooling, not compiler core.

---

### PRD v2 Requirements Update

#### REQ-LLM-001: LLM Integration Strategy
**Priority**: High
**Status**: Revised (Major Change)
**Requirement**: LLM assists development, NOT compilation:
- **In scope**: IDE suggestions, documentation, test generation, error messages
- **Out of scope**: Compile-time code transformation

#### REQ-LLM-002: Developer-Time Optimization Assistant
**Priority**: Medium
**Status**: New
**Requirement**: `aria optimize` CLI tool that:
- Analyzes code with LLM
- Suggests optimizations with explanations
- Developer approves/rejects changes
- Changes committed to source (deterministic build)

#### REQ-BUILD-001: Reproducible Builds
**Priority**: Critical
**Status**: Confirmed
**Requirement**: Same source + same compiler version = identical output
- No runtime LLM calls during compilation
- All optimizations must be deterministic

---

## Metrics: Iteration 4

- **Assumptions Validated**: 0/3
- **Assumptions Requiring Revision**: 3/3 (All LLM assumptions revised)
- **Major Pivot**: LLM moves from compiler to tooling
- **New Requirements**: 3
- **Risk Mitigated**: Non-deterministic builds

---

# EUREKA ITERATION 5: FFI & Interoperability

## Discovery Focus: Cross-Language Integration

### Original Assumptions Review

1. **C FFI**: Direct header import like Zig's `@cImport`
2. **Python interop**: Zero-copy bridge
3. **WASM exports**: First-class support

### Eureka Finding 5.1: C Header Import Complexity

**Zig's `@cImport` Analysis**:

How it works:
1. Invokes libclang to parse C headers
2. Translates C types to Zig types
3. Available at compile-time

**Challenges for Aria**:
- Requires shipping libclang (100MB+ dependency)
- C preprocessing is complex (macros, conditionals)
- Platform-specific headers vary
- Some C patterns have no safe equivalent

**Options to Preserve**:

| Option | Approach | Complexity | C Compatibility |
|--------|----------|------------|-----------------|
| A (Zig-style) | Direct clang integration | High | Excellent |
| B (Rust-style) | bindgen tool + manual | Medium | Good |
| C (Go-style) | CGo with inline C | Low | Good |
| D | Auto-generated bindings | Medium | Good |
| E | Subset of C only | Low | Limited |

**Recommendation**: Start with Option B (tooling-based), migrate to Option A later.

---

### Eureka Finding 5.2: Python Interop Reality

**"Zero-copy" claim analysis**:

- Python's GIL prevents true parallel access
- Python objects have reference counting overhead
- NumPy arrays can be zero-copy (underlying buffer)
- Python strings are immutable (copy on modification)

**Realistic Python Interop**:

| Data Type | Zero-Copy Possible | Notes |
|-----------|-------------------|-------|
| NumPy arrays | Yes | Via buffer protocol |
| Python lists | No | Must convert |
| Python dicts | No | Must convert |
| Python strings | Partial | Read-only zero-copy |
| Custom objects | No | Serialization needed |

**Options to Preserve**:

| Option | Approach | Performance | Ease of Use |
|--------|----------|-------------|-------------|
| A | PyO3-style (Rust) | Good | Medium |
| B | Cython-style | Excellent | Low |
| C | JSON/MessagePack bridge | Poor | High |
| D | Native extension API | Good | Low |
| E | WASM-based isolation | Good | Medium |

**Recommendation**: Option A (PyO3-style) provides best balance. "Zero-copy" only for compatible types.

---

### Eureka Finding 5.3: WASM Limitations

**WASM constraints not addressed in GRAMMAR.md**:

| Feature | WASM Support | Workaround |
|---------|--------------|------------|
| Threads | Limited (SharedArrayBuffer) | Single-threaded fallback |
| Filesystem | None | Virtual FS / browser APIs |
| Network | None | fetch() via JS |
| System calls | None | JS bridge |
| 64-bit integers | Yes | Native |
| SIMD | Yes (recent) | Feature detection |

**Impact on Aria**:
- `spawn` must have WASM-specific implementation
- Channels need WASM-compatible implementation
- File/network operations need JS interop

---

### PRD v2 Requirements Update

#### REQ-FFI-C-001: C Interoperability
**Priority**: High
**Status**: Revised
**Requirement**: Two-phase approach:
- **Phase 1**: `aria-bindgen` tool for header processing
- **Phase 2**: Direct `@cImport` (Zig-style) when stable

Syntax:
```ruby
# Phase 1: External tool generates bindings
# $ aria-bindgen sqlite3.h -o sqlite3.aria
import ./bindings/sqlite3

# Phase 2 (future): Direct import
extern C from "sqlite3.h"
```

#### REQ-FFI-PY-001: Python Interoperability
**Priority**: Medium
**Status**: Revised
**Requirement**: PyO3-inspired bridge with honest capabilities:
- Zero-copy for: NumPy arrays, bytes, memoryview
- Conversion required for: lists, dicts, strings, objects
- GIL management: Explicit `with py.gil()` blocks

#### REQ-WASM-001: WASM Target Requirements
**Priority**: High
**Status**: New
**Requirement**: WASM-specific adaptations:
- Single-threaded mode for `spawn` (cooperative)
- Virtual filesystem abstraction
- JS interop for browser APIs
- Feature detection for SIMD, threads

---

## Metrics: Iteration 5

- **Assumptions Validated**: 0/3 (All required revision)
- **Assumptions Requiring Revision**: 3/3
- **Complexity Adjustments**: C FFI phased, Python "zero-copy" clarified
- **New Requirements**: 3

---

# EUREKA ITERATION 6: Error Handling & Safety

## Discovery Focus: Result Types and Error Propagation

### Original Assumptions Review

From GRAMMAR.md:
1. **Result type**: `Result<T, E>` with `Ok`/`Err` variants
2. **Option type**: `T?` shorthand for `Option<T>`
3. **Error propagation**: `?` operator
4. **Panic**: For unrecoverable errors

### Eureka Finding 6.1: Error Type Hierarchy

**Current grammar lacks**:
- Standard error trait/type
- Error chaining/context
- Stack traces
- Error codes for FFI

**Comparison with other languages**:

| Language | Error Model | Error Info | Chaining |
|----------|-------------|------------|----------|
| Rust | Result + Error trait | Type + message | Via anyhow/thiserror |
| Go | Multiple returns | String only | fmt.Errorf wrapping |
| Swift | throws + Error protocol | Flexible | Limited |
| Kotlin | Exceptions | Full stack | Standard |
| Zig | Error unions | Enum values | No |

**Options to Preserve**:

| Option | Error Model | Richness | Performance |
|--------|-------------|----------|-------------|
| A | Rust-style traits | High | Good |
| B | Go-style simplicity | Low | Excellent |
| C | Exception-based | High | Poor |
| D | Zig-style error unions | Medium | Excellent |
| E | Hybrid: Result + exceptions for panic | High | Good |

**Recommendation**: Option A with Option D influence. Error trait with compile-time known error sets.

---

### Eureka Finding 6.2: The `?` Operator Edge Cases

**Current grammar**: `?` for early return on error.

**Unaddressed cases**:
1. `?` in main function (what's the return type?)
2. `?` with different error types (conversion?)
3. `?` in lambdas/closures
4. `?` in async contexts

**Proposed Semantics**:

```ruby
# 1. Main function
fn main -> Result<Unit, Error>  # Explicit, or default to Int exit code

# 2. Error type conversion (auto via Into trait)
fn example() -> Result<String, AppError>
  let data = read_file()?  # IoError -> AppError via Into
end

# 3. Lambdas (error type inferred)
items.map { |i| parse(i)? }  # Returns Array<Result<T, E>>

# 4. Async (error preserved through await)
let result = fetch_data().await?
```

---

### Eureka Finding 6.3: Panic vs Error Distinction

**Key insight**: Aria needs clear panic/error boundary.

| Situation | Should Be | Reason |
|-----------|-----------|--------|
| File not found | Error (Result) | Recoverable |
| Index out of bounds | Panic | Logic error |
| Division by zero | Configurable | Both valid |
| OOM | Panic | Unrecoverable |
| Contract violation | Configurable | Depends on mode |
| Integer overflow | Configurable | Debug vs release |

**Recommendation**: Configuration attribute for panic vs error behavior:

```ruby
@overflow(:panic)    # Panic on overflow (debug default)
@overflow(:wrap)     # Wrap on overflow (release default)
@overflow(:saturate) # Saturate on overflow

@bounds(:panic)      # Panic on out-of-bounds (always)
@bounds(:check)      # Return Option (explicit)
```

---

### PRD v2 Requirements Update

#### REQ-ERR-001: Standard Error Trait
**Priority**: High
**Status**: New
**Requirement**: Define `Error` trait in prelude:
```ruby
trait Error: Display
  fn source(self) -> Error?     # Cause chain
  fn backtrace(self) -> Trace?  # Stack trace (debug only)
end
```

#### REQ-ERR-002: Error Propagation Semantics
**Priority**: High
**Status**: New
**Requirement**: `?` operator behavior:
- Auto-converts error types via `Into` trait
- Works in all contexts (functions, lambdas, async)
- Main function defaults to `Result<Unit, Box<Error>>`

#### REQ-SAFETY-001: Panic Configuration
**Priority**: Medium
**Status**: New
**Requirement**: Per-operation panic/error configuration:
- Overflow: `@overflow(:panic | :wrap | :saturate)`
- Bounds: `@bounds(:panic | :check)`
- Contracts: `@contracts(:panic | :error | :off)`

---

## Metrics: Iteration 6

- **Assumptions Validated**: 2/4 (Result, Option types)
- **Assumptions Requiring Revision**: 2/4 (Error hierarchy, Panic semantics)
- **New Requirements**: 3
- **Clarifications**: Error propagation edge cases

---

# EUREKA ITERATION 7: Standard Library & Ecosystem

## Discovery Focus: What Ships with Aria?

### Original Assumptions Review

From GRAMMAR.md Appendix C:
- Types: Int, Float, Bool, String, Array, Map, Set, Option, Result
- Traits: Eq, Ord, Hash, Clone, Display, Iterator, From/Into
- Functions: print, assert, panic
- "Standard library prelude" concept

### Eureka Finding 7.1: Standard Library Scope

**Critical Question**: How big should std be?

| Approach | Examples | Pros | Cons |
|----------|----------|------|------|
| Minimal | Go (small std) | Fast compilation, flexibility | Fragmented ecosystem |
| Batteries-included | Python (large std) | Convenient, consistent | Slow evolution, bloat |
| Tiered | Rust (std + crates) | Flexible, official quality | Discoverability |
| Platform-specific | .NET (massive BCL) | Comprehensive | Lock-in, size |

**Options to Preserve**:

| Option | Std Size | Package Ecosystem |
|--------|----------|-------------------|
| A | Minimal (core types only) | Heavy reliance |
| B | Moderate (core + I/O + networking) | Balanced |
| C | Large (everything) | Minimal reliance |
| D | Tiered (std, std-extended, third-party) | Most flexible |

**Recommendation**: Option D (Tiered):
- `std::core` - Types, traits, memory (always available)
- `std::io` - File, console, streams
- `std::net` - HTTP, TCP, UDP
- `std::json`, `std::xml` - Serialization
- Third-party for specialized needs

---

### Eureka Finding 7.2: Async Runtime in Std?

**Contentious decision**: Should async runtime be in standard library?

| Choice | Languages | Implication |
|--------|-----------|-------------|
| Yes, one runtime | Go, Python asyncio | Simplicity, consistency |
| No, pluggable | Rust (tokio/async-std) | Flexibility, fragmentation |
| Yes, pluggable | Kotlin (dispatchers) | Balance |

**For Aria**:
- Original vision suggests Go-style (simple, built-in)
- But WASM target needs different runtime
- Embedded targets may need no-std

**Recommendation**: Ship default runtime, allow replacement:
```ruby
# Default (works everywhere)
spawn { do_work() }

# Custom runtime (advanced)
@runtime(MyCustomRuntime)
spawn { do_work() }
```

---

### Eureka Finding 7.3: Package Manager Requirements

**Milestone M17 addresses this, but key decisions**:

| Feature | Cargo (Rust) | npm (JS) | pip (Python) | Aria Choice |
|---------|--------------|----------|--------------|-------------|
| Lock file | Yes | Yes | Optional | Required |
| Semantic versioning | Enforced | Convention | Weak | Enforced |
| Security audits | cargo-audit | npm audit | safety | Built-in |
| Build scripts | build.rs | package.json | setup.py | aria.build |
| Binary caching | Limited | No | No | Yes (like Go) |
| Reproducible builds | Yes | Complex | No | Required |

**Recommendation**: Cargo-inspired with improvements:
- `aria.toml` for project config
- `aria.lock` for reproducibility (committed to VCS)
- Built-in security scanning
- Binary caching for fast CI

---

### PRD v2 Requirements Update

#### REQ-STD-001: Standard Library Tiers
**Priority**: High
**Status**: New
**Requirement**: Three-tier standard library:
- **Tier 1 (Core)**: Types, traits, memory - always available
- **Tier 2 (Standard)**: I/O, networking, serialization - opt-in
- **Tier 3 (Extended)**: Database, crypto, etc. - separate packages

#### REQ-RUNTIME-001: Async Runtime
**Priority**: High
**Status**: New
**Requirement**:
- Default runtime ships with std
- Pluggable for specialized environments
- WASM-compatible runtime variant

#### REQ-PKG-001: Package Manager Basics
**Priority**: High
**Status**: New
**Requirement**: `aria` CLI includes package management:
- `aria new` - Create project
- `aria add <pkg>` - Add dependency
- `aria build` - Build project
- `aria test` - Run tests
- `aria publish` - Publish package
- Lock file required, SemVer enforced

---

## Metrics: Iteration 7

- **Assumptions Validated**: 2/3 (Types, Traits)
- **Assumptions Requiring Revision**: 1/3 (Stdlib scope)
- **New Requirements**: 3
- **Ecosystem Decisions**: Tiered std, pluggable runtime, Cargo-inspired packages

---

# EUREKA ITERATION 8: Developer Experience & Tooling

## Discovery Focus: IDE, Debugging, Ergonomics

### Original Assumptions Review

From Milestones:
- M18: IDE Integration (LSP, debugging, profiling)
- M19: Syntax Refinement (ergonomics)

### Eureka Finding 8.1: LSP Complexity

**Language Server Protocol requirements**:

| Feature | Complexity | Priority |
|---------|------------|----------|
| Syntax highlighting | Low | P0 |
| Go to definition | Medium | P0 |
| Find references | Medium | P1 |
| Completions | High | P0 |
| Hover info | Medium | P0 |
| Diagnostics | High | P0 |
| Code actions | High | P1 |
| Rename | High | P2 |
| Formatting | Medium | P1 |
| Inlay hints | Medium | P2 |

**Challenge**: Aria's type inference means LSP needs full type checker.

**Options**:

| Option | LSP Architecture | Responsiveness | Accuracy |
|--------|------------------|----------------|----------|
| A | Full recompilation | Slow | Perfect |
| B | Incremental (rust-analyzer) | Fast | Near-perfect |
| C | Separate lightweight checker | Fast | Approximate |

**Recommendation**: Option B (Incremental). rust-analyzer architecture is proven.

---

### Eureka Finding 8.2: Error Message Quality

**Critical DX differentiator**: Error messages.

**Best-in-class examples**:
- Elm: Conversational, helpful suggestions
- Rust: Detailed, with fix suggestions
- TypeScript: Context-aware, IDE integration

**Requirements for Aria**:
1. Point to exact source location
2. Explain what went wrong (not just "type mismatch")
3. Suggest fixes when possible
4. Show relevant context
5. Link to documentation

**Example good vs bad**:

```
# BAD
Error: Type mismatch at line 42

# GOOD
Error[E0308]: expected `String`, found `Int`
  --> src/main.aria:42:10
   |
42 |   let name: String = get_id()
   |       ----           ^^^^^^^^ expected `String`, found `Int`
   |       |
   |       expected due to this type annotation
   |
Help: consider converting the Int to String:
   |
42 |   let name: String = get_id().to_s
   |                              +++++
```

---

### Eureka Finding 8.3: REPL and Interactive Development

**Question**: Should Aria have a REPL?

| Aspect | Pro-REPL | Anti-REPL |
|--------|----------|-----------|
| Exploratory coding | Essential | Notebooks alternative |
| Learning | Great for beginners | Documentation sufficient |
| Compiled language fit | Unusual | JIT required |
| Type system | Complex state | Simpler |

**Options**:

| Option | Interactive Mode | Implementation |
|--------|------------------|----------------|
| A | Full REPL | Interpreter mode (complex) |
| B | Compile-and-run | JIT or fast compiler |
| C | Notebook integration | Like Swift Playgrounds |
| D | No REPL | Focus on fast compile |

**Recommendation**: Option B or C. Fast compile-and-run with optional notebook support.

---

### PRD v2 Requirements Update

#### REQ-IDE-001: Language Server
**Priority**: Critical
**Status**: New
**Requirement**: First-class LSP implementation:
- Incremental compilation for responsiveness
- All standard LSP features (P0 features at launch)
- IDE-agnostic (VSCode, Neovim, etc.)

#### REQ-DX-001: Error Messages
**Priority**: Critical
**Status**: New
**Requirement**: Elm/Rust-quality error messages:
- Precise source locations with context
- Plain English explanations
- Fix suggestions where determinable
- Documentation links

#### REQ-DX-002: Interactive Development
**Priority**: Medium
**Status**: New
**Options**:
- **Recommended**: Fast compile-run cycle (`aria run --watch`)
- **Alternative**: Notebook/playground integration
- **Not planned**: Traditional REPL

---

## Metrics: Iteration 8

- **Assumptions Validated**: 1/2 (IDE integration concept)
- **Assumptions Requiring Revision**: 1/2 (REPL expectation)
- **New Requirements**: 3
- **DX Priorities**: Error messages, incremental LSP, fast iteration

---

# EUREKA ITERATION 9: Implementation Strategy & Phasing

## Discovery Focus: What to Build First?

### Holistic Review of All Requirements

After 8 iterations, we have identified:
- **Critical requirements**: Type system, memory model, error handling, LSP
- **Revised assumptions**: LLM (to tooling), Runtime (optional), FFI (phased)
- **New requirements**: Contract tiers, effect annotations, reproducible builds

### Eureka Finding 9.1: MVP Definition

**What's the Minimal Viable Aria?**

| Feature | MVP | V1.0 | Future |
|---------|-----|------|--------|
| Core types (Int, String, Array, etc.) | Yes | Yes | Yes |
| Type inference | Basic | Full | Extended |
| Pattern matching | Basic | Full | Full |
| Result/Option | Yes | Yes | Yes |
| Contracts | Runtime only | Tiered | Full verification |
| Effects | None | Annotation | Inference |
| FFI (C) | Manual bindings | Bindgen tool | @cImport |
| FFI (Python) | None | Basic | Full |
| WASM | Basic | Full | Optimized |
| Concurrency | spawn/await | Channels | Full runtime |
| Package manager | Basic | Full | Advanced |
| LSP | Minimal | Full | IntelliJ plugin |
| Error messages | Good | Excellent | AI-assisted |

---

### Eureka Finding 9.2: Critical Path Dependencies

```
Phase 1: Bootstrapping
├── Lexer
├── Parser
├── AST
└── Basic type checker

Phase 2: Core Compilation
├── Type inference engine
├── Ownership analysis
├── IR generation
└── LLVM backend (or Cranelift)

Phase 3: Safety Features
├── Contract runtime
├── Pattern exhaustiveness
├── Error propagation
└── Basic LSP

Phase 4: Ecosystem
├── Package manager
├── Standard library
├── Documentation generator
└── Full LSP

Phase 5: Advanced
├── WASM backend
├── FFI tooling
├── Effect system
└── Advanced optimizations
```

---

### Eureka Finding 9.3: Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Type system too complex | Medium | High | Start with simpler inference, extend |
| Ownership inference fails | High | High | Hybrid model with explicit fallback |
| Performance not competitive | Medium | High | Benchmark early, optimize bottlenecks |
| Ecosystem bootstrap | High | Medium | Excellent std library, easy FFI |
| Adoption | High | High | Killer feature focus (contracts + DX) |

---

### Final PRD v2 Requirements Update

#### REQ-IMPL-001: Phased Implementation
**Priority**: Critical
**Status**: Final
**Requirement**: Five-phase implementation roadmap (see above)

#### REQ-IMPL-002: MVP Scope
**Priority**: Critical
**Status**: Final
**Requirement**: MVP includes:
- Core language (types, functions, pattern matching)
- Basic type inference
- Runtime contracts only
- Manual FFI bindings
- Basic LLVM backend
- Minimal LSP

#### REQ-IMPL-003: Differentiators
**Priority**: High
**Status**: Final
**Requirement**: Focus development on key differentiators:
1. **Contracts**: Make Design by Contract genuinely usable
2. **Error messages**: Best-in-class developer feedback
3. **Syntax**: Clean, Ruby-inspired readability
4. **Safety**: Ownership without annotation burden

---

## Metrics: Iteration 9 (Final)

- **Total Requirements Identified**: 27
- **Original Assumptions Validated**: 12/20 (60%)
- **Original Assumptions Revised**: 8/20 (40%)
- **Breakthrough Discoveries**: 3
  1. Hybrid ownership model
  2. Tiered contract system
  3. LLM → tooling pivot
- **Risk Items Identified**: 5

---

# Summary: PRD v2 Key Decisions

## Confirmed Design Choices

| Aspect | Decision | Confidence |
|--------|----------|------------|
| Syntax | Ruby-style with `end` (hybrid for lambdas) | Medium |
| Type System | Extended H-M + bidirectional + flow-sensitive | High |
| Memory | Hybrid: Inferred default, explicit opt-in | Medium |
| Concurrency | Colored functions + goroutine spawn | Medium |
| Contracts | Three-tier (static/cached/dynamic) | High |
| LLM | Tooling only, not in build | High |
| FFI | Phased (manual → bindgen → @cImport) | High |
| Error Handling | Result + Error trait + ? operator | High |
| Standard Library | Tiered (core/standard/extended) | High |
| Package Manager | Cargo-inspired with improvements | High |

## Open Questions for Future Iterations

1. **Syntax**: Final decision on `end` vs braces vs hybrid
2. **Runtime**: Exact scope of runtime requirements
3. **Effect System**: Full algebraic effects vs lightweight annotations
4. **Verification**: SMT integration scope
5. **Governance**: Open source model, contribution guidelines

## Implementation Priority Order

1. **P0**: Lexer, Parser, Type inference, Basic compilation
2. **P1**: Ownership analysis, Contracts (runtime), LSP (basic)
3. **P2**: Full LSP, Package manager, WASM backend
4. **P3**: FFI tooling, Effect system, Advanced optimization
5. **P4**: LLM tooling, Full verification, Ecosystem growth

---

**Document Status**: Active
**Next Review**: After prototype Phase 1 completion
**Maintainer**: aria-lang core team
