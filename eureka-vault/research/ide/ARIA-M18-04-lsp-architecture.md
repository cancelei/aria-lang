# ARIA-M18-04: LSP Architecture and Query-Based Compilation Design

**Task ID**: ARIA-M18-04
**Status**: Completed
**Date**: 2026-01-15
**Agent**: LUMEN (Eureka Iteration 3)
**Focus**: Query-based compilation, incremental analysis, error recovery, LSP feature prioritization

---

## Executive Summary

This document presents a comprehensive architecture for Aria's Language Server Protocol (LSP) implementation. Drawing from deep research into rust-analyzer, Salsa, Tree-sitter, and modern IDE tooling practices, we propose a **query-based incremental compilation architecture** optimized for IDE responsiveness.

**Key Architectural Decisions:**
1. **Query-based compilation** using Salsa-style incremental computation
2. **Red-green syntax trees** (Rowan-inspired) for lossless, error-tolerant parsing
3. **Durability-aware caching** for multi-tier invalidation optimization
4. **Resilient LL parsing** with explicit recovery sets for IDE scenarios
5. **Feature-tiered implementation** prioritizing high-impact developer productivity features

---

## 1. Query-Based Compilation Architecture

### 1.1 Core Philosophy

Traditional compilers operate as pipelines: source flows through lexing, parsing, type checking, and code generation in sequence. This model fails for IDE use cases because:
- Any change requires restarting the entire pipeline
- Incomplete code causes fatal errors
- Users experience latency while waiting for full recompilation

The **query-based architecture** inverts this model:

```
Traditional Pipeline:
  Source -> Lex -> Parse -> Resolve -> TypeCheck -> Generate
  (sequential, batch processing)

Query-Based Architecture:
  Source -> Incremental Database <- Query on Demand
  (persistent, demand-driven computation)
```

