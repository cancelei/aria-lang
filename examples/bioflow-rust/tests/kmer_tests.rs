//! Integration tests for the k-mer module.

use bioflow_rust::kmer::*;
use bioflow_rust::sequence::Sequence;

#[test]
fn test_kmer_counter_new() {
    let counter = KMerCounter::new(21);
    assert_eq!(counter.k(), 21);
    assert_eq!(counter.total_kmers(), 0);
    assert_eq!(counter.unique_kmers(), 0);
}

#[test]
#[should_panic]
fn test_kmer_counter_zero_k() {
    KMerCounter::new(0);
}

#[test]
fn test_basic_counting() {
    let seq = Sequence::new("ATGATGATG").unwrap();
    let mut counter = KMerCounter::new(3);
    counter.count(&seq);

    assert_eq!(counter.get("ATG"), 3);
    assert_eq!(counter.get("TGA"), 2);
    assert_eq!(counter.get("GAT"), 2);
}

#[test]
fn test_total_kmers() {
    let seq = Sequence::new("ATGC").unwrap();
    let mut counter = KMerCounter::new(2);
    counter.count(&seq);

    // AT, TG, GC
    assert_eq!(counter.total_kmers(), 3);
}

#[test]
fn test_unique_kmers() {
    let seq = Sequence::new("ATGATGATG").unwrap();
    let mut counter = KMerCounter::new(3);
    counter.count(&seq);

    // ATG, TGA, GAT are unique
    assert_eq!(counter.unique_kmers(), 3);
}

#[test]
fn test_skip_n_bases() {
    let seq = Sequence::new("ATNGC").unwrap();
    let mut counter = KMerCounter::new(2);
    counter.count(&seq);

    assert_eq!(counter.get("AT"), 1);
    assert_eq!(counter.get("TN"), 0);
    assert_eq!(counter.get("NG"), 0);
    assert_eq!(counter.get("GC"), 1);
    assert_eq!(counter.total_kmers(), 2);
}

#[test]
fn test_include_n() {
    let seq = Sequence::new("ATNGC").unwrap();
    let mut counter = KMerCounter::new(2).include_ambiguous(true);
    counter.count(&seq);

    assert_eq!(counter.get("TN"), 1);
    assert_eq!(counter.get("NG"), 1);
    assert_eq!(counter.total_kmers(), 4);
}

#[test]
fn test_most_frequent() {
    let seq = Sequence::new("ATGATGATG").unwrap();
    let mut counter = KMerCounter::new(3);
    counter.count(&seq);

    let top = counter.most_frequent(1);
    assert_eq!(top.len(), 1);
    assert_eq!(top[0].0, "ATG");
    assert_eq!(top[0].1, 3);
}

#[test]
fn test_least_frequent() {
    let seq = Sequence::new("ATGATGCCC").unwrap();
    let mut counter = KMerCounter::new(3);
    counter.count(&seq);

    let least = counter.least_frequent(3);
    // All should have count 1 or 2
    assert!(least.iter().all(|(_, count)| *count <= 2));
}

#[test]
fn test_frequency() {
    let seq = Sequence::new("AAAA").unwrap();
    let mut counter = KMerCounter::new(2);
    counter.count(&seq);

    assert!((counter.frequency("AA") - 1.0).abs() < f64::EPSILON);
}

#[test]
fn test_get_nonexistent() {
    let seq = Sequence::new("ATGC").unwrap();
    let mut counter = KMerCounter::new(2);
    counter.count(&seq);

    assert_eq!(counter.get("XX"), 0);
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
    assert_eq!(counter1.total_kmers(), 6);
}

#[test]
#[should_panic]
fn test_merge_different_k() {
    let seq = Sequence::new("ATGC").unwrap();

    let mut counter1 = KMerCounter::new(2);
    counter1.count(&seq);

    let mut counter2 = KMerCounter::new(3);
    counter2.count(&seq);

    counter1.merge(&counter2);
}

