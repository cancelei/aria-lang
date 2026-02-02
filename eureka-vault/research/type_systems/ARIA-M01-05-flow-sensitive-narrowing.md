# ARIA-M01-05: Flow-Sensitive Type Narrowing Design

**Task ID**: ARIA-M01-05
**Status**: Completed
**Date**: 2026-01-15
**Focus**: Design flow-sensitive type narrowing for Aria's type system

---

## Executive Summary

Flow-sensitive typing allows the type checker to narrow variable types based on control flow conditions. After checking `if x != nil`, the type of `x` narrows from `T?` to `T` within that branch. This research analyzes TypeScript's control flow analysis and Kotlin's smart casts to design a narrowing system for Aria that integrates with the existing `TypeEnv` scoping and contract system.

---

## 1. Problem Statement

### 1.1 Current Limitation

Aria's current type checker (`aria-types/src/lib.rs`) uses basic Hindley-Milner unification. The `TypeEnv` tracks variable bindings in a scope chain, but does not track type refinements from control flow:

```rust
// Current TypeEnv structure
pub struct TypeEnv {
    variables: FxHashMap<String, Type>,
    types: FxHashMap<String, TypeScheme>,
    parent: Option<Rc<TypeEnv>>,
}
```

This means code like the following would require explicit unwrapping:

```aria
fn process(value: Int?) -> Int
  if value != nil
    # Currently: value is still Int?, need to unwrap
    # Goal: value should narrow to Int automatically
    return value + 1
  end
  return 0
end
```

### 1.2 Goals

1. **Automatic type narrowing** after null checks, type guards, pattern matching
2. **Integration with existing scoping** - narrowed types in child scopes
3. **Contract system integration** - `requires` clauses inform narrowing
4. **Ownership compatibility** - narrowing must work with borrow checker (M02)
5. **Performance** - lazy evaluation, avoid recomputing narrowings

---

## 2. Research: TypeScript's Control Flow Analysis

### 2.1 Architecture

TypeScript implements flow-sensitive typing through a **Control Flow Graph (CFG)**:

| Component | Description |
|-----------|-------------|
| **Flow Nodes** | DAG nodes representing statements/expressions |
| **Antecedents** | Back-edges to predecessor nodes |
| **Binder** | Eagerly constructs CFG during parsing |
| **Checker** | Lazily evaluates types via CFG traversal |

**Key insight**: TypeScript computes "flow types" by traversing **backwards** from the point of use to the symbol definition, accumulating narrowing conditions.

### 2.2 Narrowing Operations

TypeScript supports these narrowing patterns:

| Pattern | Example | Narrowed Type |
|---------|---------|---------------|
| Truthiness | `if (x)` | Excludes `null`, `undefined`, `0`, `""` |
| typeof | `if (typeof x === "string")` | `string` |
| Equality | `if (x === "foo")` | Literal type `"foo"` |
| Instanceof | `if (x instanceof Error)` | `Error` |
| Type predicate | `if (isString(x))` | Return type's predicate |
| Discriminated union | `if (x.kind === "a")` | Variant with `kind: "a"` |

### 2.3 Implementation Notes

```typescript
// TypeScript's core narrowing function
function getTypeAtFlowNode(node: FlowNode): Type {
    while (true) {
        // Match refinement patterns against flow node
        // Advance to antecedent if no match
    }
}
```

**Performance strategy**: Only compute flow types when actually needed. Store only the CFG, not computed types.

