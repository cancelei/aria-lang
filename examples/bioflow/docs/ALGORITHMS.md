# BioFlow Algorithms

## Overview

BioFlow implements core bioinformatics algorithms with formal contracts ensuring correctness. This document explains each algorithm and its implementation.

---

## 1. Sequence Validation

### Purpose
Validates that DNA/RNA sequences contain only valid nucleotide bases.

### Valid Characters

| Type | Valid Bases |
|------|-------------|
| DNA | A, C, G, T, N |
| RNA | A, C, G, U, N |

### Algorithm

```
function is_valid_dna(sequence):
    for each character c in sequence:
        if c not in {A, C, G, T, N}:
            return false
    return true
```

### Contracts

```aria
fn is_valid_dna(seq: String) -> Bool
  requires seq.len() > 0
  ensures result implies seq.all(|c| c in ['A', 'C', 'G', 'T', 'N'])
```

### Time Complexity
O(n) where n is sequence length.

---

## 2. GC Content Calculation

### Purpose
Calculate the proportion of guanine (G) and cytosine (C) bases in a sequence. GC content affects DNA stability and is important for PCR primer design.

### Formula

```
GC% = (G + C) / (A + C + G + T) * 100
```

### Algorithm

```
function gc_content(sequence):
    gc_count = 0
    total = 0
    for each base in sequence:
        if base == 'G' or base == 'C':
            gc_count += 1
        if base != 'N':
            total += 1
    return gc_count / total
```

### Contracts

```aria
fn gc_content(self) -> Float
  requires self.is_valid()
  ensures result >= 0.0 and result <= 1.0
```

### Time Complexity
O(n)

---

## 3. Complement and Reverse Complement

### Purpose
DNA base pairing rules: A pairs with T, C pairs with G. The reverse complement is crucial for working with double-stranded DNA.

### Base Pairing Rules

| Original | Complement |
|----------|------------|
| A | T |
| T | A |
| C | G |
| G | C |
| N | N |

### Algorithm

```
function complement(sequence):
    result = ""
    for each base in sequence:
        result += complement_base(base)
    return result

function reverse_complement(sequence):
    return reverse(complement(sequence))
```

### Contracts

```aria
fn complement(self) -> Sequence
  requires self.seq_type == DNA
  ensures result.len() == self.len()
  ensures result.is_valid()
```

### Time Complexity
O(n)

---

## 4. K-mer Counting

### Purpose
K-mers are subsequences of length k. K-mer analysis is fundamental to:
- Genome assembly
- Sequence comparison
- Repeat identification
- Error correction

### Algorithm

```
function count_kmers(sequence, k):
    counts = empty map
    for i from 0 to length(sequence) - k:
        kmer = sequence[i:i+k]
        if kmer not contains 'N':
            counts[kmer] += 1
    return counts
```

### Canonical K-mers

For strand-agnostic counting, use the lexicographically smaller of a k-mer and its reverse complement:

```
function canonical(kmer):
    rc = reverse_complement(kmer)
    return min(kmer, rc)
```

### K-mer Spectrum

The k-mer spectrum shows the distribution of k-mer frequencies:

```
Frequency | Count
1         | 45    (45 k-mers appear once)
2         | 23    (23 k-mers appear twice)
...
```

### Contracts

```aria
fn count_kmers(sequence: Sequence, k: Int) -> KMerCounts
  requires k > 0 and k <= sequence.len()
  ensures result.k == k
  ensures result.total_kmers == sequence.len() - k + 1
```

### Time Complexity
O(n * k) for counting, where n is sequence length.

---

## 5. K-mer Distance (Jaccard)

### Purpose
Measures similarity between sequences based on shared k-mers.

### Formula

```
Jaccard Distance = 1 - |A ∩ B| / |A ∪ B|
```

Where A and B are the sets of k-mers in each sequence.

### Algorithm

```
function kmer_distance(seq1, seq2, k):
    kmers1 = set of k-mers in seq1
    kmers2 = set of k-mers in seq2

    intersection = size of (kmers1 ∩ kmers2)
    union = size of (kmers1 ∪ kmers2)

    return 1 - intersection / union
```

### Contracts

```aria
fn kmer_distance(seq1: Sequence, seq2: Sequence, k: Int) -> Float
  requires k > 0
  requires k <= min(seq1.len(), seq2.len())
  ensures result >= 0.0 and result <= 1.0
```

### Time Complexity
O(n + m) where n, m are sequence lengths.

---

## 6. Smith-Waterman Local Alignment

### Purpose
Finds the optimal local alignment between two sequences. Used for finding similar regions within sequences.

### Scoring

| Event | Default Score |
|-------|---------------|
| Match | +2 |
| Mismatch | -1 |
| Gap | -2 |

### Algorithm

Dynamic programming approach:

