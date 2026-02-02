# ARIA-M04-04: Tiered Contract System Design

**Task ID**: ARIA-M04-04
**Status**: Completed
**Date**: 2026-01-15
**Focus**: Three-tier contract verification with SMT integration
**Agent**: SENTINEL (Research)

---

## Executive Summary

This research defines Aria's three-tier contract verification system, balancing verification power with practical usability. The system classifies contracts based on their verifiability: Tier 1 (Static) for decidable SMT-solvable properties, Tier 2 (Cached) for pure computations with memoization, and Tier 3 (Dynamic) for runtime-only verification. This hybrid approach draws from Dafny's SMT integration, SPARK's formal verification methodology, and Eiffel's practical DbC.

---

## 1. Design Philosophy

### 1.1 The Verification Spectrum

Programming languages face a fundamental tradeoff between verification power and usability:

```
        Low Annotation                                   High Guarantee
              |                                                |
              v                                                v
    +---------+----------+-----------+-----------+------------+
    | No      | Type     | Simple    | Full      | Theorem    |
    | Checks  | Checking | Contracts | Contracts | Provers    |
    +---------+----------+-----------+-----------+------------+
    Python    Java       Eiffel      Dafny       Coq/Agda

    Aria's target: Cover the middle three with graceful degradation
```

### 1.2 Core Principles

| Principle | Description |
|-----------|-------------|
| **Progressive Verification** | Simple contracts should "just work" statically |
| **Explicit Degradation** | When static verification fails, fall back gracefully |
| **Zero Runtime Cost Option** | Production builds can disable all runtime checks |
| **Actionable Feedback** | Clear messages about why verification failed |
| **SMT-Friendly Core** | Design contracts to maximize static verifiability |

### 1.3 Decidability Foundations

From SMT theory, we know certain problems are decidable:

| Theory | Decidability | Aria Application |
|--------|--------------|------------------|
| Linear Arithmetic (LIA) | Decidable | Bounds checks, indices |
| Presburger Arithmetic | Decidable | Integer comparisons |
| Uninterpreted Functions | Decidable | Method abstractions |
| Quantifier-Free Arrays | Decidable | Basic array access |
| Full First-Order Logic | Undecidable | Quantified contracts |
| Non-linear Arithmetic | Undecidable | Complex math |

---

## 2. Three-Tier Classification

### 2.1 Tier Overview

```
+-------------------------------------------------------------------+
|                    TIER 1: STATIC                                  |
|  Fully verified at compile time using SMT solver                  |
|  - Null checks, bounds, type guards, simple arithmetic            |
|  - Zero runtime overhead                                          |
+-------------------------------------------------------------------+
                              |
                              v
+-------------------------------------------------------------------+
|                    TIER 2: CACHED                                  |
|  Static analysis with caching for pure computations               |
|  - Pure method calls, array access, field access                  |
|  - Verified once, result cached                                   |
+-------------------------------------------------------------------+
                              |
                              v
+-------------------------------------------------------------------+
|                    TIER 3: DYNAMIC                                 |
|  Runtime verification only                                        |
|  - Quantifiers, closures, complex expressions                     |
|  - Property-based testing integration                             |
+-------------------------------------------------------------------+
```

### 2.2 Tier 1: Static Contracts

**Definition**: Contracts that can be fully verified at compile time using decidable SMT theories.

**Characteristics**:
- Zero runtime overhead
- Compile-time guarantee
- No annotation beyond the contract itself
- Errors are compile-time errors

**Supported Contract Forms**:

```aria
# Null checks (type system integration)
fn process(data: String?)
  requires data != nil
  # Compiler verifies via flow analysis
end

# Simple bounds
fn get_index(arr: Array[T], i: Int) -> T
  requires i >= 0
  requires i < arr.length
  # Linear arithmetic: decidable
end

# Type guards
fn narrow(value: Animal) -> Dog
  requires value is Dog
  # Type narrowing via subtype check
end

# Simple arithmetic
fn divide(a: Int, b: Int) -> Int
  requires b != 0
  ensures result * b <= a
  ensures result * b > a - b
  # Linear arithmetic with multiplication by constant
end

# Boolean combinations
fn overlap(a: Range, b: Range) -> Bool
  requires a.start < a.end
  requires b.start < b.end
  # Conjunction of linear constraints
end
```

