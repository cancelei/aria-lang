#!/bin/bash
# BioFlow Go Benchmark Script
#
# This script runs comprehensive benchmarks and saves results.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
RESULTS_DIR="$PROJECT_DIR/benchmark_results"

# Create results directory
mkdir -p "$RESULTS_DIR"

# Get timestamp for this run
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
RESULT_FILE="$RESULTS_DIR/benchmark_$TIMESTAMP.txt"

echo "BioFlow Go Benchmarks"
echo "===================="
echo "Date: $(date)"
echo "Go version: $(go version)"
echo ""

cd "$PROJECT_DIR"

# Run all benchmarks
echo "Running benchmarks..."
echo ""

{
    echo "BioFlow Go Benchmark Results"
    echo "============================"
    echo "Date: $(date)"
    echo "Go version: $(go version)"
    echo ""

    echo "=== Sequence Package ==="
    go test -bench=. -benchmem ./internal/sequence/... 2>&1
    echo ""

    echo "=== K-mer Package ==="
    go test -bench=. -benchmem ./internal/kmer/... 2>&1
    echo ""

    echo "=== Alignment Package ==="
    go test -bench=. -benchmem ./internal/alignment/... 2>&1
    echo ""

    echo "=== Stats Package ==="
    go test -bench=. -benchmem ./internal/stats/... 2>&1
    echo ""

} | tee "$RESULT_FILE"

echo ""
echo "Results saved to: $RESULT_FILE"

# Generate summary
echo ""
echo "=== Benchmark Summary ==="
grep "Benchmark" "$RESULT_FILE" | head -20 || true
