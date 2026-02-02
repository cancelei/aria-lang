//! Sequence alignment algorithms
//!
//! This module implements classic sequence alignment algorithms:
//! - Smith-Waterman (local alignment)
//! - Needleman-Wunsch (global alignment)
//!
//! Features:
//! - Configurable scoring matrices
//! - Affine gap penalties
//! - Multiple traceback support
//! - CIGAR string generation

const std = @import("std");
const Allocator = std.mem.Allocator;
const Sequence = @import("sequence").Sequence;

/// Errors that can occur during alignment
pub const AlignmentError = error{
    /// Memory allocation failed
    OutOfMemory,
    /// Invalid scoring parameters
    InvalidScoringParams,
    /// Sequences too short for alignment
    SequencesTooShort,
};

/// Scoring matrix configuration
pub const ScoringMatrix = struct {
    /// Score for matching bases
    match_score: i32,
    /// Penalty for mismatching bases
    mismatch_penalty: i32,
    /// Penalty for opening a gap
    gap_open: i32,
    /// Penalty for extending a gap
    gap_extend: i32,

    const Self = @This();

    /// Default scoring: match=2, mismatch=-1, gap=-2
    pub fn default() Self {
        return Self{
            .match_score = 2,
            .mismatch_penalty = -1,
            .gap_open = -2,
            .gap_extend = -2,
        };
    }

    /// DNA-specific scoring
    pub fn dna() Self {
        return Self{
            .match_score = 5,
            .mismatch_penalty = -4,
            .gap_open = -10,
            .gap_extend = -1,
        };
    }

    /// BLAST-like scoring
    pub fn blast() Self {
        return Self{
            .match_score = 1,
            .mismatch_penalty = -3,
            .gap_open = -5,
            .gap_extend = -2,
        };
    }

    /// Calculate score for aligning two bases
    pub fn score(self: Self, a: u8, b: u8) i32 {
        if (a == 'N' or b == 'N') {
            return 0; // Neutral score for ambiguous bases
        }
        return if (a == b) self.match_score else self.mismatch_penalty;
    }

    /// Get gap penalty (linear model)
    pub fn gapPenalty(self: Self) i32 {
        return self.gap_open;
    }

    /// Get affine gap penalty for gap of length k
    pub fn affineGapPenalty(self: Self, k: usize) i32 {
        if (k == 0) return 0;
        return self.gap_open + self.gap_extend * @as(i32, @intCast(k - 1));
    }
};

/// Direction for traceback
const Direction = enum(u8) {
    Stop = 0,
    Diagonal = 1,
    Up = 2,
    Left = 3,
};

