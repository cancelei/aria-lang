//! Integration tests for the alignment module.

use bioflow_rust::alignment::*;
use bioflow_rust::sequence::Sequence;

// =============================================================================
// Scoring Matrix Tests
// =============================================================================

#[test]
fn test_scoring_matrix_default() {
    let scoring = ScoringMatrix::default();
    assert_eq!(scoring.match_score, 2);
    assert_eq!(scoring.mismatch_penalty, -1);
    assert_eq!(scoring.gap_open_penalty, -2);
}

#[test]
fn test_scoring_matrix_custom() {
    let scoring = ScoringMatrix::new(5, -3, -4);
    assert_eq!(scoring.match_score, 5);
    assert_eq!(scoring.mismatch_penalty, -3);
    assert_eq!(scoring.gap_open_penalty, -4);
}

#[test]
fn test_scoring_matrix_affine() {
    let scoring = ScoringMatrix::with_affine_gaps(2, -1, -5, -1);
    assert_eq!(scoring.gap_open_penalty, -5);
    assert_eq!(scoring.gap_extend_penalty, -1);
}

#[test]
fn test_scoring_matrix_score() {
    let scoring = ScoringMatrix::default();

    assert_eq!(scoring.score(b'A', b'A'), 2);
    assert_eq!(scoring.score(b'A', b'T'), -1);
    assert_eq!(scoring.score(b'N', b'N'), -1); // N never matches
}

// =============================================================================
// Smith-Waterman (Local Alignment) Tests
// =============================================================================

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
    // Either there's a mismatch or a gap
    assert!(alignment.mismatches > 0 || alignment.gaps > 0);
}

#[test]
fn test_smith_waterman_partial_match() {
    // Local alignment should find the best matching region
    let seq1 = Sequence::new("NNNNACGTNNNN").unwrap();
    let seq2 = Sequence::new("ACGT").unwrap();
    let scoring = ScoringMatrix::default();

    let alignment = smith_waterman(&seq1, &seq2, &scoring);

    // Should find the ACGT match
    assert!(alignment.score > 0);
    assert!(alignment.aligned_seq1.contains('C') || alignment.aligned_seq1.contains('G'));
}

#[test]
fn test_smith_waterman_no_match() {
    let seq1 = Sequence::new("AAAA").unwrap();
    let seq2 = Sequence::new("TTTT").unwrap();
    let scoring = ScoringMatrix::default();

    let alignment = smith_waterman(&seq1, &seq2, &scoring);

    // Score should be 0 (no local alignment better than 0)
    assert_eq!(alignment.score, 0);
}

#[test]
fn test_smith_waterman_with_gap() {
    let seq1 = Sequence::new("ACGT").unwrap();
    let seq2 = Sequence::new("ACT").unwrap();
    let scoring = ScoringMatrix::default();

    let alignment = smith_waterman(&seq1, &seq2, &scoring);

    // Local alignment finds best local match, which might just be ACT matching ACT
    // (positions 0,1,3 in seq1), so the aligned region may be shorter
    assert!(alignment.score > 0);
}

// =============================================================================
// Needleman-Wunsch (Global Alignment) Tests
// =============================================================================

#[test]
fn test_needleman_wunsch_perfect_match() {
    let seq1 = Sequence::new("ACGT").unwrap();
    let seq2 = Sequence::new("ACGT").unwrap();
    let scoring = ScoringMatrix::default();

    let alignment = needleman_wunsch(&seq1, &seq2, &scoring);

    assert_eq!(alignment.aligned_seq1, "ACGT");
    assert_eq!(alignment.aligned_seq2, "ACGT");
    assert_eq!(alignment.matches, 4);
    assert_eq!(alignment.mismatches, 0);
    assert_eq!(alignment.gaps, 0);
}

#[test]
fn test_needleman_wunsch_with_mismatch() {
    let seq1 = Sequence::new("ACGT").unwrap();
    let seq2 = Sequence::new("AGGT").unwrap();
    let scoring = ScoringMatrix::default();

    let alignment = needleman_wunsch(&seq1, &seq2, &scoring);

    assert!(alignment.mismatches >= 1);
}

