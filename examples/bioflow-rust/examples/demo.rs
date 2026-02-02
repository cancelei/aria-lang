//! Demo example showcasing BioFlow Rust features.
//!
//! Run with: cargo run --example demo

use bioflow_rust::*;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║           BioFlow Rust - Production Bioinformatics           ║");
    println!("║                                                              ║");
    println!("║  Showcasing Rust's Safety and Performance Features           ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    demo_sequence_safety()?;
    demo_ownership_borrowing()?;
    demo_error_handling()?;
    demo_zero_cost_abstractions()?;
    demo_parallel_processing()?;
    demo_performance_comparison()?;

    println!();
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                    Demo Complete!                            ║");
    println!("╚══════════════════════════════════════════════════════════════╝");

    Ok(())
}

fn demo_sequence_safety() -> Result<(), Box<dyn std::error::Error>> {
    println!("┌──────────────────────────────────────────────────────────────┐");
    println!("│ 1. SEQUENCE VALIDATION AND SAFETY                           │");
    println!("└──────────────────────────────────────────────────────────────┘");
    println!();

    // Valid sequence
    println!("Creating a valid sequence...");
    let seq = Sequence::new("ATGCGATCGATCGATCG")?;
    println!("  Sequence: {}", seq.bases());
    println!("  Length: {} bp", seq.len());
    println!("  GC Content: {:.1}%", seq.gc_content() * 100.0);
    println!();

    // Invalid sequence - caught at construction time!
    println!("Attempting to create an invalid sequence...");
    match Sequence::new("ATGXYZ") {
        Ok(_) => println!("  Unexpected success!"),
        Err(e) => println!("  Error caught: {}", e),
    }
    println!();

    // Empty sequence
    println!("Attempting to create an empty sequence...");
    match Sequence::new("") {
        Ok(_) => println!("  Unexpected success!"),
        Err(e) => println!("  Error caught: {}", e),
    }
    println!();

    println!("  Key Rust Feature: Errors are caught at construction time,");
    println!("  not during analysis. This is enforced by the type system.");
    println!();

    Ok(())
}

fn demo_ownership_borrowing() -> Result<(), Box<dyn std::error::Error>> {
    println!("┌──────────────────────────────────────────────────────────────┐");
    println!("│ 2. OWNERSHIP AND BORROWING                                  │");
    println!("└──────────────────────────────────────────────────────────────┘");
    println!();

    let seq = Sequence::new("ATGCGATCGATCGATCG")?;

    // Borrowing: multiple readers simultaneously
    println!("Borrowing for analysis (immutable references):");
    let gc = seq.gc_content();      // Borrow &seq
    let len = seq.len();             // Borrow &seq again
    let comp = seq.base_composition(); // Borrow &seq once more
    println!("  GC: {:.1}%, Length: {}, A count: {}", gc * 100.0, len, comp.a_count);
    println!();

    // Original still usable
    println!("Original sequence still usable: {}", seq.bases());
    println!();

    // Moving: transfer ownership
    println!("Moving ownership:");
    let complement = seq.complement(); // Returns new Sequence
    println!("  Original: {}", seq.bases());      // Still valid!
    println!("  Complement: {}", complement.bases());
    println!();

    println!("  Key Rust Feature: The borrow checker ensures memory safety");
    println!("  at compile time with zero runtime overhead.");
    println!();

    Ok(())
}

fn demo_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    println!("┌──────────────────────────────────────────────────────────────┐");
    println!("│ 3. ERROR HANDLING WITH RESULT TYPES                         │");
    println!("└──────────────────────────────────────────────────────────────┘");
    println!();

    // Using ? operator for propagation
    println!("Error propagation with ? operator:");

    fn analyze_sequence(bases: &str) -> Result<f64, SequenceError> {
        let seq = Sequence::new(bases)?;  // Propagate error if invalid
        Ok(seq.gc_content())
    }

    match analyze_sequence("ATGCATGC") {
        Ok(gc) => println!("  Valid sequence - GC: {:.1}%", gc * 100.0),
        Err(e) => println!("  Error: {}", e),
    }

    match analyze_sequence("INVALID") {
        Ok(gc) => println!("  Valid sequence - GC: {:.1}%", gc * 100.0),
        Err(e) => println!("  Error: {}", e),
    }
    println!();

    // Pattern matching on errors
    println!("Pattern matching on error types:");
    match Sequence::new("AT@GC") {
        Err(SequenceError::InvalidBase { position, base }) => {
            println!("  Invalid base '{}' at position {}", base, position);
        }
        Err(SequenceError::EmptySequence) => {
            println!("  Sequence was empty");
        }
        _ => {}
    }
    println!();

    println!("  Key Rust Feature: Result<T, E> makes error handling explicit");
    println!("  and forces developers to handle all error cases.");
    println!();

    Ok(())
}

