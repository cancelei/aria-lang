package alignment

import (
	"testing"

	"github.com/aria-lang/bioflow-go/internal/sequence"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestScoringMatrix(t *testing.T) {
	t.Run("DefaultDNA", func(t *testing.T) {
		s := DefaultDNA()
		assert.Equal(t, 2, s.MatchScore)
		assert.Equal(t, -1, s.MismatchPenalty)
		assert.Equal(t, -2, s.GapOpenPenalty)
	})

	t.Run("BLASTLike", func(t *testing.T) {
		s := BLASTLike()
		assert.Equal(t, 1, s.MatchScore)
		assert.Equal(t, -3, s.MismatchPenalty)
	})

	t.Run("Score match", func(t *testing.T) {
		s := DefaultDNA()
		assert.Equal(t, 2, s.Score('A', 'A'))
	})

	t.Run("Score mismatch", func(t *testing.T) {
		s := DefaultDNA()
		assert.Equal(t, -1, s.Score('A', 'T'))
	})

	t.Run("Invalid scoring matrix", func(t *testing.T) {
		_, err := NewScoringMatrix(0, -1, -2, -1)
		require.Error(t, err)

		_, err = NewScoringMatrix(2, 1, -2, -1)
		require.Error(t, err)
	})
}

func TestSmithWaterman(t *testing.T) {
	tests := []struct {
		name     string
		seq1     string
		seq2     string
		minScore int
	}{
		{
			name:     "identical short",
			seq1:     "ATGC",
			seq2:     "ATGC",
			minScore: 8, // 4 matches * 2
		},
		{
			name:     "one mismatch",
			seq1:     "ATGC",
			seq2:     "ATGA",
			minScore: 4, // 3 matches - 1 mismatch
		},
		{
			name:     "with gap",
			seq1:     "ATGCATGC",
			seq2:     "ATGATGC",
			minScore: 8, // Should find good local alignment
		},
		{
			name:     "no match",
			seq1:     "AAAA",
			seq2:     "TTTT",
			minScore: 0,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			seq1, err := sequence.New(tt.seq1)
			require.NoError(t, err)

			seq2, err := sequence.New(tt.seq2)
			require.NoError(t, err)

			alignment, err := SmithWaterman(seq1, seq2, nil)
			require.NoError(t, err)

			assert.GreaterOrEqual(t, alignment.Score, tt.minScore)
			assert.Equal(t, len(alignment.AlignedSeq1), len(alignment.AlignedSeq2))
		})
	}
}

func TestSmithWatermanIdentical(t *testing.T) {
	seq1, _ := sequence.New("ACGT")
	seq2, _ := sequence.New("ACGT")

	alignment, err := SmithWaterman(seq1, seq2, nil)
	require.NoError(t, err)

	assert.Equal(t, 1.0, alignment.Identity)
	assert.Equal(t, 4, alignment.MatchCount())
	assert.Equal(t, 0, alignment.MismatchCount())
	assert.Equal(t, 0, alignment.TotalGaps())
}

func TestNeedlemanWunsch(t *testing.T) {
	tests := []struct {
		name     string
		seq1     string
		seq2     string
		hasScore bool
	}{
		{
			name:     "identical",
			seq1:     "ATGC",
			seq2:     "ATGC",
			hasScore: true,
		},
		{
			name:     "different length",
			seq1:     "ATGCATGC",
			seq2:     "ATGC",
			hasScore: true,
		},
		{
			name:     "completely different",
			seq1:     "AAAA",
			seq2:     "TTTT",
			hasScore: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			seq1, err := sequence.New(tt.seq1)
			require.NoError(t, err)

			seq2, err := sequence.New(tt.seq2)
			require.NoError(t, err)

			alignment, err := NeedlemanWunsch(seq1, seq2, nil)
			require.NoError(t, err)

			// Global alignment should always produce same-length alignments
			assert.Equal(t, len(alignment.AlignedSeq1), len(alignment.AlignedSeq2))

			// Alignment length should be at least the max of input lengths
			maxLen := max(seq1.Len(), seq2.Len())
			assert.GreaterOrEqual(t, alignment.Length(), maxLen)
		})
	}
}

func TestAlignmentIdentity(t *testing.T) {
	tests := []struct {
		name     string
		aligned1 string
		aligned2 string
		want     float64
	}{
		{"perfect match", "ATGC", "ATGC", 1.0},
		{"50% match", "ATGC", "ATTT", 0.5},
		{"no match", "AAAA", "TTTT", 0.0},
		{"with gaps", "AT-GC", "ATGGC", 0.8},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			a, err := NewAlignment(tt.aligned1, tt.aligned2, 0, Local)
			require.NoError(t, err)
			assert.InDelta(t, tt.want, a.Identity, 0.0001)
		})
	}
}

func TestAlignmentCIGAR(t *testing.T) {
	tests := []struct {
		name     string
		aligned1 string
		aligned2 string
		want     string
	}{
		{"all match", "ATGC", "ATGC", "4M"},
		{"with mismatch", "ATGC", "ATGA", "3M1X"},
		{"with gap seq1", "AT-GC", "ATGGC", "2M1I2M"},
		{"with gap seq2", "ATGGC", "AT-GC", "2M1D2M"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			a, err := NewAlignment(tt.aligned1, tt.aligned2, 0, Local)
			require.NoError(t, err)
			assert.Equal(t, tt.want, a.ToCIGAR())
		})
	}
}