#[test]
fn test_needleman_wunsch_with_gap() {
    let seq1 = Sequence::new("ACGT").unwrap();
    let seq2 = Sequence::new("ACT").unwrap();
    let scoring = ScoringMatrix::default();

    let alignment = needleman_wunsch(&seq1, &seq2, &scoring);

    // Global alignment must align entire sequences
    assert_eq!(alignment.alignment_length(), 4);
    assert!(alignment.gaps >= 1);
}

#[test]
fn test_needleman_wunsch_different_lengths() {
    let seq1 = Sequence::new("ACGTACGT").unwrap();
    let seq2 = Sequence::new("ACGT").unwrap();
    let scoring = ScoringMatrix::default();

    let alignment = needleman_wunsch(&seq1, &seq2, &scoring);

    // Should have gaps to account for length difference
    assert!(alignment.gaps >= 4);
}

// =============================================================================
// Semi-Global Alignment Tests
// =============================================================================

#[test]
fn test_semi_global_alignment() {
    let seq1 = Sequence::new("ACGTACGT").unwrap();
    let seq2 = Sequence::new("ACGT").unwrap();
    let scoring = ScoringMatrix::default();

    let alignment = semi_global_alignment(&seq1, &seq2, &scoring);

    // Semi-global should find the overlap without penalizing end gaps
    assert!(alignment.score > 0);
}

// =============================================================================
// Alignment Statistics Tests
// =============================================================================

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
fn test_alignment_identity_with_gaps() {
    let alignment = Alignment {
        aligned_seq1: "ACGT".to_string(),
        aligned_seq2: "A-GT".to_string(),
        score: 4,
        start1: 0,
        end1: 4,
        start2: 0,
        end2: 3,
        matches: 3,
        mismatches: 0,
        gaps: 1,
        alignment_type: AlignmentType::Local,
    };

    // Identity should be 100% for non-gap positions
    assert!((alignment.identity() - 1.0).abs() < f64::EPSILON);
}

#[test]
fn test_alignment_length() {
    let alignment = Alignment {
        aligned_seq1: "ACGT".to_string(),
        aligned_seq2: "ACGT".to_string(),
        score: 8,
        start1: 0,
        end1: 4,
        start2: 0,
        end2: 4,
        matches: 4,
        mismatches: 0,
        gaps: 0,
        alignment_type: AlignmentType::Global,
    };

    assert_eq!(alignment.alignment_length(), 4);
}

#[test]
fn test_alignment_display() {
    let seq1 = Sequence::new("ACGT").unwrap();
    let seq2 = Sequence::new("ACGT").unwrap();
    let scoring = ScoringMatrix::default();

    let alignment = smith_waterman(&seq1, &seq2, &scoring);

    let display = format!("{}", alignment);
    assert!(display.contains("Alignment"));
    assert!(display.contains("Score"));
    assert!(display.contains("Identity"));
}

#[test]
fn test_alignment_format() {
    let seq1 = Sequence::new("ACGTACGT").unwrap();
    let seq2 = Sequence::new("ACGTACGT").unwrap();
    let scoring = ScoringMatrix::default();

    let alignment = smith_waterman(&seq1, &seq2, &scoring);

    let formatted = alignment.format_alignment(60);
    assert!(formatted.contains("Seq1"));
    assert!(formatted.contains("Seq2"));
}

// =============================================================================
// Edit Distance Tests
// =============================================================================

#[test]
fn test_edit_distance_identical() {
    let seq1 = Sequence::new("ACGT").unwrap();
    let seq2 = Sequence::new("ACGT").unwrap();

    assert_eq!(edit_distance(&seq1, &seq2), 0);
}

#[test]
fn test_edit_distance_one_substitution() {
    let seq1 = Sequence::new("ACGT").unwrap();
    let seq2 = Sequence::new("AGGT").unwrap();

    assert_eq!(edit_distance(&seq1, &seq2), 1);
}

#[test]
fn test_edit_distance_one_insertion() {
    let seq1 = Sequence::new("ACGT").unwrap();
    let seq2 = Sequence::new("ACT").unwrap();

    assert_eq!(edit_distance(&seq1, &seq2), 1);
}

