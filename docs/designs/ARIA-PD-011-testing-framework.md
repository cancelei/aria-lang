# ARIA-PD-011: Testing Framework Design

**Decision ID**: ARIA-PD-011
**Status**: Approved
**Date**: 2026-01-15
**Author**: ARBITER (Product Decision Agent)
**Research Inputs**:
- ARIA-M12-02: Testing Framework Design Research (ANCHOR)
- ARIA-M12-01: QuickCheck Architecture Study (ANCHOR)
- ARIA-M04-03: Property-Based Testing Research (ANCHOR)
- ARIA-M04-04: Tiered Contract System Design (SENTINEL)

---

## Executive Summary

This document defines Aria's comprehensive testing framework, integrating four testing paradigms into a unified, cohesive system: unit testing with `test` blocks, property-based testing via the `property` construct, contract-to-test extraction leveraging Aria's Design by Contract system, and fuzz testing for security and robustness validation.

**Final Decision**: Aria will implement a **Contract-First Testing Framework** with:
- **Syntax**: Ruby-inspired `test` blocks with fluent assertion DSL
- **Property Testing**: QuickCheck-style `property` blocks with `forall` quantifiers and automatic shrinking
- **Contract Integration**: Automatic test generation from `requires`/`ensures` clauses based on tiered verification
- **Fuzzing**: libFuzzer-compatible `@fuzz_target` annotations with grammar-based input generation
- **Documentation Tests**: Rust-style executable examples in doc comments

---

## 1. Test Block Syntax Design

### 1.1 Design Philosophy

Aria's test syntax follows the principle: "Tests should read like specifications."

```
ARIA TESTING ARCHITECTURE

Goal: Unified testing with contract awareness
      Simple syntax, powerful capabilities

Layer 1: Unit Tests (test blocks)
  - Explicit assertions with clear failure messages
  - Setup/teardown lifecycle
  - Parameterized testing support

Layer 2: Property-Based Tests (property blocks)
  - Random input generation via Generator[T]
  - Automatic shrinking to minimal counterexamples
  - Coverage tracking and classification

Layer 3: Contract-Derived Tests (automatic)
  - Generated from requires/ensures clauses
  - Tiered: static verification or runtime testing
  - Integration test generation from contract chains

Layer 4: Fuzz Tests (@fuzz_target)
  - Coverage-guided mutation
  - Grammar-based structured fuzzing
  - Contract-aware input generation
```

### 1.2 Test Block Syntax

```ebnf
test_decl       = [ test_annotations ]
                  'test' string_lit [ test_options ]
                  newline
                  [ setup_block ]
                  { test_statement }
                  [ teardown_block ]
                  'end' ;

test_annotations = '@' annotation_name [ '(' annotation_args ')' ] ;

test_options    = 'with' '(' option_list ')' ;
option_list     = option { ',' option } ;
option          = identifier ':' expression ;

setup_block     = 'setup' newline { statement } 'end' ;
teardown_block  = 'teardown' newline { statement } 'end' ;

test_statement  = assertion_stmt
                | statement ;
```

### 1.3 Core Test Syntax Examples

```aria
# Basic test block
test "user creation with valid data"
  user = User.new("Alice", "alice@example.com")

  assert user.name == "Alice"
  assert user.email == "alice@example.com"
  assert user.active?
end

# Test with options
test "database transaction rollback" with(timeout: 5.seconds, tags: [:database, :slow])
  setup
    @conn = Database.connect(test_url)
    @conn.begin_transaction
  end

  user = User.create!(name: "Bob")
  assert User.find(user.id).some?

  @conn.rollback
  assert User.find(user.id).none?

  teardown
    @conn.close
  end
end

# Parameterized test
@parametrize([
  {input: 0, expected: 1},
  {input: 1, expected: 1},
  {input: 5, expected: 120},
  {input: 10, expected: 3628800}
])
test "factorial computes correctly" with(param: p)
  assert factorial(p.input) == p.expected
end

# Test expecting specific exception
test "division by zero raises error"
  assert_raises DivisionByZero do
    divide(10, 0)
  end
end

# Async test
@async
test "concurrent fetch completes" with(timeout: 10.seconds)
  results = await Task.gather([
    fetch_user(1),
    fetch_user(2),
    fetch_user(3)
  ])

  assert results.length == 3
  assert results.all?(&:ok?)
end
```

### 1.4 Assertion DSL

Aria provides a fluent assertion API:

```aria
module Aria.Testing

  # Core assertions
  fn assert(condition: Bool, message: String? = nil)
    if !condition
      raise AssertionError(message ?? "Assertion failed")
    end
  end

  fn assert_eq[T: Eq + Debug](actual: T, expected: T, message: String? = nil)
    if actual != expected
      raise AssertionError(
        message ?? "Expected #{expected.debug}, got #{actual.debug}"
      )
    end
  end

  fn assert_ne[T: Eq + Debug](actual: T, expected: T, message: String? = nil)
    if actual == expected
      raise AssertionError(
        message ?? "Expected value different from #{expected.debug}"
      )
    end
  end

  fn assert_approx(actual: Float, expected: Float, epsilon: Float = 1e-10)
    if (actual - expected).abs > epsilon
      raise AssertionError(
        "Expected #{expected} +/- #{epsilon}, got #{actual}"
      )
    end
  end

  # Collection assertions
  fn assert_contains[T: Eq](collection: Array[T], element: T)
    if !collection.contains?(element)
      raise AssertionError("Collection does not contain #{element.debug}")
    end
  end

  fn assert_empty[T](collection: Array[T])
    if !collection.empty?
      raise AssertionError("Expected empty collection, got #{collection.length} elements")
    end
  end

  # Exception assertions
  fn assert_raises[E: Exception](block: Fn() -> Any) -> E
    try
      block()
      raise AssertionError("Expected #{E.type_name} but no exception raised")
    catch e: E
      e  # Return caught exception for further inspection
    catch e: Exception
      raise AssertionError("Expected #{E.type_name}, got #{e.type_name}")
    end
  end

  fn assert_no_raise(block: Fn() -> Any)
    try
      block()
    catch e: Exception
      raise AssertionError("Expected no exception, got #{e.type_name}: #{e.message}")
    end
  end

  # Type assertions
  fn assert_type[T](value: Any) -> T
    match value
      v: T => v
      _ => raise AssertionError("Expected type #{T.type_name}, got #{value.type_name}")
    end
  end

  # Comparison assertions
  fn assert_lt[T: Ord](actual: T, expected: T)
    if actual >= expected
      raise AssertionError("Expected #{actual.debug} < #{expected.debug}")
    end
  end

  fn assert_le[T: Ord](actual: T, expected: T)
    if actual > expected
      raise AssertionError("Expected #{actual.debug} <= #{expected.debug}")
    end
  end

  fn assert_gt[T: Ord](actual: T, expected: T)
    if actual <= expected
      raise AssertionError("Expected #{actual.debug} > #{expected.debug}")
    end
  end

  fn assert_ge[T: Ord](actual: T, expected: T)
    if actual < expected
      raise AssertionError("Expected #{actual.debug} >= #{expected.debug}")
    end
  end

  # Pattern assertions
  fn assert_matches(value: String, pattern: Regex)
    if !pattern.matches?(value)
      raise AssertionError("Expected '#{value}' to match #{pattern}")
    end
  end

  # Eventually/retry assertions (for async/concurrent tests)
  fn assert_eventually(
    condition: Fn() -> Bool,
    timeout: Duration = 5.seconds,
    poll_interval: Duration = 100.milliseconds
  )
    deadline = Time.now + timeout
    while Time.now < deadline
      if condition()
        return
      end
      sleep(poll_interval)
    end
    raise AssertionError("Condition not satisfied within #{timeout}")
  end
end
```

### 1.5 Assertion Syntax Sugar

```aria
# Standard assertions
assert user.active?
assert_eq result, 42
assert_ne error, nil

# Fluent assertion syntax (alternative)
expect(result).to_eq(42)
expect(user).to_be_active
expect(list).to_contain(item)
expect(block).to_raise(TypeError)

# Should-style (Ruby influence)
result.should_eq(42)
user.should_be_active
list.should_contain(item)

# Compiler desugars all forms to assert_* calls
```

---

## 2. Property-Based Testing API

### 2.1 The Generator Type

```aria
# Core generator monad
struct Generator[T]
  private fn: Fn(Seed, Size) -> T

  # Create a generator from a function
  fn new(f: Fn(Seed, Size) -> T) -> Generator[T]
    Generator(fn: f)
  end

  # Generate a value
  fn generate(self, seed: Seed, size: Size) -> T
    self.fn(seed, size)
  end

  # Monadic map
  fn map[U](self, f: Fn(T) -> U) -> Generator[U]
    Generator.new |seed, size|
      f(self.generate(seed, size))
    end
  end

  # Monadic flat_map (bind)
  fn flat_map[U](self, f: Fn(T) -> Generator[U]) -> Generator[U]
    Generator.new |seed, size|
      let (seed1, seed2) = seed.split
      let intermediate = self.generate(seed1, size)
      f(intermediate).generate(seed2, size)
    end
  end

  # Filter with predicate (use sparingly - can be slow)
  fn filter(self, predicate: Fn(T) -> Bool, max_attempts: Int = 100) -> Generator[T]
    Generator.new |seed, size|
      mut current_seed = seed
      for _ in 0..max_attempts
        let candidate = self.generate(current_seed, size)
        if predicate(candidate)
          return candidate
        end
        current_seed = current_seed.next
      end
      panic!("Generator.filter: Could not satisfy predicate after #{max_attempts} attempts")
    end
  end

  # Resize the generator
  fn resize(self, new_size: Size) -> Generator[T]
    Generator.new |seed, _|
      self.generate(seed, new_size)
    end
  end

  # Scale the size parameter
  fn scale(self, f: Fn(Size) -> Size) -> Generator[T]
    Generator.new |seed, size|
      self.generate(seed, f(size))
    end
  end
end
```

### 2.2 The Generable Trait

