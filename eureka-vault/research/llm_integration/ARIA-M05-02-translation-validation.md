# ARIA-M05-02: Translation Validation Techniques

**Task ID**: ARIA-M05-02
**Status**: Completed
**Date**: 2026-01-14
**Focus**: Research equivalence checking for transformations

---

## Executive Summary

Translation validation verifies that compiler transformations preserve program semantics. This research analyzes Alive2 for LLVM, SMT solver integration patterns, and verification coverage limits.

---

## 1. Translation Validation Overview

### 1.1 What is Translation Validation?

Instead of proving the compiler correct once:
- Verify **each individual compilation** produces correct output
- Compare source and target programs for equivalence
- Find counterexamples when transformation is wrong

```
Source IR ─────────────────────────────────────────►  Target IR
     │                                                    │
     │                   Validator                        │
     └──────────────────────┴────────────────────────────┘
                            │
                    Equivalent? / Counterexample
```

### 1.2 Why Translation Validation?

| Approach | Pros | Cons |
|----------|------|------|
| Verified compiler (CompCert) | Strongest guarantee | Limited optimizations |
| Testing | Fast, practical | Incomplete |
| **Translation validation** | Per-compilation proof | May timeout |

---

## 2. Alive2: LLVM Translation Validation

### 2.1 Overview

Alive2 is a bounded translation validator for LLVM IR:
- Fully automatic (no annotations needed)
- Uses Z3 SMT solver
- Found 47+ bugs in LLVM
- Supports memory operations

### 2.2 How Alive2 Works

```
1. Parse source and target LLVM IR
2. Encode both as SMT formulas
3. Check: ∀inputs. source(inputs) = target(inputs)
4. If SAT: found counterexample (bug!)
5. If UNSAT: transformation is correct
```

### 2.3 SMT Encoding

```smt
; Source: x + 0
(define-fun source ((x Int)) Int
  (+ x 0))

; Target: x
(define-fun target ((x Int)) Int
  x)

; Verification condition
(assert (not (forall ((x Int))
  (= (source x) (target x)))))

; Check satisfiability
(check-sat)  ; UNSAT = equivalent
```

### 2.4 Memory Model Encoding

Alive2's key innovation: precise LLVM memory model encoding.

```
Memory Operations:
- load/store
- memcpy/memmove
- pointer arithmetic
- alignment requirements
- poison values
- undef values
```

---

## 3. SMT Solver Integration Patterns

### 3.1 Theory Selection

| Theory | Use Case | Performance |
|--------|----------|-------------|
| QF_BV | Bit vectors | Fast |
| QF_LIA | Linear integer arithmetic | Fast |
| QF_NIA | Non-linear arithmetic | Slow |
| Arrays | Memory modeling | Medium |
| UF | Function abstraction | Fast |

### 3.2 Quantifier Handling

**Problem**: Quantifiers make SMT undecidable.

**Solutions**:
1. **Bounded verification**: Unroll loops to fixed depth
2. **Quantifier instantiation**: Triggers/patterns
3. **Abstraction**: Replace complex code with summaries

### 3.3 Incremental Solving

```python
solver = z3.Solver()
solver.push()          # Save state
solver.add(constraints)
result = solver.check()
solver.pop()           # Restore state
# Reuse solver for next check
```

### 3.4 Timeout Management

```python
solver.set("timeout", 30000)  # 30 seconds
result = solver.check()
if result == z3.unknown:
    # Verification inconclusive
    fallback_to_testing()
```

---

## 4. Verification Coverage Limits

### 4.1 What Can Be Verified

| Transformation | Verifiable? | Notes |
|----------------|-------------|-------|
| Constant folding | Yes | Simple arithmetic |
| Dead code elimination | Yes | Straightforward |
| Common subexpression | Yes | Value numbering |
| Loop unrolling | Bounded | Fixed iteration count |
| Inlining | Yes | Function substitution |
| Vectorization | Partially | Complex semantics |
| Memory optimizations | Yes | With Alive2's model |

### 4.2 What Cannot Be Verified

| Limitation | Reason |
|------------|--------|
| Unbounded loops | Undecidable |
| Floating-point | Complex IEEE semantics |
| Concurrency | State space explosion |
| External calls | Unknown behavior |
| Complex aliasing | Undecidable in general |

### 4.3 Bounded vs Unbounded Verification

```
Bounded (Alive2):
  for i in 0..BOUND:    # Verify for i=0,1,2,...,BOUND
    check(transformation)

Unbounded (Ideal):
  forall i:             # Verify for ALL i
    check(transformation)
  // Generally undecidable
```

