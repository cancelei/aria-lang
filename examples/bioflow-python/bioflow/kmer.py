"""
BioFlow - K-mer Counting and Analysis

K-mer frequency analysis with Python implementation.

Comparison with Aria:

Aria uses compile-time invariants:
    struct KMer
      invariant self.sequence.len() == self.k
      invariant self.k > 0

    struct KMerCounts
      invariant self.counts.all(|(kmer, _)| kmer.len() == self.k)

Python relies on runtime validation:
    def __post_init__(self):
        if len(self.sequence) != self.k:
            raise ValueError(...)
"""

from typing import Dict, List, Tuple, Optional
from dataclasses import dataclass, field
from collections import defaultdict

from .sequence import Sequence


@dataclass
class KMer:
    """
    A single k-mer with its properties.

    Aria equivalent:
        struct KMer
          sequence: String
          k: Int
          invariant self.sequence.len() == self.k
          invariant self.k > 0
    """
    sequence: str
    k: int = field(init=False)

    def __post_init__(self):
        self.sequence = self.sequence.upper()
        self.k = len(self.sequence)

        if self.k == 0:
            raise ValueError("K-mer sequence cannot be empty")

    @classmethod
    def new(cls, sequence: str) -> 'KMer':
        """Create a new k-mer."""
        return cls(sequence=sequence)

    def reverse_complement(self) -> 'KMer':
        """
        Return the reverse complement of this k-mer.

        Aria equivalent:
            fn reverse_complement(self) -> KMer
              ensures result.k == self.k
        """
        comp_map = {'A': 'T', 'T': 'A', 'C': 'G', 'G': 'C', 'N': 'N'}
        rc = ''.join(comp_map.get(c, 'N') for c in reversed(self.sequence))
        return KMer(sequence=rc)

    def canonical(self) -> 'KMer':
        """
        Return the canonical form (lexicographically smaller of forward/reverse complement).

        Aria equivalent:
            fn canonical(self) -> KMer
              ensures result.k == self.k
        """
        rc = self.reverse_complement()
        if self.sequence < rc.sequence:
            return self
        return rc

    def __str__(self) -> str:
        return self.sequence

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, KMer):
            return False
        return self.sequence == other.sequence

    def __hash__(self) -> int:
        return hash(self.sequence)


