# ARIA-M18-03: Incremental Compilation Architecture for Aria LSP

**Task ID**: ARIA-M18-03
**Status**: In Progress
**Date**: 2026-01-15
**Focus**: Query-based incremental compilation for IDE responsiveness
**Prerequisites**: ARIA-M18-01 (rust-analyzer architecture study)

---

## Executive Summary

This document designs Aria's incremental compilation architecture for LSP responsiveness. Based on the salsa framework used by rust-analyzer, we propose a query-based compilation system that enables:

- **Sub-100ms response times** for typical IDE operations
- **Minimal recomputation** when editing function bodies
- **Efficient memory usage** through structural sharing
- **Cancellation support** for outdated queries

The key insight: **"Typing inside a function body should never invalidate global derived data."**

---

## 1. Salsa Framework Analysis

### 1.1 Core Concepts

[Salsa](https://github.com/salsa-rs/salsa) is an incremental computation framework with these key properties:

| Concept | Description |
|---------|-------------|
| **Input Queries** | Base data provided externally (file contents) |
| **Derived Queries** | Pure functions computed from inputs |
| **Memoization** | Results cached automatically |
| **Dependency Tracking** | Which queries depend on what |
| **Early Cutoff** | Stop revalidation when derived value unchanged |

### 1.2 Query Types

```
Input Queries (set by IDE):
  - file_text(FileId) -> String
  - file_list() -> Vec<FileId>
  - config() -> CompilerConfig

Derived Queries (computed):
  - parse(FileId) -> SyntaxTree
  - resolve_names(FileId) -> NameResolution
  - item_signatures(FileId) -> Signatures
  - infer_body(FunctionId) -> TypedBody
```

### 1.3 Durability System

Salsa's [durability optimization](https://rust-analyzer.github.io/blog/2023/07/24/durable-incrementality.html) categorizes data by change frequency:

| Level | Description | Examples |
|-------|-------------|----------|
| **HIGH** | Rarely changes | Standard library, dependencies |
| **MEDIUM** | Occasionally changes | Cargo.toml, config files |
| **LOW** | Frequently changes | User source code |

**Version Vector**: Instead of a single revision, Salsa tracks `(high_rev, medium_rev, low_rev)`. When validating queries, if only `low_rev` changed and a query only uses MEDIUM+ inputs, validation is skipped entirely.

---

## 2. Aria Query Database Design

### 2.1 Database Structure

```rust
// aria-db/src/lib.rs
use salsa::Database;

#[salsa::database(
    SourceDatabaseStorage,
    ParserDatabaseStorage,
    NameResDatabaseStorage,
    TypeDatabaseStorage,
    HirDatabaseStorage,
)]
pub struct AriaDatabase {
    storage: salsa::Storage<Self>,
}

impl salsa::Database for AriaDatabase {
    fn salsa_runtime(&self) -> &salsa::Runtime {
        self.storage.runtime()
    }
}
```

### 2.2 Query Groups

#### 2.2.1 Source Database (Input Queries)

```rust
#[salsa::query_group(SourceDatabaseStorage)]
pub trait SourceDatabase {
    /// File content - the fundamental input
    #[salsa::input]
    fn file_text(&self, file: FileId) -> Arc<String>;

    /// All known files in the workspace
    #[salsa::input]
    fn file_set(&self) -> Arc<FileSet>;

    /// Standard library files (HIGH durability)
    #[salsa::input]
    fn stdlib_files(&self) -> Arc<FileSet>;

    /// Compiler configuration
    #[salsa::input]
    fn config(&self) -> Arc<CompilerConfig>;
}
```

#### 2.2.2 Parser Database (Derived Queries)

```rust
#[salsa::query_group(ParserDatabaseStorage)]
pub trait ParserDatabase: SourceDatabase {
    /// Parse file to syntax tree
    fn parse(&self, file: FileId) -> Parse<SourceFile>;

    /// Extract top-level items (signatures only)
    fn file_item_tree(&self, file: FileId) -> Arc<ItemTree>;
}

fn parse(db: &dyn ParserDatabase, file: FileId) -> Parse<SourceFile> {
    let text = db.file_text(file);
    // Parser produces tree even with errors (error recovery)
    aria_parser::parse(&text)
}

fn file_item_tree(db: &dyn ParserDatabase, file: FileId) -> Arc<ItemTree> {
    let parse = db.parse(file);
    Arc::new(ItemTree::from_syntax(parse.syntax()))
}
```

#### 2.2.3 Name Resolution Database

```rust
#[salsa::query_group(NameResDatabaseStorage)]
pub trait NameResDatabase: ParserDatabase {
    /// Module structure for a file
    fn module_scope(&self, file: FileId) -> Arc<ModuleScope>;

    /// Resolve imports
    fn imports(&self, file: FileId) -> Arc<ImportMap>;

    /// Crate-level item resolution
    fn crate_def_map(&self) -> Arc<CrateDefMap>;
}
```

#### 2.2.4 Type Database (Function-Level)

```rust
#[salsa::query_group(TypeDatabaseStorage)]
pub trait TypeDatabase: NameResDatabase {
    /// Function signature (return type, param types)
    /// CRITICAL: Body changes DON'T invalidate this
    fn fn_signature(&self, func: FunctionId) -> Arc<FnSignature>;

    /// Type-check function body
    /// This is per-function, isolated from other functions
    fn infer_body(&self, func: FunctionId) -> Arc<InferenceResult>;

    /// Type of an expression within a function
    fn expr_type(&self, func: FunctionId, expr: ExprId) -> Type;
}
```

### 2.3 Query Dependency Graph

```
                    ┌────────────────┐
                    │  file_text()   │  [INPUT - LOW durability]
                    └───────┬────────┘
                            │
                            ▼
                    ┌────────────────┐
                    │    parse()     │
                    └───────┬────────┘
                            │
              ┌─────────────┴─────────────┐
              │                           │
              ▼                           ▼
    ┌──────────────────┐        ┌──────────────────┐
    │  file_item_tree()│        │   (body AST)     │
    │  (signatures)    │        │   (local only)   │
    └────────┬─────────┘        └────────┬─────────┘
             │                           │
             ▼                           │
    ┌──────────────────┐                 │
    │  fn_signature()  │                 │
    │  [stable edge]   │                 │
    └────────┬─────────┘                 │
             │                           │
             └───────────┬───────────────┘
                         │
                         ▼
               ┌──────────────────┐
               │   infer_body()   │
               │  [per-function]  │
               └──────────────────┘
```

**Key Invariant**: `fn_signature()` depends only on `file_item_tree()`, NOT on the function body. Therefore, editing a function body:
1. Invalidates `parse()` for that file
2. Invalidates `infer_body()` for that function
3. Does NOT invalidate `fn_signature()` (only signature syntax changes do)
4. Does NOT invalidate `infer_body()` for OTHER functions

---

## 3. Incremental Type Checking Strategy

### 3.1 Current TypeChecker Analysis

Aria's existing `TypeChecker` in `aria-types/src/lib.rs` has these characteristics:

| Component | Current State | Incremental Requirement |
|-----------|--------------|------------------------|
| `TypeEnv` | Scope-based, uses `Rc<TypeEnv>` | Need salsa-managed scopes |
| `TypeInference` | Mutable state for unification | Need per-function inference |
| `TypeChecker` | Processes whole program | Need query-based architecture |

### 3.2 Incremental Architecture

```rust
/// Per-function inference context (NOT stored in salsa)
pub struct InferenceContext<'db> {
    db: &'db dyn TypeDatabase,
    func: FunctionId,
    /// Type variables for this function only
    type_vars: Vec<Type>,
    /// Substitution map (local to this inference)
    subst: FxHashMap<TypeVar, Type>,
    /// Collected diagnostics
    diagnostics: Vec<TypeDiagnostic>,
}

impl<'db> InferenceContext<'db> {
    pub fn new(db: &'db dyn TypeDatabase, func: FunctionId) -> Self {
        Self {
            db,
            func,
            type_vars: Vec::new(),
            subst: FxHashMap::default(),
            diagnostics: Vec::new(),
        }
    }

    /// Infer types for the function body
    pub fn infer(mut self) -> InferenceResult {
        let sig = self.db.fn_signature(self.func);
        let body = self.db.fn_body(self.func);

        // Set up parameter types from signature
        for (param, ty) in body.params.iter().zip(sig.params.iter()) {
            self.bind_pattern(param, ty);
        }

        // Infer body expression
        let body_ty = self.infer_expr(body.body_expr);

        // Unify with return type
        self.unify(&body_ty, &sig.return_type);

        InferenceResult {
            expr_types: self.finalize_expr_types(),
            diagnostics: self.diagnostics,
        }
    }
}

/// Salsa query implementation
fn infer_body(db: &dyn TypeDatabase, func: FunctionId) -> Arc<InferenceResult> {
    let ctx = InferenceContext::new(db, func);
    Arc::new(ctx.infer())
}
```

### 3.3 External Type Lookup

When inferring a function body, we need types of external functions. This uses `fn_signature()`:

```rust
impl<'db> InferenceContext<'db> {
    fn infer_call(&mut self, callee: ExprId, args: &[ExprId]) -> Type {
        let callee_ty = self.infer_expr(callee);

        match callee_ty {
            Type::Function { params, return_type } => {
                // Check argument types against parameters
                for (arg, param_ty) in args.iter().zip(params.iter()) {
                    let arg_ty = self.infer_expr(*arg);
                    self.unify(&arg_ty, param_ty);
                }
                *return_type
            }
            Type::Var(_) => {
                // Unknown function - create constraints
                let arg_tys: Vec<_> = args.iter()
                    .map(|a| self.infer_expr(*a))
                    .collect();
                let ret = self.fresh_var();
                let expected = Type::Function {
                    params: arg_tys,
                    return_type: Box::new(ret.clone()),
                };
                self.unify(&callee_ty, &expected);
                ret
            }
            _ => {
                self.error("not a function");
                Type::Error
            }
        }
    }

    fn resolve_external_function(&self, name: &str) -> Option<Arc<FnSignature>> {
        // This call is tracked by salsa
        // Dependency: infer_body(self.func) -> fn_signature(resolved_func)
        let resolved = self.db.resolve_name(self.func.file(), name)?;
        match resolved {
            Definition::Function(func_id) => {
                Some(self.db.fn_signature(func_id))
            }
            _ => None,
        }
    }
}
```

---

## 4. Cache Invalidation Rules

### 4.1 Invalidation Matrix

| Change Type | Invalidates |
|-------------|-------------|
| Edit function body | `parse(file)`, `infer_body(func)` |
| Edit function signature | `parse(file)`, `fn_signature(func)`, `infer_body(callers)` |
| Add/remove function | `file_item_tree(file)`, `crate_def_map()` |
| Add/remove import | `imports(file)`, dependent resolutions |
| Edit type definition | `fn_signature(users)`, `infer_body(users)` |

### 4.2 Durability Assignment

```rust
impl AriaDatabase {
    pub fn set_file(&mut self, file: FileId, content: String, durability: Durability) {
        self.set_file_text_with_durability(file, Arc::new(content), durability);
    }

    pub fn apply_change(&mut self, change: Change) {
        for (file, content) in change.files_changed {
            // User files are LOW durability
            self.set_file(file, content, Durability::LOW);
        }

        for (file, content) in change.stdlib_added {
            // Standard library is HIGH durability
            self.set_file(file, content, Durability::HIGH);
        }
    }
}
```

### 4.3 Early Cutoff Optimization

Salsa automatically implements early cutoff. For example:

```
User edits foo.aria, adding whitespace in function body:

1. file_text(foo.aria) - CHANGED (new content)
2. parse(foo.aria) - RECOMPUTED (different source)
3. file_item_tree(foo.aria) - RECOMPUTED but UNCHANGED
   (whitespace doesn't affect item structure)
4. fn_signature(foo::bar) - NOT recomputed (early cutoff at step 3)
5. infer_body(other functions) - NOT recomputed
```

---

## 5. File Change Handling

### 5.1 LSP Document Sync

```rust
pub struct AriaServer {
    db: AriaDatabase,
    vfs: Vfs,  // Virtual file system
    analysis_host: AnalysisHost,
}

impl AriaServer {
    pub fn on_did_change(&mut self, params: DidChangeTextDocumentParams) {
        let file_id = self.vfs.file_id(&params.text_document.uri);

        // Apply incremental changes
        let new_content = self.vfs.apply_changes(
            file_id,
            &params.content_changes
        );

        // Update database (this triggers invalidation)
        self.db.set_file_text(file_id, Arc::new(new_content));

        // Trigger re-analysis (salsa handles what to recompute)
        self.request_diagnostics(file_id);
    }

    fn request_diagnostics(&self, file: FileId) {
        // Salsa recomputes only what's needed
        let diagnostics = self.db.diagnostics(file);
        self.send_diagnostics(file, diagnostics);
    }
}
```

### 5.2 Cancellation Support

```rust
impl AriaDatabase {
    pub fn check_cancelled(&self) -> Result<(), Cancelled> {
        self.salsa_runtime().check_cancelled()
    }
}

// In query implementations
fn infer_body(db: &dyn TypeDatabase, func: FunctionId) -> Arc<InferenceResult> {
    // Check cancellation periodically
    db.check_cancelled()?;

    let ctx = InferenceContext::new(db, func);

    for stmt in body.statements {
        db.check_cancelled()?;  // Allow cancellation between statements
        ctx.infer_stmt(stmt);
    }

    Arc::new(ctx.finalize())
}
```

### 5.3 Revision Tracking

```rust
pub struct AnalysisHost {
    db: AriaDatabase,
    revision: u64,
}

impl AnalysisHost {
    pub fn apply_change(&mut self, change: Change) {
        self.db.apply_change(change);
        self.revision += 1;
    }

    pub fn snapshot(&self) -> Analysis {
        Analysis {
            db: self.db.snapshot(),
            revision: self.revision,
        }
    }
}

// Analysis can be used from background threads
pub struct Analysis {
    db: salsa::Snapshot<AriaDatabase>,
    revision: u64,
}

impl Analysis {
    pub fn diagnostics(&self, file: FileId) -> Cancellable<Vec<Diagnostic>> {
        self.db.catch_cancelled(|| self.db.diagnostics(file))
    }
}
```

---

## 6. Integration with Existing TypeChecker

### 6.1 Migration Path

```
Phase 1: Query Infrastructure
  - Add salsa dependency
  - Define FileId, FunctionId interning
  - Implement SourceDatabase queries

Phase 2: Parser Integration
  - Wrap existing parser in salsa query
  - Implement ItemTree extraction
  - Add error recovery improvements

Phase 3: Name Resolution
  - Implement ModuleScope queries
  - Add import resolution
  - Build CrateDefMap

Phase 4: Type System Integration
  - Refactor TypeChecker to per-function
  - Implement fn_signature query
  - Implement infer_body query
  - Migrate TypeEnv to salsa-managed scopes
```

### 6.2 Adapter for Existing Code

```rust
// Temporary adapter during migration
pub fn check_file_legacy(db: &dyn TypeDatabase, file: FileId) -> Vec<TypeError> {
    let parse = db.parse(file);
    let mut checker = TypeChecker::new();

    // Use existing TypeChecker for now
    if let Err(e) = checker.check_program(&parse.syntax()) {
        return vec![e];
    }

    checker.errors().to_vec()
}

// Future: Query-based implementation
pub fn check_file(db: &dyn TypeDatabase, file: FileId) -> Vec<TypeError> {
    let item_tree = db.file_item_tree(file);
    let mut errors = Vec::new();

    for func in item_tree.functions() {
        let result = db.infer_body(func);
        errors.extend(result.diagnostics.iter().cloned());
    }

    errors
}
```

---

## 7. Performance Targets

| Operation | Target | Strategy |
|-----------|--------|----------|
| Keystroke response | <50ms | Incremental parsing, early cutoff |
| Go-to-definition | <100ms | Cached name resolution |
| Completion | <200ms | Pre-computed scopes |
| Full file diagnostics | <500ms | Per-function parallelism |
| Workspace diagnostics | <5s | Background computation |

### 7.1 Benchmarking Strategy

```rust
#[cfg(test)]
mod benchmarks {
    use super::*;

    #[test]
    fn bench_incremental_edit() {
        let mut db = AriaDatabase::default();

        // Initial parse
        db.set_file_text(file_id, Arc::new(LARGE_FILE.to_string()));
        let _ = db.diagnostics(file_id);

        // Simulate editing function body
        let modified = modify_function_body(LARGE_FILE);

        let start = Instant::now();
        db.set_file_text(file_id, Arc::new(modified));
        let _ = db.diagnostics(file_id);
        let elapsed = start.elapsed();

        // Should be fast - only one function re-analyzed
        assert!(elapsed < Duration::from_millis(100));
    }
}
```

---

## 8. Aria-Specific Considerations

### 8.1 Contract Checking

Aria's Design by Contract requires special handling:

```rust
#[salsa::query_group(ContractDatabaseStorage)]
pub trait ContractDatabase: TypeDatabase {
    /// Check contracts for a function
    fn check_contracts(&self, func: FunctionId) -> Arc<ContractResult>;
}

fn check_contracts(db: &dyn ContractDatabase, func: FunctionId) -> Arc<ContractResult> {
    let sig = db.fn_signature(func);
    let body = db.fn_body(func);
    let types = db.infer_body(func);

    // Contracts depend on both signature AND body types
    // But still isolated per-function
    ContractChecker::check(sig, body, types)
}
```

### 8.2 Effect Inference

For Aria's effect system (future):

```rust
/// Effect inference is per-function, similar to type inference
fn infer_effects(db: &dyn EffectDatabase, func: FunctionId) -> Arc<EffectSet> {
    let body = db.fn_body(func);
    let mut effects = EffectSet::empty();

    for call in body.calls() {
        let callee_sig = db.fn_signature(call.callee);
        effects.merge(&callee_sig.effects);
    }

    Arc::new(effects)
}
```

---

## 9. Implementation Roadmap

### Phase 1: Foundation (Week 1-2)
- [ ] Add salsa dependency to workspace
- [ ] Define core ID types (FileId, FunctionId)
- [ ] Implement SourceDatabase with file management
- [ ] Basic cancellation infrastructure

### Phase 2: Parsing Integration (Week 3-4)
- [ ] Wrap parser in salsa query
- [ ] Implement ItemTree for efficient signature extraction
- [ ] Add incremental parsing tests
- [ ] Benchmark parse invalidation

### Phase 3: Name Resolution (Week 5-6)
- [ ] Module scope computation
- [ ] Import resolution queries
- [ ] Cross-file name resolution
- [ ] Definition lookup for go-to-definition

### Phase 4: Type Integration (Week 7-8)
- [ ] Refactor TypeChecker to per-function model
- [ ] Implement fn_signature query
- [ ] Implement infer_body query
- [ ] Verify incremental invariant holds

### Phase 5: LSP Integration (Week 9-10)
- [ ] Wire up document sync
- [ ] Implement cancellation in handlers
- [ ] Background analysis host
- [ ] Performance optimization

---

## 10. Key Resources

1. [Salsa GitHub Repository](https://github.com/salsa-rs/salsa)
2. [rust-analyzer Architecture](https://rust-analyzer.github.io/book/contributing/architecture.html)
3. [Durable Incrementality Blog Post](https://rust-analyzer.github.io/blog/2023/07/24/durable-incrementality.html)
4. [Salsa Durability Documentation](https://docs.rs/salsa/0.16.1/salsa/struct.Durability.html)
5. [Rust Compiler Dev Guide - Salsa](https://rustc-dev-guide.rust-lang.org/queries/salsa.html)

---

## 11. Open Questions

1. **Salsa version**: Should we use salsa 0.16 (stable) or salsa 0.17+ (newer features)?
2. **Parallel type checking**: Can we parallelize `infer_body` across functions?
3. **Memory limits**: How do we handle GC for long-running sessions?
4. **Contract verification**: Should contract checking be a separate query or combined with type inference?
5. **Effect system integration**: How do effects interact with the incremental model?

---

## Appendix A: Existing TypeChecker Refactoring Notes

The current `TypeChecker` in `aria-types/src/lib.rs` needs these changes:

| Current | Required Change |
|---------|-----------------|
| `TypeChecker::new()` creates global env | Per-function initialization with signature |
| `check_program()` iterates all items | Query each function independently |
| `TypeEnv` uses `Rc<TypeEnv>` parent chain | Salsa-managed scope lookup |
| `TypeInference` is mutable | Create fresh instance per query |
| Errors accumulated in `Vec` | Return as part of `InferenceResult` |

The core unification logic in `TypeInference::unify()` and type resolution in `resolve_type()` can remain largely unchanged - they just need to be called from within a salsa query context.