/// Result of a sequence alignment
pub const Alignment = struct {
    /// Aligned first sequence (with gaps as '-')
    aligned_seq1: []u8,
    /// Aligned second sequence (with gaps as '-')
    aligned_seq2: []u8,
    /// Alignment score
    score: i32,
    /// Start position in seq1
    start1: usize,
    /// End position in seq1
    end1: usize,
    /// Start position in seq2
    start2: usize,
    /// End position in seq2
    end2: usize,
    /// Number of matches
    matches: usize,
    /// Number of mismatches
    mismatches: usize,
    /// Number of gaps
    gaps: usize,
    /// Allocator used
    allocator: Allocator,

    const Self = @This();

    /// Free alignment memory
    pub fn deinit(self: *Self) void {
        self.allocator.free(self.aligned_seq1);
        self.allocator.free(self.aligned_seq2);
        self.* = undefined;
    }

    /// Calculate alignment identity (matches / alignment length)
    pub fn identity(self: Self) f64 {
        const len = self.aligned_seq1.len;
        if (len == 0) return 0.0;
        return @as(f64, @floatFromInt(self.matches)) / @as(f64, @floatFromInt(len));
    }

    /// Calculate alignment coverage relative to seq1
    pub fn coverage1(self: Self, seq1_len: usize) f64 {
        if (seq1_len == 0) return 0.0;
        return @as(f64, @floatFromInt(self.end1 - self.start1)) / @as(f64, @floatFromInt(seq1_len));
    }

    /// Calculate alignment coverage relative to seq2
    pub fn coverage2(self: Self, seq2_len: usize) f64 {
        if (seq2_len == 0) return 0.0;
        return @as(f64, @floatFromInt(self.end2 - self.start2)) / @as(f64, @floatFromInt(seq2_len));
    }

    /// Generate CIGAR string
    pub fn cigar(self: Self, allocator: Allocator) ![]u8 {
        var result = std.ArrayList(u8).init(allocator);
        errdefer result.deinit();

        var current_op: u8 = 0;
        var current_count: usize = 0;

        for (self.aligned_seq1, self.aligned_seq2) |c1, c2| {
            const op: u8 = if (c1 == '-')
                'I' // Insertion in seq2
            else if (c2 == '-')
                'D' // Deletion from seq1
            else if (c1 == c2)
                '=' // Match
            else
                'X'; // Mismatch

            if (op == current_op) {
                current_count += 1;
            } else {
                if (current_count > 0) {
                    try self.appendCigarOp(&result, current_count, current_op);
                }
                current_op = op;
                current_count = 1;
            }
        }

        if (current_count > 0) {
            try self.appendCigarOp(&result, current_count, current_op);
        }

        return result.toOwnedSlice();
    }

    fn appendCigarOp(self: Self, result: *std.ArrayList(u8), count: usize, op: u8) !void {
        _ = self;
        var buf: [20]u8 = undefined;
        const len = std.fmt.formatIntBuf(&buf, count, 10, .lower, .{});
        try result.appendSlice(buf[0..len]);
        try result.append(op);
    }

    /// Generate visual alignment representation
    pub fn visualize(self: Self, allocator: Allocator, line_width: usize) ![]u8 {
        var result = std.ArrayList(u8).init(allocator);
        errdefer result.deinit();

        var i: usize = 0;
        while (i < self.aligned_seq1.len) : (i += line_width) {
            const end = @min(i + line_width, self.aligned_seq1.len);

            // Seq1 line
            try result.appendSlice("Seq1: ");
            try result.appendSlice(self.aligned_seq1[i..end]);
            try result.append('\n');

            // Match line
            try result.appendSlice("      ");
            for (self.aligned_seq1[i..end], self.aligned_seq2[i..end]) |c1, c2| {
                if (c1 == c2 and c1 != '-') {
                    try result.append('|');
                } else if (c1 != '-' and c2 != '-') {
                    try result.append('.');
                } else {
                    try result.append(' ');
                }
            }
            try result.append('\n');

            // Seq2 line
            try result.appendSlice("Seq2: ");
            try result.appendSlice(self.aligned_seq2[i..end]);
            try result.append('\n');
            try result.append('\n');
        }

        return result.toOwnedSlice();
    }
};

/// Smith-Waterman local alignment algorithm
pub fn smithWaterman(
    allocator: Allocator,
    seq1: Sequence,
    seq2: Sequence,
    scoring: ScoringMatrix,
) AlignmentError!Alignment {
    return smithWatermanBases(allocator, seq1.bases, seq2.bases, scoring);
}