fn demo_zero_cost_abstractions() -> Result<(), Box<dyn std::error::Error>> {
    println!("┌──────────────────────────────────────────────────────────────┐");
    println!("│ 4. ZERO-COST ABSTRACTIONS                                   │");
    println!("└──────────────────────────────────────────────────────────────┘");
    println!();

    let seq = Sequence::new("ATGCGATCGATCGATCGATCGATCGATCGATCG")?;

    // Iterator chains compile to optimal loops
    println!("Iterator chains (compile to optimal machine code):");

    let gc_count: usize = seq.bases()
        .bytes()
        .filter(|&b| b == b'G' || b == b'C')
        .count();
    println!("  GC bases (iterator): {}", gc_count);

    // Window iteration
    let mut counter = KMerCounter::new(3);
    counter.count(&seq);
    let top_kmers: Vec<_> = counter.most_frequent(3);
    println!("  Top 3-mers: {:?}", top_kmers.iter().map(|(k, _)| k).collect::<Vec<_>>());
    println!();

    // Functional style with map/filter/collect
    println!("Functional transformations:");
    let positions: Vec<usize> = seq.find_pattern("ATG");
    println!("  ATG positions: {:?}", positions);

    let high_gc_windows: Vec<f64> = seq.windows(10)
        .map(|w| gc_content(w))
        .filter(|&gc| gc > 0.5)
        .collect();
    println!("  Windows with >50% GC: {} out of {}", high_gc_windows.len(), seq.len() - 9);
    println!();

    println!("  Key Rust Feature: High-level abstractions compile to the same");
    println!("  assembly as hand-written loops, with no runtime overhead.");
    println!();

    Ok(())
}

fn demo_parallel_processing() -> Result<(), Box<dyn std::error::Error>> {
    println!("┌──────────────────────────────────────────────────────────────┐");
    println!("│ 5. THREAD SAFETY (RAYON PARALLELISM)                        │");
    println!("└──────────────────────────────────────────────────────────────┘");
    println!();

    use rayon::prelude::*;

    // Create multiple sequences
    let sequences: Vec<_> = (0..100)
        .map(|i| Sequence::new(format!("ATGC{}", "ATGC".repeat(100 + i))).unwrap())
        .collect();

    println!("Parallel GC content calculation:");
    let start = Instant::now();

    let gc_values: Vec<f64> = sequences.par_iter()
        .map(|seq| seq.gc_content())
        .collect();

    let duration = start.elapsed();
    let avg_gc: f64 = gc_values.iter().sum::<f64>() / gc_values.len() as f64;

    println!("  Processed {} sequences in {:?}", sequences.len(), duration);
    println!("  Average GC content: {:.2}%", avg_gc * 100.0);
    println!();

    println!("Parallel k-mer counting:");
    let start = Instant::now();

    let kmer_counts: Vec<_> = sequences.par_iter()
        .map(|seq| {
            let mut counter = KMerCounter::new(5);
            counter.count(seq);
            counter.unique_kmers()
        })
        .collect();

    let duration = start.elapsed();
    let total_unique: usize = kmer_counts.iter().sum();

    println!("  Counted k-mers in {:?}", duration);
    println!("  Total unique 5-mers across all: {}", total_unique);
    println!();

    println!("  Key Rust Feature: The Send and Sync traits ensure thread safety");
    println!("  at compile time. Data races are impossible.");
    println!();

    Ok(())
}

fn demo_performance_comparison() -> Result<(), Box<dyn std::error::Error>> {
    println!("┌──────────────────────────────────────────────────────────────┐");
    println!("│ 6. PERFORMANCE BENCHMARKS                                   │");
    println!("└──────────────────────────────────────────────────────────────┘");
    println!();

    // Create test sequences
    let small = Sequence::new("ATGC".repeat(1000))?;      // 4 KB
    let medium = Sequence::new("ATGC".repeat(10000))?;    // 40 KB
    let large = Sequence::new("ATGC".repeat(100000))?;    // 400 KB

    println!("Sequence sizes: 4KB, 40KB, 400KB");
    println!();

    // GC Content
    println!("GC Content Calculation:");
    for (name, seq) in [("4KB", &small), ("40KB", &medium), ("400KB", &large)] {
        let start = Instant::now();
        for _ in 0..100 {
            let _ = seq.gc_content();
        }
        let duration = start.elapsed() / 100;
        println!("  {}: {:?} per call", name, duration);
    }
    println!();

    // K-mer counting
    println!("K-mer Counting (k=21):");
    for (name, seq) in [("4KB", &small), ("40KB", &medium)] {
        let start = Instant::now();
        for _ in 0..10 {
            let mut counter = KMerCounter::new(21);
            counter.count(seq);
        }
        let duration = start.elapsed() / 10;
        println!("  {}: {:?} per call", name, duration);
    }
    println!();

    // Alignment (smaller sequences for O(n*m) algorithm)
    println!("Smith-Waterman Alignment:");
    let align_sizes = [(100, "100bp"), (500, "500bp"), (1000, "1KB")];
    let scoring = ScoringMatrix::default();

    for (size, name) in align_sizes {
        let seq1 = Sequence::new("ACGT".repeat(size / 4))?;
        let seq2 = Sequence::new("AGCT".repeat(size / 4))?;

        let start = Instant::now();
        for _ in 0..5 {
            let _ = smith_waterman(&seq1, &seq2, &scoring);
        }
        let duration = start.elapsed() / 5;
        println!("  {} x {}: {:?} per call", name, name, duration);
    }
    println!();

    // Memory efficiency
    println!("Memory Efficiency:");
    let seq = Sequence::new("ATGC".repeat(250000))?; // 1 MB
    println!("  1 MB sequence created");
    println!("  Base composition: {:?}", seq.base_composition().a_count);
    println!("  No copying required for analysis operations");
    println!();

    println!("  Key Rust Feature: Predictable performance with no garbage");
    println!("  collection pauses. Memory is freed immediately when dropped.");
    println!();

    Ok(())
}
