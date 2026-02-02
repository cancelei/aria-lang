// Package sequence provides DNA/RNA sequence types with validation.
//
// This module provides a type-safe representation of genomic sequences
// with runtime validation of nucleotide bases. Unlike Aria's compile-time
// contracts, Go relies on runtime checks.
package sequence

import (
	"fmt"
	"strings"
)

// SequenceType represents the type of biological sequence.
type SequenceType int

const (
	// DNA represents a DNA sequence (A, C, G, T)
	DNA SequenceType = iota
	// RNA represents an RNA sequence (A, C, G, U)
	RNA
	// Unknown represents an unknown sequence type
	Unknown
)

func (t SequenceType) String() string {
	switch t {
	case DNA:
		return "DNA"
	case RNA:
		return "RNA"
	default:
		return "Unknown"
	}
}

// Valid nucleotide bases
var (
	ValidDNABases = map[rune]bool{'A': true, 'C': true, 'G': true, 'T': true, 'N': true}
	ValidRNABases = map[rune]bool{'A': true, 'C': true, 'G': true, 'U': true, 'N': true}
)

// Sequence represents a validated genomic sequence (DNA or RNA).
//
// In Aria, invariants provide compile-time guarantees:
//
//	struct Sequence
//	  invariant self.bases.len() > 0
//	  invariant self.is_valid()
//
// In Go, we validate at construction time only.
type Sequence struct {
	Bases       string
	ID          string
	Description string
	SeqType     SequenceType
}

// New creates a new DNA sequence with validation.
//
// Aria equivalent:
//
//	fn new(bases: String) -> Result<Sequence, SequenceError>
//	  requires bases.len() > 0
//	  ensures result.is_ok() implies result.unwrap().is_valid()
func New(bases string) (*Sequence, error) {
	normalized := strings.ToUpper(bases)

	if len(normalized) == 0 {
		return nil, &EmptySequenceError{}
	}

	if err := ValidateDNA(normalized); err != nil {
		return nil, err
	}

	return &Sequence{
		Bases:   normalized,
		SeqType: DNA,
	}, nil
}

// WithID creates a new sequence with an identifier.
func WithID(bases, id string) (*Sequence, error) {
	if len(id) == 0 {
		return nil, fmt.Errorf("ID cannot be empty")
	}

	seq, err := New(bases)
	if err != nil {
		return nil, err
	}

	seq.ID = id
	return seq, nil
}

// WithMetadata creates a new sequence with full metadata.
func WithMetadata(bases, id, description string, seqType SequenceType) (*Sequence, error) {
	normalized := strings.ToUpper(bases)

	if len(normalized) == 0 {
		return nil, &EmptySequenceError{}
	}

	var validErr error
	switch seqType {
	case DNA:
		validErr = ValidateDNA(normalized)
	case RNA:
		validErr = ValidateRNA(normalized)
	default:
		validErr = ValidateDNA(normalized)
	}

	if validErr != nil {
		return nil, validErr
	}

	return &Sequence{
		Bases:       normalized,
		ID:          id,
		Description: description,
		SeqType:     seqType,
	}, nil
}

// Len returns the length of the sequence.
//
// Aria equivalent:
//
//	fn len(self) -> Int
//	  ensures result > 0
func (s *Sequence) Len() int {
	return len(s.Bases)
}

// IsValid checks if all bases are valid for the sequence type.
func (s *Sequence) IsValid() bool {
	switch s.SeqType {
	case DNA:
		return ValidateDNA(s.Bases) == nil
	case RNA:
		return ValidateRNA(s.Bases) == nil
	default:
		return ValidateDNA(s.Bases) == nil
	}
}

// HasAmbiguous checks if the sequence contains any ambiguous bases (N).
func (s *Sequence) HasAmbiguous() bool {
	return strings.ContainsRune(s.Bases, 'N')
}

// CountAmbiguous counts the number of ambiguous bases.
func (s *Sequence) CountAmbiguous() int {
	count := 0
	for _, b := range s.Bases {
		if b == 'N' {
			count++
		}
	}
	return count
}

// BaseAt returns the base at a specific index, or empty if out of bounds.
//
// Aria equivalent:
//
//	fn base_at(self, index: Int) -> Option<Char>
//	  requires index >= 0
func (s *Sequence) BaseAt(index int) (rune, bool) {
	if index < 0 || index >= len(s.Bases) {
		return 0, false
	}
	return rune(s.Bases[index]), true
}

