//! BioFlow CLI - Command-line interface for bioinformatics analysis.

use bioflow_rust::*;
use clap::{Parser, Subcommand};
use std::fs;
use std::time::Instant;

#[derive(Parser)]
#[command(name = "bioflow")]
#[command(author = "Aria Language Team")]
#[command(version = VERSION)]
#[command(about = "A production-quality bioinformatics toolkit", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze sequence composition
    Analyze {
        /// Input sequence or file path
        input: String,

        /// Treat input as a file path
        #[arg(short, long)]
        file: bool,
    },

    /// Count k-mers in sequences
    Kmer {
        /// Input sequence or file path
        input: String,

        /// K-mer size
        #[arg(short, long, default_value = "21")]
        k: usize,

        /// Number of top k-mers to display
        #[arg(short, long, default_value = "10")]
        top: usize,

        /// Treat input as a file path
        #[arg(short, long)]
        file: bool,
    },

    /// Align two sequences
    Align {
        /// First sequence
        seq1: String,

        /// Second sequence
        seq2: String,

        /// Use global alignment (Needleman-Wunsch) instead of local (Smith-Waterman)
        #[arg(short, long)]
        global: bool,

        /// Match score
        #[arg(long, default_value = "2")]
        match_score: i32,

        /// Mismatch penalty
        #[arg(long, default_value = "-1")]
        mismatch: i32,

        /// Gap penalty
        #[arg(long, default_value = "-2")]
        gap: i32,
    },

    /// Calculate quality statistics
    Quality {
        /// Quality string (Phred+33 encoded)
        quality: String,
    },

    /// Generate random sequence
    Random {
        /// Length of sequence to generate
        length: usize,

        /// Target GC content (0.0 to 1.0)
        #[arg(long, default_value = "0.5")]
        gc: f64,
    },

    /// Run a demo showing library capabilities
    Demo,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Analyze { input, file } => {
            let sequence = if file {
                read_sequence_from_file(&input)?
            } else {
                input
            };
            analyze_sequence(&sequence, cli.verbose)?;
        }

        Commands::Kmer { input, k, top, file } => {
            let sequence = if file {
                read_sequence_from_file(&input)?
            } else {
                input
            };
            count_kmers(&sequence, k, top, cli.verbose)?;
        }

        Commands::Align {
            seq1,
            seq2,
            global,
            match_score,
            mismatch,
            gap,
        } => {
            align_sequences(&seq1, &seq2, global, match_score, mismatch, gap, cli.verbose)?;
        }

        Commands::Quality { quality } => {
            analyze_quality(&quality)?;
        }

        Commands::Random { length, gc } => {
            generate_random(length, gc)?;
        }

        Commands::Demo => {
            run_demo()?;
        }
    }

    Ok(())
}

fn read_sequence_from_file(path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;

    // Simple FASTA parser
    let mut sequence = String::new();
    for line in content.lines() {
        if !line.starts_with('>') && !line.is_empty() {
            sequence.push_str(line.trim());
        }
    }

    if sequence.is_empty() {
        // Not FASTA, treat as raw sequence
        sequence = content.chars().filter(|c| !c.is_whitespace()).collect();
    }

    Ok(sequence)
}

fn analyze_sequence(input: &str, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();
    let seq = Sequence::new(input)?;
    let creation_time = start.elapsed();

    println!("=== Sequence Analysis ===");
    println!();

    if let Some(id) = seq.id() {
        println!("ID: {}", id);
    }
    println!("Length: {} bp", seq.len());
    println!();

    println!("=== Base Composition ===");
    let comp = seq.base_composition();
    println!("A: {:>8} ({:>5.1}%)", comp.a_count, comp.a_freq * 100.0);
    println!("C: {:>8} ({:>5.1}%)", comp.c_count, comp.c_freq * 100.0);
    println!("G: {:>8} ({:>5.1}%)", comp.g_count, comp.g_freq * 100.0);
    println!("T: {:>8} ({:>5.1}%)", comp.t_count, comp.t_freq * 100.0);
    if comp.n_count > 0 {
        println!("N: {:>8} ({:>5.1}%)", comp.n_count, comp.n_freq * 100.0);
    }
    println!();

    println!("=== Statistics ===");
    println!("GC Content: {:.2}%", seq.gc_content() * 100.0);
    println!("Molecular Weight: {:.2} Da", seq.molecular_weight());
    println!("Melting Temperature: {:.1} C", seq.melting_temperature());
    println!();

    if verbose {
        println!("=== Derived Sequences ===");
        let complement = seq.complement();
        let revcomp = seq.reverse_complement();

        println!("Complement (first 50bp):");
        println!("  {}", &complement.bases()[..complement.len().min(50)]);
        println!("Reverse Complement (first 50bp):");
        println!("  {}", &revcomp.bases()[..revcomp.len().min(50)]);
        println!();

        println!("=== Performance ===");
        println!("Sequence creation: {:?}", creation_time);
    }

    Ok(())
}

