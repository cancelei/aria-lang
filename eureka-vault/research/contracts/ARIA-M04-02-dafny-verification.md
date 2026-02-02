# ARIA-M04-02: Dafny's Verification Analysis

**Task ID**: ARIA-M04-02
**Status**: Completed
**Date**: 2026-01-14
**Focus**: Study Dafny's automated verification approach

---

## Executive Summary

Dafny is a verification-aware programming language that uses Z3 SMT solver for automated proof. This research analyzes Dafny's annotation requirements, verification limits, and Z3 integration patterns.

---

## 1. Dafny Overview

### 1.1 What is Dafny?

Dafny is a **verification-aware** language that:
- Combines programming with specification
- Automatically verifies correctness
- Uses SMT solving (Z3) under the hood
- Compiles to C#, Java, JavaScript, Go, Python

### 1.2 Verification Pipeline

```
Dafny Source → Boogie IR → Verification Conditions → Z3 SMT Solver
                                                          ↓
                                                    Verified / Error
```

---

## 2. Z3 Integration Patterns

### 2.1 How Dafny Uses Z3

1. **Translation**: Dafny → Boogie intermediate language
2. **VC Generation**: Boogie generates verification conditions
3. **SMT Encoding**: VCs encoded as SMT-LIB formulas
4. **Solving**: Z3 attempts to find counterexample
5. **Result**: UNSAT = verified, SAT = counterexample found

### 2.2 SMT Theories Used

| Theory | Purpose |
|--------|---------|
| **Uninterpreted Functions** | Method abstractions |
| **Linear Arithmetic** | Integer/real bounds |
| **Arrays** | Sequence operations |
| **Datatypes** | ADTs, option types |
| **Quantifiers** | Forall/exists |

### 2.3 Quantifier Handling

**Challenge**: Quantifiers make SMT solving undecidable.

**Dafny's approach**:
- Triggers/patterns to guide instantiation
- Bounded quantifier unrolling
- User-provided lemmas

```dafny
// Trigger: instantiate when sorted(a[..i]) appears
lemma SortedLemma(a: array<int>, i: int)
    requires sorted(a[..i])
    ensures forall j, k :: 0 <= j < k < i ==> a[j] <= a[k]
```

---

## 3. Annotation Burden Analysis

### 3.1 Types of Annotations

| Annotation | Purpose | Frequency |
|------------|---------|-----------|
| `requires` | Precondition | Every method |
| `ensures` | Postcondition | Every method |
| `invariant` | Loop invariant | Every loop |
| `decreases` | Termination | Loops, recursion |
| `assert` | Intermediate fact | As needed |
| `ghost` | Verification-only code | Proofs |

### 3.2 Annotation Example

```dafny
method BinarySearch(a: array<int>, key: int) returns (index: int)
    requires sorted(a)                    // Precondition
    ensures index >= 0 ==> a[index] == key  // Postcondition
    ensures index < 0 ==> key !in a[..]
{
    var lo, hi := 0, a.Length;
    while lo < hi
        invariant 0 <= lo <= hi <= a.Length    // Loop invariant
        invariant key !in a[..lo]               // What we've ruled out
        invariant key !in a[hi..]
        decreases hi - lo                       // Termination
    {
        var mid := (lo + hi) / 2;
        if a[mid] < key {
            lo := mid + 1;
        } else if a[mid] > key {
            hi := mid;
        } else {
            return mid;
        }
    }
    return -1;
}
```

### 3.3 Annotation Burden Metrics

From research studies:
- **Simple functions**: ~30% annotation overhead
- **Complex algorithms**: 50-100% annotation overhead
- **Data structures**: Can exceed 200% overhead

### 3.4 Pain Points

1. **Loop invariants**: Must capture exactly what's needed
2. **Quantifiers**: Easy to write, hard to verify
3. **Triggers**: Require SMT expertise
4. **Timeouts**: Unpredictable verification times

---

## 4. Verification Limits

### 4.1 Decidability Boundaries

| Feature | Decidable? | Notes |
|---------|------------|-------|
| Linear arithmetic | Yes | Efficient |
| Non-linear arithmetic | Limited | Often undecidable |
| Quantifiers | No | Heuristic-based |
| Induction | Manual | Requires lemmas |
| Heap reasoning | Limited | Frame conditions |

