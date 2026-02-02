"""
BioFlow - Sequence Type

DNA/RNA sequences with validation.

This module provides a representation of genomic sequences.
Unlike Aria's compile-time contracts, validation is done at runtime.

Comparison with Aria:

Aria version uses compile-time guarantees:
    struct Sequence
      invariant self.bases.len() > 0
      invariant self.is_valid()

Python relies on runtime checks only:
    def __post_init__(self):
        if not self.is_valid():
            raise SequenceError(...)
"""

from typing import Optional, List, Tuple
from dataclasses import dataclass, field
from enum import Enum


class SequenceType(Enum):
    """Type of biological sequence."""
    DNA = "DNA"
    RNA = "RNA"
    UNKNOWN = "UNKNOWN"


class SequenceError(Exception):
    """Error types for sequence operations."""
    pass


class EmptySequenceError(SequenceError):
    """Raised when sequence is empty."""
    pass


class InvalidBaseError(SequenceError):
    """Raised when an invalid base is encountered."""
    def __init__(self, position: int, found: str):
        self.position = position
        self.found = found
        super().__init__(f"Invalid base '{found}' at position {position}")


class InvalidLengthError(SequenceError):
    """Raised when sequence length is invalid."""
    def __init__(self, expected: int, actual: int):
        self.expected = expected
        self.actual = actual
        super().__init__(f"Expected length {expected}, got {actual}")


# Valid bases for DNA and RNA
VALID_DNA_BASES = {'A', 'C', 'G', 'T', 'N'}
VALID_RNA_BASES = {'A', 'C', 'G', 'U', 'N'}


