"""
BioFlow - Sequence Alignment Algorithms

Smith-Waterman and Needleman-Wunsch alignment implementations.

Comparison with Aria:

Aria provides compile-time guarantees:
    fn smith_waterman(seq1: Sequence, seq2: Sequence, scoring: ScoringMatrix) -> Alignment
      requires seq1.is_valid() and seq2.is_valid()
      requires seq1.len() > 0 and seq2.len() > 0
      ensures result.score >= 0
      ensures result.aligned_seq1.len() == result.aligned_seq2.len()

Python relies on assertions and runtime checks:
    def smith_waterman(...) -> Alignment:
        assert len(seq1) > 0 and len(seq2) > 0
        # ... algorithm ...
        assert len(aligned1) == len(aligned2)  # Runtime only
"""

from typing import Tuple, List, Optional
from dataclasses import dataclass, field
from enum import Enum

from .sequence import Sequence


class AlignDirection(Enum):
    """Alignment direction for traceback."""
    DIAGONAL = 1  # Match or mismatch
    UP = 2        # Gap in sequence 2
    LEFT = 3      # Gap in sequence 1
    STOP = 0      # End of alignment (local only)


class AlignmentType(Enum):
    """Type of alignment."""
    GLOBAL = "global"       # Needleman-Wunsch style
    LOCAL = "local"         # Smith-Waterman style
    SEMI_GLOBAL = "semi"    # Hybrid approach


@dataclass
class ScoringMatrix:
    """
    Scoring matrix for nucleotide alignment.

    Aria equivalent:
        struct ScoringMatrix
          match_score: Int
          mismatch_penalty: Int
          gap_open_penalty: Int
          gap_extend_penalty: Int
          invariant self.match_score > 0
          invariant self.mismatch_penalty <= 0
          invariant self.gap_open_penalty <= 0
          invariant self.gap_extend_penalty <= 0
    """
    match_score: int = 2
    mismatch_penalty: int = -1
    gap_open_penalty: int = -2
    gap_extend_penalty: int = -1

    def __post_init__(self):
        """Validate scoring parameters."""
        if self.match_score <= 0:
            raise ValueError("Match score must be positive")
        if self.mismatch_penalty > 0:
            raise ValueError("Mismatch penalty should be <= 0")
        if self.gap_open_penalty > 0:
            raise ValueError("Gap open penalty should be <= 0")
        if self.gap_extend_penalty > 0:
            raise ValueError("Gap extend penalty should be <= 0")

    @classmethod
    def default_dna(cls) -> 'ScoringMatrix':
        """Create default DNA scoring matrix."""
        return cls(
            match_score=2,
            mismatch_penalty=-1,
            gap_open_penalty=-2,
            gap_extend_penalty=-1
        )

    @classmethod
    def blast_like(cls) -> 'ScoringMatrix':
        """Create BLAST-like scoring matrix."""
        return cls(
            match_score=1,
            mismatch_penalty=-3,
            gap_open_penalty=-5,
            gap_extend_penalty=-2
        )

    @classmethod
    def simple(cls, match: int, mismatch: int, gap: int) -> 'ScoringMatrix':
        """Create simple scoring matrix (no affine gaps)."""
        return cls(
            match_score=match,
            mismatch_penalty=mismatch,
            gap_open_penalty=gap,
            gap_extend_penalty=gap
        )

    def score(self, base1: str, base2: str) -> int:
        """Return score for comparing two bases."""
        if base1 == base2:
            return self.match_score
        return self.mismatch_penalty

    def gap_penalty(self) -> int:
        """Return gap penalty (linear model)."""
        return self.gap_open_penalty