Reference: [rust-analyzer Architecture](https://rust-analyzer.github.io/book/contributing/architecture.html)

### 1.2 Salsa Framework Integration

[Salsa](https://github.com/salsa-rs/salsa) provides the foundation for Aria's incremental computation:

| Concept | Description |
|---------|-------------|
| **Input Queries** | Base data provided externally (file contents, config) |
| **Derived Queries** | Pure functions computed from inputs |
| **Memoization** | Results cached automatically |
| **Dependency Tracking** | Automatic tracking of query dependencies |
| **Early Cutoff** | Stop revalidation when derived value unchanged |

### 1.3 Aria Database Schema

```rust
// aria-lsp/src/database.rs
use salsa::Database;

#[salsa::database(
    SourceDatabaseStorage,
    ParserDatabaseStorage,
    NameResolutionStorage,
    TypeDatabaseStorage,
    EffectDatabaseStorage,
    ContractDatabaseStorage,
)]
pub struct AriaDatabase {
    storage: salsa::Storage<Self>,
}

// Query Groups with Clear Boundaries
#[salsa::query_group(SourceDatabaseStorage)]
pub trait SourceDatabase {
    #[salsa::input]
    fn file_text(&self, file: FileId) -> Arc<String>;

    #[salsa::input]
    fn file_set(&self) -> Arc<FileSet>;

    #[salsa::input]
    fn stdlib(&self) -> Arc<StdlibInfo>;
}

#[salsa::query_group(ParserDatabaseStorage)]
pub trait ParserDatabase: SourceDatabase {
    fn parse(&self, file: FileId) -> Parse<SourceFile>;
    fn item_tree(&self, file: FileId) -> Arc<ItemTree>;
}

#[salsa::query_group(TypeDatabaseStorage)]
pub trait TypeDatabase: NameResDatabase {
    fn fn_signature(&self, func: FunctionId) -> Arc<FnSignature>;
    fn infer_body(&self, func: FunctionId) -> Arc<InferenceResult>;
}
```

### 1.4 Query Dependency Graph

```
                        ┌─────────────────┐
                        │   file_text()   │  [INPUT - Durability::LOW]
                        └────────┬────────┘
                                 │
                                 v
                        ┌─────────────────┐
                        │    parse()      │
                        └────────┬────────┘
                                 │
               ┌─────────────────┼─────────────────┐
               │                 │                 │
               v                 v                 v
    ┌──────────────────┐  ┌────────────┐  ┌────────────────┐
    │   item_tree()    │  │ fn_body()  │  │  diagnostics() │
    │  (signatures)    │  │  (local)   │  │   (syntax)     │
    └────────┬─────────┘  └──────┬─────┘  └────────────────┘
             │                   │
             v                   │
    ┌──────────────────┐         │
    │  fn_signature()  │         │
    │  [stable edge]   │         │
    └────────┬─────────┘         │
             │                   │
             └─────────┬─────────┘
                       │
                       v
            ┌──────────────────┐
            │   infer_body()   │
            │  [per-function]  │
            └──────────────────┘
```

**Critical Invariant**: `fn_signature()` depends only on `item_tree()`, NOT on the function body. This ensures that editing a function body:
1. Invalidates `parse(file)` for that file
2. Invalidates `infer_body(func)` for that function
3. Does NOT invalidate `fn_signature(func)` (only signature syntax changes do)
4. Does NOT invalidate `infer_body()` for OTHER functions

Reference: [rust-analyzer Guide](https://rust-analyzer.github.io/book/contributing/guide.html)

---

## 2. Incremental Analysis Strategies

### 2.1 Durability System

[Salsa's durability optimization](https://rust-analyzer.github.io/blog/2023/07/24/durable-incrementality.html) categorizes inputs by change frequency:

| Durability | Description | Examples | Update Frequency |
|------------|-------------|----------|------------------|
| **HIGH** | Rarely changes | Standard library, external dependencies | Once per session |
| **MEDIUM** | Occasionally changes | Aria.toml, project config | Minutes to hours |
| **LOW** | Frequently changes | User source code being edited | Milliseconds |

**Implementation:**

```rust
impl AriaDatabase {
    pub fn set_file_with_durability(
        &mut self,
        file: FileId,
        content: String,
        durability: Durability
    ) {
        self.set_file_text_with_durability(
            file,
            Arc::new(content),
            durability
        );
    }

    pub fn apply_change(&mut self, change: FileChange) {
        match change.source {
            ChangeSource::UserEdit => {
                self.set_file_with_durability(
                    change.file,
                    change.content,
                    Durability::LOW
                );
            }
            ChangeSource::DependencyUpdate => {
                self.set_file_with_durability(
                    change.file,
                    change.content,
                    Durability::MEDIUM
                );
            }
            ChangeSource::StdlibInit => {
                self.set_file_with_durability(
                    change.file,
                    change.content,
                    Durability::HIGH
                );
            }
        }
    }
}
```

### 2.2 Version Vectors

Instead of a single global revision counter, Salsa tracks `(high_rev, medium_rev, low_rev)`. This enables significant optimization:

```
Scenario: User types in main.aria (LOW durability file)

Without version vectors:
  - All queries checked for invalidation
  - Standard library queries re-validated (~300ms overhead)

With version vectors:
  - Only low_rev incremented
  - Queries depending only on HIGH durability skip validation entirely
  - Near-instant response
```

### 2.3 Early Cutoff Optimization

Salsa automatically implements early cutoff. When a derived query recomputes but produces the same result, dependent queries avoid recomputation:

```
User adds whitespace in function body:

1. file_text(foo.aria) - CHANGED (new content)
2. parse(foo.aria) - RECOMPUTED (source differs)
3. item_tree(foo.aria) - RECOMPUTED but UNCHANGED
   (whitespace doesn't affect item structure)
4. fn_signature(foo::bar) - NOT recomputed (early cutoff)
5. infer_body(callers) - NOT recomputed (dependencies unchanged)
```

### 2.4 Cancellation Architecture

IDE operations must be cancellable when users type faster than analysis completes:

```rust
pub struct AnalysisHost {
    db: AriaDatabase,
    pending: Arc<AtomicBool>,
}

impl AnalysisHost {
    pub fn snapshot(&self) -> Analysis {
        Analysis {
            db: self.db.snapshot(),
            pending: Arc::clone(&self.pending),
        }
    }

    pub fn apply_change(&mut self, change: Change) {
        // Signal cancellation to running analyses
        self.pending.store(true, Ordering::SeqCst);

        // Apply change
        self.db.apply_change(change);

        // Reset cancellation flag
        self.pending.store(false, Ordering::SeqCst);
    }
}

// In query implementations
fn infer_body(db: &dyn TypeDatabase, func: FunctionId) -> Arc<InferenceResult> {
    let ctx = InferenceContext::new(db, func);

    for stmt in body.statements.iter() {
        // Check cancellation periodically
        db.salsa_runtime().check_cancelled()?;
        ctx.infer_stmt(stmt);
    }

    Arc::new(ctx.finalize())
}
```

Reference: [Three Architectures for Responsive IDEs](https://rust-analyzer.github.io//blog/2020/07/20/three-architectures-for-responsive-ide.html)

---

## 3. Error Recovery Patterns

### 3.1 Design Philosophy

IDE parsers must handle incomplete and invalid code gracefully. The key insight from [Resilient LL Parsing Tutorial](https://matklad.github.io/2023/05/21/resilient-ll-parsing-tutorial.html):

> "Incomplete code is the ground truth, and only the user knows how to correctly complete it."

Our parser should **recognize valid syntactic structures** rather than attempting repair.

### 3.2 Red-Green Syntax Trees

Following [Rowan's architecture](https://github.com/rust-analyzer/rowan), Aria uses a two-layer syntax tree:

**Green Tree (Immutable, Shared):**
```rust
pub struct GreenNode {
    kind: SyntaxKind,
    width: TextSize,
    children: Box<[GreenChild]>,  // Arc shared across versions
}

pub enum GreenChild {
    Node(Arc<GreenNode>),
    Token { kind: SyntaxKind, text: Arc<str> },
}
```

**Red Tree (Cursor with Position):**
```rust
pub struct SyntaxNode {
    green: Arc<GreenNode>,
    parent: Option<Rc<SyntaxNode>>,
    offset: TextSize,  // Absolute position in source
}
```

Benefits:
- **Structural sharing**: Unchanged subtrees share memory across edits
- **Incremental reparsing**: Only reparse affected regions
- **Lossless**: Preserves all source information including whitespace and comments
- **Error-tolerant**: Any node can have arbitrary children, including ERROR nodes

Reference: [Red Green Syntax Trees Overview](https://willspeak.me/2021/11/24/red-green-syntax-trees-an-overview.html)

### 3.3 Recovery Set Strategy

Explicit recovery sets guide error recovery without grammar pollution:

```rust
// Recovery tokens for different contexts
const ITEM_RECOVERY: &[SyntaxKind] = &[
    T![fn], T![struct], T![enum], T![trait], T![impl], T![mod], T![use],
];

const STMT_RECOVERY: &[SyntaxKind] = &[
    T![let], T![if], T![while], T![for], T![return], T![fn], T![}],
];

const PARAM_RECOVERY: &[SyntaxKind] = &[
    T![,], T![)], T![fn], T![{], T![->],
];

// Resilient parsing pattern
fn parse_block(p: &mut Parser) {
    p.expect(T![{]);

    while !p.at(T![}]) && !p.at_eof() {
        if p.at_any(STMT_START) {
            parse_statement(p);
        } else {
            // At invalid token - check recovery set
            if p.at_any(ITEM_RECOVERY) {
                break;  // Let outer parser handle
            }
            p.advance_with_error("expected statement");
        }
    }

    p.expect(T![}]);
}
```

### 3.4 Error Node Representation

```rust
// Errors represented as nodes, not exceptions
pub enum SyntaxKind {
    // Regular syntax kinds
    FN_DEF,
    PARAM_LIST,
    BLOCK_EXPR,
    // ...

    // Error markers
    ERROR,          // General error node
    MISSING,        // Expected but not present
}

// Typed AST accessors return Option
impl FnDef {
    pub fn name(&self) -> Option<Name> {
        // Returns None if name is missing/error
        self.syntax().child_token(T![ident]).map(Name::new)
    }

    pub fn body(&self) -> Option<BlockExpr> {
        // Returns None if body is missing
        self.syntax().child_node(BLOCK_EXPR).map(BlockExpr::cast)
    }
}
```

### 3.5 IntelliJ Trick for Completion

rust-analyzer's approach to code completion in incomplete contexts:

```rust
fn completion_context(db: &dyn Database, position: FilePosition) -> CompletionContext {
    let file = db.file_text(position.file);

    // Insert dummy identifier at cursor position
    let modified = format!(
        "{}__aria_completion_marker__{}",
        &file[..position.offset],
        &file[position.offset..]
    );

    // Parse the modified file - now we have a valid identifier at cursor
    let parse = aria_parser::parse(&modified);

    // Find the dummy identifier in the tree
    let marker = find_marker(&parse);

    // Determine context from surrounding syntax
    CompletionContext::from_syntax(marker)
}
```

Reference: [rust-analyzer Guide - Completion](https://rust-analyzer.github.io/book/contributing/guide.html)

---

## 4. LSP Feature Prioritization

### 4.1 Feature Tiers

Based on [developer productivity research](https://blog.jetbrains.com/research/2025/10/state-of-developer-ecosystem-2025/), we prioritize features by impact:

**Tier 1: Foundation (Must Have)**
| Feature | LSP Method | Impact |
|---------|------------|--------|
| Diagnostics | `textDocument/publishDiagnostics` | Core functionality - shows errors |
| Go to Definition | `textDocument/definition` | Navigation essential |
| Hover | `textDocument/hover` | Type/doc information |
| Document Symbols | `textDocument/documentSymbol` | File navigation |

**Tier 2: Productivity (High Value)**
| Feature | LSP Method | Impact |
|---------|------------|--------|
| Completion | `textDocument/completion` | Write code faster |
| Find References | `textDocument/references` | Understand code usage |
| Rename | `textDocument/rename` | Safe refactoring |
| Signature Help | `textDocument/signatureHelp` | Function argument guidance |

**Tier 3: Enhanced Experience (Nice to Have)**
| Feature | LSP Method | Impact |
|---------|------------|--------|
| Semantic Tokens | `textDocument/semanticTokens` | Context-aware highlighting |
| Inlay Hints | `textDocument/inlayHint` | Show inferred types |
| Code Actions | `textDocument/codeAction` | Quick fixes |
| Folding Ranges | `textDocument/foldingRange` | Code organization |

**Tier 4: Advanced (Future)**
| Feature | LSP Method | Impact |
|---------|------------|--------|
| Type Hierarchy | `typeHierarchy/*` | Type relationship view |
| Call Hierarchy | `callHierarchy/*` | Call graph navigation |
| Workspace Symbols | `workspace/symbol` | Project-wide search |

### 4.2 Aria-Specific Features

Beyond standard LSP, Aria requires custom features:

**Effect System Integration:**
```rust
// Custom request: Show effect annotations
#[derive(Deserialize)]
struct EffectInfoParams {
    text_document: TextDocumentIdentifier,
    position: Position,
}

#[derive(Serialize)]
struct EffectInfo {
    effects: Vec<Effect>,
    handlers: Vec<HandlerLocation>,
    suggestions: Vec<String>,
}

// Hover includes effect information
fn hover_info(db: &dyn Database, pos: FilePosition) -> Option<HoverInfo> {
    let func = find_function_at(db, pos)?;
    let sig = db.fn_signature(func);
    let effects = db.infer_effects(func);

    Some(HoverInfo {
        type_info: format_type(&sig.return_type),
        effects: format_effects(&effects),
        contracts: format_contracts(&sig.contracts),
    })
}
```

**Contract Diagnostics:**
```rust
// Diagnostics for contract violations
fn contract_diagnostics(db: &dyn Database, file: FileId) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    for func in db.file_functions(file) {
        let result = db.check_contracts(func);

        for violation in result.violations {
            diags.push(Diagnostic {
                severity: DiagnosticSeverity::WARNING,
                message: format!(
                    "Contract may not hold: {}",
                    violation.contract
                ),
                related_information: Some(vec![
                    DiagnosticRelatedInformation {
                        location: violation.definition_site,
                        message: "Contract defined here".to_string(),
                    }
                ]),
            });
        }
    }

    diags
}
```

### 4.3 Semantic Highlighting

[Semantic tokens](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/) provide context-aware highlighting:

```rust
pub enum SemanticTokenType {
    Namespace,
    Type,
    Class,
    Enum,
    Interface,      // For Aria traits
    Struct,
    TypeParameter,
    Parameter,
    Variable,
    Property,       // For Aria struct fields
    EnumMember,
    Function,
    Method,
    Macro,
    Keyword,
    Modifier,       // For effect annotations
    Comment,
    String,
    Number,
    Operator,
}

pub enum SemanticTokenModifier {
    Declaration,
    Definition,
    Readonly,
    Static,
    Deprecated,
    Abstract,
    Async,
    Modification,   // For mutable bindings
    Documentation,
    DefaultLibrary,
    // Aria-specific
    Effectful,      // Functions with effects
    Pure,           // Pure functions
    Contracted,     // Functions with contracts
}
```

Reference: [LSP Specification](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/)

---

## 5. Implementation Architecture

### 5.1 Crate Structure

```
aria-lsp/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── main.rs              # LSP server entry point
│   │
│   ├── database/            # Salsa database
│   │   ├── mod.rs
│   │   ├── source.rs        # Source input queries
│   │   ├── parser.rs        # Parsing queries
│   │   ├── name_res.rs      # Name resolution queries
│   │   ├── types.rs         # Type inference queries
│   │   ├── effects.rs       # Effect inference queries
│   │   └── contracts.rs     # Contract checking queries
│   │
│   ├── syntax/              # Syntax tree (rowan-based)
│   │   ├── mod.rs
│   │   ├── green.rs         # Green tree implementation
│   │   ├── red.rs           # Red tree (SyntaxNode)
│   │   ├── ast.rs           # Typed AST layer
│   │   └── visitors.rs      # Tree traversal
│   │
│   ├── handlers/            # LSP request handlers
│   │   ├── mod.rs
│   │   ├── completion.rs
│   │   ├── definition.rs
│   │   ├── hover.rs
│   │   ├── references.rs
│   │   ├── rename.rs
│   │   ├── diagnostics.rs
│   │   ├── semantic_tokens.rs
│   │   └── code_actions.rs
│   │
│   ├── analysis/            # Analysis coordination
│   │   ├── mod.rs
│   │   ├── host.rs          # AnalysisHost (mutable)
│   │   └── snapshot.rs      # Analysis (immutable snapshot)
│   │
│   └── server/              # LSP server infrastructure
│       ├── mod.rs
│       ├── capabilities.rs  # LSP capability negotiation
│       ├── dispatch.rs      # Request/notification dispatch
│       └── vfs.rs           # Virtual file system
```

### 5.2 Server Lifecycle

```rust
// main.rs
fn main() -> anyhow::Result<()> {
    // Initialize logger
    env_logger::init();

    // Create LSP connection
    let (connection, io_threads) = Connection::stdio();

    // Initialize server capabilities
    let server_capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(
            TextDocumentSyncKind::INCREMENTAL
        )),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec![".".to_string(), ":".to_string()]),
            ..Default::default()
        }),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        definition_provider: Some(OneOf::Left(true)),
        references_provider: Some(OneOf::Left(true)),
        rename_provider: Some(OneOf::Right(RenameOptions {
            prepare_provider: Some(true),
            work_done_progress_options: Default::default(),
        })),
        semantic_tokens_provider: Some(
            SemanticTokensServerCapabilities::SemanticTokensOptions(
                SemanticTokensOptions {
                    legend: semantic_tokens_legend(),
                    full: Some(SemanticTokensFullOptions::Bool(true)),
                    range: Some(true),
                    ..Default::default()
                }
            )
        ),
        inlay_hint_provider: Some(OneOf::Left(true)),
        ..Default::default()
    };

    // Perform initialization handshake
    let init_params = connection.initialize(serde_json::to_value(&server_capabilities)?)?;

    // Run main loop
    let mut server = AriaServer::new(init_params);
    server.run(connection)?;

    io_threads.join()?;
    Ok(())
}
```

### 5.3 Request Handling Pattern

```rust
impl AriaServer {
    fn handle_request(&mut self, req: Request) -> Response {
        // Take snapshot for concurrent analysis
        let analysis = self.analysis_host.snapshot();

        // Dispatch based on method
        match req.method.as_str() {
            "textDocument/definition" => {
                let params: GotoDefinitionParams = serde_json::from_value(req.params)?;
                let position = self.to_file_position(&params.text_document_position)?;

                match analysis.goto_definition(position) {
                    Ok(Some(location)) => {
                        Response::new_ok(req.id, serde_json::to_value(location)?)
                    }
                    Ok(None) => {
                        Response::new_ok(req.id, serde_json::Value::Null)
                    }
                    Err(Cancelled) => {
                        Response::new_err(
                            req.id,
                            ErrorCode::ContentModified as i32,
                            "Request cancelled".to_string()
                        )
                    }
                }
            }
            // ... other handlers
        }
    }
}
```

---

## 6. Performance Targets

### 6.1 Response Time Goals

| Operation | Target | Strategy |
|-----------|--------|----------|
| Keystroke response | <50ms | Incremental parsing, early cutoff |
| Go to Definition | <100ms | Cached name resolution |
| Completion | <200ms | Pre-computed scopes, lazy sorting |
| Hover | <100ms | Cached type info |
| Find References | <500ms | Index-based lookup |
| Full file diagnostics | <500ms | Per-function parallelism |
| Workspace diagnostics | <5s | Background computation |

### 6.2 Memory Management

```rust
// Periodic garbage collection of unused syntax trees
impl AriaServer {
    fn gc_if_needed(&mut self) {
        let memory_usage = self.analysis_host.memory_usage();

        if memory_usage > self.config.gc_threshold {
            // Retain only files with recent accesses
            self.analysis_host.gc(|file| {
                self.vfs.last_access(file)
                    .map(|t| t.elapsed() < Duration::from_secs(300))
                    .unwrap_or(false)
            });
        }
    }
}
```

### 6.3 Laziness Over Incrementality

Key insight from rust-analyzer research:

> "It's not incrementality that makes an IDE fast. Rather, it's laziness - the ability to skip huge swaths of code altogether."

For Aria:
- Only parse files when needed
- Only type-check functions when their types are queried
- Only check contracts when diagnostics are requested
- Defer effect inference until effects are displayed

---

## 7. Integration with Aria Compiler

### 7.1 Shared Infrastructure

The LSP server shares core infrastructure with the batch compiler:

```
aria-compiler/
├── aria-parser/      # Shared: Lexer, Parser, Syntax Trees
├── aria-hir/         # Shared: HIR definitions
├── aria-types/       # Shared: Type system core
├── aria-effects/     # Shared: Effect system
├── aria-contracts/   # Shared: Contract checking
│
aria-lsp/
├── src/
│   └── database/     # LSP-specific: Salsa integration
│   └── handlers/     # LSP-specific: Request handlers
```

### 7.2 Divergence Points

| Component | Batch Compiler | LSP |
|-----------|----------------|-----|
| Parsing | Fail on error | Error recovery |
| Type checking | All functions | On-demand per function |
| Diagnostics | Comprehensive | Incremental updates |
| Memory | Single pass | Persistent caches |

---

## 8. Implementation Roadmap

### Phase 1: Foundation (Weeks 1-3)
- [ ] Salsa database setup
- [ ] FileId/FunctionId interning
- [ ] Basic source database queries
- [ ] LSP connection and handshake

### Phase 2: Syntax (Weeks 4-5)
- [ ] Green tree implementation
- [ ] Red tree cursors
- [ ] Error recovery in parser
- [ ] Parse query with memoization

### Phase 3: Basic Features (Weeks 6-8)
- [ ] Go to Definition
- [ ] Hover information
- [ ] Document symbols
- [ ] Basic diagnostics

### Phase 4: Type Integration (Weeks 9-11)
- [ ] fn_signature query
- [ ] infer_body query (per-function)
- [ ] Type-aware hover
- [ ] Type error diagnostics

### Phase 5: Completion (Weeks 12-14)
- [ ] Completion context analysis
- [ ] Scope-based completions
- [ ] Import suggestions
- [ ] Signature help

### Phase 6: Advanced Features (Weeks 15-18)
- [ ] Find references
- [ ] Rename
- [ ] Semantic tokens
- [ ] Inlay hints
- [ ] Effect annotations in UI

### Phase 7: Polish (Weeks 19-20)
- [ ] Performance optimization
- [ ] Memory management
- [ ] Comprehensive testing
- [ ] Documentation

---

## 9. Key Resources

### Architecture References
1. [rust-analyzer Architecture](https://rust-analyzer.github.io/book/contributing/architecture.html)
2. [Salsa GitHub Repository](https://github.com/salsa-rs/salsa)
3. [Rowan Syntax Trees](https://github.com/rust-analyzer/rowan)
4. [Durable Incrementality](https://rust-analyzer.github.io/blog/2023/07/24/durable-incrementality.html)
5. [Three Architectures for Responsive IDEs](https://rust-analyzer.github.io//blog/2020/07/20/three-architectures-for-responsive-ide.html)

### Error Recovery
6. [Resilient LL Parsing Tutorial](https://matklad.github.io/2023/05/21/resilient-ll-parsing-tutorial.html)
7. [Red Green Syntax Trees Overview](https://willspeak.me/2021/11/24/red-green-syntax-trees-an-overview.html)
8. [Microsoft Tolerant PHP Parser](https://github.com/microsoft/tolerant-php-parser)

### LSP Protocol
9. [LSP Specification 3.17](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/)
10. [VS Code Language Server Extension Guide](https://code.visualstudio.com/api/language-extensions/language-server-extension-guide)

### Incremental Parsing
11. [Tree-sitter Documentation](https://tree-sitter.github.io/tree-sitter/)
12. [Incremental Parsing with Tree-sitter](https://tomassetti.me/incremental-parsing-using-tree-sitter/)

---

## 10. Open Questions

1. **Salsa Version**: Should we use salsa 0.16 (stable) or salsa 0.17+ (newer parallel features)?

2. **Tree-sitter Integration**: Should Aria use Tree-sitter for incremental lexing alongside the Rowan-style syntax tree?

3. **Effect Visualization**: How should effects be displayed in hover/inlay hints? As annotations? Separate panel?

4. **Contract Verification**: Should static contract checking run on every edit, or only on save/explicit request?

5. **Multi-target Support**: How do we handle per-target type differences in the LSP (native vs WASM)?

6. **Macro Expansion**: How should macro expansion integrate with the query system for IDE features?

---

## 11. Conclusion

Aria's LSP implementation builds on proven patterns from rust-analyzer while adapting to Aria's unique features:

- **Query-based architecture** ensures responsive incremental analysis
- **Red-green syntax trees** enable error-tolerant parsing for incomplete code
- **Durability-aware caching** optimizes for typical editing patterns
- **Feature prioritization** focuses effort on high-impact developer productivity
- **Aria-specific extensions** expose effects, contracts, and ownership information

By following this architecture, Aria can deliver a world-class IDE experience that matches or exceeds established languages while showcasing Aria's innovative features.

---

*Document prepared by LUMEN - Eureka Iteration 3 Research Agent*
*Last updated: 2026-01-15*
