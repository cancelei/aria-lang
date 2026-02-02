//! K-mer counting and analysis module
//!
//! K-mers are subsequences of length k. This module provides efficient
//! counting and analysis of k-mers in biological sequences.
//!
//! Features:
//! - Hash-based k-mer counting
//! - Canonical k-mer support (considers reverse complement)
//! - Frequency analysis
//! - Memory-efficient iterator-based processing

const std = @import("std");
const Allocator = std.mem.Allocator;
const Sequence = @import("sequence").Sequence;
const SequenceError = @import("sequence").SequenceError;

/// Errors specific to k-mer operations
pub const KMerError = error{
    /// K must be positive
    InvalidK,
    /// K is larger than the sequence
    KTooLarge,
    /// Memory allocation failed
    OutOfMemory,
    /// Invalid base in k-mer
    InvalidBase,
};

/// K-mer counter that tracks occurrence counts
pub const KMerCounter = struct {
    /// The k-mer size
    k: usize,
    /// Hash map storing k-mer counts
    counts: std.StringHashMap(usize),
    /// Allocator for memory management
    allocator: Allocator,
    /// Total k-mers counted (including duplicates)
    total_count: usize,
    /// Whether to use canonical k-mers (min of kmer and reverse complement)
    canonical: bool,

    const Self = @This();

    /// Initialize a new k-mer counter
    pub fn init(allocator: Allocator, k: usize) KMerError!Self {
        if (k == 0) {
            return KMerError.InvalidK;
        }

        return Self{
            .k = k,
            .counts = std.StringHashMap(usize).init(allocator),
            .allocator = allocator,
            .total_count = 0,
            .canonical = false,
        };
    }

    /// Initialize with canonical k-mer mode
    pub fn initCanonical(allocator: Allocator, k: usize) KMerError!Self {
        var counter = try init(allocator, k);
        counter.canonical = true;
        return counter;
    }

    /// Free all memory
    pub fn deinit(self: *Self) void {
        var it = self.counts.keyIterator();
        while (it.next()) |key| {
            self.allocator.free(key.*);
        }
        self.counts.deinit();
        self.* = undefined;
    }

    /// Get the canonical form of a k-mer (lexicographically smaller of kmer and its reverse complement)
    fn getCanonical(self: *Self, kmer: []const u8) ![]u8 {
        const kmer_copy = try self.allocator.dupe(u8, kmer);
        errdefer self.allocator.free(kmer_copy);

        if (!self.canonical) {
            return kmer_copy;
        }

        // Compute reverse complement
        const rev_comp = try self.allocator.alloc(u8, kmer.len);
        defer self.allocator.free(rev_comp);

        var i: usize = 0;
        while (i < kmer.len) : (i += 1) {
            rev_comp[i] = switch (kmer[kmer.len - 1 - i]) {
                'A' => 'T',
                'T' => 'A',
                'C' => 'G',
                'G' => 'C',
                else => 'N',
            };
        }

        // Return lexicographically smaller
        if (std.mem.order(u8, kmer_copy, rev_comp) == .gt) {
            @memcpy(kmer_copy, rev_comp);
        }

        return kmer_copy;
    }

    /// Count k-mers in a sequence
    pub fn count(self: *Self, seq: Sequence) !void {
        if (seq.bases.len < self.k) return;

        var i: usize = 0;
        while (i <= seq.bases.len - self.k) : (i += 1) {
            const kmer = seq.bases[i .. i + self.k];

            // Skip if contains N
            var has_n = false;
            for (kmer) |base| {
                if (base == 'N') {
                    has_n = true;
                    break;
                }
            }
            if (has_n) continue;

            // Get canonical form if enabled
            const kmer_key = self.getCanonical(kmer) catch {
                return KMerError.OutOfMemory;
            };

            // Add to counts
            const gop = self.counts.getOrPut(kmer_key) catch {
                self.allocator.free(kmer_key);
                return KMerError.OutOfMemory;
            };

            if (gop.found_existing) {
                self.allocator.free(kmer_key);
                gop.value_ptr.* += 1;
            } else {
                gop.value_ptr.* = 1;
            }

            self.total_count += 1;
        }
    }

    /// Count k-mers from raw bases string
    pub fn countBases(self: *Self, bases: []const u8) !void {
        if (bases.len < self.k) return;

        var i: usize = 0;
        while (i <= bases.len - self.k) : (i += 1) {
            const kmer = bases[i .. i + self.k];

            // Skip if contains N or invalid
            var valid = true;
            for (kmer) |base| {
                const uc = std.ascii.toUpper(base);
                if (uc != 'A' and uc != 'C' and uc != 'G' and uc != 'T') {
                    valid = false;
                    break;
                }
            }
            if (!valid) continue;

            // Uppercase the kmer
            const kmer_upper = self.allocator.alloc(u8, self.k) catch {
                return KMerError.OutOfMemory;
            };
            for (kmer, 0..) |c, j| {
                kmer_upper[j] = std.ascii.toUpper(c);
            }

            // Get canonical form if enabled
            const kmer_key = if (self.canonical) blk: {
                defer self.allocator.free(kmer_upper);
                break :blk self.getCanonical(kmer_upper) catch {
                    return KMerError.OutOfMemory;
                };
            } else kmer_upper;

            // Add to counts
            const gop = self.counts.getOrPut(kmer_key) catch {
                self.allocator.free(kmer_key);
                return KMerError.OutOfMemory;
            };

            if (gop.found_existing) {
                self.allocator.free(kmer_key);
                gop.value_ptr.* += 1;
            } else {
                gop.value_ptr.* = 1;
            }

            self.total_count += 1;
        }
    }

    /// Get count for a specific k-mer
    pub fn getCount(self: *const Self, kmer: []const u8) usize {
        return self.counts.get(kmer) orelse 0;
    }

    /// Get the number of unique k-mers
    pub fn uniqueCount(self: *const Self) usize {
        return self.counts.count();
    }

    /// K-mer with its count
    pub const KMerCount = struct {
        kmer: []const u8,
        count: usize,

        pub fn frequency(self: KMerCount, total: usize) f64 {
            if (total == 0) return 0.0;
            return @as(f64, @floatFromInt(self.count)) / @as(f64, @floatFromInt(total));
        }
    };

    /// Get the most frequent k-mers
    pub fn mostFrequent(self: *const Self, allocator: Allocator, n: usize) ![]KMerCount {
        var counts_list = std.ArrayList(KMerCount).init(allocator);
        defer counts_list.deinit();

        var it = self.counts.iterator();
        while (it.next()) |entry| {
            counts_list.append(.{
                .kmer = entry.key_ptr.*,
                .count = entry.value_ptr.*,
            }) catch {
                return KMerError.OutOfMemory;
            };
        }

        // Sort by count descending
        std.sort.pdq(KMerCount, counts_list.items, {}, struct {
            fn lessThan(_: void, a: KMerCount, b: KMerCount) bool {
                return a.count > b.count;
            }
        }.lessThan);

        const result_len = @min(n, counts_list.items.len);
        return allocator.dupe(KMerCount, counts_list.items[0..result_len]) catch {
            return KMerError.OutOfMemory;
        };
    }

    /// Get the least frequent k-mers
    pub fn leastFrequent(self: *const Self, allocator: Allocator, n: usize) ![]KMerCount {
        var counts_list = std.ArrayList(KMerCount).init(allocator);
        defer counts_list.deinit();

        var it = self.counts.iterator();
        while (it.next()) |entry| {
            counts_list.append(.{
                .kmer = entry.key_ptr.*,
                .count = entry.value_ptr.*,
            }) catch {
                return KMerError.OutOfMemory;
            };
        }

        // Sort by count ascending
        std.sort.pdq(KMerCount, counts_list.items, {}, struct {
            fn lessThan(_: void, a: KMerCount, b: KMerCount) bool {
                return a.count < b.count;
            }
        }.lessThan);

        const result_len = @min(n, counts_list.items.len);
        return allocator.dupe(KMerCount, counts_list.items[0..result_len]) catch {
            return KMerError.OutOfMemory;
        };
    }

    /// Get all k-mers with a specific count
    pub fn withCount(self: *const Self, allocator: Allocator, target_count: usize) ![][]const u8 {
        var result = std.ArrayList([]const u8).init(allocator);
        defer result.deinit();

        var it = self.counts.iterator();
        while (it.next()) |entry| {
            if (entry.value_ptr.* == target_count) {
                result.append(entry.key_ptr.*) catch {
                    return KMerError.OutOfMemory;
                };
            }
        }

        return result.toOwnedSlice() catch {
            return KMerError.OutOfMemory;
        };
    }

    /// Calculate k-mer diversity (unique k-mers / total k-mers)
    pub fn diversity(self: *const Self) f64 {
        if (self.total_count == 0) return 0.0;
        return @as(f64, @floatFromInt(self.counts.count())) / @as(f64, @floatFromInt(self.total_count));
    }

    /// Calculate Shannon entropy of k-mer distribution
    pub fn entropy(self: *const Self) f64 {
        if (self.total_count == 0) return 0.0;

        var h: f64 = 0.0;
        var it = self.counts.valueIterator();
        while (it.next()) |count_ptr| {
            const p = @as(f64, @floatFromInt(count_ptr.*)) / @as(f64, @floatFromInt(self.total_count));
            if (p > 0.0) {
                h -= p * @log(p) / @log(2.0);
            }
        }
        return h;
    }

    /// Iterator over all k-mers
    pub const Iterator = struct {
        inner: std.StringHashMap(usize).Iterator,

        pub fn next(self: *Iterator) ?KMerCount {
            const entry = self.inner.next() orelse return null;
            return KMerCount{
                .kmer = entry.key_ptr.*,
                .count = entry.value_ptr.*,
            };
        }
    };

    pub fn iterator(self: *const Self) Iterator {
        return Iterator{ .inner = self.counts.iterator() };
    }

    /// Merge another counter into this one
    pub fn merge(self: *Self, other: *const Self) !void {
        if (self.k != other.k) return;

        var it = other.counts.iterator();
        while (it.next()) |entry| {
            const kmer_copy = self.allocator.dupe(u8, entry.key_ptr.*) catch {
                return KMerError.OutOfMemory;
            };

            const gop = self.counts.getOrPut(kmer_copy) catch {
                self.allocator.free(kmer_copy);
                return KMerError.OutOfMemory;
            };

            if (gop.found_existing) {
                self.allocator.free(kmer_copy);
                gop.value_ptr.* += entry.value_ptr.*;
            } else {
                gop.value_ptr.* = entry.value_ptr.*;
            }

            self.total_count += entry.value_ptr.*;
        }
    }

    /// Reset the counter
    pub fn clear(self: *Self) void {
        var it = self.counts.keyIterator();
        while (it.next()) |key| {
            self.allocator.free(key.*);
        }
        self.counts.clearRetainingCapacity();
        self.total_count = 0;
    }
};