@dataclass
class KMerCounter:
    """
    K-mer counts storage and operations.

    Aria equivalent:
        struct KMerCounts
          k: Int
          counts: [(String, Int)]
          total_kmers: Int
          invariant self.k > 0
          invariant self.counts.all(|(kmer, _)| kmer.len() == self.k)
    """
    k: int
    counts: Dict[str, int] = field(default_factory=dict)
    total_kmers: int = 0

    def __post_init__(self):
        """Validate k value."""
        if self.k <= 0:
            raise ValueError("K must be positive")

    @classmethod
    def new(cls, k: int) -> 'KMerCounter':
        """
        Create an empty k-mer counter.

        Aria equivalent:
            fn new(k: Int) -> KMerCounts
              requires k > 0
              ensures result.k == k
              ensures result.total_kmers == 0
        """
        return cls(k=k)

    def add(self, kmer: str, count: int = 1) -> None:
        """
        Add a k-mer count.

        Aria equivalent:
            fn add(mut self, kmer: String, count: Int)
              requires kmer.len() == self.k
              requires count > 0
        """
        if len(kmer) != self.k:
            raise ValueError(f"K-mer length {len(kmer)} doesn't match k={self.k}")
        if count <= 0:
            raise ValueError("Count must be positive")

        kmer = kmer.upper()
        self.counts[kmer] = self.counts.get(kmer, 0) + count
        self.total_kmers += count

    def count_kmers(self, sequence: str) -> None:
        """
        Count all k-mers in a sequence string.

        This is the main counting function using the sliding window approach.
        """
        sequence = sequence.upper()
        for i in range(len(sequence) - self.k + 1):
            kmer = sequence[i:i + self.k]
            if 'N' not in kmer:  # Skip k-mers containing ambiguous bases
                self.add(kmer)

    def count_from_sequence(self, sequence: Sequence) -> None:
        """Count all k-mers from a Sequence object."""
        self.count_kmers(sequence.bases)

    def get_count(self, kmer: str) -> int:
        """
        Get the count for a specific k-mer.

        Aria equivalent:
            fn get_count(self, kmer: String) -> Int
              requires kmer.len() == self.k
              ensures result >= 0
        """
        if len(kmer) != self.k:
            raise ValueError(f"K-mer length doesn't match k={self.k}")
        return self.counts.get(kmer.upper(), 0)

    def unique_count(self) -> int:
        """
        Return the number of unique k-mers.

        Aria equivalent:
            fn unique_count(self) -> Int
              ensures result >= 0
        """
        return len(self.counts)

    def most_frequent(self, n: int) -> List[Tuple[str, int]]:
        """
        Return the n most frequent k-mers.

        Aria equivalent:
            fn most_frequent(self, n: Int) -> [(String, Int)]
              requires n > 0
              ensures result.len() <= n
              ensures result.len() <= self.counts.len()
        """
        if n <= 0:
            raise ValueError("N must be positive")

        sorted_counts = sorted(
            self.counts.items(),
            key=lambda x: x[1],
            reverse=True
        )
        return sorted_counts[:n]

    def least_frequent(self, n: int) -> List[Tuple[str, int]]:
        """
        Return the n least frequent k-mers.

        Aria equivalent:
            fn least_frequent(self, n: Int) -> [(String, Int)]
              requires n > 0
              ensures result.len() <= n
        """
        if n <= 0:
            raise ValueError("N must be positive")

        sorted_counts = sorted(
            self.counts.items(),
            key=lambda x: x[1]
        )
        return sorted_counts[:n]

    def frequency(self, kmer: str) -> float:
        """
        Calculate frequency of a k-mer.

        Aria equivalent:
            fn frequency(self, kmer: String) -> Float
              requires kmer.len() == self.k
              ensures result >= 0.0 and result <= 1.0
        """
        if self.total_kmers == 0:
            return 0.0
        return self.get_count(kmer) / self.total_kmers

    def filter_by_count(self, min_count: int) -> List[Tuple[str, int]]:
        """
        Return k-mers with count above threshold.

        Aria equivalent:
            fn filter_by_count(self, min_count: Int) -> [(String, Int)]
              requires min_count > 0
              ensures result.all(|(_, count)| count >= min_count)
        """
        if min_count <= 0:
            raise ValueError("min_count must be positive")

        return [(k, c) for k, c in self.counts.items() if c >= min_count]

    def merge(self, other: 'KMerCounter') -> None:
        """
        Merge another KMerCounter into this one.

        Aria equivalent:
            fn merge(mut self, other: KMerCounts)
              requires self.k == other.k
        """
        if self.k != other.k:
            raise ValueError("K values must match")

        for kmer, count in other.counts.items():
            self.counts[kmer] = self.counts.get(kmer, 0) + count
            self.total_kmers += count

    def __str__(self) -> str:
        return f"KMerCounter {{ k: {self.k}, unique: {self.unique_count()}, total: {self.total_kmers} }}"


def count_kmers(sequence: Sequence, k: int) -> KMerCounter:
    """
    Count all k-mers in a sequence.

    Aria equivalent:
        fn count_kmers(sequence: Sequence, k: Int) -> KMerCounts
          requires k > 0
          requires k <= sequence.len()
          ensures result.k == k
          ensures result.total_kmers == sequence.len() - k + 1
    """
    if k <= 0:
        raise ValueError("K must be positive")
    if k > len(sequence):
        raise ValueError("K cannot exceed sequence length")

    counter = KMerCounter.new(k)
    counter.count_from_sequence(sequence)
    return counter


def most_frequent_kmers(sequence: Sequence, k: int, n: int) -> List[Tuple[str, int]]:
    """
    Return the n most frequent k-mers.

    Aria equivalent:
        fn most_frequent_kmers(sequence: Sequence, k: Int, n: Int) -> [(String, Int)]
          requires k > 0 and k <= sequence.len()
          requires n > 0
          ensures result.len() <= n
    """
    counter = count_kmers(sequence, k)
    return counter.most_frequent(n)


def kmer_spectrum(sequence: Sequence, k: int) -> List[Tuple[int, int]]:
    """
    Generate k-mer spectrum (count distribution).

    Returns:
        List of (count, number_of_kmers_with_that_count) tuples

    Aria equivalent:
        fn kmer_spectrum(sequence: Sequence, k: Int) -> [(Int, Int)]
          requires k > 0 and k <= sequence.len()
    """
    counter = count_kmers(sequence, k)

    # Build spectrum (count -> number of k-mers with that count)
    spectrum_map: Dict[int, int] = defaultdict(int)
    for _, count in counter.counts.items():
        spectrum_map[count] += 1

    # Sort by count
    return sorted(spectrum_map.items())


