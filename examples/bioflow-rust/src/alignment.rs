//! Sequence alignment algorithms.
//!
//! This module provides implementations of classic sequence alignment algorithms:
//! - Smith-Waterman (local alignment)
//! - Needleman-Wunsch (global alignment)

use crate::sequence::Sequence;
use std::fmt;

/// Scoring parameters for sequence alignment.
#[derive(Debug, Clone, Copy)]
pub struct ScoringMatrix {
    /// Score for matching bases.
    pub match_score: i32,
    /// Penalty for mismatching bases (should be negative).
    pub mismatch_penalty: i32,
    /// Penalty for opening a gap (should be negative).
    pub gap_open_penalty: i32,
    /// Penalty for extending a gap (should be negative).
    pub gap_extend_penalty: i32,
}

impl Default for ScoringMatrix {
    fn default() -> Self {
        Self {
            match_score: 2,
            mismatch_penalty: -1,
            gap_open_penalty: -2,
            gap_extend_penalty: -1,
        }
    }
}

impl ScoringMatrix {
    /// Creates a new scoring matrix with simple gap penalty.
    pub fn new(match_score: i32, mismatch_penalty: i32, gap_penalty: i32) -> Self {
        Self {
            match_score,
            mismatch_penalty,
            gap_open_penalty: gap_penalty,
            gap_extend_penalty: gap_penalty,
        }
    }

    /// Creates a scoring matrix with affine gap penalties.
    pub fn with_affine_gaps(
        match_score: i32,
        mismatch_penalty: i32,
        gap_open: i32,
        gap_extend: i32,
    ) -> Self {
        Self {
            match_score,
            mismatch_penalty,
            gap_open_penalty: gap_open,
            gap_extend_penalty: gap_extend,
        }
    }

    /// BLOSUM62-like scoring for nucleotides.
    pub fn blosum_like() -> Self {
        Self {
            match_score: 5,
            mismatch_penalty: -4,
            gap_open_penalty: -10,
            gap_extend_penalty: -1,
        }
    }

    /// Returns the score for aligning two bases.
    #[inline]
    pub fn score(&self, a: u8, b: u8) -> i32 {
        if a == b && a != b'N' {
            self.match_score
        } else {
            self.mismatch_penalty
        }
    }
}

/// Direction for traceback in alignment matrices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Stop,
    Diagonal,
    Up,
    Left,
}

/// Result of a sequence alignment.
#[derive(Debug, Clone)]
pub struct Alignment {
    /// The first aligned sequence (with gaps represented as '-').
    pub aligned_seq1: String,
    /// The second aligned sequence (with gaps represented as '-').
    pub aligned_seq2: String,
    /// The alignment score.
    pub score: i32,
    /// Start position in the first sequence (0-indexed).
    pub start1: usize,
    /// End position in the first sequence.
    pub end1: usize,
    /// Start position in the second sequence (0-indexed).
    pub start2: usize,
    /// End position in the second sequence.
    pub end2: usize,
    /// Number of matching positions.
    pub matches: usize,
    /// Number of mismatching positions.
    pub mismatches: usize,
    /// Number of gaps.
    pub gaps: usize,
    /// The type of alignment performed.
    pub alignment_type: AlignmentType,
}

/// The type of alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignmentType {
    /// Local alignment (Smith-Waterman).
    Local,
    /// Global alignment (Needleman-Wunsch).
    Global,
    /// Semi-global alignment.
    SemiGlobal,
}

impl Alignment {
    /// Returns the alignment identity (fraction of matching positions).
    pub fn identity(&self) -> f64 {
        let aligned_len = self.matches + self.mismatches;
        if aligned_len == 0 {
            0.0
        } else {
            self.matches as f64 / aligned_len as f64
        }
    }

    /// Returns the alignment coverage relative to the first sequence.
    pub fn coverage_seq1(&self) -> f64 {
        (self.end1 - self.start1) as f64 / self.aligned_seq1.len().max(1) as f64
    }

    /// Returns the alignment coverage relative to the second sequence.
    pub fn coverage_seq2(&self) -> f64 {
        (self.end2 - self.start2) as f64 / self.aligned_seq2.len().max(1) as f64
    }

