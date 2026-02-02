# BioFlow Python Implementation

A Python port of the Aria BioFlow genomic data processing library, created for comparison purposes to demonstrate Aria's advantages over traditional implementations.

## Overview

This implementation mirrors the functionality of the Aria BioFlow library (`examples/bioflow/`) to enable direct comparison between:

- **Aria**: Compile-time contracts, native performance, memory safety
- **Python**: Runtime checks, interpreted execution, GC-managed memory

## Installation

```bash
# Clone the repository (if not already done)
cd examples/bioflow-python

# Create virtual environment (optional but recommended)
python -m venv venv
source venv/bin/activate  # On Windows: venv\Scripts\activate

# Install dependencies
pip install -r requirements.txt

# Install in development mode
pip install -e .
```

## Quick Start

```python
from bioflow import Sequence, count_kmers, smith_waterman

# Create a DNA sequence
seq = Sequence.new("ATGCGATCGATCGATCGATCGATCG")
print(f"GC content: {seq.gc_content() * 100:.1f}%")

# Count k-mers
kmers = count_kmers(seq, 5)
print(f"Unique 5-mers: {kmers.unique_count()}")

# Align sequences
seq1 = Sequence.new("AGTACGCA")
seq2 = Sequence.new("TATGC")
alignment = smith_waterman(seq1, seq2)
print(alignment.format())
```

## Running Examples

```bash
# Run the demo script
python examples/demo.py

# Run benchmarks
python benchmark.py

# Run benchmarks with NumPy comparison
python benchmark.py --numpy
```

## Running Tests

```bash
# Install pytest
pip install pytest

# Run all tests
pytest tests/

# Run with verbose output
pytest tests/ -v

# Run specific test file
pytest tests/test_sequence.py
```

## Project Structure

```
bioflow-python/
├── bioflow/                 # Main library
│   ├── __init__.py         # Package exports
│   ├── sequence.py         # Sequence class
│   ├── kmer.py             # K-mer counting
│   ├── alignment.py        # Smith-Waterman/Needleman-Wunsch
│   ├── quality.py          # Quality scores
│   └── stats.py            # Statistics
├── tests/                   # Test suite
│   ├── test_sequence.py
│   ├── test_kmer.py
│   └── test_alignment.py
├── examples/
│   └── demo.py             # Demo script
├── benchmark.py            # Performance benchmarks
├── requirements.txt        # Dependencies
├── COMPARISON.md           # Aria vs Python analysis
└── README.md               # This file
```

## Module Overview

### sequence.py

Core sequence representation with validation:

```python
# Aria equivalent contracts shown in comments
class Sequence:
    # invariant self.bases.len() > 0
    # invariant self.is_valid()

    def gc_content(self) -> float:
        # requires self.is_valid()
        # ensures result >= 0.0 and result <= 1.0
        ...
```

### kmer.py

K-mer counting and analysis:

- `count_kmers(sequence, k)` - Count all k-mers
- `most_frequent_kmers(sequence, k, n)` - Get top n k-mers
- `kmer_distance(seq1, seq2, k)` - Jaccard distance
- `kmer_spectrum(sequence, k)` - Frequency distribution

### alignment.py

Sequence alignment algorithms:

- `smith_waterman(seq1, seq2)` - Local alignment
- `needleman_wunsch(seq1, seq2)` - Global alignment
- `alignment_score_only(seq1, seq2)` - Memory-efficient scoring

### quality.py

Phred quality score management:

- Parse Phred+33 and Phred+64 encodings
- Calculate statistics (mean, median, etc.)
- Categorize quality levels

### stats.py

Statistical summaries:

- `SequenceStats` - Single sequence statistics
- `SequenceSetStats` - Aggregate statistics (N50, etc.)
- `GCHistogram` - GC content distribution

## Key Differences from Aria

### 1. Contract Enforcement

**Aria** (compile-time):
```aria
fn gc_content(self) -> Float
  requires self.is_valid()              # Verified at compile time
  ensures result >= 0.0 and result <= 1.0
```

**Python** (runtime):
```python
def gc_content(self) -> float:
    # No compile-time verification
    # Must rely on runtime checks
    assert self.is_valid()  # Only checked when run
```

### 2. Type Safety

**Aria**: All types checked at compile time
**Python**: Type hints are optional, checked by external tools (mypy)

### 3. Performance

**Aria**: Compiles to native code (~C performance)
**Python**: Interpreted (~10-100x slower for compute-intensive code)

### 4. Memory Management

**Aria**: Predictable, no GC pauses
**Python**: GC can cause unpredictable latency spikes

## Benchmarks

Run benchmarks to see performance differences:

```bash
python benchmark.py
```

Example output:
```
=== K-mer Counting Benchmark ===
  5,000 bp, k=11: 25.42ms
  10,000 bp, k=21: 58.31ms
  20,000 bp, k=21: 115.67ms

=== Sequence Alignment Benchmark ===
  100 x 100 bp:
    Smith-Waterman (full): 15.23ms
  500 x 500 bp:
    Smith-Waterman (full): 389.45ms
```

## Documentation

- **[COMPARISON.md](COMPARISON.md)** - Detailed Aria vs Python comparison
- **[../bioflow/ALGORITHM_ANALYSIS.md](../bioflow/ALGORITHM_ANALYSIS.md)** - Algorithm analysis
- **[../bioflow/docs/](../bioflow/docs/)** - Aria BioFlow documentation

## Contributing

This is a reference implementation for comparison purposes. For production bioinformatics work:

1. Consider using the Aria implementation for performance-critical code
2. Use established Python libraries (BioPython, etc.) for production Python code
3. This implementation prioritizes clarity over optimization

## License

Same license as the main Aria project.
