# ARIA-M12-01: QuickCheck Architecture Study

**Task ID**: ARIA-M12-01
**Status**: Completed
**Date**: 2026-01-14
**Focus**: Analyze QuickCheck's property-based testing architecture

---

## Executive Summary

QuickCheck pioneered property-based testing with automatic test case generation and shrinking. This research analyzes its architecture—generators, shrinking, and the Arbitrary typeclass—for Aria's testing framework design.

---

## 1. Overview

### 1.1 What is QuickCheck?

QuickCheck (Haskell, 2000) introduced:
- **Property-based testing**: Specify properties, not examples
- **Automatic generation**: Random test cases generated
- **Shrinking**: Minimal failing examples found
- **Type-directed**: Generators derived from types

### 1.2 Ecosystem

| Language | Library | Notes |
|----------|---------|-------|
| Haskell | QuickCheck | Original |
| Scala | ScalaCheck | Mature, widely used |
| Python | Hypothesis | Most comprehensive |
| Rust | proptest, quickcheck | Both popular |
| F# | FsCheck | .NET ecosystem |
| Erlang | PropEr | Concurrent focus |

---

## 2. Core Architecture

### 2.1 The Gen Monad

```haskell
-- Generator abstraction
newtype Gen a = Gen { unGen :: StdGen -> Int -> a }

instance Monad Gen where
  return x = Gen (\_ _ -> x)
  Gen g >>= f = Gen (\r n ->
    let (r1, r2) = split r
        x = g r1 n
    in unGen (f x) r2 n)
```

Key insight: Generators are functions from (random seed, size) → value.

### 2.2 Size Parameter

```haskell
-- Size controls complexity
sized :: (Int -> Gen a) -> Gen a
sized f = Gen (\r n -> unGen (f n) r n)

-- Resize locally
resize :: Int -> Gen a -> Gen a
resize n (Gen g) = Gen (\r _ -> g r n)
```

Benefits:
- Starts with small values
- Grows complexity gradually
- Avoids generating huge structures early

### 2.3 Arbitrary Typeclass

```haskell
class Arbitrary a where
  arbitrary :: Gen a
  shrink :: a -> [a]  -- Returns smaller values

-- Basic instances
instance Arbitrary Int where
  arbitrary = choose (minBound, maxBound)
  shrink x = [x `div` 2 | x /= 0] ++ [0 | x /= 0]

instance Arbitrary Bool where
  arbitrary = elements [True, False]
  shrink True = [False]
  shrink False = []

-- Composite instances (derived)
instance Arbitrary a => Arbitrary [a] where
  arbitrary = listOf arbitrary
  shrink xs = map ($ xs) [init, tail] ++
              [replaceAt i y xs | (i, x) <- zip [0..] xs, y <- shrink x]
```

---

## 3. Shrinking

### 3.1 Why Shrinking Matters

```
Without shrinking:
  Failed: [847293, -29384, 0, 29384, -847293]

With shrinking:
  Failed: [1, 0, -1]  (minimal counterexample)
```

### 3.2 Shrinking Algorithm

```
shrink(value):
  candidates = shrink_function(value)
  for candidate in candidates:
    if test_fails(candidate):
      return shrink(candidate)  # Recursively shrink
  return value  # Found minimal
```

### 3.3 Shrinking Strategies

| Strategy | Description |
|----------|-------------|
| Binary search | Divide and conquer for numbers |
| Structural | Remove elements from containers |
| Recursive | Shrink nested components |
| Zero-biased | Try 0/empty first |

---

## 4. Hypothesis (Python) Innovations

### 4.1 Conjecture Engine

Hypothesis uses a different approach:
- Byte buffer as source of randomness
- All generation reads from buffer
- Shrinking = minimizing buffer

```python
# Conceptually
class ConjectureData:
    def __init__(self, buffer: bytes):
        self.buffer = buffer
        self.index = 0

    def draw_bits(self, n: int) -> int:
        # Read n bits from buffer
        result = int.from_bytes(self.buffer[self.index:self.index+n], 'big')
        self.index += n
        return result
```

### 4.2 Benefits

- Shrinking is generic (minimize buffer)
- Deterministic replay (same buffer = same test)
- Stateful testing support
- Database of failing examples

### 4.3 Strategies vs Generators

```python
from hypothesis import given, strategies as st

@given(st.lists(st.integers()))
def test_sort_idempotent(xs):
    assert sorted(sorted(xs)) == sorted(xs)

# Composable strategies
user_strategy = st.builds(
    User,
    name=st.text(min_size=1),
    age=st.integers(min_value=0, max_value=150),
    email=st.emails()
)
```

---

## 5. Proptest (Rust)

### 5.1 Strategy Trait

```rust
pub trait Strategy: Sized {
    type Value: Debug;
    fn new_tree(&self, runner: &mut TestRunner) -> ValueTree<Self::Value>;
}

pub trait ValueTree {
    type Value;
    fn current(&self) -> Self::Value;
    fn simplify(&mut self) -> bool;  // Try to shrink
    fn complicate(&mut self) -> bool;  // Undo shrink
}
```

