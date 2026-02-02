# Aria C Runtime Library

This directory contains the C runtime library for compiled Aria programs. The runtime provides essential functions for I/O, memory management, string operations, and error handling.

## Files

- **aria_runtime.h** - Header file with runtime function declarations
- **aria_runtime.c** - Implementation of runtime functions
- **Makefile** - GNU Make build configuration
- **build.sh** - Alternative shell script for building and linking
- **README.md** - This file

## Building the Runtime

### Using Make

```bash
make
```

This creates `aria_runtime.o` which can be linked with compiled Aria programs.

### Using the Build Script

```bash
./build.sh build
```

## Linking Aria Programs

### Using Make

```bash
make link PROGRAM=myprogram.o OUTPUT=myprogram
```

### Using the Build Script

```bash
./build.sh link myprogram.o myprogram
```

### Manual Linking

```bash
gcc aria_runtime.o myprogram.o -o myprogram
```

## Runtime Functions

### Print Functions

- `void aria_print_int(int64_t value)` - Print an integer
- `void aria_print_float(double value)` - Print a floating-point number
- `void aria_print_string(const char* str)` - Print a string
- `void aria_print_bool(int8_t value)` - Print a boolean ("true" or "false")
- `void aria_print_newline(void)` - Print a newline

### Memory Management

- `void* aria_alloc(int64_t size)` - Allocate memory (wrapper around malloc)
- `void aria_dealloc(void* ptr, int64_t size)` - Free memory (wrapper around free)

### String Operations

- `char* aria_string_concat(const char* a, const char* b)` - Concatenate two strings
- `int8_t aria_string_eq(const char* a, const char* b)` - Compare strings for equality

### Error Handling

- `void aria_panic(const char* message)` - Print error message and exit program

## Example Usage

Given an Aria program compiled to `hello.o`:

```bash
# Build the runtime
make

# Link the program
make link PROGRAM=hello.o OUTPUT=hello

# Run the program
./hello
```

Or using the build script:

```bash
# Build and link in one step
./build.sh link hello.o hello

# Run the program
./hello
```

## Installation

To install the runtime library system-wide:

```bash
make install PREFIX=/usr/local
```

This installs:
- `aria_runtime.o` to `$PREFIX/lib/aria/`
- `aria_runtime.h` to `$PREFIX/include/aria/`

## Integration with Aria Compiler

The Aria compiler's `build` command produces object files that expect these runtime functions to be available. The compiler will suggest the linking command:

```bash
aria build myprogram.aria -o myprogram.o
gcc myprogram.o aria_runtime.o -o myprogram
```

## Implementation Details

- All print functions flush stdout after printing to ensure output appears immediately
- Memory allocation failures trigger a panic rather than returning NULL
- String functions treat NULL pointers as empty strings where appropriate
- The `main()` function wrapper calls `aria_main()`, which is implemented by the compiled Aria code
- Error messages are printed to stderr with clear formatting

## Future Enhancements

Potential improvements to the runtime:

- Command-line argument passing to Aria programs
- More sophisticated memory allocator with size classes
- String interning for better performance
- Garbage collection support
- Stack trace information in panics
- Debug mode with allocation tracking