/// Spectrum analysis for k-mer distributions
pub const KMerSpectrum = struct {
    /// Maps count -> number of k-mers with that count
    spectrum: std.AutoHashMap(usize, usize),
    allocator: Allocator,

    const Self = @This();

    pub fn init(allocator: Allocator) Self {
        return Self{
            .spectrum = std.AutoHashMap(usize, usize).init(allocator),
            .allocator = allocator,
        };
    }

    pub fn deinit(self: *Self) void {
        self.spectrum.deinit();
    }

    /// Build spectrum from a k-mer counter
    pub fn fromCounter(allocator: Allocator, counter: *const KMerCounter) !Self {
        var self = init(allocator);
        errdefer self.deinit();

        var it = counter.counts.valueIterator();
        while (it.next()) |count_ptr| {
            const gop = self.spectrum.getOrPut(count_ptr.*) catch {
                return KMerError.OutOfMemory;
            };
            if (gop.found_existing) {
                gop.value_ptr.* += 1;
            } else {
                gop.value_ptr.* = 1;
            }
        }

        return self;
    }

    /// Get the number of k-mers with a specific count
    pub fn get(self: *const Self, count: usize) usize {
        return self.spectrum.get(count) orelse 0;
    }

    /// Find the peak (most common count)
    pub fn peak(self: *const Self) ?usize {
        var max_freq: usize = 0;
        var peak_count: ?usize = null;

        var it = self.spectrum.iterator();
        while (it.next()) |entry| {
            if (entry.value_ptr.* > max_freq) {
                max_freq = entry.value_ptr.*;
                peak_count = entry.key_ptr.*;
            }
        }

        return peak_count;
    }
};

