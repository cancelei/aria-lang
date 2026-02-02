// Package alignment provides sequence alignment algorithms.
//
// This package implements Smith-Waterman (local) and Needleman-Wunsch (global)
// alignment algorithms for comparing genomic sequences.
package alignment

import "fmt"

// AlignDirection represents the traceback direction in the alignment matrix.
type AlignDirection int

const (
	// Stop represents the end of alignment (local only)
	Stop AlignDirection = iota
	// Diagonal represents a match or mismatch
	Diagonal
	// Up represents a gap in sequence 2
	Up
	// Left represents a gap in sequence 1
	Left
)

// AlignmentType represents the type of alignment.
type AlignmentType int

const (
	// Local represents Smith-Waterman local alignment
	Local AlignmentType = iota
	// Global represents Needleman-Wunsch global alignment
	Global
	// SemiGlobal represents a hybrid approach
	SemiGlobal
)

func (t AlignmentType) String() string {
	switch t {
	case Local:
		return "local"
	case Global:
		return "global"
	case SemiGlobal:
		return "semi-global"
	default:
		return "unknown"
	}
}

// ScoringMatrix represents the scoring parameters for alignment.
//
// Aria equivalent:
//
//	struct ScoringMatrix
//	  match_score: Int
//	  mismatch_penalty: Int
//	  gap_open_penalty: Int
//	  gap_extend_penalty: Int
//	  invariant self.match_score > 0
//	  invariant self.mismatch_penalty <= 0
//	  invariant self.gap_open_penalty <= 0
//	  invariant self.gap_extend_penalty <= 0
type ScoringMatrix struct {
	MatchScore       int
	MismatchPenalty  int
	GapOpenPenalty   int
	GapExtendPenalty int
}

// NewScoringMatrix creates a new scoring matrix with validation.
func NewScoringMatrix(match, mismatch, gapOpen, gapExtend int) (*ScoringMatrix, error) {
	if match <= 0 {
		return nil, fmt.Errorf("match score must be positive")
	}
	if mismatch > 0 {
		return nil, fmt.Errorf("mismatch penalty should be <= 0")
	}
	if gapOpen > 0 {
		return nil, fmt.Errorf("gap open penalty should be <= 0")
	}
	if gapExtend > 0 {
		return nil, fmt.Errorf("gap extend penalty should be <= 0")
	}

	return &ScoringMatrix{
		MatchScore:       match,
		MismatchPenalty:  mismatch,
		GapOpenPenalty:   gapOpen,
		GapExtendPenalty: gapExtend,
	}, nil
}

// DefaultDNA creates a default DNA scoring matrix.
func DefaultDNA() *ScoringMatrix {
	return &ScoringMatrix{
		MatchScore:       2,
		MismatchPenalty:  -1,
		GapOpenPenalty:   -2,
		GapExtendPenalty: -1,
	}
}

// BLASTLike creates a BLAST-like scoring matrix.
func BLASTLike() *ScoringMatrix {
	return &ScoringMatrix{
		MatchScore:       1,
		MismatchPenalty:  -3,
		GapOpenPenalty:   -5,
		GapExtendPenalty: -2,
	}
}

// Simple creates a simple scoring matrix with uniform gap penalty.
func Simple(match, mismatch, gap int) (*ScoringMatrix, error) {
	return NewScoringMatrix(match, mismatch, gap, gap)
}

// Score returns the score for comparing two bases.
func (s *ScoringMatrix) Score(base1, base2 rune) int {
	if base1 == base2 {
		return s.MatchScore
	}
	return s.MismatchPenalty
}

// GapPenalty returns the linear gap penalty.
func (s *ScoringMatrix) GapPenalty() int {
	return s.GapOpenPenalty
}

// String returns a string representation of the scoring matrix.
func (s *ScoringMatrix) String() string {
	return fmt.Sprintf("ScoringMatrix { match: %d, mismatch: %d, gap_open: %d, gap_extend: %d }",
		s.MatchScore, s.MismatchPenalty, s.GapOpenPenalty, s.GapExtendPenalty)
}
