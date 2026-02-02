# Type Inference Improvements

## Summary

This document describes comprehensive type inference improvements made to the Aria compiler's MIR (Mid-level Intermediate Representation) lowering phase. These changes eliminate the problematic default of `MirType::Int` for unknown types and leverage the existing type inference infrastructure.

## Problem Statement

The compiler previously defaulted to `MirType::Int` in several critical locations when type information was not explicitly provided:

1. **Variable bindings** (`var` statements) without type annotations
2. **Constant bindings** (`const` statements) without type annotations
3. **Let bindings** with projections (field access, array indexing)
4. **Function return types** when function lookup failed

This caused bugs where:
- String values were incorrectly typed as Int
- Boolean values were incorrectly typed as Int
- Float values were incorrectly typed as Int
- Complex expressions had incorrect types propagated

## Solution Overview

### Infrastructure Already in Place

The codebase already had a robust type inference system:

- **Type unification algorithm** (Hindley-Milner style) in `aria-types/src/lib.rs`
  - `TypeInference::unify()` - unifies two types and produces substitutions
  - `TypeInference::occurs_check()` - prevents infinite recursive types
  - `TypeInference::apply()` - applies substitutions with performance optimizations

- **Type inference helpers** in `aria-mir/src/lower_expr.rs`
  - `infer_operand_type()` - infers type from operands (constants, places)
  - `infer_place_type()` - infers type from memory locations with projections

### Changes Made

#### 1. Made Type Inference Functions Public

**File**: `crates/aria-mir/src/lower_expr.rs`

Made `infer_operand_type()` and `infer_place_type()` public so they can be used across the MIR lowering modules:

```rust
pub fn infer_place_type(ctx: &FunctionLoweringContext, place: &Place) -> MirType
pub fn infer_operand_type(ctx: &FunctionLoweringContext, operand: &Operand) -> MirType
```

These functions properly handle:
- Constants (Int, Float, String, Bool, Char, etc.)
- Place projections (field access, array indexing, dereferencing)
- Struct field type lookups
- Tuple element type extraction
- Array element type inference

#### 2. Fixed Variable Binding Type Inference

**File**: `crates/aria-mir/src/lower_stmt.rs`

**Before**:
```rust
fn lower_var(...) {
    let mir_ty = if let Some(t) = ty {
        ctx.ctx.lower_type(t)
    } else {
        match &val {
            Operand::Move(place) | Operand::Copy(place) => {
                if place.projection.is_empty() {
                    ctx.func.locals[place.local.0 as usize].ty.clone()
                } else {
                    MirType::Int // TODO: Handle projections
                }
            }
            _ => MirType::Int, // TODO: Full type inference
        }
    };
}
```

**After**:
```rust
fn lower_var(...) {
    let mir_ty = if let Some(t) = ty {
        ctx.ctx.lower_type(t)
    } else {
        // Infer type from the value using the type inference system
        infer_operand_type(ctx, &val)
    };
}
```

#### 3. Fixed Constant Binding Type Inference

**File**: `crates/aria-mir/src/lower_stmt.rs`

**Before**:
```rust
fn lower_const(...) {
    let mir_ty = ty
        .map(|t| ctx.ctx.lower_type(t))
        .unwrap_or(MirType::Int);
}
```

**After**:
```rust
fn lower_const(...) {
    let mir_ty = if let Some(t) = ty {
        ctx.ctx.lower_type(t)
    } else {
        // Infer type from the value using the type inference system
        infer_operand_type(ctx, &val)
    };
}
```

#### 4. Fixed Let Binding Type Inference

**File**: `crates/aria-mir/src/lower_stmt.rs`

**Before**: Duplicated type inference logic inline with TODO comments about handling projections

**After**: Uses `infer_operand_type()` which properly handles all projections including:
- Field access on structs and tuples
- Array indexing
- Dereferencing

#### 5. Improved Function Return Type Defaults

**File**: `crates/aria-mir/src/lower_expr.rs`

