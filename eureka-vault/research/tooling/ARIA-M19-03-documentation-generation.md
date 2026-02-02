# ARIA-M19-03: Documentation Generation System

**Task ID**: ARIA-M19-03
**Status**: Completed
**Date**: 2026-01-15
**Focus**: Comprehensive Documentation Generation System Design
**Agent**: SCRIBE-II (Research)

---

## Executive Summary

This research designs Aria's documentation generation system, integrating with the language's unique features: contracts, effects, property-based testing, and examples blocks. The system draws from Rustdoc's semantic analysis, JSDoc's flexibility, and Sphinx's cross-referencing while innovating on contract documentation and interactive playgrounds.

Key design decisions:
1. **Doc comment syntax**: `##` for doc comments (consistent with `#` line comments), with structured tags
2. **Contract integration**: Automatic extraction and display of `requires`/`ensures`/`invariant` as documentation
3. **Effect documentation**: Clear effect signatures with handler examples
4. **Interactive playgrounds**: REPL integration for verifiable examples
5. **Changelog generation**: Semantic versioning-aware changelog from structured commit messages

---

## 1. Doc Comment Syntax Design

### 1.1 Syntax Comparison

| Language | Doc Comment | Block Doc | Inline |
|----------|-------------|-----------|--------|
| **Rust** | `///` | `//!` (inner) | N/A |
| **Java/JS** | `/**...*/` | Same | `@tag` |
| **Python** | `"""..."""` (docstring) | Same | `:param:` |
| **Ruby** | `#` + YARD | Same | `@param` |
| **Haskell** | `-- |` | `{- | ... -}` | N/A |
| **Go** | `//` (conventional) | Same | N/A |
| **Zig** | `///` | `//!` | N/A |

### 1.2 Proposed Aria Syntax

Based on Aria's Ruby-inspired design with `#` for line comments:

```aria
## Single-line doc comment (for most uses)
## These are concatenated for multi-line documentation.

###
Block doc comment for longer documentation.
Useful for module-level or complex explanations.
Supports **Markdown** formatting.
###

# Regular comment (not included in docs)
```

**Rationale**:
- `##` distinguishes doc comments from regular `#` comments
- `###` block already exists in GRAMMAR.md for block comments; repurpose for docs
- Consistent with Ruby's YARD where `#` is followed by documentation

### 1.3 Inner vs Outer Documentation

```aria
###!
Module-level documentation (inner doc).
This describes the containing module itself.
###

module MyModule
  ## Function documentation (outer doc)
  ## Describes the function that follows.
  fn my_function()
    # ...
  end
end
```

### 1.4 Complete Syntax Specification

```ebnf
doc_comment     = outer_doc | inner_doc ;
outer_doc       = line_doc_outer | block_doc_outer ;
inner_doc       = line_doc_inner | block_doc_inner ;

line_doc_outer  = '##' { any_char - newline } newline ;
line_doc_inner  = '##!' { any_char - newline } newline ;
block_doc_outer = '###' newline { any_char } '###' ;
block_doc_inner = '###!' newline { any_char } '###' ;
```

---

## 2. Documentation Metadata and Tags

### 2.1 Structured Tags

Drawing from JSDoc, Rustdoc, and YARD:

| Tag | Purpose | Example |
|-----|---------|---------|
| `@param` | Parameter description | `@param name: String - The user's name` |
| `@returns` | Return value description | `@returns The computed result` |
| `@raises` | Exceptions that may be raised | `@raises ValueError - When input is invalid` |
| `@effects` | Effect annotations (auto-extracted) | `@effects IO, Async` |
| `@requires` | Precondition (auto-extracted from contract) | `@requires x > 0` |
| `@ensures` | Postcondition (auto-extracted from contract) | `@ensures result >= 0` |
| `@example` | Inline example | `@example factorial(5) == 120` |
| `@see` | Cross-reference | `@see Array.map` |
| `@since` | Version introduced | `@since 0.2.0` |
| `@deprecated` | Deprecation notice | `@deprecated Use new_function instead` |
| `@safety` | Safety considerations | `@safety This function is thread-safe` |
| `@complexity` | Time/space complexity | `@complexity O(n log n)` |
| `@pure` | Function is pure (no side effects) | `@pure` |
| `@todo` | Future work | `@todo Optimize for large inputs` |

