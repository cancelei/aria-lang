//! Statistical functions for biological sequence analysis
//!
//! This module provides statistical tools commonly used in bioinformatics:
//! - Descriptive statistics (mean, variance, etc.)
//! - Distribution functions
//! - Statistical tests
//! - Sequence statistics

const std = @import("std");
const Allocator = std.mem.Allocator;
const math = std.math;
const Sequence = @import("sequence").Sequence;

/// Statistics errors
pub const StatsError = error{
    /// Not enough data points
    InsufficientData,
    /// Memory allocation failed
    OutOfMemory,
    /// Invalid parameter
    InvalidParameter,
};

/// Basic descriptive statistics
pub const DescriptiveStats = struct {
    count: usize,
    sum: f64,
    mean: f64,
    variance: f64,
    std_dev: f64,
    min: f64,
    max: f64,
    median: f64,
    q1: f64,
    q3: f64,

    const Self = @This();

    /// Calculate descriptive statistics from a slice of values
    pub fn calculate(allocator: Allocator, values: []const f64) StatsError!Self {
        if (values.len == 0) {
            return StatsError.InsufficientData;
        }

        // Make a sorted copy for quantiles
        const sorted = allocator.dupe(f64, values) catch {
            return StatsError.OutOfMemory;
        };
        defer allocator.free(sorted);

        std.sort.pdq(f64, sorted, {}, struct {
            fn lessThan(_: void, a: f64, b: f64) bool {
                return a < b;
            }
        }.lessThan);

        // Calculate sum and mean
        var sum: f64 = 0.0;
        var min_val: f64 = values[0];
        var max_val: f64 = values[0];

        for (values) |v| {
            sum += v;
            min_val = @min(min_val, v);
            max_val = @max(max_val, v);
        }

        const n = @as(f64, @floatFromInt(values.len));
        const mean_val = sum / n;

        // Calculate variance
        var sum_sq_diff: f64 = 0.0;
        for (values) |v| {
            const diff = v - mean_val;
            sum_sq_diff += diff * diff;
        }

        const variance = if (values.len > 1)
            sum_sq_diff / (n - 1.0)
        else
            0.0;

        // Calculate quantiles
        const median_val = quantile(sorted, 0.5);
        const q1_val = quantile(sorted, 0.25);
        const q3_val = quantile(sorted, 0.75);

        return Self{
            .count = values.len,
            .sum = sum,
            .mean = mean_val,
            .variance = variance,
            .std_dev = @sqrt(variance),
            .min = min_val,
            .max = max_val,
            .median = median_val,
            .q1 = q1_val,
            .q3 = q3_val,
        };
    }

    /// Calculate interquartile range
    pub fn iqr(self: Self) f64 {
        return self.q3 - self.q1;
    }

    /// Calculate coefficient of variation
    pub fn cv(self: Self) f64 {
        if (self.mean == 0.0) return 0.0;
        return self.std_dev / self.mean;
    }

    /// Calculate range
    pub fn range(self: Self) f64 {
        return self.max - self.min;
    }
};

/// Calculate quantile from sorted data
fn quantile(sorted: []const f64, p: f64) f64 {
    if (sorted.len == 0) return 0.0;
    if (sorted.len == 1) return sorted[0];

    const index = p * @as(f64, @floatFromInt(sorted.len - 1));
    const lower = @as(usize, @intFromFloat(@floor(index)));
    const upper = @as(usize, @intFromFloat(@ceil(index)));
    const frac = index - @floor(index);

    if (lower == upper or upper >= sorted.len) {
        return sorted[lower];
    }

    return sorted[lower] * (1.0 - frac) + sorted[upper] * frac;
}

/// Running statistics calculator (online algorithm)
pub const RunningStats = struct {
    count: usize,
    mean: f64,
    m2: f64, // Sum of squared differences
    min: f64,
    max: f64,

    const Self = @This();

    pub fn init() Self {
        return Self{
            .count = 0,
            .mean = 0.0,
            .m2 = 0.0,
            .min = math.inf(f64),
            .max = -math.inf(f64),
        };
    }

    /// Add a value using Welford's online algorithm
    pub fn add(self: *Self, value: f64) void {
        self.count += 1;
        const delta = value - self.mean;
        self.mean += delta / @as(f64, @floatFromInt(self.count));
        const delta2 = value - self.mean;
        self.m2 += delta * delta2;
        self.min = @min(self.min, value);
        self.max = @max(self.max, value);
    }

    /// Get current variance
    pub fn variance(self: Self) f64 {
        if (self.count < 2) return 0.0;
        return self.m2 / @as(f64, @floatFromInt(self.count - 1));
    }

    /// Get current standard deviation
    pub fn stdDev(self: Self) f64 {
        return @sqrt(self.variance());
    }

    /// Merge another RunningStats into this one
    pub fn merge(self: *Self, other: Self) void {
        if (other.count == 0) return;
        if (self.count == 0) {
            self.* = other;
            return;
        }

        const total_count = self.count + other.count;
        const delta = other.mean - self.mean;

        const new_mean = self.mean + delta * @as(f64, @floatFromInt(other.count)) / @as(f64, @floatFromInt(total_count));

        const new_m2 = self.m2 + other.m2 +
            delta * delta * @as(f64, @floatFromInt(self.count)) * @as(f64, @floatFromInt(other.count)) / @as(f64, @floatFromInt(total_count));

        self.count = total_count;
        self.mean = new_mean;
        self.m2 = new_m2;
        self.min = @min(self.min, other.min);
        self.max = @max(self.max, other.max);
    }
};

