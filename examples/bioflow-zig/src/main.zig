//! BioFlow - Bioinformatics Toolkit in Zig
//!
//! A production-quality bioinformatics library showcasing Zig's strengths:
//! - Explicit memory management with allocators
//! - No hidden control flow
//! - Comptime features
//! - Excellent error handling
//!
//! This implementation is designed for performance comparison with Aria.

const std = @import("std");
const Sequence = @import("sequence").Sequence;
const SequenceError = @import("sequence").SequenceError;
const parseFasta = @import("sequence").parseFasta;
const KMerCounter = @import("kmer").KMerCounter;
const KMerError = @import("kmer").KMerError;
const alignment = @import("alignment");
const quality = @import("quality");
const stats = @import("stats");

/// Application version
pub const VERSION = "1.0.0";

/// Command-line subcommands
const Command = enum {
    gc,
    kmer,
    align,
    stats_cmd,
    help,

    fn fromString(str: []const u8) ?Command {
        const commands = std.StaticStringMap(Command).initComptime(.{
            .{ "gc", .gc },
            .{ "kmer", .kmer },
            .{ "align", .align },
            .{ "stats", .stats_cmd },
            .{ "help", .help },
        });
        return commands.get(str);
    }
};

pub fn main() !void {
    // Use GeneralPurposeAllocator for leak detection in debug mode
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer {
        const leaked = gpa.deinit();
        if (leaked == .leak) {
            std.debug.print("Warning: Memory leak detected!\n", .{});
        }
    }
    const allocator = gpa.allocator();

    // Parse command-line arguments
    const args = try std.process.argsAlloc(allocator);
    defer std.process.argsFree(allocator, args);

    if (args.len < 2) {
        try printUsage();
        return;
    }

    const command = Command.fromString(args[1]) orelse {
        std.debug.print("Unknown command: {s}\n\n", .{args[1]});
        try printUsage();
        return;
    };

    switch (command) {
        .gc => try runGcCommand(allocator, args[2..]),
        .kmer => try runKmerCommand(allocator, args[2..]),
        .align => try runAlignCommand(allocator, args[2..]),
        .stats_cmd => try runStatsCommand(allocator, args[2..]),
        .help => try printUsage(),
    }
}

fn printUsage() !void {
    const stdout = std.io.getStdOut().writer();

    try stdout.print(
        \\BioFlow v{s} - Bioinformatics Toolkit in Zig
        \\
        \\Usage: bioflow-zig <command> [options]
        \\
        \\Commands:
        \\  gc <sequence>        Calculate GC content
        \\  kmer <k> <sequence>  Count k-mers in sequence
        \\  align <seq1> <seq2>  Align two sequences
        \\  stats <sequence>     Calculate sequence statistics
        \\  help                 Show this help message
        \\
        \\Examples:
        \\  bioflow-zig gc ATGCGATCGATCG
        \\  bioflow-zig kmer 3 ATGATGATG
        \\  bioflow-zig align ACGTACGT ACGACGT
        \\  bioflow-zig stats ATGCGATCGATCGATCG
        \\
        \\This implementation showcases Zig's features:
        \\  - Explicit memory management
        \\  - Comptime capabilities
        \\  - No hidden control flow
        \\  - Excellent error handling
        \\
    , .{VERSION});
}

fn runGcCommand(allocator: std.mem.Allocator, args: []const []const u8) !void {
    const stdout = std.io.getStdOut().writer();

    if (args.len < 1) {
        try stdout.print("Error: Please provide a sequence\n", .{});
        try stdout.print("Usage: bioflow-zig gc <sequence>\n", .{});
        return;
    }

    var seq = Sequence.init(allocator, args[0]) catch |err| {
        try stdout.print("Error: {s}\n", .{@errorName(err)});
        return;
    };
    defer seq.deinit();

    const gc = seq.gcContent();
    const counts = seq.baseCounts();

    try stdout.print(
        \\Sequence Analysis
        \\================
        \\Length:     {d} bp
        \\GC Content: {d:.2}%
        \\
        \\Base Composition:
        \\  A: {d} ({d:.1}%)
        \\  C: {d} ({d:.1}%)
        \\  G: {d} ({d:.1}%)
        \\  T: {d} ({d:.1}%)
        \\  N: {d} ({d:.1}%)
        \\
    , .{
        seq.len(),
        gc * 100.0,
        counts.a,
        @as(f64, @floatFromInt(counts.a)) / @as(f64, @floatFromInt(seq.len())) * 100.0,
        counts.c,
        @as(f64, @floatFromInt(counts.c)) / @as(f64, @floatFromInt(seq.len())) * 100.0,
        counts.g,
        @as(f64, @floatFromInt(counts.g)) / @as(f64, @floatFromInt(seq.len())) * 100.0,
        counts.t,
        @as(f64, @floatFromInt(counts.t)) / @as(f64, @floatFromInt(seq.len())) * 100.0,
        counts.n,
        @as(f64, @floatFromInt(counts.n)) / @as(f64, @floatFromInt(seq.len())) * 100.0,
    });

    // Also show complement and reverse complement
    var comp = try seq.complement();
    defer comp.deinit();

    var rc = try seq.reverseComplement();
    defer rc.deinit();

    try stdout.print(
        \\Complement:         {s}
        \\Reverse Complement: {s}
        \\
    , .{ comp.bases, rc.bases });
}

