# ARIA-PD-018: Documentation System Design

**Decision ID**: ARIA-PD-018
**Status**: Approved
**Date**: 2026-01-15
**Author**: QUILL (Product Decision Agent)
**Research Inputs**:
- ARIA-M19-03: Documentation Generation System (SCRIBE-II)

---

## Executive Summary

This document defines Aria's documentation system, establishing concrete syntax, semantic extraction rules, output formats, and tooling integration. The system is designed around Aria's unique language features: contracts, effects, property-based testing, and verified examples.

**Final Decisions**:
- **Doc comment syntax**: `##` for line docs, `###...###` for block docs
- **Tag system**: 12 primary tags with automatic extraction for contracts/effects
- **Cross-referencing**: Backtick-wrapped links with dot-notation (`[`Type.method`]`)
- **Output formats**: HTML (primary), JSON (tooling), Markdown (README), man pages (CLI)
- **Playground**: Built-in WASM-based interactive examples
- **Doctests**: Verified examples with `aria test --doc`
- **Changelog**: Auto-generated from structured commits

---

## 1. Doc Comment Syntax

### 1.1 Line Doc Comments (`##`)

Line doc comments use double-hash prefix. Consecutive lines are concatenated into a single documentation block.

**Decision**: Use `##` for line doc comments.

**Rationale**: Consistent with Aria's `#` line comment syntax while clearly distinguishing documentation from regular comments.

```aria
## Calculates the factorial of a non-negative integer.
## Returns 1 for n=0 by mathematical convention.
fn factorial(n: Int) -> Int
  # Implementation details (regular comment, not in docs)
  match n
    0, 1 => 1
    _    => n * factorial(n - 1)
  end
end
```

### 1.2 Block Doc Comments (`###...###`)

Block doc comments use triple-hash delimiters for longer documentation.

**Decision**: Use `###` to open and close block doc comments.

**Rationale**: Mirrors the existing `###` block comment syntax from GRAMMAR.md, repurposed for documentation.

```aria
###
Performs a binary search on a sorted array.

This function efficiently locates an element in O(log n) time by
repeatedly halving the search space. The array must be sorted in
ascending order for correct results.

## Algorithm

1. Compare target with middle element
2. If equal, return index
3. If target < middle, search left half
4. If target > middle, search right half
5. Repeat until found or range exhausted
###
fn binary_search[T: Ord](arr: Array[T], target: T) -> Int?
  # ...
end
```

### 1.3 Inner Documentation (`##!` and `###!`)

Inner doc comments describe the containing item (module, struct, etc.).

**Decision**: Append `!` to doc comment markers for inner documentation.

```aria
###!
# Module: std::collections

Provides core collection types for Aria programs.

This module includes:
- `Array[T]` - Dynamic arrays with amortized O(1) append
- `Map[K, V]` - Hash-based key-value storage
- `Set[T]` - Unique element collections

## Usage

```aria
import std::collections::{Array, Map}

let names = Array.new()
names.push("Alice")
```
###

module std::collections
  # ... module contents
end
```

### 1.4 Complete Syntax Grammar

```ebnf
doc_comment     = outer_doc | inner_doc ;
outer_doc       = line_doc_outer+ | block_doc_outer ;
inner_doc       = line_doc_inner+ | block_doc_inner ;

line_doc_outer  = '##' { any_char - newline } newline ;
line_doc_inner  = '##!' { any_char - newline } newline ;
block_doc_outer = '###' newline { any_char - '###' } '###' ;
block_doc_inner = '###!' newline { any_char - '###' } '###' ;
```

---

## 2. Documentation Tags

### 2.1 Primary Tags

**Decision**: Support 12 primary documentation tags. Tags use `@` prefix and follow a consistent format.

