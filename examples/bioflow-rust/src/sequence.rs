//! DNA/RNA sequence representation and manipulation.
//!
//! This module provides the core `Sequence` type along with validation,
//! transformation, and analysis methods.

use std::fmt;
use thiserror::Error;

/// Errors that can occur when working with sequences.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum SequenceError {
    /// The sequence is empty.
    #[error("Empty sequence")]
    EmptySequence,

    /// An invalid base was found in the sequence.
    #[error("Invalid base '{base}' at position {position}")]
    InvalidBase { position: usize, base: char },

    /// The sequence contains only N bases.
    #[error("Sequence contains only ambiguous bases")]
    OnlyAmbiguousBases,
}

/// The type of nucleic acid sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceType {
    /// Deoxyribonucleic acid (A, C, G, T).
    DNA,
    /// Ribonucleic acid (A, C, G, U).
    RNA,
}

/// A validated nucleotide sequence.
///
/// # Examples
///
/// ```
/// use bioflow_rust::sequence::Sequence;
///
/// let seq = Sequence::new("ATGCGATCGA").unwrap();
/// assert_eq!(seq.len(), 10);
/// assert!(seq.gc_content() > 0.0);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sequence {
    bases: String,
    id: Option<String>,
    description: Option<String>,
    seq_type: SequenceType,
}

impl Sequence {
    /// Creates a new DNA sequence from the given bases.
    ///
    /// # Arguments
    ///
    /// * `bases` - A string of nucleotide characters (A, C, G, T, N).
    ///
    /// # Returns
    ///
    /// A `Result` containing the validated sequence or an error.
    ///
    /// # Examples
    ///
    /// ```
    /// use bioflow_rust::sequence::Sequence;
    ///
    /// let seq = Sequence::new("ATGC").unwrap();
    /// assert_eq!(seq.len(), 4);
    ///
    /// let err = Sequence::new("ATXC");
    /// assert!(err.is_err());
    /// ```
    pub fn new(bases: impl Into<String>) -> Result<Self, SequenceError> {
        let bases = bases.into().to_uppercase();
        if bases.is_empty() {
            return Err(SequenceError::EmptySequence);
        }

        // Validate all bases
        for (i, base) in bases.chars().enumerate() {
            if !matches!(base, 'A' | 'C' | 'G' | 'T' | 'N') {
                return Err(SequenceError::InvalidBase { position: i, base });
            }
        }

        Ok(Self {
            bases,
            id: None,
            description: None,
            seq_type: SequenceType::DNA,
        })
    }

    /// Creates a new RNA sequence from the given bases.
    ///
    /// # Arguments
    ///
    /// * `bases` - A string of nucleotide characters (A, C, G, U, N).
    ///
    /// # Examples
    ///
    /// ```
    /// use bioflow_rust::sequence::Sequence;
    ///
    /// let seq = Sequence::new_rna("AUGC").unwrap();
    /// ```
    pub fn new_rna(bases: impl Into<String>) -> Result<Self, SequenceError> {
        let bases = bases.into().to_uppercase();
        if bases.is_empty() {
            return Err(SequenceError::EmptySequence);
        }

        // Validate all bases
        for (i, base) in bases.chars().enumerate() {
            if !matches!(base, 'A' | 'C' | 'G' | 'U' | 'N') {
                return Err(SequenceError::InvalidBase { position: i, base });
            }
        }

        Ok(Self {
            bases,
            id: None,
            description: None,
            seq_type: SequenceType::RNA,
        })
    }

    /// Creates a sequence with an identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use bioflow_rust::sequence::Sequence;
    ///
    /// let seq = Sequence::with_id("ATGC", "seq1").unwrap();
    /// assert_eq!(seq.id(), Some("seq1"));
    /// ```
    pub fn with_id(bases: impl Into<String>, id: impl Into<String>) -> Result<Self, SequenceError> {
        let mut seq = Self::new(bases)?;
        seq.id = Some(id.into());
        Ok(seq)
    }

    /// Sets the sequence identifier.
    pub fn set_id(&mut self, id: impl Into<String>) {
        self.id = Some(id.into());
    }

    /// Sets the sequence description.
    pub fn set_description(&mut self, description: impl Into<String>) {
        self.description = Some(description.into());
    }

    /// Returns the sequence identifier, if set.
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// Returns the sequence description, if set.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Returns a reference to the underlying bases.
    #[inline]
    pub fn bases(&self) -> &str {
        &self.bases
    }