def find_unique_kmers(sequence: Sequence, k: int) -> List[str]:
    """
    Find k-mers occurring exactly once.

    Aria equivalent:
        fn find_unique_kmers(sequence: Sequence, k: Int) -> [String]
          requires k > 0 and k <= sequence.len()
          ensures result.all(|kmer| kmer.len() == k)
    """
    counter = count_kmers(sequence, k)
    return [kmer for kmer, count in counter.counts.items() if count == 1]


def kmer_distance(seq1: Sequence, seq2: Sequence, k: int) -> float:
    """
    Calculate k-mer distance between two sequences using Jaccard distance.

    Jaccard distance = 1 - (intersection / union)

    Aria equivalent:
        fn kmer_distance(seq1: Sequence, seq2: Sequence, k: Int) -> Float
          requires k > 0
          requires k <= seq1.len() and k <= seq2.len()
          ensures result >= 0.0 and result <= 1.0
    """
    if k <= 0:
        raise ValueError("K must be positive")
    if k > len(seq1) or k > len(seq2):
        raise ValueError("K cannot exceed sequence lengths")

    counter1 = count_kmers(seq1, k)
    counter2 = count_kmers(seq2, k)

    set1 = set(counter1.counts.keys())
    set2 = set(counter2.counts.keys())

    intersection = len(set1 & set2)
    union = len(set1 | set2)

    if union == 0:
        return 0.0

    return 1.0 - (intersection / union)


def shared_kmers(seq1: Sequence, seq2: Sequence, k: int) -> List[str]:
    """
    Find shared k-mers between two sequences.

    Aria equivalent:
        fn shared_kmers(seq1: Sequence, seq2: Sequence, k: Int) -> [String]
          requires k > 0
          requires k <= seq1.len() and k <= seq2.len()
    """
    counter1 = count_kmers(seq1, k)
    counter2 = count_kmers(seq2, k)

    set1 = set(counter1.counts.keys())
    set2 = set(counter2.counts.keys())

    return list(set1 & set2)


def count_kmers_canonical(sequence: Sequence, k: int) -> KMerCounter:
    """
    Count canonical k-mers (treating reverse complements as same).

    Aria equivalent:
        fn count_kmers_canonical(sequence: Sequence, k: Int) -> KMerCounts
          requires k > 0 and k <= sequence.len()
          ensures result.k == k
    """
    if k <= 0:
        raise ValueError("K must be positive")
    if k > len(sequence):
        raise ValueError("K cannot exceed sequence length")

    counter = KMerCounter.new(k)

    for i in range(len(sequence.bases) - k + 1):
        kmer_str = sequence.bases[i:i + k]

        if 'N' not in kmer_str:
            kmer = KMer(kmer_str)
            canonical = kmer.canonical()
            counter.add(canonical.sequence)

    return counter


def estimate_genome_size(total_kmers: int, peak_coverage: int, k: int) -> int:
    """
    Estimate genome size using k-mer spectrum.

    Uses the peak count method: genome_size ~ total_kmers / peak_coverage

    Aria equivalent:
        fn estimate_genome_size(total_kmers: Int, peak_coverage: Int, k: Int) -> Int
          requires total_kmers > 0
          requires peak_coverage > 0
          requires k > 0
          ensures result > 0
    """
    if total_kmers <= 0 or peak_coverage <= 0 or k <= 0:
        raise ValueError("All parameters must be positive")

    return total_kmers // peak_coverage


def kmer_positions(sequence: Sequence, kmer: str) -> List[int]:
    """
    Find all positions of a k-mer in a sequence.

    Aria equivalent:
        fn kmer_positions(sequence: Sequence, kmer: String) -> [Int]
          requires kmer.len() > 0
          requires kmer.len() <= sequence.len()
          ensures result.all(|pos| pos >= 0 and pos <= sequence.len() - kmer.len())
    """
    if len(kmer) == 0:
        raise ValueError("K-mer cannot be empty")
    if len(kmer) > len(sequence):
        raise ValueError("K-mer cannot be longer than sequence")

    kmer = kmer.upper()
    positions = []

    for i in range(len(sequence.bases) - len(kmer) + 1):
        if sequence.bases[i:i + len(kmer)] == kmer:
            positions.append(i)

    return positions