// Subsequence returns a slice of the sequence.
//
// Aria equivalent:
//
//	fn subsequence(self, start: Int, end: Int) -> Result<Sequence, SequenceError>
//	  requires start >= 0
//	  requires end > start
//	  requires end <= self.len()
//	  ensures result.is_ok() implies result.unwrap().len() == end - start
func (s *Sequence) Subsequence(start, end int) (*Sequence, error) {
	if start < 0 {
		return nil, fmt.Errorf("start index must be non-negative")
	}
	if end <= start {
		return nil, fmt.Errorf("end must be greater than start")
	}
	if end > len(s.Bases) {
		return nil, fmt.Errorf("end must not exceed sequence length")
	}

	return &Sequence{
		Bases:       s.Bases[start:end],
		ID:          s.ID,
		Description: s.Description,
		SeqType:     s.SeqType,
	}, nil
}

// complementBase returns the complement of a DNA base.
func complementBase(c rune) rune {
	switch c {
	case 'A':
		return 'T'
	case 'T':
		return 'A'
	case 'C':
		return 'G'
	case 'G':
		return 'C'
	default:
		return 'N'
	}
}

// Complement returns the complement of the sequence (A<->T, C<->G).
//
// Aria equivalent:
//
//	fn complement(self) -> Sequence
//	  requires self.seq_type == SequenceType::DNA
//	  ensures result.len() == self.len()
func (s *Sequence) Complement() (*Sequence, error) {
	if s.SeqType != DNA {
		return nil, fmt.Errorf("complement only available for DNA sequences")
	}

	comp := make([]rune, len(s.Bases))
	for i, b := range s.Bases {
		comp[i] = complementBase(b)
	}

	return &Sequence{
		Bases:       string(comp),
		ID:          s.ID,
		Description: s.Description,
		SeqType:     s.SeqType,
	}, nil
}

// Reverse returns the reverse of the sequence.
//
// Aria equivalent:
//
//	fn reverse(self) -> Sequence
//	  ensures result.len() == self.len()
func (s *Sequence) Reverse() *Sequence {
	runes := []rune(s.Bases)
	n := len(runes)
	for i := 0; i < n/2; i++ {
		runes[i], runes[n-1-i] = runes[n-1-i], runes[i]
	}

	return &Sequence{
		Bases:       string(runes),
		ID:          s.ID,
		Description: s.Description,
		SeqType:     s.SeqType,
	}
}

// ReverseComplement returns the reverse complement of the sequence.
//
// Aria equivalent:
//
//	fn reverse_complement(self) -> Sequence
//	  requires self.seq_type == SequenceType::DNA
//	  ensures result.len() == self.len()
func (s *Sequence) ReverseComplement() (*Sequence, error) {
	comp, err := s.Complement()
	if err != nil {
		return nil, err
	}
	return comp.Reverse(), nil
}

// GCContent calculates the GC content (proportion of G and C bases).
//
// Aria equivalent:
//
//	fn gc_content(self) -> Float
//	  requires self.is_valid()
//	  ensures result >= 0.0 and result <= 1.0
func (s *Sequence) GCContent() float64 {
	if len(s.Bases) == 0 {
		return 0.0
	}

	gcCount := 0
	for _, b := range s.Bases {
		if b == 'G' || b == 'C' {
			gcCount++
		}
	}

	return float64(gcCount) / float64(len(s.Bases))
}

// ATContent calculates the AT content (proportion of A and T bases).
//
// Aria equivalent:
//
//	fn at_content(self) -> Float
//	  requires self.seq_type == SequenceType::DNA
//	  ensures result >= 0.0 and result <= 1.0
func (s *Sequence) ATContent() (float64, error) {
	if s.SeqType != DNA {
		return 0, fmt.Errorf("AT content only available for DNA sequences")
	}

	if len(s.Bases) == 0 {
		return 0.0, nil
	}

	atCount := 0
	for _, b := range s.Bases {
		if b == 'A' || b == 'T' {
			atCount++
		}
	}

	return float64(atCount) / float64(len(s.Bases)), nil
}

// BaseCounts returns counts of each base type.
//
// Aria equivalent:
//
//	fn base_counts(self) -> (Int, Int, Int, Int, Int)
//	  ensures result.0 + result.1 + result.2 + result.3 + result.4 == self.len()
type BaseCounts struct {
	A int
	C int
	G int
	T int // Also counts U for RNA
	N int
}

