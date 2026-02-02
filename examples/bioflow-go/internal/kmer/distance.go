package kmer

import (
	"fmt"
	"math"

	"github.com/aria-lang/bioflow-go/internal/sequence"
)

// JaccardDistance calculates the Jaccard distance between two sequences.
//
// Jaccard distance = 1 - (intersection / union)
//
// Aria equivalent:
//
//	fn kmer_distance(seq1: Sequence, seq2: Sequence, k: Int) -> Float
//	  requires k > 0
//	  requires k <= seq1.len() and k <= seq2.len()
//	  ensures result >= 0.0 and result <= 1.0
func JaccardDistance(seq1, seq2 *sequence.Sequence, k int) (float64, error) {
	if k <= 0 {
		return 0, fmt.Errorf("k must be positive")
	}
	if k > seq1.Len() || k > seq2.Len() {
		return 0, fmt.Errorf("k cannot exceed sequence lengths")
	}

	counter1, err := CountKMers(seq1, k)
	if err != nil {
		return 0, err
	}

	counter2, err := CountKMers(seq2, k)
	if err != nil {
		return 0, err
	}

	// Calculate intersection and union
	set1 := make(map[string]bool)
	for kmer := range counter1.Counts {
		set1[kmer] = true
	}

	set2 := make(map[string]bool)
	for kmer := range counter2.Counts {
		set2[kmer] = true
	}

	intersection := 0
	for kmer := range set1 {
		if set2[kmer] {
			intersection++
		}
	}

	union := len(set1) + len(set2) - intersection

	if union == 0 {
		return 0.0, nil
	}

	return 1.0 - float64(intersection)/float64(union), nil
}

// SharedKMers finds k-mers shared between two sequences.
//
// Aria equivalent:
//
//	fn shared_kmers(seq1: Sequence, seq2: Sequence, k: Int) -> [String]
//	  requires k > 0
//	  requires k <= seq1.len() and k <= seq2.len()
func SharedKMers(seq1, seq2 *sequence.Sequence, k int) ([]string, error) {
	if k <= 0 {
		return nil, fmt.Errorf("k must be positive")
	}
	if k > seq1.Len() || k > seq2.Len() {
		return nil, fmt.Errorf("k cannot exceed sequence lengths")
	}

	counter1, err := CountKMers(seq1, k)
	if err != nil {
		return nil, err
	}

	counter2, err := CountKMers(seq2, k)
	if err != nil {
		return nil, err
	}

	result := make([]string, 0)
	for kmer := range counter1.Counts {
		if _, ok := counter2.Counts[kmer]; ok {
			result = append(result, kmer)
		}
	}

	return result, nil
}

// CosineDistance calculates the cosine distance between k-mer frequency vectors.
//
// Cosine distance = 1 - (dot product / (magnitude1 * magnitude2))
func CosineDistance(seq1, seq2 *sequence.Sequence, k int) (float64, error) {
	if k <= 0 {
		return 0, fmt.Errorf("k must be positive")
	}
	if k > seq1.Len() || k > seq2.Len() {
		return 0, fmt.Errorf("k cannot exceed sequence lengths")
	}

	counter1, err := CountKMers(seq1, k)
	if err != nil {
		return 0, err
	}

	counter2, err := CountKMers(seq2, k)
	if err != nil {
		return 0, err
	}

	// Get all unique k-mers
	allKMers := make(map[string]bool)
	for kmer := range counter1.Counts {
		allKMers[kmer] = true
	}
	for kmer := range counter2.Counts {
		allKMers[kmer] = true
	}

	// Calculate dot product and magnitudes
	var dotProduct, mag1, mag2 float64

	for kmer := range allKMers {
		v1 := float64(counter1.Counts[kmer])
		v2 := float64(counter2.Counts[kmer])

		dotProduct += v1 * v2
		mag1 += v1 * v1
		mag2 += v2 * v2
	}

	if mag1 == 0 || mag2 == 0 {
		return 1.0, nil
	}

	cosineSimilarity := dotProduct / (math.Sqrt(mag1) * math.Sqrt(mag2))
	return 1.0 - cosineSimilarity, nil
}

// EuclideanDistance calculates the Euclidean distance between k-mer frequency vectors.
func EuclideanDistance(seq1, seq2 *sequence.Sequence, k int) (float64, error) {
	if k <= 0 {
		return 0, fmt.Errorf("k must be positive")
	}
	if k > seq1.Len() || k > seq2.Len() {
		return 0, fmt.Errorf("k cannot exceed sequence lengths")
	}

	counter1, err := CountKMers(seq1, k)
	if err != nil {
		return 0, err
	}

	counter2, err := CountKMers(seq2, k)
	if err != nil {
		return 0, err
	}

	// Get all unique k-mers
	allKMers := make(map[string]bool)
	for kmer := range counter1.Counts {
		allKMers[kmer] = true
	}
	for kmer := range counter2.Counts {
		allKMers[kmer] = true
	}

	// Calculate sum of squared differences
	var sumSqDiff float64

	for kmer := range allKMers {
		v1 := float64(counter1.Counts[kmer])
		v2 := float64(counter2.Counts[kmer])
		diff := v1 - v2
		sumSqDiff += diff * diff
	}

	return math.Sqrt(sumSqDiff), nil
}

// SimilarityMatrix calculates a similarity matrix for multiple sequences.
func SimilarityMatrix(sequences []*sequence.Sequence, k int) ([][]float64, error) {
	n := len(sequences)
	if n == 0 {
		return nil, fmt.Errorf("sequence list cannot be empty")
	}

	matrix := make([][]float64, n)
	for i := range matrix {
		matrix[i] = make([]float64, n)
		matrix[i][i] = 0.0 // Distance to self is 0
	}

	for i := 0; i < n; i++ {
		for j := i + 1; j < n; j++ {
			dist, err := JaccardDistance(sequences[i], sequences[j], k)
			if err != nil {
				return nil, err
			}
			matrix[i][j] = dist
			matrix[j][i] = dist
		}
	}

	return matrix, nil
}
