# ARIA-PD-017: Build System and Incremental Compilation Design

**Decision ID**: ARIA-PD-017
**Status**: Approved
**Date**: 2026-01-15
**Author**: ACCELERATOR (Product Decision Agent)
**Research Inputs**:
- ARIA-M07-03: Incremental Compilation and Build System Design (DELTA)
- ARIA-PD-012: LSP and IDE Integration Design
- ARIA-M07-01: LLVM vs Cranelift Comparison

---

## Executive Summary

This document defines Aria's build system architecture, making concrete decisions on compilation granularity, caching strategies, dependency tracking, and parallel compilation. The design ensures fast incremental builds for developer productivity while supporting reproducible release builds for production.

**Final Decisions**:
1. **Compilation Unit Granularity**: Function-level for semantic analysis, module-level for codegen artifacts
2. **Cache Storage**: Three-layer architecture (memory LRU, disk content-addressed, optional distributed)
3. **Dependency Graph**: Salsa-style query-based with fine-grained tracking and serialized persistence
4. **Invalidation Strategy**: Hash-based with version vectors and early cutoff optimization
5. **Parallel Compilation**: Work-stealing scheduler with pipeline parallelism, default 16 CGUs
6. **Distributed Cache**: Optional cloud storage integration with sccache-compatible protocol
7. **Build Profiles**: Four profiles (dev, release, check, test) with distinct optimization strategies

---

## 1. Compilation Unit Granularity

### 1.1 Decision: Function-Level with Module Grouping

**Decision**: Aria adopts **function-level granularity** for semantic analysis and **module-level grouping** for code generation artifacts.

**Rationale**:
| Granularity | Cache Efficiency | Invalidation Precision | Complexity | Decision |
|-------------|------------------|----------------------|------------|----------|
| File | Low | Coarse | Low | **Rejected** |
| Module | Medium | Medium | Medium | **Partial** - for codegen |
| Function | High | Fine | High | **Adopted** - for analysis |
| Expression | Very High | Very Fine | Very High | **Rejected** |

### 1.2 Cache Unit Organization

```
COMPILATION UNIT HIERARCHY

Project (aria.toml boundary)
  |
  +-- Crate (library or binary)
       |
       +-- Module (file or mod block)
            |
            +-- Item (fn, struct, enum, trait, impl)
                 |
                 +-- Function Body (primary cache unit for type inference)

CACHE GRANULARITY BY PHASE

Phase               | Cache Unit     | Invalidation Trigger
--------------------|----------------|----------------------------------
Parsing             | File           | Source text change
Item Tree           | File           | Structural syntax change
Name Resolution     | Module         | Import/export change
Signature Analysis  | Function       | Declaration change
Body Inference      | Function       | Body or dependency change
Effect Inference    | Function       | Body or callee effect change
Contract Checking   | Function       | Contract or dependency change
MIR Lowering        | Function       | Any function change
Codegen             | CGU (group)    | Any contained function change
```

### 1.3 Signature Stability Principle

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

### 1.4 Codegen Unit (CGU) Strategy

**Decision**: Use adaptive CGU assignment based on build profile.

| Profile | CGU Strategy | CGU Count | Rationale |
|---------|--------------|-----------|-----------|
| dev | PerModule | ~modules | Maximum parallelism, fast iteration |
| release | Fixed | 16 | Balance parallelism and LTO opportunity |
| check | N/A | 0 | No codegen performed |
| test | PerModule | ~modules | Fast test compilation |

```rust
/// Codegen Unit assignment strategy
enum CGUStrategy {
    /// One CGU per module (default for dev/test builds)
    PerModule,

    /// Fixed number of CGUs (release builds)
    /// Enables parallel codegen with LLVM/Cranelift
    Fixed { count: usize },  // Default: 16

    /// Single CGU (maximum optimization, no parallelism)
    Single,
}
```

---

## 2. Cache Storage Format and Location

### 2.1 Decision: Three-Layer Cache Architecture

**Decision**: Implement a unified three-layer cache with automatic fallback.

