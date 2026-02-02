# Native Builtin Functions

This document lists all native builtin functions that need to be implemented in the Aria runtime (`aria-interpreter/src/builtins.rs` or similar).

## Already Implemented

These functions are already available in `aria-interpreter/src/builtins.rs`:

- `print(value)` - Print value to stdout without newline
- `println(value)` - Print value to stdout with newline
- `input()` - Read line from stdin
- `type_of(value)` - Get type name as string
- `to_string(value)` - Convert value to string
- `to_int(value)` - Convert value to integer
- `to_float(value)` - Convert value to float
- `len(collection)` - Get length of collection
- `range(start, end)` - Create range iterator
- `abs(number)` - Absolute value
- `min(a, b)` - Minimum of two values
- `max(a, b)` - Maximum of two values
- `assert(condition, message?)` - Assertion

## Need Implementation

### I/O Functions

```rust
fn eprint(value) -> ()
fn eprintln(value) -> ()
fn printf(format: String, ...args) -> ()
```

### String Operations

```rust
fn string_to_uppercase(s: String) -> String
fn string_to_lowercase(s: String) -> String
fn string_trim(s: String) -> String
fn string_trim_start(s: String) -> String
fn string_trim_end(s: String) -> String
fn string_split(s: String, delimiter: String) -> [String]
fn string_split_whitespace(s: String) -> [String]
fn string_join(parts: [String], separator: String) -> String
fn string_starts_with(s: String, prefix: String) -> Bool
fn string_ends_with(s: String, suffix: String) -> Bool
fn string_contains(s: String, substring: String) -> Bool
fn string_replace(s: String, pattern: String, replacement: String) -> String
fn string_repeat(s: String, n: Int) -> String
fn string_substring(s: String, start: Int, end: Int) -> String
fn string_char_at(s: String, index: Int) -> Option<Char>
```

### List Operations

```rust
fn list_push(list: [T], value: T) -> ()
fn list_pop(list: [T]) -> Option<T>
fn list_contains(list: [T], value: T) -> Bool
fn list_index_of(list: [T], value: T) -> Option<Int>
fn list_reverse(list: [T]) -> ()
fn list_sort(list: [T]) -> ()
fn list_flatten(list: [[T]]) -> [T]
fn list_take(list: [T], n: Int) -> [T]
fn list_skip(list: [T], n: Int) -> [T]
fn list_slice(list: [T], start: Int, end: Int) -> [T]
```

### Map Operations

```rust
fn map_get(map: {K: V}, key: K) -> Option<V>
fn map_remove(map: {K: V}, key: K) -> Option<V>
fn map_contains_key(map: {K: V}, key: K) -> Bool
fn map_keys(map: {K: V}) -> [K]
fn map_values(map: {K: V}) -> [V]
fn map_entries(map: {K: V}) -> [(K, V)]
fn map_clear(map: {K: V}) -> ()
```

### Set Operations

```rust
fn set_remove(set: Set<T>, value: T) -> Bool
fn set_contains(set: Set<T>, value: T) -> Bool
fn set_clear(set: Set<T>) -> ()
fn set_union(a: Set<T>, b: Set<T>) -> Set<T>
fn set_intersection(a: Set<T>, b: Set<T>) -> Set<T>
fn set_difference(a: Set<T>, b: Set<T>) -> Set<T>
fn set_is_subset(a: Set<T>, b: Set<T>) -> Bool
fn set_is_superset(a: Set<T>, b: Set<T>) -> Bool
```

### Math Functions

```rust
fn math_sqrt(x: Float) -> Float
fn math_pow(base: Float, exponent: Float) -> Float
fn math_ln(x: Float) -> Float
fn math_log10(x: Float) -> Float
fn math_log2(x: Float) -> Float
fn math_exp(x: Float) -> Float
fn math_sin(x: Float) -> Float
fn math_cos(x: Float) -> Float
fn math_tan(x: Float) -> Float
fn math_asin(x: Float) -> Float
fn math_acos(x: Float) -> Float
fn math_atan(x: Float) -> Float
fn math_atan2(y: Float, x: Float) -> Float
fn math_sinh(x: Float) -> Float
fn math_cosh(x: Float) -> Float
fn math_tanh(x: Float) -> Float
fn math_round(x: Float) -> Float
fn math_floor(x: Float) -> Float
fn math_ceil(x: Float) -> Float
fn math_trunc(x: Float) -> Float
fn math_is_nan(x: Float) -> Bool
fn math_is_infinite(x: Float) -> Bool
fn math_is_finite(x: Float) -> Bool
```

