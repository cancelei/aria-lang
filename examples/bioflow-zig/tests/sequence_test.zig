//! Extended tests for the Sequence module
//!
//! These tests cover edge cases and comprehensive scenarios
//! beyond the basic unit tests in the main module.

const std = @import("std");
const testing = std.testing;
const Sequence = @import("sequence").Sequence;
const SequenceError = @import("sequence").SequenceError;
const parseFasta = @import("sequence").parseFasta;

// ============================================================================
// Initialization Tests
// ============================================================================

test "sequence with valid bases" {
    const allocator = testing.allocator;

    const valid_sequences = [_][]const u8{
        "ATGC",
        "AAAA",
        "CCCC",
        "GGGG",
        "TTTT",
        "NNNN",
        "ACGTACGTACGT",
        "ATGCATGCATGCATGCATGC",
    };

    for (valid_sequences) |bases| {
        var seq = try Sequence.init(allocator, bases);
        defer seq.deinit();
        try testing.expectEqual(bases.len, seq.len());
    }
}

test "sequence with lowercase converts to uppercase" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "atgcn");
    defer seq.deinit();

    try testing.expectEqualStrings("ATGCN", seq.bases);
}

test "sequence with mixed case" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "AtGcNaTgC");
    defer seq.deinit();

    try testing.expectEqualStrings("ATGCNATGC", seq.bases);
}

test "empty sequence returns error" {
    const allocator = testing.allocator;
    const result = Sequence.init(allocator, "");
    try testing.expectError(SequenceError.EmptySequence, result);
}

test "invalid base returns error" {
    const allocator = testing.allocator;

    const invalid_sequences = [_][]const u8{
        "ATGX", // X is invalid
        "ATGCZ",
        "12345",
        "ATGC!",
        "ATG C", // Space is invalid
    };

    for (invalid_sequences) |bases| {
        const result = Sequence.init(allocator, bases);
        try testing.expectError(SequenceError.InvalidBase, result);
    }
}

test "sequence with ID" {
    const allocator = testing.allocator;

    var seq = try Sequence.initWithId(allocator, "ATGC", "seq1");
    defer seq.deinit();

    try testing.expectEqualStrings("ATGC", seq.bases);
    try testing.expectEqualStrings("seq1", seq.id.?);
}

test "sequence with ID and description" {
    const allocator = testing.allocator;

    var seq = try Sequence.initWithIdAndDesc(allocator, "ATGC", "seq1", "Test sequence");
    defer seq.deinit();

    try testing.expectEqualStrings("ATGC", seq.bases);
    try testing.expectEqualStrings("seq1", seq.id.?);
    try testing.expectEqualStrings("Test sequence", seq.description.?);
}

// ============================================================================
// Clone Tests
// ============================================================================

test "sequence clone creates independent copy" {
    const allocator = testing.allocator;

    var original = try Sequence.initWithIdAndDesc(allocator, "ATGC", "original", "Original sequence");
    defer original.deinit();

    var cloned = try original.clone();
    defer cloned.deinit();

    // Verify contents match
    try testing.expectEqualStrings(original.bases, cloned.bases);
    try testing.expectEqualStrings(original.id.?, cloned.id.?);
    try testing.expectEqualStrings(original.description.?, cloned.description.?);

    // Verify they are independent (different pointers)
    try testing.expect(original.bases.ptr != cloned.bases.ptr);
}

// ============================================================================
// GC Content Tests
// ============================================================================

test "GC content all GC" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "GCGCGCGC");
    defer seq.deinit();

    try testing.expectApproxEqAbs(@as(f64, 1.0), seq.gcContent(), 0.0001);
}

test "GC content all AT" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "ATATATAT");
    defer seq.deinit();

    try testing.expectApproxEqAbs(@as(f64, 0.0), seq.gcContent(), 0.0001);
}

test "GC content 50%" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "ACGT");
    defer seq.deinit();

    try testing.expectApproxEqAbs(@as(f64, 0.5), seq.gcContent(), 0.0001);
}

