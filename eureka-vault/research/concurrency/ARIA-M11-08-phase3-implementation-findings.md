# ARIA-M11-08: Phase 3 Structured Concurrency Implementation Findings

**Date**: 2026-01-26
**Status**: COMPLETED
**Phase**: Phase 3 - Structured Concurrency
**Milestone**: M11 - Concurrency Model

## Executive Summary

Phase 3 of the Aria Concurrency Model has been implemented, providing structured concurrency primitives with cooperative cancellation and error propagation. This document captures implementation findings, optimization opportunities, and future enhancement tasks.

## Implementation Overview

### Completed Components

| Component | Location | Status |
|-----------|----------|--------|
| `Scope` struct | `aria-runtime/src/scope.rs` | Complete |
| `CancelToken` | `aria-runtime/src/scope.rs` | Complete |
| `ScopedJoinHandle` | `aria-runtime/src/scope.rs` | Complete |
| `with_scope()` | `aria-runtime/src/scope.rs` | Complete |
| `with_supervised_scope()` | `aria-runtime/src/scope.rs` | Complete |
| Async.scope operation | `aria-effects/src/lib.rs` | Complete |
| Async.supervisor operation | `aria-effects/src/lib.rs` | Complete |
| Cancel effect | `aria-effects/src/lib.rs` | Complete |

### Test Coverage

- **57 unit tests** in aria-runtime (including 13 scope-specific tests)
- **14 doc tests** in aria-runtime
- **43 tests** in aria-effects (including Cancel effect tests)
- All tests passing

---

## Optimization Opportunities Discovered

### OPT-1: Thread Pool for Scoped Tasks (HIGH PRIORITY)

**Current Implementation**: Each `scope.spawn()` creates a new OS thread.

**Problem**: Thread creation overhead (~50-100μs per thread) makes fine-grained task spawning expensive.

**Proposed Optimization**:
- Implement a thread pool that scopes can draw from
- Pre-spawn threads based on available parallelism
- Reuse threads across scope lifetimes

**Expected Impact**: 10-50x improvement for many small tasks

**References**:
- Tokio's thread pool design
- Go's goroutine M:N scheduler
- ARIA-M11-05-green-threads-runtime.md

### OPT-2: Lock-Free Cancellation Check (MEDIUM PRIORITY)

**Current Implementation**: `CancelToken::is_cancelled()` uses `AtomicBool::load(Ordering::SeqCst)`.

**Problem**: SeqCst ordering is stronger than necessary for a simple flag check.

**Proposed Optimization**:
```rust
// Current
self.cancelled.load(Ordering::SeqCst)

// Optimized
self.cancelled.load(Ordering::Acquire)
```

**Expected Impact**: Marginal (~5-10ns per check on x86, more significant on ARM)

### OPT-3: Batch Error Notification (MEDIUM PRIORITY)

**Current Implementation**: Each task completion triggers condition variable notification.

**Problem**: Many notifications when multiple tasks complete simultaneously.

**Proposed Optimization**:
- Coalesce notifications using a small delay window
- Only notify when transitioning from non-zero to zero active count

**Expected Impact**: Reduced syscall overhead for many-task scenarios

### OPT-4: Stack-Allocated Small Scopes (LOW PRIORITY)

**Current Implementation**: All scope state is heap-allocated (Arc, Vec, etc.).

**Problem**: Heap allocation overhead for simple 2-3 task scopes.

**Proposed Optimization**:
- Small-scope optimization: stack-allocate for ≤4 tasks
- Similar to small string optimization

**Expected Impact**: Reduced allocation pressure for common cases

### OPT-5: Lazy CancelToken Cloning (MEDIUM PRIORITY)

**Current Implementation**: Each `spawn_with_cancel()` clones the CancelToken Arc.

**Problem**: Arc clone involves atomic increment.

**Proposed Optimization**:
- Pass CancelToken by reference where possible
- Use thread-local cancel token access pattern

**Expected Impact**: Reduced atomic contention in high-spawn scenarios

---

## Future Enhancement Opportunities

### ENH-1: Timeout Scopes

**Description**: Add `with_timeout_scope(duration, |scope| ...)` that auto-cancels after duration.

**Design**:
```rust
pub fn with_timeout_scope<F, R>(timeout: Duration, f: F) -> Result<R, TimeoutError>
where
    F: FnOnce(&mut Scope) -> R,
{
    let mut scope = Scope::new();
    let cancel = scope.cancel_token();

    // Spawn timeout task
    let timeout_thread = thread::spawn(move || {
        thread::sleep(timeout);
        cancel.cancel();
    });

    let result = f(&mut scope);
    scope.join_all();

    if scope.cancel_token().is_cancelled() {
        Err(TimeoutError)
    } else {
        Ok(result)
    }
}
```