@dataclass
class Sequence:
    """
    Represents a validated genomic sequence (DNA or RNA).

    Attributes:
        bases: The nucleotide sequence (uppercase)
        id: Optional sequence identifier
        description: Optional sequence description
        seq_type: Type of sequence (DNA, RNA, or UNKNOWN)

    Note: In Aria, invariants provide compile-time guarantees:
        invariant self.bases.len() > 0
        invariant self.is_valid()

    In Python, we validate at construction time only.
    """
    bases: str
    id: Optional[str] = None
    description: Optional[str] = None
    seq_type: SequenceType = SequenceType.DNA
    _validated: bool = field(default=False, repr=False)

    def __post_init__(self):
        """Validate sequence on construction."""
        # Normalize to uppercase
        self.bases = self.bases.upper()

        # Validate non-empty
        if len(self.bases) == 0:
            raise EmptySequenceError("Sequence must have at least one base")

        # Validate bases
        if not self._validated:
            self._validate()
            self._validated = True

    def _validate(self) -> None:
        """Validate all bases in the sequence."""
        valid_bases = VALID_DNA_BASES if self.seq_type != SequenceType.RNA else VALID_RNA_BASES

        for i, base in enumerate(self.bases):
            if base not in valid_bases:
                raise InvalidBaseError(i, base)

    @classmethod
    def new(cls, bases: str) -> 'Sequence':
        """
        Create a new DNA sequence with validation.

        Aria equivalent:
            fn new(bases: String) -> Result<Sequence, SequenceError>
              requires bases.len() > 0
              ensures result.is_ok() implies result.unwrap().is_valid()
        """
        return cls(bases=bases, seq_type=SequenceType.DNA)

    @classmethod
    def with_id(cls, bases: str, id: str) -> 'Sequence':
        """Create a new sequence with an identifier."""
        if len(id) == 0:
            raise ValueError("ID cannot be empty")
        return cls(bases=bases, id=id, seq_type=SequenceType.DNA)

    @classmethod
    def with_metadata(
        cls,
        bases: str,
        id: str,
        description: str,
        seq_type: SequenceType
    ) -> 'Sequence':
        """Create a new sequence with full metadata."""
        return cls(
            bases=bases,
            id=id,
            description=description,
            seq_type=seq_type
        )

    def is_valid(self) -> bool:
        """
        Check if all bases are valid for the sequence type.

        Note: This is always true after construction due to __post_init__.
        In Aria, this is guaranteed by invariants at compile time.
        """
        valid_bases = VALID_DNA_BASES if self.seq_type != SequenceType.RNA else VALID_RNA_BASES
        return all(b in valid_bases for b in self.bases)

    def __len__(self) -> int:
        """Return the length of the sequence."""
        return len(self.bases)

    def len(self) -> int:
        """
        Return the length of the sequence.

        Aria equivalent:
            fn len(self) -> Int
              ensures result > 0
        """
        return len(self.bases)

    def has_ambiguous(self) -> bool:
        """Check if the sequence contains any ambiguous bases (N)."""
        return 'N' in self.bases

    def count_ambiguous(self) -> int:
        """Count the number of ambiguous bases."""
        return self.bases.count('N')

    def base_at(self, index: int) -> Optional[str]:
        """
        Get a specific base at an index.

        Aria equivalent:
            fn base_at(self, index: Int) -> Option<Char>
              requires index >= 0
        """
        if index < 0:
            raise ValueError("Index must be non-negative")
        if index >= len(self.bases):
            return None
        return self.bases[index]

    def subsequence(self, start: int, end: int) -> 'Sequence':
        """
        Get a subsequence (slice) of the sequence.

        Aria equivalent:
            fn subsequence(self, start: Int, end: Int) -> Result<Sequence, SequenceError>
              requires start >= 0
              requires end > start
              requires end <= self.len()
              ensures result.is_ok() implies result.unwrap().len() == end - start
        """
        if start < 0:
            raise ValueError("Start index must be non-negative")
        if end <= start:
            raise ValueError("End must be greater than start")
        if end > len(self.bases):
            raise ValueError("End must not exceed sequence length")

        return Sequence(
            bases=self.bases[start:end],
            id=self.id,
            description=self.description,
            seq_type=self.seq_type
        )

    @staticmethod
    def complement_base(c: str) -> str:
        """
        Return the complement of a DNA base.

        Aria equivalent:
            fn complement_base(c: Char) -> Char
              requires c == 'A' or c == 'T' or c == 'C' or c == 'G' or c == 'N'
              ensures Self::is_valid_dna_base(result)
        """
        comp_map = {'A': 'T', 'T': 'A', 'C': 'G', 'G': 'C', 'N': 'N'}
        return comp_map.get(c, 'N')

    def complement(self) -> 'Sequence':
        """
        Return the complement of the sequence (A<->T, C<->G).

        Aria equivalent:
            fn complement(self) -> Sequence
              requires self.seq_type == SequenceType::DNA
              requires self.is_valid()
              ensures result.is_valid()
              ensures result.len() == self.len()
        """
        if self.seq_type != SequenceType.DNA:
            raise ValueError("Complement only available for DNA sequences")

        comp_bases = ''.join(self.complement_base(b) for b in self.bases)
        return Sequence(
            bases=comp_bases,
            id=self.id,
            description=self.description,
            seq_type=self.seq_type,
            _validated=True  # Skip validation since we know it's valid
        )

    def reverse(self) -> 'Sequence':
        """
        Return the reverse of the sequence.

        Aria equivalent:
            fn reverse(self) -> Sequence
              ensures result.len() == self.len()
              ensures result.is_valid()
        """
        return Sequence(
            bases=self.bases[::-1],
            id=self.id,
            description=self.description,
            seq_type=self.seq_type,
            _validated=True
        )

    def reverse_complement(self) -> 'Sequence':
        """
        Return the reverse complement of the sequence.

        Aria equivalent:
            fn reverse_complement(self) -> Sequence
              requires self.seq_type == SequenceType::DNA
              requires self.is_valid()
              ensures result.is_valid()
              ensures result.len() == self.len()
        """
        if self.seq_type != SequenceType.DNA:
            raise ValueError("Reverse complement only available for DNA sequences")

        return self.complement().reverse()

    def gc_content(self) -> float:
        """
        Calculate GC content (proportion of G and C bases).

        Aria equivalent:
            fn gc_content(self) -> Float
              requires self.is_valid()
              ensures result >= 0.0 and result <= 1.0

        In Aria, the contract guarantees the result is in [0, 1].
        In Python, we rely on the algorithm being correct.
        """
        if len(self.bases) == 0:
            return 0.0
        gc_count = sum(1 for b in self.bases if b in 'GC')
        return gc_count / len(self.bases)

    def at_content(self) -> float:
        """
        Calculate AT content (proportion of A and T bases).

        Aria equivalent:
            fn at_content(self) -> Float
              requires self.is_valid()
              requires self.seq_type == SequenceType::DNA
              ensures result >= 0.0 and result <= 1.0
        """
        if self.seq_type != SequenceType.DNA:
            raise ValueError("AT content only available for DNA sequences")

        if len(self.bases) == 0:
            return 0.0
        at_count = sum(1 for b in self.bases if b in 'AT')
        return at_count / len(self.bases)

    def base_counts(self) -> Tuple[int, int, int, int, int]:
        """
        Count occurrences of each base.

        Returns:
            Tuple of (A_count, C_count, G_count, T_count, N_count)

        Aria equivalent:
            fn base_counts(self) -> (Int, Int, Int, Int, Int)
              requires self.is_valid()
              ensures result.0 + result.1 + result.2 + result.3 + result.4 == self.len()
        """
        a_count = self.bases.count('A')
        c_count = self.bases.count('C')
        g_count = self.bases.count('G')
        # Count T for DNA, U for RNA (both counted as t_count)
        t_count = self.bases.count('T') + self.bases.count('U')
        n_count = self.bases.count('N')

        # In Aria, this postcondition is verified at compile time
        assert a_count + c_count + g_count + t_count + n_count == len(self.bases)

        return (a_count, c_count, g_count, t_count, n_count)

    def transcribe(self) -> 'Sequence':
        """
        Transcribe DNA to RNA (T -> U).

        Aria equivalent:
            fn transcribe(self) -> Sequence
              requires self.seq_type == SequenceType::DNA
              requires self.is_valid()
              ensures result.seq_type == SequenceType::RNA
              ensures result.len() == self.len()
        """
        if self.seq_type != SequenceType.DNA:
            raise ValueError("Can only transcribe DNA")

        rna_bases = self.bases.replace('T', 'U')
        return Sequence(
            bases=rna_bases,
            id=self.id,
            description=self.description,
            seq_type=SequenceType.RNA,
            _validated=True
        )

    def concat(self, other: 'Sequence') -> 'Sequence':
        """
        Concatenate two sequences.

        Aria equivalent:
            fn concat(self, other: Sequence) -> Sequence
              requires self.seq_type == other.seq_type
              ensures result.len() == self.len() + other.len()
        """
        if self.seq_type != other.seq_type:
            raise ValueError("Cannot concatenate different sequence types")

        return Sequence(
            bases=self.bases + other.bases,
            id=self.id,
            description=self.description,
            seq_type=self.seq_type,
            _validated=True
        )

    def contains_motif(self, motif: str) -> bool:
        """
        Check if this sequence contains a motif (substring).

        Aria equivalent:
            fn contains_motif(self, motif: String) -> Bool
              requires motif.len() > 0
              requires motif.len() <= self.len()
        """
        if len(motif) == 0:
            raise ValueError("Motif cannot be empty")
        if len(motif) > len(self.bases):
            raise ValueError("Motif cannot be longer than sequence")

        return motif.upper() in self.bases

    def find_motif_positions(self, motif: str) -> List[int]:
        """
        Find all positions where a motif occurs.

        Aria equivalent:
            fn find_motif_positions(self, motif: String) -> [Int]
              requires motif.len() > 0
              ensures result.all(|pos| pos >= 0 and pos < self.len())
        """
        if len(motif) == 0:
            raise ValueError("Motif cannot be empty")

        motif_upper = motif.upper()
        positions = []

        for i in range(len(self.bases) - len(motif_upper) + 1):
            if self.bases[i:i + len(motif_upper)] == motif_upper:
                positions.append(i)

        return positions

    def to_fasta(self) -> str:
        """Return the sequence in FASTA format."""
        if self.id:
            header = f">{self.id}"
            if self.description:
                header += f" {self.description}"
        else:
            header = ">sequence"

        # Split sequence into 80-character lines
        lines = [header]
        for i in range(0, len(self.bases), 80):
            lines.append(self.bases[i:i + 80])

        return '\n'.join(lines) + '\n'

    def __str__(self) -> str:
        """Return a string representation."""
        if self.id:
            return f">{self.id}\n{self.bases}"
        return self.bases

    def __eq__(self, other: object) -> bool:
        """Check equality with another sequence."""
        if not isinstance(other, Sequence):
            return False
        return self.bases == other.bases and self.seq_type == other.seq_type

    def __hash__(self) -> int:
        """Return hash for use in sets and dicts."""
        return hash((self.bases, self.seq_type))
