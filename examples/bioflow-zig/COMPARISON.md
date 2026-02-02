# Zig vs Aria - BioFlow Implementation Comparison

This document compares the Zig and Aria implementations of BioFlow, highlighting the design philosophies, trade-offs, and use cases for each language.

## Executive Summary

| Aspect | Zig | Aria |
|--------|-----|------|
| Memory Management | Manual (allocators) | Ownership-based (automatic) |
| Error Handling | Error unions | Result types with contracts |
| Compile-time | Comptime execution | Compile-time verification |
| Safety | Explicit bounds checking | Design by contract |
| Abstraction | Minimal, explicit | Higher-level, verified |
| Learning Curve | Moderate | Moderate-High |

## Performance Comparison

### Benchmark Results

| Operation | Zig (ReleaseFast) | Aria | Winner |
|-----------|-------------------|------|--------|
| GC Content (20kb) | ~0.015ms | TBD | - |
| K-mer Count (k=21, 20kb) | ~2.5ms | TBD | - |
| Smith-Waterman (1kb x 1kb) | ~45ms | TBD | - |
| Complement (20kb) | ~0.04ms | TBD | - |

*Note: Aria benchmarks to be added when Aria compiler is available.*

### Performance Analysis

**Zig Strengths:**
- Zero-cost abstractions
- Predictable performance (no hidden allocations)
- SIMD-friendly design
- Excellent cache locality control

**Aria Strengths:**
- Compile-time optimization through contracts
- Safe parallelization with ownership
- Potential for whole-program optimization
- No bounds-checking overhead with verified contracts

## Memory Management

### Zig Approach: Explicit Allocators

```zig
// Zig: Every allocation is explicit
pub fn init(allocator: Allocator, bases: []const u8) !Sequence {
    const upper = try allocator.alloc(u8, bases.len);
    errdefer allocator.free(upper);  // Cleanup on error

    // ... validation and processing ...

    return Sequence{
        .bases = upper,
        .allocator = allocator,
    };
}

// Usage: Caller manages lifetime
var seq = try Sequence.init(allocator, "ATGC");
defer seq.deinit();  // Must be explicit
```

**Advantages:**
- Complete control over memory
- Swappable allocators (arena, pool, etc.)
- No hidden allocations
- Predictable performance

**Disadvantages:**
- More verbose
- Easy to forget cleanup
- No automatic lifetime management

### Aria Approach: Ownership-Based

```aria
// Aria: Ownership-based automatic management
pub fn new(bases: str) -> Result<Sequence, SequenceError> {
    require bases.len() > 0 else SequenceError::EmptySequence

    let validated = validate_and_uppercase(bases)?

    Sequence {
        bases: validated,  // Ownership transferred
        id: None,
    }
}

// Usage: Automatic cleanup when out of scope
let seq = Sequence::new("ATGC")?
// seq automatically freed when scope ends
```

**Advantages:**
- Automatic lifetime management
- No memory leaks
- Safer by default
- Cleaner code

**Disadvantages:**
- Less control over allocation strategy
- Potential for unexpected drops
- May require explicit lifetime annotations

## Error Handling

### Zig: Error Unions

```zig
// Zig: Explicit error union types
pub const SequenceError = error{
    EmptySequence,
    InvalidBase,
    OutOfMemory,
};

pub fn init(allocator: Allocator, bases: []const u8) SequenceError!Sequence {
    if (bases.len == 0) return error.EmptySequence;

    for (bases) |c| {
        if (!isValidBase(c)) return error.InvalidBase;
    }

    // ...
}

// Handling
var seq = Sequence.init(allocator, bases) catch |err| switch (err) {
    error.EmptySequence => // handle empty
    error.InvalidBase => // handle invalid
    error.OutOfMemory => // handle OOM
};

// Or propagate
var seq = try Sequence.init(allocator, bases);
```

### Aria: Design by Contract

```aria
// Aria: Contracts verify conditions at compile time
pub fn new(bases: str) -> Result<Sequence, SequenceError>
    requires bases.len() > 0
    ensures result.is_ok() implies result.unwrap().len() == bases.len()
{
    // Contract violation is a compile-time error when provable
    require bases.all(|c| is_valid_base(c))
        else SequenceError::InvalidBase

    // ...
}

// Pattern matching for errors
match Sequence::new(bases) {
    Ok(seq) => // use seq
    Err(SequenceError::EmptySequence) => // handle empty
    Err(SequenceError::InvalidBase) => // handle invalid
}
```

## Compile-Time Features

### Zig: Comptime Execution

```zig
// Zig: Generate lookup table at compile time
const ComplementTable = comptime blk: {
    var table: [256]u8 = undefined;
    for (&table, 0..) |*entry, i| {
        entry.* = switch (@as(u8, @intCast(i))) {
            'A' => 'T',
            'T' => 'A',
            'C' => 'G',
            'G' => 'C',
            else => 'N',
        };
    }
    break :blk table;
};

// Zero runtime cost lookup
pub fn complement(base: u8) u8 {
    return ComplementTable[base];
}
```

### Aria: Compile-Time Verification

```aria
// Aria: Compile-time contract verification
@comptime
const VALID_BASES: Set<char> = {'A', 'C', 'G', 'T', 'N'}

pub fn is_valid_base(c: char) -> bool {
    VALID_BASES.contains(c.to_uppercase())
}

// Contracts verified at compile time when possible
pub fn complement(base: char) -> char
    requires is_valid_base(base)
    ensures is_valid_base(result)
{
    match base.to_uppercase() {
        'A' => 'T',
        'T' => 'A',
        'C' => 'G',
        'G' => 'C',
        _ => 'N',
    }
}
```

