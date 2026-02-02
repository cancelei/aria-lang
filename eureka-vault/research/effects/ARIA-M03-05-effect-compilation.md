# ARIA-M03-05: Effect Compilation Strategies

**Task ID**: ARIA-M03-05
**Status**: Completed
**Date**: 2026-01-15
**Focus**: Compilation strategies for algebraic effect handlers and async
**Research Agent**: PRISM (Eureka Iteration 2)

---

## Executive Summary

This document analyzes compilation strategies for algebraic effect handlers, comparing CPS transformation, evidence-passing, state machine generation, and delimited continuations. The goal is to identify the most efficient approach for Aria's effect system compilation, with specific focus on Cranelift backend compatibility and achieving near-zero overhead for common effect patterns.

**Key Findings**:
1. **Evidence-passing** (Koka's approach) achieves the best performance for tail-resumptive effects
2. **Selective CPS transformation** minimizes overhead by only transforming code that actually uses control effects
3. **State machine generation** (Rust/C# style) offers excellent performance for async but lacks generality for full algebraic effects
4. **Cranelift** requires careful design for stack-switching but recent work on exceptions and WasmFX provides a foundation
5. **FFI boundaries** require careful handling of continuation capture and unwinding

---

## 1. Compilation Strategy Overview

### 1.1 Strategy Comparison Matrix

| Strategy | Overhead (Tail-Resumptive) | Overhead (General) | Continuation Support | Implementation Complexity |
|----------|---------------------------|-------------------|---------------------|--------------------------|
| **Evidence-Passing** | ~0% | 50-200% | Multi-shot possible | Medium |
| **CPS Transformation** | 100-300% | 100-300% | Natural multi-shot | Low |
| **Selective CPS** | ~0% for pure code | 100-300% for effectful | Multi-shot possible | High |
| **Capability-Passing** | ~0% for zero-cost subset | 50-100% | One-shot typical | Medium-High |
| **State Machine** | N/A (async only) | 50-100% for async | None (pre-defined states) | Low |
| **Fiber/Stack-Based** | Small (stack switch) | Small (stack switch) | One-shot natural | Medium (runtime) |

### 1.2 Target Use Cases for Aria

| Use Case | Frequency | Performance Requirement | Best Strategy |
|----------|-----------|------------------------|---------------|
| Exception handling | Very high | Zero-cost happy path | Evidence-passing |
| State/Reader effects | High | Near-zero overhead | Evidence-passing + tail-opt |
| Async/IO | High | Competitive with native async | Selective CPS or State Machine |
| Generators/Iterators | Medium | Minimal allocation | Selective CPS |
| Backtracking/Choice | Low | Acceptable overhead | Full CPS |
| Probabilistic programming | Very low | Multi-shot required | Full CPS + copy |

---

## 2. Evidence-Passing Translation (Koka's Approach)

### 2.1 Core Mechanism

Evidence-passing compiles effect operations by explicitly threading an **evidence vector** through function calls. This vector contains references to the handlers currently in scope.

```
# Source (conceptual Aria)
effect State[T]:
  get: () -> T
  set: (T) -> ()

fn increment(): State[Int]
  x = get()
  set(x + 1)

# Compiled (evidence-passing)
fn increment(ev: EvidenceVector): ()
  state_handler = ev.lookup(State)
  x = state_handler.get()
  state_handler.set(x + 1)
```

### 2.2 Evidence Vector Optimization

**Problem**: Naive evidence passing requires O(n) lookup where n is the number of handlers in scope.

**Koka's Solution**: Type-directed evidence indexing with constant-time array access.

```
# Instead of runtime search:
handler = ev.search("State")  # O(n)

# Use type-directed offset:
handler = ev[STATE_OFFSET]    # O(1)
```

**Open Floating Optimization** ([Springer, 2022](https://link.springer.com/chapter/10.1007/978-3-031-21314-4_9)):
- Float `open` constructs upward in the call tree
- Combine multiple evidence adjustments into single operations
- Measured improvement: **2.5x** in benchmarks

### 2.3 Tail-Resumptive Optimization

Most handlers are **tail-resumptive**: the last thing they do is call `resume`. This pattern can be compiled to direct function calls without capturing continuations.

```
# Tail-resumptive handler
handle x with
  get() -> resume(current_state)     # Tail position!
  set(v) -> { state = v; resume(()) } # Tail position!

# Compiles to (optimized):
fn optimized_get(state):
  return state  # Direct return, no continuation

fn optimized_set(state, v):
  return (v, ())  # Return new state + unit
```

**Performance Impact** ([Microsoft Research](https://www.microsoft.com/en-us/research/publication/generalized-evidence-passing-for-effect-handlers/)):
- Tail-resumptive: **~150 million operations/second**
- Non-tail-resumptive: **~1.3 million operations/second**
- Difference: **~115x** performance improvement

### 2.4 Monadic Translation and Yield Bubbling

When continuations must be captured (non-tail-resumptive), Koka uses:

1. **Monadic bind**: Transform effectful code into continuation-style
2. **Yield bubbling**: Propagate yields up the call stack efficiently

```
# Monadic translation
fn example(): Effect[A]
  x <- operation1()
  y <- operation2(x)
  pure(x + y)

# With yield bubbling (optimized)
fn example(ev):
  match operation1(ev):
    Pure(x) ->
      match operation2(x, ev):
        Pure(y) -> Pure(x + y)
        Yield(op, k) -> Yield(op, compose(k, \y -> Pure(x + y)))
    Yield(op, k) -> Yield(op, compose(k, \x -> operation2_cont(x, ev)))
```

**Bind-Inlining Optimization**: The Koka compiler inlines bind operations and leaves out monadic binds for total (pure) functions, significantly reducing code expansion.

---

## 3. CPS Transformation Strategies

### 3.1 Full CPS Transformation

Every function is transformed to take a continuation parameter.

```
# Source
fn add(a, b):
  a + b

fn example():
  x = add(1, 2)
  y = add(x, 3)
  y

# Full CPS
fn add_cps(a, b, k):
  k(a + b)

fn example_cps(k):
  add_cps(1, 2, \x ->
    add_cps(x, 3, \y ->
      k(y)))
```

**Problems**:
- **Code bloat**: Every function grows by one parameter
- **Optimization barriers**: Harder for backend optimizers (LLVM, Cranelift)
- **Tail calls**: All calls become tail calls (can be good or bad)
- **Debugging**: Stack traces become continuation chains

### 3.2 Selective CPS Transformation

Only transform functions that actually use control effects ([ACM PEPM 2018](https://dl.acm.org/doi/10.1145/3162069)).

**Type-and-Effect Analysis**:
```
# Annotate based on effect usage
fn pure_add(a, b):          # No effects -> direct style
  a + b

fn effectful_choose(): Choice[Bool]  # Has effects -> CPS
  perform(Choose())

# Only effectful code is CPS transformed
fn example(): Choice[Int]
  x = pure_add(1, 2)        # Direct call
  if effectful_choose() then  # CPS call
    pure_add(x, 10)
  else
    pure_add(x, 20)
```

**Koka's Type-Selective Approach** ([Microsoft Research](https://www.microsoft.com/en-us/research/wp-content/uploads/2016/12/algeff.pdf)):
- Use effect types to determine which code needs CPS
- Pure code (no effects) remains in direct style
- Only effectful code paths get monadic transformation

**Performance**: Selective CPS achieves near-zero overhead for pure code paths, with CPS overhead only where effects are actually used.

### 3.3 Eff's Optimizing Compilation

The [Eff language compiler](https://dl.acm.org/doi/10.1145/3485479) uses multi-phase optimization:

**Phase 1: ExEff (Explicitly-typed Effects)**
- Intermediate language with explicit effect type information
- Enables type-directed optimizations

**Phase 2: Handler Specialization**
- Inline handler implementations at known call sites
- Specialize generic handlers for specific effect types

**Phase 3: NoEff (Effects Erased)**
- Final monadic calculus without effect annotations
- Standard lambda calculus optimizations apply

**Results**:
- Eliminates much of handler overhead
- Outperforms capability-passing style
- Competitive with hand-written OCaml and Multicore OCaml

---

## 4. State Machine Generation (Async Pattern)

### 4.1 Rust's Async Transformation

Rust compiles `async fn` to state machines without CPS ([Rust Compiler Guide](https://rustc-dev-guide.rust-lang.org/overview.html)).

```rust
// Source
async fn fetch_data() -> Data {
    let response = http_get(url).await;  // Suspend point 1
    let parsed = parse(response).await;   // Suspend point 2
    Data { parsed }
}

// Generated state machine (simplified)
enum FetchDataState {
    Start,
    WaitingForHttp { url: Url },
    WaitingForParse { response: Response },
    Done,
}

impl Future for FetchData {
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Data> {
        match self.state {
            Start => {
                // Start HTTP request, transition to WaitingForHttp
            }
            WaitingForHttp { url } => {
                // Check if HTTP done, transition to WaitingForParse
            }
            // ...
        }
    }
}
```

**Advantages**:
- No heap allocation for synchronous completion
- State machine stays on stack when possible
- Excellent for async/await pattern

**Limitations**:
- Only works for pre-defined suspend points
- Cannot express general algebraic effects
- No dynamic handler installation

### 4.2 C# Async State Machine

C# uses similar transformation ([.NET Blog](https://devblogs.microsoft.com/dotnet/how-async-await-really-works/)):

```csharp
// Generated struct (simplified)
struct <Method>d__0 : IAsyncStateMachine {
    public int <>1__state;
    public AsyncTaskMethodBuilder<T> <>t__builder;
    // Local variables lifted to fields

    void MoveNext() {
        switch (<>1__state) {
            case -1: // Initial
            case 0:  // After first await
            case 1:  // After second await
            // ...
        }
    }
}
```

**Key Insight**: State machine approach works well when:
1. Suspend points are known at compile time
2. Only one "effect" (async) needs handling
3. No dynamic handler composition needed

### 4.3 Loop-Match Optimization

Recent Rust improvements ([Trifecta Tech Foundation, 2025](https://trifectatech.org/blog/improving-state-machine-code-generation/)):

**Problem**: Naive state machine loops generate poor branch prediction:
```rust
loop {
    match state {
        A => { /* ... */ state = B; }
        B => { /* ... */ state = C; }
        C => { /* ... */ state = A; }
    }
}
// Each iteration: unconditional jump back + conditional branch
```

**Solution**: `#[feature(loop_match)]` enables better code generation:
- Deterministic jumps between states
- No repeated branch on state variable
- ~1.2x improvement in state machine performance

---

## 5. Fiber/Stack-Based Implementation

### 5.1 Multicore OCaml's Approach

OCaml 5 implements effect handlers using fibers ([Retrofitting Effect Handlers onto OCaml](https://dl.acm.org/doi/pdf/10.1145/3453483.3454039)):

**Fiber Structure**:
```
+------------------+
| handler_info     | <- Parent fiber pointer, handler closures
+------------------+
| context block    | <- DWARF/GC bookkeeping
+------------------+
| exception handler| <- Top-level exception forwarding
+------------------+
| return address   | <- For returning to parent
+------------------+
| OCaml frames     | <- Variable-sized, grows dynamically
+------------------+
| red zone (16w)   | <- For small tail functions
+------------------+
```

**Stack Overflow Handling**:
- Initial fiber: 16 words
- On overflow: copy to new area with 2x size
- Stack checks in function prologue (branch predictor friendly)

**Continuation Operations**:
- `caml_resume`: Attach fiber to execution stack
- `caml_perform`: Capture continuation, invoke handler

### 5.2 One-Shot vs Multi-Shot Continuations

**One-Shot** (OCaml 5, typical Aria use case):
- Continuation can be resumed at most once
- Simpler resource management
- No continuation copying needed
- Covers: async, exceptions, generators, state

**Multi-Shot** (Eff, probabilistic programming):
- Continuation can be resumed multiple times
- Requires copying continuation
- Higher overhead
- Needed for: backtracking, probabilistic choice, logic programming

**Recommendation for Aria**: Default to one-shot with optional multi-shot via explicit copy operation:
```aria
# One-shot (default)
handle action() with
  choose() -> resume(random_bool())

# Multi-shot (explicit)
handle action() with
  choose() ->
    k = copy_continuation(resume)
    results = [k(true), k(false)]
    results.flatten()
```

---

## 6. Intermediate Representation Requirements

### 6.1 Effect-Aware IR Design

An IR for effect compilation should support:

| Feature | Purpose | Representation |
|---------|---------|----------------|
| Effect annotations | Track which code is effectful | Effect row on function types |
| Handler scopes | Delimit effect handling | Explicit scope markers |
| Yield/Resume | Control flow for effects | First-class operations |
| Evidence slots | Pass handlers efficiently | Implicit parameters or registers |

**Effekt's Multi-Level IR** ([Effekt Implementation](https://effekt-lang.org/docs/implementation)):

```
Source -> Core (System Xi) -> Machine -> LLVM/JS/Chez
         [capability-passing]  [low-level]  [target-specific]
```

### 6.2 SSA and Effect Handlers

Challenge: Effect handlers create non-local control flow that complicates SSA form.

**Options**:
1. **CPS first**: Transform to CPS, then to SSA of CPS'd code
2. **Exception-style**: Use try/catch IR constructs (Cranelift's approach)
3. **Explicit continuations**: First-class continuation values in IR

### 6.3 Proposed Aria IR Extensions

```
# Effect handler installation
%handler = effect.install @State, %impl, %evidence_vector

# Effect operation
%result = effect.perform @State.get, [], %evidence_vector

# Yield operation (when continuation needed)
%cont = effect.capture
effect.yield %operation, %args, %cont

# Resume
effect.resume %cont, %value
```

---

## 7. Cranelift-Specific Considerations

### 7.1 Current Cranelift Capabilities

Based on recent development ([Bytecode Alliance](https://github.com/bytecodealliance/wasmtime/issues/5141)):

**Supported**:
- SSA-based IR with excellent optimization
- Multiple calling conventions
- Exception handling via `try_call` ([Exceptions in Cranelift](https://cfallin.org/blog/2025/11/06/exceptions/))
- Stack maps for GC integration
- Tail call optimization (in progress)

**In Development**:
- `stack_switch` instruction for coroutines
- WasmFX stack-switching integration

**Not Directly Supported**:
- First-class continuations
- Inline assembly injection
- Custom prologue generation

### 7.2 Exception Handling as Foundation

Cranelift's new exception handling provides a foundation for effects:

```
# try_call with multiple handlers
%result = try_call @function(%args)
    normal: %normal_block(%return_values)
    catch @tag1: %handler1_block(%payload)
    catch @tag2: %handler2_block(%payload)
    catch_all: %fallback_block(%exnref)
```

**Key Properties**:
- Zero-cost happy path (no overhead when no exception)
- Side tables for handler lookup (not inline checks)
- Composable with inlining
- GC-integrated exception objects

### 7.3 Implementing Effects on Cranelift

**Strategy 1: Evidence-Passing + Exception Transform**

```
# Tail-resumptive effects: pure function calls with evidence
fn state_get(evidence: *Handler) -> Int:
    evidence.state

# Non-tail-resumptive: use exception mechanism
fn choose() -> Bool:
    raise @ChooseEffect()  # Handler catches and decides
```

**Strategy 2: WasmFX Stack-Switching**

The [WasmFX proposal](https://wasmfx.dev/) adds typed continuations to WebAssembly:

```wasm
;; Six new instructions
cont.new   ;; Create continuation from function
resume     ;; Resume a suspended continuation
suspend    ;; Suspend current computation
cont.bind  ;; Partially apply a continuation
switch     ;; Combined suspend + resume
trap       ;; Trap within continuation
```

**Status**: Implemented in [wasmfxtime](https://github.com/wasmfx/wasmfxtime) fork, being merged into mainline Wasmtime ([Issue #10248](https://github.com/bytecodealliance/wasmtime/issues/10248)).

### 7.4 Stack Switching Implementation

For non-WASM targets, Aria needs native stack switching:

**Option A: Fiber Runtime**
```c
// Runtime support (C)
struct Fiber {
    void* stack_pointer;
    void* stack_base;
    size_t stack_size;
    FiberStatus status;
};

void fiber_switch(Fiber* from, Fiber* to);
void* fiber_yield(void* value);
```

**Option B: OS-Level Support**
- Linux: `ucontext_t` or io_uring
- macOS: Grand Central Dispatch
- Windows: Fibers API

**Option C: Segmented Stacks**
- Split stack model (like Go pre-1.4)
- More complex but doesn't require OS support

### 7.5 Recommended Cranelift Strategy

1. **Phase 1**: Evidence-passing for tail-resumptive effects
   - No Cranelift changes needed
   - Covers 80%+ of use cases

2. **Phase 2**: Use Cranelift exceptions for non-local control
   - Leverage existing exception infrastructure
   - Extends coverage to simple handlers

3. **Phase 3**: Adopt WasmFX stack-switching when stable
   - Full continuation support
   - Best performance for general effects

---

## 8. FFI Boundary Handling

### 8.1 The FFI Challenge

Effect handlers capture the execution context (continuation). When this context crosses FFI boundaries, several problems arise:

1. **Stack format incompatibility**: Foreign stacks may not be capturable
2. **Unwinding semantics**: Different languages have different exception models
3. **Resource lifetime**: Who owns captured resources?

### 8.2 Strategies for FFI Safety

**Strategy 1: Prohibition**
```aria
# Effect handlers cannot capture across FFI
@foreign fn c_function()

fn example():
  handle action() with
    some_effect() ->
      c_function()  # OK: no capture here
      resume(())

  handle action() with
    some_effect() ->
      # ERROR: Cannot capture continuation containing FFI call
      let result = c_function()
      if result then resume(a) else resume(b)
```

**Strategy 2: Boundary Markers**

Mark FFI boundaries explicitly:
```aria
fn with_ffi_boundary(action):
  # Install a "stopper" that prevents continuation capture
  handle action() with
    @ffi_boundary
    nested_effect() ->
      # If resume would cross FFI, convert to callback style
      ...
```

**Strategy 3: Callback Conversion**

When continuation would cross FFI, convert to callback style:
```aria
# Instead of capturing continuation
handle action() with
  async_io() ->
    result = ffi_async_operation()
    resume(result)

# Convert to callback
handle action() with
  async_io() ->
    ffi_async_operation_with_callback(\result ->
      resume(result))
```

### 8.3 Effekt's FFI Approach

The [Effekt language FFI](https://effekt-lang.org/tour/ffi) provides guidance:

> When defining FFI, try to follow the following principle: less foreign types and less foreign functions means more compatibility across backends.

**Key Rules**:
1. Foreign types and functions are opaque to optimizer
2. Minimize foreign type usage for portability
3. Effect operations should not call foreign code directly

### 8.4 Recommendation for Aria

1. **Default**: Disallow continuation capture across FFI boundaries
2. **Explicit opt-in**: `@ffi_safe` annotation for handlers that properly handle boundaries
3. **Documentation**: Clear error messages explaining why capture failed
4. **Escape hatch**: `unsafe` block for advanced use cases

---

## 9. Performance Benchmarks

### 9.1 Effect Handler Benchmarks

From [effect-bench](https://github.com/daanx/effect-bench) repository:

| Benchmark | Description | Key Metric |
|-----------|-------------|------------|
| counter | State effect, get+set per iteration | Tail-resumptive perf |
| counter1 | Counter with 1 unused handler | Handler overhead |
| counter10 | Counter with 10 unused handlers | Evidence lookup |
| mstate | Monadic (non-tail-resumptive) state | General handler perf |
| nqueens | Backtracking N-queens | Multi-shot perf |

### 9.2 Comparative Performance

| System | counter (ops/s) | mstate (ops/s) | nqueens |
|--------|-----------------|----------------|---------|
| Koka (C backend) | ~150M | ~1.3M | Competitive |
| Multicore OCaml | ~100M | ~2M | Good |
| Eff (OCaml) | ~50M | ~1M | Research |
| Hand-written C | ~200M | N/A | Baseline |

### 9.3 Key Performance Insights

1. **Tail-resumptive is critical**: 100x+ difference between optimized and unoptimized
2. **Evidence lookup matters**: 10 handlers = 2-3x slowdown without optimization
3. **Allocation pressure**: Non-tail-resumptive handlers allocate continuations
4. **Backend quality**: C backend (Koka) competitive with native (OCaml)

---

## 10. Recommendations for Aria

### 10.1 Primary Strategy: Hybrid Evidence-Passing

**Core approach**:
1. Evidence-passing for handler routing (constant-time lookup)
2. Tail-resumptive optimization for common cases
3. Selective CPS for non-tail-resumptive handlers
4. State machine generation for `async` as special case

### 10.2 IR Design

```
# Aria Effect IR (AEIR)

# Effect type annotations (for optimization)
@effect_row(IO, State[Int])
fn effectful_function(x: Int) -> Int

# Handler installation (evidence-passing)
%ev = evidence.extend %parent_ev, @State, %handler

# Tail-resumptive effect operation (inlined)
%result = effect.perform.tail @State.get, %ev

# General effect operation (may yield)
%result = effect.perform @Choice.choose, %ev
  on_yield: %yield_block
  on_return: %return_block
```

### 10.3 Compilation Phases

```
Phase 1: Effect Inference
  - Infer effect types for all functions
  - Annotate IR with effect information

Phase 2: Handler Analysis
  - Classify handlers: tail-resumptive vs general
  - Identify handler-free regions

Phase 3: Evidence Optimization
  - Open floating to minimize evidence adjustments
  - Constant propagation for known handlers

Phase 4: Code Generation
  - Tail-resumptive: direct function calls
  - General: yield/resume protocol
  - Async: state machine when beneficial

Phase 5: Backend Lowering
  - Cranelift: use exception mechanism + evidence registers
  - WASM: target WasmFX when available
  - JS: CPS transformation to callbacks
```

### 10.4 Cranelift Integration Plan

**Short-term (Aria v0.1)**:
- Evidence-passing with manual stack management
- Use Cranelift exceptions for non-local control
- Simple fiber runtime in Rust for full continuations

**Medium-term (Aria v0.2)**:
- Integrate with WasmFX stack-switching
- Optimize evidence lookup with Cranelift's register allocation
- Profile-guided optimization for hot paths

**Long-term (Aria v1.0)**:
- Native continuation support if added to Cranelift
- Multi-shot continuations for advanced use cases
- Cross-module effect optimization

### 10.5 Performance Targets

| Pattern | Target | Strategy |
|---------|--------|----------|
| Pure code | 0% overhead | Effect erasure |
| Tail-resumptive effects | <5% overhead | Evidence-passing + inlining |
| Async/IO | Competitive with Rust | State machine hybrid |
| General handlers | <50% overhead | Optimized CPS |
| Multi-shot | Acceptable | Explicit copy |

---

## 11. Open Research Questions

1. **Effect inference accuracy**: Can we infer effects precisely enough to enable aggressive optimization?

2. **Cross-module effects**: How do we handle effects that span module boundaries without whole-program compilation?

3. **Effect handler inlining**: What's the best heuristic for inlining handlers vs keeping them separate?

4. **Interaction with ownership**: How do continuations interact with Aria's ownership system?

5. **Debugging experience**: How do we provide good stack traces through effect handlers?

6. **Async special-casing**: Should `async` be a special effect with dedicated compilation, or a library effect?

---

## 12. Key Resources

### Papers
- [Generalized Evidence Passing for Effect Handlers](https://www.microsoft.com/en-us/research/publication/generalized-evidence-passing-for-effect-handlers/) - Xie & Leijen, ICFP 2021
- [Efficient Compilation of Algebraic Effect Handlers](https://dl.acm.org/doi/10.1145/3485479) - Koprivec et al., OOPSLA 2021
- [Compiling Effect Handlers in Capability-Passing Style](https://dl.acm.org/doi/10.1145/3408975) - Schuster et al., ICFP 2020
- [Retrofitting Effect Handlers onto OCaml](https://dl.acm.org/doi/pdf/10.1145/3453483.3454039) - Sivaramakrishnan et al., PLDI 2021
- [Continuing WebAssembly with Effect Handlers](https://www.microsoft.com/en-us/research/publication/continuing-webassembly-with-effect-handlers/) - Phipps-Costin et al., OOPSLA 2023

### Implementations
- [Koka Compiler](https://github.com/koka-lang/koka) - Evidence-passing to C
- [Effekt Language](https://effekt-lang.org/) - Capability-passing with multiple backends
- [Multicore OCaml](https://github.com/ocaml-multicore/docs) - Fiber-based effects
- [WasmFX/Wasmtime](https://github.com/wasmfx/wasmfxtime) - Stack-switching for WASM
- [Cranelift Exceptions](https://cfallin.org/blog/2025/11/06/exceptions/) - Foundation for effects

### Benchmarks
- [effect-bench](https://github.com/daanx/effect-bench) - Cross-language effect benchmarks

---

## Appendix A: Glossary

| Term | Definition |
|------|------------|
| **Algebraic Effects** | A programming paradigm where side effects are declared as operations and handled by effect handlers |
| **Continuation** | A representation of "the rest of the computation" that can be captured and resumed |
| **CPS** | Continuation-Passing Style - a program transformation where every function takes an explicit continuation parameter |
| **Delimited Continuation** | A continuation that represents computation up to a specific delimiter (prompt), not the entire program |
| **Effect Handler** | A construct that intercepts effect operations and provides implementations, potentially resuming the computation |
| **Effect Row** | A type-level representation of a set of effects, enabling polymorphism over effects |
| **Evidence** | Runtime information about which handler handles which effect, enabling efficient dispatch |
| **Fiber** | A lightweight, user-space thread with its own stack, used to implement continuations |
| **Multi-shot Continuation** | A continuation that can be resumed multiple times, creating multiple execution paths |
| **One-shot Continuation** | A continuation that can be resumed at most once |
| **Tail-Resumptive** | A handler where `resume` is the last operation, enabling optimization to direct calls |
| **Yield** | The operation of suspending the current computation and returning control to the handler |

---

## Appendix B: Code Examples

### B.1 State Effect Compilation (Evidence-Passing)

```aria
# Source
effect State[T]:
  get: () -> T
  set: (T) -> ()

fn counter(n: Int): State[Int] -> Int
  if n == 0 then
    get()
  else
    x = get()
    set(x + 1)
    counter(n - 1)

fn run_counter():
  with_state(0):
    counter(1000000)

# Compiled (simplified)
struct StateEvidence:
  get: () -> Int
  set: (Int) -> ()

fn counter_compiled(n: Int, ev: StateEvidence) -> Int:
  if n == 0:
    return ev.get()
  else:
    x = ev.get()
    ev.set(x + 1)
    return counter_compiled(n - 1, ev)  # Tail call preserved!

fn run_counter_compiled() -> Int:
  var state = 0
  ev = StateEvidence {
    get: || state,
    set: |x| { state = x }
  }
  return counter_compiled(1000000, ev)
```

### B.2 Choice Effect Compilation (Selective CPS)

```aria
# Source
effect Choice:
  choose: () -> Bool

fn example(): Choice -> List[Int]
  x = if choose() then 1 else 2
  y = if choose() then 10 else 20
  [x + y]

fn all_choices[A](action: () -> Choice -> A): List[A]
  handle action() with
    choose() ->
      left = resume(true)
      right = resume(false)
      left ++ right

# Compiled (CPS for multi-shot)
fn example_cps(ev: ChoiceEvidence, k: (List[Int]) -> R) -> R:
  ev.choose(|first_choice|
    x = if first_choice then 1 else 2
    ev.choose(|second_choice|
      y = if second_choice then 10 else 20
      k([x + y])))

fn all_choices_compiled[A](action: (ChoiceEvidence, (A) -> List[A]) -> List[A]) -> List[A]:
  ev = ChoiceEvidence {
    choose: |k|
      left = k(true)
      right = k(false)
      left ++ right
  }
  action(ev, |result| [result])
```

---

*Document generated by PRISM research agent, Eureka Iteration 2*
*Last updated: 2026-01-15*