```
CACHE LAYER ARCHITECTURE

+------------------------------------------------------------------+
|  L1: In-Memory LRU Cache                                          |
|  - Size: 512MB default (configurable)                             |
|  - Latency: <1ms                                                  |
|  - Scope: Current session only                                    |
|  - Contents: Hot query results, parsed ASTs, type info            |
+------------------------------------------------------------------+
                              |
                              v
+------------------------------------------------------------------+
|  L2: On-Disk Content-Addressed Cache                              |
|  - Size: 10GB default (configurable)                              |
|  - Latency: <10ms                                                 |
|  - Scope: Per-user, cross-project sharing                         |
|  - Contents: All cacheable artifacts                              |
+------------------------------------------------------------------+
                              |
                              v
+------------------------------------------------------------------+
|  L3: Distributed Cache (Optional)                                 |
|  - Size: Unlimited (cloud storage)                                |
|  - Latency: 50-500ms (network dependent)                          |
|  - Scope: Team/organization sharing                               |
|  - Contents: Pre-compiled dependencies, CI artifacts              |
+------------------------------------------------------------------+
```

### 2.2 Cache Directory Layout

**Decision**: Use XDG-compliant directory structure with content-addressed storage.

```
CACHE DIRECTORY STRUCTURE

$ARIA_CACHE_DIR (default: $XDG_CACHE_HOME/aria or ~/.cache/aria)
|
+-- index.db                    # SQLite index for fast lookup
|                               # Schema: artifact_id, path, size, created,
|                               #         accessed, metadata_json, ttl
|
+-- content/                    # Content-addressed blob storage (Blake3)
|   +-- ab/                     # First two hex chars of hash (256 buckets)
|   |   +-- ab3def...0123      # Full 64-char hash as filename
|   |   +-- ab8912...4567      # Compressed with zstd level 3
|   +-- cd/
|       +-- cd89ab...cdef
|
+-- metadata/                   # Per-project metadata
|   +-- <project_hash>/         # Blake3 hash of project root + aria.toml
|       +-- dep_graph.bin       # Serialized dependency graph (bincode)
|       +-- query_cache.db      # Query results database
|       +-- fingerprint.json    # Build fingerprint for validation
|
+-- codegen/                    # Compiled artifacts by target-profile
|   +-- x86_64-linux-gnu/
|   |   +-- dev/
|   |   |   +-- <crate_hash>/
|   |   |       +-- lib<name>.rlib
|   |   |       +-- <cgu_hash>.o
|   |   +-- release/
|   |       +-- <crate_hash>/
|   |           +-- lib<name>.rlib
|   |           +-- <cgu_hash>.o
|   +-- wasm32/
|       +-- release/
|
+-- stdlib/                     # Pre-compiled standard library
|   +-- <version>-<target>/
|       +-- libstd.rlib
|       +-- libcore.rlib
|
+-- deps/                       # Cached dependency artifacts
    +-- <dep_name>-<version>-<hash>/
        +-- lib<name>.rlib
```

### 2.3 Content-Addressed Storage Format

**Decision**: Use Blake3 hashing with zstd compression.

```rust
/// Content-addressed artifact storage
struct ArtifactStore {
    root: PathBuf,              // $ARIA_CACHE_DIR/content
    index: SqliteConnection,    // $ARIA_CACHE_DIR/index.db
    compression: CompressionLevel,  // zstd level 3 (fast)
}

/// Artifact identifier (Blake3 hash)
struct ArtifactId([u8; 32]);  // 256-bit hash

impl ArtifactId {
    fn to_path(&self, root: &Path) -> PathBuf {
        let hex = self.to_hex();
        root.join(&hex[0..2]).join(&hex)
    }
}

/// Artifact metadata stored in index
struct ArtifactMetadata {
    kind: ArtifactKind,         // parsed_ast, typed_hir, mir, object, etc.
    source_hash: ContentHash,   // Hash of source input
    compiler_version: Version,  // Aria compiler version
    created: SystemTime,        // Creation timestamp
    accessed: SystemTime,       // Last access timestamp
    size_bytes: u64,            // Uncompressed size
    compressed_size: u64,       // On-disk size
}
```

### 2.4 Cache Eviction Policy

**Decision**: LRU eviction with configurable size and age limits.

| Parameter | Default | Environment Variable |
|-----------|---------|---------------------|
| Max cache size | 10 GB | `ARIA_CACHE_MAX_SIZE` |
| Max artifact age | 30 days | `ARIA_CACHE_MAX_AGE_DAYS` |
| Min free space | 10% | `ARIA_CACHE_MIN_FREE_PERCENT` |
| L1 memory limit | 512 MB | `ARIA_CACHE_MEMORY_MB` |

```
EVICTION PRIORITY (lowest priority evicted first)

1. Artifacts older than max_age_days
2. Artifacts not accessed in 7+ days
3. Artifacts from uninstalled compiler versions
4. LRU by access time

PROTECTED (never evicted automatically):
- Standard library artifacts
- Actively used project artifacts
- Artifacts accessed in last hour
```