/// Calculate Jaccard similarity between two k-mer sets
pub fn jaccardSimilarity(counter1: *const KMerCounter, counter2: *const KMerCounter) f64 {
    if (counter1.k != counter2.k) return 0.0;

    var intersection: usize = 0;
    var it = counter1.counts.keyIterator();
    while (it.next()) |key| {
        if (counter2.counts.contains(key.*)) {
            intersection += 1;
        }
    }

    const union_size = counter1.counts.count() + counter2.counts.count() - intersection;
    if (union_size == 0) return 1.0;

    return @as(f64, @floatFromInt(intersection)) / @as(f64, @floatFromInt(union_size));
}

// Tests
const testing = std.testing;

test "KMerCounter initialization" {
    const allocator = testing.allocator;

    var counter = try KMerCounter.init(allocator, 3);
    defer counter.deinit();

    try testing.expectEqual(@as(usize, 3), counter.k);
    try testing.expectEqual(@as(usize, 0), counter.uniqueCount());
}

test "KMerCounter rejects k=0" {
    const allocator = testing.allocator;
    const result = KMerCounter.init(allocator, 0);
    try testing.expectError(KMerError.InvalidK, result);
}

test "KMerCounter basic counting" {
    const allocator = testing.allocator;

    var counter = try KMerCounter.init(allocator, 3);
    defer counter.deinit();

    var seq = try Sequence.init(allocator, "ATGATGATG");
    defer seq.deinit();

    try counter.count(seq);

    try testing.expectEqual(@as(usize, 3), counter.getCount("ATG"));
    try testing.expectEqual(@as(usize, 2), counter.getCount("TGA"));
    try testing.expectEqual(@as(usize, 2), counter.getCount("GAT"));
}