```aria
# Type-directed generation (like Haskell's Arbitrary)
trait Generable[T]
  fn arbitrary() -> Generator[T]
  fn shrink(value: T) -> Array[T] = []
end

# Built-in instances
impl Generable[Int]
  fn arbitrary() -> Generator[Int]
    Generator.new |seed, size|
      seed.next_int(-size, size)
    end
  end

  fn shrink(x: Int) -> Array[Int]
    if x == 0
      []
    else
      candidates = [0]
      if x > 0
        candidates.push(x / 2)
        candidates.push(x - 1)
      else
        candidates.push(x / 2)
        candidates.push(x + 1)
      end
      candidates.filter |c| c.abs < x.abs end
    end
  end
end

impl Generable[Bool]
  fn arbitrary() -> Generator[Bool]
    Gen.elements([true, false])
  end

  fn shrink(x: Bool) -> Array[Bool]
    if x then [false] else [] end
  end
end

impl Generable[Float]
  fn arbitrary() -> Generator[Float]
    Generator.new |seed, size|
      seed.next_float(-size.to_float, size.to_float)
    end
  end

  fn shrink(x: Float) -> Array[Float]
    if x == 0.0
      []
    else
      [0.0, x / 2.0, x.truncate.to_float]
        .filter |c| c.abs < x.abs end
    end
  end
end

impl Generable[String]
  fn arbitrary() -> Generator[String]
    Gen.string(Gen.ascii_char, 0..100)
  end

  fn shrink(s: String) -> Array[String]
    if s.empty?
      []
    else
      # Remove characters
      removals = (0..s.length).map |i|
        s[0..i] + s[(i+1)..]
      end
      # Simplify characters
      simplifications = s.chars.enumerate.flat_map |(i, c)|
        if c > 'a'
          [s[0..i] + "a" + s[(i+1)..]]
        else
          []
        end
      end
      removals ++ simplifications
    end
  end
end

impl[T: Generable] Generable[Array[T]]
  fn arbitrary() -> Generator[Array[T]]
    Gen.list(T.arbitrary)
  end

  fn shrink(arr: Array[T]) -> Array[Array[T]]
    if arr.empty?
      []
    else
      # Shrink by removing elements
      removals = (0..arr.length).map |i|
        arr[0..i] ++ arr[(i+1)..]
      end
      # Shrink individual elements
      element_shrinks = arr.enumerate.flat_map |(i, elem)|
        T.shrink(elem).map |smaller|
          arr[0..i] ++ [smaller] ++ arr[(i+1)..]
        end
      end
      removals ++ element_shrinks
    end
  end
end

impl[T: Generable] Generable[Option[T]]
  fn arbitrary() -> Generator[Option[T]]
    Gen.one_of([
      Gen.constant(None),
      T.arbitrary.map |v| Some(v) end
    ])
  end

  fn shrink(opt: Option[T]) -> Array[Option[T]]
    match opt
      None => []
      Some(v) => [None] ++ T.shrink(v).map |s| Some(s) end
    end
  end
end
```

### 2.3 Generator Combinators Module

```aria
module Gen
  # === Basic Combinators ===

  fn constant[T](value: T) -> Generator[T]
    Generator.new |_, _| value end
  end

  fn elements[T](choices: Array[T]) -> Generator[T]
    requires choices.length > 0
    Generator.new |seed, _|
      idx = seed.next_int(0, choices.length - 1)
      choices[idx]
    end
  end

  fn one_of[T](generators: Array[Generator[T]]) -> Generator[T]
    requires generators.length > 0
    Generator.new |seed, size|
      let (seed1, seed2) = seed.split
      idx = seed1.next_int(0, generators.length - 1)
      generators[idx].generate(seed2, size)
    end
  end

  fn frequency[T](weighted: Array[(Int, Generator[T])]) -> Generator[T]
    requires weighted.length > 0
    requires weighted.all? |(w, _)| w > 0 end

    total = weighted.sum |(w, _)| w end
    Generator.new |seed, size|
      let (seed1, seed2) = seed.split
      mut target = seed1.next_int(0, total - 1)
      for (weight, gen) in weighted
        if target < weight
          return gen.generate(seed2, size)
        end
        target -= weight
      end
      weighted.last.1.generate(seed2, size)  # Fallback
    end
  end

  # === Size-Aware Combinators ===

  fn sized[T](f: Fn(Size) -> Generator[T]) -> Generator[T]
    Generator.new |seed, size|
      f(size).generate(seed, size)
    end
  end

  fn resize[T](new_size: Size, gen: Generator[T]) -> Generator[T]
    gen.resize(new_size)
  end

  fn scale[T](f: Fn(Size) -> Size, gen: Generator[T]) -> Generator[T]
    gen.scale(f)
  end

  # === Numeric Generators ===

  fn int(min: Int, max: Int) -> Generator[Int]
    requires min <= max
    Generator.new |seed, _|
      seed.next_int(min, max)
    end
  end

  fn positive_int() -> Generator[Int]
    sized |size| int(1, max(1, size)) end
  end

  fn non_negative_int() -> Generator[Int]
    sized |size| int(0, size) end
  end

  fn negative_int() -> Generator[Int]
    sized |size| int(-size, -1) end
  end

  fn float(min: Float, max: Float) -> Generator[Float]
    requires min <= max
    Generator.new |seed, _|
      seed.next_float(min, max)
    end
  end

  # === String Generators ===

  fn char(alphabet: String) -> Generator[Char]
    requires alphabet.length > 0
    Generator.new |seed, _|
      idx = seed.next_int(0, alphabet.length - 1)
      alphabet[idx]
    end
  end

  fn ascii_char() -> Generator[Char]
    int(32, 126).map |i| Char.from_code(i) end
  end

  fn alpha_char() -> Generator[Char]
    elements("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ".chars)
  end

  fn alphanumeric_char() -> Generator[Char]
    elements("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".chars)
  end

  fn string(char_gen: Generator[Char], length_range: Range[Int]) -> Generator[String]
    Generator.new |seed, size|
      let (seed1, seed2) = seed.split
      len = seed1.next_int(length_range.start, min(length_range.end, size))
      mut chars = []
      mut current_seed = seed2
      for _ in 0..len
        let (s1, s2) = current_seed.split
        chars.push(char_gen.generate(s1, size))
        current_seed = s2
      end
      String.from_chars(chars)
    end
  end

  fn ascii_string() -> Generator[String]
    string(ascii_char(), 0..100)
  end

  fn alphanumeric_string() -> Generator[String]
    string(alphanumeric_char(), 0..100)
  end

  # === Collection Generators ===

  fn list[T](elem: Generator[T]) -> Generator[Array[T]]
    sized |size|
      list_of_length(elem, 0..size)
    end
  end

  fn list_of_length[T](elem: Generator[T], length_range: Range[Int]) -> Generator[Array[T]]
    Generator.new |seed, size|
      let (seed1, seed2) = seed.split
      len = seed1.next_int(length_range.start, length_range.end)
      mut result = []
      mut current_seed = seed2
      for _ in 0..len
        let (s1, s2) = current_seed.split
        result.push(elem.generate(s1, size))
        current_seed = s2
      end
      result
    end
  end

  fn non_empty_list[T](elem: Generator[T]) -> Generator[Array[T]]
    sized |size|
      list_of_length(elem, 1..max(1, size))
    end
  end

  fn vector[T](n: Int, elem: Generator[T]) -> Generator[Array[T]]
    list_of_length(elem, n..n)
  end

  fn map_of[K, V](keys: Generator[K], values: Generator[V]) -> Generator[Map[K, V]]
  where K: Hash + Eq
    sized |size|
      Generator.new |seed, sz|
        let (seed1, seed2) = seed.split
        len = seed1.next_int(0, sz)
        mut result = Map.new()
        mut current_seed = seed2
        for _ in 0..len
          let (s1, s2, s3) = current_seed.split3
          key = keys.generate(s1, sz)
          value = values.generate(s2, sz)
          result[key] = value
          current_seed = s3
        end
        result
      end
    end
  end

  # === Specialized Generators ===

  fn sorted_array[T: Ord](elem: Generator[T]) -> Generator[Array[T]]
    list(elem).map |arr| arr.sort end
  end

  fn unique_array[T: Eq + Hash](elem: Generator[T]) -> Generator[Array[T]]
    list(elem).map |arr| arr.unique end
  end

  fn permutation[T](arr: Array[T]) -> Generator[Array[T]]
    Generator.new |seed, _|
      mut result = arr.clone
      mut current_seed = seed
      for i in (result.length - 1)..0 by -1
        let (s1, s2) = current_seed.split
        j = s1.next_int(0, i)
        result.swap(i, j)
        current_seed = s2
      end
      result
    end
  end

  fn subset[T](arr: Array[T]) -> Generator[Array[T]]
    Generator.new |seed, _|
      mut result = []
      mut current_seed = seed
      for item in arr
        let (s1, s2) = current_seed.split
        if s1.next_bool
          result.push(item)
        end
        current_seed = s2
      end
      result
    end
  end
end
```

### 2.4 Property Definition Syntax

```ebnf
property_decl   = [ property_annotations ]
                  'property' string_lit [ property_options ]
                  newline
                  quantifier_clause
                  property_body
                  'end' ;

property_annotations = '@' annotation_name [ '(' annotation_args ')' ] ;

property_options = 'with' '(' option_list ')' ;

quantifier_clause = 'forall' '(' typed_param_list ')' [ 'where' expression ] ;

typed_param_list = typed_param { ',' typed_param } ;
typed_param      = identifier ':' type_or_generator ;
type_or_generator = type_expr | generator_expr ;

property_body   = expression
                | 'do' newline { statement } 'end' ;
```

### 2.5 Property Syntax Examples

```aria
# Basic property
property "addition is commutative"
  forall(a: Int, b: Int)
    a + b == b + a
  end
end

# With explicit generator
property "sorted array remains sorted after insert"
  forall(arr: Gen.sorted_array(Gen.int(-100, 100)), x: Int)
    arr.insert_sorted(x).sorted?
  end
end

# Conditional property with assuming
property "division specification"
  forall(a: Int, b: Int) where b != 0
    (a / b) * b + (a % b) == a
  end
end

# Property with classification
property "list operations are balanced"
  forall(xs: Array[Int])
  do
    classify(xs.empty?, "empty")
    classify(xs.length == 1, "singleton")
    classify(xs.length > 10, "large")

    xs.reverse.reverse == xs
  end
end

# Property with coverage requirements
property "covers edge cases" with(
  iterations: 1000,
  coverage_requirements: [
    ("empty", 5.percent),
    ("singleton", 10.percent),
    ("large", 20.percent)
  ]
)
  forall(xs: Array[Int])
  do
    classify(xs.empty?, "empty")
    classify(xs.length == 1, "singleton")
    classify(xs.length > 10, "large")

    xs.sort.sorted?
  end
end

# Property with custom shrinking limit
@shrink_limit(1000)
@shrink_depth(50)
property "finds minimal counterexample"
  forall(xs: Array[Int])
    xs.sum >= 0  # Will find minimal negative example
  end
end

# Stateful property testing (state machine)
state_machine "Stack operations"
  type State = Stack[Int]

  initial_state() = Stack.empty

  rule "push" with(x: Int)
    when true  # Always applicable
    do |state|
      state.push(x)
    end
    then |old_state, new_state|
      assert new_state.top == x
      assert new_state.length == old_state.length + 1
    end
  end

  rule "pop"
    when |state| !state.empty?
    do |state|
      state.pop
    end
    then |old_state, (new_state, popped)|
      assert popped == old_state.top
      assert new_state.length == old_state.length - 1
    end
  end

  invariant |state|
    state.length >= 0
  end
end
```

### 2.6 Property Test Runner

