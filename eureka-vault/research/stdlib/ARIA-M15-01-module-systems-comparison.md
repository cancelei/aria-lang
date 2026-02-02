# ARIA-M15-01: Module Systems Comparison

**Task ID**: ARIA-M15-01
**Status**: Completed
**Date**: 2026-01-14
**Focus**: Compare module systems across languages for Aria's design

---

## Executive Summary

Module systems vary dramatically across languages—from Java's hierarchical packages to ML's powerful functors to Rust's privacy-focused approach. This research analyzes trade-offs to inform Aria's module design.

---

## 1. Overview

### 1.1 What Module Systems Provide

- **Namespace management**: Avoid naming conflicts
- **Encapsulation**: Hide implementation details
- **Code organization**: Logical structure
- **Dependency management**: Control imports/exports
- **Separate compilation**: Compile units independently

### 1.2 Comparison Axes

| Axis | Spectrum |
|------|----------|
| Structure | Flat ↔ Hierarchical |
| Privacy | Open ↔ Explicit ↔ Closed |
| Parameterization | None ↔ Generics ↔ Functors |
| File relationship | 1:1 ↔ N:1 ↔ Flexible |
| Import style | Qualified ↔ Unqualified ↔ Mixed |

---

## 2. Rust's Module System

### 2.1 Core Design

```rust
// mod.rs or inline modules
mod utils {
    pub fn helper() { }

    fn private_helper() { }  // Not visible outside

    pub(crate) fn crate_visible() { }

    pub(super) fn parent_visible() { }
}

// File-based modules
// src/network/client.rs
mod network {
    mod client;  // Loads network/client.rs
}
```

### 2.2 Privacy Model

| Visibility | Meaning |
|------------|---------|
| (default) | Private to current module |
| `pub` | Public to everyone |
| `pub(crate)` | Public within crate |
| `pub(super)` | Public to parent module |
| `pub(in path)` | Public to specific ancestor |

### 2.3 Strengths & Weaknesses

| Strength | Weakness |
|----------|----------|
| Fine-grained privacy | Complex rules |
| Crate = compilation unit | mod.rs confusion |
| Clear exports | Re-export boilerplate |

---

## 3. ML/OCaml Module System

### 3.1 Modules and Signatures

```ocaml
(* Signature = module interface *)
module type STACK = sig
  type 'a t
  val empty : 'a t
  val push : 'a -> 'a t -> 'a t
  val pop : 'a t -> ('a * 'a t) option
end

(* Module implementation *)
module ListStack : STACK = struct
  type 'a t = 'a list
  let empty = []
  let push x s = x :: s
  let pop = function
    | [] -> None
    | x :: xs -> Some (x, xs)
end
```

### 3.2 Functors (Parameterized Modules)

```ocaml
(* Functor = function from modules to modules *)
module type COMPARABLE = sig
  type t
  val compare : t -> t -> int
end

module MakeSet (Item : COMPARABLE) : sig
  type t
  val empty : t
  val add : Item.t -> t -> t
  val member : Item.t -> t -> bool
end = struct
  type t = Item.t list
  let empty = []
  let add x s = x :: s
  let member x s = List.exists (fun y -> Item.compare x y = 0) s
end

(* Usage *)
module IntSet = MakeSet(struct
  type t = int
  let compare = compare
end)
```

### 3.3 First-Class Modules

```ocaml
(* Modules can be values *)
let stack_module =
  if use_array then (module ArrayStack : STACK)
  else (module ListStack : STACK)

let (module S : STACK) = stack_module in
S.push 42 S.empty
```

---

## 4. Go's Package System

### 4.1 Simple Model

```go
// package.go
package mypackage

// Exported (uppercase)
func PublicFunction() { }

// Unexported (lowercase)
func privateFunction() { }
```

### 4.2 Characteristics

| Aspect | Go Approach |
|--------|-------------|
| Directory = Package | One package per directory |
| Visibility | Case-based (upper = public) |
| Imports | Full path required |
| Circular | Not allowed |
| Internal | `internal/` directory convention |

### 4.3 Strengths & Weaknesses

| Strength | Weakness |
|----------|----------|
| Simple rules | No fine-grained control |
| Easy to understand | Case convention unusual |
| Fast compilation | Limited abstraction |

---

## 5. TypeScript/JavaScript

### 5.1 ES Modules

```typescript
// Named exports
export function helper() { }
export const CONSTANT = 42;

// Default export
export default class MyClass { }

// Re-exports
export { foo, bar } from './other';
export * from './everything';

// Import styles
import { helper } from './utils';
import * as utils from './utils';
import MyClass from './myclass';
```

