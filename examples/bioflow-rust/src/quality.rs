//! Quality score handling for sequencing data.
//!
//! This module provides support for quality scores commonly used in
//! sequencing data formats like FASTQ.

use std::fmt;
use thiserror::Error;

/// Errors that can occur when working with quality scores.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum QualityError {
    /// The quality string is empty.
    #[error("Empty quality string")]
    EmptyQuality,

    /// Invalid quality character.
    #[error("Invalid quality character '{char}' at position {position}")]
    InvalidCharacter { position: usize, char: char },

    /// Quality and sequence lengths don't match.
    #[error("Quality length ({quality_len}) doesn't match sequence length ({sequence_len})")]
    LengthMismatch { quality_len: usize, sequence_len: usize },
}

/// Quality encoding schemes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualityEncoding {
    /// Phred+33 encoding (Sanger, Illumina 1.8+).
    /// Quality scores range from 0-93, encoded as ASCII 33-126.
    Phred33,

    /// Phred+64 encoding (Illumina 1.3-1.7).
    /// Quality scores range from 0-62, encoded as ASCII 64-126.
    Phred64,

    /// Solexa/Illumina 1.0 encoding.
    /// Quality scores range from -5 to 62, encoded as ASCII 59-126.
    Solexa,
}

impl QualityEncoding {
    /// Returns the ASCII offset for this encoding.
    pub fn offset(&self) -> u8 {
        match self {
            QualityEncoding::Phred33 => 33,
            QualityEncoding::Phred64 => 64,
            QualityEncoding::Solexa => 64,
        }
    }

    /// Returns the valid ASCII range for this encoding.
    pub fn valid_range(&self) -> (u8, u8) {
        match self {
            QualityEncoding::Phred33 => (33, 126),
            QualityEncoding::Phred64 => (64, 126),
            QualityEncoding::Solexa => (59, 126),
        }
    }
}

/// A validated quality score string.
///
/// Quality scores represent the probability that a base call is incorrect.
/// Higher scores indicate higher confidence.
#[derive(Debug, Clone, PartialEq)]
pub struct QualityScores {
    /// Raw quality string (ASCII-encoded).
    raw: String,
    /// The encoding scheme.
    encoding: QualityEncoding,
    /// Cached numeric scores.
    scores: Vec<u8>,
}

impl QualityScores {
    /// Creates a new QualityScores from a quality string.
    ///
    /// # Arguments
    ///
    /// * `quality` - The ASCII-encoded quality string.
    /// * `encoding` - The quality encoding scheme.
    ///
    /// # Returns
    ///
    /// A validated `QualityScores` or an error.
    pub fn new(
        quality: impl Into<String>,
        encoding: QualityEncoding,
    ) -> Result<Self, QualityError> {
        let raw = quality.into();

        if raw.is_empty() {
            return Err(QualityError::EmptyQuality);
        }

        let (min_ascii, max_ascii) = encoding.valid_range();
        let offset = encoding.offset();

        let mut scores = Vec::with_capacity(raw.len());

        for (i, c) in raw.chars().enumerate() {
            let ascii = c as u8;
            if ascii < min_ascii || ascii > max_ascii {
                return Err(QualityError::InvalidCharacter { position: i, char: c });
            }
            scores.push(ascii - offset);
        }

        Ok(Self { raw, encoding, scores })
    }

    /// Creates QualityScores from Phred+33 encoding (most common).
    pub fn from_phred33(quality: impl Into<String>) -> Result<Self, QualityError> {
        Self::new(quality, QualityEncoding::Phred33)
    }

    /// Creates QualityScores from numeric scores.
    ///
    /// # Arguments
    ///
    /// * `scores` - Vector of quality scores (0-93 for Phred33).
    /// * `encoding` - The encoding scheme to use for conversion.
    pub fn from_scores(scores: Vec<u8>, encoding: QualityEncoding) -> Result<Self, QualityError> {
        if scores.is_empty() {
            return Err(QualityError::EmptyQuality);
        }

        let offset = encoding.offset();
        let raw: String = scores.iter()
            .map(|&q| (q + offset) as char)
            .collect();

        Ok(Self {
            raw,
            encoding,
            scores,
        })
    }

