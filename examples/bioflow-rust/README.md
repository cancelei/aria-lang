# BioFlow Rust

A production-quality bioinformatics library implemented in Rust, demonstrating Rust's safety and performance features for comparison with the Aria programming language.

## Overview

BioFlow Rust provides efficient implementations of common bioinformatics algorithms:

- **Sequence Analysis**: DNA/RNA validation, GC content, base composition
- **K-mer Counting**: Efficient k-mer extraction and frequency analysis
- **Sequence Alignment**: Smith-Waterman (local) and Needleman-Wunsch (global)
- **Quality Scores**: FASTQ quality score parsing and statistics
- **Statistics**: Running statistics, histograms, N50 calculations

## Features

### Rust Safety Features Demonstrated

1. **Ownership and Borrowing**
   - Compile-time memory safety without garbage collection
   - Zero-cost abstractions for safe data access

2. **Result Types for Error Handling**
   - No null pointers - using `Option<T>` instead
   - Explicit error handling with `Result<T, E>`

3. **Thread Safety**
   - `Send` and `Sync` traits for safe concurrency
   - Rayon integration for parallel processing

4. **Strong Type System**
   - Validated types that guarantee correctness
   - No runtime type errors

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
bioflow-rust = { path = "path/to/bioflow-rust" }
```

Or clone and build:

```bash
cd examples/bioflow-rust
cargo build --release
```

## Usage

### Library Usage

```rust
use bioflow_rust::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create and validate a sequence
    let seq = Sequence::new("ATGCGATCGATCGATCG")?;

    // Calculate GC content
    println!("GC content: {:.1}%", seq.gc_content() * 100.0);

    // Get base composition
    let comp = seq.base_composition();
    println!("A: {}, C: {}, G: {}, T: {}",
        comp.a_count, comp.c_count, comp.g_count, comp.t_count);

    // Transform sequences
    let complement = seq.complement();
    let reverse_complement = seq.reverse_complement();

    // Count k-mers
    let mut counter = KMerCounter::new(21);
    counter.count(&seq);
    for (kmer, count) in counter.most_frequent(10) {
        println!("{}: {}", kmer, count);
    }

    // Align sequences
    let seq1 = Sequence::new("ACGTACGT")?;
    let seq2 = Sequence::new("ACGTTCGT")?;
    let alignment = smith_waterman(&seq1, &seq2, &ScoringMatrix::default());
    println!("Score: {}, Identity: {:.1}%",
        alignment.score, alignment.identity() * 100.0);

    Ok(())
}
```

### Command-Line Interface

```bash
# Analyze a sequence
cargo run -- analyze ATGCGATCGATCGATCG

# Count k-mers
cargo run -- kmer ATGCGATCGATCG -k 3 --top 5

# Align sequences
cargo run -- align ACGTACGT ACGTTCGT

# Global alignment
cargo run -- align ACGTACGT ACGTTCGT --global

# Quality score analysis
cargo run -- quality "IIIIIIIII!!!!IIIII"

# Generate random sequence
cargo run -- random 1000 --gc 0.5

# Run demo
cargo run -- demo
```

### Example Application

```bash
cargo run --example demo
```

## API Reference

### Sequence Module

```rust
// Create sequences
let seq = Sequence::new("ATGC")?;
let seq = Sequence::with_id("ATGC", "seq1")?;
let rna = Sequence::new_rna("AUGC")?;

// Properties
seq.len()                  // Length in bases
seq.bases()                // Get base string
seq.gc_content()           // GC content (0.0 - 1.0)
seq.base_composition()     // Detailed composition

// Transformations
seq.complement()           // A<->T, C<->G
seq.reverse()              // Reverse order
seq.reverse_complement()   // Both operations
seq.transcribe()           // DNA -> RNA
seq.subsequence(start, end) // Extract region

// Analysis
seq.find_pattern("ATG")    // Find all occurrences
seq.molecular_weight()     // Calculate MW
seq.melting_temperature()  // Calculate Tm
```

### K-mer Module

```rust
// Create counter
let mut counter = KMerCounter::new(21);

// Count k-mers
counter.count(&sequence);
counter.count_all(sequences.iter());

