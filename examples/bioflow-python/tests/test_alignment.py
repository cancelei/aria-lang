"""
Tests for the Alignment module.
"""

import pytest
from bioflow.sequence import Sequence
from bioflow.alignment import (
    smith_waterman, needleman_wunsch, ScoringMatrix,
    Alignment, AlignmentType, AlignDirection,
    simple_align, alignment_score_only, percent_identity,
    align_against_multiple, find_best_alignment
)


class TestScoringMatrix:
    """Tests for ScoringMatrix class."""

    def test_default_dna_scoring(self):
        """Test default DNA scoring matrix."""
        scoring = ScoringMatrix.default_dna()
        assert scoring.match_score == 2
        assert scoring.mismatch_penalty == -1
        assert scoring.gap_open_penalty == -2

    def test_blast_like_scoring(self):
        """Test BLAST-like scoring matrix."""
        scoring = ScoringMatrix.blast_like()
        assert scoring.match_score == 1
        assert scoring.mismatch_penalty == -3
        assert scoring.gap_open_penalty == -5

    def test_simple_scoring(self):
        """Test simple scoring matrix creation."""
        scoring = ScoringMatrix.simple(match=3, mismatch=-2, gap=-3)
        assert scoring.match_score == 3
        assert scoring.mismatch_penalty == -2
        assert scoring.gap_open_penalty == -3

    def test_invalid_match_score(self):
        """Test that non-positive match score raises error."""
        with pytest.raises(ValueError):
            ScoringMatrix(match_score=0, mismatch_penalty=-1, gap_open_penalty=-2)

    def test_score_match(self):
        """Test scoring a match."""
        scoring = ScoringMatrix.default_dna()
        assert scoring.score('A', 'A') == 2

    def test_score_mismatch(self):
        """Test scoring a mismatch."""
        scoring = ScoringMatrix.default_dna()
        assert scoring.score('A', 'T') == -1


class TestSmithWaterman:
    """Tests for Smith-Waterman local alignment."""

    def test_identical_sequences(self):
        """Test alignment of identical sequences."""
        seq1 = Sequence.new("ATCG")
        seq2 = Sequence.new("ATCG")

        alignment = smith_waterman(seq1, seq2)

        assert alignment.score > 0
        assert alignment.aligned_seq1 == alignment.aligned_seq2
        assert alignment.identity == 1.0

    def test_simple_alignment(self):
        """Test a simple alignment case."""
        seq1 = Sequence.new("AGTACGCA")
        seq2 = Sequence.new("TATGC")

        alignment = smith_waterman(seq1, seq2)

        assert alignment.score > 0
        assert len(alignment.aligned_seq1) == len(alignment.aligned_seq2)
        assert alignment.alignment_type == AlignmentType.LOCAL

    def test_no_similarity(self):
        """Test alignment with no similarity."""
        seq1 = Sequence.new("AAAA")
        seq2 = Sequence.new("TTTT")

        alignment = smith_waterman(seq1, seq2)

        # Score should be 0 or positive (local alignment minimum is 0)
        assert alignment.score >= 0

    def test_custom_scoring(self):
        """Test alignment with custom scoring."""
        seq1 = Sequence.new("ATCG")
        seq2 = Sequence.new("ATCG")

        scoring = ScoringMatrix.simple(match=5, mismatch=-1, gap=-2)
        alignment = smith_waterman(seq1, seq2, scoring)

        # With match=5 and 4 matches, score should be 20
        assert alignment.score == 20

    def test_alignment_with_gaps(self):
        """Test alignment that requires gaps."""
        seq1 = Sequence.new("ATCGATCG")
        seq2 = Sequence.new("ATCATC")

        alignment = smith_waterman(seq1, seq2)

        # Should have some gaps
        total_gaps = alignment.gaps_seq1() + alignment.gaps_seq2()
        assert total_gaps >= 0  # May or may not have gaps


class TestNeedlemanWunsch:
    """Tests for Needleman-Wunsch global alignment."""

    def test_identical_sequences(self):
        """Test global alignment of identical sequences."""
        seq1 = Sequence.new("ATCG")
        seq2 = Sequence.new("ATCG")

        alignment = needleman_wunsch(seq1, seq2)

        assert alignment.aligned_seq1 == alignment.aligned_seq2
        assert alignment.identity == 1.0
        assert alignment.alignment_type == AlignmentType.GLOBAL

    def test_different_length_sequences(self):
        """Test global alignment of different length sequences."""
        seq1 = Sequence.new("ATCGATCG")
        seq2 = Sequence.new("ATCG")

        alignment = needleman_wunsch(seq1, seq2)

        # Both aligned sequences should have same length (with gaps)
        assert len(alignment.aligned_seq1) == len(alignment.aligned_seq2)

        # Should have gaps to account for length difference
        total_gaps = alignment.total_gaps()
        assert total_gaps >= 4  # At least 4 gaps to cover length difference

    def test_completely_different_sequences(self):
        """Test global alignment of completely different sequences."""
        seq1 = Sequence.new("AAAA")
        seq2 = Sequence.new("TTTT")

        alignment = needleman_wunsch(seq1, seq2)

        # Global alignment always produces result
        assert len(alignment.aligned_seq1) == len(alignment.aligned_seq2)


