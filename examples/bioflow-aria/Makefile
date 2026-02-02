# BioFlow Aria Makefile
# Convenient targets for building and running benchmarks

.PHONY: all build run benchmark compare clean help

# Default Aria compiler location
ARIA_BIN ?= ../../target/release/aria
ARIA_BUILD ?= $(ARIA_BIN) build --release --link

# Output binary
BENCHMARK_BIN = bioflow_aria

# Results directory
RESULTS_DIR = results

# Default target
all: build

# Build Aria compiler if not present
compiler:
	@if [ ! -f "$(ARIA_BIN)" ]; then \
		echo "Building Aria compiler..."; \
		cd ../.. && cargo build --release; \
	else \
		echo "Aria compiler found at $(ARIA_BIN)"; \
	fi

# Build benchmark suite
build: compiler
	@echo "Compiling Aria benchmarks..."
	$(ARIA_BUILD) benchmarks.aria -o $(BENCHMARK_BIN)
	@echo "✓ Build complete: $(BENCHMARK_BIN)"

# Run Aria benchmarks only
run: build
	@echo "Running Aria benchmarks..."
	./$(BENCHMARK_BIN)

# Run all language benchmarks
benchmark: build
	@echo "Running cross-language benchmarks..."
	./run_benchmarks.sh

# Compare with other languages (requires previous benchmark runs)
compare:
	@if [ ! -d "$(RESULTS_DIR)" ] || [ -z "$$(ls -A $(RESULTS_DIR) 2>/dev/null)" ]; then \
		echo "No benchmark results found. Run 'make benchmark' first."; \
		exit 1; \
	fi
	@LATEST=$$(ls -t $(RESULTS_DIR)/comparison_*.md 2>/dev/null | head -1); \
	if [ -n "$$LATEST" ]; then \
		cat "$$LATEST"; \
	else \
		echo "No comparison reports found."; \
	fi

# Quick benchmark (Aria + Python only, faster iteration)
quick: build
	@mkdir -p $(RESULTS_DIR)
	@echo "Running quick benchmarks (Aria + Python)..."
	@./$(BENCHMARK_BIN) | tee $(RESULTS_DIR)/aria_quick.txt
	@cd ../bioflow-python && python3 benchmark.py | tee ../bioflow-aria/$(RESULTS_DIR)/python_quick.txt
	@echo ""
	@echo "✓ Quick benchmark complete"

# Clean build artifacts
clean:
	rm -f $(BENCHMARK_BIN)
	rm -f *.o
	rm -rf $(RESULTS_DIR)
	@echo "✓ Cleaned build artifacts"

# Deep clean (including compiler)
distclean: clean
	cd ../.. && cargo clean
	@echo "✓ Deep clean complete"

# Run tests (if test framework is available)
test: build
	@echo "Running tests..."
	# TODO: Implement test runner
	@echo "⚠ Test framework not yet implemented"

# Format code (if formatter is available)
fmt:
	@echo "Formatting Aria code..."
	# TODO: Implement formatter
	@echo "⚠ Formatter not yet implemented"

# Check for issues without building
check: compiler
	@echo "Checking Aria code..."
	$(ARIA_BIN) check benchmarks.aria

# Show benchmark history
history:
	@if [ -d "$(RESULTS_DIR)" ]; then \
		echo "Benchmark History:"; \
		echo ""; \
		ls -lht $(RESULTS_DIR)/comparison_*.md 2>/dev/null || echo "No comparison reports yet."; \
	else \
		echo "No benchmark history found. Run 'make benchmark' first."; \
	fi

# Display help
help:
	@echo "BioFlow Aria Benchmark Makefile"
	@echo ""
	@echo "Available targets:"
	@echo "  make build      - Build Aria benchmarks"
	@echo "  make run        - Run Aria benchmarks only"
	@echo "  make benchmark  - Run all language benchmarks (Aria, Go, Rust, Python)"
	@echo "  make compare    - Show latest comparison report"
	@echo "  make quick      - Quick benchmark (Aria + Python only)"
	@echo "  make clean      - Remove build artifacts"
	@echo "  make distclean  - Deep clean (including compiler)"
	@echo "  make check      - Check code without building"
	@echo "  make history    - Show benchmark history"
	@echo "  make help       - Show this help message"
	@echo ""
	@echo "Environment variables:"
	@echo "  ARIA_BIN        - Path to Aria compiler (default: ../../target/release/aria)"
	@echo ""
	@echo "Examples:"
	@echo "  make build                    # Build benchmarks"
	@echo "  make run                      # Run Aria benchmarks"
	@echo "  make benchmark                # Run full comparison"
	@echo "  ARIA_BIN=/path/aria make build  # Use custom compiler"
