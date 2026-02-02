# ARIA-M02-02: Swift ARC Implementation Study

**Task ID**: ARIA-M02-02
**Status**: Completed
**Date**: 2026-01-14
**Focus**: Analyze Swift's automatic reference counting

---

## Executive Summary

Swift's Automatic Reference Counting (ARC) provides memory safety without garbage collection pauses. This research analyzes ARC internals, including the object lifecycle, reference types, and performance characteristics.

---

## 1. ARC Architecture Overview

### 1.1 How ARC Works

```
Source Code → Compiler (swiftc) → SIL with retain/release → Binary
                    ↓
              Inserts calls to:
              - swift_retain()
              - swift_release()
              - swift_unownedRetain()
              - swift_weakLoadStrong()
```

ARC is implemented at **compile time**, not runtime:
- Compiler analyzes object lifetimes
- Inserts retain/release at strategic points
- No background GC thread or stop-the-world pauses

### 1.2 HeapObject Structure

Every Swift object on the heap contains:

```c
struct HeapObject {
    HeapMetadata *metadata;   // Type information
    InlineRefCounts refCounts; // Reference counts
    // ... object data follows
};

struct InlineRefCounts {
    atomic<uint64_t> bits;
    // Encodes: strong count, unowned count, flags
};
```

### 1.3 Three Reference Counts

| Count | Purpose | When Reaches Zero |
|-------|---------|-------------------|
| **Strong** | Primary ownership | Object deinitialized |
| **Unowned** | Non-owning, assumes valid | Memory deallocated |
| **Weak** | Non-owning, nullable | Side table freed |

---

## 2. Object Lifecycle

### 2.1 Five Phases

```
LIVE → DEINITING → DEINITED → FREED → DEAD
  │         │           │         │       │
  │         │           │         │       └─ Fully deallocated
  │         │           │         └─ Object freed, side table exists
  │         │           └─ deinit complete, strong=0
  │         └─ deinit running
  └─ Normal operation
```

### 2.2 Phase Transitions

| Transition | Trigger |
|------------|---------|
| LIVE → DEINITING | Strong count reaches 0 |
| DEINITING → DEINITED | deinit() completes |
| DEINITED → FREED | Unowned count reaches 0 |
| FREED → DEAD | Weak count reaches 0 |

---

## 3. Reference Types

### 3.1 Strong References (Default)

```swift
class Person {
    var name: String
}

var person1 = Person(name: "Alice")  // Strong ref, count = 1
var person2 = person1                 // Strong ref, count = 2
person1 = nil                         // count = 1
person2 = nil                         // count = 0, deallocated
```

### 3.2 Weak References

```swift
class Apartment {
    weak var tenant: Person?  // Weak reference
}

var person = Person(name: "Alice")
var apt = Apartment()
apt.tenant = person  // Weak ref, strong count still 1

person = nil         // Strong count = 0, object deinited
print(apt.tenant)    // nil (automatically zeroed)
```

**Implementation**: Weak refs point to **side table**, not object directly.

### 3.3 Unowned References

```swift
class Customer {
    var card: CreditCard?
}

class CreditCard {
    unowned let customer: Customer  // Unowned reference
}

var john = Customer()
john.card = CreditCard(customer: john)
// CreditCard holds unowned ref to john
```

**Key Difference from Weak**:
- Unowned: Points directly to object, no side table
- Unowned: Crashes if accessed after deallocation
- Unowned: Slightly better performance

---

## 4. Side Tables

### 4.1 Purpose

Side tables enable weak references to survive object deallocation:

```
Normal object (no weak refs):
┌──────────────┐
│ HeapObject   │
│ - metadata   │
│ - refcounts  │
│ - data       │
└──────────────┘

Object with weak refs:
┌──────────────┐     ┌─────────────┐
│ HeapObject   │────►│ Side Table  │
│ - metadata   │     │ - weak count│
│ - refcounts  │     │ - object ptr│
│ - data       │     └─────────────┘
└──────────────┘           ▲
                           │
                    weak references
```

### 4.2 Lifecycle

1. First weak ref created → Side table allocated
2. Object deinited → Side table remains
3. Weak refs access side table → Return nil
4. All weak refs gone → Side table freed

---

## 5. ARC Overhead Analysis

### 5.1 Performance Costs

| Operation | Cost |
|-----------|------|
| Strong retain | Atomic increment (~5-20 cycles) |
| Strong release | Atomic decrement + check |
| Weak load | Side table lookup + atomic ops |
| Unowned access | Direct pointer + check |
| Allocation | Side table (lazy, on first weak ref) |

### 5.2 Comparison with Other Approaches