#[test]
fn test_clear() {
    let seq = Sequence::new("ATGC").unwrap();
    let mut counter = KMerCounter::new(2);
    counter.count(&seq);

    assert!(counter.total_kmers() > 0);

    counter.clear();

    assert_eq!(counter.total_kmers(), 0);
    assert_eq!(counter.unique_kmers(), 0);
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

#[test]
fn test_saturation() {
    let seq = Sequence::new("ATGC".repeat(100)).unwrap();
    let mut counter = KMerCounter::new(2);
    counter.count(&seq);

    let saturation = counter.saturation();
    // With k=2, we should see at least some k-mers
    // The ATGC repeat gives us AT, TG, GC, CA = 4 unique k-mers
    // 4/16 = 0.25
    assert!(saturation > 0.0);
    assert!(saturation <= 1.0);
}

#[test]
fn test_expected_unique_kmers() {
    let counter = KMerCounter::new(2);
    assert_eq!(counter.expected_unique_kmers(), 16); // 4^2

    let counter = KMerCounter::new(3);
    assert_eq!(counter.expected_unique_kmers(), 64); // 4^3
}

#[test]
fn test_count_all() {
    let seq1 = Sequence::new("ATGC").unwrap();
    let seq2 = Sequence::new("ATGC").unwrap();
    let seqs = vec![seq1, seq2];

    let mut counter = KMerCounter::new(2);
    counter.count_all(seqs.iter());

    assert_eq!(counter.get("AT"), 2);
}

#[test]
fn test_all_kmers() {
    let seq = Sequence::new("ATGC").unwrap();
    let mut counter = KMerCounter::new(2);
    counter.count(&seq);

    let all = counter.all_kmers();
    assert_eq!(all.len(), 3); // AT, TG, GC
}

#[test]
fn test_kmers_in_range() {
    let seq = Sequence::new("ATGATGATG").unwrap();
    let mut counter = KMerCounter::new(3);
    counter.count(&seq);

    let in_range = counter.kmers_in_range(2, 3);
    // TGA and GAT have count 2, ATG has count 3
    assert!(in_range.len() >= 2);
}

#[test]
fn test_generate_all_kmers() {
    let kmers = generate_all_kmers(2);
    assert_eq!(kmers.len(), 16);

    let kmers = generate_all_kmers(3);
    assert_eq!(kmers.len(), 64);

    let kmers = generate_all_kmers(0);
    assert_eq!(kmers.len(), 1); // Empty string
}

#[test]
fn test_jaccard_similarity_identical() {
    let seq = Sequence::new("ATGC").unwrap();

    let mut counter1 = KMerCounter::new(2);
    counter1.count(&seq);

    let mut counter2 = KMerCounter::new(2);
    counter2.count(&seq);

    let similarity = jaccard_similarity(&counter1, &counter2);
    assert!((similarity - 1.0).abs() < f64::EPSILON);
}

#[test]
fn test_jaccard_similarity_different() {
    let seq1 = Sequence::new("AAAA").unwrap();
    let seq2 = Sequence::new("CCCC").unwrap();

    let mut counter1 = KMerCounter::new(2);
    counter1.count(&seq1);

    let mut counter2 = KMerCounter::new(2);
    counter2.count(&seq2);

    let similarity = jaccard_similarity(&counter1, &counter2);
    assert!((similarity - 0.0).abs() < f64::EPSILON);
}

#[test]
fn test_bray_curtis_identical() {
    let seq = Sequence::new("ATGC").unwrap();

    let mut counter1 = KMerCounter::new(2);
    counter1.count(&seq);

    let mut counter2 = KMerCounter::new(2);
    counter2.count(&seq);

    let dissimilarity = bray_curtis_dissimilarity(&counter1, &counter2);
    assert!((dissimilarity - 0.0).abs() < f64::EPSILON);
}

#[test]
fn test_canonical_kmer_counter() {
    // ATG and CAT are reverse complements
    let seq = Sequence::new("ATGCAT").unwrap();
    let mut counter = CanonicalKMerCounter::new(3);
    counter.count(&seq);

    // Both ATG and CAT should contribute to the same canonical k-mer
    let top = counter.most_frequent(10);
    // Check that we counted k-mers
    assert!(counter.total_kmers() > 0);
}

#[test]
fn test_sequence_shorter_than_k() {
    let seq = Sequence::new("AT").unwrap();
    let mut counter = KMerCounter::new(5);
    counter.count(&seq);

    assert_eq!(counter.total_kmers(), 0);
}

#[test]
fn test_large_k() {
    let seq = Sequence::new("ATGC".repeat(100)).unwrap();
    let mut counter = KMerCounter::new(31);
    counter.count(&seq);

    assert!(counter.total_kmers() > 0);
}

#[test]
fn test_iterator() {
    let seq = Sequence::new("ATGC").unwrap();
    let mut counter = KMerCounter::new(2);
    counter.count(&seq);

    let count: usize = counter.iter().count();
    assert_eq!(count, 3);
}
