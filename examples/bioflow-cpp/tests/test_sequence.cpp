#include <gtest/gtest.h>
#include "bioflow/sequence.hpp"

using namespace bioflow;

// ============================================================================
// Constructor Tests
// ============================================================================

TEST(SequenceTest, ConstructorWithValidBases) {
    EXPECT_NO_THROW(Sequence("ATCG"));
    EXPECT_NO_THROW(Sequence("atcg"));
    EXPECT_NO_THROW(Sequence("AtCgN"));
}

TEST(SequenceTest, ConstructorWithId) {
    Sequence seq("ATCG", "test_id");
    ASSERT_TRUE(seq.id().has_value());
    EXPECT_EQ(*seq.id(), "test_id");
}

TEST(SequenceTest, ConstructorConvertsToUppercase) {
    Sequence seq("atcg");
    EXPECT_EQ(seq.bases(), "ATCG");
}

TEST(SequenceTest, ConstructorThrowsOnEmptySequence) {
    EXPECT_THROW(Sequence(""), SequenceError);
}

TEST(SequenceTest, ConstructorThrowsOnInvalidBase) {
    EXPECT_THROW(Sequence("ATXCG"), SequenceError);
    EXPECT_THROW(Sequence("ATCGZ"), SequenceError);
    EXPECT_THROW(Sequence("123"), SequenceError);
}

// ============================================================================
// Accessor Tests
// ============================================================================

TEST(SequenceTest, Length) {
    Sequence seq("ATCGATCG");
    EXPECT_EQ(seq.length(), 8);
}

TEST(SequenceTest, Bases) {
    Sequence seq("ATCG");
    EXPECT_EQ(seq.bases(), "ATCG");
}

TEST(SequenceTest, Empty) {
    Sequence seq("A");
    EXPECT_FALSE(seq.empty());
}

TEST(SequenceTest, ElementAccess) {
    Sequence seq("ATCG");
    EXPECT_EQ(seq[0], 'A');
    EXPECT_EQ(seq[1], 'T');
    EXPECT_EQ(seq.at(2), 'C');
    EXPECT_EQ(seq.at(3), 'G');
}

TEST(SequenceTest, AtThrowsOnOutOfRange) {
    Sequence seq("ATCG");
    EXPECT_THROW(seq.at(4), std::out_of_range);
}

// ============================================================================
// Validation Tests
// ============================================================================

TEST(SequenceTest, IsValidBase) {
    EXPECT_TRUE(Sequence::isValidBase('A'));
    EXPECT_TRUE(Sequence::isValidBase('a'));
    EXPECT_TRUE(Sequence::isValidBase('T'));
    EXPECT_TRUE(Sequence::isValidBase('C'));
    EXPECT_TRUE(Sequence::isValidBase('G'));
    EXPECT_TRUE(Sequence::isValidBase('N'));
    EXPECT_FALSE(Sequence::isValidBase('X'));
    EXPECT_FALSE(Sequence::isValidBase('1'));
}

TEST(SequenceTest, IsValid) {
    Sequence seq("ATCGN");
    EXPECT_TRUE(seq.isValid());
}

TEST(SequenceTest, HasAmbiguousBases) {
    Sequence seq1("ATCG");
    Sequence seq2("ATCGN");
    EXPECT_FALSE(seq1.hasAmbiguousBases());
    EXPECT_TRUE(seq2.hasAmbiguousBases());
}

// ============================================================================
// Content Analysis Tests
// ============================================================================

TEST(SequenceTest, GCContent) {
    Sequence seq1("GCGC");
    EXPECT_DOUBLE_EQ(seq1.gcContent(), 1.0);

    Sequence seq2("ATAT");
    EXPECT_DOUBLE_EQ(seq2.gcContent(), 0.0);

    Sequence seq3("ATGC");
    EXPECT_DOUBLE_EQ(seq3.gcContent(), 0.5);

    Sequence seq4("ATGCATGC");
    EXPECT_DOUBLE_EQ(seq4.gcContent(), 0.5);
}

TEST(SequenceTest, ATContent) {
    Sequence seq1("ATAT");
    EXPECT_DOUBLE_EQ(seq1.atContent(), 1.0);

    Sequence seq2("GCGC");
    EXPECT_DOUBLE_EQ(seq2.atContent(), 0.0);

    Sequence seq3("ATGC");
    EXPECT_DOUBLE_EQ(seq3.atContent(), 0.5);
}

TEST(SequenceTest, CountBase) {
    Sequence seq("AAATTTCCCGGG");
    EXPECT_EQ(seq.countBase('A'), 3);
    EXPECT_EQ(seq.countBase('T'), 3);
    EXPECT_EQ(seq.countBase('C'), 3);
    EXPECT_EQ(seq.countBase('G'), 3);
    EXPECT_EQ(seq.countBase('N'), 0);
}

