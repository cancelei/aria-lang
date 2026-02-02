# ARIA-PD-019: Module System Design

**Document Type**: Product Design Document (PDD)
**Status**: APPROVED
**Date**: 2026-01-15
**Author**: MAPPER (Product Decision Agent)
**Research Source**: NEXUS-II (ARIA-M07-04)

---

## Executive Summary

This document establishes Aria's official module system design based on comprehensive research from ARIA-M07-04. After reviewing Rust's module system, Python's imports, ES6 modules, Go packages, and Haskell/OCaml module systems, this PDD makes concrete syntax and semantic decisions.

**Final Decisions**:
1. **Import syntax**: `import` keyword with Rust-style `::` path separators
2. **Visibility modifiers**: Three tiers - private (default), `pub(package)`, `pub`
3. **File-to-module mapping**: 1:1 with `mod.aria` for parent modules
4. **Re-export syntax**: `pub use` for re-exports
5. **Circular dependencies**: Prohibited at compile time with Tarjan's SCC detection
6. **Glob imports**: Allowed with mandatory linter warnings
7. **Name resolution**: Two-phase with explicit > glob > prelude precedence
8. **Prelude**: Minimal set of essential types and traits

---

## 1. Import Syntax

### 1.1 Decision: `import` Keyword with `::` Paths

**DECIDED**: Aria uses `import` (not `use`) for importing, with Rust-style `::` path separators.

```aria
# Simple module import
import std::collections

# Selective import (preferred)
import std::collections::{Array, Map, Set}

# Aliased import
import std::collections::HashMap as Dict

# Module namespace import
import std::net::http as http

# Relative imports (within package)
import self::helper           # Same module directory
import super::parent          # Parent module
import super::super::root     # Grandparent

# Glob import (discouraged, emits warning)
import std::prelude::*

# Path-based import (vendored dependencies)
import "vendor/custom_lib" as custom

# Multiple imports from same root
import std::{
  collections::{Array, Map},
  io::{File, Error as IoError},
  net::http
}
```

**Rationale**:
- `import` is more intuitive for developers from Python, ES6, Java backgrounds
- `::` path separator clearly distinguishes module paths from member access (`.`)
- Selective imports enable tree-shaking and explicit dependency tracking
- Relative imports (`self::`, `super::`) provide clarity without ambiguity

### 1.2 Import Grammar (EBNF)

**DECIDED**: The following EBNF extends GRAMMAR.md Section 14.

```ebnf
(* Import declarations - replaces/extends Section 14.2 *)
import_decl     = 'import' import_spec ;

import_spec     = import_source [ import_modifier ] ;

import_source   = module_path
                | string_literal ;

module_path     = path_segment { '::' path_segment } ;

path_segment    = identifier
                | 'self'
                | 'super' ;

import_modifier = '::' import_selection
                | 'as' identifier ;

import_selection = '*'
                 | '{' import_list '}' ;

import_list     = import_item { ',' import_item } [ ',' ] ;

import_item     = identifier [ 'as' identifier ]
                | nested_import ;

nested_import   = identifier '::' import_selection ;
```

### 1.3 Path Resolution Rules

| Path Type | Syntax | Resolves To |
|-----------|--------|-------------|
| Absolute | `std::io::File` | Standard library module |
| Relative (self) | `self::helper` | Sibling module in same directory |
| Relative (super) | `super::parent` | Parent module |
| Package root | `MyPackage::models` | Package-level module |
| String path | `"vendor/lib"` | File system path |

---

## 2. Visibility Modifiers

### 2.1 Decision: Three-Tier Visibility Model

**DECIDED**: Aria uses exactly three visibility levels.

| Visibility | Keyword | Scope | Use Case |
|------------|---------|-------|----------|
| **Private** | (default) | Same module only | Implementation details |
| **Package** | `pub(package)` | Same package | Internal APIs |
| **Public** | `pub` | Everyone | Public API |

```aria
module MyLib::Internal

# Private (default) - only this module can access
fn helper()
  # Implementation detail
end

# Package-visible - any module in this package can access
pub(package) fn internal_api()
  # Shared within package, not public
end

# Public - anyone can access
pub fn public_api()
  # Part of public contract
end

# Struct field visibility
pub struct Config
  pub host: String              # Public field
  pub(package) port: Int        # Package-visible field
  secret: String                # Private field (default)
end
```

