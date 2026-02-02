#include <gtest/gtest.h>
#include "bioflow/alignment.hpp"

using namespace bioflow;

// ============================================================================
// Scoring Matrix Tests
// ============================================================================

TEST(ScoringMatrixTest, DefaultValues) {
    ScoringMatrix scoring;
    EXPECT_EQ(scoring.match_score, 2);
    EXPECT_EQ(scoring.mismatch_penalty, -1);
    EXPECT_EQ(scoring.gap_open_penalty, -2);
}

TEST(ScoringMatrixTest, Score) {
    ScoringMatrix scoring;
    EXPECT_EQ(scoring.score('A', 'A'), 2);   // Match
    EXPECT_EQ(scoring.score('A', 'T'), -1);  // Mismatch
}

TEST(ScoringMatrixTest, GapPenalty) {
    ScoringMatrix scoring;
    EXPECT_EQ(scoring.gapPenalty(), -2);
    EXPECT_EQ(scoring.gapPenalty(0), 0);
    EXPECT_EQ(scoring.gapPenalty(1), -2);
    EXPECT_EQ(scoring.gapPenalty(2), -3);  // -2 + -1
}

TEST(ScoringMatrixTest, Presets) {
    auto dna = ScoringMatrix::dnaMismatch();
    EXPECT_EQ(dna.match_score, 1);
    EXPECT_EQ(dna.mismatch_penalty, -1);

    auto strict = ScoringMatrix::strictMatch();
    EXPECT_EQ(strict.mismatch_penalty, -3);
    EXPECT_EQ(strict.gap_open_penalty, -5);
}

// ============================================================================
// Smith-Waterman Tests
// ============================================================================

TEST(SmithWatermanTest, IdenticalSequences) {
    Sequence seq1("ACGT");
    Sequence seq2("ACGT");

    auto result = smithWaterman(seq1, seq2);

    EXPECT_EQ(result.score, 8);  // 4 matches * 2
    EXPECT_EQ(result.matches, 4);
    EXPECT_EQ(result.mismatches, 0);
    EXPECT_EQ(result.gaps, 0);
}

TEST(SmithWatermanTest, SingleMismatch) {
    Sequence seq1("ACGT");
    Sequence seq2("AGGT");

    auto result = smithWaterman(seq1, seq2);

    EXPECT_GT(result.score, 0);
    EXPECT_EQ(result.mismatches, 1);
}

TEST(SmithWatermanTest, WithGap) {
    Sequence seq1("ACGT");
    Sequence seq2("AGT");

    auto result = smithWaterman(seq1, seq2);

    EXPECT_GT(result.score, 0);
}

TEST(SmithWatermanTest, NoAlignment) {
    Sequence seq1("AAAA");
    Sequence seq2("CCCC");

    ScoringMatrix strict;
    strict.match_score = 1;
    strict.mismatch_penalty = -10;

    auto result = smithWaterman(seq1, seq2, strict);

    EXPECT_EQ(result.score, 0);
}

TEST(SmithWatermanTest, LocalAlignment) {
    // Local alignment should find the best matching region
    Sequence seq1("AAACGTAAA");
    Sequence seq2("TTCGTTT");

    auto result = smithWaterman(seq1, seq2);

    EXPECT_GT(result.score, 0);
    // Should align the CGT portions
    EXPECT_GE(result.matches, 3);
}

// ============================================================================
// Needleman-Wunsch Tests
// ============================================================================

TEST(NeedlemanWunschTest, IdenticalSequences) {
    Sequence seq1("ACGT");
    Sequence seq2("ACGT");

    auto result = needlemanWunsch(seq1, seq2);

    EXPECT_EQ(result.score, 8);  // 4 matches * 2
    EXPECT_EQ(result.matches, 4);
    EXPECT_EQ(result.gaps, 0);
}

