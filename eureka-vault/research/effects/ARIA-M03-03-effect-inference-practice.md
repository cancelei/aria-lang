# ARIA-M03-03: Effect Inference in Practice

**Task ID**: ARIA-M03-03
**Status**: Completed
**Date**: 2026-01-14
**Focus**: Research automatic effect inference approaches

---

## Executive Summary

Effect inference allows compilers to automatically determine what side effects a function may perform. This research surveys inference algorithms, annotation requirements, and error message strategies.

---

## 1. History of Effect Inference

### 1.1 Evolution

| Era | Approach | Key Work |
|-----|----------|----------|
| 1980s | Effect systems concept | Lucassen & Gifford |
| 1990s | Effect inference algorithms | Jouvelot & Gifford (FX) |
| 1990s | Region inference | Talpin & Jouvelot |
| 2010s | Row-polymorphic effects | Koka (Leijen) |
| 2020s | Practical implementations | OCaml 5, Effekt |

### 1.2 Shift from Analysis to Type Systems

Early work treated effects as static analysis. Modern approaches integrate effects into the type system:
- Effects become part of function types
- Inference uses standard type inference techniques
- Better compositionality and error messages

---

## 2. Inference Algorithm Options

### 2.1 Constraint-Based Inference

Similar to HM type inference:

```
1. Generate fresh effect variables for each expression
2. Collect constraints from operations:
   - print(x) → e ⊇ {console}
   - read_file(p) → e ⊇ {io, exn}
   - pure computation → e = {}
3. Solve constraints via unification
4. Generalize unconstrained effect variables
```

**Example**:
```
fn foo(x)
  if x > 0
    print("positive")    // Constraint: e ⊇ {console}
  else
    0                    // Constraint: e ⊇ {}
  end
end

// Solution: e = {console}
// Inferred: fn foo(x: Int) -> {console} Int
```

### 2.2 Bidirectional Effect Checking

Propagate known effects both directions:

```
Checking mode:  If expected effect is {io}, use it
Synthesis mode: Infer effect from expression

fn example() -> {io} Int    // Known: {io}
  read_file("x")            // Check: {io} ⊇ {io, exn}?
                            // Error: exn not in expected!
```

### 2.3 Effect Polymorphism

**Rank-1 Effect Polymorphism** (simple):
```
fn map[A, B, E](f: A -> E B, xs: List[A]) -> E List[B]
```

**Higher-Rank Effects** (complex):
```
fn run_st[A](action: forall H. () -> {State[H]} A) -> A
// Effect variable H is scoped to action
```

---

## 3. Languages with Effect Inference

### 3.1 Koka

**Approach**: Full inference with row polymorphism

```koka
fun example()              // No annotation needed
  println("hello")         // console effect inferred

// Inferred: fun example(): <console> ()
```

**Strengths**: Complete inference, excellent error messages
**Weaknesses**: Complex type signatures can be overwhelming

### 3.2 Effekt

**Approach**: Contextual effect polymorphism

```effekt
def map[A, B](xs: List[A]) { f: A => B }: List[B]
// Effect of f is captured by context, not explicit
```

**Strengths**: Simpler types in common cases
**Weaknesses**: Less explicit than Koka

### 3.3 OCaml 5

**Approach**: Effects via type extensions (partial inference)

```ocaml
(* Effect must be declared *)
type _ Effect.t += Ask : string Effect.t

(* Usage inferred *)
let greeting () = "Hello, " ^ Effect.perform Ask
```

**Strengths**: Practical, good performance
**Weaknesses**: Less inference than Koka

---

## 4. Annotation Requirements

### 4.1 When Annotations Are Needed

| Scenario | Annotation Requirement |
|----------|------------------------|
| Simple functions | None |
| Polymorphic functions | Effect variable bounds |
| Handler installation | Handler type |
| Module boundaries | Often required for docs |
| Recursive effects | May need guidance |

### 4.2 Annotation Burden Comparison

| Language | Typical Annotation Burden |
|----------|---------------------------|
| Koka | Very low (full inference) |
| Effekt | Low |
| OCaml 5 | Medium (declare effects) |
| Haskell (monads) | High (explicit monad types) |

### 4.3 When to Require Annotations

**Recommendation for Aria**:
- **Infer** within function bodies
- **Require** at module public boundaries (like types)
- **Optional** explicit annotations for documentation

```aria
# Public API: annotation encouraged
@pub fn read_config(path: String) -> {IO, Parse.Error} Config
  # ...
end

# Internal: fully inferred
fn helper(x)
  # Effects inferred
end
```

---

## 5. Error Message Strategies

