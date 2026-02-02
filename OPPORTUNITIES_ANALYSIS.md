# High-Value Opportunities for Aria Compiler

Analysis based on session work, TODOs, and code patterns.

## Summary Statistics
- **69 TODOs** across codebase
- **31 warnings** in aria-mir alone
- **Type inference** most common TODO category
- **Pattern matching** second most common
- **LSP features** largely unimplemented

---

## 🎯 Top Priority Opportunities

### 1. **Array/Collection Support in Codegen**
**Impact**: 🔥🔥🔥 (Unlocks 5+ stdlib functions)
**Effort**: ⚡⚡⚡ (Moderate - needs heap allocation, indexing)
**Status**: Blocker for push, pop, first, last, reverse builtins

**Why High Value**:
- Currently blocks entire category of stdlib functions
- Collection operations are fundamental to any language
- Runtime has no array support yet - needs full implementation

**What's Needed**:
- Array literal syntax already parsed
- Need MIR array operations (index, length, slice)
- Need Cranelift array indexing and bounds checking
- Need runtime heap allocation for dynamic arrays
- Need array type representation in type system

**Evidence**:
- Runtime declares push/pop/first/last but marked "Simplified - actually generic"
- Test failures when trying to use collections
- TODOs: "elem_ty = MirType::Int // TODO: Infer element type"

---

### 2. **Comprehensive Type Inference System**
**Impact**: 🔥🔥🔥 (Reduces boilerplate, better DX)
**Effort**: ⚡⚡⚡⚡ (Complex - needs type unification)
**Status**: Partial implementation, 15+ type-related TODOs

**Why High Value**:
- Currently default to MirType::Int for unknowns - causes bugs
- Many type mismatches we encountered could be auto-inferred
- Would eliminate need for explicit type annotations in most cases

**What's Needed**:
- Type unification algorithm (Hindley-Milner)
- Bidirectional type checking
- Generic type support
- Proper field access type lookup
- Method call type resolution

**Evidence**:
- "TODO: Proper return type" (appears 3 times)
- "TODO: Full type inference" (appears 2 times)
- "TODO: Look up actual field type" (appears 2 times)
- We fixed 3 type inference bugs this session (constants, floats, function returns)

---

### 3. **Test String Builtins (Quick Win!)**
**Impact**: 🔥🔥 (Validates 9 stdlib functions)
**Effort**: ⚡ (Very easy - 1-2 hours)
**Status**: Runtime implemented, just needs testing

**Why High Value**:
- Runtime C functions already exist and compiled
- Just need Aria test files to validate
- Immediate value with minimal effort
- String operations are very commonly used

**What's Needed**:
- Test files for each builtin:
  - contains, starts_with, ends_with
  - trim, substring, replace
  - to_upper, to_lower, char_at
- Update stdlib documentation

**Evidence**:
- aria_runtime.c:271-315 has all implementations
- Codegen has all builtin call sites
- Just needs test coverage

---

### 4. **Better Error Messages**
**Impact**: 🔥🔥🔥 (Massive DX improvement)
**Effort**: ⚡⚡ (Moderate - systematic improvement)
**Status**: Basic errors, not helpful

**Why High Value**:
- Current errors like "type mismatch" don't show what was expected vs actual
- No suggestions or fix hints
- Makes debugging very difficult
- We spent significant time debugging type mismatches

**What's Needed**:
- Show expected vs actual types
- Source location with context (show the code)
- Suggest fixes where possible
- Better span tracking through compilation
- Colored output with caret pointers

**Evidence**:
- Cranelift verifier error: "type of variable doesn't match type of value"
  - Doesn't show which types!
- We had to add debug prints to diagnose issues
- Error from session: just "type mismatch" with no details

---

### 5. **Type Conversion Builtins**
**Impact**: 🔥🔥 (Common need, easy to add)
**Effort**: ⚡⚡ (Easy - mostly runtime calls)
**Status**: Declared but not implemented

**Why High Value**:
- Very common operations (string formatting, parsing)
- Enables more complex programs
- Foundation for string interpolation

**What's Needed**:
- `to_string()` for Int, Float, Bool, Char
- `to_int()` for String, Float
- `to_float()` for String, Int
- Runtime implementations (sprintf, atoi, atof equivalents)

**Evidence**:
- Already in builtin list: lines 62-65 of lower.rs
- Return types specified: MirType::String, MirType::Int, MirType::Float
- Just needs runtime + codegen

---

### 6. **Cross-Module Type Checking**
**Impact**: 🔥🔥🔥 (Safety, correctness)
**Effort**: ⚡⚡⚡ (Complex - needs symbol table sharing)
**Status**: Noted limitation in module system

**Why High Value**:
- Currently each module type-checked independently
- Can't verify imported item types match usage
- Can have type errors at link time instead of compile time
- Module system is "complete" but this is a major gap

**What's Needed**:
- Symbol table export/import
- Type signature validation across modules
- Import verification (check items exist and are public)
- Cross-module type inference

**Evidence**:
- MODULE_SYSTEM_SUMMARY.md: "Known Limitations #1"
- TODO in compiler: "Type check all modules with proper import resolution"
- Can compile incompatible modules without error

---

