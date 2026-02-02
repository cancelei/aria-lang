# BioFlow Go

A production-quality Go implementation of the BioFlow genomic pipeline for sequence analysis.

## Features

- **DNA/RNA Sequence Handling** - Validation, GC/AT content, complement, reverse complement
- **K-mer Analysis** - Counting, frequency analysis, Jaccard distance, canonical k-mers
- **Sequence Alignment** - Smith-Waterman (local) and Needleman-Wunsch (global) algorithms
- **Quality Scores** - Phred score parsing, filtering, statistics
- **Statistical Analysis** - Sequence statistics, N50 calculation, histograms
- **REST API** - HTTP endpoints for all operations
- **CLI Tool** - Command-line interface for common operations

## Installation

```bash
# Clone the repository
git clone https://github.com/aria-lang/bioflow-go.git
cd bioflow-go

# Download dependencies
go mod download

# Build binaries
make build
```

## Quick Start

### CLI Usage

```bash
# Sequence information
./bin/bioflow info -seq "ATGCATGCATGC"

# GC content calculation
./bin/bioflow gc -file sample.fasta

# K-mer analysis
./bin/bioflow kmer -seq "ATGATGATGATG" -k 3 -top 10

# Sequence alignment
./bin/bioflow align -seq1 "ATGCATGC" -seq2 "ATGCGGGG"

# Statistics for FASTA file
./bin/bioflow stats -file sample.fasta
```

### API Usage

```bash
# Start the server
./bin/bioflow-server -port 8080

# Calculate GC content
curl -X POST http://localhost:8080/api/sequence/gc-content \
  -H "Content-Type: application/json" \
  -d '{"sequence": "ATGCATGC"}'

# Count k-mers
curl -X POST http://localhost:8080/api/kmer/count \
  -H "Content-Type: application/json" \
  -d '{"sequence": "ATGATGATG", "k": 3}'

# Align sequences
curl -X POST http://localhost:8080/api/alignment/local \
  -H "Content-Type: application/json" \
  -d '{"sequence1": "ATGCATGC", "sequence2": "ATGCGGGG"}'
```

### Library Usage

```go
package main

import (
    "fmt"
    "log"

    "github.com/aria-lang/bioflow-go/pkg/bioflow"
)

func main() {
    // Create a sequence
    seq, err := bioflow.NewSequence("ATGCATGCATGC")
    if err != nil {
        log.Fatal(err)
    }

    // Calculate GC content
    fmt.Printf("GC Content: %.2f%%\n", seq.GCContent()*100)

    // Get complement
    comp, _ := seq.Complement()
    fmt.Printf("Complement: %s\n", comp.Bases)

    // Count k-mers
    counter, _ := bioflow.CountKMers(seq, 3)
    fmt.Printf("Unique 3-mers: %d\n", counter.UniqueCount())

    // Align sequences
    seq2, _ := bioflow.NewSequence("ATGCGGGG")
    alignment, _ := bioflow.Align(seq, seq2)
    fmt.Printf("Alignment score: %d\n", alignment.Score)
    fmt.Printf("Identity: %.1f%%\n", alignment.Identity*100)
}
```

## Project Structure

```
bioflow-go/
├── cmd/
│   ├── bioflow/           # CLI entry point
│   └── bioflow-server/    # HTTP API server
├── internal/
│   ├── sequence/          # Sequence types and validation
│   ├── kmer/              # K-mer counting and analysis
│   ├── alignment/         # Alignment algorithms
│   ├── quality/           # Quality scores and filtering
│   └── stats/             # Statistics calculations
├── pkg/
│   └── bioflow/           # Public API
├── api/
│   ├── handlers/          # HTTP handlers
│   └── middleware/        # HTTP middleware
├── web/
│   └── static/            # Static web files
├── scripts/
│   └── benchmark.sh       # Benchmark script
├── testdata/
│   └── sample.fasta       # Sample data
├── go.mod
├── go.sum
├── Makefile
├── README.md
└── COMPARISON.md          # Go vs Aria comparison
```

## Development

```bash
# Run tests
make test

# Run benchmarks
make bench

# Generate coverage report
make coverage

# Format code
make fmt

# Run linter
make lint

# Run all checks
make check

# Build everything
make build
```

## API Endpoints

### Sequence Operations

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/sequence/gc-content` | Calculate GC content |
| POST | `/api/sequence/at-content` | Calculate AT content |
| POST | `/api/sequence/complement` | Get complement |
| POST | `/api/sequence/reverse-complement` | Get reverse complement |
| POST | `/api/sequence/transcribe` | Transcribe DNA to RNA |
| POST | `/api/sequence/info` | Get sequence information |
| POST | `/api/sequence/validate` | Validate sequence |

### K-mer Operations

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/kmer/count` | Count k-mers |
| POST | `/api/kmer/most-frequent` | Get most frequent k-mers |
| POST | `/api/kmer/distance` | Calculate k-mer distance |
| POST | `/api/kmer/shared` | Find shared k-mers |

### Alignment Operations

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/alignment/local` | Smith-Waterman alignment |
| POST | `/api/alignment/global` | Needleman-Wunsch alignment |
| POST | `/api/alignment/score` | Get alignment score only |

### Quality Operations

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/quality/parse` | Parse quality string |
| POST | `/api/quality/stats` | Calculate quality statistics |
| POST | `/api/quality/filter` | Filter read by quality |

## Testing

The project includes comprehensive tests:

```bash
# Run all tests
go test -v ./...

# Run tests with race detection
go test -race ./...

# Run benchmarks
go test -bench=. -benchmem ./...

# Generate coverage report
go test -coverprofile=coverage.out ./...
go tool cover -html=coverage.out
```

## Benchmarks

Typical benchmark results on modern hardware:

| Operation | Time | Memory |
|-----------|------|--------|
| New Sequence (40bp) | ~200ns | ~160B |
| GC Content | ~50ns | 0B |
| Complement | ~300ns | ~96B |
| K-mer Count (100bp, k=21) | ~2us | ~5KB |
| Smith-Waterman (1kb x 1kb) | ~100ms | ~8MB |

Run benchmarks with: `make bench`

## License

MIT License - see LICENSE file for details.

## See Also

- [COMPARISON.md](COMPARISON.md) - Detailed comparison with Aria and Python implementations
- [Aria BioFlow](../bioflow/) - Original Aria implementation
- [Python BioFlow](../bioflow-python/) - Python implementation