    /// Returns the raw quality string.
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// Returns the encoding scheme.
    pub fn encoding(&self) -> QualityEncoding {
        self.encoding
    }

    /// Returns the length of the quality string.
    pub fn len(&self) -> usize {
        self.scores.len()
    }

    /// Returns true if the quality string is empty.
    pub fn is_empty(&self) -> bool {
        self.scores.is_empty()
    }

    /// Returns the numeric quality scores.
    pub fn scores(&self) -> &[u8] {
        &self.scores
    }

    /// Returns the quality score at a specific position.
    pub fn get(&self, index: usize) -> Option<u8> {
        self.scores.get(index).copied()
    }

    /// Returns the mean quality score.
    pub fn mean(&self) -> f64 {
        if self.scores.is_empty() {
            return 0.0;
        }

        let sum: usize = self.scores.iter().map(|&q| q as usize).sum();
        sum as f64 / self.scores.len() as f64
    }

    /// Returns the median quality score.
    pub fn median(&self) -> f64 {
        if self.scores.is_empty() {
            return 0.0;
        }

        let mut sorted = self.scores.clone();
        sorted.sort_unstable();

        let mid = sorted.len() / 2;
        if sorted.len() % 2 == 0 {
            (sorted[mid - 1] as f64 + sorted[mid] as f64) / 2.0
        } else {
            sorted[mid] as f64
        }
    }

    /// Returns the minimum quality score.
    pub fn min(&self) -> u8 {
        *self.scores.iter().min().unwrap_or(&0)
    }

    /// Returns the maximum quality score.
    pub fn max(&self) -> u8 {
        *self.scores.iter().max().unwrap_or(&0)
    }

    /// Returns the fraction of bases with quality >= threshold.
    pub fn fraction_above(&self, threshold: u8) -> f64 {
        if self.scores.is_empty() {
            return 0.0;
        }

        let count = self.scores.iter().filter(|&&q| q >= threshold).count();
        count as f64 / self.scores.len() as f64
    }

    /// Returns the fraction of high-quality bases (Q >= 30).
    pub fn high_quality_fraction(&self) -> f64 {
        self.fraction_above(30)
    }

    /// Converts quality score to error probability.
    ///
    /// P(error) = 10^(-Q/10)
    pub fn error_probability(quality: u8) -> f64 {
        10_f64.powf(-(quality as f64) / 10.0)
    }

    /// Converts error probability to quality score.
    ///
    /// Q = -10 * log10(P)
    pub fn from_error_probability(prob: f64) -> u8 {
        if prob <= 0.0 {
            return 93; // Maximum quality
        }
        if prob >= 1.0 {
            return 0; // Minimum quality
        }

        let q = -10.0 * prob.log10();
        (q.round() as u8).min(93)
    }

    /// Returns the average error probability.
    pub fn mean_error_probability(&self) -> f64 {
        if self.scores.is_empty() {
            return 1.0;
        }

        let sum: f64 = self.scores.iter()
            .map(|&q| Self::error_probability(q))
            .sum();
        sum / self.scores.len() as f64
    }

    /// Trims low-quality bases from both ends.
    ///
    /// # Arguments
    ///
    /// * `min_quality` - Minimum quality threshold.
    ///
    /// # Returns
    ///
    /// Tuple of (start, end) indices for the trimmed region.
    pub fn trim_ends(&self, min_quality: u8) -> (usize, usize) {
        let start = self.scores.iter()
            .position(|&q| q >= min_quality)
            .unwrap_or(self.scores.len());

        let end = self.scores.iter()
            .rposition(|&q| q >= min_quality)
            .map(|i| i + 1)
            .unwrap_or(0);

        (start, end.max(start))
    }