### 5.2 Module Resolution

| Strategy | Path |
|----------|------|
| Relative | `./foo`, `../bar` |
| Node | `node_modules/package` |
| Path mapping | `@app/utils` → `src/utils` |
| Barrel | `index.ts` re-exports |

---

## 6. Haskell Modules

### 6.1 Export Lists

```haskell
module Data.Stack
  ( Stack       -- Type only (opaque)
  , empty       -- Function
  , push        -- Function
  , pop         -- Function
  , Stack(..)   -- Type + all constructors
  ) where

-- Not exported
internalHelper :: a -> a
internalHelper = id
```

### 6.2 Import Styles

```haskell
import Data.List                    -- All exports
import Data.List (sort, nub)        -- Specific
import Data.List hiding (head)      -- Everything except
import qualified Data.Map as M      -- Qualified only
import Data.Map (Map)               -- Type unqualified
```

---

## 7. Comparison Matrix

| Feature | Rust | OCaml | Go | TS | Haskell |
|---------|------|-------|----|----|---------|
| Privacy control | Fine | Signature | Binary | Export | Export |
| Functors | No | Yes | No | No | Typeclasses |
| File = Module | Config | Yes | Yes | Yes | Yes |
| Circular deps | No | No | No | Yes | No |
| First-class | No | Yes | No | Yes* | No |
| Qualified imports | Yes | Yes | Implicit | Yes | Yes |

---

## 8. Recommendations for Aria

### 8.1 Module Syntax

```aria
# Module definition
module Collections.Stack

# Explicit exports
export Stack, empty, push, pop

# Private by default
type StackImpl[T] = Array[T]  # Not exported

# Public type with hidden implementation
type Stack[T] = opaque StackImpl[T]

fn empty[T]() -> Stack[T] = []

fn push[T](stack: Stack[T], item: T) -> Stack[T]
  stack.append(item)
end

fn pop[T](stack: Stack[T]) -> Option[(T, Stack[T])]
  if stack.is_empty
    None
  else
    Some((stack.last, stack.init))
  end
end
```

### 8.2 Visibility Modifiers

```aria
# Aria visibility (Rust-inspired but simpler)
module MyLib

# Default: module-private
fn helper() = ...

# Public to everyone
pub fn api_function() = ...

# Public within package only
pub(package) fn internal_api() = ...

# Public to submodules
pub(children) fn for_children() = ...
```

### 8.3 Module Parameters (Lightweight Functors)

```aria
# Parameterized module
module MakeSet[T: Comparable]
  export Set, empty, insert, contains

  type Set = Array[T]

  fn empty() -> Set = []

  fn insert(set: Set, item: T) -> Set
    if contains(set, item) then set else set.append(item) end
  end

  fn contains(set: Set, item: T) -> Bool
    set.any |x| x == item end
  end
end

# Instantiation
module IntSet = MakeSet[Int]

# Usage
let set = IntSet.empty().insert(1).insert(2)
```

### 8.4 Import Syntax

```aria
# Full import
import Collections.Stack

# Selective import
import Collections.Stack { Stack, push, pop }

# Qualified import
import Collections.Stack as S

# Re-export
module MyCollections
  export from Collections.Stack { Stack, push, pop }
  export from Collections.Queue { Queue }
end
```

### 8.5 File Organization

```aria
# Convention: one module per file, path matches
# src/collections/stack.aria → Collections.Stack

# Module hierarchy via directories
# src/
#   collections/
#     mod.aria      # Collections module
#     stack.aria    # Collections.Stack
#     queue.aria    # Collections.Queue

# Explicit in mod.aria:
module Collections
  pub mod stack  # Re-exports Collections.Stack
  pub mod queue  # Re-exports Collections.Queue
end
```

---

## 9. Key Resources

1. [Rust Module System](https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html)
2. [OCaml Module System](https://dev.realworldocaml.org/files-modules-and-programs.html)
3. [ML Module System](https://people.mpi-sws.org/~dreyer/papers/modules/main.pdf)
4. [TypeScript Module Resolution](https://www.typescriptlang.org/docs/handbook/module-resolution.html)
5. [Go Package Design](https://go.dev/doc/effective_go#package-names)

---

## 10. Open Questions

1. Should Aria support full OCaml-style functors or simpler parameterized modules?
2. How do modules interact with effects?
3. Should we allow circular dependencies with forward declarations?
4. What's the compilation unit—file, module, or package?