/// Smith-Waterman on raw base strings
pub fn smithWatermanBases(
    allocator: Allocator,
    bases1: []const u8,
    bases2: []const u8,
    scoring: ScoringMatrix,
) AlignmentError!Alignment {
    const m = bases1.len;
    const n = bases2.len;

    if (m == 0 or n == 0) {
        return AlignmentError.SequencesTooShort;
    }

    // Allocate score matrix
    const h = allocator.alloc([]i32, m + 1) catch {
        return AlignmentError.OutOfMemory;
    };
    defer allocator.free(h);
    for (h) |*row| {
        row.* = allocator.alloc(i32, n + 1) catch {
            return AlignmentError.OutOfMemory;
        };
    }
    defer for (h) |row| allocator.free(row);

    // Allocate traceback matrix
    const traceback = allocator.alloc([]Direction, m + 1) catch {
        return AlignmentError.OutOfMemory;
    };
    defer allocator.free(traceback);
    for (traceback) |*row| {
        row.* = allocator.alloc(Direction, n + 1) catch {
            return AlignmentError.OutOfMemory;
        };
        @memset(row.*, .Stop);
    }
    defer for (traceback) |row| allocator.free(row);

    // Initialize first row and column to 0
    for (h) |row| {
        @memset(row, 0);
    }

    var max_score: i32 = 0;
    var max_i: usize = 0;
    var max_j: usize = 0;

    // Fill matrices
    var i: usize = 1;
    while (i <= m) : (i += 1) {
        var j: usize = 1;
        while (j <= n) : (j += 1) {
            const match_score = scoring.score(bases1[i - 1], bases2[j - 1]);
            const diag = h[i - 1][j - 1] + match_score;
            const up = h[i - 1][j] + scoring.gapPenalty();
            const left = h[i][j - 1] + scoring.gapPenalty();

            // Find maximum (including 0 for local alignment)
            var max_val: i32 = 0;
            var dir: Direction = .Stop;

            if (diag > max_val) {
                max_val = diag;
                dir = .Diagonal;
            }
            if (up > max_val) {
                max_val = up;
                dir = .Up;
            }
            if (left > max_val) {
                max_val = left;
                dir = .Left;
            }

            h[i][j] = max_val;
            traceback[i][j] = dir;

            if (max_val > max_score) {
                max_score = max_val;
                max_i = i;
                max_j = j;
            }
        }
    }

    // Traceback to build alignment
    var aligned1 = std.ArrayList(u8).init(allocator);
    defer aligned1.deinit();
    var aligned2 = std.ArrayList(u8).init(allocator);
    defer aligned2.deinit();

    var matches: usize = 0;
    var mismatches: usize = 0;
    var gaps: usize = 0;

    i = max_i;
    var j = max_j;
    const end1 = max_i;
    const end2 = max_j;

    while (traceback[i][j] != .Stop and i > 0 and j > 0) {
        switch (traceback[i][j]) {
            .Diagonal => {
                aligned1.insert(0, bases1[i - 1]) catch {
                    return AlignmentError.OutOfMemory;
                };
                aligned2.insert(0, bases2[j - 1]) catch {
                    return AlignmentError.OutOfMemory;
                };
                if (bases1[i - 1] == bases2[j - 1]) {
                    matches += 1;
                } else {
                    mismatches += 1;
                }
                i -= 1;
                j -= 1;
            },
            .Up => {
                aligned1.insert(0, bases1[i - 1]) catch {
                    return AlignmentError.OutOfMemory;
                };
                aligned2.insert(0, '-') catch {
                    return AlignmentError.OutOfMemory;
                };
                gaps += 1;
                i -= 1;
            },
            .Left => {
                aligned1.insert(0, '-') catch {
                    return AlignmentError.OutOfMemory;
                };
                aligned2.insert(0, bases2[j - 1]) catch {
                    return AlignmentError.OutOfMemory;
                };
                gaps += 1;
                j -= 1;
            },
            .Stop => break,
        }
    }

    const start1 = i;
    const start2 = j;

    return Alignment{
        .aligned_seq1 = aligned1.toOwnedSlice() catch {
            return AlignmentError.OutOfMemory;
        },
        .aligned_seq2 = aligned2.toOwnedSlice() catch {
            return AlignmentError.OutOfMemory;
        },
        .score = max_score,
        .start1 = start1,
        .end1 = end1,
        .start2 = start2,
        .end2 = end2,
        .matches = matches,
        .mismatches = mismatches,
        .gaps = gaps,
        .allocator = allocator,
    };
}

/// Needleman-Wunsch global alignment algorithm
pub fn needlemanWunsch(
    allocator: Allocator,
    seq1: Sequence,
    seq2: Sequence,
    scoring: ScoringMatrix,
) AlignmentError!Alignment {
    return needlemanWunschBases(allocator, seq1.bases, seq2.bases, scoring);
}

