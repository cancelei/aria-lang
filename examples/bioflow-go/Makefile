.PHONY: help build test bench clean run-server install lint fmt vet tidy coverage

# Default target
.DEFAULT_GOAL := help

# Variables
GO := go
GOFLAGS := -v
BINARY_DIR := bin
CLI_BINARY := $(BINARY_DIR)/bioflow
SERVER_BINARY := $(BINARY_DIR)/bioflow-server

help: ## Show this help message
	@echo 'BioFlow - Genomic Sequence Analysis Tool'
	@echo ''
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Available targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-15s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

build: ## Build all binaries
	@echo "Building binaries..."
	@mkdir -p $(BINARY_DIR)
	$(GO) build $(GOFLAGS) -o $(CLI_BINARY) ./cmd/bioflow
	$(GO) build $(GOFLAGS) -o $(SERVER_BINARY) ./cmd/bioflow-server
	@echo "Build complete!"

build-cli: ## Build CLI binary only
	@mkdir -p $(BINARY_DIR)
	$(GO) build $(GOFLAGS) -o $(CLI_BINARY) ./cmd/bioflow

build-server: ## Build server binary only
	@mkdir -p $(BINARY_DIR)
	$(GO) build $(GOFLAGS) -o $(SERVER_BINARY) ./cmd/bioflow-server

test: ## Run all tests
	@echo "Running tests..."
	$(GO) test -v -race ./...

test-short: ## Run short tests only
	@echo "Running short tests..."
	$(GO) test -v -short ./...

bench: ## Run benchmarks
	@echo "Running benchmarks..."
	$(GO) test -bench=. -benchmem ./...

bench-alignment: ## Run alignment benchmarks
	@echo "Running alignment benchmarks..."
	$(GO) test -bench=. -benchmem ./internal/alignment/...

bench-kmer: ## Run k-mer benchmarks
	@echo "Running k-mer benchmarks..."
	$(GO) test -bench=. -benchmem ./internal/kmer/...

coverage: ## Run tests with coverage report
	@echo "Running tests with coverage..."
	$(GO) test -v -race -coverprofile=coverage.out -covermode=atomic ./...
	$(GO) tool cover -html=coverage.out -o coverage.html
	@echo "Coverage report generated: coverage.html"

lint: ## Run linter (golangci-lint)
	@echo "Running linter..."
	@which golangci-lint > /dev/null || (echo "golangci-lint not installed. Run 'make install-tools'" && exit 1)
	golangci-lint run --timeout=5m

fmt: ## Format code
	@echo "Formatting code..."
	$(GO) fmt ./...
	gofmt -s -w .

vet: ## Run go vet
	@echo "Running go vet..."
	$(GO) vet ./...

tidy: ## Run go mod tidy
	@echo "Tidying go.mod..."
	$(GO) mod tidy

download: ## Download dependencies
	@echo "Downloading dependencies..."
	$(GO) mod download

verify: ## Verify dependencies
	@echo "Verifying dependencies..."
	$(GO) mod verify

clean: ## Clean build artifacts
	@echo "Cleaning build artifacts..."
	rm -rf $(BINARY_DIR)
	rm -f coverage.out coverage.html
	$(GO) clean -cache -testcache

run-cli: build-cli ## Build and run CLI with example
	./$(CLI_BINARY) info -seq "ATGCATGCATGC"

run-server: build-server ## Build and run API server
	./$(SERVER_BINARY) -port 8080

install-tools: ## Install development tools
	@echo "Installing development tools..."
	@which golangci-lint > /dev/null || curl -sSfL https://raw.githubusercontent.com/golangci/golangci-lint/master/install.sh | sh -s -- -b $$(go env GOPATH)/bin
	@echo "Tools installed!"

demo: build-cli ## Run demonstration
	@echo "=== BioFlow Demo ==="
	@echo ""
	@echo "1. Sequence Info:"
	./$(CLI_BINARY) info -seq "ATGCATGCATGC"
	@echo ""
	@echo "2. GC Content:"
	./$(CLI_BINARY) gc -seq "ATGCATGCATGC"
	@echo ""
	@echo "3. K-mer Analysis:"
	./$(CLI_BINARY) kmer -seq "ATGATGATGATG" -k 3 -top 5
	@echo ""
	@echo "4. Sequence Alignment:"
	./$(CLI_BINARY) align -seq1 "ATGCATGC" -seq2 "ATGCGGGG"

check: tidy fmt vet test ## Run all checks (fmt, vet, test)

ci: check build ## Run CI pipeline (check + build)

.PHONY: all
all: clean tidy check build ## Clean, tidy, check, and build everything
