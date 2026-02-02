# ARIA-M03-02: Koka's Effect System Analysis

**Task ID**: ARIA-M03-02
**Status**: Completed
**Date**: 2026-01-14
**Focus**: Deep dive into Koka's practical effect system

---

## Executive Summary

Koka, developed by Daan Leijen at Microsoft Research, pioneered practical algebraic effects with row-polymorphic effect types. This research analyzes Koka's approach to inform Aria's effect system design.

---

## 1. Koka Overview

### 1.1 Design Philosophy

Koka is a **functional-first** language where:
- All side effects are tracked in the type system
- Effect types are inferred automatically
- Effects are handled with algebraic effect handlers
- Pure functions are guaranteed pure by types

### 1.2 Effect Example

```koka
fun divide(x: int, y: int): exn int
  if y == 0 then throw("division by zero")
  else x / y

fun safe-divide(x: int, y: int): int
  with handler
    ctl throw(msg) 0  // Handle exception, return 0
  divide(x, y)
```

---

## 2. Row-Polymorphic Effect Types

### 2.1 Effect Row Syntax

Effects are expressed as **rows** of effect labels:

```
Effect Row := ⟨⟩                    // Empty (pure)
           | ε                      // Effect variable
           | ⟨l | ε⟩               // Effect l extended with ε
```

**Examples**:
- `⟨⟩` - Pure, no effects
- `⟨exn⟩` - May throw exceptions
- `⟨io, exn⟩` - IO and exceptions
- `⟨state<int> | ε⟩` - State plus any other effects ε

### 2.2 Why Row Polymorphism?

**Problem with subtyping**:
```
// With subtyping: complex constraints
f : ∀e. (e ≤ {io, exn}) ⇒ () -> e ()
```

**With row polymorphism**:
```koka
// Simple, composable
fun f(): <io, exn | e> ()
// Effect-polymorphic: can have additional effects e
```

### 2.3 Duplicate Labels

Koka allows duplicate effect labels in rows:
```koka
fun nested-state(): <state<int>, state<string>> ()
  // Two separate state effects
```

---

## 3. Effect Inference

### 3.1 Automatic Inference

Koka infers effect types from function bodies:

```koka
fun greet(name: string)  // Effect inferred!
  println("Hello, " ++ name)

// Inferred type: fun greet(name: string): <console> ()
```

### 3.2 Inference Algorithm

Based on Hindley-Milner with effect rows:

```
1. Assign fresh effect variables to expressions
2. Generate constraints from operations:
   - println(...) generates: e ⊇ ⟨console⟩
   - throw(...) generates: e ⊇ ⟨exn⟩
3. Unify effect rows
4. Generalize remaining effect variables
```

### 3.3 Effect Abstraction

```koka
// Effect-polymorphic map
fun map(xs: list<a>, f: (a) -> e b): e list<b>
  match xs
    Nil -> Nil
    Cons(x, xx) -> Cons(f(x), map(xx, f))

// Usage with any effect:
map([1,2,3], fn(x) { println(x); x + 1 })  // console effect
map([1,2,3], fn(x) { x * 2 })               // pure
```

---

## 4. Effect Handlers

### 4.1 Basic Handler Syntax

```koka
effect ask<a>
  ctl ask(): a

fun greeting(): ask<string> string
  "Hello, " ++ ask() ++ "!"

fun main()
  with handler
    ctl ask() resume("World")
  greeting().println
```

### 4.2 Handler Semantics

| Operation | Meaning |
|-----------|---------|
| `ctl op() resume(x)` | Handle op, resume with value x |
| `ctl op() ...` (no resume) | Handle op, abort computation |
| `return(x) ...` | Transform final result |
| `finally ...` | Cleanup (like try-finally) |

### 4.3 Multi-shot Continuations

```koka
effect choice
  ctl choose(): bool

fun ambiguous(): choice int
  if choose() then 1 else 2

fun all-choices(action)
  with handler
    ctl choose()
      resume(True) ++ resume(False)  // Call resume twice!
    return(x) [x]
  action()

// all-choices(ambiguous) == [1, 2]
```

---

## 5. Effect Handlers Implementation

### 5.1 Evidence Passing

Koka compiles effects to **evidence passing**:

```koka
// Source
fun greet(): <ask<string>> string
  "Hello, " ++ ask()

// Compiled (conceptual)
fun greet(ev: evidence<ask<string>>): string
  "Hello, " ++ ev.ask()
```

