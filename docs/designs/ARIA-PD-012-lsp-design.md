# ARIA-PD-012: LSP and IDE Integration Design

**Decision ID**: ARIA-PD-012
**Status**: Approved
**Date**: 2026-01-15
**Author**: BEACON (Product Decision Agent)
**Research Inputs**:
- ARIA-M18-04: LSP Architecture and Query-Based Compilation Design (LUMEN)

---

## Executive Summary

This document defines Aria's Language Server Protocol (LSP) implementation architecture, synthesizing research from LUMEN on query-based compilation, incremental analysis, and IDE integration patterns. The architecture draws heavily from proven patterns in rust-analyzer while adapting to Aria's unique features: algebraic effects, contracts, and ownership inference.

**Final Decisions**:
1. **Query Architecture**: Salsa-based incremental computation with durability-aware caching
2. **Syntax Trees**: Red-green (Rowan-inspired) lossless, error-tolerant trees
3. **Incremental Analysis**: Three-tier durability system (HIGH/MEDIUM/LOW) with version vectors
4. **V1 Feature Set**: Four-tier priority system focusing on navigation and diagnostics first
5. **Error Recovery**: Recovery-set based resilient parsing with structural recognition
6. **Semantic Highlighting**: Context-aware tokens with Aria-specific modifiers for effects and contracts

---

## 1. Query-Based Architecture Design

### 1.1 Architectural Decision

**Decision**: Adopt Salsa-style query-based architecture over traditional pipeline compilation.

**Rationale**:
| Approach | Pros | Cons | Decision |
|----------|------|------|----------|
| Traditional Pipeline | Simple, proven | Full recompile on any change | **Rejected** |
| Tree-sitter Only | Fast incremental lexing | Limited semantic analysis | **Rejected** |
| Query-Based (Salsa) | Demand-driven, incremental, proven at scale | Learning curve | **Adopted** |

The query-based model fundamentally inverts the compilation paradigm:

```
Traditional Model:
  Source -> [Lex] -> [Parse] -> [Resolve] -> [TypeCheck] -> [Generate]
  (Forward flow, batch processing, restart on change)

Query Model:
  [Persistent Database] <- Query <- [On-Demand Computation]
  (Backward flow, incremental, cache-and-revalidate)
```

### 1.2 Database Architecture

Aria's LSP database is organized into five query groups with clear dependency boundaries:

```
QUERY GROUP HIERARCHY

+------------------+
|  SourceDatabase  |  [INPUT LAYER]
|  - file_text()   |  Durability: LOW/MEDIUM/HIGH
|  - file_set()    |
|  - stdlib()      |
+--------+---------+
         |
         v
+------------------+
|  ParserDatabase  |  [SYNTAX LAYER]
|  - parse()       |
|  - item_tree()   |
+--------+---------+
         |
    +----+----+
    |         |
    v         v
+----------+  +------------------+
| NameRes  |  |  TypeDatabase    |  [SEMANTIC LAYER]
| Database |  |  - fn_signature()|
+----+-----+  |  - infer_body()  |
     |        +--------+---------+
     +--------+        |
              |        |
              v        v
        +------------------+
        |  EffectDatabase  |  [ARIA-SPECIFIC]
        |  - infer_effects |
        +--------+---------+
                 |
                 v
        +------------------+
        | ContractDatabase |  [ARIA-SPECIFIC]
        | - check_contracts|
        +------------------+
```

### 1.3 Critical Design Invariant

**The Signature Stability Principle**: Function signatures depend ONLY on declaration syntax, NOT on function bodies.

This invariant is the cornerstone of incremental performance:

```
When user edits function body:
  1. file_text(file) -> INVALIDATED
  2. parse(file) -> RECOMPUTED
  3. item_tree(file) -> RECOMPUTED but UNCHANGED (body edits don't change signatures)
  4. fn_signature(func) -> NOT RECOMPUTED (early cutoff)
  5. infer_body(other_funcs) -> NOT RECOMPUTED (dependency unchanged)
```

