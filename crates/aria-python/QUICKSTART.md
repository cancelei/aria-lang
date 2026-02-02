# aria-python Quick Start Guide

## Installation

```bash
cd crates/aria-python

# Set environment variables for Python 3.14 compatibility
export UNSAFE_PYO3_BUILD_FREE_THREADED=1
export PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1

# Install in development mode
maturin develop

# Or build a wheel for distribution
maturin build --release
pip install target/wheels/*.whl
```

## Basic Usage

### 1. Simple Evaluation

```python
import aria_python as aria

# Quick eval
result = aria.eval_aria("""
fn main() -> Int {
    return 2 + 2;
}
""")

print(result)  # Output: 4
```

### 2. Using the Interpreter

```python
import aria_python as aria

# Create an interpreter instance
interp = aria.AriaInterpreter()

# Execute code and get result
result = interp.eval("""
fn main() -> Int {
    let x = 10;
    let y = 32;
    return x + y;
}
""")

print(result)  # Output: 42
```

### 3. Working with Functions

```python
import aria_python as aria

interp = aria.AriaInterpreter()

# Define a function
interp.exec("""
fn greet(name: String) -> String {
    return "Hello, " + name + "!";
}

fn main() -> Int {
    return 0;
}
""")

# Call it from Python
greeting = interp.call_function("greet", ("World",))
print(greeting)  # Output: Hello, World!
```

### 4. Global Variables

```python
import aria_python as aria

interp = aria.AriaInterpreter()

# Set variables from Python
interp.set_global("name", "Alice")
interp.set_global("age", 30)

# Use them in Aria code
result = interp.eval("""
fn main() -> String {
    return "Name: " + name;
}
""")

print(result)  # Output: Name: Alice

# Get them back
age = interp.get_global("age")
print(age)  # Output: 30
```

### 5. Working with Collections

```python
import aria_python as aria

interp = aria.AriaInterpreter()

# Pass a Python list to Aria
interp.set_global("numbers", [1, 2, 3, 4, 5])

# Process it in Aria
result = interp.eval("""
fn main() -> Int {
    let sum = 0;
    let i = 0;
    while i < 5 {
        sum = sum + numbers[i];
        i = i + 1;
    }
    return sum;
}
""")

print(result)  # Output: 15

# Get an array from Aria
result = interp.eval("""
fn main() -> Array {
    return [10, 20, 30];
}
""")

print(result)  # Output: [10, 20, 30]
print(type(result))  # Output: <class 'list'>
```

### 6. Type Conversions

```python
import aria_python as aria

interp = aria.AriaInterpreter()

# Python → Aria → Python roundtrip
test_values = {
    "int": 42,
    "float": 3.14,
    "string": "hello",
    "bool": True,
    "list": [1, 2, 3],
    "dict": {"a": 1, "b": 2},
}

for name, value in test_values.items():
    interp.set_global(name, value)
    retrieved = interp.get_global(name)
    print(f"{name}: {value} -> {retrieved} (match: {value == retrieved})")
```

### 7. Complex Example: Fibonacci

```python
import aria_python as aria

interp = aria.AriaInterpreter()

# Define recursive function
interp.exec("""
fn fibonacci(n: Int) -> Int {
    if n <= 1 {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

fn main() -> Int {
    return 0;
}
""")

# Compute Fibonacci numbers
for i in range(11):
    fib = interp.call_function("fibonacci", (i,))
    print(f"F({i}) = {fib}")

# Output:
# F(0) = 0
# F(1) = 1
# F(2) = 1
# F(3) = 2
# F(4) = 3
# F(5) = 5
# F(6) = 8
# F(7) = 13
# F(8) = 21
# F(9) = 34
# F(10) = 55
```

### 8. Introspection

```python
import aria_python as aria

interp = aria.AriaInterpreter()

# Define some functions
interp.exec("""
fn add(a: Int, b: Int) -> Int { return a + b; }
fn sub(a: Int, b: Int) -> Int { return a - b; }
fn mul(a: Int, b: Int) -> Int { return a * b; }

fn main() -> Int { return 0; }
""")

# List all available functions
functions = interp.list_globals()
print("Available functions:", functions)

# Filter for user-defined functions
user_funcs = [f for f in functions if f in ['add', 'sub', 'mul', 'main']]
print("User functions:", user_funcs)
```

### 9. Error Handling

```python
import aria_python as aria

interp = aria.AriaInterpreter()

try:
    # This will cause a parse error
    interp.eval("invalid syntax here")
except RuntimeError as e:
    print(f"Parse error: {e}")

try:
    # This will cause a runtime error (undefined variable)
    interp.eval("""
    fn main() -> Int {
        return undefined_var;
    }
    """)
except RuntimeError as e:
    print(f"Runtime error: {e}")

try:
    # This will cause a value error (function not found)
    interp.call_function("nonexistent", ())
except ValueError as e:
    print(f"Value error: {e}")
```