**Rationale**:
- Rust's five levels (`pub`, `pub(crate)`, `pub(super)`, `pub(in path)`, private) proved that `pub(super)` and `pub(in path)` cover less than 1% of use cases
- Two levels (like Go) lacks the critical "internal but shared" capability
- Three levels matches developer intuition from Java's package-private
- Private by default prevents accidental API exposure

### 2.2 Visibility Grammar (EBNF)

**DECIDED**: Update GRAMMAR.md Section 2 visibility rule.

```ebnf
(* Replaces: visibility = [ 'pub' | 'priv' ] ; *)
visibility      = 'pub'
                | 'pub' '(' 'package' ')'
                | (* empty = private *) ;
```

**Note**: The `priv` keyword from current GRAMMAR.md is **removed** - private is the default and needs no keyword.

### 2.3 Visibility Inheritance Rules

| Item | Default | Inheritance |
|------|---------|-------------|
| Module | Private | Explicit declaration |
| Function | Private | Explicit declaration |
| Struct | Private | Explicit declaration |
| Struct field | Private | Cannot exceed struct visibility |
| Enum | Private | Explicit declaration |
| Enum variant | Same as enum | Cannot be more restrictive |
| Trait | Private | Explicit declaration |
| Constant | Private | Explicit declaration |
| Type alias | Private | Explicit declaration |

---

## 3. File-to-Module Mapping

### 3.1 Decision: 1:1 Mapping with `mod.aria` Convention

**DECIDED**: Each `.aria` file maps to exactly one module. Directories use `mod.aria` as the parent module.

```
my_project/
  aria.toml                    # Project manifest
  src/
    main.aria                  # Entry point -> Main module
    lib.aria                   # Library root -> <PackageName> module
    config.aria                # Config module
    models/
      mod.aria                 # Models module (parent)
      user.aria                # Models::User submodule
      post.aria                # Models::Post submodule
    services/
      mod.aria                 # Services module (parent)
      auth/
        mod.aria               # Services::Auth module
        oauth.aria             # Services::Auth::OAuth submodule
        basic.aria             # Services::Auth::Basic submodule
  tests/
    user_test.aria             # Test module
  examples/
    basic.aria                 # Example program
```

### 3.2 Module Path Resolution Table

| File Path | Module Path |
|-----------|-------------|
| `src/main.aria` | `Main` |
| `src/lib.aria` | `<PackageName>` (from aria.toml) |
| `src/utils.aria` | `Utils` |
| `src/models/mod.aria` | `Models` |
| `src/models/user.aria` | `Models::User` |
| `src/models/user/mod.aria` | `Models::User` (if has submodules) |
| `src/services/auth/oauth.aria` | `Services::Auth::OAuth` |

### 3.3 Submodule Declaration Syntax

**DECIDED**: Parent modules explicitly declare submodules using `mod` keyword.

```aria
# File: src/models/mod.aria
module Models

# Declare submodules (loads from files)
pub mod user      # Loads models/user.aria, exports publicly
pub mod post      # Loads models/post.aria, exports publicly
mod internal      # Private submodule, not visible outside Models

# Re-exports for convenience (see Section 4)
pub use user::User
pub use user::create_user
pub use post::Post
```

### 3.4 Submodule Declaration Grammar (EBNF)

```ebnf
submod_decl     = visibility 'mod' identifier ;
```

---

## 4. Re-export Syntax

### 4.1 Decision: `pub use` for Re-exports

**DECIDED**: Use `pub use` to re-export items from other modules.

```aria
# File: src/lib.aria
module MyAwesomeLib

# Private implementation modules (not visible to users)
mod internal
mod impl_details

# Public submodules (if needed)
pub mod advanced

# Re-export the public API
pub use internal::core::{Engine, Config}
pub use internal::io::{read_file, write_file}
pub use impl_details::helpers::format

# Users see clean API:
#   MyAwesomeLib::Engine
#   MyAwesomeLib::Config
#   MyAwesomeLib::read_file
#   MyAwesomeLib::write_file
#   MyAwesomeLib::format
#   MyAwesomeLib::advanced::*
```

### 4.2 Re-export Visibility Rules

| Declaration | Effect |
|-------------|--------|
| `pub use X::Y` | Y is now public |
| `pub(package) use X::Y` | Y is package-visible |
| `use X::Y` | Y is a private alias (not a re-export) |

