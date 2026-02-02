package quality

import (
	"fmt"

	"github.com/aria-lang/bioflow-go/internal/sequence"
)

// FilterResult represents the result of quality filtering.
type FilterResult struct {
	Passed     bool
	Reason     string
	TrimStart  int
	TrimEnd    int
	MeanQuality float64
}

// Filter represents a quality filter configuration.
type Filter struct {
	MinQuality         int     // Minimum average quality
	MinLength          int     // Minimum sequence length after trimming
	MaxAmbiguous       int     // Maximum number of N bases allowed
	QualityThreshold   int     // Threshold for quality-based trimming
	WindowSize         int     // Window size for sliding window trimming
	MinWindowQuality   float64 // Minimum average quality in window
}

// DefaultFilter creates a filter with default settings.
func DefaultFilter() *Filter {
	return &Filter{
		MinQuality:       20,
		MinLength:        50,
		MaxAmbiguous:     5,
		QualityThreshold: 20,
		WindowSize:       4,
		MinWindowQuality: 20.0,
	}
}

// StrictFilter creates a filter with strict settings.
func StrictFilter() *Filter {
	return &Filter{
		MinQuality:       30,
		MinLength:        100,
		MaxAmbiguous:     0,
		QualityThreshold: 30,
		WindowSize:       5,
		MinWindowQuality: 28.0,
	}
}

// Check checks if a sequence and its quality scores pass the filter.
func (f *Filter) Check(seq *sequence.Sequence, scores *Scores) (*FilterResult, error) {
	if seq.Len() != scores.Len() {
		return nil, fmt.Errorf("sequence and quality scores must have the same length")
	}

	result := &FilterResult{
		Passed:      true,
		TrimStart:   0,
		TrimEnd:     seq.Len(),
		MeanQuality: scores.Average(),
	}

	// Check average quality
	if result.MeanQuality < float64(f.MinQuality) {
		result.Passed = false
		result.Reason = fmt.Sprintf("average quality %.2f below minimum %d", result.MeanQuality, f.MinQuality)
		return result, nil
	}

	// Check ambiguous bases
	ambiguous := seq.CountAmbiguous()
	if ambiguous > f.MaxAmbiguous {
		result.Passed = false
		result.Reason = fmt.Sprintf("too many ambiguous bases: %d (max: %d)", ambiguous, f.MaxAmbiguous)
		return result, nil
	}

	// Check length
	if seq.Len() < f.MinLength {
		result.Passed = false
		result.Reason = fmt.Sprintf("sequence too short: %d (min: %d)", seq.Len(), f.MinLength)
		return result, nil
	}

	return result, nil
}

// TrimByQuality trims a sequence based on quality scores.
// Returns the start and end indices for trimming.
func (f *Filter) TrimByQuality(scores *Scores) (int, int) {
	n := scores.Len()

	// Find trim start
	trimStart := 0
	for i := 0; i < n; i++ {
		if scores.Values[i] >= f.QualityThreshold {
			trimStart = i
			break
		}
	}

	// Find trim end
	trimEnd := n
	for i := n - 1; i >= trimStart; i-- {
		if scores.Values[i] >= f.QualityThreshold {
			trimEnd = i + 1
			break
		}
	}

	return trimStart, trimEnd
}

// SlidingWindowTrim performs sliding window quality trimming.
// Trims from both ends when the average quality in a window drops below threshold.
func (f *Filter) SlidingWindowTrim(scores *Scores) (int, int) {
	n := scores.Len()
	if n < f.WindowSize {
		return 0, n
	}

	// Find trim start using sliding window
	trimStart := 0
	for i := 0; i <= n-f.WindowSize; i++ {
		windowSum := 0
		for j := 0; j < f.WindowSize; j++ {
			windowSum += scores.Values[i+j]
		}
		windowAvg := float64(windowSum) / float64(f.WindowSize)

		if windowAvg >= f.MinWindowQuality {
			trimStart = i
			break
		}
	}

	// Find trim end using sliding window
	trimEnd := n
	for i := n - f.WindowSize; i >= trimStart; i-- {
		windowSum := 0
		for j := 0; j < f.WindowSize; j++ {
			windowSum += scores.Values[i+j]
		}
		windowAvg := float64(windowSum) / float64(f.WindowSize)

		if windowAvg >= f.MinWindowQuality {
			trimEnd = i + f.WindowSize
			break
		}
	}

	return trimStart, trimEnd
}

