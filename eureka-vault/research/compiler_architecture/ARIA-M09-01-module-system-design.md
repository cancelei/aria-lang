# ARIA-M09-01: Module System Design for Aria

**Task ID**: ARIA-M09-01
**Status**: Completed
**Date**: 2026-01-15
**Agent**: ATLAS2 (Research Agent)
**Focus**: Comprehensive module system design balancing simplicity (Go) with power (Rust)

---

## Executive Summary

This research document proposes Aria's module system design after analyzing Rust, Go, TypeScript, and OCaml approaches. The design philosophy follows "progressive complexity" - simple by default with power available when needed.

**Key Design Decisions**:
1. **Path-based modules** with explicit file mapping (inspired by Rust 2018+)
2. **Three-tier visibility**: private (default), `pub(package)`, and `pub`
3. **Re-exports via `pub use`** with facade pattern support
4. **No cyclic dependencies** - enforced at compile time with clear error messages

---

## 1. Module System Comparison Matrix

### 1.1 Feature Comparison

| Feature | Rust | Go | TypeScript | OCaml | **Aria (Proposed)** |
|---------|------|----|-----------:|-------|---------------------|
| **File = Module** | Configurable | Yes (directory) | Yes | Yes | Yes (1:1 mapping) |
| **Visibility Levels** | 5 (`pub`, `pub(crate)`, `pub(super)`, `pub(in path)`, private) | 2 (exported/unexported) | 2 (export/private) | Signature-based | 3 (`pub`, `pub(package)`, private) |
| **Cyclic Dependencies** | Prohibited | Prohibited | Allowed | Prohibited | Prohibited |
| **Re-exports** | `pub use` | N/A | `export from` | Included in signature | `pub use` |
| **Parameterized Modules** | No (use generics) | No | No | Functors | Lightweight (via generics) |
| **First-class Modules** | No | No | Yes (as objects) | Yes | No (by design) |
| **Import Granularity** | Item-level | Package-level | Item-level | Module-level | Item-level |
| **Inline Modules** | Yes | No | No | Yes | Yes |
| **Glob Imports** | Yes (`*`) | No | Yes (`*`) | No | Yes (discouraged) |

### 1.2 Ergonomics Comparison

| Aspect | Rust | Go | TypeScript | **Aria Target** |
|--------|------|----|-----------:|-----------------|
| Learning curve | Steep | Gentle | Moderate | **Gentle** |
| Boilerplate | Moderate | Low | Low | **Low** |
| Refactoring ease | Moderate | High | Moderate | **High** |
| IDE support difficulty | Moderate | Easy | Easy | **Easy** |
| Error message clarity | Good | Excellent | Good | **Excellent** |

---

## 2. Aria Module System Design

### 2.1 Core Principles

1. **Explicit over implicit**: Module structure should be predictable from file system
2. **Progressive complexity**: Simple defaults, power when needed
3. **Clear boundaries**: Visibility rules should be easy to reason about
4. **Fast compilation**: Module graph should enable parallel compilation
5. **Great error messages**: When things go wrong, tell users exactly why and how to fix

### 2.2 File-to-Module Mapping

Aria uses a **1:1 file-to-module mapping** with explicit path correlation:

```
src/
  main.aria           -> Main module (entry point)
  lib.aria            -> Library root (if building library)
  utils.aria          -> Utils module
  models/
    mod.aria          -> Models module (declares submodules)
    user.aria         -> Models::User submodule
    post.aria         -> Models::Post submodule
  services/
    mod.aria          -> Services module
    auth.aria         -> Services::Auth submodule
```

**Key Rules**:
- Each `.aria` file is exactly one module
- Directory with `mod.aria` creates a module with submodules
- No `mod.rs` vs `module.rs` confusion (always `mod.aria` for parent modules)
- Module names derived from file/directory names (snake_case -> PascalCase for types)

### 2.3 Module Declaration Syntax

```aria
# File: src/models/user.aria

# Module header (optional - inferred from file path)
module Models::User

# Visibility is per-item, not per-module
pub struct User
  pub name: String
  pub email: String
  age: Int                    # private to module by default
end

pub fn create_user(name: String, email: String) -> User
  User(name:, email:, age: 0)
end

# Internal helper - not exported
fn validate_email(email: String) -> Bool
  email.contains?("@")
end
```

```aria
# File: src/models/mod.aria

# Parent module declares submodules
module Models

# Declare submodules (loads from files)
pub mod user      # exports Models::User publicly
pub mod post      # exports Models::Post publicly
mod internal      # private submodule

# Re-exports for convenience
pub use user::User
pub use user::create_user
pub use post::Post
```

---

## 3. Visibility Levels Design

### 3.1 Three-Tier Visibility Model

Aria simplifies Rust's five visibility levels to three, covering 99% of use cases:

| Visibility | Keyword | Accessible From | Use Case |
|------------|---------|-----------------|----------|
| **Private** | (default) | Same module only | Implementation details |
| **Package** | `pub(package)` | Same package/crate | Internal APIs |
| **Public** | `pub` | Everywhere | Public API |

### 3.2 Visibility Syntax

```aria
module MyLib::Internal

# Private (default) - only this module can access
fn helper() = ...

# Package-visible - any module in this package can access
pub(package) fn internal_api() = ...

# Public - anyone can access
pub fn public_api() = ...

# Struct field visibility
pub struct Config
  pub host: String              # public field
  pub(package) port: Int        # package-visible field
  secret: String                # private field
end
```

### 3.3 Rationale for Three Tiers

**Why not more levels (like Rust)?**
- `pub(super)` and `pub(in path)` add complexity for rare use cases
- Package boundary is the most meaningful encapsulation unit
- Simpler mental model: "Is this for my module, my package, or everyone?"

**Why not two levels (like Go)?**
- `pub(package)` enables internal APIs that aren't part of public contract
- Critical for large projects with multiple internal modules
- Matches intuition from Java (`public`, `protected`, package-private, `private`)

### 3.4 Privacy Defaults

```aria
# Everything private by default (explicit exports)
module MyModule

struct InternalData     # private
  field: Int            # private (struct is private anyway)
end

pub struct PublicData
  pub name: String      # public
  internal: Int         # private (explicit fields must be marked pub)
end

fn helper() = ...       # private
pub fn api() = ...      # public
```

---

## 4. Import and Export Syntax

### 4.1 Import Syntax

```aria
# Simple import (brings module into scope)
import std::collections

# Use: collections::Array, collections::Map

# Selective import (specific items)
import std::collections::{Array, Map, Set}

# Use: Array, Map, Set directly

# Aliased import
import std::collections::HashMap as Dict

# Use: Dict instead of HashMap

# Qualified import (for disambiguation)
import std::net::http as http

# Use: http.get(), http.Response

# Glob import (discouraged but available)
import std::prelude::*

# Path-based import (for vendored dependencies)
import "vendor/custom_lib" as custom
```

### 4.2 Export and Re-export Syntax

```aria
# File: src/lib.aria
module MyLib

# Import internal modules
mod models
mod services
mod utils

# Re-export public API (facade pattern)
pub use models::User
pub use models::Post
pub use services::auth::{login, logout}

# Bulk re-export
pub use utils::*    # Re-export everything from utils

# Re-export with rename
pub use models::UserError as Error
```

### 4.3 Export List Alternative

```aria
# Alternative: explicit export list at module level
module MyLib

export {
  User,
  Post,
  login,
  logout,
  Error
}

# Rest of module...
```

### 4.4 Import Resolution Algorithm

```
1. Check if path starts with "std::" -> Standard library
2. Check if path starts with package name -> Current package
3. Check if path is quoted string -> File path import
4. Check if path matches dependency in aria.toml -> External package
5. Error: Module not found
```

**Resolution Order for Name Conflicts**:
1. Locally defined items (current module)
2. Explicitly imported items
3. Prelude items (if not shadowed)

---

## 5. Re-exports and Module Facades

### 5.1 Facade Pattern Support

The facade pattern is a common design in Rust. Aria supports it with cleaner syntax:

```aria
# File: src/lib.aria
# This is the public API facade

module MyAwesomeLib

# Private implementation modules
mod internal
mod impl_details

# Public submodules (if needed)
pub mod advanced

# Re-export the public API
pub use internal::core::{Engine, Config}
pub use internal::io::{read_file, write_file}
pub use impl_details::helpers::format

# Users see:
#   MyAwesomeLib::Engine
#   MyAwesomeLib::Config
#   MyAwesomeLib::read_file
#   MyAwesomeLib::write_file
#   MyAwesomeLib::format
#   MyAwesomeLib::advanced::*
```

### 5.2 Re-export Visibility Rules

```aria
# Re-export visibility is determined by the `pub` keyword
pub use internal::Public      # Now public
pub(package) use internal::SemiPublic  # Package-visible
use internal::Private         # Private re-export (rarely useful)

# Cannot increase visibility of private items
# This would be an error:
# pub use internal::private_fn  # Error: cannot re-export private item
```

### 5.3 Inline Documentation for Re-exports

```aria
## Re-exported from internal::core
##
## The main engine for processing data.
pub use internal::core::Engine
```

---

## 6. Cyclic Dependencies Handling

### 6.1 Design Decision: No Cyclic Dependencies

Aria **prohibits cyclic module dependencies** at compile time, following Rust and Go.

