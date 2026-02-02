# ARIA-M02-04: Hybrid Ownership Model Design

**Task ID**: ARIA-M02-04
**Status**: Completed
**Date**: 2026-01-15
**Focus**: Design Aria's ownership inference algorithm for 80% annotation-free code
**Blocked by**: ARIA-M02-01 (Rust), ARIA-M02-02 (Swift), ARIA-M02-03 (Vale)

---

## Executive Summary

This document presents Aria's **Hybrid Ownership Model** - a memory management system that combines compile-time ownership inference with explicit annotations for complex cases and ARC as an escape hatch. The design achieves the PRD-v2 target of 80% annotation-free code while maintaining Rust-level memory safety.

**Key Innovation**: A three-tier ownership system that infers ownership for common patterns, requires explicit annotations for ambiguous cases, and falls back to ARC for inherently cyclic or shared structures.

---

## 1. Design Goals

### 1.1 Primary Objectives

| Goal | Target | Measurement |
|------|--------|-------------|
| Annotation Freedom | 80% of code | Lines requiring no ownership annotation |
| Memory Safety | 100% | No use-after-free, no data races |
| Performance | Near-Rust | < 5% overhead vs explicit ownership |
| Learning Curve | Low | Beginner-friendly defaults |

### 1.2 Non-Goals

- Full compile-time verification of all ownership patterns (accept runtime checks for complex cases)
- Zero runtime overhead (ARC escape hatch has overhead)
- Rust compatibility (different syntax, different semantics)

---

## 2. Research Foundation

### 2.1 Insights from Lobster