---

## 3. Dependency Graph Representation

### 3.1 Decision: Salsa-Style Query Graph

**Decision**: Implement fine-grained dependency tracking using a query-based architecture.

```
QUERY DEPENDENCY GRAPH STRUCTURE

+------------------+
|  QueryKey        |  Unique identifier for each cached computation
+------------------+
| - kind: QueryKind
| - payload: Bytes (interned data)
+------------------+

QueryKind enumeration:
  INPUT QUERIES (external data)
    FileText(FileId)            # Source file content
    Config                      # aria.toml configuration
    StdlibVersion               # Standard library version

  DERIVED QUERIES (computed)
    Parse(FileId)               # Syntax tree
    ItemTree(FileId)            # Item declarations (signatures only)
    ModuleScope(ModuleId)       # Name resolution scope
    CrateDefMap(CrateId)        # Global name resolution
    FnSignature(FunctionId)     # Function type signature
    InferBody(FunctionId)       # Type inference result
    InferEffects(FunctionId)    # Effect inference result
    CheckContracts(FunctionId)  # Contract verification result
    LowerToMir(FunctionId)      # MIR representation
    MonomorphizedFn(FnId, Args) # Monomorphized function
    CodegenUnit(CGUId)          # Compiled object code
```

### 3.2 Dependency Graph Storage

**Decision**: Use bincode serialization with memory-mapped access.

```rust
/// Serialized dependency graph format
#[derive(Serialize, Deserialize)]
struct SerializedDepGraph {
    /// Format version for compatibility
    format_version: u32,  // Current: 1

    /// Query key interning table
    query_keys: Vec<SerializedQueryKey>,

    /// Adjacency list: query index -> dependent indices
    /// Stored as flattened array with offsets
    edge_offsets: Vec<u32>,
    edges: Vec<u32>,

    /// Version numbers for each query (for validation)
    versions: Vec<u64>,

    /// Durability classification per query
    durabilities: Vec<Durability>,

    /// Cached hash values for early cutoff
    value_hashes: Vec<Option<ContentHash>>,
}

impl SerializedDepGraph {
    /// Serialize to disk (bincode format)
    fn save(&self, path: &Path) -> Result<()> {
        let config = bincode::config::standard()
            .with_variable_int_encoding()
            .with_little_endian();
        let bytes = bincode::encode_to_vec(self, config)?;

        // Atomic write with temp file
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, &bytes)?;
        fs::rename(&tmp, path)?;
        Ok(())
    }

    /// Load from disk with memory mapping
    fn load(path: &Path) -> Result<Self> {
        let mmap = unsafe { Mmap::map(&File::open(path)?)? };
        let config = bincode::config::standard()
            .with_variable_int_encoding()
            .with_little_endian();
        let (graph, _): (Self, _) = bincode::decode_from_slice(&mmap, config)?;
        Ok(graph)
    }
}
```

### 3.3 Runtime Dependency Tracking

**Decision**: Track dependencies during query execution, persist on session end.

```
DEPENDENCY TRACKING FLOW

1. Query execution starts
   -> Push query onto dependency stack

2. Query reads another query result
   -> Record edge: current_query -> read_query

3. Query completes
   -> Pop from stack
   -> Hash result value
   -> Store: query -> (result, hash, dependencies, version)

4. Session ends
   -> Serialize dependency graph to disk
   -> Prune stale entries

5. Next session starts
   -> Load dependency graph from disk
   -> Validate against file system state
   -> Mark stale queries for recomputation
```

---

## 4. Invalidation Strategy

### 4.1 Decision: Hash-Based with Version Vectors

**Decision**: Combine content hashing with version vectors for efficient invalidation.

### 4.2 Content Hash Algorithm

**Decision**: Use Blake3 with semantic normalization.

```rust
/// Compute semantic hash (ignores whitespace, comments)
fn compute_semantic_hash(source: &str) -> ContentHash {
    let mut hasher = Blake3::new();

    for token in lex_semantic(source) {
        // Skip: whitespace, comments, doc comments (for body hash)
        // Include: all semantic tokens
        hasher.update(&[token.kind as u8]);
        hasher.update(token.text.as_bytes());
    }

    ContentHash(hasher.finalize().into())
}

/// Cache key incorporating all build inputs
#[derive(Hash, PartialEq, Eq)]
struct CacheKey {
    content_hash: ContentHash,       // Source content
    compiler_version: Version,       // Aria version
    target: Target,                  // e.g., x86_64-linux-gnu
    profile: Profile,                // dev, release, check, test
    config_hash: ContentHash,        // Relevant aria.toml settings
    dependency_hashes: Vec<(CrateId, ContentHash)>,  // Deps
}
```