```aria
module Aria.Testing.Property
  struct PropertyConfig
    iterations: Int = 100
    seed: Seed? = nil
    max_shrinks: Int = 1000
    coverage_requirements: Array[(String, Float)] = []
  end

  struct PropertyResult
    status: PropertyStatus
    iterations_run: Int
    counterexample: Any? = nil
    shrunk_counterexample: Any? = nil
    seed: Seed
    coverage: Map[String, Float]
  end

  enum PropertyStatus
    Passed
    Failed(reason: String)
    GaveUp(reason: String)
    InsufficientCoverage(missing: Array[(String, Float)])
  end

  fn run_property[T](
    name: String,
    gen: Generator[T],
    predicate: Fn(T) -> Bool,
    config: PropertyConfig = PropertyConfig()
  ) -> PropertyResult
    seed = config.seed ?? Seed.random()
    mut coverage_counts = Map.new()
    mut current_seed = seed

    for i in 0..config.iterations
      let (s1, s2) = current_seed.split
      size = (i * 100) / config.iterations  # Grow size with iterations

      input = gen.generate(s1, size)

      try
        if !predicate(input)
          # Found counterexample - try to shrink
          shrunk = shrink_counterexample(input, predicate, config.max_shrinks)
          return PropertyResult(
            status: PropertyStatus.Failed("Property falsified"),
            iterations_run: i + 1,
            counterexample: input,
            shrunk_counterexample: shrunk,
            seed: seed,
            coverage: coverage_counts
          )
        end
      catch e: Exception
        return PropertyResult(
          status: PropertyStatus.Failed("Exception: #{e.message}"),
          iterations_run: i + 1,
          counterexample: input,
          shrunk_counterexample: nil,
          seed: seed,
          coverage: coverage_counts
        )
      end

      current_seed = s2
    end

    # Check coverage requirements
    for (label, required) in config.coverage_requirements
      actual = coverage_counts[label] ?? 0.0
      if actual < required
        return PropertyResult(
          status: PropertyStatus.InsufficientCoverage([(label, required - actual)]),
          iterations_run: config.iterations,
          counterexample: nil,
          shrunk_counterexample: nil,
          seed: seed,
          coverage: coverage_counts
        )
      end
    end

    PropertyResult(
      status: PropertyStatus.Passed,
      iterations_run: config.iterations,
      counterexample: nil,
      shrunk_counterexample: nil,
      seed: seed,
      coverage: coverage_counts
    )
  end

  fn shrink_counterexample[T: Generable](
    input: T,
    predicate: Fn(T) -> Bool,
    max_shrinks: Int
  ) -> T
    mut current = input
    mut shrinks_performed = 0
    mut improved = true

    while improved && shrinks_performed < max_shrinks
      improved = false
      for candidate in T.shrink(current)
        shrinks_performed += 1
        if shrinks_performed >= max_shrinks
          break
        end

        try
          if !predicate(candidate)
            current = candidate
            improved = true
            break
          end
        catch _
          # Candidate also fails - use it
          current = candidate
          improved = true
          break
        end
      end
    end

    current
  end
end
```

---

## 3. Contract-to-Test Extraction Design

### 3.1 Design Philosophy

Aria treats contracts as implicit test oracles. The testing framework automatically generates tests from `requires` and `ensures` clauses, with the generation strategy determined by the contract's tier classification (from ARIA-M04-04).

```
CONTRACT-TO-TEST EXTRACTION PIPELINE

+----------------+     +------------------+     +------------------+
| Parse Contract | --> | Classify Tier    | --> | Generate Tests   |
+----------------+     +------------------+     +------------------+
                              |
        +---------------------+---------------------+
        |                     |                     |
        v                     v                     v
   +----------+          +----------+          +----------+
   | Tier 1   |          | Tier 2   |          | Tier 3   |
   | (Static) |          | (Cached) |          | (Dynamic)|
   +----------+          +----------+          +----------+
        |                     |                     |
        v                     v                     v
   No test needed      Boundary tests         Property-based
   (compiler verifies) (cached analysis)      test generation
```

### 3.2 Contract Extraction Implementation

```aria
module Aria.Testing.ContractExtraction

  struct ContractSpec
    function_name: String
    preconditions: Array[Constraint]
    postconditions: Array[Constraint]
    invariants: Array[Constraint]
    parameters: Array[Parameter]
    return_type: Type
  end

  struct Constraint
    expression: Expr
    tier: ContractTier
    message: String?
  end

  enum ContractTier
    Static    # Tier 1: SMT-solvable
    Cached    # Tier 2: Pure computation, cacheable
    Dynamic   # Tier 3: Runtime-only
  end

  # Extract test generators from contracts
  fn extract_test_generator(spec: ContractSpec) -> Generator[TestCase]
    # Build generators from preconditions
    param_generators = spec.parameters.map |param|
      generator_from_constraints(
        param.type,
        spec.preconditions.filter |c| references?(c.expression, param.name) end
      )
    end

    # Combine parameter generators
    Gen.sequence(param_generators).map |args|
      TestCase(
        function: spec.function_name,
        arguments: args,
        expected_postconditions: spec.postconditions
      )
    end
  end

  # Convert constraint to generator bounds
  fn generator_from_constraints(
    base_type: Type,
    constraints: Array[Constraint]
  ) -> Generator[Any]
    match base_type
      Type.Int =>
        let bounds = extract_int_bounds(constraints)
        Gen.int(bounds.min ?? Int.MIN, bounds.max ?? Int.MAX)

      Type.Array(elem_type) =>
        let length_bounds = extract_length_bounds(constraints)
        let elem_constraints = extract_element_constraints(constraints)

        if has_sorted_constraint?(constraints)
          Gen.sorted_array(generator_from_constraints(elem_type, elem_constraints))
        else
          Gen.list_of_length(
            generator_from_constraints(elem_type, elem_constraints),
            length_bounds.min..length_bounds.max
          )
        end

      Type.String =>
        let length_bounds = extract_length_bounds(constraints)
        Gen.string(Gen.ascii_char(), length_bounds.min..length_bounds.max)

      _ =>
        # Default: use type's Generable instance
        base_type.arbitrary
    end
  end

  # Constraint pattern matching
  fn extract_int_bounds(constraints: Array[Constraint]) -> IntBounds
    mut bounds = IntBounds(min: nil, max: nil)

    for c in constraints
      match c.expression
        BinaryOp(Var(name), ">", Literal(n: Int)) =>
          bounds.min = max(bounds.min ?? Int.MIN, n + 1)
        BinaryOp(Var(name), ">=", Literal(n: Int)) =>
          bounds.min = max(bounds.min ?? Int.MIN, n)
        BinaryOp(Var(name), "<", Literal(n: Int)) =>
          bounds.max = min(bounds.max ?? Int.MAX, n - 1)
        BinaryOp(Var(name), "<=", Literal(n: Int)) =>
          bounds.max = min(bounds.max ?? Int.MAX, n)
        BinaryOp(Var(name), "!=", Literal(n: Int)) =>
          # Handle by filtering later
          pass
        _ => pass
      end
    end

    bounds
  end

  fn has_sorted_constraint?(constraints: Array[Constraint]) -> Bool
    constraints.any? |c|
      match c.expression
        MethodCall(_, "sorted?", []) => true
        _ => false
      end
    end
  end
end
```

### 3.3 Tiered Test Generation

```aria
# Tier 1: No generated test needed - compiler verifies statically
fn abs(x: Int) -> Int
  ensures result >= 0
  if x < 0 then -x else x end
end
# No test generated - postcondition verified by compiler

# Tier 2: Boundary tests generated, cached analysis
fn binary_search(arr: Array[Int], target: Int) -> Int?
  requires arr.sorted?
  requires arr.length > 0
  ensures |result|
    result.none? || arr[result.unwrap] == target
  end
  # ... implementation
end

# Generated test (Tier 2):
@auto_generated(from: :binary_search, tier: 2)
test "binary_search boundary tests"
  # Empty boundary (just above requires)
  assert_raises ContractViolation do
    binary_search([], 0)
  end

  # Single element (minimum valid)
  assert binary_search([5], 5) == Some(0)
  assert binary_search([5], 3) == None

  # Sorted boundary
  assert binary_search([1, 2, 3, 4, 5], 1) == Some(0)
  assert binary_search([1, 2, 3, 4, 5], 5) == Some(4)
end

# Tier 3: Full property-based test generation
fn complex_sort(arr: Array[T]) -> Array[T]
where T: Ord
  requires arr.length > 0
  ensures result.permutation_of?(arr)
  ensures result.sorted?
  # ...
end

# Generated test (Tier 3):
@auto_generated(from: :complex_sort, tier: 3)
property "complex_sort satisfies contract"
  forall(arr: Gen.non_empty_list(Gen.int(-1000, 1000)))
  do
    result = complex_sort(arr)

    # Postcondition verification
    assert result.permutation_of?(arr), "Result must be permutation of input"
    assert result.sorted?, "Result must be sorted"
  end
end
```

### 3.4 Contract Chain Testing

When function A calls function B, both contracts form a chain that can be tested together:

```aria
fn process_data(raw: String) -> ProcessedData
  requires raw.length > 0
  ensures result.valid?

  parsed = parse(raw)       # parse has contracts
  validated = validate(parsed)  # validate has contracts
  transform(validated)      # transform has contracts
end

# Generated integration test from contract chain
@auto_generated(from: :process_data, chain: true)
property "process_data contract chain"
  forall(raw: Gen.non_empty_string(1..1000))
  do
    # All contracts in chain are implicitly verified:
    # - process_data.requires (raw.length > 0)
    # - parse.requires, parse.ensures
    # - validate.requires, validate.ensures
    # - transform.requires, transform.ensures
    # - process_data.ensures (result.valid?)

    result = process_data(raw)
    assert result.valid?
  end
end
```

### 3.5 Test Generation Annotations

```aria
# Opt-out of automatic test generation
@no_contract_test
fn internal_helper(x: Int) -> Int
  requires x > 0
  x * 2
end

# Customize generated test parameters
@contract_test(iterations: 500, seed: 12345)
fn important_function(data: Array[Int]) -> Int
  requires data.length > 0
  ensures result >= 0
  data.sum.abs
end

# Generate tests for specific tiers only
@contract_test(tiers: [:tier2, :tier3])
fn selective_testing(x: Float) -> Float
  requires x >= 0.0
  ensures result >= x
  x.sqrt.pow(2)
end
```

---

## 4. Fuzzing Integration

### 4.1 Fuzz Target API

```aria
# Basic fuzz target (libFuzzer-compatible)
@fuzz_target
fn fuzz_parser(data: Bytes) -> FuzzResult
  try
    let input = data.to_string_lossy
    let parsed = Parser.parse(input)
    # If we reach here, parsing succeeded
    FuzzResult.ok
  catch ParseError
    # Expected error, not a crash
    FuzzResult.ok
  catch e: Exception
    # Unexpected error - report as interesting
    FuzzResult.crash(e)
  end
end

# Structured fuzz target (grammar-based)
@fuzz_target(structured: ArithmeticExpr)
fn fuzz_evaluator(expr: ArithmeticExpr) -> FuzzResult
  try
    let result = evaluate(expr)
    assert result.finite?, "Result should be finite"
    FuzzResult.ok
  catch DivisionByZero
    FuzzResult.ok  # Expected
  catch e: Exception
    FuzzResult.crash(e)
  end
end

# Contract-aware fuzz target
@fuzz_target(contract_guided: true)
fn fuzz_binary_search(data: Bytes) -> FuzzResult
  # Deserialize respecting contract constraints
  let arr = data.deserialize_as(SortedArray[Int])  # Respects sorted? constraint
  let target = data.deserialize_as(Int)

  try
    let result = binary_search(arr.to_array, target)

    # Postconditions verified automatically
    verify_postconditions(:binary_search, arr, target, result)
    FuzzResult.ok
  catch ContractViolation(e)
    FuzzResult.crash(e)  # Contract violation is a bug
  catch e
    FuzzResult.crash(e)
  end
end
```