### 5.1 Common Error Types

| Error | Cause | Message Strategy |
|-------|-------|------------------|
| Effect mismatch | Function has unexpected effect | Show expected vs actual |
| Unhandled effect | Effect not handled | Suggest handler |
| Effect escape | Effect leaks from scope | Show escape path |

### 5.2 Good Error Messages

**Bad**:
```
Error: Effect mismatch
  Expected: <>
  Found: <io|e>
```

**Good**:
```
Error: Function `process` has IO effects but expected to be pure

  fn process(x) -> Int   // Expected: pure (no effects)
      ~~~~~~~~

  The function performs IO at line 5:
    5 |   File.read(path)
              ^^^^ This performs {IO} effect

  To fix, either:
  1. Handle the IO effect with a handler
  2. Declare the effect: fn process(x) -> {IO} Int
```

### 5.3 Effect Origin Tracking

Track where effects come from:

```aria
# Internal representation
EffectOrigin {
  effect: IO,
  location: file.aria:5:3,
  operation: "File.read",
  propagation_path: [
    {fn: "helper", line: 5},
    {fn: "process", line: 12},
  ]
}
```

---

## 6. Decidability and Complexity

### 6.1 Decidability Results

| Feature | Effect Inference |
|---------|------------------|
| Simple effects | Decidable |
| Row polymorphism | Decidable |
| Subtyping + polymorphism | May be undecidable |
| Higher-rank effects | Requires annotations |

### 6.2 Complexity

- Basic effect inference: O(n) typical, O(n³) worst case
- With row polymorphism: Similar to HM
- With subtyping: Constraint solving can explode

### 6.3 Practical Considerations

**Termination**: Always terminates with proper bounds
**Performance**: Fast in practice (similar to type inference)
**Predictability**: Users should understand what's inferred

---

## 7. Integration with Other Features

### 7.1 Effects + Ownership

```aria
# How do effects interact with moves?
fn consume(data: Data) -> {IO}
  File.write(data)      # data is moved AND IO effect
end
```

**Consideration**: Effect must be tracked even after ownership transfer.

### 7.2 Effects + Generics

```aria
fn map[A, B, E](items: Array[A], f: (A) -> E B) -> E Array[B]
# E is effect variable, polymorphic over effects
```

### 7.3 Effects + Async

**Key Question**: Is async an effect or special syntax?

**Option A**: Async as effect
```aria
fn fetch() -> {Async, IO} Data
```

**Option B**: Async inferred from IO
```aria
fn fetch() -> {IO} Data  # Compiler knows IO may be async
```

**Recommendation**: Option B (less annotation burden)

---

## 8. Recommendations for Aria

### 8.1 Inference Strategy

```
1. Full inference within function bodies
2. Effect signatures at public boundaries (optional but encouraged)
3. Propagate effects through call graph
4. One-shot handlers for simplicity
```

### 8.2 Effect Type Display

**In IDE** (hover):
```aria
fn process(data)  # Hover shows: -> {IO, Parse.Error} Result
```

**In errors**:
```
Error: Unhandled effect {IO}
  Consider adding a handler or declaring the effect
```

### 8.3 Proposed Inference Algorithm

```python
def infer_effects(function):
    # Phase 1: Collect effect constraints
    constraints = []
    for expr in function.body:
        if is_effectful_call(expr):
            constraints.append(Effect(expr.callee.effects))
        elif is_handler(expr):
            constraints.append(Handles(expr.handled_effect))

    # Phase 2: Solve constraints
    effect_set = solve(constraints)

    # Phase 3: Generalize
    if function.is_polymorphic:
        effect_set = generalize(effect_set)

    return effect_set
```

### 8.4 Handling Async

```aria
# IO effects are automatically async-capable
fn fetch_all(urls: Array[String]) -> {IO} Array[Response]
  urls.map |url|
    HTTP.get(url)  # These run concurrently automatically
  end
end
```

---

## 9. Key Resources

1. [Jouvelot & Gifford - Algebraic Reconstruction of Types and Effects](https://dl.acm.org/doi/10.1145/99370.99383)
2. [Leijen - Koka: Programming with Row-polymorphic Effect Types](https://arxiv.org/abs/1406.2061)
3. [Effekt Language](https://effekt-lang.org/)
4. [Deciding not to Decide: Sound and Complete Effect Inference](https://arxiv.org/abs/2510.20532)

---

## 10. Open Questions

1. How verbose should effect types be in IDE display?
2. Should effects be part of function identity (for caching)?
3. How do effects interact with contracts?
4. What's the migration path for code without effects?
