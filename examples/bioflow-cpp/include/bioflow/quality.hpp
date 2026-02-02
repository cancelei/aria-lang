#pragma once

#include <string>
#include <string_view>
#include <vector>
#include <cstdint>
#include <optional>
#include <stdexcept>
#include <span>

namespace bioflow {

/**
 * @brief Exception class for quality-related errors
 */
class QualityError : public std::runtime_error {
public:
    using std::runtime_error::runtime_error;
};

/**
 * @brief Quality score encoding schemes
 */
enum class QualityEncoding {
    Phred33,    // Sanger/Illumina 1.8+ (ASCII 33-126, Q 0-93)
    Phred64,    // Illumina 1.3-1.7 (ASCII 64-126, Q 0-62)
    Solexa      // Solexa/Illumina 1.0 (ASCII 59-126, Q -5 to 62)
};

/**
 * @brief Represents quality scores for a sequence
 *
 * Handles various quality encoding schemes (Phred33, Phred64, Solexa)
 * with automatic detection and conversion capabilities.
 */
class QualityScores {
public:
    /**
     * @brief Construct from ASCII quality string
     * @param quality_string ASCII-encoded quality scores
     * @param encoding Quality encoding scheme (default: Phred33)
     */
    explicit QualityScores(std::string_view quality_string,
                          QualityEncoding encoding = QualityEncoding::Phred33);

    /**
     * @brief Construct from numeric quality values
     * @param scores Vector of numeric quality scores
     */
    explicit QualityScores(std::vector<uint8_t> scores);

    // Accessors
    [[nodiscard]] size_t length() const noexcept { return scores_.size(); }
    [[nodiscard]] bool empty() const noexcept { return scores_.empty(); }
    [[nodiscard]] const std::vector<uint8_t>& scores() const noexcept { return scores_; }

    // Element access
    [[nodiscard]] uint8_t operator[](size_t index) const { return scores_[index]; }
    [[nodiscard]] uint8_t at(size_t index) const { return scores_.at(index); }

    // Iterator support
    [[nodiscard]] auto begin() const noexcept { return scores_.begin(); }
    [[nodiscard]] auto end() const noexcept { return scores_.end(); }

    // Statistics
    [[nodiscard]] double meanQuality() const noexcept;
    [[nodiscard]] double medianQuality() const noexcept;
    [[nodiscard]] uint8_t minQuality() const noexcept;
    [[nodiscard]] uint8_t maxQuality() const noexcept;
    [[nodiscard]] double standardDeviation() const noexcept;

    // Quality analysis
    [[nodiscard]] size_t countAboveThreshold(uint8_t threshold) const noexcept;
    [[nodiscard]] size_t countBelowThreshold(uint8_t threshold) const noexcept;
    [[nodiscard]] double fractionAboveThreshold(uint8_t threshold) const noexcept;

    // Probability conversion
    [[nodiscard]] double errorProbability(size_t index) const;
    [[nodiscard]] double meanErrorProbability() const;
    [[nodiscard]] std::vector<double> errorProbabilities() const;

    // Quality trimming
    [[nodiscard]] std::pair<size_t, size_t> trimPositions(uint8_t threshold,
                                                          size_t min_length = 1) const;
    [[nodiscard]] QualityScores trim(uint8_t threshold, size_t min_length = 1) const;

    // Sliding window analysis
    [[nodiscard]] std::vector<double> slidingWindowMean(size_t window_size) const;
    [[nodiscard]] std::pair<size_t, size_t> findLowQualityRegion(uint8_t threshold,
                                                                  size_t min_length = 5) const;

    // Encoding conversion
    [[nodiscard]] std::string toAscii(QualityEncoding encoding = QualityEncoding::Phred33) const;
    [[nodiscard]] static QualityEncoding detectEncoding(std::string_view quality_string);

    // Subsetting
    [[nodiscard]] QualityScores subsequence(size_t start, size_t length) const;

private:
    std::vector<uint8_t> scores_;

    static uint8_t asciiToPhred(char c, QualityEncoding encoding);
    static char phredToAscii(uint8_t q, QualityEncoding encoding);
};

/**
 * @brief Combined sequence and quality data (like a FASTQ record)
 */
struct QualifiedSequence {
    std::string id;
    std::string bases;
    QualityScores quality;
    std::optional<std::string> description;

    [[nodiscard]] size_t length() const noexcept { return bases.length(); }
    [[nodiscard]] bool isValid() const noexcept { return bases.length() == quality.length(); }

    // Quality-based filtering
    [[nodiscard]] bool passesQualityFilter(double min_mean_quality) const;
    [[nodiscard]] bool passesLengthFilter(size_t min_length,
                                          std::optional<size_t> max_length = std::nullopt) const;

    // Trimming
    [[nodiscard]] QualifiedSequence trim(uint8_t quality_threshold,
                                         size_t min_length = 1) const;
};

/**
 * @brief Quality statistics for a collection of sequences
 */
struct QualityReport {
    size_t total_sequences;
    size_t total_bases;
    double mean_sequence_length;
    double mean_quality;
    double median_quality;
    size_t bases_above_q20;
    size_t bases_above_q30;
    std::vector<double> per_position_quality;  // Mean quality at each position
    std::vector<size_t> quality_distribution;  // Count of each quality score

    [[nodiscard]] double q20Ratio() const noexcept {
        return total_bases > 0 ? static_cast<double>(bases_above_q20) / total_bases : 0.0;
    }

    [[nodiscard]] double q30Ratio() const noexcept {
        return total_bases > 0 ? static_cast<double>(bases_above_q30) / total_bases : 0.0;
    }
};

/**
 * @brief Generate quality report for multiple sequences
 */
[[nodiscard]] QualityReport generateQualityReport(
    const std::vector<QualifiedSequence>& sequences
);

/**
 * @brief Filter sequences by quality criteria
 */
[[nodiscard]] std::vector<QualifiedSequence> filterByQuality(
    const std::vector<QualifiedSequence>& sequences,
    double min_mean_quality,
    std::optional<size_t> min_length = std::nullopt,
    std::optional<size_t> max_length = std::nullopt
);

} // namespace bioflow
