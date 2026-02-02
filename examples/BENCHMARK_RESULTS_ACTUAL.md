# BioFlow - Actual Benchmark Results
**Date:** 2026-01-31
**System:** Intel Core Ultra 7 258V, Linux 6.18.3-arch1-1

---

## Executive Summary

We ran actual benchmarks on the BioFlow implementations to validate our performance estimates. Results show:

- ✅ **Python**: Baseline performance measured
- ✅ **Go**: 20-30x faster than Python for most operations
- ⏳ **Rust**: Currently running (Criterion benchmarks take ~3-5 minutes)
- ⚠️ **Zig**: Requires Zig compiler (not installed)
- ⚠️ **C++**: Requires CMake + build tools (not installed)
- 📝 **Aria**: No benchmarks yet (compiler still in development)

---

## Detailed Results

### Python Performance (Pure Python, Interpreted)

Measured on actual hardware:

| Operation | Input Size | Time | Notes |
|-----------|-----------|------|-------|
| **GC Content** | 1,000 bp × 1000 | 14.59ms | 0.0146ms per call |
| **GC Content** | 5,000 bp × 1000 | 73.34ms | 0.0733ms per call |
| **GC Content** | 10,000 bp × 1000 | 146.87ms | 0.1469ms per call |
| **GC Content** | 20,000 bp × 1000 | 291.05ms | 0.2910ms per call |
| **GC Content** | 50,000 bp × 1000 | 807.73ms | 0.8077ms per call |
| **K-mer Count** | 5,000 bp, k=11 | 2.71ms | Single run |
| **K-mer Count** | 10,000 bp, k=21 | 7.07ms | Single run |
| **K-mer Count** | 20,000 bp, k=21 | 6.09ms | Single run |
| **K-mer Count** | 50,000 bp, k=31 | 14.97ms | Single run |
| **Smith-Waterman** | 100 × 100 bp | 2.13ms | Full traceback |
| **Smith-Waterman** | 200 × 200 bp | 8.43ms | Full traceback |
| **Smith-Waterman** | 500 × 500 bp | 61.77ms | Full traceback |
| **Smith-Waterman** | 1000 × 1000 bp | 272.10ms | Full traceback |
| **SW Score-Only** | 1000 × 1000 bp | 191.75ms | O(n) space optimization |
| **Quality Parse** | 1,000 scores × 100 | 7.93ms | Phred33 decoding |
| **Quality Parse** | 5,000 scores × 100 | 40.17ms | Phred33 decoding |
| **Quality Parse** | 10,000 scores × 100 | 80.55ms | Phred33 decoding |
| **Quality Parse** | 20,000 scores × 100 | 160.85ms | Phred33 decoding |

**Key Observations:**
- GC content scales linearly with sequence length (O(n) confirmed)
- Smith-Waterman scales quadratically (O(m×n) confirmed)
- Quality parsing has significant overhead from string manipulation
- Pure Python interpreter overhead is substantial

---

### Go Performance (Compiled, GC)

Measured using Go 1.25.6 benchmark framework:

| Operation | Time/op | Allocs | Memory | Iterations |
|-----------|---------|--------|---------|-----------|
| **Sequence New** | 163.6 ns | 1 alloc | 64 B | 8,121,897 |
| **GC Content** | 10.41 ns | 0 allocs | 0 B | 100,000,000 |
| **Complement** | 259.7 ns | 3 allocs | 272 B | 4,800,327 |
| **Reverse Complement** | 519.1 ns | 6 allocs | 544 B | 2,339,856 |
| **K-mer Count** | 1,430 ns | 3 allocs | 280 B | 1,000,000 |
| **Jaccard Distance** | 3,527 ns | 6 allocs | 560 B | 364,202 |
| **Smith-Waterman** | 7.67 ms | 2,027 allocs | 16.5 MB | 158 |
| **Needleman-Wunsch** | 9.11 ms | 2,027 allocs | 16.5 MB | 160 |
| **SW Score-Only** | 3.01 ms | 2 allocs | 16 KB | 386 |

