# Generics and Polymorphism - Design Document

## Overview

This document outlines the design and implementation of generic types and polymorphism in the Aria programming language. The goal is to enable functions and data structures to work with multiple types while maintaining type safety.

## Current State

### What Already Exists

1. **AST Support** (✓ Complete)
   - `GenericParams` - container for generic parameters
   - `GenericParam` - individual type parameter with bounds
   - `TraitBound` - trait constraints on type parameters
   - `WhereClause` - additional constraints
   - `TypeExpr::Generic` - generic type usage

2. **Parser Support** (✓ Complete)
   - `parse_optional_generic_params()` - parses `<T, U: Display>`
   - `parse_generic_param()` - parses individual type parameters
   - `parse_trait_bounds()` - parses trait bounds like `T: Display + Clone`
   - `parse_optional_where_clause()` - parses `where` clauses

3. **Type System Infrastructure** (✓ Partial)
   - `Type::Var(TypeVar)` - type variables for inference
   - `TypeScheme` - polymorphic type schemes with `type_params` and `type_param_defs`
   - `TypeParamDef` - type parameter definitions with bounds
   - `TypeBound` - trait bounds representation
   - `Type::Named { name, type_args }` - instantiated generic types

### What's Missing

1. **Type Parameter Environment**
   - Need to track type parameters in scope during type checking
   - Type parameter substitution mechanism
   - Type variable generation for generic instantiation

2. **Generic Function Support**
   - Generic function type checking with type parameters in scope
   - Type argument inference at call sites
   - Trait bound verification

3. **Generic Struct/Enum Support**
   - Validation that struct/enum uses match declarations
   - Field type resolution with type parameters

4. **Monomorphization/Code Generation**
   - Decision: **Monomorphization** (Rust-style) vs Type Erasure (Java-style)
   - Generate specialized versions for each concrete type usage
   - MIR lowering for generic functions and types

## Design Decisions

### 1. Monomorphization Strategy

**Decision: Use Monomorphization (Rust/C++ approach)**

**Rationale:**
- Better runtime performance (no boxing/unboxing, no vtables needed)
- Enables optimizations specific to concrete types
- More predictable memory layout
- Aligns with Aria's systems programming goals

**Trade-offs:**
- Larger binary size (acceptable for most use cases)
- Longer compilation time
- More complex implementation

**Alternative Considered:** Type erasure (Java-style)
- Pros: Smaller binaries, faster compilation
- Cons: Runtime overhead, requires boxing primitives, less optimization opportunity

### 2. Type Parameter Inference

**Strategy: Bidirectional type checking with constraint solving**

Generic functions will use:
1. **Explicit type arguments**: `foo::<Int>(42)`
2. **Inferred from arguments**: `foo(42)` → infers `T = Int`
3. **Inferred from context**: `let x: Vec<Int> = create_vec()`

### 3. Trait Bounds Checking

**Strategy: Early validation during type checking**

- Validate trait bounds when generic types are instantiated
- Store trait implementations in `TypeChecker::trait_impls`
- Check bounds before monomorphization

## Implementation Plan

### Phase 1: Type Checking Infrastructure (HIGH PRIORITY)

#### 1.1 Type Parameter Environment

```rust
// Add to TypeChecker
type_param_scopes: Vec<FxHashMap<String, TypeVar>>,
```

This tracks type parameters currently in scope, mapping names like "T" to type variables.

#### 1.2 Enhanced resolve_type

Update `resolve_type()` to:
- Check `type_param_scopes` for type parameter names
- Return `Type::Var(var)` for type parameters
- Handle type parameter references in function/struct bodies

#### 1.3 Generic Function Type Checking

Update `check_function()` to:
1. Enter type parameter scope from `func.generic_params`
2. Create fresh type variables for each parameter
3. Resolve parameter/return types with type params in scope
4. Exit type parameter scope after checking body

#### 1.4 Generic Struct/Enum Type Checking