**Rationale**:
- Enables efficient parallel compilation
- Forces better code architecture
- Eliminates subtle initialization order bugs
- Makes dependency graph easy to reason about

### 6.2 Detection Algorithm

```
Build directed graph G where:
  - Nodes = modules
  - Edges = import relationships

Run Tarjan's SCC algorithm to find cycles

If cycle found:
  Report clear error with cycle visualization
  Suggest refactoring strategies
```

### 6.3 Error Message Design

```
error[E0391]: cyclic dependency detected

  --> src/models/user.aria:3:1
   |
 3 | import Services::Auth
   | ^^^^^^^^^^^^^^^^^^^^^ imports Services::Auth

  --> src/services/auth.aria:4:1
   |
 4 | import Models::User
   | ^^^^^^^^^^^^^^^^^^^ imports Models::User

Dependency cycle: Models::User -> Services::Auth -> Models::User

help: Break the cycle by:
  1. Extract shared types to a new module that both can import
  2. Use dependency injection to pass required functionality
  3. Merge the modules if they're truly interdependent

For more information, see: https://aria-lang.org/errors/E0391
```

### 6.4 Resolution Strategies

**Strategy 1: Extract Common Types**
```aria
# Before: User imports Auth, Auth imports User
# After: Both import Common

# File: src/common/types.aria
module Common::Types

pub struct UserId(Int)
pub struct AuthToken(String)

# File: src/models/user.aria
import Common::Types::UserId

pub struct User
  id: UserId
  # ...
end

# File: src/services/auth.aria
import Common::Types::{UserId, AuthToken}

pub fn authenticate(id: UserId) -> AuthToken?
  # ...
end
```

**Strategy 2: Dependency Injection**
```aria
# File: src/services/auth.aria
# Instead of importing User, accept a trait

pub trait Authenticatable
  fn get_credentials(self) -> (String, String)
end

pub fn login<T: Authenticatable>(entity: T) -> AuthToken?
  let (username, password) = entity.get_credentials()
  # ...
end

# File: src/models/user.aria
import Services::Auth::Authenticatable

impl Authenticatable for User
  fn get_credentials(self) -> (String, String)
    (self.email, self.password_hash)
  end
end
```

---

## 7. Name Resolution Algorithm

### 7.1 Two-Phase Resolution

**Phase 1: Build Module Graph**
1. Parse all files to identify module declarations
2. Build dependency graph from import statements
3. Check for cycles (error if found)
4. Topologically sort modules for compilation order

**Phase 2: Resolve Names**
1. For each module in topological order:
   a. Resolve imports (bring names into scope)
   b. Resolve local definitions
   c. Check visibility constraints
   d. Report unresolved names

### 7.2 Name Lookup Order

```
When resolving name `foo`:

1. Local scope (function parameters, let bindings)
2. Current module definitions
3. Explicitly imported names
4. Re-exported names from used modules
5. Prelude (std::prelude::*)

If multiple matches found:
  - Local wins over imported
  - Explicit import wins over glob import
  - Report ambiguity error if still unclear
```

### 7.3 Qualified vs Unqualified Names

```aria
import std::collections::HashMap

# Unqualified (if imported specifically or via glob)
let map: HashMap<String, Int> = HashMap.new()

# Qualified (always works)
let map: std::collections::HashMap<String, Int> = ...

# After aliased import
import std::collections::HashMap as Dict
let map: Dict<String, Int> = Dict.new()
```

### 7.4 Shadowing Rules

```aria
import std::collections::Array

# Local definition shadows import
type Array = [Int; 10]  # Shadows std::collections::Array in this scope

# Function parameter shadows outer scope
fn process(Array: Type)  # Parameter shadows type alias

# Warning: shadowing prelude items
let Option = 42  # Warning: shadows std::Option
```

---

## 8. Proposed Syntax Summary

### 8.1 Complete Syntax Reference

```ebnf
(* Module declaration *)
module_decl     = 'module' module_path ;
module_path     = identifier { '::' identifier } ;

(* Submodule declaration *)
submod_decl     = visibility 'mod' identifier ;
visibility      = [ 'pub' | 'pub' '(' 'package' ')' ] ;

(* Import declaration *)
import_decl     = 'import' import_path [ import_items ] [ import_alias ] ;
import_path     = module_path | string_lit ;
import_items    = '::' ( '*' | '{' import_list '}' ) ;
import_list     = import_item { ',' import_item } ;
import_item     = identifier [ 'as' identifier ] ;
import_alias    = 'as' identifier ;

(* Re-export declaration *)
reexport_decl   = visibility 'use' reexport_path ;
reexport_path   = module_path [ '::' ( '*' | '{' import_list '}' ) ] ;

(* Export list (alternative syntax) *)
export_decl     = 'export' '{' identifier_list '}' ;
identifier_list = identifier { ',' identifier } ;
```