| Approach | Typical Overhead | Characteristics |
|----------|------------------|-----------------|
| **Swift ARC** | 5-15% | Deterministic, no pauses |
| **Tracing GC** | 10-50% | Unpredictable pauses |
| **Rust ownership** | 0% | No runtime cost |
| **C++ manual** | 0% | Error-prone |

### 5.3 Optimizations

**Compiler Optimizations**:
- Retain/release elision when provably unnecessary
- Coalescing adjacent retain/release pairs
- Tail call optimization for release

**Runtime Optimizations**:
- Inline reference counts (no separate allocation)
- Lazy side table creation
- Atomic operations only when needed

---

## 6. Copy-on-Write (COW)

### 6.1 Concept

Value types (Array, String, Dictionary) use COW for efficiency:

```swift
var array1 = [1, 2, 3]      // Buffer allocated
var array2 = array1          // Shares buffer, ref count = 2
array2.append(4)             // Copy made, both have ref count = 1
```

### 6.2 Implementation

```swift
struct MyArray<T> {
    private var storage: ArrayStorage<T>

    mutating func append(_ element: T) {
        // Check if we're the only owner
        if !isKnownUniquelyReferenced(&storage) {
            storage = storage.copy()  // Make our own copy
        }
        storage.append(element)
    }
}
```

### 6.3 `isKnownUniquelyReferenced`

Swift provides this intrinsic to check reference count:
- Returns `true` if strong count == 1
- Enables COW without manual tracking

---

## 7. Reference Cycles

### 7.1 The Problem

```swift
class Person {
    var apartment: Apartment?
}

class Apartment {
    var tenant: Person?  // Strong reference
}

var john = Person()
var apt = Apartment()
john.apartment = apt
apt.tenant = john

// CYCLE: john → apt → john
// Neither can be deallocated!
john = nil
apt = nil
// Memory leaked!
```

### 7.2 Solutions

**Weak References** (when ref can become nil):
```swift
class Apartment {
    weak var tenant: Person?  // Weak breaks cycle
}
```

**Unowned References** (when ref always valid during lifetime):
```swift
class CreditCard {
    unowned let owner: Customer  // Unowned breaks cycle
}
```

**Closure Capture Lists**:
```swift
class ViewController {
    var handler: (() -> Void)?

    func setup() {
        handler = { [weak self] in  // Capture self weakly
            self?.doSomething()
        }
    }
}
```

---

## 8. Recommendations for Aria

### 8.1 What to Adopt

| Feature | Recommendation | Rationale |
|---------|----------------|-----------|
| Reference counting | As fallback option | For patterns where static analysis fails |
| Weak/unowned distinction | Yes | Clear semantics |
| COW for value types | Yes | Efficient collections |
| Side tables | Yes (if using RC) | Clean weak ref impl |

### 8.2 Aria's Hybrid Approach

```aria
# Primary: Static ownership (Rust-style, inferred)
fn process(data)
  result = transform(data)  # Ownership transferred
  save(result)
end

# Fallback: Reference counted (Swift-style, explicit)
@rc class SharedState
  @weak observers: Array[Observer]

  fn notify
    for obs in observers.compact  # Filter nil weak refs
      obs.update(self)
    end
  end
end
```

### 8.3 When to Use Each

| Pattern | Recommended Approach |
|---------|---------------------|
| Simple ownership | Static (inferred) |
| Trees | Static |
| Graphs | RC or generational refs |
| Observers | RC with weak refs |
| Parent-child cycles | RC with weak/unowned |
| Performance-critical | Static only |

### 8.4 Avoiding ARC Pitfalls

**Aria should provide**:
1. **Cycle detection**: Compile-time warning for potential cycles
2. **Clear escape hatches**: Explicit syntax for RC types
3. **Performance profiling**: Show retain/release hotspots
4. **Migration path**: Easy conversion between static and RC

---

## 9. Key Resources

1. [Swift ARC Documentation](https://docs.swift.org/swift-book/documentation/the-swift-programming-language/automaticreferencecounting/)
2. [WWDC21: ARC in Swift](https://developer.apple.com/videos/play/wwdc2021/10216/)
3. [Swift Memory Management Deep Dive](https://alexdremov.me/dive-into-swifts-memory-management/)
4. [Advanced iOS Memory Management](https://www.vadimbulavin.com/swift-memory-management-arc-strong-weak-and-unowned/)

---

## 10. Open Questions

1. Can Aria infer when RC is needed vs static ownership?
2. How do we handle cycles in the inferred case?
3. Should weak references be explicit or inferred?
4. What's the interaction between RC and effects?
