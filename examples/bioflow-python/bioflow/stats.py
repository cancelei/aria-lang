"""
BioFlow - Statistics Types

Statistical summaries for sequences and reads.

This module provides aggregate statistics for collections of sequences
and reads, similar to the Aria implementation.
"""

from typing import List, Dict, Tuple
from dataclasses import dataclass, field
from collections import Counter

from .sequence import Sequence
from .quality import QualityScores, QualityCategory


@dataclass
class SequenceStats:
    """
    Statistics for a single sequence.

    Aria equivalent:
        struct SequenceStats
          invariant self.a_count + self.c_count + self.g_count +
                    self.t_count + self.n_count == self.length
          invariant self.gc_content >= 0.0 and self.gc_content <= 1.0
          invariant self.at_content >= 0.0 and self.at_content <= 1.0
    """
    length: int
    gc_content: float
    at_content: float
    a_count: int
    c_count: int
    g_count: int
    t_count: int
    n_count: int
    has_ambiguous: bool

    @classmethod
    def from_sequence(cls, seq: Sequence) -> 'SequenceStats':
        """
        Calculate statistics for a sequence.

        Aria equivalent:
            fn from_sequence(seq: Sequence) -> SequenceStats
              requires seq.is_valid()
              ensures result.length == seq.len()
        """
        a, c, g, t, n = seq.base_counts()

        return cls(
            length=len(seq),
            gc_content=seq.gc_content(),
            at_content=(a + t) / len(seq) if len(seq) > 0 else 0.0,
            a_count=a,
            c_count=c,
            g_count=g,
            t_count=t,
            n_count=n,
            has_ambiguous=n > 0
        )

    def __str__(self) -> str:
        return (
            f"SequenceStats {{\n"
            f"  length: {self.length}\n"
            f"  GC content: {self.gc_content * 100:.1f}%\n"
            f"  AT content: {self.at_content * 100:.1f}%\n"
            f"  A: {self.a_count}, C: {self.c_count}, "
            f"G: {self.g_count}, T: {self.t_count}, N: {self.n_count}\n"
            f"}}"
        )


@dataclass
class SequenceSetStats:
    """
    Aggregated statistics for multiple sequences.

    Aria equivalent:
        struct SequenceSetStats
          count: Int
          total_bases: Int
          min_length: Int
          max_length: Int
          mean_length: Float
          median_length: Int
          mean_gc_content: Float
          n50: Int
          total_ambiguous: Int
    """
    count: int
    total_bases: int
    min_length: int
    max_length: int
    mean_length: float
    median_length: int
    mean_gc_content: float
    n50: int
    total_ambiguous: int

    @classmethod
    def from_sequences(cls, sequences: List[Sequence]) -> 'SequenceSetStats':
        """
        Calculate statistics for a collection of sequences.

        Aria equivalent:
            fn from_sequences(sequences: [Sequence]) -> SequenceSetStats
              requires sequences.len() > 0
              ensures result.count == sequences.len()
        """
        if len(sequences) == 0:
            raise ValueError("Sequence list cannot be empty")

        count = len(sequences)
        lengths = [len(seq) for seq in sequences]
        total_bases = sum(lengths)
        min_len = min(lengths)
        max_len = max(lengths)
        mean_len = total_bases / count

        # Calculate median
        sorted_lengths = sorted(lengths)
        mid = count // 2
        if count % 2 == 0:
            median_len = (sorted_lengths[mid - 1] + sorted_lengths[mid]) // 2
        else:
            median_len = sorted_lengths[mid]

        # Calculate mean GC content
        gc_sum = sum(seq.gc_content() for seq in sequences)
        mean_gc = gc_sum / count

        # Calculate N50 (length where 50% of bases are in longer sequences)
        sorted_desc = sorted(lengths, reverse=True)
        half_total = total_bases // 2
        running_sum = 0
        n50 = sorted_desc[0]

        for length in sorted_desc:
            running_sum += length
            if running_sum >= half_total:
                n50 = length
                break

        # Count total ambiguous bases
        total_ambiguous = sum(seq.count_ambiguous() for seq in sequences)

        return cls(
            count=count,
            total_bases=total_bases,
            min_length=min_len,
            max_length=max_len,
            mean_length=mean_len,
            median_length=median_len,
            mean_gc_content=mean_gc,
            n50=n50,
            total_ambiguous=total_ambiguous
        )

    def __str__(self) -> str:
        return (
            f"SequenceSetStats {{\n"
            f"  count: {self.count}\n"
            f"  total_bases: {self.total_bases}\n"
            f"  length range: {self.min_length} - {self.max_length}\n"
            f"  mean length: {self.mean_length:.1f}\n"
            f"  median length: {self.median_length}\n"
            f"  mean GC: {self.mean_gc_content * 100:.1f}%\n"
            f"  N50: {self.n50}\n"
            f"  ambiguous bases: {self.total_ambiguous}\n"
            f"}}"
        )