fn runKmerCommand(allocator: std.mem.Allocator, args: []const []const u8) !void {
    const stdout = std.io.getStdOut().writer();

    if (args.len < 2) {
        try stdout.print("Error: Please provide k and a sequence\n", .{});
        try stdout.print("Usage: bioflow-zig kmer <k> <sequence>\n", .{});
        return;
    }

    const k = std.fmt.parseInt(usize, args[0], 10) catch {
        try stdout.print("Error: Invalid k value: {s}\n", .{args[0]});
        return;
    };

    var seq = Sequence.init(allocator, args[1]) catch |err| {
        try stdout.print("Error: {s}\n", .{@errorName(err)});
        return;
    };
    defer seq.deinit();

    var counter = KMerCounter.init(allocator, k) catch |err| {
        try stdout.print("Error: {s}\n", .{@errorName(err)});
        return;
    };
    defer counter.deinit();

    counter.count(seq) catch |err| {
        try stdout.print("Error: {s}\n", .{@errorName(err)});
        return;
    };

    try stdout.print(
        \\K-mer Analysis (k={d})
        \\====================
        \\Sequence Length: {d} bp
        \\Total K-mers:    {d}
        \\Unique K-mers:   {d}
        \\Diversity:       {d:.4}
        \\Entropy:         {d:.4} bits
        \\
        \\Top 10 K-mers:
        \\
    , .{
        k,
        seq.len(),
        counter.total_count,
        counter.uniqueCount(),
        counter.diversity(),
        counter.entropy(),
    });

    const top = try counter.mostFrequent(allocator, 10);
    defer allocator.free(top);

    for (top, 1..) |kmer_count, i| {
        const freq = kmer_count.frequency(counter.total_count);
        try stdout.print("  {d:2}. {s}: {d} ({d:.2}%)\n", .{
            i,
            kmer_count.kmer,
            kmer_count.count,
            freq * 100.0,
        });
    }
}

fn runAlignCommand(allocator: std.mem.Allocator, args: []const []const u8) !void {
    const stdout = std.io.getStdOut().writer();

    if (args.len < 2) {
        try stdout.print("Error: Please provide two sequences\n", .{});
        try stdout.print("Usage: bioflow-zig align <seq1> <seq2>\n", .{});
        return;
    }

    var seq1 = Sequence.init(allocator, args[0]) catch |err| {
        try stdout.print("Error in sequence 1: {s}\n", .{@errorName(err)});
        return;
    };
    defer seq1.deinit();

    var seq2 = Sequence.init(allocator, args[1]) catch |err| {
        try stdout.print("Error in sequence 2: {s}\n", .{@errorName(err)});
        return;
    };
    defer seq2.deinit();

    // Perform local alignment (Smith-Waterman)
    var local_align = alignment.smithWaterman(allocator, seq1, seq2, alignment.ScoringMatrix.default()) catch |err| {
        try stdout.print("Error in alignment: {s}\n", .{@errorName(err)});
        return;
    };
    defer local_align.deinit();

    // Perform global alignment (Needleman-Wunsch)
    var global_align = alignment.needlemanWunsch(allocator, seq1, seq2, alignment.ScoringMatrix.default()) catch |err| {
        try stdout.print("Error in alignment: {s}\n", .{@errorName(err)});
        return;
    };
    defer global_align.deinit();

    try stdout.print(
        \\Sequence Alignment Results
        \\==========================
        \\Sequence 1: {s} ({d} bp)
        \\Sequence 2: {s} ({d} bp)
        \\
        \\Local Alignment (Smith-Waterman)
        \\--------------------------------
        \\Score:      {d}
        \\Identity:   {d:.1}%
        \\Matches:    {d}
        \\Mismatches: {d}
        \\Gaps:       {d}
        \\
        \\Aligned Seq1: {s}
        \\Aligned Seq2: {s}
        \\
        \\Global Alignment (Needleman-Wunsch)
        \\-----------------------------------
        \\Score:      {d}
        \\Identity:   {d:.1}%
        \\Matches:    {d}
        \\Mismatches: {d}
        \\Gaps:       {d}
        \\
        \\Aligned Seq1: {s}
        \\Aligned Seq2: {s}
        \\
    , .{
        seq1.bases,
        seq1.len(),
        seq2.bases,
        seq2.len(),
        local_align.score,
        local_align.identity() * 100.0,
        local_align.matches,
        local_align.mismatches,
        local_align.gaps,
        local_align.aligned_seq1,
        local_align.aligned_seq2,
        global_align.score,
        global_align.identity() * 100.0,
        global_align.matches,
        global_align.mismatches,
        global_align.gaps,
        global_align.aligned_seq1,
        global_align.aligned_seq2,
    });

    // Generate CIGAR string
    const cigar = try local_align.cigar(allocator);
    defer allocator.free(cigar);
    try stdout.print("CIGAR (local): {s}\n", .{cigar});

    // Calculate edit distance
    const edit_dist = try alignment.editDistance(seq1.bases, seq2.bases, allocator);
    try stdout.print("Edit Distance: {d}\n", .{edit_dist});
}

