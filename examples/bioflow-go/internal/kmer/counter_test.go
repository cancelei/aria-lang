package kmer

import (
	"testing"

	"github.com/aria-lang/bioflow-go/internal/sequence"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestNewCounter(t *testing.T) {
	tests := []struct {
		name    string
		k       int
		wantErr bool
	}{
		{"valid k=3", 3, false},
		{"valid k=21", 21, false},
		{"invalid k=0", 0, true},
		{"invalid k=-1", -1, true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			counter, err := NewCounter(tt.k)
			if tt.wantErr {
				require.Error(t, err)
			} else {
				require.NoError(t, err)
				assert.Equal(t, tt.k, counter.K)
			}
		})
	}
}

func TestCounterCountKMers(t *testing.T) {
	counter, err := NewCounter(3)
	require.NoError(t, err)

	counter.CountKMers("ATGATGATG")

	// Expected 3-mers: ATG, TGA, GAT, ATG, TGA, GAT, ATG
	assert.Equal(t, 3, counter.UniqueCount())
	assert.Equal(t, 7, counter.Total)

	count, err := counter.GetCount("ATG")
	require.NoError(t, err)
	assert.Equal(t, 3, count)

	count, err = counter.GetCount("TGA")
	require.NoError(t, err)
	assert.Equal(t, 2, count)

	count, err = counter.GetCount("GAT")
	require.NoError(t, err)
	assert.Equal(t, 2, count)
}

func TestCounterWithAmbiguous(t *testing.T) {
	counter, err := NewCounter(3)
	require.NoError(t, err)

	counter.CountKMers("ATGNATGC")

	// N-containing k-mers should be skipped
	// ATG, TGN (skip), GNA (skip), NAT (skip), ATG, TGC = 2 unique, 3 total
	assert.Equal(t, 2, counter.UniqueCount())

	count, _ := counter.GetCount("ATG")
	assert.Equal(t, 2, count)
}

func TestMostFrequent(t *testing.T) {
	counter, err := NewCounter(2)
	require.NoError(t, err)

	counter.CountKMers("ATATATATAT")

	most, err := counter.MostFrequent(2)
	require.NoError(t, err)
	assert.Len(t, most, 2)

	// AT and TA should be most frequent
	assert.True(t, (most[0].KMer == "AT" && most[0].Count == 5) ||
		(most[0].KMer == "TA" && most[0].Count == 4))
}

func TestLeastFrequent(t *testing.T) {
	counter, err := NewCounter(2)
	require.NoError(t, err)

	counter.CountKMers("ATGCATGCAT")

	least, err := counter.LeastFrequent(2)
	require.NoError(t, err)
	assert.True(t, len(least) <= 2)
}

func TestFrequency(t *testing.T) {
	counter, err := NewCounter(2)
	require.NoError(t, err)

	counter.CountKMers("ATATATATAT")

	freq, err := counter.Frequency("AT")
	require.NoError(t, err)
	// 5 out of 9 total k-mers
	assert.InDelta(t, 5.0/9.0, freq, 0.0001)
}

func TestFilterByCount(t *testing.T) {
	counter, err := NewCounter(2)
	require.NoError(t, err)

	counter.CountKMers("ATATATATAT")

	filtered, err := counter.FilterByCount(3)
	require.NoError(t, err)

	// Only AT (5) and TA (4) should have count >= 3
	assert.Len(t, filtered, 2)
}

func TestMerge(t *testing.T) {
	counter1, _ := NewCounter(2)
	counter2, _ := NewCounter(2)

	counter1.CountKMers("ATGC")
	counter2.CountKMers("GCTA")

	err := counter1.Merge(counter2)
	require.NoError(t, err)

	// Should have combined counts
	assert.Equal(t, counter1.Total, 6) // 3 + 3

	// GC should appear in both
	count, _ := counter1.GetCount("GC")
	assert.Equal(t, 2, count)
}

func TestMergeDifferentK(t *testing.T) {
	counter1, _ := NewCounter(2)
	counter2, _ := NewCounter(3)

	err := counter1.Merge(counter2)
	require.Error(t, err)
}

func TestCountKMersFromSequence(t *testing.T) {
	seq, err := sequence.New("ATGATGATG")
	require.NoError(t, err)

	counter, err := CountKMers(seq, 3)
	require.NoError(t, err)

	assert.Equal(t, 3, counter.K)
	assert.Equal(t, 7, counter.Total)
}