// BaseCounts returns the count of each base type.
func (s *Sequence) BaseCounts() BaseCounts {
	counts := BaseCounts{}

	for _, b := range s.Bases {
		switch b {
		case 'A':
			counts.A++
		case 'C':
			counts.C++
		case 'G':
			counts.G++
		case 'T', 'U':
			counts.T++
		case 'N':
			counts.N++
		}
	}

	return counts
}

// Total returns the total count of all bases.
func (bc BaseCounts) Total() int {
	return bc.A + bc.C + bc.G + bc.T + bc.N
}

// Transcribe converts DNA to RNA (T -> U).
//
// Aria equivalent:
//
//	fn transcribe(self) -> Sequence
//	  requires self.seq_type == SequenceType::DNA
//	  ensures result.seq_type == SequenceType::RNA
func (s *Sequence) Transcribe() (*Sequence, error) {
	if s.SeqType != DNA {
		return nil, fmt.Errorf("can only transcribe DNA")
	}

	rnaSeq := strings.ReplaceAll(s.Bases, "T", "U")

	return &Sequence{
		Bases:       rnaSeq,
		ID:          s.ID,
		Description: s.Description,
		SeqType:     RNA,
	}, nil
}

// Concat concatenates two sequences.
//
// Aria equivalent:
//
//	fn concat(self, other: Sequence) -> Sequence
//	  requires self.seq_type == other.seq_type
//	  ensures result.len() == self.len() + other.len()
func (s *Sequence) Concat(other *Sequence) (*Sequence, error) {
	if s.SeqType != other.SeqType {
		return nil, fmt.Errorf("cannot concatenate different sequence types")
	}

	return &Sequence{
		Bases:       s.Bases + other.Bases,
		ID:          s.ID,
		Description: s.Description,
		SeqType:     s.SeqType,
	}, nil
}

// ContainsMotif checks if the sequence contains a motif (substring).
//
// Aria equivalent:
//
//	fn contains_motif(self, motif: String) -> Bool
//	  requires motif.len() > 0
func (s *Sequence) ContainsMotif(motif string) (bool, error) {
	if len(motif) == 0 {
		return false, fmt.Errorf("motif cannot be empty")
	}
	if len(motif) > len(s.Bases) {
		return false, fmt.Errorf("motif cannot be longer than sequence")
	}

	return strings.Contains(s.Bases, strings.ToUpper(motif)), nil
}

// FindMotifPositions finds all positions where a motif occurs.
//
// Aria equivalent:
//
//	fn find_motif_positions(self, motif: String) -> [Int]
//	  requires motif.len() > 0
//	  ensures result.all(|pos| pos >= 0 and pos < self.len())
func (s *Sequence) FindMotifPositions(motif string) ([]int, error) {
	if len(motif) == 0 {
		return nil, fmt.Errorf("motif cannot be empty")
	}

	motifUpper := strings.ToUpper(motif)
	positions := make([]int, 0)

	if len(motifUpper) > len(s.Bases) {
		return positions, nil
	}

	for i := 0; i <= len(s.Bases)-len(motifUpper); i++ {
		if s.Bases[i:i+len(motifUpper)] == motifUpper {
			positions = append(positions, i)
		}
	}

	return positions, nil
}

// ToFASTA returns the sequence in FASTA format.
func (s *Sequence) ToFASTA() string {
	var header string
	if s.ID != "" {
		header = ">" + s.ID
		if s.Description != "" {
			header += " " + s.Description
		}
	} else {
		header = ">sequence"
	}

	var sb strings.Builder
	sb.WriteString(header)
	sb.WriteRune('\n')

	// Split sequence into 80-character lines
	for i := 0; i < len(s.Bases); i += 80 {
		end := i + 80
		if end > len(s.Bases) {
			end = len(s.Bases)
		}
		sb.WriteString(s.Bases[i:end])
		sb.WriteRune('\n')
	}

	return sb.String()
}

// String returns a string representation of the sequence.
func (s *Sequence) String() string {
	if s.ID != "" {
		return fmt.Sprintf(">%s\n%s", s.ID, s.Bases)
	}
	return s.Bases
}

// Equal checks equality with another sequence.
func (s *Sequence) Equal(other *Sequence) bool {
	if other == nil {
		return false
	}
	return s.Bases == other.Bases && s.SeqType == other.SeqType
}
