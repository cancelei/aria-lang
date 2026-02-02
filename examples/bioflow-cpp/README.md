# BioFlow C++20 - High-Performance Bioinformatics Library

A modern C++20 implementation of common bioinformatics algorithms, designed as a performance baseline for comparison with Aria.

## Features

- **Sequence Analysis**: DNA sequence representation with validation, GC content, complement, reverse complement
- **K-mer Counting**: Efficient hash-based k-mer counting with spectrum analysis
- **Sequence Alignment**: Smith-Waterman (local) and Needleman-Wunsch (global) algorithms
- **Quality Scores**: Phred quality score handling with trimming and statistics
- **Statistics**: Comprehensive statistical analysis including Shannon entropy, k-mer diversity metrics

## Requirements

- C++20 compatible compiler (GCC 10+, Clang 12+, MSVC 2019+)
- CMake 3.20+
- Optional: Google Test (for tests)
- Optional: Google Benchmark (for benchmarks)

## Building

### Basic Build

```bash
mkdir build && cd build
cmake ..
make -j$(nproc)
```

### Release Build (Optimized)

```bash
mkdir build && cd build
cmake -DCMAKE_BUILD_TYPE=Release ..
make -j$(nproc)
```

### With Tests and Benchmarks

```bash
# Install dependencies (Ubuntu/Debian)
sudo apt-get install libgtest-dev libbenchmark-dev

# Build
mkdir build && cd build
cmake -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTS=ON -DBUILD_BENCHMARKS=ON ..
make -j$(nproc)
```

### Without Tests/Benchmarks

```bash
cmake -DBUILD_TESTS=OFF -DBUILD_BENCHMARKS=OFF ..
```

## Usage

### Running the Demo

```bash
./build/bioflow           # Basic demo
./build/bioflow --benchmark  # With performance benchmarks
```

### Running Tests

```bash
cd build
ctest --output-on-failure
# Or directly:
./bioflow_tests
```

### Running Benchmarks

```bash
./build/bioflow_bench
```

## API Overview

### Sequence Class

```cpp
#include "bioflow/sequence.hpp"
using namespace bioflow;

// Create a sequence
Sequence seq("ATGCGATCGATCG", "my_sequence");

// Access properties
std::cout << "Length: " << seq.length() << std::endl;
std::cout << "GC Content: " << seq.gcContent() << std::endl;

// Transformations
auto complement = seq.complement();
auto reverse_comp = seq.reverseComplement();

// Motif finding
auto positions = seq.findMotifPositions("GATC");
```

### K-mer Counter

```cpp
#include "bioflow/kmer.hpp"
using namespace bioflow;

// Count k-mers
KMerCounter counter(21);
counter.count(seq);

// Get results
std::cout << "Unique 21-mers: " << counter.uniqueCount() << std::endl;
std::cout << "Total 21-mers: " << counter.totalCount() << std::endl;

// Get most frequent
auto top10 = counter.mostFrequent(10);
for (const auto& entry : top10) {
    std::cout << entry.kmer << ": " << entry.count << std::endl;
}
```

### Sequence Alignment

```cpp
#include "bioflow/alignment.hpp"
using namespace bioflow;

Sequence seq1("ACGTACGT");
Sequence seq2("ACGTTCGT");

// Smith-Waterman (local alignment)
auto local = smithWaterman(seq1, seq2);
std::cout << "Score: " << local.score << std::endl;
std::cout << "Identity: " << local.identity() << std::endl;

// Needleman-Wunsch (global alignment)
auto global = needlemanWunsch(seq1, seq2);

// Custom scoring
ScoringMatrix scoring{
    .match_score = 3,
    .mismatch_penalty = -2,
    .gap_open_penalty = -5,
    .gap_extend_penalty = -1
};
auto custom = smithWaterman(seq1, seq2, scoring);
```

### Quality Scores

```cpp
#include "bioflow/quality.hpp"
using namespace bioflow;

// Parse quality string (Phred+33)
QualityScores quality("IIIIIIIIIHHHHH55555", QualityEncoding::Phred33);

std::cout << "Mean quality: " << quality.meanQuality() << std::endl;
std::cout << "Bases >= Q30: " << quality.countAboveThreshold(30) << std::endl;

// Trim low quality bases
auto trimmed = quality.trim(20, 10);  // threshold=20, min_length=10
```

### Statistics

```cpp
#include "bioflow/stats.hpp"
using namespace bioflow;

// Sequence statistics
auto seq_stats = stats::computeStats(seq);
std::cout << "Complexity: " << seq_stats.complexity << std::endl;

// Shannon entropy
double entropy = stats::shannonEntropy(seq);

// K-mer diversity metrics
auto kmer_stats = stats::computeKMerStats(counter);
std::cout << "Simpson index: " << kmer_stats.simpson_index << std::endl;

// Pairwise comparisons
double jaccard = stats::jaccardSimilarity(counter1, counter2);
double cosine = stats::cosineSimilarity(counter1, counter2);
```

## C++20 Features Used

This library showcases modern C++20 features:

- **Concepts**: Type constraints for generic functions
- **Ranges**: Clean, composable algorithms
- **constexpr**: Compile-time computation where possible
- **Spaceship operator**: Automatic comparison operators
- **std::span**: Non-owning views over contiguous data
- **[[nodiscard]]**: Prevent accidental result ignoring
- **Structured bindings**: Clean destructuring

### Example: Concepts and Ranges

```cpp
// Concept for sequence-like types
template<typename T>
concept SequenceLike = requires(T t) {
    { t.bases() } -> std::convertible_to<std::string_view>;
    { t.length() } -> std::convertible_to<size_t>;
};

// Using ranges for clean code
auto gc_count = std::ranges::count_if(bases, [](char c) {
    return c == 'G' || c == 'C';
});
```

## Performance

Typical benchmark results (Release build, -O3 -march=native):

| Operation | Input Size | Time |
|-----------|------------|------|
| GC Content | 20,000 bp | ~5 us |
| K-mer Count (k=21) | 20,000 bp | ~2 ms |
| Smith-Waterman | 1000x1000 | ~50 ms |
| Edit Distance | 1000x1000 | ~3 ms |

Run `./bioflow_bench` for detailed benchmark results on your hardware.

## Project Structure

```
bioflow-cpp/
├── include/bioflow/     # Header files
│   ├── sequence.hpp     # DNA sequence class
│   ├── kmer.hpp         # K-mer counting
│   ├── alignment.hpp    # Alignment algorithms
│   ├── quality.hpp      # Quality scores
│   └── stats.hpp        # Statistics
├── src/                 # Implementation files
│   ├── sequence.cpp
│   ├── kmer.cpp
│   ├── alignment.cpp
│   ├── quality.cpp
│   ├── stats.cpp
│   └── main.cpp
├── tests/               # Google Test tests
├── benchmark/           # Google Benchmark benchmarks
├── CMakeLists.txt
├── README.md
└── COMPARISON.md        # C++ vs Aria comparison
```

## License

This project is part of the Aria language examples and follows the same license.
