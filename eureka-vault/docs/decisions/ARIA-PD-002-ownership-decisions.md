# ARIA-PD-002: Ownership Model Product Decisions

**Decision ID**: ARIA-PD-002
**Status**: Approved
**Date**: 2026-01-15
**Based On**: ARIA-M02-04-hybrid-ownership-design.md
**Decision Agent**: GUARDIAN

---

## 1. Decision Summary

After reviewing the FORGE research on hybrid ownership models, the following product decisions have been made for Aria's memory management system:

### Core Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Default semantics** | Move-by-default, infer borrowing | Matches Rust's safety model but reduces annotation burden |
| **Primary memory model** | Three-tier hybrid (Inferred/Explicit/ARC) | Balances safety, ergonomics, and flexibility |
| **Inference approach** | AST-based + CFG liveness (Lobster + NLL) | Achieves 80% annotation-free target |
| **Escape hatch** | `@shared`/`@weak` ARC types | Handles cyclic data without unsafe code |
| **Lifetime syntax** | `ref[L]` with `[life L]` parameters | More readable than Rust's `'a` syntax |

### Final Architecture

```
ARIA OWNERSHIP ARCHITECTURE (Approved)

Tier 1: Inferred Ownership (80% of code)
  - Single-owner patterns: automatic
  - Move semantics: automatic
  - Local borrowing: automatic
  - Function-scoped references: automatic

Tier 2: Explicit Annotations (15% of code)
  - Multiple-source returns: ref[L] syntax
  - Reference-holding structs: [life L] parameter
  - Complex lifetime bounds: where clauses
  - Performance-critical code: explicit control

Tier 3: ARC Escape Hatch (5% of code)
  - Cyclic structures: @shared class
  - Observer patterns: @weak references
  - Graph structures: reference counting
  - Shared mutable state: atomic ARC
```

---

## 2. The 80/15/5 Split in Practice

### 2.1 Tier 1: Inferred Ownership (80%)

**Who it's for**: All developers, all skill levels

**What works automatically**:
- Function parameters and returns (single-source)
- Local variables and reassignment
- Method chains and transformations
- Simple structs with owned fields
- Collection operations
- Error handling with Result/Option

**Example - No annotations needed**:
```aria
fn process_users(users: Array[User]) -> Array[String]
  users
    .filter { |u| u.active }
    .map { |u| u.name.uppercase }
    .sort
end

struct Config
  host: String
  port: Int
  timeout: Duration
end
```

**How inference works**:
1. Compiler builds ownership graph during type checking
2. AST-based analysis identifies ownership patterns
3. CFG-based liveness determines borrow scopes
4. Move semantics applied by default
5. Automatic borrow insertion where safe

### 2.2 Tier 2: Explicit Annotations (15%)

**Who it's for**: Library authors, performance-critical code, zero-copy APIs

**When annotations are required**:

| Scenario | Why | Annotation |
|----------|-----|------------|
| Return reference from multiple params | Ambiguous source | `ref[L]` |
| Struct stores a reference | Struct lifetime bound | `[life L]` |
| Complex lifetime relationships | Compiler can't infer | `where A: outlives B` |
| Override inference | Performance tuning | `move`, `copy`, `borrow` |

**Example - Explicit lifetimes**:
```aria
# Multiple reference parameters - must annotate
fn longest[life L](a: ref[L] String, b: ref[L] String) -> ref[L] String
  if a.len > b.len then a else b
end

# Struct with reference field - must annotate
struct Parser[life L]
  source: ref[L] String
  position: Int
end
```

**Upgrade path from Tier 1**:
```
1. Write code without annotations
2. Compiler error: "cannot infer lifetime for return value"
3. Compiler suggestion: add [life L] parameter
4. Copy-paste suggested fix
5. Code compiles and runs safely
```

### 2.3 Tier 3: ARC Escape Hatch (5%)

**Who it's for**: UI developers, graph algorithm authors, event systems

**When ARC is appropriate**:
- Parent-child cycles (trees with parent pointers)
- Observer/listener patterns
- Graph data structures
- Shared mutable state across threads
- Callback storage that outlives registration

**Example - ARC types**:
```aria
@shared class TreeNode
  value: Int
  @weak parent: TreeNode?    # Weak breaks cycle
  children: Array[TreeNode]  # Strong children
end

@shared class Observable[T]
  value: T
  @weak observers: Array[Observer[T]]

  fn notify()
    for obs in observers.compact  # Filter dead weak refs
      obs.on_change(value)
    end
  end
