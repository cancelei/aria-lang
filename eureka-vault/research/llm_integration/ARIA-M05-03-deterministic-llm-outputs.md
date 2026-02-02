# ARIA-M05-03: Deterministic LLM Outputs Research

**Task ID**: ARIA-M05-03
**Status**: Completed
**Date**: 2026-01-14
**Focus**: Research techniques for reproducible LLM outputs in compilation

---

## Executive Summary

LLM non-determinism is a fundamental challenge for reproducible builds. This research analyzes 2025 breakthroughs in deterministic inference, caching strategies, and versioning approaches for Aria's LLM-assisted optimization pipeline.

---

## 1. Overview

### 1.1 The Problem

```
Same prompt + same model ≠ same output

Why?
- Floating-point non-associativity
- Batch-dependent computation order
- Load balancing across GPUs
- Model updates by providers
```

### 1.2 Why It Matters for Aria

- **Reproducible builds**: Same source → same binary
- **CI/CD reliability**: Tests must be deterministic
- **Debugging**: Need to reproduce optimization decisions
- **Trust**: Users must understand what compiler does

---

## 2. Root Causes of Non-Determinism

### 2.1 Floating-Point Arithmetic

```
// Floating-point addition is not associative
(a + b) + c ≠ a + (b + c)

// Different execution orders → different results
// Small differences cascade through transformer layers
```

### 2.2 Batch Dependency

> "Our request's output does depend on the parallel user requests… it's because our forward pass lacks 'batch invariance', causing our request's output to depend on the batch size."

### 2.3 Provider Limitations

| Provider | Determinism Guarantee |
|----------|----------------------|
| OpenAI | "Mostly deterministic" with seed |
| Anthropic | No guarantee, even with temp=0 |
| Self-hosted | Controllable with effort |

---

## 3. 2025 Breakthroughs

### 3.1 Batch-Invariant Kernels (SGLang)

```
Thinking Machines Lab validated:
- Model: Qwen/Qwen3-235B-Instruct
- Prompt: "Tell me about Richard Feynman" (temp=0)
- Baseline: 80 unique completions across 1000 samples
- With batch-invariant kernels: ALL 1000 identical
```

### 3.2 TBIK (Tree-Based Invariant Kernels)

New 2025 paper proposes TBIK:
- Deterministic across tensor-parallel sizes
- Solves training-inference mismatch
- Fundamental solution for RL training

### 3.3 SGLang Performance

With batch-invariant kernels + CUDA graphs:
- 2.8x acceleration maintained
- Only 34.35% performance overhead
- Full determinism achieved

---

## 4. Practical Engineering Solutions

### 4.1 Caching Strategy

```python
# Content-addressable caching
cache_key = hash(
    model_version,
    prompt,
    retrieved_context,
    parameters
)

if cache_key in cache:
    return cache[cache_key]
else:
    result = generate(prompt)
    cache[cache_key] = result
    return result
```

### 4.2 Version Pinning

```yaml
# Lock file for LLM optimization
llm-optimization:
  model: "aria-opt-v1.2.3"
  tokenizer: "aria-tok-v1.2.3"
  inference-lib: "vllm-0.6.3"
  seed: 42
  temperature: 0
  cached-results-hash: "abc123..."
```

### 4.3 Golden Test Snapshots

```rust
// Add prompt "golden" snapshot tests to CI
#[test]
fn test_optimization_determinism() {
    let prompt = optimization_prompt(code);
    let result1 = llm_optimize(prompt);
    let result2 = llm_optimize(prompt);
    assert_eq!(result1, result2);
}
```

### 4.4 vLLM Reproducibility Settings

```python
# vLLM offline mode
export VLLM_ENABLE_V1_MULTIPROCESSING=0  # Deterministic scheduling

# Same hardware + same vLLM version required
# Batch invariance option for scheduling-independent results
```

---

## 5. Caching Benefits

### 5.1 Content-Addressable Cache

```
Benefits of stable outputs:
- Key caches by hash of inputs
- Reuse across requests, regions, deployments
- Cut inference costs
- Smooth tail latency
- No unexpected output drift
```

### 5.2 Cache Architecture

```
┌─────────────────────────────────────────────────┐
│                 Aria Compiler                    │
│                                                 │
│  Source Code                                    │
│       ↓                                         │
│  [Hash: source + context + optimization-level]  │
│       ↓                                         │
│  ┌─────────────────────────────────────────┐   │
│  │         LLM Optimization Cache           │   │
│  │                                          │   │
│  │  cache_key → verified_optimization       │   │
│  │                                          │   │
│  │  Local cache → Remote cache → Generate   │   │
│  └─────────────────────────────────────────┘   │
│       ↓                                         │
│  Apply optimization (if verified)              │
│       ↓                                         │
│  Binary                                        │
└─────────────────────────────────────────────────┘
```

