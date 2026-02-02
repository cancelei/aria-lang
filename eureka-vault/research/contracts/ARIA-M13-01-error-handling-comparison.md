# ARIA-M13-01: Error Handling Approaches Comparison

**Task ID**: ARIA-M13-01
**Status**: Completed
**Date**: 2026-01-14
**Focus**: Comprehensive comparison of error handling across languages

---

## Executive Summary

This research compares Result/Option types, exceptions, error unions, and multiple returns across languages to inform Aria's error handling design.

---

## 1. Error Handling Landscape

### 1.1 Major Approaches

| Approach | Languages | Characteristics |
|----------|-----------|-----------------|
| Exceptions | Java, Python, C# | Stack unwinding, try/catch |
| Result types | Rust, Haskell, OCaml | Explicit, type-safe |
| Error unions | Zig | Compile-time checked |
| Multiple returns | Go | Simple, explicit |
| Panic/Recover | Go, Rust | For unrecoverable errors |

### 1.2 Evolution (2025 Perspective)

The industry is shifting toward Result types:
- Distributed systems made expected failures common
- Stack traces create observability overhead
- Results provide compile-time safety

---

## 2. Exceptions (Java, Python, C#)

### 2.1 How They Work

```java
// Java example
public User getUser(String id) throws UserNotFoundException {
    User user = db.find(id);
    if (user == null) {
        throw new UserNotFoundException(id);
    }
    return user;
}

try {
    User user = getUser("123");
} catch (UserNotFoundException e) {
    handleMissing(e);
}
```

### 2.2 Checked vs Unchecked

| Type | Java | Behavior |
|------|------|----------|
| Checked | `throws` in signature | Must handle or declare |
| Unchecked | RuntimeException | Can propagate silently |

### 2.3 Problems

| Issue | Description |
|-------|-------------|
| Hidden control flow | Can't tell where exceptions go |
| Signature opacity | Unchecked exceptions invisible |
| Performance overhead | Stack trace capture expensive |
| Distributed systems | Traces add storage costs |

---

## 3. Result Types (Rust, Haskell)

### 3.1 Rust's Approach

```rust
enum Result<T, E> {
    Ok(T),
    Err(E),
}

fn read_file(path: &str) -> Result<String, io::Error> {
    let content = fs::read_to_string(path)?;  // ? propagates
    Ok(content)
}

// Usage
match read_file("config.txt") {
    Ok(content) => process(content),
    Err(e) => eprintln!("Error: {}", e),
}
```

### 3.2 The `?` Operator

```rust
// Without ?
fn process() -> Result<Output, Error> {
    let a = step_one()?;   // Early return on Err
    let b = step_two(a)?;  // Early return on Err
    Ok(final_step(b))
}

// Equivalent to:
fn process() -> Result<Output, Error> {
    let a = match step_one() {
        Ok(v) => v,
        Err(e) => return Err(e.into()),
    };
    // ...
}
```

### 3.3 Benefits

| Benefit | Description |
|---------|-------------|
| Type safety | Compiler enforces handling |
| Explicit flow | Errors visible in types |
| Composable | map, and_then, or_else |
| No runtime cost | Zero-cost abstraction |

---

## 4. Error Unions (Zig)

### 4.1 How They Work

```zig
const FileOpenError = error{
    AccessDenied,
    FileNotFound,
    OutOfMemory,
};

fn openFile(path: []const u8) FileOpenError!File {
    // Returns either File or an error
}

// Usage with catch
const file = openFile("data.txt") catch |err| {
    switch (err) {
        error.FileNotFound => return default_file,
        else => return err,
    }
};

// Or with try (propagate)
const file = try openFile("data.txt");
```

### 4.2 Compile-Time Checking

```zig
// Zig tracks which errors are possible
fn process() !void {
    // Compiler knows exact error set
    const file = try openFile("x");  // FileOpenError
    const data = try readData(file); // ReadError
    // Combined error set inferred
}
```

---

## 5. Multiple Returns (Go)

### 5.1 Pattern

```go
func readFile(path string) (string, error) {
    content, err := ioutil.ReadFile(path)
    if err != nil {
        return "", err
    }
    return string(content), nil
}

// Usage
content, err := readFile("config.txt")
if err != nil {
    log.Fatal(err)
}
process(content)
```

### 5.2 Trade-offs

| Pro | Con |
|-----|-----|
| Explicit | Verbose boilerplate |
| Simple | Easy to forget checking |
| No special syntax | No propagation operator |

---

## 6. Comparison Matrix