### 5.2 Compilation Strategies

| Strategy | Description | Performance |
|----------|-------------|-------------|
| **Evidence passing** | Implicit parameters | Low overhead |
| **CPS transformation** | Continuation-passing | Moderate |
| **Monadic** | Effect as monad | Higher overhead |

### 5.3 Tail-Resumptive Optimization

```koka
// Tail-resumptive handler (optimized to direct call)
with handler
  ctl get() resume(st)  // Last action is resume
  ctl set(x) { st := x; resume(()) }  // Last action is resume
```

When handler's last action is `resume()`, Koka optimizes to a function call.

---

## 6. Built-in Effects

### 6.1 Core Effects

| Effect | Description |
|--------|-------------|
| `total` | Pure, no effects |
| `exn` | May throw exceptions |
| `div` | May diverge (non-termination) |
| `ndet` | Non-deterministic |
| `alloc<h>` | Heap allocation in region h |
| `read<h>` | Read from region h |
| `write<h>` | Write to region h |
| `io` | General I/O |
| `console` | Console I/O |
| `net` | Network I/O |
| `fsys` | File system |

### 6.2 Effect Aliases

```koka
alias st<h> = <alloc<h>, read<h>, write<h>>  // State effect
alias pure = <>                                // No effects
alias io = <console, net, fsys, ...>          // All I/O
```

---

## 7. State Effect Deep Dive

### 7.1 Safe Encapsulation

Like Haskell's `runST`, Koka safely encapsulates state:

```koka
fun run-state<a>(init: s, action: forall<h> () -> <state<h, s>> a): a
  var st := init
  with handler
    ctl get() resume(st)
    ctl set(x) { st := x; resume(()) }
  action()

// State effect doesn't escape!
val result = run-state(0) {
  set(get() + 1)
  get()
}
// result : int (pure!)
```

### 7.2 Region-Based State

```koka
fun local-state()
  with run-state(0)           // Region 1
    with run-state("hello")   // Region 2
      set(get() + 1)          // Which state? Region 1 (int)
      // Region 2 is string-typed, so unambiguous
```

---

## 8. Recommendations for Aria

### 8.1 Row-Polymorphic Effects

**Adopt** Koka's row-polymorphic approach:

```aria
# Effect declaration
effect State[S]
  fn get() -> S
  fn set(value: S)
end

# Effect-polymorphic function
fn map[A, B, E](items: Array[A], f: (A) -> E B) -> E Array[B]
  items.map_with(f)
end
```

### 8.2 Automatic Inference

**Adopt** full effect inference:

```aria
# User writes (no annotations):
fn read_config(path: String)
  content = File.read(path)    # IO effect inferred
  parse_json(content)
end

# Compiler infers: fn read_config(path: String) -> {IO} Config
```

### 8.3 Handler Syntax

**Adapt** Koka's handler syntax for Aria:

```aria
fn with_retry(action, times: 3)
  handle action()
    on IO.Error(e) ->
      if times > 0
        with_retry(action, times - 1)
      else
        raise e
      end
  end
end
```

### 8.4 Effect Hierarchy for Aria

```
Pure < State < Exception < IO < Async
                    │
                    └── Effects can combine: {State, Exception}
```

### 8.5 Key Differences from Koka

| Aspect | Koka | Aria (Proposed) |
|--------|------|-----------------|
| Syntax | ML-style | Ruby-style |
| Continuations | Multi-shot | One-shot (simpler) |
| Handler location | Explicit | Often implicit at boundaries |
| Effect visibility | Always shown | Inferred, shown on hover |

---

## 9. Key Resources

1. [Koka Paper: Row-Polymorphic Effect Types](https://arxiv.org/abs/1406.2061)
2. [Koka Language Book](https://koka-lang.github.io/koka/doc/book.html)
3. [Type Directed Compilation of Row-Typed Algebraic Effects (POPL'17)](https://www.microsoft.com/en-us/research/publication/type-directed-compilation-row-typed-algebraic-effects/)
4. [Koka GitHub](https://github.com/koka-lang/koka)

---

## 10. Open Questions

1. How does effect inference interact with ownership inference?
2. Should Aria support multi-shot continuations for advanced patterns?
3. What's the syntax for custom effect handlers?
4. How do effects interact with async/await patterns?
