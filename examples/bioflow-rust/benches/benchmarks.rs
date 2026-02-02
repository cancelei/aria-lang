//! Benchmarks for BioFlow using Criterion.
//!
//! Run with: cargo bench

use bioflow_rust::*;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

/// Benchmark GC content calculation for different sequence lengths.
fn benchmark_gc_content(c: &mut Criterion) {
    let mut group = c.benchmark_group("gc_content");

    for size in [1_000, 10_000, 100_000, 1_000_000] {
        let seq = Sequence::new("ATGC".repeat(size / 4)).unwrap();
        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &seq, |b, seq| {
            b.iter(|| black_box(seq.gc_content()))
        });
    }

    group.finish();
}

/// Benchmark base composition calculation.
fn benchmark_base_composition(c: &mut Criterion) {
    let mut group = c.benchmark_group("base_composition");

    for size in [1_000, 10_000, 100_000] {
        let seq = Sequence::new("ATGC".repeat(size / 4)).unwrap();
        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &seq, |b, seq| {
            b.iter(|| black_box(seq.base_composition()))
        });
    }

    group.finish();
}

/// Benchmark sequence complement.
fn benchmark_complement(c: &mut Criterion) {
    let mut group = c.benchmark_group("complement");

    for size in [1_000, 10_000, 100_000] {
        let seq = Sequence::new("ATGC".repeat(size / 4)).unwrap();
        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &seq, |b, seq| {
            b.iter(|| black_box(seq.complement()))
        });
    }

    group.finish();
}

/// Benchmark k-mer counting for different k values.
fn benchmark_kmer_counting(c: &mut Criterion) {
    let mut group = c.benchmark_group("kmer_counting");

    // Different k values
    for k in [7, 11, 21, 31] {
        let seq = Sequence::new("ATGC".repeat(5000)).unwrap();
        group.throughput(Throughput::Bytes(seq.len() as u64));

        group.bench_with_input(BenchmarkId::new("k", k), &seq, |b, seq| {
            b.iter(|| {
                let mut counter = KMerCounter::new(k);
                counter.count(black_box(seq));
                black_box(counter)
            })
        });
    }

    group.finish();
}

/// Benchmark k-mer counting for different sequence lengths.
fn benchmark_kmer_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("kmer_scaling");
    let k = 21;

    for size in [1_000, 10_000, 50_000] {
        let seq = Sequence::new("ATGC".repeat(size / 4)).unwrap();
        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &seq, |b, seq| {
            b.iter(|| {
                let mut counter = KMerCounter::new(k);
                counter.count(black_box(seq));
                black_box(counter)
            })
        });
    }

    group.finish();
}

/// Benchmark Smith-Waterman alignment for different sequence lengths.
fn benchmark_smith_waterman(c: &mut Criterion) {
    let mut group = c.benchmark_group("smith_waterman");
    let scoring = ScoringMatrix::default();

    for size in [100, 250, 500, 1000] {
        let seq1 = Sequence::new("ACGT".repeat(size / 4)).unwrap();
        let seq2 = Sequence::new("AGCT".repeat(size / 4)).unwrap();

        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(seq1.clone(), seq2.clone()),
            |b, (seq1, seq2)| {
                b.iter(|| black_box(smith_waterman(seq1, seq2, &scoring)))
            },
        );
    }

    group.finish();
}

/// Benchmark Needleman-Wunsch alignment for different sequence lengths.
fn benchmark_needleman_wunsch(c: &mut Criterion) {
    let mut group = c.benchmark_group("needleman_wunsch");
    let scoring = ScoringMatrix::default();

    for size in [100, 250, 500, 1000] {
        let seq1 = Sequence::new("ACGT".repeat(size / 4)).unwrap();
        let seq2 = Sequence::new("AGCT".repeat(size / 4)).unwrap();

        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(seq1.clone(), seq2.clone()),
            |b, (seq1, seq2)| {
                b.iter(|| black_box(needleman_wunsch(seq1, seq2, &scoring)))
            },
        );
    }

    group.finish();
}

