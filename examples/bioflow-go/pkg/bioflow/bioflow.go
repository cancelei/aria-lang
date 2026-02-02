// Package bioflow provides a high-level API for genomic sequence analysis.
//
// This package exposes the core BioFlow functionality through a simple,
// easy-to-use API for common bioinformatics operations.
//
// Example usage:
//
//	seq, err := bioflow.NewSequence("ATGCATGC")
//	if err != nil {
//	    log.Fatal(err)
//	}
//
//	gc := seq.GCContent()
//	fmt.Printf("GC Content: %.2f%%\n", gc*100)
//
//	alignment, err := bioflow.Align(seq1, seq2)
//	if err != nil {
//	    log.Fatal(err)
//	}
//	fmt.Println(alignment.Format())
package bioflow

import (
	"bufio"
	"fmt"
	"io"
	"os"
	"strings"

	"github.com/aria-lang/bioflow-go/internal/alignment"
	"github.com/aria-lang/bioflow-go/internal/kmer"
	"github.com/aria-lang/bioflow-go/internal/quality"
	"github.com/aria-lang/bioflow-go/internal/sequence"
	"github.com/aria-lang/bioflow-go/internal/stats"
)

// Re-export types for convenience
type (
	Sequence      = sequence.Sequence
	SequenceType  = sequence.SequenceType
	Alignment     = alignment.Alignment
	ScoringMatrix = alignment.ScoringMatrix
	KMerCounter   = kmer.Counter
	KMerCount     = kmer.KMerCount
	QualityScores = quality.Scores
	QualityStats  = quality.Stats
	Filter        = quality.Filter
)

// Constants
const (
	DNA     = sequence.DNA
	RNA     = sequence.RNA
	Unknown = sequence.Unknown
)

// NewSequence creates a new DNA sequence.
func NewSequence(bases string) (*Sequence, error) {
	return sequence.New(bases)
}

// NewSequenceWithID creates a new sequence with an identifier.
func NewSequenceWithID(bases, id string) (*Sequence, error) {
	return sequence.WithID(bases, id)
}

// NewRNASequence creates a new RNA sequence.
func NewRNASequence(bases string) (*Sequence, error) {
	return sequence.WithMetadata(bases, "", "", sequence.RNA)
}

// Align performs local alignment between two sequences.
func Align(seq1, seq2 *Sequence) (*Alignment, error) {
	return alignment.SmithWaterman(seq1, seq2, nil)
}

// AlignGlobal performs global alignment between two sequences.
func AlignGlobal(seq1, seq2 *Sequence) (*Alignment, error) {
	return alignment.NeedlemanWunsch(seq1, seq2, nil)
}

// AlignWithScoring performs local alignment with custom scoring.
func AlignWithScoring(seq1, seq2 *Sequence, scoring *ScoringMatrix) (*Alignment, error) {
	return alignment.SmithWaterman(seq1, seq2, scoring)
}

// DefaultScoring returns the default DNA scoring matrix.
func DefaultScoring() *ScoringMatrix {
	return alignment.DefaultDNA()
}

// CountKMers counts k-mers in a sequence.
func CountKMers(seq *Sequence, k int) (*KMerCounter, error) {
	return kmer.CountKMers(seq, k)
}

// MostFrequentKMers returns the n most frequent k-mers.
func MostFrequentKMers(seq *Sequence, k, n int) ([]KMerCount, error) {
	return kmer.MostFrequentKMers(seq, k, n)
}

// KMerDistance calculates the Jaccard distance between two sequences.
func KMerDistance(seq1, seq2 *Sequence, k int) (float64, error) {
	return kmer.JaccardDistance(seq1, seq2, k)
}

// SharedKMers finds k-mers shared between two sequences.
func SharedKMers(seq1, seq2 *Sequence, k int) ([]string, error) {
	return kmer.SharedKMers(seq1, seq2, k)
}

// NewQualityScores creates quality scores from an array.
func NewQualityScores(scores []int) (*QualityScores, error) {
	return quality.New(scores)
}

// ParseQualityPhred33 parses Phred+33 encoded quality string.
func ParseQualityPhred33(encoded string) (*QualityScores, error) {
	return quality.FromPhred33(encoded)
}

// ParseQualityPhred64 parses Phred+64 encoded quality string.
func ParseQualityPhred64(encoded string) (*QualityScores, error) {
	return quality.FromPhred64(encoded)
}

// DefaultFilter creates a quality filter with default settings.
func DefaultFilter() *Filter {
	return quality.DefaultFilter()
}

// StrictFilter creates a quality filter with strict settings.
func StrictFilter() *Filter {
	return quality.StrictFilter()
}

// SequenceStats calculates statistics for a sequence.
func SequenceStats(seq *Sequence) *stats.SequenceStats {
	return stats.FromSequence(seq)
}

// SequenceSetStats calculates statistics for multiple sequences.
func SequenceSetStats(sequences []*Sequence) (*stats.SequenceSetStats, error) {
	return stats.FromSequences(sequences)
}

// ReadFASTA reads sequences from a FASTA file.
func ReadFASTA(filename string) ([]*Sequence, error) {
	file, err := os.Open(filename)
	if err != nil {
		return nil, fmt.Errorf("opening file: %w", err)
	}
	defer file.Close()

	return ParseFASTA(file)
}

