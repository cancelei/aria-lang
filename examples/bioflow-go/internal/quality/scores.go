// Package quality provides Phred quality score handling for sequencing reads.
//
// Phred quality scores are logarithmically related to base-calling error probabilities:
//
//	Q = -10 * log10(P_error)
//
// Common thresholds:
//
//	Q10 = 90% accuracy
//	Q20 = 99% accuracy
//	Q30 = 99.9% accuracy (typical threshold for "high quality")
//	Q40 = 99.99% accuracy
//
// Comparison with Aria:
//
//	Aria uses invariants for compile-time guarantees:
//	  struct QualityScores
//	    invariant self.scores.len() > 0
//	    invariant self.all_in_range()
//
//	Go uses runtime checks.
package quality

import (
	"fmt"
	"math"
	"sort"
)

// Constants for Phred scores
const (
	PhredMin = 0
	PhredMax = 40
)

// Quality thresholds
const (
	QLow       = 10 // 90% accuracy
	QMedium    = 20 // 99% accuracy
	QHigh      = 30 // 99.9% accuracy
	QExcellent = 40 // 99.99% accuracy
)

// Category represents quality category.
type Category int

const (
	// Poor represents quality < 10
	Poor Category = iota
	// Low represents quality 10-20
	Low
	// Medium represents quality 20-30
	Medium
	// High represents quality 30-40
	High
	// Excellent represents quality >= 40
	Excellent
)

func (c Category) String() string {
	switch c {
	case Poor:
		return "Poor"
	case Low:
		return "Low"
	case Medium:
		return "Medium"
	case High:
		return "High"
	case Excellent:
		return "Excellent"
	default:
		return "Unknown"
	}
}

// Error types
type QualityError interface {
	error
	IsQualityError()
}

// EmptyScoresError is returned when quality scores are empty.
type EmptyScoresError struct{}

func (e *EmptyScoresError) Error() string {
	return "quality scores cannot be empty"
}
func (e *EmptyScoresError) IsQualityError() {}

// ScoreOutOfRangeError is returned when a score is out of valid range.
type ScoreOutOfRangeError struct {
	Position int
	Score    int
}

func (e *ScoreOutOfRangeError) Error() string {
	return fmt.Sprintf("score %d at position %d is out of range [0, 40]", e.Score, e.Position)
}
func (e *ScoreOutOfRangeError) IsQualityError() {}

// InvalidEncodingError is returned when a quality encoding character is invalid.
type InvalidEncodingError struct {
	Char rune
}

func (e *InvalidEncodingError) Error() string {
	return fmt.Sprintf("invalid encoding character: '%c'", e.Char)
}
func (e *InvalidEncodingError) IsQualityError() {}

// Scores represents quality scores for a sequencing read.
//
// Each score corresponds to a base in a sequence.
//
// Aria equivalent:
//
//	struct QualityScores
//	  scores: [Int]
//	  invariant self.scores.len() > 0
//	  invariant self.all_in_range()
type Scores struct {
	Values []int
}

// New creates new quality scores from an array of integers.
//
// Aria equivalent:
//
//	fn new(scores: [Int]) -> Result<QualityScores, QualityError>
//	  requires scores.len() > 0
//	  ensures result.is_ok() implies result.unwrap().len() == scores.len()
func New(scores []int) (*Scores, error) {
	if len(scores) == 0 {
		return nil, &EmptyScoresError{}
	}

	for i, score := range scores {
		if score < PhredMin || score > PhredMax {
			return nil, &ScoreOutOfRangeError{Position: i, Score: score}
		}
	}

	// Make a copy to avoid external mutation
	values := make([]int, len(scores))
	copy(values, scores)

	return &Scores{Values: values}, nil
}

// FromPhred33 creates quality scores from a Phred+33 encoded string (Illumina 1.8+).
//
// Each ASCII character maps to a quality score: Q = ord(char) - 33
//
// Aria equivalent:
//
//	fn from_phred33(encoded: String) -> Result<QualityScores, QualityError>
//	  requires encoded.len() > 0
//	  ensures result.is_ok() implies result.unwrap().len() == encoded.len()
func FromPhred33(encoded string) (*Scores, error) {
	if len(encoded) == 0 {
		return nil, &EmptyScoresError{}
	}

	scores := make([]int, 0, len(encoded))
	for i, c := range encoded {
		asciiVal := int(c)

		// Phred+33 encoding: valid range is '!' (33) to 'J' (74) for Q0-Q41
		if asciiVal < 33 || asciiVal > 74 {
			return nil, &InvalidEncodingError{Char: c}
		}

		score := asciiVal - 33
		if score > PhredMax {
			return nil, &ScoreOutOfRangeError{Position: i, Score: score}
		}

		scores = append(scores, score)
	}

	return &Scores{Values: scores}, nil
}

