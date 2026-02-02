# C++20 vs Aria - BioFlow Comparison

This document compares the C++20 implementation of BioFlow with a hypothetical Aria implementation, highlighting differences in safety, expressiveness, and performance.

## Performance Comparison

### Benchmark Results

| Operation | C++20 | Aria (Target) | Notes |
|-----------|-------|---------------|-------|
| GC Content (20kb) | ~5 us | ~5 us | Equivalent - both compile to similar machine code |
| K-mer Count (k=21, 20kb) | ~2 ms | ~2 ms | Equivalent - same algorithmic complexity |
| Smith-Waterman (1000x1000) | ~50 ms | ~50 ms | Equivalent - same O(mn) algorithm |
| Edit Distance (1000x1000) | ~3 ms | ~3 ms | Equivalent |
| Sequence Construction | ~1 us/kb | ~1 us/kb | Validation overhead similar |

**Key Insight**: For computational bioinformatics, both languages should achieve similar performance because:
1. Both compile to native code
2. Both use similar data structures (hash maps, vectors)
3. Performance is dominated by algorithmic complexity, not language overhead

## Safety Comparison

### Memory Safety

**C++20:**
```cpp
// Potential issues even with smart pointers
std::vector<Sequence> sequences;
auto& ref = sequences[0];  // Reference invalidation risk
sequences.push_back(Sequence("ATCG"));  // ref is now dangling!

// Use-after-move
Sequence seq("ATCG");
auto moved = std::move(seq);
// seq.bases() is undefined behavior (but compiles!)
```

**Aria (Designed for):**
```aria
// Borrow checker prevents reference invalidation
let sequences = Vector[Sequence]::new()
let ref = &sequences[0]
sequences.push(Sequence::new("ATCG"))  // Compile error: cannot mutate while borrowed

// Move semantics are enforced
let seq = Sequence::new("ATCG")
let moved = seq.move()
seq.bases()  // Compile error: seq has been moved
```

### Undefined Behavior

**C++20 UB Examples:**
```cpp
// Integer overflow (UB in signed integers)
int32_t x = INT32_MAX;
x + 1;  // UB!

// Null pointer dereference
Sequence* ptr = nullptr;
ptr->bases();  // UB!

// Data race
std::vector<int> v;
// Thread 1: v.push_back(1);
// Thread 2: v[0];  // UB!
```

**Aria (No UB by design):**
```aria
// Checked arithmetic by default
let x: Int32 = Int32::MAX
x + 1  // Runtime panic or compile-time error with overflow checking

// No null pointers - use Option
let ptr: Option[Sequence] = None
ptr.unwrap().bases()  // Explicit - will panic if None

// Send/Sync traits prevent data races
let v = Vector[Int]::new()
// Cannot share mutable reference across threads without Mutex
```

## Design by Contract

### C++20 Approach

C++20 removed contracts from the standard (they were planned but pulled). Current options:

```cpp
// Manual assertions (not part of type system)
void processSequence(const Sequence& seq) {
    assert(seq.length() > 0);  // Runtime only, can be disabled
    assert(seq.isValid());     // No compiler enforcement
    // ...
}

// Documentation-only contracts
/**
 * @pre seq.length() > 0
 * @pre seq.isValid()
 * @post result.length() == seq.length()
 */
Sequence transform(const Sequence& seq);
```

### Aria Approach (Built-in)

```aria
// Contracts are part of the function signature
fn process_sequence(seq: &Sequence) -> Result
    requires seq.length() > 0
    requires seq.is_valid()
    ensures result.is_ok() implies result.unwrap().length() == seq.length()
{
    // Implementation
}

// Contracts are:
// 1. Checked at compile time where possible
// 2. Checked at runtime otherwise
// 3. Part of the function's type for documentation
// 4. Cannot be silently disabled
```

## Code Expressiveness

### Sequence Operations

**C++20:**
```cpp
// Manual validation in constructor
Sequence::Sequence(std::string_view bases) {
    validateBases(bases);  // Throws on invalid
    bases_.reserve(bases.length());
    std::ranges::transform(bases, std::back_inserter(bases_),
                          [](char c) { return toUpper(c); });
}

// GC content with ranges
double Sequence::gcContent() const noexcept {
    if (bases_.empty()) return 0.0;
    auto gc_count = std::ranges::count_if(bases_, [](char c) {
        return c == 'G' || c == 'C';
    });
    return static_cast<double>(gc_count) / bases_.length();
}
```