@dataclass
class Alignment:
    """
    Alignment result between two sequences.

    Aria equivalent:
        struct Alignment
          aligned_seq1: String
          aligned_seq2: String
          score: Int
          start1: Int
          end1: Int
          start2: Int
          end2: Int
          alignment_type: AlignmentType
          identity: Float
          invariant self.aligned_seq1.len() == self.aligned_seq2.len()
          invariant self.identity >= 0.0 and self.identity <= 1.0
    """
    aligned_seq1: str
    aligned_seq2: str
    score: int
    start1: int = 0
    end1: int = 0
    start2: int = 0
    end2: int = 0
    alignment_type: AlignmentType = AlignmentType.LOCAL
    identity: float = field(init=False)

    def __post_init__(self):
        """Validate alignment and calculate identity."""
        if len(self.aligned_seq1) != len(self.aligned_seq2):
            raise ValueError("Aligned sequences must have equal length")

        self.identity = self._calculate_identity()

    def _calculate_identity(self) -> float:
        """Calculate sequence identity."""
        if len(self.aligned_seq1) == 0:
            return 0.0

        matches = sum(
            1 for a, b in zip(self.aligned_seq1, self.aligned_seq2)
            if a == b and a != '-'
        )
        return matches / len(self.aligned_seq1)

    @classmethod
    def new(
        cls,
        aligned_seq1: str,
        aligned_seq2: str,
        score: int,
        alignment_type: AlignmentType
    ) -> 'Alignment':
        """Create a new alignment result."""
        return cls(
            aligned_seq1=aligned_seq1,
            aligned_seq2=aligned_seq2,
            score=score,
            start1=0,
            end1=len(aligned_seq1),
            start2=0,
            end2=len(aligned_seq2),
            alignment_type=alignment_type
        )

    @classmethod
    def with_positions(
        cls,
        aligned_seq1: str,
        aligned_seq2: str,
        score: int,
        start1: int,
        end1: int,
        start2: int,
        end2: int,
        alignment_type: AlignmentType
    ) -> 'Alignment':
        """Create alignment with position information."""
        return cls(
            aligned_seq1=aligned_seq1,
            aligned_seq2=aligned_seq2,
            score=score,
            start1=start1,
            end1=end1,
            start2=start2,
            end2=end2,
            alignment_type=alignment_type
        )

    def alignment_length(self) -> int:
        """Return the length of the alignment."""
        return len(self.aligned_seq1)

    def match_count(self) -> int:
        """Count the number of matches."""
        return sum(
            1 for a, b in zip(self.aligned_seq1, self.aligned_seq2)
            if a == b and a != '-'
        )

    def mismatch_count(self) -> int:
        """Count the number of mismatches."""
        return sum(
            1 for a, b in zip(self.aligned_seq1, self.aligned_seq2)
            if a != b and a != '-' and b != '-'
        )

    def gaps_seq1(self) -> int:
        """Count gaps in sequence 1."""
        return self.aligned_seq1.count('-')

    def gaps_seq2(self) -> int:
        """Count gaps in sequence 2."""
        return self.aligned_seq2.count('-')

    def total_gaps(self) -> int:
        """Return total number of gaps."""
        return self.gaps_seq1() + self.gaps_seq2()

    def gap_openings(self) -> int:
        """Count gap openings (start of gap regions)."""
        openings = 0
        in_gap1 = False
        in_gap2 = False

        for a, b in zip(self.aligned_seq1, self.aligned_seq2):
            if a == '-' and not in_gap1:
                openings += 1
                in_gap1 = True
            elif a != '-':
                in_gap1 = False

            if b == '-' and not in_gap2:
                openings += 1
                in_gap2 = True
            elif b != '-':
                in_gap2 = False

        return openings

    def to_cigar(self) -> str:
        """Generate CIGAR string representation."""
        if len(self.aligned_seq1) == 0:
            return ""

        cigar = ""
        current_op = ""
        count = 0

        for a, b in zip(self.aligned_seq1, self.aligned_seq2):
            if a == '-':
                op = 'I'  # Insertion
            elif b == '-':
                op = 'D'  # Deletion
            elif a == b:
                op = 'M'  # Match
            else:
                op = 'X'  # Mismatch

            if op == current_op:
                count += 1
            else:
                if count > 0:
                    cigar += f"{count}{current_op}"
                current_op = op
                count = 1

        if count > 0:
            cigar += f"{count}{current_op}"

        return cigar

    def format(self) -> str:
        """Format alignment for display."""
        match_line = ""
        for a, b in zip(self.aligned_seq1, self.aligned_seq2):
            if a == b and a != '-':
                match_line += '|'
            elif a == '-' or b == '-':
                match_line += ' '
            else:
                match_line += '.'

        lines = [
            f"Seq1: {self.aligned_seq1}",
            f"      {match_line}",
            f"Seq2: {self.aligned_seq2}",
            f"Score: {self.score}",
            f"Identity: {self.identity * 100:.1f}%",
            f"CIGAR: {self.to_cigar()}"
        ]
        return '\n'.join(lines)

    def __str__(self) -> str:
        return f"Alignment {{ score: {self.score}, identity: {self.identity * 100:.1f}%, length: {self.alignment_length()} }}"