| Tag | Syntax | Purpose |
|-----|--------|---------|
| `@param` | `@param name: Type - Description` | Document a parameter |
| `@returns` | `@returns Description` | Document return value |
| `@raises` | `@raises ErrorType - Condition` | Document exceptions |
| `@effects` | `@effects Effect1, Effect2` | Document effects (auto-extracted) |
| `@requires` | `@requires condition` | Document precondition (auto-extracted) |
| `@ensures` | `@ensures condition` | Document postcondition (auto-extracted) |
| `@example` | `@example expr == result` | Inline example |
| `@see` | `@see TypeName.method` | Cross-reference |
| `@since` | `@since version` | Version introduced |
| `@deprecated` | `@deprecated reason` | Deprecation notice |
| `@complexity` | `@complexity O(...)` | Time/space complexity |
| `@pure` | `@pure` | Mark as pure function |

### 2.2 Automatic Extraction

The documentation generator automatically extracts and displays:

1. **Contract clauses** - `requires`, `ensures`, `invariant` from function definitions
2. **Effect signatures** - From inferred or annotated effect types
3. **Examples blocks** - From `examples` blocks in function definitions
4. **Property blocks** - From `property` blocks for property-based testing
5. **Type signatures** - From function, struct, and trait definitions

**Decision**: Automatically extracted elements take precedence over manual tags for the same information. Manual tags can supplement but not override.

### 2.3 Tag Format Specification

**@param syntax**:
```aria
## @param name: Type - Description of the parameter
## @param timeout: Duration - Maximum wait time before timeout (default: 30.seconds)
```

**@returns syntax**:
```aria
## @returns The computed factorial value, always >= 1
```

**@raises syntax**:
```aria
## @raises NetworkError - When connection to server fails
## @raises ParseError - When response body is malformed JSON
```

**@deprecated syntax**:
```aria
## @deprecated since: 1.1.0
## @deprecated reason: Use `new_function` instead
## @deprecated removal: 2.0.0
```

### 2.4 Extended Tags (Optional)

| Tag | Syntax | Purpose |
|-----|--------|---------|
| `@safety` | `@safety Description` | Safety considerations |
| `@todo` | `@todo Description` | Future work marker |
| `@author` | `@author Name` | Author attribution |
| `@internal` | `@internal` | Mark as internal API |
| `@experimental` | `@experimental` | Mark as experimental |
| `@category` | `@category Name` | Categorization for grouping |

---

## 3. Cross-Referencing Syntax

### 3.1 Link Syntax

**Decision**: Use backtick-wrapped identifiers for cross-references: `[`identifier`]`

This syntax is clear, unambiguous, and compatible with Markdown while allowing precise linking.

```aria
## Returns the first element of the array.
##
## Similar to [`last`] but returns the first element.
## See [`Array.get`] for index-based access.
## For safe access, consider [`first?`] which returns [`Option[T]`].
##
## @see Array.last
## @see Option
fn first[T](self: Array[T]) -> T
  requires self.length > 0
  self[0]
end
```

### 3.2 Link Resolution Rules

| Syntax | Resolution Target |
|--------|-------------------|
| `[`name`]` | Resolves in current scope, then module, then imports |
| `[`Module.name`]` | Qualified path within current crate |
| `[`module::Type`]` | Full module path |
| `[`Type.method`]` | Method on a type |
| `[`Type.CONSTANT`]` | Constant on a type |
| `[`Self`]` | Current type (in impl/trait blocks) |
| `[`Self.method`]` | Method on current type |
| `[`crate::path`]` | Path in current crate |
| `[`external::Type`]` | External crate reference |

### 3.3 Link Validation

**Decision**: All links are validated at documentation generation time. Broken links produce warnings by default, errors with `--strict` flag.

```bash
# Validate all cross-references
aria doc --check-links

# Strict mode: fail on broken links
aria doc --strict
```

---

## 4. Output Formats

### 4.1 Supported Formats

**Decision**: Support four primary output formats with HTML as the default.

