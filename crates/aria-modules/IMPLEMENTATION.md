# Aria Module System Implementation

This document describes the implementation details of the Aria module system.

## Overview

The module system consists of four main components:

1. **Module Resolver** - Finds and loads module files
2. **Module Graph** - Tracks dependencies and detects cycles
3. **Module Cache** - Stores parsed modules
4. **Module Compiler** - Orchestrates the compilation process

## Component Details

### 1. Module Resolver (`resolver.rs`)

#### ModuleResolver Trait
```rust
pub trait ModuleResolver: Send {
    fn resolve(&mut self, name: &str, current_path: Option<&Path>) -> ModuleResult<ModuleId>;
    fn resolve_path(&mut self, path: &Path) -> ModuleResult<ModuleId>;
    fn load(&mut self, id: ModuleId) -> ModuleResult<ResolvedModule>;
}
```

#### FileSystemResolver
- Searches for modules in configurable search paths
- Supports relative imports (relative to importing file)
- Handles both file modules (`math.aria`) and directory modules (`utils/mod.aria`)
- Assigns unique IDs to modules
- Caches path-to-ID mappings

**Resolution Algorithm:**
1. Check if importing from relative path
2. Search in each configured search path:
   - Try `<path>/module.aria`
   - Try `<path>/module/mod.aria`
3. Canonicalize path and assign/retrieve module ID

### 2. Module Graph (`graph.rs`)

#### Data Structure
```rust
pub struct ModuleGraph {
    dependencies: FxHashMap<ModuleId, Vec<ModuleId>>,  // Forward edges
    dependents: FxHashMap<ModuleId, Vec<ModuleId>>,    // Reverse edges
    nodes: FxHashSet<ModuleId>,                        // All modules
}
```

#### Cycle Detection
Uses depth-first search (DFS) with a recursion stack:
```rust
fn dfs_cycle_detection(
    &self,
    node: ModuleId,
    visited: &mut FxHashSet<ModuleId>,
    rec_stack: &mut FxHashSet<ModuleId>,
    path: &mut Vec<ModuleId>,
) -> Option<Vec<ModuleId>>
```

- Marks nodes as visited
- Tracks nodes in current recursion stack
- When a node in rec_stack is encountered, extracts the cycle

#### Topological Sort
Uses Kahn's algorithm for topological sorting:
1. Calculate in-degree for each node
2. Add zero in-degree nodes to queue
3. Process queue, decrementing in-degrees
4. Result is dependency order (dependencies first)

**Time Complexity:** O(V + E) where V = modules, E = imports

### 3. Module Cache (`cache.rs`)

Simple hash map cache:
```rust
pub struct ModuleCache {
    modules: FxHashMap<ModuleId, Module>,
}
```

**Operations:**
- `insert`: O(1) average
- `get`: O(1) average
- `contains`: O(1) average

Prevents re-parsing the same module multiple times.

### 4. Module Compiler (`lib.rs`)

#### Module Structure
```rust
pub struct Module {
    pub id: ModuleId,
    pub ast: Program,
    pub path: PathBuf,
    pub name: SmolStr,
    pub dependencies: Vec<ModuleId>,
    pub exports: FxHashSet<SmolStr>,
    pub private_items: FxHashSet<SmolStr>,
}
```

#### Compilation Flow
```rust
pub fn compile(&mut self, entry_point: &PathBuf) -> ModuleResult<Vec<Module>>
```

**Steps:**
1. Resolve entry point path to module ID
2. Load and parse entry module
3. Build dependency graph recursively:
   - Extract imports from module
   - Resolve each import to module ID
   - Load and parse dependency
   - Repeat for each dependency
4. Detect circular dependencies
5. Topologically sort modules
6. Return modules in dependency order

#### Import Resolution
```rust
fn resolve_import(&mut self, current: &Module, import: &ImportDecl) -> ModuleResult<ModuleId>
```

Handles two import path types:
- **Module path**: `import foo::bar::baz` → resolve as module path
- **String path**: `import "relative/path"` → resolve relative to current module

### 5. Export/Import Tracking

#### Exports
Collected during module creation:
```rust
for item in &ast.items {
    match item {
        Item::Export(export) => {
            // Handle export declarations
        }
        _ => {
            if is_public_item(item) {
                exports.insert(name);
            }
        }
    }
}
```