[Lobster's memory management](https://aardappel.github.io/lobster/memory_management.html) achieves ~95% compile-time reference count elimination through:

1. **AST-based ownership inference**: Each AST node has ownership "kind" properties
2. **L-value borrowing**: Variables default to ownership, references borrow
3. **Function specialization**: Functions specialize based on caller ownership needs
4. **Flow-sensitive analysis**: Interleaved with type checking

**Key Lobster Insight for Aria**: The algorithm operates on AST nodes, matching parent-child ownership expectations:
- Agreement: No action needed
- Parent owns, child borrows: Insert retain (or error in strict mode)
- Parent borrows, child owns: Create anonymous variable for deallocation

**Limitation**: Lobster's AST-based approach may not scale to complex control flow. Rust's move to NLL (Non-Lexical Lifetimes) via CFG analysis addresses this.

### 2.2 Insights from Swift ARC

[Swift's ARC optimization](https://apple-swift.readthedocs.io/en/latest/ARCOptimization.html) provides patterns for compile-time analysis:

1. **RC Identity Analysis**: Track equivalence classes of RC-identical values
2. **Escape Analysis**: Determine if references escape their scope
3. **Copy-on-Write optimization**: `is_unique` checks for mutation safety
4. **SIL-level optimization**: Retain/release elimination at IR level

**Key Swift Insight for Aria**: RC Identity concept - operations that preserve reference count identity can be optimized together.

### 2.3 Insights from Rust (ARIA-M02-01)

1. **Polonius algorithm**: Datalog-based loan/origin tracking
2. **NLL (Non-Lexical Lifetimes)**: CFG-based liveness analysis
3. **Two-phase borrows**: Enable method-call ergonomics
4. **Explicit lifetime requirements**: Functions returning references, structs with references

### 2.4 Insights from Vale (ARIA-M02-03)

1. **Generational references**: Runtime safety check with 2-11% overhead
2. **Region borrowing**: Zero-cost immutable regions
3. **Linear-aliasing model**: Ownership + non-owning references

---

## 3. The Hybrid Ownership Model

### 3.1 Three-Tier Architecture

```
Tier 1: Inferred Ownership (Target: 80% of code)
├── Single-owner patterns
├── Move semantics
├── Local borrowing
└── Function-scoped references

Tier 2: Explicit Annotations (Target: 15% of code)
├── Ambiguous ownership
├── Reference-returning functions
├── Structs containing references
└── Complex lifetimes

Tier 3: ARC Escape Hatch (Target: 5% of code)
├── Cyclic data structures
├── Shared mutable state
├── Observer patterns
└── Graph structures
```

### 3.2 Ownership Categories

| Category | Symbol | Semantics | Use Case |
|----------|--------|-----------|----------|
| Owned | (default) | Single owner, responsible for deallocation | Most values |
| Reference | `ref` | Immutable borrow, cannot outlive referent | Read-only access |
| Mutable Reference | `mut ref` | Exclusive mutable borrow | Modification |
| Shared (ARC) | `@shared` | Reference counted, multiple owners | Cycles, observers |
| Weak | `@weak` | Non-owning, nullable | Breaking cycles |

---

## 4. Ownership Inference Algorithm

### 4.1 Algorithm Overview

```
ARIA_OWNERSHIP_INFERENCE:

Input: Type-checked AST with flow-sensitive types
Output: Ownership-annotated IR

1. INITIALIZATION
   - Mark all heap allocations as "needs owner"
   - Mark all function parameters as "ownership TBD"
   - Build def-use chains for all values

2. LOCAL INFERENCE (per function)
   For each function in topological order:
   a. Compute liveness regions using CFG
   b. Apply ownership rules (Section 4.2)
   c. Identify borrowing relationships
   d. Detect inference failures -> mark for annotation

3. INTERPROCEDURAL ANALYSIS
   For each call site:
   a. Match caller ownership with callee expectations
   b. Specialize functions if needed (like Lobster)
   c. Propagate ownership constraints

4. ESCAPE ANALYSIS
   For each value:
   a. Check if value escapes its defining scope
   b. If escapes to heap -> check for cycles
   c. If cycle detected -> suggest @shared

5. VALIDATION
   - Verify all ownership assigned
   - Check no dangling references possible
   - Report ambiguous cases requiring annotation
```

### 4.2 Inference Rules

#### Rule 1: Assignment Transfers Ownership

```aria
# Inferred: move semantics
let a = create_resource()  # a owns
let b = a                   # b owns, a invalidated
use(b)                      # OK
# use(a)                    # ERROR: a was moved
```

#### Rule 2: Last Use Determines Lifetime

```aria
# Inferred: borrow until last use (NLL-style)
fn process(data: Array[Int])
  let first = data[0]      # Borrow of data starts
  print(first)             # Last use of first
  data.push(10)            # OK: borrow ended at last use
end
```

#### Rule 3: Function Return Inference

```aria
# Case 1: No reference in return -> ownership transferred
fn create() -> Resource
  Resource.new              # Ownership returned to caller
end

# Case 2: Single reference parameter -> infer lifetime
fn first(arr: ref Array[T]) -> ref T
  arr[0]                    # Return borrows from arr (inferred)
end

# Case 3: Multiple reference parameters -> REQUIRES ANNOTATION
fn longest(a: ref String, b: ref String) -> ref String  # ERROR
  if a.len > b.len then a else b
end

# Fix: Explicit lifetime
fn longest[life L](a: ref[L] String, b: ref[L] String) -> ref[L] String
  if a.len > b.len then a else b
end
```

#### Rule 4: Struct Field Inference

```aria
# Case 1: Owned fields (default) -> no annotation
struct Person
  name: String              # Owned
  age: Int                  # Value type, no ownership
end

# Case 2: Reference fields -> REQUIRES ANNOTATION
struct Parser[life L]
  input: ref[L] String      # Must specify lifetime
  position: Int
end
```

#### Rule 5: Closure Capture Inference

```aria
# Inferred: capture by reference if possible
let data = [1, 2, 3]
let doubled = data.map { |x| x * 2 }  # data borrowed into closure

# Inferred: capture by move if escapes
let handler = create_handler {
  data.process()            # data moved into closure (escapes)
}
```

### 4.3 Inference Algorithm Pseudocode

```python
def infer_ownership(ast: AST) -> OwnershipResult:
    # Phase 1: Build ownership graph
    graph = OwnershipGraph()

    for node in ast.traverse_postorder():
        if node.is_allocation():
            graph.add_owned(node)
        elif node.is_assignment():
            graph.add_transfer(node.source, node.target)
        elif node.is_borrow():
            graph.add_borrow(node.source, node.target)
        elif node.is_function_call():
            graph.add_call_edge(node)

    # Phase 2: Compute ownership solution
    while graph.has_unresolved():
        node = graph.get_unresolved()

        # Rule-based inference
        if node.single_use():
            graph.assign_owned(node)
        elif node.all_uses_borrowing():
            graph.assign_borrowed(node)
        elif node.escapes_scope():
            if graph.detect_cycle(node):
                graph.mark_shared(node)  # Needs @shared
            else:
                graph.assign_owned_to_escaping_path(node)
        else:
            graph.mark_ambiguous(node)  # Needs annotation

    # Phase 3: Validate solution
    for node in graph.nodes():
        if not node.ownership_assigned():
            result.add_error(AnnotationRequired(node))
        elif node.may_dangle():
            result.add_error(DanglingReference(node))

    return result
```

---

## 5. Explicit Annotation Syntax

### 5.1 When Annotations Are Required

| Scenario | Why Inference Fails | Annotation Needed |
|----------|--------------------|--------------------|
| Function returns reference from multiple parameters | Ambiguous lifetime | `ref[L]` lifetime |
| Struct stores reference | Struct lifetime bound | `[life L]` parameter |
| Reference outlives scope | Escape analysis | `@shared` or lifetime |
| Mutable aliasing needed | Single-owner violated | `@shared` with `@weak` |

### 5.2 Annotation Syntax

#### Lifetime Parameters

```aria
# Explicit lifetime parameter
fn longest[life L](a: ref[L] String, b: ref[L] String) -> ref[L] String
  if a.len > b.len then a else b
end

# Multiple lifetimes
fn complex[life A, life B](x: ref[A] T, y: ref[B] U) -> ref[A] V
  where A: outlives B  # A must live at least as long as B
```

#### Ownership Annotations

```aria
# Explicit ownership modifiers
fn transfer(own data: Resource)      # Takes ownership (explicit)
fn inspect(ref data: Resource)       # Borrows immutably
fn modify(mut ref data: Resource)    # Borrows mutably

# On struct fields
struct Container
  own data: Buffer                   # Explicit owned (usually inferred)
  ref[self] parent: Container?       # Reference with self lifetime
end
```

#### ARC Escape Hatch

```aria
# Shared reference-counted type
@shared class Node
  value: Int
  @weak parent: Node?               # Weak to break cycle
  children: Array[Node]             # Strong children
end

# Using shared types
fn build_tree() -> Node
  let root = Node.new(0)
  let child = Node.new(1)
  child.parent = root               # Weak reference
  root.children.push(child)         # Strong reference
  root
end
```

### 5.3 Inference Override Syntax

```aria
# Force move when borrow would be inferred
let data = get_data()
process(move data)                  # Explicit move

# Force copy when move would be inferred
let data = get_data()
let backup = copy data              # Explicit copy
process(data)                       # Original still valid

# Force borrow when move would be inferred
let data = get_data()
inspect(borrow data)                # Explicit borrow
process(data)                       # Still valid
```

---

## 6. ARC Escape Hatch Specification

### 6.1 When ARC Is Triggered

The compiler suggests or requires `@shared` when:

1. **Cycle Detection**: Static analysis detects potential reference cycle
2. **Shared Mutation**: Multiple code paths need mutable access
3. **Observer Pattern**: Object needs to be referenced by multiple observers
4. **Graph Structures**: Nodes reference other nodes without clear hierarchy
5. **Cross-Thread Sharing**: Value shared between threads (with atomic refcount)

### 6.2 ARC Implementation

```aria
# @shared generates reference-counted wrapper
@shared class SharedBuffer
  data: Array[Byte]

  fn write(bytes: Array[Byte])
    data.extend(bytes)
  end
end

# Compiler generates (conceptually):
struct SharedBuffer_RC
  refcount: AtomicUInt
  weak_count: AtomicUInt
  value: SharedBuffer_Inner
end
```

### 6.3 Weak Reference Semantics

```aria
@shared class EventEmitter
  @weak listeners: Array[Listener]

  fn emit(event: Event)
    for listener in listeners.compact  # Filter nil weak refs
      listener.handle(event)
    end
  end
end

# Weak reference access
if let listener = weak_ref.upgrade()  # Returns Option
  listener.handle(event)
end
```

### 6.4 ARC Optimization

The compiler applies Swift-style optimizations to `@shared` types:

1. **Retain/Release Elimination**: Remove redundant operations
2. **RC Identity Analysis**: Track equivalent RC operations
3. **Escape Analysis**: Convert to stack allocation when possible
4. **Copy-on-Write**: For shared value types

---

## 7. Example Code: 80/20 Split Demonstration

### 7.1 Tier 1: Inferred Ownership (80%)

```aria
# Example 1: Simple function - fully inferred
fn process_data(input: String) -> String
  let words = input.split(" ")
  let filtered = words.filter { |w| w.len > 3 }
  filtered.join("-")
end

# Compiler infers:
# - input: owned (moved into function)
# - words: owned (result of split)
# - filtered: owned (result of filter, consumes words)
# - return: owned (result of join, consumes filtered)

# Example 2: Struct with owned fields - fully inferred
struct Config
  host: String
  port: Int
  timeout: Duration
end

fn load_config(path: String) -> Config
  let content = File.read(path)
  let parsed = parse_toml(content)
  Config(
    host: parsed["host"].as_string,
    port: parsed["port"].as_int,
    timeout: Duration.seconds(parsed["timeout"].as_int)
  )
end

# Example 3: Borrowing within function - fully inferred
fn find_longest(items: Array[String]) -> Option[String]
  if items.empty? then return None

  let mut longest = items[0]    # Borrow of items
  for item in items[1..]        # Borrow of items
    if item.len > longest.len
      longest = item            # Reborrow
    end
  end
  Some(longest.clone)           # Clone to return owned value
end

# Example 4: Method chains - fully inferred
fn transform_users(users: Array[User]) -> Array[String]
  users
    .filter { |u| u.active }
    .sort_by { |u| u.name }
    .map { |u| u.name.uppercase }
end

# Example 5: Error handling with Result - fully inferred
fn fetch_user(id: Int) -> Result[User, Error]
  let response = http_get("/users/{id}")?
  let user = User.from_json(response.body)?
  Ok(user)
end
```

### 7.2 Tier 2: Explicit Annotations (15%)

```aria
# Example 1: Function returning reference from multiple sources
fn longest[life L](a: ref[L] String, b: ref[L] String) -> ref[L] String
  if a.len > b.len then a else b
end

# Usage - lifetime inferred at call site
let result = longest(name1, name2)  # Borrows both, returns borrow

# Example 2: Struct containing references
struct Parser[life L]
  source: ref[L] String
  position: Int
end

impl Parser[life L]
  fn new(source: ref[L] String) -> Parser[L]
    Parser(source: source, position: 0)
  end

  fn peek(ref self) -> Char
    self.source[self.position]
  end

  fn advance(mut ref self)
    self.position += 1
  end
end

# Example 3: Iterator with lifetime
struct Iter[life L, T]
  data: ref[L] Array[T]
  index: Int
end

impl Iter[life L, T]: Iterator[ref[L] T]
  fn next(mut ref self) -> Option[ref[L] T]
    if self.index >= self.data.len
      None
    else
      let item = self.data[self.index]
      self.index += 1
      Some(item)
    end
  end
end

# Example 4: Complex lifetime bounds
fn process_with_cache[life A, life B](
  input: ref[A] Data,
  cache: mut ref[B] Cache
) -> ref[A] Result
  where B: outlives A

  if let cached = cache.get(input.key)
    return cached
  end

  let result = compute(input)
  cache.store(input.key, result)
  result
end

# Example 5: Self-referential via indirection
struct SelfRef[life L]
  data: String
  ref_to_data: ref[L] String
end

fn create_self_ref[life L](arena: mut ref[L] Arena) -> SelfRef[L]
  let data = "hello".to_string
  let stored = arena.store(data)         # Arena owns
  SelfRef(data: stored.clone, ref_to_data: stored)
end
```

### 7.3 Tier 3: ARC Escape Hatch (5%)

```aria
# Example 1: Parent-child cycle
@shared class TreeNode
  value: Int
  @weak parent: TreeNode?
  children: Array[TreeNode]

  fn add_child(child: TreeNode)
    child.parent = self
    children.push(child)
  end

  fn root() -> TreeNode
    match parent
      Some(p) => p.root()
      None => self
    end
  end
end

# Example 2: Observer pattern
@shared class Observable[T]
  value: T
  @weak observers: Array[Observer[T]]

  fn subscribe(observer: Observer[T])
    observers.push(observer)
  end

  fn notify()
    for obs in observers.compact
      obs.on_change(value)
    end
  end

  fn set(new_value: T)
    value = new_value
    notify()
  end
end

# Example 3: Graph structure
@shared class GraphNode[T]
  data: T
  edges: Array[GraphNode[T]]  # Can point to any node, including self

  fn connect(other: GraphNode[T])
    edges.push(other)
  end

  fn traverse(visited: mut Set[GraphNode[T]], action: fn(T))
    if visited.contains(self) then return
    visited.insert(self)
    action(data)
    for edge in edges
      edge.traverse(visited, action)
    end
  end
end

# Example 4: Shared mutable state
@shared class SharedCounter
  count: AtomicInt

  fn increment() -> Int
    count.fetch_add(1)
  end

  fn get() -> Int
    count.load()
  end
end

# Cross-thread usage
fn parallel_count() -> Int
  let counter = SharedCounter.new(0)

  let handles = (0..10).map { |_|
    spawn {
      for _ in 0..1000
        counter.increment()
      end
    }
  }

  for h in handles
    h.join()
  end

  counter.get()
end

# Example 5: Callback storage (closures capturing self)
@shared class Button
  label: String
  @shared on_click: Option[fn()]

  fn set_handler(handler: fn())
    on_click = Some(handler)
  end

  fn click()
    if let handler = on_click
      handler()
    end
  end
end

# Usage with weak self capture
@shared class ViewController
  button: Button
  count: Int

  fn setup()
    let weak_self = @weak self
    button.set_handler {
      if let this = weak_self.upgrade()
        this.count += 1
        print("Count: {this.count}")
      end
    }
  end
end
```

### 7.4 Code Distribution Analysis

```
Typical Aria Codebase Distribution:

Tier 1 (Inferred - 80%):
├── Business logic functions      ~30%
├── Data transformations          ~20%
├── Simple structs/classes        ~15%
├── Collection operations         ~10%
└── Error handling                ~5%

Tier 2 (Explicit - 15%):
├── Low-level parsers/iterators   ~5%
├── Zero-copy APIs                ~4%
├── Performance-critical code     ~3%
├── Library interfaces            ~2%
└── Complex generic bounds        ~1%

Tier 3 (ARC - 5%):
├── UI component trees            ~2%
├── Event/observer systems        ~1%
├── Graph algorithms              ~1%
└── Shared state management       ~1%
```

---

## 8. Compiler Diagnostics

### 8.1 Inference Failure Messages

```aria
# Ambiguous lifetime error
fn get_first(a: ref String, b: ref String) -> ref String
  a
end

# Compiler output:
error[E0401]: cannot infer lifetime for return value
 --> src/lib.aria:1:45
  |
1 | fn get_first(a: ref String, b: ref String) -> ref String
  |              -              -                 ^^^^^^^^^^
  |              |              |                 |
  |              |              |                 return type has reference
  |              |              parameter 'b' is a reference
  |              parameter 'a' is a reference
  |
  = help: the return value could borrow from either 'a' or 'b'
  = help: add explicit lifetime parameters:
  |
1 | fn get_first[life L](a: ref[L] String, b: ref[L] String) -> ref[L] String
  |             ++++++++    ++++++            ++++++             ++++++
```

### 8.2 Move After Use Error

```aria
let data = load_data()
process(data)
print(data.len)  # Error!

# Compiler output:
error[E0382]: borrow of moved value: `data`
 --> src/main.aria:3:7
  |
1 | let data = load_data()
  |     ---- move occurs because `data` has type `Data`
2 | process(data)
  |         ---- value moved here
3 | print(data.len)
  |       ^^^^ value borrowed here after move
  |
  = help: consider borrowing instead of moving:
  |
2 | process(borrow data)
  |         ++++++
```

### 8.3 Cycle Detection Warning

```aria
class Node
  parent: Node?
  children: Array[Node]
end

# Compiler output:
warning[W0501]: potential reference cycle detected
 --> src/tree.aria:1:1
  |
1 | class Node
  | ^^^^^^^^^^ this type may form reference cycles
2 |   parent: Node?
  |   ------------- field references same type
3 |   children: Array[Node]
  |   --------------------- field contains same type
  |
  = note: cycles cause memory leaks without reference counting
  = help: consider using @shared and @weak:
  |
1 | @shared class Node
2 |   @weak parent: Node?
  |   +++++
```

---

## 9. Implementation Roadmap

### 9.1 Phase 1: Basic Inference (MVP)

- Local ownership inference (single function)
- Simple move semantics
- Basic borrow checking
- Error messages for inference failures

### 9.2 Phase 2: Advanced Inference

- Interprocedural ownership analysis
- Function specialization (Lobster-style)
- NLL-style liveness analysis
- Closure capture inference

### 9.3 Phase 3: ARC Integration

- @shared type implementation
- Weak reference support
- Cycle detection heuristics
- ARC optimization passes

### 9.4 Phase 4: Optimization

- RC Identity analysis
- Retain/release elimination
- Escape analysis for stack promotion
- Copy-on-write for shared value types

---

## 10. Comparison with Alternatives

| Aspect | Aria Hybrid | Rust | Swift | Lobster | Vale |
|--------|-------------|------|-------|---------|------|
| Default mode | Inferred ownership | Explicit | ARC | Inferred RC | Generational |
| Annotation burden | Low (15% code) | High (all refs) | None | Low | Medium |
| Runtime overhead | 0-5% | 0% | 5-15% | ~5% | 2-11% |
| Cycle handling | @shared/weak | Manual | weak/unowned | Runtime | Gen refs |
| Learning curve | Low | High | Low | Low | Medium |
| Compile-time safety | High | Full | Partial | High | Partial |

---

## 11. Open Questions and Future Work

### 11.1 Resolved Questions

1. **Q**: Can we achieve 80% annotation-free code?
   **A**: Yes, by combining Lobster's AST-based inference with NLL-style liveness analysis.

2. **Q**: When should inference fail vs. insert runtime checks?
   **A**: Fail for ambiguity, never silently insert runtime checks.

3. **Q**: How do we handle self-referential structures?
   **A**: Tier 3 (@shared) or arena allocation pattern.

### 11.2 Open Questions

1. **Effect interaction**: How does ownership inference interact with effect system?
2. **Concurrency**: Should @shared automatically use atomic refcount?
3. **WASM target**: Can we eliminate ARC overhead for single-threaded WASM?
4. **Incremental analysis**: How to do efficient incremental ownership checking in LSP?

---

## 12. Key Resources

1. [Lobster Memory Management](https://aardappel.github.io/lobster/memory_management.html)
2. [Swift ARC Optimization](https://apple-swift.readthedocs.io/en/latest/ARCOptimization.html)
3. [WWDC21: ARC in Swift](https://developer.apple.com/videos/play/wwdc2021/10216/)
4. Rust RFC 2094 - Non-Lexical Lifetimes
5. Polonius: Rust's New Borrow Checker
6. [Vale: Zero-Cost Memory Safety](https://verdagon.dev/blog/zero-cost-memory-safety-regions-overview)

---

## 13. Conclusion

Aria's Hybrid Ownership Model achieves the 80% annotation-free target by:

1. **Leveraging Lobster's insight**: AST-based ownership inference with function specialization
2. **Adopting Rust's precision**: NLL-style CFG analysis for accurate liveness
3. **Providing Swift's escape hatch**: ARC for complex shared structures
4. **Learning from Vale**: Region concepts for zero-cost borrowing scopes

The three-tier architecture provides clear guidance for developers:
- Write simple code, get safe memory management automatically
- Add annotations when the compiler asks for help
- Use @shared when data structures are inherently cyclic or shared

This design makes memory safety accessible to developers at all skill levels while preserving the performance benefits of compile-time ownership analysis.

---

**Document Status**: Complete
**Next Steps**: ARIA-M02-05 - Prototype ownership analyzer
**Author**: FORGE Research Agent
