# ARIA-M07-03: Incremental Compilation and Build System Design

**Task ID**: ARIA-M07-03
**Status**: Completed
**Date**: 2026-01-15
**Focus**: Comprehensive build system architecture for Aria
**Research Agent**: DELTA (Eureka Research Agent)
**Prerequisites**:
- ARIA-M06-01 (Rust MIR Design)
- ARIA-M07-01 (LLVM vs Cranelift Comparison)
- ARIA-M18-03 (Incremental Compilation for LSP)
- ARIA-PD-012 (LSP and IDE Integration Design)

---

## Executive Summary

This document provides a comprehensive design for Aria's incremental compilation and build system, addressing six critical areas: compilation unit granularity, dependency tracking strategies, caching mechanisms, parallel compilation, effect system impact on incrementality, and warm vs cold build optimization.

**Key Architectural Decisions**:

1. **Compilation Unit**: Function-level granularity with module-level grouping for optimal cache reuse
2. **Dependency Tracking**: Salsa-style query-based system with three-tier durability and fine-grained dependency graphs
3. **Caching Strategy**: Three-layer cache (in-memory LRU, on-disk content-addressed, optional distributed)
4. **Parallelism**: Pipeline parallelism across phases with function-level work stealing
5. **Effect System Integration**: Effect signature fingerprinting with localized invalidation
6. **Build Optimization**: Tiered compilation with fast debug paths and profile-guided release builds

**Performance Targets**:
| Build Type | Target | Strategy |
|------------|--------|----------|
| Incremental (body edit) | <100ms | Function-level invalidation |
| Incremental (signature change) | <1s | Caller graph propagation |
| Clean build (10K LOC) | <10s | Maximum parallelism |
| Type-check only | <5s | Skip codegen phase |
| IDE keystroke response | <50ms | Demand-driven analysis |

---

## 1. Compilation Unit Granularity

### 1.1 Granularity Options Analysis

| Granularity | Cache Efficiency | Invalidation Precision | Complexity |
|-------------|------------------|----------------------|------------|
| **File** | Low | Coarse | Low |
| **Module** | Medium | Medium | Medium |
| **Function** | High | Fine | High |
| **Expression** | Very High | Very Fine | Very High |

### 1.2 Recommended: Function-Level with Module Grouping

Aria adopts **function-level granularity** for semantic analysis and **module-level grouping** for code generation artifacts.

```
COMPILATION UNIT HIERARCHY

Project
  └── Crate (aria.toml boundary)
       └── Module (file or mod block)
            └── Item (fn, struct, enum, trait, impl)
                 └── Function (primary cache unit for type inference)

CACHE ORGANIZATION

Per-Function Cache:
  - fn_signature(FunctionId) -> Signature     # Stable unless declaration changes
  - fn_body_hir(FunctionId) -> HIR            # Recomputed on body edit
  - infer_body(FunctionId) -> TypedBody       # Recomputed on body/dep change
  - infer_effects(FunctionId) -> EffectSet    # Recomputed on body/callee change

Per-Module Cache:
  - module_scope(ModuleId) -> Scope           # Recomputed on structure change
  - item_tree(FileId) -> ItemTree             # Signatures only, stable

Per-Crate Cache:
  - crate_def_map() -> DefMap                 # Global name resolution
  - codegen_unit(CGUId) -> ObjectCode         # Grouped for linking
```

### 1.3 The Signature Stability Principle

**Critical Invariant**: Function signatures depend ONLY on declaration syntax, NOT on function bodies.

```
EDIT IMPACT ANALYSIS

Scenario 1: Edit function body (whitespace/logic change)
  1. file_text(file) -> INVALIDATED
  2. parse(file) -> RECOMPUTED
  3. item_tree(file) -> RECOMPUTED but VALUE UNCHANGED
  4. fn_signature(func) -> NOT RECOMPUTED (early cutoff)
  5. infer_body(callers) -> NOT RECOMPUTED
  Result: Only the edited function's body is re-analyzed

Scenario 2: Edit function signature (parameter/return type)
  1. file_text(file) -> INVALIDATED
  2. parse(file) -> RECOMPUTED
  3. item_tree(file) -> RECOMPUTED, VALUE CHANGED
  4. fn_signature(func) -> RECOMPUTED
  5. infer_body(all callers) -> RECOMPUTED
  Result: Edited function + all callers re-analyzed
```

### 1.4 Codegen Unit (CGU) Design

For native code generation, Aria groups functions into Codegen Units to balance parallelism and link-time optimization:

```rust
/// Codegen Unit assignment strategy
enum CGUStrategy {
    /// One CGU per module (default for debug builds)
    PerModule,

    /// Fixed number of CGUs (release builds)
    /// Enables parallel codegen with LLVM/Cranelift
    Fixed { count: usize },  // Default: 16

    /// One CGU per function (maximum parallelism, poor LTO)
    PerFunction,

    /// Single CGU (maximum optimization, no parallelism)
    Single,
}
```

---

## 2. Dependency Tracking Strategies

### 2.1 Hash-Based Invalidation

#### 2.1.1 Content Hash Algorithm

```rust
/// Semantic content hash (ignores whitespace, comments)
fn compute_semantic_hash(content: &str) -> ContentHash {
    let tokens = lex_for_hash(content);  // Skip trivia
    let mut hasher = Blake3::new();

    for token in tokens {
        hasher.update(token.kind.as_bytes());
        hasher.update(token.text.as_bytes());
    }

    ContentHash(hasher.finalize())
}

/// Cache key incorporating all build inputs
struct CacheKey {
    content_hash: ContentHash,
    compiler_version: Version,
    target: Target,
    profile: Profile,
    config_hash: Hash,       // Compiler flags
    dependency_hashes: Vec<(DependencyId, ContentHash)>,
}
```

#### 2.1.2 Invalidation Propagation

