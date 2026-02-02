# Cross-Module Type Checking

This document describes the cross-module type checking functionality implemented in the Aria compiler.

## Overview

The Aria compiler now supports full type checking across module boundaries, ensuring type safety when using imports. This includes:

1. **Symbol table export/import mechanism** - Module exports are collected and shared
2. **Type signature validation** - Function and type signatures are preserved across modules
3. **Import verification** - Checks that imported items exist and are public
4. **Cross-module type inference** - Types flow correctly between modules

## Implementation

### Module Compilation Flow

1. **Module Resolution** (`aria-modules` crate)
   - Parses entry point and discovers dependencies
   - Builds dependency graph
   - Topologically sorts modules (dependencies first)

2. **Type Checking** (`aria-types` crate)
   - For each module in dependency order:
     - Register exports from previously type-checked modules
     - Type check the current module (including import validation)
     - Extract and store this module's exports

### Key Components

#### ModuleExports Type

Defined in `aria-types/src/lib.rs`:

```rust
pub struct ModuleExport {
    /// The type of the exported symbol
    pub ty: Type,
    /// Whether this is a type definition (vs. a value)
    pub is_type: bool,
}

pub type ModuleExports = FxHashMap<String, ModuleExport>;
```

#### TypeChecker Methods

- `register_module_exports(module_name: String, exports: ModuleExports)` - Register exports from a dependency
- `process_imports(program: &Program)` - Validate and process all imports
- `extract_exports(program: &Program) -> ModuleExports` - Extract exports from a type-checked module

### Error Detection

The type checker detects and reports:

1. **Import of non-existent symbol**
   ```
   Error: symbol `nonexistent` is not exported from module `lib`
   Help: add `pub` visibility to `nonexistent` in module `lib`, or use a different symbol
   ```

2. **Import of private symbol**
   ```
   Error: symbol `private_func` is not exported from module `lib2`
   Help: add `pub` visibility to `private_func` in module `lib2`, or use a different symbol
   ```

3. **Type mismatch across modules**
   ```
   Error: type mismatch
   expected `String`, found `Int`
   ```

4. **Wrong number of arguments**
   ```
   Error: wrong number of type arguments
   expected 2 type arguments, found 1
   ```

## Example Usage

### Library Module (`lib.aria`)

```aria
pub fn add(x: Int, y: Int) -> Int = x + y

pub fn multiply(x: Int, y: Int) -> Int = x * y

fn private_helper() = 42  # Not exported
```

### Main Module (`main.aria`)

```aria
import lib::{add, multiply}

fn main() -> Int
  let sum = add(10, 20)
  let product = multiply(5, 6)
  sum + product
end
```

### Compilation

```bash
aria build main.aria -L path/to/modules
```

The compiler will:
1. Compile `lib.aria` first (dependency)
2. Extract exports: `add: (Int, Int) -> Int`, `multiply: (Int, Int) -> Int`
3. Type check `main.aria` with access to `lib` exports
4. Validate that imported symbols exist and types match

## Testing

Manual testing has verified:

1. ✅ **Successful cross-module type checking** - Correct imports and usage work
2. ✅ **Import validation** - Non-existent symbols are caught
3. ✅ **Private item protection** - Private symbols cannot be imported
4. ✅ **Type signature preservation** - Function types are correctly transferred
5. ✅ **Argument count checking** - Wrong number of arguments detected

### Test Commands

```bash
# Create test modules
mkdir -p /tmp/aria-test

# lib.aria
echo 'pub fn add(x: Int, y: Int) -> Int = x + y' > /tmp/aria-test/lib.aria

# main.aria - correct usage
echo 'import lib::{add}
fn main() -> Int = add(10, 20)' > /tmp/aria-test/main.aria

# Should succeed
aria build /tmp/aria-test/main.aria -L /tmp/aria-test

# Test type error
echo 'import lib::{add}
fn main() -> Int = add(10)' > /tmp/aria-test/main_error.aria

# Should fail with "wrong number of arguments"
aria build /tmp/aria-test/main_error.aria -L /tmp/aria-test
```

## Future Enhancements

1. **Type re-exports** - `pub use other_module::Type`
2. **Generic type preservation** - Better handling of generic types across modules
3. **Module interface files** - `.ariai` files for faster compilation
4. **Incremental type checking** - Only re-check changed modules
5. **Better error messages** - Show definition location in other modules

## Related Files

- `crates/aria-modules/src/lib.rs` - Module compiler orchestration
- `crates/aria-types/src/lib.rs` - Type checker with cross-module support
- `crates/aria-compiler/src/main.rs` - Integration of module system and type checker
- `MODULE_SYSTEM_SUMMARY.md` - Overall module system architecture
