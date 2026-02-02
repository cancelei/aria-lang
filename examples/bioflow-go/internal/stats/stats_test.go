package stats

import (
	"testing"

	"github.com/aria-lang/bioflow-go/internal/quality"
	"github.com/aria-lang/bioflow-go/internal/sequence"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestFromSequence(t *testing.T) {
	seq, err := sequence.New("AATTTGGGCCCCN")
	require.NoError(t, err)

	stats := FromSequence(seq)

	assert.Equal(t, 13, stats.Length)
	assert.Equal(t, 2, stats.ACount)
	assert.Equal(t, 4, stats.CCount)
	assert.Equal(t, 3, stats.GCount)
	assert.Equal(t, 3, stats.TCount)
	assert.Equal(t, 1, stats.NCount)
	assert.True(t, stats.HasAmbiguous)

	// GC = 7/13
	assert.InDelta(t, 7.0/13.0, stats.GCContent, 0.0001)

	// AT = 5/13
	assert.InDelta(t, 5.0/13.0, stats.ATContent, 0.0001)
}

func TestFromSequences(t *testing.T) {
	sequences := make([]*sequence.Sequence, 0)

	s1, _ := sequence.New("ATGC")     // len=4, GC=0.5
	s2, _ := sequence.New("ATGCATGC") // len=8, GC=0.5
	s3, _ := sequence.New("GGCC")     // len=4, GC=1.0

	sequences = append(sequences, s1, s2, s3)

	stats, err := FromSequences(sequences)
	require.NoError(t, err)

	assert.Equal(t, 3, stats.Count)
	assert.Equal(t, 16, stats.TotalBases)
	assert.Equal(t, 4, stats.MinLength)
	assert.Equal(t, 8, stats.MaxLength)
	assert.InDelta(t, 16.0/3.0, stats.MeanLength, 0.0001)
	assert.Equal(t, 4, stats.MedianLength) // sorted: 4, 4, 8; middle = 4
}

func TestFromSequencesEmpty(t *testing.T) {
	_, err := FromSequences([]*sequence.Sequence{})
	require.Error(t, err)
}

func TestN50Calculation(t *testing.T) {
	sequences := make([]*sequence.Sequence, 0)

	// Create sequences with lengths: 100, 80, 60, 40, 20
	// Total = 300, Half = 150
	// N50 should be 80 (100 + 80 >= 150)
	s1, _ := sequence.New(generateSeq(100))
	s2, _ := sequence.New(generateSeq(80))
	s3, _ := sequence.New(generateSeq(60))
	s4, _ := sequence.New(generateSeq(40))
	s5, _ := sequence.New(generateSeq(20))

	sequences = append(sequences, s1, s2, s3, s4, s5)

	stats, err := FromSequences(sequences)
	require.NoError(t, err)

	assert.Equal(t, 80, stats.N50)
}

func generateSeq(length int) string {
	bases := []byte{'A', 'T', 'G', 'C'}
	result := make([]byte, length)
	for i := 0; i < length; i++ {
		result[i] = bases[i%4]
	}
	return string(result)
}

func TestFromCategories(t *testing.T) {
	categories := []quality.Category{
		quality.Poor,
		quality.Low,
		quality.Low,
		quality.Medium,
		quality.Medium,
		quality.Medium,
		quality.High,
		quality.High,
		quality.Excellent,
	}

	dist := FromCategories(categories)

	assert.Equal(t, 1, dist.PoorCount)
	assert.Equal(t, 2, dist.LowCount)
	assert.Equal(t, 3, dist.MediumCount)
	assert.Equal(t, 2, dist.HighCount)
	assert.Equal(t, 1, dist.ExcellentCount)
	assert.Equal(t, 9, dist.Total)

	// Acceptable = Medium + High + Excellent = 6/9
	assert.InDelta(t, 6.0/9.0, dist.AcceptableRatio(), 0.0001)
}

func TestFromReads(t *testing.T) {
	sequences := make([]*sequence.Sequence, 0)
	qualities := make([]*quality.Scores, 0)

	s1, _ := sequence.New("ATGC")
	s2, _ := sequence.New("ATGCATGC")
	sequences = append(sequences, s1, s2)

	q1, _ := quality.New([]int{30, 30, 30, 30})
	q2, _ := quality.New([]int{35, 35, 35, 35, 35, 35, 35, 35})
	qualities = append(qualities, q1, q2)

	stats, err := FromReads(sequences, qualities)
	require.NoError(t, err)

	assert.Equal(t, 2, stats.Count)
	assert.Equal(t, 12, stats.TotalBases)
	assert.InDelta(t, 32.5, stats.MeanQuality, 0.1)
	assert.Equal(t, 2, stats.HighQualityCount)
}

func TestFromReadsMismatchedLength(t *testing.T) {
	sequences := make([]*sequence.Sequence, 0)
	qualities := make([]*quality.Scores, 0)

	s1, _ := sequence.New("ATGC")
	sequences = append(sequences, s1)

	q1, _ := quality.New([]int{30, 30, 30, 30})
	q2, _ := quality.New([]int{35, 35, 35, 35})
	qualities = append(qualities, q1, q2)

	_, err := FromReads(sequences, qualities)
	require.Error(t, err)
}

func TestGCHistogram(t *testing.T) {
	sequences := make([]*sequence.Sequence, 0)

	// Create sequences with varying GC content
	s1, _ := sequence.New("AAAA")     // GC = 0%
	s2, _ := sequence.New("ATGC")     // GC = 50%
	s3, _ := sequence.New("GGCC")     // GC = 100%
	s4, _ := sequence.New("ATATATGC") // GC = 25%

	sequences = append(sequences, s1, s2, s3, s4)

	hist, err := NewGCHistogram(sequences, 10)
	require.NoError(t, err)

	assert.Equal(t, 10, hist.NumBins)
	assert.InDelta(t, 0.1, hist.BinSize, 0.0001)
}

func TestLengthHistogram(t *testing.T) {
	sequences := make([]*sequence.Sequence, 0)

	s1, _ := sequence.New("ATGC")             // len=4
	s2, _ := sequence.New("ATGCATGC")         // len=8
	s3, _ := sequence.New("ATGCATGCATGCATGC") // len=16

	sequences = append(sequences, s1, s2, s3)

	hist, err := NewLengthHistogram(sequences, 5)
	require.NoError(t, err)

	assert.Equal(t, 5, hist.NumBins)
	assert.Equal(t, 4, hist.MinLength)
	assert.Equal(t, 16, hist.MaxLength)
}

func TestEmptyHistograms(t *testing.T) {
	_, err := NewGCHistogram([]*sequence.Sequence{}, 10)
	require.Error(t, err)

	_, err = NewLengthHistogram([]*sequence.Sequence{}, 10)
	require.Error(t, err)
}

func BenchmarkFromSequences(b *testing.B) {
	sequences := make([]*sequence.Sequence, 100)
	for i := 0; i < 100; i++ {
		sequences[i], _ = sequence.New(generateSeq(1000))
	}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = FromSequences(sequences)
	}
}
