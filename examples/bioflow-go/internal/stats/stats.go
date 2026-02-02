// Package stats provides statistical summaries for sequences and reads.
//
// This module provides aggregate statistics for collections of sequences
// and reads, similar to the Aria implementation.
package stats

import (
	"fmt"
	"sort"

	"github.com/aria-lang/bioflow-go/internal/quality"
	"github.com/aria-lang/bioflow-go/internal/sequence"
)

// SequenceStats represents statistics for a single sequence.
//
// Aria equivalent:
//
//	struct SequenceStats
//	  invariant self.a_count + self.c_count + self.g_count +
//	            self.t_count + self.n_count == self.length
//	  invariant self.gc_content >= 0.0 and self.gc_content <= 1.0
//	  invariant self.at_content >= 0.0 and self.at_content <= 1.0
type SequenceStats struct {
	Length       int
	GCContent    float64
	ATContent    float64
	ACount       int
	CCount       int
	GCount       int
	TCount       int
	NCount       int
	HasAmbiguous bool
}

// FromSequence calculates statistics for a sequence.
//
// Aria equivalent:
//
//	fn from_sequence(seq: Sequence) -> SequenceStats
//	  requires seq.is_valid()
//	  ensures result.length == seq.len()
func FromSequence(seq *sequence.Sequence) *SequenceStats {
	counts := seq.BaseCounts()

	atContent := 0.0
	if seq.Len() > 0 {
		atContent = float64(counts.A+counts.T) / float64(seq.Len())
	}

	return &SequenceStats{
		Length:       seq.Len(),
		GCContent:    seq.GCContent(),
		ATContent:    atContent,
		ACount:       counts.A,
		CCount:       counts.C,
		GCount:       counts.G,
		TCount:       counts.T,
		NCount:       counts.N,
		HasAmbiguous: counts.N > 0,
	}
}

func (s *SequenceStats) String() string {
	return fmt.Sprintf(`SequenceStats {
  length: %d
  GC content: %.1f%%
  AT content: %.1f%%
  A: %d, C: %d, G: %d, T: %d, N: %d
}`, s.Length, s.GCContent*100, s.ATContent*100,
		s.ACount, s.CCount, s.GCount, s.TCount, s.NCount)
}

// SequenceSetStats represents aggregated statistics for multiple sequences.
//
// Aria equivalent:
//
//	struct SequenceSetStats
//	  count: Int
//	  total_bases: Int
//	  min_length: Int
//	  max_length: Int
//	  mean_length: Float
//	  median_length: Int
//	  mean_gc_content: Float
//	  n50: Int
//	  total_ambiguous: Int
type SequenceSetStats struct {
	Count          int
	TotalBases     int
	MinLength      int
	MaxLength      int
	MeanLength     float64
	MedianLength   int
	MeanGCContent  float64
	N50            int
	TotalAmbiguous int
}

// FromSequences calculates statistics for a collection of sequences.
//
// Aria equivalent:
//
//	fn from_sequences(sequences: [Sequence]) -> SequenceSetStats
//	  requires sequences.len() > 0
//	  ensures result.count == sequences.len()
func FromSequences(sequences []*sequence.Sequence) (*SequenceSetStats, error) {
	if len(sequences) == 0 {
		return nil, fmt.Errorf("sequence list cannot be empty")
	}

	count := len(sequences)
	lengths := make([]int, count)
	totalBases := 0

	for i, seq := range sequences {
		lengths[i] = seq.Len()
		totalBases += seq.Len()
	}

	minLen := lengths[0]
	maxLen := lengths[0]
	for _, l := range lengths {
		if l < minLen {
			minLen = l
		}
		if l > maxLen {
			maxLen = l
		}
	}

	meanLen := float64(totalBases) / float64(count)

	// Calculate median
	sortedLengths := make([]int, count)
	copy(sortedLengths, lengths)
	sort.Ints(sortedLengths)

	mid := count / 2
	var medianLen int
	if count%2 == 0 {
		medianLen = (sortedLengths[mid-1] + sortedLengths[mid]) / 2
	} else {
		medianLen = sortedLengths[mid]
	}

	// Calculate mean GC content
	gcSum := 0.0
	for _, seq := range sequences {
		gcSum += seq.GCContent()
	}
	meanGC := gcSum / float64(count)

	// Calculate N50 (length where 50% of bases are in longer sequences)
	sortedDesc := make([]int, count)
	copy(sortedDesc, lengths)
	sort.Sort(sort.Reverse(sort.IntSlice(sortedDesc)))

	halfTotal := totalBases / 2
	runningSum := 0
	n50 := sortedDesc[0]

	for _, length := range sortedDesc {
		runningSum += length
		if runningSum >= halfTotal {
			n50 = length
			break
		}
	}

	// Count total ambiguous bases
	totalAmbiguous := 0
	for _, seq := range sequences {
		totalAmbiguous += seq.CountAmbiguous()
	}

	return &SequenceSetStats{
		Count:          count,
		TotalBases:     totalBases,
		MinLength:      minLen,
		MaxLength:      maxLen,
		MeanLength:     meanLen,
		MedianLength:   medianLen,
		MeanGCContent:  meanGC,
		N50:            n50,
		TotalAmbiguous: totalAmbiguous,
	}, nil
}