TEST(NeedlemanWunschTest, WithGap) {
    Sequence seq1("ACGT");
    Sequence seq2("ACT");

    auto result = needlemanWunsch(seq1, seq2);

    // Global alignment should include the gap
    EXPECT_GE(result.gaps, 1);
}

TEST(NeedlemanWunschTest, DifferentLengths) {
    Sequence seq1("ACGTACGT");
    Sequence seq2("ACGT");

    auto result = needlemanWunsch(seq1, seq2);

    // Alignment should span the full length of longer sequence
    EXPECT_EQ(result.aligned_seq1.length(), result.aligned_seq2.length());
}

// ============================================================================
// Alignment Result Tests
// ============================================================================

TEST(AlignmentTest, Identity) {
    Alignment aln;
    aln.aligned_seq1 = "ACGT";
    aln.aligned_seq2 = "ACGT";
    aln.matches = 4;
    aln.mismatches = 0;
    aln.gaps = 0;

    EXPECT_DOUBLE_EQ(aln.identity(), 1.0);
}

TEST(AlignmentTest, IdentityWithMismatches) {
    Alignment aln;
    aln.aligned_seq1 = "ACGT";
    aln.aligned_seq2 = "AGGT";
    aln.matches = 3;
    aln.mismatches = 1;
    aln.gaps = 0;

    EXPECT_DOUBLE_EQ(aln.identity(), 0.75);
}

TEST(AlignmentTest, GapRatio) {
    Alignment aln;
    aln.aligned_seq1 = "AC-GT";
    aln.aligned_seq2 = "ACAGT";
    aln.matches = 4;
    aln.mismatches = 0;
    aln.gaps = 1;

    EXPECT_DOUBLE_EQ(aln.gapRatio(), 0.2);
}

TEST(AlignmentTest, CIGAR) {
    Alignment aln;
    aln.aligned_seq1 = "ACGT";
    aln.aligned_seq2 = "ACGT";

    auto cigar = aln.cigar();
    EXPECT_EQ(cigar, "4M");
}

TEST(AlignmentTest, CIGARWithMismatch) {
    Alignment aln;
    aln.aligned_seq1 = "ACGT";
    aln.aligned_seq2 = "AGGT";

    auto cigar = aln.cigar();
    // Should contain M and X
    EXPECT_FALSE(cigar.empty());
}

TEST(AlignmentTest, CIGARWithGaps) {
    Alignment aln;
    aln.aligned_seq1 = "AC-GT";
    aln.aligned_seq2 = "ACAGT";

    auto cigar = aln.cigar();
    // Should contain I for insertion
    EXPECT_NE(cigar.find('I'), std::string::npos);
}

// ============================================================================
// Edit Distance Tests
// ============================================================================

TEST(EditDistanceTest, IdenticalSequences) {
    Sequence seq1("ACGT");
    Sequence seq2("ACGT");

    EXPECT_EQ(editDistance(seq1, seq2), 0);
}

TEST(EditDistanceTest, SingleSubstitution) {
    Sequence seq1("ACGT");
    Sequence seq2("AGGT");

    EXPECT_EQ(editDistance(seq1, seq2), 1);
}

TEST(EditDistanceTest, SingleInsertion) {
    Sequence seq1("ACGT");
    Sequence seq2("ACGGT");

    EXPECT_EQ(editDistance(seq1, seq2), 1);
}

TEST(EditDistanceTest, SingleDeletion) {
    Sequence seq1("ACGT");
    Sequence seq2("ACT");

    EXPECT_EQ(editDistance(seq1, seq2), 1);
}

TEST(EditDistanceTest, CompletelyDifferent) {
    Sequence seq1("AAAA");
    Sequence seq2("TTTT");

    EXPECT_EQ(editDistance(seq1, seq2), 4);
}

// ============================================================================
// Hamming Distance Tests
// ============================================================================

TEST(HammingDistanceTest, IdenticalSequences) {
    Sequence seq1("ACGT");
    Sequence seq2("ACGT");

    EXPECT_EQ(hammingDistance(seq1, seq2), 0);
}