### 4.3 Version Vector System

**Decision**: Three-tier durability with version vectors.

| Durability | Change Frequency | Examples | Version Increment |
|------------|------------------|----------|-------------------|
| HIGH | Rare (weekly+) | stdlib, external deps | high_rev |
| MEDIUM | Occasional (hourly) | aria.toml, project config | medium_rev |
| LOW | Continuous (seconds) | User source code | low_rev |

```rust
/// Version vector for efficient validation
#[derive(Clone, Copy, PartialEq, Eq)]
struct VersionVector {
    high: u64,    // Incremented on HIGH durability changes
    medium: u64,  // Incremented on MEDIUM durability changes
    low: u64,     // Incremented on LOW durability changes
}

impl VersionVector {
    /// Fast check if query needs revalidation
    fn needs_revalidation(&self, query_durability: Durability, current: &VersionVector) -> bool {
        match query_durability {
            Durability::HIGH => self.high != current.high,
            Durability::MEDIUM => self.high != current.high || self.medium != current.medium,
            Durability::LOW => self != current,
        }
    }
}
```

### 4.4 Early Cutoff Optimization

**Decision**: Implement early cutoff at all derived query boundaries.

```
EARLY CUTOFF ALGORITHM

procedure recompute_if_needed(query):
    if not is_stale(query):
        return cached_result(query)

    # Recompute the query
    new_result = execute_query(query)
    new_hash = hash(new_result)
    old_hash = cached_hash(query)

    if new_hash == old_hash:
        # EARLY CUTOFF: Value unchanged, don't propagate
        mark_valid(query)
        return cached_result(query)
    else:
        # Value changed, invalidate dependents
        store_result(query, new_result, new_hash)
        for dependent in dependents(query):
            mark_stale(dependent)
        return new_result
```

### 4.5 Invalidation Propagation

**Decision**: Lazy invalidation with on-demand recomputation.

```
INVALIDATION FLOW (On File Save)

1. Detect changed files (mtime or content hash)

2. Mark input queries stale:
   FileText(changed_file) -> STALE

3. Propagate staleness lazily:
   - Don't immediately recompute anything
   - Mark direct dependents as "potentially stale"

4. On next query access:
   - Check if inputs are stale
   - Recompute if needed
   - Apply early cutoff

BENEFIT: Only recompute what's actually needed
         (IDE may only need hover, not full build)
```

---

## 5. Parallel Compilation Limits and Strategies

### 5.1 Decision: Work-Stealing with Pipeline Parallelism

**Decision**: Combine work-stealing for same-phase parallelism with pipelining across phases.

### 5.2 Parallelism Configuration

| Setting | Default | Environment Variable | aria.toml Key |
|---------|---------|---------------------|---------------|
| Worker threads | num_cpus | `ARIA_BUILD_JOBS` | `build.jobs` |
| Max CGUs | 16 | `ARIA_CODEGEN_UNITS` | `profile.*.codegen-units` |
| LLVM threads | 1 per CGU | `ARIA_LLVM_THREADS` | `build.llvm-threads` |
| Memory limit | 80% available | `ARIA_MAX_MEMORY` | `build.max-memory` |

### 5.3 Phase-Level Parallelism