@dataclass
class QualityDistribution:
    """Quality score distribution."""
    poor_count: int
    low_count: int
    medium_count: int
    high_count: int
    excellent_count: int
    total: int

    @classmethod
    def from_categories(cls, categories: List[QualityCategory]) -> 'QualityDistribution':
        """Create distribution from list of categories."""
        counts = Counter(categories)

        return cls(
            poor_count=counts[QualityCategory.POOR],
            low_count=counts[QualityCategory.LOW],
            medium_count=counts[QualityCategory.MEDIUM],
            high_count=counts[QualityCategory.HIGH],
            excellent_count=counts[QualityCategory.EXCELLENT],
            total=len(categories)
        )

    def acceptable_ratio(self) -> float:
        """Return proportion of reads at or above medium quality."""
        acceptable = self.medium_count + self.high_count + self.excellent_count
        return acceptable / self.total if self.total > 0 else 0.0

    def __str__(self) -> str:
        return (
            f"QualityDistribution {{\n"
            f"  Poor (Q<10): {self.poor_count}\n"
            f"  Low (Q10-20): {self.low_count}\n"
            f"  Medium (Q20-30): {self.medium_count}\n"
            f"  High (Q30-40): {self.high_count}\n"
            f"  Excellent (Q40): {self.excellent_count}\n"
            f"}}"
        )


@dataclass
class ReadSetStats:
    """
    Statistics for a collection of reads.

    Note: This is a simplified version. In a full implementation,
    this would work with Read objects that combine Sequence and QualityScores.
    """
    count: int
    total_bases: int
    min_length: int
    max_length: int
    mean_length: float
    mean_quality: float
    median_quality: float
    high_quality_count: int
    quality_distribution: QualityDistribution

    @classmethod
    def from_reads(
        cls,
        sequences: List[Sequence],
        qualities: List[QualityScores]
    ) -> 'ReadSetStats':
        """
        Calculate statistics for a collection of reads.

        Args:
            sequences: List of sequences
            qualities: List of quality scores (must match sequences length)
        """
        if len(sequences) != len(qualities):
            raise ValueError("Sequences and qualities must have same length")
        if len(sequences) == 0:
            raise ValueError("Read list cannot be empty")

        count = len(sequences)
        lengths = [len(seq) for seq in sequences]
        total_bases = sum(lengths)
        min_len = min(lengths)
        max_len = max(lengths)
        mean_len = total_bases / count

        # Quality statistics
        avg_qualities = [q.average() for q in qualities]
        mean_quality = sum(avg_qualities) / count

        sorted_qualities = sorted(avg_qualities)
        mid = count // 2
        if count % 2 == 0:
            median_quality = (sorted_qualities[mid - 1] + sorted_qualities[mid]) / 2
        else:
            median_quality = sorted_qualities[mid]

        # Count high quality reads (avg >= Q30)
        high_quality_count = sum(1 for q in avg_qualities if q >= 30)

        # Build quality distribution
        categories = [q.categorize() for q in qualities]
        distribution = QualityDistribution.from_categories(categories)

        return cls(
            count=count,
            total_bases=total_bases,
            min_length=min_len,
            max_length=max_len,
            mean_length=mean_len,
            mean_quality=mean_quality,
            median_quality=median_quality,
            high_quality_count=high_quality_count,
            quality_distribution=distribution
        )

    def high_quality_ratio(self) -> float:
        """Return proportion of high-quality reads."""
        return self.high_quality_count / self.count if self.count > 0 else 0.0

    def __str__(self) -> str:
        return (
            f"ReadSetStats {{\n"
            f"  count: {self.count}\n"
            f"  total_bases: {self.total_bases}\n"
            f"  length range: {self.min_length} - {self.max_length}\n"
            f"  mean length: {self.mean_length:.1f}\n"
            f"  mean quality: {self.mean_quality:.1f}\n"
            f"  median quality: {self.median_quality:.1f}\n"
            f"  high quality reads: {self.high_quality_count} ({self.high_quality_ratio() * 100:.1f}%)\n"
            f"}}"
        )


