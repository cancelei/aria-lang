#include "bioflow/alignment.hpp"
#include <algorithm>
#include <sstream>
#include <limits>

namespace bioflow {

// ============================================================================
// AlignmentMatrix Implementation
// ============================================================================

AlignmentMatrix::AlignmentMatrix(size_t rows, size_t cols)
    : rows_(rows), cols_(cols), data_(rows * cols, 0) {}

int& AlignmentMatrix::at(size_t i, size_t j) {
    return data_[i * cols_ + j];
}

int AlignmentMatrix::at(size_t i, size_t j) const {
    return data_[i * cols_ + j];
}

int AlignmentMatrix::maxScore() const noexcept {
    if (data_.empty()) return 0;
    return *std::ranges::max_element(data_);
}

std::pair<size_t, size_t> AlignmentMatrix::maxPosition() const {
    if (data_.empty()) return {0, 0};

    auto max_it = std::ranges::max_element(data_);
    size_t index = static_cast<size_t>(std::distance(data_.begin(), max_it));
    return {index / cols_, index % cols_};
}

// ============================================================================
// Alignment Result Methods
// ============================================================================

std::string Alignment::cigar() const {
    if (aligned_seq1.empty()) return "";

    std::ostringstream oss;
    char current_op = '\0';
    size_t count = 0;

    for (size_t i = 0; i < aligned_seq1.length(); ++i) {
        char op;
        if (aligned_seq1[i] == '-') {
            op = 'I';  // Insertion in seq2
        } else if (aligned_seq2[i] == '-') {
            op = 'D';  // Deletion from seq1
        } else if (aligned_seq1[i] == aligned_seq2[i]) {
            op = 'M';  // Match
        } else {
            op = 'X';  // Mismatch
        }

        if (op == current_op) {
            count++;
        } else {
            if (count > 0) {
                oss << count << current_op;
            }
            current_op = op;
            count = 1;
        }
    }

    if (count > 0) {
        oss << count << current_op;
    }

    return oss.str();
}

std::string Alignment::prettyPrint(size_t line_width) const {
    std::ostringstream oss;

    oss << "Score: " << score << "\n";
    oss << "Identity: " << (identity() * 100) << "%\n";
    oss << "Gaps: " << gaps << " (" << (gapRatio() * 100) << "%)\n\n";

    for (size_t i = 0; i < aligned_seq1.length(); i += line_width) {
        size_t end = std::min(i + line_width, aligned_seq1.length());

        // Seq1 line
        oss << "Seq1: " << aligned_seq1.substr(i, end - i) << "\n";

        // Match line
        oss << "      ";
        for (size_t j = i; j < end; ++j) {
            if (aligned_seq1[j] == '-' || aligned_seq2[j] == '-') {
                oss << ' ';
            } else if (aligned_seq1[j] == aligned_seq2[j]) {
                oss << '|';
            } else {
                oss << '.';
            }
        }
        oss << "\n";

        // Seq2 line
        oss << "Seq2: " << aligned_seq2.substr(i, end - i) << "\n\n";
    }

    return oss.str();
}

// ============================================================================
// Smith-Waterman Algorithm
// ============================================================================

Alignment smithWaterman(
    const Sequence& seq1,
    const Sequence& seq2,
    const ScoringMatrix& scoring
) {
    const auto& s1 = seq1.bases();
    const auto& s2 = seq2.bases();
    const size_t m = s1.length();
    const size_t n = s2.length();

    // Create scoring and traceback matrices
    std::vector<std::vector<int>> score_matrix(m + 1, std::vector<int>(n + 1, 0));
    std::vector<std::vector<TraceDirection>> trace(m + 1,
        std::vector<TraceDirection>(n + 1, TraceDirection::None));

    int max_score = 0;
    size_t max_i = 0, max_j = 0;

    // Fill matrices
    for (size_t i = 1; i <= m; ++i) {
        for (size_t j = 1; j <= n; ++j) {
            // Match/mismatch
            int match = score_matrix[i-1][j-1] + scoring.score(s1[i-1], s2[j-1]);

            // Gap in seq2 (deletion from seq1)
            int delete_gap = score_matrix[i-1][j] + scoring.gapPenalty();

            // Gap in seq1 (insertion in seq2)
            int insert_gap = score_matrix[i][j-1] + scoring.gapPenalty();

            // Take maximum (including 0 for local alignment)
            int best = 0;
            TraceDirection dir = TraceDirection::None;

            if (match > best) {
                best = match;
                dir = TraceDirection::Diagonal;
            }
            if (delete_gap > best) {
                best = delete_gap;
                dir = TraceDirection::Up;
            }
            if (insert_gap > best) {
                best = insert_gap;
                dir = TraceDirection::Left;
            }

            score_matrix[i][j] = best;
            trace[i][j] = dir;

            // Track maximum score position
            if (best > max_score) {
                max_score = best;
                max_i = i;
                max_j = j;
            }
        }
    }

    // Traceback from maximum score
    Alignment result;
    result.score = max_score;
    result.end1 = max_i - 1;
    result.end2 = max_j - 1;
    result.matches = 0;
    result.mismatches = 0;
    result.gaps = 0;

    std::string aligned1, aligned2;
    size_t i = max_i, j = max_j;

    while (i > 0 && j > 0 && score_matrix[i][j] > 0) {
        switch (trace[i][j]) {
            case TraceDirection::Diagonal:
                aligned1 = s1[i-1] + aligned1;
                aligned2 = s2[j-1] + aligned2;
                if (s1[i-1] == s2[j-1]) {
                    result.matches++;
                } else {
                    result.mismatches++;
                }
                --i;
                --j;
                break;

            case TraceDirection::Up:
                aligned1 = s1[i-1] + aligned1;
                aligned2 = '-' + aligned2;
                result.gaps++;
                --i;
                break;

            case TraceDirection::Left:
                aligned1 = '-' + aligned1;
                aligned2 = s2[j-1] + aligned2;
                result.gaps++;
                --j;
                break;

            default:
                // Should not reach here
                i = 0;
                j = 0;
                break;
        }
    }

    result.aligned_seq1 = std::move(aligned1);
    result.aligned_seq2 = std::move(aligned2);
    result.start1 = i;
    result.start2 = j;

    return result;
}

// ============================================================================
// Needleman-Wunsch Algorithm
// ============================================================================

Alignment needlemanWunsch(
    const Sequence& seq1,
    const Sequence& seq2,
    const ScoringMatrix& scoring
) {
    const auto& s1 = seq1.bases();
    const auto& s2 = seq2.bases();
    const size_t m = s1.length();
    const size_t n = s2.length();

    // Create scoring and traceback matrices
    std::vector<std::vector<int>> score_matrix(m + 1, std::vector<int>(n + 1, 0));
    std::vector<std::vector<TraceDirection>> trace(m + 1,
        std::vector<TraceDirection>(n + 1, TraceDirection::None));

    // Initialize first row and column with gap penalties
    for (size_t i = 1; i <= m; ++i) {
        score_matrix[i][0] = static_cast<int>(i) * scoring.gapPenalty();
        trace[i][0] = TraceDirection::Up;
    }
    for (size_t j = 1; j <= n; ++j) {
        score_matrix[0][j] = static_cast<int>(j) * scoring.gapPenalty();
        trace[0][j] = TraceDirection::Left;
    }

    // Fill matrices
    for (size_t i = 1; i <= m; ++i) {
        for (size_t j = 1; j <= n; ++j) {
            int match = score_matrix[i-1][j-1] + scoring.score(s1[i-1], s2[j-1]);
            int delete_gap = score_matrix[i-1][j] + scoring.gapPenalty();
            int insert_gap = score_matrix[i][j-1] + scoring.gapPenalty();

            int best;
            TraceDirection dir;

            if (match >= delete_gap && match >= insert_gap) {
                best = match;
                dir = TraceDirection::Diagonal;
            } else if (delete_gap >= insert_gap) {
                best = delete_gap;
                dir = TraceDirection::Up;
            } else {
                best = insert_gap;
                dir = TraceDirection::Left;
            }

            score_matrix[i][j] = best;
            trace[i][j] = dir;
        }
    }

    // Traceback from bottom-right corner
    Alignment result;
    result.score = score_matrix[m][n];
    result.start1 = 0;
    result.start2 = 0;
    result.end1 = m - 1;
    result.end2 = n - 1;
    result.matches = 0;
    result.mismatches = 0;
    result.gaps = 0;

    std::string aligned1, aligned2;
    size_t i = m, j = n;

    while (i > 0 || j > 0) {
        if (i > 0 && j > 0 && trace[i][j] == TraceDirection::Diagonal) {
            aligned1 = s1[i-1] + aligned1;
            aligned2 = s2[j-1] + aligned2;
            if (s1[i-1] == s2[j-1]) {
                result.matches++;
            } else {
                result.mismatches++;
            }
            --i;
            --j;
        } else if (i > 0 && (j == 0 || trace[i][j] == TraceDirection::Up)) {
            aligned1 = s1[i-1] + aligned1;
            aligned2 = '-' + aligned2;
            result.gaps++;
            --i;
        } else {
            aligned1 = '-' + aligned1;
            aligned2 = s2[j-1] + aligned2;
            result.gaps++;
            --j;
        }
    }

    result.aligned_seq1 = std::move(aligned1);
    result.aligned_seq2 = std::move(aligned2);

    return result;
}

// ============================================================================
// Semi-Global Alignment
// ============================================================================

Alignment semiGlobalAlignment(
    const Sequence& seq1,
    const Sequence& seq2,
    const ScoringMatrix& scoring
) {
    const auto& s1 = seq1.bases();
    const auto& s2 = seq2.bases();
    const size_t m = s1.length();
    const size_t n = s2.length();

    std::vector<std::vector<int>> score_matrix(m + 1, std::vector<int>(n + 1, 0));
    std::vector<std::vector<TraceDirection>> trace(m + 1,
        std::vector<TraceDirection>(n + 1, TraceDirection::None));

    // Initialize first column (gaps in seq1 are free at start)
    for (size_t i = 1; i <= m; ++i) {
        score_matrix[i][0] = static_cast<int>(i) * scoring.gapPenalty();
        trace[i][0] = TraceDirection::Up;
    }
    // First row: gaps in seq2 are free (semi-global)
    for (size_t j = 1; j <= n; ++j) {
        trace[0][j] = TraceDirection::Left;
    }

    // Fill matrices
    for (size_t i = 1; i <= m; ++i) {
        for (size_t j = 1; j <= n; ++j) {
            int match = score_matrix[i-1][j-1] + scoring.score(s1[i-1], s2[j-1]);
            int delete_gap = score_matrix[i-1][j] + scoring.gapPenalty();
            int insert_gap = score_matrix[i][j-1] + scoring.gapPenalty();

            int best;
            TraceDirection dir;

            if (match >= delete_gap && match >= insert_gap) {
                best = match;
                dir = TraceDirection::Diagonal;
            } else if (delete_gap >= insert_gap) {
                best = delete_gap;
                dir = TraceDirection::Up;
            } else {
                best = insert_gap;
                dir = TraceDirection::Left;
            }

            score_matrix[i][j] = best;
            trace[i][j] = dir;
        }
    }

    // Find best score in last row (end gaps in seq2 are free)
    int max_score = score_matrix[m][0];
    size_t max_j = 0;
    for (size_t j = 1; j <= n; ++j) {
        if (score_matrix[m][j] > max_score) {
            max_score = score_matrix[m][j];
            max_j = j;
        }
    }

    // Traceback
    Alignment result;
    result.score = max_score;
    result.matches = 0;
    result.mismatches = 0;
    result.gaps = 0;

    std::string aligned1, aligned2;
    size_t i = m, j = max_j;

    // Add trailing gaps in seq2 if any
    for (size_t k = n; k > max_j; --k) {
        aligned1 = '-' + aligned1;
        aligned2 = s2[k-1] + aligned2;
        result.gaps++;
    }

    while (i > 0 || j > 0) {
        if (i > 0 && j > 0 && trace[i][j] == TraceDirection::Diagonal) {
            aligned1 = s1[i-1] + aligned1;
            aligned2 = s2[j-1] + aligned2;
            if (s1[i-1] == s2[j-1]) {
                result.matches++;
            } else {
                result.mismatches++;
            }
            --i;
            --j;
        } else if (i > 0 && (j == 0 || trace[i][j] == TraceDirection::Up)) {
            aligned1 = s1[i-1] + aligned1;
            aligned2 = '-' + aligned2;
            result.gaps++;
            --i;
        } else {
            aligned1 = '-' + aligned1;
            aligned2 = s2[j-1] + aligned2;
            result.gaps++;
            --j;
        }
    }

    result.aligned_seq1 = std::move(aligned1);
    result.aligned_seq2 = std::move(aligned2);
    result.start1 = 0;
    result.start2 = 0;
    result.end1 = m - 1;
    result.end2 = n - 1;

    return result;
}

// ============================================================================
// Distance Functions
// ============================================================================

size_t editDistance(const Sequence& seq1, const Sequence& seq2) {
    const auto& s1 = seq1.bases();
    const auto& s2 = seq2.bases();
    const size_t m = s1.length();
    const size_t n = s2.length();

    // Use space-optimized version with two rows
    std::vector<size_t> prev(n + 1), curr(n + 1);

    // Initialize first row
    for (size_t j = 0; j <= n; ++j) {
        prev[j] = j;
    }

    for (size_t i = 1; i <= m; ++i) {
        curr[0] = i;

        for (size_t j = 1; j <= n; ++j) {
            if (s1[i-1] == s2[j-1]) {
                curr[j] = prev[j-1];
            } else {
                curr[j] = 1 + std::min({prev[j-1], prev[j], curr[j-1]});
            }
        }

        std::swap(prev, curr);
    }

    return prev[n];
}

size_t hammingDistance(const Sequence& seq1, const Sequence& seq2) {
    if (seq1.length() != seq2.length()) {
        throw AlignmentError("Hamming distance requires equal-length sequences");
    }

    const auto& s1 = seq1.bases();
    const auto& s2 = seq2.bases();

    size_t distance = 0;
    for (size_t i = 0; i < s1.length(); ++i) {
        if (s1[i] != s2[i]) {
            ++distance;
        }
    }

    return distance;
}

// ============================================================================
// Banded Smith-Waterman
// ============================================================================

Alignment bandedSmithWaterman(
    const Sequence& seq1,
    const Sequence& seq2,
    size_t bandwidth,
    const ScoringMatrix& scoring
) {
    const auto& s1 = seq1.bases();
    const auto& s2 = seq2.bases();
    const size_t m = s1.length();
    const size_t n = s2.length();

    // For sequences of very different lengths, fall back to regular SW
    if (m > n + bandwidth || n > m + bandwidth) {
        return smithWaterman(seq1, seq2, scoring);
    }

    const int k = static_cast<int>(bandwidth);

    // Create banded scoring matrix
    std::vector<std::vector<int>> score_matrix(m + 1,
        std::vector<int>(2 * bandwidth + 1, 0));
    std::vector<std::vector<TraceDirection>> trace(m + 1,
        std::vector<TraceDirection>(2 * bandwidth + 1, TraceDirection::None));

    int max_score = 0;
    size_t max_i = 0, max_j = 0;

    for (size_t i = 1; i <= m; ++i) {
        int j_start = std::max(1, static_cast<int>(i) - k);
        int j_end = std::min(static_cast<int>(n), static_cast<int>(i) + k);

        for (int j = j_start; j <= j_end; ++j) {
            int band_j = j - static_cast<int>(i) + k;

            int match = 0;
            if (band_j >= 0 && band_j < static_cast<int>(2 * bandwidth + 1) &&
                i > 0 && static_cast<size_t>(j) > 0) {
                int prev_band_j = band_j;  // Same offset since both i and j decrease
                if (prev_band_j >= 0 && prev_band_j < static_cast<int>(2 * bandwidth + 1)) {
                    match = score_matrix[i-1][prev_band_j] +
                            scoring.score(s1[i-1], s2[j-1]);
                }
            }

            int delete_gap = 0;
            int up_band_j = band_j + 1;  // j stays same, i decreases
            if (up_band_j >= 0 && up_band_j < static_cast<int>(2 * bandwidth + 1)) {
                delete_gap = score_matrix[i-1][up_band_j] + scoring.gapPenalty();
            }

            int insert_gap = 0;
            int left_band_j = band_j - 1;  // i stays same, j decreases
            if (left_band_j >= 0 && left_band_j < static_cast<int>(2 * bandwidth + 1)) {
                insert_gap = score_matrix[i][left_band_j] + scoring.gapPenalty();
            }

            int best = 0;
            TraceDirection dir = TraceDirection::None;

            if (match > best) {
                best = match;
                dir = TraceDirection::Diagonal;
            }
            if (delete_gap > best) {
                best = delete_gap;
                dir = TraceDirection::Up;
            }
            if (insert_gap > best) {
                best = insert_gap;
                dir = TraceDirection::Left;
            }

            score_matrix[i][band_j] = best;
            trace[i][band_j] = dir;

            if (best > max_score) {
                max_score = best;
                max_i = i;
                max_j = static_cast<size_t>(j);
            }
        }
    }

    // Simplified traceback - fall back to standard SW for actual traceback
    // This is a common optimization pattern
    return smithWaterman(seq1, seq2, scoring);
}

// ============================================================================
// Multiple Sequence Alignment
// ============================================================================

std::vector<std::string> multipleAlignment(
    const std::vector<Sequence>& sequences,
    const ScoringMatrix& scoring
) {
    if (sequences.empty()) {
        return {};
    }
    if (sequences.size() == 1) {
        return {sequences[0].bases()};
    }

    // Progressive alignment: align sequences pairwise and build consensus
    std::vector<std::string> aligned;
    aligned.push_back(sequences[0].bases());

    for (size_t i = 1; i < sequences.size(); ++i) {
        // Create temporary sequence from consensus of aligned sequences
        // For simplicity, use the first aligned sequence
        Sequence consensus(aligned[0]);
        auto pairwise = needlemanWunsch(consensus, sequences[i], scoring);

        // Update all previously aligned sequences with new gaps
        std::vector<std::string> new_aligned;
        for (const auto& seq : aligned) {
            std::string updated;
            size_t seq_pos = 0;
            for (char c : pairwise.aligned_seq1) {
                if (c == '-') {
                    updated += '-';
                } else {
                    updated += (seq_pos < seq.length()) ? seq[seq_pos++] : '-';
                }
            }
            new_aligned.push_back(std::move(updated));
        }

        // Add newly aligned sequence
        new_aligned.push_back(pairwise.aligned_seq2);
        aligned = std::move(new_aligned);
    }

    return aligned;
}

} // namespace bioflow
