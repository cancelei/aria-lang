//! DNA/RNA Sequence representation and operations
//!
//! This module provides a robust Sequence type for biological sequence analysis.
//! It supports standard nucleotide bases (A, C, G, T) and ambiguous bases (N).
//!
//! Key features:
//! - Explicit memory management with allocator support
//! - Validation of sequence data
//! - Common operations: complement, reverse, GC content
//! - FASTA format support

const std = @import("std");
const Allocator = std.mem.Allocator;
const testing = std.testing;

/// Errors that can occur during sequence operations
pub const SequenceError = error{
    /// Sequence cannot be empty
    EmptySequence,
    /// Invalid nucleotide base encountered
    InvalidBase,
    /// Memory allocation failed
    OutOfMemory,
    /// Subsequence indices are out of bounds
    IndexOutOfBounds,
    /// Invalid k-mer size
    InvalidKmerSize,
};

/// Valid nucleotide bases for DNA sequences
pub const VALID_BASES = "ACGTN";

/// Represents a biological sequence (DNA/RNA)
pub const Sequence = struct {
    /// The nucleotide bases as uppercase characters
    bases: []u8,
    /// Optional sequence identifier (e.g., from FASTA header)
    id: ?[]u8,
    /// Optional sequence description
    description: ?[]u8,
    /// Allocator used for memory management
    allocator: Allocator,

    const Self = @This();

    /// Initialize a new sequence from a string of bases.
    /// The bases are validated and converted to uppercase.
    ///
    /// Returns an error if:
    /// - The sequence is empty
    /// - Invalid nucleotide bases are found
    /// - Memory allocation fails
    pub fn init(allocator: Allocator, bases: []const u8) SequenceError!Self {
        if (bases.len == 0) {
            return SequenceError.EmptySequence;
        }

        // Allocate and validate
        const upper = allocator.alloc(u8, bases.len) catch {
            return SequenceError.OutOfMemory;
        };
        errdefer allocator.free(upper);

        for (bases, 0..) |c, i| {
            const uc = std.ascii.toUpper(c);
            if (!isValidBase(uc)) {
                allocator.free(upper);
                return SequenceError.InvalidBase;
            }
            upper[i] = uc;
        }

        return Self{
            .bases = upper,
            .id = null,
            .description = null,
            .allocator = allocator,
        };
    }

    /// Initialize a sequence with an ID
    pub fn initWithId(allocator: Allocator, bases: []const u8, id: []const u8) SequenceError!Self {
        var seq = try init(allocator, bases);
        errdefer seq.deinit();

        seq.id = allocator.dupe(u8, id) catch {
            return SequenceError.OutOfMemory;
        };

        return seq;
    }

    /// Initialize a sequence with ID and description
    pub fn initWithIdAndDesc(
        allocator: Allocator,
        bases: []const u8,
        id: []const u8,
        desc: []const u8,
    ) SequenceError!Self {
        var seq = try init(allocator, bases);
        errdefer seq.deinit();

        seq.id = allocator.dupe(u8, id) catch {
            return SequenceError.OutOfMemory;
        };
        errdefer if (seq.id) |s_id| allocator.free(s_id);

        seq.description = allocator.dupe(u8, desc) catch {
            return SequenceError.OutOfMemory;
        };

        return seq;
    }

    /// Free all memory associated with the sequence
    pub fn deinit(self: *Self) void {
        self.allocator.free(self.bases);
        if (self.id) |id| {
            self.allocator.free(id);
        }
        if (self.description) |desc| {
            self.allocator.free(desc);
        }
        self.* = undefined;
    }

    /// Create a deep copy of the sequence
    pub fn clone(self: Self) SequenceError!Self {
        const new_bases = self.allocator.dupe(u8, self.bases) catch {
            return SequenceError.OutOfMemory;
        };
        errdefer self.allocator.free(new_bases);

        var new_id: ?[]u8 = null;
        if (self.id) |id| {
            new_id = self.allocator.dupe(u8, id) catch {
                return SequenceError.OutOfMemory;
            };
        }
        errdefer if (new_id) |n_id| self.allocator.free(n_id);

        var new_desc: ?[]u8 = null;
        if (self.description) |desc| {
            new_desc = self.allocator.dupe(u8, desc) catch {
                return SequenceError.OutOfMemory;
            };
        }

        return Self{
            .bases = new_bases,
            .id = new_id,
            .description = new_desc,
            .allocator = self.allocator,
        };
    }

    /// Get the length of the sequence
    pub fn len(self: Self) usize {
        return self.bases.len;
    }

    /// Check if a character is a valid nucleotide base
    pub fn isValidBase(c: u8) bool {
        return switch (c) {
            'A', 'C', 'G', 'T', 'N' => true,
            else => false,
        };
    }

    /// Calculate the GC content (ratio of G and C bases)
    pub fn gcContent(self: Self) f64 {
        var gc_count: usize = 0;
        for (self.bases) |base| {
            if (base == 'G' or base == 'C') {
                gc_count += 1;
            }
        }
        return @as(f64, @floatFromInt(gc_count)) / @as(f64, @floatFromInt(self.bases.len));
    }

    /// Calculate the AT content (ratio of A and T bases)
    pub fn atContent(self: Self) f64 {
        return 1.0 - self.gcContent() - self.nContent();
    }

    /// Calculate the N content (ratio of ambiguous bases)
    pub fn nContent(self: Self) f64 {
        var n_count: usize = 0;
        for (self.bases) |base| {
            if (base == 'N') {
                n_count += 1;
            }
        }
        return @as(f64, @floatFromInt(n_count)) / @as(f64, @floatFromInt(self.bases.len));
    }

    /// Count occurrences of each base
    pub const BaseCounts = struct {
        a: usize,
        c: usize,
        g: usize,
        t: usize,
        n: usize,
    };

    pub fn baseCounts(self: Self) BaseCounts {
        var counts = BaseCounts{ .a = 0, .c = 0, .g = 0, .t = 0, .n = 0 };
        for (self.bases) |base| {
            switch (base) {
                'A' => counts.a += 1,
                'C' => counts.c += 1,
                'G' => counts.g += 1,
                'T' => counts.t += 1,
                'N' => counts.n += 1,
                else => {},
            }
        }
        return counts;
    }

    /// Get the complement of a single base
    fn complementBase(base: u8) u8 {
        return switch (base) {
            'A' => 'T',
            'T' => 'A',
            'C' => 'G',
            'G' => 'C',
            else => 'N',
        };
    }

    /// Create the complement sequence
    pub fn complement(self: Self) SequenceError!Self {
        const comp = self.allocator.alloc(u8, self.bases.len) catch {
            return SequenceError.OutOfMemory;
        };
        errdefer self.allocator.free(comp);

        for (self.bases, 0..) |base, i| {
            comp[i] = complementBase(base);
        }

        var new_id: ?[]u8 = null;
        if (self.id) |id| {
            new_id = self.allocator.dupe(u8, id) catch {
                return SequenceError.OutOfMemory;
            };
        }
        errdefer if (new_id) |n_id| self.allocator.free(n_id);

        var new_desc: ?[]u8 = null;
        if (self.description) |desc| {
            new_desc = self.allocator.dupe(u8, desc) catch {
                return SequenceError.OutOfMemory;
            };
        }

        return Self{
            .bases = comp,
            .id = new_id,
            .description = new_desc,
            .allocator = self.allocator,
        };
    }

    /// Create the reverse sequence
    pub fn reverse(self: Self) SequenceError!Self {
        const rev = self.allocator.alloc(u8, self.bases.len) catch {
            return SequenceError.OutOfMemory;
        };
        errdefer self.allocator.free(rev);

        var i: usize = 0;
        while (i < self.bases.len) : (i += 1) {
            rev[i] = self.bases[self.bases.len - 1 - i];
        }

        var new_id: ?[]u8 = null;
        if (self.id) |id| {
            new_id = self.allocator.dupe(u8, id) catch {
                return SequenceError.OutOfMemory;
            };
        }
        errdefer if (new_id) |n_id| self.allocator.free(n_id);

        var new_desc: ?[]u8 = null;
        if (self.description) |desc| {
            new_desc = self.allocator.dupe(u8, desc) catch {
                return SequenceError.OutOfMemory;
            };
        }

        return Self{
            .bases = rev,
            .id = new_id,
            .description = new_desc,
            .allocator = self.allocator,
        };
    }

    /// Create the reverse complement sequence
    pub fn reverseComplement(self: Self) SequenceError!Self {
        const rev_comp = self.allocator.alloc(u8, self.bases.len) catch {
            return SequenceError.OutOfMemory;
        };
        errdefer self.allocator.free(rev_comp);

        var i: usize = 0;
        while (i < self.bases.len) : (i += 1) {
            rev_comp[i] = complementBase(self.bases[self.bases.len - 1 - i]);
        }

        var new_id: ?[]u8 = null;
        if (self.id) |id| {
            new_id = self.allocator.dupe(u8, id) catch {
                return SequenceError.OutOfMemory;
            };
        }
        errdefer if (new_id) |n_id| self.allocator.free(n_id);

        var new_desc: ?[]u8 = null;
        if (self.description) |desc| {
            new_desc = self.allocator.dupe(u8, desc) catch {
                return SequenceError.OutOfMemory;
            };
        }

        return Self{
            .bases = rev_comp,
            .id = new_id,
            .description = new_desc,
            .allocator = self.allocator,
        };
    }

    /// Extract a subsequence by indices (0-based, end exclusive)
    pub fn subsequence(self: Self, start: usize, end: usize) SequenceError!Self {
        if (start >= self.bases.len or end > self.bases.len or start >= end) {
            return SequenceError.IndexOutOfBounds;
        }

        const sub_len = end - start;
        const sub_bases = self.allocator.alloc(u8, sub_len) catch {
            return SequenceError.OutOfMemory;
        };
        errdefer self.allocator.free(sub_bases);

        @memcpy(sub_bases, self.bases[start..end]);

        return Self{
            .bases = sub_bases,
            .id = null,
            .description = null,
            .allocator = self.allocator,
        };
    }

    /// Concatenate two sequences
    pub fn concat(self: Self, other: Self) SequenceError!Self {
        const new_len = self.bases.len + other.bases.len;
        const new_bases = self.allocator.alloc(u8, new_len) catch {
            return SequenceError.OutOfMemory;
        };
        errdefer self.allocator.free(new_bases);

        @memcpy(new_bases[0..self.bases.len], self.bases);
        @memcpy(new_bases[self.bases.len..], other.bases);

        return Self{
            .bases = new_bases,
            .id = null,
            .description = null,
            .allocator = self.allocator,
        };
    }

    /// Check if this sequence contains a pattern
    pub fn contains(self: Self, pattern: []const u8) bool {
        if (pattern.len > self.bases.len) return false;
        if (pattern.len == 0) return true;

        var i: usize = 0;
        while (i <= self.bases.len - pattern.len) : (i += 1) {
            var match = true;
            for (pattern, 0..) |c, j| {
                const uc = std.ascii.toUpper(c);
                if (self.bases[i + j] != uc) {
                    match = false;
                    break;
                }
            }
            if (match) return true;
        }
        return false;
    }

    /// Find all occurrences of a pattern
    pub fn findAll(self: Self, allocator: Allocator, pattern: []const u8) SequenceError![]usize {
        var positions = std.ArrayList(usize).init(allocator);
        defer positions.deinit();

        if (pattern.len > self.bases.len or pattern.len == 0) {
            return allocator.dupe(usize, positions.items) catch {
                return SequenceError.OutOfMemory;
            };
        }

        var i: usize = 0;
        while (i <= self.bases.len - pattern.len) : (i += 1) {
            var match = true;
            for (pattern, 0..) |c, j| {
                const uc = std.ascii.toUpper(c);
                if (self.bases[i + j] != uc) {
                    match = false;
                    break;
                }
            }
            if (match) {
                positions.append(i) catch {
                    return SequenceError.OutOfMemory;
                };
            }
        }

        return allocator.dupe(usize, positions.items) catch {
            return SequenceError.OutOfMemory;
        };
    }

    /// Calculate Hamming distance between two sequences of equal length
    pub fn hammingDistance(self: Self, other: Self) ?usize {
        if (self.bases.len != other.bases.len) return null;

        var distance: usize = 0;
        for (self.bases, other.bases) |a, b| {
            if (a != b) distance += 1;
        }
        return distance;
    }

    /// Format sequence as FASTA
    pub fn toFasta(self: Self, allocator: Allocator, line_width: usize) SequenceError![]u8 {
        const id_str = self.id orelse "unknown";
        const desc_str = self.description orelse "";

        // Calculate output size
        const header_len = 1 + id_str.len + (if (desc_str.len > 0) 1 + desc_str.len else 0) + 1; // >id desc\n
        const num_lines = (self.bases.len + line_width - 1) / line_width;
        const body_len = self.bases.len + num_lines; // bases + newlines
        const total_len = header_len + body_len;

        var result = allocator.alloc(u8, total_len) catch {
            return SequenceError.OutOfMemory;
        };
        errdefer allocator.free(result);

        var pos: usize = 0;

        // Write header
        result[pos] = '>';
        pos += 1;
        @memcpy(result[pos .. pos + id_str.len], id_str);
        pos += id_str.len;
        if (desc_str.len > 0) {
            result[pos] = ' ';
            pos += 1;
            @memcpy(result[pos .. pos + desc_str.len], desc_str);
            pos += desc_str.len;
        }
        result[pos] = '\n';
        pos += 1;

        // Write bases with line breaks
        var i: usize = 0;
        while (i < self.bases.len) : (i += line_width) {
            const chunk_end = @min(i + line_width, self.bases.len);
            const chunk_len = chunk_end - i;
            @memcpy(result[pos .. pos + chunk_len], self.bases[i..chunk_end]);
            pos += chunk_len;
            result[pos] = '\n';
            pos += 1;
        }

        return result[0..pos];
    }

    /// Calculate the molecular weight of the sequence (approximate, DNA)
    pub fn molecularWeight(self: Self) f64 {
        // Approximate molecular weights for DNA nucleotides (g/mol)
        // A: 331.2, C: 307.2, G: 347.2, T: 322.2
        var weight: f64 = 0.0;
        for (self.bases) |base| {
            weight += switch (base) {
                'A' => 331.2,
                'C' => 307.2,
                'G' => 347.2,
                'T' => 322.2,
                else => 327.0, // Average for N
            };
        }
        // Subtract water for phosphodiester bonds
        if (self.bases.len > 0) {
            weight -= 61.96 * @as(f64, @floatFromInt(self.bases.len - 1));
        }
        return weight;
    }

    /// Calculate melting temperature (Tm) using the Wallace rule (for short oligos)
    pub fn meltingTemperature(self: Self) f64 {
        if (self.bases.len < 14) {
            // Wallace rule for short oligos: Tm = 2(A+T) + 4(G+C)
            const counts = self.baseCounts();
            const at = counts.a + counts.t;
            const gc = counts.g + counts.c;
            return @as(f64, @floatFromInt(2 * at + 4 * gc));
        } else {
            // More accurate formula for longer sequences
            // Tm = 64.9 + 41 * (G+C - 16.4) / N
            const gc = self.gcContent();
            const n = @as(f64, @floatFromInt(self.bases.len));
            return 64.9 + 41.0 * (gc * n - 16.4) / n;
        }
    }
};