    /// Returns the alignment length (excluding terminal gaps).
    pub fn alignment_length(&self) -> usize {
        self.aligned_seq1.len()
    }

    /// Returns a formatted visualization of the alignment.
    pub fn format_alignment(&self, line_width: usize) -> String {
        let mut result = String::new();
        let seq1_bytes = self.aligned_seq1.as_bytes();
        let seq2_bytes = self.aligned_seq2.as_bytes();

        for chunk_start in (0..self.aligned_seq1.len()).step_by(line_width) {
            let chunk_end = (chunk_start + line_width).min(self.aligned_seq1.len());

            // First sequence
            result.push_str(&format!(
                "Seq1: {}\n",
                &self.aligned_seq1[chunk_start..chunk_end]
            ));

            // Match line
            result.push_str("      ");
            for i in chunk_start..chunk_end {
                if seq1_bytes[i] == b'-' || seq2_bytes[i] == b'-' {
                    result.push(' ');
                } else if seq1_bytes[i] == seq2_bytes[i] {
                    result.push('|');
                } else {
                    result.push('.');
                }
            }
            result.push('\n');

            // Second sequence
            result.push_str(&format!(
                "Seq2: {}\n",
                &self.aligned_seq2[chunk_start..chunk_end]
            ));
            result.push('\n');
        }

        result
    }
}

impl fmt::Display for Alignment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Alignment ({:?}):", self.alignment_type)?;
        writeln!(f, "  Score: {}", self.score)?;
        writeln!(f, "  Identity: {:.1}%", self.identity() * 100.0)?;
        writeln!(
            f,
            "  Matches: {}, Mismatches: {}, Gaps: {}",
            self.matches, self.mismatches, self.gaps
        )?;
        writeln!(f, "  Position: seq1[{}..{}], seq2[{}..{}]",
            self.start1, self.end1, self.start2, self.end2)?;
        writeln!(f)?;
        write!(f, "{}", self.format_alignment(60))
    }
}

/// Performs Smith-Waterman local alignment.
///
/// The Smith-Waterman algorithm finds the highest-scoring local alignment
/// between two sequences.
///
/// # Arguments
///
/// * `seq1` - The first sequence.
/// * `seq2` - The second sequence.
/// * `scoring` - The scoring matrix to use.
///
/// # Returns
///
/// An `Alignment` struct containing the aligned sequences and statistics.
///
/// # Examples
///
/// ```
/// use bioflow_rust::sequence::Sequence;
/// use bioflow_rust::alignment::{smith_waterman, ScoringMatrix};
///
/// let seq1 = Sequence::new("ACGTACGT").unwrap();
/// let seq2 = Sequence::new("CGTACG").unwrap();
/// let scoring = ScoringMatrix::default();
///
/// let alignment = smith_waterman(&seq1, &seq2, &scoring);
/// assert!(alignment.score > 0);
/// ```
pub fn smith_waterman(
    seq1: &Sequence,
    seq2: &Sequence,
    scoring: &ScoringMatrix,
) -> Alignment {
    let m = seq1.len();
    let n = seq2.len();
    let bases1 = seq1.bases().as_bytes();
    let bases2 = seq2.bases().as_bytes();

    // Initialize score matrix (H) and traceback matrix
    let mut h = vec![vec![0i32; n + 1]; m + 1];
    let mut traceback = vec![vec![Direction::Stop; n + 1]; m + 1];

    let mut max_score = 0;
    let mut max_pos = (0, 0);

    // Fill the matrices
    for i in 1..=m {
        for j in 1..=n {
            let match_score = scoring.score(bases1[i - 1], bases2[j - 1]);

            let diag = h[i - 1][j - 1] + match_score;
            let up = h[i - 1][j] + scoring.gap_open_penalty;
            let left = h[i][j - 1] + scoring.gap_open_penalty;

            // Local alignment: scores cannot go negative
            let max_val = 0.max(diag).max(up).max(left);
            h[i][j] = max_val;

            // Set traceback direction
            if max_val == 0 {
                traceback[i][j] = Direction::Stop;
            } else if max_val == diag {
                traceback[i][j] = Direction::Diagonal;
            } else if max_val == up {
                traceback[i][j] = Direction::Up;
            } else {
                traceback[i][j] = Direction::Left;
            }

            // Track maximum score position
            if max_val > max_score {
                max_score = max_val;
                max_pos = (i, j);
            }
        }
    }

    // Traceback from the maximum score position
    traceback_alignment(
        bases1,
        bases2,
        &traceback,
        max_pos,
        max_score,
        AlignmentType::Local,
    )
}

