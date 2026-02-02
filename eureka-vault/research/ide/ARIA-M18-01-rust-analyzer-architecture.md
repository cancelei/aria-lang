# ARIA-M18-01: rust-analyzer Architecture Study

**Task ID**: ARIA-M18-01
**Status**: Completed
**Date**: 2026-01-14
**Focus**: Study rust-analyzer's IDE architecture

---

## Executive Summary

rust-analyzer is a fully incremental, on-demand Rust compiler frontend for IDEs. This research analyzes its architecture—salsa-based incrementality, LSP interface, and query system—for Aria's IDE tooling design.

---

## 1. Overview

### 1.1 What is rust-analyzer?

- **IDE-first compiler frontend**: Designed for interactive use
- **Fully incremental**: Only recomputes what changed
- **On-demand**: Computes only what's requested
- **LSP-based**: Standard IDE protocol

### 1.2 Key Innovation

```
Traditional Compiler:
  Source → Parse → Type Check → Codegen → Binary
  (batch processing, start to finish)

rust-analyzer:
  Source → Incremental Database → Query on Demand
  (persistent, only compute what's needed)
```

### 1.3 2025 Status

- Official Rust LSP implementation (replaced RLS)
- Weekly releases with improvements
- December 2025: 648 MB memory saved via GC optimization
- Plugin API under development

---

## 2. Architecture Overview

### 2.1 Crate Structure

```
rust-analyzer/
├── rust-analyzer/     # LSP server, CLI
├── ide/               # IDE features (completion, go-to-def)
├── ide-db/            # IDE-specific database
├── hir/               # High-level IR (semantic model)
├── hir-ty/            # Type inference
├── hir-def/           # Name resolution
├── syntax/            # Syntax tree (rowan)
├── parser/            # Parsing
└── base-db/           # Core database traits
```

### 2.2 Layer Diagram

```
┌─────────────────────────────────────────┐
│            LSP Interface                 │
│         (rust-analyzer crate)            │
├─────────────────────────────────────────┤
│              IDE Layer                   │
│    (completion, diagnostics, refactor)   │
├─────────────────────────────────────────┤
│                 HIR                      │
│     (types, name resolution, traits)     │
├─────────────────────────────────────────┤
│               Syntax                     │
│         (parsing, syntax trees)          │
├─────────────────────────────────────────┤
│            Salsa Database                │
│       (incremental computation)          │
└─────────────────────────────────────────┘
```

---

## 3. Incrementality with Salsa

### 3.1 What is Salsa?

Salsa is a framework for incremental computation:
- Query-based: Define computations as queries
- Memoized: Results cached automatically
- Incremental: Only recompute when inputs change

### 3.2 Query Definition

```rust
#[salsa::query_group(ParserDatabaseStorage)]
pub trait ParserDatabase: SourceDatabase {
    fn parse(&self, file_id: FileId) -> Parse<SourceFile>;
}

// Implementation
fn parse(db: &dyn ParserDatabase, file_id: FileId) -> Parse<SourceFile> {
    let text = db.file_text(file_id);  // Dependency tracked
    SourceFile::parse(&text)
}
```

### 3.3 Automatic Invalidation

```
When file changes:
1. Salsa marks file_text(file_id) as changed
2. Queries depending on it are invalidated
3. On next access, recompute only what's needed

Key invariant:
"Typing inside a function body never invalidates
 global derived data"
```

### 3.4 Revision Counter

```rust
// Salsa maintains global revision
struct Database {
    revision: u64,  // Bumped on each change
    // ...
}

// Concurrent access
// If revision changes during computation → cancel
// Throw special panic, caught at ide level
```

---

## 4. Syntax Trees (Rowan)

### 4.1 Green Trees (Immutable)

```rust
// Structural sharing via Arc
struct GreenNode {
    kind: SyntaxKind,
    children: Vec<GreenChild>,  // Arc shared
}

// Benefits:
// - Incremental reparsing
// - Zero-cost cloning
// - Persistent data structure
```

### 4.2 Red Trees (Cursors)

```rust
// Red tree = Green tree + position info
struct SyntaxNode {
    green: Arc<GreenNode>,
    parent: Option<SyntaxNode>,
    offset: TextSize,
}

// Navigate like regular tree
// But computed lazily from green tree
```

### 4.3 Error Recovery

Parser produces tree even with errors:
- `ERROR` nodes mark problematic regions
- Valid siblings still parsed correctly
- IDE features work on partial code

---

## 5. HIR (High-level IR)

### 5.1 Purpose

```
Syntax Tree: "What does the text say?"
HIR: "What does it mean semantically?"
```

### 5.2 Key Components

```rust
// Items (top-level definitions)
pub struct Function {
    name: Name,
    params: Vec<Param>,
    return_type: TypeRef,
    body: Option<ExprId>,
}

// Expressions (body IR)
pub enum Expr {
    Call { callee: ExprId, args: Vec<ExprId> },
    If { condition: ExprId, then_branch: ExprId, else_branch: Option<ExprId> },
    Match { expr: ExprId, arms: Vec<MatchArm> },
    // ...
}
```