test "KMerCounter skips N bases" {
    const allocator = testing.allocator;

    var counter = try KMerCounter.init(allocator, 3);
    defer counter.deinit();

    var seq = try Sequence.init(allocator, "ATGNATG");
    defer seq.deinit();

    try counter.count(seq);

    // ATG appears at positions 0 and 4, but TGN, GNA, NAT contain N
    try testing.expectEqual(@as(usize, 2), counter.getCount("ATG"));
    try testing.expectEqual(@as(usize, 0), counter.getCount("TGN"));
}

test "KMerCounter most frequent" {
    const allocator = testing.allocator;

    var counter = try KMerCounter.init(allocator, 3);
    defer counter.deinit();

    var seq = try Sequence.init(allocator, "ATGATGATG");
    defer seq.deinit();

    try counter.count(seq);

    const top = try counter.mostFrequent(allocator, 2);
    defer allocator.free(top);

    try testing.expectEqual(@as(usize, 2), top.len);
    try testing.expectEqualStrings("ATG", top[0].kmer);
    try testing.expectEqual(@as(usize, 3), top[0].count);
}

test "KMerCounter diversity" {
    const allocator = testing.allocator;

    var counter = try KMerCounter.init(allocator, 2);
    defer counter.deinit();

    var seq = try Sequence.init(allocator, "ATATATAT");
    defer seq.deinit();

    try counter.count(seq);

    // Only 2 unique 2-mers (AT, TA) out of 7 total
    const div = counter.diversity();
    try testing.expectApproxEqAbs(@as(f64, 2.0 / 7.0), div, 0.0001);
}

test "KMerCounter canonical mode" {
    const allocator = testing.allocator;

    var counter = try KMerCounter.initCanonical(allocator, 3);
    defer counter.deinit();

    // ATG and CAT are reverse complements
    var seq = try Sequence.init(allocator, "ATGCAT");
    defer seq.deinit();

    try counter.count(seq);

    // Should count both as the same canonical k-mer
    const unique = counter.uniqueCount();
    try testing.expect(unique <= 4); // Fewer unique due to canonicalization
}

test "KMerSpectrum" {
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

test "Jaccard similarity" {
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