```
HASH PROPAGATION FLOW

            source file hash
                   │
                   ▼
            ┌─────────────┐
            │   parse()   │
            └──────┬──────┘
                   │ syntax tree hash
            ┌──────┴──────┐
            ▼             ▼
     ┌───────────┐  ┌───────────┐
     │item_tree()│  │ fn_body() │
     └─────┬─────┘  └─────┬─────┘
           │ signature    │ body hash
           │ hash         │
           ▼              ▼
    ┌────────────┐  ┌────────────┐
    │fn_signature│  │infer_body()│
    └─────┬──────┘  └────────────┘
          │
          ▼
    caller dependencies
```

### 2.2 Fine-Grained Dependency Graphs

#### 2.2.1 Dependency Graph Structure

```rust
/// Fine-grained dependency tracking
struct DependencyGraph {
    /// Query -> Dependencies it reads from
    dependencies: HashMap<QueryKey, HashSet<QueryKey>>,

    /// Query -> Queries that depend on it (reverse edges)
    dependents: HashMap<QueryKey, HashSet<QueryKey>>,

    /// Dependency version for change detection
    versions: HashMap<QueryKey, u64>,
}

/// Query keys for all cached computations
enum QueryKey {
    // Input queries
    FileText(FileId),
    Config,

    // Derived queries
    Parse(FileId),
    ItemTree(FileId),
    ModuleScope(ModuleId),
    FnSignature(FunctionId),
    InferBody(FunctionId),
    InferEffects(FunctionId),
    CheckContracts(FunctionId),

    // Codegen queries
    MonomorphizedFn(FunctionId, TypeArgs),
    CodegenUnit(CGUId),
}
```

#### 2.2.2 Incremental Invalidation Algorithm

```
INVALIDATION ALGORITHM (On Source Change)

procedure invalidate(changed_queries: Set[QueryKey]):
    worklist = changed_queries.to_queue()
    invalidated = Set::new()

    while query = worklist.pop():
        if query in invalidated:
            continue

        invalidated.add(query)

        // Mark query result as stale
        mark_stale(query)

        // Propagate to dependents
        for dependent in dependents[query]:
            if not is_early_cutoff_candidate(dependent, query):
                worklist.push(dependent)

    return invalidated

// Early cutoff: If recomputed value equals old value, don't propagate
procedure is_early_cutoff_candidate(dependent, changed_dep):
    old_value = cache.get(changed_dep)
    new_value = recompute(changed_dep)

    return old_value == new_value
```

### 2.3 Salsa-Style Query-Based Architecture

#### 2.3.1 Query Database Design

Building on ARIA-PD-012's architecture, Aria's compiler uses a layered query database:

```
QUERY DATABASE HIERARCHY

┌──────────────────────────────────────────────────────────────┐
│  SourceDatabase (INPUT LAYER)                                │
│  - file_text(FileId) -> Arc<String>         [Durability: Varies]
│  - file_set() -> Arc<FileSet>               [Durability: MEDIUM]
│  - stdlib() -> Arc<StdlibData>              [Durability: HIGH]
│  - config() -> Arc<CompilerConfig>          [Durability: MEDIUM]
└──────────────────────────────┬───────────────────────────────┘
                               │
                               ▼
┌──────────────────────────────────────────────────────────────┐
│  ParserDatabase (SYNTAX LAYER)                               │
│  - parse(FileId) -> Parse<SourceFile>                        │
│  - item_tree(FileId) -> Arc<ItemTree>                        │
│  - syntax_errors(FileId) -> Vec<SyntaxError>                 │
└──────────────────────────────┬───────────────────────────────┘
                               │
                   ┌───────────┴───────────┐
                   ▼                       ▼
┌─────────────────────────┐   ┌─────────────────────────────────┐
│  NameResDatabase        │   │  TypeDatabase                   │
│  - module_scope()       │   │  - fn_signature(FunctionId)     │
│  - crate_def_map()      │   │  - infer_body(FunctionId)       │
│  - resolve_path()       │   │  - type_of_expr(FunctionId,Expr)│
└────────────┬────────────┘   └───────────────┬─────────────────┘
             │                                │
             └───────────────┬────────────────┘
                             ▼
┌──────────────────────────────────────────────────────────────┐
│  EffectDatabase (ARIA-SPECIFIC)                              │
│  - infer_effects(FunctionId) -> Arc<EffectSet>               │
│  - effect_handlers(FileId) -> Vec<HandlerInfo>               │
│  - effect_dependencies(FunctionId) -> Vec<FunctionId>        │
└──────────────────────────────┬───────────────────────────────┘
                               │
                               ▼
┌──────────────────────────────────────────────────────────────┐
│  ContractDatabase (ARIA-SPECIFIC)                            │
│  - check_contracts(FunctionId) -> Arc<ContractResult>        │
│  - contract_dependencies(FunctionId) -> Vec<ContractId>      │
│  - verification_status(FunctionId) -> VerificationResult     │
└──────────────────────────────┬───────────────────────────────┘
                               │
                               ▼
┌──────────────────────────────────────────────────────────────┐
│  CodegenDatabase (BACKEND LAYER)                             │
│  - lower_to_mir(FunctionId) -> Arc<MIR>                      │
│  - monomorphize(FunctionId, TypeArgs) -> Arc<MIR>            │
│  - codegen_unit(CGUId) -> Arc<ObjectCode>                    │
└──────────────────────────────────────────────────────────────┘
```

#### 2.3.2 Durability System

