# ARIA-M03-01: Survey of Algebraic Effects Implementations

**Task ID**: ARIA-M03-01
**Status**: Completed
**Date**: 2026-01-14
**Focus**: Study algebraic effects in research and production languages

---

## Executive Summary

Algebraic effects provide a principled way to handle side effects (IO, exceptions, state, async) with composability and type safety. This survey analyzes implementations across Koka, Eff, Unison, and OCaml 5 to inform Aria's effect system design.

---

## 1. What Are Algebraic Effects?

### 1.1 Core Concepts

**Effects** are computational side effects that can be:
- **Declared**: What effects a function may perform
- **Handled**: How effects are interpreted
- **Composed**: Multiple effects combined cleanly

**Effect Handlers** intercept effect operations and provide implementations.

```koka
// Declare an effect
effect ask<a>
  ctl ask(): a

// Use the effect
fun greeting(): ask<string> string
  "Hello, " ++ ask() ++ "!"

// Handle the effect
fun main()
  with handler
    ctl ask() resume("World")
  greeting().println  // "Hello, World!"
```

### 1.2 Key Properties

| Property | Description |
|----------|-------------|
| **Abstraction** | Effects abstract over implementation |
| **Composition** | Multiple effects combine naturally |
| **Type safety** | Effect types tracked statically |
| **Resumption** | Handlers can resume, abort, or replay |

---

## 2. Language Implementations

### 2.1 Koka

**Background**: Research language by Daan Leijen (Microsoft Research)

**Key Features**:
- Row-polymorphic effect types
- Evidence passing for efficiency
- Compiles to C without runtime
- Tail-resumptive optimization

**Effect Syntax**:
```koka
// Effect declaration
effect state<s>
  ctl get(): s
  ctl set(x: s): ()

// Effect-polymorphic function
fun increment(): state<int> ()
  val x = get()
  set(x + 1)

// Handler
fun run-state(init: s, action: () -> <state<s>|e> a): e a
  var st := init
  with handler
    return(x) x
    ctl get() resume(st)
    ctl set(x) { st := x; resume(()) }
  action()
```

**Implementation Strategy**:
- **Evidence passing**: Effects compiled to implicit parameters
- **Monadic transformation**: Effects become monadic operations
- **No runtime overhead** for tail-resumptive effects

### 2.2 OCaml 5

**Background**: Production language with effects added in v5.0

**Key Features**:
- One-shot continuations (single resume)
- Fiber-based implementation
- Efficient for concurrency use case
- New syntax in OCaml 5.3+

**Effect Syntax**:
```ocaml
(* Effect declaration *)
type _ Effect.t +=
  | Ask : string Effect.t

(* Effect usage *)
let greeting () =
  "Hello, " ^ Effect.perform Ask ^ "!"

(* Handler *)
let () =
  Effect.Deep.try_with greeting ()
    { effc = fun (type a) (eff : a Effect.t) ->
        match eff with
        | Ask -> Some (fun (k : (a, _) continuation) ->
            continue k "World")
        | _ -> None }
  |> print_endline
```

**Implementation Strategy**:
- **Fibers**: Heap-allocated stack segments
- **One-shot restriction**: Continuations used at most once
- **Efficient**: Designed for async/concurrency patterns

### 2.3 Unison

**Background**: Content-addressed functional language

**Key Features**:
- Effects called "abilities"
- Built-in to core language
- Multi-shot continuations
- Distributed computing focus

**Ability Syntax**:
```unison
-- Ability declaration
ability Ask a where
  ask : a

-- Using ability
greeting : '{Ask Text} Text
greeting = '("Hello, " ++ ask ++ "!")

-- Handler
runAsk : '{Ask a, g} r -> a -> '{g} r
runAsk prog default =
  handle !prog with cases
    { ask -> resume } -> handle resume default with cases ...
    { r } -> r
```

### 2.4 Eff

**Background**: Research language for studying effects

