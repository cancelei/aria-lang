package alignment

import (
	"fmt"
	"strings"

	"github.com/aria-lang/bioflow-go/internal/sequence"
)

// Alignment represents the result of an alignment between two sequences.
//
// Aria equivalent:
//
//	struct Alignment
//	  aligned_seq1: String
//	  aligned_seq2: String
//	  score: Int
//	  start1: Int
//	  end1: Int
//	  start2: Int
//	  end2: Int
//	  alignment_type: AlignmentType
//	  identity: Float
//	  invariant self.aligned_seq1.len() == self.aligned_seq2.len()
//	  invariant self.identity >= 0.0 and self.identity <= 1.0
type Alignment struct {
	AlignedSeq1   string
	AlignedSeq2   string
	Score         int
	Start1        int
	End1          int
	Start2        int
	End2          int
	AlignmentType AlignmentType
	Identity      float64
}

// NewAlignment creates a new alignment result.
func NewAlignment(aligned1, aligned2 string, score int, alignType AlignmentType) (*Alignment, error) {
	if len(aligned1) != len(aligned2) {
		return nil, fmt.Errorf("aligned sequences must have equal length")
	}

	a := &Alignment{
		AlignedSeq1:   aligned1,
		AlignedSeq2:   aligned2,
		Score:         score,
		Start1:        0,
		End1:          len(aligned1),
		Start2:        0,
		End2:          len(aligned2),
		AlignmentType: alignType,
	}
	a.Identity = a.calculateIdentity()
	return a, nil
}

// NewAlignmentWithPositions creates an alignment with position information.
func NewAlignmentWithPositions(aligned1, aligned2 string, score int,
	start1, end1, start2, end2 int, alignType AlignmentType) (*Alignment, error) {
	if len(aligned1) != len(aligned2) {
		return nil, fmt.Errorf("aligned sequences must have equal length")
	}

	a := &Alignment{
		AlignedSeq1:   aligned1,
		AlignedSeq2:   aligned2,
		Score:         score,
		Start1:        start1,
		End1:          end1,
		Start2:        start2,
		End2:          end2,
		AlignmentType: alignType,
	}
	a.Identity = a.calculateIdentity()
	return a, nil
}

// calculateIdentity calculates the sequence identity.
func (a *Alignment) calculateIdentity() float64 {
	if len(a.AlignedSeq1) == 0 {
		return 0.0
	}

	matches := 0
	for i := 0; i < len(a.AlignedSeq1); i++ {
		if a.AlignedSeq1[i] == a.AlignedSeq2[i] && a.AlignedSeq1[i] != '-' {
			matches++
		}
	}
	return float64(matches) / float64(len(a.AlignedSeq1))
}

// Length returns the length of the alignment.
func (a *Alignment) Length() int {
	return len(a.AlignedSeq1)
}

// MatchCount returns the number of matches.
func (a *Alignment) MatchCount() int {
	count := 0
	for i := 0; i < len(a.AlignedSeq1); i++ {
		if a.AlignedSeq1[i] == a.AlignedSeq2[i] && a.AlignedSeq1[i] != '-' {
			count++
		}
	}
	return count
}

// MismatchCount returns the number of mismatches.
func (a *Alignment) MismatchCount() int {
	count := 0
	for i := 0; i < len(a.AlignedSeq1); i++ {
		if a.AlignedSeq1[i] != a.AlignedSeq2[i] &&
			a.AlignedSeq1[i] != '-' && a.AlignedSeq2[i] != '-' {
			count++
		}
	}
	return count
}

// GapsSeq1 returns the number of gaps in sequence 1.
func (a *Alignment) GapsSeq1() int {
	return strings.Count(a.AlignedSeq1, "-")
}

// GapsSeq2 returns the number of gaps in sequence 2.
func (a *Alignment) GapsSeq2() int {
	return strings.Count(a.AlignedSeq2, "-")
}

// TotalGaps returns the total number of gaps.
func (a *Alignment) TotalGaps() int {
	return a.GapsSeq1() + a.GapsSeq2()
}

// GapOpenings counts the number of gap openings.
func (a *Alignment) GapOpenings() int {
	openings := 0
	inGap1, inGap2 := false, false

	for i := 0; i < len(a.AlignedSeq1); i++ {
		if a.AlignedSeq1[i] == '-' && !inGap1 {
			openings++
			inGap1 = true
		} else if a.AlignedSeq1[i] != '-' {
			inGap1 = false
		}

		if a.AlignedSeq2[i] == '-' && !inGap2 {
			openings++
			inGap2 = true
		} else if a.AlignedSeq2[i] != '-' {
			inGap2 = false
		}
	}

	return openings
}