TEST(SequenceTest, BaseComposition) {
    Sequence seq("AATTCCGGN");
    auto comp = seq.baseComposition();
    EXPECT_EQ(comp[0], 2);  // A
    EXPECT_EQ(comp[1], 2);  // C
    EXPECT_EQ(comp[2], 2);  // G
    EXPECT_EQ(comp[3], 2);  // T
    EXPECT_EQ(comp[4], 1);  // N
}

// ============================================================================
// Transformation Tests
// ============================================================================

TEST(SequenceTest, Complement) {
    Sequence seq("ATCG");
    auto comp = seq.complement();
    EXPECT_EQ(comp.bases(), "TAGC");
}

TEST(SequenceTest, ComplementWithN) {
    Sequence seq("ATNCG");
    auto comp = seq.complement();
    EXPECT_EQ(comp.bases(), "TANGC");
}

TEST(SequenceTest, Reverse) {
    Sequence seq("ATCG");
    auto rev = seq.reverse();
    EXPECT_EQ(rev.bases(), "GCTA");
}

TEST(SequenceTest, ReverseComplement) {
    Sequence seq("ATCG");
    auto rc = seq.reverseComplement();
    EXPECT_EQ(rc.bases(), "CGAT");
}

TEST(SequenceTest, ReverseComplementPreservesId) {
    Sequence seq("ATCG", "test_id");
    auto rc = seq.reverseComplement();
    ASSERT_TRUE(rc.id().has_value());
    EXPECT_EQ(*rc.id(), "test_id");
}

TEST(SequenceTest, Subsequence) {
    Sequence seq("ATCGATCG");
    auto sub = seq.subsequence(2, 4);
    EXPECT_EQ(sub.bases(), "CGAT");
}

TEST(SequenceTest, SubsequenceAtEnd) {
    Sequence seq("ATCGATCG");
    auto sub = seq.subsequence(6, 10);  // Longer than remaining
    EXPECT_EQ(sub.bases(), "CG");
}

TEST(SequenceTest, SubsequenceOutOfRange) {
    Sequence seq("ATCG");
    EXPECT_THROW(seq.subsequence(10, 2), SequenceError);
}

// ============================================================================
// Motif Finding Tests
// ============================================================================

TEST(SequenceTest, ContainsMotif) {
    Sequence seq("ATCGATCGATCG");
    EXPECT_TRUE(seq.containsMotif("GATC"));
    EXPECT_TRUE(seq.containsMotif("ATC"));
    EXPECT_FALSE(seq.containsMotif("GGGG"));
}

TEST(SequenceTest, FindMotifPositions) {
    Sequence seq("ATCGATCGATCG");
    auto positions = seq.findMotifPositions("ATC");

    ASSERT_EQ(positions.size(), 3);
    EXPECT_EQ(positions[0], 0);
    EXPECT_EQ(positions[1], 4);
    EXPECT_EQ(positions[2], 8);
}

TEST(SequenceTest, FindOverlappingMotifs) {
    Sequence seq("AAAA");
    auto positions = seq.findMotifPositions("AA");

    ASSERT_EQ(positions.size(), 3);
    EXPECT_EQ(positions[0], 0);
    EXPECT_EQ(positions[1], 1);
    EXPECT_EQ(positions[2], 2);
}

TEST(SequenceTest, CountMotif) {
    Sequence seq("ATCGATCGATCG");
    EXPECT_EQ(seq.countMotif("ATC"), 3);
    EXPECT_EQ(seq.countMotif("GATC"), 2);
    EXPECT_EQ(seq.countMotif("XYZ"), 0);
}

// ============================================================================
// Operator Tests
// ============================================================================

TEST(SequenceTest, Equality) {
    Sequence seq1("ATCG");
    Sequence seq2("ATCG");
    Sequence seq3("GCTA");

    EXPECT_TRUE(seq1 == seq2);
    EXPECT_FALSE(seq1 == seq3);
}

TEST(SequenceTest, Concatenation) {
    Sequence seq1("ATCG");
    Sequence seq2("GCTA");
    auto concat = seq1 + seq2;
    EXPECT_EQ(concat.bases(), "ATCGGCTA");
}

// ============================================================================
// Iterator Tests
// ============================================================================

TEST(SequenceTest, RangeBasedFor) {
    Sequence seq("ATCG");
    std::string result;
    for (char c : seq) {
        result += c;
    }
    EXPECT_EQ(result, "ATCG");
}

// ============================================================================
// Factory Function Tests
// ============================================================================

TEST(SequenceTest, MakeSequence) {
    auto seq = makeSequence("ATCG");
    EXPECT_EQ(seq.bases(), "ATCG");
}