```aria
# Re-export visibility is determined by the visibility modifier
pub use internal::Public              # Now public
pub(package) use internal::SemiPublic # Package-visible
use internal::Private                 # Private alias (not exported)

# Cannot increase visibility of private items
# Error: cannot re-export private item
# pub use internal::private_fn        # COMPILE ERROR
```

### 4.3 Re-export Grammar (EBNF)

```ebnf
reexport_decl   = visibility 'use' reexport_spec ;

reexport_spec   = module_path [ '::' import_selection ] ;
```

### 4.4 Selective Re-exports

```aria
module MyLib

# Re-export specific items with renaming
pub use std::collections::HashMap as Map
pub use std::collections::HashSet as Set

# Re-export with grouping
pub use internal::{
  UserError as Error,
  Result,
  Config
}

# Bulk re-export (use sparingly, emits warning)
pub use utils::*
```

---

## 5. Circular Dependency Handling

### 5.1 Decision: Prohibit Circular Dependencies

**DECIDED**: Aria **prohibits circular module dependencies** at compile time.

**Rationale**:
- Enables parallel compilation
- Forces better architecture
- Eliminates initialization order bugs
- Makes dependency graph easy to reason about
- Proven successful in Rust, Go, Haskell

### 5.2 Detection Algorithm: Tarjan's SCC

**DECIDED**: Use Tarjan's Strongly Connected Components algorithm for cycle detection.

The compiler performs these steps:
1. Parse all files to identify module declarations
2. Build directed graph from import statements
3. Run Tarjan's SCC algorithm
4. Any SCC with size > 1 is a cycle (error)
5. Topologically sort remaining DAG for compilation order

### 5.3 Error Message Format

**DECIDED**: Provide rich error messages with visualization and resolution suggestions.

```
error[E0391]: cyclic dependency detected

  --> src/models/user.aria:3:1
   |
 3 | import Services::Auth
   | ^^^^^^^^^^^^^^^^^^^^^ imports Services::Auth
   |

  --> src/services/auth.aria:4:1
   |
 4 | import Models::User
   | ^^^^^^^^^^^^^^^^^^^ imports Models::User

Dependency cycle visualization:

    Models::User
         |
         v
    Services::Auth
         |
         +-------> Models::User (CYCLE!)

help: Break the cycle by extracting shared types:

  1. Create a new module for shared types:

     # File: src/common/types.aria
     module Common::Types

     pub struct UserId(Int)
     pub trait Authenticatable
       fn get_credentials(self) -> (String, String)
     end

  2. Have both modules import from Common::Types

For more information, see: https://aria-lang.org/errors/E0391
```

### 5.4 Cycle Resolution Strategies

**DECIDED**: Document three recommended strategies for breaking cycles.

#### Strategy 1: Extract Common Types

```aria
# BEFORE: User imports Auth, Auth imports User (CYCLE!)

# AFTER: Both import Common (no cycle)

# File: src/common/types.aria
module Common::Types

pub struct UserId(Int)
pub struct AuthToken(String)

# File: src/models/user.aria
import Common::Types::UserId

pub struct User
  id: UserId
  name: String
end

# File: src/services/auth.aria
import Common::Types::{UserId, AuthToken}

pub fn authenticate(id: UserId) -> AuthToken?
  # ...
end
```

#### Strategy 2: Dependency Injection via Traits

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

#### Strategy 3: Merge Tightly Coupled Modules

If two modules are truly interdependent, they may belong together:

```aria
# If User and Auth are inseparable, merge them:

# File: src/auth/mod.aria
module Auth

pub struct User
  id: UserId
  email: String
end

pub struct AuthToken(String)

pub fn login(user: User) -> AuthToken?
  # ...
end
```

### 5.5 Decision: No Forward Declarations

**DECIDED**: Aria does NOT support forward declarations.

**Rationale**:
- Forward declarations add complexity to the type system
- They are a workaround for poor architecture
- Dependency injection via traits is cleaner
- Extract-common-types pattern is more explicit

---

## 6. Glob Imports

### 6.1 Decision: Allowed with Warnings

**DECIDED**: Glob imports (`*`) are allowed but emit mandatory linter warnings.

```aria
# This works but emits warning:
# warning[W0103]: glob import may pollute namespace
import std::prelude::*

# Silence warning with attribute
@allow(glob_import)
import std::prelude::*
```

### 6.2 Glob Import Rules

