# MIR Lowering Improvements for BioFlow Patterns

## Summary

This document details the improvements made to the MIR lowering implementation to better support BioFlow code patterns.

## Completed Tasks

### 1. String Operations ✅
**File**: `crates/aria-mir/src/lower_expr.rs`

**Improvements**:
- String literals are properly interned and lowered to `Constant::String`
- String concatenation using `+` operator is now supported
- Result type for string concatenation is correctly inferred as `MirType::String`
- String comparisons (`==`, `!=`, etc.) work properly with existing comparison operators

**BioFlow Usage**:
```aria
let seq_str = ">" + id + "\n" + bases  # String concatenation
if sub == motif_upper                  # String comparison
```

### 2. Pattern Matching for Enums (Result/Option) ✅
**File**: `crates/aria-mir/src/lower_expr.rs`

**Improvements**:
- Enhanced `lower_match_expr` to detect enum types (Result, Option, custom enums)
- For enum matches, now uses proper pattern matching via `lower_pattern_match`
- Generates decision trees for enum variant checking
- Each match arm properly binds variant fields to variables

**BioFlow Usage**:
```aria
match result
  Ok(value) => # use value
  Err(e) => # handle error
end
```

### 3. Method Calls ✅
**File**: `crates/aria-mir/src/lower_expr.rs` (already working)

**Status**: Already implemented and working correctly

**Features**:
- Method calls like `seq.gc_content()` are lowered as function calls with receiver as first argument
- Return type is inferred from function definition
- Supports chaining (each method call creates a proper call terminator)

**BioFlow Usage**:
```aria
let gc = seq.gc_content()
let trimmed = read.trim_quality(20)
```

### 4. Loop with Break ✅
**File**: `crates/aria-mir/src/lower_stmt.rs` (already working)

**Status**: Already implemented

**Features**:
- `loop { if cond break end end` generates proper CFG
- Break statements jump to loop exit block
- Continue statements jump to loop header

**BioFlow Usage**:
```aria
loop
  if i >= bases.len()
    break
  end
  # process bases[i]
  i = i + 1
end
```

### 5. Array Operations ✅
**File**: `crates/aria-mir/src/lower_expr.rs`

**Status**: Basic support already present

**Features**:
- Array initialization: `let arr = []` or `let arr = [1, 2, 3]`
- Array access: `arr[i]` via index operation
- Array push: Requires builtin function call
- Type inference for element types

**BioFlow Usage**:
```aria
let mut positions = []
positions.push(i)  # Via builtin
let base = seq.bases[i]
```

## Partially Implemented / Placeholder

### 6. Closures/Lambdas ⚠️
**File**: `crates/aria-mir/src/lower_expr.rs`

**Status**: Placeholder implementation

**Current State**:
- `lower_lambda` and `lower_block_lambda` return UnsupportedFeature errors
- Closure types are defined in MIR (`MirType::Closure`)
- Rvalue::Closure exists for closure creation

**Why Not Fully Implemented**:
Full lambda/closure support requires:
1. Closure conversion pass to lift lambdas to top-level functions
2. Environment capture analysis
3. Closure struct generation (function pointer + captured variables)
4. Modifications to the lowering API to support creating new functions during expression lowering

**BioFlow Impact**:
Functions like `filter` and `map` that take closures (e.g., `|c| c == 'G' or c == 'C'`) will need to be implemented as named functions or wait for full closure support.

**Workaround**:
```aria
# Instead of: positions.filter(|pos| pos >= 0)
# Use: positions.filter_positive() with a named helper function

fn is_positive(pos: Int) -> Bool
  pos >= 0
end
```

## Testing

### Test Coverage
The improvements support the following BioFlow patterns:

1. ✅ String manipulation in sequences
2. ✅ Result/Option handling in validation
3. ✅ Method chaining for sequence operations
4. ✅ Loop-based iteration over bases
5. ✅ Array initialization and access
6. ⚠️ Higher-order functions (partial - no lambdas yet)

### Verification
To verify these improvements work correctly, the BioFlow code patterns can be lowered to MIR successfully. The generated MIR should show:

- Proper control flow for match expressions
- String operations as BinaryOp with String type
- Method calls as Call terminators with receiver as first argument
- Loop constructs with proper break/continue blocks

## Next Steps

To fully complete BioFlow support, the following would be needed:

1. **Closure Lifting Pass**:
   - Add a pre-lowering pass to extract lambdas into named functions
   - Analyze captures and create closure structs
   - Update call sites to pass closure structures

2. **String Interpolation**:
   - Currently returns UnsupportedFeature
   - Lower to string concatenation calls

3. **Comprehensions**:
   - Array comprehensions `[x * 2 for x in arr]`
   - Lower to loop + push operations

4. **Better Type Inference**:
   - Infer closure return types from body
   - Propagate types through complex expressions

## Files Modified

- `crates/aria-mir/src/lower_expr.rs`:
  - Enhanced match expression lowering for enums
  - Added string type handling in binary operations
  - Added lambda/closure placeholder stubs

- `crates/aria-mir/src/lower_stmt.rs`:
  - Already had proper loop/break/continue support

- `crates/aria-mir/src/lower_pattern.rs`:
  - Already had comprehensive pattern matching support
  - Used by enhanced match expression lowering

## Conclusion

The MIR lowering now supports the core patterns needed for BioFlow:
- String operations work correctly
- Pattern matching on Result/Option is functional
- Method calls and loops work as expected
- Arrays can be created and accessed

The main limitation is lambda/closure support, which would require more significant architectural changes to implement properly.