### 4.2 Fuzz Result Type

```aria
enum FuzzResult
  Ok                              # Test passed
  Interesting(reason: String)     # Keep in corpus but not a crash
  Crash(error: Exception)         # Found a bug
  Timeout                         # Execution timed out
  OutOfMemory                     # Memory limit exceeded

  fn ok() -> FuzzResult = FuzzResult.Ok

  fn crash(e: Exception) -> FuzzResult = FuzzResult.Crash(error: e)

  fn interesting(reason: String) -> FuzzResult =
    FuzzResult.Interesting(reason: reason)
end
```

### 4.3 Grammar-Based Fuzzing

```aria
# Grammar definition for structured fuzzing
grammar AriaExpr
  start: expr

  expr: literal
      | binary_op
      | unary_op
      | call
      | if_expr
      | paren_expr

  literal: INT_LIT | FLOAT_LIT | STRING_LIT | BOOL_LIT

  binary_op: expr OP expr
  OP: "+" | "-" | "*" | "/" | "==" | "!=" | "<" | ">" | "<=" | ">="

  unary_op: "-" expr | "!" expr

  call: IDENT "(" arg_list? ")"
  arg_list: expr ("," expr)*

  if_expr: "if" expr "then" expr "else" expr "end"

  paren_expr: "(" expr ")"

  INT_LIT: /[0-9]+/
  FLOAT_LIT: /[0-9]+\.[0-9]+/
  STRING_LIT: /"[^"]*"/
  BOOL_LIT: "true" | "false"
  IDENT: /[a-zA-Z_][a-zA-Z0-9_]*/
end

# Fuzz with grammar
@fuzz_target(grammar: AriaExpr, iterations: 10000)
fn fuzz_aria_compiler(program_text: String) -> FuzzResult
  try
    let ast = Compiler.parse(program_text)
    let compiled = Compiler.compile(ast)

    # Differential testing: compare interpreted vs compiled
    let interpreted = Interpreter.eval(ast)
    let executed = compiled.run()

    if interpreted != executed
      FuzzResult.crash(
        DifferentialTestFailure("Interpreted: #{interpreted}, Compiled: #{executed}")
      )
    else
      FuzzResult.ok
    end
  catch CompilerError
    FuzzResult.ok  # Expected for some inputs
  catch e
    FuzzResult.crash(e)
  end
end
```

### 4.4 Custom Mutators

```aria
# Custom mutator for structured fuzzing
@custom_mutator(for: ArithmeticExpr)
fn mutate_expr(expr: ArithmeticExpr, seed: Seed) -> ArithmeticExpr
  match seed.choose(5)
    0 => expr.replace_random_operand(seed.next_int(-1000, 1000))
    1 => expr.swap_operators
    2 => expr.nest_in_operation(seed.choose_op)
    3 => expr.simplify_random_subtree
    4 => expr.duplicate_subtree
  end
end

# Mutation strategies enum
enum MutationStrategy
  ChangeByte        # Flip single byte
  ChangeBit         # Flip single bit
  InsertByte        # Insert random byte
  EraseBytes        # Remove bytes
  CrossOver         # Combine two corpus entries
  CopyPart          # Duplicate section
  Dictionary        # Insert dictionary words
  Custom            # Use custom mutator
end
```

### 4.5 Fuzzer Configuration

```aria
# Fuzzer configuration block
fuzz_config "parser_fuzzing"
  corpus_dir: "corpus/parser"
  crash_dir: "crashes/parser"
  artifact_dir: "artifacts/parser"

  max_len: 10000          # Maximum input size in bytes
  timeout: 5.seconds      # Per-test timeout
  memory_limit: 512.mb    # Memory limit

  # Sanitizers
  address_sanitizer: true
  undefined_behavior_sanitizer: true

  # Mutation settings
  mutation_depth: 5
  dictionary: ["fn", "end", "if", "then", "else", "match", "struct"]

  # Coverage targets
  coverage_target: 80.percent
  edge_coverage: true

  # Parallelism
  workers: 4

  # Reporting
  report_interval: 10.seconds
  save_coverage_map: true
end

# Run fuzzer with config
fn run_fuzzer(target: FuzzTarget, config_name: String)
  config = load_fuzz_config(config_name)
  fuzzer = Fuzzer.new(target, config)
  fuzzer.run()
end
```

### 4.6 Coverage-Guided Fuzzing Implementation

```aria
module Aria.Fuzzing

  struct Fuzzer
    target: FuzzTarget
    config: FuzzConfig
    corpus: Corpus
    coverage_map: CoverageMap

    fn run(self)
      # Initialize corpus
      self.corpus.load_initial(self.config.corpus_dir)

      # Main fuzzing loop
      for iteration in 0..
        # Select input from corpus
        input = self.corpus.select_for_mutation

        # Mutate
        mutated = self.mutate(input)

        # Execute
        let (result, coverage) = self.execute(mutated)

        match result
          FuzzResult.Ok =>
            # Check if new coverage
            if self.coverage_map.has_new_coverage?(coverage)
              self.corpus.add(mutated)
              self.coverage_map.merge(coverage)
            end

          FuzzResult.Interesting(reason) =>
            self.corpus.add(mutated, interesting: true)
            self.report_interesting(mutated, reason)

          FuzzResult.Crash(e) =>
            self.save_crash(mutated, e)

          FuzzResult.Timeout =>
            self.save_timeout(mutated)

          FuzzResult.OutOfMemory =>
            self.save_oom(mutated)
        end

        # Report progress
        if iteration % 1000 == 0
          self.report_progress(iteration)
        end
      end
    end

    fn mutate(self, input: Bytes) -> Bytes
      # Apply random mutation strategy
      strategy = Gen.elements(MutationStrategy.all).generate(Seed.random, 10)

      match strategy
        MutationStrategy.ChangeByte =>
          idx = Gen.int(0, input.length - 1).generate(Seed.random, 10)
          value = Gen.int(0, 255).generate(Seed.random, 10)
          input.replace_at(idx, value.to_byte)

        MutationStrategy.InsertByte =>
          idx = Gen.int(0, input.length).generate(Seed.random, 10)
          value = Gen.int(0, 255).generate(Seed.random, 10)
          input.insert_at(idx, value.to_byte)

        MutationStrategy.EraseBytes =>
          if input.length > 1
            start = Gen.int(0, input.length - 1).generate(Seed.random, 10)
            len = Gen.int(1, min(10, input.length - start)).generate(Seed.random, 10)
            input.remove_range(start, start + len)
          else
            input
          end

        MutationStrategy.CrossOver =>
          other = self.corpus.select_random
          self.crossover(input, other)

        MutationStrategy.Dictionary =>
          word = Gen.elements(self.config.dictionary).generate(Seed.random, 10)
          idx = Gen.int(0, input.length).generate(Seed.random, 10)
          input.insert_bytes_at(idx, word.to_bytes)

        MutationStrategy.Custom =>
          if self.target.custom_mutator?
            self.target.custom_mutator.mutate(input)
          else
            input
          end

        _ => input
      end
    end

    fn execute(self, input: Bytes) -> (FuzzResult, Coverage)
      # Set up timeout
      result = timeout(self.config.timeout) do
        self.target.execute(input)
      end

      coverage = self.coverage_map.capture_current

      (result, coverage)
    end
  end
end
```

---

## 5. Documentation Testing (Doctest)

### 5.1 Doctest Syntax

```aria
## Sorts an array in ascending order.
##
## # Examples
##
## ```aria
## arr = [3, 1, 4, 1, 5]
## result = sort(arr)
## assert result == [1, 1, 3, 4, 5]
## ```
##
## ```aria should_panic
## # This demonstrates error handling
## sort(nil)  # Raises ContractViolation
## ```
##
## ```aria no_run
## # This compiles but doesn't execute in tests
## large_array = Array.fill(1_000_000, random_int())
## sort(large_array)  # Would be too slow for doctest
## ```
##
## ```aria ignore
## # This is skipped entirely (broken example)
## sort("not an array")
## ```
fn sort[T: Ord](arr: Array[T]) -> Array[T]
  # ... implementation
end
```

### 5.2 Doctest Attributes

| Attribute | Behavior |
|-----------|----------|
| (none) | Compile and run, must not panic |
| `should_panic` | Must panic to pass |
| `should_panic(ContractViolation)` | Must panic with specific error type |
| `no_run` | Compile only, don't execute |
| `ignore` | Skip entirely |
| `compile_fail` | Must fail to compile |

### 5.3 Hidden Code Lines

```aria
## Binary search implementation.
##
## # Examples
##
## ```aria
## # Hidden setup code (not shown in docs)
## # fn main() {
## #   let arr = [1, 2, 3, 4, 5]
## let idx = binary_search(arr, 3)
## assert idx == Some(2)
## # }
## ```
##
## Lines starting with `#` followed by space are hidden from docs
## but included in the test.
fn binary_search(arr: Array[Int], target: Int) -> Int?
  # ...
end
```

### 5.4 Doctest Extraction Pipeline

```aria
module Aria.Testing.Doctest

  struct DoctestCase
    file: String
    line: Int
    function_name: String
    code: String
    attribute: DoctestAttribute
  end

  enum DoctestAttribute
    Run                           # Default: run and expect success
    ShouldPanic(error_type: Type?)
    NoRun
    Ignore
    CompileFail
  end

  fn extract_doctests(source: String) -> Array[DoctestCase]
    mut cases = []
    mut current_function = nil

    for (line_num, line) in source.lines.enumerate
      # Track current function
      if line.starts_with?("fn ")
        current_function = parse_function_name(line)
      end

      # Look for doc comment code blocks
      if line.trim.starts_with?("## ```aria")
        attribute = parse_doctest_attribute(line)
        code = extract_code_block(source, line_num)

        if current_function.some?
          cases.push(DoctestCase(
            file: source.path,
            line: line_num,
            function_name: current_function.unwrap,
            code: remove_hidden_lines(code),
            attribute: attribute
          ))
        end
      end
    end

    cases
  end

  fn generate_test_harness(case: DoctestCase) -> String
    match case.attribute
      DoctestAttribute.Run =>
        """
        @doctest(file: "#{case.file}", line: #{case.line})
        test "#{case.function_name} example at line #{case.line}"
          #{case.code}
        end
        """

      DoctestAttribute.ShouldPanic(error_type) =>
        error_match = error_type.map |t| ": #{t.name}" end ?? ""
        """
        @doctest(file: "#{case.file}", line: #{case.line})
        test "#{case.function_name} example at line #{case.line}"
          assert_raises #{error_type?.map(&:name) ?? "Exception"} do
            #{case.code}
          end
        end
        """

      DoctestAttribute.NoRun =>
        """
        @doctest(file: "#{case.file}", line: #{case.line}, no_run: true)
        @compile_only
        test "#{case.function_name} example at line #{case.line} (compile only)"
          #{case.code}
        end
        """

      DoctestAttribute.Ignore =>
        """
        @ignore
        test "#{case.function_name} example at line #{case.line} (ignored)"
          #{case.code}
        end
        """

      DoctestAttribute.CompileFail =>
        """
        @compile_fail
        test "#{case.function_name} example at line #{case.line} (should not compile)"
          #{case.code}
        end
        """
    end
  end

  fn remove_hidden_lines(code: String) -> String
    code.lines
      .filter |line| !line.trim.starts_with?("# ") end
      .join("\n")
  end