**Reference**: [Effective TypeScript - Flow Nodes](https://effectivetypescript.com/2024/03/24/flownodes/)

---

## 3. Research: Kotlin's Smart Casts

### 3.1 Lattice-Based Analysis

Kotlin uses a sophisticated **product lattice** for smart casts:

```
SmartCastData = Expression -> SmartCastType
SmartCastType = P x N
  where P = type expression definitely has
        N = type expression definitely does NOT have
```

**Join operation** (merging branches):
```
P1 x N1 ⊔ P2 x N2 = LUB(P1, P2) x GLB(N1, N2)
```

**Meet operation** (intersecting conditions):
```
P1 x N1 ⊓ P2 x N2 = GLB(P1, P2) x LUB(N1, N2)
```

### 3.2 Transfer Functions

| Condition | Transfer |
|-----------|----------|
| `x is T` | `s[x -> s(x) ⊓ (T x Top)]` |
| `x !is T` | `s[x -> s(x) ⊓ (Top x T)]` |
| `x == null` | `s[x -> s(x) ⊓ (Nothing? x Top)]` |
| `x != null` | `s[x -> s(x) ⊓ (Top x Nothing?)]` |
| `x == y` | `s[x -> s(x) ⊓ s(y), y -> s(x) ⊓ s(y)]` |
| `x = y` | `s[x -> s(y)]` |

### 3.3 Stability Requirements

Smart casts only apply to **stable** expressions:

| Property Kind | Smart Cast Eligible |
|---------------|---------------------|
| Immutable local (`let`) | Yes |
| Mutable local (`var`) | Yes, if not reassigned between check and use |
| Immutable property | Yes, if no custom getter |
| Mutable property | No (could change externally) |

### 3.4 Bound Smart Casts (Aliasing)

Kotlin propagates narrowing across aliased variables:

```kotlin
val a: Any? = ...
val b = a

if (b is Int) {
    a.inc()  // a also narrows to Int
}
```

**Reference**: [Kotlin Language Specification - Type Inference](https://kotlinlang.org/spec/type-inference.html)

---

## 4. Design: Aria Flow-Sensitive Narrowing

### 4.1 Core Data Structures

```rust
/// Narrowed type information for a variable
#[derive(Debug, Clone)]
pub struct NarrowedType {
    /// Base type (from declaration)
    pub base: Type,
    /// Type the variable definitely IS (positive refinement)
    pub positive: Option<Type>,
    /// Type the variable definitely is NOT (negative refinement)
    pub negative: Option<Type>,
}

impl NarrowedType {
    /// Compute the effective narrowed type
    pub fn effective_type(&self) -> Type {
        match (&self.positive, &self.negative) {
            (Some(p), None) => type_intersection(&self.base, p),
            (None, Some(n)) => type_difference(&self.base, n),
            (Some(p), Some(n)) => type_difference(
                &type_intersection(&self.base, p),
                n
            ),
            (None, None) => self.base.clone(),
        }
    }
}

/// Flow-sensitive type environment
#[derive(Debug, Clone)]
pub struct FlowTypeEnv {
    /// Variable bindings: name -> declared type
    variables: FxHashMap<String, Type>,
    /// Narrowing information: name -> narrowed type
    narrowings: FxHashMap<String, NarrowedType>,
    /// Type definitions
    types: FxHashMap<String, TypeScheme>,
    /// Parent scope
    parent: Option<Rc<FlowTypeEnv>>,
    /// Stability tracking for smart casts
    stability: FxHashMap<String, Stability>,
}

/// Whether a variable is stable for smart casting
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Stability {
    /// Immutable binding (let)
    Immutable,
    /// Mutable but not reassigned in relevant scope
    EffectivelyImmutable,
    /// Mutable and reassigned - no smart cast
    Mutable,
    /// Captured by closure that could mutate
    Captured,
}
```

### 4.2 Narrowing Rules

#### 4.2.1 Null Checks

```aria
fn process(x: Int?) -> Int
  if x != nil
    # x narrows from Int? to Int
    return x + 1
  else
    # x narrows to type that IS nil (Nothing? effectively)
    return 0
  end
end
```

**Transfer function**:
```
[[x != nil]](env) = env.narrow(x, positive=inner_type(x))
[[x == nil]](env) = env.narrow(x, negative=inner_type(x))
```

#### 4.2.2 Type Checks with `is`

```aria
fn describe(value: Any) -> String
  if value is String
    # value narrows to String
    return "string: #{value.length}"
  elsif value is Int
    # value narrows to Int
    return "int: #{value}"
  else
    return "unknown"
  end
end
```

**Transfer function**:
```
[[x is T]](env) = env.narrow(x, positive=T)
[[x !is T]](env) = env.narrow(x, negative=T)
```

#### 4.2.3 Pattern Matching

```aria
match result
  Ok(value) =>
    # result narrows to Ok type
    # value binds to inner type
    process(value)
  Err(error) =>
    # result narrows to Err type
    log_error(error)
end
```

**Pattern narrowing**:
- Variant patterns narrow the scrutinee to that variant
- Binding patterns introduce narrowed bindings
- Guard expressions add additional narrowings

#### 4.2.4 Truthiness

```aria
fn safe_process(x: String?) -> Int
  if x
    # x narrows from String? to String (excludes nil)
    return x.length
  end
  return 0
end
```

**Truthiness rules for Aria**:
| Type | Falsy Values |
|------|--------------|
| `T?` | `nil` |
| `Bool` | `false` |
| `Result<T,E>` | `Err(_)` |

### 4.3 Control Flow Analysis

#### 4.3.1 Branch Merging

When branches rejoin, narrowings are merged using **join** (least upper bound):

```aria
fn example(x: Int?) -> Int
  if condition
    x = get_value()  # x may be nil or not
  else
    if x != nil
      # x is Int here
    end
  end
  # After merge: x is Int? (LUB of both branches)
  return x ?? 0
end
```

#### 4.3.2 Early Returns and Never Type

```aria
fn process(x: Int?) -> Int
  if x == nil
    return 0  # Returns, rest of function assumes x != nil
  end
  # x narrows to Int (early return eliminates nil case)
  return x * 2
end
```

**Never type propagation**: Branches ending in `return`, `throw`, or expressions of type `Never` do not contribute to join.

#### 4.3.3 Loop Handling

```aria
fn find_first_positive(arr: Array[Int?]) -> Int?
  for item in arr
    if item != nil and item > 0
      return item  # item is Int here
    end
  end
  return nil
end
```

**Loop rules**:
- `while(true)` and `do-while` guarantee at least one iteration
- Loop body narrowings do NOT propagate outside (could be zero iterations)
- `break` from infinite loop propagates narrowings

### 4.4 Integration with TypeEnv

```rust
impl FlowTypeEnv {
    /// Look up variable type with narrowing applied
    pub fn lookup_var(&self, name: &str) -> Option<Type> {
        // First check for narrowing in current scope
        if let Some(narrowed) = self.narrowings.get(name) {
            return Some(narrowed.effective_type());
        }

        // Then check declared type
        if let Some(ty) = self.variables.get(name) {
            return Some(ty.clone());
        }

        // Finally check parent scope
        self.parent.as_ref().and_then(|p| p.lookup_var(name))
    }

    /// Create child scope with additional narrowing
    pub fn with_narrowing(&self, name: &str, narrowing: NarrowedType) -> Self {
        let mut child = FlowTypeEnv::with_parent(Rc::new(self.clone()));
        child.narrowings.insert(name.to_string(), narrowing);
        child
    }

    /// Check if variable is stable for smart casting
    pub fn is_stable(&self, name: &str) -> bool {
        self.stability.get(name)
            .map(|s| matches!(s, Stability::Immutable | Stability::EffectivelyImmutable))
            .unwrap_or(false)
    }
}
```

### 4.5 Contract System Integration

Aria's contract system (requires/ensures) provides additional narrowing information:

```aria
fn binary_search(arr: Array[Int], target: Int) -> Int?
  requires arr.sorted?
  requires arr.length > 0
  # Within function body:
  # - arr.sorted? is truthy (can use sorted-specific optimizations)
  # - arr.length narrows to Int > 0
end
```

**Contract-based narrowing**:

```rust
/// Extract narrowing information from requires clauses
fn narrowings_from_contract(contract: &ContractClause) -> Vec<(String, NarrowedType)> {
    match &contract.condition {
        // x != nil
        Expr::Binary { op: NotEq, left, right }
            if is_nil(right) && is_var(left) => {
            vec![(var_name(left), NarrowedType::non_null(...))]
        }
        // x > 0
        Expr::Binary { op: Gt, left, right }
            if is_var(left) && is_zero(right) => {
            vec![(var_name(left), NarrowedType::positive_int())]
        }
        // arr.length > 0
        Expr::Binary { op: Gt, left: Expr::Field { object, field }, right }
            if field == "length" && is_zero(right) => {
            // Could enable non-empty array optimizations
            vec![]
        }
        _ => vec![]
    }
}
```

### 4.6 Ensures Clause and Result Narrowing

```aria
fn find_user(id: Int) -> User?
  ensures |result|
    result.none? or result.unwrap.id == id
  # The ensures clause confirms:
  # - If result is Some, the user.id matches
  # This information available to callers via effect analysis
end
```

---

## 5. Implementation Strategy

### 5.1 Phase 1: Basic Narrowing (Milestone M01)

1. **Extend TypeEnv** with narrowing map
2. **Implement null check narrowing** (`!= nil`, `== nil`)
3. **Implement type check narrowing** (`is`, `!is`)
4. **Early return narrowing** (Never type propagation)

### 5.2 Phase 2: Pattern Integration (Milestone M14)

1. **Pattern match narrowing** - scrutinee narrows to matched variant
2. **Guard expression narrowing** - additional refinements in guards
3. **Or-pattern handling** - union of pattern types

### 5.3 Phase 3: Advanced Features

1. **Contract-based narrowing** - requires clauses inform function body
2. **Bound smart casts** - aliased variable narrowing
3. **Closure capture analysis** - track stability through closures
4. **Cross-function narrowing** - type predicates (user-defined type guards)

### 5.4 Performance Considerations

| Strategy | Benefit |
|----------|---------|
| Lazy evaluation | Only compute narrowings when needed |
| CFG caching | Reuse control flow graph structure |
| Incremental updates | Update narrowings incrementally on edits |
| Scope-limited | Narrowings don't escape function boundaries |

---

## 6. Example Code

### 6.1 Before Flow-Sensitive Typing

```aria
fn process_value(opt: Int?) -> String
  # Must use explicit unwrapping
  if opt != nil
    let value = opt!  # Explicit unwrap required
    return "Value: #{value}"
  end
  return "None"
end
```

### 6.2 After Flow-Sensitive Typing

```aria
fn process_value(opt: Int?) -> String
  if opt != nil
    # opt automatically narrows to Int
    return "Value: #{opt}"
  end
  return "None"
end

# More complex example with multiple narrowings
fn complex_example(x: Any?) -> String
  if x == nil
    return "nil"
  end

  # x is now Any (not nil)

  if x is String
    # x is now String
    return "string of length #{x.length}"
  elsif x is Int
    # x is now Int
    return "integer: #{x}"
  end

  # x is Any but NOT String and NOT Int
  return "other: #{x}"
end

# Pattern matching with narrowing
fn handle_result(result: Result[Int, String]) -> Int
  match result
    Ok(value) =>
      # value is Int, result narrows to Ok variant
      value * 2
    Err(msg) =>
      # msg is String, result narrows to Err variant
      log(msg)
      -1
  end
end

# Contract-informed narrowing
fn binary_search(arr: Array[Int], target: Int) -> Int?
  requires arr.length > 0
  requires arr.sorted?

  # arr.length is known to be > 0
  # Can safely access arr[0] without bounds check concern

  var low = 0
  var high = arr.length - 1  # Safe: length > 0

  while low <= high
    let mid = (low + high) // 2
    let value = arr[mid]

    match value <=> target
      -1 => low = mid + 1
       0 => return Some(mid)
       1 => high = mid - 1
    end
  end

  return None
end
```

---

## 7. Type Operations

### 7.1 Type Intersection

```rust
/// Compute intersection of two types (for positive narrowing)
fn type_intersection(t1: &Type, t2: &Type) -> Type {
    match (t1, t2) {
        // Same types intersect to themselves
        _ if t1 == t2 => t1.clone(),

        // Optional with non-optional: the non-optional
        (Type::Optional(inner), t) | (t, Type::Optional(inner))
            if **inner == *t => t.clone(),

        // Any intersects with anything
        (Type::Any, t) | (t, Type::Any) => t.clone(),

        // Named types: check subtyping
        (Type::Named { name: n1, .. }, Type::Named { name: n2, .. })
            if is_subtype(t1, t2) => t1.clone(),
        (Type::Named { name: n1, .. }, Type::Named { name: n2, .. })
            if is_subtype(t2, t1) => t2.clone(),

        // No intersection possible
        _ => Type::Never,
    }
}
```

### 7.2 Type Difference

```rust
/// Compute type difference (for negative narrowing)
fn type_difference(t: &Type, exclude: &Type) -> Type {
    match (t, exclude) {
        // Excluding nil from optional gives the inner type
        (Type::Optional(inner), Type::Named { name, .. })
            if name == "Nothing" || is_nil_type(exclude) => {
            (**inner).clone()
        }

        // Cannot exclude non-overlapping type
        _ if !types_overlap(t, exclude) => t.clone(),

        // Excluding exact type from union
        // (Would need union types for full implementation)

        // Default: cannot narrow
        _ => t.clone(),
    }
}
```

---

## 8. Error Messages

### 8.1 Narrowing Not Applied

```
Warning: Smart cast not applied

  10 | var x: Int? = get_value()
  11 | if x != nil
  12 |   modify(&x)  # x reassigned
  13 |   print(x)    # x is still Int? here
                    ^^^^^^

Note: 'x' is reassigned on line 12, invalidating the null check.
Help: Use a local binding: let value = x; if value != nil ...
```

### 8.2 Unstable Property

```
Warning: Cannot smart cast 'self.value' - property is mutable

  5 | if self.value != nil
  6 |   process(self.value)
              ^^^^^^^^^^^

Note: 'value' is a mutable property and could be modified between
      the check and use.
Help: Copy to local: let v = self.value; if v != nil { process(v) }
```

---

## 9. Comparison with Other Languages

| Feature | TypeScript | Kotlin | Aria (Proposed) |
|---------|------------|--------|-----------------|
| Null narrowing | Yes | Yes | Yes |
| Type guards (`is`) | Yes | Yes | Yes |
| User-defined guards | Type predicates | No | Future (Phase 3) |
| Pattern narrowing | Limited | Yes (when) | Yes |
| Contract integration | No | No | **Yes** |
| Mutable var narrowing | Limited | Conditional | Conditional |
| Cross-variable | Limited | Yes (bound) | Future |
| Ownership-aware | No | No | **Yes** (Phase 3) |

---

## 10. Open Questions

1. **Closure capture**: How do we track mutations through closures?
2. **Concurrent access**: How does narrowing interact with concurrent/async code?
3. **Custom type guards**: Syntax for user-defined type predicates?
4. **Performance**: Should we cache CFG per function or rebuild?
5. **IDE integration**: How to show narrowed types in hover information?

---

## 11. Key Resources

1. [TypeScript Narrowing Documentation](https://www.typescriptlang.org/docs/handbook/2/narrowing.html)
2. [Effective TypeScript - Flow Nodes](https://effectivetypescript.com/2024/03/24/flownodes/)
3. [Kotlin Language Specification - Type Inference](https://kotlinlang.org/spec/type-inference.html)
4. [Kotlin Smart Casts Documentation](https://kotlinlang.org/docs/typecasts.html)
5. [Rust Pattern Syntax](https://doc.rust-lang.org/book/ch19-03-pattern-syntax.html)
6. [Concise Control Flow with if let](https://doc.rust-lang.org/book/ch06-03-if-let.html)

---

## 12. Conclusions

Flow-sensitive type narrowing is essential for Aria's goal of combining Rust-level safety with Ruby/Python-like ergonomics. The proposed design:

1. **Extends TypeEnv** with a narrowing map and stability tracking
2. **Uses a lattice-based approach** (inspired by Kotlin) for sound narrowing
3. **Integrates with contracts** - a unique feature not found in TypeScript or Kotlin
4. **Supports pattern matching** - narrowing through match arms
5. **Tracks stability** - ensuring smart casts are only applied when safe

The phased implementation approach allows early benefits (null check narrowing) while building toward advanced features (contract integration, ownership-aware narrowing).

---

## Appendix A: Modified TypeEnv API

```rust
impl FlowTypeEnv {
    // === Construction ===
    pub fn new() -> Self;
    pub fn with_parent(parent: Rc<FlowTypeEnv>) -> Self;

    // === Basic operations ===
    pub fn define_var(&mut self, name: String, ty: Type);
    pub fn define_var_mutable(&mut self, name: String, ty: Type);
    pub fn lookup_var(&self, name: &str) -> Option<Type>;

    // === Narrowing operations ===
    pub fn narrow_positive(&mut self, name: &str, ty: Type);
    pub fn narrow_negative(&mut self, name: &str, ty: Type);
    pub fn clear_narrowing(&mut self, name: &str);

    // === Branching ===
    pub fn branch(&self) -> Self;  // Create copy for branch
    pub fn join(&self, other: &Self) -> Self;  // Merge branches

    // === Stability ===
    pub fn mark_reassigned(&mut self, name: &str);
    pub fn mark_captured(&mut self, name: &str);
    pub fn is_stable(&self, name: &str) -> bool;

    // === Contract integration ===
    pub fn apply_requires(&mut self, contracts: &[ContractClause]);
    pub fn narrowings_at_ensures(&self) -> Vec<(String, Type)>;
}
```

---

## Appendix B: CFG Node Types

```rust
/// Control Flow Graph node for narrowing analysis
#[derive(Debug, Clone)]
pub enum FlowNode {
    /// Start of function/block
    Start,

    /// Variable assignment
    Assignment {
        target: String,
        value_type: Type,
    },

    /// Condition branch
    Condition {
        condition: ConditionKind,
        then_node: Box<FlowNode>,
        else_node: Option<Box<FlowNode>>,
    },

    /// Pattern match
    Match {
        scrutinee: String,
        arms: Vec<MatchArmNode>,
    },

    /// Loop entry
    LoopEntry {
        kind: LoopKind,
        body: Box<FlowNode>,
    },

    /// Early exit (return, break, throw)
    Exit {
        kind: ExitKind,
    },

    /// Sequence of nodes
    Sequence(Vec<FlowNode>),

    /// Join point (merge of branches)
    Join(Vec<Box<FlowNode>>),
}

#[derive(Debug, Clone)]
pub enum ConditionKind {
    NullCheck { var: String, is_null: bool },
    TypeCheck { var: String, ty: Type, is_type: bool },
    Truthiness { var: String },
    Equality { left: String, right: String },
    Custom(Box<Expr>),
}

#[derive(Debug, Clone)]
pub enum LoopKind {
    While,
    DoWhile,
    For,
    Loop,  // infinite loop
}

#[derive(Debug, Clone)]
pub enum ExitKind {
    Return,
    Break,
    Continue,
    Throw,
}
```
