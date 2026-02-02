#pragma once

#include "bioflow/sequence.hpp"
#include <unordered_map>
#include <vector>
#include <utility>
#include <span>
#include <ranges>
#include <functional>

namespace bioflow {

/**
 * @brief Exception class for k-mer related errors
 */
class KMerError : public std::runtime_error {
public:
    using std::runtime_error::runtime_error;
};

/**
 * @brief Represents a single k-mer with its frequency
 */
struct KMerEntry {
    std::string kmer;
    size_t count;

    [[nodiscard]] double frequency(size_t total) const noexcept {
        return total > 0 ? static_cast<double>(count) / total : 0.0;
    }

    [[nodiscard]] bool operator<(const KMerEntry& other) const noexcept {
        return count < other.count;
    }

    [[nodiscard]] bool operator>(const KMerEntry& other) const noexcept {
        return count > other.count;
    }
};

/**
 * @brief K-mer spectrum statistics
 */
struct KMerSpectrum {
    size_t k;
    size_t unique_kmers;
    size_t total_kmers;
    size_t singleton_count;  // k-mers appearing exactly once
    double complexity;       // unique/total ratio

    [[nodiscard]] double singletonRatio() const noexcept {
        return unique_kmers > 0 ? static_cast<double>(singleton_count) / unique_kmers : 0.0;
    }
};

/**
 * @brief Efficient k-mer counting using modern C++20 features
 *
 * This class provides high-performance k-mer counting with:
 * - Hash-based storage for O(1) lookup
 * - Ranges support for iteration
 * - Parallel counting capability
 * - Memory-efficient design
 */
class KMerCounter {
public:
    using CountMap = std::unordered_map<std::string, size_t>;
    using const_iterator = CountMap::const_iterator;

    /**
     * @brief Construct a k-mer counter with specified k
     * @param k The k-mer length (must be > 0)
     * @throws KMerError if k is 0
     */
    explicit KMerCounter(size_t k);

    /**
     * @brief Count all k-mers in a sequence
     * @param seq The sequence to analyze
     */
    void count(const Sequence& seq);

    /**
     * @brief Count k-mers from multiple sequences
     * @param sequences Range of sequences
     */
    template<std::ranges::range R>
        requires std::same_as<std::ranges::range_value_t<R>, Sequence>
    void countAll(R&& sequences) {
        for (const auto& seq : sequences) {
            count(seq);
        }
    }

    /**
     * @brief Count k-mers from raw string
     * @param bases The DNA string
     */
    void countRaw(std::string_view bases);

    /**
     * @brief Get count for a specific k-mer
     * @param kmer The k-mer to look up
     * @return The count (0 if not found)
     */
    [[nodiscard]] size_t getCount(std::string_view kmer) const;

    /**
     * @brief Check if a k-mer exists
     * @param kmer The k-mer to check
     * @return true if k-mer has been counted
     */
    [[nodiscard]] bool contains(std::string_view kmer) const;

    /**
     * @brief Get the n most frequent k-mers
     * @param n Number of k-mers to return
     * @return Vector of (k-mer, count) pairs sorted by frequency
     */
    [[nodiscard]] std::vector<KMerEntry> mostFrequent(size_t n) const;

    /**
     * @brief Get the n least frequent k-mers
     * @param n Number of k-mers to return
     * @return Vector of (k-mer, count) pairs sorted by frequency
     */
    [[nodiscard]] std::vector<KMerEntry> leastFrequent(size_t n) const;

    /**
     * @brief Get k-mers with count >= threshold
     * @param threshold Minimum count
     * @return Vector of qualifying k-mers
     */
    [[nodiscard]] std::vector<KMerEntry> aboveThreshold(size_t threshold) const;

    /**
     * @brief Compute k-mer spectrum statistics
     * @return KMerSpectrum with various metrics
     */
    [[nodiscard]] KMerSpectrum spectrum() const;

    /**
     * @brief Get all k-mers as a vector
     * @return Vector of all k-mer entries
     */
    [[nodiscard]] std::vector<KMerEntry> allKmers() const;

    // Accessors
    [[nodiscard]] size_t uniqueCount() const noexcept { return counts_.size(); }
    [[nodiscard]] size_t totalCount() const noexcept { return total_; }
    [[nodiscard]] size_t k() const noexcept { return k_; }
    [[nodiscard]] bool empty() const noexcept { return counts_.empty(); }

    // Iterator support
    [[nodiscard]] const_iterator begin() const noexcept { return counts_.begin(); }
    [[nodiscard]] const_iterator end() const noexcept { return counts_.end(); }
    [[nodiscard]] const_iterator cbegin() const noexcept { return counts_.cbegin(); }
    [[nodiscard]] const_iterator cend() const noexcept { return counts_.cend(); }

    // Reset
    void clear() noexcept;

    // Merge another counter
    void merge(const KMerCounter& other);

private:
    size_t k_;
    CountMap counts_;
    size_t total_ = 0;

    [[nodiscard]] static bool hasAmbiguous(std::string_view kmer) noexcept;
};

/**
 * @brief Compute canonical k-mer (lexicographically smaller of k-mer and its reverse complement)
 * @param kmer The k-mer string
 * @return The canonical form
 */
[[nodiscard]] std::string canonicalKmer(std::string_view kmer);

/**
 * @brief Count k-mers using canonical representation
 */
class CanonicalKMerCounter {
public:
    explicit CanonicalKMerCounter(size_t k);

    void count(const Sequence& seq);
    [[nodiscard]] size_t getCount(std::string_view kmer) const;
    [[nodiscard]] std::vector<KMerEntry> mostFrequent(size_t n) const;
    [[nodiscard]] size_t uniqueCount() const noexcept { return counts_.size(); }
    [[nodiscard]] size_t totalCount() const noexcept { return total_; }
    [[nodiscard]] size_t k() const noexcept { return k_; }

private:
    size_t k_;
    std::unordered_map<std::string, size_t> counts_;
    size_t total_ = 0;
};

} // namespace bioflow