```rust
/// Three-tier durability for optimal cache invalidation
#[derive(Clone, Copy, PartialEq, Eq)]
enum Durability {
    /// Changes rarely (standard library, external dependencies)
    /// Session-level caching, expensive to invalidate
    HIGH,

    /// Changes occasionally (project config, aria.toml)
    /// Minutes-level caching
    MEDIUM,

    /// Changes frequently (user source code being edited)
    /// Keystroke-level caching
    LOW,
}

/// Version vector for efficient validation
struct VersionVector {
    high: u64,    // Incremented on HIGH durability changes
    medium: u64,  // Incremented on MEDIUM durability changes
    low: u64,     // Incremented on LOW durability changes
}

impl VersionVector {
    /// Quick check if query needs revalidation
    fn needs_revalidation(&self, query_deps: &QueryDeps, new_versions: &VersionVector) -> bool {
        // If query only depends on HIGH durability, only check high_rev
        if query_deps.max_durability == Durability::HIGH {
            return self.high != new_versions.high;
        }
        // ... similar for MEDIUM, LOW
    }
}
```

#### 2.3.3 Early Cutoff Optimization

```rust
/// Salsa query with early cutoff support
#[salsa::query_group(TypeDatabaseStorage)]
pub trait TypeDatabase: ParserDatabase + NameResDatabase {
    /// Function signature - stable even when body changes
    /// Enables early cutoff for callers
    fn fn_signature(&self, func: FunctionId) -> Arc<FnSignature>;

    /// Body type inference - changes when body or deps change
    fn infer_body(&self, func: FunctionId) -> Arc<InferenceResult>;
}

fn fn_signature(db: &dyn TypeDatabase, func: FunctionId) -> Arc<FnSignature> {
    let item_tree = db.item_tree(func.file());
    let item = item_tree.get(func.local_id());

    // Extract signature from declaration syntax only
    // Body changes don't affect this!
    Arc::new(FnSignature::from_item(item))
}
```

---

## 3. Caching Strategies

### 3.1 On-Disk Artifact Caching

#### 3.1.1 Cache Directory Structure

```
ARIA CACHE DIRECTORY LAYOUT

$ARIA_HOME/cache/
├── index.db                    # SQLite index for fast lookup
├── content/                    # Content-addressed blob storage
│   ├── ab/                     # First two hex chars of hash
│   │   └── ab3def...          # Full hash as filename
│   └── cd/
│       └── cd89ab...
├── metadata/                   # Per-project metadata
│   └── <project_hash>/
│       ├── dep_graph.bincode   # Serialized dependency graph
│       └── query_cache.db      # Query results
└── codegen/                    # Compiled artifacts
    └── <target>-<profile>/
        ├── lib<name>.rlib      # Rust-style library format
        └── <cgu_hash>.o        # Object files
```

#### 3.1.2 Content-Addressed Storage

```rust
/// Content-addressed artifact storage
struct ArtifactStore {
    root: PathBuf,
    index: SqliteConnection,
}

impl ArtifactStore {
    /// Store artifact with content-derived key
    fn store(&self, content: &[u8], metadata: &ArtifactMetadata) -> Result<ArtifactId> {
        let hash = blake3::hash(content);
        let id = ArtifactId(hash);

        // Content-addressed path: cache/content/ab/ab3def...
        let path = self.content_path(&id);

        if !path.exists() {
            fs::create_dir_all(path.parent()?)?;
            fs::write(&path, content)?;
        }

        // Update index
        self.index.execute(
            "INSERT OR REPLACE INTO artifacts (id, size, created, metadata) VALUES (?, ?, ?, ?)",
            params![id.to_hex(), content.len(), now(), metadata.to_json()],
        )?;

        Ok(id)
    }

    /// Retrieve artifact by content hash
    fn get(&self, id: &ArtifactId) -> Result<Option<Vec<u8>>> {
        let path = self.content_path(id);
        if path.exists() {
            Ok(Some(fs::read(&path)?))
        } else {
            Ok(None)
        }
    }
}
```

#### 3.1.3 Cache Eviction Policy

```rust
/// LRU-based cache eviction with size limits
struct CachePolicy {
    max_size_bytes: u64,      // Default: 10GB
    max_age_days: u32,        // Default: 30 days
    min_free_percent: u8,     // Default: 10%
}

impl CachePolicy {
    /// Evict stale entries to meet policy
    fn enforce(&self, store: &mut ArtifactStore) -> Result<EvictionStats> {
        let mut evicted = 0;
        let mut freed_bytes = 0;

        // Phase 1: Evict entries older than max_age
        let old_entries = store.query_older_than(self.max_age_days)?;
        for entry in old_entries {
            store.remove(&entry.id)?;
            evicted += 1;
            freed_bytes += entry.size;
        }

        // Phase 2: If still over size, evict LRU
        while store.total_size()? > self.max_size_bytes {
            if let Some(lru) = store.pop_lru()? {
                store.remove(&lru.id)?;
                evicted += 1;
                freed_bytes += lru.size;
            } else {
                break;
            }
        }

        Ok(EvictionStats { evicted, freed_bytes })
    }
}
```

### 3.2 Cross-Project Caching (sccache/ccache Patterns)

#### 3.2.1 Shared Cache Architecture

```
SHARED CACHE ARCHITECTURE

┌─────────────────────────────────────────────────────────────┐
│  Local Machine                                              │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐                     │
│  │Project A│  │Project B│  │Project C│                     │
│  └────┬────┘  └────┬────┘  └────┬────┘                     │
│       │            │            │                           │
│       └────────────┼────────────┘                           │
│                    │                                        │
│            ┌───────▼───────┐                               │
│            │  Local Cache  │  <- L1: In-memory LRU         │
│            │   (per-user)  │  <- L2: On-disk content-addr  │
│            └───────┬───────┘                               │
└────────────────────┼────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────┐
│  Remote Cache (Optional)                                    │
│  ┌─────────────────────────────────────────────────────────┐│
│  │  Cloud Storage / Redis / Custom Server                  ││
│  │  - Deduplication across team                            ││
│  │  - Pre-warmed with CI artifacts                         ││
│  │  - Region-aware for latency                             ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

#### 3.2.2 Cache Key Generation for Cross-Project Sharing

```rust
/// Cache key for cross-project artifact sharing
struct SharedCacheKey {
    /// Normalized source content hash (whitespace-insensitive)
    source_hash: ContentHash,