/// Parse a FASTA format string into sequences
pub fn parseFasta(allocator: Allocator, data: []const u8) SequenceError![]Sequence {
    var sequences = std.ArrayList(Sequence).init(allocator);
    errdefer {
        for (sequences.items) |*seq| {
            seq.deinit();
        }
        sequences.deinit();
    }

    var lines = std.mem.splitSequence(u8, data, "\n");
    var current_id: ?[]u8 = null;
    var current_desc: ?[]u8 = null;
    var current_bases = std.ArrayList(u8).init(allocator);
    defer current_bases.deinit();

    while (lines.next()) |line| {
        const trimmed = std.mem.trim(u8, line, " \t\r");
        if (trimmed.len == 0) continue;

        if (trimmed[0] == '>') {
            // Save previous sequence if exists
            if (current_bases.items.len > 0) {
                var seq = try Sequence.init(allocator, current_bases.items);
                seq.id = current_id;
                seq.description = current_desc;
                sequences.append(seq) catch {
                    return SequenceError.OutOfMemory;
                };
                current_bases.clearRetainingCapacity();
                current_id = null;
                current_desc = null;
            }

            // Parse header
            const header = trimmed[1..];
            var parts = std.mem.splitScalar(u8, header, ' ');
            const id_part = parts.next() orelse "";
            current_id = allocator.dupe(u8, id_part) catch {
                return SequenceError.OutOfMemory;
            };

            // Rest is description
            const rest = parts.rest();
            if (rest.len > 0) {
                current_desc = allocator.dupe(u8, rest) catch {
                    return SequenceError.OutOfMemory;
                };
            }
        } else {
            // Sequence line
            for (trimmed) |c| {
                if (!std.ascii.isWhitespace(c)) {
                    current_bases.append(c) catch {
                        return SequenceError.OutOfMemory;
                    };
                }
            }
        }
    }

    // Save last sequence
    if (current_bases.items.len > 0) {
        var seq = try Sequence.init(allocator, current_bases.items);
        seq.id = current_id;
        seq.description = current_desc;
        sequences.append(seq) catch {
            return SequenceError.OutOfMemory;
        };
    } else {
        // Clean up if no bases
        if (current_id) |id| allocator.free(id);
        if (current_desc) |desc| allocator.free(desc);
    }

    return sequences.toOwnedSlice() catch {
        return SequenceError.OutOfMemory;
    };
}