def smith_waterman(
    seq1: Sequence,
    seq2: Sequence,
    scoring: Optional[ScoringMatrix] = None
) -> Alignment:
    """
    Smith-Waterman local alignment algorithm.

    Finds the optimal local alignment between two sequences.

    Aria equivalent:
        fn smith_waterman(seq1: Sequence, seq2: Sequence, scoring: ScoringMatrix) -> Alignment
          requires seq1.is_valid() and seq2.is_valid()
          requires seq1.len() > 0 and seq2.len() > 0
          ensures result.score >= 0
          ensures result.aligned_seq1.len() == result.aligned_seq2.len()

    Args:
        seq1: First sequence
        seq2: Second sequence
        scoring: Scoring matrix (default: default_dna)

    Returns:
        Alignment result
    """
    if scoring is None:
        scoring = ScoringMatrix.default_dna()

    # Runtime validation (in Aria, these are compile-time contracts)
    if len(seq1) == 0 or len(seq2) == 0:
        raise ValueError("Sequences must be non-empty")

    m, n = len(seq1), len(seq2)
    s1, s2 = seq1.bases, seq2.bases

    # Initialize scoring matrix with zeros
    H = [[0] * (n + 1) for _ in range(m + 1)]

    # Initialize traceback matrix
    traceback = [[AlignDirection.STOP] * (n + 1) for _ in range(m + 1)]

    # Track maximum score and position
    max_score = 0
    max_i, max_j = 0, 0

    # Fill matrices
    for i in range(1, m + 1):
        for j in range(1, n + 1):
            match_score = scoring.score(s1[i - 1], s2[j - 1])

            # Calculate scores for each direction
            diag = H[i - 1][j - 1] + match_score
            up = H[i - 1][j] + scoring.gap_penalty()
            left = H[i][j - 1] + scoring.gap_penalty()

            # Find maximum (including 0 for local alignment)
            best = 0
            direction = AlignDirection.STOP

            if diag > best:
                best = diag
                direction = AlignDirection.DIAGONAL

            if up > best:
                best = up
                direction = AlignDirection.UP

            if left > best:
                best = left
                direction = AlignDirection.LEFT

            H[i][j] = best
            traceback[i][j] = direction

            # Update maximum
            if best > max_score:
                max_score = best
                max_i, max_j = i, j

    # Traceback
    aligned1, aligned2, start1, start2 = _traceback_local(
        s1, s2, traceback, max_i, max_j
    )

    return Alignment.with_positions(
        aligned1, aligned2, max_score,
        start1, max_i, start2, max_j,
        AlignmentType.LOCAL
    )


def _traceback_local(
    seq1: str,
    seq2: str,
    traceback: List[List[AlignDirection]],
    start_i: int,
    start_j: int
) -> Tuple[str, str, int, int]:
    """Perform traceback for local alignment."""
    aligned1 = ""
    aligned2 = ""
    i, j = start_i, start_j

    while i > 0 and j > 0:
        direction = traceback[i][j]

        if direction == AlignDirection.STOP:
            break
        elif direction == AlignDirection.DIAGONAL:
            aligned1 = seq1[i - 1] + aligned1
            aligned2 = seq2[j - 1] + aligned2
            i -= 1
            j -= 1
        elif direction == AlignDirection.UP:
            aligned1 = seq1[i - 1] + aligned1
            aligned2 = '-' + aligned2
            i -= 1
        elif direction == AlignDirection.LEFT:
            aligned1 = '-' + aligned1
            aligned2 = seq2[j - 1] + aligned2
            j -= 1

    return aligned1, aligned2, i, j