end
```

### 5.5 Example-Property Bridge

Doctests can be promoted to property tests:

```aria
## Reverses an array.
##
## # Examples
##
## ```aria
## assert reverse([1, 2, 3]) == [3, 2, 1]
## assert reverse([]) == []
## assert reverse([42]) == [42]
## ```
##
## # Properties
##
## ```aria property
## forall(arr: Array[Int])
##   reverse(reverse(arr)) == arr
## end
## ```
fn reverse[T](arr: Array[T]) -> Array[T]
  ensures result.length == arr.length
  ensures reverse(result) == arr
  # ...
end

# Compiler infers additional properties from contracts:
# - reverse(reverse(x)) == x (from postcondition)
# - reverse(x).length == x.length (from postcondition)
```

---

## 6. Mock/Stub Framework

### 6.1 Effect-Based Mocking

**Decision**: Use Aria's effect system for test isolation. Effect handlers replace production effects with test doubles.

```aria
# Production code with effects
fn process_order(order: Order) -> Receipt !{Database, Payment, Email}
  # Validate inventory
  items = Database.query[InventoryItem](
    "SELECT * FROM inventory WHERE product_id IN ?",
    [order.product_ids]
  )

  if items.any?(|i| i.quantity < order.quantity)
    Exception.raise(OutOfStock)
  end

  # Process payment
  transaction = Payment.charge(order.customer_id, order.total)

  # Send confirmation
  Email.send(order.customer_email, "Order Confirmed", receipt_body(order))

  Receipt(order_id: order.id, transaction_id: transaction.id)
end

# Test with mocked effects
test "process_order handles payment failure"
  order = Order(
    customer_id: 1,
    product_ids: [1, 2],
    quantity: 1,
    total: 100.00,
    customer_email: "test@example.com"
  )

  result = handle process_order(order) with
    Database.query(sql, params) =>
      # Mock: return sufficient inventory
      resume([
        InventoryItem(product_id: 1, quantity: 10),
        InventoryItem(product_id: 2, quantity: 10)
      ])

    Payment.charge(customer, amount) =>
      # Mock: simulate payment failure
      Exception.raise(PaymentDeclined("Insufficient funds"))

    Email.send(to, subject, body) =>
      # Mock: should not be reached
      panic!("Email should not be sent on payment failure")

    Exception.raise(e) => Err(e)
    return(receipt) => Ok(receipt)
  end

  assert result.is_err?
  assert result.unwrap_err() is PaymentDeclined
end
```

### 6.2 Mock Builder API

For reusable and composable test doubles:

```aria
module Aria.Testing.Mocks

  struct MockBuilder[E: Effect]
    expectations: Array[Expectation]
    strict: Bool

    fn on(self, operation: Symbol) -> ExpectationBuilder[E]
      ExpectationBuilder(mock: self, operation: operation)
    end

    fn build(self) -> MockHandler[E]
      MockHandler(expectations: self.expectations, strict: self.strict)
    end
  end

  struct ExpectationBuilder[E]
    mock: MockBuilder[E]
    operation: Symbol
    args_matcher: Fn(Array[Any]) -> Bool = |_| true
    return_value: Any?
    side_effect: Fn(Array[Any]) -> Any?
    call_limit: Int?

    fn with_args(self, matcher: Fn(Array[Any]) -> Bool) -> Self
      Self(..self, args_matcher: matcher)
    end

    fn with_args_eq(self, expected: Array[Any]) -> Self
      self.with_args(|args| args == expected)
    end

    fn returns(self, value: Any) -> MockBuilder[E]
      self.mock.expectations.push(Expectation(
        operation: self.operation,
        args_matcher: self.args_matcher,
        return_value: Some(value),
        side_effect: nil,
        call_limit: self.call_limit
      ))
      self.mock
    end

    fn raises(self, error: Exception) -> MockBuilder[E]
      self.mock.expectations.push(Expectation(
        operation: self.operation,
        args_matcher: self.args_matcher,
        return_value: nil,
        side_effect: Some(|_| Exception.raise(error)),
        call_limit: self.call_limit
      ))
      self.mock
    end

    fn called_times(self, n: Int) -> Self
      Self(..self, call_limit: n)
    end
  end

  fn mock[E: Effect]() -> MockBuilder[E]
    MockBuilder(expectations: [], strict: false)
  end

  fn strict_mock[E: Effect]() -> MockBuilder[E]
    MockBuilder(expectations: [], strict: true)
  end
end

# Usage example
test "database interactions with mock builder"
  mock_db = mock[Database]()
    .on(:query)
      .with_args(|args| args[0].contains?("SELECT"))
      .returns([InventoryItem(product_id: 1, quantity: 10)])
    .on(:execute)
      .with_args(|args| args[0].contains?("INSERT"))
      .returns(1)
      .called_times(1)
    .build

  result = handle process_order(order) with
    Database => mock_db.handler
    # ... other effects
  end

  # Verify mock expectations
  mock_db.verify!
  assert_eq mock_db.call_count(:query), 2
  assert_eq mock_db.call_count(:execute), 1
end
```

### 6.3 Spy and Capture

For recording effect invocations without replacing behavior:

```aria
module Aria.Testing.Spies

  struct Capture[E: Effect]
    calls: Array[EffectCall]

    fn handler(self) -> CaptureHandler[E]
      CaptureHandler(capture: self)
    end

    fn calls_for(self, operation: Symbol) -> Array[EffectCall]
      self.calls.filter(|c| c.operation == operation)
    end

    fn was_called?(self, operation: Symbol) -> Bool
      self.calls_for(operation).length > 0
    end

    fn call_count(self, operation: Symbol) -> Int
      self.calls_for(operation).length
    end
  end

  struct EffectCall
    operation: Symbol
    arguments: Array[Any]
    timestamp: Time
    result: Any?
  end

  fn capture[E: Effect]() -> Capture[E]
    Capture(calls: [])
  end

  # Spy that both records AND executes
  fn spy[E: Effect](real_handler: Handler[E]) -> SpyHandler[E]
    SpyHandler(real: real_handler, calls: [])
  end
end

# Usage example
test "email content verification"
  captured_emails = capture[Email]()

  handle send_welcome_email(user) with
    Email => captured_emails.handler
  end

  assert captured_emails.call_count(:send) == 1
  let call = captured_emails.calls_for(:send)[0]
  assert call.arguments[0] == user.email
  assert call.arguments[1].contains?("Welcome")
  assert call.arguments[2].contains?(user.name)
end
```

### 6.4 Test Fixtures

Reusable test setup with composition:

```aria
module Aria.Testing.Fixtures

  # Fixture registry (built at compile time)
  @fixture
  fn test_user() -> User
    User.new(
      name: "Test User",
      email: "test@example.com",
      role: Role.User
    )
  end

  @fixture
  fn admin_user() -> User
    User.new(
      name: "Admin User",
      email: "admin@example.com",
      role: Role.Admin
    )
  end

  # Composable fixtures with dependencies
  @fixture(depends: [:test_user])
  fn authenticated_session(user: User) -> Session
    Session.create(user, expires_in: 1.hour)
  end

  @fixture(depends: [:admin_user])
  fn admin_session(user: User) -> Session
    Session.create(user, expires_in: 1.hour)
  end

  # Factory-style fixture with parameters
  @fixture
  fn user_with_role(role: Role = Role.User) -> User
    User.new(
      name: "#{role.name} User",
      email: "#{role.name.downcase}@example.com",
      role: role
    )
  end
end

# Usage in tests
test "admin can access admin panel" with(fixtures: [:admin_user, :admin_session])
  |user, session|

  response = handle get("/admin") with
    Auth.current_session() => resume(session)
  end

  assert response.status == 200
end

# Using fixture factory
test "moderator has limited access" with(fixtures: [user_with_role(Role.Moderator)])
  |user|

  session = Session.create(user)
  response = handle get("/admin/settings") with
    Auth.current_session() => resume(session)
  end

  assert response.status == 403
end
```

### 6.5 Mock Decision Table

| Feature | Decision | Rationale |
|---------|----------|-----------|
| Primary mechanism | Effect handlers | Native to Aria, type-safe |
| Mock builder | Fluent API | Readable, composable |
| Verification | Explicit `verify!` | Clear test intent |
| Spies | Built-in `Capture` | Common pattern support |
| Fixtures | Annotation-based | Compile-time discovery |
| Strict mocks | Opt-in | Flexibility for different styles |

---

## 7. Coverage Tracking

### 7.1 Coverage Architecture

**Decision**: Coverage is integrated into the compiler with zero-overhead production builds.

```
+------------------+     +------------------+     +------------------+
| Compiler Pass    | --> | Instrumented IR  | --> | Coverage Runtime |
| (--coverage)     |     | (counters added) |     | (data collection)|
+------------------+     +------------------+     +------------------+
                                                          |
                                                          v
+------------------+     +------------------+     +------------------+
| Report Generator | <-- | Coverage Data    | <-- | Test Execution   |
| (HTML/JSON/LCOV) |     | (hit counts)     |     |                  |
+------------------+     +------------------+     +------------------+
```

### 7.2 Coverage Types

| Type | Description | Metrics |
|------|-------------|---------|
| **Line** | Source lines executed | % lines covered |
| **Branch** | Decision outcomes taken | % branches covered |
| **Function** | Functions called | % functions covered |
| **Contract** | Contract clauses exercised | % requires/ensures tested |
| **Property** | Property input space coverage | Generator coverage % |

### 7.3 Coverage Instrumentation

```aria
# Compiler inserts counters during --coverage build

# Original code
fn calculate(x: Int, y: Int) -> Int
  requires x >= 0
  requires y > 0
  ensures result >= 0

  if x > 10
    x * y
  else
    x + y
  end
end

# Instrumented (conceptual):
fn calculate(x: Int, y: Int) -> Int
  __coverage_hit(file: "math.aria", line: 1, func: "calculate")

  requires x >= 0
  __coverage_contract(file: "math.aria", line: 2, type: :requires, idx: 0)

  requires y > 0
  __coverage_contract(file: "math.aria", line: 3, type: :requires, idx: 1)

  ensures result >= 0
  __coverage_contract(file: "math.aria", line: 4, type: :ensures, idx: 0)

  if x > 10
    __coverage_branch(file: "math.aria", line: 6, branch: :true)
    x * y
  else
    __coverage_branch(file: "math.aria", line: 8, branch: :false)
    x + y
  end
end
```

### 7.4 Coverage Commands

```bash
# Run tests with coverage collection
aria test --coverage

# Generate reports
aria coverage report              # Console summary
aria coverage report --html       # HTML report to coverage/
aria coverage report --json       # JSON export
aria coverage report --lcov       # LCOV format (for CI tools)

