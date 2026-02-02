#include "bioflow/quality.hpp"
#include <algorithm>
#include <numeric>
#include <cmath>
#include <ranges>

namespace bioflow {

// ============================================================================
// QualityScores Implementation
// ============================================================================

QualityScores::QualityScores(std::string_view quality_string, QualityEncoding encoding) {
    scores_.reserve(quality_string.length());

    for (char c : quality_string) {
        scores_.push_back(asciiToPhred(c, encoding));
    }
}

QualityScores::QualityScores(std::vector<uint8_t> scores)
    : scores_(std::move(scores)) {}

uint8_t QualityScores::asciiToPhred(char c, QualityEncoding encoding) {
    int ascii = static_cast<int>(static_cast<unsigned char>(c));
    int offset;

    switch (encoding) {
        case QualityEncoding::Phred33:
            offset = 33;
            break;
        case QualityEncoding::Phred64:
            offset = 64;
            break;
        case QualityEncoding::Solexa:
            offset = 64;  // Solexa uses same offset but different scale
            break;
    }

    int phred = ascii - offset;

    if (phred < 0) {
        throw QualityError("Invalid quality character for encoding");
    }

    return static_cast<uint8_t>(std::min(phred, 93));
}

char QualityScores::phredToAscii(uint8_t q, QualityEncoding encoding) {
    int offset;

    switch (encoding) {
        case QualityEncoding::Phred33:
            offset = 33;
            break;
        case QualityEncoding::Phred64:
        case QualityEncoding::Solexa:
            offset = 64;
            break;
    }

    return static_cast<char>(q + offset);
}

double QualityScores::meanQuality() const noexcept {
    if (scores_.empty()) return 0.0;

    double sum = std::accumulate(scores_.begin(), scores_.end(), 0.0);
    return sum / scores_.size();
}

double QualityScores::medianQuality() const noexcept {
    if (scores_.empty()) return 0.0;

    std::vector<uint8_t> sorted = scores_;
    std::ranges::sort(sorted);

    size_t n = sorted.size();
    if (n % 2 == 0) {
        return (sorted[n/2 - 1] + sorted[n/2]) / 2.0;
    } else {
        return sorted[n/2];
    }
}

uint8_t QualityScores::minQuality() const noexcept {
    if (scores_.empty()) return 0;
    return *std::ranges::min_element(scores_);
}

uint8_t QualityScores::maxQuality() const noexcept {
    if (scores_.empty()) return 0;
    return *std::ranges::max_element(scores_);
}

double QualityScores::standardDeviation() const noexcept {
    if (scores_.size() < 2) return 0.0;

    double m = meanQuality();
    double sum_sq = 0.0;

    for (uint8_t q : scores_) {
        double diff = q - m;
        sum_sq += diff * diff;
    }

    return std::sqrt(sum_sq / (scores_.size() - 1));
}

size_t QualityScores::countAboveThreshold(uint8_t threshold) const noexcept {
    return static_cast<size_t>(std::ranges::count_if(scores_,
        [threshold](uint8_t q) { return q >= threshold; }));
}

size_t QualityScores::countBelowThreshold(uint8_t threshold) const noexcept {
    return static_cast<size_t>(std::ranges::count_if(scores_,
        [threshold](uint8_t q) { return q < threshold; }));
}

double QualityScores::fractionAboveThreshold(uint8_t threshold) const noexcept {
    if (scores_.empty()) return 0.0;
    return static_cast<double>(countAboveThreshold(threshold)) / scores_.size();
}

double QualityScores::errorProbability(size_t index) const {
    if (index >= scores_.size()) {
        throw QualityError("Index out of range");
    }
    return std::pow(10.0, -static_cast<double>(scores_[index]) / 10.0);
}

double QualityScores::meanErrorProbability() const {
    if (scores_.empty()) return 0.0;

    double sum = 0.0;
    for (uint8_t q : scores_) {
        sum += std::pow(10.0, -static_cast<double>(q) / 10.0);
    }
    return sum / scores_.size();
}

std::vector<double> QualityScores::errorProbabilities() const {
    std::vector<double> probs;
    probs.reserve(scores_.size());

    for (uint8_t q : scores_) {
        probs.push_back(std::pow(10.0, -static_cast<double>(q) / 10.0));
    }

    return probs;
}

std::pair<size_t, size_t> QualityScores::trimPositions(uint8_t threshold,
                                                        size_t min_length) const {
    if (scores_.empty()) return {0, 0};

    // Find first position with quality >= threshold
    size_t start = 0;
    while (start < scores_.size() && scores_[start] < threshold) {
        ++start;
    }

    // Find last position with quality >= threshold
    size_t end = scores_.size();
    while (end > start && scores_[end - 1] < threshold) {
        --end;
    }

    // Ensure minimum length
    if (end - start < min_length) {
        return {0, scores_.size()};  // Return original range
    }

    return {start, end};
}

QualityScores QualityScores::trim(uint8_t threshold, size_t min_length) const {
    auto [start, end] = trimPositions(threshold, min_length);
    return subsequence(start, end - start);
}

std::vector<double> QualityScores::slidingWindowMean(size_t window_size) const {
    if (scores_.empty() || window_size == 0 || window_size > scores_.size()) {
        return {};
    }

    std::vector<double> means;
    means.reserve(scores_.size() - window_size + 1);

    // Calculate first window
    double sum = 0.0;
    for (size_t i = 0; i < window_size; ++i) {
        sum += scores_[i];
    }
    means.push_back(sum / window_size);

    // Slide window
    for (size_t i = window_size; i < scores_.size(); ++i) {
        sum = sum - scores_[i - window_size] + scores_[i];
        means.push_back(sum / window_size);
    }

    return means;
}

std::pair<size_t, size_t> QualityScores::findLowQualityRegion(uint8_t threshold,
                                                               size_t min_length) const {
    size_t best_start = 0, best_length = 0;
    size_t curr_start = 0, curr_length = 0;
    bool in_region = false;

    for (size_t i = 0; i < scores_.size(); ++i) {
        if (scores_[i] < threshold) {
            if (!in_region) {
                curr_start = i;
                curr_length = 0;
                in_region = true;
            }
            curr_length++;
        } else {
            if (in_region && curr_length >= min_length && curr_length > best_length) {
                best_start = curr_start;
                best_length = curr_length;
            }
            in_region = false;
        }
    }

    // Check final region
    if (in_region && curr_length >= min_length && curr_length > best_length) {
        best_start = curr_start;
        best_length = curr_length;
    }

    return {best_start, best_length};
}

std::string QualityScores::toAscii(QualityEncoding encoding) const {
    std::string result;
    result.reserve(scores_.size());

    for (uint8_t q : scores_) {
        result += phredToAscii(q, encoding);
    }

    return result;
}

QualityEncoding QualityScores::detectEncoding(std::string_view quality_string) {
    int min_char = 127, max_char = 0;

    for (char c : quality_string) {
        int val = static_cast<int>(static_cast<unsigned char>(c));
        min_char = std::min(min_char, val);
        max_char = std::max(max_char, val);
    }

    // Phred+33: ASCII 33-126 (! to ~)
    // Phred+64: ASCII 64-126 (@ to ~)
    // Solexa: ASCII 59-126 (; to ~)

    if (min_char < 59) {
        return QualityEncoding::Phred33;
    } else if (min_char < 64) {
        return QualityEncoding::Solexa;
    } else {
        return QualityEncoding::Phred64;
    }
}

QualityScores QualityScores::subsequence(size_t start, size_t length) const {
    if (start >= scores_.size()) {
        return QualityScores(std::vector<uint8_t>{});
    }

    size_t actual_length = std::min(length, scores_.size() - start);
    std::vector<uint8_t> sub(scores_.begin() + start,
                             scores_.begin() + start + actual_length);
    return QualityScores(std::move(sub));
}

// ============================================================================
// QualifiedSequence Implementation
// ============================================================================

bool QualifiedSequence::passesQualityFilter(double min_mean_quality) const {
    return quality.meanQuality() >= min_mean_quality;
}

bool QualifiedSequence::passesLengthFilter(size_t min_length,
                                           std::optional<size_t> max_length) const {
    if (bases.length() < min_length) return false;
    if (max_length && bases.length() > *max_length) return false;
    return true;
}

QualifiedSequence QualifiedSequence::trim(uint8_t quality_threshold,
                                          size_t min_length) const {
    auto [start, end] = quality.trimPositions(quality_threshold, min_length);

    QualifiedSequence result;
    result.id = id;
    result.bases = bases.substr(start, end - start);
    result.quality = quality.subsequence(start, end - start);
    result.description = description;

    return result;
}

// ============================================================================
// Quality Report Functions
// ============================================================================

QualityReport generateQualityReport(const std::vector<QualifiedSequence>& sequences) {
    QualityReport report{};

    if (sequences.empty()) {
        return report;
    }

    report.total_sequences = sequences.size();
    report.quality_distribution.resize(94, 0);  // Q0-Q93

    size_t max_length = 0;
    std::vector<double> lengths;
    std::vector<double> mean_qualities;

    for (const auto& seq : sequences) {
        report.total_bases += seq.bases.length();
        lengths.push_back(static_cast<double>(seq.bases.length()));
        max_length = std::max(max_length, seq.bases.length());

        double mean_q = seq.quality.meanQuality();
        mean_qualities.push_back(mean_q);

        for (size_t i = 0; i < seq.quality.length(); ++i) {
            uint8_t q = seq.quality[i];
            if (q < 94) {
                report.quality_distribution[q]++;
            }
            if (q >= 20) report.bases_above_q20++;
            if (q >= 30) report.bases_above_q30++;
        }
    }

    report.mean_sequence_length = std::accumulate(lengths.begin(), lengths.end(), 0.0) /
                                  lengths.size();
    report.mean_quality = std::accumulate(mean_qualities.begin(), mean_qualities.end(), 0.0) /
                          mean_qualities.size();

    // Median quality
    std::ranges::sort(mean_qualities);
    if (mean_qualities.size() % 2 == 0) {
        report.median_quality = (mean_qualities[mean_qualities.size()/2 - 1] +
                                mean_qualities[mean_qualities.size()/2]) / 2.0;
    } else {
        report.median_quality = mean_qualities[mean_qualities.size()/2];
    }

    // Per-position quality (up to max_length)
    report.per_position_quality.resize(max_length, 0.0);
    std::vector<size_t> position_counts(max_length, 0);

    for (const auto& seq : sequences) {
        for (size_t i = 0; i < seq.quality.length(); ++i) {
            report.per_position_quality[i] += seq.quality[i];
            position_counts[i]++;
        }
    }

    for (size_t i = 0; i < max_length; ++i) {
        if (position_counts[i] > 0) {
            report.per_position_quality[i] /= position_counts[i];
        }
    }

    return report;
}

std::vector<QualifiedSequence> filterByQuality(
    const std::vector<QualifiedSequence>& sequences,
    double min_mean_quality,
    std::optional<size_t> min_length,
    std::optional<size_t> max_length
) {
    std::vector<QualifiedSequence> filtered;

    for (const auto& seq : sequences) {
        if (!seq.passesQualityFilter(min_mean_quality)) continue;
        if (min_length && !seq.passesLengthFilter(*min_length, max_length)) continue;
        if (!min_length && max_length && seq.bases.length() > *max_length) continue;

        filtered.push_back(seq);
    }

    return filtered;
}

} // namespace bioflow