| Format | Command | Primary Use Case |
|--------|---------|------------------|
| **HTML** | `aria doc --format html` | Web documentation (default) |
| **JSON** | `aria doc --format json` | Tooling integration, LSP |
| **Markdown** | `aria doc --format md` | README, GitHub integration |
| **Man Pages** | `aria doc --format man` | CLI tool documentation |

### 4.2 HTML Output

HTML is the primary documentation format with these features:

- **Search**: Client-side fuzzy search using generated JSON index
- **Theming**: Light/dark themes with custom CSS support
- **Navigation**: Hierarchical sidebar with module tree
- **Responsive**: Mobile-friendly layout
- **Playground**: Integrated WASM interpreter for examples

**Theme Configuration** (`aria.toml`):
```toml
[docs.theme]
name = "aria-light"  # "aria-light", "aria-dark", or "custom"
custom_css = "docs/custom.css"
logo = "docs/logo.svg"
favicon = "docs/favicon.ico"
highlight = "github"  # Syntax highlighting theme
```

### 4.3 JSON Output Schema

The JSON format enables tooling integration (LSP, IDE plugins, external doc generators).

```json
{
  "$schema": "https://aria-lang.org/schemas/doc-v1.json",
  "version": "1.0.0",
  "crate": {
    "name": "my_library",
    "version": "1.2.0",
    "modules": [
      {
        "name": "math",
        "path": "std::math",
        "doc": "Mathematical functions and utilities.",
        "items": [
          {
            "kind": "function",
            "name": "factorial",
            "visibility": "pub",
            "signature": "fn factorial(n: Int) -> Int",
            "doc": "Calculates the factorial of a non-negative integer.",
            "params": [
              {
                "name": "n",
                "type": "Int",
                "doc": "The number to compute factorial of"
              }
            ],
            "returns": {
              "type": "Int",
              "doc": "The factorial value (n!)"
            },
            "contracts": {
              "requires": [
                {"expr": "n >= 0", "message": "factorial requires non-negative input"}
              ],
              "ensures": [
                {"expr": "result >= 1", "message": null}
              ]
            },
            "effects": [],
            "examples": [
              "factorial(0) == 1",
              "factorial(5) == 120"
            ],
            "properties": [
              {
                "name": "result is always positive",
                "expr": "forall n: Int where n >= 0 -> factorial(n) > 0"
              }
            ],
            "since": "0.1.0",
            "deprecated": null,
            "complexity": "O(n) time, O(1) space"
          }
        ]
      }
    ]
  }
}
```

### 4.4 Man Page Output

For CLI tools, generate Unix man pages:

```bash
aria doc --format man --section 1  # User commands
aria doc --format man --section 3  # Library functions
```

---

## 5. Playground Integration

### 5.1 Architecture

**Decision**: Use a WASM-based Aria interpreter for in-browser code execution.

```
+------------------------------------------+
| Interactive Example                       |
+------------------------------------------+
| fn factorial(n: Int) -> Int              |
|   requires n >= 0                        |
|   match n                                |
|     0, 1 => 1                            |
|     _    => n * factorial(n - 1)         |
|   end                                    |
| end                                      |
+------------------------------------------+
| > factorial(5)                           |
| => 120                                   |
|                                          |
| > factorial(-1)                          |
| ! ContractViolation: n >= 0              |
+------------------------------------------+
| [Run] [Reset] [Share] [Copy]             |
+------------------------------------------+
```

### 5.2 Playground Features

| Feature | Description |
|---------|-------------|
| **Run** | Execute code in WASM sandbox |
| **Reset** | Restore original example code |
| **Share** | Generate permalink to example |
| **Copy** | Copy code to clipboard |
| **Contract Display** | Show contract violations clearly |
| **Effect Handling** | Simulate effect handlers |

### 5.3 Security Model

- **Sandboxed execution**: No file system or network access
- **Resource limits**: 5 second timeout, 16MB memory
- **No persistent state**: Each run starts fresh

### 5.4 REPL Integration

