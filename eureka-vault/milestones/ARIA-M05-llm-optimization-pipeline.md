# Milestone M05: LLM Optimization Pipeline

## Overview

Design the groundbreaking LLM-assisted optimization system where an AI can suggest code optimizations during compilation, with formal verification ensuring correctness.

## Research Questions

1. How do we make LLM suggestions deterministic for reproducible builds?
2. What verification techniques can prove LLM suggestions are equivalent?
3. How do we cache and version LLM optimizations?
4. What's the security model for LLM in the compiler?

## Core Innovation Target

```ruby
@optimize(level: :aggressive, verify: :formal)
fn matrix_multiply(a, b)
  # LLM might suggest:
  # - Strassen's algorithm for large matrices
  # - SIMD vectorization
  # - Cache-friendly tiling
  #
  # But ONLY if verifier proves equivalence

  # Naive implementation
  result = Matrix.new(a.rows, b.cols)
  for i in 0..<a.rows
    for j in 0..<b.cols
      for k in 0..<a.cols
        result[i,j] += a[i,k] * b[k,j]
      end
    end
  end
  result
end
```

## Competitive Analysis Required

| Tool | Approach | Study Focus |
|------|----------|-------------|
| Compiler Explorer | Optimization visualization | User interface |
| MLIR | Optimization passes | IR for optimization |
| Alive2 | Translation validation | LLVM verification |
| CompCert | Verified compilation | Formal verification |
| Enzyme | AD optimization | Domain-specific opt |

## Tasks

### ARIA-M05-01: Survey LLM code optimization research
- **Description**: Study academic work on LLM for code optimization
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, llm, optimization, academic
- **Deliverables**:
  - Paper survey and synthesis
  - Success/failure case analysis
  - Feasibility assessment

### ARIA-M05-02: Study translation validation techniques
- **Description**: Research equivalence checking for transformations
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, verification, equivalence
- **Deliverables**:
  - Alive2 analysis
  - SMT solver integration patterns
  - Verification coverage limits

### ARIA-M05-03: Research deterministic LLM outputs
- **Description**: Study techniques for reproducible LLM suggestions
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, llm, determinism, builds
- **Deliverables**:
  - Caching strategies
  - Versioning approaches
  - Hash-based verification

### ARIA-M05-04: Design LLM optimization architecture
- **Description**: Design the LLM integration in compiler pipeline
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Blocked by**: ARIA-M05-01, ARIA-M05-02
- **Tags**: research, llm, architecture, design
- **Deliverables**:
  - Pipeline integration points
  - Verification requirements
  - Fallback strategies

### ARIA-M05-05: Design security model
- **Description**: Design security model for LLM suggestions
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, llm, security, design
- **Deliverables**:
  - Threat model
  - Sandboxing requirements
  - Audit logging

### ARIA-M05-06: Prototype LLM optimizer
- **Description**: Build proof-of-concept LLM optimization
- **Dependency**: AGENT_CAPABLE
- **Priority**: normal
- **Blocked by**: ARIA-M05-04, ARIA-M05-05
- **Tags**: prototype, llm, implementation
- **Deliverables**:
  - Working LLM suggestion pipeline
  - Basic equivalence checking
  - Performance measurements

## Implementation Progress

### LLM Optimization Core (COMPLETED - Jan 2026)
- [x] `crates/aria-llm/` crate with full architecture
- [x] OptimizationPipeline with LLM provider integration
- [x] LlmConfig with optimize levels, verify modes, security settings
- [x] OptimizationRequest with domains, hints, type/contract context
- [x] LlmProvider trait with MockProvider, OpenAI/Anthropic stubs
- [x] 24 unit tests passing

### Verification System (COMPLETED - Jan 2026)
- [x] Verifier with pluggable EquivalenceChecker trait
- [x] SyntacticChecker for trivial equivalence
- [x] TestingChecker for property-based verification
- [x] SmtChecker stub for formal verification
- [x] VerificationResult with counterexamples and proof witnesses
- [x] VerificationHint system for guided verification

### Caching System (COMPLETED - Jan 2026)
- [x] OptimizationCache with memory and disk persistence
- [x] CacheKey based on code hash, params, model
- [x] CacheEntry with TTL, hit counting
- [x] LRU eviction policy
- [x] Cache statistics tracking

### Security Model (COMPLETED - Jan 2026)
- [x] SecurityPolicy with sandboxing, blocked patterns
- [x] Input/output size validation
- [x] Endpoint allowlisting
- [x] AuditLog with entry types and violation tracking
- [x] File-based audit persistence

### Remaining Work
- [ ] Actual LLM API integration (OpenAI, Anthropic)
- [ ] SMT solver integration (Z3) for formal verification
- [ ] MIR-level optimization pass integration
- [ ] Performance benchmarking

## Success Criteria

- [x] LLM optimization architecture designed
- [x] Verification approach proven feasible
- [x] Deterministic builds achievable
- [x] Security model documented
- [ ] Prototype demonstrates value (needs LLM integration)

## Key Papers/Resources

1. "Alive2: Bounded Translation Validation for LLVM" - Lopes et al.
2. "CompCert: A Certified Compiler" - Leroy
3. "Large Language Models for Code" - Various surveys
4. "Translation Validation" - Pnueli et al.
5. "Formal Verification of LLVM Passes" - Research papers

## Risks

- **High**: LLM suggestions may not be verifiable in general
- **Medium**: Performance overhead of verification
- **Medium**: Determinism may limit LLM usefulness

## Mitigation

- Start with narrow optimization domains (SIMD, loop tiling)
- Cache verified optimizations aggressively
- Allow opt-in for non-deterministic suggestions with warnings

## Timeline

Target: Q2-Q3 2026 (Innovative, higher risk)

## Related Milestones

- **Depends on**: M06 (Compiler IR)
- **Enables**: M20 (Performance Validation)
- **Novel**: This is Aria's unique differentiator