| Rule | Behavior |
|------|----------|
| Warning | Always emits unless `@allow(glob_import)` |
| Scope | Only imports `pub` items |
| Precedence | Lower than explicit imports |
| Re-export | Does NOT re-export (must be explicit) |
| Shadowing | Can be shadowed by explicit imports |

### 6.3 When Glob Imports are Acceptable

**DECIDED**: The following are legitimate uses of glob imports:

1. **Prelude modules**: `import std::prelude::*`
2. **Test helpers**: `import test::helpers::*` (in test files)
3. **Trait method imports**: `import MyTrait::*` (for extension methods)
4. **DSL modules**: `import dsl::html::*` (for domain-specific languages)

---

## 7. Name Resolution

### 7.1 Decision: Two-Phase Resolution with Clear Precedence

**DECIDED**: Name resolution follows this precedence order (highest to lowest):

```
When resolving identifier `foo`:

1. Local scope (innermost to outermost)
   - Function parameters
   - Let bindings
   - Loop variables

2. Current module definitions
   - Functions, types, constants in this module

3. Explicitly imported names
   - From `import X::{foo}` statements

4. Glob-imported names
   - From `import X::*` statements

5. Prelude (automatic imports)
   - std::prelude::*

Precedence: Local > Module > Explicit Import > Glob Import > Prelude
```

### 7.2 Shadowing Rules

**DECIDED**: The following shadowing behaviors apply:

| Scenario | Behavior |
|----------|----------|
| Shadow local variable | No warning |
| Shadow import | Warning |
| Shadow prelude item | Strong warning |
| Shadow keyword | Compile error |

```aria
import std::collections::Array

# Shadowing import with local definition (warning)
type Array = [Int; 10]        # warning: shadows std::collections::Array

fn example(Array: Type)       # Parameter shadows type alias (warning)
  let Array = 42              # Variable shadows parameter (no warning)
  Array                       # Refers to innermost (Int value)
end
```

### 7.3 Ambiguity Handling

**DECIDED**: Ambiguous imports are compile errors with helpful suggestions.

```aria
import pkg_a::Helper
import pkg_b::Helper          # Conflict!

# Error message:
# error[E0252]: name `Helper` defined multiple times
#   --> src/main.aria:2:1
#    |
#  1 | import pkg_a::Helper
#    |        ------------- first imported here
#  2 | import pkg_b::Helper
#    | ^^^^^^^^^^^^^^^^^^^^^ `Helper` reimported here
#    |
# help: use `as` to rename one of the imports:
#    |
#  2 | import pkg_b::Helper as HelperB
#    |                      ^^^^^^^^^^
```

### 7.4 Qualified vs Unqualified Access

```aria
import std::collections::HashMap

# Unqualified access (after import)
let map: HashMap<String, Int> = HashMap.new()

# Fully qualified (always works, no import needed)
let map: std::collections::HashMap<String, Int> = ...

# Qualified through module alias
import std::collections as C
let map: C::HashMap<String, Int> = C::HashMap.new()
```

---

## 8. Prelude Contents

### 8.1 Decision: Minimal Essential Prelude

**DECIDED**: The prelude contains only universally-needed types, traits, and functions.

```aria
# Automatically imported into every Aria program:

# === Primitive Types ===
Int, Int8, Int16, Int32, Int64, Int128
UInt, UInt8, UInt16, UInt32, UInt64, UInt128
Float, Float32, Float64
Bool, Char, String, Bytes

# === Collection Types ===
Array, Map, Set, Tuple

# === Result Types ===
Option, Some, None
Result, Ok, Err
Never, Unit

# === Core Traits ===
Eq, Ord, Hash, Clone, Copy, Default
Debug, Display
Add, Sub, Mul, Div, Rem, Neg
Iterator, IntoIterator, FromIterator
Into, From, TryInto, TryFrom

# === Core Functions ===
print, println, debug, panic
assert, assert_eq, assert_ne

# === Macros ===
todo!, unimplemented!, unreachable!
```

### 8.2 Prelude Customization

**DECIDED**: Custom preludes can be defined per-package in `aria.toml`.

```toml
# aria.toml
[package]
name = "my_project"

[prelude]
# Add to default prelude
include = ["MyLib::Extensions::*"]

# Exclude from default prelude
exclude = ["print", "println"]  # Use custom logging instead
```

---

## 9. Complete Grammar Updates

### 9.1 Updated Module Declaration (Section 14.1)

