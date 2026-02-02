# Milestone M03: Effect System Design

## Overview

Design Aria's effect system that tracks side effects (IO, mutation, async, exceptions) in the type system, enabling automatic async handling and eliminating the "function coloring" problem.

## Research Questions

1. How do we track effects without async/await syntax pollution?
2. Can effects be fully inferred, or do some need annotation?
3. How do we handle effect polymorphism elegantly?
4. What's the runtime cost of effect tracking?

## Core Innovation Target

```ruby
# No async/await keywords - effects inferred
fn fetch_users(ids)
  ids.map { |id| http.get("/users/#{id}") }  # Compiler knows: IO effect
end

fn process_all(ids)
  users = fetch_users(ids).await_all  # Only place async surfaces
  users.map(&:validate)               # Pure - no effect
end

# Compiler generates efficient async runtime automatically
```

## Competitive Analysis Required

| Language | Approach | Study Focus |
|----------|----------|-------------|
| Koka | Algebraic effects | Full effect system |
| Eff | Effect handlers | Academic reference |
| Unison | Abilities | Effect polymorphism |
| OCaml 5 | Effect handlers | Practical implementation |
| Scala 3 | Capability tracking | Effect as capabilities |

## Tasks

### ARIA-M03-01: Survey algebraic effects implementations
- **Description**: Study algebraic effects in research languages
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, effects, academic, survey
- **Deliverables**:
  - Effect handler comparison
  - Performance overhead analysis
  - User ergonomics evaluation

### ARIA-M03-02: Analyze Koka's effect system
- **Description**: Deep dive into Koka's practical effect system
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, effects, koka, competitive
- **Deliverables**:
  - Effect inference mechanics
  - Row polymorphism for effects
  - Runtime implementation

### ARIA-M03-03: Study effect inference in practice
- **Description**: Research automatic effect inference approaches
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, effects, inference
- **Deliverables**:
  - Inference algorithm options
  - Annotation requirements
  - Error message strategies

### ARIA-M03-04: Design Aria effect system
- **Description**: Design Aria's approach to effects
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Blocked by**: ARIA-M03-01, ARIA-M03-02, ARIA-M03-03
- **Tags**: research, effects, design, innovation
- **Deliverables**:
  - Effect type syntax (if any)
  - Inference rules
  - Async/IO handling strategy

### ARIA-M03-05: Prototype effect inference
- **Description**: Build effect inference prototype
- **Dependency**: AGENT_CAPABLE
- **Priority**: normal
- **Blocked by**: ARIA-M03-04
- **Tags**: prototype, effects, implementation
- **Deliverables**:
  - Working effect inference
  - Async code generation
  - Performance benchmarks

## Implementation Progress

### Effect Type System (COMPLETED - Jan 2026)
- [x] `crates/aria-effects/` crate with comprehensive effect system
- [x] Effect, EffectKind, EffectSet, EffectRow types
- [x] Row polymorphism with unification
- [x] Standard library effects: IO, Console, Exception, Async, State, Reader, Choice, Channel
- [x] Effect inference engine with generalization/instantiation
- [x] 50+ unit tests passing

### Effect Code Generation (COMPLETED - Jan 2026)
- [x] Effect statements in MIR (PerformEffect, InstallHandler, UninstallHandler)
- [x] TailResumptive effect compilation to direct calls
- [x] Console effect -> C runtime bridge (print, read_line)
- [x] IO effect -> C runtime bridge (read, write)
- [x] Async effect infrastructure (spawn, await, yield operations)

### Runtime FFI Bridge (COMPLETED - Jan 2026)
- [x] `crates/aria-runtime/src/ffi.rs` with C-callable functions
- [x] aria_async_spawn, aria_async_await, aria_async_yield
- [x] Thread-safe task handle storage
- [x] Unit tests for FFI functions

### Remaining Work
- [ ] Link aria-runtime FFI to compiled executables
- [ ] Full CPS transformation for General (multi-shot) effects
- [ ] Handler vtable infrastructure for custom handlers
- [ ] Fiber-based continuation capture

## Success Criteria

- [x] Effect system designed and documented
- [ ] No explicit async/await in user code (goal) - partial: effect inference works
- [x] Effect polymorphism supported
- [x] Clear error messages for effect mismatches
- [ ] Runtime overhead < 5% vs explicit async - needs benchmarking

## Key Papers/Resources

1. "Algebraic Effects for Functional Programming" - Leijen
2. "Koka: Programming with Row-Polymorphic Effect Types" - Leijen
3. "Handlers in Action" - Kammar et al.
4. "Effect Handlers, Evidently" - Xie et al.
5. "Retrofitting Effect Handlers onto OCaml" - Sivaramakrishnan

## Timeline

Target: Q1-Q2 2026

## Related Milestones

- **Depends on**: M01 (Type System)
- **Enables**: M11 (Concurrency), M13 (Error Handling)
- **Parallel**: M02 (Ownership)
