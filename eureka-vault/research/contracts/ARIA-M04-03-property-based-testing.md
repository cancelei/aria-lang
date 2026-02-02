# ARIA-M04-03: Property-Based Testing Research

**Task ID**: ARIA-M04-03
**Status**: Completed
**Date**: 2026-01-14
**Focus**: Study QuickCheck-style property testing

---

## Executive Summary

Property-based testing (PBT) automatically generates test inputs to verify properties hold. This research analyzes shrinking strategies, generator design, and integration with contracts.

---

## 1. Property-Based Testing Overview

### 1.1 Core Concept

```
Traditional Testing:
  test("add works") { assert(add(2, 3) == 5) }

Property-Based Testing:
  property("add is commutative") {
    forall(x: Int, y: Int) { add(x, y) == add(y, x) }
  }
  // Framework generates 100s of random x, y values
```

### 1.2 Key Components

| Component | Purpose |
|-----------|---------|
| **Property** | Boolean assertion that should always hold |
| **Generator** | Produces random test inputs |
| **Shrinker** | Minimizes failing input |
| **Runner** | Executes tests, reports failures |

---

## 2. Shrinking Strategies

### 2.1 Why Shrinking Matters

```
Without shrinking:
  Failed: add(839274, -3847291) != add(-3847291, 839274)
  // Hard to debug!

With shrinking:
  Failed: add(1, 0) != add(0, 1)
  // Minimal counterexample!
```

### 2.2 Type-Based Shrinking (QuickCheck Classic)

Shrinking is defined per type:

```haskell
-- Haskell QuickCheck
instance Arbitrary Int where
    shrink n = [n `div` 2, n - 1, 0]  -- Try smaller values

-- Shrinking [1,2,3,4,5]:
-- → [2,3,4,5], [1,3,4,5], ..., [1,2], [1], []
```

**Pros**: Simple, predictable
**Cons**: Must write shrinker for each type

### 2.3 Integrated Shrinking (Hypothesis/Hedgehog)

Shrinking is built into generators:

```python
# Hypothesis (Python)
from hypothesis import given
from hypothesis.strategies import integers

@given(integers())  # Generator includes shrinking
def test_abs(x):
    assert abs(x) >= 0
```

**Key insight**: Shrink the **choice sequence**, not the value.

```
Generation: [random bytes] → value
Shrinking:  [smaller bytes] → smaller value

Example:
  [0x8F, 0x3A, 0x01] → 839274
  [0x00, 0x00, 0x01] → 1  (shrunk)
```

**Pros**: Shrinking preserves invariants automatically
**Cons**: More complex implementation

### 2.4 Comparison

| Approach | Invariant Preservation | Complexity | Control |
|----------|------------------------|------------|---------|
| Type-based | Manual | Low | High |
| Integrated | Automatic | Medium | Medium |
| Hybrid | Configurable | High | High |

---

## 3. Generator Design

### 3.1 Basic Generators

```python
# Hypothesis strategies
integers()              # Any integer
integers(min_value=0)   # Non-negative
text()                  # Unicode strings
lists(integers())       # List of integers
```

### 3.2 Composite Generators

```python
# Building complex generators
@composite
def sorted_lists(draw):
    xs = draw(lists(integers()))
    return sorted(xs)

@composite
def binary_trees(draw):
    if draw(booleans()):
        return Leaf(draw(integers()))
    else:
        return Node(draw(binary_trees()), draw(binary_trees()))
```

### 3.3 Size Control

Generators should respect a "size" parameter:

```python
# Start small, grow larger
# Size 0: simple values
# Size 10: moderate complexity
# Size 100: stress test

lists(integers(), min_size=0, max_size=size)
```

### 3.4 Generator Pitfalls

| Pitfall | Problem | Solution |
|---------|---------|----------|
| Infinite recursion | Tree generator never terminates | Size-bounded recursion |
| Sparse valid inputs | Most generated values invalid | Custom generators |
| Slow generation | Complex invariants | Rejection-free generation |

---

## 4. Property Patterns

### 4.1 Common Properties

```python
# Roundtrip
forall(x) { deserialize(serialize(x)) == x }

# Idempotence
forall(x) { f(f(x)) == f(x) }

# Commutativity
forall(x, y) { f(x, y) == f(y, x) }

# Associativity
forall(x, y, z) { f(f(x, y), z) == f(x, f(y, z)) }

# Identity
forall(x) { f(x, identity) == x }

# Invariant preservation
forall(x) { valid(x) implies valid(transform(x)) }
```

### 4.2 Model-Based Testing

Compare implementation against simple model:

```python
@given(operations=lists(queue_operations()))
def test_queue(operations):
    real_queue = MyQueue()
    model_queue = []  # Simple list as model

    for op in operations:
        if op.is_push:
            real_queue.push(op.value)
            model_queue.append(op.value)
        else:
            assert real_queue.pop() == model_queue.pop(0)
```

---

## 5. Integration with Contracts

### 5.1 Contracts as Properties

```aria
fn binary_search(arr, key) -> Int?
  requires arr.sorted?
  ensures |result|
    result.none? or arr[result.unwrap] == key
end

# Auto-generated property test:
property "binary_search contract" do
  forall arr: sorted_array(), key: Int do
    result = binary_search(arr, key)
    assert result.none? or arr[result.unwrap] == key
  end
end
```

### 5.2 Generator Inference from Contracts

```aria
# From requires clause:
requires arr.sorted?
# Infer: need sorted_array() generator

requires x > 0
# Infer: need integers(min: 1) generator
```

### 5.3 Shrinking with Contracts

```aria
# Shrinking must preserve preconditions!
requires arr.sorted?

# Naive shrink [1,2,3,4,5] → [3,1,5]  # INVALID: not sorted
# Smart shrink [1,2,3,4,5] → [1,2,3]  # Valid: still sorted
```

---

## 6. Existing Frameworks

### 6.1 QuickCheck (Haskell)

```haskell
prop_reverse :: [Int] -> Bool
prop_reverse xs = reverse (reverse xs) == xs

main = quickCheck prop_reverse
```

- **Strengths**: Original, well-understood
- **Weaknesses**: Type-based shrinking can be tedious

### 6.2 Hypothesis (Python)

```python
from hypothesis import given, strategies as st

@given(st.lists(st.integers()))
def test_reverse(xs):
    assert list(reversed(list(reversed(xs)))) == xs
```

- **Strengths**: Integrated shrinking, stateful testing
- **Weaknesses**: Python-specific

### 6.3 PropEr (Erlang)

```erlang
prop_reverse() ->
    ?FORALL(L, list(integer()),
        lists:reverse(lists:reverse(L)) == L).
```

- **Strengths**: Great for concurrent systems
- **Weaknesses**: Erlang ecosystem only

### 6.4 fast-check (JavaScript)

```javascript
fc.assert(
  fc.property(fc.array(fc.integer()), (arr) => {
    return arr.reverse().reverse().toString() === arr.toString();
  })
);
```

---

## 7. Recommendations for Aria

### 7.1 Built-in Property Testing

```aria
# Property block in function definition
fn reverse[T](arr: Array[T]) -> Array[T]
  ensures |result| result.length == arr.length

  property "double reverse is identity"
    forall arr: Array[Int]
      reverse(reverse(arr)) == arr
    end
  end

  # Implementation
  arr.fold_right([], |x, acc| [x] + acc)
end
```

### 7.2 Automatic Generator Inference

```aria
fn binary_search(arr, key)
  requires arr.sorted?
  requires arr.length > 0
end

# Aria infers generator:
# sorted_non_empty_array() from requires clauses
```

### 7.3 Integrated Shrinking

Use Hypothesis-style integrated shrinking:
- Shrinking preserves `requires` clauses automatically
- No manual shrinker writing needed

### 7.4 Syntax Design

```aria
# Standalone property test
test property "list operations"
  forall xs: Array[Int], x: Int do
    # push then pop returns same element
    xs.push(x).pop == x
  end
end

# Inline with function
fn sort(arr)
  ensures |result| result.sorted?
  ensures |result| result.permutation_of?(arr)

  property "idempotent"
    forall arr: Array[Int]
      sort(sort(arr)) == sort(arr)
    end
  end
end
```

### 7.5 Integration Points

| Integration | Approach |
|-------------|----------|
| Contracts → Properties | Auto-generate from ensures |
| Contracts → Generators | Infer from requires |
| IDE | Show property coverage |
| CI | Run on every commit |

---

## 8. Key Resources

1. [QuickCheck Paper](https://dl.acm.org/doi/10.1145/357766.351266) - Claessen & Hughes
2. [Hypothesis Documentation](https://hypothesis.readthedocs.io/)
3. [Integrated vs Type-Based Shrinking](https://hypothesis.works/articles/integrated-shrinking/)
4. [Property-Based Testing in Practice](https://dl.acm.org/doi/10.1145/3510003.3510043)

---

## 9. Open Questions

1. How do we handle generators for custom types?
2. Should property tests run on every build or separately?
3. How do we ensure sufficient coverage?
4. What's the UX for debugging shrunk counterexamples?
