//! Quality score handling for sequencing data
//!
//! This module handles Phred quality scores commonly used in FASTQ files.
//! Phred scores represent the probability of a base call being incorrect:
//! Q = -10 * log10(P_error)
//!
//! Common encodings:
//! - Sanger/Illumina 1.8+: ASCII 33-126 (Phred+33)
//! - Illumina 1.3-1.7: ASCII 64-126 (Phred+64)

const std = @import("std");
const Allocator = std.mem.Allocator;
const math = std.math;

/// Quality score encoding types
pub const QualityEncoding = enum {
    /// Sanger/Illumina 1.8+ (ASCII 33-126, Phred 0-93)
    Phred33,
    /// Illumina 1.3-1.7 (ASCII 64-126, Phred 0-62)
    Phred64,
    /// Solexa (ASCII 59-126, Phred -5 to 62)
    Solexa,
};

/// Quality score errors
pub const QualityError = error{
    /// Quality string is empty
    EmptyQuality,
    /// Invalid quality character for the encoding
    InvalidQualityChar,
    /// Quality and sequence lengths don't match
    LengthMismatch,
    /// Memory allocation failed
    OutOfMemory,
};

/// Quality scores for a sequence
pub const QualityScores = struct {
    /// Raw quality string (ASCII encoded)
    raw: []u8,
    /// Decoded Phred scores
    scores: []u8,
    /// Encoding used
    encoding: QualityEncoding,
    /// Allocator
    allocator: Allocator,

    const Self = @This();

    /// Initialize from a quality string
    pub fn init(allocator: Allocator, quality_str: []const u8, encoding: QualityEncoding) QualityError!Self {
        if (quality_str.len == 0) {
            return QualityError.EmptyQuality;
        }

        const raw = allocator.dupe(u8, quality_str) catch {
            return QualityError.OutOfMemory;
        };
        errdefer allocator.free(raw);

        const scores = allocator.alloc(u8, quality_str.len) catch {
            return QualityError.OutOfMemory;
        };
        errdefer allocator.free(scores);

        const offset: u8 = switch (encoding) {
            .Phred33 => 33,
            .Phred64 => 64,
            .Solexa => 64,
        };

        for (quality_str, 0..) |c, i| {
            if (c < offset) {
                allocator.free(raw);
                allocator.free(scores);
                return QualityError.InvalidQualityChar;
            }
            scores[i] = c - offset;
        }

        return Self{
            .raw = raw,
            .scores = scores,
            .encoding = encoding,
            .allocator = allocator,
        };
    }

    /// Initialize with automatic encoding detection
    pub fn initAutoDetect(allocator: Allocator, quality_str: []const u8) QualityError!Self {
        if (quality_str.len == 0) {
            return QualityError.EmptyQuality;
        }

        // Detect encoding based on character range
        var min_char: u8 = 255;
        var max_char: u8 = 0;

        for (quality_str) |c| {
            min_char = @min(min_char, c);
            max_char = @max(max_char, c);
        }

        const encoding: QualityEncoding = if (min_char < 59)
            .Phred33
        else if (min_char < 64)
            .Solexa
        else
            .Phred64;

        return init(allocator, quality_str, encoding);
    }

    /// Free memory
    pub fn deinit(self: *Self) void {
        self.allocator.free(self.raw);
        self.allocator.free(self.scores);
        self.* = undefined;
    }

    /// Get the length
    pub fn len(self: Self) usize {
        return self.scores.len;
    }

    /// Get a single quality score
    pub fn get(self: Self, index: usize) ?u8 {
        if (index >= self.scores.len) return null;
        return self.scores[index];
    }

    /// Calculate mean quality score
    pub fn mean(self: Self) f64 {
        if (self.scores.len == 0) return 0.0;

        var sum: usize = 0;
        for (self.scores) |q| {
            sum += q;
        }
        return @as(f64, @floatFromInt(sum)) / @as(f64, @floatFromInt(self.scores.len));
    }

    /// Calculate median quality score
    pub fn median(self: Self, allocator: Allocator) QualityError!f64 {
        if (self.scores.len == 0) return 0.0;

        const sorted = allocator.dupe(u8, self.scores) catch {
            return QualityError.OutOfMemory;
        };
        defer allocator.free(sorted);

        std.sort.pdq(u8, sorted, {}, struct {
            fn lessThan(_: void, a: u8, b: u8) bool {
                return a < b;
            }
        }.lessThan);

        const mid = sorted.len / 2;
        if (sorted.len % 2 == 0) {
            return (@as(f64, @floatFromInt(sorted[mid - 1])) + @as(f64, @floatFromInt(sorted[mid]))) / 2.0;
        } else {
            return @as(f64, @floatFromInt(sorted[mid]));
        }
    }

    /// Calculate minimum quality score
    pub fn min(self: Self) u8 {
        if (self.scores.len == 0) return 0;

        var min_val: u8 = 255;
        for (self.scores) |q| {
            min_val = @min(min_val, q);
        }
        return min_val;
    }

    /// Calculate maximum quality score
    pub fn max(self: Self) u8 {
        if (self.scores.len == 0) return 0;

        var max_val: u8 = 0;
        for (self.scores) |q| {
            max_val = @max(max_val, q);
        }
        return max_val;
    }

    /// Convert Phred score to error probability
    pub fn phredToProb(phred: u8) f64 {
        return math.pow(f64, 10.0, -@as(f64, @floatFromInt(phred)) / 10.0);
    }

    /// Convert error probability to Phred score
    pub fn probToPhred(prob: f64) u8 {
        if (prob <= 0.0) return 93; // Max Phred
        if (prob >= 1.0) return 0;
        const phred = -10.0 * @log10(prob);
        return @intFromFloat(@min(93.0, @max(0.0, phred)));
    }

    /// Calculate the probability of the read being error-free
    pub fn errorFreeProb(self: Self) f64 {
        var prob: f64 = 1.0;
        for (self.scores) |q| {
            prob *= (1.0 - phredToProb(q));
        }
        return prob;
    }

    /// Calculate expected number of errors
    pub fn expectedErrors(self: Self) f64 {
        var errors: f64 = 0.0;
        for (self.scores) |q| {
            errors += phredToProb(q);
        }
        return errors;
    }

    /// Count bases with quality >= threshold
    pub fn countAboveThreshold(self: Self, threshold: u8) usize {
        var count: usize = 0;
        for (self.scores) |q| {
            if (q >= threshold) count += 1;
        }
        return count;
    }

    /// Calculate fraction of bases with quality >= threshold
    pub fn fractionAboveThreshold(self: Self, threshold: u8) f64 {
        if (self.scores.len == 0) return 0.0;
        return @as(f64, @floatFromInt(self.countAboveThreshold(threshold))) /
            @as(f64, @floatFromInt(self.scores.len));
    }

    /// Trim low-quality bases from the end (returns new QualityScores)
    pub fn trimEnd(self: Self, threshold: u8, allocator: Allocator) QualityError!Self {
        var end = self.scores.len;
        while (end > 0 and self.scores[end - 1] < threshold) {
            end -= 1;
        }

        if (end == 0) {
            return QualityError.EmptyQuality;
        }

        const new_raw = allocator.dupe(u8, self.raw[0..end]) catch {
            return QualityError.OutOfMemory;
        };
        errdefer allocator.free(new_raw);

        const new_scores = allocator.dupe(u8, self.scores[0..end]) catch {
            return QualityError.OutOfMemory;
        };

        return Self{
            .raw = new_raw,
            .scores = new_scores,
            .encoding = self.encoding,
            .allocator = allocator,
        };
    }

    /// Trim low-quality bases from the start
    pub fn trimStart(self: Self, threshold: u8, allocator: Allocator) QualityError!Self {
        var start: usize = 0;
        while (start < self.scores.len and self.scores[start] < threshold) {
            start += 1;
        }

        if (start >= self.scores.len) {
            return QualityError.EmptyQuality;
        }

        const new_raw = allocator.dupe(u8, self.raw[start..]) catch {
            return QualityError.OutOfMemory;
        };
        errdefer allocator.free(new_raw);

        const new_scores = allocator.dupe(u8, self.scores[start..]) catch {
            return QualityError.OutOfMemory;
        };

        return Self{
            .raw = new_raw,
            .scores = new_scores,
            .encoding = self.encoding,
            .allocator = allocator,
        };
    }

    /// Sliding window quality trimming
    pub fn trimSlidingWindow(
        self: Self,
        window_size: usize,
        threshold: f64,
        allocator: Allocator,
    ) QualityError!Self {
        if (window_size == 0 or window_size > self.scores.len) {
            return QualityError.EmptyQuality;
        }

        var end = self.scores.len;

        // Find position where window average drops below threshold
        var i: usize = 0;
        while (i + window_size <= self.scores.len) : (i += 1) {
            var sum: usize = 0;
            for (self.scores[i .. i + window_size]) |q| {
                sum += q;
            }
            const avg = @as(f64, @floatFromInt(sum)) / @as(f64, @floatFromInt(window_size));

            if (avg < threshold) {
                end = i;
                break;
            }
        }

        if (end == 0) {
            return QualityError.EmptyQuality;
        }

        const new_raw = allocator.dupe(u8, self.raw[0..end]) catch {
            return QualityError.OutOfMemory;
        };
        errdefer allocator.free(new_raw);

        const new_scores = allocator.dupe(u8, self.scores[0..end]) catch {
            return QualityError.OutOfMemory;
        };

        return Self{
            .raw = new_raw,
            .scores = new_scores,
            .encoding = self.encoding,
            .allocator = allocator,
        };
    }

    /// Get quality distribution (histogram)
    pub fn distribution(self: Self) [94]usize {
        var dist: [94]usize = [_]usize{0} ** 94;
        for (self.scores) |q| {
            if (q < 94) {
                dist[q] += 1;
            }
        }
        return dist;
    }

    /// Convert to a different encoding
    pub fn convertEncoding(self: Self, new_encoding: QualityEncoding, allocator: Allocator) QualityError!Self {
        const new_offset: u8 = switch (new_encoding) {
            .Phred33 => 33,
            .Phred64 => 64,
            .Solexa => 64,
        };

        const new_raw = allocator.alloc(u8, self.scores.len) catch {
            return QualityError.OutOfMemory;
        };
        errdefer allocator.free(new_raw);

        for (self.scores, 0..) |q, i| {
            new_raw[i] = q + new_offset;
        }

        const new_scores = allocator.dupe(u8, self.scores) catch {
            return QualityError.OutOfMemory;
        };

        return Self{
            .raw = new_raw,
            .scores = new_scores,
            .encoding = new_encoding,
            .allocator = allocator,
        };
    }
};

