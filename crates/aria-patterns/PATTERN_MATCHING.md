# Pattern Matching System

This document describes the pattern matching system in Aria, including exhaustiveness checking, unreachable pattern detection, and decision tree compilation.

## Overview

The pattern matching system consists of several components:

1. **Exhaustiveness Checking**: Detects when match expressions don't cover all possible values
2. **Unreachable Pattern Detection**: Identifies patterns that can never be reached
3. **Decision Tree Compilation**: Compiles patterns into efficient decision trees
4. **Or-Pattern Support**: Handles `A | B | C` patterns correctly
5. **Guard Clause Handling**: Conservative treatment of runtime guards

## Architecture

### Core Modules

- `lib.rs` - Main types and pattern matrix
- `constructor.rs` - Constructor representation and sets
- `exhaustive.rs` - Exhaustiveness checking algorithm
- `usefulness.rs` - Simplified usefulness predicate
- `witness.rs` - Witness generation for missing patterns
- `decision_tree.rs` - Decision tree compilation
- `or_pattern.rs` - Or-pattern expansion utilities

### Pattern Matrix

The pattern matrix is a 2D structure where:
- Each **row** represents a pattern match arm
- Each **column** represents a position in the scrutinee structure
- Cells contain **deconstructed patterns** (constructor + sub-patterns)

```rust
let matrix = PatternMatrix::new(column_types);
matrix.push_row(PatternRow {
    patterns: vec![...],
    arm_index: 0,
});
```

### Constructors

Constructors represent ways to build values:

- `Bool(true)`, `Bool(false)` - Boolean constructors
- `Int(n)` - Integer literal
- `String(s)` - String literal
- `Tuple(arity)` - Tuple with N elements
- `Variant { name, index }` - Enum variant
- `Wildcard` - Matches anything
- `Range { start, end, inclusive }` - Numeric ranges

Constructor sets are finite for types like `Bool` and `Enum`, but infinite for `Int`, `String`, etc.

## Exhaustiveness Checking

Based on Maranget's "Warnings for Pattern Matching" algorithm.

### Algorithm

The algorithm uses a "usefulness" predicate:

```
useful(P, q) = can pattern q match a value not matched by any pattern in P?
```

A pattern matrix P is exhaustive iff:
```
not useful(P, wildcard)
```

### Process

1. Construct a pattern matrix from match arms
2. Test if a wildcard pattern is useful against the matrix
3. If useful, generate witness patterns showing what's missing
4. Report missing patterns to the user

### Example

```rust
match x {
    true => 1,
    // Missing: false
}
```

Exhaustiveness check detects missing `false` case and generates witness pattern.

## Unreachable Pattern Detection

Patterns are unreachable if they can never match any value:

```rust
match x {
    _ => 1,        // Arm 0: matches everything
    true => 2,     // Arm 1: UNREACHABLE (wildcard already matched)
}
```

### Algorithm

For each arm in order:
1. Check if the arm's pattern is useful given all previous arms
2. If not useful, the arm is unreachable
3. Add the arm to the matrix for checking subsequent arms

## Decision Tree Compilation

Pattern matching is compiled to a decision tree for efficient execution.

### Decision Tree Structure

```rust
enum DecisionTree {
    /// Execute this match arm
    Leaf { arm_index },

    /// Test a constructor and branch
    Switch {
        place: TestPlace,      // What to test
        ty: PatternType,       // Type being tested
        cases: Vec<SwitchCase>, // Branch for each constructor
        default: Option<Box<DecisionTree>>, // Catch-all
    },

    /// No match (runtime error)
    Fail,
}
```

### Compilation Strategy

1. Choose the best column to split on (heuristics: prefer specific patterns, finite types)
2. For each constructor in that column, specialize the matrix
3. Recursively compile subtrees
4. Optimize the result

### Optimizations

- **Sharing common prefixes**: Identical tests are factored out
- **Reordering**: Tests are ordered to minimize average cost
- **Collapsing**: Switches where all branches lead to the same arm are collapsed
- **Early exit**: Leaf nodes are recognized as soon as all columns are consumed

### Example

```rust
match (x, y) {
    (true, _) => 1,
    (false, true) => 2,
    (false, false) => 3,
}
```

Compiles to:
```
Switch on x:
  true  -> Leaf(1)
  false -> Switch on y:
             true  -> Leaf(2)
             false -> Leaf(3)
```