// Tests
test "Sequence initialization" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "ATGC");
    defer seq.deinit();

    try testing.expectEqualStrings("ATGC", seq.bases);
    try testing.expectEqual(@as(usize, 4), seq.len());
}

test "Sequence rejects empty" {
    const allocator = testing.allocator;
    const result = Sequence.init(allocator, "");
    try testing.expectError(SequenceError.EmptySequence, result);
}

test "Sequence rejects invalid bases" {
    const allocator = testing.allocator;
    const result = Sequence.init(allocator, "ATXGC");
    try testing.expectError(SequenceError.InvalidBase, result);
}

test "Sequence converts to uppercase" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "atgc");
    defer seq.deinit();

    try testing.expectEqualStrings("ATGC", seq.bases);
}

test "GC content calculation" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "ATGCGC");
    defer seq.deinit();

    const gc = seq.gcContent();
    try testing.expectApproxEqAbs(@as(f64, 4.0 / 6.0), gc, 0.0001);
}

test "Complement sequence" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "ATGC");
    defer seq.deinit();

    var comp = try seq.complement();
    defer comp.deinit();

    try testing.expectEqualStrings("TACG", comp.bases);
}

test "Reverse sequence" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "ATGC");
    defer seq.deinit();

    var rev = try seq.reverse();
    defer rev.deinit();

    try testing.expectEqualStrings("CGTA", rev.bases);
}

