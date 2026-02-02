const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    // Create module for shared code
    const sequence_mod = b.addModule("sequence", .{
        .root_source_file = b.path("src/sequence.zig"),
    });

    const kmer_mod = b.addModule("kmer", .{
        .root_source_file = b.path("src/kmer.zig"),
    });
    kmer_mod.addImport("sequence", sequence_mod);

    const alignment_mod = b.addModule("alignment", .{
        .root_source_file = b.path("src/alignment.zig"),
    });
    alignment_mod.addImport("sequence", sequence_mod);

    const quality_mod = b.addModule("quality", .{
        .root_source_file = b.path("src/quality.zig"),
    });

    const stats_mod = b.addModule("stats", .{
        .root_source_file = b.path("src/stats.zig"),
    });
    stats_mod.addImport("sequence", sequence_mod);

    // Main executable
    const exe = b.addExecutable(.{
        .name = "bioflow-zig",
        .root_source_file = b.path("src/main.zig"),
        .target = target,
        .optimize = optimize,
    });
    exe.root_module.addImport("sequence", sequence_mod);
    exe.root_module.addImport("kmer", kmer_mod);
    exe.root_module.addImport("alignment", alignment_mod);
    exe.root_module.addImport("quality", quality_mod);
    exe.root_module.addImport("stats", stats_mod);
    b.installArtifact(exe);

    // Run command
    const run_cmd = b.addRunArtifact(exe);
    run_cmd.step.dependOn(b.getInstallStep());
    if (b.args) |args| {
        run_cmd.addArgs(args);
    }
    const run_step = b.step("run", "Run the bioflow application");
    run_step.dependOn(&run_cmd.step);

    // Unit tests for each module
    const sequence_tests = b.addTest(.{
        .root_source_file = b.path("tests/sequence_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    sequence_tests.root_module.addImport("sequence", sequence_mod);

    const kmer_tests = b.addTest(.{
        .root_source_file = b.path("tests/kmer_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    kmer_tests.root_module.addImport("sequence", sequence_mod);
    kmer_tests.root_module.addImport("kmer", kmer_mod);

    const alignment_tests = b.addTest(.{
        .root_source_file = b.path("tests/alignment_test.zig"),
        .target = target,
        .optimize = optimize,
    });
    alignment_tests.root_module.addImport("sequence", sequence_mod);
    alignment_tests.root_module.addImport("alignment", alignment_mod);

    const run_sequence_tests = b.addRunArtifact(sequence_tests);
    const run_kmer_tests = b.addRunArtifact(kmer_tests);
    const run_alignment_tests = b.addRunArtifact(alignment_tests);

    const test_step = b.step("test", "Run all unit tests");
    test_step.dependOn(&run_sequence_tests.step);
    test_step.dependOn(&run_kmer_tests.step);
    test_step.dependOn(&run_alignment_tests.step);

    // Benchmarks
    const bench = b.addExecutable(.{
        .name = "bench",
        .root_source_file = b.path("benchmark/bench.zig"),
        .target = target,
        .optimize = .ReleaseFast,
    });
    bench.root_module.addImport("sequence", sequence_mod);
    bench.root_module.addImport("kmer", kmer_mod);
    bench.root_module.addImport("alignment", alignment_mod);
    bench.root_module.addImport("quality", quality_mod);
    bench.root_module.addImport("stats", stats_mod);
    b.installArtifact(bench);

    const bench_cmd = b.addRunArtifact(bench);
    bench_cmd.step.dependOn(b.getInstallStep());
    const bench_step = b.step("bench", "Run benchmarks");
    bench_step.dependOn(&bench_cmd.step);
}