#### Import Validation (Future)
Currently not validated, but the infrastructure is ready:
```rust
pub fn is_exported(&self, name: &str) -> bool {
    self.exports.contains(name)
}
```

## Data Flow

```
Entry Point (main.aria)
    ↓
FileSystemResolver::resolve_path()
    ↓
ModuleCompiler::compile()
    ↓
parse() → Module::new()
    ↓
Extract imports → resolve_import()
    ↓
Recursively load dependencies
    ↓
Build ModuleGraph
    ↓
Detect cycles (DFS)
    ↓
Topological sort (Kahn)
    ↓
Return Vec<Module> (dependency order)
```

## Error Handling

### Error Types
```rust
pub enum ModuleError {
    ModuleNotFound(String),
    FileNotFound(PathBuf),
    IoError(PathBuf, std::io::Error),
    ParseError { path: PathBuf, errors: Vec<ParseError> },
    CircularDependency(Vec<ModuleId>),
    ImportResolutionFailed(String),
    ItemNotFound { module: String, item: String },
    PrivateItem { module: String, item: String },
    ConflictingImports(String),
    NameConflict(String),
}
```

### Error Recovery
- Parse errors are collected but don't stop dependency resolution
- Circular dependencies are detected before type checking
- Missing modules fail fast with clear error messages

## Performance Characteristics

### Time Complexity
- Module resolution: O(S × N) where S = search paths, N = path components
- Dependency graph construction: O(M + I) where M = modules, I = imports
- Cycle detection: O(M + I)
- Topological sort: O(M + I)
- **Total**: O(M × (S + I/M) + M + I) ≈ O(M × S + I)

### Space Complexity
- Module cache: O(M × P) where P = average module size
- Dependency graph: O(M + I)
- Resolution maps: O(M)
- **Total**: O(M × P)

### Optimizations
1. **Module caching**: Parse each module only once
2. **Efficient hash maps**: Use FxHashMap (fast non-cryptographic hash)
3. **Incremental resolution**: Only resolve imports when needed
4. **Path canonicalization**: Ensures same file = same module ID

## Integration with Compiler

### In `aria-compiler/src/main.rs`:
```rust
// Create resolver with search paths
let mut resolver = FileSystemResolver::new();
for lib_path in lib_paths {
    resolver.add_search_path(lib_path);
}

// Compile all modules
let mut compiler = ModuleCompiler::new(Box::new(resolver), mode);
let modules = compiler.compile(path)?;

// Type check each module (future: with cross-module resolution)
for module in &modules {
    checker.check_program(&module.ast)?;
}

// Lower to MIR (future: all modules)
let mir = aria_mir::lower_program(&entry_module.ast)?;
```

## Future Enhancements

### 1. Incremental Compilation
- Track file modification times
- Only recompile changed modules
- Propagate changes to dependents

### 2. Module Metadata
- Pre-compile to `.ariac` files
- Store type signatures
- Enable faster import resolution

### 3. Package Manager Integration
- Resolve external dependencies
- Semantic versioning
- Dependency locking

### 4. Advanced Import Features
- Re-exports: `pub use module::item`
- Glob imports with filtering
- Trait/impl imports
- Macro imports

### 5. Module Visibility
- `pub(crate)` - visible in current crate
- `pub(super)` - visible in parent module
- `pub(in path)` - visible in specific path

### 6. Workspace Support
- Multi-crate projects
- Shared dependencies
- Workspace-wide caching

## Testing Strategy

### Unit Tests
- Module resolver path resolution
- Graph cycle detection
- Topological sorting
- Cache operations

### Integration Tests
- Simple two-module compilation
- Transitive dependencies (A → B → C)
- Circular dependency detection
- Module not found handling
- Private item access

### End-to-End Tests
- Full compilation pipeline
- Multiple search paths
- Library vs binary mode
- Error reporting

## Debugging

### Logging Points
1. Module resolution: Print resolved paths
2. Dependency graph: Print edges as added
3. Cycle detection: Print DFS traversal
4. Import resolution: Print import → module mapping

### Common Issues
1. **Module not found**: Check search paths and file names
2. **Circular dependency**: Visualize dependency graph
3. **Parse errors**: Show full error with context
4. **Import resolution**: Verify module exports match imports
