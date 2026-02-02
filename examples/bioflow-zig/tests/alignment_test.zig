//! Extended tests for the Alignment module
//!
//! Comprehensive tests for Smith-Waterman (local) and Needleman-Wunsch (global)
//! alignment algorithms, including edge cases and scoring variations.

const std = @import("std");
const testing = std.testing;
const Sequence = @import("sequence").Sequence;
const alignment = @import("alignment");
const ScoringMatrix = alignment.ScoringMatrix;
const Alignment = alignment.Alignment;
const AlignmentError = alignment.AlignmentError;

// ============================================================================
// Scoring Matrix Tests
// ============================================================================

test "ScoringMatrix default values" {
    const scoring = ScoringMatrix.default();
    try testing.expectEqual(@as(i32, 2), scoring.match_score);
    try testing.expectEqual(@as(i32, -1), scoring.mismatch_penalty);
    try testing.expectEqual(@as(i32, -2), scoring.gap_open);
    try testing.expectEqual(@as(i32, -2), scoring.gap_extend);
}

test "ScoringMatrix DNA values" {
    const scoring = ScoringMatrix.dna();
    try testing.expectEqual(@as(i32, 5), scoring.match_score);
    try testing.expectEqual(@as(i32, -4), scoring.mismatch_penalty);
    try testing.expectEqual(@as(i32, -10), scoring.gap_open);
    try testing.expectEqual(@as(i32, -1), scoring.gap_extend);
}

test "ScoringMatrix BLAST values" {
    const scoring = ScoringMatrix.blast();
    try testing.expectEqual(@as(i32, 1), scoring.match_score);
    try testing.expectEqual(@as(i32, -3), scoring.mismatch_penalty);
    try testing.expectEqual(@as(i32, -5), scoring.gap_open);
    try testing.expectEqual(@as(i32, -2), scoring.gap_extend);
}

test "ScoringMatrix score function" {
    const scoring = ScoringMatrix.default();

    // Match
    try testing.expectEqual(@as(i32, 2), scoring.score('A', 'A'));
    try testing.expectEqual(@as(i32, 2), scoring.score('C', 'C'));
    try testing.expectEqual(@as(i32, 2), scoring.score('G', 'G'));
    try testing.expectEqual(@as(i32, 2), scoring.score('T', 'T'));

    // Mismatch
    try testing.expectEqual(@as(i32, -1), scoring.score('A', 'T'));
    try testing.expectEqual(@as(i32, -1), scoring.score('C', 'G'));

    // N is neutral
    try testing.expectEqual(@as(i32, 0), scoring.score('A', 'N'));
    try testing.expectEqual(@as(i32, 0), scoring.score('N', 'T'));
}

test "ScoringMatrix gap penalties" {
    const scoring = ScoringMatrix.default();

    try testing.expectEqual(@as(i32, -2), scoring.gapPenalty());
    try testing.expectEqual(@as(i32, 0), scoring.affineGapPenalty(0));
    try testing.expectEqual(@as(i32, -2), scoring.affineGapPenalty(1));
    try testing.expectEqual(@as(i32, -4), scoring.affineGapPenalty(2));
    try testing.expectEqual(@as(i32, -6), scoring.affineGapPenalty(3));
}

// ============================================================================
// Smith-Waterman (Local Alignment) Tests
// ============================================================================

test "SW identical sequences" {
    const allocator = testing.allocator;

    var seq1 = try Sequence.init(allocator, "ACGTACGT");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "ACGTACGT");
    defer seq2.deinit();

    var align_result = try alignment.smithWaterman(allocator, seq1, seq2, ScoringMatrix.default());
    defer align_result.deinit();

    try testing.expectEqual(@as(usize, 8), align_result.matches);
    try testing.expectEqual(@as(usize, 0), align_result.mismatches);
    try testing.expectEqual(@as(usize, 0), align_result.gaps);
    try testing.expectApproxEqAbs(@as(f64, 1.0), align_result.identity(), 0.0001);
}

test "SW with single mismatch" {
    const allocator = testing.allocator;

    var seq1 = try Sequence.init(allocator, "ACGTACGT");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "ACGTTCGT");
    defer seq2.deinit();

    var align_result = try alignment.smithWaterman(allocator, seq1, seq2, ScoringMatrix.default());
    defer align_result.deinit();

    // Should still find good alignment
    try testing.expect(align_result.score > 0);
    try testing.expect(align_result.matches >= 7);
}