**Impact**: Editing a function body only recomputes type inference for THAT function, not callers.

### 1.4 Query Interning Strategy

All semantic entities use interned IDs to enable efficient comparison and hashing:

| Entity | Interned ID | Storage |
|--------|-------------|---------|
| Source Files | `FileId` | u32 index into file table |
| Functions | `FunctionId` | (FileId, LocalDefId) |
| Types | `TypeId` | De Bruijn indexed type table |
| Effects | `EffectId` | Global effect registry index |
| Contracts | `ContractId` | (FunctionId, ContractIndex) |

**Decision**: Use u32 indices over pointers for cache-friendliness and snapshot compatibility.

---

## 2. Incremental Analysis API

### 2.1 Durability System

**Decision**: Implement three-tier durability with version vectors.

| Durability | Change Frequency | Examples | Invalidation Cost |
|------------|------------------|----------|-------------------|
| **HIGH** | Rare (session-level) | Standard library, external dependencies | Expensive (full revalidation) |
| **MEDIUM** | Occasional (minutes) | Aria.toml, project config, dependency manifests | Moderate |
| **LOW** | Continuous (keystrokes) | User source code being actively edited | Cheap (targeted revalidation) |

### 2.2 Version Vector Optimization

Instead of a single global revision counter, the database tracks `(high_rev, medium_rev, low_rev)`:

```
USER TYPES IN main.aria:

Without Version Vectors:
  - Global revision increments
  - ALL queries check for invalidation
  - Standard library queries (~300ms validation overhead)

With Version Vectors:
  - Only low_rev increments
  - Queries depending only on HIGH durability SKIP validation
  - Response time: <50ms
```

**Performance Target**: Keystroke-to-diagnostics latency < 50ms for typical edits.

### 2.3 Early Cutoff Propagation

The query system automatically implements early cutoff. When a derived query recomputes but produces the same result, dependents avoid recomputation:

```
SCENARIO: User adds whitespace/comment in function body

1. file_text("main.aria") - CHANGED (new text content)
2. parse("main.aria") - RECOMPUTED (input changed)
3. item_tree("main.aria") - RECOMPUTED -> SAME VALUE
   (whitespace/comments don't affect item structure)
4. fn_signature("main::process") - NOT RECOMPUTED (early cutoff triggered)
5. infer_body("callers") - NOT RECOMPUTED (signatures unchanged)

RESULT: Only parse() does significant work
```

### 2.4 Cancellation Architecture

**Decision**: Implement cooperative cancellation for long-running queries.

```
CANCELLATION FLOW

[User Types] -> [AnalysisHost.apply_change()]
                        |
                        v
              [Set cancellation flag]
                        |
                        v
              [Running queries check flag periodically]
                        |
                        v
              [Return Cancelled error, release snapshot]
                        |
                        v
              [Apply change to database]
                        |
                        v
              [Clear cancellation flag]
                        |
                        v
              [New queries use updated data]
```

**Cancellation Check Points**:
- Every N statements in body inference (N = 50)
- Every function boundary in cross-function analysis
- Before expensive operations (name resolution, effect inference)

### 2.5 Snapshot Isolation

**Decision**: All LSP requests operate on immutable database snapshots.

```
REQUEST HANDLING PATTERN

1. [Request Received] -> Take db.snapshot()
2. [Analysis Runs] -> Reads from frozen snapshot
3. [User Types] -> Triggers cancellation
4. [Request Returns] -> Cancelled or result
5. [Main Thread] -> Applies change to mutable db
6. [New Request] -> Takes fresh snapshot
```

**Guarantee**: Analysis never sees partial/inconsistent state.

---

## 3. Feature Set for V1 LSP

### 3.1 Tier-Based Implementation Strategy

**Decision**: Implement features in four tiers based on developer productivity impact.

#### Tier 1: Foundation (MVP Release)

