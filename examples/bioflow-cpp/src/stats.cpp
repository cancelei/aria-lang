#include "bioflow/stats.hpp"
#include <cmath>
#include <numeric>
#include <set>

namespace bioflow {
namespace stats {

// ============================================================================
// Sequence Statistics
// ============================================================================

SequenceStats computeStats(const Sequence& seq) {
    SequenceStats stats{};

    stats.length = seq.length();
    stats.gc_content = seq.gcContent();
    stats.at_content = seq.atContent();

    auto composition = seq.baseComposition();
    stats.count_a = composition[0];
    stats.count_c = composition[1];
    stats.count_g = composition[2];
    stats.count_t = composition[3];
    stats.count_n = composition[4];

    // Compute linguistic complexity (using k=3 by default)
    stats.complexity = linguisticComplexity(seq, 3);

    return stats;
}

double linguisticComplexity(const Sequence& seq, size_t k) {
    if (seq.length() < k) return 0.0;

    KMerCounter counter(k);
    counter.count(seq);

    // Theoretical maximum for sequence of this length
    size_t max_possible = std::min(
        static_cast<size_t>(std::pow(4, k)),  // 4^k possible k-mers
        seq.length() - k + 1                    // k-mers in sequence
    );

    return max_possible > 0
        ? static_cast<double>(counter.uniqueCount()) / max_possible
        : 0.0;
}

double shannonEntropy(const Sequence& seq) {
    if (seq.empty()) return 0.0;

    auto composition = seq.baseComposition();
    double entropy = 0.0;
    double n = static_cast<double>(seq.length());

    for (size_t i = 0; i < 4; ++i) {  // Only A, C, G, T
        if (composition[i] > 0) {
            double p = composition[i] / n;
            entropy -= p * std::log2(p);
        }
    }

    return entropy;
}

std::unordered_map<std::string, double> dinucleotideFrequencies(const Sequence& seq) {
    std::unordered_map<std::string, double> freqs;

    if (seq.length() < 2) return freqs;

    KMerCounter counter(2);
    counter.count(seq);

    for (const auto& entry : counter) {
        freqs[entry.first] = static_cast<double>(entry.second) / counter.totalCount();
    }

    return freqs;
}

double cpgRatio(const Sequence& seq) {
    if (seq.length() < 2) return 0.0;

    const auto& bases = seq.bases();
    size_t cpg_count = 0;
    size_t c_count = 0;
    size_t g_count = 0;

    for (size_t i = 0; i < bases.length(); ++i) {
        if (bases[i] == 'C') {
            c_count++;
            if (i + 1 < bases.length() && bases[i + 1] == 'G') {
                cpg_count++;
            }
        } else if (bases[i] == 'G') {
            g_count++;
        }
    }

    // CpG O/E = (CpG count * length) / (C count * G count)
    if (c_count == 0 || g_count == 0) return 0.0;

    double expected = static_cast<double>(c_count * g_count) / bases.length();
    return expected > 0 ? cpg_count / expected : 0.0;
}

// ============================================================================
// Collection Statistics
// ============================================================================

CollectionStats computeCollectionStats(const std::vector<Sequence>& sequences) {
    CollectionStats stats{};

    if (sequences.empty()) return stats;

    stats.sequence_count = sequences.size();

    std::vector<size_t> lengths;
    std::vector<double> gc_values;
    lengths.reserve(sequences.size());
    gc_values.reserve(sequences.size());

    for (const auto& seq : sequences) {
        lengths.push_back(seq.length());
        gc_values.push_back(seq.gcContent());
        stats.total_bases += seq.length();
    }

    stats.mean_length = mean(lengths);
    stats.median_length = median(lengths);
    stats.std_length = standardDeviation(lengths);

    auto [min_it, max_it] = std::minmax_element(lengths.begin(), lengths.end());
    stats.min_length = *min_it;
    stats.max_length = *max_it;

    stats.mean_gc = mean(gc_values);
    stats.std_gc = standardDeviation(gc_values);

    auto [n50, l50] = computeN50L50(lengths);
    stats.n50 = n50;
    stats.l50 = l50;

    return stats;
}

std::pair<size_t, size_t> computeN50L50(std::vector<size_t> lengths) {
    if (lengths.empty()) return {0, 0};

    // Sort in descending order
    std::ranges::sort(lengths, std::greater{});

    size_t total = std::accumulate(lengths.begin(), lengths.end(), size_t{0});
    size_t half = total / 2;

    size_t cumsum = 0;
    size_t l50 = 0;

    for (size_t len : lengths) {
        cumsum += len;
        l50++;
        if (cumsum >= half) {
            return {len, l50};
        }
    }

    return {lengths.back(), lengths.size()};
}

// ============================================================================
// K-mer Statistics
// ============================================================================

KMerStats computeKMerStats(const KMerCounter& counter) {
    KMerStats stats{};

    stats.k = counter.k();
    stats.unique_kmers = counter.uniqueCount();
    stats.total_kmers = counter.totalCount();
    stats.theoretical_max = static_cast<size_t>(std::pow(4, counter.k()));
    stats.coverage = static_cast<double>(stats.unique_kmers) / stats.theoretical_max;

    stats.singleton_count = 0;
    stats.doubleton_count = 0;

    for (const auto& [kmer, count] : counter) {
        if (count == 1) stats.singleton_count++;
        else if (count == 2) stats.doubleton_count++;
    }

    stats.simpson_index = simpsonIndex(counter);
    stats.shannon_index = shannonIndex(counter);

    return stats;
}

double simpsonIndex(const KMerCounter& counter) {
    if (counter.totalCount() < 2) return 0.0;

    double sum = 0.0;
    size_t n = counter.totalCount();

    for (const auto& [kmer, count] : counter) {
        sum += count * (count - 1);
    }

    return 1.0 - sum / (n * (n - 1));
}

double shannonIndex(const KMerCounter& counter) {
    if (counter.empty()) return 0.0;

    double entropy = 0.0;
    double n = static_cast<double>(counter.totalCount());

    for (const auto& [kmer, count] : counter) {
        double p = count / n;
        entropy -= p * std::log(p);
    }

    return entropy;
}

// ============================================================================
// Comparative Statistics
// ============================================================================

double jaccardSimilarity(const KMerCounter& counter1, const KMerCounter& counter2) {
    if (counter1.empty() && counter2.empty()) return 1.0;
    if (counter1.empty() || counter2.empty()) return 0.0;

    std::set<std::string> set1, set2;

    for (const auto& [kmer, count] : counter1) {
        set1.insert(kmer);
    }
    for (const auto& [kmer, count] : counter2) {
        set2.insert(kmer);
    }

    size_t intersection = 0;
    for (const auto& kmer : set1) {
        if (set2.count(kmer)) intersection++;
    }

    size_t union_size = set1.size() + set2.size() - intersection;
    return union_size > 0 ? static_cast<double>(intersection) / union_size : 0.0;
}

double cosineSimilarity(const KMerCounter& counter1, const KMerCounter& counter2) {
    if (counter1.empty() || counter2.empty()) return 0.0;

    // Get all k-mers
    std::set<std::string> all_kmers;
    for (const auto& [kmer, count] : counter1) {
        all_kmers.insert(kmer);
    }
    for (const auto& [kmer, count] : counter2) {
        all_kmers.insert(kmer);
    }

    double dot_product = 0.0;
    double norm1 = 0.0;
    double norm2 = 0.0;

    for (const auto& kmer : all_kmers) {
        double c1 = static_cast<double>(counter1.getCount(kmer));
        double c2 = static_cast<double>(counter2.getCount(kmer));

        dot_product += c1 * c2;
        norm1 += c1 * c1;
        norm2 += c2 * c2;
    }

    double denom = std::sqrt(norm1) * std::sqrt(norm2);
    return denom > 0 ? dot_product / denom : 0.0;
}

double brayCurtisDissimilarity(const KMerCounter& counter1, const KMerCounter& counter2) {
    if (counter1.empty() && counter2.empty()) return 0.0;

    std::set<std::string> all_kmers;
    for (const auto& [kmer, count] : counter1) {
        all_kmers.insert(kmer);
    }
    for (const auto& [kmer, count] : counter2) {
        all_kmers.insert(kmer);
    }

    double sum_min = 0.0;
    double sum_total = 0.0;

    for (const auto& kmer : all_kmers) {
        double c1 = static_cast<double>(counter1.getCount(kmer));
        double c2 = static_cast<double>(counter2.getCount(kmer));

        sum_min += std::min(c1, c2);
        sum_total += c1 + c2;
    }

    return sum_total > 0 ? 1.0 - (2.0 * sum_min / sum_total) : 0.0;
}

} // namespace stats
} // namespace bioflow