**Key Features**:
- Multi-shot continuations
- Full effect polymorphism
- Academic reference implementation
- Compilation to OCaml

**Effect Syntax**:
```eff
effect Decide : unit -> bool

let choose_all = handler
  | effect Decide () k ->
      (k true) @ (k false)  (* Multi-shot! *)
  | x -> [x]

let example () =
  let x = perform (Decide ()) in
  let y = perform (Decide ()) in
  (x, y)

(* choose_all example () = [(true,true), (true,false), (false,true), (false,false)] *)
```

---

## 3. Handler Lookup Strategies

### 3.1 Dynamic Search (Eff, Unison, OCaml)

```
call stack: [main] -> [f] -> [g] -> [perform Ask]
                ^        ^
                |        L-- Look for handler here first
                L----------- Then here if not found
```

**Pros**: Simple implementation, intuitive scoping
**Cons**: Runtime lookup cost, harder to optimize

### 3.2 Static Routing (Koka)

```
evidence passing: Ask.evidence flows as implicit parameter
                  Handler creates evidence, passes to callees
```

**Pros**: No runtime search, enables optimization
**Cons**: More complex compilation, evidence threading

---

## 4. Effect Handler Comparison

| Feature | Koka | OCaml 5 | Unison | Eff |
|---------|------|---------|--------|-----|
| **Continuations** | Multi-shot | One-shot | Multi-shot | Multi-shot |
| **Effect typing** | Row polymorphic | Extensible variants | Abilities | Row polymorphic |
| **Handler lookup** | Static (evidence) | Dynamic | Dynamic | Dynamic |
| **Compilation** | To C | Native | Bytecode | To OCaml |
| **Tail-resumptive opt** | Yes | Yes | Partial | No |
| **Production ready** | Experimental | Yes | Experimental | Research |

---

## 5. Performance Overhead Analysis

### 5.1 Costs of Effect Handlers

| Operation | Cost |
|-----------|------|
| Effect declaration | Zero (compile-time) |
| Handler installation | Stack/heap allocation |
| Effect performance | Handler lookup + context switch |
| Resumption | Continuation restoration |

### 5.2 Optimization Techniques

**Tail-Resumptive Effects**:
```koka
// Handler that immediately resumes
ctl get() resume(st)  // Tail-resumptive: optimized to function call
```

**Evidence Passing** (Koka):
- Effects compiled to dictionary passing
- No runtime handler search
- Inlining opportunities

**One-Shot Restriction** (OCaml):
- Enables direct-style implementation
- No copying of continuations
- Lower memory overhead

### 5.3 Benchmarks

From Koka research:
- **Tail-resumptive effects**: ~0% overhead vs direct code
- **General handlers**: 2-5x slowdown
- **With optimization**: Approaches native performance

From OCaml 5:
- **Async patterns**: Competitive with hand-written code
- **Concurrency**: Efficient fiber switching
- **General effects**: Some overhead vs direct style

---

## 6. User Ergonomics Evaluation

### 6.1 Syntax Comparison

**Effect Declaration**:
```
Koka:    effect ask<a> { ctl ask(): a }
OCaml:   type _ Effect.t += Ask : string Effect.t
Unison:  ability Ask a where ask : a
```

**Effect Usage**:
```
Koka:    ask()
OCaml:   Effect.perform Ask
Unison:  ask
```

**Handler**:
```
Koka:    with handler { ctl ask() resume("x") }
OCaml:   Effect.Deep.try_with f () { effc = ... }
Unison:  handle !prog with cases { ask -> resume } -> ...
```

### 6.2 Ergonomics Ranking

| Aspect | Koka | OCaml 5 | Unison |
|--------|------|---------|--------|
| Syntax simplicity | High | Low | High |
| Type inference | Good | Complex | Good |
| Error messages | Good | Improving | Good |
| IDE support | Limited | Good | Limited |
| Documentation | Good | Growing | Good |

### 6.3 Common Pain Points