func (s *SequenceSetStats) String() string {
	return fmt.Sprintf(`SequenceSetStats {
  count: %d
  total_bases: %d
  length range: %d - %d
  mean length: %.1f
  median length: %d
  mean GC: %.1f%%
  N50: %d
  ambiguous bases: %d
}`, s.Count, s.TotalBases, s.MinLength, s.MaxLength,
		s.MeanLength, s.MedianLength, s.MeanGCContent*100, s.N50, s.TotalAmbiguous)
}

// QualityDistribution represents quality score distribution.
type QualityDistribution struct {
	PoorCount      int
	LowCount       int
	MediumCount    int
	HighCount      int
	ExcellentCount int
	Total          int
}

// FromCategories creates distribution from list of categories.
func FromCategories(categories []quality.Category) *QualityDistribution {
	dist := &QualityDistribution{Total: len(categories)}

	for _, cat := range categories {
		switch cat {
		case quality.Poor:
			dist.PoorCount++
		case quality.Low:
			dist.LowCount++
		case quality.Medium:
			dist.MediumCount++
		case quality.High:
			dist.HighCount++
		case quality.Excellent:
			dist.ExcellentCount++
		}
	}

	return dist
}

// AcceptableRatio returns proportion of reads at or above medium quality.
func (d *QualityDistribution) AcceptableRatio() float64 {
	acceptable := d.MediumCount + d.HighCount + d.ExcellentCount
	if d.Total == 0 {
		return 0.0
	}
	return float64(acceptable) / float64(d.Total)
}

func (d *QualityDistribution) String() string {
	return fmt.Sprintf(`QualityDistribution {
  Poor (Q<10): %d
  Low (Q10-20): %d
  Medium (Q20-30): %d
  High (Q30-40): %d
  Excellent (Q40): %d
}`, d.PoorCount, d.LowCount, d.MediumCount, d.HighCount, d.ExcellentCount)
}

// ReadSetStats represents statistics for a collection of reads.
type ReadSetStats struct {
	Count               int
	TotalBases          int
	MinLength           int
	MaxLength           int
	MeanLength          float64
	MeanQuality         float64
	MedianQuality       float64
	HighQualityCount    int
	QualityDistribution *QualityDistribution
}

// FromReads calculates statistics for a collection of reads.
func FromReads(sequences []*sequence.Sequence, qualities []*quality.Scores) (*ReadSetStats, error) {
	if len(sequences) != len(qualities) {
		return nil, fmt.Errorf("sequences and qualities must have same length")
	}
	if len(sequences) == 0 {
		return nil, fmt.Errorf("read list cannot be empty")
	}

	count := len(sequences)
	lengths := make([]int, count)
	totalBases := 0

	for i, seq := range sequences {
		lengths[i] = seq.Len()
		totalBases += seq.Len()
	}

	minLen := lengths[0]
	maxLen := lengths[0]
	for _, l := range lengths {
		if l < minLen {
			minLen = l
		}
		if l > maxLen {
			maxLen = l
		}
	}

	meanLen := float64(totalBases) / float64(count)

	// Quality statistics
	avgQualities := make([]float64, count)
	for i, q := range qualities {
		avgQualities[i] = q.Average()
	}

	qualitySum := 0.0
	for _, avg := range avgQualities {
		qualitySum += avg
	}
	meanQuality := qualitySum / float64(count)

	// Median quality
	sortedQualities := make([]float64, count)
	copy(sortedQualities, avgQualities)
	sort.Float64s(sortedQualities)

	mid := count / 2
	var medianQuality float64
	if count%2 == 0 {
		medianQuality = (sortedQualities[mid-1] + sortedQualities[mid]) / 2
	} else {
		medianQuality = sortedQualities[mid]
	}

	// Count high quality reads (avg >= Q30)
	highQualityCount := 0
	for _, avg := range avgQualities {
		if avg >= 30.0 {
			highQualityCount++
		}
	}

	// Build quality distribution
	categories := make([]quality.Category, count)
	for i, q := range qualities {
		categories[i] = q.Categorize()
	}
	distribution := FromCategories(categories)

	return &ReadSetStats{
		Count:               count,
		TotalBases:          totalBases,
		MinLength:           minLen,
		MaxLength:           maxLen,
		MeanLength:          meanLen,
		MeanQuality:         meanQuality,
		MedianQuality:       medianQuality,
		HighQualityCount:    highQualityCount,
		QualityDistribution: distribution,
	}, nil
}

