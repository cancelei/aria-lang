# Aria C Runtime Integration Guide

This guide explains how the C runtime integrates with the Aria compiler and how to use it.

## Overview

The Aria C runtime provides essential functions that compiled Aria code needs to execute:

1. **I/O Operations**: Print functions for different types
2. **Memory Management**: Allocation and deallocation
3. **String Operations**: Concatenation and comparison
4. **Error Handling**: Panic function for runtime errors
5. **Entry Point**: C `main()` that calls Aria's `aria_main()`

## Architecture

```
┌─────────────────────┐
│  Aria Source Code   │
│   (program.aria)    │
└──────────┬──────────┘
           │
           │ aria build
           ↓
┌─────────────────────┐
│  Aria Compiler      │
│  (aria-compiler)    │
└──────────┬──────────┘
           │
           │ generates
           ↓
┌─────────────────────┐     ┌─────────────────────┐
│  Aria Object File   │     │  Runtime Library    │
│   (program.o)       │     │  (aria_runtime.o)   │
└──────────┬──────────┘     └──────────┬──────────┘
           │                           │
           └────────────┬──────────────┘
                        │
                        │ gcc linker
                        ↓
              ┌─────────────────────┐
              │  Executable Binary  │
              │    (program)        │
              └─────────────────────┘
```

## Compilation Workflow

### Method 1: Using the Compiler's --link Flag (Recommended)

The easiest way to create an executable:

```bash
# Build the runtime first (one-time setup)
cd crates/aria-runtime/c_runtime
make

# Compile and link in one step
aria build program.aria --link
./program
```

The compiler will automatically:
1. Compile your Aria source to an object file
2. Find the runtime library
3. Link them together
4. Produce a ready-to-run executable

### Method 2: Manual Compilation and Linking

For more control over the process:

```bash
# Step 1: Compile Aria source to object file
aria build program.aria -o program.o

# Step 2: Link with runtime
gcc aria_runtime.o program.o -o program

# Step 3: Run
./program
```

### Method 3: Using the Build Script

The runtime includes a helper script:

```bash
# Build runtime
cd crates/aria-runtime/c_runtime
./build.sh build

# Link your program
./build.sh link program.o program

# Run
./program
```

## Runtime Functions

All runtime functions are declared in `aria_runtime.h` and use C linkage (no name mangling).

### Print Functions

```c
void aria_print_int(int64_t value);
void aria_print_float(double value);
void aria_print_string(const char* str);
void aria_print_bool(int8_t value);
void aria_print_newline(void);
```

**Generated Aria IR Example:**
```
// Aria code:
print(42)

// Generates call to:
call aria_print_int(42)
```

### Memory Management

```c
void* aria_alloc(int64_t size);
void aria_dealloc(void* ptr, int64_t size);
```

**Usage in Generated Code:**
- `aria_alloc`: Called for heap allocations (strings, arrays, objects)
- `aria_dealloc`: Called when memory is no longer needed
- Both functions handle NULL gracefully
- Allocation failures trigger a panic

**Example:**
```c
// Allocate 100 bytes
void* ptr = aria_alloc(100);

// Use the memory
// ...

// Free when done
aria_dealloc(ptr, 100);
```

### String Operations

```c
char* aria_string_concat(const char* a, const char* b);
int8_t aria_string_eq(const char* a, const char* b);
```

**String Representation:**
- Strings are null-terminated C strings (`char*`)
- String literals are stored in the data section
- Concatenation creates a new heap-allocated string
- The caller must free concatenated strings with `aria_dealloc`

**Example:**
```c
const char* a = "Hello, ";
const char* b = "World!";

// Concatenate (allocates new string)
char* result = aria_string_concat(a, b);
aria_print_string(result);  // Prints: Hello, World!

// Must free the result
aria_dealloc(result, 0);

// Compare strings
int8_t equal = aria_string_eq(a, b);  // Returns 0 (false)
```

### Error Handling

```c
void aria_panic(const char* message) __attribute__((noreturn));
```

**When to Call:**
- Out of bounds array access
- Division by zero
- Failed pattern matches
- Allocation failures (automatically called by `aria_alloc`)
- Contract violations

**Behavior:**
- Prints formatted error message to stderr
- Terminates program with exit code 1
- Never returns

**Example Panic Output:**
```
==========================================
ARIA RUNTIME PANIC
==========================================

Error: Array index out of bounds

The program has encountered a fatal error
and cannot continue execution.
==========================================
```

### Entry Point

```c
extern void aria_main(void);
int main(int argc, char** argv);
```

The runtime provides a C `main()` function that:
1. Handles command-line arguments (future enhancement)
2. Calls the Aria program's main function (`aria_main`)
3. Returns 0 on success

The Aria compiler generates `aria_main()` which contains the compiled Aria code.

## Code Generation Examples

### Simple Print Statement