end
```

**Upgrade path from Tier 1/2**:
```
1. Compiler warning: "potential reference cycle detected"
2. Compiler suggestion: use @shared and @weak
3. Add @shared to class, @weak to back-references
4. Code compiles with automatic reference counting
```

---

## 3. Migration Story

### 3.1 Learning Journey

**Stage 1: Beginner (Day 1-7)**
- Write code without thinking about memory
- 80% of code "just works"
- Focus on logic, not memory management
- Mental model: "values move when assigned"

**Stage 2: Intermediate (Week 2-4)**
- Encounter first "cannot infer lifetime" error
- Learn to read compiler suggestions
- Add annotations by copy-pasting fixes
- Mental model: "sometimes I need to tell the compiler which reference to return"

**Stage 3: Advanced (Month 2+)**
- Understand when to use @shared
- Design data structures with ownership in mind
- Write zero-copy APIs for performance
- Mental model: "ownership is a tool for expressing intent"

### 3.2 Error Message Philosophy

All ownership errors must:
1. **Explain what went wrong** - Clear problem statement
2. **Show where** - Precise source location
3. **Suggest a fix** - Copy-pasteable solution
4. **Link to learn more** - Documentation URL

**Example error with full guidance**:
```
error[E0401]: cannot infer lifetime for return value
 --> src/lib.aria:1:45
  |
1 | fn longest(a: ref String, b: ref String) -> ref String
  |               ^^^^^^^^^^    ^^^^^^^^^^      ^^^^^^^^^^
  |               |             |               |
  |               |             |               return type needs lifetime
  |               |             parameter 'b' could be the source
  |               parameter 'a' could be the source
  |
  = help: add explicit lifetime to clarify which parameter the return borrows from
  = suggestion:
  |
1 | fn longest[life L](a: ref[L] String, b: ref[L] String) -> ref[L] String
  |           ++++++++    ++++             ++++               ++++
  |
  = note: learn more about lifetimes: https://aria-lang.org/docs/ownership/lifetimes
```

### 3.3 Documentation Priorities

| Priority | Topic | Target Audience |
|----------|-------|-----------------|
| 1 | "Values and Moves" tutorial | Beginners |
| 2 | "Understanding Ownership Errors" | All levels |
| 3 | "Zero-Copy APIs with Lifetimes" | Library authors |
| 4 | "Shared Data with @shared" | UI/graph developers |
| 5 | "Ownership Internals" | Language contributors |

---

## 4. Escape Hatches

### 4.1 Override Inference Keywords

| Keyword | Effect | Use Case |
|---------|--------|----------|
| `move x` | Force move semantics | When borrow would be inferred but you want to transfer ownership |
| `copy x` | Force deep copy | When move would be inferred but you need the original |
| `borrow x` | Force borrow | When move would be inferred but function only needs read access |

**Example**:
```aria
let data = load_large_data()

# Inference would move, but we want to keep the original
let backup = copy data
process(data)
verify(backup)

# Inference would borrow, but we want to transfer ownership
let handle = create_handle()
spawn { worker(move handle) }  # Explicit move into thread
```

### 4.2 Unsafe Escape Hatch

**Decision**: Aria will NOT have an `unsafe` block in v1.0.

**Rationale**:
- `@shared` handles most "unsafe" use cases safely
- FFI provides controlled escape for interop
- Adding unsafe later is easier than removing it

**Alternative for low-level code**:
```aria
# Use FFI for truly unsafe operations
extern "C" fn memcpy(dest: RawPtr, src: RawPtr, n: USize) -> RawPtr

# Wrap in safe interface
fn copy_bytes(dest: mut ref Array[Byte], src: ref Array[Byte])
  assert dest.len >= src.len
  unsafe_ffi {
    memcpy(dest.data_ptr, src.data_ptr, src.len)
  }
end
```

### 4.3 Arena Allocation Pattern

For performance-critical code that needs to avoid individual allocations:

```aria
# Arena provides bulk allocation with single deallocation
fn parse_document[life L](arena: mut ref[L] Arena, input: String) -> AST[L]
  let tokens = arena.alloc(tokenize(input))
  let ast = arena.alloc(parse(tokens))
  ast
end

# All arena memory freed when arena drops
fn compile(source: String) -> Result[Binary, Error]
  let arena = Arena.new(megabytes(10))
  let ast = parse_document(arena, source)
  let ir = generate_ir(arena, ast)
  let binary = codegen(ir)
  # arena drops here, all temp allocations freed
  Ok(binary)
