// Package kmer provides k-mer counting and analysis functionality.
//
// K-mers are subsequences of length k. This package provides efficient
// counting, frequency analysis, and distance calculations.
//
// Comparison with Aria:
//
//	Aria uses compile-time invariants:
//	  struct KMerCounts
//	    invariant self.counts.all(|(kmer, _)| kmer.len() == self.k)
//
//	Go relies on runtime validation.
package kmer

import (
	"fmt"
	"sort"
	"strings"

	"github.com/aria-lang/bioflow-go/internal/sequence"
)

// KMer represents a single k-mer with its properties.
//
// Aria equivalent:
//
//	struct KMer
//	  sequence: String
//	  k: Int
//	  invariant self.sequence.len() == self.k
//	  invariant self.k > 0
type KMer struct {
	Sequence string
	K        int
}

// NewKMer creates a new k-mer from a sequence string.
func NewKMer(seq string) (*KMer, error) {
	seq = strings.ToUpper(seq)
	if len(seq) == 0 {
		return nil, fmt.Errorf("k-mer sequence cannot be empty")
	}

	return &KMer{
		Sequence: seq,
		K:        len(seq),
	}, nil
}

// ReverseComplement returns the reverse complement of this k-mer.
//
// Aria equivalent:
//
//	fn reverse_complement(self) -> KMer
//	  ensures result.k == self.k
func (km *KMer) ReverseComplement() *KMer {
	compMap := map[rune]rune{'A': 'T', 'T': 'A', 'C': 'G', 'G': 'C', 'N': 'N'}
	runes := []rune(km.Sequence)
	n := len(runes)
	result := make([]rune, n)

	for i := 0; i < n; i++ {
		if comp, ok := compMap[runes[n-1-i]]; ok {
			result[i] = comp
		} else {
			result[i] = 'N'
		}
	}

	return &KMer{
		Sequence: string(result),
		K:        km.K,
	}
}

// Canonical returns the canonical form (lexicographically smaller of forward/reverse complement).
//
// Aria equivalent:
//
//	fn canonical(self) -> KMer
//	  ensures result.k == self.k
func (km *KMer) Canonical() *KMer {
	rc := km.ReverseComplement()
	if km.Sequence < rc.Sequence {
		return km
	}
	return rc
}

func (km *KMer) String() string {
	return km.Sequence
}

// KMerCount represents a k-mer and its count.
type KMerCount struct {
	KMer  string
	Count int
}

// Counter provides k-mer counting functionality.
//
// Aria equivalent:
//
//	struct KMerCounts
//	  k: Int
//	  counts: Map<String, Int>
//	  total_kmers: Int
//	  invariant self.k > 0
//	  invariant self.counts.all(|(kmer, _)| kmer.len() == self.k)
type Counter struct {
	K      int
	Counts map[string]int
	Total  int
}

// NewCounter creates a new k-mer counter with the specified k value.
//
// Aria equivalent:
//
//	fn new(k: Int) -> KMerCounts
//	  requires k > 0
//	  ensures result.k == k
//	  ensures result.total_kmers == 0
func NewCounter(k int) (*Counter, error) {
	if k <= 0 {
		return nil, fmt.Errorf("k must be positive")
	}

	return &Counter{
		K:      k,
		Counts: make(map[string]int),
		Total:  0,
	}, nil
}

// Add adds a k-mer count.
//
// Aria equivalent:
//
//	fn add(mut self, kmer: String, count: Int)
//	  requires kmer.len() == self.k
//	  requires count > 0
func (c *Counter) Add(kmer string, count int) error {
	if len(kmer) != c.K {
		return fmt.Errorf("k-mer length %d doesn't match k=%d", len(kmer), c.K)
	}
	if count <= 0 {
		return fmt.Errorf("count must be positive")
	}

	kmer = strings.ToUpper(kmer)
	c.Counts[kmer] += count
	c.Total += count
	return nil
}

// CountKMers counts all k-mers in a sequence string.
func (c *Counter) CountKMers(seq string) {
	seq = strings.ToUpper(seq)
	for i := 0; i <= len(seq)-c.K; i++ {
		kmer := seq[i : i+c.K]
		if !strings.ContainsRune(kmer, 'N') {
			c.Counts[kmer]++
			c.Total++
		}
	}
}

// CountFromSequence counts all k-mers from a Sequence object.
func (c *Counter) CountFromSequence(seq *sequence.Sequence) {
	c.CountKMers(seq.Bases)
}

