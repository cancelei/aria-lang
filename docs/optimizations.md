# Aria Compiler Optimizations

This document describes the optimization passes implemented in the Aria compiler to achieve performance competitive with Go and Rust.

## Overview

Aria implements a multi-pass optimization pipeline operating on MIR (Mid-level Intermediate Representation) before code generation. The optimization passes are designed to:

1. Reduce instruction count and code size
2. Eliminate redundant operations
3. Enable better CPU utilization
4. Minimize memory allocations
5. Leverage Cranelift's backend optimizations

## Optimization Levels

### `OptLevel::None`
No optimizations. Used for fastest compilation during development and debugging.

### `OptLevel::Basic`
Fast optimizations that don't significantly increase compile time:
- Constant folding
- Algebraic simplification
- Dead code elimination
- CFG simplification

### `OptLevel::Aggressive`
Slower, more thorough optimizations for release builds:
- All basic optimizations (run multiple times to fixed point)
- Copy propagation
- Function inlining
- Loop optimizations
- Bounds check elimination
- String optimizations

## Individual Optimization Passes

### 1. Constant Folding

**Purpose**: Evaluate constant expressions at compile time.

**Examples**:
```aria
# Before
let x = 5 + 3
let y = 2 * 4

# After
let x = 8
let y = 8
```

**Implementation**:
- Detects binary operations on constant operands
- Evaluates operations at compile time
- Replaces operation with constant result
- Supports int, float, and boolean operations

**Benefits**:
- Eliminates runtime computation
- Reduces instruction count
- Enables further optimizations

### 2. Algebraic Simplification

**Purpose**: Simplify expressions using algebraic identities.

**Identities Applied**:
- `x + 0` → `x`
- `x * 1` → `x`
- `x * 0` → `0`
- `x - 0` → `x`
- `x / 1` → `x`
- `x ^ 0` → `x`
- `x - x` → `0`
- `x && true` → `x`
- `x || false` → `x`
- `x == x` → `true`
- `x != x` → `false`

**Benefits**:
- Reduces instruction count
- Simplifies control flow
- Enables dead code elimination

### 3. Dead Code Elimination (DCE)

**Purpose**: Remove unreachable code and unused assignments.

**Eliminates**:
- Unreachable basic blocks
- Assignments to variables that are never read
- Unused local variables

**Algorithm**:
1. Find reachable blocks via BFS from entry
2. Find used locals via def-use analysis
3. Remove assignments to unused locals
4. Mark unreachable blocks as empty

**Benefits**:
- Reduces code size
- Improves cache utilization
- Speeds up compilation

### 4. Copy Propagation

**Purpose**: Replace uses of copied values with the original.

**Example**:
```aria
# Before
let y = x
let z = y

# After
let y = x
let z = x  # Uses x directly
```

**Algorithm**:
1. Build copy map: `dest -> source`
2. Resolve transitive copies
3. Substitute all uses

**Benefits**:
- Reduces register pressure
- Enables other optimizations
- May eliminate intermediate variables

### 5. CFG Simplification

**Purpose**: Simplify control flow graph structure.

**Optimizations**:
- Fold switches on constant discriminants
- Merge trivial blocks
- Eliminate redundant jumps

**Example**:
```aria
# Before
if true then
  doSomething()
end

# After
doSomething()  # Jump eliminated
```

**Benefits**:
- Reduces branch mispredictions
- Improves instruction cache utilization
- Simplifies later analysis

### 6. Function Inlining

**Purpose**: Replace function calls with the function body.

**Heuristics**:
- Always inline very small functions (< 10 instructions)
- Inline functions called exactly once
- Inline small functions with few call sites
- **Inline contract checks in release mode (critical!)**
- Respect `#[inline(always)]` and `#[inline(never)]`

**Configuration**:
```rust
InlineConfig {
    release_mode: true,     // Enable contract inlining
    max_inline_size: 100,   // Max function size to inline
    max_inline_depth: 8,    // Prevent excessive nesting
}
```

**Benefits**:
- Eliminates call overhead
- Enables interprocedural optimizations
- **Eliminates contract verification overhead in release builds**
- Improves locality

**Tradeoffs**:
- Increases code size
- May hurt instruction cache if overdone

### 7. Loop Optimizations

**Purpose**: Optimize loop performance.

#### 7.1 Loop-Invariant Code Motion (Hoisting)

Move computations that don't change in the loop outside the loop.

**Example**:
```aria
# Before
for i in 0..n do
  let limit = n * 2  # Computed every iteration!
  if i < limit then
    process(i)
  end
end

# After
let limit = n * 2  # Computed once
for i in 0..n do
  if i < limit then
    process(i)
  end
end
```

#### 7.2 Strength Reduction

Replace expensive operations with cheaper equivalents.

**Example**:
```aria
# Before
for i in 0..n do
  let x = i * 8  # Multiplication in loop
  arr[x] = value
end

# After
let x = 0
for i in 0..n do
  arr[x] = value
  x = x + 8  # Addition instead
end
```

#### 7.3 Loop Unrolling

Duplicate loop body to reduce branch overhead.

**Example**:
```aria
# Before
for i in 0..4 do
  process(i)
end

# After (unrolled)
process(0)
process(1)
process(2)
process(3)
```

**Benefits**:
- Reduces branch overhead
- Enables better instruction-level parallelism
- Improves pipeline utilization

### 8. Bounds Check Elimination

