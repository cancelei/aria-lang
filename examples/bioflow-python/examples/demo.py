#!/usr/bin/env python3
"""
BioFlow Demo - Python Implementation

This demo mirrors the functionality of the Aria BioFlow main.aria file,
demonstrating equivalent workflows in Python.

Usage:
    python demo.py
"""

import sys
sys.path.insert(0, '..')

from bioflow.sequence import Sequence, SequenceType
from bioflow.quality import QualityScores, Q_HIGH
from bioflow.kmer import count_kmers, kmer_distance, kmer_spectrum
from bioflow.alignment import smith_waterman, needleman_wunsch, ScoringMatrix
from bioflow.stats import SequenceStats, SequenceSetStats


def main():
    """Main entry point."""
    print("BioFlow Genomic Pipeline (Python)")
    print("==================================\n")

    # Run all example workflows
    example_sequence_operations()
    example_quality_analysis()
    example_kmer_analysis()
    example_alignment()
    example_batch_analysis()

    print("\nAll examples completed successfully!")
    return 0


def example_sequence_operations():
    """Example 1: Sequence Operations"""
    print("Example 1: Sequence Operations")
    print("------------------------------")

    # Create a DNA sequence with validation
    try:
        seq = Sequence.new("ATGCGATCGATCGATCGATCGATCGATCG")

        print(f"Created sequence: {seq.bases[:30]}...")
        print(f"Length: {len(seq)}")
        print(f"GC content: {seq.gc_content() * 100:.1f}%")
        print(f"AT content: {seq.at_content() * 100:.1f}%")

        # Get base counts
        a, c, g, t, n = seq.base_counts()
        print(f"Base counts: A={a} C={c} G={g} T={t} N={n}")

        # Complement and reverse complement
        complement = seq.complement()
        print(f"Complement: {complement.bases}")

        rev_comp = seq.reverse_complement()
        print(f"Reverse complement: {rev_comp.bases}")

        # Transcribe to RNA
        rna = seq.transcribe()
        print(f"Transcribed RNA: {rna.bases}")

        # Check for motifs
        if seq.contains_motif("GATC"):
            positions = seq.find_motif_positions("GATC")
            print(f"Found GATC motif at positions: {positions}")

        # Calculate statistics
        stats = SequenceStats.from_sequence(seq)
        print(f"Statistics:\n{stats}")

    except Exception as e:
        print(f"Error: {e}")

    print()


def example_quality_analysis():
    """Example 2: Quality Analysis"""
    print("Example 2: Quality Analysis")
    print("---------------------------")

    # Create quality scores from Phred+33 encoded string
    quality_string = "IIIIIIIIIIIIIIIIIIIIIIIIIIIII"  # All Q40
    try:
        quality = QualityScores.from_phred33(quality_string)

        print(f"Quality scores created, length: {len(quality)}")
        print(f"Average quality: {quality.average():.1f}")
        print(f"Min quality: {quality.min()}")
        print(f"Max quality: {quality.max()}")
        print(f"Median quality: {quality.median()}")
        print(f"High quality ratio: {quality.high_quality_ratio() * 100:.1f}%")
        print(f"Category: {quality.categorize().value}")

        # Get statistics
        stats = quality.statistics()
        print(f"Quality stats: {stats}")

    except Exception as e:
        print(f"Error creating quality: {e}")

    # Show error probability conversion
    print("\nPhred score to error probability examples:")
    for q in [10, 20, 30, 40]:
        prob = QualityScores.score_to_probability(q)
        print(f"  Q{q}: {prob:.6f} ({(1-prob)*100:.4f}% accuracy)")

    print()


def example_kmer_analysis():
    """Example 3: K-mer Analysis"""
    print("Example 3: K-mer Analysis")
    print("-------------------------")

    seq = Sequence.new("ATGATGATGATGATGATGATGATGATG")

    # Count 3-mers
    kmer_counts = count_kmers(seq, 3)
    print(f"K-mer counts (k=3): {kmer_counts}")
    print(f"Unique 3-mers: {kmer_counts.unique_count()}")
    print(f"Total 3-mers: {kmer_counts.total_kmers}")

    # Most frequent k-mers
    top_kmers = kmer_counts.most_frequent(5)
    print("\nTop 5 most frequent 3-mers:")
    for kmer, count in top_kmers:
        print(f"  {kmer}: {count}")

    # K-mer distance between sequences
    seq2 = Sequence.new("GCTAGCTAGCTAGCTAGCTAGCTAGCT")
    distance = kmer_distance(seq, seq2, 3)
    print(f"\nK-mer distance (k=3) between sequences: {distance:.3f}")

    # K-mer spectrum
    spectrum = kmer_spectrum(seq, 4)
    print("\n4-mer spectrum:")
    for count, num_kmers in spectrum:
        print(f"  Count {count}: {num_kmers} k-mers")

    print()


def example_alignment():
    """Example 4: Sequence Alignment"""
    print("Example 4: Sequence Alignment")
    print("-----------------------------")

    seq1 = Sequence.new("AGTACGCA")
    seq2 = Sequence.new("TATGC")

    # Smith-Waterman local alignment
    scoring = ScoringMatrix.default_dna()
    sw_alignment = smith_waterman(seq1, seq2, scoring)

    print("Smith-Waterman (Local) Alignment:")
    print(sw_alignment.format())
    print()

    # Needleman-Wunsch global alignment
    nw_alignment = needleman_wunsch(seq1, seq2, scoring)

    print("Needleman-Wunsch (Global) Alignment:")
    print(nw_alignment.format())
    print()

    # Alignment comparison
    print("Comparison:")
    print(f"  Local score: {sw_alignment.score}")
    print(f"  Global score: {nw_alignment.score}")
    print(f"  Local identity: {sw_alignment.identity * 100:.1f}%")
    print(f"  Global identity: {nw_alignment.identity * 100:.1f}%")

    print()


def example_batch_analysis():
    """Example 5: Batch Sequence Analysis"""
    print("Example 5: Batch Sequence Analysis")
    print("----------------------------------")

    # Create sample sequences
    sequences = [
        Sequence.new("ATGCGATCGATCGATCGATCGATCGATCG"),
        Sequence.new("GCTAGCTAGCTAGCTAGCTAGCTAGCTAG"),
        Sequence.new("TATATATATATATATATATATATATATATAT"),
        Sequence.new("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"),
        Sequence.new("GCGCGCGCGCGCGCGCGCGCGCGCGCGCGC"),
    ]

    print(f"Analyzing {len(sequences)} sequences...")

    # Calculate set statistics
    stats = SequenceSetStats.from_sequences(sequences)
    print(f"\n{stats}")

    # Individual sequence analysis
    print("\nPer-sequence analysis:")
    for i, seq in enumerate(sequences):
        gc = seq.gc_content()
        kmers = count_kmers(seq, 5)
        print(f"  Seq {i+1}: len={len(seq)}, GC={gc*100:.1f}%, unique 5-mers={kmers.unique_count()}")

    # Compare sequences using k-mer distance
    print("\nK-mer distance matrix (k=5):")
    print("     ", end="")
    for i in range(len(sequences)):
        print(f"  {i+1}  ", end="")
    print()

    for i, seq1 in enumerate(sequences):
        print(f"  {i+1}: ", end="")
        for j, seq2 in enumerate(sequences):
            if i == j:
                print(" 0.00", end="")
            else:
                dist = kmer_distance(seq1, seq2, 5)
                print(f" {dist:.2f}", end="")
        print()

    print()


if __name__ == '__main__':
    sys.exit(main())