### 2.2 Automatic Extraction

The documentation generator automatically extracts:

1. **Contract clauses** (`requires`, `ensures`, `invariant`)
2. **Effect signatures** from inferred or annotated effects
3. **Examples blocks** from function definitions
4. **Property blocks** from function definitions
5. **Type signatures** from function definitions

```aria
## Calculates the factorial of a non-negative integer.
##
## This function computes n! using tail recursion for efficiency.
##
## @param n: Int - The number to compute factorial of
## @returns The factorial value (n!)
## @complexity O(n) time, O(1) space
## @see gamma_function - For non-integer factorials
fn factorial(n: Int) -> Int
  requires n >= 0 : "factorial requires non-negative input"
  ensures result >= 1
  ensures n == 0 implies result == 1
  ensures n > 0 implies result == n * factorial(n - 1)

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
    forall n: Int where n >= 0
      factorial(n) > 0
    end
  end
end
```

**Generated Documentation Output**:

```markdown
## factorial

```aria
fn factorial(n: Int) -> Int
```

Calculates the factorial of a non-negative integer.

This function computes n! using tail recursion for efficiency.

### Parameters

| Name | Type | Description |
|------|------|-------------|
| `n` | `Int` | The number to compute factorial of |

### Returns

`Int` - The factorial value (n!)

### Contracts

**Preconditions:**
- `n >= 0` - factorial requires non-negative input

**Postconditions:**
- `result >= 1`
- `n == 0 implies result == 1`
- `n > 0 implies result == n * factorial(n - 1)`

### Examples

```aria
factorial(0) == 1
factorial(1) == 1
factorial(5) == 120
factorial(10) == 3628800
```

### Properties

- **result is always positive**: `forall n: Int where n >= 0 -> factorial(n) > 0`

### Complexity

O(n) time, O(1) space

### See Also

- [`gamma_function`](#gamma_function) - For non-integer factorials
```

---

## 3. Documentation Tool Comparison

### 3.1 Feature Matrix

| Feature | Rustdoc | JSDoc | Sphinx | Doxygen | Aria (Proposed) |
|---------|---------|-------|--------|---------|-----------------|
| **Markdown support** | Yes | Limited | reStructuredText | No | Yes |
| **Cross-references** | Excellent | Basic | Excellent | Good | Excellent |
| **Search** | Built-in | Plugin | Built-in | Built-in | Built-in |
| **Versioning** | Via docs.rs | Manual | Manual | Manual | Built-in |
| **Type linking** | Automatic | Annotations | Manual | Automatic | Automatic |
| **Example testing** | `cargo test --doc` | None | doctest | None | Built-in |
| **Contract display** | N/A | N/A | N/A | N/A | **Yes** |
| **Effect display** | N/A | N/A | N/A | N/A | **Yes** |
| **Playground** | play.rust-lang.org | JSFiddle link | None | None | **Built-in** |

### 3.2 Rustdoc Deep Dive

**Strengths**:
- Seamless type linking: `[`Vec`]` auto-resolves
- Doctests as first-class tests
- Intra-doc links: `[`Self::method`]`
- Search with fuzzy matching
- JSON output for external tools

**Adoptable for Aria**:
- Intra-doc link syntax: `[`TypeName.method`]`
- Doctest integration
- JSON documentation format
- Search index generation

### 3.3 JSDoc Deep Dive

**Strengths**:
- Flexible tag system (`@param`, `@returns`, etc.)
- Type annotations in comments (for untyped JS)
- Plugin ecosystem
- Template documentation

**Adoptable for Aria**:
- Tag syntax for unstructured metadata
- Flexible description format

### 3.4 Sphinx Deep Dive

**Strengths**:
- Cross-project documentation
- Internationalization support
- Multiple output formats (HTML, PDF, ePub)
- Directive system for custom content

**Adoptable for Aria**:
- Cross-project linking for packages
- Multi-format output
- Custom directives for Aria-specific content

---

## 4. Effect Documentation

