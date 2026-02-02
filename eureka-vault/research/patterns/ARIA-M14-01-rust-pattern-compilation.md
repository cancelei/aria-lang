# ARIA-M14-01: Rust Pattern Compilation Study

**Task ID**: ARIA-M14-01
**Status**: Completed
**Date**: 2026-01-14
**Focus**: Analyze Rust's pattern compilation and exhaustiveness checking

---

## Executive Summary

Rust's pattern matching combines exhaustiveness checking, optimization through decision trees, and precise error messages. This research analyzes the algorithms for Aria's pattern matching implementation.

---

## 1. Overview

### 1.1 What Rust's Pattern Matching Provides

- **Exhaustiveness checking**: Ensure all cases handled
- **Redundancy detection**: Warn about unreachable patterns
- **Efficient compilation**: Decision tree generation
- **Rich patterns**: Destructuring, guards, or-patterns

### 1.2 Where It Happens

```
rustc Pipeline:
  HIR → Pattern Analysis → MIR

Pattern checking in:
  compiler/rustc_pattern_analysis/usefulness
```

---

## 2. Exhaustiveness Checking

### 2.1 Core Concept: Usefulness

Given patterns `p_1 .. p_n`, a pattern `q` is **useful** if there exists a value matched by `q` but not by any `p_i`.

```rust
match value {
    Some(0) => ...,      // p_1
    Some(1) => ...,      // p_2
    None => ...,         // p_3
    Some(_) => ...,      // p_4 - useful! (matches Some(2), Some(3), ...)
}
```

### 2.2 Exhaustiveness = Wildcard Usefulness

A match is **exhaustive** if `_` (wildcard) is NOT useful:
- If `_` matches something uncovered, match is non-exhaustive
- If `_` is useless, all values are covered

### 2.3 Complexity

**Fun fact**: Exhaustiveness checking is NP-complete (can encode SAT).

But in practice:
- Most patterns are simple
- Optimization heuristics work well
- Rarely hits worst case

---

## 3. The Usefulness Algorithm

### 3.1 Matrix Representation

Patterns form a matrix, values are rows:

```
Match:                    Matrix:
match (x, y) {            | (Some(_), true)  |
    (Some(_), true) =>    | (None, _)        |
    (None, _) =>          | (_, false)       |
    (_, false) =>
}
```

### 3.2 Algorithm Sketch

```
usefulness(matrix, pattern):
  if pattern is empty:
    return matrix is empty ? NOT_USEFUL : USEFUL(witness)

  head = first column of pattern
  for each constructor c in head's type:
    if c is covered by head:
      specialized_matrix = specialize(matrix, c)
      specialized_pattern = specialize(pattern, c)
      if useful(specialized_matrix, specialized_pattern):
        return USEFUL

  return NOT_USEFUL
```

### 3.3 Constructor Splitting

For each type, enumerate constructors:

| Type | Constructors |
|------|--------------|
| `bool` | `true`, `false` |
| `Option<T>` | `Some(_)`, `None` |
| `Result<T,E>` | `Ok(_)`, `Err(_)` |
| `u8` | `0`, `1`, ..., `255` |
| `(A, B)` | `(_, _)` |

---

## 4. Integer Pattern Matching

### 4.1 Rust's Innovation

Rust is (reportedly) the **first language** to support exhaustive integer pattern matching:

```rust
fn describe_u8(n: u8) -> &'static str {
    match n {
        0..=9 => "single digit",
        10..=99 => "two digits",
        100..=255 => "three digits",
    } // Exhaustive!
}
```

### 4.2 Implementation

Uses interval arithmetic:
- Ranges become intervals
- Check if intervals cover entire type
- Warn about gaps

---

## 5. Decision Tree Compilation

### 5.1 Goal

Convert pattern match to efficient code:

```rust
match value {
    (Some(x), true) => f(x),
    (None, _) => g(),
    (_, false) => h(),
}
```

### 5.2 Decision Tree

```
         ┌─── value.0 ───┐
         │               │
      Some(x)          None
         │               │
    ┌─value.1─┐         g()
    │         │
   true     false
    │         │
   f(x)      h()
```

### 5.3 Compilation Strategy

**Maranget's Algorithm** (from OCaml):

1. Choose a column to split on
2. Generate switch/if for constructors
3. Recursively compile sub-matrices
4. Optimize redundant tests

### 5.4 Heuristics for Column Selection

| Heuristic | Description |
|-----------|-------------|
| First column | Simple, predictable |
| Most distinct | Maximize branching |
| Smallest first | Handle rare cases early |
| Necessary | Must test this column |

---

## 6. Pattern Features

### 6.1 Or-Patterns