**Aria Code:**
```aria
fn main() {
    print("Hello, World!")
}
```

**Generated Assembly (pseudocode):**
```asm
.data
str_0: .asciz "Hello, World!"

.text
aria_main:
    push rbp
    mov rbp, rsp
    lea rdi, [str_0]      ; Load string address
    call aria_print_string
    pop rbp
    ret
```

### String Concatenation

**Aria Code:**
```aria
fn main() {
    let greeting = "Hello, " + "World!"
    print(greeting)
}
```

**Generated Assembly (pseudocode):**
```asm
.data
str_0: .asciz "Hello, "
str_1: .asciz "World!"

.text
aria_main:
    push rbp
    mov rbp, rsp

    ; Concatenate strings
    lea rdi, [str_0]
    lea rsi, [str_1]
    call aria_string_concat
    mov [rbp-8], rax      ; Save result

    ; Print result
    mov rdi, [rbp-8]
    call aria_print_string

    ; Free concatenated string
    mov rdi, [rbp-8]
    mov rsi, 0
    call aria_dealloc

    pop rbp
    ret
```

## Platform Support

### Linux (Native)
- Fully supported
- No special requirements
- Uses System V AMD64 ABI

### Windows
- Supported with MinGW-w64 or Cygwin
- Use `gcc` from MinGW-w64
- Executables have `.exe` extension

### macOS
- Supported
- Use system `clang` or `gcc`
- May need to adjust calling convention

## Performance Considerations

### Memory Allocation
- Uses standard `malloc`/`free`
- No pooling or custom allocator yet
- Future: size classes and arena allocation

### String Operations
- String concatenation allocates new memory
- Consider string builders for multiple concatenations
- String comparison is O(n) using `strcmp`

### I/O Buffering
- All print functions flush stdout
- Ensures immediate output visibility
- May impact performance in tight loops
- Future: add buffered print variants

## Debugging

### Viewing Generated Calls

Use `objdump` to inspect what runtime functions are called:

```bash
objdump -d program.o | grep aria_
```

### Checking Undefined Symbols

Before linking, check what runtime symbols are needed:

```bash
nm -u program.o | grep aria_
```

### Testing Runtime Separately

The included `test_example.c` can be built and run to verify runtime functionality:

```bash
make
gcc -c test_example.c -o test_example.o
gcc aria_runtime.o test_example.o -o test_example
./test_example
```

## Extending the Runtime

### Adding New Functions

1. Declare in `aria_runtime.h`:
```c
void aria_my_function(int64_t arg);
```

2. Implement in `aria_runtime.c`:
```c
void aria_my_function(int64_t arg) {
    // Implementation
}
```

3. Declare in `aria-codegen/src/runtime.rs`:
```rust
pub my_function: Option<FuncId>,
```

4. Add declaration in `RuntimeFunctions::declare_all`:
```rust
self.my_function = Some(declare_function(
    module,
    "aria_my_function",
    &[types::I64],
    None,
    call_conv,
)?);
```

5. Rebuild runtime:
```bash
cd crates/aria-runtime/c_runtime
make clean
make
```

## Troubleshooting

### "Runtime library not found" Error

**Problem:** Compiler can't find `aria_runtime.o`

**Solutions:**
1. Build the runtime: `cd crates/aria-runtime/c_runtime && make`
2. Specify path: `aria build program.aria --link --runtime /path/to/aria_runtime.o`
3. Install system-wide: `cd crates/aria-runtime/c_runtime && make install`

### Undefined Reference Errors

**Problem:** Linker complains about undefined `aria_*` functions

**Solutions:**
1. Ensure runtime is linked: `gcc aria_runtime.o program.o -o program`
2. Check that runtime was built: `ls aria_runtime.o`
3. Verify symbols: `nm aria_runtime.o | grep aria_`

### Segmentation Fault

**Common Causes:**
1. Null pointer passed to string functions
2. Calling `aria_dealloc` on non-allocated memory
3. Using freed memory
4. Stack overflow in generated code

**Debug Steps:**
1. Run with GDB: `gdb ./program`
2. Get backtrace: `bt`
3. Check register values
4. Verify object file correctness

### Performance Issues

**If program is slow:**
1. Compile runtime with `-O3` instead of `-O2`
2. Check for excessive allocations
3. Profile with `perf` or `valgrind`
4. Consider buffered I/O for bulk printing

## Future Enhancements

Planned improvements to the runtime:

1. **Garbage Collection**: Automatic memory management
2. **Exception Handling**: Try/catch support
3. **Advanced String Types**: Rope strings, string interning
4. **Async Runtime**: Support for Aria's concurrency features
5. **JIT Support**: Runtime code generation hooks
6. **Debugging Info**: Source location tracking, stack traces
7. **Cross-platform**: Better Windows/macOS support
8. **Performance**: Custom allocators, SIMD operations
