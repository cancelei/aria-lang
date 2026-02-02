# ARIA-PD-001: Type System Product Decisions

**Decision ID**: ARIA-PD-001
**Status**: Approved
**Date**: 2026-01-15
**Author**: ARCHITECT (Product Decision Agent)
**Research Inputs**:
- ARIA-M01-04: Bidirectional Type Checking Enhancement (ATLAS)
- ARIA-M01-05: Flow-Sensitive Type Narrowing Design (NOVA)

---

## Decision Summary

Aria's type system will implement a **Hybrid Bidirectional-Flow-Sensitive Type System** that combines:

1. **Bidirectional Type Checking** for ergonomic lambda/closure typing and precise error messages
2. **Flow-Sensitive Narrowing** for automatic type refinement after guards and pattern matches
3. **Contract-Aware Inference** where `requires` clauses inform type narrowing within function bodies

**Core Philosophy**: Write code like Ruby/Python, get safety like Rust, receive errors like Elm.

---

## 1. Synthesis: How Bidirectional Checking and Flow Narrowing Work Together

### 1.1 Unified Type Information Flow

```
                     +-----------------------+
                     |  Type Annotation or   |
                     |  Expected Context     |
                     +-----------+-----------+
                                 |
                    TOP-DOWN (Bidirectional)
                                 |
                                 v
+----------------+     +-------------------+     +------------------+
| Control Flow   |---->| Unified Type Env  |---->| Final Resolved   |
| Narrowings     |     | (FlowTypeEnv)     |     | Type             |
+----------------+     +-------------------+     +------------------+
        ^                       |
        |          BOTTOM-UP (Synthesis)
        |                       |
        +-------<---------------+
              Narrowing Feedback
```

### 1.2 Interaction Points

| Scenario | Bidirectional Contribution | Flow-Sensitive Contribution |
|----------|---------------------------|----------------------------|
| Lambda parameter types | Propagates expected type from context | N/A (parameters fixed) |
| Lambda body type | Checks body against expected return | Narrows captured variables |
| Conditional expressions | Both branches checked against expected | Each branch has narrowed env |
| Callback patterns | Parameter types flow from caller | Narrowed types available in body |
| Pattern match arms | Scrutinee checked against patterns | Each arm narrows scrutinee type |

### 1.3 Example: Combined System in Action

```aria
fn process_user_data(callback: (User?) -> String) -> String {
  callback(get_current_user())
}

// COMBINED SYSTEM WORKING:
let result = process_user_data(|user| {
  // Step 1: BIDIRECTIONAL - user gets type User? from callback signature

  if user != nil {
    // Step 2: FLOW-SENSITIVE - user narrows from User? to User
    // Step 3: BIDIRECTIONAL - return checked against String
    return "Hello, #{user.name}"
  }
  return "No user"  // Also checked against String
})
```

### 1.4 Precedence Rules

When bidirectional context and flow narrowing provide type information:

1. **Flow narrowing takes precedence** for already-bound variables (narrows expected type)
2. **Bidirectional context provides** initial type for new bindings (lambda params)
3. **Unification resolves** any remaining type variables

---

## 2. API Surface: What Users Will See

### 2.1 Syntax Design

**No new syntax required.** The type system works transparently with existing Aria syntax:

```aria
# Lambdas: parameter types inferred from context
let nums = [1, 2, 3]
let strs = nums.map(|x| x.to_string())  # x: Int inferred

# Type guards: automatic narrowing
fn describe(value: Any?) -> String
  if value != nil
    if value is String
      return "String: #{value.length}"  # value is String here
    end
  end
  return "unknown"
end

# Pattern matching: arm narrowing
match result
  Ok(data) => process(data)   # data has success type
  Err(e) => log(e.message)    # e has error type
end

# Contracts: body narrowing
fn safe_divide(a: Int, b: Int) -> Int
  requires b != 0
  return a / b  # b known to be non-zero
end
```

### 2.2 Error Message Design (Elm-Level Clarity)

**Goal**: Every error message answers three questions:
1. What went wrong?
2. Why is it wrong?
3. How can I fix it?

**Error Template**:

```
TYPE MISMATCH in [location]

[code snippet with pointer]

I expected [expected_type] because [source_reason]
but found [actual_type] from [found_reason]

[contextual help if available]
```

**Example Error Messages**:

```
TYPE MISMATCH in callback return

  15 |   process_data(|item| {
  16 |     item + 1
              ^^^^^

I expected `String` because:
    The callback parameter of `process_data` has type `(Item) -> String`
    (defined at line 8)

but found `Int` from:
    The expression `item + 1` where `item: Int` and `1: Int`

Hint: Convert the result to String:
    (item + 1).to_string()
```

```
NULL SAFETY ERROR at line 23

  22 |   let user = get_user(id)
  23 |   return user.name
                ^^^^

Cannot access `.name` on type `User?` which may be nil.

The variable `user` has type `User?` because:
    `get_user` returns `User?` (see definition at line 5)

To fix this, check for nil first:
    if user != nil {
      return user.name  # user is narrowed to User here
    }

Or use the nil-coalescing operator:
    return user?.name ?? "Unknown"
```

### 2.3 IDE/Tooling Surface

| Feature | Visibility |
|---------|------------|
| Hover types | Show narrowed type at cursor position |
| Inlay hints | Display inferred lambda parameter types |
| Error squiggles | Underline exact error location |
| Quick fixes | Offer type conversion suggestions |
| Go to definition | Navigate to type source (annotation, inference point) |

---

## 3. Trade-offs Accepted

### 3.1 What We Chose NOT To Do

| Alternative | Decision | Rationale |
|-------------|----------|-----------|
| **Full dependent types** | Deferred | Too complex for 2026 timeline; contracts provide 80% value |
| **User-defined type guards** | Phase 3 | Adds complexity; standard guards cover common cases |
| **Mutable property smart casts** | Not supported | Unsound without whole-program analysis; explicit unwrap required |
| **Higher-rank polymorphism** | Limited support | Full support requires significant complexity; cover common patterns |
| **Global type inference** | Local only | Better error locality; explicit signatures at module boundaries |

### 3.2 Explicit Design Limits

1. **Narrowing scope**: Narrowings do NOT escape function boundaries
   - Rationale: Modularity, separate compilation, clear contracts

2. **Stability requirements**: Smart casts only on stable (effectively immutable) bindings
   - Rationale: Soundness; mutable aliases could invalidate narrowing

3. **Loop body narrowing**: Does not propagate outside loop
   - Rationale: Zero iterations possible; would require dependent types

4. **Cross-closure narrowing**: Captured variables lose narrowing
   - Rationale: Closure could be called in different context

### 3.3 Complexity Budget

| Feature | Complexity Cost | User Value | Include |
|---------|-----------------|------------|---------|
| Bidirectional lambda inference | Medium | High | Yes |
| Null check narrowing | Low | High | Yes |
| Type guard (`is`) narrowing | Low | High | Yes |
| Pattern match narrowing | Medium | High | Yes |
| Contract-based narrowing | Medium | Medium | Yes |
| Aliased variable narrowing | High | Low | No (Phase 3) |
| Closure capture analysis | High | Medium | No (Phase 3) |
| User-defined type predicates | Medium | Low | No (Phase 3) |

---

## 4. Dependencies: What Implementation Needs

### 4.1 Required Infrastructure

| Component | Dependency | Status |
|-----------|------------|--------|
| `TypeChecker.check_expr` | New method required | To implement |
| `FlowTypeEnv` | Extended TypeEnv | To implement |
| `NarrowedType` struct | New data structure | To implement |
| `TypeSource` tracking | Error message enhancement | To implement |
| CFG construction | For loop/branch analysis | To implement |
| Span tracking | Already exists | Complete |
| Unification engine | Already exists | Complete |

### 4.2 Implementation Order

```
Phase 1 (M01 - Weeks 1-2): Core Infrastructure
├── 1.1 Add check_expr to TypeChecker
├── 1.2 Implement FlowTypeEnv with narrowing map
├── 1.3 Basic null check narrowing (x != nil)
└── 1.4 Type guard narrowing (x is T)

Phase 2 (M01 - Weeks 3-4): Error Enhancement
├── 2.1 TypeSource tracking in errors
├── 2.2 Bidirectional context in messages
├── 2.3 Contextual help generation
└── 2.4 IDE integration hooks

Phase 3 (M01 - Week 5): Integration
├── 3.1 Pattern match narrowing
├── 3.2 Early return narrowing (Never type)
├── 3.3 Contract-based narrowing (requires)
└── 3.4 Test suite and validation

Phase 4 (Future - M02+): Advanced
├── 4.1 Ownership-aware narrowing
├── 4.2 Effect type propagation
├── 4.3 User-defined type predicates
└── 4.4 Cross-function analysis
```