```
COMPILATION PIPELINE PARALLELISM

Phase 1: Parsing (File-Level Parallel)
  +-----+-----+-----+-----+-----+-----+-----+-----+
  | f1  | f2  | f3  | f4  | f5  | f6  | f7  | f8  |  <- All files parallel
  +-----+-----+-----+-----+-----+-----+-----+-----+
                           |
                           v
Phase 2: Name Resolution (Sequential then Parallel)
  +---------------------------+
  | Build CrateDefMap         |  <- Must be sequential
  +---------------------------+
                           |
                           v
  +-----+-----+-----+-----+-----+-----+-----+-----+
  | m1  | m2  | m3  | m4  | m5  | m6  | m7  | m8  |  <- Module-level parallel
  +-----+-----+-----+-----+-----+-----+-----+-----+
                           |
                           v
Phase 3: Type Inference (Function-Level Parallel)
  +---+---+---+---+---+---+---+---+---+---+---+---+
  |fn1|fn2|fn3|fn4|fn5|fn6|fn7|fn8|fn9|f10|f11|f12|
  +---+---+---+---+---+---+---+---+---+---+---+---+
                           |
                           v
Phase 4: Effect/Contract Analysis (Function-Level Parallel)
  +---+---+---+---+---+---+---+---+---+---+---+---+
  |ef1|ef2|ef3|ct1|ct2|ct3|ef4|ct4|ef5|ct5|ef6|ct6|
  +---+---+---+---+---+---+---+---+---+---+---+---+
                           |
                           v
Phase 5: MIR Lowering (Function-Level Parallel)
  +---+---+---+---+---+---+---+---+---+---+---+---+
  |mr1|mr2|mr3|mr4|mr5|mr6|mr7|mr8|mr9|m10|m11|m12|
  +---+---+---+---+---+---+---+---+---+---+---+---+
                           |
                           v
Phase 6: Codegen (CGU-Level Parallel, default 16)
  +---------+---------+---------+---------+
  |  CGU1   |  CGU2   |  CGU3   |  CGU4   |
  +---------+---------+---------+---------+
  |  CGU5   |  CGU6   |  CGU7   |  CGU8   |
  +---------+---------+---------+---------+
  |  CGU9   |  CGU10  |  CGU11  |  CGU12  |
  +---------+---------+---------+---------+
  |  CGU13  |  CGU14  |  CGU15  |  CGU16  |
  +---------+---------+---------+---------+
                           |
                           v
Phase 7: Linking (Sequential)
  +---------------------------------------+
  |         Link all object files          |
  +---------------------------------------+
```

### 5.4 Work-Stealing Scheduler

**Decision**: Use Rayon-style work-stealing with query-aware scheduling.

```rust
/// Parallel query scheduler
struct QueryScheduler {
    /// Rayon thread pool
    pool: ThreadPool,

    /// Query database
    db: Arc<Database>,

    /// Pending work items
    pending: DashMap<QueryKey, QueryState>,

    /// Dependency ordering
    order: TopologicalOrder,
}

impl QueryScheduler {
    fn execute_parallel(&self, queries: Vec<QueryKey>) -> Vec<QueryResult> {
        // Group queries into waves (no intra-wave dependencies)
        let waves = self.order.compute_waves(&queries);

        let mut results = Vec::new();
        for wave in waves {
            // Execute wave in parallel
            let wave_results: Vec<_> = self.pool.install(|| {
                wave.par_iter()
                    .map(|q| self.db.execute(q))
                    .collect()
            });
            results.extend(wave_results);
        }

        results
    }
}
```

### 5.5 Pipeline Parallelism

**Decision**: Start later phases before earlier phases complete.

```
PIPELINE OVERLAP

Time -->

Thread 1: [Parse f1][Parse f2][Parse f3][Parse f4]...
Thread 2:          [Parse f5][Parse f6][Parse f7][Parse f8]...
Thread 3:                    [Resolve m1][Resolve m2]...
Thread 4:                              [Infer fn1][Infer fn2]...
Thread 5:                                        [Codegen cgu1]...

CONSTRAINT: Must respect dependencies
  - Can't infer fn1 until fn1's module is resolved
  - Can start inferring fn1 while still parsing other files
```

### 5.6 Memory Management

**Decision**: Bound memory usage with backpressure.

```
MEMORY BACKPRESSURE STRATEGY

1. Track memory usage per phase:
   - Parsing: ~10MB per file (peak)
   - Type inference: ~50MB per 1000 functions
   - Codegen: ~200MB per CGU (LLVM)

2. Apply backpressure when approaching limit:
   - Reduce parallelism (fewer concurrent tasks)
   - Force GC of completed artifacts
   - Serialize intermediate results to disk

3. Emergency measures at 95% limit:
   - Pause new work
   - Evict LRU cache entries
   - Single-threaded completion

MEMORY TARGETS

Project Size | Peak Memory | Strategy
-------------|-------------|---------------------------
1K LOC       | <500MB      | Full parallelism
10K LOC      | <2GB        | Full parallelism
100K LOC     | <4GB        | Moderate backpressure
1M LOC       | <8GB        | Aggressive backpressure
```

---

## 6. Distributed Cache Support

### 6.1 Decision: Optional Cloud Storage Integration

**Decision**: Support distributed caching as opt-in feature with sccache-compatible protocol.