**Purpose**: Remove redundant array bounds checks.

**Example**:
```aria
# Before
if i < arr.len() then
  let x = arr[i]  # Bounds check still performed!
end

# After
if i < arr.len() then
  let x = arr[i]  # Check eliminated - already proven safe
end
```

**Algorithm**:
1. Track array lengths through SSA values
2. Use value range analysis to prove safety
3. Eliminate checks that are provably safe

**Benefits**:
- Eliminates unnecessary branches
- Reduces instruction count
- Critical for tight loops over arrays

### 9. String Optimizations

**Purpose**: Optimize string operations.

#### 9.1 Concatenation Chain Optimization

**Example**:
```aria
# Before
let s1 = a + b      # Allocation 1
let s2 = s1 + c     # Allocation 2
let s3 = s2 + d     # Allocation 3

# After
let s3 = concat(a, b, c, d)  # Single allocation
```

#### 9.2 String Slicing Optimization

Use string views instead of copies where possible.

**Benefits**:
- Reduces memory allocations
- Eliminates unnecessary copies
- Improves cache utilization

### 10. SIMD Vectorization (Future)

**Purpose**: Use SIMD instructions for data parallelism.

**Target Operations**:
- GC content calculation (scanning DNA sequences)
- Array operations (map, filter, reduce)
- Mathematical operations on arrays

**Example** (conceptual):
```rust
// Scalar version
fn gc_content(seq: &[u8]) -> f64 {
    let mut gc_count = 0;
    for &base in seq {
        if base == b'G' || base == b'C' {
            gc_count += 1;
        }
    }
    gc_count as f64 / seq.len() as f64
}

// SIMD version (AVX2, process 32 bytes at once)
fn gc_content_simd(seq: &[u8]) -> f64 {
    use std::arch::x86_64::*;
    // Process 32 bytes in parallel using AVX2
    // ...
}
```

## Integration with Cranelift

Aria's optimizations complement Cranelift's built-in optimizations:

### Aria (MIR-level)
- High-level semantic optimizations
- Language-specific optimizations (contracts, effects)
- Cross-function optimizations (inlining)

### Cranelift (IR-level)
- Register allocation
- Instruction selection
- Instruction scheduling
- Peephole optimizations
- Machine-specific optimizations

## Performance Targets

### BioFlow Benchmarks

Target: Within 2-3x of Go performance on:

1. **GC Content Calculation** (tight loop)
   - Hot path: Array access + conditional
   - Key optimizations: Bounds check elimination, loop optimization

2. **K-mer Counting** (HashMap)
   - Hot path: Hash computation + map access
   - Key optimizations: Inlining, strength reduction

3. **String Operations**
   - Hot path: Concatenation, slicing
   - Key optimizations: String optimization, allocation reduction

### Optimization Impact

Expected speedup from optimizations:

| Optimization | Expected Speedup |
|-------------|------------------|
| Constant Folding | 1.1-1.3x |
| Algebraic Simplification | 1.05-1.15x |
| Dead Code Elimination | 1.0-1.1x (code size) |
| Copy Propagation | 1.05-1.2x |
| Function Inlining | 1.2-2.0x |
| Loop Optimizations | 1.3-2.5x |
| Bounds Check Elimination | 1.1-1.5x (array-heavy) |
| String Optimizations | 1.2-3.0x (string-heavy) |
| **Combined (Aggressive)** | **2.0-5.0x** |

## Usage

### Command Line

```bash
# No optimization (fastest compilation)
aria build --opt-level 0

# Basic optimization (default)
aria build --opt-level 1
aria build  # Same as -O1

# Aggressive optimization (release)
aria build --opt-level 2
aria build --release  # Same as -O2
```

### API

```rust
use aria_mir::optimize::{optimize_program, OptLevel};
use aria_codegen::{inline_functions, InlineConfig};

// Basic optimization
optimize_program(&mut mir_program, OptLevel::Basic);

// Aggressive optimization with inlining
optimize_program(&mut mir_program, OptLevel::Aggressive);
let config = InlineConfig::release();
inline_functions(&mut mir_program, &config);

// Compile with Cranelift
let object = compile_to_object(&mir_program, Target::native())?;
```

## Benchmarking

Run the optimization benchmarks:

```bash
cargo bench --bench optimization_benchmarks
```

Compare with Go/Rust implementations:

```bash
cd examples/bioflow-rust
cargo bench
```

## Future Work

1. **Profile-Guided Optimization (PGO)**
   - Collect runtime profiles
   - Use feedback to guide inlining and optimization

2. **Interprocedural Analysis**
   - Whole-program optimization
   - Cross-module inlining

3. **Advanced Loop Optimizations**
   - Loop fusion
   - Loop interchange
   - Automatic vectorization (SIMD)

4. **Escape Analysis**
   - Stack-allocate objects that don't escape
   - Eliminate unnecessary heap allocations

5. **Devirtualization**
   - Replace virtual calls with direct calls
   - Enable inlining of trait methods

6. **Polyhedral Optimization**
   - Advanced loop nest optimization
   - Automatic parallelization

## References

- [Cranelift Documentation](https://cranelift.dev/)
- [LLVM Optimization Passes](https://llvm.org/docs/Passes.html)
- [Rust Compiler Optimization](https://doc.rust-lang.org/rustc/codegen-options/index.html)
- [Go Compiler Optimizations](https://github.com/golang/go/wiki/CompilerOptimizations)
