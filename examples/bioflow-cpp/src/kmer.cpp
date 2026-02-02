#include "bioflow/kmer.hpp"
#include <algorithm>
#include <stdexcept>

namespace bioflow {

// ============================================================================
// KMerCounter Implementation
// ============================================================================

KMerCounter::KMerCounter(size_t k) : k_(k) {
    if (k == 0) {
        throw KMerError("K-mer length must be greater than 0");
    }
}

void KMerCounter::count(const Sequence& seq) {
    countRaw(seq.bases());
}

void KMerCounter::countRaw(std::string_view bases) {
    if (bases.length() < k_) return;

    for (size_t i = 0; i <= bases.length() - k_; ++i) {
        auto kmer = std::string(bases.substr(i, k_));

        // Skip if contains N (ambiguous base)
        if (hasAmbiguous(kmer)) continue;

        counts_[std::move(kmer)]++;
        total_++;
    }
}

bool KMerCounter::hasAmbiguous(std::string_view kmer) noexcept {
    return kmer.find('N') != std::string_view::npos;
}

size_t KMerCounter::getCount(std::string_view kmer) const {
    auto it = counts_.find(std::string(kmer));
    return it != counts_.end() ? it->second : 0;
}

bool KMerCounter::contains(std::string_view kmer) const {
    return counts_.find(std::string(kmer)) != counts_.end();
}

std::vector<KMerEntry> KMerCounter::mostFrequent(size_t n) const {
    std::vector<KMerEntry> result;
    result.reserve(counts_.size());

    for (const auto& [kmer, count] : counts_) {
        result.push_back({kmer, count});
    }

    // Partial sort to get top n efficiently
    auto end_it = result.begin() + std::min(n, result.size());
    std::partial_sort(result.begin(), end_it, result.end(),
                     [](const KMerEntry& a, const KMerEntry& b) {
                         return a.count > b.count;
                     });

    result.resize(std::min(n, result.size()));
    return result;
}

std::vector<KMerEntry> KMerCounter::leastFrequent(size_t n) const {
    std::vector<KMerEntry> result;
    result.reserve(counts_.size());

    for (const auto& [kmer, count] : counts_) {
        result.push_back({kmer, count});
    }

    auto end_it = result.begin() + std::min(n, result.size());
    std::partial_sort(result.begin(), end_it, result.end(),
                     [](const KMerEntry& a, const KMerEntry& b) {
                         return a.count < b.count;
                     });

    result.resize(std::min(n, result.size()));
    return result;
}

std::vector<KMerEntry> KMerCounter::aboveThreshold(size_t threshold) const {
    std::vector<KMerEntry> result;

    for (const auto& [kmer, count] : counts_) {
        if (count >= threshold) {
            result.push_back({kmer, count});
        }
    }

    // Sort by count descending
    std::ranges::sort(result, [](const KMerEntry& a, const KMerEntry& b) {
        return a.count > b.count;
    });

    return result;
}

KMerSpectrum KMerCounter::spectrum() const {
    KMerSpectrum spec;
    spec.k = k_;
    spec.unique_kmers = counts_.size();
    spec.total_kmers = total_;
    spec.singleton_count = 0;

    for (const auto& [kmer, count] : counts_) {
        if (count == 1) {
            spec.singleton_count++;
        }
    }

    spec.complexity = total_ > 0
        ? static_cast<double>(spec.unique_kmers) / total_
        : 0.0;

    return spec;
}

std::vector<KMerEntry> KMerCounter::allKmers() const {
    std::vector<KMerEntry> result;
    result.reserve(counts_.size());

    for (const auto& [kmer, count] : counts_) {
        result.push_back({kmer, count});
    }

    return result;
}

void KMerCounter::clear() noexcept {
    counts_.clear();
    total_ = 0;
}

void KMerCounter::merge(const KMerCounter& other) {
    if (other.k_ != k_) {
        throw KMerError("Cannot merge k-mer counters with different k values");
    }

    for (const auto& [kmer, count] : other.counts_) {
        counts_[kmer] += count;
        total_ += count;
    }
}

// ============================================================================
// Canonical K-mer Functions
// ============================================================================

std::string canonicalKmer(std::string_view kmer) {
    std::string rc;
    rc.reserve(kmer.length());

    // Compute reverse complement
    for (auto it = kmer.rbegin(); it != kmer.rend(); ++it) {
        switch (*it) {
            case 'A': rc += 'T'; break;
            case 'T': rc += 'A'; break;
            case 'C': rc += 'G'; break;
            case 'G': rc += 'C'; break;
            default: rc += 'N'; break;
        }
    }

    // Return lexicographically smaller
    return std::string(kmer) < rc ? std::string(kmer) : rc;
}

// ============================================================================
// CanonicalKMerCounter Implementation
// ============================================================================

CanonicalKMerCounter::CanonicalKMerCounter(size_t k) : k_(k) {
    if (k == 0) {
        throw KMerError("K-mer length must be greater than 0");
    }
}

void CanonicalKMerCounter::count(const Sequence& seq) {
    const auto& bases = seq.bases();
    if (bases.length() < k_) return;

    for (size_t i = 0; i <= bases.length() - k_; ++i) {
        auto kmer = bases.substr(i, k_);

        // Skip if contains N
        if (kmer.find('N') != std::string::npos) continue;

        auto canonical = canonicalKmer(kmer);
        counts_[std::move(canonical)]++;
        total_++;
    }
}

size_t CanonicalKMerCounter::getCount(std::string_view kmer) const {
    auto canonical = canonicalKmer(kmer);
    auto it = counts_.find(canonical);
    return it != counts_.end() ? it->second : 0;
}

std::vector<KMerEntry> CanonicalKMerCounter::mostFrequent(size_t n) const {
    std::vector<KMerEntry> result;
    result.reserve(counts_.size());

    for (const auto& [kmer, count] : counts_) {
        result.push_back({kmer, count});
    }

    auto end_it = result.begin() + std::min(n, result.size());
    std::partial_sort(result.begin(), end_it, result.end(),
                     [](const KMerEntry& a, const KMerEntry& b) {
                         return a.count > b.count;
                     });

    result.resize(std::min(n, result.size()));
    return result;
}

} // namespace bioflow