test "GC content with N bases" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "GCNN");
    defer seq.deinit();

    // GC count = 2, total = 4, so 50%
    try testing.expectApproxEqAbs(@as(f64, 0.5), seq.gcContent(), 0.0001);
}

test "N content" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "ANNN");
    defer seq.deinit();

    try testing.expectApproxEqAbs(@as(f64, 0.75), seq.nContent(), 0.0001);
}

// ============================================================================
// Complement Tests
// ============================================================================

test "complement basic" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "ATGC");
    defer seq.deinit();

    var comp = try seq.complement();
    defer comp.deinit();

    try testing.expectEqualStrings("TACG", comp.bases);
}

test "complement with N" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "ATNGC");
    defer seq.deinit();

    var comp = try seq.complement();
    defer comp.deinit();

    try testing.expectEqualStrings("TANCG", comp.bases);
}

test "complement preserves ID" {
    const allocator = testing.allocator;

    var seq = try Sequence.initWithId(allocator, "ATGC", "test_seq");
    defer seq.deinit();

    var comp = try seq.complement();
    defer comp.deinit();

    try testing.expectEqualStrings("test_seq", comp.id.?);
}

test "double complement returns original" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "ATGCGATCGA");
    defer seq.deinit();

    var comp1 = try seq.complement();
    defer comp1.deinit();

    var comp2 = try comp1.complement();
    defer comp2.deinit();

    try testing.expectEqualStrings(seq.bases, comp2.bases);
}

// ============================================================================
// Reverse Tests
// ============================================================================

test "reverse basic" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "ATGC");
    defer seq.deinit();

    var rev = try seq.reverse();
    defer rev.deinit();

    try testing.expectEqualStrings("CGTA", rev.bases);
}

test "reverse palindrome unchanged" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "ACGTACGT");
    defer seq.deinit();

    var rev = try seq.reverse();
    defer rev.deinit();

    try testing.expectEqualStrings("TGCATGCA", rev.bases);
}

test "double reverse returns original" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "ATGCGATCGA");
    defer seq.deinit();

    var rev1 = try seq.reverse();
    defer rev1.deinit();

    var rev2 = try rev1.reverse();
    defer rev2.deinit();

    try testing.expectEqualStrings(seq.bases, rev2.bases);
}

// ============================================================================
// Reverse Complement Tests
// ============================================================================

test "reverse complement basic" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "ATGC");
    defer seq.deinit();

    var rc = try seq.reverseComplement();
    defer rc.deinit();

    try testing.expectEqualStrings("GCAT", rc.bases);
}

test "reverse complement of reverse complement" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "ATGCGATCGA");
    defer seq.deinit();

    var rc1 = try seq.reverseComplement();
    defer rc1.deinit();

    var rc2 = try rc1.reverseComplement();
    defer rc2.deinit();

    try testing.expectEqualStrings(seq.bases, rc2.bases);
}

// ============================================================================
// Subsequence Tests
// ============================================================================

test "subsequence valid range" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "ATGCGATCGA");
    defer seq.deinit();

    var sub = try seq.subsequence(2, 6);
    defer sub.deinit();

    try testing.expectEqualStrings("GCGA", sub.bases);
}

test "subsequence full length" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "ATGC");
    defer seq.deinit();

    var sub = try seq.subsequence(0, 4);
    defer sub.deinit();

    try testing.expectEqualStrings("ATGC", sub.bases);
}

test "subsequence single base" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "ATGC");
    defer seq.deinit();

    var sub = try seq.subsequence(1, 2);
    defer sub.deinit();

    try testing.expectEqualStrings("T", sub.bases);
}