### 8.2 Quick Reference Card

```aria
# Module declaration
module MyLib::SubModule

# Import styles
import std::io                        # Module import
import std::io::{File, Error}         # Selective import
import std::io::File as IoFile        # Aliased import
import "vendor/lib" as vendor         # Path import
import std::prelude::*                # Glob import (avoid)

# Visibility
fn private_fn() = ...                 # Private (default)
pub(package) fn internal_fn() = ...   # Package-visible
pub fn public_fn() = ...              # Public

# Submodules
pub mod submodule                     # Public submodule
mod private_mod                       # Private submodule

# Re-exports
pub use internal::Type                # Re-export single item
pub use internal::{A, B, C}           # Re-export multiple
pub use internal::*                   # Re-export all (avoid)
```

---

## 9. Comparison with Existing Aria Grammar

The current Aria grammar (from GRAMMAR.md) already supports:

```aria
# Current syntax
import std::collections::{Array, Map, Set}
import MyApp::Models::*

module MyApp::Models
  pub struct User
    pub name: String
    priv password_hash: String
  end
end
```

**Proposed Changes**:

| Current | Proposed | Rationale |
|---------|----------|-----------|
| `priv` keyword | (default private) | Simpler, matches modern languages |
| No `pub(package)` | Add `pub(package)` | Internal API support |
| `export {items}` | Keep as alternative | Explicit export list option |
| `mod.aria` convention | Formalize | Clear parent module pattern |

---

## 10. Implementation Considerations

### 10.1 Compiler Phases

1. **Lexing/Parsing**: Recognize module syntax
2. **Module Graph Construction**: Build dependency DAG
3. **Cycle Detection**: Tarjan's algorithm
4. **Name Resolution**: Two-phase as described
5. **Type Checking**: With module-aware visibility
6. **Code Generation**: Per-module compilation units

### 10.2 Incremental Compilation Support

```
Module change -> Recompile module + dependents
Import change -> Recheck name resolution for importing modules
Visibility change -> Recheck all modules importing this item
```

### 10.3 IDE Integration

- **Go to definition**: Follow module paths
- **Find references**: Respect visibility boundaries
- **Refactoring**: Move items with visibility updates
- **Completion**: Show visible items only

---

## 11. Key Resources

### Research Sources

1. [Rust Module System Explained](https://confidence.sh/blog/rust-module-system-explained/) - Comprehensive Rust overview
2. [Rust Modules and Visibility](https://www.buildwithrs.dev/docs/rust-2025/module) - 2025 best practices
3. [Rust Module Reference](https://doc.rust-lang.org/reference/items/modules.html) - Official specification
4. [Go vs Rust Package Comparison](https://bitfieldconsulting.com/posts/rust-vs-go) - Architecture trade-offs
5. [TypeScript Module Resolution](https://www.typescriptlang.org/docs/handbook/module-resolution.html) - Resolution strategies
6. [OCaml Functors](https://dev.realworldocaml.org/functors.html) - Parameterized modules
7. [OCaml First-Class Modules](https://dev.realworldocaml.org/first-class-modules.html) - Runtime module selection
8. [Rust Visibility Reference](https://doc.rust-lang.org/reference/visibility-and-privacy.html) - Privacy model
9. [Rust Re-exports](https://doc.rust-lang.org/rustdoc/write-documentation/re-exports.html) - Facade pattern

### Related Aria Documents

- `eureka-vault/milestones/ARIA-M15-module-system.md` - Module system milestone
- `eureka-vault/research/stdlib/ARIA-M15-01-module-systems-comparison.md` - Prior comparison research
- `GRAMMAR.md` - Current Aria grammar specification

---

## 12. Open Questions for Future Work

1. **Conditional Compilation**: How do `@cfg` attributes interact with modules?
2. **Effect Boundaries**: Should module boundaries be effect polymorphic?
3. **Build System Integration**: How does `aria.toml` specify dependencies?
4. **Versioning**: How to handle breaking changes in module APIs?
5. **Documentation Generation**: How to generate docs from re-export chains?

---

## 13. Conclusion

Aria's module system achieves the goal of balancing simplicity with power:

- **Simple defaults**: Private by default, 1:1 file mapping, no cycles
- **Progressive complexity**: Three-tier visibility, facade support, qualified imports
- **Clear errors**: Cycle detection with actionable suggestions
- **IDE-friendly**: Predictable resolution enables excellent tooling

The design learns from Rust's expressiveness while avoiding its complexity, and from Go's simplicity while adding necessary flexibility for large projects.

---

**Document Status**: Research Complete
**Next Steps**:
1. Review with ARIA team
2. Update GRAMMAR.md with finalized syntax
3. Implement module graph builder in compiler
4. Add comprehensive test cases for edge cases