/// Performs Needleman-Wunsch global alignment.
///
/// The Needleman-Wunsch algorithm finds the optimal global alignment
/// between two sequences, aligning them from end to end.
///
/// # Arguments
///
/// * `seq1` - The first sequence.
/// * `seq2` - The second sequence.
/// * `scoring` - The scoring matrix to use.
///
/// # Returns
///
/// An `Alignment` struct containing the aligned sequences and statistics.
///
/// # Examples
///
/// ```
/// use bioflow_rust::sequence::Sequence;
/// use bioflow_rust::alignment::{needleman_wunsch, ScoringMatrix};
///
/// let seq1 = Sequence::new("ACGT").unwrap();
/// let seq2 = Sequence::new("ACGT").unwrap();
/// let scoring = ScoringMatrix::default();
///
/// let alignment = needleman_wunsch(&seq1, &seq2, &scoring);
/// assert_eq!(alignment.aligned_seq1, "ACGT");
/// ```
pub fn needleman_wunsch(
    seq1: &Sequence,
    seq2: &Sequence,
    scoring: &ScoringMatrix,
) -> Alignment {
    let m = seq1.len();
    let n = seq2.len();
    let bases1 = seq1.bases().as_bytes();
    let bases2 = seq2.bases().as_bytes();

    // Initialize score matrix and traceback matrix
    let mut f = vec![vec![0i32; n + 1]; m + 1];
    let mut traceback = vec![vec![Direction::Stop; n + 1]; m + 1];

    // Initialize first row and column with gap penalties
    for i in 1..=m {
        f[i][0] = scoring.gap_open_penalty * i as i32;
        traceback[i][0] = Direction::Up;
    }
    for j in 1..=n {
        f[0][j] = scoring.gap_open_penalty * j as i32;
        traceback[0][j] = Direction::Left;
    }

    // Fill the matrices
    for i in 1..=m {
        for j in 1..=n {
            let match_score = scoring.score(bases1[i - 1], bases2[j - 1]);

            let diag = f[i - 1][j - 1] + match_score;
            let up = f[i - 1][j] + scoring.gap_open_penalty;
            let left = f[i][j - 1] + scoring.gap_open_penalty;

            // Global alignment: no lower bound of 0
            let max_val = diag.max(up).max(left);
            f[i][j] = max_val;

            if max_val == diag {
                traceback[i][j] = Direction::Diagonal;
            } else if max_val == up {
                traceback[i][j] = Direction::Up;
            } else {
                traceback[i][j] = Direction::Left;
            }
        }
    }

    // Traceback from bottom-right corner
    traceback_alignment(
        bases1,
        bases2,
        &traceback,
        (m, n),
        f[m][n],
        AlignmentType::Global,
    )
}

/// Performs semi-global alignment.
///
/// Semi-global alignment allows free end gaps, useful for aligning
/// a shorter sequence against a longer one.
pub fn semi_global_alignment(
    seq1: &Sequence,
    seq2: &Sequence,
    scoring: &ScoringMatrix,
) -> Alignment {
    let m = seq1.len();
    let n = seq2.len();
    let bases1 = seq1.bases().as_bytes();
    let bases2 = seq2.bases().as_bytes();

    let mut f = vec![vec![0i32; n + 1]; m + 1];
    let mut traceback = vec![vec![Direction::Stop; n + 1]; m + 1];

    // Initialize first row (free gaps at start of seq1)
    for j in 1..=n {
        traceback[0][j] = Direction::Left;
    }

    // Initialize first column (free gaps at start of seq2)
    for i in 1..=m {
        traceback[i][0] = Direction::Up;
    }

    // Fill matrices
    for i in 1..=m {
        for j in 1..=n {
            let match_score = scoring.score(bases1[i - 1], bases2[j - 1]);

            let diag = f[i - 1][j - 1] + match_score;
            let up = f[i - 1][j] + scoring.gap_open_penalty;
            let left = f[i][j - 1] + scoring.gap_open_penalty;

            let max_val = diag.max(up).max(left);
            f[i][j] = max_val;

            if max_val == diag {
                traceback[i][j] = Direction::Diagonal;
            } else if max_val == up {
                traceback[i][j] = Direction::Up;
            } else {
                traceback[i][j] = Direction::Left;
            }
        }
    }

    // Find best ending position (last row or last column)
    let mut max_score = i32::MIN;
    let mut max_pos = (m, n);

    // Check last row (end of seq1)
    for j in 0..=n {
        if f[m][j] > max_score {
            max_score = f[m][j];
            max_pos = (m, j);
        }
    }

    // Check last column (end of seq2)
    for i in 0..=m {
        if f[i][n] > max_score {
            max_score = f[i][n];
            max_pos = (i, n);
        }
    }

    traceback_alignment(
        bases1,
        bases2,
        &traceback,
        max_pos,
        max_score,
        AlignmentType::SemiGlobal,
    )
}