| Feature | LSP Method | Dependency | Impact Score |
|---------|------------|------------|--------------|
| Diagnostics | `publishDiagnostics` | parse, typecheck | 10/10 |
| Go to Definition | `textDocument/definition` | name resolution | 9/10 |
| Hover | `textDocument/hover` | type inference | 8/10 |
| Document Symbols | `textDocument/documentSymbol` | parse | 7/10 |

**Release Criteria**: All Tier 1 features functional with <100ms latency.

#### Tier 2: Productivity (v1.1)

| Feature | LSP Method | Dependency | Impact Score |
|---------|------------|------------|--------------|
| Completion | `textDocument/completion` | scope analysis | 9/10 |
| Find References | `textDocument/references` | index | 8/10 |
| Rename | `textDocument/rename` | references + validation | 8/10 |
| Signature Help | `textDocument/signatureHelp` | type inference | 7/10 |

#### Tier 3: Enhanced Experience (v1.2)

| Feature | LSP Method | Dependency | Impact Score |
|---------|------------|------------|--------------|
| Semantic Tokens | `textDocument/semanticTokens` | full analysis | 7/10 |
| Inlay Hints | `textDocument/inlayHint` | type inference | 6/10 |
| Code Actions | `textDocument/codeAction` | diagnostics + fixes | 6/10 |
| Folding Ranges | `textDocument/foldingRange` | parse | 5/10 |

#### Tier 4: Advanced (v2.0)

| Feature | LSP Method | Dependency | Impact Score |
|---------|------------|------------|--------------|
| Type Hierarchy | `typeHierarchy/*` | full type graph | 5/10 |
| Call Hierarchy | `callHierarchy/*` | call graph index | 5/10 |
| Workspace Symbols | `workspace/symbol` | project index | 5/10 |
| Code Lens | `textDocument/codeLens` | various | 4/10 |

### 3.2 Aria-Specific Extensions

Beyond standard LSP, Aria requires custom features to expose its unique capabilities:

#### Effect Information

**Custom Request**: `aria/effectInfo`

```
PURPOSE: Display effect annotations for functions/expressions

REQUEST: { textDocument, position }
RESPONSE: {
  effects: ["IO", "Async", "Exception[HttpError]"],
  handlers: [{ location, effect, strategy }],
  suggestions: ["Consider handling Exception locally"]
}

DISPLAY:
  - Hover popups include effect list
  - Inlay hints show inferred effects
  - Code lens shows "3 effects" annotation
```

#### Contract Information

**Custom Request**: `aria/contractInfo`

```
PURPOSE: Display contract status for functions

REQUEST: { textDocument, position }
RESPONSE: {
  requires: ["x > 0", "y != null"],
  ensures: ["return >= 0"],
  status: "verified" | "unverified" | "violated",
  violations: [{ contract, counterexample }]
}

DISPLAY:
  - Gutter icons for contract status
  - Hover shows contract text
  - Diagnostics for violations
```

#### Ownership Visualization

**Custom Request**: `aria/ownershipInfo`

```
PURPOSE: Explain ownership/borrowing decisions

REQUEST: { textDocument, position }
RESPONSE: {
  ownership: "owned" | "borrowed" | "shared",
  lifetime: "stack" | "heap" | "static",
  inference_reason: "Moved to async closure",
  suggestions: ["Consider cloning to avoid move"]
}
```

### 3.3 Feature Dependencies

```
FEATURE DEPENDENCY GRAPH

[Diagnostics] <-- requires --> [Parse] + [TypeCheck]
       |
       v
[Go to Definition] <-- requires --> [Name Resolution]
       |
       v
[Find References] <-- requires --> [Definition] + [Index]
       |
       v
[Rename] <-- requires --> [References] + [Validation]

[Hover] <-- requires --> [Type Inference] + [Effect Inference]
   |
   v
[Inlay Hints] <-- requires --> [Hover] + [Layout]

[Completion] <-- requires --> [Scope Analysis] + [Type Inference]
   |
   v
[Signature Help] <-- requires --> [Completion Context]
```