**Classification Criteria**:

| Criterion | Tier 1 Eligible | Example |
|-----------|-----------------|---------|
| Only linear arithmetic | Yes | `x + y < z * 2` |
| Equality/inequality on primitives | Yes | `x != 0`, `s == ""` |
| Null/nil checks | Yes | `obj != nil` |
| Type tests (is/as) | Yes | `x is String` |
| Boolean connectives | Yes | `a && b`, `a \|\| b` |
| Enum membership | Yes | `status in [:ok, :err]` |

### 2.3 Tier 2: Cached Contracts

**Definition**: Contracts involving pure computations that can be statically analyzed once and cached.

**Characteristics**:
- Compile-time verification with memoization
- Requires purity analysis
- May require abstract interpretation
- Results cached across call sites

**Supported Contract Forms**:

```aria
# Pure method calls
fn binary_search(arr: Array[Int], target: Int) -> Int?
  requires arr.sorted?  # Pure method: cached analysis
  # sorted? is pure, result cached per arr identity
end

# Field access chains
fn update_user(user: User)
  requires user.profile.verified
  # Immutable field access: static analysis
end

# Array element access
fn swap(arr: Array[Int], i: Int, j: Int)
  requires arr[i] > arr[j]  # Requires value analysis
  # May need abstract interpretation
end

# Collection operations
fn merge_sorted(a: Array[Int], b: Array[Int]) -> Array[Int]
  requires a.sorted? && b.sorted?
  ensures result.sorted?
  ensures result.length == a.length + b.length
  # sorted? analyzed once per input
end

# Computed properties
fn process_order(order: Order)
  requires order.total > 0
  requires order.items.all? |item| item.price > 0 end
  # all? is pure, can be cached
end
```

**Caching Strategy**:

```
Tier 2 Verification Pipeline:
1. Identify pure expressions in contract
2. Compute abstract values where possible
3. Generate verification conditions
4. Cache: (expression, context) -> verifiability
5. On re-encounter, use cached result
```

**Classification Criteria**:

| Criterion | Tier 2 Eligible | Example |
|-----------|-----------------|---------|
| Pure method calls | Yes | `arr.sorted?`, `str.empty?` |
| Immutable field access | Yes | `user.age`, `point.x` |
| Array element access | Yes | `arr[0]`, `matrix[i][j]` |
| Collection predicates | Yes | `list.all?`, `set.any?` |
| Computed properties | Yes | `order.subtotal` |
| Side-effect free closures | Partial | Limited to simple predicates |

### 2.4 Tier 3: Dynamic Contracts

**Definition**: Contracts that cannot be statically verified and must be checked at runtime.

**Characteristics**:
- Runtime overhead (can be disabled)
- Violation is a runtime error
- Integrates with property-based testing
- May use probabilistic verification

**Supported Contract Forms**:

```aria
# Universal quantifiers
fn is_sorted(arr: Array[Int]) -> Bool
  ensures forall i, j: Int |
    0 <= i < j < arr.length implies arr[i] <= arr[j]
  end
  # Quantifier over unbounded domain
end

# Existential quantifiers
fn contains_duplicate(arr: Array[Int]) -> Bool
  ensures result == exists i, j: Int |
    i != j && arr[i] == arr[j]
  end
end

# Complex closures
fn transform(data: Data, f: (Int) -> Int)
  ensures data.values.map(f).all? |x| x > 0 end
  # Closure f is opaque
end

# Higher-order contracts
fn sort_by[T](arr: Array[T], cmp: (T, T) -> Bool) -> Array[T]
  requires forall a, b: T | cmp(a, b) implies !cmp(b, a) end  # Antisymmetric
  requires forall a, b, c: T | cmp(a, b) && cmp(b, c) implies cmp(a, c) end  # Transitive
  # Cannot verify comparator properties statically
end

# Database/IO dependent
fn save_user(user: User)
  ensures User.find_by_id(user.id) == Some(user)
  # Depends on external state
end

# Performance contracts
fn quick_sort(arr: Array[Int]) -> Array[Int]
  ensures result.sorted?
  ensures result.permutation_of?(arr)
  complexity O(n * log(n)) average
  # Complexity cannot be proven statically
end
```