test "Reverse complement" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "ATGC");
    defer seq.deinit();

    var rc = try seq.reverseComplement();
    defer rc.deinit();

    try testing.expectEqualStrings("GCAT", rc.bases);
}

test "Subsequence extraction" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "ATGCGATCGA");
    defer seq.deinit();

    var sub = try seq.subsequence(2, 6);
    defer sub.deinit();

    try testing.expectEqualStrings("GCGA", sub.bases);
}

test "Pattern finding" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "ATGATGATG");
    defer seq.deinit();

    try testing.expect(seq.contains("ATG"));
    try testing.expect(!seq.contains("CCC"));

    const positions = try seq.findAll(allocator, "ATG");
    defer allocator.free(positions);

    try testing.expectEqual(@as(usize, 3), positions.len);
    try testing.expectEqual(@as(usize, 0), positions[0]);
    try testing.expectEqual(@as(usize, 3), positions[1]);
    try testing.expectEqual(@as(usize, 6), positions[2]);
}

test "Hamming distance" {
    const allocator = testing.allocator;

    var seq1 = try Sequence.init(allocator, "ATGC");
    defer seq1.deinit();

    var seq2 = try Sequence.init(allocator, "ATCC");
    defer seq2.deinit();

    const dist = seq1.hammingDistance(seq2);
    try testing.expectEqual(@as(?usize, 1), dist);
}

