//! Extended tests for the K-mer module
//!
//! These tests cover comprehensive k-mer counting scenarios including
//! edge cases, canonical k-mers, and statistical analysis.

const std = @import("std");
const testing = std.testing;
const Sequence = @import("sequence").Sequence;
const KMerCounter = @import("kmer").KMerCounter;
const KMerError = @import("kmer").KMerError;
const KMerSpectrum = @import("kmer").KMerSpectrum;
const jaccardSimilarity = @import("kmer").jaccardSimilarity;

// ============================================================================
// Initialization Tests
// ============================================================================

test "KMerCounter init valid k" {
    const allocator = testing.allocator;

    const k_values = [_]usize{ 1, 2, 3, 5, 10, 21, 31 };
    for (k_values) |k| {
        var counter = try KMerCounter.init(allocator, k);
        defer counter.deinit();
        try testing.expectEqual(k, counter.k);
    }
}

test "KMerCounter init k=0 fails" {
    const allocator = testing.allocator;
    const result = KMerCounter.init(allocator, 0);
    try testing.expectError(KMerError.InvalidK, result);
}

// ============================================================================
// Basic Counting Tests
// ============================================================================

test "count simple sequence k=1" {
    const allocator = testing.allocator;

    var counter = try KMerCounter.init(allocator, 1);
    defer counter.deinit();

    var seq = try Sequence.init(allocator, "AACCCGGGGTTTTT");
    defer seq.deinit();

    try counter.count(seq);

    try testing.expectEqual(@as(usize, 2), counter.getCount("A"));
    try testing.expectEqual(@as(usize, 3), counter.getCount("C"));
    try testing.expectEqual(@as(usize, 4), counter.getCount("G"));
    try testing.expectEqual(@as(usize, 5), counter.getCount("T"));
}

test "count simple sequence k=2" {
    const allocator = testing.allocator;

    var counter = try KMerCounter.init(allocator, 2);
    defer counter.deinit();

    var seq = try Sequence.init(allocator, "ATAT");
    defer seq.deinit();

    try counter.count(seq);

    try testing.expectEqual(@as(usize, 2), counter.getCount("AT"));
    try testing.expectEqual(@as(usize, 1), counter.getCount("TA"));
    try testing.expectEqual(@as(usize, 0), counter.getCount("AA"));
}

test "count simple sequence k=3" {
    const allocator = testing.allocator;

    var counter = try KMerCounter.init(allocator, 3);
    defer counter.deinit();

    var seq = try Sequence.init(allocator, "ATGATGATG");
    defer seq.deinit();

    try counter.count(seq);

    try testing.expectEqual(@as(usize, 3), counter.getCount("ATG"));
    try testing.expectEqual(@as(usize, 2), counter.getCount("TGA"));
    try testing.expectEqual(@as(usize, 2), counter.getCount("GAT"));
    try testing.expectEqual(@as(usize, 0), counter.getCount("GGG"));
}

test "count with k larger than sequence" {
    const allocator = testing.allocator;

    var counter = try KMerCounter.init(allocator, 10);
    defer counter.deinit();

    var seq = try Sequence.init(allocator, "ATGC");
    defer seq.deinit();

    try counter.count(seq);

    try testing.expectEqual(@as(usize, 0), counter.uniqueCount());
    try testing.expectEqual(@as(usize, 0), counter.total_count);
}

test "count skips N bases" {
    const allocator = testing.allocator;

    var counter = try KMerCounter.init(allocator, 3);
    defer counter.deinit();

    var seq = try Sequence.init(allocator, "ATGNATGCATG");
    defer seq.deinit();

    try counter.count(seq);

    // ATG appears at 0, 4, 8 but 4 would include N in some adjacent k-mers
    // Let's trace: ATG(0), TGN(1-skip), GNA(2-skip), NAT(3-skip), ATG(4), TGC(5), GCA(6), CAT(7), ATG(8)
    try testing.expectEqual(@as(usize, 3), counter.getCount("ATG"));
    try testing.expectEqual(@as(usize, 0), counter.getCount("TGN"));
    try testing.expectEqual(@as(usize, 0), counter.getCount("NAT"));
}

// ============================================================================
// Multiple Sequences Tests
// ============================================================================

