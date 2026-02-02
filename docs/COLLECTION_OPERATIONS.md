# Collection Operations in Aria

This document describes the collection operations available in Aria for working with arrays.

## Overview

Aria provides a comprehensive set of collection operations divided into two categories:

1. **Utility Operations** - Fully implemented and working
2. **Higher-Order Operations** - Declared but awaiting full generic type support

## Utility Operations (Fully Functional)

### `slice(array, start, end)` - Extract Subarray

Extracts a portion of an array from `start` (inclusive) to `end` (exclusive).

```aria
let numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
let subset = slice(numbers, 2, 5);  // [3, 4, 5]
```

**Parameters:**
- `array`: The source array
- `start`: Starting index (inclusive)
- `end`: Ending index (exclusive)

**Returns:** A new array containing the sliced elements

**Behavior:**
- Negative or out-of-bounds indices are clamped to valid ranges
- If `start >= end`, returns an empty array
- Does not modify the original array

**Type Support:**
- Works with `Array<Int>`, `Array<Float>`, and other array types
- Type-specific implementations for optimal performance

### `concat(array1, array2)` - Combine Arrays

Concatenates two arrays into a single new array.

```aria
let first = [1, 2, 3];
let second = [4, 5, 6];
let combined = concat(first, second);  // [1, 2, 3, 4, 5, 6]
```

**Parameters:**
- `array1`: First array
- `array2`: Second array

**Returns:** A new array containing all elements from both arrays

**Behavior:**
- Elements from `array1` come first, followed by elements from `array2`
- Does not modify the original arrays
- Can be chained: `concat(concat(a, b), c)`

**Type Support:**
- Both arrays must have the same element type
- Works with `Array<Int>`, `Array<Float>`, etc.

## Higher-Order Operations (Declared, Not Yet Fully Functional)

These operations are declared in the type system and runtime but require full function pointer and generic type support to work correctly. They will be fully functional once Task #6 (Generic Types and Polymorphism) is complete.

### `map(array, fn)` - Transform Each Element

Applies a function to each element, creating a new transformed array.

```aria
// Planned usage (not yet working):
let numbers = [1, 2, 3, 4, 5];
let doubled = map(numbers, fn(x) => x * 2);  // [2, 4, 6, 8, 10]
```

**Status:**
- ⚠️ Interpreter: Limited support with builtin functions only
- ⚠️ Codegen: Not yet implemented (returns error)

### `filter(array, fn)` - Keep Matching Elements

Keeps only elements that match a predicate function.

```aria
// Planned usage (not yet working):
let numbers = [1, 2, 3, 4, 5, 6];
let evens = filter(numbers, fn(x) => x % 2 == 0);  // [2, 4, 6]
```

**Status:**
- ⚠️ Interpreter: Limited support with builtin functions only
- ⚠️ Codegen: Not yet implemented (returns error)

### `reduce(array, fn, initial)` - Fold/Accumulate Values

Reduces an array to a single value using an accumulator function.

```aria
// Planned usage (not yet working):
let numbers = [1, 2, 3, 4, 5];
let sum = reduce(numbers, fn(acc, x) => acc + x, 0);  // 15
```

**Status:**
- ⚠️ Interpreter: Limited support with builtin functions only
- ⚠️ Codegen: Not yet implemented (returns error)

### `find(array, fn)` - Find First Matching Element

Returns `Some(element)` for the first matching element, or `None` if not found.

```aria
// Planned usage (not yet working):
let numbers = [1, 2, 3, 4, 5];
let result = find(numbers, fn(x) => x > 3);  // Some(4)
```

**Returns:** `Option<T>` where `T` is the array element type

**Status:**
- ⚠️ Interpreter: Limited support with builtin functions only
- ⚠️ Codegen: Not yet implemented (returns error)

### `any(array, fn)` - Check If Any Element Matches

Returns `true` if at least one element matches the predicate.

```aria
// Planned usage (not yet working):
let numbers = [1, 2, 3, 4, 5];
let has_even = any(numbers, fn(x) => x % 2 == 0);  // true
```

**Returns:** `Bool`

**Status:**
- ⚠️ Interpreter: Limited support with builtin functions only
- ⚠️ Codegen: Not yet implemented (returns error)

