//! BioFlow - A production-quality bioinformatics library in Rust.
//!
//! This library provides efficient implementations of common bioinformatics
//! algorithms and data structures, including:
//!
//! - DNA/RNA sequence handling and validation
//! - K-mer counting and analysis
//! - Sequence alignment (Smith-Waterman, Needleman-Wunsch)
//! - Quality score handling for FASTQ data
//! - Statistical analysis utilities
//!
//! # Safety and Performance
//!
//! BioFlow leverages Rust's ownership model and borrow checker to provide
//! memory-safe operations with zero-cost abstractions. All sequence data
//! is validated at construction time, eliminating runtime checks during
//! analysis.
//!
//! # Example
//!
//! ```rust
//! use bioflow_rust::sequence::Sequence;
//! use bioflow_rust::kmer::KMerCounter;
//! use bioflow_rust::alignment::{smith_waterman, ScoringMatrix};
//!
//! // Create and validate a sequence
//! let seq = Sequence::new("ATGCGATCGATCGATCG").unwrap();
//!
//! // Calculate GC content
//! println!("GC content: {:.1}%", seq.gc_content() * 100.0);
//!
//! // Count k-mers
//! let mut counter = KMerCounter::new(3);
//! counter.count(&seq);
//! for (kmer, count) in counter.most_frequent(5) {
//!     println!("{}: {}", kmer, count);
//! }
//!
//! // Align sequences
//! let seq1 = Sequence::new("ACGT").unwrap();
//! let seq2 = Sequence::new("ACGT").unwrap();
//! let alignment = smith_waterman(&seq1, &seq2, &ScoringMatrix::default());
//! println!("Alignment score: {}", alignment.score);
//! ```
//!
//! # Comparison with Aria
//!
//! This library is designed to showcase Rust's safety features for comparison
//! with the Aria programming language. While both languages provide:
//!
//! - Compile-time memory safety
//! - Zero-cost abstractions
//! - Strong type systems
//!
//! Aria additionally provides built-in design-by-contract with `requires`,
//! `ensures`, and `invariant` keywords for formal verification.

pub mod alignment;
pub mod kmer;
pub mod quality;
pub mod sequence;
pub mod stats;

// Re-export commonly used types for convenience
pub use alignment::{
    edit_distance, hamming_distance, needleman_wunsch, semi_global_alignment, smith_waterman,
    Alignment, AlignmentType, ScoringMatrix,
};
pub use kmer::{generate_all_kmers, jaccard_similarity, CanonicalKMerCounter, KMerCounter};
pub use quality::{QualityEncoding, QualityError, QualityScores, QualityStats};
pub use sequence::{BaseComposition, Sequence, SequenceError, SequenceType};
pub use stats::{
    gc_content, l50, n50, pearson_correlation, shannon_entropy, Histogram, RunningStats,
    SummaryStats,
};

/// Library version information.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Returns the library name and version.
pub fn version_string() -> String {
    format!("BioFlow Rust v{}", VERSION)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
        assert!(version_string().contains("BioFlow"));
    }

    #[test]
    fn test_integration_workflow() {
        // Create sequences
        let seq1 = Sequence::new("ATGCGATCGATCGATCG").unwrap();
        let seq2 = Sequence::new("ATGCGATCGATCGATCG").unwrap();

        // Check basic properties
        assert!(seq1.gc_content() > 0.4);

        // Count k-mers
        let mut counter = KMerCounter::new(3);
        counter.count(&seq1);
        assert!(counter.total_kmers() > 0);

        // Align sequences
        let alignment = smith_waterman(&seq1, &seq2, &ScoringMatrix::default());
        assert!(alignment.score > 0);
        assert!(alignment.identity() > 0.9);
    }
}