/// FASTQ record
pub const FastqRecord = struct {
    /// Sequence identifier
    id: []u8,
    /// Sequence bases
    bases: []u8,
    /// Quality scores
    quality: QualityScores,
    /// Optional description
    description: ?[]u8,
    /// Allocator
    allocator: Allocator,

    const Self = @This();

    pub fn deinit(self: *Self) void {
        self.allocator.free(self.id);
        self.allocator.free(self.bases);
        self.quality.deinit();
        if (self.description) |desc| {
            self.allocator.free(desc);
        }
        self.* = undefined;
    }

    /// Get the length of the record
    pub fn len(self: Self) usize {
        return self.bases.len;
    }

    /// Calculate mean quality
    pub fn meanQuality(self: Self) f64 {
        return self.quality.mean();
    }
};

/// Parse FASTQ format data
pub fn parseFastq(
    allocator: Allocator,
    data: []const u8,
    encoding: QualityEncoding,
) QualityError![]FastqRecord {
    var records = std.ArrayList(FastqRecord).init(allocator);
    errdefer {
        for (records.items) |*rec| {
            rec.deinit();
        }
        records.deinit();
    }

    var lines = std.mem.splitSequence(u8, data, "\n");
    var line_num: usize = 0;

    while (true) {
        // Line 1: @ID description
        const header_line = lines.next() orelse break;
        const header = std.mem.trim(u8, header_line, " \t\r");
        if (header.len == 0) continue;
        if (header[0] != '@') continue;

        line_num += 1;

        // Parse ID and description
        const header_content = header[1..];
        var parts = std.mem.splitScalar(u8, header_content, ' ');
        const id_part = parts.next() orelse continue;
        const id = allocator.dupe(u8, id_part) catch {
            return QualityError.OutOfMemory;
        };
        errdefer allocator.free(id);

        const rest = parts.rest();
        var description: ?[]u8 = null;
        if (rest.len > 0) {
            description = allocator.dupe(u8, rest) catch {
                return QualityError.OutOfMemory;
            };
        }
        errdefer if (description) |d| allocator.free(d);

        // Line 2: Sequence
        const seq_line = lines.next() orelse {
            allocator.free(id);
            if (description) |d| allocator.free(d);
            break;
        };
        const bases = allocator.dupe(u8, std.mem.trim(u8, seq_line, " \t\r")) catch {
            return QualityError.OutOfMemory;
        };
        errdefer allocator.free(bases);

        // Line 3: + (separator)
        _ = lines.next(); // Skip the + line

        // Line 4: Quality
        const qual_line = lines.next() orelse {
            allocator.free(id);
            if (description) |d| allocator.free(d);
            allocator.free(bases);
            break;
        };
        const qual_str = std.mem.trim(u8, qual_line, " \t\r");

        if (qual_str.len != bases.len) {
            allocator.free(id);
            if (description) |d| allocator.free(d);
            allocator.free(bases);
            return QualityError.LengthMismatch;
        }

        const quality = QualityScores.init(allocator, qual_str, encoding) catch |err| {
            allocator.free(id);
            if (description) |d| allocator.free(d);
            allocator.free(bases);
            return err;
        };

        records.append(.{
            .id = id,
            .bases = bases,
            .quality = quality,
            .description = description,
            .allocator = allocator,
        }) catch {
            allocator.free(id);
            if (description) |d| allocator.free(d);
            allocator.free(bases);
            return QualityError.OutOfMemory;
        };
    }

    return records.toOwnedSlice() catch {
        return QualityError.OutOfMemory;
    };
}

