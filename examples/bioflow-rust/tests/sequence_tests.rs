//! Integration tests for the sequence module.

use bioflow_rust::sequence::*;

#[test]
fn test_sequence_creation_basic() {
    let seq = Sequence::new("ATGC").unwrap();
    assert_eq!(seq.len(), 4);
    assert_eq!(seq.bases(), "ATGC");
}

#[test]
fn test_sequence_creation_lowercase() {
    let seq = Sequence::new("atgc").unwrap();
    assert_eq!(seq.bases(), "ATGC");
}

#[test]
fn test_sequence_creation_with_n() {
    let seq = Sequence::new("ATNGC").unwrap();
    assert!(seq.bases().contains('N'));
}

#[test]
fn test_sequence_empty_error() {
    let result = Sequence::new("");
    assert!(matches!(result, Err(SequenceError::EmptySequence)));
}

#[test]
fn test_sequence_invalid_base_error() {
    let result = Sequence::new("ATXGC");
    match result {
        Err(SequenceError::InvalidBase { position, base }) => {
            assert_eq!(position, 2);
            assert_eq!(base, 'X');
        }
        _ => panic!("Expected InvalidBase error"),
    }
}

#[test]
fn test_sequence_with_id() {
    let seq = Sequence::with_id("ATGC", "test_seq").unwrap();
    assert_eq!(seq.id(), Some("test_seq"));
}

#[test]
fn test_gc_content_50_percent() {
    let seq = Sequence::new("ATGC").unwrap();
    assert!((seq.gc_content() - 0.5).abs() < f64::EPSILON);
}

#[test]
fn test_gc_content_100_percent() {
    let seq = Sequence::new("GGCC").unwrap();
    assert!((seq.gc_content() - 1.0).abs() < f64::EPSILON);
}

#[test]
fn test_gc_content_0_percent() {
    let seq = Sequence::new("AATT").unwrap();
    assert!((seq.gc_content() - 0.0).abs() < f64::EPSILON);
}

#[test]
fn test_complement_dna() {
    let seq = Sequence::new("ATGC").unwrap();
    assert_eq!(seq.complement().bases(), "TACG");
}

#[test]
fn test_reverse() {
    let seq = Sequence::new("ATGC").unwrap();
    assert_eq!(seq.reverse().bases(), "CGTA");
}

#[test]
fn test_reverse_complement() {
    let seq = Sequence::new("ATGC").unwrap();
    assert_eq!(seq.reverse_complement().bases(), "GCAT");
}

#[test]
fn test_reverse_complement_property() {
    // Reverse complement of reverse complement should equal original
    let seq = Sequence::new("ATGCGATCGA").unwrap();
    let double_revcomp = seq.reverse_complement().reverse_complement();
    assert_eq!(double_revcomp.bases(), seq.bases());
}

#[test]
fn test_transcribe_dna_to_rna() {
    let dna = Sequence::new("ATGC").unwrap();
    let rna = dna.transcribe();
    assert_eq!(rna.bases(), "AUGC");
    assert_eq!(rna.sequence_type(), SequenceType::RNA);
}

#[test]
fn test_base_composition() {
    let seq = Sequence::new("AACCGGTT").unwrap();
    let comp = seq.base_composition();

    assert_eq!(comp.a_count, 2);
    assert_eq!(comp.c_count, 2);
    assert_eq!(comp.g_count, 2);
    assert_eq!(comp.t_count, 2);
    assert_eq!(comp.n_count, 0);

    assert!((comp.a_freq - 0.25).abs() < f64::EPSILON);
    assert!((comp.gc_content() - 0.5).abs() < f64::EPSILON);
}

#[test]
fn test_subsequence() {
    let seq = Sequence::new("ATGCATGC").unwrap();
    let sub = seq.subsequence(2, 6).unwrap();
    assert_eq!(sub.bases(), "GCAT");
}

#[test]
fn test_subsequence_out_of_bounds() {
    let seq = Sequence::new("ATGC").unwrap();
    let result = seq.subsequence(2, 10);
    assert!(result.is_err());
}

#[test]
fn test_find_pattern() {
    let seq = Sequence::new("ATGATGATG").unwrap();
    let positions = seq.find_pattern("ATG");
    assert_eq!(positions, vec![0, 3, 6]);
}

#[test]
fn test_find_pattern_not_found() {
    let seq = Sequence::new("ATGATGATG").unwrap();
    let positions = seq.find_pattern("CCC");
    assert!(positions.is_empty());
}

#[test]
fn test_find_pattern_case_insensitive() {
    let seq = Sequence::new("ATGATGATG").unwrap();
    let positions = seq.find_pattern("atg");
    assert_eq!(positions, vec![0, 3, 6]);
}

#[test]
fn test_molecular_weight() {
    let seq = Sequence::new("ATGC").unwrap();
    let mw = seq.molecular_weight();
    // Should be positive and reasonable
    assert!(mw > 0.0);
    assert!(mw < 10000.0);
}

#[test]
fn test_melting_temperature_short() {
    let seq = Sequence::new("ATGC").unwrap();
    let tm = seq.melting_temperature();
    // Wallace rule for short sequences
    assert!(tm > 0.0);
    assert!(tm < 100.0);
}

#[test]
fn test_windows() {
    let seq = Sequence::new("ATGCATGC").unwrap();
    let windows: Vec<&str> = seq.windows(3).collect();
    assert_eq!(windows.len(), 6);
    assert_eq!(windows[0], "ATG");
    assert_eq!(windows[1], "TGC");
}

#[test]
fn test_sequence_display() {
    let mut seq = Sequence::new("ATGC").unwrap();
    seq.set_id("test");
    seq.set_description("Test sequence");

    let display = format!("{}", seq);
    assert!(display.contains(">test"));
    assert!(display.contains("ATGC"));
}

#[test]
fn test_long_sequence() {
    let long = "ATGC".repeat(10000);
    let seq = Sequence::new(&long).unwrap();
    assert_eq!(seq.len(), 40000);

    // Operations should still work
    let gc = seq.gc_content();
    assert!((gc - 0.5).abs() < f64::EPSILON);
}

#[test]
fn test_complement_with_n() {
    let seq = Sequence::new("ATNGC").unwrap();
    let comp = seq.complement();
    assert_eq!(comp.bases(), "TANCG");
}

#[test]
fn test_sequence_equality() {
    let seq1 = Sequence::new("ATGC").unwrap();
    let seq2 = Sequence::new("ATGC").unwrap();
    let seq3 = Sequence::new("ATGG").unwrap();

    assert_eq!(seq1, seq2);
    assert_ne!(seq1, seq3);
}

#[test]
fn test_sequence_clone() {
    let seq1 = Sequence::with_id("ATGC", "test").unwrap();
    let seq2 = seq1.clone();

    assert_eq!(seq1.bases(), seq2.bases());
    assert_eq!(seq1.id(), seq2.id());
}
