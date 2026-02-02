#pragma once

#include "bioflow/sequence.hpp"
#include <memory>
#include <vector>
#include <optional>
#include <functional>

namespace bioflow {

/**
 * @brief Exception class for alignment-related errors
 */
class AlignmentError : public std::runtime_error {
public:
    using std::runtime_error::runtime_error;
};

/**
 * @brief Scoring parameters for sequence alignment
 *
 * Configurable scoring matrix supporting:
 * - Match/mismatch scoring
 * - Linear gap penalty
 * - Affine gap penalty (gap open + gap extend)
 */
struct ScoringMatrix {
    int match_score = 2;
    int mismatch_penalty = -1;
    int gap_open_penalty = -2;
    int gap_extend_penalty = -1;

    // Simple gap penalty (linear)
    [[nodiscard]] constexpr int gapPenalty() const noexcept {
        return gap_open_penalty;
    }

    // Affine gap penalty
    [[nodiscard]] constexpr int gapPenalty(size_t gap_length) const noexcept {
        if (gap_length == 0) return 0;
        return gap_open_penalty + static_cast<int>(gap_length - 1) * gap_extend_penalty;
    }

    // Score for matching two bases
    [[nodiscard]] constexpr int score(char a, char b) const noexcept {
        return (a == b) ? match_score : mismatch_penalty;
    }

    // Predefined scoring schemes
    [[nodiscard]] static constexpr ScoringMatrix dnaMismatch() noexcept {
        return ScoringMatrix{1, -1, -2, -1};
    }

    [[nodiscard]] static constexpr ScoringMatrix dnaSimilarity() noexcept {
        return ScoringMatrix{2, -1, -2, -1};
    }

    [[nodiscard]] static constexpr ScoringMatrix strictMatch() noexcept {
        return ScoringMatrix{1, -3, -5, -2};
    }
};

/**
 * @brief Traceback direction for alignment matrix
 */
enum class TraceDirection : uint8_t {
    None = 0,
    Diagonal = 1,
    Up = 2,
    Left = 3
};

/**
 * @brief Result of a sequence alignment
 */
struct Alignment {
    std::string aligned_seq1;
    std::string aligned_seq2;
    int score;
    size_t start1;  // Start position in seq1
    size_t end1;    // End position in seq1
    size_t start2;  // Start position in seq2
    size_t end2;    // End position in seq2
    size_t matches;
    size_t mismatches;
    size_t gaps;

    // Computed properties
    [[nodiscard]] size_t alignmentLength() const noexcept {
        return aligned_seq1.length();
    }

    [[nodiscard]] double identity() const noexcept {
        size_t len = alignmentLength();
        return len > 0 ? static_cast<double>(matches) / len : 0.0;
    }

    [[nodiscard]] double similarity() const noexcept {
        size_t len = alignmentLength();
        return len > 0 ? static_cast<double>(matches) / (matches + mismatches) : 0.0;
    }

    [[nodiscard]] double gapRatio() const noexcept {
        size_t len = alignmentLength();
        return len > 0 ? static_cast<double>(gaps) / len : 0.0;
    }

    // CIGAR string representation
    [[nodiscard]] std::string cigar() const;

    // Pretty-print alignment
    [[nodiscard]] std::string prettyPrint(size_t line_width = 60) const;
};

/**
 * @brief Smith-Waterman local alignment algorithm
 *
 * Finds the best local alignment between two sequences.
 * Uses dynamic programming with O(mn) time and space complexity.
 *
 * @param seq1 First sequence
 * @param seq2 Second sequence
 * @param scoring Scoring parameters
 * @return Alignment result with traceback
 */
[[nodiscard]] Alignment smithWaterman(
    const Sequence& seq1,
    const Sequence& seq2,
    const ScoringMatrix& scoring = ScoringMatrix{}
);

/**
 * @brief Needleman-Wunsch global alignment algorithm
 *
 * Finds the optimal global alignment between two sequences.
 * Uses dynamic programming with O(mn) time and space complexity.
 *
 * @param seq1 First sequence
 * @param seq2 Second sequence
 * @param scoring Scoring parameters
 * @return Alignment result with traceback
 */
[[nodiscard]] Alignment needlemanWunsch(
    const Sequence& seq1,
    const Sequence& seq2,
    const ScoringMatrix& scoring = ScoringMatrix{}
);

/**
 * @brief Semi-global alignment (fitting alignment)
 *
 * Aligns seq1 globally within seq2, allowing free gaps at ends of seq2.
 * Useful for finding where a short sequence fits within a longer one.
 *
 * @param seq1 Shorter sequence (pattern)
 * @param seq2 Longer sequence (text)
 * @param scoring Scoring parameters
 * @return Alignment result
 */
[[nodiscard]] Alignment semiGlobalAlignment(
    const Sequence& seq1,
    const Sequence& seq2,
    const ScoringMatrix& scoring = ScoringMatrix{}
);

/**
 * @brief Compute edit distance (Levenshtein distance) between sequences
 *
 * @param seq1 First sequence
 * @param seq2 Second sequence
 * @return Edit distance (minimum number of edits to transform seq1 into seq2)
 */
[[nodiscard]] size_t editDistance(
    const Sequence& seq1,
    const Sequence& seq2
);

/**
 * @brief Compute Hamming distance between equal-length sequences
 *
 * @param seq1 First sequence
 * @param seq2 Second sequence
 * @return Number of positions where bases differ
 * @throws AlignmentError if sequences have different lengths
 */
[[nodiscard]] size_t hammingDistance(
    const Sequence& seq1,
    const Sequence& seq2
);

/**
 * @brief Banded Smith-Waterman for faster alignment of similar sequences
 *
 * @param seq1 First sequence
 * @param seq2 Second sequence
 * @param bandwidth Maximum distance from diagonal to consider
 * @param scoring Scoring parameters
 * @return Alignment result
 */
[[nodiscard]] Alignment bandedSmithWaterman(
    const Sequence& seq1,
    const Sequence& seq2,
    size_t bandwidth,
    const ScoringMatrix& scoring = ScoringMatrix{}
);

/**
 * @brief Multiple sequence alignment (progressive)
 *
 * Aligns multiple sequences using a simple progressive approach.
 *
 * @param sequences Vector of sequences to align
 * @param scoring Scoring parameters
 * @return Vector of aligned sequences (with gaps)
 */
[[nodiscard]] std::vector<std::string> multipleAlignment(
    const std::vector<Sequence>& sequences,
    const ScoringMatrix& scoring = ScoringMatrix{}
);

/**
 * @brief Alignment matrix for detailed analysis
 */
class AlignmentMatrix {
public:
    AlignmentMatrix(size_t rows, size_t cols);

    [[nodiscard]] int& at(size_t i, size_t j);
    [[nodiscard]] int at(size_t i, size_t j) const;

    [[nodiscard]] size_t rows() const noexcept { return rows_; }
    [[nodiscard]] size_t cols() const noexcept { return cols_; }

    [[nodiscard]] int maxScore() const noexcept;
    [[nodiscard]] std::pair<size_t, size_t> maxPosition() const;

private:
    size_t rows_;
    size_t cols_;
    std::vector<int> data_;
};

} // namespace bioflow
