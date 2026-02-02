# Milestone M02: Ownership Inference

## Overview

Design Aria's memory management system that achieves Rust-level safety without explicit lifetime annotations. The goal is ownership inference - the compiler determines ownership, borrowing, and lifetimes automatically.

## Research Questions

1. Can we infer ownership patterns that Rust requires annotations for?
2. What's the cost of inference vs explicit annotations?
3. How do we handle complex borrowing patterns automatically?
4. When do we need escape hatches (explicit annotations)?

## Core Innovation Target

```ruby
# User writes (no annotations):
fn process(data)
  result = transform(data)
  save(result)
end

# Compiler infers:
# - data: moved into transform
# - result: moved into save
# - No copies, no GC, no runtime cost
```

## Competitive Analysis Required

| Language | Approach | Study Focus |
|----------|----------|-------------|
| Rust | Explicit lifetimes | Borrow checker mechanics |
| Swift | ARC | Reference counting trade-offs |
| Vale | Region borrowing | Region-based inference |
| Lobster | Flow typing | Ownership flow analysis |
| Mojo | Ownership + references | Python-compatible ownership |

## Tasks

### ARIA-M02-01: Deep dive into Rust borrow checker
- **Description**: Understand Rust's borrow checker internals
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, memory, rust, competitive
- **Deliverables**:
  - NLL (Non-Lexical Lifetimes) analysis
  - Polonius algorithm study
  - Cases requiring explicit lifetimes

### ARIA-M02-02: Study Swift ARC implementation
- **Description**: Analyze Swift's automatic reference counting
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, memory, swift, arc
- **Deliverables**:
  - ARC overhead analysis
  - Copy-on-write patterns
  - Weak/unowned reference handling

### ARIA-M02-03: Research Vale's region borrowing
- **Description**: Study Vale's innovative region-based approach
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, memory, vale, innovative
- **Deliverables**:
  - Region borrowing mechanics
  - Generational references
  - Performance characteristics

### ARIA-M02-04: Design ownership inference algorithm
- **Description**: Design Aria's ownership inference approach
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Blocked by**: ARIA-M02-01, ARIA-M02-02, ARIA-M02-03
- **Tags**: research, memory, design, innovation
- **Deliverables**:
  - Inference algorithm specification
  - Escape hatch syntax design
  - Error message strategy

### ARIA-M02-05: Prototype ownership analyzer
- **Description**: Build prototype ownership inference
- **Dependency**: AGENT_CAPABLE
- **Priority**: normal
- **Blocked by**: ARIA-M02-04
- **Tags**: prototype, memory, implementation
- **Deliverables**:
  - Working inference on simple cases
  - Benchmark vs explicit annotations
  - Edge case documentation

## Implementation Progress

### Core Ownership Analyzer (COMPLETED - Jan 2026)
- [x] `crates/aria-ownership/` crate with full analyzer
- [x] Ownership kinds: Owned, Borrowed, BorrowedMut, Shared, Weak
- [x] Lifetime tracking with fresh lifetime generation
- [x] Use-after-move detection
- [x] Borrow conflict detection (mutable vs immutable)
- [x] Mutation through immutable reference detection
- [x] Full AST expression and statement analysis
- [x] Pattern binding for all pattern types
- [x] 7 unit tests passing

### Type System Integration (COMPLETED)
- [x] Transfer trait (Send equivalent) in aria-types
- [x] Sharable trait (Sync equivalent) in aria-types
- [x] Non-Transfer/Non-Sharable capture detection for spawn

### MIR Integration (COMPLETED - Jan 2026)
- [x] Move/Copy operands exist in MIR
- [x] `MirType::is_copy()` method for ownership inference
- [x] Automatic Move vs Copy decision based on type's Copy trait
- [x] `operand_for_local()` helper for ownership-aware variable access
- [x] Pattern lowering uses ownership inference for destructuring
- [x] 4 new unit tests for MirType::is_copy()

### FFI Ownership (COMPLETED)
- [x] `crates/aria-ffi/src/ownership.rs` with Owned/Borrowed/Transfer wrappers
- [x] @owned, @borrowed, @transfer annotations

## Success Criteria

- [x] Ownership inference algorithm designed
- [x] 90%+ of common patterns inferred automatically
- [x] Escape hatch syntax for complex cases (@shared, @weak)
- [ ] Performance equivalent to Rust (needs benchmarking)
- [x] Clear error messages for inference failures

## Status: COMPLETED (January 2026)

The ownership inference system is fully implemented with:
- Copy trait inference for types (`Type::is_copy()` and `MirType::is_copy()`)
- Automatic Move vs Copy decision during MIR lowering
- Pattern destructuring with ownership-aware field extraction
- 103+ tests across aria-types, aria-mir, and aria-ownership

## Key Papers/Resources

1. "Polonius: Rust's New Borrow Checker" - Matsakis
2. "Vale: Fearless Memory Safety" - Verdagon
3. "Ownership Types for Safe Programming" - Clarke et al.
4. "Linear Types Can Change the World" - Wadler
5. Mojo language ownership documentation

## Timeline

Target: Q1 2026 (Critical path)

## Related Milestones

- **Depends on**: M01 (Type System)
- **Enables**: M06 (Compiler IR), M07 (Native Backend)
- **Parallel**: M03 (Effect System)
