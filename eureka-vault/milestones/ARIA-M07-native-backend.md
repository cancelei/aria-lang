# Milestone M07: Native Backend

## Overview

Design Aria's native code generation targeting x86_64, ARM64, and other platforms via LLVM or Cranelift.

## Research Questions

1. LLVM vs Cranelift vs custom backend?
2. How do we achieve fast compilation times?
3. What optimization levels should we support?
4. How do we handle platform-specific features (SIMD, etc.)?

## Target Performance

- Compile time: < 1s for 10K lines (incremental)
- Runtime: Within 10% of C for compute-intensive code
- Binary size: Competitive with Go/Rust

## Competitive Analysis Required

| Compiler | Backend | Study Focus |
|----------|---------|-------------|
| Rust | LLVM | Quality codegen |
| Zig | LLVM + custom | Fast debug builds |
| Go | Custom | Fast compilation |
| Swift | LLVM | Optimization levels |
| Cranelift | Custom | WASM + native |

## Implementation Progress

### Cranelift Backend (COMPLETED - Jan 2026)
- [x] Backend choice: **Cranelift** selected for fast compilation and WASM support
- [x] `crates/aria-codegen/` with full Cranelift integration
- [x] MIR to Cranelift IR translation
- [x] Function compilation with locals, parameters, returns
- [x] Control flow: if/else, match, loops (while, for, loop)
- [x] Expression compilation: arithmetic, comparison, logical ops
- [x] Type compilation: integers, floats, booleans, strings, arrays, tuples
- [x] Effect compilation: Console, IO, Async effect bridging
- [x] Object file generation via Cranelift's object backend
- [x] Linking via gcc with aria_runtime.o

### Compiler CLI (COMPLETED - Jan 2026)
- [x] `aria build <file>` command for native compilation
- [x] `aria run <file>` command for interpretation
- [x] Pipeline: parse → typecheck → MIR lower → codegen → link
- [x] Error reporting with source locations

### Architecture Decision Rationale
**Cranelift chosen over LLVM because:**
1. Significantly faster compilation (critical for developer experience)
2. Native Rust integration (simpler build, no C++ dependencies)
3. Same backend works for both native and WASM targets
4. Good enough optimization for most use cases
5. Easier to embed and customize

## Tasks

### ARIA-M07-01: Compare LLVM vs Cranelift
- **Description**: In-depth comparison for Aria's needs
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Status**: COMPLETED (Cranelift selected)
- **Tags**: research, compiler, backend, comparison
- **Deliverables**:
  - ~~Feature comparison matrix~~ Decision documented above
  - ~~Compilation speed benchmarks~~ Cranelift known for speed
  - ~~Ecosystem integration analysis~~ Rust ecosystem fit

### ARIA-M07-02: Study Zig's debug build speed
- **Description**: Analyze how Zig achieves fast debug builds
- **Dependency**: AGENT_CAPABLE
- **Priority**: normal (deferred)
- **Tags**: research, compiler, performance, zig
- **Deliverables**:
  - Debug build strategy
  - Optimization deferral patterns
  - Incremental compilation

### ARIA-M07-03: Research incremental compilation
- **Description**: Study incremental compilation approaches
- **Dependency**: AGENT_CAPABLE
- **Priority**: normal
- **Tags**: research, compiler, incremental
- **Deliverables**:
  - Dependency tracking strategies
  - Cache invalidation patterns
  - Rust's incremental analysis

### ARIA-M07-04: Design optimization levels
- **Description**: Design Aria's optimization level system
- **Dependency**: AGENT_CAPABLE
- **Priority**: normal
- **Tags**: research, compiler, optimization, design
- **Deliverables**:
  - Optimization level definitions
  - Debug info strategies
  - LTO integration

### ARIA-M07-05: Design platform targeting
- **Description**: Design platform-specific code generation
- **Dependency**: AGENT_CAPABLE
- **Priority**: normal
- **Tags**: research, compiler, platform, design
- **Deliverables**:
  - SIMD abstraction layer
  - Platform feature detection
  - Cross-compilation support

## Success Criteria

- [x] Backend choice made with rationale
- [ ] Incremental compilation designed
- [ ] Optimization levels defined
- [ ] Platform targeting strategy documented

## Key Resources

1. LLVM documentation and tutorials
2. Cranelift documentation
3. "Engineering a Compiler" - Cooper & Torczon
4. Zig compiler source code
5. rustc development guide

## Timeline

Target: Q2 2026

## Related Milestones

- **Depends on**: M06 (IR Design)
- **Enables**: M20 (Performance)
- **Parallel**: M08 (WASM Backend)