// GetCount returns the count for a specific k-mer.
//
// Aria equivalent:
//
//	fn get_count(self, kmer: String) -> Int
//	  requires kmer.len() == self.k
//	  ensures result >= 0
func (c *Counter) GetCount(kmer string) (int, error) {
	if len(kmer) != c.K {
		return 0, fmt.Errorf("k-mer length doesn't match k=%d", c.K)
	}
	return c.Counts[strings.ToUpper(kmer)], nil
}

// UniqueCount returns the number of unique k-mers.
//
// Aria equivalent:
//
//	fn unique_count(self) -> Int
//	  ensures result >= 0
func (c *Counter) UniqueCount() int {
	return len(c.Counts)
}

// MostFrequent returns the n most frequent k-mers.
//
// Aria equivalent:
//
//	fn most_frequent(self, n: Int) -> [(String, Int)]
//	  requires n > 0
//	  ensures result.len() <= n
func (c *Counter) MostFrequent(n int) ([]KMerCount, error) {
	if n <= 0 {
		return nil, fmt.Errorf("n must be positive")
	}

	counts := make([]KMerCount, 0, len(c.Counts))
	for kmer, count := range c.Counts {
		counts = append(counts, KMerCount{KMer: kmer, Count: count})
	}

	sort.Slice(counts, func(i, j int) bool {
		return counts[i].Count > counts[j].Count
	})

	if n > len(counts) {
		n = len(counts)
	}
	return counts[:n], nil
}

// LeastFrequent returns the n least frequent k-mers.
//
// Aria equivalent:
//
//	fn least_frequent(self, n: Int) -> [(String, Int)]
//	  requires n > 0
//	  ensures result.len() <= n
func (c *Counter) LeastFrequent(n int) ([]KMerCount, error) {
	if n <= 0 {
		return nil, fmt.Errorf("n must be positive")
	}

	counts := make([]KMerCount, 0, len(c.Counts))
	for kmer, count := range c.Counts {
		counts = append(counts, KMerCount{KMer: kmer, Count: count})
	}

	sort.Slice(counts, func(i, j int) bool {
		return counts[i].Count < counts[j].Count
	})

	if n > len(counts) {
		n = len(counts)
	}
	return counts[:n], nil
}

// Frequency calculates the frequency of a k-mer.
//
// Aria equivalent:
//
//	fn frequency(self, kmer: String) -> Float
//	  requires kmer.len() == self.k
//	  ensures result >= 0.0 and result <= 1.0
func (c *Counter) Frequency(kmer string) (float64, error) {
	if c.Total == 0 {
		return 0.0, nil
	}
	count, err := c.GetCount(kmer)
	if err != nil {
		return 0, err
	}
	return float64(count) / float64(c.Total), nil
}

// FilterByCount returns k-mers with count above threshold.
//
// Aria equivalent:
//
//	fn filter_by_count(self, min_count: Int) -> [(String, Int)]
//	  requires min_count > 0
//	  ensures result.all(|(_, count)| count >= min_count)
func (c *Counter) FilterByCount(minCount int) ([]KMerCount, error) {
	if minCount <= 0 {
		return nil, fmt.Errorf("min_count must be positive")
	}

	result := make([]KMerCount, 0)
	for kmer, count := range c.Counts {
		if count >= minCount {
			result = append(result, KMerCount{KMer: kmer, Count: count})
		}
	}
	return result, nil
}

// Merge merges another Counter into this one.
//
// Aria equivalent:
//
//	fn merge(mut self, other: KMerCounts)
//	  requires self.k == other.k
func (c *Counter) Merge(other *Counter) error {
	if c.K != other.K {
		return fmt.Errorf("k values must match")
	}

	for kmer, count := range other.Counts {
		c.Counts[kmer] += count
		c.Total += count
	}
	return nil
}

func (c *Counter) String() string {
	return fmt.Sprintf("KMerCounter { k: %d, unique: %d, total: %d }", c.K, c.UniqueCount(), c.Total)
}

// CountKMers counts all k-mers in a sequence.
//
// Aria equivalent:
//
//	fn count_kmers(sequence: Sequence, k: Int) -> KMerCounts
//	  requires k > 0
//	  requires k <= sequence.len()
//	  ensures result.k == k
func CountKMers(seq *sequence.Sequence, k int) (*Counter, error) {
	if k <= 0 {
		return nil, fmt.Errorf("k must be positive")
	}
	if k > seq.Len() {
		return nil, fmt.Errorf("k cannot exceed sequence length")
	}

	counter, err := NewCounter(k)
	if err != nil {
		return nil, err
	}
	counter.CountFromSequence(seq)
	return counter, nil
}