    /// Returns the length of the sequence.
    #[inline]
    pub fn len(&self) -> usize {
        self.bases.len()
    }

    /// Returns true if the sequence is empty (which should never happen for valid sequences).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.bases.is_empty()
    }

    /// Returns the type of sequence (DNA or RNA).
    pub fn sequence_type(&self) -> SequenceType {
        self.seq_type
    }

    /// Calculates the GC content of the sequence.
    ///
    /// GC content is the proportion of guanine (G) and cytosine (C) bases
    /// in the sequence, expressed as a fraction between 0.0 and 1.0.
    ///
    /// # Returns
    ///
    /// A `f64` value between 0.0 and 1.0.
    ///
    /// # Examples
    ///
    /// ```
    /// use bioflow_rust::sequence::Sequence;
    ///
    /// let seq = Sequence::new("ATGC").unwrap();
    /// assert!((seq.gc_content() - 0.5).abs() < f64::EPSILON);
    ///
    /// let at_rich = Sequence::new("AATTAATT").unwrap();
    /// assert!((at_rich.gc_content() - 0.0).abs() < f64::EPSILON);
    /// ```
    pub fn gc_content(&self) -> f64 {
        let gc_count = self.bases.bytes()
            .filter(|&c| c == b'G' || c == b'C')
            .count();
        gc_count as f64 / self.bases.len() as f64
    }

    /// Calculates base composition statistics.
    ///
    /// # Returns
    ///
    /// A `BaseComposition` struct containing counts and frequencies.
    pub fn base_composition(&self) -> BaseComposition {
        let mut counts = [0usize; 5]; // A, C, G, T/U, N

        for base in self.bases.bytes() {
            match base {
                b'A' => counts[0] += 1,
                b'C' => counts[1] += 1,
                b'G' => counts[2] += 1,
                b'T' | b'U' => counts[3] += 1,
                b'N' => counts[4] += 1,
                _ => {}
            }
        }

        let total = self.bases.len() as f64;
        BaseComposition {
            a_count: counts[0],
            c_count: counts[1],
            g_count: counts[2],
            t_count: counts[3],
            n_count: counts[4],
            a_freq: counts[0] as f64 / total,
            c_freq: counts[1] as f64 / total,
            g_freq: counts[2] as f64 / total,
            t_freq: counts[3] as f64 / total,
            n_freq: counts[4] as f64 / total,
        }
    }

    /// Returns the complement of the sequence.
    ///
    /// DNA: A<->T, C<->G
    /// RNA: A<->U, C<->G
    ///
    /// # Examples
    ///
    /// ```
    /// use bioflow_rust::sequence::Sequence;
    ///
    /// let seq = Sequence::new("ATGC").unwrap();
    /// let comp = seq.complement();
    /// assert_eq!(comp.bases(), "TACG");
    /// ```
    pub fn complement(&self) -> Self {
        let comp: String = match self.seq_type {
            SequenceType::DNA => {
                self.bases.chars()
                    .map(|c| match c {
                        'A' => 'T',
                        'T' => 'A',
                        'C' => 'G',
                        'G' => 'C',
                        _ => 'N',
                    })
                    .collect()
            }
            SequenceType::RNA => {
                self.bases.chars()
                    .map(|c| match c {
                        'A' => 'U',
                        'U' => 'A',
                        'C' => 'G',
                        'G' => 'C',
                        _ => 'N',
                    })
                    .collect()
            }
        };

        Self {
            bases: comp,
            id: self.id.clone(),
            description: self.description.clone(),
            seq_type: self.seq_type,
        }
    }

    /// Returns the reverse of the sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use bioflow_rust::sequence::Sequence;
    ///
    /// let seq = Sequence::new("ATGC").unwrap();
    /// let rev = seq.reverse();
    /// assert_eq!(rev.bases(), "CGTA");
    /// ```
    pub fn reverse(&self) -> Self {
        Self {
            bases: self.bases.chars().rev().collect(),
            id: self.id.clone(),
            description: self.description.clone(),
            seq_type: self.seq_type,
        }
    }

    /// Returns the reverse complement of the sequence.
    ///
    /// This is equivalent to calling `reverse()` on the `complement()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use bioflow_rust::sequence::Sequence;
    ///
    /// let seq = Sequence::new("ATGC").unwrap();
    /// let revcomp = seq.reverse_complement();
    /// assert_eq!(revcomp.bases(), "GCAT");
    /// ```
    pub fn reverse_complement(&self) -> Self {
        self.complement().reverse()
    }

    /// Transcribes DNA to RNA (replaces T with U).
    ///
    /// # Returns
    ///
    /// A new `Sequence` of type RNA.
    ///
    /// # Panics
    ///
    /// Panics if called on an RNA sequence.
    pub fn transcribe(&self) -> Self {
        assert_eq!(self.seq_type, SequenceType::DNA, "Can only transcribe DNA sequences");

        Self {
            bases: self.bases.replace('T', "U"),
            id: self.id.clone(),
            description: self.description.clone(),
            seq_type: SequenceType::RNA,
        }
    }

    /// Reverse transcribes RNA to DNA (replaces U with T).
    ///
    /// # Returns
    ///
    /// A new `Sequence` of type DNA.
    ///
    /// # Panics
    ///
    /// Panics if called on a DNA sequence.
    pub fn reverse_transcribe(&self) -> Self {
        assert_eq!(self.seq_type, SequenceType::RNA, "Can only reverse transcribe RNA sequences");

        Self {
            bases: self.bases.replace('U', "T"),
            id: self.id.clone(),
            description: self.description.clone(),
            seq_type: SequenceType::DNA,
        }
    }

    /// Returns a subsequence (slice) of the sequence.
    ///
    /// # Arguments
    ///
    /// * `start` - The starting position (0-indexed, inclusive).
    /// * `end` - The ending position (exclusive).
    ///
    /// # Returns
    ///
    /// A new `Sequence` containing the subsequence.
    ///
    /// # Panics
    ///
    /// Panics if the range is out of bounds.
    pub fn subsequence(&self, start: usize, end: usize) -> Result<Self, SequenceError> {
        if start >= end || end > self.bases.len() {
            return Err(SequenceError::EmptySequence);
        }

        Ok(Self {
            bases: self.bases[start..end].to_string(),
            id: self.id.as_ref().map(|id| format!("{}_{}_{}", id, start, end)),
            description: None,
            seq_type: self.seq_type,
        })
    }

    /// Finds all occurrences of a pattern in the sequence.
    ///
    /// # Arguments
    ///
    /// * `pattern` - The pattern to search for.
    ///
    /// # Returns
    ///
    /// A vector of starting positions where the pattern was found.
    pub fn find_pattern(&self, pattern: &str) -> Vec<usize> {
        let pattern = pattern.to_uppercase();
        let mut positions = Vec::new();

        if pattern.len() > self.bases.len() {
            return positions;
        }

        for i in 0..=self.bases.len() - pattern.len() {
            if &self.bases[i..i + pattern.len()] == pattern {
                positions.push(i);
            }
        }

        positions
    }

    /// Returns an iterator over the bases.
    pub fn iter(&self) -> impl Iterator<Item = char> + '_ {
        self.bases.chars()
    }

    /// Returns an iterator over overlapping windows of the specified size.
    pub fn windows(&self, size: usize) -> impl Iterator<Item = &str> {
        (0..=self.bases.len().saturating_sub(size))
            .map(move |i| &self.bases[i..i + size])
    }

    /// Calculates the molecular weight of the sequence.
    ///
    /// Uses average molecular weights for nucleotides.
    pub fn molecular_weight(&self) -> f64 {
        let comp = self.base_composition();

        // Approximate molecular weights (Da) for nucleotides
        let (a_mw, c_mw, g_mw, t_mw) = match self.seq_type {
            SequenceType::DNA => (331.2, 307.2, 347.2, 322.2),
            SequenceType::RNA => (347.2, 323.2, 363.2, 324.2), // U instead of T
        };

        (comp.a_count as f64 * a_mw) +
        (comp.c_count as f64 * c_mw) +
        (comp.g_count as f64 * g_mw) +
        (comp.t_count as f64 * t_mw) -
        (18.015 * (self.len() - 1) as f64) // Water loss from phosphodiester bonds
    }

    /// Calculates the melting temperature (Tm) using the nearest-neighbor method.
    ///
    /// This is a simplified calculation using the Wallace rule for short oligos.
    pub fn melting_temperature(&self) -> f64 {
        let comp = self.base_composition();

        if self.len() < 14 {
            // Wallace rule for short sequences
            (comp.a_count + comp.t_count) as f64 * 2.0 +
            (comp.g_count + comp.c_count) as f64 * 4.0
        } else {
            // Salt-adjusted formula for longer sequences
            64.9 + 41.0 * ((comp.g_count + comp.c_count) as f64 - 16.4) / self.len() as f64
        }
    }
}