The `aria repl` command supports documentation queries:

```
aria> :doc factorial
## factorial(n: Int) -> Int

Calculates the factorial of a non-negative integer.

Contracts:
  requires n >= 0
  ensures result >= 1

Examples:
  factorial(0) == 1
  factorial(5) == 120

aria> :source factorial
fn factorial(n: Int) -> Int
  requires n >= 0
  ensures result >= 1
  match n
    0, 1 => 1
    _    => n * factorial(n - 1)
  end
end

aria> :search facto
Did you mean:
  1. factorial (std::math) - Calculates the factorial of a non-negative integer.
  2. factorize (std::math) - Prime factorization of an integer.
```

---

## 6. Effect and Contract Documentation Display

### 6.1 Effect Signature Display

**Decision**: Effects are displayed as part of the function signature with detailed handling examples.

```aria
## Fetches user data from the API.
##
## @effects IO, Async
## @raises NetworkError - When the network is unavailable
## @raises ParseError - When response is malformed
fn fetch_user(id: UserId) -> {IO, Async} User
  http.get("/users/#{id}").await?.parse_json()?
end
```

**Generated Documentation**:

```markdown
### fetch_user

```aria
fn fetch_user(id: UserId) -> {IO, Async} User
```

Fetches user data from the API.

#### Effects

This function performs the following effects:

| Effect | Description |
|--------|-------------|
| `IO` | Performs network I/O |
| `Async` | Suspends execution asynchronously |

#### Handling Effects

```aria
# Using handle block
result = handle
  on IO.Error(e) => log_error(e); retry()
  on Async.Timeout => default_user()
  fetch_user(user_id)
end

# Using effect handlers
with io_handler, async_runtime
  user = fetch_user(user_id)
end
```

#### Raises

| Exception | Condition |
|-----------|-----------|
| `NetworkError` | When the network is unavailable |
| `ParseError` | When response is malformed |
```

### 6.2 Contract Display

**Decision**: Contracts are displayed in a structured table with tier classification.

```aria
## Performs binary search on a sorted array.
##
## @complexity O(log n)
fn binary_search[T: Ord](arr: Array[T], target: T) -> Int?
  requires arr.length > 0     : "array cannot be empty"
  requires arr.sorted?        : "array must be sorted"
  ensures result.nil? or arr[result.unwrap] == target
  # ...
end
```

**Generated Documentation**:

```markdown
### Contracts

| Type | Expression | Tier | Message |
|------|------------|------|---------|
| **Precondition** | `arr.length > 0` | Cached | array cannot be empty |
| **Precondition** | `arr.sorted?` | Cached | array must be sorted |
| **Postcondition** | `result.nil? or arr[result.unwrap] == target` | Runtime | - |

**Contract Tiers:**
- **Static** (Tier 1): Verified at compile time, zero runtime cost
- **Cached** (Tier 2): Verified with memoization for pure expressions
- **Runtime** (Tier 3): Checked at runtime (can be disabled in production)

See [Contract System Documentation](/docs/contracts) for details.
```

### 6.3 Effect Handler Documentation

```aria
## A retry handler that automatically retries failed IO operations.
##
## @param max_retries: Int - Maximum retry attempts (default: 3)
## @param delay: Duration - Delay between retries (default: 1.second)
## @handles IO.Error - Retries the operation on IO errors
effect_handler retry_io(max_retries: 3, delay: 1.second)
  on IO.Error(e) ->
    if retries < max_retries
      sleep(delay)
      resume  # Retry the operation
    else
      raise e  # Propagate after max retries
    end
  end
end
```

**Generated Documentation**:

```markdown
### retry_io (Effect Handler)

A retry handler that automatically retries failed IO operations.

#### Parameters

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `max_retries` | `Int` | `3` | Maximum retry attempts |
| `delay` | `Duration` | `1.second` | Delay between retries |

#### Handled Effects

| Effect | Behavior |
|--------|----------|
| `IO.Error` | Retries up to `max_retries` times with `delay` between attempts |

#### Usage

```aria
with retry_io(max_retries: 5, delay: 2.seconds)
  data = fetch_data(url)