**Classification Criteria**:

| Criterion | Tier 3 (Runtime Only) | Example |
|-----------|----------------------|---------|
| Universal quantifiers | Yes | `forall x: T \| P(x)` |
| Existential quantifiers | Yes | `exists x: T \| P(x)` |
| Opaque closures | Yes | `f(x) > 0` where f unknown |
| IO/Effect-dependent | Yes | Database queries |
| Non-linear arithmetic | Yes | `x * y > z * w` |
| Recursive predicates | Yes | Tree balance checks |

---

## 3. Tier Classification Algorithm

### 3.1 Algorithm Overview

```
TierClassify(contract: ContractExpr) -> Tier:
  1. Parse contract into AST
  2. For each sub-expression:
     a. Check against Tier 1 rules
     b. If fails, check Tier 2 rules
     c. If fails, assign Tier 3
  3. Contract tier = max tier of all sub-expressions
  4. Cache classification result
```

### 3.2 Detailed Classification Rules

```aria
# Pseudocode for tier classification
module ContractClassifier
  fn classify(expr: ContractExpr) -> Tier
    match expr
      # === TIER 1 PATTERNS ===

      # Null checks
      NullCheck(e) => Tier.Static

      # Linear arithmetic comparisons
      Compare(left, op, right) when linear_arithmetic?(left, right) =>
        Tier.Static

      # Type tests
      TypeTest(e, type) => Tier.Static

      # Boolean combinations of Tier 1
      And(a, b) when classify(a) == Tier.Static && classify(b) == Tier.Static =>
        Tier.Static
      Or(a, b) when classify(a) == Tier.Static && classify(b) == Tier.Static =>
        Tier.Static
      Not(a) when classify(a) == Tier.Static =>
        Tier.Static

      # === TIER 2 PATTERNS ===

      # Pure method calls
      MethodCall(receiver, method, args) when pure_method?(method) =>
        max(Tier.Cached, classify(receiver), *args.map(&classify))

      # Field access
      FieldAccess(obj, field) when immutable_field?(field) =>
        max(Tier.Cached, classify(obj))

      # Array access with static index
      ArrayAccess(arr, index) when classify(index) == Tier.Static =>
        Tier.Cached

      # === TIER 3 PATTERNS ===

      # Quantifiers
      Forall(vars, body) => Tier.Dynamic
      Exists(vars, body) => Tier.Dynamic

      # Opaque closures
      ClosureCall(closure, args) => Tier.Dynamic

      # Old expression (special handling)
      Old(expr) =>
        # old() can be Tier 1/2 if expr is, but requires snapshot
        max(classify(expr), Tier.Cached)

      # Default: Dynamic
      _ => Tier.Dynamic
    end
  end

  fn linear_arithmetic?(exprs: Array[Expr]) -> Bool
    # Check all expressions use only:
    # - Integer constants
    # - Variables
    # - Addition, subtraction
    # - Multiplication by constants only
    exprs.all? |e| linear_expr?(e) end
  end

  fn pure_method?(method: Method) -> Bool
    # Method is pure if:
    # - Annotated @pure
    # - Inferred pure by effect system
    # - Built-in pure method (length, empty?, etc.)
    method.has_annotation?(:pure) ||
      method.inferred_effects.empty? ||
      BUILTIN_PURE_METHODS.include?(method.name)
  end
end
```

### 3.3 Classification Examples

```aria
# Example 1: Tier 1 (Static)
fn example1(x: Int, y: Int)
  requires x > 0           # Tier 1: linear comparison
  requires y >= x          # Tier 1: linear comparison
  requires x + y < 100     # Tier 1: linear arithmetic
  # Overall: Tier 1
end

# Example 2: Tier 2 (Cached)
fn example2(arr: Array[Int])
  requires arr.length > 0       # Tier 2: pure method call
  requires arr.sorted?          # Tier 2: pure method call
  requires arr[0] < arr[-1]     # Tier 2: array access
  # Overall: Tier 2
end

# Example 3: Tier 3 (Dynamic)
fn example3(arr: Array[Int], pred: (Int) -> Bool)
  requires arr.all?(pred)       # Tier 3: opaque closure
  requires forall i | 0 <= i < arr.length implies arr[i] > 0 end
  # Overall: Tier 3
end

# Example 4: Mixed Tiers
fn example4(arr: Array[Int], n: Int)
  requires n > 0                # Tier 1
  requires arr.length >= n      # Tier 2
  requires arr.sorted?          # Tier 2
  requires forall i | i < n implies arr[i] < 100 end  # Tier 3
  # Overall: Tier 3 (max of all)
end
```