test "subsequence invalid range" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "ATGC");
    defer seq.deinit();

    // Start >= end
    const result1 = seq.subsequence(2, 2);
    try testing.expectError(SequenceError.IndexOutOfBounds, result1);

    // Start >= length
    const result2 = seq.subsequence(10, 12);
    try testing.expectError(SequenceError.IndexOutOfBounds, result2);

    // End > length
    const result3 = seq.subsequence(0, 10);
    try testing.expectError(SequenceError.IndexOutOfBounds, result3);
}

// ============================================================================
// Concatenation Tests
// ============================================================================

test "concat two sequences" {
    const allocator = testing.allocator;

    var seq1 = try Sequence.init(allocator, "ATGC");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "GATC");
    defer seq2.deinit();

    var concat_seq = try seq1.concat(seq2);
    defer concat_seq.deinit();

    try testing.expectEqualStrings("ATGCGATC", concat_seq.bases);
}

// ============================================================================
// Pattern Finding Tests
// ============================================================================

test "contains pattern" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "ATGCGATCGA");
    defer seq.deinit();

    try testing.expect(seq.contains("ATG"));
    try testing.expect(seq.contains("GAT"));
    try testing.expect(seq.contains("CGA"));
    try testing.expect(!seq.contains("TTT"));
    try testing.expect(!seq.contains("ATGCGATCGAA")); // Longer than sequence
}

test "contains case insensitive" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "ATGCGATCGA");
    defer seq.deinit();

    try testing.expect(seq.contains("atg"));
    try testing.expect(seq.contains("Atg"));
}

test "find all occurrences" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "ATGATGATG");
    defer seq.deinit();

    const positions = try seq.findAll(allocator, "ATG");
    defer allocator.free(positions);

    try testing.expectEqual(@as(usize, 3), positions.len);
    try testing.expectEqual(@as(usize, 0), positions[0]);
    try testing.expectEqual(@as(usize, 3), positions[1]);
    try testing.expectEqual(@as(usize, 6), positions[2]);
}

test "find all no occurrences" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "AAAAAAA");
    defer seq.deinit();

    const positions = try seq.findAll(allocator, "TTT");
    defer allocator.free(positions);

    try testing.expectEqual(@as(usize, 0), positions.len);
}

// ============================================================================
// Hamming Distance Tests
// ============================================================================

test "hamming distance identical" {
    const allocator = testing.allocator;

    var seq1 = try Sequence.init(allocator, "ATGC");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "ATGC");
    defer seq2.deinit();

    const dist = seq1.hammingDistance(seq2);
    try testing.expectEqual(@as(?usize, 0), dist);
}

test "hamming distance one mismatch" {
    const allocator = testing.allocator;

    var seq1 = try Sequence.init(allocator, "ATGC");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "ATCC");
    defer seq2.deinit();

    const dist = seq1.hammingDistance(seq2);
    try testing.expectEqual(@as(?usize, 1), dist);
}

test "hamming distance all mismatches" {
    const allocator = testing.allocator;

    var seq1 = try Sequence.init(allocator, "AAAA");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "TTTT");
    defer seq2.deinit();

    const dist = seq1.hammingDistance(seq2);
    try testing.expectEqual(@as(?usize, 4), dist);
}

test "hamming distance different lengths" {
    const allocator = testing.allocator;

    var seq1 = try Sequence.init(allocator, "ATGC");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "ATGCG");
    defer seq2.deinit();

    const dist = seq1.hammingDistance(seq2);
    try testing.expectEqual(@as(?usize, null), dist);
}

// ============================================================================
// Base Counts Tests
// ============================================================================

test "base counts" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "AACCCGGGGTTTTTN");
    defer seq.deinit();

    const counts = seq.baseCounts();
    try testing.expectEqual(@as(usize, 2), counts.a);
    try testing.expectEqual(@as(usize, 3), counts.c);
    try testing.expectEqual(@as(usize, 4), counts.g);
    try testing.expectEqual(@as(usize, 5), counts.t);
    try testing.expectEqual(@as(usize, 1), counts.n);
}

// ============================================================================
// FASTA Parsing Tests
// ============================================================================

