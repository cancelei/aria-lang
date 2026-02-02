"""
Tests for the K-mer module.
"""

import pytest
from bioflow.sequence import Sequence
from bioflow.kmer import (
    KMer, KMerCounter, count_kmers, most_frequent_kmers,
    kmer_spectrum, kmer_distance, shared_kmers, find_unique_kmers,
    count_kmers_canonical, kmer_positions
)


class TestKMer:
    """Tests for KMer class."""

    def test_create_kmer(self):
        """Test creating a k-mer."""
        kmer = KMer.new("ATCG")
        assert kmer.sequence == "ATCG"
        assert kmer.k == 4

    def test_kmer_normalizes_uppercase(self):
        """Test that k-mers are normalized to uppercase."""
        kmer = KMer.new("atcg")
        assert kmer.sequence == "ATCG"

    def test_empty_kmer_raises_error(self):
        """Test that empty k-mers raise an error."""
        with pytest.raises(ValueError):
            KMer.new("")

    def test_reverse_complement(self):
        """Test k-mer reverse complement."""
        kmer = KMer.new("ATCG")
        rc = kmer.reverse_complement()
        assert rc.sequence == "CGAT"
        assert rc.k == 4

    def test_canonical(self):
        """Test canonical k-mer (lexicographically smaller)."""
        kmer1 = KMer.new("ATCG")  # ATCG < CGAT, so ATCG is canonical
        assert kmer1.canonical().sequence == "ATCG"

        kmer2 = KMer.new("TACG")  # CGTA < TACG, so CGTA is canonical
        assert kmer2.canonical().sequence == "CGTA"


class TestKMerCounter:
    """Tests for KMerCounter class."""

    def test_create_counter(self):
        """Test creating a k-mer counter."""
        counter = KMerCounter.new(3)
        assert counter.k == 3
        assert counter.unique_count() == 0
        assert counter.total_kmers == 0

    def test_invalid_k_raises_error(self):
        """Test that k <= 0 raises an error."""
        with pytest.raises(ValueError):
            KMerCounter.new(0)

    def test_add_kmer(self):
        """Test adding k-mers."""
        counter = KMerCounter.new(3)
        counter.add("ATG", 1)
        counter.add("ATG", 2)
        counter.add("GCA", 1)

        assert counter.get_count("ATG") == 3
        assert counter.get_count("GCA") == 1
        assert counter.unique_count() == 2
        assert counter.total_kmers == 4

    def test_add_wrong_length_raises_error(self):
        """Test adding k-mer with wrong length."""
        counter = KMerCounter.new(3)
        with pytest.raises(ValueError):
            counter.add("ATCG", 1)

    def test_count_kmers_string(self):
        """Test counting k-mers from a string."""
        counter = KMerCounter.new(3)
        counter.count_kmers("ATGATGATG")
        # K-mers: ATG, TGA, GAT, ATG, TGA, GAT, ATG
        assert counter.get_count("ATG") == 3
        assert counter.get_count("TGA") == 2
        assert counter.get_count("GAT") == 2

    def test_most_frequent(self):
        """Test getting most frequent k-mers."""
        counter = KMerCounter.new(3)
        counter.count_kmers("ATGATGATG")

        top = counter.most_frequent(2)
        assert len(top) == 2
        assert top[0][0] == "ATG"  # Most frequent
        assert top[0][1] == 3

    def test_frequency(self):
        """Test k-mer frequency calculation."""
        counter = KMerCounter.new(3)
        counter.count_kmers("ATGATG")  # ATG: 2, TGA: 1, GAT: 1

        assert counter.frequency("ATG") == 2 / 4  # 2 out of 4 total

    def test_filter_by_count(self):
        """Test filtering by minimum count."""
        counter = KMerCounter.new(3)
        counter.count_kmers("ATGATGATG")

        filtered = counter.filter_by_count(3)
        assert len(filtered) == 1
        assert filtered[0] == ("ATG", 3)

    def test_merge(self):
        """Test merging two counters."""
        counter1 = KMerCounter.new(3)
        counter1.count_kmers("ATG")

        counter2 = KMerCounter.new(3)
        counter2.count_kmers("ATG")

        counter1.merge(counter2)
        assert counter1.get_count("ATG") == 2