### 7. **Pattern Matching Improvements**
**Impact**: 🔥🔥 (Expressiveness, safety)
**Effort**: ⚡⚡⚡⚡ (Complex - needs decision trees)
**Status**: Basic matching only

**Why High Value**:
- Match expressions are powerful but limited
- No exhaustiveness checking
- No decision tree optimization
- Nested patterns not properly handled

**What's Needed**:
- Exhaustiveness checking
- Unreachable pattern detection
- Decision tree compilation
- Nested pattern support
- Or-patterns, guard clauses

**Evidence**:
- "TODO: Proper pattern matching" (appears 2 times)
- "TODO: Proper nested matching"
- "TODO: Proper pattern matching with decision trees"

---

### 8. **Code Quality Cleanup**
**Impact**: 🔥 (Maintainability, technical debt)
**Effort**: ⚡⚡ (Moderate - systematic cleanup)
**Status**: 69 TODOs, 31+ warnings

**Why High Value**:
- Unused code suggests incomplete features
- Warnings hide real problems
- TODOs become stale and forgotten
- Cleaner code easier to maintain

**What's Needed**:
- Fix all compiler warnings (unused variables, imports)
- Audit and categorize all 69 TODOs
- Remove or implement placeholder code
- Add rustdoc comments for public APIs
- Run clippy and fix suggestions

**Evidence**:
- `cargo build -p aria-mir`: "31 warnings"
- 69 TODOs across codebase
- Many unused variables in match arms

---

### 9. **Generic Types / Polymorphism**
**Impact**: 🔥🔥🔥 (Expressiveness, code reuse)
**Effort**: ⚡⚡⚡⚡⚡ (Very complex)
**Status**: Not implemented, several functions marked "Simplified"

**Why High Value**:
- abs() works on Int only, should work on any numeric type
- Collections need generic element types
- Can't write reusable generic functions
- Foundation for modern type system

**What's Needed**:
- Generic type parameters syntax
- Type parameter inference
- Monomorphization or type erasure
- Generic constraints (trait bounds)
- Generic builtins (min/max over any Ord)

**Evidence**:
- "// Simplified - actually polymorphic" for abs()
- "// Simplified - actually generic" for pop()
- Collection builtins can't be properly typed without generics

---

### 10. **Float Literal Improvements**
**Impact**: 🔥 (Precision control)
**Effort**: ⚡ (Easy - lexer + type annotation)
**Status**: Only f64 supported, no suffix notation

**Why High Value**:
- Can't specify f32 vs f64 precision
- GPU and embedded systems need f32
- No hex float literals

**What's Needed**:
- Lexer support for 3.14f32, 3.14f64 suffixes
- Type inference from literal suffixes
- Hex float notation: 0x1.5p3

**Evidence**:
- Only MirType::Float (defaults to F64)
- No Float32/Float64 distinction in tests
- Runtime uses `double` (64-bit) everywhere

---

## Priority Matrix

```
High Impact, Low-Medium Effort (DO FIRST):
1. Test String Builtins ⚡ 🔥🔥
2. Type Conversion Builtins ⚡⚡ 🔥🔥
3. Better Error Messages ⚡⚡ 🔥🔥🔥
4. Float Literal Improvements ⚡ 🔥

High Impact, High Effort (PLAN CAREFULLY):
5. Array/Collection Support ⚡⚡⚡ 🔥🔥🔥
6. Comprehensive Type Inference ⚡⚡⚡⚡ 🔥🔥🔥
7. Cross-Module Type Checking ⚡⚡⚡ 🔥🔥🔥
8. Generic Types ⚡⚡⚡⚡⚡ 🔥🔥🔥

Medium Priority:
9. Pattern Matching Improvements ⚡⚡⚡⚡ 🔥🔥
10. Code Quality Cleanup ⚡⚡ 🔥
```

---

## Quick Wins (Can complete in 1-2 sessions):
1. ✅ **Test String Builtins** - 1-2 hours
2. ✅ **Float Literal Suffixes** - 2-3 hours
3. ✅ **Type Conversion Builtins** - 3-4 hours
4. ✅ **Basic Error Message Improvements** - 4-6 hours

## Medium Projects (1-2 weeks):
5. **Array Literals & Indexing** - Basic array support
6. **Better Type Inference** - Incremental improvements
7. **Code Quality Sweep** - Fix warnings and TODOs

## Long-term Projects (1+ months):
8. **Full Generic Types** - Complete type system overhaul
9. **Comprehensive Type Inference** - Hindley-Milner implementation
10. **Advanced Pattern Matching** - Decision trees, exhaustiveness

---

## Recommended Next Steps

### Immediate (Next Session):
1. **Test all String builtins** - validate what's already there
2. **Implement type conversion builtins** - high value, clear scope
3. **Improve 3-5 most common error messages** - better DX

### Short-term (Next 2-3 sessions):
4. **Array literals and indexing** - enables collections
5. **Code cleanup sweep** - fix warnings, organize TODOs
6. **Float literal suffixes** - precision control

### Medium-term (Next month):
7. **Comprehensive type inference** - reduce type annotations
8. **Cross-module type checking** - make module system safer
9. **Pattern matching improvements** - exhaustiveness checking

This provides a clear roadmap based on actual codebase needs and high-value opportunities!
