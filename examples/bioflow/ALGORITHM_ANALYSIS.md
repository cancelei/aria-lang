# BioFlow Algorithm Analysis

## Executive Summary

BioFlow is a genomic data processing pipeline written in Aria that demonstrates the language's strengths in scientific computing. This document provides a comprehensive analysis of the algorithms implemented, their computational characteristics, and how Aria's Design by Contract features ensure correctness.

---

## Table of Contents

1. [What the Code Does](#section-1-what-the-code-does)
   - [K-mer Counting](#k-mer-counting)
   - [Smith-Waterman Alignment](#smith-waterman-alignment)
   - [Needleman-Wunsch Alignment](#needleman-wunsch-alignment)
   - [Sequence Operations](#sequence-operations)
   - [Quality Score Management](#quality-score-management)
   - [Read Filtering Pipeline](#read-filtering-pipeline)
2. [Performance Characteristics](#section-2-performance-characteristics)
   - [Time Complexity Analysis](#time-complexity-analysis)
   - [Space Complexity Analysis](#space-complexity-analysis)
   - [Aria-Specific Optimizations](#aria-specific-optimizations)
   - [Performance Bottlenecks](#performance-bottlenecks)
3. [Design by Contract Usage](#section-3-design-by-contract-usage)
4. [Recommendations](#section-4-recommendations)

---

## Section 1: What the Code Does

### K-mer Counting

**Location:** `algorithms/kmer.aria`

**Purpose:** Count and analyze k-length substrings (k-mers) in DNA sequences. K-mers are fundamental units in many bioinformatics applications including sequence assembly, error correction, and similarity detection.

**Algorithm: Sliding Window Approach**

```
For sequence S of length n and k-mer size k:
1. For each position i from 0 to (n - k):
   a. Extract substring S[i:i+k]
   b. Skip if contains 'N' (ambiguous base)
   c. Increment count for this k-mer in hash table
2. Return all k-mer counts
```

**Key Functions:**

| Function | Description | Contracts |
|----------|-------------|-----------|
| `count_kmers(seq, k)` | Count all k-mers in a sequence | `requires k > 0 and k <= seq.len()`, `ensures result.total_kmers == seq.len() - k + 1` |
| `most_frequent_kmers(seq, k, n)` | Get top n most common k-mers | `ensures result.len() <= n` |
| `kmer_distance(seq1, seq2, k)` | Jaccard distance between k-mer sets | `ensures result >= 0.0 and result <= 1.0` |
| `kmer_spectrum(seq, k)` | Distribution of k-mer frequencies | Used for genome size estimation |
| `count_kmers_canonical(seq, k)` | Count treating reverse complements as same | Important for double-stranded DNA |

**Use Cases:**
- **Sequence Assembly:** K-mers form the basis of de Bruijn graph assemblers
- **Error Correction:** Low-frequency k-mers often indicate sequencing errors
- **Similarity Detection:** K-mer Jaccard distance provides fast sequence comparison
- **Genome Size Estimation:** K-mer spectrum analysis estimates genome characteristics

**Data Structures:**

```aria
struct KMer
  sequence: String
  k: Int
  invariant self.sequence.len() == self.k  # Compile-time guarantee

struct KMerCounts
  k: Int
  counts: [(String, Int)]  # Array of (kmer, count) pairs
  total_kmers: Int
  invariant self.counts.all(|(kmer, _)| kmer.len() == self.k)
```

---

### Smith-Waterman Alignment

**Location:** `algorithms/alignment.aria`

**Purpose:** Find optimal local alignment between two sequences. This is the gold standard for finding conserved regions and detecting homology in sequences that may only share partial similarity.

**Algorithm: Dynamic Programming with Traceback**

```
Given sequences S1 (length m) and S2 (length n):

1. Initialize (m+1) x (n+1) scoring matrix H with zeros
2. Initialize (m+1) x (n+1) traceback matrix

3. For i = 1 to m:
   For j = 1 to n:
     match_score = scoring.score(S1[i-1], S2[j-1])

     diagonal = H[i-1][j-1] + match_score
     up = H[i-1][j] + gap_penalty
     left = H[i][j-1] + gap_penalty

     H[i][j] = max(0, diagonal, up, left)  # Key difference from global
     traceback[i][j] = direction of maximum

     Track global maximum position

4. Traceback from maximum position until score reaches 0
5. Return aligned sequences
```

**Key Characteristics:**
- **Local Alignment:** Finds best matching subsequences (allows free gaps at ends)
- **Score >= 0:** Minimum score is zero (negative scores reset to zero)
- **Traceback from Maximum:** Starts at highest-scoring cell, not corner

**Scoring Matrix:**

```aria
struct ScoringMatrix
  match_score: Int           # Positive (default: 2)
  mismatch_penalty: Int      # Negative (default: -1)
  gap_open_penalty: Int      # Negative (default: -2)
  gap_extend_penalty: Int    # Negative (default: -1)

  invariant self.match_score > 0
  invariant self.mismatch_penalty <= 0
  invariant self.gap_open_penalty <= 0
```

**Use Cases:**
- Finding conserved domains in proteins
- Detecting local sequence homology
- Identifying similar regions in divergent sequences

---

### Needleman-Wunsch Alignment

**Location:** `algorithms/alignment.aria`

**Purpose:** Find optimal global alignment between two sequences. Aligns the entire length of both sequences, useful when sequences are expected to be related throughout their length.

**Algorithm: Dynamic Programming (Global)**

```
Given sequences S1 (length m) and S2 (length n):

1. Initialize scoring matrix H:
   H[0][j] = j * gap_penalty  (gap costs for first row)
   H[i][0] = i * gap_penalty  (gap costs for first column)

2. For i = 1 to m:
   For j = 1 to n:
     match_score = scoring.score(S1[i-1], S2[j-1])

     diagonal = H[i-1][j-1] + match_score
     up = H[i-1][j] + gap_penalty
     left = H[i][j-1] + gap_penalty

     H[i][j] = max(diagonal, up, left)  # No zero threshold
     traceback[i][j] = direction of maximum

3. Traceback from H[m][n] to H[0][0]
4. Return aligned sequences
```

**Key Differences from Smith-Waterman:**
- Initialized with gap penalties (not zeros)
- No zero threshold in recurrence
- Traceback from corner to corner

---

### Sequence Operations

**Location:** `core/sequence.aria`

**Purpose:** Provide fundamental operations on DNA/RNA sequences with comprehensive validation.

**Key Functions:**

| Function | Description | Complexity |
|----------|-------------|------------|
| `gc_content()` | Calculate proportion of G and C bases | O(n) |
| `at_content()` | Calculate proportion of A and T bases | O(n) |
| `complement()` | DNA complement (A<->T, C<->G) | O(n) |
| `reverse_complement()` | Reverse complement | O(n) |
| `transcribe()` | DNA to RNA (T -> U) | O(n) |
| `contains_motif(motif)` | Check if motif exists | O(n*m) |
| `find_motif_positions(motif)` | Find all motif occurrences | O(n*m) |
| `base_counts()` | Count each nucleotide | O(n) |

**Validation with Contracts:**

```aria
fn gc_content(self) -> Float
  requires self.is_valid()                    # Precondition
  ensures result >= 0.0 and result <= 1.0     # Postcondition

fn complement(self) -> Sequence
  requires self.seq_type == SequenceType::DNA
  requires self.is_valid()
  ensures result.is_valid()
  ensures result.len() == self.len()
```

**Data Structure:**

```aria
struct Sequence
  bases: String
  id: Option<String>
  description: Option<String>
  seq_type: SequenceType  # DNA, RNA, or Unknown

  invariant self.bases.len() > 0
  invariant self.is_valid()  # All bases are valid nucleotides
```

---

### Quality Score Management

**Location:** `core/quality.aria`

**Purpose:** Manage Phred quality scores for sequencing reads, enabling quality-based filtering and analysis.

**Phred Score Formula:**
```
Q = -10 * log10(P_error)

Where:
- Q10 = 90% accuracy (10% error)
- Q20 = 99% accuracy (1% error)
- Q30 = 99.9% accuracy (0.1% error) - High quality threshold
- Q40 = 99.99% accuracy
```

**Key Functions:**

| Function | Description |
|----------|-------------|
| `from_phred33(encoded)` | Parse Illumina 1.8+ quality string |
| `from_phred64(encoded)` | Parse older Illumina quality string |
| `average()` | Calculate mean quality score |
| `median()` | Calculate median quality score |
| `high_quality_ratio()` | Proportion of bases >= Q30 |
| `low_quality_positions(threshold)` | Find positions below threshold |

**Quality Categories:**

```aria
enum QualityCategory
  Poor        # Q < 10
  Low         # 10 <= Q < 20
  Medium      # 20 <= Q < 30
  High        # 30 <= Q < 40
  Excellent   # Q >= 40
```

---

### Read Filtering Pipeline

**Location:** `algorithms/quality_filter.aria`

**Purpose:** Filter sequencing reads based on multiple quality criteria.

**Filter Configuration:**

```aria
struct FilterConfig
  min_quality: Int        # Minimum average quality score
  min_length: Int         # Minimum read length
  max_length: Int         # Maximum read length (0 = no limit)
  min_gc: Float           # Minimum GC content
  max_gc: Float           # Maximum GC content
  max_ambiguous: Float    # Maximum proportion of N bases
  trim_quality: Int       # Quality threshold for trimming
```

**Filter Pipeline:**

```
For each read:
1. Apply quality trimming (if configured)
2. Check average quality >= min_quality
3. Check length >= min_length
4. Check length <= max_length (if configured)
5. Check GC content in [min_gc, max_gc]
6. Check ambiguous bases <= max_ambiguous

Mark as passed/failed accordingly
```

**Preset Configurations:**

| Config | min_quality | min_length | max_ambiguous |
|--------|-------------|------------|---------------|
| Default | Q20 | 50 | 5% |
| Strict | Q30 | 100 | 1% |
| Permissive | Q10 | 20 | 10% |

---

## Section 2: Performance Characteristics

### Time Complexity Analysis

| Algorithm | Time Complexity | Variables |
|-----------|-----------------|-----------|
| **K-mer Counting** | O(n * k) | n = sequence length, k = k-mer size |
| **K-mer Sorting** | O(u^2) | u = unique k-mers (bubble sort) |
| **K-mer Distance** | O(n1 + n2 + u1 * u2) | n1, n2 = sequence lengths, u1, u2 = unique k-mers |
| **Smith-Waterman** | O(m * n) | m, n = sequence lengths |
| **Needleman-Wunsch** | O(m * n) | m, n = sequence lengths |
| **GC Content** | O(n) | n = sequence length |
| **Motif Finding** | O(n * m) | n = sequence length, m = motif length |
| **Complement** | O(n) | n = sequence length |
| **Quality Average** | O(n) | n = read length |
| **Read Filtering** | O(r * L) | r = reads, L = average read length |

### Space Complexity Analysis

| Algorithm | Space Complexity | Notes |
|-----------|------------------|-------|
| **K-mer Counting** | O(4^k) worst case | Hash table for all possible k-mers |
| **K-mer Counting** | O(u) typical | u = unique k-mers in sequence |
| **Smith-Waterman** | O(m * n) | Full scoring matrix |
| **SW Score Only** | O(n) | Uses two-row optimization |
| **Needleman-Wunsch** | O(m * n) | Full scoring matrix |
| **Sequence Storage** | O(n) | String storage |
| **Quality Scores** | O(n) | Integer array |
| **Filter Result** | O(r) | r = number of reads |

### Aria-Specific Optimizations

**1. Monomorphization (Zero-Cost Generics)**

Aria compiles generic functions to specialized versions at compile time:

```aria
# This generic function:
fn count_kmers(sequence: Sequence, k: Int) -> KMerCounts

# Gets specialized at compile time, no runtime dispatch overhead
```

**2. Design by Contract Compilation**

Contracts are checked at compile time where possible and compiled out in release builds:

```aria
fn gc_content(self) -> Float
  requires self.is_valid()              # Checked once at call site
  ensures result >= 0.0 and result <= 1.0  # Verified by type system
```

In release mode, contract checks have zero runtime overhead.

**3. Direct Memory Access**

Aria compiles to native code via Cranelift, providing:
- Direct CPU instructions without interpreter overhead
- Efficient memory layout for structs
- No garbage collection pauses

**4. Type-Safe Operations**

All type checks happen at compile time:

```aria
# Compile-time guarantee that alignment result is valid
struct Alignment
  invariant self.aligned_seq1.len() == self.aligned_seq2.len()
```

### Performance Bottlenecks

**1. Bubble Sort in K-mer Operations**

The current implementation uses bubble sort for k-mer ranking:

```aria
# In sorted_by_count() - O(n^2) complexity
let mut i = 0
loop
  let mut j = 0
  loop
    if sorted[j].1 < sorted[j + 1].1
      let temp = sorted[j]
      sorted[j] = sorted[j + 1]
      sorted[j + 1] = temp
    end
```

**Recommendation:** Use quicksort or mergesort for O(n log n) complexity.

**2. String Concatenation in Loops**

Several functions build strings character by character:

```aria
# In complement() - Creates new string each iteration
let mut comp_bases = ""
loop
  let comp = Self::complement_base(c)
  comp_bases = comp_bases + comp.to_string()  # O(n^2) total
```

**Recommendation:** Use StringBuilder pattern for O(n) total complexity.

**3. Linear Search in K-mer Counts**

K-mer lookup uses linear search through array:

```aria
fn get_count(self, kmer: String) -> Int
  let mut i = 0
  loop
    if self.counts[i].0 == kmer
      return self.counts[i].1
```

**Recommendation:** Use hash map for O(1) average lookup.

**4. Full Matrix in Alignment**

Both alignment algorithms store full O(m*n) matrices:

```aria
let mut H = []  # Full (m+1) x (n+1) matrix
```

For score-only computation, the `alignment_score_only` function already uses O(n) space optimization with two rows.

---

## Section 3: Design by Contract Usage

BioFlow extensively uses Aria's Design by Contract features to ensure correctness.

### Invariants (Struct-Level Contracts)

```aria
struct Sequence
  invariant self.bases.len() > 0
  invariant self.is_valid()

struct QualityScores
  invariant self.scores.len() > 0
  invariant self.all_in_range()

struct Alignment
  invariant self.aligned_seq1.len() == self.aligned_seq2.len()
  invariant self.identity >= 0.0 and self.identity <= 1.0
```

### Preconditions (requires)

```aria
fn subsequence(self, start: Int, end: Int) -> Result<Sequence, SequenceError>
  requires start >= 0 : "Start index must be non-negative"
  requires end > start : "End must be greater than start"
  requires end <= self.len() : "End must not exceed sequence length"

fn smith_waterman(seq1: Sequence, seq2: Sequence, scoring: ScoringMatrix)
  requires seq1.is_valid() and seq2.is_valid()
  requires seq1.len() > 0 and seq2.len() > 0
```

### Postconditions (ensures)

```aria
fn gc_content(self) -> Float
  ensures result >= 0.0 and result <= 1.0

fn count_kmers(sequence: Sequence, k: Int) -> KMerCounts
  ensures result.k == k
  ensures result.total_kmers == sequence.len() - k + 1

fn filter_by_quality(reads: [Read], min_quality: Int) -> [Read]
  ensures result.all(|r| r.avg_quality() >= min_quality.to_float())
  ensures result.len() <= reads.len()
```

### Contract Coverage by Module

| Module | Invariants | Preconditions | Postconditions |
|--------|------------|---------------|----------------|
| sequence.aria | 2 | 15+ | 20+ |
| quality.aria | 2 | 10+ | 15+ |
| read.aria | 2 | 10+ | 12+ |
| alignment.aria | 2 | 8+ | 10+ |
| kmer.aria | 3 | 12+ | 15+ |
| quality_filter.aria | 2 | 20+ | 15+ |

---

## Section 4: Recommendations

### Algorithm Improvements

1. **Replace Bubble Sort with QuickSort**
   - Impact: K-mer sorting from O(n^2) to O(n log n)
   - Affected functions: `sorted_by_count()`, `sorted_scores()`

2. **Use StringBuilder Pattern**
   - Impact: String building from O(n^2) to O(n)
   - Affected functions: `complement()`, `reverse()`, `transcribe()`

3. **Implement Hash Map for K-mers**
   - Impact: K-mer lookup from O(u) to O(1)
   - Affected: `KMerCounts` struct

4. **Add Hirschberg's Algorithm**
   - Impact: Alignment space from O(mn) to O(min(m,n))
   - For memory-constrained environments

### API Improvements

1. **Streaming K-mer Counting**
   - Process sequences in chunks for large files
   - Iterator-based API

2. **Parallel Alignment**
   - Parallelize multiple sequence alignments
   - Use Aria's effect system for safe concurrency

3. **Memory-Mapped File Support**
   - For processing large FASTA/FASTQ files
   - Lazy loading of sequences

### Benchmark Suite

Recommended benchmarks for performance testing:

```
| Test | Sequence Size | Expected Time (Aria) |
|------|---------------|----------------------|
| GC Content | 20 kb | < 1 ms |
| K-mer (k=21) | 20 kb | < 5 ms |
| Smith-Waterman | 1 kb x 1 kb | < 100 ms |
| Filter Pipeline | 10,000 reads | < 500 ms |
```

---

## Conclusion

BioFlow demonstrates how Aria's Design by Contract features provide:

1. **Correctness Guarantees:** Compile-time verification of biological constraints
2. **Safety:** Invalid sequences are rejected before processing
3. **Documentation:** Contracts serve as executable specifications
4. **Performance:** Zero-cost abstractions with native code compilation

The implementation provides a solid foundation for genomic analysis while showcasing Aria's advantages over both traditional systems languages (C/C++) and interpreted languages (Python).