---

## 6. Recommendations for Aria

### 6.1 Determinism Strategy

```aria
# Three-tier approach:

# Tier 1: Cached optimizations (default)
# - Pre-computed, verified optimizations
# - 100% deterministic
# - Fastest

# Tier 2: Self-hosted deterministic inference
# - Batch-invariant kernels
# - Version-pinned models
# - Reproducible within same hardware

# Tier 3: Best-effort with logging
# - Cloud API calls
# - Log all decisions for debugging
# - Flag as non-deterministic in build
```

### 6.2 Build Modes

```toml
# aria.toml
[build.llm-optimization]
mode = "cached"  # cached | deterministic | best-effort

[build.llm-optimization.cached]
cache-url = "https://aria-cache.example.com"
fallback = "deterministic"

[build.llm-optimization.deterministic]
model = "aria-opt:1.0"
inference-engine = "vllm"
batch-invariant = true

[build.llm-optimization.best-effort]
provider = "anthropic"
log-decisions = true
cache-results = true
```

### 6.3 Verification Pipeline

```aria
# Every LLM suggestion must be verified
LLMOptimization {
  fn suggest(code: IR) -> Option[OptimizedIR]
    suggestion = self.model.generate(prompt_for(code))

    # Verify equivalence
    if Verifier.equivalent(code, suggestion)
      Some(suggestion)
    else
      # Log failed verification
      log_verification_failure(code, suggestion)
      None
    end
  end
}
```

### 6.4 Cache Key Design

```aria
# Comprehensive cache key
struct OptimizationCacheKey {
  source_hash: Hash,           # Hash of input IR
  optimization_level: Level,   # O0, O1, O2, O3
  target: Target,              # native, wasm32
  model_version: String,       # "aria-opt:1.0.3"
  context_hash: Hash,          # Surrounding code context
}

fn cache_key(ir: IR, config: Config) -> OptimizationCacheKey
  OptimizationCacheKey {
    source_hash: ir.content_hash(),
    optimization_level: config.opt_level,
    target: config.target,
    model_version: config.llm.model_version,
    context_hash: ir.context.content_hash(),
  }
end
```

### 6.5 Audit Trail

```aria
# Track all LLM decisions for debugging
struct OptimizationDecision {
  timestamp: Instant,
  cache_key: OptimizationCacheKey,
  cache_hit: Bool,
  suggestion: Option[String],
  verification_result: VerificationResult,
  applied: Bool,
}

# Store in build artifacts
fn record_decision(decision: OptimizationDecision)
  BuildArtifacts.append("llm-decisions.jsonl", decision.to_json())
end
```

---

## 7. Security Considerations

### 7.1 Threat Model

| Threat | Mitigation |
|--------|------------|
| Malicious suggestions | Formal verification |
| Cache poisoning | Signed cache entries |
| Model manipulation | Self-hosted models |
| Supply chain attack | Pinned model hashes |

### 7.2 Defense in Depth

```aria
# Multi-layer verification
fn apply_optimization(original: IR, suggested: IR) -> Result[IR, Error]
  # Layer 1: Syntactic validation
  if not suggested.is_valid_ir()
    return Err(InvalidIR)
  end

  # Layer 2: Type checking
  if not TypeChecker.check(suggested)
    return Err(TypeMismatch)
  end

  # Layer 3: Equivalence verification
  if not Verifier.equivalent(original, suggested)
    return Err(NotEquivalent)
  end

  # Layer 4: Resource bounds check
  if suggested.estimated_size() > original.estimated_size() * 2
    return Err(SuspiciousGrowth)
  end

  Ok(suggested)
end
```

---

## 8. Key Resources

1. [LLM Consistency 2025](https://www.keywordsai.co/blog/llm_consistency_2025)
2. [SGLang Deterministic Inference](https://lmsys.org/blog/2025-09-22-sglang-deterministic/)
3. [vLLM Reproducibility](https://docs.vllm.ai/en/latest/usage/reproducibility/)
4. [Defeating Nondeterminism in LLM Inference](https://www.propelcode.ai/blog/defeating-nondeterminism-in-llm-inference-ramifications)
5. [Engineering Near-Deterministic LLM Systems](https://medium.com/@adnanmasood/from-probabilistic-to-predictable-engineering-near-deterministic-llm-systems-for-consistent-6e8e62cf45f6)

---

## 9. Open Questions

1. Should Aria ship pre-computed optimization caches?
2. How do we version LLM models for reproducibility?
3. What's the fallback when cache misses occur?
4. How do we handle cross-platform optimization differences?
