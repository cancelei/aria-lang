package alignment

import (
	"fmt"
	"strings"

	"github.com/aria-lang/bioflow-go/internal/sequence"
)

// NeedlemanWunsch performs global alignment using the Needleman-Wunsch algorithm.
//
// Aligns the entire length of both sequences.
//
// Aria equivalent:
//
//	fn needleman_wunsch(seq1: Sequence, seq2: Sequence, scoring: ScoringMatrix) -> Alignment
//	  requires seq1.is_valid() and seq2.is_valid()
//	  requires seq1.len() > 0 and seq2.len() > 0
//	  ensures result.aligned_seq1.len() == result.aligned_seq2.len()
func NeedlemanWunsch(seq1, seq2 *sequence.Sequence, scoring *ScoringMatrix) (*Alignment, error) {
	if scoring == nil {
		scoring = DefaultDNA()
	}

	if seq1.Len() == 0 || seq2.Len() == 0 {
		return nil, fmt.Errorf("sequences must be non-empty")
	}

	m, n := seq1.Len(), seq2.Len()
	s1, s2 := seq1.Bases, seq2.Bases

	// Initialize scoring matrix with gap penalties
	H := make([][]int, m+1)
	traceback := make([][]AlignDirection, m+1)
	for i := range H {
		H[i] = make([]int, n+1)
		traceback[i] = make([]AlignDirection, n+1)
	}

	// First row and column initialized with gap penalties
	for i := 0; i <= m; i++ {
		H[i][0] = i * scoring.GapPenalty()
		if i > 0 {
			traceback[i][0] = Up
		}
	}
	for j := 0; j <= n; j++ {
		H[0][j] = j * scoring.GapPenalty()
		if j > 0 {
			traceback[0][j] = Left
		}
	}

	// Fill matrices
	for i := 1; i <= m; i++ {
		for j := 1; j <= n; j++ {
			matchScore := scoring.Score(rune(s1[i-1]), rune(s2[j-1]))

			diag := H[i-1][j-1] + matchScore
			up := H[i-1][j] + scoring.GapPenalty()
			left := H[i][j-1] + scoring.GapPenalty()

			// Find maximum (no zero threshold for global)
			best := diag
			direction := Diagonal

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
		}
	}

	// Traceback from bottom-right corner
	aligned1, aligned2 := tracebackGlobal(s1, s2, traceback, m, n)

	return NewAlignment(aligned1, aligned2, H[m][n], Global)
}

// tracebackGlobal performs traceback for global alignment.
func tracebackGlobal(seq1, seq2 string, traceback [][]AlignDirection, m, n int) (string, string) {
	var aligned1, aligned2 strings.Builder
	i, j := m, n

	for i > 0 || j > 0 {
		if i == 0 {
			aligned1.WriteByte('-')
			aligned2.WriteByte(seq2[j-1])
			j--
		} else if j == 0 {
			aligned1.WriteByte(seq1[i-1])
			aligned2.WriteByte('-')
			i--
		} else {
			direction := traceback[i][j]

			switch direction {
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
			default:
				break
			}
		}
	}

	a1 := aligned1.String()
	a2 := aligned2.String()
	return reverse(a1), reverse(a2)
}

// SemiGlobalAlignment performs semi-global alignment.
//
// This is useful when one sequence should fit entirely within another,
// like aligning a read to a reference.
func SemiGlobalAlignment(seq1, seq2 *sequence.Sequence, scoring *ScoringMatrix) (*Alignment, error) {
	if scoring == nil {
		scoring = DefaultDNA()
	}

	if seq1.Len() == 0 || seq2.Len() == 0 {
		return nil, fmt.Errorf("sequences must be non-empty")
	}

	m, n := seq1.Len(), seq2.Len()
	s1, s2 := seq1.Bases, seq2.Bases

	// Initialize scoring matrix
	H := make([][]int, m+1)
	traceback := make([][]AlignDirection, m+1)
	for i := range H {
		H[i] = make([]int, n+1)
		traceback[i] = make([]AlignDirection, n+1)
	}

	// First row initialized with zeros (no penalty for gaps at start of seq1)
	// First column initialized with gap penalties
	for i := 1; i <= m; i++ {
		H[i][0] = i * scoring.GapPenalty()
		traceback[i][0] = Up
	}

	// Fill matrices
	for i := 1; i <= m; i++ {
		for j := 1; j <= n; j++ {
			matchScore := scoring.Score(rune(s1[i-1]), rune(s2[j-1]))

			diag := H[i-1][j-1] + matchScore
			up := H[i-1][j] + scoring.GapPenalty()
			left := H[i][j-1] + scoring.GapPenalty()

			best := diag
			direction := Diagonal

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
		}
	}

	// Find best score in last row (allowing free end gaps in seq1)
	maxScore := H[m][0]
	maxJ := 0
	for j := 1; j <= n; j++ {
		if H[m][j] > maxScore {
			maxScore = H[m][j]
			maxJ = j
		}
	}

	// Traceback
	aligned1, aligned2 := tracebackGlobal(s1, s2, traceback, m, maxJ)

	// Add trailing gaps if needed
	for j := maxJ + 1; j <= n; j++ {
		aligned1 = aligned1 + "-"
		aligned2 = aligned2 + string(s2[j-1])
	}

	return NewAlignment(aligned1, aligned2, maxScore, SemiGlobal)
}