// ParseFASTA parses FASTA format from a reader.
func ParseFASTA(r io.Reader) ([]*Sequence, error) {
	sequences := make([]*Sequence, 0)
	scanner := bufio.NewScanner(r)

	var currentID, currentDesc string
	var currentBases strings.Builder

	flushSequence := func() error {
		if currentBases.Len() > 0 {
			seq, err := sequence.WithMetadata(
				currentBases.String(),
				currentID,
				currentDesc,
				sequence.DNA,
			)
			if err != nil {
				return err
			}
			sequences = append(sequences, seq)
			currentBases.Reset()
		}
		return nil
	}

	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())

		if len(line) == 0 {
			continue
		}

		if line[0] == '>' {
			// Flush previous sequence
			if err := flushSequence(); err != nil {
				return nil, err
			}

			// Parse header
			header := line[1:]
			parts := strings.SplitN(header, " ", 2)
			currentID = parts[0]
			if len(parts) > 1 {
				currentDesc = parts[1]
			} else {
				currentDesc = ""
			}
		} else {
			currentBases.WriteString(line)
		}
	}

	// Flush last sequence
	if err := flushSequence(); err != nil {
		return nil, err
	}

	if err := scanner.Err(); err != nil {
		return nil, fmt.Errorf("reading file: %w", err)
	}

	return sequences, nil
}

// WriteFASTA writes sequences to a FASTA file.
func WriteFASTA(filename string, sequences []*Sequence) error {
	file, err := os.Create(filename)
	if err != nil {
		return fmt.Errorf("creating file: %w", err)
	}
	defer file.Close()

	for _, seq := range sequences {
		_, err := file.WriteString(seq.ToFASTA())
		if err != nil {
			return fmt.Errorf("writing sequence: %w", err)
		}
	}

	return nil
}

// Read represents a sequencing read with sequence and quality.
type Read struct {
	Sequence *Sequence
	Quality  *QualityScores
}

// NewRead creates a new read from sequence and quality.
func NewRead(bases string, qualityScores []int) (*Read, error) {
	seq, err := sequence.New(bases)
	if err != nil {
		return nil, err
	}

	qual, err := quality.New(qualityScores)
	if err != nil {
		return nil, err
	}

	if seq.Len() != qual.Len() {
		return nil, fmt.Errorf("sequence and quality must have same length")
	}

	return &Read{
		Sequence: seq,
		Quality:  qual,
	}, nil
}

// ParseFASTQ parses FASTQ format from a reader.
func ParseFASTQ(r io.Reader) ([]*Read, error) {
	reads := make([]*Read, 0)
	scanner := bufio.NewScanner(r)

	lineNum := 0
	var id, bases, qualStr string

	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		lineNum++

		switch (lineNum - 1) % 4 {
		case 0: // Header
			if len(line) == 0 || line[0] != '@' {
				return nil, fmt.Errorf("line %d: expected header starting with @", lineNum)
			}
			id = line[1:]
		case 1: // Sequence
			bases = line
		case 2: // Quality header
			if len(line) == 0 || line[0] != '+' {
				return nil, fmt.Errorf("line %d: expected '+' line", lineNum)
			}
		case 3: // Quality
			qualStr = line

			// Create read
			seq, err := sequence.WithID(bases, id)
			if err != nil {
				return nil, fmt.Errorf("line %d: %w", lineNum, err)
			}

			qual, err := quality.FromPhred33(qualStr)
			if err != nil {
				return nil, fmt.Errorf("line %d: %w", lineNum, err)
			}

			reads = append(reads, &Read{
				Sequence: seq,
				Quality:  qual,
			})
		}
	}

	if err := scanner.Err(); err != nil {
		return nil, fmt.Errorf("reading file: %w", err)
	}

	return reads, nil
}

// ReadFASTQ reads reads from a FASTQ file.
func ReadFASTQ(filename string) ([]*Read, error) {
	file, err := os.Open(filename)
	if err != nil {
		return nil, fmt.Errorf("opening file: %w", err)
	}
	defer file.Close()

	return ParseFASTQ(file)
}

// Pipeline represents a processing pipeline for reads.
type Pipeline struct {
	filter *Filter
}

// NewPipeline creates a new processing pipeline.
func NewPipeline(filter *Filter) *Pipeline {
	if filter == nil {
		filter = quality.DefaultFilter()
	}
	return &Pipeline{filter: filter}
}

// ProcessReads processes reads through the pipeline.
func (p *Pipeline) ProcessReads(reads []*Read) (*quality.BatchFilterResult, error) {
	sequences := make([]*Sequence, len(reads))
	qualities := make([]*QualityScores, len(reads))

	for i, read := range reads {
		sequences[i] = read.Sequence
		qualities[i] = read.Quality
	}

	return p.filter.BatchFilter(sequences, qualities)
}

// Version returns the BioFlow version.
func Version() string {
	return "1.0.0"
}

// Info returns information about BioFlow.
func Info() string {
	return fmt.Sprintf(`BioFlow v%s - Genomic Sequence Analysis Library

A production-quality Go implementation of the BioFlow genomic pipeline.

Features:
  - DNA/RNA sequence handling with validation
  - GC/AT content calculation
  - Sequence complement and reverse complement
  - K-mer counting and analysis
  - Smith-Waterman local alignment
  - Needleman-Wunsch global alignment
  - Phred quality score handling
  - Quality-based read filtering
  - FASTA/FASTQ file parsing

For more information, see: https://github.com/aria-lang/bioflow-go
`, Version())
}
