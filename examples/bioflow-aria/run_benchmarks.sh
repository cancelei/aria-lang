#!/bin/bash
# BioFlow Aria - Cross-Language Benchmark Runner
# Runs benchmarks across all implementations and generates comparison

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EXAMPLES_DIR="$(dirname "$SCRIPT_DIR")"
PROJECT_ROOT="$(dirname "$EXAMPLES_DIR")"

echo "======================================================================"
echo "BioFlow Cross-Language Benchmark Suite"
echo "======================================================================"
echo "Date: $(date)"
echo "System: $(uname -s) $(uname -r)"
echo "CPU: $(lscpu | grep 'Model name' | cut -d: -f2 | xargs)"
echo ""

# Create results directory
RESULTS_DIR="$SCRIPT_DIR/results"
mkdir -p "$RESULTS_DIR"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")

# ============================================================================
# Aria Benchmarks
# ============================================================================

echo ""
echo "=== Building Aria Implementation ==="
echo ""

if [ -f "$PROJECT_ROOT/target/debug/aria" ]; then
    ARIA_BIN="$PROJECT_ROOT/target/debug/aria"
elif [ -f "$PROJECT_ROOT/target/release/aria" ]; then
    ARIA_BIN="$PROJECT_ROOT/target/release/aria"
else
    echo "ERROR: Aria compiler not found!"
    echo "Please build Aria first: cargo build --release"
    exit 1
fi

echo "Found Aria compiler at: $ARIA_BIN"

# Compile Aria benchmarks
cd "$SCRIPT_DIR"
echo "Compiling benchmarks.aria..."

if $ARIA_BIN build benchmarks.aria --release --link -o bioflow_aria 2>&1; then
    echo "✓ Aria compilation successful"
else
    echo "✗ Aria compilation failed"
    echo ""
    echo "Note: Aria compiler is still in development."
    echo "Some features may not be fully implemented yet."
    echo "Skipping Aria benchmarks for now."
    SKIP_ARIA=1
fi

if [ -z "$SKIP_ARIA" ] && [ -f "./bioflow_aria" ]; then
    echo ""
    echo "=== Running Aria Benchmarks ==="
    echo ""
    ./bioflow_aria | tee "$RESULTS_DIR/aria_${TIMESTAMP}.txt"
else
    echo "⚠ Skipping Aria benchmarks (compiler not ready)"
    echo "# Aria benchmarks skipped - compiler in development" > "$RESULTS_DIR/aria_${TIMESTAMP}.txt"
fi

# ============================================================================
# Python Benchmarks
# ============================================================================

echo ""
echo "=== Running Python Benchmarks ==="
echo ""

cd "$EXAMPLES_DIR/bioflow-python"

if command -v python3 &> /dev/null; then
    python3 benchmark.py | tee "$RESULTS_DIR/python_${TIMESTAMP}.txt"
else
    echo "⚠ Python3 not found, skipping Python benchmarks"
    echo "# Python not available" > "$RESULTS_DIR/python_${TIMESTAMP}.txt"
fi

# ============================================================================
# Go Benchmarks
# ============================================================================

echo ""
echo "=== Running Go Benchmarks ==="
echo ""

cd "$EXAMPLES_DIR/bioflow-go"

if command -v go &> /dev/null; then
    echo "Go version: $(go version)"
    bash scripts/benchmark.sh | tee "$RESULTS_DIR/go_${TIMESTAMP}.txt"
else
    echo "⚠ Go not found, skipping Go benchmarks"
    echo "# Go not available" > "$RESULTS_DIR/go_${TIMESTAMP}.txt"
fi

# ============================================================================
# Rust Benchmarks
# ============================================================================

echo ""
echo "=== Running Rust Benchmarks ==="
echo ""

cd "$EXAMPLES_DIR/bioflow-rust"

if command -v cargo &> /dev/null; then
    echo "Rust version: $(rustc --version)"
    cargo bench --quiet | tee "$RESULTS_DIR/rust_${TIMESTAMP}.txt"
else
    echo "⚠ Cargo not found, skipping Rust benchmarks"
    echo "# Rust not available" > "$RESULTS_DIR/rust_${TIMESTAMP}.txt"
fi

# ============================================================================
# Generate Comparison Report
# ============================================================================

echo ""
echo "=== Generating Comparison Report ==="
echo ""

cd "$SCRIPT_DIR"
python3 compare_results.py \
    --aria "$RESULTS_DIR/aria_${TIMESTAMP}.txt" \
    --python "$RESULTS_DIR/python_${TIMESTAMP}.txt" \
    --go "$RESULTS_DIR/go_${TIMESTAMP}.txt" \
    --rust "$RESULTS_DIR/rust_${TIMESTAMP}.txt" \
    --output "$RESULTS_DIR/comparison_${TIMESTAMP}.md"

echo ""
echo "======================================================================"
echo "Benchmark Complete!"
echo "======================================================================"
echo ""
echo "Results saved to: $RESULTS_DIR/"
echo ""
echo "Files generated:"
echo "  - aria_${TIMESTAMP}.txt"
echo "  - python_${TIMESTAMP}.txt"
echo "  - go_${TIMESTAMP}.txt"
echo "  - rust_${TIMESTAMP}.txt"
echo "  - comparison_${TIMESTAMP}.md"
echo ""
echo "View comparison report:"
echo "  cat $RESULTS_DIR/comparison_${TIMESTAMP}.md"
echo ""