### 4.1 Effect Signature Display

Effects are a first-class part of Aria's type system (from ARIA-M03 research). Documentation must clearly communicate:

1. What effects a function performs
2. How to handle those effects
3. Effect composition patterns

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
### Effects

This function performs the following effects:

| Effect | Description |
|--------|-------------|
| `IO` | Performs network I/O |
| `Async` | Suspends execution asynchronously |

**Handling Effects:**

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
```

### 4.2 Effect Handler Documentation

```aria
## A retry handler that automatically retries failed IO operations.
##
## @param max_retries: Int - Maximum number of retry attempts (default: 3)
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
## retry_io (Effect Handler)

A retry handler that automatically retries failed IO operations.

### Parameters

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `max_retries` | `Int` | `3` | Maximum number of retry attempts |
| `delay` | `Duration` | `1.second` | Delay between retries |

### Handled Effects

| Effect | Behavior |
|--------|----------|
| `IO.Error` | Retries the operation up to `max_retries` times with `delay` between attempts |

### Usage

```aria
with retry_io(max_retries: 5, delay: 2.seconds)
  data = fetch_data(url)
end
```
```

---

## 5. Contract Documentation

### 5.1 Contract Display Strategy

Contracts (`requires`/`ensures`/`invariant`) are central to Aria's Design by Contract system. The documentation system presents them in a structured, readable format.

**Tier Classification Display** (from ARIA-M04-04 research):

```aria
## Performs binary search on a sorted array.
##
## @complexity O(log n)
fn binary_search[T: Ord](arr: Array[T], target: T) -> Int?
  requires arr.length > 0     : "array cannot be empty"         # Tier 2
  requires arr.sorted?        : "array must be sorted"          # Tier 2
  ensures result.nil? or arr[result.unwrap] == target           # Tier 3
  ensures forall i: result.some? implies                        # Tier 3
          not exists j: arr[j] == target and j < result
  # ...
end
```

**Generated Documentation**:

```markdown
### Contracts

| Contract | Expression | Tier | Message |
|----------|------------|------|---------|
| **Precondition** | `arr.length > 0` | Cached | array cannot be empty |
| **Precondition** | `arr.sorted?` | Cached | array must be sorted |
| **Postcondition** | `result.nil? or arr[result.unwrap] == target` | Runtime | - |
| **Postcondition** | `forall i: result.some? implies ...` | Runtime | - |

**Contract Tiers:**
- **Static** (Tier 1): Verified at compile time, zero runtime cost
- **Cached** (Tier 2): Verified with memoization for pure expressions
- **Runtime** (Tier 3): Checked at runtime (can be disabled in production)

See [Contract System Documentation](/docs/contracts) for more details.
```

### 5.2 Verified Examples

Examples blocks become verified documentation:

```aria
fn add(a: Int, b: Int) -> Int
  ensures result == a + b
  a + b

  examples
    add(2, 3) == 5        # Verified at test time
    add(-1, 1) == 0       # Verified at test time
    add(0, 0) == 0        # Verified at test time
  end
end
```

**Generated Documentation with Verification Status**:

```markdown
### Examples

All examples are verified by `aria test --doc`.

| Example | Status |
|---------|--------|
| `add(2, 3) == 5` | Passed |
| `add(-1, 1) == 0` | Passed |
| `add(0, 0) == 0` | Passed |

Last verified: 2026-01-15 14:30:00 UTC
```

### 5.3 Contract Inheritance Documentation

```aria
trait Comparable
  ## Compares self with other.
  ## @returns Ordering (Less, Equal, Greater)
  fn compare(self, other: Self) -> Ordering
    ensures result == Less implies other.compare(self) == Greater
    ensures result == Equal implies other.compare(self) == Equal
  end
end

struct Date
  year: Int
  month: Int
  day: Int

  derive(Comparable)
end
```

**Generated Documentation for Date**:

```markdown
## Date

```aria
struct Date
  year: Int
  month: Int
  day: Int
end
```

### Implemented Traits

#### Comparable

