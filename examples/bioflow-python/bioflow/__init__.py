"""
BioFlow - Genomic Data Processing Pipeline (Python Implementation)

A Python port of the Aria BioFlow library for comparison purposes.
Demonstrates the differences between Aria's compile-time guarantees
and Python's runtime-only checking.

Modules:
    - sequence: DNA/RNA sequence operations
    - kmer: K-mer counting and analysis
    - alignment: Smith-Waterman and Needleman-Wunsch alignment
    - quality: Phred quality score management
    - stats: Sequence and read statistics
"""

from .sequence import Sequence, SequenceType, SequenceError
from .kmer import KMerCounter, KMer
from .alignment import smith_waterman, needleman_wunsch, ScoringMatrix
from .quality import QualityScores, QualityCategory, QualityError
from .stats import SequenceStats, SequenceSetStats, ReadSetStats

__version__ = "0.1.0"
__all__ = [
    "Sequence",
    "SequenceType",
    "SequenceError",
    "KMerCounter",
    "KMer",
    "smith_waterman",
    "needleman_wunsch",
    "ScoringMatrix",
    "QualityScores",
    "QualityCategory",
    "QualityError",
    "SequenceStats",
    "SequenceSetStats",
    "ReadSetStats",
]