# Coverage thresholds (fail CI if below)
aria coverage check --min 80                # Overall minimum
aria coverage check --min-line 85           # Line coverage minimum
aria coverage check --min-branch 75         # Branch coverage minimum
aria coverage check --min-contract 90       # Contract coverage minimum

# Differential coverage (for PRs)
aria coverage diff origin/main    # Show coverage delta

# Coverage by file/module
aria coverage report --by-file
aria coverage report --by-module
aria coverage report --uncovered  # Show only uncovered code
```

### 7.5 Coverage Report Format

```
Coverage Report
===============

Overall Coverage: 87.3%
  Line:     89.2% (1,234 / 1,383 lines)
  Branch:   76.5% (432 / 565 branches)
  Function: 95.1% (97 / 102 functions)
  Contract: 82.4% (75 / 91 clauses)

By File:
--------------------------------------------------------------------------------
File                          Lines    Branches  Functions  Contracts
--------------------------------------------------------------------------------
src/user.aria                 95.2%    88.0%     100.0%     90.0%      [OK]
src/order.aria                87.5%    75.0%      90.0%     80.0%      [OK]
src/payment.aria              72.3%    65.0%      80.0%     70.0%      [WARN]
--------------------------------------------------------------------------------

Uncovered Code:
  src/payment.aria:45-52      refund_order() - function not tested
  src/payment.aria:78         else branch in validate_card()
  src/order.aria:123          requires price > 0 - not exercised

Contract Coverage Details:
  src/user.aria
    create_user: 4/4 clauses covered
    delete_user: 2/3 clauses covered
      - ensures audit_log.contains?(event) NOT COVERED
```

### 7.6 Contract Coverage (Unique to Aria)

```aria
fn withdraw(account: Account, amount: Float) -> Result[Float, Error]
  requires amount > 0              # Clause 1 - covered?
  requires account.balance >= amount  # Clause 2 - covered?
  ensures account.balance == old(account.balance) - amount  # Clause 3 - covered?
  ensures result.ok? implies result.unwrap == amount  # Clause 4 - covered?

  # ...
end

# Contract coverage tracking:
# - Each requires/ensures is tracked independently
# - Shows which contract clauses have been exercised by tests
# - Helps identify untested edge cases in specifications
```

### 7.7 Coverage Integration with CI

```yaml
# Example CI configuration
test:
  script:
    - aria test --coverage --json > coverage.json
    - aria coverage check --min 80 --min-branch 70

  artifacts:
    paths:
      - coverage/
    reports:
      coverage_report:
        coverage_format: cobertura
        path: coverage.xml
```

---

## 8. Benchmark Framework

### 8.1 Benchmark Syntax

**Decision**: `@benchmark` annotation with statistical analysis built-in.

```aria
# Simple benchmark
@benchmark
fn bench_sort_1000()
  data = Array.fill(1000, || Random.int(0, 10000))
  sort(data)
end

# Parameterized benchmark
@benchmark(
  name: "sort performance by size",
  params: [100, 1000, 10000, 100000],
  warmup: 5,
  iterations: 100
)
fn bench_sort_sizes(size: Int)
  data = Array.fill(size, || Random.int(0, 10000))
  sort(data)
end

# Comparison benchmark (baseline)
@benchmark(baseline: :stdlib_sort)
fn bench_custom_sort()
  data = generate_test_data()
  custom_sort(data)
end

@benchmark
fn bench_stdlib_sort()
  data = generate_test_data()
  data.sort
end

# Memory and allocation benchmark
@benchmark(measure: [:time, :memory, :allocations])
fn bench_memory_usage()
  data = create_large_structure()
  process(data)
end

# Async benchmark
@benchmark(async: true)
fn bench_concurrent_fetch()
  urls = ["http://a.com", "http://b.com", "http://c.com"]
  with Async.scope |scope|
    urls.each |url| scope.spawn fetch(url) end
  end
end
```

### 8.2 Benchmark Runner

```bash
# CLI for benchmarks
aria bench [OPTIONS] [PATHS...]

Options:
  --filter PATTERN    Run benchmarks matching pattern
  --warmup N          Warmup iterations (default: 3)
  --iterations N      Measurement iterations (default: 100)
  --time DURATION     Run for duration instead of fixed iterations

  --compare FILE      Compare against saved baseline
  --save FILE         Save results for future comparison
  --threshold PCT     Regression threshold percentage

  --json              JSON output
  --csv               CSV output
  --plot              Generate performance charts

  --measure METRICS   What to measure: time,memory,allocs,cycles
  --parallel N        Run N benchmarks in parallel

Examples:
  aria bench                          # Run all benchmarks
  aria bench --filter "sort*"         # Pattern match
  aria bench --compare baseline.json  # Compare to baseline
  aria bench --save current.json      # Save for comparison
```

### 8.3 Benchmark Implementation

```aria
module Aria.Benchmarking

  struct BenchmarkConfig
    name: String
    warmup_iterations: Int = 3
    measurement_iterations: Int = 100
    min_time: Duration? = nil
    measures: Array[Measure] = [Measure.Time]
    params: Array[Any]? = nil
    baseline: Symbol? = nil
    async: Bool = false
  end

  enum Measure
    Time          # Wall clock time
    Memory        # Peak memory usage
    Allocations   # Number of allocations
    CpuCycles     # CPU cycles (platform-specific)
    Instructions  # Instructions executed
  end

  struct BenchmarkResult
    name: String
    iterations: Int
    param: Any?

    # Time statistics
    mean: Duration
    median: Duration
    std_dev: Duration
    min: Duration
    max: Duration
    percentiles: Map[Int, Duration]  # p50, p90, p95, p99

    # Memory statistics (if measured)
    peak_memory: Bytes?
    allocations: Int?

    # Comparison (if baseline)
    baseline_comparison: Comparison?
  end

  struct Comparison
    baseline_name: String
    ratio: Float           # 1.0 = same, >1 = slower, <1 = faster
    difference: Duration
    significant: Bool      # Statistical significance
    p_value: Float
  end

  fn run_benchmark(config: BenchmarkConfig, block: Fn()) -> BenchmarkResult
    # Warmup phase
    for _ in 0..config.warmup_iterations
      block()
    end

    # Collection phase
    mut times = []
    mut memories = []
    mut allocs = []

    for _ in 0..config.measurement_iterations
      gc_collect()  # Consistent state

      start_memory = if Measure.Memory in config.measures then memory_usage() else nil end
      start_allocs = if Measure.Allocations in config.measures then allocation_count() else nil end

      start = Time.precise_now
      block()
      elapsed = Time.precise_now - start

      times.push(elapsed)

      if start_memory.some?
        memories.push(memory_usage() - start_memory.unwrap)
      end
      if start_allocs.some?
        allocs.push(allocation_count() - start_allocs.unwrap)
      end
    end

    # Statistical analysis
    BenchmarkResult(
      name: config.name,
      iterations: config.measurement_iterations,
      param: nil,
      mean: times.mean,
      median: times.median,
      std_dev: times.std_dev,
      min: times.min,
      max: times.max,
      percentiles: compute_percentiles(times, [50, 90, 95, 99]),
      peak_memory: memories.max,
      allocations: allocs.sum,
      baseline_comparison: nil
    )
  end

  fn compare_benchmarks(current: BenchmarkResult, baseline: BenchmarkResult) -> Comparison
    ratio = current.mean / baseline.mean
    difference = current.mean - baseline.mean

    # Statistical significance test (Welch's t-test)
    (t_stat, p_value) = welch_t_test(current.times, baseline.times)
    significant = p_value < 0.05

    Comparison(
      baseline_name: baseline.name,
      ratio: ratio,
      difference: difference,
      significant: significant,
      p_value: p_value
    )
  end
end
```

### 8.4 Benchmark Output

```
Benchmark Results
=================

bench_sort_1000
  Time:     125.3 us +/- 5.2 us (100 iterations)
  Memory:   48 KB peak
  Allocs:   23
  p50: 124.1 us | p90: 132.5 us | p95: 138.2 us | p99: 145.7 us

bench_sort_sizes (parameterized)
  size=100:    12.5 us +/- 0.8 us
  size=1000:   125 us +/- 5.2 us
  size=10000:  1.45 ms +/- 0.12 ms
  size=100000: 18.2 ms +/- 1.5 ms

Comparison: bench_custom_sort vs bench_stdlib_sort (baseline)
  custom_sort:  145 us +/- 8.2 us
  stdlib_sort:  125 us +/- 5.2 us
  Difference:   +16.0% slower (p < 0.001, statistically significant)
  Recommendation: Consider optimizing custom_sort

Regression Detection:
  [PASS] bench_sort_1000: -2.3% vs baseline (within threshold)
  [FAIL] bench_parse_json: +12.5% vs baseline (threshold: 5%)
         Regression detected! Investigate commit abc1234
```

### 8.5 Benchmark Decision Table

| Feature | Decision | Rationale |
|---------|----------|-----------|
| Statistical rigor | Required confidence intervals | Scientific validity |
| Warmup | Automatic detection | JIT/cache effects |
| Memory tracking | Optional, explicit | Not always needed |
| Regression detection | Built-in comparison | CI integration |
| Output formats | Console, JSON, CSV | Various use cases |
| Async support | Native | Match Aria's concurrency |

---

## 9. Testing Framework Integration

### 9.1 Unified Test Runner

```bash
# Test runner CLI
aria test [OPTIONS] [PATHS...]

Options:
  --unit              Run unit tests only
  --property          Run property-based tests
  --doctest           Run documentation tests
  --fuzz              Run fuzz tests
  --contract          Run contract-derived tests
  --all               Run all test types (default)

  --coverage          Generate coverage report
  --parallel N        Run N tests in parallel (default: CPU count)
  --timeout T         Per-test timeout (default: 30s)
  --seed S            Random seed for reproducibility
  --iterations N      Property test iterations (default: 100)

  --filter PATTERN    Only run tests matching pattern
  --exclude PATTERN   Skip tests matching pattern
  --tags TAGS         Only run tests with specified tags

  --watch             Watch for changes and re-run
  --fail-fast         Stop on first failure
  --verbose           Verbose output
  --quiet             Minimal output

  --report FORMAT     Output format: text, json, junit, html
  --output FILE       Write report to file
```

### 9.2 Test Discovery Rules

```aria
module Aria.Testing.Discovery

  enum TestType
    Unit          # test "..." blocks
    Property      # property "..." blocks
    Doctest       # ```aria in doc comments
    Fuzz          # @fuzz_target functions
    Contract      # Auto-generated from contracts
    StateMachine  # state_machine "..." blocks
  end

  struct TestCase
    name: String
    type: TestType
    file: String
    line: Int
    tags: Array[String]
    timeout: Duration?
    annotations: Map[String, Any]
  end

  fn discover_tests(paths: Array[String]) -> Array[TestCase]
    mut tests = []

    for path in paths
      for file in glob_files(path, "**/*.aria")
        source = File.read(file)

        # Unit tests: test "..." blocks
        tests.append(discover_unit_tests(file, source))

        # Property tests: property "..." blocks
        tests.append(discover_property_tests(file, source))

        # Doctests: ```aria in doc comments
        tests.append(discover_doctests(file, source))

        # Fuzz targets: @fuzz_target functions
        tests.append(discover_fuzz_targets(file, source))

        # Contract tests: auto-generated from contracts
        tests.append(discover_contract_tests(file, source))

        # State machine tests
        tests.append(discover_state_machine_tests(file, source))
      end
    end

    tests
  end