/// Benchmark edit distance calculation.
fn benchmark_edit_distance(c: &mut Criterion) {
    let mut group = c.benchmark_group("edit_distance");

    for size in [100, 500, 1000] {
        let seq1 = Sequence::new("ACGT".repeat(size / 4)).unwrap();
        let seq2 = Sequence::new("AGCT".repeat(size / 4)).unwrap();

        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(seq1.clone(), seq2.clone()),
            |b, (seq1, seq2)| {
                b.iter(|| black_box(edit_distance(seq1, seq2)))
            },
        );
    }

    group.finish();
}

/// Benchmark quality score parsing.
fn benchmark_quality_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("quality_parsing");

    for size in [100, 500, 1000, 5000] {
        // Generate a quality string with mixed scores
        let quality_str: String = (0..size)
            .map(|i| {
                let q = 33 + (i % 40) as u8;
                q as char
            })
            .collect();

        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &quality_str, |b, q| {
            b.iter(|| black_box(QualityScores::from_phred33(q.clone()).unwrap()))
        });
    }

    group.finish();
}

/// Benchmark quality statistics.
fn benchmark_quality_stats(c: &mut Criterion) {
    let mut group = c.benchmark_group("quality_stats");

    for size in [100, 500, 1000] {
        let quality_str: String = (0..size)
            .map(|i| {
                let q = 33 + (i % 40) as u8;
                q as char
            })
            .collect();
        let quality = QualityScores::from_phred33(&quality_str).unwrap();

        group.bench_with_input(BenchmarkId::from_parameter(size), &quality, |b, q| {
            b.iter(|| {
                black_box(q.mean());
                black_box(q.median());
                black_box(q.high_quality_fraction());
            })
        });
    }

    group.finish();
}

/// Benchmark sequence validation.
fn benchmark_sequence_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("sequence_validation");

    for size in [1_000, 10_000, 100_000] {
        let bases = "ATGC".repeat(size / 4);
        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &bases, |b, bases| {
            b.iter(|| black_box(Sequence::new(bases.clone()).unwrap()))
        });
    }

    group.finish();
}

/// Benchmark pattern finding.
fn benchmark_pattern_finding(c: &mut Criterion) {
    let mut group = c.benchmark_group("pattern_finding");

    let seq = Sequence::new("ATGC".repeat(10000)).unwrap();

    for pattern_len in [4, 8, 12] {
        let pattern = "ATGC"[..pattern_len.min(4)].to_string();

        group.bench_with_input(
            BenchmarkId::new("pattern_len", pattern_len),
            &pattern,
            |b, pattern| {
                b.iter(|| black_box(seq.find_pattern(pattern)))
            },
        );
    }

    group.finish();
}

/// Benchmark running statistics.
fn benchmark_running_stats(c: &mut Criterion) {
    let mut group = c.benchmark_group("running_stats");

    for size in [100, 1000, 10000] {
        let data: Vec<f64> = (0..size).map(|i| (i as f64) * 0.1).collect();

        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
            b.iter(|| {
                let mut stats = RunningStats::new();
                for &v in data {
                    stats.push(black_box(v));
                }
                black_box(stats)
            })
        });
    }

    group.finish();
}

/// Benchmark N50 calculation.
fn benchmark_n50(c: &mut Criterion) {
    let mut group = c.benchmark_group("n50");

    for size in [100, 1000, 10000] {
        let lengths: Vec<usize> = (1..=size).collect();

        group.bench_with_input(BenchmarkId::from_parameter(size), &lengths, |b, lengths| {
            b.iter(|| black_box(n50(lengths)))
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_gc_content,
    benchmark_base_composition,
    benchmark_complement,
    benchmark_kmer_counting,
    benchmark_kmer_scaling,
    benchmark_smith_waterman,
    benchmark_needleman_wunsch,
    benchmark_edit_distance,
    benchmark_quality_parsing,
    benchmark_quality_stats,
    benchmark_sequence_validation,
    benchmark_pattern_finding,
    benchmark_running_stats,
    benchmark_n50,
);

criterion_main!(benches);