// FromPhred64 creates quality scores from a Phred+64 encoded string (older Illumina).
//
// Each ASCII character maps to a quality score: Q = ord(char) - 64
//
// Aria equivalent:
//
//	fn from_phred64(encoded: String) -> Result<QualityScores, QualityError>
//	  requires encoded.len() > 0
//	  ensures result.is_ok() implies result.unwrap().len() == encoded.len()
func FromPhred64(encoded string) (*Scores, error) {
	if len(encoded) == 0 {
		return nil, &EmptyScoresError{}
	}

	scores := make([]int, 0, len(encoded))
	for i, c := range encoded {
		asciiVal := int(c)

		// Phred+64 encoding: valid range is '@' (64) to 'h' (104) for Q0-Q40
		if asciiVal < 64 || asciiVal > 104 {
			return nil, &InvalidEncodingError{Char: c}
		}

		score := asciiVal - 64
		if score > PhredMax {
			return nil, &ScoreOutOfRangeError{Position: i, Score: score}
		}

		scores = append(scores, score)
	}

	return &Scores{Values: scores}, nil
}

// AllInRange checks if all scores are within the valid Phred range.
func (s *Scores) AllInRange() bool {
	for _, score := range s.Values {
		if score < PhredMin || score > PhredMax {
			return false
		}
	}
	return true
}

// Len returns the number of quality scores.
func (s *Scores) Len() int {
	return len(s.Values)
}

// ScoreAt returns the quality score at a specific position.
//
// Aria equivalent:
//
//	fn score_at(self, index: Int) -> Option<Int>
//	  requires index >= 0
func (s *Scores) ScoreAt(index int) (int, bool) {
	if index < 0 || index >= len(s.Values) {
		return 0, false
	}
	return s.Values[index], true
}

// Average calculates the average quality score.
//
// Aria equivalent:
//
//	fn average(self) -> Float
//	  requires self.scores.len() > 0
//	  ensures result >= 0.0 and result <= PHRED_MAX.to_float()
func (s *Scores) Average() float64 {
	sum := 0
	for _, score := range s.Values {
		sum += score
	}
	return float64(sum) / float64(len(s.Values))
}

// Median calculates the median quality score.
//
// Aria equivalent:
//
//	fn median(self) -> Int
//	  requires self.scores.len() > 0
//	  ensures result >= PHRED_MIN and result <= PHRED_MAX
func (s *Scores) Median() int {
	sorted := make([]int, len(s.Values))
	copy(sorted, s.Values)
	sort.Ints(sorted)

	mid := len(sorted) / 2
	if len(sorted)%2 == 0 {
		return (sorted[mid-1] + sorted[mid]) / 2
	}
	return sorted[mid]
}

// Min returns the minimum quality score.
//
// Aria equivalent:
//
//	fn min(self) -> Int
//	  requires self.scores.len() > 0
//	  ensures result >= PHRED_MIN and result <= PHRED_MAX
func (s *Scores) Min() int {
	min := s.Values[0]
	for _, score := range s.Values[1:] {
		if score < min {
			min = score
		}
	}
	return min
}

// Max returns the maximum quality score.
//
// Aria equivalent:
//
//	fn max(self) -> Int
//	  requires self.scores.len() > 0
//	  ensures result >= PHRED_MIN and result <= PHRED_MAX
func (s *Scores) Max() int {
	max := s.Values[0]
	for _, score := range s.Values[1:] {
		if score > max {
			max = score
		}
	}
	return max
}

// CountAbove counts scores above a threshold.
//
// Aria equivalent:
//
//	fn count_above(self, threshold: Int) -> Int
//	  requires threshold >= PHRED_MIN and threshold <= PHRED_MAX
//	  ensures result >= 0 and result <= self.len()
func (s *Scores) CountAbove(threshold int) int {
	count := 0
	for _, score := range s.Values {
		if score > threshold {
			count++
		}
	}
	return count
}

// CountAtOrAbove counts scores at or above a threshold.
//
// Aria equivalent:
//
//	fn count_at_or_above(self, threshold: Int) -> Int
//	  requires threshold >= PHRED_MIN and threshold <= PHRED_MAX
//	  ensures result >= 0 and result <= self.len()
func (s *Scores) CountAtOrAbove(threshold int) int {
	count := 0
	for _, score := range s.Values {
		if score >= threshold {
			count++
		}
	}
	return count
}

// HighQualityRatio calculates the proportion of high-quality bases (Q >= 30).
//
// Aria equivalent:
//
//	fn high_quality_ratio(self) -> Float
//	  ensures result >= 0.0 and result <= 1.0
func (s *Scores) HighQualityRatio() float64 {
	return float64(s.CountAtOrAbove(QHigh)) / float64(len(s.Values))
}