    /// Compiler version and feature flags
    compiler_fingerprint: CompilerFingerprint,

    /// Target triple (e.g., x86_64-unknown-linux-gnu)
    target: Target,

    /// Build profile settings that affect output
    profile_flags: ProfileFlags,

    /// Dependencies' compiled artifact hashes
    dependency_hashes: Vec<(CrateName, ArtifactHash)>,
}

impl SharedCacheKey {
    fn to_cache_key(&self) -> String {
        // Deterministic serialization for cache key
        let mut hasher = Blake3::new();
        hasher.update(&self.source_hash.0);
        hasher.update(&self.compiler_fingerprint.to_bytes());
        hasher.update(self.target.triple().as_bytes());
        hasher.update(&self.profile_flags.to_bytes());

        for (name, hash) in &self.dependency_hashes {
            hasher.update(name.as_bytes());
            hasher.update(&hash.0);
        }

        format!("aria-{}", hex::encode(hasher.finalize()))
    }
}
```

### 3.3 Distributed Caching

#### 3.3.1 Distributed Cache Protocol

```rust
/// Distributed cache client interface
trait DistributedCache {
    /// Check if artifact exists in remote cache
    async fn contains(&self, key: &SharedCacheKey) -> Result<bool>;

    /// Fetch artifact from remote cache
    async fn get(&self, key: &SharedCacheKey) -> Result<Option<Vec<u8>>>;

    /// Store artifact in remote cache
    async fn put(&self, key: &SharedCacheKey, artifact: &[u8]) -> Result<()>;

    /// Batch operations for efficiency
    async fn get_batch(&self, keys: &[SharedCacheKey]) -> Result<HashMap<SharedCacheKey, Vec<u8>>>;
}

/// Implementation for cloud object storage (S3, GCS, Azure Blob)
struct CloudStorageCache {
    bucket: String,
    prefix: String,
    client: CloudClient,
    compression: CompressionLevel,
}

/// Implementation for Redis-backed cache
struct RedisCache {
    cluster: RedisCluster,
    ttl: Duration,
    max_artifact_size: usize,
}
```

#### 3.3.2 Cache Warming Strategies

```
CI/CD CACHE WARMING WORKFLOW

1. Main Branch Build (CI)
   - Compile with --emit-cache-artifacts
   - Upload to distributed cache
   - Tag with git commit hash

2. PR Build (CI)
   - Fetch cache from main branch baseline
   - Only compile changed files
   - Upload new artifacts for PR branch

3. Developer Build (Local)
   - Check distributed cache first
   - Fall back to local compilation
   - Upload novel artifacts (opt-in)

WARM CACHE SOURCES (Priority Order)

1. Local in-memory LRU cache
2. Local on-disk content-addressed cache
3. Team distributed cache (same project)
4. Organization distributed cache (shared deps)
5. Public artifact cache (common crates)
```

### 3.4 Three-Layer Cache Architecture

```rust
/// Unified cache interface with fallback layers
struct LayeredCache {
    /// L1: In-memory LRU (fastest, smallest)
    memory: LruCache<QueryKey, CachedResult>,

    /// L2: On-disk content-addressed (fast, larger)
    disk: ArtifactStore,

    /// L3: Distributed cache (slower, largest, shared)
    remote: Option<Box<dyn DistributedCache>>,
}

impl LayeredCache {
    async fn get(&self, key: &QueryKey) -> Option<CachedResult> {
        // L1: Check memory
        if let Some(result) = self.memory.get(key) {
            return Some(result.clone());
        }

        // L2: Check disk
        if let Some(bytes) = self.disk.get(&key.to_artifact_id())? {
            let result = deserialize(&bytes)?;
            self.memory.insert(key.clone(), result.clone());
            return Some(result);
        }

        // L3: Check remote (if enabled)
        if let Some(remote) = &self.remote {
            if let Some(bytes) = remote.get(&key.to_shared_key()).await? {
                let result = deserialize(&bytes)?;
                // Populate L1 and L2
                self.disk.store(&bytes, &key.metadata())?;
                self.memory.insert(key.clone(), result.clone());
                return Some(result);
            }
        }

        None
    }
}
```

---

## 4. Parallel Compilation Strategies

### 4.1 Parallelism Opportunities

```
ARIA COMPILATION PIPELINE PARALLELISM

Phase 1: Parsing (File-Level Parallelism)
  ┌─────┬─────┬─────┬─────┐
  │ f1  │ f2  │ f3  │ f4  │  <- Each file parsed independently
  └──┬──┴──┬──┴──┬──┴──┬──┘
     │     │     │     │
     └─────┴─────┴─────┘
              │
Phase 2: Name Resolution (Module-Level)
  ┌─────────────────────────┐
  │ Build CrateDefMap       │  <- Sequential, but fast
  └───────────┬─────────────┘
              │
Phase 3: Type Inference (Function-Level Parallelism)
  ┌─────┬─────┬─────┬─────┬─────┬─────┬─────┬─────┐
  │fn1  │fn2  │fn3  │fn4  │fn5  │fn6  │fn7  │fn8  │
  └──┬──┴──┬──┴──┬──┴──┬──┴──┬──┴──┬──┴──┬──┴──┬──┘
     │     │     │     │     │     │     │     │
     └─────┴─────┴─────┴─────┴─────┴─────┴─────┘
              │
Phase 4: Effect/Contract Analysis (Function-Level)
  ┌─────┬─────┬─────┬─────┬─────┬─────┬─────┬─────┐
  │eff1 │eff2 │eff3 │con1 │con2 │con3 │con4 │con5 │
  └──┬──┴──┬──┴──┬──┴──┬──┴──┬──┴──┬──┴──┬──┴──┬──┘
     │     │     │     │     │     │     │     │
     └─────┴─────┴─────┴─────┴─────┴─────┴─────┘
              │