### 4.3 Module Dependencies

```
aria-types/
├── src/
│   ├── lib.rs           # TypeChecker modifications
│   ├── flow_env.rs      # NEW: FlowTypeEnv
│   ├── narrowing.rs     # NEW: NarrowedType, type operations
│   ├── errors.rs        # Enhanced with TypeSource
│   └── bidirectional.rs # NEW: check_expr implementation
```

### 4.4 External Dependencies

- None required for core implementation
- IDE integration may need LSP protocol extensions (standard)
- Test infrastructure already in place

---

## 5. Success Metrics

### 5.1 Quantitative Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Type annotation reduction** | 60% fewer annotations vs explicit typing | Count annotations in sample programs |
| **Error localization accuracy** | 90% errors point to exact problematic expression | Manual review of error positions |
| **Compile-time regression** | <10% slowdown vs current | Benchmark suite timing |
| **IDE responsiveness** | <100ms for type hover | Performance profiling |

### 5.2 Qualitative Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Developer satisfaction** | "Feels like Python/Ruby" | User survey after beta |
| **Error message clarity** | "Understood without docs" | User study with newcomers |
| **Migration effort** | <1 day to onboard from TypeScript | Onboarding time tracking |

### 5.3 Test Coverage Requirements

| Test Category | Minimum Coverage |
|---------------|------------------|
| Bidirectional inference | 95% of documented patterns |
| Flow narrowing | All transfer functions |
| Error messages | All error variants |
| Edge cases | Loop exits, early returns, nested scopes |
| Regression | All PRD-v2 examples |

### 5.4 Validation Approach

1. **Dogfooding**: Use Aria to write Aria compiler components
2. **Comparison suite**: Same programs in TypeScript, Kotlin, Aria - compare experience
3. **Error message A/B testing**: Show errors to developers, measure comprehension
4. **Performance regression testing**: CI benchmark on each PR

---

## 6. Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Error messages still confusing | Medium | High | User testing during development |
| Performance regression | Low | Medium | Incremental rollout, profiling |
| Edge case unsoundness | Low | High | Extensive property-based testing |
| Feature creep | Medium | Medium | Strict phase boundaries, cut scope early |

---

## 7. Open Questions for Future Decisions

1. **Async/concurrent narrowing**: How do narrowings interact with async boundaries?
2. **Macro system integration**: Do macros see narrowed types?
3. **Type predicates syntax**: What should user-defined type guards look like?
4. **Serialization boundaries**: How do narrowings work across FFI?

---

## Appendix A: Comparison with Existing Languages

| Feature | TypeScript | Kotlin | Swift | Aria |
|---------|------------|--------|-------|------|
| Bidirectional inference | Yes | Yes | Yes | Yes |
| Null narrowing | Yes | Yes | Optional | Yes |
| Pattern narrowing | Limited | Yes | Yes | Yes |
| Contract narrowing | No | No | No | **Yes** |
| Mutable smart casts | Limited | Conditional | No | No |
| Error localization | Good | Good | Excellent | **Target: Elm-level** |
| Ownership-aware | No | No | ARC | **Planned (M02)** |

---

## Appendix B: Research Attribution

This decision document synthesizes research from:

1. **ARIA-M01-04** (ATLAS): Bidirectional type checking fundamentals, industry implementations (TypeScript, Kotlin, Swift), error message architecture
2. **ARIA-M01-05** (NOVA): Flow-sensitive narrowing, lattice-based analysis, stability requirements, contract integration

Key academic references:
- Pierce & Turner (2000): Local Type Inference
- Dunfield & Krishnaswami (2013): Complete and Easy Bidirectional Typechecking
- TypeScript Control Flow Analysis (2024)
- Kotlin Language Specification: Smart Casts

---

## Appendix C: Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-01-15 | Adopt hybrid bidirectional + flow-sensitive | Best of both worlds for ergonomics and safety |
| 2026-01-15 | Contract-based narrowing | Unique differentiator, high value for testing |
| 2026-01-15 | No mutable property smart casts | Soundness over convenience |
| 2026-01-15 | Elm-inspired error messages | Developer experience is key differentiator |
| 2026-01-15 | Phased implementation | Ship value early, iterate based on feedback |

---

**Document Status**: This product decision document is complete and approved for implementation. Implementation should proceed according to the phased approach in Section 4.2.