## Type Safety

### Zig: Simple Type System

```zig
// Zig: Clear, simple types
pub const Sequence = struct {
    bases: []u8,
    id: ?[]u8,
    allocator: Allocator,

    // Methods as namespaced functions
    pub fn gcContent(self: Sequence) f64 {
        // ...
    }
};
```

### Aria: Rich Type System

```aria
// Aria: Algebraic data types with contracts
pub struct Sequence {
    bases: String,
    id: Option<String>,

    // Associated contracts
    invariant self.bases.len() > 0
    invariant self.bases.all(|c| is_valid_base(c))
}

impl Sequence {
    pub fn gc_content(&self) -> f64
        ensures result >= 0.0 && result <= 1.0
    {
        // ...
    }
}
```

## Code Comparison

### K-mer Counter

#### Zig Implementation

```zig
pub const KMerCounter = struct {
    k: usize,
    counts: std.StringHashMap(usize),
    allocator: Allocator,
    total_count: usize,

    pub fn init(allocator: Allocator, k: usize) !KMerCounter {
        if (k == 0) return error.InvalidK;
        return KMerCounter{
            .k = k,
            .counts = std.StringHashMap(usize).init(allocator),
            .allocator = allocator,
            .total_count = 0,
        };
    }

    pub fn deinit(self: *KMerCounter) void {
        var it = self.counts.keyIterator();
        while (it.next()) |key| {
            self.allocator.free(key.*);
        }
        self.counts.deinit();
    }

    pub fn count(self: *KMerCounter, seq: Sequence) !void {
        if (seq.bases.len < self.k) return;

        var i: usize = 0;
        while (i <= seq.bases.len - self.k) : (i += 1) {
            const kmer = seq.bases[i .. i + self.k];
            // ... process kmer
        }
    }
};
```

#### Aria Implementation

```aria
pub struct KMerCounter {
    k: usize,
    counts: HashMap<String, usize>,
    total_count: usize,
}

impl KMerCounter {
    pub fn new(k: usize) -> Result<Self, KMerError>
        requires k > 0
    {
        Ok(KMerCounter {
            k,
            counts: HashMap::new(),
            total_count: 0,
        })
    }

    pub fn count(&mut self, seq: &Sequence)
        requires seq.len() >= self.k
        ensures self.total_count >= old(self.total_count)
    {
        for i in 0..=(seq.len() - self.k) {
            let kmer = &seq.bases[i..i + self.k]
            // ... process kmer
        }
    }
}
// Automatic cleanup when KMerCounter goes out of scope
```

## When to Use Each

### Choose Zig When:

1. **Maximum Performance Control**
   - Need custom allocators
   - Require precise memory layout
   - Building embedded systems

2. **C Interoperability**
   - Interfacing with C libraries
   - Building C-callable libraries
   - Gradual migration from C

3. **Predictable Runtime**
   - Real-time systems
   - No hidden allocations needed
   - Deterministic performance

4. **Simple Mental Model**
   - Prefer explicit over implicit
   - Want to see all allocations
   - Value simplicity over convenience

### Choose Aria When:

1. **Safety-Critical Code**
   - Contracts prevent bugs at compile time
   - Formal verification possible
   - Regulatory compliance needed

2. **Complex Domain Logic**
   - Rich type system helps model domain
   - Invariants enforce business rules
   - Pattern matching simplifies code

3. **Large Team Development**
   - Contracts serve as documentation
   - Type system catches errors early
   - Safer refactoring

4. **Rapid Development**
   - Automatic memory management
   - Higher-level abstractions
   - Less boilerplate

## Interoperability

### Zig with C

```zig
// Zig can directly call C code
const c = @cImport({
    @cInclude("zlib.h");
});

pub fn compress(data: []const u8) ![]u8 {
    // Direct C interop
    var dest_len: c.uLongf = undefined;
    _ = c.compress(dest.ptr, &dest_len, data.ptr, data.len);
    // ...
}
```

### Aria with Zig/C

```aria
// Aria FFI (hypothetical)
extern "C" {
    fn compress(dest: *mut u8, dest_len: *mut usize,
                src: *const u8, src_len: usize) -> i32
}

pub fn compress(data: &[u8]) -> Result<Vec<u8>, CompressionError>
    ensures result.is_ok() implies result.unwrap().len() <= data.len()
{
    // Safe wrapper around unsafe FFI
    unsafe {
        // ...
    }
}
```

## Conclusion

Both Zig and Aria offer compelling approaches to systems programming:

**Zig** excels when you need:
- Direct hardware control
- C interoperability
- Explicit memory management
- Minimal runtime

**Aria** excels when you need:
- Compile-time safety guarantees
- Rich type system with contracts
- Automatic resource management
- Higher-level abstractions

For BioFlow specifically:
- **Zig** provides excellent performance with complete control
- **Aria** offers safety guarantees that could prevent bioinformatics errors

The choice depends on your priorities: raw performance and control (Zig) vs. safety and expressiveness (Aria).

## Future Comparisons

Once the Aria compiler is available, we will:
1. Run identical benchmarks
2. Compare binary sizes
3. Measure compile times
4. Evaluate error message quality
5. Test real-world bioinformatics workloads