### 6.2 Distributed Cache Protocol

```rust
/// Distributed cache client interface
#[async_trait]
trait DistributedCache: Send + Sync {
    /// Check if artifact exists in remote cache
    async fn contains(&self, key: &CacheKey) -> Result<bool>;

    /// Fetch artifact from remote cache
    async fn get(&self, key: &CacheKey) -> Result<Option<Vec<u8>>>;

    /// Store artifact in remote cache
    async fn put(&self, key: &CacheKey, artifact: &[u8]) -> Result<()>;

    /// Batch fetch for efficiency
    async fn get_batch(&self, keys: &[CacheKey]) -> Result<HashMap<CacheKey, Vec<u8>>>;
}
```

### 6.3 Supported Backends

| Backend | Configuration | Use Case |
|---------|---------------|----------|
| S3 | `ARIA_CACHE_S3_BUCKET` | AWS/S3-compatible storage |
| GCS | `ARIA_CACHE_GCS_BUCKET` | Google Cloud Storage |
| Azure Blob | `ARIA_CACHE_AZURE_CONTAINER` | Azure Storage |
| Redis | `ARIA_CACHE_REDIS_URL` | Low-latency team cache |
| HTTP | `ARIA_CACHE_HTTP_URL` | Custom cache server |

### 6.4 Cache Configuration

```toml
# aria.toml distributed cache configuration

[cache]
# Enable distributed cache
distributed = true

# Backend selection (s3, gcs, azure, redis, http)
backend = "s3"

# S3-specific configuration
[cache.s3]
bucket = "aria-cache-team"
region = "us-west-2"
prefix = "v1/"

# Authentication (defaults to environment/IAM)
# Explicitly set if needed:
# access_key_id = "..."
# secret_access_key = "..."

# Cache behavior
[cache.behavior]
# Upload local artifacts to remote
upload = true

# Download from remote before local compile
download = true

# Maximum artifact size to cache remotely (MB)
max_artifact_size = 50

# Retry configuration
max_retries = 3
timeout_seconds = 30
```

### 6.5 CI/CD Integration

**Decision**: Provide first-class CI integration for cache warming.

```yaml
# Example GitHub Actions integration

name: Build with Cache

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Configure Aria Cache
        env:
          ARIA_CACHE_S3_BUCKET: ${{ secrets.CACHE_BUCKET }}
          AWS_ACCESS_KEY_ID: ${{ secrets.AWS_KEY }}
          AWS_SECRET_ACCESS_KEY: ${{ secrets.AWS_SECRET }}
        run: |
          aria cache configure --backend s3

      - name: Build (with remote cache)
        run: |
          aria build --release

      - name: Upload cache artifacts
        if: github.ref == 'refs/heads/main'
        run: |
          aria cache push --tag main
```

### 6.6 Cache Key Generation

**Decision**: Use deterministic, reproducible cache keys.

```
DISTRIBUTED CACHE KEY FORMAT

aria-v1-<target>-<profile>-<source_hash>-<config_hash>-<deps_hash>

Components:
  aria-v1         : Protocol version
  target          : e.g., x86_64-linux-gnu
  profile         : dev, release, check, test
  source_hash     : Blake3 of normalized source
  config_hash     : Blake3 of relevant aria.toml
  deps_hash       : Blake3 of dependency versions + hashes

Example:
  aria-v1-x86_64-linux-gnu-release-a1b2c3d4...-e5f6a7b8...-c9d0e1f2...
```

---

## 7. Build Profiles

### 7.1 Decision: Four Standard Profiles

**Decision**: Define four profiles with distinct optimization strategies.

### 7.2 Profile Definitions

```toml
# Default profiles (built-in, can be overridden)

[profile.dev]
# Fast iteration for development
opt-level = 0                 # No optimization
debug = true                  # Full debug info
debug-assertions = true       # Enable debug assertions
overflow-checks = true        # Check integer overflow
incremental = true            # Enable incremental compilation
codegen-units = 256           # Maximum parallelism
backend = "cranelift"         # Faster compilation
contracts = "runtime"         # Runtime contract checks
effects = "inferred"          # Full effect inference
lto = false                   # No LTO

[profile.release]
# Production-ready binaries
opt-level = 3                 # Maximum optimization
debug = false                 # No debug info
debug-assertions = false      # Disable assertions
overflow-checks = false       # No overflow checks
incremental = false           # Clean builds
codegen-units = 16            # Balance parallel/LTO
backend = "llvm"              # Better optimization
contracts = "verified"        # Compile-time verification
effects = "checked"           # Effect type checking
lto = "thin"                  # Thin LTO

[profile.check]
# Type checking only (no codegen)
typecheck-only = true         # Skip codegen entirely
contracts = "check"           # Check but don't prove
effects = "inferred"          # Full inference

[profile.test]
# Test execution
opt-level = 0                 # No optimization
debug = true                  # Debug info for failures
debug-assertions = true       # Enable assertions
overflow-checks = true        # Check overflow
incremental = true            # Fast recompilation
codegen-units = 256           # Maximum parallelism
backend = "cranelift"         # Faster compilation
contracts = "runtime"         # Runtime checks in tests
test-threads = "auto"         # Parallel test execution
```