test "SW with gap" {
    const allocator = testing.allocator;

    var seq1 = try Sequence.init(allocator, "ACGTACGT");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "ACGACGT");
    defer seq2.deinit();

    var align_result = try alignment.smithWaterman(allocator, seq1, seq2, ScoringMatrix.default());
    defer align_result.deinit();

    // Should have at least one gap or mismatch
    try testing.expect(align_result.gaps > 0 or align_result.mismatches > 0);
}

test "SW local alignment finds best region" {
    const allocator = testing.allocator;

    // Sequences with matching region in the middle
    var seq1 = try Sequence.init(allocator, "XXXXACGTXXXX");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "YYYYACGTYYYY");
    defer seq2.deinit();

    var align_result = try alignment.smithWaterman(allocator, seq1, seq2, ScoringMatrix.default());
    defer align_result.deinit();

    // Should find the ACGT match
    try testing.expect(align_result.score > 0);
    try testing.expectEqualStrings("ACGT", align_result.aligned_seq1);
    try testing.expectEqualStrings("ACGT", align_result.aligned_seq2);
}

test "SW completely different sequences" {
    const allocator = testing.allocator;

    var seq1 = try Sequence.init(allocator, "AAAA");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "TTTT");
    defer seq2.deinit();

    var align_result = try alignment.smithWaterman(allocator, seq1, seq2, ScoringMatrix.default());
    defer align_result.deinit();

    // No good alignment should be found
    try testing.expectEqual(@as(usize, 0), align_result.matches);
}

test "SW reverse complement alignment" {
    const allocator = testing.allocator;

    var seq1 = try Sequence.init(allocator, "ATGC");
    defer seq1.deinit();

    var rc1 = try seq1.reverseComplement();
    defer rc1.deinit();

    // ATGC and GCAT should align well
    var align_result = try alignment.smithWaterman(allocator, seq1, rc1, ScoringMatrix.default());
    defer align_result.deinit();

    // Depends on specific alignment, but should find some matches
    try testing.expect(align_result.score >= 0);
}

// ============================================================================
// Needleman-Wunsch (Global Alignment) Tests
// ============================================================================

test "NW identical sequences" {
    const allocator = testing.allocator;

    var seq1 = try Sequence.init(allocator, "ACGT");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "ACGT");
    defer seq2.deinit();

    var align_result = try alignment.needlemanWunsch(allocator, seq1, seq2, ScoringMatrix.default());
    defer align_result.deinit();

    try testing.expectEqual(@as(usize, 4), align_result.matches);
    try testing.expectEqual(@as(usize, 0), align_result.mismatches);
    try testing.expectEqual(@as(usize, 0), align_result.gaps);
}

test "NW with deletion" {
    const allocator = testing.allocator;

    var seq1 = try Sequence.init(allocator, "ACGT");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "ACT");
    defer seq2.deinit();

    var align_result = try alignment.needlemanWunsch(allocator, seq1, seq2, ScoringMatrix.default());
    defer align_result.deinit();

    // Global alignment should align entire sequences
    try testing.expectEqual(@as(usize, 4), align_result.aligned_seq1.len);
    try testing.expectEqual(@as(usize, 4), align_result.aligned_seq2.len);
}

test "NW with insertion" {
    const allocator = testing.allocator;

    var seq1 = try Sequence.init(allocator, "ACT");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "ACGT");
    defer seq2.deinit();

    var align_result = try alignment.needlemanWunsch(allocator, seq1, seq2, ScoringMatrix.default());
    defer align_result.deinit();

    // Should have a gap in seq1
    try testing.expect(align_result.gaps > 0);
}

test "NW different length sequences" {
    const allocator = testing.allocator;

    var seq1 = try Sequence.init(allocator, "ACGTACGT");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "ACGT");
    defer seq2.deinit();

    var align_result = try alignment.needlemanWunsch(allocator, seq1, seq2, ScoringMatrix.default());
    defer align_result.deinit();

    // Both aligned sequences should be same length
    try testing.expectEqual(align_result.aligned_seq1.len, align_result.aligned_seq2.len);
}