/// Performs traceback to construct the aligned sequences.
fn traceback_alignment(
    bases1: &[u8],
    bases2: &[u8],
    traceback: &[Vec<Direction>],
    start_pos: (usize, usize),
    score: i32,
    alignment_type: AlignmentType,
) -> Alignment {
    let mut aligned1 = Vec::new();
    let mut aligned2 = Vec::new();
    let mut i = start_pos.0;
    let mut j = start_pos.1;

    let end1 = i;
    let end2 = j;

    // Follow traceback
    while i > 0 || j > 0 {
        match traceback[i][j] {
            Direction::Stop => break,
            Direction::Diagonal => {
                aligned1.push(bases1[i - 1]);
                aligned2.push(bases2[j - 1]);
                i -= 1;
                j -= 1;
            }
            Direction::Up => {
                aligned1.push(bases1[i - 1]);
                aligned2.push(b'-');
                i -= 1;
            }
            Direction::Left => {
                aligned1.push(b'-');
                aligned2.push(bases2[j - 1]);
                j -= 1;
            }
        }
    }

    let start1 = i;
    let start2 = j;

    // Reverse since we built the strings backwards
    aligned1.reverse();
    aligned2.reverse();

    // Calculate statistics
    let mut matches = 0;
    let mut mismatches = 0;
    let mut gaps = 0;

    for (a, b) in aligned1.iter().zip(aligned2.iter()) {
        if *a == b'-' || *b == b'-' {
            gaps += 1;
        } else if a == b {
            matches += 1;
        } else {
            mismatches += 1;
        }
    }

    Alignment {
        aligned_seq1: String::from_utf8(aligned1).unwrap(),
        aligned_seq2: String::from_utf8(aligned2).unwrap(),
        score,
        start1,
        end1,
        start2,
        end2,
        matches,
        mismatches,
        gaps,
        alignment_type,
    }
}

/// Calculates the edit distance (Levenshtein distance) between two sequences.
///
/// # Arguments
///
/// * `seq1` - The first sequence.
/// * `seq2` - The second sequence.
///
/// # Returns
///
/// The minimum number of single-character edits needed to transform seq1 into seq2.
pub fn edit_distance(seq1: &Sequence, seq2: &Sequence) -> usize {
    let m = seq1.len();
    let n = seq2.len();
    let bases1 = seq1.bases().as_bytes();
    let bases2 = seq2.bases().as_bytes();

    // Use two rows to save memory
    let mut prev = (0..=n).collect::<Vec<_>>();
    let mut curr = vec![0; n + 1];

    for i in 1..=m {
        curr[0] = i;

        for j in 1..=n {
            let cost = if bases1[i - 1] == bases2[j - 1] { 0 } else { 1 };

            curr[j] = (prev[j] + 1)           // deletion
                .min(curr[j - 1] + 1)         // insertion
                .min(prev[j - 1] + cost);     // substitution
        }

        std::mem::swap(&mut prev, &mut curr);
    }

    prev[n]
}