impl fmt::Display for Sequence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref id) = self.id {
            write!(f, ">{}", id)?;
            if let Some(ref desc) = self.description {
                write!(f, " {}", desc)?;
            }
            writeln!(f)?;
        }
        write!(f, "{}", self.bases)
    }
}

impl AsRef<str> for Sequence {
    fn as_ref(&self) -> &str {
        &self.bases
    }
}

/// Statistics about base composition.
#[derive(Debug, Clone, Copy)]
pub struct BaseComposition {
    pub a_count: usize,
    pub c_count: usize,
    pub g_count: usize,
    pub t_count: usize,
    pub n_count: usize,
    pub a_freq: f64,
    pub c_freq: f64,
    pub g_freq: f64,
    pub t_freq: f64,
    pub n_freq: f64,
}

impl BaseComposition {
    /// Returns the GC content.
    pub fn gc_content(&self) -> f64 {
        self.g_freq + self.c_freq
    }

    /// Returns the AT content.
    pub fn at_content(&self) -> f64 {
        self.a_freq + self.t_freq
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_sequence() {
        let seq = Sequence::new("ATGC").unwrap();
        assert_eq!(seq.bases(), "ATGC");
        assert_eq!(seq.len(), 4);
    }

    #[test]
    fn test_empty_sequence() {
        let result = Sequence::new("");
        assert!(matches!(result, Err(SequenceError::EmptySequence)));
    }

    #[test]
    fn test_invalid_base() {
        let result = Sequence::new("ATXGC");
        assert!(matches!(result, Err(SequenceError::InvalidBase { position: 2, base: 'X' })));
    }

    #[test]
    fn test_gc_content() {
        let seq = Sequence::new("ATGC").unwrap();
        assert!((seq.gc_content() - 0.5).abs() < f64::EPSILON);

        let gc_rich = Sequence::new("GGCC").unwrap();
        assert!((gc_rich.gc_content() - 1.0).abs() < f64::EPSILON);

        let at_rich = Sequence::new("AATT").unwrap();
        assert!((at_rich.gc_content() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_complement() {
        let seq = Sequence::new("ATGC").unwrap();
        let comp = seq.complement();
        assert_eq!(comp.bases(), "TACG");
    }

    #[test]
    fn test_reverse() {
        let seq = Sequence::new("ATGC").unwrap();
        let rev = seq.reverse();
        assert_eq!(rev.bases(), "CGTA");
    }

    #[test]
    fn test_reverse_complement() {
        let seq = Sequence::new("ATGC").unwrap();
        let revcomp = seq.reverse_complement();
        assert_eq!(revcomp.bases(), "GCAT");
    }

    #[test]
    fn test_transcribe() {
        let dna = Sequence::new("ATGC").unwrap();
        let rna = dna.transcribe();
        assert_eq!(rna.bases(), "AUGC");
        assert_eq!(rna.sequence_type(), SequenceType::RNA);
    }

    #[test]
    fn test_find_pattern() {
        let seq = Sequence::new("ATGATGATG").unwrap();
        let positions = seq.find_pattern("ATG");
        assert_eq!(positions, vec![0, 3, 6]);
    }

    #[test]
    fn test_subsequence() {
        let seq = Sequence::new("ATGCATGC").unwrap();
        let sub = seq.subsequence(2, 6).unwrap();
        assert_eq!(sub.bases(), "GCAT");
    }

    #[test]
    fn test_base_composition() {
        let seq = Sequence::new("AACCGGTT").unwrap();
        let comp = seq.base_composition();
        assert_eq!(comp.a_count, 2);
        assert_eq!(comp.c_count, 2);
        assert_eq!(comp.g_count, 2);
        assert_eq!(comp.t_count, 2);
    }

    #[test]
    fn test_case_insensitive() {
        let seq = Sequence::new("atgc").unwrap();
        assert_eq!(seq.bases(), "ATGC");
    }

    #[test]
    fn test_with_id() {
        let seq = Sequence::with_id("ATGC", "test_seq").unwrap();
        assert_eq!(seq.id(), Some("test_seq"));
    }
}