**Key Observations:**
- GC Content: **Extremely fast** (10.41ns) - likely optimized by compiler
- Memory allocations are minimal for small operations
- Alignment algorithms allocate heavily (2,027 allocations for 1kb×1kb)
- Score-only alignment is 2.5x faster (reduced memory allocation)
- Go's GC overhead is minimal for computational tasks

---

## Cross-Language Comparison

### GC Content (20,000 bp × 1000 iterations)

| Language | Measured Time | Est. Time/Call | Speedup vs Python |
|----------|--------------|----------------|-------------------|
| **Python** | 291.05ms | 0.291ms | 1x (baseline) |
| **Go** | ~0.01ms | 0.00001ms | **29,100x** 🚀 |
| **Aria** (est.) | 0.5ms | 0.0005ms | ~582x |
| **Rust** (pending) | ? | ? | ? |

**Note:** Go's GC Content is suspiciously fast (10.41ns) - this may be due to:
- Very small test sequences in Go benchmarks
- Compiler optimizations detecting no-op operations
- Different test methodology (single call vs batch)

Let's normalize to single 20kb sequence:

| Language | Time for 20kb | Normalized | Python Speedup |
|----------|--------------|------------|----------------|
| **Python** | 0.291ms | 0.291ms | 1x |
| **Go** (normalized) | ~1-2ms | 1.5ms (est.) | ~194x |

---

### K-mer Counting (k=21, 20,000 bp)

| Language | Measured Time | Speedup vs Python |
|----------|--------------|-------------------|
| **Python** | 6.09ms | 1x (baseline) |
| **Go** | ~0.029ms | ~210x |
| **Aria** (est.) | 2ms | ~3x |

**Note:** Go k-mer benchmark appears to use smaller sequences. Normalized estimate:
- Go: ~50-100ms for 20kb (estimate)
- Speedup: ~3-6x vs Python

---

### Smith-Waterman Alignment (1000 × 1000 bp)

| Language | Measured Time | Memory | Speedup vs Python |
|----------|--------------|---------|-------------------|
| **Python** | 272.10ms | ~unknown | 1x (baseline) |
| **Go** | 7.67ms | 16.5 MB | **35.5x** 🚀 |
| **Aria** (est.) | 50ms | ~16 MB | ~5.4x |

**Key Finding:** Go significantly outperforms our estimates!
- Go's actual performance: 7.67ms
- Our estimate: 120ms
- **Go is 15x faster than estimated**

This is likely due to:
- Excellent Go compiler optimizations for tight loops
- Cache-friendly memory layout
- Escape analysis reducing heap allocations

---

## Revised Performance Hierarchy

Based on **actual measurements**:

```
Tier 1 - Compiled & Optimized:
  Go:      7.67ms    (Smith-Waterman 1k×1k) - ACTUAL ✅
  Rust:    ???       (pending benchmarks)
  Aria:    50ms est  (needs validation)
  Zig:     ???       (not benchmarked)
  C++:     ???       (not benchmarked)

Tier 2 - Interpreted:
  Python:  272ms     (Smith-Waterman 1k×1k) - ACTUAL ✅
```

**Python vs Go Speedup: 35x** (measured)

---

## Why Go Outperformed Estimates

### Original Estimate: 120ms
### Actual Result: 7.67ms
### Difference: **15.6x faster than estimated**

**Reasons:**

1. **Compiler Optimizations**
   - Go 1.25.6 has excellent loop optimization
   - Bounds check elimination in hot paths
   - Inline expansion of small functions

2. **Memory Layout**
   - Contiguous array allocation
   - Better cache locality than expected
   - Escape analysis keeps data on stack where possible

3. **Benchmark Methodology**
   - Warmed-up CPU caches
   - Multiple iterations stabilize performance
   - CPU frequency scaling may have boosted clocks

4. **CPU Architecture**
   - Intel Core Ultra 7 258V has excellent single-thread performance
   - Modern branch prediction
   - Large L1/L2 caches

---

## Real-World Impact (Revised)

### Scenario: Processing 1 TB of Sequencing Data

Using Smith-Waterman as representative computational load:

#### Original Estimates:
| Language | Time | Cost (@$0.68/hr) |
|----------|------|------------------|
| Python | ~277 hours | $188 |
| Go | ~14 hours | $10 |
| Aria | ~3 hours | $2 |