---

## 4. Error Recovery Strategy

### 4.1 Design Philosophy

**Decision**: Adopt recognition-based error recovery over repair-based.

| Strategy | Description | Decision |
|----------|-------------|----------|
| Error Repair | Parser guesses missing tokens | **Rejected** - Too speculative |
| Error Productions | Grammar includes error rules | **Rejected** - Pollutes grammar |
| Recovery Sets | Parser skips to known sync points | **Adopted** - Predictable |
| Resilient LL | Explicit recovery at each production | **Adopted** - Precise control |

**Core Principle**: "Only the user knows how to correctly complete incomplete code."

### 4.2 Red-Green Syntax Tree Architecture

**Decision**: Implement Rowan-style two-layer syntax trees.

```
GREEN TREE (Immutable, Shared)
+------------------+
| GreenNode        |
| - kind: SyntaxKind
| - width: TextSize
| - children: Arc<[GreenChild]>  <- Shared across edits
+------------------+

RED TREE (Cursor with Position)
+------------------+
| SyntaxNode       |
| - green: Arc<GreenNode>  <- Points into green tree
| - parent: Option<Rc<SyntaxNode>>
| - offset: TextSize  <- Absolute position
+------------------+
```

**Benefits**:
1. **Structural Sharing**: Unchanged subtrees share memory across edits
2. **Incremental Reparsing**: Only reparse affected regions
3. **Lossless**: Preserves whitespace, comments, trivia
4. **Error-Tolerant**: ERROR nodes are valid tree nodes

### 4.3 Recovery Set Definitions

Each parsing context defines explicit recovery tokens:

| Context | Recovery Set | Behavior |
|---------|--------------|----------|
| Item Level | `fn`, `struct`, `enum`, `trait`, `impl`, `mod`, `use`, `effect` | Close current item, start new |
| Statement Level | `let`, `if`, `while`, `for`, `return`, `}`, `fn` | Close current statement |
| Expression Level | `;`, `)`, `]`, `}`, `,` | Complete current expression |
| Parameter Level | `,`, `)`, `->`, `{` | Move to next parameter |
| Block Level | `}`, `fn`, `struct` | Close current block |

### 4.4 Error Node Representation

**Decision**: Errors are represented as syntax nodes, not exceptions.

```
SYNTAX KIND ENUMERATION

// Valid syntax kinds
FN_DEF, PARAM_LIST, BLOCK_EXPR, IF_EXPR, ...

// Error markers
ERROR       - General error container
MISSING     - Expected token not present

// Recovery markers
RECOVERED   - Parser recovered at this point
SKIPPED     - Tokens skipped during recovery
```

**Typed AST Pattern**:
```
FnDef.name() -> Option<Name>     // None if name missing
FnDef.body() -> Option<BlockExpr> // None if body missing
FnDef.effects() -> Vec<Effect>   // Empty if no effects declared
```

### 4.5 Completion Context Strategy

**Decision**: Use the "IntelliJ Trick" - insert dummy identifier at cursor.

```
COMPLETION FLOW

1. User types: "foo.b|" (| = cursor)
2. Insert marker: "foo.b__ARIA_MARKER__"
3. Parse modified source
4. Find marker in syntax tree
5. Determine context from surrounding syntax:
   - Is marker after "."? -> Method/field completion
   - Is marker in type position? -> Type completion
   - Is marker after ":"? -> Type annotation completion
6. Generate completions based on context
7. Filter by prefix "b"
```

### 4.6 Recovery Examples