/// Histogram for discrete distributions
pub const Histogram = struct {
    bins: []usize,
    bin_edges: []f64,
    allocator: Allocator,

    const Self = @This();

    /// Create a histogram with uniform bins
    pub fn init(allocator: Allocator, num_bins: usize, min_val: f64, max_val: f64) StatsError!Self {
        if (num_bins == 0) {
            return StatsError.InvalidParameter;
        }

        const bins = allocator.alloc(usize, num_bins) catch {
            return StatsError.OutOfMemory;
        };
        errdefer allocator.free(bins);
        @memset(bins, 0);

        const bin_edges = allocator.alloc(f64, num_bins + 1) catch {
            return StatsError.OutOfMemory;
        };

        const step = (max_val - min_val) / @as(f64, @floatFromInt(num_bins));
        var i: usize = 0;
        while (i <= num_bins) : (i += 1) {
            bin_edges[i] = min_val + step * @as(f64, @floatFromInt(i));
        }

        return Self{
            .bins = bins,
            .bin_edges = bin_edges,
            .allocator = allocator,
        };
    }

    pub fn deinit(self: *Self) void {
        self.allocator.free(self.bins);
        self.allocator.free(self.bin_edges);
        self.* = undefined;
    }

    /// Add a value to the histogram
    pub fn add(self: *Self, value: f64) void {
        const min_val = self.bin_edges[0];
        const max_val = self.bin_edges[self.bin_edges.len - 1];

        if (value < min_val or value > max_val) return;

        const num_bins = self.bins.len;
        const step = (max_val - min_val) / @as(f64, @floatFromInt(num_bins));

        var bin_idx = @as(usize, @intFromFloat((value - min_val) / step));
        if (bin_idx >= num_bins) bin_idx = num_bins - 1;

        self.bins[bin_idx] += 1;
    }

    /// Get total count
    pub fn totalCount(self: Self) usize {
        var total: usize = 0;
        for (self.bins) |count| {
            total += count;
        }
        return total;
    }

    /// Get normalized frequencies
    pub fn frequencies(self: Self, allocator: Allocator) StatsError![]f64 {
        const total = @as(f64, @floatFromInt(self.totalCount()));
        if (total == 0.0) {
            return StatsError.InsufficientData;
        }

        const freqs = allocator.alloc(f64, self.bins.len) catch {
            return StatsError.OutOfMemory;
        };

        for (self.bins, 0..) |count, i| {
            freqs[i] = @as(f64, @floatFromInt(count)) / total;
        }

        return freqs;
    }
};

