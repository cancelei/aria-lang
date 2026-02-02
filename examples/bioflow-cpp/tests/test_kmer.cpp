#include <gtest/gtest.h>
#include "bioflow/kmer.hpp"

using namespace bioflow;

// ============================================================================
// Constructor Tests
// ============================================================================

TEST(KMerCounterTest, ConstructorWithValidK) {
    EXPECT_NO_THROW(KMerCounter(1));
    EXPECT_NO_THROW(KMerCounter(21));
    EXPECT_NO_THROW(KMerCounter(100));
}

TEST(KMerCounterTest, ConstructorThrowsOnZeroK) {
    EXPECT_THROW(KMerCounter(0), KMerError);
}

TEST(KMerCounterTest, GetK) {
    KMerCounter counter(21);
    EXPECT_EQ(counter.k(), 21);
}

// ============================================================================
// Counting Tests
// ============================================================================

TEST(KMerCounterTest, CountSimpleSequence) {
    KMerCounter counter(2);
    Sequence seq("ATCG");
    counter.count(seq);

    EXPECT_EQ(counter.getCount("AT"), 1);
    EXPECT_EQ(counter.getCount("TC"), 1);
    EXPECT_EQ(counter.getCount("CG"), 1);
    EXPECT_EQ(counter.uniqueCount(), 3);
    EXPECT_EQ(counter.totalCount(), 3);
}

TEST(KMerCounterTest, CountRepeatedKmers) {
    KMerCounter counter(2);
    Sequence seq("ATATAT");
    counter.count(seq);

    EXPECT_EQ(counter.getCount("AT"), 3);
    EXPECT_EQ(counter.getCount("TA"), 2);
    EXPECT_EQ(counter.uniqueCount(), 2);
    EXPECT_EQ(counter.totalCount(), 5);
}

TEST(KMerCounterTest, CountSkipsAmbiguousBases) {
    KMerCounter counter(2);
    Sequence seq("ATNTA");
    counter.count(seq);

    // Should skip "TN" and "NT"
    EXPECT_EQ(counter.getCount("AT"), 1);
    EXPECT_EQ(counter.getCount("TA"), 1);
    EXPECT_EQ(counter.getCount("TN"), 0);
    EXPECT_EQ(counter.getCount("NT"), 0);
}

TEST(KMerCounterTest, CountSequenceShorterThanK) {
    KMerCounter counter(10);
    Sequence seq("ATCG");
    counter.count(seq);

    EXPECT_EQ(counter.uniqueCount(), 0);
    EXPECT_EQ(counter.totalCount(), 0);
}

TEST(KMerCounterTest, Contains) {
    KMerCounter counter(2);
    Sequence seq("ATCG");
    counter.count(seq);

    EXPECT_TRUE(counter.contains("AT"));
    EXPECT_TRUE(counter.contains("TC"));
    EXPECT_FALSE(counter.contains("GG"));
}

// ============================================================================
// Most/Least Frequent Tests
// ============================================================================

TEST(KMerCounterTest, MostFrequent) {
    KMerCounter counter(2);
    Sequence seq("ATATATATAT");
    counter.count(seq);

    auto top = counter.mostFrequent(2);
    ASSERT_EQ(top.size(), 2);

    // AT appears 5 times, TA appears 4 times
    EXPECT_EQ(top[0].kmer, "AT");
    EXPECT_EQ(top[0].count, 5);
    EXPECT_EQ(top[1].kmer, "TA");
    EXPECT_EQ(top[1].count, 4);
}

TEST(KMerCounterTest, MostFrequentMoreThanAvailable) {
    KMerCounter counter(2);
    Sequence seq("ATCG");
    counter.count(seq);

    auto top = counter.mostFrequent(10);
    EXPECT_EQ(top.size(), 3);  // Only 3 unique k-mers
}

TEST(KMerCounterTest, LeastFrequent) {
    KMerCounter counter(2);
    Sequence seq("ATATATAT");
    counter.count(seq);

    auto bottom = counter.leastFrequent(1);
    ASSERT_EQ(bottom.size(), 1);
    EXPECT_EQ(bottom[0].kmer, "TA");
    EXPECT_EQ(bottom[0].count, 3);  // TA appears 3 times, AT appears 4 times
}