// Tests
const testing = std.testing;

test "QualityScores initialization" {
    const allocator = testing.allocator;

    // Phred33: '!' = 33 -> Q0, 'I' = 73 -> Q40
    var qs = try QualityScores.init(allocator, "!!!!IIII", .Phred33);
    defer qs.deinit();

    try testing.expectEqual(@as(usize, 8), qs.len());
    try testing.expectEqual(@as(u8, 0), qs.scores[0]);
    try testing.expectEqual(@as(u8, 40), qs.scores[4]);
}

test "QualityScores mean" {
    const allocator = testing.allocator;

    // All Q20 (ASCII 53 in Phred33)
    var qs = try QualityScores.init(allocator, "55555555", .Phred33);
    defer qs.deinit();

    const m = qs.mean();
    try testing.expectApproxEqAbs(@as(f64, 20.0), m, 0.0001);
}

test "QualityScores phred to probability" {
    // Q10 = 10% error rate
    const prob10 = QualityScores.phredToProb(10);
    try testing.expectApproxEqAbs(@as(f64, 0.1), prob10, 0.0001);

    // Q20 = 1% error rate
    const prob20 = QualityScores.phredToProb(20);
    try testing.expectApproxEqAbs(@as(f64, 0.01), prob20, 0.0001);

    // Q30 = 0.1% error rate
    const prob30 = QualityScores.phredToProb(30);
    try testing.expectApproxEqAbs(@as(f64, 0.001), prob30, 0.0001);
}

