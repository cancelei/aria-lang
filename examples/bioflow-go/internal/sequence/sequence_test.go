package sequence

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestNew(t *testing.T) {
	tests := []struct {
		name    string
		bases   string
		wantErr bool
		errType interface{}
	}{
		{
			name:    "valid DNA sequence",
			bases:   "ATGCATGC",
			wantErr: false,
		},
		{
			name:    "valid DNA with lowercase",
			bases:   "atgcatgc",
			wantErr: false,
		},
		{
			name:    "valid DNA with ambiguous base",
			bases:   "ATGCNATGC",
			wantErr: false,
		},
		{
			name:    "empty sequence",
			bases:   "",
			wantErr: true,
			errType: &EmptySequenceError{},
		},
		{
			name:    "invalid base X",
			bases:   "ATGCXATGC",
			wantErr: true,
			errType: &InvalidBaseError{},
		},
		{
			name:    "invalid base Z",
			bases:   "ATGCZ",
			wantErr: true,
			errType: &InvalidBaseError{},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			seq, err := New(tt.bases)

			if tt.wantErr {
				require.Error(t, err)
				if tt.errType != nil {
					assert.IsType(t, tt.errType, err)
				}
			} else {
				require.NoError(t, err)
				assert.NotNil(t, seq)
				assert.Equal(t, DNA, seq.SeqType)
			}
		})
	}
}

func TestGCContent(t *testing.T) {
	tests := []struct {
		name     string
		sequence string
		want     float64
	}{
		{"all GC", "GCGCGC", 1.0},
		{"all AT", "ATATAT", 0.0},
		{"mixed 50%", "ATGC", 0.5},
		{"mixed 75%", "GCGC", 1.0},
		{"single G", "G", 1.0},
		{"single A", "A", 0.0},
		{"with N", "ATGCN", 0.4},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			seq, err := New(tt.sequence)
			require.NoError(t, err)

			got := seq.GCContent()
			assert.InDelta(t, tt.want, got, 0.0001)
		})
	}
}

func TestATContent(t *testing.T) {
	tests := []struct {
		name     string
		sequence string
		want     float64
	}{
		{"all AT", "ATATAT", 1.0},
		{"all GC", "GCGCGC", 0.0},
		{"mixed 50%", "ATGC", 0.5},
		{"single A", "A", 1.0},
		{"single G", "G", 0.0},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			seq, err := New(tt.sequence)
			require.NoError(t, err)

			got, err := seq.ATContent()
			require.NoError(t, err)
			assert.InDelta(t, tt.want, got, 0.0001)
		})
	}
}

func TestComplement(t *testing.T) {
	tests := []struct {
		name     string
		sequence string
		want     string
	}{
		{"ATGC", "ATGC", "TACG"},
		{"AAAA", "AAAA", "TTTT"},
		{"TTTT", "TTTT", "AAAA"},
		{"GCGC", "GCGC", "CGCG"},
		{"with N", "ATNCG", "TANGC"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			seq, err := New(tt.sequence)
			require.NoError(t, err)

			comp, err := seq.Complement()
			require.NoError(t, err)
			assert.Equal(t, tt.want, comp.Bases)
		})
	}
}

func TestReverse(t *testing.T) {
	tests := []struct {
		name     string
		sequence string
		want     string
	}{
		{"ATGC", "ATGC", "CGTA"},
		{"single", "A", "A"},
		{"palindrome", "GAATTC", "CTTAAG"},
		{"longer", "ABCDEFG", "GFEDCBA"}, // Note: Invalid DNA but tests reversal
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			seq := &Sequence{Bases: tt.sequence, SeqType: DNA}
			rev := seq.Reverse()
			assert.Equal(t, tt.want, rev.Bases)
		})
	}
}

func TestReverseComplement(t *testing.T) {
	tests := []struct {
		name     string
		sequence string
		want     string
	}{
		{"ATGC", "ATGC", "GCAT"},
		{"palindrome", "GAATTC", "GAATTC"},
		{"simple", "AAGT", "ACTT"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			seq, err := New(tt.sequence)
			require.NoError(t, err)

			rc, err := seq.ReverseComplement()
			require.NoError(t, err)
			assert.Equal(t, tt.want, rc.Bases)
		})
	}
}

func TestBaseCounts(t *testing.T) {
	seq, err := New("AATTTGGGCCCCN")
	require.NoError(t, err)

	counts := seq.BaseCounts()
	assert.Equal(t, 2, counts.A)
	assert.Equal(t, 4, counts.C)
	assert.Equal(t, 3, counts.G)
	assert.Equal(t, 3, counts.T)
	assert.Equal(t, 1, counts.N)
	assert.Equal(t, 13, counts.Total())
}