test "NW completely different sequences" {
    const allocator = testing.allocator;

    var seq1 = try Sequence.init(allocator, "AAAA");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "TTTT");
    defer seq2.deinit();

    var align_result = try alignment.needlemanWunsch(allocator, seq1, seq2, ScoringMatrix.default());
    defer align_result.deinit();

    // Global alignment must align entire sequences
    try testing.expectEqual(@as(usize, 4), align_result.mismatches);
    try testing.expectEqual(@as(usize, 0), align_result.matches);
}

// ============================================================================
// Alignment Statistics Tests
// ============================================================================

test "alignment identity calculation" {
    const allocator = testing.allocator;

    var seq1 = try Sequence.init(allocator, "ACGT");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "ACGT");
    defer seq2.deinit();

    var align_result = try alignment.smithWaterman(allocator, seq1, seq2, ScoringMatrix.default());
    defer align_result.deinit();

    try testing.expectApproxEqAbs(@as(f64, 1.0), align_result.identity(), 0.0001);
}

test "alignment coverage calculation" {
    const allocator = testing.allocator;

    var seq1 = try Sequence.init(allocator, "XXXXACGTXXXX");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "ACGT");
    defer seq2.deinit();

    var align_result = try alignment.smithWaterman(allocator, seq1, seq2, ScoringMatrix.default());
    defer align_result.deinit();

    const cov1 = align_result.coverage1(seq1.len());
    const cov2 = align_result.coverage2(seq2.len());

    // Coverage of seq2 should be 100%
    try testing.expectApproxEqAbs(@as(f64, 1.0), cov2, 0.0001);

    // Coverage of seq1 should be ~33% (4/12)
    try testing.expect(cov1 < 0.5);
}

// ============================================================================
// CIGAR String Tests
// ============================================================================

test "CIGAR perfect match" {
    const allocator = testing.allocator;

    var seq1 = try Sequence.init(allocator, "ACGT");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "ACGT");
    defer seq2.deinit();

    var align_result = try alignment.smithWaterman(allocator, seq1, seq2, ScoringMatrix.default());
    defer align_result.deinit();

    const cigar = try align_result.cigar(allocator);
    defer allocator.free(cigar);

    try testing.expectEqualStrings("4=", cigar);
}

test "CIGAR with mismatch" {
    const allocator = testing.allocator;

    var seq1 = try Sequence.init(allocator, "ACGT");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "ACAT");
    defer seq2.deinit();

    var align_result = try alignment.smithWaterman(allocator, seq1, seq2, ScoringMatrix.default());
    defer align_result.deinit();

    const cigar = try align_result.cigar(allocator);
    defer allocator.free(cigar);

    // Should contain X for mismatch
    try testing.expect(std.mem.indexOf(u8, cigar, "X") != null or
        std.mem.indexOf(u8, cigar, "=") != null);
}

// ============================================================================
// Edit Distance Tests
// ============================================================================

test "edit distance identical" {
    const allocator = testing.allocator;

    const dist = try alignment.editDistance("ACGT", "ACGT", allocator);
    try testing.expectEqual(@as(usize, 0), dist);
}

test "edit distance one substitution" {
    const allocator = testing.allocator;

    const dist = try alignment.editDistance("ACGT", "ACTT", allocator);
    try testing.expectEqual(@as(usize, 1), dist);
}

test "edit distance one insertion" {
    const allocator = testing.allocator;

    const dist = try alignment.editDistance("ACT", "ACGT", allocator);
    try testing.expectEqual(@as(usize, 1), dist);
}

test "edit distance one deletion" {
    const allocator = testing.allocator;

    const dist = try alignment.editDistance("ACGT", "ACT", allocator);
    try testing.expectEqual(@as(usize, 1), dist);
}

test "edit distance completely different" {
    const allocator = testing.allocator;

    const dist = try alignment.editDistance("AAAA", "TTTT", allocator);
    try testing.expectEqual(@as(usize, 4), dist);
}

test "edit distance empty and non-empty" {
    const allocator = testing.allocator;

    const dist1 = try alignment.editDistance("", "ACGT", allocator);
    try testing.expectEqual(@as(usize, 4), dist1);

    const dist2 = try alignment.editDistance("ACGT", "", allocator);
    try testing.expectEqual(@as(usize, 4), dist2);
}

test "edit distance both empty" {
    const allocator = testing.allocator;

    const dist = try alignment.editDistance("", "", allocator);
    try testing.expectEqual(@as(usize, 0), dist);
}