/// Calculates Hamming distance between two equal-length sequences.
///
/// # Arguments
///
/// * `seq1` - The first sequence.
/// * `seq2` - The second sequence.
///
/// # Returns
///
/// The number of positions where the sequences differ, or None if lengths differ.
pub fn hamming_distance(seq1: &Sequence, seq2: &Sequence) -> Option<usize> {
    if seq1.len() != seq2.len() {
        return None;
    }

    let distance = seq1.bases().bytes()
        .zip(seq2.bases().bytes())
        .filter(|(a, b)| a != b)
        .count();

    Some(distance)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smith_waterman_perfect_match() {
        let seq1 = Sequence::new("ACGT").unwrap();
        let seq2 = Sequence::new("ACGT").unwrap();
        let scoring = ScoringMatrix::default();

        let alignment = smith_waterman(&seq1, &seq2, &scoring);

        assert_eq!(alignment.aligned_seq1, "ACGT");
        assert_eq!(alignment.aligned_seq2, "ACGT");
        assert_eq!(alignment.score, 8); // 4 matches * 2
        assert_eq!(alignment.matches, 4);
        assert_eq!(alignment.mismatches, 0);
        assert_eq!(alignment.gaps, 0);
    }

    #[test]
    fn test_smith_waterman_with_mismatch() {
        let seq1 = Sequence::new("ACGT").unwrap();
        let seq2 = Sequence::new("AGGT").unwrap();
        let scoring = ScoringMatrix::default();

        let alignment = smith_waterman(&seq1, &seq2, &scoring);

        assert!(alignment.score > 0);
        assert!(alignment.mismatches >= 1 || alignment.gaps >= 1);
    }

    #[test]
    fn test_needleman_wunsch_perfect_match() {
        let seq1 = Sequence::new("ACGT").unwrap();
        let seq2 = Sequence::new("ACGT").unwrap();
        let scoring = ScoringMatrix::default();

        let alignment = needleman_wunsch(&seq1, &seq2, &scoring);

        assert_eq!(alignment.aligned_seq1, "ACGT");
        assert_eq!(alignment.aligned_seq2, "ACGT");
        assert_eq!(alignment.matches, 4);
    }

    #[test]
    fn test_needleman_wunsch_with_gap() {
        let seq1 = Sequence::new("ACGT").unwrap();
        let seq2 = Sequence::new("ACT").unwrap();
        let scoring = ScoringMatrix::default();

        let alignment = needleman_wunsch(&seq1, &seq2, &scoring);

        assert!(alignment.gaps >= 1);
    }

    #[test]
    fn test_edit_distance() {
        let seq1 = Sequence::new("ACGT").unwrap();
        let seq2 = Sequence::new("ACGT").unwrap();
        assert_eq!(edit_distance(&seq1, &seq2), 0);

        let seq3 = Sequence::new("ACGT").unwrap();
        let seq4 = Sequence::new("AGGT").unwrap();
        assert_eq!(edit_distance(&seq3, &seq4), 1);

        let seq5 = Sequence::new("ACGT").unwrap();
        let seq6 = Sequence::new("ACT").unwrap();
        assert_eq!(edit_distance(&seq5, &seq6), 1);
    }

    #[test]
    fn test_hamming_distance() {
        let seq1 = Sequence::new("ACGT").unwrap();
        let seq2 = Sequence::new("ACGT").unwrap();
        assert_eq!(hamming_distance(&seq1, &seq2), Some(0));

        let seq3 = Sequence::new("ACGT").unwrap();
        let seq4 = Sequence::new("AGGT").unwrap();
        assert_eq!(hamming_distance(&seq3, &seq4), Some(1));

        let seq5 = Sequence::new("ACGT").unwrap();
        let seq6 = Sequence::new("ACT").unwrap();
        assert_eq!(hamming_distance(&seq5, &seq6), None); // Different lengths
    }

    #[test]
    fn test_alignment_identity() {
        let alignment = Alignment {
            aligned_seq1: "ACGT".to_string(),
            aligned_seq2: "AGGT".to_string(),
            score: 5,
            start1: 0,
            end1: 4,
            start2: 0,
            end2: 4,
            matches: 3,
            mismatches: 1,
            gaps: 0,
            alignment_type: AlignmentType::Global,
        };

        assert!((alignment.identity() - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn test_scoring_matrix() {
        let scoring = ScoringMatrix::default();
        assert_eq!(scoring.score(b'A', b'A'), 2);
        assert_eq!(scoring.score(b'A', b'T'), -1);
        assert_eq!(scoring.score(b'N', b'N'), -1); // N never matches
    }
}
