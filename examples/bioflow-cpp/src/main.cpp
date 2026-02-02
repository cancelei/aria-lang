#include "bioflow/sequence.hpp"
#include "bioflow/kmer.hpp"
#include "bioflow/alignment.hpp"
#include "bioflow/quality.hpp"
#include "bioflow/stats.hpp"

#include <iostream>
#include <iomanip>
#include <chrono>
#include <string>
#include <vector>

using namespace bioflow;

// Helper for timing
template<typename Func>
auto measureTime(Func&& func, const std::string& name) {
    auto start = std::chrono::high_resolution_clock::now();
    auto result = func();
    auto end = std::chrono::high_resolution_clock::now();

    auto duration = std::chrono::duration_cast<std::chrono::microseconds>(end - start);
    std::cout << name << ": " << duration.count() << " us" << std::endl;

    return result;
}

void printSeparator(const std::string& title) {
    std::cout << "\n" << std::string(60, '=') << "\n";
    std::cout << " " << title << "\n";
    std::cout << std::string(60, '=') << "\n\n";
}

void demonstrateSequence() {
    printSeparator("Sequence Operations");

    // Create a sequence
    Sequence seq("ATGCGATCGATCGATCGATCGATCGATCGATCGATCG", "demo_seq_1");

    std::cout << "Sequence: " << seq.bases() << "\n";
    std::cout << "Length: " << seq.length() << "\n";
    std::cout << "ID: " << (seq.id() ? *seq.id() : "none") << "\n\n";

    // Content analysis
    std::cout << "GC Content: " << std::fixed << std::setprecision(2)
              << (seq.gcContent() * 100) << "%\n";
    std::cout << "AT Content: " << (seq.atContent() * 100) << "%\n\n";

    // Base composition
    auto composition = seq.baseComposition();
    std::cout << "Base Composition:\n";
    std::cout << "  A: " << composition[0] << "\n";
    std::cout << "  C: " << composition[1] << "\n";
    std::cout << "  G: " << composition[2] << "\n";
    std::cout << "  T: " << composition[3] << "\n";
    std::cout << "  N: " << composition[4] << "\n\n";

    // Transformations
    auto comp = seq.complement();
    auto rc = seq.reverseComplement();

    std::cout << "Original:           " << seq.bases().substr(0, 30) << "...\n";
    std::cout << "Complement:         " << comp.bases().substr(0, 30) << "...\n";
    std::cout << "Reverse Complement: " << rc.bases().substr(0, 30) << "...\n\n";

    // Motif finding
    std::string motif = "GATC";
    auto positions = seq.findMotifPositions(motif);
    std::cout << "Motif '" << motif << "' found at positions: ";
    for (size_t i = 0; i < std::min(positions.size(), size_t{5}); ++i) {
        std::cout << positions[i];
        if (i < 4 && i < positions.size() - 1) std::cout << ", ";
    }
    if (positions.size() > 5) std::cout << "...";
    std::cout << " (" << positions.size() << " total)\n";
}

void demonstrateKMerCounting() {
    printSeparator("K-mer Counting");

    // Generate a longer sequence for meaningful k-mer analysis
    std::string bases;
    for (int i = 0; i < 1000; ++i) {
        bases += "ATGCGATCGATCGATCGATCGATCG";
    }
    Sequence seq(bases);

    std::cout << "Sequence length: " << seq.length() << " bp\n\n";

    // Count k-mers with different k values
    for (size_t k : {5, 11, 21}) {
        auto counter = measureTime([&]() {
            KMerCounter c(k);
            c.count(seq);
            return c;
        }, "K=" + std::to_string(k) + " counting");

        std::cout << "  Unique " << k << "-mers: " << counter.uniqueCount() << "\n";
        std::cout << "  Total " << k << "-mers: " << counter.totalCount() << "\n";

        auto top = counter.mostFrequent(3);
        std::cout << "  Top 3: ";
        for (const auto& entry : top) {
            std::cout << entry.kmer << "(" << entry.count << ") ";
        }
        std::cout << "\n\n";
    }

    // K-mer spectrum
    KMerCounter counter(21);
    counter.count(seq);
    auto spectrum = counter.spectrum();

    std::cout << "K-mer Spectrum (k=21):\n";
    std::cout << "  Unique k-mers: " << spectrum.unique_kmers << "\n";
    std::cout << "  Singletons: " << spectrum.singleton_count << "\n";
    std::cout << "  Complexity: " << std::setprecision(4) << spectrum.complexity << "\n";
}

