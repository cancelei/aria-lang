#include <benchmark/benchmark.h>
#include "bioflow/sequence.hpp"
#include "bioflow/kmer.hpp"
#include "bioflow/alignment.hpp"
#include "bioflow/quality.hpp"
#include "bioflow/stats.hpp"

#include <random>
#include <string>

using namespace bioflow;

// ============================================================================
// Helper Functions
// ============================================================================

static std::string generateRandomSequence(size_t length, unsigned seed = 42) {
    static const char bases[] = "ACGT";
    std::mt19937 rng(seed);
    std::uniform_int_distribution<int> dist(0, 3);

    std::string result;
    result.reserve(length);
    for (size_t i = 0; i < length; ++i) {
        result += bases[dist(rng)];
    }
    return result;
}

static std::string generateRepeatingSequence(size_t length) {
    std::string pattern = "ATGCGATCGATCGATCGATCGATCG";
    std::string result;
    result.reserve(length);
    while (result.length() < length) {
        result += pattern;
    }
    result.resize(length);
    return result;
}

// ============================================================================
// Sequence Benchmarks
// ============================================================================

static void BM_SequenceConstruction(benchmark::State& state) {
    auto bases = generateRandomSequence(static_cast<size_t>(state.range(0)));

    for (auto _ : state) {
        Sequence seq(bases);
        benchmark::DoNotOptimize(seq);
    }

    state.SetBytesProcessed(state.iterations() * state.range(0));
}
BENCHMARK(BM_SequenceConstruction)->Range(100, 100000);

static void BM_GCContent(benchmark::State& state) {
    auto bases = generateRandomSequence(static_cast<size_t>(state.range(0)));
    Sequence seq(bases);

    for (auto _ : state) {
        auto gc = seq.gcContent();
        benchmark::DoNotOptimize(gc);
    }

    state.SetBytesProcessed(state.iterations() * state.range(0));
}
BENCHMARK(BM_GCContent)->Range(100, 100000);

static void BM_GCContentLarge(benchmark::State& state) {
    // 20000 bases as specified in requirements
    auto bases = generateRepeatingSequence(20000);
    Sequence seq(bases);

    for (auto _ : state) {
        auto gc = seq.gcContent();
        benchmark::DoNotOptimize(gc);
    }

    state.SetItemsProcessed(state.iterations());
}
BENCHMARK(BM_GCContentLarge);

static void BM_Complement(benchmark::State& state) {
    auto bases = generateRandomSequence(static_cast<size_t>(state.range(0)));
    Sequence seq(bases);

    for (auto _ : state) {
        auto comp = seq.complement();
        benchmark::DoNotOptimize(comp);
    }

    state.SetBytesProcessed(state.iterations() * state.range(0));
}
BENCHMARK(BM_Complement)->Range(100, 100000);

static void BM_ReverseComplement(benchmark::State& state) {
    auto bases = generateRandomSequence(static_cast<size_t>(state.range(0)));
    Sequence seq(bases);

    for (auto _ : state) {
        auto rc = seq.reverseComplement();
        benchmark::DoNotOptimize(rc);
    }

    state.SetBytesProcessed(state.iterations() * state.range(0));
}
BENCHMARK(BM_ReverseComplement)->Range(100, 100000);

static void BM_MotifFinding(benchmark::State& state) {
    auto bases = generateRepeatingSequence(static_cast<size_t>(state.range(0)));
    Sequence seq(bases);
    std::string motif = "GATC";

    for (auto _ : state) {
        auto positions = seq.findMotifPositions(motif);
        benchmark::DoNotOptimize(positions);
    }

    state.SetBytesProcessed(state.iterations() * state.range(0));
}
BENCHMARK(BM_MotifFinding)->Range(1000, 100000);

// ============================================================================
// K-mer Benchmarks
// ============================================================================

static void BM_KMerCounting(benchmark::State& state) {
    size_t k = static_cast<size_t>(state.range(0));
    auto bases = generateRepeatingSequence(20000);
    Sequence seq(bases);

    for (auto _ : state) {
        KMerCounter counter(k);
        counter.count(seq);
        benchmark::DoNotOptimize(counter);
    }

    state.SetItemsProcessed(state.iterations());
}
BENCHMARK(BM_KMerCounting)->Arg(5)->Arg(11)->Arg(21)->Arg(31);