---

## 4. Checking Modes

### 4.1 Mode Overview

Aria provides four checking modes via the `@contracts` annotation:

| Mode | Syntax | Behavior |
|------|--------|----------|
| **Static** | `@contracts(:static)` | Only Tier 1 contracts verified; Tier 2/3 ignored |
| **Runtime** | `@contracts(:runtime)` | All contracts checked at runtime |
| **Off** | `@contracts(:off)` | All contract checking disabled |
| **Full** | `@contracts(:full)` | Static + Runtime: Tier 1 static, Tier 2/3 runtime |

### 4.2 Mode Semantics

#### @contracts(:static)

```aria
@contracts(:static)
fn fast_divide(a: Int, b: Int) -> Int
  requires b != 0              # Tier 1: verified at compile time
  requires a.abs < 1000000     # Tier 2: WARNING - not verified
  a / b
end

# Compile output:
# Warning: Contract `a.abs < 1000000` is Tier 2, skipped in :static mode
# Consider using @contracts(:full) for comprehensive checking
```

**Semantics**:
- Tier 1 contracts: Compile-time verification, compile error if fails
- Tier 2 contracts: Warning issued, not verified
- Tier 3 contracts: Warning issued, not verified
- Zero runtime overhead guaranteed

#### @contracts(:runtime)

```aria
@contracts(:runtime)
fn flexible_search(arr: Array[Int], target: Int) -> Int?
  requires arr.sorted?         # Checked at each call
  requires arr.length > 0      # Checked at each call
  # ...
end

# Every call to flexible_search evaluates preconditions
# Contract violation raises ContractError at runtime
```

**Semantics**:
- All contracts: Evaluated at runtime
- Tier 1 contracts: Could be static but runtime chosen
- Contract violation: Raises `ContractError` exception
- Use case: Testing, debugging, development

#### @contracts(:off)

```aria
@contracts(:off)
fn performance_critical(data: LargeData) -> Result
  requires data.valid?         # IGNORED
  ensures result.consistent?   # IGNORED
  # ... performance-critical code ...
end

# No contract checking whatsoever
# Programmer asserts correctness manually
```

**Semantics**:
- All contracts: Completely ignored
- Zero overhead, zero safety
- Use case: Verified hot paths, trusted code
- **Warning**: Use with extreme caution

#### @contracts(:full)

```aria
@contracts(:full)  # DEFAULT MODE
fn binary_search(arr: Array[Int], target: Int) -> Int?
  requires arr.length > 0      # Tier 1: static
  requires arr.sorted?         # Tier 2: static with caching
  requires target != Int.MIN   # Tier 1: static
  ensures |result|             # Tier 3: runtime
    result.none? || arr[result.unwrap] == target
  end
  # ...
end

# Tier 1: Compile-time error if unprovable
# Tier 2: Static analysis with caching
# Tier 3: Runtime check inserted
```

**Semantics**:
- Tier 1: Compile-time verification
- Tier 2: Static verification with caching
- Tier 3: Runtime verification
- Best balance of safety and performance

### 4.3 Mode Inheritance and Scoping

```aria
# Module-level default
@contracts(:full)
module BankingCore

  # Override for specific function
  @contracts(:static)
  fn calculate_interest(principal: Float, rate: Float) -> Float
    requires principal >= 0
    requires rate >= 0 && rate <= 1
    principal * rate
  end

  # Inherits module-level :full
  fn transfer(from: Account, to: Account, amount: Float)
    requires from.balance >= amount
    requires amount > 0
    ensures from.balance == old(from.balance) - amount
    ensures to.balance == old(to.balance) + amount
    # ...
  end
end

# Project-level configuration (aria.toml)
[contracts]
default_mode = "full"
production_mode = "static"  # Used with --release flag
```

