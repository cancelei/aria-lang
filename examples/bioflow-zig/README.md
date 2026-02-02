# BioFlow - Zig Implementation

A production-quality bioinformatics toolkit implemented in Zig, designed for performance comparison with the Aria programming language.

## Overview

BioFlow demonstrates Zig's strengths as a systems programming language:

- **Explicit Memory Management**: Clear ownership through allocators
- **Comptime Features**: Compile-time code execution and lookup tables
- **No Hidden Control Flow**: What you see is what you get
- **Excellent Error Handling**: Error unions and explicit error propagation
- **Zero-Cost Abstractions**: No runtime overhead for safety features

## Project Structure

```
bioflow-zig/
├── src/
│   ├── main.zig         # Entry point and CLI
│   ├── sequence.zig     # DNA/RNA sequence type
│   ├── kmer.zig         # K-mer counting and analysis
│   ├── alignment.zig    # Smith-Waterman & Needleman-Wunsch
│   ├── quality.zig      # FASTQ quality score handling
│   └── stats.zig        # Statistical functions
├── tests/
│   ├── sequence_test.zig
│   ├── kmer_test.zig
│   └── alignment_test.zig
├── benchmark/
│   └── bench.zig        # Performance benchmarks
├── build.zig            # Build configuration
├── README.md
└── COMPARISON.md        # Zig vs Aria comparison
```

## Building

### Prerequisites

- Zig 0.11.0 or later

### Build Commands

```bash
# Build the main executable
zig build

# Build with optimizations
zig build -Doptimize=ReleaseFast

# Run the application
zig build run

# Run tests
zig build test

# Run benchmarks
zig build bench
```

## Usage

### Command Line Interface

```bash
# Calculate GC content
./zig-out/bin/bioflow-zig gc ATGCGATCGATCG

# Count k-mers
./zig-out/bin/bioflow-zig kmer 3 ATGATGATG

# Align two sequences
./zig-out/bin/bioflow-zig align ACGTACGT ACGACGT

# Calculate sequence statistics
./zig-out/bin/bioflow-zig stats ATGCGATCGATCGATCG

# Show help
./zig-out/bin/bioflow-zig help
```

### Example Output

```
$ ./zig-out/bin/bioflow-zig gc ATGCGATCGATCG

Sequence Analysis
================
Length:     13 bp
GC Content: 53.85%

Base Composition:
  A: 3 (23.1%)
  C: 3 (23.1%)
  G: 4 (30.8%)
  T: 3 (23.1%)
  N: 0 (0.0%)

Complement:         TACGCTAGCTAGC
Reverse Complement: CGATCGATCGCAT
```

## API Examples

### Sequence Operations

```zig
const std = @import("std");
const Sequence = @import("sequence").Sequence;

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    // Create a sequence
    var seq = try Sequence.init(allocator, "ATGCGATCGA");
    defer seq.deinit();

    // Calculate GC content
    const gc = seq.gcContent();
    std.debug.print("GC Content: {d:.2}%\n", .{gc * 100.0});

    // Get complement
    var comp = try seq.complement();
    defer comp.deinit();
    std.debug.print("Complement: {s}\n", .{comp.bases});

    // Get reverse complement
    var rc = try seq.reverseComplement();
    defer rc.deinit();
    std.debug.print("Reverse Complement: {s}\n", .{rc.bases});
}
```

### K-mer Counting

```zig
const KMerCounter = @import("kmer").KMerCounter;

// Count 3-mers
var counter = try KMerCounter.init(allocator, 3);
defer counter.deinit();

try counter.count(seq);

// Get most frequent k-mers
const top = try counter.mostFrequent(allocator, 5);
defer allocator.free(top);

for (top) |kmer_count| {
    std.debug.print("{s}: {d}\n", .{kmer_count.kmer, kmer_count.count});
}
```

### Sequence Alignment

```zig
const alignment = @import("alignment");

// Smith-Waterman local alignment
var align = try alignment.smithWaterman(
    allocator,
    seq1,
    seq2,
    alignment.ScoringMatrix.default()
);
defer align.deinit();

std.debug.print("Score: {d}\n", .{align.score});
std.debug.print("Identity: {d:.1}%\n", .{align.identity() * 100.0});
```

## Features

### Sequence Module
- Sequence initialization and validation
- GC/AT/N content calculation
- Complement and reverse complement
- Subsequence extraction
- Pattern finding
- Hamming distance
- FASTA parsing and output
- Molecular weight and melting temperature

### K-mer Module
- Hash-based k-mer counting
- Canonical k-mer support
- Frequency analysis
- Shannon entropy
- K-mer spectrum analysis
- Jaccard similarity

### Alignment Module
- Smith-Waterman (local alignment)
- Needleman-Wunsch (global alignment)
- Configurable scoring matrices
- CIGAR string generation
- Edit distance calculation
- Visual alignment output

### Quality Module
- Phred33/Phred64 encoding support
- Auto-detection of encoding
- Quality statistics (mean, median, min, max)
- Expected error calculation
- Quality trimming
- FASTQ parsing

### Statistics Module
- Descriptive statistics
- Running statistics (online algorithm)
- Histograms
- Correlation analysis
- Distance metrics (Euclidean, Manhattan, Cosine)
- N50/L50 calculation

## Performance

Run benchmarks with:

```bash
zig build bench
./zig-out/bin/bench
```

Example benchmark output:

```
======================================================================
BioFlow Zig Performance Benchmarks
======================================================================

Sequence Operations
----------------------------------------------------------------------
GC Content (20kb)                         0.015ms (min: 0.014, max: 0.018)
Complement (20kb)                         0.042ms (min: 0.040, max: 0.048)
Reverse Complement (20kb)                 0.045ms (min: 0.043, max: 0.052)

K-mer Counting
----------------------------------------------------------------------
K-mer Count (k=21, 20kb)                  2.534ms (min: 2.489, max: 2.612)

Sequence Alignment
----------------------------------------------------------------------
Smith-Waterman (1kb x 1kb)               45.234ms (min: 44.123, max: 47.892)
```

## Testing

Run the test suite:

```bash
zig build test
```

Tests cover:
- Sequence initialization and validation
- All sequence operations
- K-mer counting correctness
- Alignment algorithm correctness
- Edge cases and error handling

## Design Principles

### Memory Management
All memory allocation is explicit through Zig's allocator interface:

```zig
// Allocator is passed explicitly
var seq = try Sequence.init(allocator, bases);
defer seq.deinit();  // Explicit cleanup
```

### Error Handling
Errors are explicit and must be handled:

```zig
pub fn init(allocator: Allocator, bases: []const u8) SequenceError!Sequence {
    if (bases.len == 0) return error.EmptySequence;
    // ...
}

// Caller must handle errors
var seq = try Sequence.init(allocator, bases);
// or
var seq = Sequence.init(allocator, bases) catch |err| {
    std.debug.print("Error: {}\n", .{err});
    return;
};
```

### Comptime Features
Compile-time code execution for performance:

```zig
// Complement lookup table generated at compile time
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
```

## License

This project is part of the Aria programming language examples and is provided for educational and comparison purposes.

## See Also

- [COMPARISON.md](COMPARISON.md) - Detailed comparison between Zig and Aria implementations
- [Aria Language](https://github.com/aria-lang/aria) - The Aria programming language
