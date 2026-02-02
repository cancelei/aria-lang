# ARIA-M12-02: Testing Framework Design

**Task ID**: ARIA-M12-02
**Status**: Completed
**Date**: 2026-01-15
**Focus**: Comprehensive testing framework design for Aria
**Agent**: ANCHOR (Research)

---

## Executive Summary

This research document defines Aria's comprehensive testing framework, integrating four major testing paradigms: contract-to-test extraction (from Dafny/Eiffel), property-based testing (from QuickCheck/Hypothesis), documentation example testing (from Rust doctests), and fuzz testing (from AFL/libFuzzer). The design leverages Aria's contract system to automatically generate tests while providing explicit APIs for each testing style.

---

## 1. Contract-to-Test Extraction Design

### 1.1 Foundation: Contracts as Test Oracles

Drawing from [Eiffel AutoTest](https://www.eiffel.com/values/automatic-testing/) and [Dafny's DTest toolkit](https://www.cs.tufts.edu/~jfoster/papers/nfm2023.pdf), Aria treats contracts as implicit test oracles:

```aria
fn binary_search(arr: Array[Int], target: Int) -> Option[Int]
  requires arr.sorted?
  requires arr.length > 0
  ensures |result|
    match result
      Some(idx) => arr[idx] == target
      None => !arr.contains(target)
    end
  end
  # ... implementation
end
```

**Automatic Test Generation**:
1. Preconditions define valid input space
2. Postconditions define expected behavior (oracle)
3. Invariants define state constraints

### 1.2 Contract Extraction Pipeline

```
+----------------+     +------------------+     +----------------+
| Parse Contract | --> | Extract Clauses  | --> | Generate Tests |
+----------------+     +------------------+     +----------------+
                              |
                              v
                    +-------------------+
                    | Constraint Solver |
                    | (for valid inputs)|
                    +-------------------+
```

**Phase 1: Contract Parsing**
```aria
# Internal representation
ContractSpec {
  preconditions: Array[Constraint]
  postconditions: Array[Constraint]
  invariants: Array[Constraint]

  fn to_test_generator() -> TestGenerator
    # Convert constraints to generators
  end
}
```

**Phase 2: Constraint-to-Generator Transformation**

| Contract Clause | Generator Strategy |
|-----------------|-------------------|
| `requires x > 0` | `Gen.int(1, Int.MAX)` |
| `requires x >= 0 && x < 100` | `Gen.int(0, 99)` |
| `requires arr.sorted?` | `Gen.sorted_array()` |
| `requires arr.length > 0` | `Gen.non_empty_array()` |
| `requires obj != nil` | `Gen.non_nil()` |

**Phase 3: Test Generation**

```aria
# Compiler-generated test from contracts
@auto_generated(from: :binary_search)
test "binary_search satisfies contract"
  property(iterations: 100) do |sorted_arr: SortedArray[Int], target: Int|
    # Preconditions become input constraints
    assume sorted_arr.length > 0

    result = binary_search(sorted_arr.to_array, target)

    # Postconditions become assertions
    match result
      Some(idx) => assert sorted_arr[idx] == target
      None => assert !sorted_arr.contains(target)
    end
  end
end
```

### 1.3 Tiered Contract Testing

Integration with Aria's [tiered contract system](./ARIA-M04-04-tiered-contract-system.md):

| Contract Tier | Test Strategy |
|---------------|---------------|
| **Tier 1 (Static)** | No runtime tests needed; compiler verifies |
| **Tier 2 (Cached)** | Generate boundary tests; cache results |
| **Tier 3 (Dynamic)** | Full property-based testing required |

```aria
# Tier 1: Compiler verifies, no generated test
fn abs(x: Int) -> Int
  ensures result >= 0  # Statically verified
  if x < 0 then -x else x
end

# Tier 3: Full test generation required
fn complex_sort(arr: Array[T]) -> Array[T]
  requires arr.length > 0
  ensures result.permutation_of?(arr)  # Dynamic: needs PBT
  ensures result.sorted?               # Dynamic: needs PBT
  # ...
end

# Generated test for Tier 3
@auto_generated(from: :complex_sort, tier: 3)
test "complex_sort contract verification"
  property(iterations: 1000) do |arr: NonEmptyArray[Int]|
    result = complex_sort(arr.to_array)
    assert result.permutation_of?(arr)
    assert result.sorted?
  end
end
```

### 1.4 Contract Chain Testing

Inspired by [Property Tests + Contracts = Integration Tests](https://www.hillelwayne.com/post/pbt-contracts/):

```aria
# When function A calls function B, both contracts are tested
fn process_data(raw: String) -> ProcessedData
  requires raw.length > 0
  ensures result.valid?

  parsed = parse(raw)  # parse has its own contracts
  validated = validate(parsed)  # validate has its own contracts
  transform(validated)  # transform has its own contracts
end

# Generated integration test
@auto_generated(from: :process_data, chain: true)
test "process_data contract chain"
  property do |raw: NonEmptyString|
    # All contracts in the chain are verified:
    # - process_data.requires
    # - parse.requires, parse.ensures
    # - validate.requires, validate.ensures
    # - transform.requires, transform.ensures
    # - process_data.ensures
    result = process_data(raw)
    assert result.valid?
  end
end
```

---

## 2. Property-Based Testing API

### 2.1 Core Design (QuickCheck-Inspired)

Based on research from [QuickCheck](https://hackage.haskell.org/package/QuickCheck) and [Hypothesis](https://hypothesis.readthedocs.io/):

#### 2.1.1 The Generator Type

```aria
# Core generator abstraction (Gen monad pattern)
struct Generator[T]
  private fn: (Seed, Size) -> T

  # Monadic operations
  fn map[U](f: T -> U) -> Generator[U]
    Generator.new |seed, size|
      f(self.generate(seed, size))
    end
  end

  fn flat_map[U](f: T -> Generator[U]) -> Generator[U]
    Generator.new |seed, size|
      let (seed1, seed2) = seed.split
      let intermediate = self.generate(seed1, size)
      f(intermediate).generate(seed2, size)
    end
  end

  fn filter(predicate: T -> Bool) -> Generator[T]
    Generator.new |seed, size|
      loop
        let candidate = self.generate(seed.next, size)
        if predicate(candidate) then return candidate end
      end
    end
  end

  fn generate(seed: Seed, size: Size) -> T
    self.fn(seed, size)
  end
end
```

#### 2.1.2 The Generable Trait (Arbitrary equivalent)

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
    if x == 0 then []
    else [0, x / 2, x - 1].filter |v| v.abs < x.abs end
    end
  end
end

impl Generable[Bool]
  fn arbitrary() -> Generator[Bool]
    Generator.elements([true, false])
  end

  fn shrink(x: Bool) -> Array[Bool]
    if x then [false] else [] end
  end
end

impl[T: Generable] Generable[Array[T]]
  fn arbitrary() -> Generator[Array[T]]
    Generator.sized |size|
      let length = Generator.int(0, size)
      length.flat_map |n|
        Generator.sequence(Array.fill(n, T.arbitrary))
      end
    end
  end

  fn shrink(arr: Array[T]) -> Array[Array[T]]
    # Shrinking strategies:
    # 1. Remove elements
    # 2. Shrink individual elements
    let removals = (0..arr.length).map |i| arr.remove_at(i) end
    let element_shrinks = arr.flat_map_indexed |i, elem|
      T.shrink(elem).map |smaller| arr.replace_at(i, smaller) end
    end
    removals ++ element_shrinks
  end
end
```

#### 2.1.3 Generator Combinators

```aria
module Generator
  # Basic combinators
  fn constant[T](value: T) -> Generator[T]
  fn elements[T](choices: Array[T]) -> Generator[T]
  fn one_of[T](generators: Array[Generator[T]]) -> Generator[T]
  fn frequency[T](weighted: Array[(Int, Generator[T])]) -> Generator[T]

  # Size-aware combinators
  fn sized[T](f: Size -> Generator[T]) -> Generator[T]
  fn resize[T](new_size: Size, gen: Generator[T]) -> Generator[T]
  fn scale[T](f: Size -> Size, gen: Generator[T]) -> Generator[T]

  # Collection combinators
  fn list_of[T](elem: Generator[T]) -> Generator[Array[T]]
  fn non_empty_list[T](elem: Generator[T]) -> Generator[Array[T]]
  fn vector_of[T](n: Int, elem: Generator[T]) -> Generator[Array[T]]
  fn map_of[K, V](keys: Generator[K], values: Generator[V]) -> Generator[Map[K, V]]

  # Numeric generators
  fn int(min: Int, max: Int) -> Generator[Int]
  fn float(min: Float, max: Float) -> Generator[Float]
  fn positive_int() -> Generator[Int]
  fn negative_int() -> Generator[Int]

  # String generators
  fn string(alphabet: String, max_length: Int) -> Generator[String]
  fn ascii_string() -> Generator[String]
  fn unicode_string() -> Generator[String]
  fn alphanumeric() -> Generator[String]

  # Specialized generators
  fn sorted_array[T: Ord](elem: Generator[T]) -> Generator[Array[T]]
  fn unique_array[T: Eq](elem: Generator[T]) -> Generator[Array[T]]
end
```

### 2.2 Property Definition Syntax

```aria
# Basic property
property "addition is commutative"
  forall(a: Int, b: Int)
    a + b == b + a
  end
end

# With explicit generators
property "sorted array remains sorted after insert"
  forall(arr: Generator.sorted_array(Generator.int(-100, 100)), x: Int)
    arr.insert_sorted(x).sorted?
  end
end

# Conditional properties (==> equivalent)
property "division specification"
  forall(a: Int, b: Int)
    assuming b != 0
    (a / b) * b + (a % b) == a
  end
end

# With classification (like QuickCheck's classify)
property "list operations"
  forall(xs: Array[Int])
    classify(xs.empty?, "empty")
    classify(xs.length == 1, "singleton")
    classify(xs.length > 10, "large")

    xs.reverse.reverse == xs
  end
end

# Coverage requirements (like Hypothesis's @settings)
property "covers edge cases" with(
  min_examples: 100,
  coverage: [
    ("empty", 5%),
    ("singleton", 10%),
    ("large", 20%)
  ]
)
  forall(xs: Array[Int])
    classify(xs.empty?, "empty")
    classify(xs.length == 1, "singleton")
    classify(xs.length > 10, "large")
    xs.sort.sorted?
  end
end
```

### 2.3 Shrinking Strategy

Based on [QuickCheck shrinking](https://begriffs.com/posts/2017-01-14-design-use-quickcheck.html) and Hypothesis's conjecture approach:

```aria
# Shrinking algorithm
fn find_minimal_counterexample[T: Generable](
  property: T -> Bool,
  failing_input: T
) -> T
  let mut current = failing_input
  let mut shrunk = true

  while shrunk
    shrunk = false
    for candidate in T.shrink(current)
      if !property(candidate)
        current = candidate
        shrunk = true
        break
      end
    end
  end

  current
end

# Configurable shrinking
@property(shrink_limit: 1000, shrink_depth: 50)
property "finds minimal counterexample"
  forall(xs: Array[Int])
    xs.sum >= 0  # Will find minimal negative example
  end
end
```

### 2.4 Stateful Testing

Inspired by [Hypothesis stateful testing](https://github.com/HypothesisWorks/hypothesis/blob/master/hypothesis-python/docs/stateful.rst):

```aria
# State machine testing
state_machine "Stack operations"
  type State = Stack[Int]

  initial_state() = Stack.empty

  rule "push" with(x: Int)
    when true  # Always applicable
    do |state|
      state.push(x)
    end
    then |state, result|
      assert result.top == x
      assert result.length == state.length + 1
    end
  end

  rule "pop"
    when |state| !state.empty?  # Precondition
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

---

## 3. Fuzzing Integration Strategy

### 3.1 Architecture Overview

Based on [libFuzzer](https://llvm.org/docs/LibFuzzer.html) and [AFL++](https://github.com/AFLplusplus/LibAFL):

```
+------------------+     +------------------+     +------------------+
| Aria Fuzzer API  | --> | Coverage Tracker | --> | Mutation Engine  |
+------------------+     +------------------+     +------------------+
        |                        |                        |
        v                        v                        v
+------------------+     +------------------+     +------------------+
| Test Harness     | <-- | Corpus Manager   | <-- | Crash Reporter   |
+------------------+     +------------------+     +------------------+
```

### 3.2 Fuzz Target API

```aria
# libFuzzer-compatible entry point
@fuzz_target
fn fuzz_parser(data: ByteArray) -> FuzzResult
  try
    let parsed = Parser.parse(data.to_string)
    # If we get here, parsing succeeded
    FuzzResult.ok
  catch ParseError
    # Expected error, not a crash
    FuzzResult.ok
  catch e: Exception
    # Unexpected error - report as interesting
    FuzzResult.crash(e)
  end
end

# With structured input (grammar-based fuzzing)
@fuzz_target(structured: ArithmeticExpr)
fn fuzz_evaluator(expr: ArithmeticExpr) -> FuzzResult
  try
    let result = evaluate(expr)
    # Verify result is reasonable
    assert result.finite?
    FuzzResult.ok
  catch DivisionByZero
    FuzzResult.ok  # Expected
  catch e: Exception
    FuzzResult.crash(e)
  end
end
```

### 3.3 Coverage-Guided Mutation

```aria
# Mutation strategies (based on libFuzzer)
enum MutationStrategy
  ChangeByte      # Flip single byte
  ChangeBit       # Flip single bit
  InsertByte      # Insert random byte
  EraseBytes      # Remove bytes
  CrossOver       # Combine two corpus entries
  CopyPart        # Duplicate section
  Dictionary      # Insert dictionary words
end

# Custom mutators for structured fuzzing
@custom_mutator(for: ArithmeticExpr)
fn mutate_expr(expr: ArithmeticExpr, seed: Seed) -> ArithmeticExpr
  match seed.choose(5)
    0 => expr.replace_random_operand(seed.next_int)
    1 => expr.swap_operators
    2 => expr.nest_in_operation(seed.choose_op)
    3 => expr.simplify_random_subtree
    4 => expr.duplicate_subtree
  end
end
```

### 3.4 Grammar-Based Fuzzing

Inspired by [CSmith](https://arxiv.org/pdf/2306.06884) for compiler testing:

```aria
# Grammar definition for fuzzing
grammar AriaExpr
  start: expr

  expr: literal
      | binary_op
      | unary_op
      | call
      | if_expr

  literal: INT | FLOAT | STRING | BOOL

  binary_op: expr OP expr
  OP: "+" | "-" | "*" | "/" | "==" | "!=" | "<" | ">"

  unary_op: "-" expr | "!" expr

  call: IDENT "(" arg_list ")"
  arg_list: expr ("," expr)*

  if_expr: "if" expr "then" expr "else" expr "end"
end

# Generate syntactically valid programs
@fuzz_target(grammar: AriaExpr, iterations: 10000)
fn fuzz_aria_compiler(program: AriaExpr)
  try
    let compiled = Compiler.compile(program.to_string)
    # Differential testing: compare interpreted vs compiled
    let interpreted = Interpreter.eval(program)
    let executed = compiled.run()
    assert interpreted == executed
  catch CompilerError
    FuzzResult.ok  # Expected for some inputs
  catch e
    FuzzResult.crash(e)
  end
end
```

### 3.5 Contract-Aware Fuzzing

Unique to Aria: leverage contracts to guide fuzzing:

```aria
# Fuzz with contract awareness
@fuzz_target(contract_guided: true)
fn fuzz_binary_search(data: ByteArray)
  # Deserialize to valid inputs based on contracts
  let arr = data.deserialize_as(SortedArray[Int])  # Respects sorted? contract
  let target = data.deserialize_as(Int)

  let result = binary_search(arr.to_array, target)

  # Postconditions verified automatically
  verify_postconditions(binary_search, arr, target, result)
end

# Precondition-guided input generation
fn generate_valid_input[T](
  contract: ContractSpec,
  seed: Seed
) -> T
  # Use SMT solver to generate inputs satisfying preconditions
  let constraints = contract.preconditions
  SMTSolver.solve_for_example(constraints, seed)
end
```

### 3.6 Fuzzer Configuration

```aria
# Fuzzer settings
fuzz_config "parser_fuzzing"
  corpus_dir: "corpus/parser"
  crash_dir: "crashes/parser"

  max_len: 10000        # Maximum input size
  timeout: 5.seconds    # Per-test timeout
  memory_limit: 512.mb  # Memory limit

  # Sanitizers
  address_sanitizer: true
  undefined_behavior: true

  # Mutation settings
  mutation_depth: 5
  dictionary: ["fn", "end", "if", "then", "else", "match"]

  # Coverage targets
  coverage_target: 80%
  edge_coverage: true
end
```

---

## 4. Example Testing Patterns (Doctest-Style)

### 4.1 Design Philosophy

Based on [Rust's doctest system](https://doc.rust-lang.org/rustdoc/documentation-tests.html):

> "Documentation examples typically aren't expected to immediately work, but [doctests ensure] examples you give in your doc comments will actually work."

### 4.2 Syntax Design

```aria
# Documentation with executable examples
/// Sorts an array in ascending order.
///
/// # Examples
///
/// ```aria
/// arr = [3, 1, 4, 1, 5]
/// result = sort(arr)
/// assert result == [1, 1, 3, 4, 5]
/// ```
///
/// ```aria should_panic
/// # This demonstrates error handling
/// sort(nil)  # Raises ContractViolation
/// ```
///
/// ```aria no_run
/// # This compiles but doesn't execute in tests
/// large_array = Array.fill(1_000_000, random_int)
/// sort(large_array)  # Would be too slow for doctest
/// ```
fn sort[T: Ord](arr: Array[T]) -> Array[T]
  # ... implementation
end
```

### 4.3 Doctest Attributes

| Attribute | Behavior |
|-----------|----------|
| (none) | Compile and run, must not panic |
| `should_panic` | Must panic to pass |
| `no_run` | Compile only, don't execute |
| `ignore` | Skip entirely (for broken examples) |
| `compile_fail` | Must fail to compile |

### 4.4 Hidden Code Lines

```aria
/// Binary search implementation.
///
/// # Examples
///
/// ```aria
/// # fn main() {
/// #   let arr = [1, 2, 3, 4, 5]
/// let idx = binary_search(arr, 3)
/// assert idx == Some(2)
/// # }
/// ```
///
/// Lines starting with `#` are hidden from docs but included in test.
fn binary_search(arr: Array[Int], target: Int) -> Option[Int]
  # ...
end
```

### 4.5 Example Extraction Pipeline

```
+-------------------+     +------------------+     +------------------+
| Parse Doc Comment | --> | Extract Examples | --> | Wrap in Harness  |
+-------------------+     +------------------+     +------------------+
                                                          |
                                                          v
                                              +------------------+
                                              | Compile & Run    |
                                              +------------------+
```

**Harness Generation**:

```aria
# Original doctest:
# ```aria
# result = sort([3, 1, 2])
# assert result == [1, 2, 3]
# ```

# Generated harness:
@doctest(file: "sort.aria", line: 5)
test "sort example at line 5"
  result = sort([3, 1, 2])
  assert result == [1, 2, 3]
end
```

### 4.6 Example-Property Bridge

Unique to Aria: promote examples to property tests:

```aria
/// Reverses an array.
///
/// # Examples
///
/// ```aria
/// assert reverse([1, 2, 3]) == [3, 2, 1]
/// assert reverse([]) == []
/// assert reverse([42]) == [42]
/// ```
///
/// # Properties (auto-generated from examples + contracts)
///
/// ```aria property
/// forall(arr: Array[Int])
///   reverse(reverse(arr)) == arr
/// end
/// ```
fn reverse[T](arr: Array[T]) -> Array[T]
  ensures result.length == arr.length
  ensures result.reverse == arr  # This is the invariant!
  # ...
end

# Compiler can infer properties from contracts:
# - reverse(reverse(x)) == x (from postcondition)
# - reverse(x).length == x.length (from postcondition)
```

---

## 5. Mutation Testing Approach

### 5.1 Overview

Based on [PIT](https://pitest.org/quickstart/mutators/) and [Stryker](https://stryker-mutator.io/):

```
Mutation Score = (Killed Mutants) / (Total Mutants - Equivalent Mutants) * 100
```

### 5.2 Mutation Operators

```aria
# Aria mutation operators
enum MutationOperator
  # Arithmetic
  ArithmeticReplace   # + -> -, * -> /
  ArithmeticDelete    # a + b -> a

  # Relational
  RelationalReplace   # < -> <=, == -> !=
  BoundaryMutate      # x < 10 -> x <= 10

  # Boolean
  NegateCondition     # if x -> if !x
  RemoveCondition     # if x then a else b -> a

  # Return values
  EmptyReturn         # return x -> return default
  NullReturn          # return x -> return nil

  # Collection
  EmptyCollection     # [1,2,3] -> []
  RemoveElement       # [1,2,3] -> [1,3]

  # Contract-specific
  WeakenPrecondition  # requires x > 0 -> requires x >= 0
  StrengthenPost      # ensures x > 0 -> ensures x > 1
end
```

### 5.3 Mutation Testing API

```aria
# Run mutation testing
mutation_test "sort function"
  target: sort
  operators: [
    :arithmetic_replace,
    :relational_replace,
    :boundary_mutate,
    :negate_condition
  ]

  # Test suite to run against mutants
  tests: [
    "sort_empty_array",
    "sort_single_element",
    "sort_already_sorted",
    "sort_reverse_sorted",
    "sort_duplicates"
  ]

  # Thresholds
  min_mutation_score: 80%
  timeout_per_mutant: 5.seconds
end
```

### 5.4 Equivalent Mutant Detection

Addressing the [equivalent mutant problem](https://en.wikipedia.org/wiki/Mutation_testing):

```aria
# Strategies for equivalent mutant detection
enum EquivalentMutantStrategy
  # Compiler-based detection
  CompilerEquivalence   # If mutant compiles to same bytecode

  # Contract-based detection
  ContractViolation     # If mutant violates contract statically

  # Semantic analysis
  DataFlowAnalysis      # If mutation affects unreachable code

  # Higher-order mutation
  HigherOrderMutation   # Combine mutations to reduce equivalents
end

# Aria-specific: use contracts to detect equivalents
fn detect_equivalent_mutants(
  original: Function,
  mutants: Array[Mutant]
) -> Array[Mutant]
  mutants.filter |mutant|
    # If mutant still satisfies all contracts, might be equivalent
    !contracts_violated_statically?(original.contracts, mutant)
  end
end
```

### 5.5 Contract-Driven Mutation

Unique to Aria: use contracts to generate meaningful mutations:

```aria
fn divide(a: Int, b: Int) -> Int
  requires b != 0
  ensures result * b <= a
  ensures result * b > a - b.abs
  # ...
end

# Contract-aware mutations:
# 1. Weaken precondition: requires b != 0 -> (removed)
#    -> Should be caught by division-by-zero test
#
# 2. Break postcondition: return a / b -> return a / b + 1
#    -> Should be caught by contract verification test
#
# 3. Boundary mutation: result * b <= a -> result * b < a
#    -> Tests boundary behavior
```

---

## 6. Unified Testing Framework

### 6.1 Test Runner Architecture

```aria
# Unified test command
aria test [OPTIONS] [PATHS...]

Options:
  --unit           Run unit tests only
  --property       Run property-based tests
  --doctest        Run documentation tests
  --fuzz           Run fuzz tests
  --mutation       Run mutation analysis
  --contract       Run contract-derived tests
  --all            Run all test types (default)

  --coverage       Generate coverage report
  --parallel N     Run N tests in parallel
  --timeout T      Per-test timeout
  --seed S         Random seed for reproducibility
```

### 6.2 Test Discovery

```aria
# Test discovery rules
TestDiscovery {
  # Unit tests: functions with @test attribute
  unit: @test fn ...

  # Property tests: property blocks
  property: property "..." { ... }

  # Doctests: examples in doc comments
  doctest: /// ```aria ... ```

  # Fuzz targets: @fuzz_target functions
  fuzz: @fuzz_target fn ...

  # Contract tests: auto-generated from contracts
  contract: fn with requires/ensures
}
```

### 6.3 Integrated Reporting

```
Test Results Summary
====================

Unit Tests:        42 passed,  0 failed,  0 skipped
Property Tests:    15 passed,  1 failed,  0 skipped
Doc Tests:         28 passed,  0 failed,  2 skipped
Fuzz Tests:         5 passed,  0 failed,  0 skipped (10000 iterations)
Contract Tests:    67 passed,  0 failed,  0 skipped (auto-generated)

Coverage: 87.3% (target: 80%)
Mutation Score: 82.1% (target: 80%)

Failed Tests:
-------------
property "sort preserves elements" (sort.aria:45)
  Counterexample (shrunk): [0, -1]

  Expected: sorted result contains all original elements
  Actual:   result = [0, 0] (duplicate element)

  Seed: 12345 (for reproduction)
```

---

## 7. Key Design Decisions

### 7.1 Contract-First Philosophy

| Decision | Rationale |
|----------|-----------|
| Contracts generate tests automatically | Reduces test maintenance; DRY principle |
| Preconditions filter property inputs | Valid inputs by construction |
| Postconditions become assertions | Contracts as oracles |
| Tiered verification determines test type | Static where possible, dynamic where needed |

### 7.2 Property-Based Testing Choices

| Decision | Rationale |
|----------|-----------|
| Hypothesis-style byte buffer for shrinking | Generic shrinking without custom shrinkers |
| Size parameter for bounded generation | Prevents huge test cases early |
| Coverage requirements in properties | Ensures edge cases are tested |
| Stateful testing via state machines | Complex interactions testable |

### 7.3 Fuzzing Integration

| Decision | Rationale |
|----------|-----------|
| libFuzzer-compatible API | Industry standard, tool reuse |
| Grammar-based fuzzing for compiler | Structured input for complex targets |
| Contract-guided input generation | Leverage existing specifications |
| Custom mutators for structured data | Better coverage for domain types |

---

## 8. Implementation Roadmap

### Phase 1: Core Property Testing (M12)
- Generator monad implementation
- Generable trait and built-in instances
- Basic shrinking
- Property syntax and test runner

### Phase 2: Contract Integration (M12 + M04)
- Contract-to-test extraction
- Precondition-based filtering
- Postcondition assertion generation
- Tiered test generation

### Phase 3: Doctest Support (M12)
- Doc comment parsing
- Example extraction
- Harness generation
- Hidden line support

### Phase 4: Fuzzing Infrastructure (M12 + M20)
- libFuzzer-compatible targets
- Coverage tracking
- Corpus management
- Grammar-based fuzzing

### Phase 5: Mutation Testing (M12 + M20)
- Mutation operators
- Equivalent mutant detection
- Mutation score reporting
- Contract-driven mutations

---

## 9. Key Resources

### Property-Based Testing
1. [QuickCheck: Automatic testing of Haskell programs](https://hackage.haskell.org/package/QuickCheck) - Original PBT library
2. [Hypothesis Documentation](https://hypothesis.readthedocs.io/) - Python PBT with strategies
3. [The Design and Use of QuickCheck](https://begriffs.com/posts/2017-01-14-design-use-quickcheck.html) - Design patterns
4. [Property Tests + Contracts = Integration Tests](https://www.hillelwayne.com/post/pbt-contracts/) - Contract integration

### Contract-Based Testing
5. [Eiffel AutoTest](https://www.eiffel.com/values/automatic-testing/) - Contract-aware test generation
6. [A Toolkit for Automated Testing of Dafny](https://www.cs.tufts.edu/~jfoster/papers/nfm2023.pdf) - DTest, DMock, DUnit

### Documentation Testing
7. [Rust Documentation Tests](https://doc.rust-lang.org/rustdoc/documentation-tests.html) - Doctest design

### Fuzzing
8. [libFuzzer Documentation](https://llvm.org/docs/LibFuzzer.html) - Coverage-guided fuzzing
9. [LibAFL: Advanced Fuzzing Library](https://github.com/AFLplusplus/LibAFL) - Rust fuzzing framework
10. [Compiler Fuzzing Survey](https://arxiv.org/pdf/2306.06884) - Grammar-based approaches

### Mutation Testing
11. [PIT Mutation Operators](https://pitest.org/quickstart/mutators/) - Java mutation testing
12. [Stryker Mutator](https://stryker-mutator.io/) - Multi-language mutation testing
13. [Equivalent Mutant Problem](https://en.wikipedia.org/wiki/Mutation_testing#Equivalent_mutants) - Detection strategies

---

## 10. Open Questions

1. **Shrinking Strategy**: Should Aria use Hypothesis-style byte buffer or QuickCheck-style explicit shrink functions? Trade-off between generality and customization.

2. **Contract Tier Interaction**: How do Tier 2 (cached) contracts interact with property-based testing? Should caching be disabled during PBT?

3. **Fuzz-Property Bridge**: Can we automatically convert property tests to fuzz targets and vice versa?

4. **Mutation + Contracts**: Should contract violations by mutants count as "killed" or be filtered as expected failures?

5. **Performance Budget**: How do we balance thorough testing with reasonable CI times? What's the default iteration count for properties?

---

## 11. Appendix: Comparison Matrix

| Feature | QuickCheck | Hypothesis | Rust/proptest | Aria (Proposed) |
|---------|------------|------------|---------------|-----------------|
| Generator abstraction | Gen monad | Strategies | Strategy trait | Generator[T] |
| Shrinking | Explicit function | Byte buffer | ValueTree | Hybrid (configurable) |
| Contracts integration | None | None | None | Native |
| Doctest support | No | No | Yes | Yes |
| Fuzz integration | No | No | Via proptest | Native |
| Mutation testing | No | No | No | Native |
| Coverage tracking | No | Yes | Yes | Yes |
| Stateful testing | Limited | Excellent | Good | State machines |
| Type-directed gen | Arbitrary class | Strategies | Strategy trait | Generable trait |
