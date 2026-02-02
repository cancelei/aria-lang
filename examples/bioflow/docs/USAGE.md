# BioFlow Usage Guide

## Getting Started

BioFlow is a bioinformatics pipeline framework for the Aria programming language. This guide covers basic usage patterns.

## Basic Operations

### Creating Sequences

```aria
import core::sequence::{Sequence, SequenceType}

# Create a DNA sequence
let seq = Sequence::new("ATGCGATCGATCGATCG").unwrap()

# Create with ID
let seq_with_id = Sequence::with_id("ATGCGATC", "my_sequence").unwrap()

# Create with full metadata
let full_seq = Sequence::with_metadata(
  "ATGCGATC",
  "seq_001",
  "Sample sequence for testing",
  SequenceType::DNA
).unwrap()
```

### Sequence Analysis

```aria
# Length
println("Length: " + seq.len().to_string())

# GC Content
println("GC: " + (seq.gc_content() * 100.0).to_string() + "%")

# Base counts
let (a, c, g, t, n) = seq.base_counts()
println("A: " + a.to_string())

# Check for ambiguous bases
if seq.has_ambiguous()
  println("Contains N bases")
end
```

### Transformations

```aria
# Complement (A<->T, C<->G)
let comp = seq.complement()

# Reverse complement
let rc = seq.reverse_complement()

# Transcribe to RNA
let rna = seq.transcribe()

# Concatenate
let combined = seq1.concat(seq2)

# Subsequence
let sub = seq.subsequence(10, 20).unwrap()
```

### Motif Finding

```aria
# Check if motif exists
if seq.contains_motif("GATC")
  println("Found GATC!")
end

# Find all positions
let positions = seq.find_motif_positions("ATG")
for pos in positions
  println("Found at position: " + pos.to_string())
end
```

## Working with Quality Scores

### Creating Quality Scores

```aria
import core::quality::{QualityScores, QualityCategory}

# From integer array
let scores = QualityScores::new([30, 35, 40, 38, 32]).unwrap()

# From Phred+33 encoded string (FASTQ format)
let qual = QualityScores::from_phred33("IIIIIIIIIII").unwrap()

# From Phred+64 encoded string (older format)
let qual64 = QualityScores::from_phred64("hhhhhhhhhhh").unwrap()
```

### Quality Analysis

```aria
# Statistics
println("Average: " + qual.average().to_string())
println("Min: " + qual.min().to_string())
println("Max: " + qual.max().to_string())
println("Median: " + qual.median().to_string())

# High quality ratio (Q >= 30)
println("High quality: " + (qual.high_quality_ratio() * 100.0).to_string() + "%")

# Category
match qual.categorize()
  QualityCategory::Excellent => println("Excellent quality!")
  QualityCategory::High => println("High quality")
  QualityCategory::Medium => println("Medium quality")
  QualityCategory::Low => println("Low quality")
  QualityCategory::Poor => println("Poor quality!")
end
```

## Working with Reads

### Creating Reads

```aria
import core::read::{Read, ReadPair}

# From raw strings
let read = Read::from_strings(
  "read_001",           # ID
  "ATGCGATCGATCGATCG",  # Sequence
  "IIIIIIIIIIIIIIIII"   # Quality (Phred+33)
).unwrap()

# Check quality
println("Average quality: " + read.avg_quality().to_string())
println("Is high quality: " + read.is_high_quality().to_string())
```

### Read Operations

```aria
# Trim low-quality bases
let trimmed = read.trim_quality(20).unwrap()

# Mask low-quality bases with N
let masked = read.mask_low_quality(20)

# Get reverse complement
let rc = read.reverse_complement()

# Extract subread
let sub = read.subread(10, 50).unwrap()

# Export as FASTQ
println(read.to_fastq())
```

## K-mer Analysis

### Counting K-mers

```aria
import algorithms::kmer::{count_kmers, most_frequent_kmers, kmer_distance}

let seq = Sequence::new("ATGATGATGATGATG").unwrap()

# Count 3-mers
let counts = count_kmers(seq, 3)

println("Unique 3-mers: " + counts.unique_count().to_string())
println("Total 3-mers: " + counts.total_kmers.to_string())

# Get count for specific k-mer
let atg_count = counts.get_count("ATG")
println("ATG appears: " + atg_count.to_string() + " times")
```

### Most Frequent K-mers

```aria
# Get top 5 most frequent
let top = most_frequent_kmers(seq, 3, 5)

for (kmer, count) in top
  println(kmer + ": " + count.to_string())
end
```

### K-mer Distance

```aria
let seq1 = Sequence::new("ATGATGATG").unwrap()
let seq2 = Sequence::new("ATGATCATG").unwrap()

# Jaccard distance based on 3-mers
let dist = kmer_distance(seq1, seq2, 3)
println("Distance: " + dist.to_string())  # 0.0 = identical, 1.0 = no shared k-mers
```

## Quality Filtering

### Basic Filtering

```aria
import algorithms::quality_filter::{filter_by_quality, filter_by_length}

# Filter by minimum quality
let high_qual_reads = filter_by_quality(reads, 30)

# Filter by minimum length
let long_reads = filter_by_length(reads, 100)
```

### Using Filter Configuration

