# aria-python Implementation Details

## Overview

The `aria-python` crate provides complete Python bindings for the Aria programming language using PyO3. This document describes the implementation.

## Components

### 1. PyAriaValue

A Python-exposed wrapper around Aria's `Value` type, supporting:

- **Type inspection**: `type_name` property
- **Python conversion**: `to_python()` method
- **Truthiness**: `is_truthy()` and `__bool__()`
- **Type conversion**: `__int__()`, `__float__()` for compatible types
- **String representation**: `__repr__()` and `__str__()`

Marked as `unsendable` because Aria values use `Rc` internally.

### 2. PyAriaInterpreter

The main interpreter class providing:

- **Code execution**:
  - `eval(code)` - Execute and return result
  - `exec(code)` - Execute without returning

- **Global variable access**:
  - `get_global(name)` - Retrieve a global variable
  - `set_global(name, value)` - Set a global variable

- **Function calls**:
  - `call_function(name, args)` - Call an Aria function from Python

- **Introspection**:
  - `list_globals()` - List all available functions and variables

Marked as `unsendable` because the interpreter uses `Rc<RefCell<Environment>>`.

### 3. Type Conversions

#### Aria to Python (aria_to_python)

| Aria Type | Python Type | Notes |
|-----------|-------------|-------|
| `Nil` | `None` | - |
| `Bool` | `bool` | Direct conversion |
| `Int` | `int` | i64 to Python int |
| `Float` | `float` | f64 to Python float |
| `String` | `str` | SmolStr to Python str |
| `Array` | `list` | Recursive conversion |
| `Map` | `dict` | String keys, recursive values |
| `Tuple` | `tuple` | Immutable, recursive conversion |
| `Range` | `tuple` | Converted to (start, end) |
| `Function` | `PyAriaValue` | Wrapped for later use |
| `BuiltinFunction` | `PyAriaValue` | Wrapped for later use |
| `Struct` | `dict` | With `__struct_name__` key |
| `EnumVariant` | `dict` | With `__enum_name__` and `__variant_name__` keys |

#### Python to Aria (python_to_aria)

| Python Type | Aria Type | Notes |
|-------------|-----------|-------|
| `None` | `Nil` | - |
| `bool` | `Bool` | Direct conversion |
| `int` | `Int` | Python int to i64 |
| `float` | `Float` | Python float to f64 |
| `str` | `String` | Python str to SmolStr |
| `list` | `Array` | Recursive conversion to Rc<RefCell<Vec<Value>>> |
| `tuple` | `Tuple` | Recursive conversion to Rc<Vec<Value>> |
| `dict` | `Map` | String keys required, Rc<RefCell<IndexMap<SmolStr, Value>>> |
| `PyAriaValue` | `Value` | Unwrap inner value |

### 4. Module Functions

Convenience functions for quick operations:

- `eval_aria(code)` - Create interpreter and eval in one call
- `exec_aria(code)` - Create interpreter and exec in one call

## Implementation Notes

### Thread Safety

Both `PyAriaValue` and `PyAriaInterpreter` are marked with `#[pyclass(unsendable)]` because:

1. Aria's `Value` type uses `Rc` for reference-counted data (Arrays, Maps, Tuples)
2. The `Interpreter` uses `Rc<RefCell<Environment>>` for the environment chain
3. `Rc` is not `Send` and cannot be safely shared across threads

This is acceptable because:
- Python's GIL ensures single-threaded execution
- Each Python thread can create its own interpreter instance
- Free-threaded Python (PEP 703) is handled via PyO3's `unsendable` marker

### Function Calling Strategy

The `call_function()` method uses a workaround because the interpreter's `call_value()` method is private:

1. Convert Python arguments to Aria values
2. Set each argument as a global variable (`__arg0`, `__arg1`, etc.)
3. Generate Aria code: `fn main() -> Int { return func_name(__arg0, __arg1); }`
4. Parse and execute the generated code
5. Return the result

While not the most elegant solution, this works reliably and doesn't require modifying the interpreter's API.

### Error Handling

All potential errors are converted to Python exceptions:

- Lexer errors → `PyRuntimeError`
- Parser errors → `PyRuntimeError`
- Runtime errors → `PyRuntimeError`
- Type conversion errors → `PyTypeError`
- Missing variables/functions → `PyValueError`

## Building

### Prerequisites

```bash
# Install maturin for building Python extensions
pip install maturin

# Or use cargo
cargo install maturin
```

### Development Build

```bash
cd crates/aria-python
maturin develop
```

### Release Build

```bash
maturin build --release
pip install target/wheels/*.whl
```

### Environment Variables

Due to Python 3.14's experimental features:

```bash
export UNSAFE_PYO3_BUILD_FREE_THREADED=1
export PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1
```

## Testing

### Rust Tests

```bash
cargo test -p aria-python
```

The test suite includes:

- `test_value_conversion_roundtrip` - Verify bidirectional type conversion
- `test_interpreter_basic` - Basic arithmetic execution
- `test_python_eval_aria_simple` - Simple eval from Python
- `test_python_interpreter_basic` - Full interpreter usage
- `test_python_aria_string_values` - String handling
- `test_python_aria_boolean_values` - Boolean values
- `test_python_aria_arrays` - Array conversion
- `test_python_set_get_globals` - Global variable access
- `test_python_call_aria_function` - Function calling
- `test_python_list_globals` - Introspection
- `test_python_aria_complex_computation` - Fibonacci recursion
- `test_python_aria_type_conversions` - Comprehensive type tests

### Python Example

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

print(f"Result: {result}")  # Output: 55
```

## API Compatibility

### Python Version Support

- Minimum: Python 3.8
- Maximum tested: Python 3.14 (with forward compatibility)
- ABI: Using stable ABI3 for Python 3.8+

### Rust Version

- Minimum: Rust 1.75
- Uses 2021 edition features

## Future Improvements

1. **Direct function calling**: Expose interpreter's call mechanism publicly
2. **Async support**: Add async/await integration between Python and Aria
3. **Better error messages**: Include source locations in error messages
4. **Performance**: Use Arc instead of Rc for thread-safe value sharing
5. **Type hints**: Add Python type stubs (.pyi files)
6. **Streaming results**: Support for generators and iterators

## Dependencies

### Rust Dependencies

- `pyo3 = "0.22"` - Python bindings
- `aria-interpreter` - Aria interpreter
- `aria-parser` - Aria parser
- `aria-ast` - Aria AST definitions
- `aria-lexer` - Aria lexer
- `smol_str` - Efficient string storage
- `indexmap` - Ordered hash map

### Build Dependencies

- `maturin` - Python extension builder
- PyO3 build tools

## License

Licensed under either of:

- MIT License
- Apache License, Version 2.0

at your option.