end
```
```

---

## 7. Verified Examples (Doctests)

### 7.1 Example Block Integration

**Decision**: `examples` blocks in function definitions are extracted as verified documentation.

```aria
fn add(a: Int, b: Int) -> Int
  ensures result == a + b
  a + b

  examples
    add(2, 3) == 5
    add(-1, 1) == 0
    add(0, 0) == 0
  end
end
```

### 7.2 Doctest Execution

```bash
# Run all documentation examples as tests
aria test --doc

# Output:
Running doc tests for my_library...
  std::math::factorial ... ok
    example 1: factorial(0) == 1 ... ok
    example 2: factorial(5) == 120 ... ok
  std::string::split ... ok
    example 1: "a,b,c".split(",") == ["a", "b", "c"] ... ok

Doc tests: 4 passed, 0 failed
```

### 7.3 Documentation with Verification Status

**Decision**: Generated documentation displays verification status for all examples.

```markdown
### Examples

All examples are verified by `aria test --doc`.

| Example | Status |
|---------|--------|
| `add(2, 3) == 5` | Passed |
| `add(-1, 1) == 0` | Passed |
| `add(0, 0) == 0` | Passed |

*Last verified: 2026-01-15 14:30:00 UTC*
```

### 7.4 Property Documentation

```aria
fn sort[T: Ord](arr: Array[T]) -> Array[T]
  ensures result.sorted?
  ensures result.permutation_of?(arr)

  property "result is sorted"
    forall arr: Array[Int]
      sort(arr).sorted?
    end
  end

  property "result is permutation"
    forall arr: Array[Int]
      sort(arr).permutation_of?(arr)
    end
  end
  # ...
end
```

**Generated Documentation**:

```markdown
### Properties

These properties are verified through property-based testing:

| Property | Description | Expression |
|----------|-------------|------------|
| sorted | Result is always sorted | `forall arr: Array[Int] -> sort(arr).sorted?` |
| permutation | Result is a permutation of input | `forall arr: Array[Int] -> sort(arr).permutation_of?(arr)` |

Run `aria test --property` to verify these properties.
```

---

## 8. Changelog Generation

### 8.1 Structured Commit Format

**Decision**: Use conventional commit format for automatic changelog generation.

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

**Supported Types**:

| Type | Changelog Section |
|------|-------------------|
| `feat` | Added |
| `fix` | Fixed |
| `BREAKING` | Breaking Changes |
| `deprecate` | Deprecated |
| `perf` | Performance |
| `docs` | Documentation |
| `refactor` | (Hidden by default) |
| `test` | (Hidden by default) |
| `chore` | (Hidden by default) |

### 8.2 Example Commits

```
feat(math): add factorial function
fix(array): correct off-by-one error in slice
BREAKING(string): rename `split` to `split_by`
deprecate(io): mark old_read as deprecated
```

### 8.3 Generated Changelog

```markdown
# Changelog

All notable changes to this project will be documented in this file.

## [1.2.0] - 2026-01-15

### Added
- `factorial` function in `std::math` module ([#123](https://github.com/aria-lang/aria/pull/123))

### Fixed
- Off-by-one error in `Array.slice` ([#125](https://github.com/aria-lang/aria/pull/125))

### Breaking Changes
- `String.split` renamed to `String.split_by` ([#127](https://github.com/aria-lang/aria/pull/127))
  - Migration: Replace all `str.split(sep)` with `str.split_by(sep)`

### Deprecated
- `IO.old_read` - Use `IO.read` instead

## [1.1.0] - 2026-01-01
...
```

### 8.4 Changelog Commands

