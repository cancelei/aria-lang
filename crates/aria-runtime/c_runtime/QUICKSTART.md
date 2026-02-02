# Aria C Runtime - Quick Start

## Setup (One-time)

```bash
cd crates/aria-runtime/c_runtime
make
```

This creates `aria_runtime.o`.

## Compile Aria Programs

### Option 1: Automatic Linking (Easiest)

```bash
aria build myprogram.aria --link
./myprogram
```

### Option 2: Manual Linking

```bash
aria build myprogram.aria           # Creates myprogram.o
gcc aria_runtime.o myprogram.o -o myprogram
./myprogram
```

### Option 3: Using Build Script

```bash
./build.sh link myprogram.o myprogram
./myprogram
```

## Runtime Functions Reference

| Function | Signature | Purpose |
|----------|-----------|---------|
| `aria_print_int` | `(int64_t)` | Print integer |
| `aria_print_float` | `(double)` | Print float |
| `aria_print_string` | `(const char*)` | Print string |
| `aria_print_bool` | `(int8_t)` | Print boolean |
| `aria_print_newline` | `()` | Print newline |
| `aria_alloc` | `(int64_t) -> void*` | Allocate memory |
| `aria_dealloc` | `(void*, int64_t)` | Free memory |
| `aria_string_concat` | `(const char*, const char*) -> char*` | Concatenate strings |
| `aria_string_eq` | `(const char*, const char*) -> int8_t` | Compare strings |
| `aria_panic` | `(const char*)` | Runtime error (exits) |

## Common Commands

```bash
# Build runtime
make

# Clean build files
make clean

# Install system-wide
make install PREFIX=/usr/local

# Link program
make link PROGRAM=prog.o OUTPUT=prog

# Test runtime
gcc -c test_example.c -o test_example.o
gcc aria_runtime.o test_example.o -o test_example
./test_example
```

## Directory Structure

```
crates/aria-runtime/c_runtime/
├── aria_runtime.h       # Header file
├── aria_runtime.c       # Implementation
├── aria_runtime.o       # Compiled library (after 'make')
├── Makefile            # Build configuration
├── build.sh            # Alternative build script
├── test_example.c      # Test program
├── README.md           # Full documentation
├── INTEGRATION.md      # Integration guide
└── QUICKSTART.md       # This file
```

## Troubleshooting

**"Runtime library not found"**
- Run `make` in `crates/aria-runtime/c_runtime/`
- Or specify path: `aria build prog.aria --link --runtime /path/to/aria_runtime.o`

**Undefined reference errors**
- Make sure to link with `aria_runtime.o`
- Check: `nm aria_runtime.o | grep aria_`

**Segmentation fault**
- Run with debugger: `gdb ./program`
- Check for null pointers in generated code
- Verify memory management

## Examples

### Simple Hello World

```c
// This is what compiled Aria code looks like
#include "aria_runtime.h"

void aria_main(void) {
    aria_print_string("Hello, World!");
    aria_print_newline();
}
```

Compile:
```bash
gcc -c aria_runtime.c -o aria_runtime.o
gcc -c hello.c -o hello.o
gcc aria_runtime.o hello.o -o hello
./hello
```

### String Operations

```c
#include "aria_runtime.h"

void aria_main(void) {
    const char* a = "Hello, ";
    const char* b = "World!";

    char* result = aria_string_concat(a, b);
    aria_print_string(result);
    aria_print_newline();

    aria_dealloc(result, 0);
}
```

### Error Handling

```c
#include "aria_runtime.h"

void aria_main(void) {
    if (some_error_condition) {
        aria_panic("Something went wrong!");
    }
}
```

## Next Steps

- Read **README.md** for full documentation
- Read **INTEGRATION.md** for compiler integration details
- Study **test_example.c** for usage examples
- Check **aria_runtime.h** for complete API