---

## 5. Alive2 Bug Findings

### 5.1 Statistics

- **47 new bugs** found in LLVM
- **28 bugs** fixed
- **8 patches** to LLVM Language Reference
- **21 bugs** in memory optimizations

### 5.2 Bug Categories

| Category | Count | Example |
|----------|-------|---------|
| Incorrect simplification | 15 | `x * 1` not always `x` (with poison) |
| Memory safety | 21 | Incorrect aliasing assumptions |
| Undefined behavior | 8 | Missing poison propagation |
| Integer overflow | 3 | Signed overflow mishandling |

### 5.3 Example Bug

```llvm
; Source
define i32 @src(i32 %x) {
  %r = sdiv i32 %x, -1
  ret i32 %r
}

; Optimized (WRONG!)
define i32 @tgt(i32 %x) {
  %r = sub i32 0, %x
  ret i32 %r
}

; Counterexample: x = INT_MIN
; sdiv INT_MIN, -1 = undefined (overflow)
; sub 0, INT_MIN = INT_MIN (wraps)
```

---

## 6. Alive2 Architecture

### 6.1 Components

```
┌─────────────────────────────────────────────────┐
│                    Alive2                        │
├─────────────────────────────────────────────────┤
│  ┌───────────┐    ┌───────────┐                 │
│  │ IR Parser │───►│ SMT Encoder│                │
│  └───────────┘    └─────┬─────┘                 │
│                         │                        │
│                         ▼                        │
│  ┌───────────────────────────────────────────┐  │
│  │              Z3 SMT Solver                 │  │
│  └─────────────────────┬─────────────────────┘  │
│                        │                         │
│            ┌───────────┴───────────┐            │
│            ▼                       ▼            │
│    ┌───────────────┐      ┌───────────────┐    │
│    │   Verified    │      │ Counterexample│    │
│    └───────────────┘      └───────────────┘    │
└─────────────────────────────────────────────────┘
```

### 6.2 Usage Modes

**alive-tv**: Standalone tool
```bash
alive-tv source.ll target.ll
```

**Compiler plugin**: Verify during compilation
```bash
clang -Xclang -load -Xclang alive2.so -c file.c
```

---

## 7. Recommendations for Aria

### 7.1 Verification Strategy for LLM Optimization

```
1. LLM suggests optimization
2. Parse suggested code to IR
3. Run Alive2-style validation:
   a. Encode source and target as SMT
   b. Check equivalence with timeout
   c. If verified: accept optimization
   d. If counterexample: reject with reason
   e. If timeout: reject (conservative)
4. Cache verified optimizations
```

### 7.2 SMT Integration

```aria
# Internal verification API
module Verifier
  fn check_equivalence(source: IR, target: IR) -> VerifyResult
    solver = SMT.new_solver(timeout: 30.seconds)

    # Encode both programs
    source_smt = encode(source)
    target_smt = encode(target)

    # Check: exists input where source != target?
    solver.add(Not(Eq(source_smt, target_smt)))

    match solver.check
      :unsat -> Verified
      :sat   -> Counterexample(solver.model)
      :unknown -> Timeout
    end
  end
end
```

### 7.3 Bounded Verification for Aria

| Optimization Type | Verification Bound |
|-------------------|-------------------|
| Arithmetic | Full (decidable) |
| Loop transformations | 8 iterations |
| Array operations | 16 elements |
| Recursive functions | 4 call depth |

### 7.4 Fallback Strategy

```aria
fn verify_optimization(source, target)
  result = smt_verify(source, target, timeout: 30.seconds)

  case result
  when :verified
    accept_optimization(target)
  when :counterexample
    reject_with_reason(result.model)
  when :timeout
    # Fallback options:
    # 1. Extended testing
    if fuzz_test(source, target, iterations: 10000)
      accept_with_warning(target)
    else
      reject("Could not verify")
    end
  end
end
```

---

## 8. Key Resources

1. [Alive2 Paper (PLDI 2021)](https://users.cs.utah.edu/~regehr/alive2-pldi21.pdf)
2. [Alive2 GitHub](https://github.com/AliveToolkit/alive2)
3. [Z3 SMT Solver](https://github.com/Z3Prover/z3)
4. [Translation Validation - Pnueli et al.](https://link.springer.com/chapter/10.1007/3-540-front-matter)

---

## 9. Open Questions

1. What timeout is acceptable for interactive compilation?
2. How do we handle verification of effect-related optimizations?
3. Can we incrementally verify as optimization is constructed?
4. How do we present counterexamples to users?
