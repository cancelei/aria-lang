# ARIA-M11-02: Kotlin Coroutines Structured Concurrency

**Task ID**: ARIA-M11-02
**Status**: Completed
**Date**: 2026-01-14
**Focus**: Study Kotlin's structured concurrency model and coroutine implementation

---

## Executive Summary

Kotlin's structured concurrency model ensures concurrent operations form a hierarchy where parent-child relationships govern lifecycle, cancellation, and error propagation. This research analyzes Kotlin's coroutine design for Aria's effect-based concurrency model.

---

## 1. Overview

### 1.1 What is Structured Concurrency?

```
Unstructured (traditional):
  main() → spawn(task1) → spawn(task2) → ???
  (Tasks live independently, no guaranteed cleanup)

Structured:
  main() {
    coroutineScope {
      launch(task1)  // Child of scope
      launch(task2)  // Child of scope
    }
    // Both tasks guaranteed complete here
  }
```

### 1.2 Key Principles

| Principle | Description |
|-----------|-------------|
| **Scoped lifetime** | Children cannot outlive parents |
| **Cancellation propagation** | Cancel parent → cancel all children |
| **Error propagation** | Child failure → parent failure |
| **Resource cleanup** | Automatic on scope exit |

---

## 2. Kotlin Coroutine Basics

### 2.1 Core Concepts

| Concept | Description |
|---------|-------------|
| `suspend fun` | Function that can suspend execution |
| `CoroutineScope` | Defines lifecycle boundary |
| `launch` | Start coroutine, returns Job |
| `async` | Start coroutine, returns Deferred<T> |
| `Job` | Handle to coroutine lifecycle |
| `Dispatcher` | Thread pool for execution |

### 2.2 Basic Example

```kotlin
suspend fun fetchUserData(): UserData {
    return coroutineScope {
        val profile = async { fetchProfile() }
        val preferences = async { fetchPreferences() }

        // Wait for both
        UserData(profile.await(), preferences.await())
    }
}
```

---

## 3. CoroutineScope Hierarchy

### 3.1 Scope Types

| Scope | Behavior |
|-------|----------|
| `coroutineScope` | Waits for children, propagates exceptions |
| `supervisorScope` | Children fail independently |
| `GlobalScope` | No parent (avoid in production) |
| `viewModelScope` | Android lifecycle-aware |

### 3.2 Parent-Child Relationships

```kotlin
fun main() = runBlocking {      // Root scope
    launch {                     // Child of runBlocking
        launch {                 // Child of parent launch
            delay(100)
        }
        async {                  // Another child
            compute()
        }
    }
}  // All children complete before exit
```

### 3.3 Job Hierarchy

```
runBlocking (Job)
    └── launch (Job)
         ├── launch (Job)
         └── async (Deferred<T>)
```

---

## 4. Cancellation Model

### 4.1 Cooperative Cancellation

```kotlin
suspend fun doWork() {
    while (isActive) {  // Check for cancellation
        // Do work
        yield()  // Cancellation check point
    }
}

// Or use built-in suspension points
suspend fun fetchData() {
    val data = httpClient.get(url)  // Checks cancellation
    process(data)
}
```

### 4.2 Cancellation Propagation

```kotlin
val job = launch {
    val child1 = launch { task1() }
    val child2 = launch { task2() }
}

job.cancel()  // Cancels child1 and child2 too
```

### 4.3 NonCancellable

```kotlin
suspend fun cleanup() = withContext(NonCancellable) {
    // Runs even after cancellation
    closeResources()
}
```

---

## 5. Error Handling

### 5.1 Exception Propagation

```kotlin
coroutineScope {
    launch {
        throw RuntimeException("Oops")
    }
    launch {
        // This gets cancelled due to sibling failure
        delay(1000)
    }
}
// coroutineScope rethrows the exception
```

### 5.2 SupervisorScope

```kotlin
supervisorScope {
    launch {
        throw RuntimeException("Child 1 fails")
    }
    launch {
        // This continues running!
        delay(1000)
        println("Child 2 completes")
    }
}
```

### 5.3 Exception Handler

```kotlin
val handler = CoroutineExceptionHandler { _, exception ->
    log.error("Caught: $exception")
}

GlobalScope.launch(handler) {
    throw RuntimeException("Uncaught")
}
```

---

## 6. Dispatchers (Execution Contexts)

### 6.1 Built-in Dispatchers

| Dispatcher | Use Case |
|------------|----------|
| `Dispatchers.Default` | CPU-intensive work |
| `Dispatchers.IO` | I/O operations |
| `Dispatchers.Main` | UI updates (Android) |
| `Dispatchers.Unconfined` | Start in caller thread |

### 6.2 Context Switching

```kotlin
suspend fun process() {
    withContext(Dispatchers.IO) {
        val data = readFile()  // On IO thread pool
    }
    withContext(Dispatchers.Default) {
        compute(data)  // On CPU thread pool
    }
}
```

---

## 7. Flow (Reactive Streams)

### 7.1 Cold Streams

```kotlin
fun numbers(): Flow<Int> = flow {
    for (i in 1..3) {
        delay(100)
        emit(i)
    }
}

// Collection
numbers().collect { value ->
    println(value)
}
```