```
EXAMPLE 1: Missing function body

Input:
  fn foo(x: Int) -> Int

Recovery:
  FN_DEF
    FN_KW "fn"
    NAME "foo"
    PARAM_LIST "(x: Int)"
    RETURN_TYPE "-> Int"
    MISSING [body]  <- Marked as missing, not error

EXAMPLE 2: Incomplete expression

Input:
  let x = foo.

Recovery:
  LET_STMT
    LET_KW "let"
    NAME "x"
    EQ "="
    FIELD_EXPR
      NAME_REF "foo"
      DOT "."
      MISSING [field_name]

EXAMPLE 3: Unclosed block

Input:
  fn foo() {
    if true {
      bar()

Recovery:
  FN_DEF
    ...
    BLOCK_EXPR
      IF_EXPR
        ...
        BLOCK_EXPR
          CALL_EXPR "bar()"
          MISSING [}]  <- First missing
        MISSING [}]    <- Second missing
```

---

## 5. Semantic Highlighting Rules

### 5.1 Token Type Definitions

**Decision**: Extend standard LSP semantic tokens with Aria-specific types.

#### Standard Token Types

| Token Type | Usage in Aria |
|------------|---------------|
| `namespace` | Module names |
| `type` | Type references |
| `class` | Struct definitions |
| `enum` | Enum definitions |
| `interface` | Trait definitions |
| `struct` | Struct definitions (alias) |
| `typeParameter` | Generic type parameters |
| `parameter` | Function parameters |
| `variable` | Local variables |
| `property` | Struct fields |
| `enumMember` | Enum variants |
| `function` | Function definitions/calls |
| `method` | Method definitions/calls |
| `macro` | Macro invocations |
| `keyword` | Language keywords |
| `comment` | Comments |
| `string` | String literals |
| `number` | Numeric literals |
| `operator` | Operators |

#### Aria-Specific Token Types

| Token Type | Usage |
|------------|-------|
| `effect` | Effect names (Console, Async, IO) |
| `handler` | Handler keywords and blocks |
| `contract` | Contract annotations (requires, ensures) |
| `lifetime` | Lifetime annotations (if exposed) |

### 5.2 Token Modifier Definitions

**Decision**: Use modifiers to provide rich semantic context.

#### Standard Modifiers

| Modifier | Meaning |
|----------|---------|
| `declaration` | First occurrence (definition site) |
| `definition` | Full definition (not just declaration) |
| `readonly` | Immutable binding |
| `static` | Static/module-level |
| `deprecated` | Marked deprecated |
| `abstract` | Abstract method/type |
| `async` | Async function/block |
| `modification` | Mutable variable being modified |
| `documentation` | Doc comment |
| `defaultLibrary` | Standard library item |

#### Aria-Specific Modifiers

| Modifier | Meaning | Visual Suggestion |
|----------|---------|-------------------|
| `effectful` | Function performs effects | Italic or underline |
| `pure` | Function is pure (no effects) | Normal weight |
| `contracted` | Has pre/post conditions | Bold or icon |
| `unsafe` | Unsafe block/function | Red tint |
| `inferred` | Type/effect was inferred | Dimmed |
| `handler` | Effect handler context | Background highlight |

### 5.3 Highlighting Rules

#### Functions

```
FUNCTION HIGHLIGHTING RULES

Pure function:
  Token: function
  Modifiers: [declaration, pure]

Effectful function:
  Token: function
  Modifiers: [declaration, effectful]

Async function:
  Token: function
  Modifiers: [declaration, async, effectful]

Contracted function:
  Token: function
  Modifiers: [declaration, contracted]
  (Additional: contract annotations highlighted separately)

Deprecated function:
  Token: function
  Modifiers: [deprecated]
  (Visual: strikethrough)
```

#### Variables

```
VARIABLE HIGHLIGHTING RULES

Immutable local:
  Token: variable
  Modifiers: [readonly]

Mutable local:
  Token: variable
  Modifiers: []

Mutable local being mutated:
  Token: variable
  Modifiers: [modification]

Parameter:
  Token: parameter
  Modifiers: [readonly] (if not mut)

Static/const:
  Token: variable
  Modifiers: [static, readonly]
```

#### Effects and Contracts