fn count_kmers(
    input: &str,
    k: usize,
    top: usize,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let seq = Sequence::new(input)?;

    let start = Instant::now();
    let mut counter = KMerCounter::new(k);
    counter.count(&seq);
    let duration = start.elapsed();

    println!("=== K-mer Analysis (k={}) ===", k);
    println!();
    println!("Sequence length: {} bp", seq.len());
    println!("Total k-mers: {}", counter.total_kmers());
    println!("Unique k-mers: {}", counter.unique_kmers());
    println!(
        "Saturation: {:.2}% of possible k-mers",
        counter.saturation() * 100.0
    );
    println!("Entropy: {:.2} bits", counter.entropy());
    println!();

    println!("=== Top {} K-mers ===", top);
    for (kmer, count) in counter.most_frequent(top) {
        let freq = counter.frequency(&kmer);
        println!("{}: {} ({:.2}%)", kmer, count, freq * 100.0);
    }

    if verbose {
        println!();
        println!("=== Performance ===");
        println!("K-mer counting: {:?}", duration);
        println!(
            "Rate: {:.2} k-mers/sec",
            counter.total_kmers() as f64 / duration.as_secs_f64()
        );
    }

    Ok(())
}

fn align_sequences(
    seq1_str: &str,
    seq2_str: &str,
    global: bool,
    match_score: i32,
    mismatch: i32,
    gap: i32,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let seq1 = Sequence::new(seq1_str)?;
    let seq2 = Sequence::new(seq2_str)?;
    let scoring = ScoringMatrix::new(match_score, mismatch, gap);

    let start = Instant::now();
    let alignment = if global {
        needleman_wunsch(&seq1, &seq2, &scoring)
    } else {
        smith_waterman(&seq1, &seq2, &scoring)
    };
    let duration = start.elapsed();

    let alignment_name = if global { "Global" } else { "Local" };
    println!("=== {} Alignment ===", alignment_name);
    println!();

    println!("Scoring: match={}, mismatch={}, gap={}", match_score, mismatch, gap);
    println!();

    println!("{}", alignment);

    if verbose {
        println!("=== Performance ===");
        println!("Alignment time: {:?}", duration);
        println!(
            "Matrix size: {}x{}",
            seq1.len() + 1,
            seq2.len() + 1
        );
    }

    Ok(())
}

fn analyze_quality(quality_str: &str) -> Result<(), Box<dyn std::error::Error>> {
    let quality = QualityScores::from_phred33(quality_str)?;

    println!("=== Quality Analysis ===");
    println!();
    println!("Length: {} bases", quality.len());
    println!();

    println!("=== Quality Statistics ===");
    println!("Mean quality: {:.1}", quality.mean());
    println!("Median quality: {:.1}", quality.median());
    println!("Min quality: {}", quality.min());
    println!("Max quality: {}", quality.max());
    println!();

    println!("=== Quality Distribution ===");
    println!("Q >= 30 (high quality): {:.1}%", quality.fraction_above(30) * 100.0);
    println!("Q >= 20 (medium quality): {:.1}%", quality.fraction_above(20) * 100.0);
    println!("Q >= 10 (low quality): {:.1}%", quality.fraction_above(10) * 100.0);
    println!();

    println!("=== Trimming Suggestion ===");
    let (start, end) = quality.trim_ends(20);
    println!("Trim positions (Q >= 20): {} to {} ({} bases)", start, end, end - start);

    let mean_error = quality.mean_error_probability();
    println!("Mean error probability: {:.4}%", mean_error * 100.0);

    Ok(())
}

fn generate_random(length: usize, gc: f64) -> Result<(), Box<dyn std::error::Error>> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::SystemTime;

    // Simple pseudo-random number generator using system time
    let mut hasher = DefaultHasher::new();
    SystemTime::now().hash(&mut hasher);
    let mut seed = hasher.finish();

    let gc_threshold = (gc * 1000.0) as u64;
    let mut sequence = String::with_capacity(length);

    for _ in 0..length {
        // Simple LCG random number generator
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let rand = seed % 1000;

        let base = if rand < gc_threshold {
            if seed % 2 == 0 { 'G' } else { 'C' }
        } else {
            if seed % 2 == 0 { 'A' } else { 'T' }
        };

        sequence.push(base);
    }

    let seq = Sequence::new(&sequence)?;
    println!(">random_seq length={} target_gc={:.2}", length, gc);
    println!("{}", seq.bases());

    // Verify GC content
    eprintln!("# Actual GC content: {:.2}%", seq.gc_content() * 100.0);

    Ok(())
}