// ToCIGAR generates a CIGAR string representation.
func (a *Alignment) ToCIGAR() string {
	if len(a.AlignedSeq1) == 0 {
		return ""
	}

	var cigar strings.Builder
	currentOp := byte(0)
	count := 0

	for i := 0; i < len(a.AlignedSeq1); i++ {
		var op byte
		if a.AlignedSeq1[i] == '-' {
			op = 'I' // Insertion
		} else if a.AlignedSeq2[i] == '-' {
			op = 'D' // Deletion
		} else if a.AlignedSeq1[i] == a.AlignedSeq2[i] {
			op = 'M' // Match
		} else {
			op = 'X' // Mismatch
		}

		if op == currentOp {
			count++
		} else {
			if count > 0 {
				cigar.WriteString(fmt.Sprintf("%d%c", count, currentOp))
			}
			currentOp = op
			count = 1
		}
	}

	if count > 0 {
		cigar.WriteString(fmt.Sprintf("%d%c", count, currentOp))
	}

	return cigar.String()
}

// Format returns a formatted string representation of the alignment.
func (a *Alignment) Format() string {
	var matchLine strings.Builder
	for i := 0; i < len(a.AlignedSeq1); i++ {
		if a.AlignedSeq1[i] == a.AlignedSeq2[i] && a.AlignedSeq1[i] != '-' {
			matchLine.WriteByte('|')
		} else if a.AlignedSeq1[i] == '-' || a.AlignedSeq2[i] == '-' {
			matchLine.WriteByte(' ')
		} else {
			matchLine.WriteByte('.')
		}
	}

	return fmt.Sprintf("Seq1: %s\n      %s\nSeq2: %s\nScore: %d\nIdentity: %.1f%%\nCIGAR: %s",
		a.AlignedSeq1, matchLine.String(), a.AlignedSeq2,
		a.Score, a.Identity*100, a.ToCIGAR())
}

func (a *Alignment) String() string {
	return fmt.Sprintf("Alignment { score: %d, identity: %.1f%%, length: %d }",
		a.Score, a.Identity*100, a.Length())
}

// SmithWaterman performs local alignment using the Smith-Waterman algorithm.
//
// Finds the optimal local alignment between two sequences.
//
// Aria equivalent:
//
//	fn smith_waterman(seq1: Sequence, seq2: Sequence, scoring: ScoringMatrix) -> Alignment
//	  requires seq1.is_valid() and seq2.is_valid()
//	  requires seq1.len() > 0 and seq2.len() > 0
//	  ensures result.score >= 0
//	  ensures result.aligned_seq1.len() == result.aligned_seq2.len()
func SmithWaterman(seq1, seq2 *sequence.Sequence, scoring *ScoringMatrix) (*Alignment, error) {
	if scoring == nil {
		scoring = DefaultDNA()
	}

	if seq1.Len() == 0 || seq2.Len() == 0 {
		return nil, fmt.Errorf("sequences must be non-empty")
	}

	m, n := seq1.Len(), seq2.Len()
	s1, s2 := seq1.Bases, seq2.Bases

	// Initialize scoring matrix with zeros
	H := make([][]int, m+1)
	traceback := make([][]AlignDirection, m+1)
	for i := range H {
		H[i] = make([]int, n+1)
		traceback[i] = make([]AlignDirection, n+1)
	}

	// Track maximum score and position
	maxScore := 0
	maxI, maxJ := 0, 0

	// Fill matrices
	for i := 1; i <= m; i++ {
		for j := 1; j <= n; j++ {
			matchScore := scoring.Score(rune(s1[i-1]), rune(s2[j-1]))

			diag := H[i-1][j-1] + matchScore
			up := H[i-1][j] + scoring.GapPenalty()
			left := H[i][j-1] + scoring.GapPenalty()

			// Find maximum (including 0 for local alignment)
			best := 0
			direction := Stop

			if diag > best {
				best = diag
				direction = Diagonal
			}
			if up > best {
				best = up
				direction = Up
			}
			if left > best {
				best = left
				direction = Left
			}

			H[i][j] = best
			traceback[i][j] = direction

			if best > maxScore {
				maxScore = best
				maxI, maxJ = i, j
			}
		}
	}

	// Traceback
	aligned1, aligned2, start1, start2 := tracebackLocal(s1, s2, traceback, maxI, maxJ)

	return NewAlignmentWithPositions(aligned1, aligned2, maxScore,
		start1, maxI, start2, maxJ, Local)
}