// AlignAgainstMultiple aligns a sequence against multiple targets.
//
// Aria equivalent:
//
//	fn align_against_multiple(query: Sequence, targets: [Sequence], scoring: ScoringMatrix)
//	  -> [(Int, Alignment)]
//	  requires query.is_valid()
//	  requires targets.len() > 0
//	  ensures result.len() == targets.len()
func AlignAgainstMultiple(query *sequence.Sequence, targets []*sequence.Sequence,
	scoring *ScoringMatrix) ([]IndexedAlignment, error) {
	if scoring == nil {
		scoring = DefaultDNA()
	}

	if len(targets) == 0 {
		return nil, fmt.Errorf("target list cannot be empty")
	}

	results := make([]IndexedAlignment, len(targets))
	for i, target := range targets {
		alignment, err := SmithWaterman(query, target, scoring)
		if err != nil {
			return nil, err
		}
		results[i] = IndexedAlignment{Index: i, Alignment: alignment}
	}

	return results, nil
}

// IndexedAlignment pairs an alignment with its index.
type IndexedAlignment struct {
	Index     int
	Alignment *Alignment
}

// FindBestAlignment finds the best alignment among multiple targets.
//
// Aria equivalent:
//
//	fn find_best_alignment(query: Sequence, targets: [Sequence], scoring: ScoringMatrix)
//	  -> Option<(Int, Alignment)>
//	  requires query.is_valid()
//	  requires targets.len() > 0
func FindBestAlignment(query *sequence.Sequence, targets []*sequence.Sequence,
	scoring *ScoringMatrix) (*IndexedAlignment, error) {
	alignments, err := AlignAgainstMultiple(query, targets, scoring)
	if err != nil {
		return nil, err
	}

	if len(alignments) == 0 {
		return nil, nil
	}

	best := alignments[0]
	for _, a := range alignments[1:] {
		if a.Alignment.Score > best.Alignment.Score {
			best = a
		}
	}

	return &best, nil
}

// GlobalAlignmentScoreOnly calculates global alignment score without traceback.
func GlobalAlignmentScoreOnly(seq1, seq2 *sequence.Sequence, scoring *ScoringMatrix) (int, error) {
	if scoring == nil {
		scoring = DefaultDNA()
	}

	if seq1.Len() == 0 || seq2.Len() == 0 {
		return 0, fmt.Errorf("sequences must be non-empty")
	}

	m, n := seq1.Len(), seq2.Len()
	s1, s2 := seq1.Bases, seq2.Bases

	// Use two rows
	prevRow := make([]int, n+1)
	currRow := make([]int, n+1)

	// Initialize first row
	for j := 0; j <= n; j++ {
		prevRow[j] = j * scoring.GapPenalty()
	}

	for i := 1; i <= m; i++ {
		currRow[0] = i * scoring.GapPenalty()

		for j := 1; j <= n; j++ {
			matchScore := scoring.Score(rune(s1[i-1]), rune(s2[j-1]))

			diag := prevRow[j-1] + matchScore
			up := prevRow[j] + scoring.GapPenalty()
			left := currRow[j-1] + scoring.GapPenalty()

			currRow[j] = max(diag, max(up, left))
		}

		prevRow, currRow = currRow, prevRow
	}

	return prevRow[n], nil
}