    /// Returns positions of low-quality bases.
    pub fn low_quality_positions(&self, threshold: u8) -> Vec<usize> {
        self.scores.iter()
            .enumerate()
            .filter(|(_, &q)| q < threshold)
            .map(|(i, _)| i)
            .collect()
    }

    /// Calculates a sliding window average of quality scores.
    pub fn sliding_window_mean(&self, window_size: usize) -> Vec<f64> {
        if window_size == 0 || window_size > self.scores.len() {
            return vec![];
        }

        let mut means = Vec::with_capacity(self.scores.len() - window_size + 1);
        let mut sum: usize = self.scores[..window_size].iter().map(|&q| q as usize).sum();

        means.push(sum as f64 / window_size as f64);

        for i in window_size..self.scores.len() {
            sum -= self.scores[i - window_size] as usize;
            sum += self.scores[i] as usize;
            means.push(sum as f64 / window_size as f64);
        }

        means
    }

    /// Returns an iterator over (position, quality) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (usize, u8)> + '_ {
        self.scores.iter().enumerate().map(|(i, &q)| (i, q))
    }
}

impl fmt::Display for QualityScores {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "QualityScores(len={}, mean={:.1}, min={}, max={})",
            self.len(),
            self.mean(),
            self.min(),
            self.max()
        )
    }
}

/// Quality statistics for a collection of reads.
#[derive(Debug, Clone)]
pub struct QualityStats {
    /// Per-position quality sums.
    position_sums: Vec<u64>,
    /// Per-position quality counts.
    position_counts: Vec<u64>,
    /// Overall quality histogram (index = quality score).
    histogram: [u64; 94],
    /// Total number of bases.
    total_bases: u64,
    /// Number of reads processed.
    read_count: u64,
}

impl QualityStats {
    /// Creates a new empty QualityStats.
    pub fn new() -> Self {
        Self {
            position_sums: Vec::new(),
            position_counts: Vec::new(),
            histogram: [0; 94],
            total_bases: 0,
            read_count: 0,
        }
    }

    /// Adds quality scores from a read.
    pub fn add(&mut self, quality: &QualityScores) {
        // Extend position vectors if needed
        if quality.len() > self.position_sums.len() {
            self.position_sums.resize(quality.len(), 0);
            self.position_counts.resize(quality.len(), 0);
        }

        // Update per-position statistics
        for (i, &q) in quality.scores().iter().enumerate() {
            self.position_sums[i] += q as u64;
            self.position_counts[i] += 1;
            if (q as usize) < self.histogram.len() {
                self.histogram[q as usize] += 1;
            }
        }

        self.total_bases += quality.len() as u64;
        self.read_count += 1;
    }

    /// Returns the mean quality at each position.
    pub fn per_position_mean(&self) -> Vec<f64> {
        self.position_sums.iter()
            .zip(self.position_counts.iter())
            .map(|(&sum, &count)| {
                if count > 0 {
                    sum as f64 / count as f64
                } else {
                    0.0
                }
            })
            .collect()
    }

    /// Returns the overall mean quality.
    pub fn mean(&self) -> f64 {
        if self.total_bases == 0 {
            return 0.0;
        }

        let sum: u64 = self.histogram.iter()
            .enumerate()
            .map(|(q, &count)| q as u64 * count)
            .sum();

        sum as f64 / self.total_bases as f64
    }

    /// Returns the quality score histogram.
    pub fn histogram(&self) -> &[u64; 94] {
        &self.histogram
    }

    /// Returns the total number of bases processed.
    pub fn total_bases(&self) -> u64 {
        self.total_bases
    }

    /// Returns the number of reads processed.
    pub fn read_count(&self) -> u64 {
        self.read_count
    }
}