```ebnf
(* Complete module declaration *)
module_decl     = 'module' module_path ;

module_path     = identifier { '::' identifier } ;
```

### 9.2 Updated Import Declaration (Section 14.2)

```ebnf
(* Complete import grammar *)
import_decl     = 'import' import_spec ;

import_spec     = import_source [ import_modifier ] ;

import_source   = module_path
                | string_literal ;

module_path     = path_segment { '::' path_segment } ;

path_segment    = identifier
                | 'self'
                | 'super' ;

import_modifier = '::' import_selection
                | 'as' identifier ;

import_selection = '*'
                 | '{' import_list '}' ;

import_list     = import_item { ',' import_item } [ ',' ] ;

import_item     = identifier [ 'as' identifier ]
                | nested_import ;

nested_import   = identifier '::' import_selection ;
```

### 9.3 New Submodule Declaration

```ebnf
(* Submodule declaration *)
submod_decl     = visibility 'mod' identifier ;
```

### 9.4 Updated Re-export Declaration (Section 14.3)

```ebnf
(* Re-export declaration *)
reexport_decl   = visibility 'use' reexport_spec ;

reexport_spec   = module_path [ '::' import_selection ] ;
```

### 9.5 Updated Visibility (Section 2)

```ebnf
(* Three-tier visibility model *)
visibility      = 'pub'
                | 'pub' '(' 'package' ')'
                | (* empty = private *) ;
```

### 9.6 Complete Module Grammar Summary

```ebnf
(* Full module system grammar *)

program         = { module_item } ;

module_item     = module_decl
                | import_decl
                | reexport_decl
                | submod_decl
                | visibility item_decl ;

module_decl     = 'module' module_path ;

import_decl     = 'import' import_spec ;

import_spec     = import_source [ import_modifier ] ;

import_source   = module_path | string_literal ;

module_path     = path_segment { '::' path_segment } ;

path_segment    = identifier | 'self' | 'super' ;

import_modifier = '::' import_selection | 'as' identifier ;

import_selection = '*' | '{' import_list '}' ;

import_list     = import_item { ',' import_item } [ ',' ] ;

import_item     = identifier [ 'as' identifier ] | nested_import ;

nested_import   = identifier '::' import_selection ;

reexport_decl   = visibility 'use' reexport_spec ;

reexport_spec   = module_path [ '::' import_selection ] ;

submod_decl     = visibility 'mod' identifier ;

visibility      = 'pub' | 'pub' '(' 'package' ')' | (* empty *) ;
```

---

## 10. Directory Layout Examples

### 10.1 Simple Application

```
hello_world/
  aria.toml
  src/
    main.aria                  # fn main entry point
```

```aria
# src/main.aria
module Main

fn main
  println("Hello, World!")
end
```

### 10.2 Library with Multiple Modules

```
my_lib/
  aria.toml
  src/
    lib.aria                   # Library entry, re-exports
    config.aria
    utils.aria
    models/
      mod.aria
      user.aria
      post.aria
```

```aria
# src/lib.aria
module MyLib

pub mod config
pub mod utils
pub mod models

# Convenience re-exports
pub use models::{User, Post}
pub use config::Config
```

```aria
# src/models/mod.aria
module MyLib::Models

pub mod user
pub mod post

pub use user::User
pub use post::Post
```

```aria
# src/models/user.aria
module MyLib::Models::User

import super::post::Post

pub struct User
  pub name: String
  pub email: String
  posts: Array<Post>
end

pub fn create_user(name: String, email: String) -> User
  User(name:, email:, posts: [])
end
```

### 10.3 Application with Private Modules

```
web_app/
  aria.toml
  src/
    main.aria
    routes.aria
    handlers/
      mod.aria
      auth.aria
      api.aria
    internal/
      mod.aria
      database.aria
      cache.aria
```

```aria
# src/main.aria
module Main

import self::routes
import self::handlers

fn main
  server = Server.new(port: 8080)
  routes.configure(server)
  server.run()
end
```

```aria
# src/handlers/mod.aria
module Main::Handlers

pub mod auth
pub mod api

# These handlers use internal modules
mod internal_helpers
```

```aria
# src/internal/mod.aria
module Main::Internal

# These are package-visible, not public
pub(package) mod database
pub(package) mod cache
```

### 10.4 Workspace with Multiple Packages

