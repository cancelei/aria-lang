//! K-mer counting and analysis.
//!
//! This module provides efficient k-mer extraction, counting, and analysis
//! for DNA/RNA sequences.

use std::collections::HashMap;
use crate::sequence::Sequence;

/// A k-mer counter that tracks the frequency of k-mers in sequences.
///
/// # Type Parameters
///
/// * `K` - The size of k-mers to count (compile-time constant when possible).
///
/// # Examples
///
/// ```
/// use bioflow_rust::sequence::Sequence;
/// use bioflow_rust::kmer::KMerCounter;
///
/// let seq = Sequence::new("ATGATGATG").unwrap();
/// let mut counter = KMerCounter::new(3);
/// counter.count(&seq);
///
/// let top = counter.most_frequent(1);
/// assert_eq!(top[0].0, "ATG");
/// assert_eq!(top[0].1, 3);
/// ```
#[derive(Debug, Clone)]
pub struct KMerCounter {
    k: usize,
    counts: HashMap<String, usize>,
    total_kmers: usize,
    include_n: bool,
}

impl KMerCounter {
    /// Creates a new k-mer counter with the specified k value.
    ///
    /// # Arguments
    ///
    /// * `k` - The length of k-mers to count.
    ///
    /// # Panics
    ///
    /// Panics if `k` is 0.
    pub fn new(k: usize) -> Self {
        assert!(k > 0, "k must be greater than 0");
        Self {
            k,
            counts: HashMap::new(),
            total_kmers: 0,
            include_n: false,
        }
    }

    /// Sets whether to include k-mers containing 'N' (ambiguous base).
    ///
    /// By default, k-mers with N are excluded.
    pub fn include_ambiguous(mut self, include: bool) -> Self {
        self.include_n = include;
        self
    }

    /// Returns the k value.
    pub fn k(&self) -> usize {
        self.k
    }

    /// Returns the total number of k-mers counted.
    pub fn total_kmers(&self) -> usize {
        self.total_kmers
    }

    /// Returns the number of unique k-mers.
    pub fn unique_kmers(&self) -> usize {
        self.counts.len()
    }

    /// Counts k-mers in the given sequence.
    ///
    /// # Arguments
    ///
    /// * `sequence` - The sequence to count k-mers from.
    pub fn count(&mut self, sequence: &Sequence) {
        let bases = sequence.bases();
        if bases.len() < self.k {
            return;
        }

        for i in 0..=bases.len() - self.k {
            let kmer = &bases[i..i + self.k];

            if !self.include_n && kmer.contains('N') {
                continue;
            }

            *self.counts.entry(kmer.to_string()).or_insert(0) += 1;
            self.total_kmers += 1;
        }
    }

    /// Counts k-mers in multiple sequences.
    pub fn count_all<'a>(&mut self, sequences: impl IntoIterator<Item = &'a Sequence>) {
        for seq in sequences {
            self.count(seq);
        }
    }

    /// Returns the count for a specific k-mer.
    ///
    /// # Arguments
    ///
    /// * `kmer` - The k-mer to look up.
    ///
    /// # Returns
    ///
    /// The count of the k-mer, or 0 if not found.
    pub fn get(&self, kmer: &str) -> usize {
        *self.counts.get(&kmer.to_uppercase()).unwrap_or(&0)
    }

    /// Returns the frequency of a specific k-mer.
    ///
    /// # Returns
    ///
    /// The frequency as a fraction of total k-mers.
    pub fn frequency(&self, kmer: &str) -> f64 {
        if self.total_kmers == 0 {
            return 0.0;
        }
        self.get(kmer) as f64 / self.total_kmers as f64
    }

    /// Returns the most frequent k-mers.
    ///
    /// # Arguments
    ///
    /// * `n` - The number of top k-mers to return.
    ///
    /// # Returns
    ///
    /// A vector of (kmer, count) pairs, sorted by count descending.
    pub fn most_frequent(&self, n: usize) -> Vec<(String, usize)> {
        let mut counts: Vec<_> = self.counts.iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        counts.into_iter().take(n).collect()
    }

    /// Returns the least frequent k-mers.
    ///
    /// # Arguments
    ///
    /// * `n` - The number of k-mers to return.
    ///
    /// # Returns
    ///
    /// A vector of (kmer, count) pairs, sorted by count ascending.
    pub fn least_frequent(&self, n: usize) -> Vec<(String, usize)> {
        let mut counts: Vec<_> = self.counts.iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        counts.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
        counts.into_iter().take(n).collect()
    }

    /// Returns all k-mers and their counts.
    pub fn all_kmers(&self) -> Vec<(String, usize)> {
        let mut counts: Vec<_> = self.counts.iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        counts.sort_by(|a, b| a.0.cmp(&b.0));
        counts
    }

    /// Returns k-mers with counts within the specified range.
    pub fn kmers_in_range(&self, min: usize, max: usize) -> Vec<(String, usize)> {
        self.counts.iter()
            .filter(|(_, &v)| v >= min && v <= max)
            .map(|(k, v)| (k.clone(), *v))
            .collect()
    }

    /// Clears all counts.
    pub fn clear(&mut self) {
        self.counts.clear();
        self.total_kmers = 0;
    }

    /// Merges another counter into this one.
    ///
    /// # Arguments
    ///
    /// * `other` - The counter to merge from.
    ///
    /// # Panics
    ///
    /// Panics if the k values don't match.
    pub fn merge(&mut self, other: &KMerCounter) {
        assert_eq!(self.k, other.k, "Cannot merge counters with different k values");

        for (kmer, count) in &other.counts {
            *self.counts.entry(kmer.clone()).or_insert(0) += count;
        }
        self.total_kmers += other.total_kmers;
    }

    /// Returns an iterator over all k-mers and their counts.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &usize)> {
        self.counts.iter()
    }

    /// Calculates the Shannon entropy of the k-mer distribution.
    pub fn entropy(&self) -> f64 {
        if self.total_kmers == 0 {
            return 0.0;
        }

        let mut entropy = 0.0;
        let total = self.total_kmers as f64;

        for &count in self.counts.values() {
            if count > 0 {
                let p = count as f64 / total;
                entropy -= p * p.log2();
            }
        }

        entropy
    }

    /// Calculates the expected number of unique k-mers given the sequence length.
    ///
    /// Uses the formula: 4^k for DNA (all possible k-mers).
    pub fn expected_unique_kmers(&self) -> usize {
        4_usize.pow(self.k as u32)
    }

    /// Returns the saturation (fraction of all possible k-mers observed).
    pub fn saturation(&self) -> f64 {
        self.unique_kmers() as f64 / self.expected_unique_kmers() as f64
    }
}