| Method | Inherited Contracts |
|--------|---------------------|
| `compare(self, other: Date) -> Ordering` | `result == Less implies other.compare(self) == Greater`, `result == Equal implies other.compare(self) == Equal` |
```

---

## 6. Interactive Documentation

### 6.1 Playground Integration

Inspired by Rust Playground and Go Playground, Aria documentation includes runnable examples:

**Web Documentation View**:

```
+------------------------------------------+
| fn factorial(n: Int) -> Int              |
|   requires n >= 0                        |
|   # ...                                  |
+------------------------------------------+
|                                          |
| > factorial(5)                           |
| => 120                                   |
|                                          |
| > factorial(0)                           |
| => 1                                     |
|                                          |
| [Run] [Reset] [Share]                    |
+------------------------------------------+
```

### 6.2 REPL Integration

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

aria> :doc Array.map
## Array.map[A, B, E](self, f: (A) -> E B) -> E Array[B]

Transforms each element using the provided function.
...

aria> :source factorial
fn factorial(n: Int) -> Int
  requires n >= 0
  ...
end
```

### 6.3 Documentation Server

```bash
# Start local documentation server
aria doc --serve --port 8080

# Features:
# - Live reload on source changes
# - Search across all modules
# - Interactive examples
# - Contract verification status
```

---

## 7. Cross-Referencing System

### 7.1 Link Syntax

```aria
## Returns the first element of the array.
##
## Similar to [`last`] but returns the first element.
## See [`Array.get`] for index-based access.
## For safe access, consider [`first?`] which returns [`Option[T]`].
##
## @see Array.last
## @see Array.get
## @see Option
fn first[T](self: Array[T]) -> T
  requires self.length > 0
  self[0]
end
```

**Link Resolution Rules**:

| Syntax | Resolution |
|--------|------------|
| `[`TypeName`]` | Resolves to type definition |
| `[`module::TypeName`]` | Fully qualified type |
| `[`TypeName.method`]` | Method on type |
| `[`TypeName.CONSTANT`]` | Constant on type |
| `[`Self`]` | Current type (in impl blocks) |
| `[`Self.method`]` | Method on current type |
| `[`crate::module`]` | Module in current crate |
| `[`external_crate::Type`]` | External crate reference |

### 7.2 Search Index Generation

```json
{
  "index": {
    "factorial": {
      "type": "function",
      "module": "std::math",
      "signature": "fn factorial(n: Int) -> Int",
      "summary": "Calculates the factorial of a non-negative integer.",
      "effects": [],
      "contracts": ["requires n >= 0", "ensures result >= 1"]
    },
    "Array": {
      "type": "struct",
      "module": "std::collections",
      "summary": "A dynamically-sized array type.",
      "methods": ["map", "filter", "reduce", "first", "last"]
    }
  },
  "aliases": {
    "fact": "factorial",
    "List": "Array"
  }
}
```

### 7.3 Fuzzy Search

```
aria> :search facto
Did you mean:
  1. factorial (std::math) - Calculates the factorial of a non-negative integer.
  2. factorize (std::math) - Prime factorization of an integer.
```

---

## 8. Versioning and Changelog

### 8.1 Documentation Versioning

```toml
# aria.toml
[package]
name = "my_library"
version = "1.2.0"

[docs]
versions = ["1.2.0", "1.1.0", "1.0.0"]
default_version = "1.2.0"
deprecation_warnings = true
```

**Generated Documentation Header**:

```
+------------------------------------------+
| my_library Documentation                 |
| Version: [1.2.0 v] | 1.1.0 | 1.0.0      |
+------------------------------------------+
```

### 8.2 Version-Aware Deprecation

```aria
## @deprecated since: 1.1.0
## @deprecated reason: Use `new_function` instead
## @deprecated removal: 2.0.0
fn old_function()
  # ...
end

## @since 1.1.0
fn new_function()
  # ...
end
```

**Generated Documentation**:

```markdown
## old_function

> **Deprecated since 1.1.0**: Use `new_function` instead.
> Scheduled for removal in 2.0.0.

...

## new_function

*Added in version 1.1.0*

...
```

### 8.3 Changelog Generation

**From Structured Commits**:

```
feat(math): add factorial function
fix(array): correct off-by-one error in slice
BREAKING(string): rename `split` to `split_by`
deprecate(io): mark old_read as deprecated
```

**Generated CHANGELOG.md**:

```markdown
# Changelog

## [1.2.0] - 2026-01-15

### Added
- `factorial` function in `std::math` module

### Fixed
- Off-by-one error in `Array.slice`

### Breaking Changes
- `String.split` renamed to `String.split_by`

### Deprecated
- `IO.old_read` - Use `IO.read` instead

## [1.1.0] - 2026-01-01
...
```

### 8.4 API Diff Generation

```bash
# Compare API between versions
aria doc --diff 1.1.0 1.2.0

Output:
+ fn factorial(n: Int) -> Int           # Added
~ fn slice(start, end) -> Array[T]      # Modified (signature changed)
- fn old_split(sep) -> Array[String]    # Removed
! fn deprecated_fn()                    # Deprecated
```

---

## 9. Output Formats

### 9.1 Supported Formats

| Format | Use Case | Command |
|--------|----------|---------|
| **HTML** | Web documentation | `aria doc --format html` |
| **JSON** | Tooling integration | `aria doc --format json` |
| **Markdown** | README, GitHub | `aria doc --format md` |
| **Man pages** | CLI tools | `aria doc --format man` |
| **PDF** | Printable docs | `aria doc --format pdf` |

### 9.2 HTML Theme System

```toml
# aria.toml
[docs.theme]
name = "aria-light"  # or "aria-dark", "custom"
custom_css = "docs/custom.css"
logo = "docs/logo.svg"
favicon = "docs/favicon.ico"
```

### 9.3 JSON Documentation Schema

```json
{
  "$schema": "https://aria-lang.org/schemas/doc-v1.json",
  "crate": {
    "name": "my_library",
    "version": "1.2.0",
    "description": "A utility library",
    "modules": [
      {
        "name": "math",
        "doc": "Mathematical functions",
        "items": [
          {
            "kind": "function",
            "name": "factorial",
            "signature": "fn factorial(n: Int) -> Int",
            "doc": "Calculates the factorial...",
            "params": [
              {"name": "n", "type": "Int", "doc": "The number..."}
            ],
            "returns": {"type": "Int", "doc": "The factorial value"},
            "contracts": {
              "requires": [
                {"expr": "n >= 0", "message": "factorial requires non-negative input"}
              ],
              "ensures": [
                {"expr": "result >= 1", "message": null}
              ]
            },
            "effects": [],
            "examples": ["factorial(5) == 120"],
            "properties": [
              {"name": "result is always positive", "expr": "forall n: Int where n >= 0 -> factorial(n) > 0"}
            ],
            "since": "1.0.0",
            "deprecated": null
          }
        ]
      }
    ]
  }
}
```

---

## 10. Documentation Testing

### 10.1 Doctest Integration

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

### 10.2 Coverage Requirements

```toml
# aria.toml
[docs.coverage]
require_doc = ["pub fn", "pub struct", "pub trait"]
min_coverage = 90  # Percentage of public items that must be documented
warn_missing_examples = true
warn_missing_contracts = false  # Contracts are optional for docs
```

```bash
# Check documentation coverage
aria doc --coverage

Output:
Documentation coverage: 87%
Missing documentation:
  - std::io::read_bytes (function)
  - std::net::TcpStream (struct)

Warning: Coverage below minimum threshold (90%)
```

### 10.3 Link Validation

```bash
# Validate all cross-references
aria doc --check-links

Output:
Checking documentation links...
  Error: Broken link in std::math::factorial:
    - [`gamma_function`] not found
  Warning: External link may be stale:
    - https://example.com/old-docs

Links: 142 valid, 1 broken, 1 warning
```

---

## 11. IDE Integration

### 11.1 Hover Documentation

When hovering over a symbol in the IDE, display:

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

### 11.2 LSP Documentation Protocol

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

### 11.3 Inline Documentation Hints

```aria
fn process(data: Array[Int]) -> Int
  let result = data
    .filter(|x| x > 0)    # [Array[Int]] Filter positive numbers
    .map(|x| x * 2)       # [Array[Int]] Double each value
    .sum()                # [Int] Sum all values
  result
end
```