impl Default for QualityStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phred33_decoding() {
        // '!' = ASCII 33 = Q0, 'I' = ASCII 73 = Q40
        let quality = QualityScores::from_phred33("!IIIII").unwrap();
        assert_eq!(quality.scores()[0], 0);
        assert_eq!(quality.scores()[1], 40);
    }

    #[test]
    fn test_mean_quality() {
        let quality = QualityScores::from_scores(vec![10, 20, 30], QualityEncoding::Phred33).unwrap();
        assert!((quality.mean() - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_median_quality() {
        let quality = QualityScores::from_scores(vec![10, 20, 30], QualityEncoding::Phred33).unwrap();
        assert!((quality.median() - 20.0).abs() < f64::EPSILON);

        let quality2 = QualityScores::from_scores(vec![10, 20, 30, 40], QualityEncoding::Phred33).unwrap();
        assert!((quality2.median() - 25.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_error_probability() {
        // Q10 = 10% error rate
        let prob = QualityScores::error_probability(10);
        assert!((prob - 0.1).abs() < 1e-10);

        // Q20 = 1% error rate
        let prob = QualityScores::error_probability(20);
        assert!((prob - 0.01).abs() < 1e-10);

        // Q30 = 0.1% error rate
        let prob = QualityScores::error_probability(30);
        assert!((prob - 0.001).abs() < 1e-10);
    }

    #[test]
    fn test_from_error_probability() {
        assert_eq!(QualityScores::from_error_probability(0.1), 10);
        assert_eq!(QualityScores::from_error_probability(0.01), 20);
        assert_eq!(QualityScores::from_error_probability(0.001), 30);
    }

    #[test]
    fn test_trim_ends() {
        let quality = QualityScores::from_scores(
            vec![5, 10, 30, 30, 30, 10, 5],
            QualityEncoding::Phred33
        ).unwrap();

        let (start, end) = quality.trim_ends(20);
        assert_eq!(start, 2);
        assert_eq!(end, 5);
    }

    #[test]
    fn test_high_quality_fraction() {
        let quality = QualityScores::from_scores(
            vec![20, 30, 35, 40, 10],
            QualityEncoding::Phred33
        ).unwrap();

        assert!((quality.high_quality_fraction() - 0.6).abs() < f64::EPSILON);
    }

    #[test]
    fn test_sliding_window() {
        let quality = QualityScores::from_scores(
            vec![10, 20, 30, 40, 50],
            QualityEncoding::Phred33
        ).unwrap();

        let means = quality.sliding_window_mean(3);
        assert_eq!(means.len(), 3);
        assert!((means[0] - 20.0).abs() < f64::EPSILON); // (10+20+30)/3
        assert!((means[1] - 30.0).abs() < f64::EPSILON); // (20+30+40)/3
        assert!((means[2] - 40.0).abs() < f64::EPSILON); // (30+40+50)/3
    }

    #[test]
    fn test_quality_stats() {
        let mut stats = QualityStats::new();

        let q1 = QualityScores::from_scores(vec![20, 30, 40], QualityEncoding::Phred33).unwrap();
        let q2 = QualityScores::from_scores(vec![10, 20, 30], QualityEncoding::Phred33).unwrap();

        stats.add(&q1);
        stats.add(&q2);

        assert_eq!(stats.read_count(), 2);
        assert_eq!(stats.total_bases(), 6);

        let per_pos = stats.per_position_mean();
        assert!((per_pos[0] - 15.0).abs() < f64::EPSILON); // (20+10)/2
        assert!((per_pos[1] - 25.0).abs() < f64::EPSILON); // (30+20)/2
        assert!((per_pos[2] - 35.0).abs() < f64::EPSILON); // (40+30)/2
    }

    #[test]
    fn test_invalid_quality() {
        // ASCII 32 (space) is below Phred33 minimum
        let result = QualityScores::from_phred33(" ");
        assert!(result.is_err());
    }
}