Phase 5: Codegen (CGU-Level Parallelism)
  ┌─────────┬─────────┬─────────┬─────────┐
  │  CGU1   │  CGU2   │  CGU3   │  CGU4   │  <- Fixed number (default: 16)
  └────┬────┴────┬────┴────┬────┴────┬────┘
       │         │         │         │
       └─────────┴─────────┴─────────┘
              │
Phase 6: Linking (Sequential)
  ┌─────────────────────────────────────────┐
  │         Link all object files            │
  └─────────────────────────────────────────┘
```

### 4.2 Work-Stealing Scheduler

```rust
/// Work-stealing parallel scheduler for compilation
struct ParallelScheduler {
    /// Thread pool with work-stealing deques
    pool: ThreadPool,

    /// Pending work items
    pending: DashMap<QueryKey, WorkItem>,

    /// Completed results
    completed: DashMap<QueryKey, CompletedWork>,

    /// Dependency graph for ordering
    deps: Arc<DependencyGraph>,
}

impl ParallelScheduler {
    /// Execute query with automatic parallelization
    fn execute(&self, root_query: QueryKey) -> Result<QueryResult> {
        // Find all queries needed (topological order)
        let all_queries = self.deps.transitive_deps(&root_query);

        // Partition into waves (queries with no pending deps)
        let waves = self.compute_waves(&all_queries);

        for wave in waves {
            // Execute wave in parallel
            self.pool.install(|| {
                wave.par_iter().for_each(|query| {
                    let result = self.execute_single(query);
                    self.completed.insert(query.clone(), result);
                });
            });
        }

        self.completed.get(&root_query).cloned()
    }

    /// Execute single query, blocking on dependencies
    fn execute_single(&self, query: &QueryKey) -> CompletedWork {
        // Wait for dependencies (already in completed map due to wave ordering)
        let deps: Vec<_> = self.deps.immediate_deps(query)
            .iter()
            .map(|d| self.completed.get(d).unwrap().clone())
            .collect();

        // Execute the query
        match query {
            QueryKey::Parse(file) => self.db.parse(*file),
            QueryKey::InferBody(func) => self.db.infer_body(*func),
            // ... other query types
        }
    }
}
```

### 4.3 Pipeline Parallelism

```rust
/// Pipeline stage that can overlap with previous stage
trait PipelineStage {
    type Input;
    type Output;

    /// Process item, may be called before all inputs are ready
    fn process(&self, item: Self::Input) -> Self::Output;

    /// Check if item is ready to process
    fn is_ready(&self, item: &Self::Input) -> bool;
}

/// Pipelined compilation execution
struct PipelinedCompiler {
    parse_stage: ParseStage,
    typecheck_stage: TypeCheckStage,
    codegen_stage: CodegenStage,
}

impl PipelinedCompiler {
    /// Start codegen for file1 while still typechecking file2
    async fn compile_pipelined(&self, files: Vec<FileId>) -> Result<Vec<ObjectFile>> {
        let (parse_tx, parse_rx) = channel::bounded(64);
        let (typecheck_tx, typecheck_rx) = channel::bounded(64);
        let (codegen_tx, codegen_rx) = channel::bounded(64);

        // Spawn pipeline stages
        let parse_handle = spawn(async move {
            for file in files {
                let parsed = self.parse_stage.process(file);
                parse_tx.send(parsed).await?;
            }
        });

        let typecheck_handle = spawn(async move {
            while let Some(parsed) = parse_rx.recv().await {
                // Can start typechecking as soon as parsing completes
                let typed = self.typecheck_stage.process(parsed);
                typecheck_tx.send(typed).await?;
            }
        });

        let codegen_handle = spawn(async move {
            while let Some(typed) = typecheck_rx.recv().await {
                // Can start codegen as soon as typechecking completes
                let object = self.codegen_stage.process(typed);
                codegen_tx.send(object).await?;
            }
        });

        // Collect results
        let mut results = Vec::new();
        while let Some(object) = codegen_rx.recv().await {
            results.push(object);
        }

        Ok(results)
    }
}
```

### 4.4 Parallel Codegen Configuration

```rust
/// Codegen parallelism settings
struct CodegenConfig {
    /// Number of parallel codegen units
    /// Higher = more parallelism, less inlining opportunity
    /// Default: min(16, num_cpus)
    codegen_units: usize,

    /// Backend selection affects parallelism
    backend: Backend,

    /// LLVM thread count per CGU
    llvm_threads: Option<usize>,

    /// Enable Thin LTO for parallel link-time optimization
    lto: LtoConfig,
}

enum LtoConfig {
    /// No LTO
    Off,

    /// Thin LTO (parallel, good optimization)
    Thin {
        /// Number of parallel ThinLTO jobs
        jobs: usize,
    },

    /// Fat LTO (sequential, best optimization)
    Fat,
}
```

---

## 5. Effect System Impact on Incrementality

### 5.1 Effect Signature Fingerprinting

The effect system introduces additional dependencies that must be tracked for correct incremental compilation:

```rust
/// Effect-related query dependencies
struct EffectDependencies {
    /// Functions whose effects this function depends on
    callee_effects: Vec<FunctionId>,

    /// Effect handlers in scope that may transform effects
    handlers_in_scope: Vec<HandlerId>,

    /// Fingerprint of effect row for change detection
    effect_fingerprint: EffectFingerprint,
}

/// Fingerprint for effect row comparison
#[derive(Hash, Eq, PartialEq)]
struct EffectFingerprint {
    /// Sorted list of concrete effects
    effects: Vec<EffectId>,

    /// Whether row is open (has row variable)
    is_open: bool,

    /// Bounds on row variable (if open)
    bounds: Option<EffectBounds>,
}
```

### 5.2 Effect Signature Changes and Cascading Invalidation

```
EFFECT CHANGE PROPAGATION

Scenario: Function `foo` gains new effect `Console`