def needleman_wunsch(
    seq1: Sequence,
    seq2: Sequence,
    scoring: Optional[ScoringMatrix] = None
) -> Alignment:
    """
    Needleman-Wunsch global alignment algorithm.

    Aligns the entire length of both sequences.

    Aria equivalent:
        fn needleman_wunsch(seq1: Sequence, seq2: Sequence, scoring: ScoringMatrix) -> Alignment
          requires seq1.is_valid() and seq2.is_valid()
          requires seq1.len() > 0 and seq2.len() > 0
          ensures result.aligned_seq1.len() == result.aligned_seq2.len()

    Args:
        seq1: First sequence
        seq2: Second sequence
        scoring: Scoring matrix (default: default_dna)

    Returns:
        Alignment result
    """
    if scoring is None:
        scoring = ScoringMatrix.default_dna()

    if len(seq1) == 0 or len(seq2) == 0:
        raise ValueError("Sequences must be non-empty")

    m, n = len(seq1), len(seq2)
    s1, s2 = seq1.bases, seq2.bases

    # Initialize scoring matrix with gap penalties
    H = [[0] * (n + 1) for _ in range(m + 1)]

    # First row and column initialized with gap penalties
    for i in range(m + 1):
        H[i][0] = i * scoring.gap_penalty()
    for j in range(n + 1):
        H[0][j] = j * scoring.gap_penalty()

    # Initialize traceback matrix
    traceback = [[AlignDirection.STOP] * (n + 1) for _ in range(m + 1)]
    for i in range(1, m + 1):
        traceback[i][0] = AlignDirection.UP
    for j in range(1, n + 1):
        traceback[0][j] = AlignDirection.LEFT

    # Fill matrices
    for i in range(1, m + 1):
        for j in range(1, n + 1):
            match_score = scoring.score(s1[i - 1], s2[j - 1])

            diag = H[i - 1][j - 1] + match_score
            up = H[i - 1][j] + scoring.gap_penalty()
            left = H[i][j - 1] + scoring.gap_penalty()

            # Find maximum (no zero threshold for global)
            best = diag
            direction = AlignDirection.DIAGONAL

            if up > best:
                best = up
                direction = AlignDirection.UP

            if left > best:
                best = left
                direction = AlignDirection.LEFT

            H[i][j] = best
            traceback[i][j] = direction

    # Traceback from bottom-right corner
    aligned1, aligned2 = _traceback_global(s1, s2, traceback, m, n)

    return Alignment.new(aligned1, aligned2, H[m][n], AlignmentType.GLOBAL)


def _traceback_global(
    seq1: str,
    seq2: str,
    traceback: List[List[AlignDirection]],
    m: int,
    n: int
) -> Tuple[str, str]:
    """Perform traceback for global alignment."""
    aligned1 = ""
    aligned2 = ""
    i, j = m, n

    while i > 0 or j > 0:
        if i == 0:
            aligned1 = '-' + aligned1
            aligned2 = seq2[j - 1] + aligned2
            j -= 1
        elif j == 0:
            aligned1 = seq1[i - 1] + aligned1
            aligned2 = '-' + aligned2
            i -= 1
        else:
            direction = traceback[i][j]

            if direction == AlignDirection.DIAGONAL:
                aligned1 = seq1[i - 1] + aligned1
                aligned2 = seq2[j - 1] + aligned2
                i -= 1
                j -= 1
            elif direction == AlignDirection.UP:
                aligned1 = seq1[i - 1] + aligned1
                aligned2 = '-' + aligned2
                i -= 1
            elif direction == AlignDirection.LEFT:
                aligned1 = '-' + aligned1
                aligned2 = seq2[j - 1] + aligned2
                j -= 1
            else:
                break

    return aligned1, aligned2