/// Sequence-specific statistics
pub const SequenceStats = struct {
    /// Calculate GC content for multiple sequences
    pub fn meanGcContent(sequences: []const Sequence) f64 {
        if (sequences.len == 0) return 0.0;

        var total_gc: f64 = 0.0;
        for (sequences) |seq| {
            total_gc += seq.gcContent();
        }
        return total_gc / @as(f64, @floatFromInt(sequences.len));
    }

    /// Calculate length statistics for multiple sequences
    pub fn lengthStats(allocator: Allocator, sequences: []const Sequence) StatsError!DescriptiveStats {
        if (sequences.len == 0) {
            return StatsError.InsufficientData;
        }

        const lengths = allocator.alloc(f64, sequences.len) catch {
            return StatsError.OutOfMemory;
        };
        defer allocator.free(lengths);

        for (sequences, 0..) |seq, i| {
            lengths[i] = @as(f64, @floatFromInt(seq.len()));
        }

        return DescriptiveStats.calculate(allocator, lengths);
    }

    /// Calculate N50 (weighted median of sequence lengths)
    pub fn n50(allocator: Allocator, sequences: []const Sequence) StatsError!usize {
        if (sequences.len == 0) {
            return StatsError.InsufficientData;
        }

        // Get lengths and sort descending
        const lengths = allocator.alloc(usize, sequences.len) catch {
            return StatsError.OutOfMemory;
        };
        defer allocator.free(lengths);

        var total_length: usize = 0;
        for (sequences, 0..) |seq, i| {
            lengths[i] = seq.len();
            total_length += seq.len();
        }

        std.sort.pdq(usize, lengths, {}, struct {
            fn cmp(_: void, a: usize, b: usize) bool {
                return a > b; // Descending
            }
        }.cmp);

        const half_total = total_length / 2;
        var cumulative: usize = 0;

        for (lengths) |len| {
            cumulative += len;
            if (cumulative >= half_total) {
                return len;
            }
        }

        return lengths[lengths.len - 1];
    }

    /// Calculate L50 (number of sequences comprising N50)
    pub fn l50(allocator: Allocator, sequences: []const Sequence) StatsError!usize {
        if (sequences.len == 0) {
            return StatsError.InsufficientData;
        }

        const lengths = allocator.alloc(usize, sequences.len) catch {
            return StatsError.OutOfMemory;
        };
        defer allocator.free(lengths);

        var total_length: usize = 0;
        for (sequences, 0..) |seq, i| {
            lengths[i] = seq.len();
            total_length += seq.len();
        }

        std.sort.pdq(usize, lengths, {}, struct {
            fn cmp(_: void, a: usize, b: usize) bool {
                return a > b;
            }
        }.cmp);

        const half_total = total_length / 2;
        var cumulative: usize = 0;

        for (lengths, 0..) |len, i| {
            cumulative += len;
            if (cumulative >= half_total) {
                return i + 1;
            }
        }

        return sequences.len;
    }
};

/// Shannon entropy calculation
pub fn entropy(probabilities: []const f64) f64 {
    var h: f64 = 0.0;
    for (probabilities) |p| {
        if (p > 0.0) {
            h -= p * @log(p) / @log(2.0);
        }
    }
    return h;
}

/// Normalized entropy (0-1 scale)
pub fn normalizedEntropy(probabilities: []const f64) f64 {
    if (probabilities.len <= 1) return 0.0;

    const max_entropy = @log(@as(f64, @floatFromInt(probabilities.len))) / @log(2.0);
    if (max_entropy == 0.0) return 0.0;

    return entropy(probabilities) / max_entropy;
}

/// Pearson correlation coefficient
pub fn correlation(x: []const f64, y: []const f64) StatsError!f64 {
    if (x.len != y.len or x.len < 2) {
        return StatsError.InsufficientData;
    }

    const n = @as(f64, @floatFromInt(x.len));

    var sum_x: f64 = 0.0;
    var sum_y: f64 = 0.0;
    for (x, y) |xi, yi| {
        sum_x += xi;
        sum_y += yi;
    }

    const mean_x = sum_x / n;
    const mean_y = sum_y / n;

    var sum_xy: f64 = 0.0;
    var sum_xx: f64 = 0.0;
    var sum_yy: f64 = 0.0;

    for (x, y) |xi, yi| {
        const dx = xi - mean_x;
        const dy = yi - mean_y;
        sum_xy += dx * dy;
        sum_xx += dx * dx;
        sum_yy += dy * dy;
    }

    const denominator = @sqrt(sum_xx * sum_yy);
    if (denominator == 0.0) return 0.0;

    return sum_xy / denominator;
}

/// Euclidean distance between two vectors
pub fn euclideanDistance(x: []const f64, y: []const f64) StatsError!f64 {
    if (x.len != y.len) {
        return StatsError.InvalidParameter;
    }

    var sum_sq: f64 = 0.0;
    for (x, y) |xi, yi| {
        const diff = xi - yi;
        sum_sq += diff * diff;
    }

    return @sqrt(sum_sq);
}

/// Manhattan distance between two vectors
pub fn manhattanDistance(x: []const f64, y: []const f64) StatsError!f64 {
    if (x.len != y.len) {
        return StatsError.InvalidParameter;
    }

    var sum: f64 = 0.0;
    for (x, y) |xi, yi| {
        sum += @abs(xi - yi);
    }

    return sum;
}

/// Cosine similarity between two vectors
pub fn cosineSimilarity(x: []const f64, y: []const f64) StatsError!f64 {
    if (x.len != y.len) {
        return StatsError.InvalidParameter;
    }

    var dot_product: f64 = 0.0;
    var norm_x: f64 = 0.0;
    var norm_y: f64 = 0.0;

    for (x, y) |xi, yi| {
        dot_product += xi * yi;
        norm_x += xi * xi;
        norm_y += yi * yi;
    }

    const denominator = @sqrt(norm_x) * @sqrt(norm_y);
    if (denominator == 0.0) return 0.0;

    return dot_product / denominator;
}

// Tests
const testing = std.testing;

