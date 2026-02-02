# Milestone M06: Compiler IR Design

## Overview

Design Aria's Intermediate Representation (IR) that supports multi-target compilation (Native, WASM, JS), optimization passes, and LLM integration.

## Research Questions

1. What IR level is optimal for Aria's needs?
2. How do we represent ownership/effects in IR?
3. Should we use existing IR (LLVM, Cranelift) or custom?
4. How does the IR support LLM optimization integration?

## IR Requirements

```
Source → AST → HIR → MIR → LIR → Backend
                ↑       ↑
           Type-rich   Ownership-aware
                       Effect-annotated
```

- **HIR (High-level IR)**: Type-checked, trait-resolved, generic-free
- **MIR (Mid-level IR)**: Ownership analyzed, control flow graph
- **LIR (Low-level IR)**: Target-specific, ready for codegen

## Competitive Analysis Required

| Compiler | IR Design | Study Focus |
|----------|-----------|-------------|
| Rust | HIR→MIR→LLVM | Ownership in IR |
| Swift | SIL→LLVM | Reference counting |
| Zig | AIR→LLVM | Comptime in IR |
| Go | SSA IR | GC integration |
| MLIR | Multi-level | Extensible IR |

## Tasks

### ARIA-M06-01: Study Rust's MIR design
- **Description**: Deep dive into Rust's Mid-level IR
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, compiler, ir, rust
- **Deliverables**:
  - MIR structure analysis
  - Borrow checking integration
  - Optimization passes

### ARIA-M06-02: Analyze MLIR architecture
- **Description**: Study MLIR's extensible IR approach
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, compiler, ir, mlir
- **Deliverables**:
  - Dialect system analysis
  - Multi-level representation
  - Optimization infrastructure

### ARIA-M06-03: Study Cranelift IR
- **Description**: Analyze Cranelift's simple IR design
- **Dependency**: AGENT_CAPABLE
- **Priority**: normal
- **Tags**: research, compiler, ir, cranelift
- **Deliverables**:
  - IR simplicity analysis
  - WASM targeting patterns
  - Compilation speed

### ARIA-M06-04: Design HIR specification
- **Description**: Design Aria's High-level IR
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Blocked by**: ARIA-M06-01
- **Tags**: research, compiler, ir, design
- **Deliverables**:
  - HIR node types
  - Type representation
  - Trait resolution output

### ARIA-M06-05: Design MIR specification
- **Description**: Design Aria's Mid-level IR
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Blocked by**: ARIA-M06-01, ARIA-M06-04
- **Tags**: research, compiler, ir, design
- **Deliverables**:
  - MIR structure
  - Ownership representation
  - Effect annotations

### ARIA-M06-06: Design LLM integration points
- **Description**: Define where LLM suggestions integrate in IR
- **Dependency**: AGENT_CAPABLE
- **Priority**: normal
- **Blocked by**: ARIA-M06-05
- **Tags**: research, compiler, ir, llm
- **Deliverables**:
  - Optimization hook points
  - Verification interfaces
  - Transformation rules

## Implementation Progress

### Core IR Implementation (COMPLETED - Jan 2026)
- [x] MIR data structures (MirProgram, MirFunction, BasicBlock, Statement, Terminator)
- [x] Place/Rvalue/Operand representation
- [x] Effect system integration (EffectRow, EffectType, EvidenceSlot)
- [x] AST to MIR lowering (LoweringContext)
- [x] Type inference support (TypeVarId, unification)
- [x] Pretty printing (pretty_print)
- [x] 44 unit tests passing

### Code Generation (COMPLETED - Jan 2026)
- [x] Cranelift backend for native (x86_64, aarch64)
- [x] WASM backend (wasm32)
- [x] Multi-target abstraction (Target enum)
- [x] compile_to_object entry point
- [x] 16 unit tests passing

### Optimization Passes (COMPLETED - Jan 2026)
- [x] Constant folding (arithmetic, comparison, boolean, bitwise)
- [x] Dead code elimination (unreachable blocks, unused locals)
- [x] Copy propagation (with transitive resolution)
- [x] CFG simplification (constant switch folding)
- [x] 9 unit tests for optimization passes

### Documentation (COMPLETED - Jan 2026)
- [x] Formal IR specification document (eureka-vault/docs/designs/ARIA-MIR-SPECIFICATION.md)
- [x] LLM integration points definition (eureka-vault/docs/designs/ARIA-LLM-INTEGRATION.md)

## Success Criteria

- [x] IR design documented with formal specification
- [x] Ownership/effects representable in IR
- [x] Multi-target support designed
- [x] LLM integration points defined
- [x] Prototype IR builder

## Status: COMPLETED (January 2026)

## Key Papers/Resources

1. "The Rust MIR" - Rust documentation
2. "MLIR: A Compiler Infrastructure for the End of Moore's Law"
3. "Cranelift: A New Backend for Rust" - Bytecode Alliance
4. "SSA Form and Control Flow Graphs" - Compiler textbooks
5. "Sea of Nodes and the HotSpot JIT" - Click

## Timeline

Target: Q1-Q2 2026

## Related Milestones

- **Depends on**: M01 (Types), M02 (Ownership), M03 (Effects)
- **Enables**: M07 (Native), M08 (WASM), M05 (LLM)