### 10. Multiple Interpreters

```python
import aria_python as aria

# Each interpreter has its own state
interp1 = aria.AriaInterpreter()
interp2 = aria.AriaInterpreter()

interp1.set_global("x", 100)
interp2.set_global("x", 200)

print(interp1.get_global("x"))  # Output: 100
print(interp2.get_global("x"))  # Output: 200

# Define function in one interpreter
interp1.exec("""
fn double(n: Int) -> Int {
    return n * 2;
}
fn main() -> Int { return 0; }
""")

# It's not available in the other
try:
    interp2.call_function("double", (21,))
except ValueError as e:
    print("Function not in interp2:", e)

# But available in interp1
result = interp1.call_function("double", (21,))
print("Result from interp1:", result)  # Output: 42
```

## Type Conversion Reference

| Python Type | Aria Type | Example |
|------------|-----------|---------|
| `None` | `Nil` | `None` → `nil` |
| `bool` | `Bool` | `True` → `true` |
| `int` | `Int` | `42` → `42` |
| `float` | `Float` | `3.14` → `3.14` |
| `str` | `String` | `"hello"` → `"hello"` |
| `list` | `Array` | `[1,2,3]` → `[1, 2, 3]` |
| `tuple` | `Tuple` | `(1,2)` → `(1, 2)` |
| `dict` | `Map` | `{"a":1}` → `{"a": 1}` |

## Common Patterns

### Pattern 1: Batch Processing

```python
import aria_python as aria

interp = aria.AriaInterpreter()

interp.exec("""
fn process(n: Int) -> Int {
    return n * n + 2 * n + 1;
}
fn main() -> Int { return 0; }
""")

# Process a batch
inputs = [1, 2, 3, 4, 5]
results = [interp.call_function("process", (x,)) for x in inputs]
print(results)  # [4, 9, 16, 25, 36]
```

### Pattern 2: Configuration via Python

```python
import aria_python as aria

interp = aria.AriaInterpreter()

# Set configuration from Python
config = {
    "max_iterations": 1000,
    "threshold": 0.001,
    "debug": True,
}

for key, value in config.items():
    interp.set_global(key, value)

# Use in Aria code
result = interp.eval("""
fn main() -> Int {
    if debug {
        print("Max iterations: ");
        print(max_iterations);
    }
    return max_iterations;
}
""")
```

### Pattern 3: Data Pipeline

```python
import aria_python as aria

interp = aria.AriaInterpreter()

# Define processing steps in Aria
interp.exec("""
fn normalize(x: Float) -> Float {
    return x / 100.0;
}

fn transform(x: Float) -> Float {
    return x * x;
}

fn main() -> Int { return 0; }
""")

# Build pipeline in Python
data = [10.0, 20.0, 30.0, 40.0, 50.0]

# Step 1: Normalize
normalized = [interp.call_function("normalize", (x,)) for x in data]

# Step 2: Transform
transformed = [interp.call_function("transform", (x,)) for x in normalized]

print(transformed)
```

## Tips and Best Practices

1. **Reuse interpreters**: Creating an interpreter has overhead. Reuse instances when possible.

2. **Error handling**: Always wrap `eval()`, `exec()`, and `call_function()` in try-except blocks.

3. **Type conversion**: Be aware that all Aria integers are i64 and all floats are f64.

4. **Function calling**: The `main()` function is special - it's called automatically by `eval()`.

5. **Global scope**: Variables set with `set_global()` are available in all subsequent code.

6. **Threading**: Each Python thread should have its own interpreter instance (marked as `unsendable`).

## Next Steps

- Read [README.md](README.md) for complete API documentation
- Check [examples/basic_usage.py](examples/basic_usage.py) for more examples
- See [IMPLEMENTATION.md](IMPLEMENTATION.md) for implementation details
- Run tests: `cargo test -p aria-python`

## Troubleshooting

### Import Error
```python
ImportError: No module named 'aria_python'
```
**Solution**: Run `maturin develop` in the crate directory.

### Build Error with Python 3.14
```
error: the configured Python interpreter version (3.14) is newer than PyO3's maximum
```
**Solution**: Set environment variables:
```bash
export UNSAFE_PYO3_BUILD_FREE_THREADED=1
export PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1
```

### Type Conversion Error
```python
TypeError: Cannot convert Python type ... to Aria value
```
**Solution**: Check the type conversion table above. Only supported Python types can be converted.

### Function Not Found
```python
ValueError: Function 'foo' not found
```
**Solution**: Make sure the function is defined with `exec()` before calling it, and that you defined a `main()` function.