```bash
# Generate changelog for current version
aria changelog

# Generate changelog between versions
aria changelog --from v1.0.0 --to v1.2.0

# Generate changelog and update CHANGELOG.md
aria changelog --write

# Preview without writing
aria changelog --dry-run
```

### 8.5 API Diff Generation

```bash
# Compare API between versions
aria doc --diff v1.1.0 v1.2.0

# Output:
+ fn factorial(n: Int) -> Int           # Added
~ fn slice(start, end) -> Array[T]      # Modified
- fn old_split(sep) -> Array[String]    # Removed
! fn deprecated_fn()                    # Deprecated
```

---

## 9. Complete Documentation Example

### 9.1 Module-Level Documentation

```aria
###!
# Module: std::math

Mathematical functions and utilities.

This module provides common mathematical operations including:
- Basic arithmetic functions (`factorial`, `gcd`, `lcm`)
- Trigonometric functions (`sin`, `cos`, `tan`)
- Statistical functions (`mean`, `median`, `std_dev`)

## Usage

```aria
import std::math::{factorial, sqrt, sin, cos}

result = factorial(5)  # 120
root = sqrt(16.0)      # 4.0
```

## See Also

- [`std::complex`] - Complex number operations
- [`std::random`] - Random number generation
###

module std::math
  # ... module contents
end
```

### 9.2 Function Documentation

```aria
## Calculates the factorial of a non-negative integer.
##
## The factorial of n (written as n!) is the product of all positive
## integers less than or equal to n:
##
## ```
## n! = n * (n-1) * (n-2) * ... * 2 * 1
## ```
##
## By definition, 0! = 1.
##
## @param n: Int - The number to compute factorial of (must be >= 0)
## @returns The factorial value (n!)
## @raises OverflowError - When result exceeds Int.MAX
## @complexity O(n) time, O(1) space (tail recursive)
## @pure
## @since 0.1.0
## @see gamma_function - For non-integer factorials
## @see permutations - Uses factorial for counting
pub fn factorial(n: Int) -> Int
  requires n >= 0 : "factorial requires non-negative input"
  ensures result >= 1
  ensures n <= 1 implies result == 1
  ensures n > 1 implies result == n * factorial(n - 1)

  match n
    0, 1 => 1
    _    => n * factorial(n - 1)
  end

  examples
    factorial(0) == 1
    factorial(1) == 1
    factorial(5) == 120
    factorial(10) == 3628800
  end

  property "result is always positive"
    forall n: Int where 0 <= n <= 20
      factorial(n) > 0
    end
  end

  property "factorial grows monotonically"
    forall n: Int where 1 <= n <= 19
      factorial(n + 1) > factorial(n)
    end
  end
end
```

### 9.3 Struct Documentation

```aria
## A two-dimensional point with x and y coordinates.
##
## Points are immutable and support basic vector operations.
##
## @since 0.1.0
## @category Geometry
pub struct Point
  ## The x-coordinate of the point.
  x: Float

  ## The y-coordinate of the point.
  y: Float

  invariant x.finite? and y.finite? : "coordinates must be finite"

  ## Creates a new point at the origin (0, 0).
  ##
  ## @returns A point at coordinates (0, 0)
  pub fn origin() -> Point
    Point { x: 0.0, y: 0.0 }
  end

  ## Calculates the Euclidean distance from this point to another.
  ##
  ## @param other: Point - The target point
  ## @returns The distance between the two points
  ## @complexity O(1)
  ## @pure
  pub fn distance(self, other: Point) -> Float
    ensures result >= 0.0
    sqrt((self.x - other.x).pow(2) + (self.y - other.y).pow(2))

    examples
      Point.origin().distance(Point { x: 3.0, y: 4.0 }) == 5.0
    end
  end