end
```

---

## 5. Trade-offs Accepted

### 5.1 Accepted Complexity

| Trade-off | Cost | Benefit |
|-----------|------|---------|
| Lifetime syntax | Learning curve for 15% of code | Zero-copy APIs, Rust-level performance |
| ARC overhead | 5-15% runtime cost for @shared types | Safe handling of cycles without manual management |
| Inference limitations | Some patterns need annotations | 80% of code is truly annotation-free |
| No unsafe block | Can't do arbitrary pointer manipulation | Stronger safety guarantees, simpler mental model |

### 5.2 Rejected Alternatives

| Alternative | Why Rejected |
|-------------|--------------|
| Full ARC everywhere (Swift) | Too much runtime overhead, no compile-time guarantees |
| Full explicit ownership (Rust) | Too high annotation burden for target audience |
| Garbage collection | Unpredictable latency, not suitable for systems programming |
| Tracing GC + escape hatch | Complex interaction, hard to reason about |
| Runtime borrow checking (Vale) | 2-11% overhead, weaker compile-time guarantees |

### 5.3 Open Trade-offs (Deferred Decisions)

| Question | Options | Decision Deadline |
|----------|---------|-------------------|
| Atomic vs non-atomic @shared | Always atomic / per-thread flag | Before threading design |
| WASM ARC optimization | Eliminate for single-threaded / keep for consistency | Before WASM target |
| Incremental ownership checking | Full recheck / dependency tracking | Before LSP implementation |

---

## 6. Dependencies

### 6.1 Type System Requirements

The ownership system requires these type system features:

| Feature | Status | Blocking |
|---------|--------|----------|
| Generic type parameters | Required | Yes |
| Lifetime parameters (`[life L]`) | Required | Yes |
| Associated types | Required | Yes |
| Where clauses | Required | Yes |
| Type inference | Required | Yes |
| Flow-sensitive typing | Required | Yes |
| Trait/protocol system | Required | For Drop/Clone traits |

### 6.2 Compiler Requirements

| Component | Purpose | Priority |
|-----------|---------|----------|
| CFG builder | Liveness analysis for NLL | P0 |
| Def-use chains | Ownership tracking | P0 |
| Escape analysis | Detecting ARC requirements | P0 |
| Cycle detection | Warning about reference cycles | P1 |
| Function specialization | Lobster-style ownership polymorphism | P1 |
| ARC optimization pass | Retain/release elimination | P2 |

### 6.3 Runtime Requirements

| Component | Purpose | Priority |
|-----------|---------|----------|
| Reference count type | @shared implementation | P0 |
| Weak reference type | @weak implementation | P0 |
| Atomic operations | Thread-safe @shared | P1 |
| Arena allocator | Performance escape hatch | P2 |

---

## 7. Success Metrics

### 7.1 Annotation-Free Targets

| Metric | Target | Measurement Method |
|--------|--------|-------------------|
| Lines without ownership annotation | >= 80% | Static analysis of stdlib + example code |
| Functions without lifetime params | >= 85% | Count of `[life L]` in function signatures |
| Structs without lifetime params | >= 90% | Count of `[life L]` in struct definitions |
| Uses of @shared in typical app | <= 5% | Survey of types in sample applications |

### 7.2 Developer Experience Targets

| Metric | Target | Measurement Method |
|--------|--------|-------------------|
| Ownership error resolution time | < 2 minutes | User study / telemetry |
| Error message actionability | 90% have fix suggestion | Audit of all ownership errors |
| Tutorial completion rate | > 80% | Documentation analytics |
| "Would recommend" score | > 8/10 | Developer survey |

### 7.3 Performance Targets

| Metric | Target | Measurement Method |
|--------|--------|-------------------|
| Runtime overhead vs Rust | < 5% | Benchmark suite |
| ARC operation elimination | >= 70% | Compiler statistics |
| Compile time overhead | < 20% vs no ownership checking | Benchmark suite |
| Binary size overhead | < 10% vs C | Binary comparison |

### 7.4 Validation Milestones

| Milestone | Criteria | Target Date |
|-----------|----------|-------------|
| MVP ownership inference | 50% annotation-free in test suite | M02-05 |
| Beta quality | 70% annotation-free, all errors have suggestions | M02-06 |
| Release quality | 80% annotation-free, performance targets met | M02-07 |

---

## 8. Implementation Recommendations

### 8.1 Phase 1: Core Inference (8 weeks)

1. **Week 1-2**: Implement ownership graph construction
2. **Week 3-4**: Implement basic move/borrow inference
3. **Week 5-6**: Implement CFG-based liveness analysis
4. **Week 7-8**: Error messages with suggestions

### 8.2 Phase 2: Explicit Annotations (4 weeks)

1. **Week 1-2**: Lifetime parameter parsing and checking
2. **Week 3-4**: Struct lifetime bounds, where clauses

### 8.3 Phase 3: ARC Escape Hatch (4 weeks)

1. **Week 1-2**: @shared type implementation
2. **Week 3-4**: @weak references, cycle detection warnings

### 8.4 Phase 4: Optimization (6 weeks)

1. **Week 1-2**: Function specialization
2. **Week 3-4**: ARC optimization passes
3. **Week 5-6**: Escape analysis for stack promotion

---

## 9. Risk Mitigation

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| 80% target not achievable | Medium | High | Monitor during development, adjust tier boundaries |
| Error messages too complex | Medium | High | User testing, iteration on message format |
| ARC overhead too high | Low | Medium | Aggressive optimization, consider non-atomic option |
| Lifetime syntax confusing | Medium | Medium | Extensive documentation, good IDE support |
| Interop with C difficult | Low | High | Prioritize FFI design, test early |

---

## 10. Appendix: Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-01-15 | Adopt three-tier model | Balances safety and ergonomics |
| 2026-01-15 | Use `ref[L]` syntax over `'a` | More readable, consistent with type params |
| 2026-01-15 | No unsafe block in v1.0 | Stronger safety guarantees |
| 2026-01-15 | @shared always atomic | Simplicity over micro-optimization |
| 2026-01-15 | Require annotations on failure | Never silently insert runtime checks |

---

**Document Status**: Approved
**Next Steps**: ARIA-M02-05 - Prototype ownership analyzer
**Owner**: Product Team
**Reviewers**: FORGE Research Agent, GUARDIAN Product Agent
