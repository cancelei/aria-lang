"""
BioFlow - Quality Scores

Phred quality scores for sequencing reads.

Phred quality scores are logarithmically related to base-calling error probabilities:
    Q = -10 * log10(P_error)

Common thresholds:
    Q10 = 90% accuracy
    Q20 = 99% accuracy
    Q30 = 99.9% accuracy (typical threshold for "high quality")
    Q40 = 99.99% accuracy

Comparison with Aria:

Aria uses invariants for compile-time guarantees:
    struct QualityScores
      invariant self.scores.len() > 0
      invariant self.all_in_range()

Python uses runtime checks:
    def __post_init__(self):
        if len(self.scores) == 0:
            raise QualityError(...)
"""

from typing import List, Optional, Tuple
from dataclasses import dataclass, field
from enum import Enum
import math


# Phred score bounds
PHRED_MIN = 0
PHRED_MAX = 40

# Common quality thresholds
Q_LOW = 10      # 90% accuracy
Q_MEDIUM = 20   # 99% accuracy
Q_HIGH = 30     # 99.9% accuracy
Q_EXCELLENT = 40  # 99.99% accuracy


class QualityError(Exception):
    """Error types for quality operations."""
    pass


class EmptyScoresError(QualityError):
    """Raised when quality scores are empty."""
    pass


class ScoreOutOfRangeError(QualityError):
    """Raised when a score is out of valid range."""
    def __init__(self, position: int, score: int):
        self.position = position
        self.score = score
        super().__init__(f"Score {score} at position {position} is out of range [0, 40]")


class InvalidEncodingError(QualityError):
    """Raised when quality encoding character is invalid."""
    def __init__(self, char: str):
        self.char = char
        super().__init__(f"Invalid encoding character: '{char}'")


class QualityCategory(Enum):
    """Quality category enumeration."""
    POOR = "Poor"           # Q < 10
    LOW = "Low"             # 10 <= Q < 20
    MEDIUM = "Medium"       # 20 <= Q < 30
    HIGH = "High"           # 30 <= Q < 40
    EXCELLENT = "Excellent"  # Q >= 40