/// Needleman-Wunsch on raw base strings
pub fn needlemanWunschBases(
    allocator: Allocator,
    bases1: []const u8,
    bases2: []const u8,
    scoring: ScoringMatrix,
) AlignmentError!Alignment {
    const m = bases1.len;
    const n = bases2.len;

    if (m == 0 or n == 0) {
        return AlignmentError.SequencesTooShort;
    }

    // Allocate score matrix
    const h = allocator.alloc([]i32, m + 1) catch {
        return AlignmentError.OutOfMemory;
    };
    defer allocator.free(h);
    for (h) |*row| {
        row.* = allocator.alloc(i32, n + 1) catch {
            return AlignmentError.OutOfMemory;
        };
    }
    defer for (h) |row| allocator.free(row);

    // Allocate traceback matrix
    const traceback = allocator.alloc([]Direction, m + 1) catch {
        return AlignmentError.OutOfMemory;
    };
    defer allocator.free(traceback);
    for (traceback) |*row| {
        row.* = allocator.alloc(Direction, n + 1) catch {
            return AlignmentError.OutOfMemory;
        };
        @memset(row.*, .Stop);
    }
    defer for (traceback) |row| allocator.free(row);

    // Initialize first row and column (global alignment requires gap penalties)
    const gap = scoring.gapPenalty();
    var i: usize = 0;
    while (i <= m) : (i += 1) {
        h[i][0] = gap * @as(i32, @intCast(i));
        if (i > 0) traceback[i][0] = .Up;
    }
    var j: usize = 0;
    while (j <= n) : (j += 1) {
        h[0][j] = gap * @as(i32, @intCast(j));
        if (j > 0) traceback[0][j] = .Left;
    }

    // Fill matrices
    i = 1;
    while (i <= m) : (i += 1) {
        j = 1;
        while (j <= n) : (j += 1) {
            const match_score = scoring.score(bases1[i - 1], bases2[j - 1]);
            const diag = h[i - 1][j - 1] + match_score;
            const up = h[i - 1][j] + gap;
            const left = h[i][j - 1] + gap;

            // Find maximum (no zero clipping for global alignment)
            var max_val = diag;
            var dir: Direction = .Diagonal;

            if (up > max_val) {
                max_val = up;
                dir = .Up;
            }
            if (left > max_val) {
                max_val = left;
                dir = .Left;
            }

            h[i][j] = max_val;
            traceback[i][j] = dir;
        }
    }

    // Traceback from bottom-right corner
    var aligned1 = std.ArrayList(u8).init(allocator);
    defer aligned1.deinit();
    var aligned2 = std.ArrayList(u8).init(allocator);
    defer aligned2.deinit();

    var matches: usize = 0;
    var mismatches: usize = 0;
    var gaps_count: usize = 0;

    i = m;
    j = n;

    while (i > 0 or j > 0) {
        if (i > 0 and j > 0 and traceback[i][j] == .Diagonal) {
            aligned1.insert(0, bases1[i - 1]) catch {
                return AlignmentError.OutOfMemory;
            };
            aligned2.insert(0, bases2[j - 1]) catch {
                return AlignmentError.OutOfMemory;
            };
            if (bases1[i - 1] == bases2[j - 1]) {
                matches += 1;
            } else {
                mismatches += 1;
            }
            i -= 1;
            j -= 1;
        } else if (i > 0 and (j == 0 or traceback[i][j] == .Up)) {
            aligned1.insert(0, bases1[i - 1]) catch {
                return AlignmentError.OutOfMemory;
            };
            aligned2.insert(0, '-') catch {
                return AlignmentError.OutOfMemory;
            };
            gaps_count += 1;
            i -= 1;
        } else if (j > 0) {
            aligned1.insert(0, '-') catch {
                return AlignmentError.OutOfMemory;
            };
            aligned2.insert(0, bases2[j - 1]) catch {
                return AlignmentError.OutOfMemory;
            };
            gaps_count += 1;
            j -= 1;
        }
    }

    return Alignment{
        .aligned_seq1 = aligned1.toOwnedSlice() catch {
            return AlignmentError.OutOfMemory;
        },
        .aligned_seq2 = aligned2.toOwnedSlice() catch {
            return AlignmentError.OutOfMemory;
        },
        .score = h[m][n],
        .start1 = 0,
        .end1 = m,
        .start2 = 0,
        .end2 = n,
        .matches = matches,
        .mismatches = mismatches,
        .gaps = gaps_count,
        .allocator = allocator,
    };
}

/// Calculate edit distance (Levenshtein distance) between two sequences
pub fn editDistance(bases1: []const u8, bases2: []const u8, allocator: Allocator) AlignmentError!usize {
    const m = bases1.len;
    const n = bases2.len;

    if (m == 0) return n;
    if (n == 0) return m;

    // Use only two rows to save memory
    var prev_row = allocator.alloc(usize, n + 1) catch {
        return AlignmentError.OutOfMemory;
    };
    defer allocator.free(prev_row);

    var curr_row = allocator.alloc(usize, n + 1) catch {
        return AlignmentError.OutOfMemory;
    };
    defer allocator.free(curr_row);

    // Initialize first row
    var j: usize = 0;
    while (j <= n) : (j += 1) {
        prev_row[j] = j;
    }

    // Fill matrix
    var i: usize = 1;
    while (i <= m) : (i += 1) {
        curr_row[0] = i;

        j = 1;
        while (j <= n) : (j += 1) {
            const cost: usize = if (bases1[i - 1] == bases2[j - 1]) 0 else 1;
            curr_row[j] = @min(@min(prev_row[j] + 1, curr_row[j - 1] + 1), prev_row[j - 1] + cost);
        }

        // Swap rows
        const temp = prev_row;
        prev_row = curr_row;
        curr_row = temp;
    }

    return prev_row[n];
}