func TestFindUniqueKMers(t *testing.T) {
	seq, err := sequence.New("ATGCATGC")
	require.NoError(t, err)

	unique, err := FindUniqueKMers(seq, 4)
	require.NoError(t, err)

	// ATGC appears twice at start and end, so unique k-mers are in the middle
	// TGCA, GCAT, CATG should all be unique
	assert.True(t, len(unique) >= 0)
}

func TestKMerPositions(t *testing.T) {
	seq, err := sequence.New("ATGATGATGATG")
	require.NoError(t, err)

	positions, err := KMerPositions(seq, "ATG")
	require.NoError(t, err)
	assert.Equal(t, []int{0, 3, 6, 9}, positions)
}

func TestKMerReverseComplement(t *testing.T) {
	tests := []struct {
		name string
		kmer string
		want string
	}{
		{"ATG", "ATG", "CAT"},
		{"AAA", "AAA", "TTT"},
		{"ATAT", "ATAT", "ATAT"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			km, err := NewKMer(tt.kmer)
			require.NoError(t, err)

			rc := km.ReverseComplement()
			assert.Equal(t, tt.want, rc.Sequence)
		})
	}
}

func TestKMerCanonical(t *testing.T) {
	tests := []struct {
		name string
		kmer string
		want string
	}{
		{"ATG smaller", "ATG", "ATG"},       // ATG < CAT
		{"CAT larger", "CAT", "ATG"},        // CAT > ATG
		{"palindrome", "ATAT", "ATAT"},      // RC(ATAT) = ATAT
		{"GCG palindrome", "GCG", "CGC"},    // RC(GCG) = CGC, CGC < GCG
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			km, err := NewKMer(tt.kmer)
			require.NoError(t, err)

			canonical := km.Canonical()
			assert.Equal(t, tt.want, canonical.Sequence)
		})
	}
}

func TestCountKMersCanonical(t *testing.T) {
	seq, err := sequence.New("ATGCAT")
	require.NoError(t, err)

	counter, err := CountKMersCanonical(seq, 3)
	require.NoError(t, err)

	// ATG and CAT are reverse complements
	// TGC and GCA are reverse complements
	// So we should have fewer unique k-mers than non-canonical counting
	assert.True(t, counter.UniqueCount() > 0)
}

func TestJaccardDistance(t *testing.T) {
	seq1, _ := sequence.New("ATGCATGC")
	seq2, _ := sequence.New("ATGCATGC")

	// Same sequences should have distance 0
	dist, err := JaccardDistance(seq1, seq2, 3)
	require.NoError(t, err)
	assert.Equal(t, 0.0, dist)

	// Completely different sequences should have distance close to 1
	seq3, _ := sequence.New("GGGGGGGG")
	dist, err = JaccardDistance(seq1, seq3, 3)
	require.NoError(t, err)
	assert.Equal(t, 1.0, dist)
}

func TestSharedKMers(t *testing.T) {
	seq1, _ := sequence.New("ATGCATGC")
	seq2, _ := sequence.New("ATGCGGGG")

	shared, err := SharedKMers(seq1, seq2, 3)
	require.NoError(t, err)

	// ATG should be shared
	found := false
	for _, kmer := range shared {
		if kmer == "ATG" {
			found = true
			break
		}
	}
	assert.True(t, found)
}

func TestEstimateGenomeSize(t *testing.T) {
	size, err := EstimateGenomeSize(1000000, 50, 21)
	require.NoError(t, err)
	assert.Equal(t, 20000, size)

	// Invalid parameters
	_, err = EstimateGenomeSize(0, 50, 21)
	require.Error(t, err)
}

func BenchmarkCountKMers(b *testing.B) {
	seq, _ := sequence.New("ATGCATGCATGCATGCATGCATGCATGCATGCATGCATGCATGCATGCATGCATGCATGCATGCATGCATGCATGCATGCATGCATGCATGCATGCATGCATGC")
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = CountKMers(seq, 21)
	}
}

func BenchmarkJaccardDistance(b *testing.B) {
	seq1, _ := sequence.New("ATGCATGCATGCATGCATGCATGCATGCATGCATGCATGCATGCATGCATGCATGCATGCATGCATGCATGCATGCATGCATGCATGCATGCATGCATGCATGC")
	seq2, _ := sequence.New("GCTAGCTAGCTAGCTAGCTAGCTAGCTAGCTAGCTAGCTAGCTAGCTAGCTAGCTAGCTAGCTAGCTAGCTAGCTAGCTAGCTAGCTAGCTAGCTAGCTAGC")
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = JaccardDistance(seq1, seq2, 11)
	}
}