1. **Handler verbosity**: Lots of boilerplate for simple handlers
2. **Type complexity**: Row polymorphism can be confusing
3. **Debugging**: Stack traces through handlers unclear
4. **Composition**: Ordering handlers affects behavior

---

## 7. Effect Inference Possibilities

### 7.1 What Can Be Inferred

| Aspect | Inference Feasibility |
|--------|----------------------|
| Effect set of function | High (from body analysis) |
| Effect parameters | Medium (from usage) |
| Handler requirements | High (from unhandled effects) |
| Effect ordering | Low (often ambiguous) |

### 7.2 Koka's Approach

```koka
// Effects inferred from body
fun example()     // Inferred: () -> <console,state<int>> ()
  println("hi")   // console effect
  set(get() + 1)  // state<int> effect
```

### 7.3 Aria Opportunity

**Infer effect annotations entirely**:
```aria
# User writes (no effect annotations):
fn fetch_user(id)
  data = http.get("/users/#{id}")  # IO effect inferred
  parse_json(data)
end

# Compiler infers:
# fetch_user : Int -> {IO} User
```

---

## 8. Recommendations for Aria

### 8.1 Core Design Decisions

| Decision | Recommendation | Rationale |
|----------|----------------|-----------|
| Continuation model | **One-shot** | Simpler, covers most use cases |
| Handler lookup | **Hybrid** (static preferred) | Performance + flexibility |
| Effect typing | **Row polymorphic** | Composability |
| Syntax | **Minimal/inferred** | Ergonomics focus |

### 8.2 Proposed Aria Effect System

**Effect Declaration** (rarely needed):
```aria
effect IO
effect State[T]
effect Async
```

**Effect Usage** (implicit):
```aria
fn read_file(path)
  # IO effect inferred from File.read
  File.read(path)
end
```

**Handling** (only at boundaries):
```aria
fn main
  # Effects automatically handled at program boundary
  data = read_file("config.json")
  process(data)
end

# Explicit handling for custom behavior:
fn with_retry(action, times: 3)
  handle action()
    on IO.Error(e) ->
      if times > 0 then with_retry(action, times - 1)
      else raise e
  end
end
```

### 8.3 Key Innovations for Aria

1. **Effect inference by default**: No explicit effect annotations
2. **Automatic async**: IO effects automatically concurrent when possible
3. **Implicit handlers**: Standard handlers at program boundaries
4. **Escape hatch**: Explicit effects when needed

### 8.4 Effect Hierarchy

```
Pure < State < IO < Async
      ^
      |
   Exceptions
```

- Functions default to Pure
- Effects inferred up the hierarchy
- Incompatible effects (e.g., State + Async) flagged

---

## 9. Key Resources

1. **Leijen** - "Algebraic Effects for Functional Programming" (2017)
2. **Leijen** - "Koka: Programming with Row-Polymorphic Effect Types"
3. **Kammar et al.** - "Handlers in Action" (2013)
4. **Sivaramakrishnan** - "Retrofitting Effect Handlers onto OCaml"
5. **Pretnar** - "An Introduction to Algebraic Effects and Handlers"

---

## 10. Open Questions

1. How do effects interact with ownership/borrowing?
2. Can we infer async behavior from IO patterns?
3. What's the minimal syntax for effect handling?
4. How do effects compose with contracts?

---

## Appendix: Effect System Comparison Table

| Language | Effect Tracking | Inference | Handlers | Async Model |
|----------|-----------------|-----------|----------|-------------|
| Koka | Row polymorphic | Full | First-class | Effects |
| OCaml 5 | Type extensions | Partial | First-class | Effects/Fibers |
| Unison | Abilities | Full | First-class | Effects |
| Haskell | Monads/MTL | None | Via monads | Async monad |
| Rust | None (traits) | N/A | N/A | async/await |
| Go | None | N/A | N/A | goroutines |
| **Aria (proposed)** | Row polymorphic | **Full** | First-class | **Auto-inferred** |