// Categorize categorizes the overall quality of the read.
func (s *Scores) Categorize() Category {
	avg := s.Average()

	if avg >= float64(QExcellent) {
		return Excellent
	} else if avg >= float64(QHigh) {
		return High
	} else if avg >= float64(QMedium) {
		return Medium
	} else if avg >= float64(QLow) {
		return Low
	}
	return Poor
}

// ScoreToProbability converts a Phred score to error probability.
//
// P_error = 10^(-Q/10)
//
// Aria equivalent:
//
//	fn score_to_probability(score: Int) -> Float
//	  requires score >= PHRED_MIN and score <= PHRED_MAX
//	  ensures result >= 0.0 and result <= 1.0
func ScoreToProbability(score int) (float64, error) {
	if score < PhredMin || score > PhredMax {
		return 0, fmt.Errorf("score %d out of range [%d, %d]", score, PhredMin, PhredMax)
	}
	return math.Pow(10.0, float64(-score)/10.0), nil
}

// ProbabilityToScore converts an error probability to Phred score.
//
// Q = -10 * log10(P_error)
//
// Aria equivalent:
//
//	fn probability_to_score(prob: Float) -> Int
//	  requires prob > 0.0 and prob <= 1.0
//	  ensures result >= PHRED_MIN
func ProbabilityToScore(prob float64) (int, error) {
	if prob <= 0.0 || prob > 1.0 {
		return 0, fmt.Errorf("probability %f must be in (0, 1]", prob)
	}

	q := -10.0 * math.Log10(prob)

	// Clamp to valid range
	if q < float64(PhredMin) {
		return PhredMin, nil
	} else if q > float64(PhredMax) {
		return PhredMax, nil
	}
	return int(math.Round(q)), nil
}

// Slice returns a subsequence of quality scores.
//
// Aria equivalent:
//
//	fn slice(self, start: Int, end: Int) -> Result<QualityScores, QualityError>
//	  requires start >= 0
//	  requires end > start
//	  requires end <= self.len()
//	  ensures result.is_ok() implies result.unwrap().len() == end - start
func (s *Scores) Slice(start, end int) (*Scores, error) {
	if start < 0 {
		return nil, fmt.Errorf("start index must be non-negative")
	}
	if end <= start {
		return nil, fmt.Errorf("end must be greater than start")
	}
	if end > len(s.Values) {
		return nil, fmt.Errorf("end must not exceed length")
	}

	slicedValues := make([]int, end-start)
	copy(slicedValues, s.Values[start:end])

	return &Scores{Values: slicedValues}, nil
}

// ToPhred33 encodes quality scores to Phred+33 format.
//
// Aria equivalent:
//
//	fn to_phred33(self) -> String
//	  ensures result.len() == self.len()
func (s *Scores) ToPhred33() string {
	result := make([]byte, len(s.Values))
	for i, score := range s.Values {
		result[i] = byte(score + 33)
	}
	return string(result)
}

// ToPhred64 encodes quality scores to Phred+64 format.
//
// Aria equivalent:
//
//	fn to_phred64(self) -> String
//	  ensures result.len() == self.len()
func (s *Scores) ToPhred64() string {
	result := make([]byte, len(s.Values))
	for i, score := range s.Values {
		result[i] = byte(score + 64)
	}
	return string(result)
}

// LowQualityPositions finds positions of low-quality bases.
//
// Aria equivalent:
//
//	fn low_quality_positions(self, threshold: Int) -> [Int]
//	  requires threshold >= PHRED_MIN and threshold <= PHRED_MAX
//	  ensures result.all(|pos| pos >= 0 and pos < self.len())
func (s *Scores) LowQualityPositions(threshold int) []int {
	positions := make([]int, 0)
	for i, score := range s.Values {
		if score < threshold {
			positions = append(positions, i)
		}
	}
	return positions
}

// Statistics calculates quality statistics.
func (s *Scores) Statistics() *Stats {
	return &Stats{
		Count:            len(s.Values),
		MinScore:         s.Min(),
		MaxScore:         s.Max(),
		Mean:             s.Average(),
		Median:           s.Median(),
		HighQualityRatio: s.HighQualityRatio(),
		Category:         s.Categorize(),
	}
}

func (s *Scores) String() string {
	return fmt.Sprintf("QualityScores { len: %d, avg: %.1f }", len(s.Values), s.Average())
}

// Stats represents quality statistics summary.
type Stats struct {
	Count            int
	MinScore         int
	MaxScore         int
	Mean             float64
	Median           int
	HighQualityRatio float64
	Category         Category
}

func (s *Stats) String() string {
	return fmt.Sprintf("QualityStats { count: %d, min: %d, max: %d, mean: %.2f, median: %d, high_quality_ratio: %.2f%% }",
		s.Count, s.MinScore, s.MaxScore, s.Mean, s.Median, s.HighQualityRatio*100)
}