### 4.2 What Dafny Can't Verify Automatically

1. **Complex mathematical properties**
2. **Non-linear arithmetic** (x * y = z)
3. **Deep induction**
4. **Unbounded data structures**
5. **Floating-point properties**

### 4.3 Verification Timeouts

```dafny
// This might timeout with complex invariants
method ComplexAlgorithm()
    // Z3 may take > 30 seconds or give up
```

**Mitigations**:
- Split into smaller lemmas
- Add intermediate assertions
- Use `{:fuel}` to limit unfolding

---

## 5. Verification Strategies

### 5.1 Modular Verification

```dafny
// Verify each method independently
method A() ensures P() { ... }
method B() requires P() ensures Q() { ... }

// Verification of B assumes P(), doesn't re-verify A
```

### 5.2 Ghost Code

```dafny
ghost method Lemma(x: int)
    ensures x * x >= 0
{
    // Proof code, erased at runtime
}

method RealCode(x: int)
{
    Lemma(x);  // Invoke lemma for verification
    var y := x * x;
    assert y >= 0;  // Now verifies!
}
```

### 5.3 Calculation Proofs

```dafny
lemma SquarePositive(x: int)
    ensures x * x >= 0
{
    calc {
        x * x;
    >= { /* x * x = |x|² */ }
        0;
    }
}
```

---

## 6. AI-Assisted Verification

### 6.1 Dafny-Annotator (2024)

Recent research uses LLMs to generate annotations:

```
Input: Unannotated Dafny method
LLM: Suggests requires, ensures, invariants
Search: Iteratively refine until verification succeeds
```

**Results**: Successfully annotates many methods automatically

### 6.2 Implications for Aria

- LLMs can help generate contract annotations
- Interactive mode: suggest contracts as user writes code
- Verification failure → LLM suggests fix

---

## 7. Recommendations for Aria

### 7.1 Verification Levels

| Level | Checking | Annotation Burden |
|-------|----------|-------------------|
| **0: None** | Runtime only | None |
| **1: Simple** | Type bounds, nullability | Auto-inferred |
| **2: Contracts** | Pre/post conditions | User-written |
| **3: Full** | Loop invariants, termination | Heavy |

### 7.2 Hybrid Approach

```aria
# Level 1: Auto-verified (no annotation)
fn abs(x: Int) -> Int
  if x < 0 then -x else x
end
# Compiler proves: result >= 0

# Level 2: Explicit contracts
fn binary_search(arr, key) -> Int?
  requires arr.sorted?
  ensures |result| result.none? or arr[result.unwrap] == key
  # ...
end

# Level 3: Full verification (opt-in)
@verify(level: :full)
fn critical_algorithm(data)
  invariant data.length > 0
  decreases data.length
  # ...
end
```

### 7.3 SMT Integration Strategy

**Don't** try to verify everything with SMT:
- Use for simple arithmetic bounds
- Use for nullability proofs
- Fall back to runtime for complex properties

**Do** provide escape hatches:
```aria
@assume sorted(arr)  # Trust programmer, skip verification
fn fast_search(arr, key)
  # ...
end
```

### 7.4 Error Messages

```
Verification failed for `binary_search`:

  Postcondition might not hold:
    ensures result.none? or arr[result.unwrap] == key
                           ^^^^^^^^^^^^^^^^^^^^^^^^

  Counterexample:
    arr = [1, 3, 5]
    key = 2
    result = Some(1)  # arr[1] = 3 ≠ 2

  Suggestion: Check the binary search logic at line 12
```

---

## 8. Key Resources

1. [Dafny: An Automatic Program Verifier](https://www.microsoft.com/en-us/research/publication/dafny-automatic-program-verifier-functional-correctness/)
2. [Dafny Documentation](https://dafny.org/latest/)
3. [Z3 SMT Solver](https://github.com/Z3Prover/z3)
4. [Dafny-Annotator: AI-Assisted Verification](https://arxiv.org/abs/2411.15143)

---

## 9. Open Questions

1. What annotation level should be the default for Aria?
2. How do we make verification errors actionable?
3. Can we integrate LLM annotation suggestions into IDE?
4. How do contracts interact with effects?