### 7.2 Flow Operators

```kotlin
numbers()
    .map { it * 2 }
    .filter { it > 2 }
    .take(5)
    .collect { println(it) }
```

### 7.3 StateFlow and SharedFlow

```kotlin
// StateFlow: Single value, always has current value
val _state = MutableStateFlow(0)
val state: StateFlow<Int> = _state

// SharedFlow: Multiple values, configurable replay
val _events = MutableSharedFlow<Event>()
val events: SharedFlow<Event> = _events
```

---

## 8. Comparison: Kotlin vs Alternatives

### 8.1 Feature Matrix

| Feature | Kotlin | Go | Rust async | Aria (Proposed) |
|---------|--------|----|-----------|----|
| Structured concurrency | ✓ | Partial | ✓ (Tokio) | ✓ |
| Cancellation | Cooperative | None | Cooperative | Effect-based |
| Error propagation | ✓ | Manual | ✓ | Effect-based |
| Colored functions | ✓ (suspend) | No | ✓ (async) | No (effects) |
| Back-pressure | Flow | Channels | Streams | Effects |

### 8.2 Strengths and Weaknesses

**Strengths:**
- Clean structured concurrency
- Comprehensive cancellation
- Good IDE support
- Mature ecosystem

**Weaknesses:**
- Colored functions (suspend)
- Runtime overhead
- Dispatcher selection complexity
- Learning curve for scope rules

---

## 9. Recommendations for Aria

### 9.1 Structured Concurrency

```aria
# Aria could use effect-based structured concurrency
fn fetch_user_data() -> {Async} UserData
  # Effect scope provides structure
  with Async.scope |scope|
    profile = scope.spawn fetch_profile()
    prefs = scope.spawn fetch_preferences()

    UserData(profile.await, prefs.await)
  end
  # All spawned tasks complete when scope exits
end
```

### 9.2 Cancellation via Effects

```aria
# Cancellation as an effect
effect Cancel {
  fn check() -> Unit
  fn is_cancelled() -> Bool
}

fn long_task() -> {Async, Cancel} Result
  for item in items
    Cancel.check()  # Cancellation point
    process(item)
  end
end

# Handler provides cancellation token
with Cancel.handle(token)
  long_task()
end
```

### 9.3 Scope-Based Lifecycle

```aria
# Aria scopes similar to Kotlin
effect Scope {
  fn spawn(f: () -> {Async} T) -> Task[T]
  fn cancel_all() -> Unit
}

fn parallel_fetch() -> {Async} Data
  with Scope.new |scope|
    a = scope.spawn(fetch_a)
    b = scope.spawn(fetch_b)

    Data(a.await, b.await)
  end
  # Scope automatically waits for/cancels children
end
```

### 9.4 Error Propagation

```aria
# Supervisor scope equivalent
fn resilient_fetch() -> {Async} Array[Result[Data, Error]]
  with Scope.supervisor |scope|
    tasks = urls.map |url|
      scope.spawn fetch(url)
    end

    # Each task completes independently
    tasks.map |t| t.await end
  end
end
```

### 9.5 No Colored Functions

```aria
# Key Aria advantage: no function coloring
# These compose naturally:
fn sync_op() -> Int = 42
fn async_op() -> {Async} Int = fetch_data()

fn combined() -> {Async} Int
  x = sync_op()      # Just works
  y = async_op()     # Just works
  x + y
end
```

---

## 10. Implementation Considerations

### 10.1 Runtime Requirements

| Component | Purpose |
|-----------|---------|
| Task scheduler | Manages coroutine execution |
| Work-stealing queue | Load balancing |
| Continuation storage | Suspended state |
| Scope tracking | Parent-child relationships |

### 10.2 Compilation Strategy

```
Aria async function:
  → Effect inference identifies {Async}
  → Transform to continuation-passing style (CPS)
  → Generate state machine for suspension points
  → Scope handlers manage lifecycle
```

---

## 11. Key Takeaways

1. **Structured concurrency prevents leaks** - Children bound to parent scope
2. **Cooperative cancellation is essential** - Check points throughout code
3. **Error propagation should be configurable** - Normal vs supervisor scopes
4. **Effects can replace colored functions** - Aria advantage
5. **Scopes provide resource safety** - Automatic cleanup

---

## 12. Key Resources

1. [Kotlin Coroutines Guide](https://kotlinlang.org/docs/coroutines-guide.html)
2. [Structured Concurrency](https://elizarov.medium.com/structured-concurrency-722d765aa952)
3. [Notes on Structured Concurrency](https://vorpus.org/blog/notes-on-structured-concurrency-or-go-statement-considered-harmful/)
4. [Coroutine Context and Dispatchers](https://kotlinlang.org/docs/coroutine-context-and-dispatchers.html)
5. [Kotlin Flow](https://kotlinlang.org/docs/flow.html)

---

## 13. Open Questions

1. How do Aria's effects integrate with structured concurrency scope?
2. Should cancellation be an effect or built into Async?
3. What's the syntax for supervisor scopes?
4. How do we handle back-pressure in effect-based streams?