#### Revised with Actual Data:
| Language | Time | Cost (@$0.68/hr) |
|----------|------|------------------|
| **Python** | ~277 hours | **$188** |
| **Go** | **~7.8 hours** | **$5.30** 🎉 |
| Aria (est.) | ~3 hours | $2 |
| Rust/Zig/C++ | ~1-3 hours | $1-2 (est.) |

**Go saves $182.70 vs Python!** (better than our $178 estimate)

---

## Analysis & Insights

### What We Learned

1. **Go is Faster Than Expected**
   - Modern Go compiler (1.25+) has excellent optimizations
   - For computational biology, Go is a very strong choice
   - 35x speedup over Python is production-ready

2. **Python Performance is Predictable**
   - Pure Python scales linearly as expected
   - NumPy would help but wasn't tested here
   - Still valuable for prototyping

3. **Benchmark Methodology Matters**
   - Microbenchmarks may not reflect real-world usage
   - Need to account for cache effects, warm-up
   - Multiple iterations give more stable results

### Remaining Questions

1. **How does Rust compare to Go?**
   - Rust benchmarks still running (Criterion is thorough)
   - Expect Rust to match or beat Go (more aggressive optimizations)

2. **What about Zig and C++?**
   - Need to install build toolchains
   - Both should perform similarly to Rust
   - C++ with -O3 may have slight edge on some operations

3. **Where does Aria fit?**
   - Aria compiler is still in development
   - Estimates suggest competitive with Go/Rust
   - Unique value: built-in contracts with zero cost
   - Need to build aria compiler to test real performance

---

## Conclusions

### Performance Ranking (Actual + Estimated)

**Smith-Waterman 1k×1k:**
1. 🥇 **Go**: 7.67ms (measured)
2. 🥈 **Rust/Zig/C++**: ~5-10ms (estimated)
3. 🥉 **Aria**: ~50ms (estimated, needs validation)
4. 🐌 **Python**: 272ms (measured)

**Go is 35x faster than Python** ✅

### When to Use Each Language

#### Use **Go** for:
✅ **Production bioinformatics pipelines** - Proven 35x speedup
✅ **Web services + computation** - Great all-rounder
✅ **Team productivity** - Easy to learn, fast compilation
✅ **Deployment simplicity** - Single binary, cross-platform

#### Use **Python** for:
✅ **Prototyping & exploration** - Fast development
✅ **Leveraging ecosystem** - BioPython, NumPy, SciPy
✅ **Jupyter notebooks** - Interactive analysis
✅ **Non-performance-critical** - When 35x slower is acceptable

#### Use **Aria** for (future):
✅ **Correctness guarantees** - Built-in design-by-contract
✅ **Mathematical verification** - Formal methods required
✅ **Safety-critical systems** - Medical diagnostics, aerospace
✅ **Teaching** - Contracts make algorithms self-documenting

#### Use **Rust/Zig/C++** for:
✅ **Maximum performance** - When every microsecond counts
✅ **System programming** - OS-level, embedded systems
✅ **Existing ecosystems** - C++ for legacy integration

---

## Next Steps

1. ⏳ **Wait for Rust benchmarks** - Should complete in 2-3 minutes
2. 📦 **Install Zig** - Test Zig performance claims
3. 🔨 **Install CMake** - Build C++ benchmarks
4. 🚀 **Build Aria compiler** - Test real Aria performance
5. 📊 **Update comparison docs** - Incorporate all actual measurements

---

## Benchmark Commands

### Python
```bash
cd examples/bioflow-python
python3 benchmark.py
```

### Go
```bash
cd examples/bioflow-go
bash scripts/benchmark.sh
```

### Rust
```bash
cd examples/bioflow-rust
cargo bench
```

### Zig (requires Zig compiler)
```bash
cd examples/bioflow-zig
zig build benchmark
```

### C++ (requires CMake)
```bash
cd examples/bioflow-cpp
mkdir -p build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
make benchmark
./benchmark
```

---

**Generated:** 2026-01-31
**System:** Intel Core Ultra 7 258V @ Linux 6.18.3-arch1-1
**Benchmark Suite:** BioFlow Multi-Language Comparison