### `all(array, fn)` - Check If All Elements Match

Returns `true` if all elements match the predicate.

```aria
// Planned usage (not yet working):
let numbers = [1, 2, 3, 4, 5];
let all_positive = all(numbers, fn(x) => x > 0);  // true
```

**Returns:** `Bool`

**Status:**
- ⚠️ Interpreter: Limited support with builtin functions only
- ⚠️ Codegen: Not yet implemented (returns error)

## Implementation Status

### What Works Now (✓)

1. **Slice Operation**
   - ✓ Interpreter implementation
   - ✓ C runtime functions (`aria_array_slice_int`, `aria_array_slice_float`)
   - ✓ Cranelift codegen
   - ✓ Type-specific implementations for Int and Float arrays

2. **Concat Operation**
   - ✓ Interpreter implementation
   - ✓ C runtime functions (`aria_array_concat_int`, `aria_array_concat_float`)
   - ✓ Cranelift codegen
   - ✓ Type-specific implementations for Int and Float arrays

### What's Partially Working (⚠️)

**Higher-Order Operations (map, filter, reduce, find, any, all):**
- ✓ Builtin declarations in MIR
- ✓ C runtime function declarations
- ⚠️ Interpreter: Works only with builtin functions, not user-defined functions
- ⚠️ Codegen: Returns error - requires function pointer support

**Limitation:** These operations require full support for:
- Function pointers/first-class functions
- Generic types (to work with any element type)
- Closure capture (for complex predicates)

These are planned for completion with Task #6 (Generic Types and Polymorphism).

## Type-Specific Implementations

Due to the current limitation of not having full generic types, collection operations are implemented with type-specific variants:

### Integer Arrays
- `aria_array_slice_int(array, start, end)`
- `aria_array_concat_int(array1, array2)`
- `aria_array_map_int(array, fn)`
- `aria_array_filter_int(array, predicate)`
- `aria_array_reduce_int(array, fn, initial)`
- `aria_array_find_int(array, predicate)` → returns index or -1
- `aria_array_any_int(array, predicate)` → returns bool
- `aria_array_all_int(array, predicate)` → returns bool

### Float Arrays
- `aria_array_slice_float(array, start, end)`
- `aria_array_concat_float(array1, array2)`
- `aria_array_map_float(array, fn)`
- `aria_array_filter_float(array, predicate)`
- `aria_array_reduce_float(array, fn, initial)`
- `aria_array_find_float(array, predicate)` → returns index or -1
- `aria_array_any_float(array, predicate)` → returns bool
- `aria_array_all_float(array, predicate)` → returns bool

Once generic types are fully implemented, these will be unified into single polymorphic operations.

## Examples

See the `examples/` directory for complete working examples:

- `collections_slice.aria` - Demonstrates slice operations
- `collections_concat.aria` - Demonstrates concat operations
- `collections_higher_order.aria` - Documents planned higher-order operations

## Future Enhancements

Planned improvements when generic types are complete:

1. **Generic Operations** - Single `slice<T>`, `concat<T>`, etc. instead of type-specific variants
2. **Function Pointers** - Full support for passing user-defined functions
3. **Additional Operations** - `zip`, `unzip`, `partition`, `take`, `drop`, `chunks`
4. **Lazy Evaluation** - Iterator-based operations for better performance
5. **String Operations** - Similar collection ops for strings
6. **Custom Collections** - Support for user-defined collection types

## Technical Notes

### Runtime Implementation

Collection operations are implemented in the C runtime (`aria_runtime.c`) with:
- Efficient memory management
- Proper bounds checking
- Type-safe operations through function overloading

### Memory Model

- All operations create new arrays rather than modifying in-place
- Original arrays remain unchanged (immutable by default)
- Proper cleanup is required to avoid memory leaks

### Performance Considerations

- Slice: O(n) where n is the slice length
- Concat: O(n + m) where n and m are the array lengths
- Higher-order ops: O(n) iterations with function call overhead

## Related Documentation

- [Array Types](./ARRAYS.md) - Basic array operations
- [Type System](./TYPES.md) - Type inference and checking
- [Generic Types](./GENERICS.md) - Future generic type support