// ============================================================================
// Threshold Tests
// ============================================================================

TEST(KMerCounterTest, AboveThreshold) {
    KMerCounter counter(2);
    Sequence seq("ATATATATAT");
    counter.count(seq);

    auto above = counter.aboveThreshold(5);
    ASSERT_EQ(above.size(), 1);
    EXPECT_EQ(above[0].kmer, "AT");
    EXPECT_EQ(above[0].count, 5);
}

// ============================================================================
// Spectrum Tests
// ============================================================================

TEST(KMerCounterTest, Spectrum) {
    KMerCounter counter(2);
    Sequence seq("ATCGATCGATCG");
    counter.count(seq);

    auto spectrum = counter.spectrum();
    EXPECT_EQ(spectrum.k, 2);
    EXPECT_GT(spectrum.unique_kmers, 0);
    EXPECT_GT(spectrum.total_kmers, 0);
}

// ============================================================================
// Clear and Merge Tests
// ============================================================================

TEST(KMerCounterTest, Clear) {
    KMerCounter counter(2);
    Sequence seq("ATCG");
    counter.count(seq);

    EXPECT_GT(counter.uniqueCount(), 0);

    counter.clear();
    EXPECT_EQ(counter.uniqueCount(), 0);
    EXPECT_EQ(counter.totalCount(), 0);
}

TEST(KMerCounterTest, Merge) {
    KMerCounter counter1(2);
    KMerCounter counter2(2);

    Sequence seq1("ATAT");
    Sequence seq2("GGGG");

    counter1.count(seq1);
    counter2.count(seq2);

    counter1.merge(counter2);

    EXPECT_GT(counter1.getCount("AT"), 0);
    EXPECT_GT(counter1.getCount("GG"), 0);
}

TEST(KMerCounterTest, MergeDifferentK) {
    KMerCounter counter1(2);
    KMerCounter counter2(3);

    EXPECT_THROW(counter1.merge(counter2), KMerError);
}

// ============================================================================
// Iterator Tests
// ============================================================================

TEST(KMerCounterTest, Iteration) {
    KMerCounter counter(2);
    Sequence seq("ATCG");
    counter.count(seq);

    size_t count = 0;
    for (const auto& [kmer, freq] : counter) {
        EXPECT_FALSE(kmer.empty());
        EXPECT_GT(freq, 0);
        count++;
    }

    EXPECT_EQ(count, counter.uniqueCount());
}

// ============================================================================
// Canonical K-mer Tests
// ============================================================================

TEST(CanonicalKMerTest, CanonicalKmer) {
    // AT and AT (complement: TA, RC: AT) -> canonical is AT
    EXPECT_EQ(canonicalKmer("AT"), "AT");

    // GC and GC (complement: CG, RC: GC) -> canonical is GC
    EXPECT_EQ(canonicalKmer("GC"), "GC");

    // TA and TA (RC: TA) -> canonical is TA
    EXPECT_EQ(canonicalKmer("TA"), "TA");

    // ACGT and ACGT (RC: ACGT) -> palindromic, canonical is ACGT
    EXPECT_EQ(canonicalKmer("ACGT"), "ACGT");
}

TEST(CanonicalKMerCounterTest, CountCanonical) {
    CanonicalKMerCounter counter(2);
    Sequence seq("ATCG");  // Contains AT, TC, CG
    counter.count(seq);

    // AT and its RC (AT) are the same canonical
    // TC's RC is GA, so TC and GA would be grouped
    // CG's RC is CG (palindrome)

    EXPECT_GT(counter.uniqueCount(), 0);
    EXPECT_GT(counter.totalCount(), 0);
}

// ============================================================================
// K-mer Entry Tests
// ============================================================================

TEST(KMerEntryTest, Frequency) {
    KMerEntry entry{"ATG", 5};
    EXPECT_DOUBLE_EQ(entry.frequency(10), 0.5);
    EXPECT_DOUBLE_EQ(entry.frequency(0), 0.0);
}

TEST(KMerEntryTest, Comparison) {
    KMerEntry entry1{"ATG", 5};
    KMerEntry entry2{"GTA", 10};

    EXPECT_TRUE(entry1 < entry2);
    EXPECT_TRUE(entry2 > entry1);
}
