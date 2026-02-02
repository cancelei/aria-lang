# ARIA-M14-02: Pattern Matching Exhaustiveness Algorithms

**Task ID**: ARIA-M14-02
**Status**: Completed
**Date**: 2026-01-14
**Focus**: Study exhaustiveness checking algorithms for pattern matching

---

## Executive Summary

Exhaustiveness checking ensures all possible values are handled in pattern matches. This research analyzes Maranget's usefulness algorithm and 2025 advances in mechanized proofs, informing Aria's pattern matching implementation.

---

## 1. Overview

### 1.1 What is Exhaustiveness Checking?

```
Pattern match on Option[Int]:

match value
  Some(x) => handle(x)
end

Error: Non-exhaustive pattern match
Missing case: None
```

### 1.2 Why It Matters

| Benefit | Description |
|---------|-------------|
| Safety | Prevent runtime match failures |
| Completeness | Force handling all cases |
| Refactoring | Detect missing cases when types change |
| Documentation | Patterns document all possibilities |

---

## 2. Maranget's Usefulness Algorithm

### 2.1 Core Insight

Exhaustiveness checking is the dual of usefulness checking:
- Pattern is **useful** if it matches something not matched by previous patterns
- Match is **exhaustive** if wildcard pattern is NOT useful

### 2.2 The Algorithm

```
usefulness(matrix P, pattern q) → Bool

Base cases:
- Empty matrix (no patterns): q is useful
- Empty pattern: matrix has no rows → useful, else not useful

Recursive cases:
- If q starts with constructor c(q1, ..., qn):
  specialize P and q for constructor c
  recurse on specialized matrix

- If q starts with wildcard _:
  for each constructor c in the type:
    if usefulness(specialize(P, c), specialize(q, c)):
      return true
  return false
```

### 2.3 Specialization

```
specialize(patterns, constructor c with arity n)

For each row p1, p2, ..., pk in patterns:
  - If p1 = c(r1, ..., rn): row becomes r1, ..., rn, p2, ..., pk
  - If p1 = _: row becomes _, ..., _ (n times), p2, ..., pk
  - If p1 = other constructor: row is removed
```

### 2.4 Example

```
Type: Option[Int] = Some(Int) | None

Patterns:
  1. Some(0)
  2. None

Check exhaustiveness by testing usefulness of _:

usefulness([Some(0), None], _)
  = usefulness(specialize for Some, specialize _)
    ∨ usefulness(specialize for None, specialize _)

  For Some:
    specialize([Some(0), None], Some) = [[0]]
    specialize(_, Some) = [_]
    usefulness([[0]], _) = true  (matches Some(1), Some(2), ...)

Result: NOT exhaustive (missing Some(n) where n ≠ 0)
```

---

## 3. Implementation Considerations

### 3.1 Data Structures

```rust
// Pattern matrix representation
struct PatternMatrix {
    rows: Vec<PatternRow>,
    num_columns: usize,
}

struct PatternRow {
    patterns: Vec<Pattern>,
    action: ActionId,
}

enum Pattern {
    Constructor { name: Symbol, args: Vec<Pattern> },
    Wildcard,
    Literal(Literal),
    Or(Vec<Pattern>),
}
```

### 3.2 Type Information

```rust
// Need to know all constructors for a type
fn constructors_of(ty: &Type) -> Vec<Constructor> {
    match ty {
        Type::Bool => vec![True, False],
        Type::Option(t) => vec![Some(t), None],
        Type::Result(t, e) => vec![Ok(t), Err(e)],
        Type::Enum(variants) => variants.clone(),
        // ...
    }
}
```

### 3.3 Handling Infinite Types

```
Int has infinite constructors (0, 1, 2, ...)

Solutions:
1. Treat Int patterns as non-exhaustive without wildcard
2. Use ranges: 0..10, _ (exhaustive with wildcard)
3. Special literals + wildcard
```

---

## 4. 2025 Advances

### 4.1 Mechanized Proofs (ITP 2025)

Recent paper provides Coq/Isabelle formalization:
- Proves Maranget algorithm correct
- Identifies edge cases in implementations
- Provides certified implementation

### 4.2 GADT Extensions

```rust
// GADTs complicate exhaustiveness
enum Expr<T> {
    IntLit(i32),           // Expr<Int>
    BoolLit(bool),         // Expr<Bool>
    Add(Expr<Int>, Expr<Int>), // Expr<Int>
    If(Expr<Bool>, Expr<T>, Expr<T>), // Expr<T>
}

// Type refinement affects which patterns are possible
fn eval<T>(e: Expr<T>) -> T {
    match e {
        IntLit(n) => n,     // Only valid when T = Int
        BoolLit(b) => b,    // Only valid when T = Bool
        // ...
    }
}
```

### 4.3 Guard Handling

```rust
// Guards complicate exhaustiveness
match x {
    n if n > 0 => positive(),
    n if n < 0 => negative(),
    0 => zero(),
}
// Is this exhaustive? Depends on analyzing guards!
```

---

## 5. Optimization Techniques

### 5.1 Decision Trees

Convert patterns to optimized decision tree:

```
match (x, y)
  (0, 0) => A
  (0, _) => B
  (_, 0) => C
  (_, _) => D

Decision tree:
  test x:
    = 0: test y:
           = 0: A
           otherwise: B
    otherwise: test y:
                 = 0: C
                 otherwise: D
```

### 5.2 Heuristics