**Complexity**: Low
**Value**: High (common pattern)

### ENH-2: Nursery-Style Dynamic Spawning

**Description**: Trio-inspired nursery that allows spawning after initial setup.

**Design**:
```aria
with Async.nursery |nursery|
  for url in urls
    nursery.start_soon fetch(url)
  end

  # Can spawn more based on results
  nursery.start_soon process_results()
end
```

**Complexity**: Medium
**Value**: Medium (more flexible task patterns)

### ENH-3: Scope-Local Storage

**Description**: Allow data to be shared across all tasks in a scope.

**Design**:
```rust
impl Scope {
    pub fn set_local<T: Send + Sync + 'static>(&self, value: T);
    pub fn get_local<T: Send + Sync + 'static>(&self) -> Option<&T>;
}
```

**Complexity**: Medium
**Value**: Medium (useful for logging context, tracing)

### ENH-4: Structured Concurrency Visualization

**Description**: Debug tool to visualize scope hierarchy and task states.

**Design**:
- Add optional scope naming
- Track parent-child relationships
- Generate dot/mermaid diagrams

**Complexity**: Medium
**Value**: High for debugging

### ENH-5: Selective Error Propagation

**Description**: Allow configuring which error types trigger sibling cancellation.

**Design**:
```rust
impl Scope {
    pub fn cancel_on<E: Error>(&mut self);
    pub fn ignore_error<E: Error>(&mut self);
}
```

**Complexity**: Medium
**Value**: Medium (fine-grained control)

---

## Comparison with Other Implementations

| Feature | Aria (Current) | Tokio | Go | Kotlin |
|---------|---------------|-------|-----|--------|
| Structured concurrency | Yes | No (manual) | No | Yes (coroutineScope) |
| Cooperative cancellation | Yes | Yes | Yes (context) | Yes |
| Error propagation | Yes (first error) | Manual | Manual | Yes |
| Thread model | Thread-per-task | M:N | M:N | M:N |
| Supervision | Yes | No | No | Yes (supervisorScope) |

### Gaps to Address

1. **M:N Scheduling**: Current thread-per-task limits scalability
2. **Select on Scopes**: Can't select between scope completion and channels
3. **Cancellation Handlers**: No cleanup callbacks on cancellation

---

## Performance Characteristics

### Current Benchmarks (Thread-per-Task)

| Operation | Time | Notes |
|-----------|------|-------|
| Scope creation | ~1μs | Mostly Arc allocations |
| Task spawn | ~50-100μs | OS thread creation |
| Cancel check | ~5-10ns | Atomic load |
| Scope join (empty) | ~100ns | Lock + check |
| Scope join (1 task) | ~50-100μs | Thread join |

### Target Benchmarks (With Thread Pool)

| Operation | Target | Improvement |
|-----------|--------|-------------|
| Task spawn | <1μs | 50-100x |
| Context switch | <300ns | Per design doc |

---

## Recommendations

### Immediate (Phase 4 Prerequisites)

1. **OPT-1**: Implement thread pool before Phase 4 runtime optimization
2. **ENH-1**: Add timeout scopes (common pattern, low complexity)

### Short-Term

3. **OPT-2**: Optimize memory ordering for cancellation checks
4. **ENH-4**: Add scope visualization for debugging

### Medium-Term

5. **ENH-2**: Nursery-style dynamic spawning
6. **ENH-3**: Scope-local storage
7. **OPT-3**: Batch error notifications

---

## Related Documents

- [ARIA-PD-006: Concurrency Model Design](../../docs/designs/ARIA-PD-006-concurrency-model.md)
- [ARIA-M11-05: Green Threads Runtime](ARIA-M11-05-green-threads-runtime.md)
- [ARIA-M11-06: Channel Patterns](ARIA-M11-06-channel-patterns.md)
- [ARIA-M11-07: Concurrency Benchmarks](ARIA-M11-07-concurrency-benchmarks.md)

---

## WeDo Tasks Generated

| Task ID | Description | Priority | Status |
|---------|-------------|----------|--------|
| ARIA-CONC-OPT-01 | Implement thread pool for scoped tasks | high | pending |
| ARIA-CONC-OPT-02 | Optimize cancellation check memory ordering | normal | pending |
| ARIA-CONC-ENH-01 | Implement timeout scopes | normal | pending |
| ARIA-CONC-ENH-02 | Add scope visualization/debugging tools | normal | pending |

---

*Generated from Phase 3 implementation findings. Next phase: Phase 4 - Runtime Optimization.*