```
EFFECT HIGHLIGHTING RULES

Effect name in declaration:
  Token: effect
  Modifiers: [declaration]

Effect name in annotation:
  Token: effect
  Modifiers: []

Effect operation call:
  Token: function
  Modifiers: [effectful]

Handler keyword:
  Token: keyword
  Modifiers: [handler]

CONTRACT HIGHLIGHTING RULES

requires keyword:
  Token: keyword
  Modifiers: [contract]

ensures keyword:
  Token: keyword
  Modifiers: [contract]

Contract expression:
  Token: (expression tokens)
  Modifiers: [+contract] (added to normal modifiers)
```

### 5.4 Context-Aware Highlighting

**Decision**: Highlighting should reflect semantic context, not just syntax.

```
CONTEXT EXAMPLES

1. Type vs Value Position:
   let x: Foo = Foo.new()
        ^---^   ^---^
        type    constructor

   Token: type    Token: function

2. Effect vs Type:
   fn foo() -> Int !Console
                    ^------^
                    effect (not type)

   Token: effect

3. Shadowing:
   let x = 1
   let x = 2  // Shadows previous x
       ^
   Token: variable
   Modifiers: [declaration, shadows]

4. Unused:
   let unused_var = 1  // Never used
       ^---------^
   Token: variable
   Modifiers: [declaration, readonly, unused]
   (Visual: dimmed)
```

### 5.5 Performance Considerations

**Decision**: Support both full and range-based semantic tokens.

```
SEMANTIC TOKEN STRATEGIES

Full Document:
  - Compute on file open
  - Recompute on significant changes
  - Cache with file revision

Range-Based:
  - Compute for visible viewport + margin
  - Recompute on scroll
  - Faster for large files

Delta Updates:
  - LSP 3.17 semantic tokens delta
  - Send only changed tokens
  - Requires edit tracking
```

**Performance Targets**:
- Full file semantic tokens: <200ms for files <5000 lines
- Range-based tokens: <50ms for visible range
- Delta computation: <20ms for typical edits

---

## 6. Implementation Priorities

### 6.1 Phase 1: Foundation (Weeks 1-5)

| Component | Tasks | Exit Criteria |
|-----------|-------|---------------|
| Database | Salsa setup, interning, source queries | file_text() query works |
| Syntax | Green tree, red tree, AST layer | Parse recovers from errors |
| LSP Core | Connection, capabilities, dispatch | Connects to VS Code |
| Diagnostics | Syntax errors, basic type errors | Errors appear in editor |

### 6.2 Phase 2: Navigation (Weeks 6-9)

| Component | Tasks | Exit Criteria |
|-----------|-------|---------------|
| Name Resolution | Scope tracking, import resolution | Go to definition works |
| Symbol Index | Document symbols, workspace index | Outline view works |
| Hover | Type display, doc extraction | Hover shows type info |

### 6.3 Phase 3: Intelligence (Weeks 10-14)

| Component | Tasks | Exit Criteria |
|-----------|-------|---------------|
| Completion | Context analysis, candidate generation | Basic completion works |
| References | Cross-file index, reference tracking | Find references works |
| Rename | Validation, cross-file edits | Safe rename works |

### 6.4 Phase 4: Aria Features (Weeks 15-18)

| Component | Tasks | Exit Criteria |
|-----------|-------|---------------|
| Effect Integration | Effect inference, display | Effects shown in hover |
| Contract Integration | Contract checking, diagnostics | Contract errors shown |
| Semantic Tokens | Full highlighting implementation | Rich syntax coloring |

### 6.5 Phase 5: Polish (Weeks 19-20)

| Component | Tasks | Exit Criteria |
|-----------|-------|---------------|
| Performance | Profiling, optimization | Targets met |
| Memory | GC strategy, leak detection | Stable memory usage |
| Testing | Integration tests, fuzzing | 90%+ coverage |
| Documentation | User guide, API docs | Docs complete |

---

## 7. Performance Architecture

### 7.1 Response Time Targets