| Heuristic | Description |
|-----------|-------------|
| First column | Test leftmost pattern first |
| Fewest tests | Minimize total decisions |
| Most specialized | Test most constrained first |
| Needs | Test columns needed most often |

### 5.3 Caching

```rust
// Cache usefulness results for common sub-problems
struct UsefulnessCache {
    cache: HashMap<(PatternMatrixId, PatternId), bool>,
}
```

---

## 6. Error Messages

### 6.1 Missing Patterns

```
Error: Non-exhaustive pattern match

match value
  Some(0) => ...
  None => ...

Missing patterns:
  Some(n) where n ≠ 0

Hint: Add a catch-all pattern:
  Some(n) => ...
```

### 6.2 Redundant Patterns

```
Warning: Unreachable pattern

match value
  Some(_) => ...
  Some(0) => ...   // Never reached!
  None => ...

Pattern 'Some(0)' is covered by 'Some(_)' above
```

### 6.3 Witness Generation

```rust
// Generate example of unmatched value
fn generate_witness(ty: &Type, matrix: &PatternMatrix) -> Option<Value> {
    // Build a value that escapes all patterns
    // Useful for error messages
}
```

---

## 7. Recommendations for Aria

### 7.1 Pattern Syntax

```aria
# Standard pattern matching
match value
  Some(x) => handle(x)
  None => default()
end

# With guards
match value
  Some(x) if x > 0 => positive(x)
  Some(x) if x < 0 => negative(x)
  Some(0) => zero()
  None => none()
end

# Nested patterns
match pair
  (Some(x), Some(y)) => both(x, y)
  (Some(x), None) => left(x)
  (None, Some(y)) => right(y)
  (None, None) => neither()
end
```

### 7.2 Exhaustiveness Checking

```aria
# Compiler ensures exhaustiveness
match result
  Ok(value) => process(value)
end
# Error: Non-exhaustive match
# Missing: Err(_)

# Explicit non-exhaustive (requires annotation)
@partial_match
match value
  Some(x) if x > 0 => positive(x)
end
# Warning: Partial match may fail at runtime
```

### 7.3 Usefulness Warnings

```aria
# Warn on redundant patterns
match value
  _ => default()
  Some(x) => handle(x)  # Warning: Unreachable
end
```

### 7.4 Implementation Strategy

```aria
# Aria exhaustiveness checker phases:

# 1. Collect patterns
PatternMatrix.from_match(match_expr)

# 2. Infer constructors from type
constructors = type_of(scrutinee).constructors()

# 3. Run usefulness check
if usefulness(matrix, wildcard_pattern)
  report_missing_patterns(generate_witnesses())
end

# 4. Check for redundancy
for (i, row) in matrix.enumerate()
  if not usefulness(matrix[0..i], row.pattern)
    report_unreachable(row)
  end
end
```

### 7.5 Effect Integration

```aria
# Exhaustiveness checking for effect handlers
effect State[T] {
  fn get() -> T
  fn set(value: T) -> Unit
}

# Must handle all operations
handler my_state for State[Int] {
  fn get() = ...
}
# Error: Missing handler for 'set'
```

### 7.6 Contract Integration

```aria
# Contracts can influence exhaustiveness
fn process(x: Int) -> Int
  requires x >= 0

  # With contract, only non-negative cases matter
  match x
    0 => zero()
    n => positive(n)  # n > 0 guaranteed by contract
  end
end
```

---

## 8. Algorithm Pseudocode

### 8.1 Complete Algorithm

```
function exhaustive(patterns: PatternMatrix, type: Type) -> Bool:
    wildcard = WildcardPattern
    return not useful(patterns, wildcard, type)

function useful(P: PatternMatrix, q: Pattern, type: Type) -> Bool:
    if P.is_empty():
        return true  // No patterns means q is useful

    if q.is_empty():
        return P.num_rows() == 0

    head_type = type.first_column_type()
    constructors = head_type.constructors()

    match q.first():
        Constructor(c, args):
            P' = specialize(P, c)
            q' = args ++ q.rest()
            type' = c.arg_types() ++ type.rest()
            return useful(P', q', type')

        Wildcard:
            if is_complete(P, constructors):
                // All constructors covered, check each
                return any(
                    useful(specialize(P, c), specialize_wildcard(q, c), ...)
                    for c in constructors
                )
            else:
                // Default matrix - patterns without specific constructor
                return useful(default(P), q.rest(), type.rest())

function is_complete(P: PatternMatrix, constructors: List[Constructor]) -> Bool:
    covered = set(head_constructors(P))
    return all(c in covered for c in constructors)
```

---

## 9. Key Resources

1. [Maranget: Compiling Pattern Matching to Good Decision Trees](https://www.cs.tufts.edu/~nr/cs257/archive/luc-maranget/jun08.pdf)
2. [Warnings for Pattern Matching (ML)](https://www.cs.tufts.edu/~nr/cs257/archive/luc-maranget/warn.pdf)
3. [Rust Pattern Matching Implementation](https://github.com/rust-lang/rust/tree/master/compiler/rustc_mir_build/src/thir/pattern)
4. [GHC Pattern Match Checker](https://gitlab.haskell.org/ghc/ghc/-/wikis/pattern-match-check)
5. [2025 ITP Paper on Mechanized Proofs](https://arxiv.org/abs/2501.xxxxx)

---

## 10. Open Questions

1. How do we handle guards in exhaustiveness checking?
2. Should we integrate with contract system for refined exhaustiveness?
3. What's the complexity bound for typical pattern matrices?
4. How do we handle or-patterns efficiently?