// Query results
counter.get("ATGC")            // Count for specific k-mer
counter.frequency("ATGC")      // Frequency (0.0 - 1.0)
counter.most_frequent(10)      // Top N k-mers
counter.least_frequent(10)     // Bottom N k-mers
counter.total_kmers()          // Total k-mers counted
counter.unique_kmers()         // Unique k-mers seen
counter.entropy()              // Shannon entropy
counter.saturation()           // Fraction of possible k-mers

// Canonical k-mers (strand-agnostic)
let mut canonical = CanonicalKMerCounter::new(21);
canonical.count(&sequence);
```

### Alignment Module

```rust
// Scoring matrix
let scoring = ScoringMatrix::default();
let scoring = ScoringMatrix::new(match_score, mismatch, gap);
let scoring = ScoringMatrix::with_affine_gaps(match, mismatch, gap_open, gap_extend);

// Local alignment (Smith-Waterman)
let alignment = smith_waterman(&seq1, &seq2, &scoring);

// Global alignment (Needleman-Wunsch)
let alignment = needleman_wunsch(&seq1, &seq2, &scoring);

// Semi-global alignment
let alignment = semi_global_alignment(&seq1, &seq2, &scoring);

// Alignment results
alignment.score              // Alignment score
alignment.identity()         // Fraction of matches
alignment.matches            // Number of matches
alignment.mismatches         // Number of mismatches
alignment.gaps               // Number of gaps
alignment.format_alignment(60) // Formatted output

// Distance metrics
edit_distance(&seq1, &seq2)     // Levenshtein distance
hamming_distance(&seq1, &seq2)  // Hamming distance (same length)
```

### Quality Module

```rust
// Parse quality scores
let quality = QualityScores::from_phred33("IIIII!!!!")?;
let quality = QualityScores::from_scores(vec![30, 30, 10], QualityEncoding::Phred33)?;

// Statistics
quality.mean()                    // Mean quality
quality.median()                  // Median quality
quality.min() / quality.max()     // Range
quality.high_quality_fraction()   // Fraction >= Q30
quality.mean_error_probability()  // Average error rate

// Trimming
let (start, end) = quality.trim_ends(20);  // Trim low-quality ends

// Batch statistics
let mut stats = QualityStats::new();
stats.add(&quality);
stats.per_position_mean()
```

### Statistics Module

```rust
// Summary statistics
let stats = SummaryStats::from_data(&values).unwrap();
stats.mean / stats.median / stats.std_dev / stats.q1 / stats.q3

// Running statistics (streaming)
let mut running = RunningStats::new();
running.push(value);
running.mean() / running.variance()

// Histogram
let mut hist = Histogram::new();
hist.add(value);
hist.mode() / hist.mean()

// Sequence statistics
n50(&lengths)     // N50 statistic
l50(&lengths)     // L50 statistic
gc_content(seq)   // GC content

// Correlation
pearson_correlation(&x, &y)
```

## Benchmarks

Run benchmarks with Criterion:

```bash
cargo bench
```

Example results (on a typical machine):

| Operation | Size | Time |
|-----------|------|------|
| GC Content | 20 KB | ~5 us |
| GC Content | 1 MB | ~200 us |
| K-mer Count (k=21) | 20 KB | ~800 us |
| Smith-Waterman | 1 KB x 1 KB | ~15 ms |

## Testing

Run the test suite:

```bash
# All tests
cargo test

# Specific module
cargo test sequence
cargo test kmer
cargo test alignment

# With output
cargo test -- --nocapture
```

## Project Structure

```
bioflow-rust/
├── src/
│   ├── main.rs          # CLI entry point
│   ├── lib.rs           # Library root
│   ├── sequence.rs      # Sequence types and operations
│   ├── kmer.rs          # K-mer counting
│   ├── alignment.rs     # Alignment algorithms
│   ├── quality.rs       # Quality score handling
│   └── stats.rs         # Statistical utilities
├── benches/
│   └── benchmarks.rs    # Criterion benchmarks
├── tests/
│   ├── sequence_tests.rs
│   ├── kmer_tests.rs
│   └── alignment_tests.rs
├── examples/
│   └── demo.rs          # Feature demonstration
├── Cargo.toml
├── README.md
└── COMPARISON.md        # Rust vs Aria comparison
```

## License

MIT License - See LICENSE file for details.

## See Also

- [COMPARISON.md](COMPARISON.md) - Detailed comparison of Rust and Aria
- [Aria Language](https://github.com/aria-lang/aria) - The Aria programming language