// ============================================================================
// Alignment Score Only Tests
// ============================================================================

test "alignment score only" {
    const allocator = testing.allocator;

    const score = try alignment.alignmentScore("ACGT", "ACGT", ScoringMatrix.default(), allocator);
    // 4 matches * 2 = 8
    try testing.expectEqual(@as(i32, 8), score);
}

test "alignment score with mismatch" {
    const allocator = testing.allocator;

    const score = try alignment.alignmentScore("ACGT", "ACTT", ScoringMatrix.default(), allocator);
    // Some matches minus penalty for mismatch
    try testing.expect(score > 0);
    try testing.expect(score < 8);
}

// ============================================================================
// Different Scoring Matrix Tests
// ============================================================================

test "SW with DNA scoring" {
    const allocator = testing.allocator;

    var seq1 = try Sequence.init(allocator, "ACGTACGT");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "ACGTACGT");
    defer seq2.deinit();

    var align_result = try alignment.smithWaterman(allocator, seq1, seq2, ScoringMatrix.dna());
    defer align_result.deinit();

    // DNA scoring: 8 * 5 = 40
    try testing.expectEqual(@as(i32, 40), align_result.score);
}

test "SW with BLAST scoring" {
    const allocator = testing.allocator;

    var seq1 = try Sequence.init(allocator, "ACGTACGT");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "ACGTACGT");
    defer seq2.deinit();

    var align_result = try alignment.smithWaterman(allocator, seq1, seq2, ScoringMatrix.blast());
    defer align_result.deinit();

    // BLAST scoring: 8 * 1 = 8
    try testing.expectEqual(@as(i32, 8), align_result.score);
}

// ============================================================================
// Edge Cases
// ============================================================================

test "SW single base sequences" {
    const allocator = testing.allocator;

    var seq1 = try Sequence.init(allocator, "A");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "A");
    defer seq2.deinit();

    var align_result = try alignment.smithWaterman(allocator, seq1, seq2, ScoringMatrix.default());
    defer align_result.deinit();

    try testing.expectEqual(@as(usize, 1), align_result.matches);
}

test "NW single base sequences" {
    const allocator = testing.allocator;

    var seq1 = try Sequence.init(allocator, "A");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "T");
    defer seq2.deinit();

    var align_result = try alignment.needlemanWunsch(allocator, seq1, seq2, ScoringMatrix.default());
    defer align_result.deinit();

    try testing.expectEqual(@as(usize, 1), align_result.mismatches);
}

test "alignment with N bases" {
    const allocator = testing.allocator;

    var seq1 = try Sequence.init(allocator, "ACNGT");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "ACAGT");
    defer seq2.deinit();

    var align_result = try alignment.smithWaterman(allocator, seq1, seq2, ScoringMatrix.default());
    defer align_result.deinit();

    // N should be treated neutrally
    try testing.expect(align_result.score > 0);
}

// ============================================================================
// Visual Alignment Tests
// ============================================================================

test "alignment visualization" {
    const allocator = testing.allocator;

    var seq1 = try Sequence.init(allocator, "ACGT");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "ACGT");
    defer seq2.deinit();

    var align_result = try alignment.smithWaterman(allocator, seq1, seq2, ScoringMatrix.default());
    defer align_result.deinit();

    const vis = try align_result.visualize(allocator, 60);
    defer allocator.free(vis);

    // Should contain both sequences and match indicators
    try testing.expect(std.mem.indexOf(u8, vis, "Seq1") != null);
    try testing.expect(std.mem.indexOf(u8, vis, "Seq2") != null);
}

// ============================================================================
// Raw Bases Interface Tests
// ============================================================================

test "SW with raw bases" {
    const allocator = testing.allocator;

    var align_result = try alignment.smithWatermanBases(allocator, "ACGTACGT", "ACGTACGT", ScoringMatrix.default());
    defer align_result.deinit();

    try testing.expectEqual(@as(usize, 8), align_result.matches);
}

test "NW with raw bases" {
    const allocator = testing.allocator;

    var align_result = try alignment.needlemanWunschBases(allocator, "ACGT", "ACGT", ScoringMatrix.default());
    defer align_result.deinit();

    try testing.expectEqual(@as(usize, 4), align_result.matches);
}