### 4.4 Compile-Time Mode Selection

```bash
# Development build (full checking)
aria build                    # Uses @contracts(:full) default

# Release build (static only, maximum performance)
aria build --release          # Uses production_mode from config

# Explicit override
aria build --contracts=runtime  # Force runtime checking everywhere

# Testing with full contracts
aria test --contracts=full    # Maximum safety for tests
```

---

## 5. SMT Integration Points

### 5.1 SMT Solver Architecture

```
                    Aria Contract
                         |
                         v
              +--------------------+
              | Contract Classifier |
              +--------------------+
                    |    |    |
         Tier 1     |    |    |     Tier 3
            |       |    |    |        |
            v       |    |    |        v
     +----------+   |    |    |   +---------+
     | SMT Gen  |   |    |    |   | Codegen |
     +----------+   |    |    |   +---------+
            |       |    |    |        |
            v       |    |    |        v
     +----------+   |    |    |   +------------+
     | Z3/CVC5  |   |    |    |   | Runtime    |
     +----------+   |    |    |   | Checker    |
            |       v    v    v   +------------+
            |    +----------+
            +--->| Tier 2   |
                 | Analyzer |
                 +----------+
                      |
                      v
               +------------+
               | Result     |
               | Cache      |
               +------------+
```

### 5.2 SMT-LIB Generation for Tier 1

```aria
# Aria contract
fn safe_array_access(arr: Array[Int], i: Int) -> Int
  requires i >= 0
  requires i < arr.length
  arr[i]
end

# Generated SMT-LIB (simplified)
; Declarations
(declare-const i Int)
(declare-const arr_length Int)

; Axioms
(assert (>= arr_length 0))  ; Array length non-negative

; Preconditions to verify at call site
(assert (>= i 0))
(assert (< i arr_length))

; Check satisfiability of negation (counterexample search)
(check-sat)
```

### 5.3 Verification Condition Generation

```aria
# For each function call, generate VC:

# Given:
fn caller()
  x = 5
  y = 10
  result = safe_divide(x, y)  # requires y != 0
end

# Generated VC:
# prove: y != 0
# i.e., prove: 10 != 0
# SMT: (assert (not (= 10 0))) -> UNSAT -> verified
```

### 5.4 Handling Tier 2 with Abstract Interpretation

```aria
# For pure method calls, use abstract interpretation:

fn process(arr: Array[Int])
  requires arr.sorted?
  # ...
end

# Abstract interpretation:
# 1. Track abstract state of arr
# 2. If arr comes from sort(), mark as sorted
# 3. If arr modified, invalidate sorted property
# 4. At requires, check abstract state

# Example analysis:
fn caller()
  arr = [3, 1, 4, 1, 5]
  sorted_arr = arr.sort()      # Abstract: sorted = true
  process(sorted_arr)          # Verified: sorted = true
end
```

### 5.5 SMT Timeout Handling

```aria
# Configuration for SMT solver limits
@contracts(:full, smt_timeout: 5000)  # 5 second timeout
fn complex_verification(data: ComplexData)
  requires complex_invariant(data)
  # ...
end

# If SMT times out:
# 1. Downgrade to Tier 3 (runtime)
# 2. Issue warning with suggestion
# 3. Continue compilation

# Warning output:
# Warning: Contract `complex_invariant(data)` SMT verification timed out
#   Downgraded to runtime check
#   Suggestion: Simplify contract or increase timeout
```

---

## 6. Contract Examples by Tier

### 6.1 Complete Tier 1 Examples

```aria
# Example: Safe division
@contracts(:static)
fn safe_divide(a: Int, b: Int) -> Int
  requires b != 0
  ensures result == a / b
  a / b
end

# Example: Bounded array creation
@contracts(:static)
fn create_buffer(size: Int) -> Array[Byte]
  requires size > 0
  requires size <= 1024 * 1024  # Max 1MB
  ensures result.length == size
  Array.new(size, 0)
end

# Example: Range validation
@contracts(:static)
fn clamp(value: Int, min: Int, max: Int) -> Int
  requires min <= max
  ensures result >= min
  ensures result <= max
  if value < min then min
  elif value > max then max
  else value
  end
end

# Example: Non-null unwrap
@contracts(:static)
fn unwrap_or_default[T](opt: Option[T], default: T) -> T
  ensures result != nil || default == nil
  match opt
    Some(v) => v
    None => default
  end
end
```