test "count multiple sequences" {
    const allocator = testing.allocator;

    var counter = try KMerCounter.init(allocator, 2);
    defer counter.deinit();

    var seq1 = try Sequence.init(allocator, "ATAT");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "ATGC");
    defer seq2.deinit();

    try counter.count(seq1);
    try counter.count(seq2);

    try testing.expectEqual(@as(usize, 3), counter.getCount("AT")); // 2 from seq1 + 1 from seq2
}

// ============================================================================
// Canonical K-mer Tests
// ============================================================================

test "canonical k-mers basic" {
    const allocator = testing.allocator;

    var counter = try KMerCounter.initCanonical(allocator, 3);
    defer counter.deinit();

    // ATG and its reverse complement CAT should be counted together
    var seq = try Sequence.init(allocator, "ATGCAT");
    defer seq.deinit();

    try counter.count(seq);

    // ATG and CAT are reverse complements, so they should be counted as one canonical k-mer
    const unique = counter.uniqueCount();
    try testing.expect(unique < 4); // Fewer unique due to canonicalization
}

test "canonical vs non-canonical comparison" {
    const allocator = testing.allocator;

    // Same sequence, different modes
    const bases = "ATGATGATG";

    var non_canonical = try KMerCounter.init(allocator, 3);
    defer non_canonical.deinit();

    var canonical = try KMerCounter.initCanonical(allocator, 3);
    defer canonical.deinit();

    var seq = try Sequence.init(allocator, bases);
    defer seq.deinit();

    try non_canonical.count(seq);
    try canonical.count(seq);

    // Canonical should have fewer or equal unique k-mers
    try testing.expect(canonical.uniqueCount() <= non_canonical.uniqueCount());
}

// ============================================================================
// Statistical Tests
// ============================================================================

test "diversity calculation" {
    const allocator = testing.allocator;

    // Sequence with repeated k-mers has low diversity
    var counter1 = try KMerCounter.init(allocator, 2);
    defer counter1.deinit();

    var seq1 = try Sequence.init(allocator, "ATATAT");
    defer seq1.deinit();

    try counter1.count(seq1);

    const div1 = counter1.diversity();
    // Only 2 unique 2-mers (AT, TA) out of 5 total
    try testing.expectApproxEqAbs(@as(f64, 2.0 / 5.0), div1, 0.0001);

    // Sequence with all unique k-mers has high diversity
    var counter2 = try KMerCounter.init(allocator, 2);
    defer counter2.deinit();

    var seq2 = try Sequence.init(allocator, "ACGT");
    defer seq2.deinit();

    try counter2.count(seq2);

    const div2 = counter2.diversity();
    // 3 unique 2-mers (AC, CG, GT) out of 3 total = 1.0
    try testing.expectApproxEqAbs(@as(f64, 1.0), div2, 0.0001);
}

test "entropy calculation" {
    const allocator = testing.allocator;

    var counter = try KMerCounter.init(allocator, 1);
    defer counter.deinit();

    // Uniform distribution has maximum entropy
    var seq = try Sequence.init(allocator, "ACGT");
    defer seq.deinit();

    try counter.count(seq);

    const ent = counter.entropy();
    // 4 equally frequent 1-mers: entropy = log2(4) = 2.0
    try testing.expectApproxEqAbs(@as(f64, 2.0), ent, 0.0001);
}

test "entropy with non-uniform distribution" {
    const allocator = testing.allocator;

    var counter = try KMerCounter.init(allocator, 1);
    defer counter.deinit();

    // Non-uniform: more A's
    var seq = try Sequence.init(allocator, "AAACGT");
    defer seq.deinit();

    try counter.count(seq);

    const ent = counter.entropy();
    // Should be less than 2.0 (non-uniform)
    try testing.expect(ent < 2.0);
    try testing.expect(ent > 0.0);
}

// ============================================================================
// Ranking Tests
// ============================================================================

test "most frequent k-mers" {
    const allocator = testing.allocator;

    var counter = try KMerCounter.init(allocator, 2);
    defer counter.deinit();

    var seq = try Sequence.init(allocator, "ATATATATAT");
    defer seq.deinit();

    try counter.count(seq);

    const top = try counter.mostFrequent(allocator, 3);
    defer allocator.free(top);

    // AT should be most frequent
    try testing.expect(top.len >= 2);
    try testing.expect(top[0].count >= top[1].count);
}