## Or-Patterns

Or-patterns like `A | B | C` are expanded during analysis:

```rust
match x {
    Some(1) | Some(2) | None => ...,
}
```

Is treated as three separate patterns:
```rust
Some(1) => ...
Some(2) => ...
None => ...
```

### Expansion

The `or_pattern` module provides utilities:

```rust
// Check if a pattern contains or-patterns
contains_or_pattern(&pattern)

// Expand or-patterns into multiple patterns
expand_ast_or_pattern(&pattern)
```

### Nested Or-Patterns

Or-patterns can appear in nested positions:

```rust
match x {
    Some(1 | 2) => ...,  // Nested or-pattern
}
```

These are expanded recursively.

## Guard Clauses

Guards are runtime conditions:

```rust
match x {
    n if n > 0 => positive(),
    n if n < 0 => negative(),
    _ => zero(),  // Still needed!
}
```

### Conservative Treatment

Guards are **opaque** to exhaustiveness checking:
- A guard might fail at runtime
- Therefore, guarded patterns don't contribute to exhaustiveness
- A catch-all pattern is still required

Implementation: Guarded patterns are treated as wildcards during analysis.

## Type Support

### Finite Types

- `Bool` - 2 constructors: `true`, `false`
- `Unit` - 1 constructor: `()`
- `Enum` - Fixed set of variants
- `Tuple` - Single constructor with N fields
- `Struct` - Single constructor with named fields

Finite types can be exhaustively checked without wildcards.

### Infinite Types

- `Int` - Infinite integer values
- `Float` - Infinite floating-point values
- `String` - Infinite strings
- `Array` - Infinite possible lengths

Infinite types **require a wildcard** to be exhaustive:

```rust
match n {
    1 => ...,
    2 => ...,
    _ => ...,  // Required for exhaustiveness
}
```

### Range Patterns

Range patterns for numeric types:

```rust
match n {
    0..10 => ...,      // Exclusive end
    10..=20 => ...,    // Inclusive end
    _ => ...,
}
```

Ranges are treated conservatively in exhaustiveness checking.

## Integration with MIR

The pattern matching system integrates with MIR lowering:

### Current Status

- ✅ Basic exhaustiveness checking
- ✅ Unreachable pattern detection
- ✅ Decision tree compilation
- ✅ Or-pattern expansion
- ✅ Guard clause handling
- ⏸️ Decision tree to MIR code generation (TODO)

### MIR Integration Points

Files that reference pattern matching:
- `crates/aria-mir/src/lower_expr.rs` - Match expressions
- `crates/aria-mir/src/lower_stmt.rs` - For-in loops with patterns

## Future Enhancements

### Short Term
1. Integrate decision trees into MIR code generation
2. Add pattern compilation benchmarks
3. Improve witness pattern display (show field names)

### Medium Term
1. Pattern specialization for better performance
2. Range pattern overlap detection
3. Const pattern evaluation

### Long Term
1. LLVM-style switch optimization
2. Pattern compilation profiling
3. Exhaustiveness for recursive types

## References

- Luc Maranget, "Warnings for Pattern Matching" (2007)
- Luc Maranget, "Compiling Pattern Matching to Good Decision Trees" (2008)
- Rust RFC 2008 - "Non-exhaustive pattern matching"
- "The Implementation of Functional Programming Languages" - Simon Peyton Jones

## Testing

Comprehensive test coverage in:
- `src/lib.rs` - Unit tests for pattern matrix
- `src/exhaustive.rs` - Exhaustiveness algorithm tests
- `src/decision_tree.rs` - Decision tree compilation tests
- `tests/integration_tests.rs` - End-to-end integration tests

Run tests:
```bash
cargo test -p aria-patterns
```

## Performance Characteristics

### Time Complexity

- Exhaustiveness checking: O(2^n) worst case, O(n*m) typical
  - n = number of columns
  - m = number of rows

- Decision tree compilation: O(n*m*k)
  - k = average number of constructors per type

### Space Complexity

- Pattern matrix: O(n*m)
- Decision tree: O(depth * branches) = O(n * c)
  - c = average constructor count

### Optimization Opportunities

1. **Memoization**: Cache usefulness checks
2. **Early pruning**: Skip obviously redundant checks
3. **Lazy expansion**: Delay or-pattern expansion until needed
4. **Smart constructor ordering**: Test more discriminating constructors first