void demonstrateAlignment() {
    printSeparator("Sequence Alignment");

    // Test sequences
    Sequence seq1("ACGTACGTACGTACGT");
    Sequence seq2("ACGTTCGTACGTACGT");

    std::cout << "Sequence 1: " << seq1.bases() << "\n";
    std::cout << "Sequence 2: " << seq2.bases() << "\n\n";

    // Smith-Waterman (local alignment)
    auto sw_result = measureTime([&]() {
        return smithWaterman(seq1, seq2);
    }, "Smith-Waterman");

    std::cout << "Local Alignment Score: " << sw_result.score << "\n";
    std::cout << "Identity: " << (sw_result.identity() * 100) << "%\n";
    std::cout << "Aligned 1: " << sw_result.aligned_seq1 << "\n";
    std::cout << "Aligned 2: " << sw_result.aligned_seq2 << "\n";
    std::cout << "CIGAR: " << sw_result.cigar() << "\n\n";

    // Needleman-Wunsch (global alignment)
    auto nw_result = measureTime([&]() {
        return needlemanWunsch(seq1, seq2);
    }, "Needleman-Wunsch");

    std::cout << "Global Alignment Score: " << nw_result.score << "\n";
    std::cout << "Aligned 1: " << nw_result.aligned_seq1 << "\n";
    std::cout << "Aligned 2: " << nw_result.aligned_seq2 << "\n\n";

    // Edit distance
    auto edit_dist = editDistance(seq1, seq2);
    std::cout << "Edit Distance: " << edit_dist << "\n";

    // Longer alignment for benchmarking
    std::string long_bases1, long_bases2;
    for (int i = 0; i < 100; ++i) {
        long_bases1 += "ACGTACGT";
        long_bases2 += "ACGTTCGT";
    }
    Sequence long_seq1(long_bases1);
    Sequence long_seq2(long_bases2);

    std::cout << "\nLonger sequences (" << long_seq1.length() << " bp):\n";

    measureTime([&]() {
        return smithWaterman(long_seq1, long_seq2);
    }, "Smith-Waterman (long)");
}

void demonstrateQuality() {
    printSeparator("Quality Score Analysis");

    // Sample quality string (Phred+33)
    std::string quality_str = "IIIIIIIIIIIIIIIIIIIIIIIIIIIII"
                              "HHHHHHHHHHHHHHHHHHH555555555"
                              "22222222222BBBBBBB";

    QualityScores quality(quality_str, QualityEncoding::Phred33);

    std::cout << "Quality string length: " << quality.length() << "\n";
    std::cout << "Mean quality: " << std::fixed << std::setprecision(2)
              << quality.meanQuality() << "\n";
    std::cout << "Median quality: " << quality.medianQuality() << "\n";
    std::cout << "Min quality: " << static_cast<int>(quality.minQuality()) << "\n";
    std::cout << "Max quality: " << static_cast<int>(quality.maxQuality()) << "\n";
    std::cout << "Std deviation: " << quality.standardDeviation() << "\n\n";

    std::cout << "Bases with Q >= 20: " << quality.countAboveThreshold(20)
              << " (" << (quality.fractionAboveThreshold(20) * 100) << "%)\n";
    std::cout << "Bases with Q >= 30: " << quality.countAboveThreshold(30)
              << " (" << (quality.fractionAboveThreshold(30) * 100) << "%)\n\n";

    std::cout << "Mean error probability: " << std::scientific
              << quality.meanErrorProbability() << "\n";

    // Trimming
    auto [trim_start, trim_end] = quality.trimPositions(20, 10);
    std::cout << "\nTrim positions (Q >= 20): " << trim_start << " to " << trim_end << "\n";
}

void demonstrateStatistics() {
    printSeparator("Statistical Analysis");

    // Create multiple sequences
    std::vector<Sequence> sequences;
    std::vector<std::string> base_patterns = {
        "ATGCGATCGATCGATCG",
        "GCGCGCGCGCGCGCGCGCGC",
        "ATATATATATATATATAT",
        "ACGTACGTACGTACGTACGT"
    };

    for (const auto& pattern : base_patterns) {
        std::string bases;
        for (int i = 0; i < 100; ++i) bases += pattern;
        sequences.emplace_back(bases);
    }

    // Collection statistics
    auto coll_stats = stats::computeCollectionStats(sequences);

    std::cout << "Collection Statistics:\n";
    std::cout << "  Sequences: " << coll_stats.sequence_count << "\n";
    std::cout << "  Total bases: " << coll_stats.total_bases << "\n";
    std::cout << "  Mean length: " << coll_stats.mean_length << "\n";
    std::cout << "  N50: " << coll_stats.n50 << "\n";
    std::cout << "  Mean GC: " << (coll_stats.mean_gc * 100) << "%\n\n";

    // Per-sequence statistics
    std::cout << "Per-sequence Statistics:\n";
    for (size_t i = 0; i < sequences.size(); ++i) {
        auto seq_stats = stats::computeStats(sequences[i]);
        std::cout << "  Seq " << (i+1) << ": "
                  << "GC=" << std::setprecision(2) << (seq_stats.gc_content * 100) << "%, "
                  << "Complexity=" << std::setprecision(3) << seq_stats.complexity << ", "
                  << "Entropy=" << stats::shannonEntropy(sequences[i]) << "\n";
    }

    // K-mer diversity comparison
    std::cout << "\nK-mer Diversity Comparison (k=5):\n";
    std::vector<KMerCounter> counters;
    for (const auto& seq : sequences) {
        KMerCounter counter(5);
        counter.count(seq);
        counters.push_back(std::move(counter));
    }

    for (size_t i = 0; i < counters.size(); ++i) {
        auto kmer_stats = stats::computeKMerStats(counters[i]);
        std::cout << "  Seq " << (i+1) << ": "
                  << "Unique=" << kmer_stats.unique_kmers << ", "
                  << "Simpson=" << std::setprecision(4) << kmer_stats.simpson_index << ", "
                  << "Shannon=" << kmer_stats.shannon_index << "\n";
    }

    // Pairwise similarities
    std::cout << "\nPairwise Jaccard Similarities:\n";
    for (size_t i = 0; i < counters.size(); ++i) {
        for (size_t j = i + 1; j < counters.size(); ++j) {
            double jaccard = stats::jaccardSimilarity(counters[i], counters[j]);
            std::cout << "  Seq " << (i+1) << " vs Seq " << (j+1) << ": "
                      << std::setprecision(3) << jaccard << "\n";
        }
    }
}