test "least frequent k-mers" {
    const allocator = testing.allocator;

    var counter = try KMerCounter.init(allocator, 2);
    defer counter.deinit();

    var seq = try Sequence.init(allocator, "ACGTACGTAC");
    defer seq.deinit();

    try counter.count(seq);

    const bottom = try counter.leastFrequent(allocator, 2);
    defer allocator.free(bottom);

    try testing.expect(bottom.len >= 1);
    // Least frequent should have smallest count
    if (bottom.len >= 2) {
        try testing.expect(bottom[0].count <= bottom[1].count);
    }
}

test "k-mers with specific count" {
    const allocator = testing.allocator;

    var counter = try KMerCounter.init(allocator, 3);
    defer counter.deinit();

    var seq = try Sequence.init(allocator, "ATGATGATG");
    defer seq.deinit();

    try counter.count(seq);

    // Get k-mers that appear exactly twice
    const twice = try counter.withCount(allocator, 2);
    defer allocator.free(twice);

    // TGA and GAT appear twice
    try testing.expectEqual(@as(usize, 2), twice.len);
}

// ============================================================================
// Spectrum Tests
// ============================================================================

test "k-mer spectrum basic" {
    const allocator = testing.allocator;

    var counter = try KMerCounter.init(allocator, 3);
    defer counter.deinit();

    var seq = try Sequence.init(allocator, "ATGATGATG");
    defer seq.deinit();

    try counter.count(seq);

    var spectrum = try KMerSpectrum.fromCounter(allocator, &counter);
    defer spectrum.deinit();

    // ATG appears 3 times, TGA and GAT appear 2 times each
    try testing.expectEqual(@as(usize, 1), spectrum.get(3)); // 1 k-mer with count 3
    try testing.expectEqual(@as(usize, 2), spectrum.get(2)); // 2 k-mers with count 2
}

test "k-mer spectrum peak" {
    const allocator = testing.allocator;

    var counter = try KMerCounter.init(allocator, 2);
    defer counter.deinit();

    // Create sequence where most k-mers appear twice
    var seq = try Sequence.init(allocator, "ACGTACGT");
    defer seq.deinit();

    try counter.count(seq);

    var spectrum = try KMerSpectrum.fromCounter(allocator, &counter);
    defer spectrum.deinit();

    const peak_count = spectrum.peak();
    try testing.expect(peak_count != null);
}

// ============================================================================
// Similarity Tests
// ============================================================================

test "Jaccard similarity identical sequences" {
    const allocator = testing.allocator;

    var counter1 = try KMerCounter.init(allocator, 3);
    defer counter1.deinit();

    var counter2 = try KMerCounter.init(allocator, 3);
    defer counter2.deinit();

    var seq = try Sequence.init(allocator, "ATGCGATCGA");
    defer seq.deinit();

    try counter1.count(seq);
    try counter2.count(seq);

    const similarity = jaccardSimilarity(&counter1, &counter2);
    try testing.expectApproxEqAbs(@as(f64, 1.0), similarity, 0.0001);
}

test "Jaccard similarity completely different" {
    const allocator = testing.allocator;

    var counter1 = try KMerCounter.init(allocator, 3);
    defer counter1.deinit();

    var counter2 = try KMerCounter.init(allocator, 3);
    defer counter2.deinit();

    var seq1 = try Sequence.init(allocator, "AAAAAAA");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "TTTTTTT");
    defer seq2.deinit();

    try counter1.count(seq1);
    try counter2.count(seq2);

    const similarity = jaccardSimilarity(&counter1, &counter2);
    try testing.expectApproxEqAbs(@as(f64, 0.0), similarity, 0.0001);
}

test "Jaccard similarity partial overlap" {
    const allocator = testing.allocator;

    var counter1 = try KMerCounter.init(allocator, 3);
    defer counter1.deinit();

    var counter2 = try KMerCounter.init(allocator, 3);
    defer counter2.deinit();

    var seq1 = try Sequence.init(allocator, "ATGATG");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "ATGCCC");
    defer seq2.deinit();

    try counter1.count(seq1);
    try counter2.count(seq2);

    const similarity = jaccardSimilarity(&counter1, &counter2);
    // ATG is common, others are different
    try testing.expect(similarity > 0.0);
    try testing.expect(similarity < 1.0);
}

