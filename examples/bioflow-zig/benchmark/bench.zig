//! BioFlow Zig Performance Benchmarks
//!
//! This benchmark suite measures the performance of core BioFlow operations
//! for comparison with Aria and other implementations.
//!
//! Run with: zig build bench
//!
//! Benchmarks include:
//! - Sequence operations (GC content, complement, reverse complement)
//! - K-mer counting
//! - Sequence alignment (Smith-Waterman, Needleman-Wunsch)
//! - Quality score processing

const std = @import("std");
const Sequence = @import("sequence").Sequence;
const KMerCounter = @import("kmer").KMerCounter;
const alignment = @import("alignment");
const quality = @import("quality");
const stats = @import("stats");

/// Benchmark configuration
const Config = struct {
    /// Number of warmup iterations
    warmup_iterations: usize = 10,
    /// Number of timed iterations
    timed_iterations: usize = 100,
    /// Whether to print verbose output
    verbose: bool = true,
};

/// Benchmark result
const BenchResult = struct {
    name: []const u8,
    iterations: usize,
    total_ns: i128,
    min_ns: i128,
    max_ns: i128,
    mean_ns: f64,
    std_dev_ns: f64,
    throughput: ?f64, // Operations per second or bytes per second

    pub fn print(self: BenchResult) void {
        const mean_ms = @as(f64, @floatFromInt(self.mean_ns)) / 1_000_000.0;
        const min_ms = @as(f64, @floatFromInt(self.min_ns)) / 1_000_000.0;
        const max_ms = @as(f64, @floatFromInt(self.max_ns)) / 1_000_000.0;
        const std_ms = self.std_dev_ns / 1_000_000.0;

        std.debug.print("{s:<35} {d:>10.3}ms (min: {d:.3}, max: {d:.3}, std: {d:.3})", .{
            self.name,
            mean_ms,
            min_ms,
            max_ms,
            std_ms,
        });

        if (self.throughput) |tp| {
            if (tp > 1_000_000.0) {
                std.debug.print("  [{d:.2} M ops/s]", .{tp / 1_000_000.0});
            } else if (tp > 1_000.0) {
                std.debug.print("  [{d:.2} K ops/s]", .{tp / 1_000.0});
            } else {
                std.debug.print("  [{d:.2} ops/s]", .{tp});
            }
        }

        std.debug.print("\n", .{});
    }
};

/// Run a benchmark function
fn benchmark(
    name: []const u8,
    config: Config,
    comptime func: anytype,
    args: anytype,
) BenchResult {
    // Warmup
    var i: usize = 0;
    while (i < config.warmup_iterations) : (i += 1) {
        _ = @call(.auto, func, args);
    }

    // Timed runs
    var timings = std.ArrayList(i128).init(std.heap.page_allocator);
    defer timings.deinit();

    var total_ns: i128 = 0;
    var min_ns: i128 = std.math.maxInt(i128);
    var max_ns: i128 = 0;

    i = 0;
    while (i < config.timed_iterations) : (i += 1) {
        const start = std.time.nanoTimestamp();
        _ = @call(.auto, func, args);
        const end = std.time.nanoTimestamp();
        const elapsed = end - start;

        timings.append(elapsed) catch {};
        total_ns += elapsed;
        min_ns = @min(min_ns, elapsed);
        max_ns = @max(max_ns, elapsed);
    }

    const mean_ns = @as(f64, @floatFromInt(total_ns)) / @as(f64, @floatFromInt(config.timed_iterations));

    // Calculate standard deviation
    var sum_sq_diff: f64 = 0.0;
    for (timings.items) |t| {
        const diff = @as(f64, @floatFromInt(t)) - mean_ns;
        sum_sq_diff += diff * diff;
    }
    const std_dev = @sqrt(sum_sq_diff / @as(f64, @floatFromInt(config.timed_iterations)));

    const throughput = @as(f64, @floatFromInt(config.timed_iterations)) * 1_000_000_000.0 / @as(f64, @floatFromInt(total_ns));

    return BenchResult{
        .name = name,
        .iterations = config.timed_iterations,
        .total_ns = total_ns,
        .min_ns = min_ns,
        .max_ns = max_ns,
        .mean_ns = mean_ns,
        .std_dev_ns = std_dev,
        .throughput = throughput,
    };
}