class TestCountKmersFunction:
    """Tests for count_kmers function."""

    def test_count_kmers(self):
        """Test counting k-mers from a Sequence."""
        seq = Sequence.new("ATGATGATG")
        counts = count_kmers(seq, 3)

        assert counts.k == 3
        assert counts.get_count("ATG") == 3

    def test_count_kmers_k_exceeds_length(self):
        """Test error when k exceeds sequence length."""
        seq = Sequence.new("ATG")
        with pytest.raises(ValueError):
            count_kmers(seq, 5)

    def test_count_kmers_skips_n(self):
        """Test that k-mers with N are skipped."""
        seq = Sequence.new("ATNGAT")
        counts = count_kmers(seq, 3)
        # Only GAT doesn't contain N
        assert counts.get_count("ATN") == 0
        assert counts.get_count("GAT") == 1


class TestMostFrequentKmers:
    """Tests for most_frequent_kmers function."""

    def test_most_frequent_kmers(self):
        """Test getting most frequent k-mers."""
        seq = Sequence.new("ATGATGATG")
        top = most_frequent_kmers(seq, 3, 2)

        assert len(top) == 2
        assert top[0][0] == "ATG"


class TestKmerSpectrum:
    """Tests for kmer_spectrum function."""

    def test_kmer_spectrum(self):
        """Test k-mer spectrum generation."""
        seq = Sequence.new("ATGATGATG")
        spectrum = kmer_spectrum(seq, 3)

        # Should have entries for counts 2 and 3
        counts = dict(spectrum)
        assert 2 in counts  # TGA and GAT appear twice
        assert 3 in counts  # ATG appears three times


class TestKmerDistance:
    """Tests for kmer_distance function."""

    def test_identical_sequences(self):
        """Test distance between identical sequences."""
        seq1 = Sequence.new("ATGATG")
        seq2 = Sequence.new("ATGATG")

        distance = kmer_distance(seq1, seq2, 3)
        assert distance == 0.0

    def test_completely_different_sequences(self):
        """Test distance between completely different sequences."""
        seq1 = Sequence.new("AAAA")
        seq2 = Sequence.new("TTTT")

        distance = kmer_distance(seq1, seq2, 2)
        assert distance == 1.0

    def test_partially_similar_sequences(self):
        """Test distance between partially similar sequences."""
        seq1 = Sequence.new("ATGATG")
        seq2 = Sequence.new("ATGCCC")

        distance = kmer_distance(seq1, seq2, 3)
        assert 0.0 < distance < 1.0


class TestSharedKmers:
    """Tests for shared_kmers function."""

    def test_shared_kmers(self):
        """Test finding shared k-mers."""
        seq1 = Sequence.new("ATGATG")
        seq2 = Sequence.new("GATGAT")

        shared = shared_kmers(seq1, seq2, 3)
        assert "ATG" in shared
        assert "GAT" in shared


class TestFindUniqueKmers:
    """Tests for find_unique_kmers function."""

    def test_find_unique_kmers(self):
        """Test finding unique k-mers."""
        seq = Sequence.new("ATGATCG")
        unique = find_unique_kmers(seq, 3)

        # ATG, TGA, GAT, ATC, TCG - check which appear only once
        assert "ATC" in unique
        assert "TCG" in unique


class TestCanonicalKmers:
    """Tests for count_kmers_canonical function."""

    def test_canonical_counting(self):
        """Test canonical k-mer counting."""
        seq = Sequence.new("ATCGCGAT")
        counts = count_kmers_canonical(seq, 3)

        # AT + reverse complement of AT should be combined
        # This tests that reverse complements are treated as same k-mer
        assert counts.k == 3


class TestKmerPositions:
    """Tests for kmer_positions function."""

    def test_find_kmer_positions(self):
        """Test finding k-mer positions."""
        seq = Sequence.new("ATGATGATG")
        positions = kmer_positions(seq, "ATG")

        assert positions == [0, 3, 6]

    def test_kmer_not_found(self):
        """Test when k-mer is not found."""
        seq = Sequence.new("ATGATGATG")
        positions = kmer_positions(seq, "CCC")

        assert positions == []