### 5.3 Incrementality Boundary

```
hir-def (item structure):
  - Changes rarely
  - Changing function signature invalidates dependents

hir-ty (function bodies):
  - Changes frequently
  - Body changes DON'T invalidate external callers
```

---

## 6. Type Inference

### 6.1 Chalk Integration

rust-analyzer uses Chalk for trait solving:
- Prolog-like logic programming
- Handles complex trait bounds
- Incremental trait resolution

### 6.2 Inference Algorithm

```rust
fn infer_body(db: &dyn HirDatabase, func: FunctionId) -> InferenceResult {
    let mut ctx = InferenceContext::new(db, func);

    // Bidirectional type inference
    for stmt in body.statements {
        ctx.infer_stmt(stmt);
    }

    // Unify constraints
    ctx.resolve_all();

    ctx.result
}
```

---

## 7. LSP Interface

### 7.1 Request Handling

```
src/handlers/request.rs

// Each LSP request maps to handler
pub fn handle_goto_definition(
    snap: GlobalStateSnapshot,
    params: GotoDefinitionParams,
) -> Result<Option<GotoDefinitionResponse>> {
    let position = from_proto::file_position(&snap, params.text_document_position)?;
    let nav = snap.analysis.goto_definition(position)?;
    Ok(to_proto::goto_definition_response(&snap, nav))
}
```

### 7.2 Cancellation

```rust
// Long-running queries can be cancelled
impl GlobalState {
    fn on_request(&mut self, req: Request) {
        let result = panic::catch_unwind(|| {
            handle_request(req)
        });

        match result {
            Ok(response) => self.send(response),
            Err(e) if is_cancelled(&e) => {
                // Silently ignore cancelled requests
            }
            Err(e) => panic::resume_unwind(e),
        }
    }
}
```

---

## 8. Performance Optimizations

### 8.1 2025 Improvements

| Optimization | Impact |
|--------------|--------|
| GC for solver types | 648 MB saved |
| Salsa interning | 31 seconds faster |
| Thin LTO parallelism | Faster compilation |
| Plugin API | Extensibility |

### 8.2 General Strategies

- **Lazy computation**: Only compute what's queried
- **Structural sharing**: Syntax trees share nodes
- **Cancellation**: Abort outdated work
- **Caching**: Memoize expensive computations

---

## 9. Recommendations for Aria

### 9.1 Query-Based Architecture

```aria
# Aria IDE database (Salsa-inspired)
@query_group
trait AriaDatabase
  # Input queries (set by IDE)
  fn file_text(file: FileId) -> String
  fn file_list() -> Array[FileId]

  # Derived queries (computed)
  fn parse(file: FileId) -> SyntaxTree
  fn resolve_names(file: FileId) -> NameResolution
  fn infer_types(func: FunctionId) -> TypeInfo
  fn check_contracts(func: FunctionId) -> ContractResult
end
```

### 9.2 Incrementality Boundaries

```aria
# Key insight: body changes are local
ModuleStructure
  ├── Function signatures    # Changing invalidates callers
  ├── Type definitions       # Changing invalidates users
  └── Imports               # Changing affects resolution

FunctionBody
  └── Implementation        # Changing ONLY invalidates this function
```

### 9.3 Syntax Tree Design

```aria
# Green tree (immutable, shared)
struct GreenNode {
  kind: SyntaxKind
  children: Array[Arc[GreenChild]]
  text_len: Int
}

# Red tree (cursor with parent info)
struct SyntaxNode {
  green: Arc[GreenNode]
  parent: Option[SyntaxNode]
  offset: Int
}
```

### 9.4 Error Recovery

```aria
# Parser produces tree even with errors
fn parse(text: String) -> (SyntaxTree, Array[ParseError])
  # Never fails - always returns tree
  # Errors stored separately
  # IDE features work on partial code
end
```

### 9.5 Effect-Aware IDE Features

```aria
# Unique to Aria: effect tracking in IDE
fn infer_effects(func: FunctionId) -> EffectSet
  # Analyze function body
  # Report used effects
  # Suggest effect handlers
end

# IDE feature: "Add effect handler"
fn suggest_handler(effect: Effect, location: Position) -> CodeAction
```

---

## 10. Key Resources

1. [rust-analyzer Architecture](https://rust-analyzer.github.io/book/contributing/architecture.html)
2. [Salsa Book](https://salsa-rs.github.io/salsa/)
3. [Rowan (Syntax Trees)](https://github.com/rust-analyzer/rowan)
4. [rust-analyzer GitHub](https://github.com/rust-lang/rust-analyzer)
5. [RFC 2912: rust-analyzer](https://rust-lang.github.io/rfcs/2912-rust-analyzer.html)

---

## 11. Open Questions

1. Should Aria use Salsa directly or build custom incrementality?
2. How do we handle effects in IDE features (completion, diagnostics)?
3. What's the integration strategy with LSP?
4. How do we support multiple backends (native, WASM) in IDE?
