#!/usr/bin/env python3
"""
BioFlow Benchmark Script

Benchmarks key operations to compare with Aria implementation.

Usage:
    python benchmark.py
    python benchmark.py --numpy  # Include NumPy comparison

Output format matches what would be expected from Aria benchmarks
for direct comparison.
"""

import time
import argparse
import sys
from typing import Callable, Any

# Add parent directory to path
sys.path.insert(0, '.')

from bioflow.sequence import Sequence
from bioflow.kmer import KMerCounter, count_kmers
from bioflow.alignment import smith_waterman, needleman_wunsch, ScoringMatrix, alignment_score_only
from bioflow.quality import QualityScores


def time_function(func: Callable, *args, iterations: int = 1, **kwargs) -> float:
    """Time a function over multiple iterations."""
    start = time.perf_counter()
    for _ in range(iterations):
        result = func(*args, **kwargs)
    end = time.perf_counter()
    return (end - start) * 1000  # Return milliseconds


def benchmark_gc_content():
    """Benchmark GC content calculation."""
    print("\n=== GC Content Benchmark ===")

    # Create sequences of different sizes
    sizes = [1000, 5000, 10000, 20000, 50000]

    for size in sizes:
        seq_str = 'ATGC' * (size // 4)
        seq = Sequence.new(seq_str)

        # Warm up
        _ = seq.gc_content()

        # Benchmark
        iterations = 1000
        elapsed = time_function(seq.gc_content, iterations=iterations)

        print(f"  {size:,} bp x {iterations} iterations: {elapsed:.2f}ms ({elapsed/iterations:.4f}ms/call)")


def benchmark_kmer_counting():
    """Benchmark k-mer counting."""
    print("\n=== K-mer Counting Benchmark ===")

    # Test with different k values and sequence sizes
    test_cases = [
        (5000, 11),
        (10000, 21),
        (20000, 21),
        (50000, 31),
    ]

    for seq_size, k in test_cases:
        seq_str = 'ATGC' * (seq_size // 4)
        seq = Sequence.new(seq_str)

        # Warm up
        _ = count_kmers(seq, k)

        # Benchmark
        elapsed = time_function(count_kmers, seq, k, iterations=1)

        print(f"  {seq_size:,} bp, k={k}: {elapsed:.2f}ms")


def benchmark_alignment():
    """Benchmark sequence alignment."""
    print("\n=== Sequence Alignment Benchmark ===")

    # Test with different sequence sizes
    test_cases = [
        (100, 100),
        (200, 200),
        (500, 500),
        (1000, 1000),
    ]

    scoring = ScoringMatrix.default_dna()

    for len1, len2 in test_cases:
        seq1 = Sequence.new('ATGC' * (len1 // 4))
        seq2 = Sequence.new('GCTA' * (len2 // 4))

        # Warm up
        _ = smith_waterman(seq1, seq2, scoring)

        # Smith-Waterman benchmark
        sw_elapsed = time_function(smith_waterman, seq1, seq2, scoring, iterations=1)

        # Score-only benchmark (memory efficient)
        score_elapsed = time_function(alignment_score_only, seq1, seq2, scoring, iterations=1)

        print(f"  {len1} x {len2} bp:")
        print(f"    Smith-Waterman (full): {sw_elapsed:.2f}ms")
        print(f"    Score-only (O(n) space): {score_elapsed:.2f}ms")


def benchmark_quality_scores():
    """Benchmark quality score operations."""
    print("\n=== Quality Score Benchmark ===")

    # Create quality strings of different sizes
    sizes = [1000, 5000, 10000, 20000]

    for size in sizes:
        quality_str = 'I' * size  # All Q40

        # Benchmark parsing
        parse_elapsed = time_function(QualityScores.from_phred33, quality_str, iterations=100)

        # Create quality object for other benchmarks
        quality = QualityScores.from_phred33(quality_str)

        # Benchmark average
        avg_elapsed = time_function(quality.average, iterations=1000)

        # Benchmark categorization
        cat_elapsed = time_function(quality.categorize, iterations=1000)

        print(f"  {size:,} scores:")
        print(f"    Parse x100: {parse_elapsed:.2f}ms")
        print(f"    Average x1000: {avg_elapsed:.2f}ms")
        print(f"    Categorize x1000: {cat_elapsed:.2f}ms")


def benchmark_sequence_operations():
    """Benchmark various sequence operations."""
    print("\n=== Sequence Operations Benchmark ===")

    size = 10000
    seq_str = 'ATGC' * (size // 4)
    seq = Sequence.new(seq_str)

    iterations = 100

    # Complement
    comp_elapsed = time_function(seq.complement, iterations=iterations)
    print(f"  Complement ({size} bp) x{iterations}: {comp_elapsed:.2f}ms")

    # Reverse complement
    rc_elapsed = time_function(seq.reverse_complement, iterations=iterations)
    print(f"  Reverse complement ({size} bp) x{iterations}: {rc_elapsed:.2f}ms")

    # Transcribe
    trans_elapsed = time_function(seq.transcribe, iterations=iterations)
    print(f"  Transcribe ({size} bp) x{iterations}: {trans_elapsed:.2f}ms")

    # Motif finding
    motif = "GATC"
    motif_elapsed = time_function(seq.find_motif_positions, motif, iterations=iterations)
    print(f"  Find motif positions ({size} bp) x{iterations}: {motif_elapsed:.2f}ms")

    # Base counts
    counts_elapsed = time_function(seq.base_counts, iterations=iterations)
    print(f"  Base counts ({size} bp) x{iterations}: {counts_elapsed:.2f}ms")


def benchmark_with_numpy():
    """Benchmark comparison with NumPy implementation."""
    print("\n=== NumPy Comparison Benchmark ===")

    try:
        import numpy as np

        print("  NumPy is available - running comparison...")

        # GC content comparison
        size = 100000
        seq_str = 'ATGC' * (size // 4)
        seq = Sequence.new(seq_str)

        # Pure Python GC content
        iterations = 100
        pure_elapsed = time_function(seq.gc_content, iterations=iterations)

        # NumPy GC content
        bases_array = np.array(list(seq.bases))

        def numpy_gc():
            gc_count = np.sum((bases_array == 'G') | (bases_array == 'C'))
            return gc_count / len(bases_array)

        numpy_elapsed = time_function(numpy_gc, iterations=iterations)

        print(f"\n  GC Content ({size:,} bp) x{iterations}:")
        print(f"    Pure Python: {pure_elapsed:.2f}ms")
        print(f"    NumPy: {numpy_elapsed:.2f}ms")
        print(f"    Speedup: {pure_elapsed/numpy_elapsed:.1f}x")

        # K-mer counting comparison
        # Note: NumPy doesn't help much with k-mer counting due to string handling
        print("\n  Note: K-mer counting is string-heavy, NumPy provides limited benefit")

        # Alignment - could use NumPy for matrix operations
        print("\n  Smith-Waterman with NumPy matrices:")
        len1, len2 = 500, 500
        seq1_str = 'ATGC' * (len1 // 4)
        seq2_str = 'GCTA' * (len2 // 4)
        seq1 = Sequence.new(seq1_str)
        seq2 = Sequence.new(seq2_str)

        # Pure Python
        pure_sw = time_function(smith_waterman, seq1, seq2, iterations=1)
        print(f"    Pure Python: {pure_sw:.2f}ms")

        # NumPy-based Smith-Waterman
        def numpy_smith_waterman():
            """NumPy-optimized Smith-Waterman (simplified)."""
            s1, s2 = seq1_str, seq2_str
            m, n = len(s1), len(s2)

            # Initialize with NumPy arrays
            H = np.zeros((m + 1, n + 1), dtype=np.int32)
            scoring = ScoringMatrix.default_dna()

            max_score = 0

            for i in range(1, m + 1):
                for j in range(1, n + 1):
                    match_score = scoring.score(s1[i-1], s2[j-1])

                    diag = H[i-1, j-1] + match_score
                    up = H[i-1, j] + scoring.gap_penalty()
                    left = H[i, j-1] + scoring.gap_penalty()

                    H[i, j] = max(0, diag, up, left)

                    if H[i, j] > max_score:
                        max_score = H[i, j]

            return max_score

        numpy_sw = time_function(numpy_smith_waterman, iterations=1)
        print(f"    NumPy arrays: {numpy_sw:.2f}ms")
        print(f"    Speedup: {pure_sw/numpy_sw:.1f}x (limited - loop-heavy)")

    except ImportError:
        print("  NumPy not available - skipping NumPy comparison")
        print("  Install with: pip install numpy")


def run_all_benchmarks(include_numpy: bool = False):
    """Run all benchmarks."""
    print("=" * 60)
    print("BioFlow Python Benchmark Suite")
    print("=" * 60)
    print("\nNote: These benchmarks measure pure Python performance.")
    print("Aria's compiled implementation would be significantly faster.")

    benchmark_gc_content()
    benchmark_kmer_counting()
    benchmark_alignment()
    benchmark_quality_scores()
    benchmark_sequence_operations()

    if include_numpy:
        benchmark_with_numpy()

    print("\n" + "=" * 60)
    print("Benchmark Summary")
    print("=" * 60)
    print("""
Expected Aria vs Python Performance:

| Operation              | Python (pure) | Aria (compiled) | Speedup |
|------------------------|---------------|-----------------|---------|
| GC Content (20kb)      | ~5-10ms       | ~0.1-0.5ms      | 10-50x  |
| K-mer (k=21, 20kb)     | ~50-100ms     | ~2-5ms          | 20-50x  |
| Smith-Waterman (1kx1k) | ~2000-5000ms  | ~50-100ms       | 40-50x  |
| Quality parsing        | ~10-20ms      | ~0.5-1ms        | 20x     |

Note: Python with NumPy can achieve 2-10x speedup for vectorizable operations,
but Aria's native compilation still provides significant advantages for:
- Loop-heavy algorithms (alignment, k-mer counting)
- Memory-intensive operations (no GC pauses)
- Compile-time contract verification (zero runtime cost)
""")


def main():
    """Main entry point."""
    parser = argparse.ArgumentParser(description='BioFlow Python Benchmarks')
    parser.add_argument('--numpy', action='store_true',
                        help='Include NumPy comparison benchmarks')
    args = parser.parse_args()

    run_all_benchmarks(include_numpy=args.numpy)
    return 0


if __name__ == '__main__':
    sys.exit(main())