test "parse simple FASTA" {
    const allocator = testing.allocator;

    const fasta =
        \\>seq1
        \\ATGC
    ;

    const sequences = try parseFasta(allocator, fasta);
    defer {
        for (sequences) |*seq| {
            seq.deinit();
        }
        allocator.free(sequences);
    }

    try testing.expectEqual(@as(usize, 1), sequences.len);
    try testing.expectEqualStrings("seq1", sequences[0].id.?);
    try testing.expectEqualStrings("ATGC", sequences[0].bases);
}

test "parse FASTA with multiline sequence" {
    const allocator = testing.allocator;

    const fasta =
        \\>seq1
        \\ATGC
        \\GATC
        \\AAAA
    ;

    const sequences = try parseFasta(allocator, fasta);
    defer {
        for (sequences) |*seq| {
            seq.deinit();
        }
        allocator.free(sequences);
    }

    try testing.expectEqual(@as(usize, 1), sequences.len);
    try testing.expectEqualStrings("ATGCGATCAAAA", sequences[0].bases);
}

test "parse FASTA with description" {
    const allocator = testing.allocator;

    const fasta =
        \\>seq1 This is a description
        \\ATGC
    ;

    const sequences = try parseFasta(allocator, fasta);
    defer {
        for (sequences) |*seq| {
            seq.deinit();
        }
        allocator.free(sequences);
    }

    try testing.expectEqual(@as(usize, 1), sequences.len);
    try testing.expectEqualStrings("seq1", sequences[0].id.?);
    try testing.expectEqualStrings("This is a description", sequences[0].description.?);
}

test "parse multiple FASTA sequences" {
    const allocator = testing.allocator;

    const fasta =
        \\>seq1
        \\ATGC
        \\>seq2
        \\GGGG
        \\>seq3
        \\CCCC
    ;

    const sequences = try parseFasta(allocator, fasta);
    defer {
        for (sequences) |*seq| {
            seq.deinit();
        }
        allocator.free(sequences);
    }

    try testing.expectEqual(@as(usize, 3), sequences.len);
    try testing.expectEqualStrings("seq1", sequences[0].id.?);
    try testing.expectEqualStrings("seq2", sequences[1].id.?);
    try testing.expectEqualStrings("seq3", sequences[2].id.?);
}

// ============================================================================
// Molecular Properties Tests
// ============================================================================

test "molecular weight calculation" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "ATGC");
    defer seq.deinit();

    const mw = seq.molecularWeight();
    try testing.expect(mw > 0.0);
    // Approximate expected value (sum of nucleotide weights minus water)
    try testing.expect(mw > 1000.0);
    try testing.expect(mw < 2000.0);
}

test "melting temperature short oligo" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "ATGC");
    defer seq.deinit();

    const tm = seq.meltingTemperature();
    // Wallace rule: 2(A+T) + 4(G+C) = 2*2 + 4*2 = 12
    try testing.expectApproxEqAbs(@as(f64, 12.0), tm, 0.1);
}

test "melting temperature long sequence" {
    const allocator = testing.allocator;

    // Create a sequence longer than 14 bp
    var seq = try Sequence.init(allocator, "ATGCATGCATGCATGC");
    defer seq.deinit();

    const tm = seq.meltingTemperature();
    // Should use the more accurate formula
    try testing.expect(tm > 0.0);
    try testing.expect(tm < 100.0);
}

// ============================================================================
// FASTA Output Tests
// ============================================================================

test "sequence to FASTA format" {
    const allocator = testing.allocator;

    var seq = try Sequence.initWithIdAndDesc(allocator, "ATGCGATCGA", "test_seq", "A test sequence");
    defer seq.deinit();

    const fasta = try seq.toFasta(allocator, 4);
    defer allocator.free(fasta);

    // Should have header and wrapped sequence
    try testing.expect(std.mem.startsWith(u8, fasta, ">test_seq"));
    try testing.expect(std.mem.indexOf(u8, fasta, "A test sequence") != null);
}