**Aria (More concise with pattern matching):**
```aria
// Constructor with validation
impl Sequence {
    fn new(bases: String) -> Result[Sequence, SequenceError] {
        validate_bases(&bases)?
        Ok(Sequence { bases: bases.to_uppercase() })
    }

    // GC content - more concise
    fn gc_content(&self) -> Float64 {
        if self.bases.is_empty() { return 0.0 }

        let gc_count = self.bases.chars()
            .filter(|c| c == 'G' or c == 'C')
            .count()

        gc_count as Float64 / self.bases.len() as Float64
    }
}
```

### Error Handling

**C++20:**
```cpp
// Exceptions or optional - inconsistent across ecosystem
class SequenceError : public std::runtime_error { /* ... */ };

// Option 1: Exceptions (hidden control flow)
Sequence seq("INVALID");  // Throws SequenceError

// Option 2: std::optional (no error info)
std::optional<Sequence> maybe_seq = Sequence::try_create("INVALID");

// Option 3: std::expected (C++23)
std::expected<Sequence, SequenceError> result = Sequence::create("INVALID");
```

**Aria (Unified Result type):**
```aria
// Consistent Result type throughout
let seq = Sequence::new("INVALID")?  // Propagates error
// or
match Sequence::new("INVALID") {
    Ok(s) => process(s),
    Err(e) => log_error(e),
}
```

## Generic Programming

### C++20 Concepts

```cpp
// Concept definition
template<typename T>
concept SequenceLike = requires(T t) {
    { t.bases() } -> std::convertible_to<std::string_view>;
    { t.length() } -> std::convertible_to<size_t>;
};

// Usage
template<SequenceLike S>
double computeGC(const S& seq) {
    // ...
}
```

### Aria Traits

```aria
// Trait definition
trait SequenceLike {
    fn bases(&self) -> &str
    fn length(&self) -> usize
}

// Usage with trait bounds
fn compute_gc[S: SequenceLike](seq: &S) -> Float64 {
    // ...
}
```

## Build System

### C++ (CMake)

```cmake
cmake_minimum_required(VERSION 3.20)
project(bioflow-cpp VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_library(bioflow
    src/sequence.cpp
    src/kmer.cpp
    # ... more files
)

target_include_directories(bioflow PUBLIC include)
target_compile_options(bioflow PRIVATE
    -Wall -Wextra -Wpedantic
    $<$<CONFIG:Release>:-O3 -march=native>
)

# External dependencies require find_package or FetchContent
find_package(GTest REQUIRED)
```

### Aria (Built-in Package Manager)

```toml
# aria.toml
[package]
name = "bioflow"
version = "0.1.0"

[dependencies]
# Standard library included

[dev-dependencies]
test_framework = "1.0"
benchmark = "1.0"
```

## Summary

| Aspect | C++20 | Aria |
|--------|-------|------|
| **Performance** | Excellent | Excellent (target) |
| **Memory Safety** | Manual (smart pointers help) | Guaranteed |
| **Null Safety** | std::optional (opt-in) | No null (enforced) |
| **Data Race Safety** | Manual | Enforced by type system |
| **Contracts** | Not in standard | Built-in |
| **Error Handling** | Multiple patterns | Unified Result type |
| **Build System** | External (CMake) | Built-in |
| **Compile Times** | Often slow | Target: fast |
| **Ecosystem** | Mature, vast | New, growing |

## When to Use Each

### Choose C++20 When:
- Working with existing C++ codebases
- Need access to mature ecosystem and libraries
- Team has C++ expertise
- Interfacing with C libraries

### Choose Aria When:
- Starting a new project from scratch
- Safety is a primary concern
- Want built-in design by contract
- Prefer unified tooling and build system
- Want guaranteed memory and thread safety

## Conclusion

C++20 provides excellent performance and modern features, but requires discipline to write safe code. Aria aims to provide equivalent performance with guaranteed safety, making it suitable for applications where reliability is paramount, such as bioinformatics analysis pipelines.