// MostFrequentKMers returns the n most frequent k-mers.
//
// Aria equivalent:
//
//	fn most_frequent_kmers(sequence: Sequence, k: Int, n: Int) -> [(String, Int)]
//	  requires k > 0 and k <= sequence.len()
//	  requires n > 0
//	  ensures result.len() <= n
func MostFrequentKMers(seq *sequence.Sequence, k, n int) ([]KMerCount, error) {
	counter, err := CountKMers(seq, k)
	if err != nil {
		return nil, err
	}
	return counter.MostFrequent(n)
}

// FindUniqueKMers finds k-mers occurring exactly once.
//
// Aria equivalent:
//
//	fn find_unique_kmers(sequence: Sequence, k: Int) -> [String]
//	  requires k > 0 and k <= sequence.len()
//	  ensures result.all(|kmer| kmer.len() == k)
func FindUniqueKMers(seq *sequence.Sequence, k int) ([]string, error) {
	counter, err := CountKMers(seq, k)
	if err != nil {
		return nil, err
	}

	result := make([]string, 0)
	for kmer, count := range counter.Counts {
		if count == 1 {
			result = append(result, kmer)
		}
	}
	return result, nil
}

// KMerSpectrum generates k-mer spectrum (count distribution).
//
// Returns a list of (count, number_of_kmers_with_that_count) pairs.
//
// Aria equivalent:
//
//	fn kmer_spectrum(sequence: Sequence, k: Int) -> [(Int, Int)]
//	  requires k > 0 and k <= sequence.len()
func KMerSpectrum(seq *sequence.Sequence, k int) ([]KMerCount, error) {
	counter, err := CountKMers(seq, k)
	if err != nil {
		return nil, err
	}

	// Build spectrum (count -> number of k-mers with that count)
	spectrumMap := make(map[int]int)
	for _, count := range counter.Counts {
		spectrumMap[count]++
	}

	// Convert to slice and sort
	result := make([]KMerCount, 0, len(spectrumMap))
	for count, numKMers := range spectrumMap {
		result = append(result, KMerCount{KMer: "", Count: count})
		result[len(result)-1].Count = numKMers
	}

	// Sort by the count value (stored as metadata)
	sort.Slice(result, func(i, j int) bool {
		return result[i].Count < result[j].Count
	})

	return result, nil
}

// KMerPositions finds all positions of a k-mer in a sequence.
//
// Aria equivalent:
//
//	fn kmer_positions(sequence: Sequence, kmer: String) -> [Int]
//	  requires kmer.len() > 0
//	  requires kmer.len() <= sequence.len()
func KMerPositions(seq *sequence.Sequence, kmer string) ([]int, error) {
	if len(kmer) == 0 {
		return nil, fmt.Errorf("k-mer cannot be empty")
	}
	if len(kmer) > seq.Len() {
		return nil, fmt.Errorf("k-mer cannot be longer than sequence")
	}

	kmer = strings.ToUpper(kmer)
	positions := make([]int, 0)

	for i := 0; i <= seq.Len()-len(kmer); i++ {
		if seq.Bases[i:i+len(kmer)] == kmer {
			positions = append(positions, i)
		}
	}

	return positions, nil
}

// CountKMersCanonical counts canonical k-mers (treating reverse complements as same).
//
// Aria equivalent:
//
//	fn count_kmers_canonical(sequence: Sequence, k: Int) -> KMerCounts
//	  requires k > 0 and k <= sequence.len()
//	  ensures result.k == k
func CountKMersCanonical(seq *sequence.Sequence, k int) (*Counter, error) {
	if k <= 0 {
		return nil, fmt.Errorf("k must be positive")
	}
	if k > seq.Len() {
		return nil, fmt.Errorf("k cannot exceed sequence length")
	}

	counter, err := NewCounter(k)
	if err != nil {
		return nil, err
	}

	for i := 0; i <= seq.Len()-k; i++ {
		kmerStr := seq.Bases[i : i+k]

		if !strings.ContainsRune(kmerStr, 'N') {
			km, _ := NewKMer(kmerStr)
			canonical := km.Canonical()
			counter.Counts[canonical.Sequence]++
			counter.Total++
		}
	}

	return counter, nil
}

// EstimateGenomeSize estimates genome size using k-mer spectrum.
//
// Uses the peak count method: genome_size ~ total_kmers / peak_coverage
//
// Aria equivalent:
//
//	fn estimate_genome_size(total_kmers: Int, peak_coverage: Int, k: Int) -> Int
//	  requires total_kmers > 0
//	  requires peak_coverage > 0
//	  requires k > 0
//	  ensures result > 0
func EstimateGenomeSize(totalKMers, peakCoverage, k int) (int, error) {
	if totalKMers <= 0 || peakCoverage <= 0 || k <= 0 {
		return 0, fmt.Errorf("all parameters must be positive")
	}
	return totalKMers / peakCoverage, nil
}
