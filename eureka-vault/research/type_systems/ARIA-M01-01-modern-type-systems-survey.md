# ARIA-M01-01: Survey of Modern Type System Approaches

**Task ID**: ARIA-M01-01
**Status**: Completed
**Date**: 2026-01-14
**Focus**: Comprehensive survey of type systems in systems programming languages

---

## Executive Summary

This research surveys modern type system approaches across systems programming languages, analyzing their inference capabilities, safety guarantees, and ergonomics. Key findings inform Aria's type system design decisions.

---

## 1. Comparison Matrix of Type Features

| Language | Type System | Inference Level | Safety Guarantees | Polymorphism | Ergonomics |
|----------|-------------|-----------------|-------------------|--------------|------------|
| **Rust** | Affine types, traits | Local inference | Memory safety, null safety | Parametric + ad-hoc (traits) | Moderate (lifetimes) |
| **Haskell** | HM + type classes | Full inference | Type safety, purity | Parametric + ad-hoc | High (minimal annotations) |
| **TypeScript** | Structural, gradual | Partial inference | Optional type safety | Parametric | High (JS interop) |
| **Kotlin** | Nullable, smart casts | Local inference | Null safety | Parametric + ad-hoc | High |
| **Swift** | Protocol-oriented | Local inference | ARC safety | Parametric + protocols | High |
| **OCaml** | HM + modules | Full inference | Type safety | Parametric + functors | High |
| **Scala 3** | DOT calculus | Bidirectional | Type safety | Full dependent path types | Moderate |

### Key Observations

1. **Full inference (Haskell/OCaml style)** eliminates annotation burden but can produce cryptic error messages
2. **Local inference (Rust/Swift)** requires annotations at function boundaries but provides clearer errors
3. **Structural typing (TypeScript)** enables easier interop but weaker guarantees
4. **Trait/typeclass systems** provide ad-hoc polymorphism without inheritance

---

## 2. Inference Algorithm Comparison

### 2.1 Hindley-Milner (Algorithm W/J)
- **Languages**: Haskell, OCaml, ML family
- **Strengths**:
  - Infers principal (most general) types
  - No annotations needed for most code
  - Decidable and complete
- **Limitations**:
  - Cannot handle higher-rank types without extensions
  - Monomorphization at let-bindings only
  - Object-oriented features break decidability

### 2.2 Local Type Inference
- **Languages**: Rust, Scala, Swift, Kotlin
- **Strengths**:
  - Clearer error messages with bounded inference scope
  - Works with complex features (traits, protocols)
  - Bidirectional information flow
- **Limitations**:
  - Requires function signatures
  - Less polymorphic inference

### 2.3 Flow Typing / Smart Casts
- **Languages**: TypeScript, Kotlin, Ceylon
- **Strengths**:
  - Type narrows after null/type checks
  - Natural for imperative code
- **Implementation**: Control-flow analysis refines types

### 2.4 Colored Local Type Inference
- **Reference**: Odersky et al.
- **Approach**: Propagates type information bidirectionally
- **Used in**: Scala, influences Rust

---

## 3. Safety Guarantee Analysis

### 3.1 Memory Safety Approaches

| Approach | Example | Compile-time | Runtime | Performance |
|----------|---------|--------------|---------|-------------|
| Ownership/borrowing | Rust | Full | None | Optimal |
| Reference counting | Swift ARC | Partial | Counting | ~5% overhead |
| Tracing GC | Go, Java | None | Full | Variable |
| Region-based | Vale | Hybrid | Minimal | ~2-10% |

### 3.2 Null Safety

| Strategy | Language | Approach |
|----------|----------|----------|
| Option types | Rust, Haskell | No null, explicit Option<T> |
| Nullable types | Kotlin, TypeScript | T? vs T distinction |
| Flow analysis | TypeScript, Ceylon | Narrowing after checks |

### 3.3 Effect Tracking

- **Explicit**: Haskell (IO monad), Koka (effect types)
- **Implicit**: Most imperative languages
- **Aria opportunity**: Inferred effect tracking (see M03)

---

## 4. Advanced Type Features Analysis

### 4.1 Dependent Types
- **Full**: Idris, Agda (types can depend on values)
- **Limited**: Rust (const generics), TypeScript (literal types)
- **Trade-off**: Power vs. decidability and usability

### 4.2 Refinement Types
- **Liquid Haskell**: SMT-verified refinements
- **Example**: `{v: Int | v > 0}` for positive integers
- **Aria relevance**: Contract system foundation (M04)

### 4.3 Type Classes / Traits / Protocols

| Feature | Haskell | Rust | Swift |
|---------|---------|------|-------|
| Ad-hoc polymorphism | Type classes | Traits | Protocols |
| Orphan rules | Flexible | Strict | Strict |
| Associated types | Yes | Yes | Yes |
| Default implementations | Yes | Yes | Yes |
| Coherence | Global | Crate-local | Module-local |

---

## 5. Recommendations for Aria

### 5.1 Type Inference Strategy

**Recommended**: Bidirectional local inference with HM core
- Function signatures required (like Rust/Swift)
- Full inference within function bodies
- Let-polymorphism for local bindings
- Clearer error messages than full HM

### 5.2 Polymorphism

**Recommended**: Trait-based system (Rust-inspired)
- Traits for ad-hoc polymorphism
- Parametric generics with trait bounds
- Associated types for complex abstractions
- Consider: orphan rule flexibility

### 5.3 Safety Features

**Recommended**:
- Ownership inference (see M02) - innovate beyond Rust
- Option types for null safety (no nullable types)
- Effect inference (see M03) - avoid explicit IO monad

### 5.4 Advanced Features (Phase 2)

- Const generics for compile-time computation
- Limited dependent types for contracts
- Refinement types integration with contract system

---

## 6. Key Papers and Resources

1. Damas & Milner - "Principal Type Schemes for Functional Programs" (1982)
2. Wadler & Blott - "How to make ad-hoc polymorphism less ad hoc" (1989)
3. Odersky et al. - "Colored Local Type Inference" (2001)
4. Rust Reference - Type system and traits
5. Swift Language Guide - Protocols and Generics

---

## 7. Open Questions for Further Research

1. Can we achieve Rust-level safety with Haskell-level inference?
2. How do we handle gradual typing for FFI scenarios?
3. What's the minimal annotation burden for ownership inference?
4. How do we integrate effect tracking without monads?

---

## Appendix: Language-Specific Insights

### Rust
- Trait system resembles Haskell type classes
- Affine types prevent use-after-move
- Lifetimes are novel contribution to type theory
- Error messages have improved significantly

### Haskell
- Type inference reduces boilerplate significantly
- Type classes enable powerful abstractions
- Monad requirement for effects is ergonomic burden
- GHC extensions enable dependent-type-like features

### TypeScript
- Structural typing enables JS interop
- Control flow analysis for type narrowing
- Template literal types are powerful but complex
- Error messages less helpful than Rust

### Swift
- Protocol-oriented design philosophy
- ARC provides memory safety without GC
- Protocol witnesses enable dynamic dispatch
- Good balance of inference and explicitness