def simple_align(seq1: Sequence, seq2: Sequence) -> Alignment:
    """
    Simple alignment using default settings.

    Aria equivalent:
        fn simple_align(seq1: Sequence, seq2: Sequence) -> Alignment
          requires seq1.is_valid() and seq2.is_valid()
          requires seq1.len() > 0 and seq2.len() > 0
    """
    return smith_waterman(seq1, seq2, ScoringMatrix.default_dna())


def alignment_score_only(
    seq1: Sequence,
    seq2: Sequence,
    scoring: Optional[ScoringMatrix] = None
) -> int:
    """
    Calculate alignment score without full traceback (memory efficient).

    Uses O(n) space instead of O(m*n) by only keeping two rows.

    Aria equivalent:
        fn alignment_score_only(seq1: Sequence, seq2: Sequence, scoring: ScoringMatrix) -> Int
          requires seq1.is_valid() and seq2.is_valid()
          requires seq1.len() > 0 and seq2.len() > 0
    """
    if scoring is None:
        scoring = ScoringMatrix.default_dna()

    if len(seq1) == 0 or len(seq2) == 0:
        raise ValueError("Sequences must be non-empty")

    m, n = len(seq1), len(seq2)
    s1, s2 = seq1.bases, seq2.bases

    # Use two rows instead of full matrix
    prev_row = [0] * (n + 1)
    curr_row = [0] * (n + 1)

    max_score = 0

    for i in range(1, m + 1):
        # Reset current row
        curr_row = [0] * (n + 1)

        for j in range(1, n + 1):
            match_score = scoring.score(s1[i - 1], s2[j - 1])

            diag = prev_row[j - 1] + match_score
            up = prev_row[j] + scoring.gap_penalty()
            left = curr_row[j - 1] + scoring.gap_penalty()

            best = max(0, diag, up, left)
            curr_row[j] = best

            if best > max_score:
                max_score = best

        # Swap rows
        prev_row, curr_row = curr_row, prev_row

    return max_score


def percent_identity(aligned1: str, aligned2: str) -> float:
    """
    Calculate percent identity between two aligned sequences.

    Aria equivalent:
        fn percent_identity(aligned1: String, aligned2: String) -> Float
          requires aligned1.len() == aligned2.len()
          requires aligned1.len() > 0
          ensures result >= 0.0 and result <= 100.0
    """
    if len(aligned1) != len(aligned2):
        raise ValueError("Aligned sequences must have equal length")
    if len(aligned1) == 0:
        raise ValueError("Aligned sequences cannot be empty")

    matches = sum(
        1 for a, b in zip(aligned1, aligned2)
        if a == b and a != '-'
    )

    return (matches / len(aligned1)) * 100.0


def align_against_multiple(
    query: Sequence,
    targets: List[Sequence],
    scoring: Optional[ScoringMatrix] = None
) -> List[Tuple[int, Alignment]]:
    """
    Align a sequence against multiple targets.

    Aria equivalent:
        fn align_against_multiple(query: Sequence, targets: [Sequence], scoring: ScoringMatrix)
          -> [(Int, Alignment)]
          requires query.is_valid()
          requires targets.len() > 0
          ensures result.len() == targets.len()
    """
    if scoring is None:
        scoring = ScoringMatrix.default_dna()

    if len(targets) == 0:
        raise ValueError("Target list cannot be empty")

    results = []
    for i, target in enumerate(targets):
        alignment = smith_waterman(query, target, scoring)
        results.append((i, alignment))

    return results


def find_best_alignment(
    query: Sequence,
    targets: List[Sequence],
    scoring: Optional[ScoringMatrix] = None
) -> Optional[Tuple[int, Alignment]]:
    """
    Find the best alignment among multiple targets.

    Aria equivalent:
        fn find_best_alignment(query: Sequence, targets: [Sequence], scoring: ScoringMatrix)
          -> Option<(Int, Alignment)>
          requires query.is_valid()
          requires targets.len() > 0
    """
    alignments = align_against_multiple(query, targets, scoring)

    if not alignments:
        return None

    return max(alignments, key=lambda x: x[1].score)