| Operation | Target | Strategy |
|-----------|--------|----------|
| Keystroke response | <50ms | Incremental parsing, early cutoff |
| Go to Definition | <100ms | Cached name resolution |
| Hover | <100ms | Cached type + effect info |
| Completion | <200ms | Pre-computed scopes, lazy sorting |
| Find References | <500ms | Indexed lookups |
| Semantic Tokens (full) | <500ms | Parallel computation |
| Workspace Diagnostics | <5s | Background, cancellable |

### 7.2 Memory Budget

| Component | Budget | Strategy |
|-----------|--------|----------|
| Syntax Trees | 100MB | GC unused trees after 5min |
| Type Cache | 50MB | LRU eviction |
| Symbol Index | 50MB | Disk-backed for large projects |
| Effect Cache | 20MB | Per-function granularity |
| Total Target | <300MB | For medium projects (100k LOC) |

### 7.3 Laziness Principle

**Key Insight**: Speed comes from laziness, not just incrementality.

```
LAZINESS OVER INCREMENTALITY

Don't: Incrementally recompute everything
Do: Only compute what's actually needed

Example - User opens file and scrolls:
  - DON'T: Type-check entire file immediately
  - DO: Parse file, delay type-checking until hover/completion

Example - User requests completion:
  - DON'T: Analyze all functions in scope
  - DO: Analyze only visible completions, sort lazily
```

---

## 8. Testing Strategy

### 8.1 Test Categories

| Category | Coverage Target | Method |
|----------|-----------------|--------|
| Unit Tests | Query functions, parsing | Rust unit tests |
| Integration Tests | LSP protocol compliance | LSP test harness |
| Snapshot Tests | Error recovery, highlighting | Golden file comparison |
| Fuzz Tests | Parser robustness | cargo-fuzz |
| Performance Tests | Latency regression | Benchmark suite |

### 8.2 Error Recovery Test Suite

Every error recovery pattern requires test coverage:

```
ERROR RECOVERY TEST PATTERN

1. Input: Incomplete/invalid source
2. Expected: Specific tree structure
3. Assert: No panics
4. Assert: Specific ERROR/MISSING nodes
5. Assert: Valid nodes for valid portions
```

### 8.3 IDE Scenario Tests

Real-world editing scenarios as integration tests:

```
SCENARIO TESTS

1. "Type and complete" - User types, requests completion mid-word
2. "Rename across files" - Rename symbol used in 5+ files
3. "Error then fix" - Introduce error, verify diagnostic, fix, verify clear
4. "Large file scroll" - Scroll through 10k line file, verify highlighting
5. "Rapid typing" - Simulate 10 chars/second, verify no lag
```

---

## 9. Open Decisions

### 9.1 Deferred to Implementation

| Question | Options | Decision Timeline |
|----------|---------|-------------------|
| Salsa version | 0.16 stable vs 0.17+ | Implementation Phase 1 |
| Tree-sitter integration | Lexing layer vs parallel | Implementation Phase 2 |
| Macro expansion | Eager vs lazy in queries | Phase 4 (Aria features) |

### 9.2 User Configuration

| Setting | Default | Range |
|---------|---------|-------|
| Inlay hints enabled | true | true/false |
| Effect display style | inline | inline/hover/both |
| Contract checking | on-save | on-save/on-type/manual |
| Max completion items | 50 | 10-200 |

---

## 10. Conclusion

Aria's LSP architecture adopts proven patterns from rust-analyzer while extending them for Aria's unique features:

1. **Query-based architecture** provides responsive incremental analysis
2. **Red-green syntax trees** enable robust error recovery for IDE scenarios
3. **Durability-aware caching** optimizes for real editing patterns
4. **Tiered feature rollout** ensures quality at each release
5. **Aria-specific extensions** expose effects, contracts, and ownership to developers

This architecture positions Aria to deliver a world-class IDE experience that showcases the language's innovative features while meeting modern developer expectations for responsiveness and intelligence.

---

*Document prepared by BEACON - Product Decision Agent*
*Research input from LUMEN - Eureka Iteration 3*
*Last updated: 2026-01-15*