**Before**:
```rust
let return_ty = ctx.ctx.get_function(&fn_id)
    .map(|f| f.return_ty.clone())
    .unwrap_or(MirType::Int);
```

**After**:
```rust
// If the function is not found (shouldn't happen in correct code),
// default to Unit rather than assuming Int
let return_ty = ctx.ctx.get_function(&fn_id)
    .map(|f| f.return_ty.clone())
    .unwrap_or(MirType::Unit);
```

#### 6. Improved Function Parameter Type Handling

**File**: `crates/aria-mir/src/lower.rs`

Added documentation explaining that parameters without type annotations default to Unit (rather than Int), with a note that stricter enforcement or bidirectional inference from call sites should be added in the future.

## Impact

### TODOs Eliminated

The following TODO comments were resolved:
- `TODO: Handle projections` (2 instances in lower_stmt.rs)
- `TODO: Full type inference` (1 instance in lower_stmt.rs)
- `TODO: Better type inference` (1 instance in lower.rs)

### Type Safety Improvements

1. **Correct type inference for constants**:
   - `let x = 42` → `Int` (not Int by default, but by inference)
   - `let y = 3.14` → `Float` (not Int!)
   - `let s = "hello"` → `String` (not Int!)
   - `let b = true` → `Bool` (not Int!)

2. **Correct type inference for projections**:
   - `let field = obj.x` → correctly looks up field type from struct definition
   - `let elem = arr[0]` → correctly extracts element type from array
   - `let val = *ptr` → correctly dereferences pointer type

3. **Better error messages**: Type mismatches now occur with the correct types, making errors easier to understand

## Testing

All existing tests pass:
```
test result: ok. 90 passed; 0 failed; 0 ignored; 0 measured
```

### Test Example

The following code now correctly infers all types:

```aria
fn main()
  # Test type inference for let bindings
  let x = 42           # Infers Int
  let y = 3.14         # Infers Float
  let z = "hello"      # Infers String
  let b = true         # Infers Bool
  let c = 'a'          # Infers Char

  # Test type inference for var bindings
  var a = 100          # Infers Int
  var f = 2.5          # Infers Float

  # Test type inference for const bindings
  const PI = 3.14159   # Infers Float
  const NAME = "Aria"  # Infers String

  # Test tuple and array inference
  let tuple = (1, 2.0, "three")  # Infers (Int, Float, String)
  let array = [1, 2, 3]           # Infers Array[Int]
end
```

## Future Work

While these improvements significantly enhance type inference, there are still opportunities for further enhancement:

### 1. Bidirectional Type Checking for Function Calls

Currently, function parameters without type annotations default to Unit. A better approach would be:
- Infer parameter types from call sites (bidirectional type checking)
- Require explicit type annotations in public APIs
- Allow inference in local/private functions

### 2. Generic Type Support

The type system already has infrastructure for type variables and unification, but MIR lowering doesn't fully utilize it yet:
- Type parameters in function signatures
- Constraint solving for generic bounds
- Monomorphization during lowering

### 3. Method Call Type Resolution

Method calls currently need better integration with:
- Trait/impl definitions
- Type inference for method arguments
- Return type propagation

### 4. Advanced Pattern Matching

Pattern matching could benefit from:
- Type narrowing based on pattern structure
- Exhaustiveness checking using type information
- Better integration with enum variant types

## Performance Considerations

The type inference functions include several optimizations:

1. **Short-circuit evaluation**: `needs_apply()` checks avoid expensive substitution when not needed
2. **Efficient projection handling**: Direct type lookups for struct fields and tuple elements
3. **Cached lookups**: Function and struct definitions are looked up once and cached

No performance regressions were observed in testing.

## Conclusion

These changes represent a significant improvement in the Aria compiler's type inference capabilities. By leveraging existing infrastructure and eliminating arbitrary defaults, the compiler now produces more accurate type information, leading to:

- Better error messages
- Fewer type-related bugs
- Improved developer experience
- A solid foundation for future enhancements like generics and bidirectional type checking