test "Base counts" {
    const allocator = testing.allocator;

    var seq = try Sequence.init(allocator, "AATTGGCCNN");
    defer seq.deinit();

    const counts = seq.baseCounts();
    try testing.expectEqual(@as(usize, 2), counts.a);
    try testing.expectEqual(@as(usize, 2), counts.t);
    try testing.expectEqual(@as(usize, 2), counts.g);
    try testing.expectEqual(@as(usize, 2), counts.c);
    try testing.expectEqual(@as(usize, 2), counts.n);
}

test "FASTA parsing" {
    const allocator = testing.allocator;

    const fasta_data =
        \\>seq1 test sequence
        \\ATGC
        \\GATC
        \\>seq2
        \\AAAA
    ;

    const sequences = try parseFasta(allocator, fasta_data);
    defer {
        for (sequences) |*seq| {
            seq.deinit();
        }
        allocator.free(sequences);
    }

    try testing.expectEqual(@as(usize, 2), sequences.len);
    try testing.expectEqualStrings("seq1", sequences[0].id.?);
    try testing.expectEqualStrings("test sequence", sequences[0].description.?);
    try testing.expectEqualStrings("ATGCGATC", sequences[0].bases);
    try testing.expectEqualStrings("seq2", sequences[1].id.?);
    try testing.expectEqualStrings("AAAA", sequences[1].bases);
}