fn runStatsCommand(allocator: std.mem.Allocator, args: []const []const u8) !void {
    const stdout = std.io.getStdOut().writer();

    if (args.len < 1) {
        try stdout.print("Error: Please provide a sequence\n", .{});
        try stdout.print("Usage: bioflow-zig stats <sequence>\n", .{});
        return;
    }

    var seq = Sequence.init(allocator, args[0]) catch |err| {
        try stdout.print("Error: {s}\n", .{@errorName(err)});
        return;
    };
    defer seq.deinit();

    const counts = seq.baseCounts();
    const gc = seq.gcContent();

    // Calculate base frequencies for entropy
    const total = @as(f64, @floatFromInt(seq.len()));
    const probs = [_]f64{
        @as(f64, @floatFromInt(counts.a)) / total,
        @as(f64, @floatFromInt(counts.c)) / total,
        @as(f64, @floatFromInt(counts.g)) / total,
        @as(f64, @floatFromInt(counts.t)) / total,
    };

    const ent = stats.entropy(&probs);
    const norm_ent = stats.normalizedEntropy(&probs);

    try stdout.print(
        \\Sequence Statistics
        \\==================
        \\Length:            {d} bp
        \\GC Content:        {d:.2}%
        \\Molecular Weight:  {d:.2} g/mol
        \\Melting Temp (Tm): {d:.1}C
        \\
        \\Information Theory:
        \\  Shannon Entropy:    {d:.4} bits
        \\  Normalized Entropy: {d:.4}
        \\
        \\Base Composition:
        \\  A: {d:5} ({d:5.1}%)
        \\  C: {d:5} ({d:5.1}%)
        \\  G: {d:5} ({d:5.1}%)
        \\  T: {d:5} ({d:5.1}%)
        \\  N: {d:5} ({d:5.1}%)
        \\
    , .{
        seq.len(),
        gc * 100.0,
        seq.molecularWeight(),
        seq.meltingTemperature(),
        ent,
        norm_ent,
        counts.a,
        probs[0] * 100.0,
        counts.c,
        probs[1] * 100.0,
        counts.g,
        probs[2] * 100.0,
        counts.t,
        probs[3] * 100.0,
        counts.n,
        @as(f64, @floatFromInt(counts.n)) / total * 100.0,
    });

    // Show complement sequences
    var comp = try seq.complement();
    defer comp.deinit();

    var rc = try seq.reverseComplement();
    defer rc.deinit();

    try stdout.print(
        \\Derived Sequences:
        \\  Complement:         {s}
        \\  Reverse:
    , .{comp.bases});

    var rev = try seq.reverse();
    defer rev.deinit();
    try stdout.print("{s}\n", .{rev.bases});

    try stdout.print("  Reverse Complement: {s}\n", .{rc.bases});

    // Search for common motifs
    try stdout.print("\nMotif Search:\n", .{});

    const motifs = [_][]const u8{ "ATG", "TAA", "TAG", "TGA", "TATA", "GAATTC" };
    const motif_names = [_][]const u8{ "Start codon", "Stop (TAA)", "Stop (TAG)", "Stop (TGA)", "TATA box", "EcoRI site" };

    for (motifs, motif_names) |motif, name| {
        const positions = try seq.findAll(allocator, motif);
        defer allocator.free(positions);

        if (positions.len > 0) {
            try stdout.print("  {s} ({s}): {d} occurrence(s) at positions ", .{ motif, name, positions.len });
            for (positions, 0..) |pos, i| {
                if (i > 0) try stdout.print(", ", .{});
                try stdout.print("{d}", .{pos});
                if (i >= 4) {
                    try stdout.print("...", .{});
                    break;
                }
            }
            try stdout.print("\n", .{});
        }
    }
}

// Comptime demonstration: Generate a lookup table at compile time
const ComplementTable = comptime blk: {
    var table: [256]u8 = undefined;
    for (&table, 0..) |*entry, i| {
        entry.* = switch (@as(u8, @intCast(i))) {
            'A' => 'T',
            'T' => 'A',
            'C' => 'G',
            'G' => 'C',
            'a' => 't',
            't' => 'a',
            'c' => 'g',
            'g' => 'c',
            else => 'N',
        };
    }
    break :blk table;
};

/// Fast complement using comptime-generated table
pub fn fastComplement(base: u8) u8 {
    return ComplementTable[base];
}

// Unit tests for main module
test "fast complement using comptime table" {
    const testing = std.testing;

    try testing.expectEqual(@as(u8, 'T'), fastComplement('A'));
    try testing.expectEqual(@as(u8, 'A'), fastComplement('T'));
    try testing.expectEqual(@as(u8, 'G'), fastComplement('C'));
    try testing.expectEqual(@as(u8, 'C'), fastComplement('G'));
    try testing.expectEqual(@as(u8, 'N'), fastComplement('X'));
}

test "version is defined" {
    const testing = std.testing;
    try testing.expect(VERSION.len > 0);
}
