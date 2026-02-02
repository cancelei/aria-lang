# Aria Build Command Implementation

## Overview

The `aria build` command has been successfully implemented to compile Aria programs to native executables using the Cranelift code generation backend.

## Command Structure

```bash
aria build <file.aria> [options]
```

## Options

- `-o, --output <path>` - Output file path (defaults to input filename with appropriate extension)
- `-l, --link` - Link with runtime to produce executable (default: object file only)
- `-r, --release` - Build with optimizations (applies MIR optimization passes)
- `--runtime <path>` - Path to aria_runtime.o (auto-detected if not specified)
- `--lib` - Compile as library (default: binary)
- `-L, --lib-path <paths>` - Additional module search paths
- `--target <target>` - Target platform: `native` or `wasm32` (default: native)

## Compilation Pipeline

The build command follows this pipeline:

1. **Parse Source** - Parse the `.aria` file into an AST
2. **Type Check** - Perform type checking with module-aware imports/exports
3. **Lower to MIR** - Convert AST to Mid-level Intermediate Representation
4. **Optimize** (if `--release` flag) - Apply MIR optimization passes:
   - Constant folding
   - Dead code elimination
   - Copy propagation
   - CFG simplification
5. **Generate Code** - Compile MIR to native code via Cranelift
6. **Link** (if `--link` flag) - Link object file with Aria runtime using gcc

## Examples

### 1. Compile to Object File

```bash
aria build hello.aria
# Output: hello.o
```

### 2. Compile and Link to Executable

```bash
aria build hello.aria --link
# Output: hello (executable)
./hello
```

### 3. Optimized Release Build

```bash
aria build hello.aria --link --release -o hello_optimized
# Applies aggressive MIR optimizations
./hello_optimized
```

### 4. WASM Target

```bash
aria build hello.aria --target wasm32
# Output: hello.wasm
wasmtime hello.wasm
```

### 5. Custom Output Path

```bash
aria build hello.aria --link -o bin/my_program
./bin/my_program
```

## Test Results

### Test 1: Hello World

**Source** (`test_hello_world.aria`):
```aria
fn main()
  print("Hello, World!")
end
```

**Build & Run**:
```bash
$ aria build test_hello_world.aria --link
Compiled 1 module(s)
  - test_hello_world (/home/cancelei/Projects/aria-lang/test_hello_world.aria)
  Type-checked: test_hello_world (0 exports)
Compiled and linked test_hello_world.aria -> test_hello_world
Executable ready to run: ./test_hello_world

$ ./test_hello_world
Hello, World!
```

### Test 2: Math Operations

**Source** (`test_math.aria`):
```aria
fn add(a: Int, b: Int) -> Int
  a + b
end

fn multiply(a: Int, b: Int) -> Int
  a * b
end

fn main()
  let x = 5
  let y = 3
  let sum = add(x, y)
  let product = multiply(x, y)

  print("5 + 3 = ")
  print(sum)
  print("\n5 * 3 = ")
  print(product)
end
```

**Build & Run**:
```bash
$ aria build test_math.aria --link
Compiled 1 module(s)
  - test_math (/home/cancelei/Projects/aria-lang/test_math.aria)
  Type-checked: test_math (0 exports)
Compiled and linked test_math.aria -> test_math
Executable ready to run: ./test_math

$ ./test_math
5 + 3 = 8
5 * 3 = 15
```

### Test 3: Optimized Build

**Build**:
```bash
$ aria build test_hello_world.aria --link --release -o test_hello_world_optimized
Compiled 1 module(s)
  - test_hello_world (/home/cancelei/Projects/aria-lang/test_hello_world.aria)
  Type-checked: test_hello_world (0 exports)
Applying optimizations...
Compiled and linked test_hello_world.aria -> test_hello_world_optimized
Executable ready to run: ./test_hello_world_optimized

$ ./test_hello_world_optimized
Hello, World!
```

### Test 4: WebAssembly Target