### 7.3 Profile Inheritance

**Decision**: Support profile inheritance for customization.

```toml
# Custom profile inheriting from release
[profile.release-with-debug]
inherits = "release"
debug = "line-tables-only"    # Debug info for profiling

# Benchmark profile
[profile.bench]
inherits = "release"
debug = "line-tables-only"
lto = "fat"                   # Maximum optimization

# Size-optimized profile
[profile.release-small]
inherits = "release"
opt-level = "z"               # Optimize for size
lto = "fat"                   # Maximum size reduction
```

### 7.4 Backend Selection

| Backend | Compile Speed | Runtime Speed | Debug Quality | Use Case |
|---------|--------------|---------------|---------------|----------|
| Cranelift | Fast | Moderate | Good | dev, test |
| LLVM | Slow | Best | Best | release, bench |

**Decision**: Default to Cranelift for dev/test, LLVM for release.

### 7.5 LTO Configuration

| LTO Mode | Description | Compile Time | Optimization |
|----------|-------------|--------------|--------------|
| off | No LTO | Fastest | Baseline |
| thin | Parallel LTO | Moderate | Good |
| fat | Full LTO | Slowest | Best |

**Decision**: Default `thin` for release, `off` for dev/test.

---

## 8. Performance Targets

### 8.1 Build Time Targets

| Metric | Target | Strategy |
|--------|--------|----------|
| Incremental (body edit) | <100ms | Function-level invalidation, early cutoff |
| Incremental (signature change) | <1s | Caller graph propagation |
| Incremental (new file) | <2s | Module-level rebuild |
| Clean build (1K LOC) | <2s | Full parallelism |
| Clean build (10K LOC) | <10s | Full parallelism |
| Clean build (100K LOC) | <60s | Backpressure management |
| Check only (10K LOC) | <5s | Skip codegen phase |
| IDE keystroke response | <50ms | Demand-driven analysis |

### 8.2 Cache Performance Targets

| Operation | L1 (Memory) | L2 (Disk) | L3 (Remote) |
|-----------|-------------|-----------|-------------|
| Lookup | <1ms | <10ms | <500ms |
| Store | <1ms | <50ms | <2s |
| Hit rate (warm) | >90% | >95% | >80% |

### 8.3 Memory Targets

| Project Size | Peak Memory | Target |
|--------------|-------------|--------|
| 1K LOC | <500MB | Normal laptop |
| 10K LOC | <2GB | Standard dev machine |
| 100K LOC | <4GB | Well-equipped dev machine |
| 1M LOC | <8GB | CI server |

### 8.4 Benchmark Suite

**Decision**: Ship comprehensive benchmark suite for regression detection.

```
BENCHMARK CATEGORIES

1. Incremental Builds
   - Single function body edit
   - Function signature change
   - New file addition
   - Dependency version bump

2. Clean Builds
   - Small project (1K LOC)
   - Medium project (10K LOC)
   - Large project (100K LOC)
   - Monorepo (1M LOC)

3. Check-Only Builds
   - Type checking performance
   - Effect inference performance
   - Contract verification performance

4. IDE Scenarios
   - Hover response time
   - Completion response time
   - Diagnostic refresh time

5. Cache Operations
   - Cache hit/miss ratio
   - Cache read/write throughput
   - Cache eviction overhead
```

---

## 9. Implementation Priorities

### Phase 1: Foundation (Weeks 1-4)

| Task | Description | Exit Criteria |
|------|-------------|---------------|
| Query Infrastructure | Salsa-based query system | Basic queries execute |
| File Watcher | Detect source changes | Changes trigger rebuild |
| Basic Caching | L2 disk cache | Artifacts stored/retrieved |
| Dependency Graph | Build and serialize | Graph persists across sessions |