// ============================================================================
// Benchmark Functions
// ============================================================================

fn benchGcContent(seq: *Sequence) f64 {
    return seq.gcContent();
}

fn benchComplement(allocator: std.mem.Allocator, seq: *Sequence) void {
    var comp = seq.complement() catch return;
    comp.deinit();
    _ = allocator;
}

fn benchReverseComplement(allocator: std.mem.Allocator, seq: *Sequence) void {
    var rc = seq.reverseComplement() catch return;
    rc.deinit();
    _ = allocator;
}

fn benchKmerCount(allocator: std.mem.Allocator, seq: *Sequence, k: usize) void {
    var counter = KMerCounter.init(allocator, k) catch return;
    defer counter.deinit();
    counter.count(seq.*) catch return;
}

fn benchSmithWaterman(allocator: std.mem.Allocator, seq1: *Sequence, seq2: *Sequence) void {
    var align_result = alignment.smithWaterman(allocator, seq1.*, seq2.*, alignment.ScoringMatrix.default()) catch return;
    align_result.deinit();
}

fn benchNeedlemanWunsch(allocator: std.mem.Allocator, seq1: *Sequence, seq2: *Sequence) void {
    var align_result = alignment.needlemanWunsch(allocator, seq1.*, seq2.*, alignment.ScoringMatrix.default()) catch return;
    align_result.deinit();
}

fn benchEditDistance(allocator: std.mem.Allocator, s1: []const u8, s2: []const u8) void {
    _ = alignment.editDistance(s1, s2, allocator) catch return;
}

fn benchQualityMean(qs: *quality.QualityScores) f64 {
    return qs.mean();
}

fn benchQualityExpectedErrors(qs: *quality.QualityScores) f64 {
    return qs.expectedErrors();
}

fn benchBaseCounts(seq: *Sequence) Sequence.BaseCounts {
    return seq.baseCounts();
}

fn benchPatternFind(allocator: std.mem.Allocator, seq: *Sequence, pattern: []const u8) void {
    const positions = seq.findAll(allocator, pattern) catch return;
    allocator.free(positions);
}

