# ARIA-M01-03: Trait/Protocol System Design

**Task ID**: ARIA-M01-03
**Status**: Completed
**Date**: 2026-01-14
**Focus**: Design Aria's approach to ad-hoc polymorphism

---

## Executive Summary

Ad-hoc polymorphism enables functions to work over different types with type-specific implementations. This research compares Rust traits, Haskell type classes, and Swift protocols to design Aria's approach.

---

## 1. Terminology Comparison

| Language | Interface | Implementation | Concept |
|----------|-----------|----------------|---------|
| Haskell | Type class | Instance | Ad-hoc polymorphism |
| Rust | Trait | impl | Bounded parametric polymorphism |
| Swift | Protocol | Extension/conformance | Protocol-oriented programming |
| C++ | Concept | - | Constrained templates |
| Scala | Trait | extends/with | Mixin composition |

---

## 2. Rust Traits Analysis

### 2.1 Core Features

```rust
trait Display {
    fn fmt(&self, f: &mut Formatter) -> Result;
}

impl Display for Point {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}
```

### 2.2 Key Properties

| Feature | Rust Approach |
|---------|---------------|
| **Self type** | Implicit in trait block |
| **Associated types** | `type Item;` in trait |
| **Default impls** | Supported |
| **Orphan rules** | Strict (crate coherence) |
| **Static dispatch** | Default (monomorphization) |
| **Dynamic dispatch** | `dyn Trait` trait objects |
| **Multiple bounds** | `T: A + B + C` |

### 2.3 Orphan Rules

```rust
// Allowed: implementing foreign trait for local type
impl Display for MyType { ... }

// Allowed: implementing local trait for foreign type
impl MyTrait for Vec<i32> { ... }

// FORBIDDEN: implementing foreign trait for foreign type
impl Display for Vec<i32> { ... }  // Error!
```

**Rationale**: Prevents conflicting implementations across crates.

### 2.4 Associated Types vs Type Parameters

```rust
// Associated type (single implementation per type)
trait Iterator {
    type Item;
    fn next(&mut self) -> Option<Self::Item>;
}

// Type parameter (multiple implementations possible)
trait From<T> {
    fn from(value: T) -> Self;
}
```

---

## 3. Haskell Type Classes Analysis

### 3.1 Core Features

```haskell
class Show a where
    show :: a -> String

instance Show Point where
    show (Point x y) = "(" ++ show x ++ ", " ++ show y ++ ")"
```

### 3.2 Key Properties

| Feature | Haskell Approach |
|---------|------------------|
| **Self type** | Explicit type variable `a` |
| **Associated types** | Type families extension |
| **Default impls** | Supported |
| **Orphan rules** | Flexible (warnings only) |
| **Multi-param** | Extension required |
| **Functional deps** | `| a -> b` determines type |

### 3.3 Multi-Parameter Type Classes

```haskell
{-# LANGUAGE MultiParamTypeClasses #-}
{-# LANGUAGE FunctionalDependencies #-}

class Collection c e | c -> e where
    insert :: e -> c -> c
    member :: e -> c -> Bool
```

### 3.4 Superclasses

```haskell
class Eq a => Ord a where
    compare :: a -> a -> Ordering
-- Ord requires Eq to be implemented first
```

---

## 4. Swift Protocols Analysis

### 4.1 Core Features

```swift
protocol Displayable {
    func display() -> String
}

extension Point: Displayable {
    func display() -> String {
        return "(\(x), \(y))"
    }
}
```

### 4.2 Key Properties

| Feature | Swift Approach |
|---------|----------------|
| **Self type** | `Self` keyword |
| **Associated types** | `associatedtype` |
| **Default impls** | Protocol extensions |
| **Existentials** | `any Protocol` (boxed) |
| **Generics** | `some Protocol` (opaque) |
| **Inheritance** | Protocols can inherit |

### 4.3 Protocol Extensions (Default Implementations)

```swift
protocol Greetable {
    var name: String { get }
    func greet() -> String
}

extension Greetable {
    func greet() -> String {
        return "Hello, \(name)!"
    }
}
```

### 4.4 Protocol Composition

```swift
func process<T: Displayable & Hashable>(_ item: T) { ... }
// Or with 'any':
func process(_ item: any Displayable & Hashable) { ... }
```

---

## 5. Trade-off Analysis

### 5.1 Coherence vs Flexibility

| Approach | Coherence | Flexibility | Risk |
|----------|-----------|-------------|------|
| **Rust (strict)** | Guaranteed | Limited | None |
| **Haskell (flexible)** | Best-effort | High | Conflicts |
| **Swift (module)** | Module-local | Moderate | Contained |