@dataclass
class GCHistogram:
    """
    GC content histogram with bins.

    Divides GC content (0-100%) into bins for visualization.
    """
    bins: List[int]
    bin_size: float
    num_bins: int

    @classmethod
    def from_sequences(cls, sequences: List[Sequence], num_bins: int = 20) -> 'GCHistogram':
        """Create a GC content histogram from sequences."""
        if len(sequences) == 0:
            raise ValueError("Sequence list cannot be empty")

        bin_size = 1.0 / num_bins
        bins = [0] * num_bins

        for seq in sequences:
            gc = seq.gc_content()
            bin_index = min(int(gc / bin_size), num_bins - 1)
            bins[bin_index] += 1

        return cls(bins=bins, bin_size=bin_size, num_bins=num_bins)

    def mode_bin(self) -> Tuple[float, float]:
        """Return the most common GC content range."""
        max_count = max(self.bins)
        max_bin = self.bins.index(max_count)

        start = max_bin * self.bin_size
        end = start + self.bin_size
        return (start, end)

    def __str__(self) -> str:
        lines = ["GC Content Histogram:"]
        for i in range(self.num_bins):
            start = int(i * self.bin_size * 100)
            end = start + int(self.bin_size * 100)
            count = self.bins[i]
            bar = '#' * (count // 10)  # Scale down for display
            lines.append(f"{start:2d}-{end:2d}%: {bar} ({count})")
        return '\n'.join(lines)


@dataclass
class LengthHistogram:
    """Length histogram for sequences."""
    bins: List[int]
    min_length: int
    max_length: int
    bin_width: int
    num_bins: int

    @classmethod
    def from_sequences(cls, sequences: List[Sequence], num_bins: int = 10) -> 'LengthHistogram':
        """Create a length histogram from sequences."""
        if len(sequences) == 0:
            raise ValueError("Sequence list cannot be empty")
        if num_bins <= 0:
            raise ValueError("num_bins must be positive")

        lengths = [len(seq) for seq in sequences]
        min_len = min(lengths)
        max_len = max(lengths)

        length_range = max_len - min_len
        bin_width = max(1, (length_range // num_bins) + 1)

        bins = [0] * num_bins

        for length in lengths:
            bin_index = min((length - min_len) // bin_width, num_bins - 1)
            bins[bin_index] += 1

        return cls(
            bins=bins,
            min_length=min_len,
            max_length=max_len,
            bin_width=bin_width,
            num_bins=num_bins
        )

    def __str__(self) -> str:
        lines = ["Length Histogram:"]
        for i in range(self.num_bins):
            start = self.min_length + i * self.bin_width
            end = start + self.bin_width
            count = self.bins[i]
            bar = '#' * (count // 5)  # Scale down for display
            lines.append(f"{start:5d}-{end:5d}: {bar} ({count})")
        return '\n'.join(lines)