// HighQualityRatio returns proportion of high-quality reads.
func (s *ReadSetStats) HighQualityRatio() float64 {
	if s.Count == 0 {
		return 0.0
	}
	return float64(s.HighQualityCount) / float64(s.Count)
}

func (s *ReadSetStats) String() string {
	return fmt.Sprintf(`ReadSetStats {
  count: %d
  total_bases: %d
  length range: %d - %d
  mean length: %.1f
  mean quality: %.1f
  median quality: %.1f
  high quality reads: %d (%.1f%%)
}`, s.Count, s.TotalBases, s.MinLength, s.MaxLength,
		s.MeanLength, s.MeanQuality, s.MedianQuality,
		s.HighQualityCount, s.HighQualityRatio()*100)
}

// GCHistogram represents a GC content histogram with bins.
type GCHistogram struct {
	Bins    []int
	BinSize float64
	NumBins int
}

// NewGCHistogram creates a GC content histogram from sequences.
func NewGCHistogram(sequences []*sequence.Sequence, numBins int) (*GCHistogram, error) {
	if len(sequences) == 0 {
		return nil, fmt.Errorf("sequence list cannot be empty")
	}

	binSize := 1.0 / float64(numBins)
	bins := make([]int, numBins)

	for _, seq := range sequences {
		gc := seq.GCContent()
		binIndex := int(gc / binSize)
		if binIndex >= numBins {
			binIndex = numBins - 1
		}
		bins[binIndex]++
	}

	return &GCHistogram{
		Bins:    bins,
		BinSize: binSize,
		NumBins: numBins,
	}, nil
}

// ModeBin returns the most common GC content range.
func (h *GCHistogram) ModeBin() (float64, float64) {
	maxCount := h.Bins[0]
	maxBin := 0

	for i, count := range h.Bins {
		if count > maxCount {
			maxCount = count
			maxBin = i
		}
	}

	start := float64(maxBin) * h.BinSize
	end := start + h.BinSize
	return start, end
}

func (h *GCHistogram) String() string {
	result := "GC Content Histogram:\n"
	for i := 0; i < h.NumBins; i++ {
		start := int(float64(i) * h.BinSize * 100)
		end := start + int(h.BinSize*100)
		count := h.Bins[i]

		bar := ""
		for j := 0; j < count/10; j++ {
			bar += "#"
		}

		result += fmt.Sprintf("%2d-%2d%%: %s (%d)\n", start, end, bar, count)
	}
	return result
}

// LengthHistogram represents a length histogram for sequences.
type LengthHistogram struct {
	Bins      []int
	MinLength int
	MaxLength int
	BinWidth  int
	NumBins   int
}

// NewLengthHistogram creates a length histogram from sequences.
func NewLengthHistogram(sequences []*sequence.Sequence, numBins int) (*LengthHistogram, error) {
	if len(sequences) == 0 {
		return nil, fmt.Errorf("sequence list cannot be empty")
	}
	if numBins <= 0 {
		return nil, fmt.Errorf("numBins must be positive")
	}

	lengths := make([]int, len(sequences))
	for i, seq := range sequences {
		lengths[i] = seq.Len()
	}

	minLen := lengths[0]
	maxLen := lengths[0]
	for _, l := range lengths {
		if l < minLen {
			minLen = l
		}
		if l > maxLen {
			maxLen = l
		}
	}

	lengthRange := maxLen - minLen
	binWidth := lengthRange / numBins
	if binWidth < 1 {
		binWidth = 1
	}

	bins := make([]int, numBins)

	for _, length := range lengths {
		binIndex := (length - minLen) / binWidth
		if binIndex >= numBins {
			binIndex = numBins - 1
		}
		bins[binIndex]++
	}

	return &LengthHistogram{
		Bins:      bins,
		MinLength: minLen,
		MaxLength: maxLen,
		BinWidth:  binWidth,
		NumBins:   numBins,
	}, nil
}

func (h *LengthHistogram) String() string {
	result := "Length Histogram:\n"
	for i := 0; i < h.NumBins; i++ {
		start := h.MinLength + i*h.BinWidth
		end := start + h.BinWidth
		count := h.Bins[i]

		bar := ""
		for j := 0; j < count/5; j++ {
			bar += "#"
		}

		result += fmt.Sprintf("%5d-%5d: %s (%d)\n", start, end, bar, count)
	}
	return result
}