---

## 12. Aria-Specific Documentation Features

### 12.1 Property Documentation

```aria
## Sorts an array in ascending order.
##
## @property sorted: The result is always sorted
## @property permutation: The result is a permutation of the input
## @property stable: Equal elements maintain their relative order
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
| sorted | The result is always sorted | `forall arr: Array[Int] -> sort(arr).sorted?` |
| permutation | The result is a permutation of the input | `forall arr: Array[Int] -> sort(arr).permutation_of?(arr)` |
| stable | Equal elements maintain their relative order | *(manual verification)* |

Run `aria test --property` to verify these properties.
```

### 12.2 Ownership Documentation

```aria
## Transfers ownership of the value.
##
## @ownership moves: self
## @ownership borrows: none
## @ownership returns: owned
fn take[T](self: Box[T]) -> T
  *self
end

## Borrows the value immutably.
##
## @ownership borrows: self (immutable)
## @ownership returns: reference
fn borrow[T](self: &Box[T]) -> &T
  &*self
end
```

### 12.3 FFI Documentation

```aria
## Calls the C `strlen` function.
##
## @ffi "libc"
## @safety unsafe - Requires valid null-terminated string
## @abi C
extern C fn strlen(s: *const c_char) -> c_size_t

## Safe wrapper around C strlen.
##
## @safety safe - Validates input before FFI call
## @see strlen (FFI)
fn safe_strlen(s: String) -> Int
  unsafe { strlen(s.as_c_str()) as Int }
end
```

---

## 13. Implementation Architecture

### 13.1 Documentation Pipeline

```
Source Files (.aria)
        |
        v
  +-------------+
  | AST Parser  |
  +-------------+
        |
        v
  +-------------+
  | Doc Extract |  <- Extract ## comments, contracts, examples
  +-------------+
        |
        v
  +-------------+
  | Link Resolve|  <- Resolve [`Type.method`] references
  +-------------+
        |
        v
  +-------------+
  | Markdown    |  <- Parse Markdown in doc comments
  +-------------+
        |
        v
  +-------------+
  | Generator   |  <- HTML, JSON, MD, PDF output
  +-------------+
        |
        v
  Output Files
```

### 13.2 Crate Structure

```
aria-doc/
  src/
    lib.rs            # Main library
    extract.rs        # Doc comment extraction
    parse.rs          # Markdown parsing
    resolve.rs        # Link resolution
    render/
      html.rs         # HTML renderer
      json.rs         # JSON renderer
      markdown.rs     # Markdown renderer
    search.rs         # Search index generation
    test.rs           # Doctest runner
    server.rs         # Documentation server
```

### 13.3 Configuration

```toml
# aria.toml
[docs]
# Output directory
output = "target/doc"

# Documentation title
title = "My Library Documentation"

# Features to document (default: all)
features = ["std", "async"]

# External documentation links
[docs.external]
"std" = "https://aria-lang.org/std"
"serde" = "https://docs.serde.rs"

# Theme configuration
[docs.theme]
name = "aria-light"
highlight = "github"

# Search configuration
[docs.search]
enable = true
fuzzy = true
```

---

## 14. Recommendations

### 14.1 Priority Implementation Order

1. **Phase 1 (MVP)**:
   - `##` doc comment extraction
   - Basic HTML generation
   - Contract extraction and display
   - Simple search

2. **Phase 2 (Enhanced)**:
   - Cross-reference resolution
   - JSON output format
   - LSP hover integration
   - Doctest runner

3. **Phase 3 (Advanced)**:
   - Interactive playground
   - Versioned documentation
   - Changelog generation
   - PDF output

4. **Phase 4 (Polish)**:
   - Theme system
   - Fuzzy search
   - Coverage reporting
   - Link validation

### 14.2 Key Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Doc comment syntax | `##` line, `###` block | Consistent with `#` comments |
| Markdown flavor | CommonMark + extensions | Wide adoption, well-specified |
| Contract display | Automatic extraction | Contracts are documentation |
| Effect display | Automatic extraction | Effects are part of signature |
| Search | Client-side with JSON index | No server required |
| Playground | WASM-based interpreter | Works offline, secure |