### 5.2 Type Safety Concerns

**Problem**: Ad-hoc polymorphism can cause silent behavior changes after refactoring.

```rust
// Before refactor
fn process<T: Display>(x: T) { println!("{}", x); }

// After: accidentally use Debug instead
fn process<T: Debug>(x: T) { println!("{:?}", x); }
// Different output, but compiles!
```

**Mitigation**: Clear naming, explicit bounds, careful API design.

### 5.3 Dispatch Strategies

| Strategy | Performance | Flexibility | Use Case |
|----------|-------------|-------------|----------|
| **Static (monomorphization)** | Optimal | Limited | Known types at compile time |
| **Dynamic (vtable)** | Overhead | High | Heterogeneous collections |
| **Specialization** | Optimal | Moderate | Performance-critical paths |

---

## 6. Recommendations for Aria

### 6.1 Core Trait System

```aria
# Trait definition
trait Display
  fn display(self) -> String
end

# Implementation
impl Display for Point
  fn display(self) -> String
    "(#{self.x}, #{self.y})"
  end
end

# Usage with bounds
fn print_all[T: Display](items: Array[T])
  for item in items
    println(item.display)
  end
end
```

### 6.2 Associated Types

```aria
trait Iterator
  type Item

  fn next(self) -> Option[Self.Item]
end

impl Iterator for Range[Int]
  type Item = Int

  fn next(self) -> Option[Int]
    # ...
  end
end
```

### 6.3 Default Implementations

```aria
trait Greetable
  fn name(self) -> String

  # Default implementation
  fn greet(self) -> String
    "Hello, #{self.name}!"
  end
end
```

### 6.4 Orphan Rules (Recommended)

**Aria should adopt module-scoped coherence** (like Swift):

```aria
# Allowed in any module:
impl Display for MyType          # Local type
impl MyTrait for ExternalType    # Local trait

# Requires explicit opt-in or same-package:
impl ExternalTrait for ExternalType  # Needs annotation
```

**Proposed syntax for opting in**:
```aria
@orphan_impl  # Explicit acknowledgment of potential conflicts
impl Display for Vec[Int]
  # ...
end
```

### 6.5 Multiple Trait Bounds

```aria
# Combined bounds
fn process[T: Display + Hash + Clone](item: T)
  # ...
end

# Where clause for complex bounds
fn complex[T, U](x: T, y: U) -> Result[T, Error]
  where T: Iterator[Item = U],
        U: Display + Default
  # ...
end
```

### 6.6 Dynamic Dispatch

```aria
# Static dispatch (default, optimal performance)
fn static_print[T: Display](item: T)
  println(item.display)
end

# Dynamic dispatch (explicit, for heterogeneous collections)
fn dynamic_print(item: dyn Display)
  println(item.display)
end

# Collection of mixed types
items: Array[dyn Display] = [point, circle, text]
```

### 6.7 Trait Inheritance

```aria
trait Eq
  fn eq(self, other: Self) -> Bool
end

trait Ord: Eq  # Ord requires Eq
  fn cmp(self, other: Self) -> Ordering

  # Default implementations using cmp
  fn lt(self, other: Self) -> Bool
    self.cmp(other) == Ordering.Less
  end
end
```

---

## 7. Design Decisions Summary

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Name** | `trait` | Familiar to Rust users |
| **Self type** | Implicit | Less verbose |
| **Associated types** | Yes | Essential for iterators, etc. |
| **Default impls** | Yes | Reduces boilerplate |
| **Orphan rules** | Module-scoped | Balance coherence/flexibility |
| **Static dispatch** | Default | Performance |
| **Dynamic dispatch** | Explicit `dyn` | Clear cost model |
| **Multi-bounds** | `+` syntax | Intuitive |
| **Where clauses** | Yes | Complex bounds readability |

---

## 8. Key Resources

1. [ACCU: Concepts vs Typeclasses vs Traits vs Protocols](https://accu.org/conf-docs/PDFs_2021/conor_hoekstra_c_concepts_vs_haskell_typeclasses_vs_rust_traits_vs_swift_protocols.pdf)
2. [Comparing Traits and Typeclasses](https://terbium.io/2021/02/traits-typeclasses/)
3. [Principled Ad-Hoc Polymorphism](https://typesanitizer.com/blog/ad-hoc-polymorphism.html)
4. Rust Reference: Traits
5. Swift Language Guide: Protocols

---

## 9. Open Questions

1. Should Aria support negative trait bounds (`T: !Copy`)?
2. How do traits interact with effect inference?
3. Should we allow specialization for performance?
4. How do contracts integrate with trait bounds?