/// A canonical k-mer counter that treats a k-mer and its reverse complement as the same.
///
/// This is useful for double-stranded DNA analysis where the strand is unknown.
#[derive(Debug, Clone)]
pub struct CanonicalKMerCounter {
    inner: KMerCounter,
}

impl CanonicalKMerCounter {
    /// Creates a new canonical k-mer counter.
    pub fn new(k: usize) -> Self {
        Self {
            inner: KMerCounter::new(k),
        }
    }

    /// Returns the k value.
    pub fn k(&self) -> usize {
        self.inner.k
    }

    /// Counts canonical k-mers in the sequence.
    pub fn count(&mut self, sequence: &Sequence) {
        let bases = sequence.bases();
        if bases.len() < self.inner.k {
            return;
        }

        for i in 0..=bases.len() - self.inner.k {
            let kmer = &bases[i..i + self.inner.k];

            if kmer.contains('N') {
                continue;
            }

            // Compute reverse complement
            let revcomp: String = kmer.chars()
                .rev()
                .map(|c| match c {
                    'A' => 'T',
                    'T' => 'A',
                    'C' => 'G',
                    'G' => 'C',
                    _ => 'N',
                })
                .collect();

            // Use the lexicographically smaller of kmer and revcomp
            let canonical = if kmer <= revcomp.as_str() {
                kmer.to_string()
            } else {
                revcomp
            };

            *self.inner.counts.entry(canonical).or_insert(0) += 1;
            self.inner.total_kmers += 1;
        }
    }

    /// Returns the most frequent canonical k-mers.
    pub fn most_frequent(&self, n: usize) -> Vec<(String, usize)> {
        self.inner.most_frequent(n)
    }

    /// Returns the total number of k-mers counted.
    pub fn total_kmers(&self) -> usize {
        self.inner.total_kmers
    }

    /// Returns the number of unique canonical k-mers.
    pub fn unique_kmers(&self) -> usize {
        self.inner.unique_kmers()
    }
}

/// Generates all possible k-mers of a given length.
///
/// # Arguments
///
/// * `k` - The length of k-mers to generate.
///
/// # Returns
///
/// A vector of all possible k-mers (4^k sequences).
pub fn generate_all_kmers(k: usize) -> Vec<String> {
    if k == 0 {
        return vec![String::new()];
    }

    let bases = ['A', 'C', 'G', 'T'];
    let mut kmers = Vec::with_capacity(4_usize.pow(k as u32));

    fn generate_recursive(current: &mut String, k: usize, bases: &[char], result: &mut Vec<String>) {
        if current.len() == k {
            result.push(current.clone());
            return;
        }

        for &base in bases {
            current.push(base);
            generate_recursive(current, k, bases, result);
            current.pop();
        }
    }

    let mut current = String::with_capacity(k);
    generate_recursive(&mut current, k, &bases, &mut kmers);
    kmers
}