#[test]
fn test_edit_distance_multiple() {
    let seq1 = Sequence::new("ACGT").unwrap();
    let seq2 = Sequence::new("TTTT").unwrap();

    let distance = edit_distance(&seq1, &seq2);
    assert!(distance > 0);
    assert!(distance <= 4);
}

#[test]
fn test_edit_distance_empty() {
    let seq1 = Sequence::new("ACGT").unwrap();
    let seq2 = Sequence::new("A").unwrap();

    let distance = edit_distance(&seq1, &seq2);
    assert_eq!(distance, 3);
}

// =============================================================================
// Hamming Distance Tests
// =============================================================================

#[test]
fn test_hamming_distance_identical() {
    let seq1 = Sequence::new("ACGT").unwrap();
    let seq2 = Sequence::new("ACGT").unwrap();

    assert_eq!(hamming_distance(&seq1, &seq2), Some(0));
}

#[test]
fn test_hamming_distance_one_difference() {
    let seq1 = Sequence::new("ACGT").unwrap();
    let seq2 = Sequence::new("AGGT").unwrap();

    assert_eq!(hamming_distance(&seq1, &seq2), Some(1));
}

#[test]
fn test_hamming_distance_all_different() {
    let seq1 = Sequence::new("AAAA").unwrap();
    let seq2 = Sequence::new("TTTT").unwrap();

    assert_eq!(hamming_distance(&seq1, &seq2), Some(4));
}

#[test]
fn test_hamming_distance_different_lengths() {
    let seq1 = Sequence::new("ACGT").unwrap();
    let seq2 = Sequence::new("ACG").unwrap();

    assert_eq!(hamming_distance(&seq1, &seq2), None);
}

// =============================================================================
// Performance Tests
// =============================================================================

#[test]
fn test_alignment_medium_sequences() {
    let seq1 = Sequence::new("ACGT".repeat(100)).unwrap();
    let seq2 = Sequence::new("AGCT".repeat(100)).unwrap();
    let scoring = ScoringMatrix::default();

    let alignment = smith_waterman(&seq1, &seq2, &scoring);

    assert!(alignment.score > 0);
}

#[test]
fn test_alignment_long_sequences() {
    let seq1 = Sequence::new("ACGT".repeat(250)).unwrap();
    let seq2 = Sequence::new("AGCT".repeat(250)).unwrap();
    let scoring = ScoringMatrix::default();

    let alignment = smith_waterman(&seq1, &seq2, &scoring);

    assert!(alignment.score > 0);
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_alignment_single_base() {
    let seq1 = Sequence::new("A").unwrap();
    let seq2 = Sequence::new("A").unwrap();
    let scoring = ScoringMatrix::default();

    let alignment = smith_waterman(&seq1, &seq2, &scoring);

    assert_eq!(alignment.score, 2);
    assert_eq!(alignment.matches, 1);
}

#[test]
fn test_alignment_single_base_mismatch() {
    let seq1 = Sequence::new("A").unwrap();
    let seq2 = Sequence::new("T").unwrap();
    let scoring = ScoringMatrix::default();

    let alignment = smith_waterman(&seq1, &seq2, &scoring);

    assert_eq!(alignment.score, 0);
}

#[test]
fn test_alignment_with_ns() {
    let seq1 = Sequence::new("ACNGT").unwrap();
    let seq2 = Sequence::new("ACNGT").unwrap();
    let scoring = ScoringMatrix::default();

    let alignment = smith_waterman(&seq1, &seq2, &scoring);

    // N should be treated as mismatch
    assert!(alignment.score > 0);
}

#[test]
fn test_alignment_type_local() {
    let seq1 = Sequence::new("ACGT").unwrap();
    let seq2 = Sequence::new("ACGT").unwrap();
    let scoring = ScoringMatrix::default();

    let alignment = smith_waterman(&seq1, &seq2, &scoring);

    assert_eq!(alignment.alignment_type, AlignmentType::Local);
}

#[test]
fn test_alignment_type_global() {
    let seq1 = Sequence::new("ACGT").unwrap();
    let seq2 = Sequence::new("ACGT").unwrap();
    let scoring = ScoringMatrix::default();

    let alignment = needleman_wunsch(&seq1, &seq2, &scoring);

    assert_eq!(alignment.alignment_type, AlignmentType::Global);
}
