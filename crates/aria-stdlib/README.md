# Aria Standard Library

The standard library for the Aria programming language, providing core types, traits, and functions.

## Overview

The Aria standard library (`aria-stdlib`) is embedded directly into the compiler and interpreter. It provides essential data structures, algorithms, and utilities that are available to all Aria programs.

## Modules

### `std::prelude`

Automatically imported into every Aria module. Contains commonly used types and functions:

- Type aliases: `Int`, `Float`, `Bool`, `String`, `Char`
- Core types: `Option`, `Result`, `List`, `Map`, `Set`
- Common functions: `print`, `println`, `panic`, `assert`

### `std::io`

Input/output operations:

- `print(value)` - Print to stdout without newline
- `println(value)` - Print to stdout with newline
- `read_line() -> String` - Read a line from stdin
- `eprint(value)` - Print to stderr
- `eprintln(value)` - Print to stderr with newline
- `File` - File operations (reading/writing files)

### `std::option`

Optional values with the `Option<T>` enum:

```aria
enum Option<T>
  Some(T)
  None
end
```

Methods: `is_some()`, `is_none()`, `unwrap()`, `unwrap_or()`, `map()`, `and_then()`, `filter()`, etc.

### `std::result`

Error handling with the `Result<T, E>` enum:

```aria
enum Result<T, E>
  Ok(T)
  Err(E)
end
```

Methods: `is_ok()`, `is_err()`, `unwrap()`, `map()`, `map_err()`, `and_then()`, etc.

### `std::string`

String manipulation and formatting:

- `len()` - Get string length
- `is_empty()` - Check if empty
- `to_uppercase()`, `to_lowercase()` - Case conversion
- `trim()`, `trim_start()`, `trim_end()` - Whitespace removal
- `split(delimiter)` - Split string
- `contains(substring)` - Check for substring
- `replace(pattern, replacement)` - Replace text
- `parse_int()`, `parse_float()` - Parse numbers

### `std::collections`

Collection types and operations:

#### List (Array)
- `new()` - Create empty list
- `len()`, `is_empty()` - Size queries
- `push()`, `pop()` - Add/remove elements
- `get(index)`, `first()`, `last()` - Access elements
- `map()`, `filter()`, `fold()` - Functional operations
- `any()`, `all()` - Predicates
- `reverse()`, `sort()` - Mutation

#### Map (Dictionary)
- `new()` - Create empty map
- `get(key)`, `insert(key, value)` - Access/modify
- `contains_key(key)` - Check for key
- `keys()`, `values()`, `entries()` - Iteration
- `map_values()`, `filter()` - Transformations

#### Set
- `new()` - Create empty set
- `insert()`, `remove()` - Modify
- `contains()` - Check membership
- `union()`, `intersection()`, `difference()` - Set operations

### `std::math`

Mathematical functions and constants:

Constants: `PI`, `E`, `TAU`, `SQRT_2`, `LN_2`, etc.

Functions:
- Basic: `abs()`, `min()`, `max()`, `clamp()`, `sign()`
- Power: `sqrt()`, `pow()`, `exp()`, `ln()`, `log10()`
- Trigonometric: `sin()`, `cos()`, `tan()`, `asin()`, `acos()`, `atan()`
- Hyperbolic: `sinh()`, `cosh()`, `tanh()`
- Rounding: `round()`, `floor()`, `ceil()`, `trunc()`
- Number theory: `gcd()`, `lcm()`, `factorial()`

### `std::iter`

Iterator trait and methods:

```aria
trait Iterator<T>
  fn next(mut self) -> Option<T>
end
```

Methods: `collect()`, `map()`, `filter()`, `fold()`, `take()`, `skip()`, `count()`, `any()`, `all()`, `find()`, etc.

## Usage

The prelude is automatically imported. Other modules must be imported explicitly:

```aria
import std::io
import std::collections::{List, Map}
import std::math

fn main()
  # Prelude types are available
  let x: Option<Int> = Some(42)

  # Import required for other modules
  io::println("Hello, World!")

  let numbers = List::new()
  numbers.push(1)
  numbers.push(2)

  let angle = math::PI / 4.0
  let sine = math::sin(angle)
end
```

## Implementation

The standard library is implemented in two parts:

1. **Aria source files** (`*.aria`) - Define types, traits, and high-level functions
2. **Native builtins** (in `aria-interpreter`) - Provide low-level operations implemented in Rust

This hybrid approach allows the stdlib to be:
- **Type-safe** - Checked by the Aria compiler
- **Efficient** - Critical operations use native code
- **Extensible** - Easy to add new functionality in Aria

## Development

To add a new stdlib module:

1. Create `src/std/modulename.aria` with your Aria code
2. Add the module to `StdModule` enum in `lib.rs`
3. Include the file with `include_str!()` macro
4. Add the module to `StdModule::all()` and path/source methods
5. Add to `get_module_source()` function
6. Add corresponding native functions to `aria-interpreter/src/builtins.rs` if needed

## Testing

Run tests with:

```bash
cargo test -p aria-stdlib
```

## License

MIT OR Apache-2.0