### 5.2 Shrinking via ValueTree

```rust
// Shrinking is built into the tree structure
fn find_minimal_failure<V: ValueTree>(mut tree: V, test: impl Fn(&V::Value) -> bool) -> V::Value {
    while tree.simplify() {
        if !test(&tree.current()) {
            // Still failing, keep this simplification
        } else {
            // Went too far, complicate back
            tree.complicate();
        }
    }
    tree.current()
}
```

---

## 6. Comparison Matrix

| Feature | QuickCheck | Hypothesis | Proptest |
|---------|------------|------------|----------|
| Shrinking | Explicit function | Buffer minimization | ValueTree |
| Type direction | Arbitrary class | Strategies | Strategy trait |
| Composability | Monad | Flatmap | Proptest combinators |
| State testing | Limited | Excellent | Good |
| Determinism | Seed-based | Database | Runner state |
| Performance | Fast | Moderate | Fast |

---

## 7. Integration with Contract Systems

### 7.1 Property Extraction

```haskell
-- Property from contract
-- Original contract:
--   requires x > 0
--   ensures result >= x

-- Generated property:
prop_sqrt :: Positive Int -> Bool
prop_sqrt (Positive x) =
  let result = sqrt x
  in result >= x  -- From ensures
```

### 7.2 Precondition as Filter

```haskell
-- Discard invalid inputs
prop_divide :: Int -> Int -> Property
prop_divide x y =
  y /= 0 ==>  -- Precondition filter
  (x `div` y) * y + (x `mod` y) == x
```

---

## 8. Recommendations for Aria

### 8.1 Generator Design

```aria
# Aria generator type
Generator[T] {
  fn generate(seed: Int, size: Int) -> T
  fn shrink(value: T) -> Array[T]
}

# Generable trait (like Arbitrary)
trait Generable[T]
  fn arbitrary() -> Generator[T]
  fn shrink(value: T) -> Array[T] = []
end

# Built-in instances
impl Generable[Int]
  fn arbitrary() -> Generator[Int]
    Generator.int_range(Int.MIN, Int.MAX)
  end

  fn shrink(x: Int) -> Array[Int]
    if x == 0 then [] else [x / 2, 0] end
  end
end
```

### 8.2 Property Syntax

```aria
# Property definition
@property
fn sort_preserves_length(xs: Array[Int]) -> Bool
  xs.sort.length == xs.length
end

# With explicit generator
@property(gen: list_of(positive_int(), max_size: 100))
fn sum_positive(xs: Array[Int]) -> Bool
  xs.sum >= 0
end

# Integration with contracts
fn binary_search(arr: Array[Int], target: Int) -> Option[Int]
  requires arr.is_sorted
  ensures |result|
    match result
      Some(i) => arr[i] == target
      None => !arr.contains(target)
    end
  end
  # ... implementation
end

# Auto-generated property test:
@property(derived_from: binary_search)
fn test_binary_search_contract(arr: Array[Int], target: Int) -> Bool
  # Automatically tests the contract
end
```

### 8.3 Hypothesis-Style Buffer

```aria
# Consider Hypothesis approach for better shrinking
ConjectureData {
  buffer: ByteArray
  index: Int

  fn draw_int(min: Int, max: Int) -> Int
    # Read from buffer, decode to range
  end
}

# All generators use same buffer
# Shrinking = minimize buffer bytes
```

### 8.4 Contract Integration

```aria
# Contracts become properties automatically
fn factorial(n: Int) -> Int
  requires n >= 0
  ensures |result| result >= 1
  ensures |result| n > 0 implies result >= n
  # ...
end

# Compiler generates:
@property(max_examples: 100)
fn test_factorial_contract()
  n = Gen.int(0, 20)  # Reasonable range

  # Test precondition filtering
  if n >= 0
    result = factorial(n)
    assert result >= 1
    assert n > 0 implies result >= n
  end
end
```

---

## 9. Key Resources

1. [QuickCheck Paper (2000)](https://www.cs.tufts.edu/~nr/cs257/archive/john-hughes/quick.pdf)
2. [Hypothesis Documentation](https://hypothesis.readthedocs.io/)
3. [Proptest Book](https://proptest-rs.github.io/proptest/intro.html)
4. [Choosing Properties for Property-Based Testing](https://fsharpforfunandprofit.com/posts/property-based-testing-2/)
5. [How Hypothesis Works](https://hypothesis.works/articles/how-hypothesis-works/)

---

## 10. Open Questions

1. Should Aria use Hypothesis-style buffer or QuickCheck-style explicit shrink?
2. How do we handle generators for effect-laden types?
3. What's the relationship between contracts and generated properties?
4. Should shrinking respect invariants (e.g., sorted arrays stay sorted)?