**Build**:
```bash
$ aria build test_hello_world.aria --target wasm32
Compiled 1 module(s)
  - test_hello_world (/home/cancelei/Projects/aria-lang/test_hello_world.aria)
  Type-checked: test_hello_world (0 exports)
Compiled test_hello_world.aria -> test_hello_world.wasm

WASM module ready! Run with:
  wasmtime test_hello_world.wasm
```

## Error Reporting

The build command provides detailed error reporting:

### Parse Errors
- Shows exact line and column of syntax errors
- Provides helpful hints and suggestions

### Type Errors
- Displays type mismatches with context
- Suggests type conversion functions when applicable
- "Did you mean" suggestions for typos

### MIR Lowering Errors
- Reports unsupported features
- Shows span information for debugging

### Codegen Errors
- Reports code generation failures
- Provides context for debugging

## Architecture

### File Locations

- **Compiler**: `/home/cancelei/Projects/aria-lang/crates/aria-compiler/src/main.rs`
- **MIR**: `/home/cancelei/Projects/aria-lang/crates/aria-mir/`
- **Codegen**: `/home/cancelei/Projects/aria-lang/crates/aria-codegen/`
- **Runtime**: `/home/cancelei/Projects/aria-lang/crates/aria-runtime/c_runtime/`

### Key Components

1. **aria-parser** - Lexing and parsing
2. **aria-types** - Type checking
3. **aria-modules** - Module resolution and compilation
4. **aria-mir** - MIR representation and optimization
5. **aria-codegen** - Cranelift backend for code generation
6. **aria-runtime** - C runtime library for I/O, memory, etc.

### Output Formats

- **Native (default)**: ELF object file (.o) on Linux
- **Linked executable**: Native executable (no extension on Unix, .exe on Windows)
- **WebAssembly**: .wasm module

## Implementation Details

### Changes Made

1. **Enhanced Build Command** (`crates/aria-compiler/src/main.rs`)
   - Added `--release` flag for optimizations
   - Updated documentation strings
   - Integrated MIR optimization passes

2. **Fixed MIR Lowering** (`crates/aria-mir/src/lower_expr.rs`)
   - Fixed import issues with `FxHashMap`
   - Fixed mutability issues in pattern matching
   - Simplified lambda lowering (marked as unsupported for now)

3. **Fixed Codegen** (`crates/aria-codegen/src/cranelift_backend.rs`)
   - Added missing `alloc`, `free`, and `string_concat` runtime function refs
   - These are needed for struct/enum allocation and string operations

4. **Created Test Programs**
   - `test_hello_world.aria` - Basic hello world
   - `test_math.aria` - Function calls and arithmetic

## Supported Features

- ✅ Function definitions
- ✅ Integer arithmetic
- ✅ String literals
- ✅ Print statements
- ✅ Variable bindings
- ✅ Function calls
- ✅ Type checking
- ✅ Multi-module compilation
- ✅ MIR optimizations
- ✅ Native code generation
- ✅ WebAssembly compilation
- ✅ Runtime linking

## Known Limitations

- ❌ Recursive functions (scoping issue in type checker)
- ❌ Lambda expressions (requires closure conversion)
- ❌ Advanced pattern matching
- ❌ Generics (in progress)
- ❌ Effect system (partial support)

## Future Enhancements

1. Add debug symbol generation (DWARF)
2. Implement incremental compilation
3. Add compilation caching
4. Profile-guided optimizations
5. Link-time optimization (LTO)
6. Better error recovery
7. Parallel compilation
8. Static library generation (`.a` files)

## Performance Notes

The `--release` flag enables aggressive MIR optimizations which can significantly improve runtime performance:

- **Constant folding**: Evaluates constant expressions at compile time
- **Dead code elimination**: Removes unreachable code
- **Copy propagation**: Eliminates redundant copies
- **CFG simplification**: Simplifies control flow graphs

For production builds, always use `--release`.
