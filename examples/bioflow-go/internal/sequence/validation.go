package sequence

import "fmt"

// SequenceError is the base error type for sequence operations.
type SequenceError interface {
	error
	IsSequenceError()
}

// EmptySequenceError is returned when a sequence is empty.
type EmptySequenceError struct{}

func (e *EmptySequenceError) Error() string {
	return "sequence must have at least one base"
}

func (e *EmptySequenceError) IsSequenceError() {}

// InvalidBaseError is returned when an invalid base is encountered.
type InvalidBaseError struct {
	Position int
	Found    rune
}

func (e *InvalidBaseError) Error() string {
	return fmt.Sprintf("invalid base '%c' at position %d", e.Found, e.Position)
}

func (e *InvalidBaseError) IsSequenceError() {}

// InvalidLengthError is returned when sequence length is invalid.
type InvalidLengthError struct {
	Expected int
	Actual   int
}

func (e *InvalidLengthError) Error() string {
	return fmt.Sprintf("expected length %d, got %d", e.Expected, e.Actual)
}

func (e *InvalidLengthError) IsSequenceError() {}

// ValidateDNA validates that a string contains only valid DNA bases.
func ValidateDNA(bases string) error {
	for i, b := range bases {
		if !ValidDNABases[b] {
			return &InvalidBaseError{Position: i, Found: b}
		}
	}
	return nil
}

// ValidateRNA validates that a string contains only valid RNA bases.
func ValidateRNA(bases string) error {
	for i, b := range bases {
		if !ValidRNABases[b] {
			return &InvalidBaseError{Position: i, Found: b}
		}
	}
	return nil
}

// IsValidDNABase checks if a character is a valid DNA base.
func IsValidDNABase(c rune) bool {
	return ValidDNABases[c]
}

// IsValidRNABase checks if a character is a valid RNA base.
func IsValidRNABase(c rune) bool {
	return ValidRNABases[c]
}