test "QualityScores probability to phred" {
    try testing.expectEqual(@as(u8, 10), QualityScores.probToPhred(0.1));
    try testing.expectEqual(@as(u8, 20), QualityScores.probToPhred(0.01));
    try testing.expectEqual(@as(u8, 30), QualityScores.probToPhred(0.001));
}

test "QualityScores expected errors" {
    const allocator = testing.allocator;

    // 10 bases at Q10 = 10 * 0.1 = 1 expected error
    var qs = try QualityScores.init(allocator, "++++++++++", .Phred33); // Q10
    defer qs.deinit();

    const ee = qs.expectedErrors();
    try testing.expectApproxEqAbs(@as(f64, 1.0), ee, 0.01);
}

test "QualityScores trimming" {
    const allocator = testing.allocator;

    // High quality at start, low at end
    var qs = try QualityScores.init(allocator, "IIII!!!!", .Phred33);
    defer qs.deinit();

    var trimmed = try qs.trimEnd(20, allocator);
    defer trimmed.deinit();

    try testing.expectEqual(@as(usize, 4), trimmed.len());
}

test "QualityScores distribution" {
    const allocator = testing.allocator;

    var qs = try QualityScores.init(allocator, "!!555III", .Phred33);
    defer qs.deinit();

    const dist = qs.distribution();
    try testing.expectEqual(@as(usize, 2), dist[0]); // Q0
    try testing.expectEqual(@as(usize, 3), dist[20]); // Q20
    try testing.expectEqual(@as(usize, 3), dist[40]); // Q40
}

test "FASTQ parsing" {
    const allocator = testing.allocator;

    const fastq_data =
        \\@read1 description here
        \\ACGT
        \\+
        \\IIII
        \\@read2
        \\AAAA
        \\+
        \\!!!!
    ;

    const records = try parseFastq(allocator, fastq_data, .Phred33);
    defer {
        for (records) |*rec| {
            var r = rec;
            r.deinit();
        }
        allocator.free(records);
    }

    try testing.expectEqual(@as(usize, 2), records.len);
    try testing.expectEqualStrings("read1", records[0].id);
    try testing.expectEqualStrings("description here", records[0].description.?);
    try testing.expectEqualStrings("ACGT", records[0].bases);
    try testing.expectEqual(@as(u8, 40), records[0].quality.scores[0]);

    try testing.expectEqualStrings("read2", records[1].id);
    try testing.expect(records[1].description == null);
}

test "Auto-detect encoding" {
    const allocator = testing.allocator;

    // Phred33 (low ASCII values)
    var qs33 = try QualityScores.initAutoDetect(allocator, "!\"#$%");
    defer qs33.deinit();
    try testing.expectEqual(QualityEncoding.Phred33, qs33.encoding);

    // Phred64 (high ASCII values only)
    var qs64 = try QualityScores.initAutoDetect(allocator, "efghi");
    defer qs64.deinit();
    try testing.expectEqual(QualityEncoding.Phred64, qs64.encoding);
}