test "DescriptiveStats calculation" {
    const allocator = testing.allocator;

    const values = [_]f64{ 1.0, 2.0, 3.0, 4.0, 5.0 };
    const stats = try DescriptiveStats.calculate(allocator, &values);

    try testing.expectEqual(@as(usize, 5), stats.count);
    try testing.expectApproxEqAbs(@as(f64, 15.0), stats.sum, 0.0001);
    try testing.expectApproxEqAbs(@as(f64, 3.0), stats.mean, 0.0001);
    try testing.expectApproxEqAbs(@as(f64, 2.5), stats.variance, 0.0001);
    try testing.expectApproxEqAbs(@as(f64, 1.0), stats.min, 0.0001);
    try testing.expectApproxEqAbs(@as(f64, 5.0), stats.max, 0.0001);
    try testing.expectApproxEqAbs(@as(f64, 3.0), stats.median, 0.0001);
}

test "RunningStats online calculation" {
    var rs = RunningStats.init();

    rs.add(1.0);
    rs.add(2.0);
    rs.add(3.0);
    rs.add(4.0);
    rs.add(5.0);

    try testing.expectEqual(@as(usize, 5), rs.count);
    try testing.expectApproxEqAbs(@as(f64, 3.0), rs.mean, 0.0001);
    try testing.expectApproxEqAbs(@as(f64, 2.5), rs.variance(), 0.0001);
    try testing.expectApproxEqAbs(@as(f64, 1.0), rs.min, 0.0001);
    try testing.expectApproxEqAbs(@as(f64, 5.0), rs.max, 0.0001);
}

test "RunningStats merge" {
    var rs1 = RunningStats.init();
    rs1.add(1.0);
    rs1.add(2.0);

    var rs2 = RunningStats.init();
    rs2.add(3.0);
    rs2.add(4.0);
    rs2.add(5.0);

    rs1.merge(rs2);

    try testing.expectEqual(@as(usize, 5), rs1.count);
    try testing.expectApproxEqAbs(@as(f64, 3.0), rs1.mean, 0.0001);
}

test "Histogram" {
    const allocator = testing.allocator;

    var hist = try Histogram.init(allocator, 5, 0.0, 10.0);
    defer hist.deinit();

    hist.add(1.0); // bin 0
    hist.add(3.0); // bin 1
    hist.add(5.0); // bin 2
    hist.add(7.0); // bin 3
    hist.add(9.0); // bin 4

    try testing.expectEqual(@as(usize, 5), hist.totalCount());
    try testing.expectEqual(@as(usize, 1), hist.bins[0]);
    try testing.expectEqual(@as(usize, 1), hist.bins[2]);
}

test "Entropy calculation" {
    // Uniform distribution has maximum entropy
    const uniform = [_]f64{ 0.25, 0.25, 0.25, 0.25 };
    const max_h = entropy(&uniform);
    try testing.expectApproxEqAbs(@as(f64, 2.0), max_h, 0.0001);

    // Degenerate distribution has zero entropy
    const degenerate = [_]f64{ 1.0, 0.0, 0.0, 0.0 };
    const min_h = entropy(&degenerate);
    try testing.expectApproxEqAbs(@as(f64, 0.0), min_h, 0.0001);
}

test "Correlation coefficient" {
    const x = [_]f64{ 1.0, 2.0, 3.0, 4.0, 5.0 };
    const y = [_]f64{ 2.0, 4.0, 6.0, 8.0, 10.0 };

    const r = try correlation(&x, &y);
    try testing.expectApproxEqAbs(@as(f64, 1.0), r, 0.0001); // Perfect correlation

    const z = [_]f64{ 5.0, 4.0, 3.0, 2.0, 1.0 };
    const r_neg = try correlation(&x, &z);
    try testing.expectApproxEqAbs(@as(f64, -1.0), r_neg, 0.0001); // Perfect negative correlation
}

test "Distance functions" {
    const x = [_]f64{ 0.0, 0.0 };
    const y = [_]f64{ 3.0, 4.0 };

    const euc = try euclideanDistance(&x, &y);
    try testing.expectApproxEqAbs(@as(f64, 5.0), euc, 0.0001); // 3-4-5 triangle

    const man = try manhattanDistance(&x, &y);
    try testing.expectApproxEqAbs(@as(f64, 7.0), man, 0.0001); // 3 + 4
}

test "Cosine similarity" {
    const x = [_]f64{ 1.0, 0.0 };
    const y = [_]f64{ 0.0, 1.0 };

    const sim_orthogonal = try cosineSimilarity(&x, &y);
    try testing.expectApproxEqAbs(@as(f64, 0.0), sim_orthogonal, 0.0001);

    const z = [_]f64{ 2.0, 0.0 };
    const sim_parallel = try cosineSimilarity(&x, &z);
    try testing.expectApproxEqAbs(@as(f64, 1.0), sim_parallel, 0.0001);
}