/// Calculates the Jaccard similarity between two k-mer sets.
///
/// # Arguments
///
/// * `counter1` - First k-mer counter.
/// * `counter2` - Second k-mer counter.
///
/// # Returns
///
/// The Jaccard similarity coefficient (0.0 to 1.0).
pub fn jaccard_similarity(counter1: &KMerCounter, counter2: &KMerCounter) -> f64 {
    let set1: std::collections::HashSet<_> = counter1.counts.keys().collect();
    let set2: std::collections::HashSet<_> = counter2.counts.keys().collect();

    let intersection = set1.intersection(&set2).count();
    let union = set1.union(&set2).count();

    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

/// Calculates the Bray-Curtis dissimilarity between two k-mer counters.
///
/// # Returns
///
/// A value between 0.0 (identical) and 1.0 (completely different).
pub fn bray_curtis_dissimilarity(counter1: &KMerCounter, counter2: &KMerCounter) -> f64 {
    let all_kmers: std::collections::HashSet<_> = counter1.counts.keys()
        .chain(counter2.counts.keys())
        .collect();

    let mut sum_min = 0usize;
    let mut sum_total = 0usize;

    for kmer in all_kmers {
        let c1 = *counter1.counts.get(kmer).unwrap_or(&0);
        let c2 = *counter2.counts.get(kmer).unwrap_or(&0);
        sum_min += c1.min(c2);
        sum_total += c1 + c2;
    }

    if sum_total == 0 {
        0.0
    } else {
        1.0 - (2.0 * sum_min as f64) / sum_total as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kmer_counting() {
        let seq = Sequence::new("ATGATGATG").unwrap();
        let mut counter = KMerCounter::new(3);
        counter.count(&seq);

        assert_eq!(counter.get("ATG"), 3);
        assert_eq!(counter.get("TGA"), 2);
        assert_eq!(counter.get("GAT"), 2);
    }

    #[test]
    fn test_most_frequent() {
        let seq = Sequence::new("ATGATGATG").unwrap();
        let mut counter = KMerCounter::new(3);
        counter.count(&seq);

        let top = counter.most_frequent(1);
        assert_eq!(top[0].0, "ATG");
        assert_eq!(top[0].1, 3);
    }

    #[test]
    fn test_total_kmers() {
        let seq = Sequence::new("ATGC").unwrap();
        let mut counter = KMerCounter::new(2);
        counter.count(&seq);

        assert_eq!(counter.total_kmers(), 3); // AT, TG, GC
    }

    #[test]
    fn test_skip_n_bases() {
        let seq = Sequence::new("ATNGC").unwrap();
        let mut counter = KMerCounter::new(2);
        counter.count(&seq);

        assert_eq!(counter.get("AT"), 1);
        assert_eq!(counter.get("TN"), 0); // Skipped
        assert_eq!(counter.get("NG"), 0); // Skipped
        assert_eq!(counter.get("GC"), 1);
    }

    #[test]
    fn test_include_n() {
        let seq = Sequence::new("ATNGC").unwrap();
        let mut counter = KMerCounter::new(2).include_ambiguous(true);
        counter.count(&seq);

        assert_eq!(counter.get("TN"), 1);
        assert_eq!(counter.get("NG"), 1);
    }

    #[test]
    fn test_frequency() {
        let seq = Sequence::new("AAAA").unwrap();
        let mut counter = KMerCounter::new(2);
        counter.count(&seq);

        assert!((counter.frequency("AA") - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_merge() {
        let seq1 = Sequence::new("ATGC").unwrap();
        let seq2 = Sequence::new("ATGC").unwrap();

        let mut counter1 = KMerCounter::new(2);
        counter1.count(&seq1);

        let mut counter2 = KMerCounter::new(2);
        counter2.count(&seq2);

        counter1.merge(&counter2);

        assert_eq!(counter1.get("AT"), 2);
        assert_eq!(counter1.get("TG"), 2);
        assert_eq!(counter1.get("GC"), 2);
    }

    #[test]
    fn test_generate_all_kmers() {
        let kmers = generate_all_kmers(2);
        assert_eq!(kmers.len(), 16); // 4^2

        let kmers = generate_all_kmers(3);
        assert_eq!(kmers.len(), 64); // 4^3
    }

    #[test]
    fn test_canonical_kmer() {
        let seq = Sequence::new("ATGCAT").unwrap(); // Contains ATG and CAT (revcomp of ATG)
        let mut counter = CanonicalKMerCounter::new(3);
        counter.count(&seq);

        // ATG and CAT should be counted together as ATG (lexicographically smaller)
        let top = counter.most_frequent(10);
        // Both ATG and its complement should contribute to the same canonical k-mer
        assert!(top.iter().any(|(kmer, _)| kmer == "ATG"));
    }

    #[test]
    fn test_jaccard_similarity() {
        let seq1 = Sequence::new("ATGC").unwrap();
        let seq2 = Sequence::new("ATGC").unwrap();

        let mut counter1 = KMerCounter::new(2);
        counter1.count(&seq1);

        let mut counter2 = KMerCounter::new(2);
        counter2.count(&seq2);

        let similarity = jaccard_similarity(&counter1, &counter2);
        assert!((similarity - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_entropy() {
        // Uniform distribution should have higher entropy
        let seq = Sequence::new("ATGC".repeat(100)).unwrap();
        let mut counter = KMerCounter::new(2);
        counter.count(&seq);

        let entropy = counter.entropy();
        assert!(entropy > 0.0);
    }
}