### 6.2 Complete Tier 2 Examples

```aria
# Example: Sorted array operations
@contracts(:full)
fn binary_search(arr: Array[Int], target: Int) -> Int?
  requires arr.sorted?           # Pure method call
  requires arr.length > 0        # Pure property access
  ensures |result|
    result.none? || arr[result.unwrap] == target
  end
  # ... binary search implementation ...
end

# Example: String validation
@contracts(:full)
fn parse_email(input: String) -> Email?
  requires !input.empty?         # Pure method
  requires input.contains?("@")  # Pure method
  # ... parsing logic ...
end

# Example: Collection invariants
@contracts(:full)
fn merge_sorted(a: Array[Int], b: Array[Int]) -> Array[Int]
  requires a.sorted?
  requires b.sorted?
  ensures result.sorted?
  ensures result.length == a.length + b.length
  # ... merge implementation ...
end

# Example: Object state validation
@contracts(:full)
fn process_order(order: Order)
  requires order.items.length > 0
  requires order.total == order.items.sum |i| i.price end
  requires order.customer.verified?
  # ... order processing ...
end
```

### 6.3 Complete Tier 3 Examples

```aria
# Example: Universal quantifier
@contracts(:full)
fn is_sorted[T: Comparable](arr: Array[T]) -> Bool
  ensures forall i, j: Int |
    0 <= i < j < arr.length implies arr[i] <= arr[j]
  end
  # ... implementation ...
end

# Example: Existential quantifier
@contracts(:full)
fn find_duplicate[T: Eq](arr: Array[T]) -> Option[(Int, Int)]
  ensures |result|
    result.some? implies
      let (i, j) = result.unwrap
      i != j && arr[i] == arr[j]
    end
  end
  # ... implementation ...
end

# Example: Higher-order contract
@contracts(:full)
fn sort_by[T](arr: Array[T], cmp: (T, T) -> Int) -> Array[T]
  requires forall a, b, c: T |
    (cmp(a, b) < 0 && cmp(b, c) < 0) implies cmp(a, c) < 0
  end  # Transitivity
  ensures forall i: Int |
    0 <= i < result.length - 1 implies cmp(result[i], result[i+1]) <= 0
  end  # Sorted result
  # ... sort implementation ...
end

# Example: Database invariant
@contracts(:full)
fn create_user(name: String, email: String) -> User
  requires !User.exists?(email: email)  # DB query
  ensures User.find_by_email(email).some?
  # ... user creation ...
end

# Example: Property-based contract
@contracts(:full)
fn reverse[T](arr: Array[T]) -> Array[T]
  ensures result.length == arr.length
  ensures forall i: Int |
    0 <= i < arr.length implies result[i] == arr[arr.length - 1 - i]
  end
  ensures reverse(result) == arr  # Involution property

  property "double reverse is identity"
    forall xs: Array[Int]
      reverse(reverse(xs)) == xs
    end
  end

  # ... implementation ...
end
```

---

## 7. Error Messages and Diagnostics

### 7.1 Tier 1 Verification Failure

```
error[E0401]: Contract verification failed
  --> src/math.aria:15:3
   |
15 |   requires b != 0
   |            ^^^^^^ Cannot prove `b != 0`
   |
   = Counterexample found:
       b = 0 (from call at src/main.aria:42)

   = Call site:
       42 | result = safe_divide(x, user_input)
                                    ^^^^^^^^^^
       `user_input` may be 0

   = Suggestion: Add validation before call
       if user_input != 0 {
         result = safe_divide(x, user_input)
       }
```

### 7.2 Tier 2 Caching Information

```
info[I0402]: Contract verification cached
  --> src/search.aria:8:3
   |
 8 |   requires arr.sorted?
   |            ^^^^^^^^^^^ Verified via cached analysis
   |
   = Note: `arr` traced from `data.sort()` at line 5
   = Cache hit: sorted property propagated through dataflow
```

