# BioFlow Aria - Usage Guide

Quick reference for running benchmarks and comparing performance.

---

## Quick Start (3 commands)

```bash
# 1. Build Aria compiler (if needed)
cd /path/to/aria-lang
cargo build --release

# 2. Run benchmarks
cd examples/bioflow-aria
make benchmark

# 3. View results
make compare
```

---

## Using Make Commands

### Build Only

```bash
make build
```

Compiles `benchmarks.aria` to native binary.

### Run Aria Benchmarks

```bash
make run
```

Runs only Aria benchmarks (fastest iteration).

### Full Cross-Language Benchmark

```bash
make benchmark
```

Runs benchmarks for:
- Aria (compiled)
- Python (interpreted)
- Go (compiled)
- Rust (compiled)

Generates comparison report automatically.

### View Latest Results

```bash
make compare
```

Displays the most recent comparison report.

### Quick Test (Aria + Python)

```bash
make quick
```

Faster than full benchmark - compares only Aria vs Python.

---

## Manual Benchmark Runs

### Aria Only

```bash
# Build
aria build benchmarks.aria --release --link -o bioflow_aria

# Run
./bioflow_aria
```

### Cross-Language Script

```bash
./run_benchmarks.sh
```

Automatically:
1. Builds Aria implementation
2. Runs Python benchmarks
3. Runs Go benchmarks
4. Runs Rust benchmarks
5. Generates comparison report

---

## Understanding Output

### Aria Benchmark Output

```
=== GC Content Benchmarks ===

| Benchmark | Avg Time | Min Time | Max Time | Iterations |
|-----------|----------|----------|----------|------------|
| GC Content (1000bp) | 0.15ms | 0.14ms | 0.17ms | 1000 |
| GC Content (5000bp) | 0.73ms | 0.71ms | 0.76ms | 1000 |
```

**Columns:**
- **Benchmark:** Operation and input size
- **Avg Time:** Average time per operation
- **Min Time:** Fastest iteration
- **Max Time:** Slowest iteration
- **Iterations:** Number of runs

### Comparison Report

```markdown
## GC Content Calculation

| Input Size | Aria | Go | Rust | Python | Aria vs Python |
|------------|------|-----|------|--------|----------------|
| 1000 bp | 0.15ms | 0.01ms | 0.03ms | 14.59ms | 97.3x |
| 5000 bp | 0.73ms | 0.05ms | 0.15ms | 73.34ms | 100.5x |
```

**Reading the table:**
- **Aria:** Compiled Aria performance
- **Go/Rust:** Compiled alternatives
- **Python:** Interpreted baseline
- **Aria vs Python:** Speedup factor

---

## Interpreting Results

### Performance Tiers

**Tier 1: Compiled, No GC (Fastest)**
- Rust: Maximum performance
- Aria: Performance + safety

**Tier 2: Compiled, GC (Fast)**
- Go: Excellent all-around performance

**Tier 3: Interpreted (Baseline)**
- Python: Development speed priority

### Expected Speedups

| Operation | Typical Aria vs Python Speedup |
|-----------|-------------------------------|
| GC Content | 50-100x |
| K-mer Counting | 10-50x |
| Alignment | 5-20x |

### When Speedups Vary

**Lower speedups (5-20x):**
- Complex algorithms (alignment)
- Many allocations
- Branching-heavy code

**Higher speedups (50-100x):**
- Simple loops
- Cache-friendly access patterns
- Minimal allocations

---

## Customizing Benchmarks

### Adjust Iteration Counts

Edit `benchmarks.aria`:

```aria
# More iterations = more stable results, but slower
let iterations = 1000  # Default

# For quick testing
let iterations = 10

# For production benchmarks
let iterations = 10000
```

### Add New Test Sizes

```aria
# Original
let sizes = [1000, 5000, 10000, 20000, 50000]

# Add larger sequences
let sizes = [1000, 5000, 10000, 20000, 50000, 100000, 500000]

# Add smaller sequences
let sizes = [100, 500, 1000, 5000, 10000]
```

### Benchmark Custom Algorithms

