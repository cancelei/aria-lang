# Changelog

All notable changes to the Aria Standard Library will be documented in this file.

## [0.1.0] - 2026-01-16

### Added

#### Core Infrastructure
- Created `aria-stdlib` crate with module loading system
- Implemented `Stdlib` struct for parsing and caching standard library modules
- Added `StdModule` enum for type-safe module references
- Created `load_prelude()` function for automatic imports
- Added `get_module_source()` helper for module path resolution

#### Standard Library Modules

##### std::prelude (28 lines)
- Core type aliases: `Int`, `Float`, `Bool`, `String`, `Char`
- Re-exports of common types: `Option`, `Result`, `List`, `Map`, `Set`
- Re-exports of common functions: `print`, `println`, `panic`, `assert`

##### std::io (88 lines)
- Input/output functions: `print()`, `println()`, `read_line()`
- Error output: `eprint()`, `eprintln()`
- File operations: `File` struct with `open()`, `create()`, `read_to_string()`, `write()`
- Helper functions: `read_file()`, `write_file()`

##### std::option (121 lines)
- `Option<T>` enum with `Some(T)` and `None` variants
- Methods: `is_some()`, `is_none()`, `unwrap()`, `unwrap_or()`, `unwrap_or_else()`
- Combinators: `map()`, `and_then()`, `or()`, `filter()`, `flatten()`
- Helper functions: `Some()`, `None()`

##### std::result (141 lines)
- `Result<T, E>` enum with `Ok(T)` and `Err(E)` variants
- Methods: `is_ok()`, `is_err()`, `unwrap()`, `unwrap_err()`, `expect()`
- Combinators: `map()`, `map_err()`, `and_then()`, `or_else()`
- Conversion: `ok()`, `err()`, `flatten()`
- Helper functions: `Ok()`, `Err()`

##### std::string (145 lines)
- String methods: `len()`, `is_empty()`, `to_uppercase()`, `to_lowercase()`
- Trimming: `trim()`, `trim_start()`, `trim_end()`
- Splitting: `split()`, `split_whitespace()`
- Searching: `starts_with()`, `ends_with()`, `contains()`
- Manipulation: `replace()`, `repeat()`, `substring()`
- Parsing: `parse_int()`, `parse_float()`
- Helpers: `format()`, `to_string()`

##### std::collections (301 lines)
- `List<T>` type with methods:
  - Creation: `new()`
  - Access: `get()`, `first()`, `last()`, `len()`, `is_empty()`
  - Modification: `push()`, `pop()`, `reverse()`, `sort()`
  - Functional: `map()`, `filter()`, `fold()`, `any()`, `all()`, `flatten()`
  - Slicing: `take()`, `skip()`, `slice()`
- `Map<K, V>` type with methods:
  - Creation: `new()`
  - Access: `get()`, `contains_key()`, `keys()`, `values()`, `entries()`
  - Modification: `insert()`, `remove()`, `clear()`
  - Functional: `map_values()`, `filter()`
- `Set<T>` type with methods:
  - Creation: `new()`
  - Modification: `insert()`, `remove()`, `clear()`
  - Queries: `contains()`, `len()`, `is_empty()`
  - Set operations: `union()`, `intersection()`, `difference()`
  - Predicates: `is_subset()`, `is_superset()`

##### std::math (281 lines)
- Constants: `PI`, `EULER`, `TAU`, `SQRT_2`, `SQRT_3`, `LN_2`, `LN_10`
- Basic: `abs()`, `min()`, `max()`, `clamp()`, `sign()`
- Power: `sqrt()`, `pow()`, `exp()`, `ln()`, `log10()`, `log2()`
- Trigonometric: `sin()`, `cos()`, `tan()`, `asin()`, `acos()`, `atan()`, `atan2()`
- Hyperbolic: `sinh()`, `cosh()`, `tanh()`
- Rounding: `round()`, `floor()`, `ceil()`, `trunc()`
- Conversions: `to_radians()`, `to_degrees()`
- Number theory: `gcd()`, `lcm()`, `factorial()`
- Checks: `is_nan()`, `is_infinite()`, `is_finite()`

##### std::iter (277 lines)
- `Iterator<T>` trait with `next()` method
- `IntoIterator<T>` trait for conversion to iterators
- `RangeIterator` implementation for range iteration
- `ArrayIterator<T>` implementation for array iteration
- Iterator methods:
  - Collection: `collect()`
  - Transformation: `map()`, `filter()`, `flatten()`
  - Slicing: `take()`, `skip()`
  - Combining: `chain()`, `zip()`, `enumerate()`
  - Reduction: `fold()`, `sum()`, `product()`, `count()`
  - Searching: `find()`, `nth()`, `first()`, `last()`
  - Predicates: `any()`, `all()`
  - Min/max: `min()`, `max()`
  - Utilities: `for_each()`, `partition()`

#### Documentation
- Created comprehensive README.md with module descriptions and usage examples
- Added inline documentation for all public functions and types
- Created examples/usage.rs demonstrating stdlib loading

#### Testing
- Added unit tests for module loading
- Added tests for module path resolution
- Added tests for source embedding
- Tests verify infrastructure works correctly

### Implementation Notes

All stdlib modules are written in pure Aria syntax following these conventions:
- Functions use `fn name(params) body end` syntax
- Enums use `enum Name variant end` syntax
- Structs use `struct Name fields end` syntax
- Implementations use `impl Type methods end` syntax
- Comments use `#` for single-line comments
- Type annotations use `: Type` syntax
- Generic types use `<T>` syntax

Native builtin functions are marked with `# Native builtin` comments and implemented in `aria-interpreter/src/builtins.rs`.

### Known Limitations

1. Parser support incomplete for some features (ongoing work)
2. Many functions marked as `TODO: Native implementation` pending runtime support
3. File I/O operations not yet connected to FFI layer
4. String manipulation awaiting UTF-8 aware native implementations
5. Iterator combinators need lazy evaluation support
6. Some numeric operations await libm integration

### Statistics

- Total lines of Aria code: ~1,382 lines across 8 modules
- Total lines of Rust code: ~258 lines in lib.rs
- Number of modules: 8 (prelude, io, option, result, string, collections, math, iter)
- Number of public API items: 12+ exports
- Test coverage: 7 unit tests

### Integration

The stdlib is now a member of the workspace and can be imported by:
- `aria-interpreter` for runtime evaluation
- `aria-compiler` for compilation
- `aria-lsp` for code intelligence
- Any other crate needing standard library definitions