```
workspace/
  aria.toml                    # Workspace manifest
  packages/
    core/
      aria.toml
      src/
        lib.aria
        types.aria
    server/
      aria.toml
      src/
        main.aria
    client/
      aria.toml
      src/
        lib.aria
```

```toml
# workspace/aria.toml
[workspace]
members = ["packages/core", "packages/server", "packages/client"]
```

---

## 11. Effect Visibility in Modules

### 11.1 Decision: Effects Are Always Visible Unless Handled

**DECIDED**: Effects propagate through module boundaries and are visible in function signatures unless explicitly handled.

```aria
module DataService

# Effects visible in signature
pub fn fetch_user(id: Int) -> {IO, Http.Error} User
  response = Http.get("/users/#{id}")
  parse_user(response.body)
end

# Effects handled internally - signature is clean
pub fn fetch_user_safe(id: Int) -> User?
  handle fetch_user(id)
    on Http.Error(_) => None
    on IO.Error(_) => None
    on success(user) => Some(user)
  end
end
```

### 11.2 Effect Re-exports

```aria
module MyLib

# Re-export effects like types
pub use internal::errors::NetworkError
pub use internal::effects::Async

# Effect type aliases
pub effect IOError = std::io::Error
```

---

## 12. Implementation Requirements

### 12.1 Compiler Phases

| Phase | Description |
|-------|-------------|
| 1. Lexing/Parsing | Recognize module syntax |
| 2. Module Discovery | Find all `.aria` files in project |
| 3. Graph Construction | Build dependency DAG from imports |
| 4. Cycle Detection | Tarjan's SCC algorithm |
| 5. Topological Sort | Determine compilation order |
| 6. Name Resolution | Two-phase resolution per module |
| 7. Type Checking | With module-aware visibility |
| 8. Effect Inference | Propagate effects through call graph |
| 9. Code Generation | Per-module compilation units |

### 12.2 Incremental Compilation Impact

| Change Type | Recompilation Required |
|-------------|----------------------|
| Module body change | Module + dependents |
| Import added | Check for cycles, recheck names |
| Visibility change | Recheck all importers |
| Public type change | Module + all dependents |
| Private implementation | Module only |
| Effect signature change | Module + all callers |

### 12.3 IDE/LSP Support Requirements

| Feature | Implementation |
|---------|---------------|
| Go to definition | Follow module paths to source files |
| Find references | Index all imports and uses |
| Rename | Update across visibility boundaries |
| Completion | Show only visible items |
| Hover | Display effect signatures |
| Diagnostics | Real-time cycle detection |

---

## 13. Decision Summary

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Import keyword | `import` | Intuitive for most developers |
| Path separator | `::` | Distinguishes from member access |
| Visibility levels | 3 (private, pub(package), pub) | Covers 99% of use cases |
| Default visibility | Private | Prevents accidental exposure |
| File mapping | 1:1 with mod.aria | Predictable, IDE-friendly |
| Circular deps | Prohibited | Better architecture, parallel compile |
| Detection algorithm | Tarjan's SCC | Efficient, proven |
| Glob imports | Allowed with warning | Flexibility with guardrails |
| Name precedence | Local > Import > Glob > Prelude | Clear, unsurprising |
| Re-export syntax | `pub use` | Rust-proven, explicit |

---

## 14. Migration from Current Grammar

### 14.1 Changes Required

| Current GRAMMAR.md | New Decision | Action |
|--------------------|--------------|--------|
| `visibility = ['pub' \| 'priv']` | `visibility = 'pub' \| 'pub(package)' \| empty` | Remove `priv`, add `pub(package)` |
| Basic import syntax | Rich import with `::` paths | Extend grammar |
| No submodule declaration | `mod` keyword | Add new production |
| No re-export | `pub use` | Add new production |

### 14.2 Backwards Compatibility

- `priv` keyword deprecated (warning for 2 releases, then error)
- Existing import syntax continues to work
- New features are additive

---

## 15. Open Decisions (Deferred)

| Question | Deferred To | Rationale |
|----------|-------------|-----------|
| Conditional compilation in modules | ARIA-PD-020 | Separate concern |
| Macro visibility rules | ARIA-PD-021 | Macro system not yet designed |
| Hot module reloading | Future | Advanced runtime feature |
| ABI stability versioning | Future | Needs real-world usage data |

---

*Product Design Document approved by MAPPER, January 2026*
*Based on NEXUS-II research document ARIA-M07-04*