end
```

### 9.3 Test Execution Engine

```aria
module Aria.Testing.Runner

  struct TestRunner
    config: TestConfig
    reporter: TestReporter

    fn run(self, tests: Array[TestCase]) -> TestResults
      # Group tests by type for optimal execution
      grouped = tests.group_by(&:type)

      mut results = TestResults.new()

      # Run in optimal order
      for test_type in [TestType.Unit, TestType.Property, TestType.Doctest,
                        TestType.Contract, TestType.Fuzz, TestType.StateMachine]
        if self.config.should_run?(test_type)
          type_tests = grouped[test_type] ?? []

          if self.config.parallel > 1
            type_results = self.run_parallel(type_tests)
          else
            type_results = self.run_sequential(type_tests)
          end

          results.merge(type_results)

          if self.config.fail_fast && results.has_failures?
            break
          end
        end
      end

      results
    end

    fn run_single(self, test: TestCase) -> TestResult
      self.reporter.test_started(test)

      start_time = Time.now

      result = timeout(test.timeout ?? self.config.timeout) do
        match test.type
          TestType.Unit => run_unit_test(test)
          TestType.Property => run_property_test(test)
          TestType.Doctest => run_doctest(test)
          TestType.Fuzz => run_fuzz_test(test)
          TestType.Contract => run_contract_test(test)
          TestType.StateMachine => run_state_machine_test(test)
        end
      end

      duration = Time.now - start_time

      self.reporter.test_finished(test, result, duration)

      TestResult(
        test: test,
        status: result.status,
        duration: duration,
        error: result.error,
        output: result.output
      )
    end
  end
end
```

### 9.4 Test Results and Reporting

```aria
struct TestResults
  passed: Int = 0
  failed: Int = 0
  skipped: Int = 0
  errors: Int = 0

  unit_results: Array[TestResult] = []
  property_results: Array[TestResult] = []
  doctest_results: Array[TestResult] = []
  fuzz_results: Array[TestResult] = []
  contract_results: Array[TestResult] = []

  coverage: CoverageReport?
  mutation_score: Float?

  total_duration: Duration

  fn summary(self) -> String
    """
    Test Results Summary
    ====================

    Unit Tests:        #{self.unit_count} passed, #{self.unit_failed} failed
    Property Tests:    #{self.property_count} passed, #{self.property_failed} failed
    Doc Tests:         #{self.doctest_count} passed, #{self.doctest_failed} failed
    Contract Tests:    #{self.contract_count} passed, #{self.contract_failed} failed (auto-generated)
    Fuzz Tests:        #{self.fuzz_count} passed, #{self.fuzz_failed} failed

    Total:             #{self.passed} passed, #{self.failed} failed, #{self.skipped} skipped
    Duration:          #{self.total_duration}

    Coverage:          #{self.coverage?.percentage ?? "N/A"}%
    Mutation Score:    #{self.mutation_score?.map |s| "#{s}%" end ?? "N/A"}
    """
  end
end

# Sample output:
# Test Results Summary
# ====================
#
# Unit Tests:        42 passed,  0 failed,  0 skipped
# Property Tests:    15 passed,  1 failed,  0 skipped
# Doc Tests:         28 passed,  0 failed,  2 skipped
# Contract Tests:    67 passed,  0 failed,  0 skipped (auto-generated)
# Fuzz Tests:         5 passed,  0 found issues (10000 iterations)
#
# Coverage: 87.3% (target: 80%)
# Mutation Score: 82.1% (target: 80%)
#
# Failed Tests:
# -------------
# property "sort preserves elements" (sort.aria:45)
#   Counterexample (shrunk): [0, -1]
#
#   Expected: sorted result contains all original elements
#   Actual:   result = [0, 0] (duplicate element)
#
#   Seed: 12345 (for reproduction)
```

---

## 10. Example Testing Patterns

### 10.1 Unit Test Pattern

```aria
module Tests.UserService

  # Setup shared fixtures
  fn setup_test_user() -> User
    User.new(
      name: "Test User",
      email: "test@example.com",
      role: Role.User
    )
  end

  fn setup_admin_user() -> User
    User.new(
      name: "Admin User",
      email: "admin@example.com",
      role: Role.Admin
    )
  end

  test "user creation with valid data"
    user = setup_test_user()

    assert_eq user.name, "Test User"
    assert_eq user.email, "test@example.com"
    assert user.active?
    assert !user.admin?
  end

  test "user role upgrade requires admin"
    user = setup_test_user()
    admin = setup_admin_user()

    # Non-admin cannot upgrade roles
    assert_raises AuthorizationError do
      user.upgrade_role(user, Role.Admin)
    end

    # Admin can upgrade roles
    assert_no_raise do
      admin.upgrade_role(user, Role.Moderator)
    end

    assert_eq user.role, Role.Moderator
  end

  @async
  test "user fetch with timeout" with(timeout: 5.seconds)
    user_id = 123

    result = await UserService.fetch(user_id)

    assert result.ok?
    assert_eq result.unwrap.id, user_id
  end

  @parametrize([
    {email: "valid@example.com", valid: true},
    {email: "also.valid@test.org", valid: true},
    {email: "invalid", valid: false},
    {email: "no@tld", valid: false},
    {email: "@nodomain.com", valid: false}
  ])
  test "email validation" with(param: p)
    result = validate_email(p.email)
    assert_eq result.valid?, p.valid
  end
end
```

### 10.2 Property Test Pattern

```aria
module Tests.Sorting

  property "sort produces sorted output"
    forall(arr: Array[Int])
      sort(arr).sorted?
    end
  end

  property "sort preserves elements (permutation)"
    forall(arr: Array[Int])
      sort(arr).permutation_of?(arr)
    end
  end

  property "sort is idempotent"
    forall(arr: Array[Int])
      sort(sort(arr)) == sort(arr)
    end
  end

  property "sort with custom comparator"
    forall(arr: Array[Int])
    do
      # Sort descending
      result = sort_by(arr) |a, b| b <=> a end

      # Verify descending order
      result.windows(2).all? |[a, b]| a >= b end
    end
  end

  property "binary search finds element if present" with(iterations: 500)
    forall(arr: Gen.non_empty_list(Gen.int(-1000, 1000)))
    do
      sorted = sort(arr)
      target = Gen.elements(sorted).generate(Seed.random, 10)

      result = binary_search(sorted, target)

      assert result.some?, "Should find #{target} in #{sorted}"
      assert_eq sorted[result.unwrap], target
    end
  end

  property "binary search returns None for absent element"
    forall(arr: Gen.sorted_array(Gen.int(0, 100)), target: Gen.int(101, 200))
      # target is always > max possible element in arr
      binary_search(arr, target) == None
    end
  end
end
```

### 10.3 Contract Test Pattern

```aria
module Banking

  fn transfer(from: Account, to: Account, amount: Money) -> TransferResult
    requires amount > Money.zero                    # Tier 1
    requires from.balance >= amount                  # Tier 2
    requires from.id != to.id                        # Tier 1
    requires from.active? && to.active?              # Tier 2
    ensures from.balance == old(from.balance) - amount  # Tier 3
    ensures to.balance == old(to.balance) + amount      # Tier 3
    ensures result.ok? implies result.unwrap.amount == amount  # Tier 3

    # Implementation
    from.withdraw(amount)
    to.deposit(amount)

    TransferResult.ok(Transfer(
      from: from.id,
      to: to.id,
      amount: amount,
      timestamp: Time.now
    ))
  end
end

# Auto-generated contract tests:

@auto_generated(from: :transfer, tier: 1)
test "transfer tier 1 contracts"
  # Boundary tests for Tier 1 (static) contracts
  from = Account.new(balance: Money.new(1000))
  to = Account.new(balance: Money.new(500))

  # amount > Money.zero
  assert_raises ContractViolation do
    transfer(from, to, Money.zero)
  end
  assert_raises ContractViolation do
    transfer(from, to, Money.new(-100))
  end

  # from.id != to.id
  assert_raises ContractViolation do
    transfer(from, from, Money.new(100))
  end
end

@auto_generated(from: :transfer, tier: 2)
test "transfer tier 2 contracts"
  # Cached analysis contracts
  from = Account.new(balance: Money.new(100))
  to = Account.new(balance: Money.new(500))

  # from.balance >= amount
  assert_raises ContractViolation do
    transfer(from, to, Money.new(200))  # Insufficient funds
  end

  # active? checks
  inactive_account = Account.new(balance: Money.new(1000), active: false)
  assert_raises ContractViolation do
    transfer(inactive_account, to, Money.new(100))
  end
end

@auto_generated(from: :transfer, tier: 3)
property "transfer tier 3 contracts"
  forall(
    from_balance: Gen.int(100, 10000).map |n| Money.new(n) end,
    to_balance: Gen.int(0, 10000).map |n| Money.new(n) end,
    amount: Gen.int(1, 99).map |n| Money.new(n) end
  )
  do
    from = Account.new(balance: from_balance, active: true)
    to = Account.new(balance: to_balance, active: true)

    old_from_balance = from.balance
    old_to_balance = to.balance

    result = transfer(from, to, amount)

    # Verify postconditions
    assert_eq from.balance, old_from_balance - amount,
              "from.balance should decrease by amount"
    assert_eq to.balance, old_to_balance + amount,
              "to.balance should increase by amount"
    assert result.ok?, "Transfer should succeed"
    assert_eq result.unwrap.amount, amount
  end
end
```

### 10.4 Fuzz Test Pattern

```aria
module Tests.Parser

  @fuzz_target
  fn fuzz_json_parser(data: Bytes) -> FuzzResult
    try
      let input = data.to_string_lossy
      let parsed = JSON.parse(input)

      # Round-trip test: parse -> serialize -> parse should be equal
      let serialized = JSON.serialize(parsed)
      let reparsed = JSON.parse(serialized)

      if parsed != reparsed
        FuzzResult.crash(
          RoundTripError("Round-trip failed: #{parsed} != #{reparsed}")
        )
      else
        FuzzResult.ok
      end
    catch JSON.ParseError
      # Invalid JSON is expected
      FuzzResult.ok
    catch e
      FuzzResult.crash(e)
    end
  end

  @fuzz_target(structured: JSONValue)
  fn fuzz_json_structured(value: JSONValue) -> FuzzResult
    try
      # Serialize and reparse
      let serialized = JSON.serialize(value)
      let reparsed = JSON.parse(serialized)

      if value != reparsed
        FuzzResult.crash(
          RoundTripError("Structured round-trip failed")
        )
      else
        FuzzResult.ok
      end
    catch e
      FuzzResult.crash(e)
    end
  end

  grammar JSONValue
    start: value

    value: object | array | string | number | bool | null

    object: "{" (pair ("," pair)*)? "}"
    pair: string ":" value

    array: "[" (value ("," value)*)? "]"

    string: '"' char* '"'
    char: /[^"\\]/ | "\\" /["\\\/bfnrt]/ | "\\u" /[0-9a-fA-F]{4}/

    number: "-"? ("0" | /[1-9]/ /[0-9]*/) ("." /[0-9]+/)? (/[eE]/ /[+-]?/ /[0-9]+/)?

    bool: "true" | "false"
    null: "null"
  end

  fuzz_config "json_parsing"
    corpus_dir: "corpus/json"
    crash_dir: "crashes/json"

    max_len: 100000
    timeout: 1.second

    dictionary: [
      "{", "}", "[", "]", ":", ",",
      "\"", "true", "false", "null",
      "\\n", "\\r", "\\t", "\\u0000"
    ]

    coverage_target: 90.percent
  end