Update `check_struct()` and `check_enum()` to:
1. Enter type parameter scope
2. Validate field types use declared type parameters
3. Store type parameter info in `generic_type_params`

### Phase 2: Call Site Inference (HIGH PRIORITY)

#### 2.1 Type Argument Inference

Add to `TypeChecker`:
```rust
fn infer_type_arguments(
    &mut self,
    func_name: &str,
    type_params: &[TypeParamDef],
    args: &[ast::Expr],
    expected_param_types: &[Type],
) -> TypeResult<Vec<Type>>
```

This function:
1. Creates fresh type variables for each type parameter
2. Unifies argument types with expected parameter types
3. Solves for type variables
4. Returns concrete types

#### 2.2 Enhanced Function Call Checking

Update call expression checking to:
1. Look up function's generic parameters
2. If generic, infer type arguments from call arguments
3. Substitute type arguments into function signature
4. Check call with instantiated signature

### Phase 3: Trait Bounds Validation (MEDIUM PRIORITY)

#### 3.1 Bounds Checking at Instantiation

Add:
```rust
fn check_trait_bounds(
    &self,
    type_args: &[(String, Type)], // (param_name, concrete_type)
    bounds: &[TypeParamDef],
    span: Span,
) -> TypeResult<()>
```

This validates that concrete types satisfy trait bounds.

#### 3.2 Enhanced Trait Implementation Lookup

Improve trait implementation checking to:
- Support generic trait implementations
- Check trait bounds recursively

### Phase 4: Monomorphization (MEDIUM PRIORITY)

#### 4.1 MIR Monomorphization Pass

Add new module: `crates/aria-mir/src/monomorphize.rs`

```rust
pub struct Monomorphizer {
    /// Map of (generic_function, type_args) -> monomorphized_function_id
    instantiations: FxHashMap<(FunctionId, Vec<MirType>), FunctionId>,
    /// Queue of functions to monomorphize
    work_queue: Vec<(FunctionId, Vec<MirType>)>,
}
```

Process:
1. Start with non-generic entry points (main)
2. When encountering generic function call:
   - Extract concrete type arguments
   - Check if already instantiated
   - If not, create specialized version and queue it
3. Repeat until work queue is empty

#### 4.2 Type Substitution in MIR

Add:
```rust
fn substitute_types(
    func: &MirFunction,
    type_params: &[String],
    type_args: &[MirType],
) -> MirFunction
```

Replaces all type parameter references with concrete types.

### Phase 5: Generic Builtins (MEDIUM PRIORITY)

#### 5.1 Update Standard Library

Convert current functions to generic versions:

```aria
# Before (current)
fn abs(x: Int) -> Int
fn abs_int(x: Int) -> Int
fn abs_float(x: Float) -> Float

# After (with generics)
fn abs<T: Numeric>(x: T) -> T
  # Implementation...
end

fn min<T: Ord>(a: T, b: T) -> T
  if a < b then a else b end
end

fn max<T: Ord>(a: T, b: T) -> T
  if a > b then a else b end
end
```

#### 5.2 Generic Collection Types

```aria
struct Vec<T>
  data: [T]
  length: Int
  capacity: Int
end

impl<T> Vec<T>
  fn push(mut self, value: T) -> ()
    # Implementation...
  end

  fn pop(mut self) -> T?
    # Implementation...
  end

  fn first(self) -> T?
    # Implementation...
  end

  fn last(self) -> T?
    # Implementation...
  end
end
```

### Phase 6: Advanced Features (LOW PRIORITY)

#### 6.1 Higher-Kinded Types
- Types that take type constructors as parameters
- Example: `Functor<F<_>>`

#### 6.2 Associated Types
- Already in trait system, ensure they work with generics
- Example: `trait Iterator { type Item; ... }`

#### 6.3 Generic Constraints in Where Clauses
- Complex constraints beyond simple bounds
- Example: `where T: Display, U: From<T>`

## Testing Strategy