end
```

### 9.4 Trait Documentation

```aria
## A type that can be compared for ordering.
##
## Types implementing `Ord` support comparison operations (`<`, `>`, `<=`, `>=`)
## and can be used with sorting functions.
##
## ## Contract Guarantees
##
## Implementations must satisfy:
## - **Antisymmetry**: If a <= b and b <= a, then a == b
## - **Transitivity**: If a <= b and b <= c, then a <= c
## - **Totality**: Either a <= b or b <= a (or both)
##
## @since 0.1.0
pub trait Ord
  ## Compares self with other and returns an ordering.
  ##
  ## @param other: Self - The value to compare against
  ## @returns `Less`, `Equal`, or `Greater`
  fn compare(self, other: Self) -> Ordering
    ensures result == Less implies other.compare(self) == Greater
    ensures result == Equal implies other.compare(self) == Equal
  end

  ## Returns true if self is less than other.
  ##
  ## @param other: Self - The value to compare against
  ## @returns `true` if self < other
  fn lt(self, other: Self) -> Bool
    self.compare(other) == Less
  end

  ## Returns true if self is greater than other.
  ##
  ## @param other: Self - The value to compare against
  ## @returns `true` if self > other
  fn gt(self, other: Self) -> Bool
    self.compare(other) == Greater
  end
end
```

### 9.5 Effect Handler Documentation

```aria
## Provides automatic retry capability for IO operations.
##
## This handler intercepts `IO.Error` effects and automatically retries
## the failing operation with exponential backoff.
##
## @param max_attempts: Int - Maximum number of attempts (default: 3)
## @param base_delay: Duration - Initial delay between retries (default: 100.ms)
## @param max_delay: Duration - Maximum delay cap (default: 10.seconds)
## @handles IO.Error - Retries with exponential backoff
## @since 0.2.0
##
## ## Example
##
## ```aria
## with retry_with_backoff(max_attempts: 5)
##   data = fetch_remote_data(url)
## end
## ```
##
## ## Backoff Strategy
##
## Delay doubles after each failure: 100ms -> 200ms -> 400ms -> ...
## Capped at `max_delay` to prevent excessive waits.
pub effect_handler retry_with_backoff(
  max_attempts: 3,
  base_delay: 100.ms,
  max_delay: 10.seconds
)
  on IO.Error(e) ->
    if attempts < max_attempts
      delay = min(base_delay * (2 ** attempts), max_delay)
      sleep(delay)
      resume
    else
      raise e
    end
  end

  examples
    # Succeeds after 2 retries
    with retry_with_backoff()
      simulate_flaky_operation()
    end
  end
end
```

---

## 10. CLI Reference

### 10.1 Documentation Commands

```bash
# Generate documentation (HTML by default)
aria doc

# Specify output format
aria doc --format html     # Web documentation
aria doc --format json     # JSON for tooling
aria doc --format md       # Markdown
aria doc --format man      # Man pages

# Output directory
aria doc --output ./docs

# Open in browser after generation
aria doc --open

# Include private items
aria doc --document-private
```

### 10.2 Documentation Server

```bash
# Start local documentation server
aria doc --serve

# Custom port
aria doc --serve --port 8080

# Watch mode (auto-reload on changes)
aria doc --serve --watch
```

### 10.3 Documentation Testing

```bash
# Run all doctests
aria test --doc

# Check documentation coverage
aria doc --coverage

# Validate cross-references
aria doc --check-links

# Strict mode (fail on warnings)
aria doc --strict
```

### 10.4 Version Documentation

```bash
# Compare API between versions
aria doc --diff v1.0.0 v1.1.0

# Generate docs for specific version
aria doc --version v1.0.0
```

---

## 11. Configuration

### 11.1 Project Configuration (`aria.toml`)

```toml
[docs]
# Output directory (default: target/doc)
output = "target/doc"

# Documentation title
title = "My Library Documentation"

# Features to document
features = ["std", "async"]

# Minimum coverage threshold (percentage)
min_coverage = 90

# Items that must be documented
require_doc = ["pub fn", "pub struct", "pub trait"]