end
```

### 10.5 State Machine Test Pattern

```aria
module Tests.Database

  state_machine "database connection lifecycle"
    type State = DatabaseConnection

    initial_state() = DatabaseConnection.disconnected()

    rule "connect" with(url: Gen.elements(["localhost", "remote.db"]))
      when |state| state.disconnected?
      do |state|
        state.connect(url)
      end
      then |old_state, new_state|
        assert new_state.connected?
        assert new_state.url == url
      end
    end

    rule "execute query" with(query: Gen.elements(["SELECT 1", "SELECT * FROM test"]))
      when |state| state.connected?
      do |state|
        state.execute(query)
        state
      end
      then |old_state, new_state|
        assert new_state.connected?
        assert new_state.query_count == old_state.query_count + 1
      end
    end

    rule "begin transaction"
      when |state| state.connected? && !state.in_transaction?
      do |state|
        state.begin_transaction
      end
      then |_, new_state|
        assert new_state.in_transaction?
      end
    end

    rule "commit transaction"
      when |state| state.in_transaction?
      do |state|
        state.commit
      end
      then |_, new_state|
        assert !new_state.in_transaction?
      end
    end

    rule "rollback transaction"
      when |state| state.in_transaction?
      do |state|
        state.rollback
      end
      then |_, new_state|
        assert !new_state.in_transaction?
      end
    end

    rule "disconnect"
      when |state| state.connected? && !state.in_transaction?
      do |state|
        state.disconnect
      end
      then |_, new_state|
        assert new_state.disconnected?
      end
    end

    invariant |state|
      # Cannot be in transaction while disconnected
      !(state.disconnected? && state.in_transaction?)
    end
  end
end
```

---

## 11. Design Decisions Summary

### 11.1 Syntax Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Test block keyword | `test` | Ruby influence, familiar to developers |
| Property keyword | `property` | Clear intent, distinct from `test` |
| Quantifier syntax | `forall(x: Type)` | Explicit typing, clear scope |
| Assertion function | `assert`, `assert_eq`, etc. | Explicit, debuggable |
| Generator module | `Gen` | Short, matches QuickCheck convention |
| Fuzz annotation | `@fuzz_target` | Annotation for special behavior |

### 11.2 Semantic Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Shrinking strategy | Hybrid (type-based + byte buffer) | Balance simplicity and customization |
| Contract test generation | Automatic based on tier | Leverage existing contract infrastructure |
| Default iterations | 100 for properties | Balance thoroughness and speed |
| Coverage integration | Built-in | Critical for test quality assessment |
| Parallel execution | Default | Modern CPUs benefit from parallelism |

### 11.3 Integration Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Single CLI entry point | `aria test` | Unified interface for all test types |
| Tiered contract tests | Follow ARIA-M04-04 tiers | Consistent with contract system |
| Effect-aware testing | Support `@async` tests | Integrate with effect system |
| Doctest format | Rust-style triple backticks | Familiar, well-documented |

---

## 12. Implementation Roadmap

### Phase 1: Core Unit Testing (4 weeks)
- `test` block parsing and AST
- Basic assertions (`assert`, `assert_eq`, etc.)
- Test discovery and sequential runner
- CLI integration (`aria test`)

### Phase 2: Property-Based Testing (6 weeks)
- Generator[T] type implementation
- Generable trait and built-in instances
- Shrinking algorithm (hybrid approach)
- `property` block syntax
- Coverage classification

### Phase 3: Contract Integration (4 weeks)
- Contract-to-generator extraction
- Tiered test generation (Tier 1/2/3)
- Contract chain analysis
- Auto-generated test naming

### Phase 4: Mock/Stub Framework (3 weeks)
- Effect-based mock handlers
- MockBuilder fluent API
- Capture/Spy implementation
- Fixture system with composition

### Phase 5: Coverage Tracking (3 weeks)
- Compiler instrumentation pass (`--coverage`)
- Coverage runtime data collection
- Report generation (HTML/JSON/LCOV)
- Contract coverage tracking

### Phase 6: Benchmark Framework (3 weeks)
- `@benchmark` annotation parsing
- Statistical analysis (mean, std_dev, percentiles)
- Comparison and regression detection
- Output formats (console, JSON, CSV)

### Phase 7: Doctest Support (3 weeks)
- Doc comment parsing
- Example extraction
- Harness generation
- Hidden line support

### Phase 8: Fuzzing Infrastructure (6 weeks)
- @fuzz_target annotation
- Coverage-guided mutation
- Corpus management
- Grammar-based fuzzing
- Custom mutators

### Phase 9: Parallel Execution (3 weeks)
- Work-stealing test scheduler
- Deterministic output ordering
- Test isolation via effect scopes
- Resource contention handling

### Phase 10: Polish and Integration (2 weeks)
- Error message refinement
- IDE integration (test lens, coverage gutter)
- Documentation and examples
- Performance optimization

---

## 13. Success Metrics

### 13.1 Quantitative Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Test discovery time | < 100ms for 10K tests | Benchmark suite |
| Property test throughput | > 10K iterations/second | Benchmark suite |
| Shrinking effectiveness | Find minimal in < 1000 steps | Benchmark suite |
| Contract test generation | > 95% coverage of contracts | Static analysis |
| Coverage overhead | < 10% runtime | Benchmark suite |
| Parallel speedup | > 0.8 * CPU cores | Scaling tests |
| Fuzz test crash detection | Match libFuzzer baseline | Benchmark |
| Benchmark precision | < 5% coefficient of variation | Statistical analysis |

### 13.2 Qualitative Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Test readability | "Tests read like specs" | Code review |
| Error message clarity | "Understood without docs" | User study |
| Mock ergonomics | "Effect mocking is natural" | Developer feedback |
| Learning curve | < 1 hour to first test | Onboarding time |
| Coverage actionability | "Uncovered code is clear" | User survey |

---

## Appendix A: Assertion Reference

| Assertion | Description | Example |
|-----------|-------------|---------|
| `assert(cond)` | Boolean condition | `assert user.active?` |
| `assert_eq(a, b)` | Equality | `assert_eq result, 42` |
| `assert_ne(a, b)` | Inequality | `assert_ne error, nil` |
| `assert_lt(a, b)` | Less than | `assert_lt count, 100` |
| `assert_le(a, b)` | Less or equal | `assert_le age, 150` |
| `assert_gt(a, b)` | Greater than | `assert_gt balance, 0` |
| `assert_ge(a, b)` | Greater or equal | `assert_ge score, 0` |
| `assert_approx(a, b, e)` | Float equality | `assert_approx pi, 3.14, 0.01` |
| `assert_contains(c, e)` | Collection contains | `assert_contains list, item` |
| `assert_empty(c)` | Collection empty | `assert_empty errors` |
| `assert_raises(E) { }` | Exception raised | `assert_raises DivZero { divide(1, 0) }` |
| `assert_no_raise { }` | No exception | `assert_no_raise { safe_op() }` |
| `assert_type[T](v)` | Type check | `assert_type[User](obj)` |
| `assert_matches(s, r)` | Regex match | `assert_matches email, /\w+@\w+/` |
| `assert_eventually(f)` | Async/retry | `assert_eventually { ready?() }` |

---

## Appendix B: Generator Reference

| Generator | Description | Example |
|-----------|-------------|---------|
| `Gen.constant(v)` | Always same value | `Gen.constant(42)` |
| `Gen.elements(arr)` | Pick from array | `Gen.elements([1, 2, 3])` |
| `Gen.one_of(gens)` | Pick generator | `Gen.one_of([gen1, gen2])` |
| `Gen.frequency(ws)` | Weighted pick | `Gen.frequency([(3, gen1), (1, gen2)])` |
| `Gen.int(min, max)` | Integer range | `Gen.int(0, 100)` |
| `Gen.float(min, max)` | Float range | `Gen.float(0.0, 1.0)` |
| `Gen.positive_int()` | Positive integer | `Gen.positive_int()` |
| `Gen.string(c, r)` | String from chars | `Gen.string(Gen.alpha_char, 1..20)` |
| `Gen.ascii_string()` | ASCII string | `Gen.ascii_string()` |
| `Gen.list(elem)` | List of elements | `Gen.list(Gen.int(0, 100))` |
| `Gen.non_empty_list(e)` | Non-empty list | `Gen.non_empty_list(Gen.int(0, 100))` |
| `Gen.vector(n, elem)` | Fixed-size array | `Gen.vector(5, Gen.int(0, 10))` |
| `Gen.map_of(k, v)` | Map/dict | `Gen.map_of(Gen.string(), Gen.int())` |
| `Gen.sorted_array(e)` | Sorted array | `Gen.sorted_array(Gen.int())` |
| `Gen.unique_array(e)` | Unique elements | `Gen.unique_array(Gen.int())` |
| `Gen.permutation(arr)` | Shuffle array | `Gen.permutation([1, 2, 3, 4])` |
| `Gen.subset(arr)` | Subset of array | `Gen.subset([1, 2, 3, 4])` |

---

## Appendix C: Test Annotations

| Annotation | Target | Description |
|------------|--------|-------------|
| `@async` | test | Async test execution |
| `@timeout(duration)` | test | Custom timeout |
| `@tags(array)` | test | Test categorization |
| `@parametrize(data)` | test | Data-driven test |
| `@skip(reason)` | test | Skip test |
| `@skip_if(cond)` | test | Conditional skip |
| `@slow` | test | Mark as slow |
| `@flaky(retries)` | test | Retry on failure |
| `@shrink_limit(n)` | property | Max shrink attempts |
| `@shrink_depth(n)` | property | Max shrink depth |
| `@no_shrink` | property | Disable shrinking |
| `@fuzz_target` | fn | Mark as fuzz target |
| `@custom_mutator(T)` | fn | Custom fuzz mutator |
| `@auto_generated` | test | Compiler-generated test |
| `@contract_test` | fn | Configure contract test generation |
| `@no_contract_test` | fn | Disable contract test generation |
| `@doctest` | test | Mark as doctest |
| `@compile_only` | test | Only compile, don't run |
| `@compile_fail` | test | Expect compilation failure |

---

**Document Status**: Approved for implementation
**Next Steps**: ARIA-M12-03 - Implement Generator[T] type and basic properties
**Owner**: Testing Infrastructure Team
**Reviewers**: ANCHOR (Research), SENTINEL (Contracts), ARBITER (Product)