```
function smith_waterman(seq1, seq2, scoring):
    # Initialize matrix
    H = matrix[m+1][n+1] filled with 0

    # Fill matrix
    for i from 1 to m:
        for j from 1 to n:
            match = H[i-1][j-1] + score(seq1[i], seq2[j])
            delete = H[i-1][j] + gap_penalty
            insert = H[i][j-1] + gap_penalty

            H[i][j] = max(0, match, delete, insert)

            track maximum score position

    # Traceback from maximum
    return traceback(H, max_position)
```

### Key Difference from Global

- Matrix initialized with 0s (not gap penalties)
- Negative scores reset to 0
- Traceback starts from maximum, ends at 0

### Contracts

```aria
fn smith_waterman(seq1: Sequence, seq2: Sequence, scoring: ScoringMatrix) -> Alignment
  requires seq1.is_valid() and seq2.is_valid()
  requires seq1.len() > 0 and seq2.len() > 0
  ensures result.score >= 0
  ensures result.aligned_seq1.len() == result.aligned_seq2.len()
```

### Time Complexity
O(mn) time and space.

---

## 7. Needleman-Wunsch Global Alignment

### Purpose
Finds the optimal global alignment (end-to-end) between two sequences.

### Algorithm

```
function needleman_wunsch(seq1, seq2, scoring):
    # Initialize with gap penalties
    H[0][j] = j * gap_penalty
    H[i][0] = i * gap_penalty

    # Fill matrix (no 0 threshold)
    for i from 1 to m:
        for j from 1 to n:
            match = H[i-1][j-1] + score(seq1[i], seq2[j])
            delete = H[i-1][j] + gap_penalty
            insert = H[i][j-1] + gap_penalty

            H[i][j] = max(match, delete, insert)

    # Traceback from bottom-right corner
    return traceback(H, (m, n))
```

### Key Difference from Local

- First row/column initialized with cumulative gap penalties
- No 0 threshold (can be negative)
- Traceback from [m][n] to [0][0]

### Time Complexity
O(mn) time and space.

---

## 8. Quality Filtering

### Purpose
Filter sequencing reads based on quality scores and other criteria.

### Phred Quality Scores

| Q Score | Error Probability | Accuracy |
|---------|------------------|----------|
| 10 | 1 in 10 | 90% |
| 20 | 1 in 100 | 99% |
| 30 | 1 in 1000 | 99.9% |
| 40 | 1 in 10000 | 99.99% |

### Formula

```
Q = -10 * log10(P_error)
P_error = 10^(-Q/10)
```

### Filter Criteria

| Criterion | Purpose |
|-----------|---------|
| Min quality | Remove low-quality reads |
| Min length | Remove too-short reads |
| Max length | Remove abnormally long reads |
| GC range | Remove compositionally extreme reads |
| Max ambiguous | Remove reads with too many N bases |

### Algorithm

```
function filter_reads(reads, config):
    passed = []
    failed = []

    for each read in reads:
        if config.trim_quality > 0:
            read = trim_low_quality_ends(read, config.trim_quality)

        if passes_all_criteria(read, config):
            passed.append(read)
        else:
            failed.append(read)

    return FilterResult(passed, failed)
```

### Contracts

```aria
fn filter_by_quality(reads: [Read], min_quality: Int) -> [Read]
  requires min_quality >= 0 and min_quality <= 40
  ensures result.all(|r| r.avg_quality() >= min_quality)
  ensures result.len() <= reads.len()
```

### Time Complexity
O(n * m) where n is number of reads, m is average read length.

---

## 9. Quality-Based Trimming

### Purpose
Remove low-quality bases from read ends while preserving the high-quality core.

### Algorithm

```
function trim_quality(read, threshold):
    # Find first position above threshold
    start = 0
    while start < length and quality[start] < threshold:
        start += 1

    # Find last position above threshold
    end = length - 1
    while end > start and quality[end] < threshold:
        end -= 1

    return read[start:end+1]
```

### Time Complexity
O(n) where n is read length.

---

## 10. Dinucleotide Frequencies

### Purpose
Analyze patterns of adjacent nucleotide pairs. Important for:
- CpG island detection
- Codon bias analysis
- Species identification

### Algorithm

```
function dinucleotide_frequencies(sequence):
    counts = array of 16 zeros  # AA, AC, AG, AT, CA, ...

    for i from 0 to length-1:
        dinuc = sequence[i:i+2]
        index = dinuc_to_index(dinuc)
        counts[index] += 1

    # Convert to frequencies
    total = length - 1
    return counts / total
```

### Time Complexity
O(n)

---

## Summary

| Algorithm | Complexity | Key Contract |
|-----------|------------|--------------|
| Validation | O(n) | All chars valid |
| GC Content | O(n) | Result in [0,1] |
| Complement | O(n) | Length preserved |
| K-mer Count | O(nk) | Total = n-k+1 |
| K-mer Distance | O(n+m) | Result in [0,1] |
| Smith-Waterman | O(mn) | Score >= 0 |
| Needleman-Wunsch | O(mn) | Full coverage |
| Quality Filter | O(nm) | All pass threshold |
| Trimming | O(n) | Quality improved |