# Show warnings for missing elements
warn_missing_examples = true
warn_missing_contracts = false

[docs.theme]
name = "aria-light"  # or "aria-dark", "custom"
custom_css = "docs/custom.css"
logo = "docs/logo.svg"
favicon = "docs/favicon.ico"
highlight = "github"

[docs.search]
enable = true
fuzzy = true

[docs.external]
# Links to external documentation
std = "https://aria-lang.org/std"
serde = "https://docs.serde.rs"

[docs.versions]
# Available documentation versions
list = ["1.2.0", "1.1.0", "1.0.0"]
default = "1.2.0"
```

---

## 12. IDE Integration

### 12.1 Hover Documentation

When hovering over a symbol in the IDE:

```
+------------------------------------------+
| fn factorial(n: Int) -> Int              |
| ---------------------------------------- |
| Calculates the factorial of a non-       |
| negative integer.                        |
|                                          |
| **Contracts:**                           |
| - requires n >= 0                        |
| - ensures result >= 1                    |
|                                          |
| **Examples:**                            |
| factorial(5) == 120                      |
|                                          |
| [View Full Documentation]                |
+------------------------------------------+
```

### 12.2 LSP Documentation Protocol

The LSP server provides extended hover information:

```typescript
interface AriaHoverResult extends Hover {
  contents: MarkupContent;
  contracts?: {
    requires: ContractClause[];
    ensures: ContractClause[];
    invariants: ContractClause[];
  };
  effects?: string[];
  examples?: string[];
  deprecated?: {
    since: string;
    message: string;
    replacement?: string;
  };
}
```

---

## 13. Implementation Phases

### Phase 1: MVP
- `##` doc comment extraction
- Basic HTML generation
- Contract and effect extraction
- Simple search functionality

### Phase 2: Enhanced
- Cross-reference resolution (`[`Type.method`]`)
- JSON output format
- LSP hover integration
- Doctest runner (`aria test --doc`)

### Phase 3: Advanced
- Interactive playground (WASM)
- Versioned documentation
- Changelog generation
- API diff tool

### Phase 4: Polish
- Theme system with customization
- Fuzzy search
- Coverage reporting
- PDF output format
- Man page generation

---

## Appendix A: Quick Reference Card

### Doc Comment Syntax

| Syntax | Purpose |
|--------|---------|
| `##` | Line doc comment |
| `###...###` | Block doc comment |
| `##!` | Inner line doc |
| `###!...###` | Inner block doc |

### Common Tags

| Tag | Usage |
|-----|-------|
| `@param name: Type - Desc` | Parameter documentation |
| `@returns Desc` | Return value documentation |
| `@raises Error - When` | Exception documentation |
| `@see Type.method` | Cross-reference |
| `@since x.y.z` | Version introduced |
| `@deprecated reason` | Deprecation notice |
| `@complexity O(...)` | Complexity annotation |
| `@pure` | Pure function marker |

### Cross-Reference Syntax

| Syntax | Target |
|--------|--------|
| `[`name`]` | Local scope |
| `[`Type.method`]` | Method on type |
| `[`module::Type`]` | Full path |
| `[`Self`]` | Current type |

### CLI Commands

| Command | Purpose |
|---------|---------|
| `aria doc` | Generate HTML docs |
| `aria doc --serve` | Start doc server |
| `aria test --doc` | Run doctests |
| `aria doc --coverage` | Check coverage |
| `aria doc --check-links` | Validate links |

---

## References

### Research Inputs
- ARIA-M19-03: Documentation Generation System (SCRIBE-II)
- ARIA-M04-04: Tiered Contract System Design
- ARIA-M03-02: Koka Effect System Analysis

### External References
- [Rustdoc Book](https://doc.rust-lang.org/rustdoc/)
- [JSDoc Reference](https://jsdoc.app/)
- [YARD Ruby Documentation](https://yardoc.org/)
- [CommonMark Specification](https://commonmark.org/)