// tracebackLocal performs traceback for local alignment.
func tracebackLocal(seq1, seq2 string, traceback [][]AlignDirection,
	startI, startJ int) (string, string, int, int) {
	var aligned1, aligned2 strings.Builder
	i, j := startI, startJ

	for i > 0 && j > 0 {
		direction := traceback[i][j]

		switch direction {
		case Stop:
			goto done
		case Diagonal:
			aligned1.WriteByte(seq1[i-1])
			aligned2.WriteByte(seq2[j-1])
			i--
			j--
		case Up:
			aligned1.WriteByte(seq1[i-1])
			aligned2.WriteByte('-')
			i--
		case Left:
			aligned1.WriteByte('-')
			aligned2.WriteByte(seq2[j-1])
			j--
		}
	}
done:

	// Reverse the strings
	a1 := aligned1.String()
	a2 := aligned2.String()
	return reverse(a1), reverse(a2), i, j
}

// reverse reverses a string.
func reverse(s string) string {
	runes := []rune(s)
	for i, j := 0, len(runes)-1; i < j; i, j = i+1, j-1 {
		runes[i], runes[j] = runes[j], runes[i]
	}
	return string(runes)
}

// AlignmentScoreOnly calculates alignment score without full traceback (memory efficient).
//
// Uses O(n) space instead of O(m*n) by only keeping two rows.
//
// Aria equivalent:
//
//	fn alignment_score_only(seq1: Sequence, seq2: Sequence, scoring: ScoringMatrix) -> Int
//	  requires seq1.is_valid() and seq2.is_valid()
//	  requires seq1.len() > 0 and seq2.len() > 0
func AlignmentScoreOnly(seq1, seq2 *sequence.Sequence, scoring *ScoringMatrix) (int, error) {
	if scoring == nil {
		scoring = DefaultDNA()
	}

	if seq1.Len() == 0 || seq2.Len() == 0 {
		return 0, fmt.Errorf("sequences must be non-empty")
	}

	m, n := seq1.Len(), seq2.Len()
	s1, s2 := seq1.Bases, seq2.Bases

	// Use two rows instead of full matrix
	prevRow := make([]int, n+1)
	currRow := make([]int, n+1)

	maxScore := 0

	for i := 1; i <= m; i++ {
		// Reset current row
		for j := range currRow {
			currRow[j] = 0
		}

		for j := 1; j <= n; j++ {
			matchScore := scoring.Score(rune(s1[i-1]), rune(s2[j-1]))

			diag := prevRow[j-1] + matchScore
			up := prevRow[j] + scoring.GapPenalty()
			left := currRow[j-1] + scoring.GapPenalty()

			best := max(0, max(diag, max(up, left)))
			currRow[j] = best

			if best > maxScore {
				maxScore = best
			}
		}

		// Swap rows
		prevRow, currRow = currRow, prevRow
	}

	return maxScore, nil
}

// max returns the maximum of two integers.
func max(a, b int) int {
	if a > b {
		return a
	}
	return b
}

// SimpleAlign performs alignment using default settings.
//
// Aria equivalent:
//
//	fn simple_align(seq1: Sequence, seq2: Sequence) -> Alignment
//	  requires seq1.is_valid() and seq2.is_valid()
//	  requires seq1.len() > 0 and seq2.len() > 0
func SimpleAlign(seq1, seq2 *sequence.Sequence) (*Alignment, error) {
	return SmithWaterman(seq1, seq2, DefaultDNA())
}

// PercentIdentity calculates percent identity between two aligned sequences.
//
// Aria equivalent:
//
//	fn percent_identity(aligned1: String, aligned2: String) -> Float
//	  requires aligned1.len() == aligned2.len()
//	  requires aligned1.len() > 0
//	  ensures result >= 0.0 and result <= 100.0
func PercentIdentity(aligned1, aligned2 string) (float64, error) {
	if len(aligned1) != len(aligned2) {
		return 0, fmt.Errorf("aligned sequences must have equal length")
	}
	if len(aligned1) == 0 {
		return 0, fmt.Errorf("aligned sequences cannot be empty")
	}

	matches := 0
	for i := 0; i < len(aligned1); i++ {
		if aligned1[i] == aligned2[i] && aligned1[i] != '-' {
			matches++
		}
	}

	return float64(matches) / float64(len(aligned1)) * 100.0, nil
}