### Phase 2: Incremental Analysis (Weeks 5-8)

| Task | Description | Exit Criteria |
|------|-------------|---------------|
| Function-Level Queries | Per-function inference | Body edits fast |
| Early Cutoff | Signature stability | Unchanged signatures skip callers |
| Version Vectors | Durability system | Stdlib changes cheap |
| Invalidation | Lazy propagation | Minimal recomputation |

### Phase 3: Parallel Compilation (Weeks 9-12)

| Task | Description | Exit Criteria |
|------|-------------|---------------|
| Work Stealing | Rayon integration | Parallel parsing/inference |
| Pipeline Parallelism | Phase overlap | Codegen starts before parse ends |
| CGU Parallelism | Parallel codegen | 16 CGUs compile in parallel |
| Memory Management | Backpressure | Large projects don't OOM |

### Phase 4: Distributed Cache (Weeks 13-16)

| Task | Description | Exit Criteria |
|------|-------------|---------------|
| L3 Protocol | Remote cache interface | S3 backend works |
| CI Integration | Cache warming | Main branch warms cache |
| Cross-Project | Shared deps | Common deps cached |
| Pre-built Stdlib | Ship compiled stdlib | Stdlib from cache |

### Phase 5: Polish (Weeks 17-20)

| Task | Description | Exit Criteria |
|------|-------------|---------------|
| Profiles | All four profiles | dev/release/check/test work |
| Performance | Meet targets | All benchmarks pass |
| Documentation | User guide | Build config documented |
| Telemetry | Build analytics | Metrics collected |

---

## 10. Configuration Reference

### 10.1 Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `ARIA_CACHE_DIR` | `~/.cache/aria` | Cache directory location |
| `ARIA_CACHE_MAX_SIZE` | `10GB` | Maximum L2 cache size |
| `ARIA_CACHE_MAX_AGE_DAYS` | `30` | Maximum artifact age |
| `ARIA_CACHE_MEMORY_MB` | `512` | L1 memory cache size |
| `ARIA_BUILD_JOBS` | `num_cpus` | Parallel build jobs |
| `ARIA_CODEGEN_UNITS` | `16` | Codegen parallelism |
| `ARIA_LLVM_THREADS` | `1` | LLVM threads per CGU |
| `ARIA_MAX_MEMORY` | `80%` | Maximum memory usage |

### 10.2 aria.toml Configuration

```toml
[build]
# Number of parallel jobs
jobs = 8

# Target triple (auto-detected if not set)
target = "x86_64-unknown-linux-gnu"

# Additional targets for cross-compilation
extra-targets = ["wasm32-unknown-unknown"]

[cache]
# Enable local disk cache
enabled = true

# Enable distributed cache
distributed = false

# Backend: s3, gcs, azure, redis, http
backend = "s3"

[cache.s3]
bucket = "my-aria-cache"
region = "us-west-2"
prefix = "v1/"

[profile.dev]
opt-level = 0
debug = true
incremental = true

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 16
```

---

## 11. Open Questions and Future Work

### 11.1 Deferred Decisions

| Question | Options | Timeline |
|----------|---------|----------|
| Salsa version | 0.16 stable vs 2.0 | Phase 1 evaluation |
| WASM caching | Different strategy? | Phase 4 |
| Effect caching granularity | Function vs expression | Phase 2 evaluation |

### 11.2 Future Enhancements

1. **Build Telemetry Dashboard**: Real-time build performance visualization
2. **Predictive Caching**: Pre-compile likely-needed artifacts
3. **Cross-Language Caching**: Share artifacts with Rust/C dependencies
4. **Cloud Build Service**: Offload compilation to cloud workers

---

## 12. Conclusion

Aria's build system provides:

1. **Fast Incremental Builds**: Function-level granularity with early cutoff
2. **Efficient Caching**: Three-layer cache with content-addressed storage
3. **Scalable Parallelism**: Work-stealing scheduler with pipeline overlap
4. **Reproducible Builds**: Hash-based validation with version vectors
5. **Team Collaboration**: Optional distributed cache for shared artifacts
6. **Flexible Profiles**: Four profiles covering development to production

This architecture ensures developers experience sub-second feedback during development while producing optimized, reproducible binaries for production.

---

*Document prepared by ACCELERATOR - Product Decision Agent*
*Research input from DELTA - Eureka Vault Research*
*Last updated: 2026-01-15*