TEST(HammingDistanceTest, SingleDifference) {
    Sequence seq1("ACGT");
    Sequence seq2("AGGT");

    EXPECT_EQ(hammingDistance(seq1, seq2), 1);
}

TEST(HammingDistanceTest, MultipleDifferences) {
    Sequence seq1("ACGT");
    Sequence seq2("TGCA");

    EXPECT_EQ(hammingDistance(seq1, seq2), 4);
}

TEST(HammingDistanceTest, DifferentLengthsThrows) {
    Sequence seq1("ACGT");
    Sequence seq2("ACG");

    EXPECT_THROW(hammingDistance(seq1, seq2), AlignmentError);
}

// ============================================================================
// Semi-Global Alignment Tests
// ============================================================================

TEST(SemiGlobalAlignmentTest, ShortInLong) {
    Sequence seq1("ACGT");
    Sequence seq2("AAACGTAAA");

    auto result = semiGlobalAlignment(seq1, seq2);

    EXPECT_GT(result.score, 0);
}

// ============================================================================
// Alignment Matrix Tests
// ============================================================================

TEST(AlignmentMatrixTest, Construction) {
    AlignmentMatrix matrix(5, 10);
    EXPECT_EQ(matrix.rows(), 5);
    EXPECT_EQ(matrix.cols(), 10);
}

TEST(AlignmentMatrixTest, Access) {
    AlignmentMatrix matrix(3, 3);
    matrix.at(1, 2) = 5;
    EXPECT_EQ(matrix.at(1, 2), 5);
}

TEST(AlignmentMatrixTest, MaxScore) {
    AlignmentMatrix matrix(3, 3);
    matrix.at(0, 0) = 1;
    matrix.at(1, 1) = 10;
    matrix.at(2, 2) = 5;

    EXPECT_EQ(matrix.maxScore(), 10);
}

TEST(AlignmentMatrixTest, MaxPosition) {
    AlignmentMatrix matrix(3, 3);
    matrix.at(0, 0) = 1;
    matrix.at(1, 2) = 10;
    matrix.at(2, 2) = 5;

    auto [row, col] = matrix.maxPosition();
    EXPECT_EQ(row, 1);
    EXPECT_EQ(col, 2);
}

// ============================================================================
// Multiple Alignment Tests
// ============================================================================

TEST(MultipleAlignmentTest, EmptyInput) {
    std::vector<Sequence> sequences;
    auto result = multipleAlignment(sequences);
    EXPECT_TRUE(result.empty());
}

TEST(MultipleAlignmentTest, SingleSequence) {
    std::vector<Sequence> sequences;
    sequences.emplace_back("ACGT");

    auto result = multipleAlignment(sequences);
    ASSERT_EQ(result.size(), 1);
    EXPECT_EQ(result[0], "ACGT");
}

TEST(MultipleAlignmentTest, TwoSequences) {
    std::vector<Sequence> sequences;
    sequences.emplace_back("ACGT");
    sequences.emplace_back("ACGT");

    auto result = multipleAlignment(sequences);
    ASSERT_EQ(result.size(), 2);
    EXPECT_EQ(result[0].length(), result[1].length());
}

// ============================================================================
// Banded Alignment Tests
// ============================================================================

TEST(BandedAlignmentTest, SimilarSequences) {
    Sequence seq1("ACGTACGTACGT");
    Sequence seq2("ACGTACGTACGT");

    auto result = bandedSmithWaterman(seq1, seq2, 3);

    EXPECT_EQ(result.score, 24);  // 12 matches * 2
}

TEST(BandedAlignmentTest, FallsBackForDifferentLengths) {
    Sequence seq1("ACGT");
    Sequence seq2("ACGTACGTACGTACGT");

    // Should fall back to regular SW for very different lengths
    auto result = bandedSmithWaterman(seq1, seq2, 2);

    EXPECT_GT(result.score, 0);
}
