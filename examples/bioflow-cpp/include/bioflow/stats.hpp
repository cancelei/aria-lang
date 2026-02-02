#pragma once

#include "bioflow/sequence.hpp"
#include "bioflow/kmer.hpp"
#include <vector>
#include <cmath>
#include <numeric>
#include <algorithm>
#include <ranges>
#include <concepts>
#include <optional>

namespace bioflow {

/**
 * @brief Statistical functions for sequence analysis
 */
namespace stats {

// ============================================================================
// Basic Statistical Functions (Generic)
// ============================================================================

/**
 * @brief Compute mean of a range of values
 */
template<std::ranges::range R>
    requires std::floating_point<std::ranges::range_value_t<R>> ||
             std::integral<std::ranges::range_value_t<R>>
[[nodiscard]] double mean(R&& values) {
    auto begin = std::ranges::begin(values);
    auto end = std::ranges::end(values);

    if (begin == end) return 0.0;

    double sum = 0.0;
    size_t count = 0;
    for (auto it = begin; it != end; ++it) {
        sum += static_cast<double>(*it);
        ++count;
    }

    return sum / count;
}

/**
 * @brief Compute variance of a range of values
 */
template<std::ranges::range R>
    requires std::floating_point<std::ranges::range_value_t<R>> ||
             std::integral<std::ranges::range_value_t<R>>
[[nodiscard]] double variance(R&& values) {
    auto begin = std::ranges::begin(values);
    auto end = std::ranges::end(values);

    if (begin == end) return 0.0;

    double m = mean(values);
    double sum_sq = 0.0;
    size_t count = 0;

    for (auto it = begin; it != end; ++it) {
        double diff = static_cast<double>(*it) - m;
        sum_sq += diff * diff;
        ++count;
    }

    return count > 1 ? sum_sq / (count - 1) : 0.0;
}

/**
 * @brief Compute standard deviation of a range of values
 */
template<std::ranges::range R>
    requires std::floating_point<std::ranges::range_value_t<R>> ||
             std::integral<std::ranges::range_value_t<R>>
[[nodiscard]] double standardDeviation(R&& values) {
    return std::sqrt(variance(std::forward<R>(values)));
}

/**
 * @brief Compute median of a range of values
 */
template<std::ranges::range R>
    requires std::floating_point<std::ranges::range_value_t<R>> ||
             std::integral<std::ranges::range_value_t<R>>
[[nodiscard]] double median(R&& values) {
    std::vector<double> sorted;
    for (const auto& v : values) {
        sorted.push_back(static_cast<double>(v));
    }

    if (sorted.empty()) return 0.0;

    std::ranges::sort(sorted);
    size_t n = sorted.size();

    if (n % 2 == 0) {
        return (sorted[n/2 - 1] + sorted[n/2]) / 2.0;
    } else {
        return sorted[n/2];
    }
}

/**
 * @brief Compute percentile of a range of values
 * @param values Input range
 * @param p Percentile (0-100)
 */
template<std::ranges::range R>
    requires std::floating_point<std::ranges::range_value_t<R>> ||
             std::integral<std::ranges::range_value_t<R>>
[[nodiscard]] double percentile(R&& values, double p) {
    std::vector<double> sorted;
    for (const auto& v : values) {
        sorted.push_back(static_cast<double>(v));
    }

    if (sorted.empty()) return 0.0;

    std::ranges::sort(sorted);

    double index = (p / 100.0) * (sorted.size() - 1);
    size_t lower = static_cast<size_t>(std::floor(index));
    size_t upper = static_cast<size_t>(std::ceil(index));

    if (lower == upper) {
        return sorted[lower];
    }

    double fraction = index - lower;
    return sorted[lower] * (1 - fraction) + sorted[upper] * fraction;
}

// ============================================================================
// Sequence-Specific Statistics
// ============================================================================

/**
 * @brief Comprehensive sequence statistics
 */
struct SequenceStats {
    size_t length;
    double gc_content;
    double at_content;
    size_t count_a;
    size_t count_c;
    size_t count_g;
    size_t count_t;
    size_t count_n;
    double complexity;  // Linguistic complexity