### Panic and Control

```rust
fn panic(message: String) -> !
fn todo(message: String) -> !
fn unimplemented(message: String) -> !
fn unreachable(message: String) -> !
```

## Implementation Strategy

### Phase 1: Core Functions (Highest Priority)
1. String operations: `to_uppercase`, `to_lowercase`, `trim`, `split`, `contains`
2. List operations: `push`, `pop`, `contains`, `reverse`, `sort`
3. Math basics: `sqrt`, `pow`, `sin`, `cos`, `tan`, `floor`, `ceil`, `round`
4. Control: `panic`, `todo`, `unreachable`

### Phase 2: Extended Functions
1. Advanced string: `replace`, `substring`, `split_whitespace`, `join`
2. Map/Set operations: `get`, `remove`, `keys`, `values`, `union`, `intersection`
3. Advanced math: `ln`, `exp`, `log10`, trigonometric inverses
4. List transformations: `flatten`, `take`, `skip`, `slice`

### Phase 3: File I/O (Requires FFI)
1. File operations: `open`, `create`, `read`, `write`, `close`
2. Error output: `eprint`, `eprintln`
3. Formatted output: `printf`

### Phase 4: Iterators (Requires Runtime Support)
1. Lazy evaluation for iterator combinators
2. Iterator protocol implementation
3. Iterator adapter types

## Integration Points

### aria-interpreter/src/builtins.rs

Add new builtin functions following this pattern:

```rust
fn builtin_string_to_uppercase(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::String(s) => Ok(Value::String(s.to_uppercase().into())),
        v => Err(RuntimeError::TypeError {
            message: format!("to_uppercase expects string, got {}", v.type_name()),
            span: Span::default(),
        }),
    }
}

// Register in the environment
pub fn register(env: &mut Environment) {
    // ... existing builtins ...
    env.define("to_uppercase".into(), make_builtin("to_uppercase", Some(1), builtin_string_to_uppercase));
}
```

### Calling Convention

Stdlib functions marked with `# Native builtin` call the corresponding Rust function directly. For example:

```aria
# In std/string.aria
fn to_uppercase(self) -> String
  # Native builtin
  to_uppercase(self)
end
```

Maps to:

```rust
// In aria-interpreter/src/builtins.rs
env.define("to_uppercase".into(), make_builtin("to_uppercase", Some(1), builtin_string_to_uppercase));
```

### Testing

Each native function should have tests in `aria-interpreter/src/builtins.rs`:

```rust
#[test]
fn test_builtin_string_to_uppercase() {
    assert_eq!(
        builtin_string_to_uppercase(vec![Value::String("hello".into())]).unwrap(),
        Value::String("HELLO".into())
    );
}
```

## External Dependencies

For optimal implementations, consider using:

- **libm** - Math functions (already available in std)
- **unicode-segmentation** - Proper Unicode handling for strings
- **regex** - Pattern matching for string operations
- **indexmap** - Order-preserving maps (already in workspace)

## Performance Considerations

1. String operations should use `SmolStr` for small strings (already used)
2. Collections should use `Rc<RefCell<>>` for shared mutation (already used)
3. Math functions can directly use Rust's `f64` methods
4. Consider SIMD for bulk operations in the future

## Documentation

Each native function should be documented in both places:
1. Aria stdlib file (`.aria`) with usage examples
2. Rust implementation with technical details

Example:

```aria
# In std/string.aria
# Returns a new string with all characters in uppercase
# Example: "hello".to_uppercase() # => "HELLO"
fn to_uppercase(self) -> String
  # Native builtin
  to_uppercase(self)
end
```

```rust
// In aria-interpreter/src/builtins.rs
/// Converts a string to uppercase
/// This function uses Unicode-aware case conversion
fn builtin_string_to_uppercase(args: Vec<Value>) -> Result<Value> {
    // implementation
}
```