/// Calculate alignment score without full alignment (more memory efficient)
pub fn alignmentScore(
    bases1: []const u8,
    bases2: []const u8,
    scoring: ScoringMatrix,
    allocator: Allocator,
) AlignmentError!i32 {
    const m = bases1.len;
    const n = bases2.len;

    if (m == 0 or n == 0) return 0;

    // Use only two rows
    var prev_row = allocator.alloc(i32, n + 1) catch {
        return AlignmentError.OutOfMemory;
    };
    defer allocator.free(prev_row);

    var curr_row = allocator.alloc(i32, n + 1) catch {
        return AlignmentError.OutOfMemory;
    };
    defer allocator.free(curr_row);

    @memset(prev_row, 0);

    var max_score: i32 = 0;

    var i: usize = 1;
    while (i <= m) : (i += 1) {
        curr_row[0] = 0;

        var j: usize = 1;
        while (j <= n) : (j += 1) {
            const match_score = scoring.score(bases1[i - 1], bases2[j - 1]);
            const diag = prev_row[j - 1] + match_score;
            const up = prev_row[j] + scoring.gapPenalty();
            const left = curr_row[j - 1] + scoring.gapPenalty();

            curr_row[j] = @max(0, @max(@max(diag, up), left));
            max_score = @max(max_score, curr_row[j]);
        }

        // Swap rows
        const temp = prev_row;
        prev_row = curr_row;
        curr_row = temp;
    }

    return max_score;
}

// Tests
const testing = std.testing;

test "ScoringMatrix defaults" {
    const scoring = ScoringMatrix.default();
    try testing.expectEqual(@as(i32, 2), scoring.score('A', 'A'));
    try testing.expectEqual(@as(i32, -1), scoring.score('A', 'T'));
    try testing.expectEqual(@as(i32, -2), scoring.gapPenalty());
}

test "Smith-Waterman basic alignment" {
    const allocator = testing.allocator;

    var seq1 = try Sequence.init(allocator, "ACGTACGT");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "ACGTACGT");
    defer seq2.deinit();

    var align = try smithWaterman(allocator, seq1, seq2, ScoringMatrix.default());
    defer align.deinit();

    try testing.expectEqual(@as(usize, 8), align.matches);
    try testing.expectEqual(@as(usize, 0), align.mismatches);
    try testing.expectEqual(@as(usize, 0), align.gaps);
}

test "Smith-Waterman with gap" {
    const allocator = testing.allocator;

    var seq1 = try Sequence.init(allocator, "ACGTACGT");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "ACGACGT");
    defer seq2.deinit();

    var align = try smithWaterman(allocator, seq1, seq2, ScoringMatrix.default());
    defer align.deinit();

    try testing.expect(align.gaps > 0 or align.mismatches > 0);
}

test "Smith-Waterman local alignment" {
    const allocator = testing.allocator;

    var seq1 = try Sequence.init(allocator, "XXXXACGTXXXX");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "YYYYACGTYYYY");
    defer seq2.deinit();

    var align = try smithWaterman(allocator, seq1, seq2, ScoringMatrix.default());
    defer align.deinit();

    // Should find the ACGT match in the middle
    try testing.expect(align.score > 0);
    try testing.expect(align.matches >= 4);
}

test "Needleman-Wunsch global alignment" {
    const allocator = testing.allocator;

    var seq1 = try Sequence.init(allocator, "ACGT");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "ACT");
    defer seq2.deinit();

    var align = try needlemanWunsch(allocator, seq1, seq2, ScoringMatrix.default());
    defer align.deinit();

    // Global alignment should include entire sequences
    try testing.expectEqual(@as(usize, 4), align.aligned_seq1.len);
    try testing.expectEqual(@as(usize, 4), align.aligned_seq2.len);
}

test "Edit distance" {
    const allocator = testing.allocator;

    const dist1 = try editDistance("ACGT", "ACGT", allocator);
    try testing.expectEqual(@as(usize, 0), dist1);

    const dist2 = try editDistance("ACGT", "ACT", allocator);
    try testing.expectEqual(@as(usize, 1), dist2);

    const dist3 = try editDistance("ACGT", "TGCA", allocator);
    try testing.expectEqual(@as(usize, 4), dist3);
}

test "Alignment identity" {
    const allocator = testing.allocator;

    var seq1 = try Sequence.init(allocator, "ACGT");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "ACGT");
    defer seq2.deinit();

    var align = try smithWaterman(allocator, seq1, seq2, ScoringMatrix.default());
    defer align.deinit();

    try testing.expectApproxEqAbs(@as(f64, 1.0), align.identity(), 0.0001);
}

test "CIGAR string generation" {
    const allocator = testing.allocator;

    var seq1 = try Sequence.init(allocator, "ACGT");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "ACGT");
    defer seq2.deinit();

    var align = try smithWaterman(allocator, seq1, seq2, ScoringMatrix.default());
    defer align.deinit();

    const cig = try align.cigar(allocator);
    defer allocator.free(cig);

    // Perfect match should be "4="
    try testing.expectEqualStrings("4=", cig);
}

test "Alignment score only" {
    const allocator = testing.allocator;

    const score = try alignmentScore("ACGT", "ACGT", ScoringMatrix.default(), allocator);
    try testing.expect(score > 0);
}