| Aspect | Exceptions | Result | Error Union | Multi-Return |
|--------|------------|--------|-------------|--------------|
| Compile-time safety | Partial | Yes | Yes | No |
| Propagation ease | try/catch | `?` | `try` | Manual |
| Performance | Overhead | Zero-cost | Zero-cost | Zero-cost |
| Error info | Stack trace | Custom | Enum | Interface |
| Learning curve | Familiar | Moderate | Moderate | Simple |
| Boilerplate | Low | Low | Low | High |

---

## 7. Best Practices (2025)

### 7.1 Modern Consensus

```
Results for:
- Domain operations
- Validation
- Business rules
- Service calls
- Expected failures

Exceptions/Panic for:
- Programming errors
- Contract violations
- Unrecoverable state
- Bugs
```

### 7.2 Distributed Systems

Exceptions create observable overhead:
- Stack trace storage
- Distributed tracing noise
- Results avoid this entirely

---

## 8. Context & Error Chaining

### 8.1 Error Context

```rust
// Rust with anyhow
fn process_config() -> anyhow::Result<Config> {
    let content = fs::read_to_string("config.json")
        .context("Failed to read config file")?;

    let config: Config = serde_json::from_str(&content)
        .context("Failed to parse config JSON")?;

    Ok(config)
}

// Error output:
// Error: Failed to parse config JSON
// Caused by: expected `:` at line 5 column 3
```

### 8.2 Backtrace Capture

```rust
// Capture backtraces for debugging
use std::backtrace::Backtrace;

struct MyError {
    message: String,
    backtrace: Backtrace,
}
```

---

## 9. Recommendations for Aria

### 9.1 Core Design

```aria
# Result type with ? propagation
Result[T, E] = Ok(T) | Err(E)

fn read_config(path: String) -> Result[Config, ConfigError]
  content = File.read(path)?          # Propagates on error
  json = JSON.parse(content)?         # Propagates on error
  Config.from_json(json)
end
```

### 9.2 Error Effect Integration

```aria
# Errors as effects (unique to Aria)
fn risky_operation() -> {Error[MyError]} Int
  if condition_bad
    raise MyError.new("something wrong")
  else
    42
  end
end

# Handle with effect handler
handle risky_operation()
  case Ok(value) => process(value)
  case Err(e: MyError) => fallback(e)
end
```

### 9.3 Recommended Syntax

```aria
# Result type (explicit)
fn divide(a: Int, b: Int) -> Result[Int, DivideByZero]
  if b == 0
    Err(DivideByZero.new)
  else
    Ok(a / b)
  end
end

# Pattern matching
match divide(10, 2)
  Ok(result) => println("Result: #{result}")
  Err(e)     => println("Error: #{e}")
end

# ? operator for propagation
fn calculate() -> Result[Int, MathError]
  a = divide(10, 2)?
  b = divide(a, 3)?
  Ok(a + b)
end

# Panic for unrecoverable (like Rust)
fn index[T](arr: Array[T], i: Int) -> T
  requires 0 <= i < arr.length  # Panic if violated
  arr.get_unchecked(i)
end
```

### 9.4 Error Hierarchy

```aria
# Base error trait
trait Error
  fn message() -> String
  fn source() -> Option[Error]
  fn backtrace() -> Option[Backtrace]
end

# User-defined errors
struct ConfigError : Error
  path: String
  reason: String

  fn message() -> String = "Config error at #{@path}: #{@reason}"
end

# Error context wrapping
fn load_config() -> Result[Config, ConfigError]
  File.read("config.json")
    .map_err |e| ConfigError.new(path: "config.json", reason: e.message) end
    .and_then |content| JSON.parse(content) end
end
```

### 9.5 Automatic Context

```aria
# Compiler adds source location automatically
fn might_fail() -> Result[T, E]
  something()?  # On error, captures file:line
end

# Error output includes trace:
# Error: Something failed
#   at src/lib.aria:42 in might_fail
#   at src/main.aria:10 in main
```

---

## 10. Key Resources

1. [Let Results Speak - Exceptions vs Result](https://stevenstuartm.com/blog/2025/10/29/result-pattern-vs-exceptions-revisited.html)
2. [Error Handling in Rust](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
3. [Error Handling Across Languages](https://blog.frankel.ch/error-handling/)
4. [Error Models for Systems Programming](https://typesanitizer.com/blog/errors.html)
5. [Musings About Error Handling](https://www.amazingcto.com/best-way-to-handle-errors-for-a-programming-language/)

---

## 11. Open Questions

1. Should Aria have checked effects for errors?
2. How do we make error context automatic?
3. What's the relationship between Result and effects?
4. Should we support error unions like Zig?