### 7.3 Tier 3 Runtime Check Insertion

```
info[I0403]: Runtime contract check inserted
  --> src/query.aria:22:3
   |
22 |   requires forall item: Item | item.price > 0 end
   |            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Tier 3: Runtime check
   |
   = Note: Quantified contract cannot be verified statically
   = Runtime overhead: O(n) where n = items.length
   = Suggestion: Consider @contracts(:static) if performance critical
```

### 7.4 Mode Mismatch Warning

```
warning[W0404]: Contract skipped in current mode
  --> src/core.aria:10:3
   |
10 |   requires complex_invariant(data)
   |            ^^^^^^^^^^^^^^^^^^^^^^^ Tier 3 contract, mode is :static
   |
   = Note: Function annotated with @contracts(:static)
   = This contract will NOT be verified
   = Change to @contracts(:full) for runtime verification
```

---

## 8. Integration with Other Systems

### 8.1 Effect System Integration

```aria
# Contracts can specify effect requirements
fn pure_transform(data: Data) -> Data
  @pure  # No side effects
  requires data.valid?
  ensures result.valid?
  ensures result.size == data.size
  # Effect system verifies purity, enabling Tier 2 analysis
end

# IO effects prevent static verification
fn load_config(path: String) -> {IO} Config
  requires File.exists?(path)  # IO effect -> Tier 3
  ensures result.valid?
  # ...
end
```

### 8.2 Property-Based Testing Integration

```aria
# Tier 3 contracts automatically generate property tests
fn sort[T: Ord](arr: Array[T]) -> Array[T]
  ensures result.sorted?
  ensures result.permutation_of?(arr)

  # Compiler generates:
  @property(max_examples: 100)
  fn test_sort_contract()
    forall arr: Array[Int]
      result = sort(arr)
      assert result.sorted?
      assert result.permutation_of?(arr)
    end
  end
end
```

### 8.3 IDE Integration

```
# IDE shows contract tier inline
fn binary_search(arr, target)
  requires arr.sorted?         # [Tier 2: Cached] tooltip
  requires arr.length > 0      # [Tier 2: Static with cache]
  requires target > Int.MIN    # [Tier 1: Static]
  ensures ...                  # [Tier 3: Runtime]
end

# Hover over contract shows:
# - Current tier classification
# - Why this tier was assigned
# - Suggestions for tier promotion
```

---

## 9. Performance Considerations

### 9.1 Compile-Time Overhead

| Tier | Compile Overhead | Strategy |
|------|------------------|----------|
| Tier 1 | Moderate | SMT solving (cached) |
| Tier 2 | Low-Moderate | Abstract interpretation |
| Tier 3 | Minimal | Just codegen |

**Mitigation**:
- Incremental verification (only changed functions)
- SMT result caching across builds
- Parallel verification of independent functions

### 9.2 Runtime Overhead

| Mode | Tier 1 | Tier 2 | Tier 3 |
|------|--------|--------|--------|
| :static | 0 | 0 | 0 |
| :runtime | O(1) | O(expr) | O(expr) |
| :off | 0 | 0 | 0 |
| :full | 0 | 0* | O(expr) |

*Tier 2 may have minimal runtime cost for cache lookups

### 9.3 Memory Overhead

```aria
# Contract checking memory usage
@contracts(:full, max_contract_memory: 10_mb)
module LargeDataProcessing
  # Contracts with large data structures are bounded
end
```

---

## 10. Future Extensions

### 10.1 LLM-Assisted Tier Promotion

```aria
# Future: LLM suggests contract strengthening for tier promotion
fn complex_function(data: Data)
  requires data.valid?  # Tier 3

  # LLM suggestion:
  # "Consider adding: requires data.size < 1000
  #  This would enable Tier 2 verification via bounded analysis"
end
```

### 10.2 Gradual Verification

```aria
# Start with Tier 3, progressively verify
@contracts(:gradual)
fn evolving_function(x: Int)
  requires x > 0                    # Verified: Tier 1
  requires complex_property(x)      # Pending: Tier 3
  # IDE shows verification progress
end
```

### 10.3 Contract Inference

