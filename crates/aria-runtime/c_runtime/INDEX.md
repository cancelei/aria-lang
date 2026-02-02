# Aria C Runtime - Documentation Index

Welcome to the Aria C Runtime Library documentation. This index will help you find the information you need.

## For New Users

Start here if you're new to the Aria runtime:

1. **[QUICKSTART.md](QUICKSTART.md)** - Get up and running in 5 minutes
   - Quick setup instructions
   - Basic compilation commands
   - Common troubleshooting

2. **[README.md](README.md)** - Complete overview
   - File descriptions
   - Building and linking instructions
   - Runtime functions reference
   - Usage examples

## For Developers

Detailed technical documentation:

1. **[INTEGRATION.md](INTEGRATION.md)** - Deep integration guide
   - Architecture overview
   - Compilation workflow
   - Code generation examples
   - Platform support
   - Performance considerations
   - Debugging techniques
   - Extending the runtime

## Source Files

Core implementation files:

- **[aria_runtime.h](aria_runtime.h)** - Header file with all function declarations
- **[aria_runtime.c](aria_runtime.c)** - Complete runtime implementation
- **[test_example.c](test_example.c)** - Comprehensive test and example program

## Build System

Build configuration and scripts:

- **[Makefile](Makefile)** - GNU Make build system
  - Simple `make` to build
  - `make link` to link programs
  - `make install` for system-wide installation

- **[build.sh](build.sh)** - Shell script alternative
  - `./build.sh build` to build runtime
  - `./build.sh link` to link programs
  - More portable than Make

## Quick Reference

### Common Tasks

| Task | Command |
|------|---------|
| Build runtime | `make` or `./build.sh build` |
| Compile Aria program | `aria build program.aria` |
| Link to executable | `aria build program.aria --link` |
| Manual linking | `gcc aria_runtime.o program.o -o program` |
| Run tests | `./test_example` |
| Clean build | `make clean` |
| Install system-wide | `make install` |

### File Organization

```
c_runtime/
├── Documentation
│   ├── INDEX.md          ← You are here
│   ├── QUICKSTART.md     ← Start here for quick setup
│   ├── README.md         ← Complete overview
│   └── INTEGRATION.md    ← Deep technical guide
│
├── Source Code
│   ├── aria_runtime.h    ← Header declarations
│   ├── aria_runtime.c    ← Implementation
│   └── test_example.c    ← Test program
│
├── Build System
│   ├── Makefile          ← GNU Make
│   ├── build.sh          ← Shell script
│   └── .gitignore        ← Git ignore rules
│
└── Build Artifacts (after 'make')
    ├── aria_runtime.o    ← Runtime library
    ├── test_example.o    ← Test object file
    └── test_example      ← Test executable
```

## Runtime API Summary

### Print Functions
```c
void aria_print_int(int64_t value);
void aria_print_float(double value);
void aria_print_string(const char* str);
void aria_print_bool(int8_t value);
void aria_print_newline(void);
```

### Memory Management
```c
void* aria_alloc(int64_t size);
void aria_dealloc(void* ptr, int64_t size);
```

### String Operations
```c
char* aria_string_concat(const char* a, const char* b);
int8_t aria_string_eq(const char* a, const char* b);
```

### Error Handling
```c
void aria_panic(const char* message) __attribute__((noreturn));
```

### Entry Point
```c
extern void aria_main(void);  // Implemented by compiled Aria code
int main(int argc, char** argv);  // Provided by runtime
```

## Compilation Flow

```
Aria Source (.aria)
        ↓
    [Aria Compiler]
        ↓
  Aria Object (.o) ──┐
                     │
  Runtime Library ───┼→ [GCC Linker] → Executable
  (aria_runtime.o)   │
```

## Getting Help

### Documentation Hierarchy

1. **QUICKSTART.md** - If you just want to build and run
2. **README.md** - If you need detailed instructions
3. **INTEGRATION.md** - If you're working on the compiler
4. **Source code** - If you need implementation details

### Troubleshooting

Check these in order:

1. **QUICKSTART.md** - Common issues and solutions
2. **README.md** - Troubleshooting section
3. **INTEGRATION.md** - Debugging section with advanced techniques

### Examples

- **test_example.c** - Shows all runtime functions in action
- **INTEGRATION.md** - Contains code generation examples
- **README.md** - Has usage examples

## Contributing

If you want to extend or improve the runtime:

1. Read **INTEGRATION.md** section on "Extending the Runtime"
2. Study the source code in **aria_runtime.c**
3. Add your function following the existing patterns
4. Update **aria_runtime.h** with declarations
5. Test with **test_example.c**

## Version Information

- **Runtime Version**: 0.1.0 (initial implementation)
- **Compatibility**: Aria Compiler 0.1.0+
- **C Standard**: C11
- **Platforms**: Linux, macOS, Windows (MinGW)

## License

See the main Aria project LICENSE file.

---

**Next Steps:**
- New user? → Start with [QUICKSTART.md](QUICKSTART.md)
- Need details? → Read [README.md](README.md)
- Deep dive? → Study [INTEGRATION.md](INTEGRATION.md)