static void BM_KMerCountingLarge(benchmark::State& state) {
    // Standard benchmark: 20000 bp sequence, k=21
    auto bases = generateRepeatingSequence(20000);
    Sequence seq(bases);

    for (auto _ : state) {
        KMerCounter counter(21);
        counter.count(seq);
        benchmark::DoNotOptimize(counter);
    }

    state.SetItemsProcessed(state.iterations());
}
BENCHMARK(BM_KMerCountingLarge);

static void BM_KMerMostFrequent(benchmark::State& state) {
    auto bases = generateRandomSequence(10000);
    Sequence seq(bases);

    KMerCounter counter(11);
    counter.count(seq);

    for (auto _ : state) {
        auto top = counter.mostFrequent(static_cast<size_t>(state.range(0)));
        benchmark::DoNotOptimize(top);
    }
}
BENCHMARK(BM_KMerMostFrequent)->Arg(10)->Arg(100)->Arg(1000);

static void BM_CanonicalKMerCounting(benchmark::State& state) {
    auto bases = generateRandomSequence(20000);
    Sequence seq(bases);

    for (auto _ : state) {
        CanonicalKMerCounter counter(21);
        counter.count(seq);
        benchmark::DoNotOptimize(counter);
    }

    state.SetItemsProcessed(state.iterations());
}
BENCHMARK(BM_CanonicalKMerCounting);

// ============================================================================
// Alignment Benchmarks
// ============================================================================

static void BM_SmithWaterman(benchmark::State& state) {
    size_t len = static_cast<size_t>(state.range(0));
    auto bases1 = generateRandomSequence(len, 42);
    auto bases2 = generateRandomSequence(len, 123);
    Sequence seq1(bases1), seq2(bases2);

    for (auto _ : state) {
        auto alignment = smithWaterman(seq1, seq2);
        benchmark::DoNotOptimize(alignment);
    }

    state.SetComplexityN(state.range(0));
}
BENCHMARK(BM_SmithWaterman)->Range(50, 500)->Complexity();

static void BM_SmithWatermanStandard(benchmark::State& state) {
    // Standard benchmark: 1000 x 1000
    auto bases1 = generateRandomSequence(1000, 42);
    auto bases2 = generateRandomSequence(1000, 123);
    Sequence seq1(bases1), seq2(bases2);

    for (auto _ : state) {
        auto alignment = smithWaterman(seq1, seq2);
        benchmark::DoNotOptimize(alignment);
    }

    state.SetItemsProcessed(state.iterations());
}
BENCHMARK(BM_SmithWatermanStandard);

static void BM_NeedlemanWunsch(benchmark::State& state) {
    size_t len = static_cast<size_t>(state.range(0));
    auto bases1 = generateRandomSequence(len, 42);
    auto bases2 = generateRandomSequence(len, 123);
    Sequence seq1(bases1), seq2(bases2);

    for (auto _ : state) {
        auto alignment = needlemanWunsch(seq1, seq2);
        benchmark::DoNotOptimize(alignment);
    }

    state.SetComplexityN(state.range(0));
}
BENCHMARK(BM_NeedlemanWunsch)->Range(50, 500)->Complexity();

static void BM_EditDistance(benchmark::State& state) {
    size_t len = static_cast<size_t>(state.range(0));
    auto bases1 = generateRandomSequence(len, 42);
    auto bases2 = generateRandomSequence(len, 123);
    Sequence seq1(bases1), seq2(bases2);

    for (auto _ : state) {
        auto dist = editDistance(seq1, seq2);
        benchmark::DoNotOptimize(dist);
    }

    state.SetComplexityN(state.range(0));
}
BENCHMARK(BM_EditDistance)->Range(50, 1000)->Complexity();

static void BM_HammingDistance(benchmark::State& state) {
    size_t len = static_cast<size_t>(state.range(0));
    auto bases1 = generateRandomSequence(len, 42);
    auto bases2 = generateRandomSequence(len, 123);
    Sequence seq1(bases1), seq2(bases2);

    for (auto _ : state) {
        auto dist = hammingDistance(seq1, seq2);
        benchmark::DoNotOptimize(dist);
    }

    state.SetBytesProcessed(state.iterations() * state.range(0));
}
BENCHMARK(BM_HammingDistance)->Range(100, 100000);

// ============================================================================
// Quality Score Benchmarks
// ============================================================================