Before: fn foo() -> Int !IO
After:  fn foo() -> Int !{IO, Console}

INVALIDATION CASCADE:

1. parse(foo.aria) -> RECOMPUTED (source changed)

2. item_tree(foo.aria) -> RECOMPUTED, CHANGED
   (effect annotation is part of signature)

3. fn_signature(foo) -> RECOMPUTED, CHANGED
   (new effect in signature)

4. infer_effects(foo) -> RECOMPUTED, CHANGED
   (effect set expanded)

5. For each caller of foo:
   a. infer_body(caller) -> RECOMPUTED
      (may need to handle new effect)

   b. infer_effects(caller) -> RECOMPUTED
      (effect propagates up call chain)

   c. check_contracts(caller) -> RECOMPUTED
      (contracts may reference effect state)

6. Cascades transitively to all transitive callers

MITIGATION STRATEGIES:

1. Effect stability for internal functions:
   - Inferred effects don't affect callers until signature exported

2. Handler scope boundaries:
   - Handler blocks limit effect propagation
   - Changes inside handler don't cascade past it

3. Effect row coarsening for compilation:
   - Use effect categories (IO, Console) not specific ops
   - Finer-grained effects only for static analysis
```

### 5.3 Contract Changes and Verification Recomputation

```rust
/// Contract-related dependency tracking
struct ContractDependencies {
    /// Functions whose contracts this function's contracts depend on
    /// (for composed contracts or contract inheritance)
    contract_deps: Vec<FunctionId>,

    /// Type information needed for contract verification
    type_deps: Vec<TypeId>,

    /// SMT solver state that may be cached
    solver_cache_key: SolverCacheKey,
}

/// Incremental contract verification
struct IncrementalContractChecker {
    /// Cache of SMT solver states
    solver_cache: HashMap<SolverCacheKey, SolverState>,

    /// Dependency graph for contracts
    contract_deps: DependencyGraph,
}

impl IncrementalContractChecker {
    /// Check contracts incrementally
    fn check_contracts(&mut self, func: FunctionId) -> ContractResult {
        let cache_key = self.compute_cache_key(func);

        // Check if we can reuse previous verification
        if let Some(cached) = self.solver_cache.get(&cache_key) {
            if self.is_still_valid(cached, func) {
                return cached.result.clone();
            }
        }

        // Re-verify with incremental SMT solving
        let result = self.verify_contracts(func);
        self.solver_cache.insert(cache_key, result.clone());

        result
    }

    /// Invalidation when contract changes
    fn invalidate_contract(&mut self, func: FunctionId) {
        // Remove from solver cache
        self.solver_cache.remove(&self.compute_cache_key(func));

        // Invalidate dependent contracts
        for dep in self.contract_deps.dependents(&func) {
            self.invalidate_contract(dep);
        }
    }
}
```

### 5.4 Effect-Aware Compilation Strategy

```
EFFECT-AWARE INCREMENTAL COMPILATION

QUERY GRAPH WITH EFFECTS:

            file_text(f)
                 │
                 ▼
             parse(f)
                 │
        ┌────────┴────────┐
        │                 │
        ▼                 ▼
   item_tree(f)      fn_body(f)
        │                 │
        ├─────────────────┤
        │                 │
        ▼                 ▼
  fn_signature(fn)  infer_body(fn)
        │                 │
        │    ┌────────────┤
        │    │            │
        ▼    ▼            ▼
   infer_effects(fn)  check_contracts(fn)
        │                 │
        │                 │
        └────────┬────────┘
                 │
                 ▼
    codegen_mir(fn)  [includes effect evidence slots]
                 │
                 ▼
    codegen_object(cgu)

EFFECT STABILITY OPTIMIZATION:

For internal (non-exported) functions:
  - Infer effects but don't include in signature hash
  - Only signature changes trigger caller invalidation
  - Effect changes handled at codegen time

For exported (pub) functions:
  - Effect signature is part of public API
  - Changes invalidate all external dependents
  - May trigger semver warning if effects removed
```

---

## 6. Warm vs Cold Build Optimization

### 6.1 Build Profiles

```toml
# aria.toml build profiles

[profile.dev]
# Fast iteration, minimal optimization
opt-level = 0
debug = true
incremental = true
codegen-units = 256       # Maximum parallelism
backend = "cranelift"     # Faster compilation
contracts = "runtime"     # Runtime contract checks
effects = "inferred"      # Full effect inference

[profile.release]
# Production-ready, maximum optimization
opt-level = 3
debug = false
incremental = false       # Clean builds for reproducibility
codegen-units = 16
backend = "llvm"          # Better optimization
lto = "thin"
contracts = "static"      # Compile-time verification
effects = "checked"       # Effect type checking

[profile.check]
# Type checking only, no codegen
typecheck-only = true
codegen = false
contracts = "check"       # Check but don't prove

[profile.bench]
# Benchmarking profile
inherits = "release"
debug = "line-tables"     # Debug info for profiling
```

### 6.2 Cold Build Optimization

```rust
/// Cold build optimization strategies
struct ColdBuildOptimizer {
    /// Pre-compile standard library (ship pre-built)
    precompiled_stdlib: Option<PathBuf>,

    /// Fetch pre-built dependencies from cache
    dependency_cache: DependencyCacheConfig,

    /// Maximum parallelism for cold builds
    max_threads: usize,

    /// Use faster backend for initial build
    initial_backend: Backend,
}

