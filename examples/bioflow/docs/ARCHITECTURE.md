# BioFlow Architecture

## Overview

BioFlow is a bioinformatics pipeline framework implemented in Aria, demonstrating the language's strengths in scientific computing through Design by Contract, generic types, and effect tracking.

## System Architecture

```
+------------------+     +------------------+     +------------------+
|   Core Types     |---->|   Algorithms     |---->|   Pipeline       |
|   (Validated)    |     |   (Contracts)    |     |   (Composable)   |
+------------------+     +------------------+     +------------------+
        |                        |                        |
        v                        v                        v
+------------------+     +------------------+     +------------------+
|   Sequence       |     |   K-mer          |     |   Quality        |
|   Quality        |     |   Alignment      |     |   Statistics     |
|   Read           |     |   Stats          |     |   Filter         |
+------------------+     +------------------+     +------------------+
```

## Module Organization

### Core Module (`src/core/`)

The core module contains fundamental data types with built-in validation:

| File | Purpose | Key Types |
|------|---------|-----------|
| `sequence.aria` | DNA/RNA sequences | `Sequence`, `SequenceType`, `SequenceError` |
| `quality.aria` | Phred quality scores | `QualityScores`, `QualityCategory` |
| `read.aria` | Sequencing reads | `Read`, `ReadPair`, `ReadOrientation` |
| `alignment.aria` | Alignment results | `Alignment`, `AlignmentResult` |
| `stats.aria` | Statistical types | `SequenceStats`, `ReadSetStats` |

### Algorithms Module (`src/algorithms/`)

The algorithms module provides bioinformatics functions with comprehensive contracts:

| File | Purpose | Key Functions |
|------|---------|---------------|
| `validation.aria` | Sequence validation | `is_valid_dna`, `validate_dna_detailed` |
| `stats.aria` | Statistical analysis | `gc_content`, `base_frequencies` |
| `kmer.aria` | K-mer counting | `count_kmers`, `kmer_distance` |
| `quality_filter.aria` | Read filtering | `filter_by_quality`, `filter_reads` |
| `alignment.aria` | Sequence alignment | `smith_waterman`, `needleman_wunsch` |

## Design Principles

### 1. Design by Contract

Every function includes preconditions (`requires`) and postconditions (`ensures`):

```aria
fn gc_content(self) -> Float
  requires self.is_valid()              # Precondition
  ensures result >= 0.0 and result <= 1.0  # Postcondition
```

Struct invariants ensure data validity:

```aria
struct Sequence
  bases: String
  invariant self.bases.len() > 0
  invariant self.is_valid()
end
```

### 2. Type Safety

Strong typing prevents common errors:
- `QualityScores` ensures values are in Phred range (0-40)
- `Sequence` validates nucleotide characters
- `Read` enforces matching sequence/quality lengths

### 3. Immutability by Default

Data structures are immutable unless explicitly marked `mut`:

```aria
fn complement(self) -> Sequence
  # Returns new Sequence, doesn't modify self
end

fn trim_quality(mut self, threshold: Int)
  # Explicitly modifies self
end
```

### 4. Effect Tracking

Side effects are tracked in function signatures:

```aria
fn process_file(path: String) -> Result<Data, Error> !IO, !Compute
  # Declares IO and Compute effects
end
```

## Data Flow

### Read Processing Pipeline

```
FASTQ File
    |
    v
[Parse Reads] --> [Validate] --> [Filter Quality] --> [Trim] --> [Output]
    |                |                |                |
    v                v                v                v
  Reads           Errors          Filtered         Trimmed
```

### Analysis Pipeline

```
Sequence(s)
    |
    +---> [Statistics] ---> SequenceStats
    |
    +---> [K-mer Count] ---> KMerCounts
    |
    +---> [Alignment] ---> AlignmentResult
    |
    +---> [Validation] ---> ValidationResult
```

## Error Handling

BioFlow uses `Result<T, E>` for recoverable errors:

```aria
fn new(bases: String) -> Result<Sequence, SequenceError>
  if !is_valid_dna(bases)
    return Err(SequenceError::InvalidBase(...))
  end
  Ok(Sequence { bases: bases })
end
```

Error types are specific and informative:

```aria
enum SequenceError
  EmptySequence
  InvalidBase(position: Int, found: Char)
  InvalidLength(expected: Int, actual: Int)
end
```

## Performance Considerations

### Memory Efficiency

- K-mer counting uses hash maps for O(1) lookups
- Alignment uses O(mn) space (can be optimized to O(min(m,n)))
- Quality filtering is O(n) single-pass

### Time Complexity

| Operation | Complexity |
|-----------|------------|
| Sequence validation | O(n) |
| GC content | O(n) |
| K-mer counting | O(n) |
| Smith-Waterman | O(mn) |
| Quality filtering | O(n) |

## Extensibility

### Adding New Sequence Types

1. Define alphabet validation:
```aria
fn is_valid_protein(c: Char) -> Bool
  c in VALID_AMINO_ACIDS
end
```

2. Create type with invariants:
```aria
struct ProteinSequence
  residues: String
  invariant residues.all(is_valid_protein)
end
```

### Adding New Pipeline Stages

1. Implement the `PipelineStage` trait:
```aria
impl PipelineStage<Input, Output> for MyStage
  fn process(self, input: Input) -> Result<Output, String>
  fn name(self) -> String
end
```

2. Compose into pipelines:
```aria
let pipeline = Pipeline::new("my_analysis")
  .add_stage(Stage1::new())
  .add_stage(Stage2::new())
```

## Testing Strategy

### Unit Tests
- Each core type has comprehensive tests
- All algorithms tested with edge cases
- Contract violations tested for proper errors

### Integration Tests
- Pipeline composition tests
- Real FASTA/FASTQ parsing tests
- Performance benchmarks

## Future Directions

1. **WASM Compilation**: Browser-based analysis tools
2. **Parallel Processing**: Multi-threaded k-mer counting
3. **Advanced Algorithms**: BLAST-like heuristics, HMM profiles
4. **File Format Support**: SAM/BAM, VCF, GFF parsing
