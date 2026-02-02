# aria-python

Python bindings for the Aria programming language, enabling seamless interoperability between Python and Aria.

## Features

- **Execute Aria code from Python**: Run Aria programs directly from Python scripts
- **Bidirectional type conversion**: Automatic conversion between Python and Aria types
- **Function calls**: Call Aria functions from Python with Python arguments
- **Global variable access**: Get and set Aria global variables from Python
- **Full type support**: Support for all Aria types including primitives, arrays, maps, structs, and enums

## Installation

### Building from source

```bash
# Build the Python extension module
cd crates/aria-python
maturin develop  # For development
# or
maturin build --release  # For production
pip install target/wheels/*.whl
```

## Usage

### Basic Example

```python
import aria_python as aria

# Create an interpreter
interp = aria.AriaInterpreter()

# Execute Aria code
result = interp.eval("""
fn main() -> Int {
    let x = 10;
    let y = 32;
    return x + y;
}
""")

print(result)  # Output: 42
```

### Type Conversions

The following type conversions are supported:

| Aria Type | Python Type |
|-----------|-------------|
| `Nil` | `None` |
| `Bool` | `bool` |
| `Int` | `int` |
| `Float` | `float` |
| `String` | `str` |
| `Array` | `list` |
| `Map` | `dict` |
| `Tuple` | `tuple` |
| `Struct` | `dict` (with `__struct_name__` key) |
| `EnumVariant` | `dict` (with `__enum_name__` and `__variant_name__` keys) |

### Calling Aria Functions from Python

```python
import aria_python as aria

interp = aria.AriaInterpreter()

# Define a function in Aria
interp.exec("""
fn add(a: Int, b: Int) -> Int {
    return a + b;
}

fn main() -> Int {
    return 0;
}
""")

# Call it from Python
result = interp.call_function("add", (10, 32))
print(result)  # Output: 42
```

### Setting and Getting Global Variables

```python
import aria_python as aria

interp = aria.AriaInterpreter()

# Set global variables from Python
interp.set_global("x", 100)
interp.set_global("name", "Aria")

# Get them back
x = interp.get_global("x")
name = interp.get_global("name")

print(f"x = {x}, name = {name}")  # Output: x = 100, name = Aria
```

### Working with Complex Types

```python
import aria_python as aria

interp = aria.AriaInterpreter()

# Pass Python list to Aria
interp.set_global("numbers", [1, 2, 3, 4, 5])

result = interp.eval("""
fn main() -> Int {
    let sum = 0;
    for n in numbers {
        sum = sum + n;
    }
    return sum;
}
""")

print(result)  # Output: 15
```

### Advanced: Fibonacci Example

```python
import aria_python as aria

interp = aria.AriaInterpreter()

result = interp.eval("""
fn fibonacci(n: Int) -> Int {
    if n <= 1 {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

fn main() -> Int {
    return fibonacci(10);
}
""")

print(f"10th Fibonacci number: {result}")  # Output: 55
```

### Listing Available Functions

```python
import aria_python as aria

interp = aria.AriaInterpreter()

# Define some functions
interp.exec("""
fn foo() -> Int { return 1; }
fn bar() -> Int { return 2; }
fn main() -> Int { return 0; }
""")

# List all global functions and variables
globals_list = interp.list_globals()
print(f"Available: {globals_list}")
```

## API Reference

### `AriaInterpreter`

The main interpreter class for executing Aria code.

#### Methods

- `__init__()`: Create a new Aria interpreter
- `eval(code: str) -> Any`: Execute Aria code and return the result
- `exec(code: str) -> None`: Execute Aria code without returning a value
- `get_global(name: str) -> Any`: Get a global variable by name
- `set_global(name: str, value: Any) -> None`: Set a global variable
- `call_function(name: str, args: tuple) -> Any`: Call an Aria function by name
- `list_globals() -> List[str]`: List all global variables and functions

### `AriaValue`

Wrapper class for Aria values that can't be directly converted to Python types (e.g., functions).

#### Properties

- `type_name`: Get the type name of the value

#### Methods

- `to_python()`: Convert to Python object (if possible)
- `is_truthy()`: Check if value is truthy

### Module Functions

- `eval_aria(code: str) -> Any`: Convenience function to evaluate Aria code
- `exec_aria(code: str) -> None`: Convenience function to execute Aria code

## Development

### Running Tests

```bash
# Run Rust tests
cargo test

# Run Python tests (requires maturin develop)
maturin develop
pytest
```

### Building Documentation

```bash
cargo doc --open
```

## Requirements

- Rust 1.75 or higher
- Python 3.8 or higher
- PyO3 dependencies

## License

Licensed under either of:

- MIT License
- Apache License, Version 2.0

at your option.