void runBenchmarks() {
    printSeparator("Performance Benchmarks");

    // Generate test data
    std::string bases;
    for (int i = 0; i < 5000; ++i) bases += "ATGC";
    Sequence seq(bases);

    std::cout << "Sequence length: " << seq.length() << " bp\n\n";

    // GC Content benchmark
    std::cout << "GC Content (10000 iterations):\n";
    auto gc_start = std::chrono::high_resolution_clock::now();
    for (int i = 0; i < 10000; ++i) {
        volatile auto gc = seq.gcContent();
        (void)gc;
    }
    auto gc_end = std::chrono::high_resolution_clock::now();
    auto gc_duration = std::chrono::duration_cast<std::chrono::microseconds>(gc_end - gc_start);
    std::cout << "  Total: " << gc_duration.count() << " us\n";
    std::cout << "  Per iteration: " << (gc_duration.count() / 10000.0) << " us\n\n";

    // K-mer counting benchmark
    std::cout << "K-mer Counting (k=21):\n";
    auto kmer_start = std::chrono::high_resolution_clock::now();
    for (int i = 0; i < 100; ++i) {
        KMerCounter counter(21);
        counter.count(seq);
    }
    auto kmer_end = std::chrono::high_resolution_clock::now();
    auto kmer_duration = std::chrono::duration_cast<std::chrono::milliseconds>(kmer_end - kmer_start);
    std::cout << "  Total (100 iterations): " << kmer_duration.count() << " ms\n";
    std::cout << "  Per iteration: " << (kmer_duration.count() / 100.0) << " ms\n\n";

    // Alignment benchmark
    std::string align_bases1, align_bases2;
    for (int i = 0; i < 250; ++i) {
        align_bases1 += "ACGT";
        align_bases2 += "AGCT";
    }
    Sequence align_seq1(align_bases1);
    Sequence align_seq2(align_bases2);

    std::cout << "Smith-Waterman (" << align_seq1.length() << " x "
              << align_seq2.length() << "):\n";
    auto sw_start = std::chrono::high_resolution_clock::now();
    for (int i = 0; i < 10; ++i) {
        auto alignment = smithWaterman(align_seq1, align_seq2);
        (void)alignment;
    }
    auto sw_end = std::chrono::high_resolution_clock::now();
    auto sw_duration = std::chrono::duration_cast<std::chrono::milliseconds>(sw_end - sw_start);
    std::cout << "  Total (10 iterations): " << sw_duration.count() << " ms\n";
    std::cout << "  Per iteration: " << (sw_duration.count() / 10.0) << " ms\n";
}

int main(int argc, char* argv[]) {
    std::cout << "BioFlow C++20 - Bioinformatics Library Demo\n";
    std::cout << "============================================\n";

    bool run_benchmarks = false;
    for (int i = 1; i < argc; ++i) {
        if (std::string(argv[i]) == "--benchmark" || std::string(argv[i]) == "-b") {
            run_benchmarks = true;
        }
    }

    try {
        demonstrateSequence();
        demonstrateKMerCounting();
        demonstrateAlignment();
        demonstrateQuality();
        demonstrateStatistics();

        if (run_benchmarks) {
            runBenchmarks();
        }

        std::cout << "\n" << std::string(60, '=') << "\n";
        std::cout << " All demonstrations completed successfully!\n";
        std::cout << std::string(60, '=') << "\n";

    } catch (const std::exception& e) {
        std::cerr << "Error: " << e.what() << std::endl;
        return 1;
    }

    return 0;
}