// ============================================================================
// Main Benchmark Runner
// ============================================================================

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const config = Config{
        .warmup_iterations = 10,
        .timed_iterations = 100,
        .verbose = true,
    };

    std.debug.print("\n", .{});
    std.debug.print("=" ** 70 ++ "\n", .{});
    std.debug.print("BioFlow Zig Performance Benchmarks\n", .{});
    std.debug.print("=" ** 70 ++ "\n\n", .{});

    std.debug.print("Configuration:\n", .{});
    std.debug.print("  Warmup iterations: {d}\n", .{config.warmup_iterations});
    std.debug.print("  Timed iterations:  {d}\n", .{config.timed_iterations});
    std.debug.print("\n", .{});

    // ========================================================================
    // Sequence Operations
    // ========================================================================

    std.debug.print("-" ** 70 ++ "\n", .{});
    std.debug.print("Sequence Operations\n", .{});
    std.debug.print("-" ** 70 ++ "\n", .{});

    // Small sequence (100 bp)
    {
        const bases_100 = "ATGC" ** 25;
        var seq_100 = try Sequence.init(allocator, bases_100);
        defer seq_100.deinit();

        const result1 = benchmark("GC Content (100bp)", config, benchGcContent, .{&seq_100});
        result1.print();

        const result2 = benchmark("Base Counts (100bp)", config, benchBaseCounts, .{&seq_100});
        result2.print();

        const result3 = benchmark("Complement (100bp)", config, benchComplement, .{ allocator, &seq_100 });
        result3.print();

        const result4 = benchmark("Reverse Complement (100bp)", config, benchReverseComplement, .{ allocator, &seq_100 });
        result4.print();
    }

    std.debug.print("\n", .{});

    // Medium sequence (1kb)
    {
        const bases_1k = "ATGC" ** 250;
        var seq_1k = try Sequence.init(allocator, bases_1k);
        defer seq_1k.deinit();

        const result1 = benchmark("GC Content (1kb)", config, benchGcContent, .{&seq_1k});
        result1.print();

        const result2 = benchmark("Base Counts (1kb)", config, benchBaseCounts, .{&seq_1k});
        result2.print();

        const result3 = benchmark("Complement (1kb)", config, benchComplement, .{ allocator, &seq_1k });
        result3.print();

        const result4 = benchmark("Reverse Complement (1kb)", config, benchReverseComplement, .{ allocator, &seq_1k });
        result4.print();
    }

    std.debug.print("\n", .{});

    // Large sequence (20kb)
    {
        const bases_20k = "ATGC" ** 5000;
        var seq_20k = try Sequence.init(allocator, bases_20k);
        defer seq_20k.deinit();

        const result1 = benchmark("GC Content (20kb)", config, benchGcContent, .{&seq_20k});
        result1.print();

        const result2 = benchmark("Base Counts (20kb)", config, benchBaseCounts, .{&seq_20k});
        result2.print();

        const result3 = benchmark("Complement (20kb)", config, benchComplement, .{ allocator, &seq_20k });
        result3.print();

        const result4 = benchmark("Reverse Complement (20kb)", config, benchReverseComplement, .{ allocator, &seq_20k });
        result4.print();
    }

    std.debug.print("\n", .{});

    // ========================================================================
    // Pattern Finding
    // ========================================================================

    std.debug.print("-" ** 70 ++ "\n", .{});
    std.debug.print("Pattern Finding\n", .{});
    std.debug.print("-" ** 70 ++ "\n", .{});

    {
        const bases = "ATGCGATCGATGCGATCGATGCGATCG" ** 100;
        var seq = try Sequence.init(allocator, bases);
        defer seq.deinit();

        const result1 = benchmark("Find ATG (2.7kb)", config, benchPatternFind, .{ allocator, &seq, "ATG" });
        result1.print();

        const result2 = benchmark("Find GATCGA (2.7kb)", config, benchPatternFind, .{ allocator, &seq, "GATCGA" });
        result2.print();
    }

    std.debug.print("\n", .{});

    // ========================================================================
    // K-mer Counting
    // ========================================================================

    std.debug.print("-" ** 70 ++ "\n", .{});
    std.debug.print("K-mer Counting\n", .{});
    std.debug.print("-" ** 70 ++ "\n", .{});

    // Small k
    {
        const bases = "ATGC" ** 250;
        var seq = try Sequence.init(allocator, bases);
        defer seq.deinit();

        const result1 = benchmark("K-mer Count (k=3, 1kb)", config, benchKmerCount, .{ allocator, &seq, 3 });
        result1.print();

        const result2 = benchmark("K-mer Count (k=7, 1kb)", config, benchKmerCount, .{ allocator, &seq, 7 });
        result2.print();
    }

    // Standard k=21
    {
        const bases_5k = "ATGC" ** 1250;
        var seq_5k = try Sequence.init(allocator, bases_5k);
        defer seq_5k.deinit();

        const result = benchmark("K-mer Count (k=21, 5kb)", config, benchKmerCount, .{ allocator, &seq_5k, 21 });
        result.print();
    }

    // Large sequence
    {
        const bases_20k = "ATGC" ** 5000;
        var seq_20k = try Sequence.init(allocator, bases_20k);
        defer seq_20k.deinit();

        const result = benchmark("K-mer Count (k=21, 20kb)", config, benchKmerCount, .{ allocator, &seq_20k, 21 });
        result.print();
    }

    std.debug.print("\n", .{});

    // ========================================================================
    // Sequence Alignment
    // ========================================================================

    std.debug.print("-" ** 70 ++ "\n", .{});
    std.debug.print("Sequence Alignment\n", .{});
    std.debug.print("-" ** 70 ++ "\n", .{});

    // Small sequences
    {
        const bases1 = "ACGTACGTACGTACGT";
        const bases2 = "ACGACGTACGTACGT";

        var seq1 = try Sequence.init(allocator, bases1);
        defer seq1.deinit();

        var seq2 = try Sequence.init(allocator, bases2);
        defer seq2.deinit();

        const result1 = benchmark("Smith-Waterman (16bp x 15bp)", config, benchSmithWaterman, .{ allocator, &seq1, &seq2 });
        result1.print();

        const result2 = benchmark("Needleman-Wunsch (16bp x 15bp)", config, benchNeedlemanWunsch, .{ allocator, &seq1, &seq2 });
        result2.print();
    }

    // Medium sequences
    {
        const bases1 = "ACGT" ** 50;
        const bases2 = "AGCT" ** 50;

        var seq1 = try Sequence.init(allocator, bases1);
        defer seq1.deinit();

        var seq2 = try Sequence.init(allocator, bases2);
        defer seq2.deinit();

        const result1 = benchmark("Smith-Waterman (200bp x 200bp)", config, benchSmithWaterman, .{ allocator, &seq1, &seq2 });
        result1.print();

        const result2 = benchmark("Needleman-Wunsch (200bp x 200bp)", config, benchNeedlemanWunsch, .{ allocator, &seq1, &seq2 });
        result2.print();
    }

    // Larger sequences (reduced iterations for longer runtime)
    {
        const bases1 = "ACGT" ** 250;
        const bases2 = "AGCT" ** 250;

        var seq1 = try Sequence.init(allocator, bases1);
        defer seq1.deinit();

        var seq2 = try Sequence.init(allocator, bases2);
        defer seq2.deinit();

        const slow_config = Config{
            .warmup_iterations = 2,
            .timed_iterations = 10,
            .verbose = true,
        };

        const result = benchmark("Smith-Waterman (1kb x 1kb)", slow_config, benchSmithWaterman, .{ allocator, &seq1, &seq2 });
        result.print();
    }

    std.debug.print("\n", .{});

    // Edit distance
    {
        const s1 = "ACGT" ** 50;
        const s2 = "AGCT" ** 50;

        const result = benchmark("Edit Distance (200bp x 200bp)", config, benchEditDistance, .{ allocator, s1, s2 });
        result.print();
    }

    std.debug.print("\n", .{});

    // ========================================================================
    // Quality Score Processing
    // ========================================================================

    std.debug.print("-" ** 70 ++ "\n", .{});
    std.debug.print("Quality Score Processing\n", .{});
    std.debug.print("-" ** 70 ++ "\n", .{});

    {
        // Generate quality string (ASCII 33-73 = Phred 0-40)
        var qual_str: [1000]u8 = undefined;
        for (&qual_str, 0..) |*c, i| {
            c.* = @as(u8, @intCast(33 + (i % 41)));
        }

        var qs = try quality.QualityScores.init(allocator, &qual_str, .Phred33);
        defer qs.deinit();

        const result1 = benchmark("Quality Mean (1kb)", config, benchQualityMean, .{&qs});
        result1.print();

        const result2 = benchmark("Expected Errors (1kb)", config, benchQualityExpectedErrors, .{&qs});
        result2.print();
    }

    std.debug.print("\n", .{});

    // ========================================================================
    // Summary
    // ========================================================================

    std.debug.print("=" ** 70 ++ "\n", .{});
    std.debug.print("Benchmark Complete\n", .{});
    std.debug.print("=" ** 70 ++ "\n\n", .{});

    std.debug.print("Notes:\n", .{});
    std.debug.print("  - Times shown are per-operation averages\n", .{});
    std.debug.print("  - Throughput is operations per second\n", .{});
    std.debug.print("  - Memory allocations are included in timing\n", .{});
    std.debug.print("  - Build with -Doptimize=ReleaseFast for best performance\n", .{});
    std.debug.print("\n", .{});
}