### 14.3 Unique Aria Features

1. **Contract-First Documentation**: Contracts (`requires`/`ensures`) are extracted and displayed prominently, serving as both specification and documentation.

2. **Effect Signatures**: Effect types are displayed alongside function signatures, showing what side effects a function may perform.

3. **Verified Examples**: Examples blocks are run as tests, with verification status shown in documentation.

4. **Property Display**: Property-based testing properties are documented as formal specifications.

5. **Tier Classification**: Contract verification tiers (Static/Cached/Runtime) are shown to help users understand performance implications.

---

## 15. References

### Academic Sources

1. [Literate Programming](https://www-cs-faculty.stanford.edu/~knuth/lp.html) - Donald Knuth
2. [Design by Contract](https://www.eiffel.com/values/design-by-contract/) - Bertrand Meyer
3. [How to Write Doc Comments for the Javadoc Tool](https://www.oracle.com/technical-resources/articles/java/javadoc-tool.html) - Oracle

### Tool Documentation

1. [Rustdoc Book](https://doc.rust-lang.org/rustdoc/)
2. [JSDoc Reference](https://jsdoc.app/)
3. [Sphinx Documentation](https://www.sphinx-doc.org/)
4. [YARD Ruby Documentation](https://yardoc.org/)
5. [Haddock (Haskell)](https://haskell-haddock.readthedocs.io/)

### Prior Aria Research

1. ARIA-M04-04: Tiered Contract System Design
2. ARIA-M03-02: Koka Effect System Analysis
3. ARIA-M18-02: Error Message Design Research
4. ARIA-M19-02: Syntax Pain Points Analysis

---

## Appendix A: Complete Tag Reference

| Tag | Syntax | Description |
|-----|--------|-------------|
| `@param` | `@param name: Type - Description` | Document a parameter |
| `@returns` | `@returns Description` | Document return value |
| `@raises` | `@raises ErrorType - When condition` | Document exceptions |
| `@effects` | `@effects Effect1, Effect2` | Document effects (auto-extracted) |
| `@requires` | `@requires condition` | Document precondition (auto-extracted) |
| `@ensures` | `@ensures condition` | Document postcondition (auto-extracted) |
| `@invariant` | `@invariant condition` | Document invariant (auto-extracted) |
| `@example` | `@example expression == result` | Inline example |
| `@property` | `@property name: Description` | Document a property |
| `@see` | `@see TypeName.method` | Cross-reference |
| `@since` | `@since version` | Version introduced |
| `@deprecated` | `@deprecated reason` | Deprecation notice |
| `@safety` | `@safety Description` | Safety considerations |
| `@complexity` | `@complexity O(...)` | Time/space complexity |
| `@pure` | `@pure` | Mark as pure function |
| `@todo` | `@todo Description` | Future work |
| `@author` | `@author Name` | Author attribution |
| `@license` | `@license Type` | License information |
| `@version` | `@version x.y.z` | Item version |
| `@category` | `@category Name` | Categorization |
| `@internal` | `@internal` | Mark as internal (hidden from public docs) |
| `@experimental` | `@experimental` | Mark as experimental |

---

## Appendix B: Example Documentation Template

```aria
###!
# Module: std::math

Mathematical functions and utilities.

This module provides common mathematical operations including:
- Basic arithmetic functions
- Trigonometric functions
- Statistical functions

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

---

## Appendix C: CLI Reference

```bash
# Generate documentation
aria doc                          # Generate HTML docs
aria doc --format json            # Generate JSON docs
aria doc --format md              # Generate Markdown docs
aria doc --open                   # Generate and open in browser

# Documentation server
aria doc --serve                  # Start local doc server
aria doc --serve --port 8080      # Custom port
aria doc --serve --watch          # Auto-reload on changes

# Documentation testing
aria test --doc                   # Run doctests
aria doc --coverage               # Check doc coverage
aria doc --check-links            # Validate links

# Version comparison
aria doc --diff v1.0 v1.1         # Show API changes

# Configuration
aria doc --config docs.toml       # Custom config file
```