```rust
match value {
    Ok(x) | Cached(x) => process(x),
    Err(NotFound | Timeout) => retry(),
    Err(e) => fail(e),
}
```

Compilation: duplicate subtrees or merge.

### 6.2 Guards

```rust
match value {
    x if x > 0 => positive(x),
    x if x < 0 => negative(x),
    _ => zero(),
}
```

Guards prevent exhaustiveness proof - need fallback.

### 6.3 Bindings

```rust
match point {
    Point { x, y } => use(x, y),          // Destructure
    Point { x: a, y: b } => use(a, b),    // Rename
    Point { x, .. } => use(x),            // Partial
}
```

---

## 7. Error Messages

### 7.1 Non-Exhaustive Error

```rust
match option {
    Some(x) => x,
    // Missing: None
}
```

Error:
```
error[E0004]: non-exhaustive patterns: `None` not covered
 --> src/main.rs:4:11
  |
4 |     match option {
  |           ^^^^^^ pattern `None` not covered
  |
  = help: ensure that all possible cases are being handled
```

### 7.2 Unreachable Pattern Warning

```rust
match option {
    Some(_) => 1,
    None => 2,
    Some(42) => 3,  // Unreachable!
}
```

Warning:
```
warning: unreachable pattern
 --> src/main.rs:6:5
  |
6 |     Some(42) => 3,
  |     ^^^^^^^^ unreachable pattern
```

---

## 8. Recommendations for Aria

### 8.1 Pattern Syntax

```aria
# Basic matching
match user
  User(name: "admin", role:) => admin_panel(role)
  User(name:, age:) if age >= 18 => adult_view(name)
  User(name:, _) => guest_view(name)
end

# Array patterns
match items
  [] => "empty"
  [single] => "one: #{single}"
  [first, ...rest] => "first: #{first}, rest: #{rest.length}"
end

# Or-patterns
match status
  Ok(value) | Cached(value) => process(value)
  Err(NotFound | Timeout) => retry()
  Err(e) => fail(e)
end
```

### 8.2 Exhaustiveness Checking

```aria
# Aria should enforce exhaustiveness
match option
  Some(x) => process(x)
  # Error: non-exhaustive, missing None
end

# With wildcard (allowed but discouraged for enums)
match option
  Some(x) => process(x)
  _ => default()          # Covers None
end
```

### 8.3 Implementation Strategy

```aria
# Phase 1: Usefulness algorithm
PatternChecker {
  fn is_useful(matrix: Matrix, pattern: Pattern) -> UseResult
  fn check_exhaustiveness(patterns: Array[Pattern], type: Type) -> ExhResult
}

# Phase 2: Decision tree compilation
DecisionTree {
  fn compile(patterns: Array[Pattern], actions: Array[Action]) -> Tree
  fn optimize(tree: Tree) -> Tree
  fn to_mir(tree: Tree) -> MIR
}
```

### 8.4 Error Messages

```aria
# Good error message example
Error: Non-exhaustive patterns in match

  12 | match result
  13 |   Ok(value) => process(value)
  14 | end
        ^^^ patterns not covered: `Err(_)`

  Suggestion: Add a case for `Err(_)`:
    Err(e) => handle_error(e)

  Or use a wildcard if you want to ignore:
    _ => default()
```

### 8.5 Integration with Contracts

```aria
# Pattern exhaustiveness + contract verification
fn process(option: Option[Int]) -> Int
  ensures |result| result >= 0

  match option
    Some(x) if x >= 0 => x          # Satisfies ensures
    Some(x) => -x                   # Satisfies ensures
    None => 0                       # Satisfies ensures
  end
end
```

---

## 9. Key Resources

1. [Pattern and Exhaustiveness Checking - rustc dev guide](https://rustc-dev-guide.rust-lang.org/pat-exhaustive-checking.html)
2. [rustc_pattern_analysis::usefulness](https://doc.rust-lang.org/beta/nightly-rustc/rustc_pattern_analysis/usefulness/index.html)
3. [Compiling Pattern Matching to Good Decision Trees - Maranget](https://www.researchgate.net/publication/221057054_Compiling_pattern_matching_to_good_decision_trees)
4. [Pattern Matching in Rust (Implementation)](https://github.com/yorickpeterse/pattern-matching-in-rust)
5. [RFC 2591: Exhaustive Integer Patterns](https://rust-lang.github.io/rfcs/2591-exhaustive-integer-pattern-matching.html)

---

## 10. Open Questions

1. Should Aria warn on wildcard patterns for small enums?
2. How do we handle patterns with effects?
3. Should we support active patterns (F#-style)?
4. What's the compilation target for patterns in Aria MIR?