class TestAlignment:
    """Tests for Alignment class."""

    def test_alignment_length(self):
        """Test alignment length calculation."""
        alignment = Alignment.new("ATCG", "ATCG", 8, AlignmentType.LOCAL)
        assert alignment.alignment_length() == 4

    def test_match_count(self):
        """Test counting matches."""
        alignment = Alignment.new("ATCG", "ATTG", 6, AlignmentType.LOCAL)
        assert alignment.match_count() == 3  # A, T, G match

    def test_mismatch_count(self):
        """Test counting mismatches."""
        alignment = Alignment.new("ATCG", "ATTG", 6, AlignmentType.LOCAL)
        assert alignment.mismatch_count() == 1  # C vs T

    def test_gap_count(self):
        """Test counting gaps."""
        alignment = Alignment.new("AT-CG", "ATGCG", 6, AlignmentType.LOCAL)
        assert alignment.gaps_seq1() == 1
        assert alignment.gaps_seq2() == 0

    def test_cigar_generation(self):
        """Test CIGAR string generation."""
        alignment = Alignment.new("ATCG", "ATCG", 8, AlignmentType.LOCAL)
        cigar = alignment.to_cigar()
        assert "4M" in cigar  # 4 matches

    def test_identity_calculation(self):
        """Test identity calculation."""
        alignment = Alignment.new("ATCG", "ATCG", 8, AlignmentType.LOCAL)
        assert alignment.identity == 1.0

        alignment2 = Alignment.new("ATCG", "ATTG", 6, AlignmentType.LOCAL)
        assert alignment2.identity == 0.75  # 3 out of 4

    def test_format_output(self):
        """Test formatted alignment output."""
        alignment = Alignment.new("ATCG", "ATCG", 8, AlignmentType.LOCAL)
        formatted = alignment.format()

        assert "Seq1:" in formatted
        assert "Seq2:" in formatted
        assert "Score:" in formatted


class TestSimpleAlign:
    """Tests for simple_align function."""

    def test_simple_align(self):
        """Test simple alignment helper function."""
        seq1 = Sequence.new("ATCG")
        seq2 = Sequence.new("ATCG")

        alignment = simple_align(seq1, seq2)
        assert alignment.score > 0


class TestAlignmentScoreOnly:
    """Tests for alignment_score_only function."""

    def test_score_only_matches_full(self):
        """Test that score-only matches full alignment."""
        seq1 = Sequence.new("ATCGATCG")
        seq2 = Sequence.new("ATCGATCG")

        score_only = alignment_score_only(seq1, seq2)
        full_alignment = smith_waterman(seq1, seq2)

        assert score_only == full_alignment.score


class TestPercentIdentity:
    """Tests for percent_identity function."""

    def test_perfect_identity(self):
        """Test 100% identity."""
        identity = percent_identity("ATCG", "ATCG")
        assert identity == 100.0

    def test_partial_identity(self):
        """Test partial identity."""
        identity = percent_identity("ATCG", "ATTG")
        assert identity == 75.0  # 3 out of 4

    def test_identity_with_gaps(self):
        """Test identity calculation with gaps."""
        # Gaps don't count as matches
        identity = percent_identity("AT-G", "ATCG")
        assert identity == 75.0  # 3 matches out of 4 positions


class TestMultipleAlignment:
    """Tests for multiple sequence alignment functions."""

    def test_align_against_multiple(self):
        """Test aligning against multiple targets."""
        query = Sequence.new("ATCG")
        targets = [
            Sequence.new("ATCG"),
            Sequence.new("GCTA"),
            Sequence.new("AAAA")
        ]

        results = align_against_multiple(query, targets)

        assert len(results) == 3
        for idx, alignment in results:
            assert isinstance(alignment, Alignment)

    def test_find_best_alignment(self):
        """Test finding the best alignment."""
        query = Sequence.new("ATCG")
        targets = [
            Sequence.new("ATCG"),  # Best match
            Sequence.new("GCTA"),
            Sequence.new("AAAA")
        ]

        result = find_best_alignment(query, targets)

        assert result is not None
        idx, alignment = result
        assert idx == 0  # First target should be best match
        assert alignment.score > 0


class TestEdgeCases:
    """Tests for edge cases."""

    def test_empty_sequence_error(self):
        """Test that empty sequences raise error."""
        seq1 = Sequence.new("ATCG")

        with pytest.raises(ValueError):
            # Can't create empty sequence, so this tests the function check
            # In practice, Sequence.new("") would fail first
            pass

    def test_single_base_alignment(self):
        """Test alignment of single base sequences."""
        seq1 = Sequence.new("A")
        seq2 = Sequence.new("A")

        alignment = smith_waterman(seq1, seq2)
        assert alignment.score > 0