impl ColdBuildOptimizer {
    fn optimize_cold_build(&self, project: &Project) -> BuildPlan {
        let mut plan = BuildPlan::new();

        // 1. Use pre-compiled stdlib
        if let Some(stdlib) = &self.precompiled_stdlib {
            plan.add_precompiled(stdlib);
        }

        // 2. Fetch cached dependencies in parallel
        let deps = project.dependencies();
        let cache_hits = self.dependency_cache.fetch_batch(&deps);

        for dep in deps {
            if let Some(cached) = cache_hits.get(&dep) {
                plan.add_cached_dependency(dep, cached);
            } else {
                plan.add_build_dependency(dep);
            }
        }

        // 3. Build project with maximum parallelism
        plan.set_parallelism(self.max_threads);
        plan.set_backend(self.initial_backend);

        plan
    }
}
```

### 6.3 Warm Build Optimization

```rust
/// Warm build optimization (incremental)
struct WarmBuildOptimizer {
    /// Query database with cached results
    db: AriaDatabase,

    /// Dependency graph for minimal recomputation
    deps: DependencyGraph,

    /// Changed file detection
    file_watcher: FileWatcher,
}

impl WarmBuildOptimizer {
    /// Minimal rebuild after file change
    fn incremental_build(&mut self, changed: &[FileId]) -> BuildResult {
        // 1. Identify invalidated queries
        let invalidated = self.compute_invalidation_set(changed);

        // 2. Check for early cutoff opportunities
        let actually_changed = self.filter_early_cutoff(&invalidated);

        // 3. Recompute only what's needed
        let mut results = Vec::new();
        for query in actually_changed.in_dependency_order() {
            let result = self.db.execute(query);
            results.push(result);

            // Check for early cutoff after each query
            if result.value_unchanged() {
                self.prune_dependents(&query, &mut actually_changed);
            }
        }

        BuildResult { recomputed: results }
    }

    fn compute_invalidation_set(&self, changed: &[FileId]) -> HashSet<QueryKey> {
        let mut invalidated = HashSet::new();

        for file in changed {
            // Start with input query
            invalidated.insert(QueryKey::FileText(*file));

            // Propagate through dependency graph
            let mut worklist: VecDeque<_> = vec![QueryKey::Parse(*file)].into();

            while let Some(query) = worklist.pop_front() {
                if invalidated.insert(query.clone()) {
                    for dep in self.deps.dependents(&query) {
                        worklist.push_back(dep);
                    }
                }
            }
        }

        invalidated
    }
}
```

### 6.4 Build Fingerprinting for Cache Hits

```rust
/// Complete build fingerprint for cache matching
struct BuildFingerprint {
    /// Project source fingerprint
    source: SourceFingerprint,

    /// Compiler configuration
    compiler: CompilerFingerprint,

    /// Dependency versions and hashes
    dependencies: Vec<(CrateName, Version, ArtifactHash)>,

    /// Environment that affects build
    environment: EnvFingerprint,
}

struct SourceFingerprint {
    /// Hash of all source files
    source_hash: ContentHash,

    /// Timestamp of newest file (for quick staleness check)
    newest_mtime: SystemTime,

    /// Number of source files (sanity check)
    file_count: usize,
}

struct CompilerFingerprint {
    /// Compiler version
    version: Version,

    /// Git commit if development build
    commit: Option<String>,

    /// Feature flags enabled
    features: Vec<String>,

    /// Target triple
    target: String,

    /// Profile settings hash
    profile_hash: Hash,
}

impl BuildFingerprint {
    /// Check if cached build is valid
    fn matches_cache(&self, cached: &CachedBuild) -> bool {
        self.source.source_hash == cached.fingerprint.source.source_hash
            && self.compiler == cached.fingerprint.compiler
            && self.dependencies == cached.fingerprint.dependencies
    }
}
```

---

## 7. Implementation Roadmap

### Phase 1: Foundation (Weeks 1-4)

| Task | Description | Dependencies |
|------|-------------|--------------|
| Query Infrastructure | Implement Salsa-based query system | None |
| File Change Detection | Integrate with file watcher | Query Infrastructure |
| Basic Caching | On-disk content-addressed storage | Query Infrastructure |
| Dependency Graph | Build and serialize dep graph | Query Infrastructure |

### Phase 2: Incremental Type Checking (Weeks 5-8)

| Task | Description | Dependencies |
|------|-------------|--------------|
| Function-Level Queries | Implement per-function inference | Phase 1 |
| Early Cutoff | Implement signature stability | Function-Level Queries |
| Effect Tracking | Add effect dependency tracking | Function-Level Queries |
| Contract Caching | Incremental contract verification | Function-Level Queries |

### Phase 3: Parallel Compilation (Weeks 9-12)

| Task | Description | Dependencies |
|------|-------------|--------------|
| Work Stealing | Implement parallel scheduler | Phase 2 |
| Pipeline Parallelism | Overlap parsing/typechecking/codegen | Work Stealing |
| CGU Parallelism | Parallel codegen units | Pipeline Parallelism |
| Benchmarking | Measure parallelism efficiency | All above |

### Phase 4: Advanced Caching (Weeks 13-16)

| Task | Description | Dependencies |
|------|-------------|--------------|
| Distributed Cache | Remote cache protocol | Phase 1 |
| Cross-Project Sharing | Shared dependency cache | Distributed Cache |
| Cache Warming | CI integration for warm caches | Distributed Cache |
| Pre-compiled Stdlib | Ship pre-built standard library | Cross-Project Sharing |

### Phase 5: Polish and Optimization (Weeks 17-20)

| Task | Description | Dependencies |
|------|-------------|--------------|
| Profile-Guided | Use build profiles for optimization | All phases |
| Memory Optimization | Reduce peak memory usage | All phases |
| Telemetry | Build time analytics | All phases |
| Documentation | User guide for build optimization | All phases |

---

## 8. Performance Targets and Metrics

### 8.1 Performance Targets

| Metric | Cold Build | Warm Build (1 file) | IDE Response |
|--------|-----------|---------------------|--------------|
| 1K LOC project | <2s | <100ms | <20ms |
| 10K LOC project | <10s | <200ms | <50ms |
| 100K LOC project | <60s | <500ms | <100ms |
| 1M LOC project | <10min | <2s | <200ms |

### 8.2 Benchmark Suite

```rust
/// Build system benchmark suite
struct BuildBenchmarks {
    /// Small project (1K LOC, 10 files)
    small_project: ProjectFixture,

