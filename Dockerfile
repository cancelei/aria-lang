# syntax=docker/dockerfile:1
# check=error=true

# ============================================================================
# Aria Language - Multi-stage Rust Build
# ============================================================================
# Build: docker build -t aria-lang .
# Run:   docker run --rm aria-lang aria --version
# Test:  docker run --rm aria-lang cargo test

ARG RUST_VERSION=1.75
FROM rust:${RUST_VERSION}-slim AS builder

WORKDIR /app

# Install build dependencies (LLVM for codegen)
RUN apt-get update -qq && \
    apt-get install --no-install-recommends -y \
    build-essential \
    pkg-config \
    llvm-dev \
    libclang-dev \
    clang \
    lld \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace manifests first for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY benches ./benches

# Build release binaries
RUN cargo build --release --workspace

# Run tests to verify build
RUN cargo test --release --workspace

# ============================================================================
# Runtime Stage - Minimal image
# ============================================================================
FROM debian:bookworm-slim AS runtime

WORKDIR /app

# Install runtime dependencies
RUN apt-get update -qq && \
    apt-get install --no-install-recommends -y \
    ca-certificates \
    libclang1 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd --system --gid 1000 aria && \
    useradd aria --uid 1000 --gid 1000 --create-home --shell /bin/bash

# Copy built binaries
COPY --from=builder /app/target/release/aria* /usr/local/bin/

# Copy examples for testing
COPY examples ./examples

USER aria:aria

# Default command shows version
CMD ["aria", "--help"]