fn run_demo() -> Result<(), Box<dyn std::error::Error>> {
    println!("===========================================");
    println!("    BioFlow Rust - Feature Demonstration");
    println!("===========================================");
    println!();

    // Demo 1: Sequence creation and analysis
    println!("1. SEQUENCE ANALYSIS");
    println!("   -----------------");

    let seq = Sequence::new("ATGCGATCGATCGATCGAATTCCGGAATTCCGG")?;
    println!("   Sequence: {}", seq.bases());
    println!("   Length: {} bp", seq.len());
    println!("   GC Content: {:.1}%", seq.gc_content() * 100.0);

    let comp = seq.base_composition();
    println!("   Composition: A={}, C={}, G={}, T={}",
        comp.a_count, comp.c_count, comp.g_count, comp.t_count);
    println!();

    // Demo 2: Sequence transformations
    println!("2. SEQUENCE TRANSFORMATIONS");
    println!("   ------------------------");

    let short_seq = Sequence::new("ATGCATGC")?;
    println!("   Original:           {}", short_seq.bases());
    println!("   Complement:         {}", short_seq.complement().bases());
    println!("   Reverse:            {}", short_seq.reverse().bases());
    println!("   Reverse Complement: {}", short_seq.reverse_complement().bases());
    println!("   RNA Transcription:  {}", short_seq.transcribe().bases());
    println!();

    // Demo 3: K-mer counting
    println!("3. K-MER COUNTING");
    println!("   --------------");

    let kmer_seq = Sequence::new("ATGATGATGATGATGATG")?;
    let mut counter = KMerCounter::new(3);
    counter.count(&kmer_seq);

    println!("   Sequence: {}", kmer_seq.bases());
    println!("   K-mer size: 3");
    println!("   Total k-mers: {}", counter.total_kmers());
    println!("   Unique k-mers: {}", counter.unique_kmers());
    println!("   Top 3 k-mers:");
    for (kmer, count) in counter.most_frequent(3) {
        println!("     {}: {}", kmer, count);
    }
    println!();

    // Demo 4: Sequence alignment
    println!("4. SEQUENCE ALIGNMENT (Smith-Waterman)");
    println!("   -----------------------------------");

    let align_seq1 = Sequence::new("ACGTACGTACGT")?;
    let align_seq2 = Sequence::new("ACGTTCGTACGT")?;
    let scoring = ScoringMatrix::default();

    println!("   Seq1: {}", align_seq1.bases());
    println!("   Seq2: {}", align_seq2.bases());

    let alignment = smith_waterman(&align_seq1, &align_seq2, &scoring);
    println!("   Score: {}", alignment.score);
    println!("   Identity: {:.1}%", alignment.identity() * 100.0);
    println!("   Matches: {}, Mismatches: {}, Gaps: {}",
        alignment.matches, alignment.mismatches, alignment.gaps);
    println!();
    println!("   Alignment:");
    println!("   {}", alignment.aligned_seq1);
    println!("   {}", alignment.aligned_seq2);
    println!();

    // Demo 5: Quality scores
    println!("5. QUALITY SCORE ANALYSIS");
    println!("   ----------------------");

    let quality = QualityScores::from_phred33("IIIIIIIII!!!!IIIII")?;
    println!("   Quality string: {}", quality.raw());
    println!("   Mean quality: {:.1}", quality.mean());
    println!("   Min/Max: {}/{}", quality.min(), quality.max());
    println!("   High quality (Q>=30): {:.1}%", quality.high_quality_fraction() * 100.0);
    println!();

    // Demo 6: Performance showcase
    println!("6. PERFORMANCE BENCHMARK");
    println!("   ---------------------");

    let large_seq = Sequence::new("ATGC".repeat(5000))?;
    println!("   Sequence length: {} bp", large_seq.len());

    let start = Instant::now();
    let _ = large_seq.gc_content();
    println!("   GC content calculation: {:?}", start.elapsed());

    let start = Instant::now();
    let mut counter = KMerCounter::new(21);
    counter.count(&large_seq);
    println!("   21-mer counting: {:?}", start.elapsed());

    let seq_1kb = Sequence::new("ACGT".repeat(250))?;
    let seq_1kb_2 = Sequence::new("AGCT".repeat(250))?;

    let start = Instant::now();
    let _ = smith_waterman(&seq_1kb, &seq_1kb_2, &ScoringMatrix::default());
    println!("   1kb alignment: {:?}", start.elapsed());
    println!();

    // Summary
    println!("===========================================");
    println!("   Rust Features Demonstrated:");
    println!("   - Ownership and borrowing (compile-time safety)");
    println!("   - Result types for error handling");
    println!("   - Zero-cost abstractions");
    println!("   - Strong type system");
    println!("   - Iterator chains");
    println!("===========================================");

    Ok(())
}