    /// Medium project (10K LOC, 100 files)
    medium_project: ProjectFixture,

    /// Large project (100K LOC, 1000 files)
    large_project: ProjectFixture,

    /// Monorepo (1M LOC, 10K files)
    monorepo: ProjectFixture,
}

impl BuildBenchmarks {
    fn run_all(&self) -> BenchmarkResults {
        let mut results = BenchmarkResults::new();

        for project in [&self.small_project, &self.medium_project, ...] {
            // Cold build
            results.record("cold_build", project, || {
                clean_cache();
                build(project)
            });

            // Warm build (single file change)
            results.record("warm_single_file", project, || {
                modify_single_file(project);
                build(project)
            });

            // Warm build (signature change)
            results.record("warm_signature_change", project, || {
                modify_function_signature(project);
                build(project)
            });

            // Type check only
            results.record("typecheck_only", project, || {
                check(project)
            });

            // IDE diagnostics
            results.record("ide_diagnostics", project, || {
                get_diagnostics_for_file(project)
            });
        }

        results
    }
}
```

### 8.3 Regression Detection

```yaml
# .github/workflows/build-perf.yml
name: Build Performance

on: [push, pull_request]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Run benchmarks
        run: aria bench --output bench-results.json

      - name: Compare with baseline
        run: |
          aria bench compare \
            --baseline main-bench.json \
            --current bench-results.json \
            --threshold 10%  # Fail if >10% regression
```

---

## 9. Key Resources and References

### Academic Papers

1. Erdweg et al. (2011) - "A Framework for Defining Incremental Languages"
2. Acar et al. (2008) - "An Experimental Analysis of Self-Adjusting Computation"
3. Hammer et al. (2014) - "Adapton: Composable, Demand-Driven Incremental Computation"

### Implementation References

1. [Salsa Framework](https://github.com/salsa-rs/salsa) - Rust's incremental computation
2. [rust-analyzer Architecture](https://rust-analyzer.github.io/book/contributing/architecture.html)
3. [Rust Compiler Dev Guide - Queries](https://rustc-dev-guide.rust-lang.org/queries/incremental-compilation.html)
4. [sccache](https://github.com/mozilla/sccache) - Shared compilation cache
5. [Bazel Remote Caching](https://bazel.build/remote/caching)

### Aria-Specific References

- ARIA-PD-012: LSP and IDE Integration Design
- ARIA-M18-03: Incremental Compilation for LSP
- ARIA-M07-01: LLVM vs Cranelift Comparison
- ARIA-PD-005: Effect System Design

---

## 10. Open Questions

1. **Salsa Version**: Should we use Salsa 0.16 (stable) or invest in Salsa 2.0 (newer, simpler API)?

2. **Distributed Cache Protocol**: Should Aria define its own protocol or adopt existing solutions (Bazel, sccache)?

3. **Effect Caching**: How granular should effect tracking be for caching purposes?

4. **Contract Verification Caching**: Can we cache SMT solver states effectively across incremental builds?

5. **LLM Optimization Integration**: How do LLM-suggested optimizations interact with incremental compilation?

6. **WASM Target Caching**: Does WASM target require different caching strategies due to different compilation model?

---

## Appendix A: Cache Key Specification

```rust
/// Complete cache key specification for all artifact types
mod cache_keys {
    /// Source file cache key
    pub struct SourceCacheKey {
        pub path: PathBuf,
        pub content_hash: Blake3Hash,
        pub encoding: Encoding,
    }

    /// Parsed AST cache key
    pub struct ParseCacheKey {
        pub source: SourceCacheKey,
        pub parser_version: u32,
        pub syntax_edition: Edition,
    }

    /// Type inference cache key
    pub struct InferenceCacheKey {
        pub function: FunctionId,
        pub signature_hash: Blake3Hash,
        pub body_hash: Blake3Hash,
        pub dependency_hashes: Vec<(FunctionId, Blake3Hash)>,
        pub type_system_version: u32,
    }

    /// Effect inference cache key
    pub struct EffectCacheKey {
        pub function: FunctionId,
        pub body_hash: Blake3Hash,
        pub callee_effect_hashes: Vec<(FunctionId, Blake3Hash)>,
        pub handler_scopes: Vec<HandlerScopeId>,
        pub effect_system_version: u32,
    }

    /// Codegen cache key
    pub struct CodegenCacheKey {
        pub mir_hash: Blake3Hash,
        pub target: Target,
        pub opt_level: OptLevel,
        pub backend: Backend,
        pub backend_version: Version,
        pub codegen_flags: CodegenFlags,
    }
}
```

---

## Appendix B: Dependency Graph Serialization

```rust
/// Efficient serialization of dependency graph for on-disk storage
mod dep_graph_format {
    use bincode::{Decode, Encode};

    #[derive(Encode, Decode)]
    pub struct SerializedDepGraph {
        /// Version for format compatibility
        pub format_version: u32,

        /// Query key interning table
        pub query_keys: Vec<SerializedQueryKey>,

        /// Adjacency list: query index -> dependent indices
        pub edges: Vec<Vec<u32>>,

        /// Version numbers for each query
        pub versions: Vec<u64>,

        /// Durability classification
        pub durabilities: Vec<Durability>,
    }

    impl SerializedDepGraph {
        pub fn serialize(&self) -> Vec<u8> {
            let config = bincode::config::standard();
            bincode::encode_to_vec(self, config).unwrap()
        }

        pub fn deserialize(bytes: &[u8]) -> Result<Self, Error> {
            let config = bincode::config::standard();
            bincode::decode_from_slice(bytes, config)
                .map(|(graph, _)| graph)
        }
    }
}
```

---

*Document generated by DELTA research agent*
*Eureka Vault - Aria Language Design Research*
*Last updated: 2026-01-15*
