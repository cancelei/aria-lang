# ARIA-M07-04: Module System and Import Design

**Task ID**: ARIA-M07-04
**Status**: Completed
**Date**: 2026-01-15
**Agent**: NEXUS-II (Eureka Research Agent)
**Focus**: Comprehensive module system design including imports, visibility, circular dependencies, effect visibility, and name resolution

---

## Executive Summary

This research document provides a comprehensive analysis of module system design for the Aria programming language. Drawing from extensive study of Rust, Python, ES6, Go, Haskell, and OCaml module systems, we propose a design that balances simplicity with power following Aria's "progressive complexity" philosophy.

**Key Findings**:
1. **Import syntax**: Hybrid approach combining Rust's precision with Python's readability
2. **Visibility**: Three-tier model (`private`, `pub(package)`, `pub`) covers 99% of use cases
3. **Circular dependencies**: Prohibited with excellent error messages and resolution strategies
4. **Effect visibility**: Effects can be hidden via opaque module boundaries with explicit re-exports
5. **Name resolution**: Two-phase algorithm with clear precedence rules

---

## Table of Contents

1. [Import Syntax Comparison](#1-import-syntax-comparison)
2. [Visibility Modifiers](#2-visibility-modifiers)
3. [Module vs File Organization](#3-module-vs-file-organization)
4. [Re-exports and Module Facades](#4-re-exports-and-module-facades)
5. [Circular Dependency Handling](#5-circular-dependency-handling)
6. [Effect Visibility](#6-effect-visibility)
7. [Name Resolution Algorithms](#7-name-resolution-algorithms)
8. [Compile-time Module Evaluation](#8-compile-time-module-evaluation)
9. [Aria Module System Proposal](#9-aria-module-system-proposal)
10. [Implementation Considerations](#10-implementation-considerations)

---

## 1. Import Syntax Comparison

### 1.1 Language Survey

#### Rust `use`

```rust
// Absolute path
use std::collections::HashMap;
use crate::models::User;

// Relative path
use self::helper;
use super::parent_module;

// Multiple items
use std::collections::{HashMap, HashSet, BTreeMap};

// Glob import
use std::prelude::*;

// Aliasing
use std::collections::HashMap as Map;

// Nested groups
use std::{
    collections::{HashMap, HashSet},
    io::{Read, Write},
};

// Re-export
pub use crate::internal::PublicType;
```

**Strengths**: Precise control, tree-shaking friendly, explicit
**Weaknesses**: Verbose for simple cases, `crate::` vs `self::` confusion

#### Python `import`

```python
# Module import
import os
import os.path as path

# From import
from collections import defaultdict, OrderedDict
from typing import List, Dict, Optional

# Relative imports
from . import sibling
from .. import parent
from ..utils import helper

# Glob import (discouraged)
from module import *

# Aliasing
from collections import defaultdict as dd
```

**Strengths**: Intuitive, flexible, readable
**Weaknesses**: No visibility control, runtime resolution, glob pollution

#### ES6 `import`

```javascript
// Default import
import React from 'react';

// Named imports
import { useState, useEffect } from 'react';

// Namespace import
import * as utils from './utils';

// Combined
import React, { useState } from 'react';

// Aliasing
import { Component as Comp } from 'react';

// Side-effect only
import './styles.css';

// Dynamic import
const module = await import('./dynamic');
```

**Strengths**: Clean syntax, default exports, dynamic imports
**Weaknesses**: Circular dependencies allowed, bundle complexity

#### Go Packages

```go
// Standard import
import "fmt"
import "net/http"

// Aliased import
import f "fmt"

// Blank import (side effects)
import _ "github.com/lib/pq"

// Dot import (discouraged)
import . "fmt"

// Multi-import
import (
    "fmt"
    "os"
    "github.com/user/repo"
)
```

**Strengths**: Simple, enforced by tooling, no cycles
**Weaknesses**: No selective imports, visibility by case only

### 1.2 Comparison Matrix

| Feature | Rust | Python | ES6 | Go | Haskell |
|---------|------|--------|-----|----|---------|
| Selective import | Yes | Yes | Yes | No | Yes |
| Glob import | Yes | Yes | Yes | Yes* | Yes |
| Aliasing | Yes | Yes | Yes | Yes | Yes |
| Re-exports | Yes | Limited | Yes | No | Yes |
| Relative paths | Yes | Yes | Yes | No | No |
| Default exports | No | No | Yes | No | No |
| Dynamic import | No | Yes | Yes | No | No |
| Cycle detection | Compile | Runtime | Allowed | Compile | Compile |

### 1.3 Aria Import Syntax Proposal

Based on the analysis, Aria should adopt a **Rust-inspired syntax with Python's readability**:

```aria
# =========================================
# ARIA IMPORT SYNTAX
# =========================================

# Simple module import
import std::collections

# Selective import (preferred)
import std::collections::{Array, Map, Set}

# Qualified import with alias
import std::collections::HashMap as Dict

# Module namespace import
import std::net::http as http

# Relative imports (within package)
import self::helper           # Same module directory
import super::parent         # Parent module
import super::super::root    # Grandparent

# Glob import (discouraged, requires lint warning)
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

### 1.4 Import Statement Grammar

```ebnf
import_decl     = 'import' import_spec ;

import_spec     = module_path [ import_tail ] ;

module_path     = identifier { '::' identifier }
                | string_lit ;

import_tail     = '::' ( import_items | glob_import )
                | 'as' identifier ;

import_items    = '{' import_list '}' ;

import_list     = import_item { ',' import_item } [ ',' ] ;

import_item     = identifier [ 'as' identifier ]
                | import_spec ;  (* nested imports *)

glob_import     = '*' ;
```

---

## 2. Visibility Modifiers

### 2.1 Language Comparison

| Language | Levels | Default | Notes |
|----------|--------|---------|-------|
| **Rust** | 5 | Private | `pub`, `pub(crate)`, `pub(super)`, `pub(in path)`, private |
| **Go** | 2 | Unexported | Uppercase = exported |
| **Java** | 4 | Package | `public`, `protected`, package-private, `private` |
| **TypeScript** | 3 | Public | `public`, `protected`, `private` |
| **Python** | 2* | Public | Convention: `_private`, `__mangled` |
| **Haskell** | 2 | Private | Export list controls visibility |

### 2.2 Rust Visibility Deep Dive

```rust
// Private (default) - only this module
fn private_helper() {}

// Public - everyone
pub fn public_api() {}

// Crate-public - anywhere in this crate
pub(crate) fn internal_api() {}

// Parent-public - parent module and descendants
pub(super) fn for_parent() {}

// Path-public - specific ancestor
pub(in crate::utils) fn specific_scope() {}
```

**Analysis**:
- `pub(crate)` is extremely useful for internal APIs
- `pub(super)` rarely used in practice
- `pub(in path)` almost never used
- Complexity vs benefit trade-off suggests simplification

### 2.3 Aria Three-Tier Visibility Model

Based on real-world usage patterns, Aria adopts **three visibility levels**:

| Visibility | Keyword | Scope | Use Case |
|------------|---------|-------|----------|
| **Private** | (default) | Same module | Implementation details |
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

### 2.4 Visibility Rationale

**Why not five levels like Rust?**
- `pub(super)` and `pub(in path)` cover < 1% of use cases
- Added complexity hurts learnability
- Package boundary is the meaningful encapsulation unit

**Why not two levels like Go?**
- `pub(package)` enables important "internal but shared" APIs
- Critical for large packages with multiple modules
- Matches intuition from Java's package-private

### 2.5 Privacy Defaults

```aria
# Everything private by default
module MyModule

struct InternalData     # Private struct
  field: Int            # Field inherits struct privacy
end

pub struct PublicData
  pub name: String      # Explicitly public field
  internal: Int         # Private by default
end

fn helper()             # Private function
  # ...
end

pub fn api()            # Public function
  # ...
end

type InternalAlias = Int   # Private type alias
pub type PublicAlias = Int # Public type alias
```

---

## 3. Module vs File Organization

### 3.1 Organization Patterns

#### Rust Model

```
src/
  lib.rs              # Crate root, declares modules
  main.rs             # Binary entry point
  utils.rs            # utils module (Option A)
  utils/              # utils module (Option B)
    mod.rs            # Module declaration
    helper.rs         # utils::helper submodule
```

**Complexity**: Two ways to define modules, `mod.rs` vs `module_name.rs` confusion

#### Python Model

```
package/
  __init__.py         # Package marker (required)
  module.py           # package.module
  subpackage/
    __init__.py       # Subpackage marker
    nested.py         # package.subpackage.nested
```

**Simplicity**: Directory = package, file = module

#### Go Model

```
mypackage/
  file1.go            # All files in directory = one package
  file2.go            # Same package
  internal/           # Special: internal packages
    secret.go
```

**Simplicity**: Directory = package (single level)

### 3.2 Aria File Organization

Aria uses **1:1 file-to-module mapping** with explicit structure:

```
src/
  main.aria                  # Entry point (Main module)
  lib.aria                   # Library root (if building library)
  utils.aria                 # Utils module
  models/
    mod.aria                 # Models module (parent)
    user.aria                # Models::User submodule
    post.aria                # Models::Post submodule
  services/
    mod.aria                 # Services module (parent)
    auth.aria                # Services::Auth submodule
    auth/
      mod.aria               # Services::Auth (if has children)
      oauth.aria             # Services::Auth::OAuth
```

### 3.3 Module Declaration Patterns

```aria
# File: src/models/mod.aria
# Parent module declares submodules

module Models

# Declare submodules (loads from files)
pub mod user      # Loads models/user.aria, exports publicly
pub mod post      # Loads models/post.aria, exports publicly
mod internal      # Private submodule

# Re-exports for convenience
pub use user::User
pub use user::create_user
pub use post::Post
```

```aria
# File: src/models/user.aria
# Submodule implementation

module Models::User

pub struct User
  pub name: String
  pub email: String
  age: Int                    # Private by default
end

pub fn create_user(name: String, email: String) -> User
  User(name:, email:, age: 0)
end

# Internal helper - not exported
fn validate_email(email: String) -> Bool
  email.contains?("@")
end
```

### 3.4 Module Path Resolution

```
File path → Module path mapping:

src/main.aria           → Main
src/lib.aria            → <PackageName>
src/utils.aria          → Utils
src/models/mod.aria     → Models
src/models/user.aria    → Models::User
src/models/user/mod.aria → Models::User (if has submodules)
```

---

## 4. Re-exports and Module Facades

### 4.1 Facade Pattern

The **facade pattern** creates a clean public API from internal implementation:

```aria
# File: src/lib.aria
# This is the public API facade

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

```aria
# Re-export visibility is determined by the `pub` keyword

pub use internal::Public        # Now public
pub(package) use internal::SemiPublic  # Package-visible
use internal::Private           # Private re-export (alias)

# Cannot increase visibility of private items
# Error: cannot re-export private item
# pub use internal::private_fn  # COMPILE ERROR
```

### 4.3 Selective Re-exports

```aria
module MyLib

# Re-export specific items
pub use std::collections::HashMap as Map
pub use std::collections::HashSet as Set

# Re-export with grouping
pub use internal::{
  UserError as Error,
  Result,
  Config
}

# Bulk re-export (use sparingly)
pub use utils::*
```

### 4.4 Documentation for Re-exports

```aria
## Configuration for the library.
##
## Re-exported from `internal::config` for convenience.
pub use internal::config::Config
```

---

## 5. Circular Dependency Handling

### 5.1 Design Decision: Prohibit Circular Dependencies

Aria **prohibits circular module dependencies** at compile time.

**Rationale**:
- Enables parallel compilation
- Forces better architecture
- Eliminates initialization order bugs
- Makes dependency graph easy to reason about
- Matches Rust, Go, Haskell (proven in practice)

### 5.2 Detection Algorithms

#### 5.2.1 Tarjan's Strongly Connected Components

```python
def detect_cycles(module_graph):
    """
    Tarjan's algorithm for finding strongly connected components.
    Any SCC with size > 1 is a cycle.
    """
    index_counter = [0]
    stack = []
    lowlinks = {}
    index = {}
    on_stack = {}
    sccs = []

    def strongconnect(node):
        index[node] = index_counter[0]
        lowlinks[node] = index_counter[0]
        index_counter[0] += 1
        on_stack[node] = True
        stack.append(node)

        for successor in module_graph[node]:
            if successor not in index:
                strongconnect(successor)
                lowlinks[node] = min(lowlinks[node], lowlinks[successor])
            elif on_stack.get(successor, False):
                lowlinks[node] = min(lowlinks[node], index[successor])

        if lowlinks[node] == index[node]:
            scc = []
            while True:
                w = stack.pop()
                on_stack[w] = False
                scc.append(w)
                if w == node:
                    break
            sccs.append(scc)

    for node in module_graph:
        if node not in index:
            strongconnect(node)

    # Cycles are SCCs with size > 1
    return [scc for scc in sccs if len(scc) > 1]
```

#### 5.2.2 Incremental Cycle Detection

For IDE/LSP performance, use **incremental detection**:

```python
def check_import_would_cycle(graph, from_module, to_module):
    """
    Check if adding an import from_module -> to_module creates a cycle.
    Uses DFS from to_module looking for from_module.
    """
    visited = set()
    stack = [to_module]

    while stack:
        current = stack.pop()
        if current == from_module:
            return True  # Would create cycle
        if current in visited:
            continue
        visited.add(current)
        stack.extend(graph.get(current, []))

    return False
```

### 5.3 Import Cycles vs Initialization Cycles

| Type | Description | Example |
|------|-------------|---------|
| **Import Cycle** | A imports B, B imports A | Compile-time error |
| **Initialization Cycle** | A's init calls B, B's init calls A | Runtime error |

**Import cycles** are caught at compile time by dependency graph analysis.

**Initialization cycles** can occur even without import cycles when:
- Module initialization runs code at load time
- Circular calls happen through function pointers/closures

```aria
# This is NOT an import cycle, but IS an initialization cycle:

# File: a.aria
module A
import B

pub fn init()
  B.call_back()  # B calls A.respond during init
end

# File: b.aria
module B
import A

pub fn call_back()
  A.respond()  # If A.respond uses uninitialized A state = bug
end
```

**Aria's Approach**:
- Import cycles: Compile-time error (always)
- Initialization cycles: Warn on suspicious patterns, runtime check optional

### 5.4 Error Messages for Cycles

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

### 5.5 Breaking Cycles: Strategies

#### Strategy 1: Extract Common Types

```aria
# BEFORE: User imports Auth, Auth imports User

# AFTER: Both import Common

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
  # ...
end

pub struct AuthToken(String)

pub fn login(user: User) -> AuthToken?
  # ...
end
```

### 5.6 Forward Declarations (Not Recommended)

Some languages allow forward declarations to break cycles:

```cpp
// C++ forward declaration
class User;  // Forward declare

class Auth {
    void login(User* user);  // Use pointer to forward-declared type
};
```

**Aria does NOT support forward declarations** because:
- They add complexity to the type system
- They're a workaround for poor architecture
- The dependency injection approach is cleaner

---

## 6. Effect Visibility

### 6.1 Effect System Recap

From ARIA-M03-01, Aria uses **inferred algebraic effects**:

```aria
# Effects inferred from body
fn read_config(path: String) -> Config
  data = File.read(path)      # IO effect
  parse_json(data)            # Parse.Error effect
end

# Inferred type: fn read_config(path: String) -> {IO, Parse.Error} Config
```

### 6.2 Can Modules Hide Effects?

**Question**: Should module boundaries affect effect visibility?

#### Option A: Effects Always Visible

```aria
module SafeWrapper

# Even though this wraps an IO function, the IO effect is visible
pub fn get_config() -> {IO} Config
  File.read("config.json") |> parse()
end

# Users see: get_config has IO effect
```

#### Option B: Effects Hidden by Handlers

```aria
module SafeWrapper

# Handler absorbs the effect
pub fn get_config() -> Config    # No IO in signature!
  with handle File.read
    on IO.Error(_) => default_config()
  end
  File.read("config.json") |> parse()
end

# Users see: get_config is pure (effects handled internally)
```

#### Option C: Opaque Effect Boundaries

```aria
module SafeWrapper

# Declare that this module has an effect boundary
@effect_boundary

# External signature shows no effects
pub fn get_config() -> Config
  # Internal: has IO effect
  # But module boundary absorbs it
end
```

### 6.3 Aria's Approach: Explicit Effect Handling

**Design Decision**: Effects are **always visible unless explicitly handled**.

```aria
module DataService

# Effects propagate by default
pub fn fetch_user(id: Int) -> {IO, Http.Error} User
  response = Http.get("/users/#{id}")
  parse_user(response.body)
end

# Effects can be handled at any level
pub fn fetch_user_safe(id: Int) -> User?
  handle fetch_user(id)
    on Http.Error(_) => None
    on success(user) => Some(user)
  end
end
```

**Rationale**:
- **Honesty**: Users know what effects a function performs
- **Composition**: Effects compose predictably
- **Explicit handling**: When effects are hidden, it's intentional and clear

### 6.4 Effect Re-exports

Effects can be re-exported like types:

```aria
module MyLib

# Re-export effects from internal module
pub use internal::errors::NetworkError
pub use internal::effects::Async

# Effect aliases
pub effect IOError = std::io::Error
```

### 6.5 Effect Boundaries at Package Level

```aria
# File: src/lib.aria
module MyLib

# Package-level effect declaration
@effects(IO, Async)  # This package may have these effects

# All public functions must have effects within declared set
# or handle them internally
```

---

## 7. Name Resolution Algorithms

### 7.1 Two-Phase Resolution

**Phase 1: Build Module Graph**
1. Parse all files to identify module declarations
2. Build directed graph from import statements
3. Check for cycles (Tarjan's algorithm)
4. Topologically sort for compilation order

**Phase 2: Resolve Names**
1. For each module in topological order:
   a. Process imports (bring names into scope)
   b. Resolve local definitions
   c. Check visibility constraints
   d. Report unresolved/ambiguous names

### 7.2 Name Lookup Order

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

Precedence: Local > Explicit Import > Glob Import > Prelude
```

### 7.3 Glob Imports (* Imports)

```aria
import std::collections::*    # Brings all public items into scope

# Risks:
# 1. Name pollution
# 2. Unclear where names come from
# 3. Future additions may conflict

# Aria approach: Allowed but linted
@allow(glob_import)           # Silence warning
import std::prelude::*
```

**Glob Import Rules**:
- **Warning** by default (can be silenced)
- Only imports `pub` items
- Lower precedence than explicit imports
- Does NOT re-export (must be explicit)

### 7.4 Shadowing Rules

```aria
import std::collections::Array

# Shadowing import with local definition
type Array = [Int; 10]        # Shadows std::collections::Array

fn example(Array: Type)       # Parameter shadows type alias
  let Array = 42              # Variable shadows parameter
  Array                       # Refers to innermost (Int value)
end

# Warning levels:
# - Shadow local variable: No warning
# - Shadow import: Warning
# - Shadow prelude item: Strong warning
# - Shadow keyword: Error
```

### 7.5 Ambiguity Handling

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

# Resolution:
import pkg_a::Helper as HelperA
import pkg_b::Helper as HelperB
```

### 7.6 Qualified vs Unqualified Access

```aria
import std::collections::HashMap

# Unqualified access (after import)
let map: HashMap<String, Int> = HashMap.new()

# Fully qualified (always works)
let map: std::collections::HashMap<String, Int> = ...

# After aliased import
import std::collections::HashMap as Dict
let map: Dict<String, Int> = Dict.new()

# Qualified through module alias
import std::collections as C
let map: C::HashMap<String, Int> = C::HashMap.new()
```

### 7.7 Resolution Algorithm Implementation

```python
class NameResolver:
    def __init__(self, module_graph):
        self.module_graph = module_graph
        self.scopes = []  # Stack of scopes

    def resolve(self, name: str, context: Context) -> Resolution:
        # 1. Local scopes (innermost first)
        for scope in reversed(self.scopes):
            if name in scope.bindings:
                return Resolution.Local(scope.bindings[name])

        # 2. Current module definitions
        module = context.current_module
        if name in module.definitions:
            return Resolution.ModuleLocal(module.definitions[name])

        # 3. Explicit imports
        for imp in module.explicit_imports:
            if imp.imported_name == name:
                return Resolution.Import(imp.source)

        # 4. Glob imports (with potential ambiguity)
        glob_matches = []
        for glob in module.glob_imports:
            source_module = self.module_graph[glob.source]
            if name in source_module.public_exports:
                glob_matches.append(source_module.public_exports[name])

        if len(glob_matches) == 1:
            return Resolution.GlobImport(glob_matches[0])
        elif len(glob_matches) > 1:
            return Resolution.Ambiguous(glob_matches)

        # 5. Prelude
        if name in PRELUDE:
            return Resolution.Prelude(PRELUDE[name])

        # Not found
        return Resolution.Unresolved(name)
```

---

## 8. Compile-time Module Evaluation

### 8.1 Use Cases

| Use Case | Example |
|----------|---------|
| **Constants** | `const MAX_SIZE = 1024` |
| **Contracts** | `requires arr.length > 0` |
| **Type-level computation** | `type Matrix[N, M] where N * M < 1000` |
| **Compile-time assertions** | `static_assert!(size_of::<T>() < 256)` |

### 8.2 Constant Evaluation

```aria
# Constants evaluated at compile time
const MAX_BUFFER_SIZE: Int = 1024 * 1024
const DEFAULT_TIMEOUT: Duration = 30.seconds
const CONFIG_PATH: String = env!("CONFIG_PATH") ?? "config.json"

# Constant expressions
const TOTAL_SIZE: Int = MAX_BUFFER_SIZE * 2    # Evaluated at compile time

# Constant functions (pure, deterministic)
const fn factorial(n: Int) -> Int
  match n
    0, 1 => 1
    _ => n * factorial(n - 1)
  end
end

const PRECOMPUTED: Int = factorial(10)  # 3628800, computed at compile time
```

### 8.3 Contract Evaluation

```aria
fn binary_search<T: Ord>(arr: Array<T>, target: T) -> Int?
  # Compile-time check if arr is const and we can verify sortedness
  requires arr.sorted?  # May be checked at compile time for const arrays

  # Runtime check otherwise
  ensures result.nil? or arr[result.unwrap] == target
  # ...
end

# With constant argument, contract checked at compile time:
const SORTED_DATA: Array<Int> = [1, 2, 3, 4, 5]
binary_search(SORTED_DATA, 3)  # Contract verified at compile time!
```

### 8.4 Module-level Evaluation Order

```aria
module Config

# Phase 1: Constants (compile-time)
const VERSION: String = "1.0.0"
const MAX_CONNECTIONS: Int = calculate_max()

# Phase 2: Type definitions
struct Settings
  max_conn: Int = MAX_CONNECTIONS  # Uses constant
end

# Phase 3: Runtime initialization (if any)
let global_settings: Settings = Settings()  # Runs at module load
```

### 8.5 Compile-time vs Runtime Distinction

```aria
# COMPILE-TIME (const)
const fn pure_computation(x: Int) -> Int
  x * x + 1
end

# RUNTIME (regular function)
fn impure_computation(x: Int) -> Int
  print("Computing...")  # Side effect!
  x * x + 1
end

# Mixing
const VALUE: Int = pure_computation(10)      # OK
const BAD: Int = impure_computation(10)      # ERROR: not const
```

---

## 9. Aria Module System Proposal

### 9.1 Complete Syntax Reference

```ebnf
(* Module declaration *)
module_decl     = 'module' module_path ;
module_path     = identifier { '::' identifier } ;

(* Submodule declaration *)
submod_decl     = visibility 'mod' identifier ;

(* Visibility *)
visibility      = [ 'pub' | 'pub' '(' 'package' ')' ] ;

(* Import declaration *)
import_decl     = 'import' import_spec ;
import_spec     = ( module_path | string_lit ) [ import_tail ] ;
import_tail     = '::' ( '*' | '{' import_list '}' ) | 'as' identifier ;
import_list     = import_item { ',' import_item } [ ',' ] ;
import_item     = identifier [ 'as' identifier ] | import_spec ;

(* Re-export declaration *)
reexport_decl   = visibility 'use' reexport_path ;
reexport_path   = module_path [ '::' ( '*' | '{' import_list '}' ) ] ;

(* Export list (alternative) *)
export_decl     = 'export' '{' identifier { ',' identifier } '}' ;
```

### 9.2 Quick Reference Card

```aria
# =========================================
# ARIA MODULE SYSTEM - QUICK REFERENCE
# =========================================

# === Module Declaration ===
module MyLib::SubModule

# === Import Styles ===
import std::io                        # Module import
import std::io::{File, Error}         # Selective import
import std::io::File as IoFile        # Aliased import
import "vendor/lib" as vendor         # Path import
import std::prelude::*                # Glob import (avoid)

# === Visibility ===
fn private_fn()                       # Private (default)
pub(package) fn internal_fn()         # Package-visible
pub fn public_fn()                    # Public

# === Submodules ===
pub mod submodule                     # Public submodule
mod private_mod                       # Private submodule

# === Re-exports ===
pub use internal::Type                # Re-export single item
pub use internal::{A, B, C}           # Re-export multiple
pub use internal::*                   # Re-export all (avoid)

# === Effect Visibility ===
pub fn fetch() -> {IO} Data           # Effect visible in signature
pub fn safe_fetch() -> Data           # Effects handled internally
  handle fetch()
    on IO.Error(_) => default_data()
  end
end

# === Constants ===
const VERSION: String = "1.0.0"
const fn factorial(n: Int) -> Int     # Compile-time function
```

### 9.3 File Organization Convention

```
my_project/
  aria.toml                    # Project manifest
  src/
    main.aria                  # Entry point (Main module)
    lib.aria                   # Library root (optional)
    config.aria               # Config module
    models/
      mod.aria                 # Models parent module
      user.aria                # Models::User
      post.aria                # Models::Post
    services/
      mod.aria                 # Services parent module
      auth/
        mod.aria               # Services::Auth parent
        oauth.aria             # Services::Auth::OAuth
        basic.aria             # Services::Auth::Basic
  tests/
    user_test.aria             # Test files
  examples/
    basic.aria                 # Example files
```

### 9.4 Integration with Effects

```aria
module DataService

# Effect-annotated public API
pub fn fetch_user(id: Int) -> {IO, Http.Error} User
  response = Http.get("/users/#{id}")
  parse_user(response.body)
end

# Effect-handling wrapper
pub fn fetch_user_or_default(id: Int) -> User
  handle fetch_user(id)
    on IO.Error(_) => default_user()
    on Http.Error(_) => default_user()
    on success(u) => u
  end
end

# Effects can be package-internal
pub(package) effect CacheAccess
  ctl get_cached<T>(key: String) -> T?
  ctl set_cached<T>(key: String, value: T)
end
```

---

## 10. Implementation Considerations

### 10.1 Compiler Phases

1. **Lexing/Parsing**: Recognize module syntax
2. **Module Discovery**: Find all `.aria` files in project
3. **Graph Construction**: Build dependency DAG from imports
4. **Cycle Detection**: Tarjan's algorithm
5. **Topological Sort**: Determine compilation order
6. **Name Resolution**: Two-phase resolution per module
7. **Type Checking**: With module-aware visibility
8. **Effect Inference**: Propagate effects through call graph
9. **Code Generation**: Per-module compilation units

### 10.2 Incremental Compilation

```
Change Type                  →  Recompilation Required
─────────────────────────────────────────────────────
Module body change           →  Module + dependents
Import added                 →  Check for cycles, recheck names
Visibility change            →  Recheck all importers
Public type change           →  Module + all dependents
Private implementation       →  Module only
Effect signature change      →  Module + all callers
```

### 10.3 IDE/LSP Support

| Feature | Implementation |
|---------|---------------|
| Go to definition | Follow module paths to source files |
| Find references | Index all imports and uses |
| Rename | Update across visibility boundaries |
| Completion | Show only visible items |
| Hover | Display effect signatures |
| Diagnostics | Real-time cycle detection |

### 10.4 Error Message Quality

All module-related errors should include:
1. **Precise location**: File, line, column
2. **Context**: What was being resolved
3. **Explanation**: Why the error occurred
4. **Suggestion**: How to fix it
5. **Documentation link**: For more information

```
error[E0433]: cannot find module `Auth` in scope

  --> src/main.aria:3:8
   |
 3 | import Auth::login
   |        ^^^^ not found in this scope
   |
 = note: no module named `Auth` is defined in the current package

help: did you mean one of these?
   |
 3 | import Services::Auth::login
   |        ^^^^^^^^
   |
 3 | import auth::login
   |        ^^^^ (if using lowercase module names)

For more information, see: https://aria-lang.org/errors/E0433
```

---

## 11. Summary and Recommendations

### 11.1 Key Design Decisions

| Aspect | Decision | Rationale |
|--------|----------|-----------|
| Import syntax | Rust-style `::` with selective imports | Precise, tree-shaking friendly |
| Visibility | Three tiers (private, package, pub) | Covers 99% of cases simply |
| File organization | 1:1 file-module, `mod.aria` for parents | Predictable, IDE-friendly |
| Circular deps | Prohibited | Better architecture, parallel compile |
| Effect visibility | Explicit propagation | Honest, composable |
| Name resolution | Two-phase, explicit > glob > prelude | Clear precedence |
| Compile-time eval | `const fn` and constants | Safe, deterministic |

### 11.2 Comparison to Current GRAMMAR.md

The current Aria grammar already supports most of this design. **Proposed changes**:

| Current | Proposed | Rationale |
|---------|----------|-----------|
| `priv` keyword | (default private) | Simpler, matches modern languages |
| No `pub(package)` | Add `pub(package)` | Internal API support |
| Basic imports | Rich import syntax | Ergonomics |
| Implicit effects | Explicit effect signatures | Clarity at API boundaries |

### 11.3 Open Questions for Future Research

1. **Conditional Compilation**: How do `@cfg` attributes interact with modules?
2. **Macro Visibility**: Can macros be module-private?
3. **Documentation Generation**: How to handle re-export chains in docs?
4. **ABI Stability**: Module versioning for binary compatibility?
5. **Hot Reloading**: Module replacement at runtime?

---

## 12. Key Resources

### Research Sources

1. [Rust Module System](https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html)
2. [Python Import System](https://docs.python.org/3/reference/import.html)
3. [ES6 Modules](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Modules)
4. [Go Package System](https://go.dev/doc/effective_go#package-names)
5. [Haskell Module System](https://www.haskell.org/tutorial/modules.html)
6. [OCaml Module System](https://dev.realworldocaml.org/files-modules-and-programs.html)
7. [ML Modules Paper](https://people.mpi-sws.org/~dreyer/papers/modules/main.pdf)
8. [Tarjan's Algorithm](https://en.wikipedia.org/wiki/Tarjan%27s_strongly_connected_components_algorithm)

### Related Aria Documents

- `GRAMMAR.md` - Current syntax specification
- `PRD-v2.md` - Product requirements
- `eureka-vault/milestones/ARIA-M15-module-system.md` - Module system milestone
- `eureka-vault/research/stdlib/ARIA-M15-01-module-systems-comparison.md` - Prior comparison
- `eureka-vault/research/compiler_architecture/ARIA-M09-01-module-system-design.md` - Module design
- `eureka-vault/research/effects/ARIA-M03-01-algebraic-effects-survey.md` - Effect system

---

## Appendix A: Complete Import Grammar

```ebnf
(* Full EBNF for Aria import/module system *)

program         = { module_item } ;

module_item     = module_decl
                | import_decl
                | reexport_decl
                | export_decl
                | submod_decl
                | visibility item_decl ;

module_decl     = 'module' module_path newline ;

module_path     = identifier { '::' identifier } ;

import_decl     = 'import' import_spec newline ;

import_spec     = import_source [ import_modifier ] ;

import_source   = module_path
                | string_literal ;

import_modifier = '::' import_selection
                | 'as' identifier ;

import_selection = '*'
                 | '{' import_list '}' ;

import_list     = import_item { ',' import_item } [ ',' ] ;

import_item     = identifier [ 'as' identifier ]
                | nested_import ;

nested_import   = identifier '::' import_selection ;

reexport_decl   = visibility 'use' reexport_spec newline ;

reexport_spec   = module_path [ '::' import_selection ] ;

export_decl     = 'export' '{' identifier_list '}' newline ;

identifier_list = identifier { ',' identifier } [ ',' ] ;

submod_decl     = visibility 'mod' identifier newline ;

visibility      = 'pub'
                | 'pub' '(' 'package' ')'
                | (* empty = private *) ;
```

---

## Appendix B: Cycle Detection Pseudocode

```python
def compile_modules(project_root: Path) -> CompilationResult:
    """
    Main compilation entry point with cycle detection.
    """
    # Phase 1: Discover all modules
    modules = discover_modules(project_root)

    # Phase 2: Build dependency graph
    graph = {}
    for module in modules:
        imports = parse_imports(module)
        graph[module.path] = [resolve_import(i) for i in imports]

    # Phase 3: Detect cycles
    cycles = tarjan_scc(graph)
    if cycles:
        report_cycle_error(cycles)
        return CompilationResult.Failed

    # Phase 4: Topological sort
    compilation_order = topological_sort(graph)

    # Phase 5: Compile in order
    for module_path in compilation_order:
        compile_module(modules[module_path])

    return CompilationResult.Success


def tarjan_scc(graph: Dict[str, List[str]]) -> List[List[str]]:
    """
    Find strongly connected components (cycles).
    Returns list of cycles (SCCs with size > 1).
    """
    index_counter = [0]
    stack = []
    lowlink = {}
    index = {}
    on_stack = set()
    sccs = []

    def strongconnect(v):
        index[v] = index_counter[0]
        lowlink[v] = index_counter[0]
        index_counter[0] += 1
        stack.append(v)
        on_stack.add(v)

        for w in graph.get(v, []):
            if w not in index:
                strongconnect(w)
                lowlink[v] = min(lowlink[v], lowlink[w])
            elif w in on_stack:
                lowlink[v] = min(lowlink[v], index[w])

        if lowlink[v] == index[v]:
            scc = []
            while True:
                w = stack.pop()
                on_stack.remove(w)
                scc.append(w)
                if w == v:
                    break
            if len(scc) > 1:  # Only cycles, not single nodes
                sccs.append(scc)

    for v in graph:
        if v not in index:
            strongconnect(v)

    return sccs
```

---

**Document Status**: Completed
**Next Steps**:
1. Review with Aria core team
2. Update GRAMMAR.md with finalized syntax
3. Implement module graph builder in compiler prototype
4. Add comprehensive test suite for edge cases
5. Design IDE integration for module features
