"""
Tests for the Sequence module.
"""

import pytest
from bioflow.sequence import (
    Sequence, SequenceType, SequenceError,
    EmptySequenceError, InvalidBaseError
)


class TestSequenceCreation:
    """Tests for sequence creation and validation."""

    def test_create_valid_dna_sequence(self):
        """Test creating a valid DNA sequence."""
        seq = Sequence.new("ATCGATCG")
        assert seq.bases == "ATCGATCG"
        assert seq.seq_type == SequenceType.DNA
        assert len(seq) == 8

    def test_create_sequence_normalizes_to_uppercase(self):
        """Test that sequences are normalized to uppercase."""
        seq = Sequence.new("atcgatcg")
        assert seq.bases == "ATCGATCG"

    def test_create_empty_sequence_raises_error(self):
        """Test that empty sequences raise an error."""
        with pytest.raises(EmptySequenceError):
            Sequence.new("")

    def test_create_invalid_base_raises_error(self):
        """Test that invalid bases raise an error."""
        with pytest.raises(InvalidBaseError) as exc_info:
            Sequence.new("ATCGXATCG")
        assert exc_info.value.position == 4
        assert exc_info.value.found == 'X'

    def test_sequence_with_id(self):
        """Test creating sequence with ID."""
        seq = Sequence.with_id("ATCG", "seq1")
        assert seq.id == "seq1"
        assert seq.bases == "ATCG"

    def test_sequence_with_metadata(self):
        """Test creating sequence with full metadata."""
        seq = Sequence.with_metadata(
            "ATCG",
            "seq1",
            "Test sequence",
            SequenceType.DNA
        )
        assert seq.id == "seq1"
        assert seq.description == "Test sequence"
        assert seq.seq_type == SequenceType.DNA


class TestSequenceOperations:
    """Tests for sequence operations."""

    def test_gc_content(self):
        """Test GC content calculation."""
        seq = Sequence.new("GCGC")  # 100% GC
        assert seq.gc_content() == 1.0

        seq2 = Sequence.new("ATAT")  # 0% GC
        assert seq2.gc_content() == 0.0

        seq3 = Sequence.new("ATGC")  # 50% GC
        assert seq3.gc_content() == 0.5

    def test_at_content(self):
        """Test AT content calculation."""
        seq = Sequence.new("ATAT")  # 100% AT
        assert seq.at_content() == 1.0

        seq2 = Sequence.new("GCGC")  # 0% AT
        assert seq2.at_content() == 0.0

    def test_base_counts(self):
        """Test base counting."""
        seq = Sequence.new("AACCCGGGTTTN")
        a, c, g, t, n = seq.base_counts()
        assert a == 2
        assert c == 3
        assert g == 3
        assert t == 3
        assert n == 1

    def test_complement(self):
        """Test DNA complement."""
        seq = Sequence.new("ATCG")
        comp = seq.complement()
        assert comp.bases == "TAGC"

    def test_reverse(self):
        """Test sequence reversal."""
        seq = Sequence.new("ATCG")
        rev = seq.reverse()
        assert rev.bases == "GCTA"

    def test_reverse_complement(self):
        """Test reverse complement."""
        seq = Sequence.new("ATCG")
        rc = seq.reverse_complement()
        assert rc.bases == "CGAT"

    def test_transcribe(self):
        """Test DNA to RNA transcription."""
        seq = Sequence.new("ATCG")
        rna = seq.transcribe()
        assert rna.bases == "AUCG"
        assert rna.seq_type == SequenceType.RNA

    def test_concat(self):
        """Test sequence concatenation."""
        seq1 = Sequence.new("ATCG")
        seq2 = Sequence.new("GCTA")
        concat = seq1.concat(seq2)
        assert concat.bases == "ATCGGCTA"
        assert len(concat) == 8


class TestMotifOperations:
    """Tests for motif finding."""

    def test_contains_motif(self):
        """Test motif detection."""
        seq = Sequence.new("ATCGATCGATCG")
        assert seq.contains_motif("GATC")
        assert not seq.contains_motif("GGGG")

    def test_find_motif_positions(self):
        """Test finding motif positions."""
        seq = Sequence.new("ATCGATCGATCG")
        positions = seq.find_motif_positions("GATC")
        assert positions == [3, 7]

    def test_find_motif_positions_no_match(self):
        """Test finding motif positions with no matches."""
        seq = Sequence.new("ATCGATCGATCG")
        positions = seq.find_motif_positions("GGGG")
        assert positions == []


class TestSubsequence:
    """Tests for subsequence extraction."""

    def test_subsequence(self):
        """Test subsequence extraction."""
        seq = Sequence.new("ATCGATCG")
        subseq = seq.subsequence(2, 6)
        assert subseq.bases == "CGAT"

    def test_subsequence_invalid_start(self):
        """Test subsequence with invalid start."""
        seq = Sequence.new("ATCG")
        with pytest.raises(ValueError):
            seq.subsequence(-1, 4)

    def test_subsequence_end_exceeds_length(self):
        """Test subsequence with end exceeding length."""
        seq = Sequence.new("ATCG")
        with pytest.raises(ValueError):
            seq.subsequence(0, 10)


class TestSequenceOutput:
    """Tests for sequence output formats."""

    def test_to_fasta(self):
        """Test FASTA output."""
        seq = Sequence.with_id("ATCG", "seq1")
        fasta = seq.to_fasta()
        assert ">seq1" in fasta
        assert "ATCG" in fasta

    def test_str_with_id(self):
        """Test string representation with ID."""
        seq = Sequence.with_id("ATCG", "seq1")
        s = str(seq)
        assert ">seq1" in s
        assert "ATCG" in s


class TestAmbiguousBases:
    """Tests for handling ambiguous bases."""

    def test_has_ambiguous(self):
        """Test detection of ambiguous bases."""
        seq1 = Sequence.new("ATCG")
        assert not seq1.has_ambiguous()

        seq2 = Sequence.new("ATNCG")
        assert seq2.has_ambiguous()

    def test_count_ambiguous(self):
        """Test counting ambiguous bases."""
        seq = Sequence.new("ATNNCG")
        assert seq.count_ambiguous() == 2


class TestEquality:
    """Tests for sequence equality."""

    def test_equal_sequences(self):
        """Test equality of identical sequences."""
        seq1 = Sequence.new("ATCG")
        seq2 = Sequence.new("ATCG")
        assert seq1 == seq2

    def test_unequal_sequences(self):
        """Test inequality of different sequences."""
        seq1 = Sequence.new("ATCG")
        seq2 = Sequence.new("GCTA")
        assert seq1 != seq2

    def test_hash_equal_sequences(self):
        """Test that equal sequences have equal hashes."""
        seq1 = Sequence.new("ATCG")
        seq2 = Sequence.new("ATCG")
        assert hash(seq1) == hash(seq2)