@dataclass
class QualityScores:
    """
    Quality scores for a sequencing read.

    Each score corresponds to a base in a sequence.

    Aria equivalent:
        struct QualityScores
          scores: [Int]
          invariant self.scores.len() > 0
          invariant self.all_in_range()
    """
    scores: List[int]

    def __post_init__(self):
        """Validate quality scores on construction."""
        if len(self.scores) == 0:
            raise EmptyScoresError("Quality scores cannot be empty")

        for i, score in enumerate(self.scores):
            if score < PHRED_MIN or score > PHRED_MAX:
                raise ScoreOutOfRangeError(i, score)

    @classmethod
    def new(cls, scores: List[int]) -> 'QualityScores':
        """
        Create new quality scores from an array of integers.

        Aria equivalent:
            fn new(scores: [Int]) -> Result<QualityScores, QualityError>
              requires scores.len() > 0
              ensures result.is_ok() implies result.unwrap().len() == scores.len()
        """
        return cls(scores=list(scores))

    @classmethod
    def from_phred33(cls, encoded: str) -> 'QualityScores':
        """
        Create quality scores from a Phred+33 encoded string (Illumina 1.8+).

        Each ASCII character maps to a quality score: Q = ord(char) - 33

        Aria equivalent:
            fn from_phred33(encoded: String) -> Result<QualityScores, QualityError>
              requires encoded.len() > 0
              ensures result.is_ok() implies result.unwrap().len() == encoded.len()
        """
        if len(encoded) == 0:
            raise EmptyScoresError("Encoded string cannot be empty")

        scores = []
        for i, c in enumerate(encoded):
            ascii_val = ord(c)

            # Phred+33 encoding: valid range is '!' (33) to 'J' (74) for Q0-Q41
            if ascii_val < 33 or ascii_val > 74:
                raise InvalidEncodingError(c)

            score = ascii_val - 33
            if score > PHRED_MAX:
                raise ScoreOutOfRangeError(i, score)

            scores.append(score)

        return cls(scores=scores)

    @classmethod
    def from_phred64(cls, encoded: str) -> 'QualityScores':
        """
        Create quality scores from a Phred+64 encoded string (older Illumina).

        Each ASCII character maps to a quality score: Q = ord(char) - 64

        Aria equivalent:
            fn from_phred64(encoded: String) -> Result<QualityScores, QualityError>
              requires encoded.len() > 0
              ensures result.is_ok() implies result.unwrap().len() == encoded.len()
        """
        if len(encoded) == 0:
            raise EmptyScoresError("Encoded string cannot be empty")

        scores = []
        for i, c in enumerate(encoded):
            ascii_val = ord(c)

            # Phred+64 encoding: valid range is '@' (64) to 'h' (104) for Q0-Q40
            if ascii_val < 64 or ascii_val > 104:
                raise InvalidEncodingError(c)

            score = ascii_val - 64
            if score > PHRED_MAX:
                raise ScoreOutOfRangeError(i, score)

            scores.append(score)

        return cls(scores=scores)

    def all_in_range(self) -> bool:
        """Check if all scores are within the valid Phred range."""
        return all(PHRED_MIN <= s <= PHRED_MAX for s in self.scores)

    def __len__(self) -> int:
        """Return the number of quality scores."""
        return len(self.scores)

    def len(self) -> int:
        """Return the number of quality scores."""
        return len(self.scores)

    def score_at(self, index: int) -> Optional[int]:
        """
        Get the quality score at a specific position.

        Aria equivalent:
            fn score_at(self, index: Int) -> Option<Int>
              requires index >= 0
        """
        if index < 0:
            raise ValueError("Index must be non-negative")
        if index >= len(self.scores):
            return None
        return self.scores[index]

    def average(self) -> float:
        """
        Calculate the average quality score.

        Aria equivalent:
            fn average(self) -> Float
              requires self.scores.len() > 0
              ensures result >= 0.0 and result <= PHRED_MAX.to_float()
        """
        return sum(self.scores) / len(self.scores)

    def median(self) -> int:
        """
        Calculate the median quality score.

        Aria equivalent:
            fn median(self) -> Int
              requires self.scores.len() > 0
              ensures result >= PHRED_MIN and result <= PHRED_MAX
        """
        sorted_scores = sorted(self.scores)
        mid = len(sorted_scores) // 2

        if len(sorted_scores) % 2 == 0:
            return (sorted_scores[mid - 1] + sorted_scores[mid]) // 2
        return sorted_scores[mid]

    def min(self) -> int:
        """
        Return the minimum quality score.

        Aria equivalent:
            fn min(self) -> Int
              requires self.scores.len() > 0
              ensures result >= PHRED_MIN and result <= PHRED_MAX
        """
        return min(self.scores)

    def max(self) -> int:
        """
        Return the maximum quality score.

        Aria equivalent:
            fn max(self) -> Int
              requires self.scores.len() > 0
              ensures result >= PHRED_MIN and result <= PHRED_MAX
        """
        return max(self.scores)

    def count_above(self, threshold: int) -> int:
        """
        Count scores above a threshold.

        Aria equivalent:
            fn count_above(self, threshold: Int) -> Int
              requires threshold >= PHRED_MIN and threshold <= PHRED_MAX
              ensures result >= 0 and result <= self.len()
        """
        return sum(1 for s in self.scores if s > threshold)

    def count_at_or_above(self, threshold: int) -> int:
        """
        Count scores at or above a threshold.

        Aria equivalent:
            fn count_at_or_above(self, threshold: Int) -> Int
              requires threshold >= PHRED_MIN and threshold <= PHRED_MAX
              ensures result >= 0 and result <= self.len()
        """
        return sum(1 for s in self.scores if s >= threshold)

    def high_quality_ratio(self) -> float:
        """
        Calculate the proportion of high-quality bases (Q >= 30).

        Aria equivalent:
            fn high_quality_ratio(self) -> Float
              ensures result >= 0.0 and result <= 1.0
        """
        return self.count_at_or_above(Q_HIGH) / len(self.scores)

    def categorize(self) -> QualityCategory:
        """
        Categorize the overall quality of the read.

        Returns quality category based on average score.
        """
        avg = self.average()

        if avg >= Q_EXCELLENT:
            return QualityCategory.EXCELLENT
        elif avg >= Q_HIGH:
            return QualityCategory.HIGH
        elif avg >= Q_MEDIUM:
            return QualityCategory.MEDIUM
        elif avg >= Q_LOW:
            return QualityCategory.LOW
        else:
            return QualityCategory.POOR

    @staticmethod
    def score_to_probability(score: int) -> float:
        """
        Convert a Phred score to error probability.

        P_error = 10^(-Q/10)

        Aria equivalent:
            fn score_to_probability(score: Int) -> Float
              requires score >= PHRED_MIN and score <= PHRED_MAX
              ensures result >= 0.0 and result <= 1.0
        """
        if score < PHRED_MIN or score > PHRED_MAX:
            raise ValueError(f"Score {score} out of range [{PHRED_MIN}, {PHRED_MAX}]")
        return 10.0 ** (-score / 10.0)

    @staticmethod
    def probability_to_score(prob: float) -> int:
        """
        Convert an error probability to Phred score.

        Q = -10 * log10(P_error)

        Aria equivalent:
            fn probability_to_score(prob: Float) -> Int
              requires prob > 0.0 and prob <= 1.0
              ensures result >= PHRED_MIN
        """
        if prob <= 0.0 or prob > 1.0:
            raise ValueError(f"Probability {prob} must be in (0, 1]")

        q = -10.0 * math.log10(prob)

        # Clamp to valid range
        if q < PHRED_MIN:
            return PHRED_MIN
        elif q > PHRED_MAX:
            return PHRED_MAX
        return round(q)

    def slice(self, start: int, end: int) -> 'QualityScores':
        """
        Return a subsequence of quality scores.

        Aria equivalent:
            fn slice(self, start: Int, end: Int) -> Result<QualityScores, QualityError>
              requires start >= 0
              requires end > start
              requires end <= self.len()
              ensures result.is_ok() implies result.unwrap().len() == end - start
        """
        if start < 0:
            raise ValueError("Start index must be non-negative")
        if end <= start:
            raise ValueError("End must be greater than start")
        if end > len(self.scores):
            raise ValueError("End must not exceed length")

        return QualityScores(scores=self.scores[start:end])

    def to_phred33(self) -> str:
        """
        Encode quality scores to Phred+33 format.

        Aria equivalent:
            fn to_phred33(self) -> String
              ensures result.len() == self.len()
        """
        return ''.join(chr(s + 33) for s in self.scores)

    def to_phred64(self) -> str:
        """
        Encode quality scores to Phred+64 format.

        Aria equivalent:
            fn to_phred64(self) -> String
              ensures result.len() == self.len()
        """
        return ''.join(chr(s + 64) for s in self.scores)

    def low_quality_positions(self, threshold: int) -> List[int]:
        """
        Find positions of low-quality bases.

        Aria equivalent:
            fn low_quality_positions(self, threshold: Int) -> [Int]
              requires threshold >= PHRED_MIN and threshold <= PHRED_MAX
              ensures result.all(|pos| pos >= 0 and pos < self.len())
        """
        return [i for i, s in enumerate(self.scores) if s < threshold]

    def statistics(self) -> 'QualityStats':
        """Calculate quality statistics."""
        return QualityStats(
            count=len(self.scores),
            min_score=self.min(),
            max_score=self.max(),
            mean=self.average(),
            median=self.median(),
            high_quality_ratio=self.high_quality_ratio(),
            category=self.categorize()
        )

    def __str__(self) -> str:
        return f"QualityScores {{ len: {len(self.scores)}, avg: {self.average():.1f} }}"


@dataclass
class QualityStats:
    """Quality statistics summary."""
    count: int
    min_score: int
    max_score: int
    mean: float
    median: int
    high_quality_ratio: float
    category: QualityCategory

    def __str__(self) -> str:
        return (
            f"QualityStats {{ count: {self.count}, "
            f"min: {self.min_score}, max: {self.max_score}, "
            f"mean: {self.mean:.2f}, median: {self.median}, "
            f"high_quality_ratio: {self.high_quality_ratio:.2%} }}"
        )