```aria
fn bench_my_algorithm() -> [BenchmarkResult]
  println("\n=== My Algorithm ===\n")

  benchmark_scaling(
    "My Algorithm",
    [1000, 10000, 100000],  # sizes
    100,                     # iterations
    |size| {
      let input = generate_sequence(size)
      my_algorithm(input)
    }
  )
end

fn main()
  # ... existing benchmarks ...

  let my_results = bench_my_algorithm()
  all_results.extend(my_results)

  # ...
end
```

---

## Troubleshooting

### Aria Compiler Not Found

**Error:**
```
make: aria: Command not found
```

**Solution:**
```bash
cd ../..
cargo build --release
export ARIA_BIN=$(pwd)/target/release/aria
cd examples/bioflow-aria
make build
```

### Compilation Fails

**Error:**
```
aria build benchmarks.aria --release --link -o bioflow_aria
Error: [compilation error details]
```

**Causes:**
- Aria compiler still in development
- Language features not yet implemented
- Syntax changes in progress

**Workaround:**
The benchmark script will automatically skip Aria if compilation fails and continue with other languages.

### Python Benchmarks Too Slow

Python benchmarks run 1000 iterations by default (20+ seconds).

**Speed up:**
```bash
cd ../bioflow-python
# Edit benchmark.py
vim benchmark.py
# Change iterations = 1000 to iterations = 100
```

### Go Benchmarks Not Running

**Error:**
```
go: command not found
```

**Install Go:**
```bash
# Ubuntu/Debian
sudo apt install golang-go

# Arch Linux
sudo pacman -S go

# macOS
brew install go
```

### Rust Benchmarks Take Forever

Criterion (Rust benchmark framework) is thorough - it runs many iterations to get stable results.

**Speed up:**
```bash
cd ../bioflow-rust
# Quick benchmark (less accurate)
cargo bench --profile bench -- --quick
```

---

## Advanced Usage

### Profile Aria Code

```bash
# Build with debug symbols
aria build benchmarks.aria --debug -o bioflow_aria_debug

# Run with perf (Linux)
perf record -g ./bioflow_aria_debug
perf report

# Run with Valgrind
valgrind --tool=callgrind ./bioflow_aria_debug
kcachegrind callgrind.out.*
```

### Memory Usage Analysis

```bash
# Valgrind massif
valgrind --tool=massif ./bioflow_aria
ms_print massif.out.*

# Heaptrack (if available)
heaptrack ./bioflow_aria
heaptrack_gui heaptrack.bioflow_aria.*
```

### Compare Specific Operations

```bash
# Extract GC content results only
grep "GC Content" results/comparison_*.md | tail -1

# Compare k-mer performance
grep "K-mer" results/comparison_*.md | tail -1
```

### Benchmark Over Time

```bash
# Run daily benchmarks
make benchmark
mv results/comparison_*.md results/comparison_$(date +%Y%m%d).md

# Track performance regression
diff results/comparison_20260130.md results/comparison_20260131.md
```

---

## Environment Variables

### ARIA_BIN

Override Aria compiler location:

```bash
export ARIA_BIN=/custom/path/to/aria
make build
```

### Benchmark Configuration

```bash
# Increase iterations
export BENCH_ITERATIONS=10000
./bioflow_aria

# Change result directory
export RESULTS_DIR=custom_results
./run_benchmarks.sh
```

---

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Benchmark

on: [push, pull_request]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1

      - name: Build Aria
        run: cargo build --release

      - name: Run Benchmarks
        run: |
          cd examples/bioflow-aria
          make benchmark

      - name: Upload Results
        uses: actions/upload-artifact@v3
        with:
          name: benchmark-results
          path: examples/bioflow-aria/results/
```

---

## Best Practices

### For Development

1. Use `make quick` for fast iteration
2. Run `make benchmark` before committing
3. Track performance with `make history`

### For Production

1. Use release builds (`--release`)
2. Run many iterations (1000+)
3. Test on target hardware
4. Consider warm-up effects

### For Comparison

1. Run all languages on same hardware
2. Same input data across languages
3. Multiple runs for stability
4. Report min/max/avg times

---

## Getting Help

### Documentation

- `README.md` - Full documentation
- `USAGE.md` - This guide
- `make help` - Quick command reference

### Issues

If benchmarks fail or produce unexpected results:

1. Check Aria compiler version
2. Verify input data integrity
3. Review error messages
4. Check system resources (CPU, memory)

---

**Last Updated:** 2026-01-31
**Aria Version:** Development