```aria
# Compiler infers contracts from implementation
fn inferred_contracts(arr: Array[Int], i: Int) -> Int
  # Inferred: requires i >= 0 && i < arr.length
  arr[i]
end

# IDE shows:
# "Inferred contract: requires i >= 0 && i < arr.length"
# "Accept? [Yes] [No] [Modify]"
```

---

## 11. Key Decisions

### 11.1 Design Decisions Made

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Default mode | `:full` | Balance safety and performance |
| Tier promotion | Automatic | Reduce annotation burden |
| SMT solver | Z3 (primary) | Industry standard, well-tested |
| Timeout handling | Graceful degradation | Don't break builds on timeout |
| Quantifier handling | Tier 3 always | Undecidable in general |

### 11.2 Open Design Questions

1. Should Tier 2 caching persist across compilation sessions?
2. How to handle contracts in generic/polymorphic code?
3. What's the interaction between contracts and async/await?
4. Should there be a "Tier 1.5" for bounded quantifiers?

---

## 12. References

### Academic Sources

1. [Dafny: An Automatic Program Verifier](https://www.microsoft.com/en-us/research/publication/dafny-automatic-program-verifier-functional-correctness/) - Leino, K.R.M.
2. [Satisfiability Modulo Theories](https://people.eecs.berkeley.edu/~sseshia/pubdir/SMT-BookChapter.pdf) - Barrett, C., Sebastiani, R., et al.
3. [SPARK 2014 Rationale](https://docs.adacore.com/spark2014-docs/html/lrm/introduction.html) - AdaCore
4. [Towards Proof Stability in SMT-based Program Verification](https://popl25.sigplan.org/details/dafny-2025-papers/5/Towards-Proof-Stability-in-SMT-based-Program-Verification) - Dafny 2025

### Implementation References

1. [Z3 SMT Solver](https://github.com/Z3Prover/z3)
2. [Dafny Documentation](https://dafny.org/latest/)
3. [SPARK User's Guide](https://docs.adacore.com/spark2014-docs/html/ug/en/usage_scenarios.html)
4. [Hypothesis: Property-Based Testing](https://hypothesis.readthedocs.io/)

### Prior Aria Research

1. ARIA-M04-01: Eiffel's Design by Contract Study
2. ARIA-M04-02: Dafny's Verification Analysis
3. ARIA-M04-03: Property-Based Testing Research
4. ARIA-M12-01: QuickCheck Architecture Study

---

## Appendix A: Quick Reference

### Contract Mode Cheat Sheet

```aria
# Static only (maximum performance, minimum safety)
@contracts(:static)

# Runtime only (maximum safety, development mode)
@contracts(:runtime)

# No checking (trusted code, use sparingly)
@contracts(:off)

# Full hybrid (default, recommended)
@contracts(:full)
```

### Tier Classification Cheat Sheet

| Contract Pattern | Tier | Reason |
|------------------|------|--------|
| `x > 0` | 1 | Linear arithmetic |
| `x != nil` | 1 | Null check |
| `arr.length > 0` | 2 | Pure method |
| `arr.sorted?` | 2 | Pure method |
| `arr[i] > 0` | 2 | Array access |
| `forall x \| P(x)` | 3 | Quantifier |
| `f(x) > 0` (closure) | 3 | Opaque |
| `x * y > z` | 3 | Non-linear |

---

## Appendix B: SMT Theory Support

### Supported SMT-LIB Theories

| Theory | SMT-LIB Name | Aria Usage |
|--------|--------------|------------|
| Core | Core | Boolean operations |
| Integers | Ints | Int type |
| Reals | Reals | Float type |
| Arrays | ArraysEx | Array type |
| Strings | Strings | String type |
| Datatypes | Datatypes | Enums, structs |

### Z3 Integration API (Internal)

```aria
# Internal compiler module for SMT integration
module AriaCompiler.SMT
  fn generate_vc(contract: ContractExpr, context: VerificationContext) -> SmtLib
  fn check_sat(formula: SmtLib, timeout: Duration) -> SatResult
  fn get_counterexample(result: SatResult) -> Option[Counterexample]
  fn cache_result(key: CacheKey, result: SatResult) -> Unit
end
```
