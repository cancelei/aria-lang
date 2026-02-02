# aria-modules

The Aria module system provides module resolution, dependency tracking, and import/export handling for the Aria programming language.

## Features

- **Module Resolution**: FileSystemResolver for loading `.aria` files from disk
- **Dependency Graph**: Track and visualize module dependencies
- **Circular Dependency Detection**: Automatically detect and report circular imports
- **Module Caching**: Cache parsed modules to avoid redundant parsing
- **Import Resolution**: Support for multiple import styles
  - `import module::item`
  - `import module::{item1, item2}`
  - `import module::*`
  - `import module as alias`
- **Export Control**: Public/private visibility for module items
- **Multi-file Compilation**: Compile projects with multiple modules

## Architecture

### Components

1. **ModuleResolver** (`resolver.rs`)
   - Trait for resolving module names to files
   - `FileSystemResolver`: Searches for modules in configurable paths
   - Supports both relative and absolute imports

2. **ModuleGraph** (`graph.rs`)
   - Directed graph of module dependencies
   - Cycle detection using DFS
   - Topological sorting for compilation order

3. **ModuleCache** (`cache.rs`)
   - Stores parsed modules to avoid re-parsing
   - Fast lookups by module ID

4. **ModuleCompiler** (`lib.rs`)
   - Orchestrates module compilation
   - Resolves all imports recursively
   - Produces modules in dependency order

## Usage

### Basic Example

```rust
use aria_modules::{ModuleCompiler, FileSystemResolver, CompilationMode};
use std::path::PathBuf;

// Create a resolver
let mut resolver = FileSystemResolver::new();
resolver.add_search_path("./lib");
resolver.add_search_path("./src");

// Create a compiler
let mut compiler = ModuleCompiler::new(
    Box::new(resolver),
    CompilationMode::Binary
);

// Compile from entry point
let modules = compiler.compile(&PathBuf::from("main.aria"))?;

// Modules are returned in dependency order (dependencies first)
for module in modules {
    println!("Module: {} ({})", module.name, module.path.display());
}
```

### Module Resolution

The resolver searches for modules in the following order:

1. Relative to the importing file
2. In configured search paths
3. As a directory with `mod.aria`

Example file structure:
```
project/
├── main.aria
├── math.aria
└── utils/
    ├── mod.aria
    ├── strings.aria
    └── numbers.aria
```

Import examples:
```aria
// Import from math.aria
import math::{add, multiply}

// Import from utils/mod.aria
import utils

// Import from utils/strings.aria
import utils::strings::{trim, uppercase}
```

### Compilation Modes

- **Binary**: Entry point should have a `main()` function
- **Library**: No entry point required, all public items are exported

```bash
# Compile as binary
aria build main.aria

# Compile as library
aria build lib.aria --lib

# Add search paths
aria build main.aria -L ./lib -L ./vendor
```

### Module Structure

```aria
// math.aria
pub fn add(x: Int, y: Int) -> Int = x + y

fn helper() = 42  // private, not exported

pub const PI: Float = 3.14159

// Export specific items
export {add, PI}
```

## Implementation Details

### Module IDs

Each module is assigned a unique `ModuleId` when first encountered. This ID is used throughout the compilation process to reference modules.

### Dependency Resolution

1. Start from entry point
2. Parse the module
3. Extract import declarations
4. Recursively resolve each import
5. Build dependency graph
6. Check for cycles
7. Return modules in topological order

### Circular Dependency Detection

The module graph uses depth-first search to detect cycles. When found, the full cycle path is reported:

```
Error: Circular dependency detected:
  module A
  -> module B
  -> module C
  -> module A
```

### Performance

- Modules are cached after parsing
- Dependency graph uses efficient hash maps
- Topological sort is O(V + E) using Kahn's algorithm

## Testing

Run the test suite:
```bash
cargo test --package aria-modules
```

## Future Enhancements

- [ ] Incremental compilation
- [ ] Module pre-compilation and binary caching
- [ ] Package manager integration
- [ ] Remote module resolution
- [ ] Workspace support
- [ ] Module versioning
- [ ] Re-export support (`pub use`)
- [ ] Glob imports with filtering
