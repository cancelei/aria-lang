# Milestone M01: Type System Foundations

## Overview

Establish the foundational type system for Aria that enables type inference, safety guarantees, and contract integration while maintaining Ruby/Python-like ergonomics.

## Research Questions

1. How do we achieve Hindley-Milner-level inference with Rust-level safety?
2. What type system features enable compile-time contract verification?
3. How do we handle gradual typing for interop scenarios?
4. What's the right balance between explicitness and inference?

## Competitive Analysis Required

| Language | Type System | Study Focus |
|----------|-------------|-------------|
| Rust | Affine types, traits | Safety + inference |
| Haskell | HM + type classes | Pure inference |
| TypeScript | Structural, gradual | JavaScript interop |
| Kotlin | Nullable, smart casts | Null safety patterns |
| Swift | Protocol-oriented | Type witnesses |

## Tasks

### ARIA-M01-01: Survey modern type system approaches
- **Description**: Comprehensive survey of type systems in systems programming languages
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, type-system, survey, milestone
- **Deliverables**:
  - Comparison matrix of type features
  - Inference algorithm comparison
  - Safety guarantee analysis

### ARIA-M01-02: Analyze Hindley-Milner extensions
- **Description**: Study HM extensions (System F, Fω) for Aria's needs
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, type-system, inference, academic
- **Deliverables**:
  - Algorithm W vs Algorithm J analysis
  - Let-polymorphism patterns
  - Type class integration approaches

### ARIA-M01-03: Design trait/protocol system
- **Description**: Design Aria's approach to ad-hoc polymorphism
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Blocked by**: ARIA-M01-01
- **Tags**: research, type-system, traits, design
- **Deliverables**:
  - Trait vs protocol vs type class comparison
  - Orphan rule analysis
  - Associated types design

### ARIA-M01-04: Prototype type checker
- **Description**: Build minimal type checker prototype
- **Dependency**: AGENT_CAPABLE
- **Priority**: normal
- **Blocked by**: ARIA-M01-02, ARIA-M01-03
- **Tags**: prototype, type-system, implementation
- **Deliverables**:
  - Working type inference for core types
  - Trait resolution prototype
  - Error message design

## Success Criteria

- [ ] Type system design documented with formal semantics
- [ ] Inference algorithm selected and prototyped
- [ ] Trait system design finalized
- [ ] Error message strategy defined
- [ ] At least 5 memories stored from research

## Key Papers to Study

1. "Principal Type Schemes for Functional Programs" - Damas & Milner
2. "Type Classes: An Exploration" - Wadler & Blott
3. "Colored Local Type Inference" - Odersky et al.
4. "Outrageous Fortune: Dependent Types in Swift" - Apple
5. "Rust's Type System is Turing Complete" - Analysis

## Timeline

Target: Q1 2026 (Foundation for all other milestones)

## Related Milestones

- **Enables**: M03 (Effect System), M04 (Contracts), M12 (Property Testing)
- **Parallel**: M02 (Ownership Inference)