static void BM_QualityConstruction(benchmark::State& state) {
    size_t len = static_cast<size_t>(state.range(0));
    std::string quality(len, 'I');  // High quality scores

    for (auto _ : state) {
        QualityScores scores(quality);
        benchmark::DoNotOptimize(scores);
    }

    state.SetBytesProcessed(state.iterations() * state.range(0));
}
BENCHMARK(BM_QualityConstruction)->Range(100, 100000);

static void BM_QualityMean(benchmark::State& state) {
    size_t len = static_cast<size_t>(state.range(0));
    std::string quality(len, 'I');
    QualityScores scores(quality);

    for (auto _ : state) {
        auto mean = scores.meanQuality();
        benchmark::DoNotOptimize(mean);
    }

    state.SetBytesProcessed(state.iterations() * state.range(0));
}
BENCHMARK(BM_QualityMean)->Range(100, 100000);

static void BM_QualityTrimming(benchmark::State& state) {
    size_t len = static_cast<size_t>(state.range(0));
    // Create quality string with varying quality
    std::string quality;
    for (size_t i = 0; i < len; ++i) {
        quality += (i < len/4 || i > 3*len/4) ? '5' : 'I';
    }
    QualityScores scores(quality);

    for (auto _ : state) {
        auto trimmed = scores.trim(20, 10);
        benchmark::DoNotOptimize(trimmed);
    }

    state.SetBytesProcessed(state.iterations() * state.range(0));
}
BENCHMARK(BM_QualityTrimming)->Range(100, 10000);

// ============================================================================
// Statistics Benchmarks
// ============================================================================

static void BM_ShannonEntropy(benchmark::State& state) {
    auto bases = generateRandomSequence(static_cast<size_t>(state.range(0)));
    Sequence seq(bases);

    for (auto _ : state) {
        auto entropy = stats::shannonEntropy(seq);
        benchmark::DoNotOptimize(entropy);
    }

    state.SetBytesProcessed(state.iterations() * state.range(0));
}
BENCHMARK(BM_ShannonEntropy)->Range(100, 100000);

static void BM_LinguisticComplexity(benchmark::State& state) {
    auto bases = generateRandomSequence(static_cast<size_t>(state.range(0)));
    Sequence seq(bases);

    for (auto _ : state) {
        auto complexity = stats::linguisticComplexity(seq, 3);
        benchmark::DoNotOptimize(complexity);
    }
}
BENCHMARK(BM_LinguisticComplexity)->Range(100, 10000);

static void BM_JaccardSimilarity(benchmark::State& state) {
    auto bases1 = generateRandomSequence(10000, 42);
    auto bases2 = generateRandomSequence(10000, 123);
    Sequence seq1(bases1), seq2(bases2);

    KMerCounter counter1(11), counter2(11);
    counter1.count(seq1);
    counter2.count(seq2);

    for (auto _ : state) {
        auto similarity = stats::jaccardSimilarity(counter1, counter2);
        benchmark::DoNotOptimize(similarity);
    }
}
BENCHMARK(BM_JaccardSimilarity);

static void BM_CosineSimilarity(benchmark::State& state) {
    auto bases1 = generateRandomSequence(10000, 42);
    auto bases2 = generateRandomSequence(10000, 123);
    Sequence seq1(bases1), seq2(bases2);

    KMerCounter counter1(11), counter2(11);
    counter1.count(seq1);
    counter2.count(seq2);

    for (auto _ : state) {
        auto similarity = stats::cosineSimilarity(counter1, counter2);
        benchmark::DoNotOptimize(similarity);
    }
}
BENCHMARK(BM_CosineSimilarity);

// ============================================================================
// Memory Allocation Benchmarks
// ============================================================================

static void BM_SequenceAllocation(benchmark::State& state) {
    size_t count = static_cast<size_t>(state.range(0));
    auto bases = generateRandomSequence(1000);

    for (auto _ : state) {
        std::vector<Sequence> sequences;
        sequences.reserve(count);
        for (size_t i = 0; i < count; ++i) {
            sequences.emplace_back(bases);
        }
        benchmark::DoNotOptimize(sequences);
    }
}
BENCHMARK(BM_SequenceAllocation)->Range(10, 10000);

// ============================================================================
// Main
// ============================================================================

BENCHMARK_MAIN();