// TrimAndFilter trims a sequence based on quality and checks if it passes filters.
func (f *Filter) TrimAndFilter(seq *sequence.Sequence, scores *Scores) (*TrimAndFilterResult, error) {
	if seq.Len() != scores.Len() {
		return nil, fmt.Errorf("sequence and quality scores must have the same length")
	}

	// Perform sliding window trimming
	trimStart, trimEnd := f.SlidingWindowTrim(scores)

	// Check if remaining sequence is long enough
	trimmedLen := trimEnd - trimStart
	if trimmedLen < f.MinLength {
		return &TrimAndFilterResult{
			Passed:      false,
			Reason:      fmt.Sprintf("sequence too short after trimming: %d (min: %d)", trimmedLen, f.MinLength),
			TrimStart:   trimStart,
			TrimEnd:     trimEnd,
			TrimmedSeq:  nil,
			TrimmedQual: nil,
		}, nil
	}

	// Create trimmed sequence and quality
	trimmedSeq, err := seq.Subsequence(trimStart, trimEnd)
	if err != nil {
		return nil, err
	}

	trimmedQual, err := scores.Slice(trimStart, trimEnd)
	if err != nil {
		return nil, err
	}

	// Check the trimmed sequence
	result, err := f.Check(trimmedSeq, trimmedQual)
	if err != nil {
		return nil, err
	}

	return &TrimAndFilterResult{
		Passed:      result.Passed,
		Reason:      result.Reason,
		TrimStart:   trimStart,
		TrimEnd:     trimEnd,
		TrimmedSeq:  trimmedSeq,
		TrimmedQual: trimmedQual,
		MeanQuality: result.MeanQuality,
	}, nil
}

// TrimAndFilterResult represents the result of trimming and filtering.
type TrimAndFilterResult struct {
	Passed      bool
	Reason      string
	TrimStart   int
	TrimEnd     int
	TrimmedSeq  *sequence.Sequence
	TrimmedQual *Scores
	MeanQuality float64
}

// BatchFilter filters multiple sequences.
func (f *Filter) BatchFilter(sequences []*sequence.Sequence, qualities []*Scores) (*BatchFilterResult, error) {
	if len(sequences) != len(qualities) {
		return nil, fmt.Errorf("sequences and qualities must have the same length")
	}

	result := &BatchFilterResult{
		PassedSequences: make([]*sequence.Sequence, 0),
		PassedQualities: make([]*Scores, 0),
		FailedIndices:   make([]int, 0),
		FailReasons:     make(map[int]string),
	}

	for i := range sequences {
		filterResult, err := f.TrimAndFilter(sequences[i], qualities[i])
		if err != nil {
			return nil, err
		}

		if filterResult.Passed {
			result.PassedSequences = append(result.PassedSequences, filterResult.TrimmedSeq)
			result.PassedQualities = append(result.PassedQualities, filterResult.TrimmedQual)
		} else {
			result.FailedIndices = append(result.FailedIndices, i)
			result.FailReasons[i] = filterResult.Reason
		}
	}

	result.TotalProcessed = len(sequences)
	result.PassedCount = len(result.PassedSequences)
	result.FailedCount = len(result.FailedIndices)

	return result, nil
}

// BatchFilterResult represents the result of batch filtering.
type BatchFilterResult struct {
	TotalProcessed   int
	PassedCount      int
	FailedCount      int
	PassedSequences  []*sequence.Sequence
	PassedQualities  []*Scores
	FailedIndices    []int
	FailReasons      map[int]string
}

// PassRate returns the proportion of sequences that passed filtering.
func (r *BatchFilterResult) PassRate() float64 {
	if r.TotalProcessed == 0 {
		return 0.0
	}
	return float64(r.PassedCount) / float64(r.TotalProcessed)
}

func (r *BatchFilterResult) String() string {
	return fmt.Sprintf("BatchFilterResult { processed: %d, passed: %d (%.1f%%), failed: %d }",
		r.TotalProcessed, r.PassedCount, r.PassRate()*100, r.FailedCount)
}

// QualityTrimmer provides quality-based sequence trimming functionality.
type QualityTrimmer struct {
	Threshold int // Quality threshold for trimming
}

// NewQualityTrimmer creates a new quality trimmer.
func NewQualityTrimmer(threshold int) *QualityTrimmer {
	return &QualityTrimmer{Threshold: threshold}
}

// Trim trims low-quality bases from both ends.
func (t *QualityTrimmer) Trim(scores *Scores) (int, int) {
	n := scores.Len()

	// Find trim start
	trimStart := 0
	for i := 0; i < n; i++ {
		if scores.Values[i] >= t.Threshold {
			trimStart = i
			break
		}
	}

	// Find trim end
	trimEnd := n
	for i := n - 1; i >= trimStart; i-- {
		if scores.Values[i] >= t.Threshold {
			trimEnd = i + 1
			break
		}
	}

	return trimStart, trimEnd
}

// TrimSequence trims a sequence based on quality scores.
func (t *QualityTrimmer) TrimSequence(seq *sequence.Sequence, scores *Scores) (*sequence.Sequence, *Scores, error) {
	if seq.Len() != scores.Len() {
		return nil, nil, fmt.Errorf("sequence and quality scores must have the same length")
	}

	trimStart, trimEnd := t.Trim(scores)

	if trimEnd <= trimStart {
		return nil, nil, fmt.Errorf("no high-quality bases found")
	}

	trimmedSeq, err := seq.Subsequence(trimStart, trimEnd)
	if err != nil {
		return nil, nil, err
	}

	trimmedQual, err := scores.Slice(trimStart, trimEnd)
	if err != nil {
		return nil, nil, err
	}

	return trimmedSeq, trimmedQual, nil
}