    [[nodiscard]] double purine_ratio() const noexcept {
        size_t purines = count_a + count_g;
        size_t pyrimidines = count_c + count_t;
        return pyrimidines > 0 ? static_cast<double>(purines) / pyrimidines : 0.0;
    }
};

/**
 * @brief Compute comprehensive statistics for a sequence
 */
[[nodiscard]] SequenceStats computeStats(const Sequence& seq);

/**
 * @brief Compute linguistic complexity (ratio of observed to possible k-mers)
 */
[[nodiscard]] double linguisticComplexity(const Sequence& seq, size_t k = 3);

/**
 * @brief Shannon entropy of base composition
 */
[[nodiscard]] double shannonEntropy(const Sequence& seq);

/**
 * @brief Dinucleotide frequencies
 */
[[nodiscard]] std::unordered_map<std::string, double> dinucleotideFrequencies(
    const Sequence& seq
);

/**
 * @brief CpG observed/expected ratio
 */
[[nodiscard]] double cpgRatio(const Sequence& seq);

// ============================================================================
// Collection Statistics
// ============================================================================

/**
 * @brief Statistics for a collection of sequences
 */
struct CollectionStats {
    size_t sequence_count;
    size_t total_bases;
    double mean_length;
    double median_length;
    double std_length;
    size_t min_length;
    size_t max_length;
    double mean_gc;
    double std_gc;
    size_t n50;  // N50 statistic
    size_t l50;  // L50 statistic
};

/**
 * @brief Compute statistics for a collection of sequences
 */
[[nodiscard]] CollectionStats computeCollectionStats(
    const std::vector<Sequence>& sequences
);

/**
 * @brief Compute N50 and L50 statistics
 * @param lengths Vector of sequence lengths
 * @return Pair of (N50, L50)
 */
[[nodiscard]] std::pair<size_t, size_t> computeN50L50(
    std::vector<size_t> lengths
);

// ============================================================================
// K-mer Statistics
// ============================================================================

/**
 * @brief K-mer frequency statistics
 */
struct KMerStats {
    size_t k;
    size_t unique_kmers;
    size_t total_kmers;
    size_t theoretical_max;  // 4^k
    double coverage;         // unique/theoretical
    double simpson_index;    // Simpson's diversity index
    double shannon_index;    // Shannon diversity index
    size_t singleton_count;
    size_t doubleton_count;
};

/**
 * @brief Compute k-mer diversity statistics
 */
[[nodiscard]] KMerStats computeKMerStats(const KMerCounter& counter);

/**
 * @brief Simpson's diversity index for k-mers
 */
[[nodiscard]] double simpsonIndex(const KMerCounter& counter);

/**
 * @brief Shannon diversity index for k-mers
 */
[[nodiscard]] double shannonIndex(const KMerCounter& counter);

// ============================================================================
// Comparative Statistics
// ============================================================================

/**
 * @brief Jaccard similarity between two k-mer sets
 */
[[nodiscard]] double jaccardSimilarity(
    const KMerCounter& counter1,
    const KMerCounter& counter2
);

/**
 * @brief Cosine similarity between two k-mer frequency vectors
 */
[[nodiscard]] double cosineSimilarity(
    const KMerCounter& counter1,
    const KMerCounter& counter2
);

/**
 * @brief Bray-Curtis dissimilarity between two k-mer profiles
 */
[[nodiscard]] double brayCurtisDissimilarity(
    const KMerCounter& counter1,
    const KMerCounter& counter2
);

// ============================================================================
// Histogram and Distribution
// ============================================================================

/**
 * @brief Histogram bin
 */
struct HistogramBin {
    double lower;
    double upper;
    size_t count;
};

/**
 * @brief Create histogram from values
 */
template<std::ranges::range R>
    requires std::floating_point<std::ranges::range_value_t<R>> ||
             std::integral<std::ranges::range_value_t<R>>
[[nodiscard]] std::vector<HistogramBin> histogram(R&& values, size_t num_bins) {
    std::vector<double> vec;
    for (const auto& v : values) {
        vec.push_back(static_cast<double>(v));
    }

    if (vec.empty() || num_bins == 0) return {};

    auto [min_it, max_it] = std::ranges::minmax_element(vec);
    double min_val = *min_it;
    double max_val = *max_it;

    if (min_val == max_val) {
        return {{min_val, max_val, vec.size()}};
    }

    double bin_width = (max_val - min_val) / num_bins;
    std::vector<HistogramBin> bins(num_bins);

    for (size_t i = 0; i < num_bins; ++i) {
        bins[i].lower = min_val + i * bin_width;
        bins[i].upper = min_val + (i + 1) * bin_width;
        bins[i].count = 0;
    }

    for (double v : vec) {
        size_t bin_idx = static_cast<size_t>((v - min_val) / bin_width);
        if (bin_idx >= num_bins) bin_idx = num_bins - 1;
        bins[bin_idx].count++;
    }

    return bins;
}

} // namespace stats
} // namespace bioflow