test "Jaccard similarity different k" {
    const allocator = testing.allocator;

    var counter1 = try KMerCounter.init(allocator, 3);
    defer counter1.deinit();

    var counter2 = try KMerCounter.init(allocator, 5);
    defer counter2.deinit();

    var seq = try Sequence.init(allocator, "ATGCGATCGA");
    defer seq.deinit();

    try counter1.count(seq);
    try counter2.count(seq);

    // Different k values should return 0
    const similarity = jaccardSimilarity(&counter1, &counter2);
    try testing.expectApproxEqAbs(@as(f64, 0.0), similarity, 0.0001);
}

// ============================================================================
// Merge Tests
// ============================================================================

test "merge two counters" {
    const allocator = testing.allocator;

    var counter1 = try KMerCounter.init(allocator, 2);
    defer counter1.deinit();

    var counter2 = try KMerCounter.init(allocator, 2);
    defer counter2.deinit();

    var seq1 = try Sequence.init(allocator, "ATAT");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "ATGC");
    defer seq2.deinit();

    try counter1.count(seq1);
    try counter2.count(seq2);

    try counter1.merge(&counter2);

    // AT should appear 2 (from seq1) + 1 (from seq2) = 3 times
    try testing.expectEqual(@as(usize, 3), counter1.getCount("AT"));
}

// ============================================================================
// Clear Tests
// ============================================================================

test "clear counter" {
    const allocator = testing.allocator;

    var counter = try KMerCounter.init(allocator, 2);
    defer counter.deinit();

    var seq = try Sequence.init(allocator, "ATGC");
    defer seq.deinit();

    try counter.count(seq);
    try testing.expect(counter.uniqueCount() > 0);

    counter.clear();
    try testing.expectEqual(@as(usize, 0), counter.uniqueCount());
    try testing.expectEqual(@as(usize, 0), counter.total_count);
}

// ============================================================================
// Iterator Tests
// ============================================================================

test "iterator over k-mers" {
    const allocator = testing.allocator;

    var counter = try KMerCounter.init(allocator, 2);
    defer counter.deinit();

    var seq = try Sequence.init(allocator, "ACGT");
    defer seq.deinit();

    try counter.count(seq);

    var count: usize = 0;
    var it = counter.iterator();
    while (it.next()) |_| {
        count += 1;
    }

    try testing.expectEqual(counter.uniqueCount(), count);
}

// ============================================================================
// Frequency Tests
// ============================================================================

test "k-mer frequency calculation" {
    const allocator = testing.allocator;

    var counter = try KMerCounter.init(allocator, 2);
    defer counter.deinit();

    var seq = try Sequence.init(allocator, "ATATAT");
    defer seq.deinit();

    try counter.count(seq);

    const top = try counter.mostFrequent(allocator, 1);
    defer allocator.free(top);

    // AT appears 3 times out of 5 total
    const freq = top[0].frequency(counter.total_count);
    try testing.expectApproxEqAbs(@as(f64, 3.0 / 5.0), freq, 0.0001);
}

// ============================================================================
// Edge Cases
// ============================================================================

test "single character sequence" {
    const allocator = testing.allocator;

    var counter = try KMerCounter.init(allocator, 1);
    defer counter.deinit();

    var seq = try Sequence.init(allocator, "A");
    defer seq.deinit();

    try counter.count(seq);

    try testing.expectEqual(@as(usize, 1), counter.getCount("A"));
    try testing.expectEqual(@as(usize, 1), counter.uniqueCount());
}

test "all same base" {
    const allocator = testing.allocator;

    var counter = try KMerCounter.init(allocator, 3);
    defer counter.deinit();

    var seq = try Sequence.init(allocator, "AAAAAAAAAA");
    defer seq.deinit();

    try counter.count(seq);

    try testing.expectEqual(@as(usize, 1), counter.uniqueCount());
    try testing.expectEqual(@as(usize, 8), counter.getCount("AAA"));
}

test "countBases with raw string" {
    const allocator = testing.allocator;

    var counter = try KMerCounter.init(allocator, 2);
    defer counter.deinit();

    try counter.countBases("atgc");

    try testing.expectEqual(@as(usize, 1), counter.getCount("AT"));
    try testing.expectEqual(@as(usize, 1), counter.getCount("TG"));
    try testing.expectEqual(@as(usize, 1), counter.getCount("GC"));
}