func TestHasAmbiguous(t *testing.T) {
	tests := []struct {
		name     string
		sequence string
		want     bool
	}{
		{"no N", "ATGC", false},
		{"single N", "ATNGC", true},
		{"multiple N", "ATNNNGC", true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			seq, err := New(tt.sequence)
			require.NoError(t, err)
			assert.Equal(t, tt.want, seq.HasAmbiguous())
		})
	}
}

func TestCountAmbiguous(t *testing.T) {
	tests := []struct {
		name     string
		sequence string
		want     int
	}{
		{"no N", "ATGC", 0},
		{"single N", "ATNGC", 1},
		{"multiple N", "ATNNNGC", 3},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			seq, err := New(tt.sequence)
			require.NoError(t, err)
			assert.Equal(t, tt.want, seq.CountAmbiguous())
		})
	}
}

func TestSubsequence(t *testing.T) {
	seq, err := New("ATGCATGC")
	require.NoError(t, err)

	tests := []struct {
		name    string
		start   int
		end     int
		want    string
		wantErr bool
	}{
		{"first half", 0, 4, "ATGC", false},
		{"second half", 4, 8, "ATGC", false},
		{"middle", 2, 6, "GCAT", false},
		{"single", 0, 1, "A", false},
		{"negative start", -1, 4, "", true},
		{"end before start", 4, 2, "", true},
		{"end out of bounds", 0, 10, "", true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			sub, err := seq.Subsequence(tt.start, tt.end)
			if tt.wantErr {
				require.Error(t, err)
			} else {
				require.NoError(t, err)
				assert.Equal(t, tt.want, sub.Bases)
			}
		})
	}
}

func TestContainsMotif(t *testing.T) {
	seq, err := New("ATGCATGCATGC")
	require.NoError(t, err)

	tests := []struct {
		name  string
		motif string
		want  bool
	}{
		{"present motif", "ATGC", true},
		{"present lowercase", "atgc", true},
		{"not present", "AAAA", false},
		{"partial match", "ATGA", false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			contains, err := seq.ContainsMotif(tt.motif)
			require.NoError(t, err)
			assert.Equal(t, tt.want, contains)
		})
	}
}

func TestFindMotifPositions(t *testing.T) {
	seq, err := New("ATGATGATGATG")
	require.NoError(t, err)

	positions, err := seq.FindMotifPositions("ATG")
	require.NoError(t, err)
	assert.Equal(t, []int{0, 3, 6, 9}, positions)
}

func TestTranscribe(t *testing.T) {
	seq, err := New("ATGCATGC")
	require.NoError(t, err)

	rna, err := seq.Transcribe()
	require.NoError(t, err)
	assert.Equal(t, "AUGCAUGC", rna.Bases)
	assert.Equal(t, RNA, rna.SeqType)
}

func TestToFASTA(t *testing.T) {
	seq := &Sequence{
		Bases:       "ATGC",
		ID:          "seq1",
		Description: "Test sequence",
		SeqType:     DNA,
	}

	fasta := seq.ToFASTA()
	assert.Contains(t, fasta, ">seq1 Test sequence")
	assert.Contains(t, fasta, "ATGC")
}

func TestEqual(t *testing.T) {
	seq1, _ := New("ATGC")
	seq2, _ := New("ATGC")
	seq3, _ := New("GCTA")

	assert.True(t, seq1.Equal(seq2))
	assert.False(t, seq1.Equal(seq3))
	assert.False(t, seq1.Equal(nil))
}

func BenchmarkNew(b *testing.B) {
	bases := "ATGCATGCATGCATGCATGCATGCATGCATGCATGCATGC"
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = New(bases)
	}
}

func BenchmarkGCContent(b *testing.B) {
	seq, _ := New("ATGCATGCATGCATGCATGCATGCATGCATGCATGCATGC")
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = seq.GCContent()
	}
}

func BenchmarkComplement(b *testing.B) {
	seq, _ := New("ATGCATGCATGCATGCATGCATGCATGCATGCATGCATGC")
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = seq.Complement()
	}
}

func BenchmarkReverseComplement(b *testing.B) {
	seq, _ := New("ATGCATGCATGCATGCATGCATGCATGCATGCATGCATGC")
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = seq.ReverseComplement()
	}
}