```aria
import algorithms::quality_filter::{filter_reads, FilterConfig}

# Default configuration
let config = FilterConfig::default()

# Strict configuration
let strict_config = FilterConfig::strict()

# Custom configuration using builder pattern
let custom_config = FilterConfig::default()
  .with_min_quality(25)
  .with_min_length(50)
  .with_gc_range(0.3, 0.7)
  .with_max_ambiguous(0.05)
  .with_trim_quality(20)

# Apply filter
let result = filter_reads(reads, custom_config)

println("Passed: " + result.total_passed.to_string())
println("Failed: " + result.total_failed.to_string())
println("Pass rate: " + (result.pass_rate * 100.0).to_string() + "%")

# Access filtered reads
for read in result.passed
  println(read.to_string())
end
```

## Sequence Alignment

### Smith-Waterman (Local)

```aria
import algorithms::alignment::{smith_waterman, ScoringMatrix}

let seq1 = Sequence::new("AGTACGCA").unwrap()
let seq2 = Sequence::new("TATGC").unwrap()

# Use default scoring
let scoring = ScoringMatrix::default_dna()
let alignment = smith_waterman(seq1, seq2, scoring)

println(alignment.format())
println("Score: " + alignment.score.to_string())
println("Identity: " + (alignment.identity * 100.0).to_string() + "%")
```

### Needleman-Wunsch (Global)

```aria
import algorithms::alignment::{needleman_wunsch}

let alignment = needleman_wunsch(seq1, seq2, scoring)
println(alignment.format())
```

### Custom Scoring

```aria
# Create custom scoring matrix
let scoring = ScoringMatrix::simple(
  2,   # Match score
  -1,  # Mismatch penalty
  -2   # Gap penalty
)

# Or use BLAST-like scoring
let blast_scoring = ScoringMatrix::blast_like()
```

## Statistics

### Sequence Statistics

```aria
import core::stats::{SequenceStats, SequenceSetStats}

# Single sequence stats
let stats = SequenceStats::from_sequence(seq)
println(stats.to_string())

# Multiple sequence stats
let set_stats = SequenceSetStats::from_sequences(sequences)
println("N50: " + set_stats.n50.to_string())
println("Total bases: " + set_stats.total_bases.to_string())
```

### Read Set Statistics

```aria
import core::stats::{ReadSetStats}

let read_stats = ReadSetStats::from_reads(reads)
println("Mean quality: " + read_stats.mean_quality.to_string())
println("Mean length: " + read_stats.mean_length.to_string())
```

## Building Pipelines

### Basic Pipeline Pattern

```aria
# Load data
let reads = load_fastq("input.fastq")

# Step 1: Filter by quality
let filtered = filter_reads(reads, FilterConfig::default())
println("After quality filter: " + filtered.total_passed.to_string())

# Step 2: Analyze k-mers on passed reads
for read in filtered.passed
  let kmers = count_kmers(read.sequence, 5)
  # Process k-mers...
end

# Step 3: Calculate statistics
let stats = ReadSetStats::from_reads(filtered.passed)
println(stats.to_string())
```

### Error Handling

```aria
# Always handle potential errors
match Sequence::new(user_input)
  Ok(seq) => {
    # Process valid sequence
    let gc = seq.gc_content()
  }
  Err(SequenceError::InvalidBase(pos, char)) => {
    println("Invalid base '" + char.to_string() + "' at position " + pos.to_string())
  }
  Err(SequenceError::EmptySequence) => {
    println("Sequence cannot be empty")
  }
  Err(e) => {
    println("Error: " + e.to_string())
  }
end
```

## Best Practices

### 1. Validate Input Early

```aria
# Check preconditions at entry points
fn process_sequence(seq: Sequence) -> Result<Stats, Error>
  requires seq.is_valid()
  requires seq.len() >= 10

  # Process...
end
```

### 2. Use Type System

```aria
# Don't use raw strings for sequences
let bad: String = "ATGC"  # No validation!

# Use validated types
let good: Sequence = Sequence::new("ATGC").unwrap()
```

### 3. Handle Edge Cases

```aria
# Check for empty results
let kmers = count_kmers(seq, k)
if kmers.unique_count() == 0
  println("No valid k-mers found")
  return
end
```

### 4. Use Appropriate Algorithms

- **Local alignment** (Smith-Waterman): Finding similar regions
- **Global alignment** (Needleman-Wunsch): Aligning full sequences
- **K-mer distance**: Quick similarity estimate
- **Statistical analysis**: Quality assessment

## Common Workflows

### Quality Control Pipeline

```aria
fn qc_pipeline(reads: [Read]) -> QCReport
  # 1. Initial statistics
  let initial_stats = ReadSetStats::from_reads(reads)

  # 2. Filter
  let filtered = filter_reads(reads, FilterConfig::default())

  # 3. Final statistics
  let final_stats = ReadSetStats::from_reads(filtered.passed)

  QCReport {
    input_reads: reads.len(),
    passed_reads: filtered.total_passed,
    mean_quality_before: initial_stats.mean_quality,
    mean_quality_after: final_stats.mean_quality
  }
end
```

### Sequence Comparison

```aria
fn compare_sequences(seqs: [Sequence], k: Int) -> [[Float]]
  # Build distance matrix
  let n = seqs.len()
  let mut distances = []

  for i in 0..n
    let mut row = []
    for j in 0..n
      let dist = kmer_distance(seqs[i], seqs[j], k)
      row.push(dist)
    end
    distances.push(row)
  end

  distances
end
```