func TestGapOpenings(t *testing.T) {
	tests := []struct {
		name     string
		aligned1 string
		aligned2 string
		want     int
	}{
		{"no gaps", "ATGC", "ATGC", 0},
		{"one gap", "AT-GC", "ATGGC", 1},
		{"two gaps same seq", "AT--GC", "ATGGGC", 1},
		{"two gaps diff seq", "AT-GC-", "ATGG-C", 3}, // 2 gaps in seq1 (at pos 2 and 5), 1 gap in seq2 (at pos 4)
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			a, err := NewAlignment(tt.aligned1, tt.aligned2, 0, Local)
			require.NoError(t, err)
			assert.Equal(t, tt.want, a.GapOpenings())
		})
	}
}

func TestAlignmentScoreOnly(t *testing.T) {
	seq1, _ := sequence.New("ATGCATGCATGC")
	seq2, _ := sequence.New("ATGCATGCATGC")

	score, err := AlignmentScoreOnly(seq1, seq2, nil)
	require.NoError(t, err)

	// Should match full Smith-Waterman score
	alignment, _ := SmithWaterman(seq1, seq2, nil)
	assert.Equal(t, alignment.Score, score)
}

func TestPercentIdentity(t *testing.T) {
	tests := []struct {
		name     string
		aligned1 string
		aligned2 string
		want     float64
		wantErr  bool
	}{
		{"perfect", "ATGC", "ATGC", 100.0, false},
		{"50%", "ATGC", "ATTT", 50.0, false},
		{"different lengths", "ATGC", "ATG", 0, true},
		{"empty", "", "", 0, true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := PercentIdentity(tt.aligned1, tt.aligned2)
			if tt.wantErr {
				require.Error(t, err)
			} else {
				require.NoError(t, err)
				assert.InDelta(t, tt.want, got, 0.0001)
			}
		})
	}
}

func TestAlignAgainstMultiple(t *testing.T) {
	query, _ := sequence.New("ATGCATGC")
	targets := []*sequence.Sequence{}

	t1, _ := sequence.New("ATGCATGC")
	t2, _ := sequence.New("GCTAGCTA")
	t3, _ := sequence.New("ATGCGGGG")
	targets = append(targets, t1, t2, t3)

	alignments, err := AlignAgainstMultiple(query, targets, nil)
	require.NoError(t, err)
	assert.Len(t, alignments, 3)

	// First target should have best match
	assert.Greater(t, alignments[0].Alignment.Score, alignments[1].Alignment.Score)
}

func TestFindBestAlignment(t *testing.T) {
	query, _ := sequence.New("ATGCATGC")
	targets := []*sequence.Sequence{}

	t1, _ := sequence.New("GCTAGCTA")
	t2, _ := sequence.New("ATGCATGC")
	t3, _ := sequence.New("AAAAAAAA")
	targets = append(targets, t1, t2, t3)

	best, err := FindBestAlignment(query, targets, nil)
	require.NoError(t, err)
	assert.NotNil(t, best)

	// Best should be the identical sequence (index 1)
	assert.Equal(t, 1, best.Index)
}

func TestSimpleAlign(t *testing.T) {
	seq1, _ := sequence.New("ATGC")
	seq2, _ := sequence.New("ATGC")

	alignment, err := SimpleAlign(seq1, seq2)
	require.NoError(t, err)
	assert.Equal(t, 1.0, alignment.Identity)
}

func TestGlobalAlignmentScoreOnly(t *testing.T) {
	seq1, _ := sequence.New("ATGCATGC")
	seq2, _ := sequence.New("ATGCATGC")

	score, err := GlobalAlignmentScoreOnly(seq1, seq2, nil)
	require.NoError(t, err)

	alignment, _ := NeedlemanWunsch(seq1, seq2, nil)
	assert.Equal(t, alignment.Score, score)
}

func BenchmarkSmithWaterman(b *testing.B) {
	s1 := ""
	s2 := ""
	for i := 0; i < 250; i++ {
		s1 += "ACGT"
		s2 += "AGCT"
	}
	seq1, _ := sequence.New(s1)
	seq2, _ := sequence.New(s2)

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = SmithWaterman(seq1, seq2, DefaultDNA())
	}
}

func BenchmarkNeedlemanWunsch(b *testing.B) {
	s1 := ""
	s2 := ""
	for i := 0; i < 250; i++ {
		s1 += "ACGT"
		s2 += "AGCT"
	}
	seq1, _ := sequence.New(s1)
	seq2, _ := sequence.New(s2)

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = NeedlemanWunsch(seq1, seq2, DefaultDNA())
	}
}

func BenchmarkAlignmentScoreOnly(b *testing.B) {
	s1 := ""
	s2 := ""
	for i := 0; i < 250; i++ {
		s1 += "ACGT"
		s2 += "AGCT"
	}
	seq1, _ := sequence.New(s1)
	seq2, _ := sequence.New(s2)

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = AlignmentScoreOnly(seq1, seq2, DefaultDNA())
	}
}