### Unit Tests

1. **Parser Tests**
   - Parse generic function declarations
   - Parse generic struct/enum declarations
   - Parse complex trait bounds
   - Parse where clauses

2. **Type Checker Tests**
   - Generic function type checking
   - Type parameter inference
   - Trait bound validation
   - Error cases (undefined type params, bound violations)

3. **Monomorphization Tests**
   - Simple generic function instantiation
   - Multiple instantiations of same function
   - Nested generic calls

### Integration Tests

1. **Generic Math Functions**
   ```aria
   fn abs<T: Numeric>(x: T) -> T { ... }
   assert abs(42) == 42
   assert abs(-3.14) == 3.14
   ```

2. **Generic Collections**
   ```aria
   let int_vec = Vec<Int>::new()
   int_vec.push(1)
   int_vec.push(2)
   assert int_vec.length() == 2
   ```

3. **Generic Algorithms**
   ```aria
   fn map<T, U>(arr: [T], f: fn(T) -> U) -> [U] { ... }
   let doubled = map([1, 2, 3], |x| x * 2)
   ```

## Error Messages

High-quality error messages for:

1. **Type Mismatch in Generic Context**
   ```
   Error: Type mismatch in generic function
     --> test.aria:5:10
      |
    5 |   foo(42, "hello")
      |           ^^^^^^^ expected Int, found String
      |
      = note: type parameter T was inferred as Int from first argument
      = help: all uses of T must be the same type
   ```

2. **Trait Bound Violation**
   ```
   Error: Type does not satisfy trait bound
     --> test.aria:10:5
      |
   10 |   sort(vec)
      |   ^^^^^^^^^ the trait `Ord` is not implemented for `MyType`
      |
      = note: required by trait bound in function `sort<T: Ord>`
      = help: consider implementing `Ord` for `MyType`
   ```

3. **Wrong Number of Type Arguments**
   ```
   Error: Wrong number of type arguments
     --> test.aria:3:15
      |
    3 |   let v: Vec<Int, String> = Vec::new()
      |               ^^^^^^^^^^^ expected 1 type argument, found 2
      |
      = note: struct `Vec` has 1 type parameter
   ```

## Performance Considerations

1. **Compilation Time**
   - Monomorphization happens after type checking
   - Cache instantiations to avoid duplicates
   - Consider compilation time vs runtime trade-off

2. **Binary Size**
   - Monitor growth with generic usage
   - Consider whole-program optimization to deduplicate
   - Profile and document size impact

3. **Runtime Performance**
   - Monomorphization provides zero-cost abstractions
   - Specialized code can be optimized per-type
   - No runtime type checking overhead

## Documentation Requirements

1. **Language Guide** - Add section on generics
2. **Standard Library** - Document generic functions/types
3. **Compiler Internals** - Document monomorphization process
4. **Migration Guide** - How to convert non-generic to generic code

## Success Criteria

1. ✓ Parse generic syntax correctly
2. ✓ Type check generic functions and types
3. ✓ Infer type arguments at call sites
4. ✓ Validate trait bounds
5. ✓ Monomorphize generic code successfully
6. ✓ Standard library uses generics (abs, min, max, collections)
7. ✓ All tests pass
8. ✓ Performance benchmarks show no regression
9. ✓ Documentation is complete

## Timeline Estimate

- **Phase 1** (Type Checking): 4-6 hours
- **Phase 2** (Call Site Inference): 3-4 hours
- **Phase 3** (Trait Bounds): 2-3 hours
- **Phase 4** (Monomorphization): 4-6 hours
- **Phase 5** (Generic Builtins): 2-3 hours
- **Phase 6** (Advanced Features): 6-8 hours (future work)

**Total Core Implementation**: 15-22 hours
**Total with Advanced Features**: 21-30 hours

## References

- Rust's trait system and monomorphization
- Hindley-Milner type inference
- MLton's monomorphization approach
- Swift's generic implementation
