# Milestone M15: Module System

## Overview

Design Aria's module system for imports, exports, visibility control, and crate-level organization.

## Research Questions

1. What's the right balance of explicit vs implicit imports?
2. How do we handle circular dependencies?
3. What visibility levels do we need?
4. How do we support conditional compilation?

## Core Innovation Target

```ruby
# Module definition
module MyApp::Models

  pub struct User
    pub name: String
    priv password_hash: String  # Private field
    email: String               # Module-private by default
  end

end

# Imports
import std::collections::{Array, Map, Set}
import MyApp::Models::User
import MyApp::Models::*                    # Glob import

# Re-exports
module MyApp
  pub use MyApp::Models::User
  pub use MyApp::Services::*
end

# Conditional compilation
@cfg(target: :wasm)
fn browser_specific()
  # Only compiled for WASM
end
```

## Competitive Analysis Required

| Language | Module System | Study Focus |
|----------|---------------|-------------|
| Rust | mod + use | Crate system |
| Go | packages | Simplicity |
| Python | import | Flexibility |
| TypeScript | ES modules | Web compat |
| Zig | @import | Build system |

## Tasks

### ARIA-M15-01: Compare module systems
- **Description**: Comprehensive comparison of module systems
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, modules, comparison
- **Deliverables**:
  - Feature comparison
  - Ergonomics analysis
  - Build integration

### ARIA-M15-02: Study Rust's crate system
- **Description**: Analyze Rust's module and crate design
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, modules, rust
- **Deliverables**:
  - Module resolution
  - Visibility rules
  - Orphan rules

### ARIA-M15-03: Research circular dependency handling
- **Description**: Study circular import approaches
- **Dependency**: AGENT_CAPABLE
- **Priority**: normal
- **Tags**: research, modules, circular
- **Deliverables**:
  - Detection algorithms
  - Resolution strategies
  - Error messages

### ARIA-M15-04: Design module syntax
- **Description**: Design Aria's module syntax
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Blocked by**: ARIA-M15-01
- **Tags**: research, modules, syntax, design
- **Deliverables**:
  - Module declaration syntax
  - Import syntax
  - Visibility modifiers

### ARIA-M15-05: Design visibility rules
- **Description**: Design visibility and access control
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Blocked by**: ARIA-M15-02
- **Tags**: research, modules, visibility, design
- **Deliverables**:
  - Visibility levels
  - Access control rules
  - Orphan rule design

### ARIA-M15-06: Design conditional compilation
- **Description**: Design cfg/feature system
- **Dependency**: AGENT_CAPABLE
- **Priority**: normal
- **Tags**: research, modules, cfg, design
- **Deliverables**:
  - Conditional compilation syntax
  - Feature flags
  - Platform targeting

## Implementation Progress

### Module System Infrastructure (COMPLETED - Jan 2026)
- [x] `crates/aria-modules/` crate with comprehensive module support
- [x] Module struct with exports, private_items, dependencies tracking
- [x] ModuleCompiler with resolver, graph, and cache
- [x] ModuleGraph for dependency tracking with cycle detection
- [x] FileSystemResolver for path resolution
- [x] ModuleCache for caching compiled modules
- [x] Topological sorting for compilation order
- [x] Circular dependency detection with clear error messages
- [x] 40 unit + integration tests passing

### Module Parsing (COMPLETED - Jan 2026)
- [x] `module MyApp::Models ... end` syntax parsing
- [x] `import std::collections::{Array, Map}` with selection
- [x] `import MyApp::Models::*` glob imports
- [x] `import Foo::Bar as Alias` with aliasing
- [x] `pub use` re-export declarations
- [x] `use Path::{A, B}` with item selection
- [x] `use Path::* ` glob re-exports
- [x] `use LongName as Short` with aliasing

### Visibility System (COMPLETED - Jan 2026)
- [x] `pub` visibility keyword in lexer/parser
- [x] `priv` explicit private visibility
- [x] Default visibility is Private
- [x] Visibility tracking in all declarations
- [x] Export/private item tracking in Module struct

### AST Infrastructure (COMPLETED - Jan 2026)
- [x] Attribute struct in AST
- [x] attributes field added to FunctionDecl, StructDecl, DataDecl
- [x] attributes field added to EnumDecl, TraitDecl, ImplDecl
- [x] attributes field added to ConstDecl, TypeAlias
- [x] UseDecl for re-exports with visibility, path, selection, alias

### Remaining Work
- [ ] Attribute parsing (@cfg, @derive, etc.)
- [ ] Conditional compilation based on @cfg attributes
- [ ] Module visibility access checking in type checker
- [ ] Re-export resolution in module graph
- [ ] Platform-specific compilation flags

## Success Criteria

- [x] Module syntax finalized
- [x] Visibility rules clear
- [x] Circular dependency strategy defined
- [ ] Conditional compilation designed

## Key Resources

1. Rust module documentation
2. "The Design of the Go Module System"
3. Python import system docs
4. TypeScript module resolution
5. Zig build system documentation

## Timeline

Target: Q2 2026

## Related Milestones

- **Depends on**: M01 (Types)
- **Enables**: M17 (Package Manager)
- **Parallel**: M16 (Standard Library)
