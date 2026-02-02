# ARIA-M05-04: LLM-Assisted Optimization Verification Framework

**Task ID**: ARIA-M05-04
**Status**: Completed
**Date**: 2026-01-15
**Agent**: VECTOR (Research)
**Focus**: Comprehensive verification framework for LLM-suggested optimizations

---

## Executive Summary

This research synthesizes findings from ARIA-M05-01 through ARIA-M05-03 into a unified verification framework for Aria's LLM-assisted optimization pipeline. The framework addresses: (1) how to verify LLM suggestions are semantics-preserving, (2) test generation techniques for validation, (3) determinism guarantees for reproducible builds, and (4) security considerations for LLM integration in the compiler.

---

## 1. Translation Validation Techniques Survey

### 1.1 Theoretical Foundation

Translation validation, introduced by [Pnueli et al.](https://link.springer.com/chapter/10.1007/3-540-front-matter), verifies each individual compilation produces correct output rather than proving the compiler correct once. This per-compilation approach is ideal for LLM-suggested optimizations where suggestions vary unpredictably.

```
Source IR -----> [LLM Optimization] -----> Optimized IR
     |                                          |
     |            [Validator]                   |
     +------------------+----------------------+
                        |
                 Equivalent? / Counterexample
```

### 1.2 Alive2: State-of-the-Art LLVM Validation

[Alive2](https://github.com/AliveToolkit/alive2) is the gold standard for translation validation:

**Architecture**:
- **IR Parser**: Converts LLVM IR to internal representation
- **SMT Encoder**: Translates semantics to Z3 constraints
- **Refinement Checker**: Verifies target refines source
- **Counterexample Generator**: Produces failing inputs

**Key Results** (from [PLDI 2021 paper](https://users.cs.utah.edu/~regehr/alive2-pldi21.pdf)):
- Found **47 new bugs** in LLVM
- **21 bugs** in memory optimizations specifically
- **8 patches** to LLVM Language Reference
- Zero false positives by design

**Capabilities**:
| Feature | Status |
|---------|--------|
| Arithmetic operations | Full support |
| Memory operations | Full support with precise model |
| Poison/undef values | Full support |
| Bounded loops | Supported with bounds |
| Inter-procedural | Not supported |

### 1.3 CompCert: Verified Compilation Approach

[CompCert](https://compcert.org/) takes a different approach - the compiler itself is formally verified in Coq:

**Properties** (from [Release 25.04i](https://www.absint.com/factsheets/factsheet_compcert_c_web.pdf)):
- Mathematical proof of correctness (no undefined behavior introduction)
- **16 transformation passes** with 10 intermediate languages
- All passes proven to preserve semantics
- 2022 ACM Software System Award recipient

**Trade-offs**:
| Aspect | CompCert | Translation Validation |
|--------|----------|------------------------|
| Guarantee strength | Compiler-wide proof | Per-compilation |
| Optimization power | Limited | Unrestricted |
| Development cost | Extremely high | Moderate |
| Flexibility | Low | High |

### 1.4 SMT-Based Verification

[Z3 Theorem Prover](https://github.com/Z3Prover/z3) underpins modern translation validation:

**Theory Selection for Aria**:
| Theory | Use Case | Decidability |
|--------|----------|--------------|
| QF_BV | Bit-vector operations | Decidable |
| QF_LIA | Linear integer arithmetic | Decidable |
| QF_NIA | Non-linear arithmetic | Semi-decidable |
| Arrays | Memory modeling | Decidable |
| UF | Function abstraction | Decidable |

**SMT Encoding Example**:
```smt
; Source: x + 0
(define-fun source ((x (_ BitVec 32))) (_ BitVec 32)
  (bvadd x #x00000000))

; Target: x (LLM suggested simplification)
(define-fun target ((x (_ BitVec 32))) (_ BitVec 32)
  x)

; Verification: Check if there exists any counterexample
(assert (not (forall ((x (_ BitVec 32)))
  (= (source x) (target x)))))
(check-sat)  ; UNSAT = equivalent
```

### 1.5 AI + Formal Verification Synergy

[Martin Kleppmann's 2025 analysis](https://martin.kleppmann.com/2025/12/08/ai-formal-verification.html) argues for combining LLMs with formal verification:

**Key Arguments**:
1. **LLMs need verification**: AI-generated code necessitates verification because human review becomes impractical at scale
2. **Verification counteracts imprecision**: Formal proofs counteract "the imprecise and probabilistic nature of LLMs"
3. **Proof checking as oracle**: If LLMs "hallucinate nonsense, the proof checker will reject any invalid proof"
4. **Automation trajectory**: Full automation of proof writing appears feasible soon

**Implication for Aria**: LLM optimization suggestions can be treated as untrusted input that must pass formal verification before acceptance.

---

## 2. Verification Framework Design

### 2.1 Multi-Layer Verification Architecture

```
+------------------------------------------------------------------+
|                    Aria LLM Optimization Pipeline                  |
+------------------------------------------------------------------+
|                                                                    |
|  Source IR                                                         |
|       |                                                            |
|       v                                                            |
|  +-------------------+         +----------------------------+      |
|  | LLM Optimization  |-------->| Layer 1: Syntactic Check   |      |
|  | Suggestion Engine |         | - Valid IR syntax          |      |
|  +-------------------+         | - Type preservation        |      |
|                                +-------------+--------------+      |
|                                              |                     |
|                                              v                     |
|                                +----------------------------+      |
|                                | Layer 2: Semantic Check    |      |
|                                | - Effect preservation      |      |
|                                | - Contract satisfaction    |      |
|                                +-------------+--------------+      |
|                                              |                     |
|                                              v                     |
|                                +----------------------------+      |
|                                | Layer 3: SMT Verification  |      |
|                                | - Alive2-style encoding    |      |
|                                | - Z3 equivalence check     |      |
|                                +-------------+--------------+      |
|                                              |                     |
|                                              v                     |
|                                +----------------------------+      |
|                                | Layer 4: Property Testing  |      |
|                                | - Fuzzing (Csmith-style)   |      |
|                                | - Metamorphic testing      |      |
|                                +-------------+--------------+      |
|                                              |                     |
|                           +--------+--------+--------+             |
|                           |        |                 |             |
|                        Verified  Counterexample   Timeout          |
|                           |        |                 |             |
|                           v        v                 v             |
|                      Accept    Reject with      Extended Test      |
|                                 reason          or Reject          |
+------------------------------------------------------------------+
```

### 2.2 Layer 1: Syntactic Validation

Before expensive verification, validate basic structural properties:

```aria
module SyntacticValidator
  fn validate(source: IR, target: IR) -> Result[Unit, SyntaxError]
    # Check IR well-formedness
    target.verify_structure()?

    # Ensure type signatures match
    if source.type_signature != target.type_signature
      return Err(TypeMismatch(source.type_signature, target.type_signature))
    end

    # Verify no new external dependencies
    new_deps = target.external_calls - source.external_calls
    if not new_deps.empty?
      return Err(NewDependencies(new_deps))
    end

    Ok(())
  end
end
```

### 2.3 Layer 2: Semantic Property Checking

Leverage Aria's contract system for semantic validation:

```aria
module SemanticValidator
  fn validate(source: IR, target: IR) -> Result[Unit, SemanticError]
    # Extract contracts from source
    contracts = source.contracts

    # Verify target satisfies all source contracts
    for contract in contracts
      if not target.satisfies(contract)
        return Err(ContractViolation(contract))
      end
    end

    # Check effect preservation
    if source.effects != target.effects
      # Allow effect reduction (pure is subset of all)
      if not target.effects.subset_of(source.effects)
        return Err(EffectEscalation(source.effects, target.effects))
      end
    end

    Ok(())
  end
end
```

### 2.4 Layer 3: SMT-Based Equivalence Verification

Core verification using Z3:

```aria
module SMTVerifier
  struct Config {
    timeout: Duration = 30.seconds,
    memory_precision: MemoryModel = :precise,
    loop_bound: Int = 8,
  }

  fn verify_equivalence(
    source: IR,
    target: IR,
    config: Config = Config.default
  ) -> VerificationResult

    solver = Z3.Solver.new(timeout: config.timeout)

    # Encode source semantics
    source_formula = SMTEncoder.encode(source, config.memory_precision)

    # Encode target semantics
    target_formula = SMTEncoder.encode(target, config.memory_precision)

    # Check: exists input where source != target?
    solver.add(Not(ForAll(
      source.inputs,
      Eq(source_formula, target_formula)
    )))

    match solver.check
      :unsat ->
        VerificationResult.Verified(
          proof_time: solver.elapsed,
          strategy: :smt_complete
        )
      :sat ->
        VerificationResult.Counterexample(
          model: solver.model,
          witness: extract_witness(solver.model)
        )
      :unknown ->
        VerificationResult.Timeout(
          elapsed: solver.elapsed,
          partial_result: solver.partial_model
        )
    end
  end
end
```

### 2.5 Layer 4: Property-Based Testing Fallback

When SMT times out, use property-based testing:

```aria
module PropertyTester
  # Inspired by QuickCheck/Hypothesis
  fn test_equivalence(
    source: IR,
    target: IR,
    config: TestConfig
  ) -> TestResult

    generator = InputGenerator.for_signature(source.type_signature)

    # Run extensive random testing
    for _ in 0..<config.iterations
      input = generator.generate()

      source_output = Interpreter.execute(source, input)
      target_output = Interpreter.execute(target, input)

      if source_output != target_output
        # Shrink to minimal counterexample
        minimal = shrink(input, |i| {
          Interpreter.execute(source, i) != Interpreter.execute(target, i)
        })
        return TestResult.Counterexample(minimal)
      end
    end

    TestResult.PassedWithConfidence(
      iterations: config.iterations,
      coverage: generator.coverage_estimate
    )
  end

  # Metamorphic testing relations
  fn metamorphic_test(
    source: IR,
    target: IR,
    relations: List[MetamorphicRelation]
  ) -> TestResult

    for relation in relations
      for _ in 0..<1000
        input = relation.generate_input()
        transformed = relation.transform(input)

        # Both versions should satisfy the relation
        source_satisfies = relation.check(
          Interpreter.execute(source, input),
          Interpreter.execute(source, transformed)
        )
        target_satisfies = relation.check(
          Interpreter.execute(target, input),
          Interpreter.execute(target, transformed)
        )

        if source_satisfies != target_satisfies
          return TestResult.MetamorphicViolation(relation, input)
        end
      end
    end

    TestResult.Passed
  end
end
```

---

## 3. Test Generation Techniques

### 3.1 Compiler Fuzzing (Csmith-Style)

[Csmith](https://github.com/csmith-project/csmith) pioneered random program generation for compiler testing:

**Statistics**:
- Found **325+ previously unknown bugs**
- Every tested compiler crashed or generated wrong code
- [YARPGen](https://dl.acm.org/doi/10.1145/3428264) extension found **220+ bugs** in GCC/LLVM

**Adaptation for Aria LLM Verification**:

```aria
module AriaFuzzer
  # Generate random Aria programs for verification testing
  fn generate_test_program(config: FuzzConfig) -> Program
    context = GenerationContext.new(
      max_depth: config.depth,
      available_types: config.types,
      probability_table: config.probabilities
    )

    # Generate well-formed program avoiding UB
    program = ProgramBuilder.new(context)
      .add_function(generate_function(context))
      .add_assertions(generate_contracts(context))
      .build()

    # Ensure program terminates
    if not TerminationChecker.will_terminate(program)
      return generate_test_program(config)  # Retry
    end

    program
  end

  fn differential_test(program: Program) -> TestResult
    # Compile with and without LLM optimization
    baseline = Compiler.compile(program, llm_opt: false)
    optimized = Compiler.compile(program, llm_opt: true)

    # Execute both versions
    for input in InputGenerator.for_program(program).take(1000)
      baseline_result = baseline.execute(input)
      optimized_result = optimized.execute(input)

      if baseline_result != optimized_result
        return TestResult.Discrepancy(program, input, baseline_result, optimized_result)
      end
    end

    TestResult.Consistent
  end
end
```

### 3.2 Metamorphic Testing

From [FSE 2025 research](https://arxiv.org/abs/2504.04321), metamorphic testing for compiler optimization:

**Key Relations**:

| Relation | Description | Example |
|----------|-------------|---------|
| Algebraic equivalence | `x + 0 = x`, `x * 1 = x` | Simplification |
| Control-flow equivalence | Loop transformation | Unrolling |
| Dead code preservation | Removing unreachable code | DCE |
| Commutative operations | `a + b = b + a` | Reordering |

```aria
module MetamorphicRelations
  # Relation 1: Dead code elimination should not affect output
  fn dead_code_relation(program: Program) -> MetamorphicRelation
    MetamorphicRelation {
      name: "dead_code_elimination",
      transform: |p| {
        # Insert dead code
        dead_branch = generate_unreachable_code()
        p.insert_after_return(dead_branch)
      },
      check: |original_out, transformed_out| {
        original_out == transformed_out
      }
    }
  end

  # Relation 2: Loop unrolling preservation
  fn loop_unroll_relation(iterations: Int) -> MetamorphicRelation
    MetamorphicRelation {
      name: "loop_unroll",
      transform: |p| {
        p.transform_loops(|loop| {
          loop.unroll(factor: iterations)
        })
      },
      check: |original_out, transformed_out| {
        original_out == transformed_out
      }
    }
  end

  # Relation 3: Commutative operation reordering
  fn commutative_relation -> MetamorphicRelation
    MetamorphicRelation {
      name: "commutative_reorder",
      transform: |p| {
        p.transform_expressions(|expr| {
          match expr
            Add(a, b) -> Add(b, a)
            Mul(a, b) -> Mul(b, a)
            _ -> expr
          end
        })
      },
      check: |original_out, transformed_out| {
        original_out == transformed_out
      }
    }
  end
end
```

### 3.3 Equivalence Modulo Inputs (EMI)

[EMI technique](https://dl.acm.org/doi/10.1145/2594291.2594334) generates equivalent programs:

```aria
module EMIGenerator
  fn generate_emi_variants(program: Program, input: Input) -> List[Program]
    # Execute program to find which branches are taken
    trace = Tracer.execute(program, input)

    variants = []

    # For each untaken branch, modify its code
    for branch in trace.untaken_branches
      # Variant 1: Remove untaken code
      variants.push(program.remove_branch(branch))

      # Variant 2: Replace with different (dead) code
      variants.push(program.replace_branch(branch, generate_dead_code()))

      # Variant 3: Add assertions that would fail if branch taken
      variants.push(program.add_assertion_in_branch(branch, False))
    end

    variants
  end

  fn emi_test(program: Program) -> TestResult
    for input in InputGenerator.for_program(program).take(100)
      variants = generate_emi_variants(program, input)

      original_output = Compiler.compile(program).execute(input)

      for variant in variants
        variant_output = Compiler.compile(variant).execute(input)

        if original_output != variant_output
          return TestResult.EMIViolation(program, variant, input)
        end
      end
    end

    TestResult.Passed
  end
end
```

---

## 4. Determinism Guarantees

### 4.1 Root Causes of LLM Non-Determinism

From [SGLang research](https://lmsys.org/blog/2025-09-22-sglang-deterministic/):

| Cause | Impact | Mitigation |
|-------|--------|------------|
| Floating-point non-associativity | Different batch sizes change results | Batch-invariant kernels |
| CUDA kernel non-determinism | Different runs yield different outputs | Fixed reduction order |
| Load balancing | Request depends on parallel requests | Isolation |
| Model updates | Provider changes behavior | Version pinning |

### 4.2 SGLang Deterministic Inference Techniques

**Batch-Invariant Kernels**:
```
Standard: (a + b) + c  -- depends on batch grouping
TBIK:     tree-structured reduction with fixed order
```

**Implementation Components**:
- **RMSNorm**: Batch-invariant normalization
- **Attention**: Fixed split-KV sizes
- **MatMul**: Tree-based invariant kernels (TBIK)
- **Sampling**: Seeded Gumbel noise for `multinomial`

**Performance** (from SGLang):
| Configuration | Overhead | Throughput |
|--------------|----------|------------|
| Baseline (non-deterministic) | 0% | 1.0x |
| Batch-invariant kernels | 34.35% | ~0.75x |
| With CUDA graphs | 34.35% | 2.8x |

### 4.3 Aria Determinism Strategy

**Three-Tier Approach**:

```aria
enum DeterminismTier
  # Tier 1: Cached optimizations (100% deterministic)
  Cached
  # Tier 2: Self-hosted with batch-invariant inference
  Deterministic
  # Tier 3: Cloud API with best-effort reproducibility
  BestEffort
end

module DeterministicCompilation
  fn optimize(ir: IR, config: CompilerConfig) -> OptimizedIR
    cache_key = compute_cache_key(ir, config)

    # Tier 1: Check cache first
    if cached = OptimizationCache.get(cache_key)
      log_decision("cache_hit", cache_key)
      return cached.optimized_ir
    end

    # Tier 2: Self-hosted deterministic inference
    if config.determinism >= DeterminismTier.Deterministic
      suggestion = LocalLLM.optimize(ir, {
        batch_invariant: true,
        seed: config.seed,
        temperature: 0
      })
    else
      # Tier 3: Cloud API
      suggestion = CloudLLM.optimize(ir, {
        seed: config.seed,
        temperature: 0
      })
    end

    # Verify suggestion
    result = Verifier.verify(ir, suggestion)

    match result
      :verified ->
        OptimizationCache.store(cache_key, suggestion)
        suggestion
      _ ->
        ir  # Return original on failure
    end
  end
end
```

### 4.4 Build Reproducibility Protocol

```yaml
# aria-lock.yaml - Reproducible build manifest
version: "1.0"
llm-optimization:
  model: "aria-opt-v1.2.3"
  model-hash: "sha256:abc123..."
  inference-engine: "sglang-0.5.0"
  determinism-tier: "cached"

cached-optimizations:
  - key: "sha256:def456..."
    source-hash: "sha256:789..."
    optimization-hash: "sha256:012..."
    verified: true
    timestamp: "2026-01-15T10:30:00Z"

verification:
  smt-solver: "z3-4.13.0"
  timeout: 30s
  loop-bound: 8
```

---

## 5. Security Considerations

### 5.1 Threat Model

From [OWASP LLM Top 10 2025](https://genai.owasp.org/llmrisk/llm032025-supply-chain/):

| Threat | Severity | Description |
|--------|----------|-------------|
| Malicious model weights | Critical | Backdoored models produce malicious code |
| Trigger-based backdoors | Critical | Model normal except for specific triggers |
| Prompt injection | High | Code comments manipulate optimization |
| Model hallucination | High | Semantically incorrect suggestions |
| Supply chain poisoning | High | Compromised training data/dependencies |
| Data exfiltration | Medium | Model leaks sensitive code patterns |

### 5.2 Security Architecture

```
+------------------------------------------------------------------+
|                    Security Boundary                              |
+------------------------------------------------------------------+
|                                                                    |
|  +------------------------+    +---------------------------+       |
|  | Input Sanitization     |    | Model Provenance          |       |
|  | - Strip comments       |    | - Signed model weights    |       |
|  | - Normalize whitespace |    | - Hash verification       |       |
|  | - Limit code size      |    | - Trusted source only     |       |
|  +------------------------+    +---------------------------+       |
|            |                              |                        |
|            v                              v                        |
|  +----------------------------------------------------------+     |
|  |                  Sandboxed LLM Execution                   |     |
|  |  - Isolated process                                        |     |
|  |  - No network access                                       |     |
|  |  - Memory limits                                           |     |
|  |  - Timeout enforcement                                     |     |
|  |  - Output size limits                                      |     |
|  +----------------------------------------------------------+     |
|            |                                                       |
|            v                                                       |
|  +----------------------------------------------------------+     |
|  |                  Output Validation                         |     |
|  |  - Syntactic check                                         |     |
|  |  - Type check                                              |     |
|  |  - Size bounds                                             |     |
|  |  - No new capabilities                                     |     |
|  +----------------------------------------------------------+     |
|            |                                                       |
|            v                                                       |
|  +----------------------------------------------------------+     |
|  |                  Formal Verification                       |     |
|  |  - SMT equivalence                                         |     |
|  |  - Property testing                                        |     |
|  |  - Contract satisfaction                                   |     |
|  +----------------------------------------------------------+     |
|            |                                                       |
|            v                                                       |
|        ACCEPT (only if all checks pass)                           |
+------------------------------------------------------------------+
```

### 5.3 Defense-in-Depth Implementation

```aria
module SecurityLayer
  struct SecurityConfig {
    max_input_size: Int = 100_000,       # bytes
    max_output_size: Int = 200_000,      # bytes
    max_size_ratio: Float = 2.0,         # output/input
    execution_timeout: Duration = 60.s,
    memory_limit: Int = 4.GB,
    allowed_model_sources: List[String],
    require_model_signature: Bool = true
  }

  fn secure_optimize(
    ir: IR,
    config: SecurityConfig
  ) -> Result[OptimizedIR, SecurityError]

    # Check 1: Input size
    if ir.size > config.max_input_size
      return Err(InputTooLarge(ir.size))
    end

    # Check 2: Model provenance
    if config.require_model_signature
      if not ModelRegistry.verify_signature(config.model)
        return Err(UntrustedModel(config.model))
      end
    end

    # Check 3: Sandboxed execution
    suggestion = Sandbox.execute(
      timeout: config.execution_timeout,
      memory: config.memory_limit,
      network: false
    ) {
      LLM.optimize(ir)
    }?

    # Check 4: Output validation
    if suggestion.size > config.max_output_size
      return Err(OutputTooLarge(suggestion.size))
    end

    if suggestion.size > ir.size * config.max_size_ratio
      return Err(SuspiciousGrowth(ir.size, suggestion.size))
    end

    # Check 5: No new capabilities
    if not (suggestion.external_calls <= ir.external_calls)
      return Err(NewCapabilities(
        suggestion.external_calls - ir.external_calls
      ))
    end

    # Check 6: Formal verification
    verification = Verifier.verify(ir, suggestion)
    match verification
      :verified -> Ok(suggestion)
      :counterexample(ce) -> Err(SemanticMismatch(ce))
      :timeout -> Err(VerificationTimeout)
    end
  end
end
```

### 5.4 Audit Trail

```aria
module AuditLog
  struct OptimizationAudit {
    timestamp: Instant,
    session_id: UUID,
    source_hash: Hash,
    model_version: String,
    model_hash: Hash,
    suggestion_hash: Hash,
    verification_result: VerificationResult,
    security_checks: Map[String, Bool],
    accepted: Bool,
    reason: Option[String]
  }

  fn log_optimization(audit: OptimizationAudit)
    # Append to append-only log
    AuditStorage.append(audit)

    # Alert on suspicious patterns
    if audit.security_checks.values.any(|v| not v)
      SecurityAlerts.notify(audit)
    end
  end
end
```

---

## 6. Integration with Aria Compiler Pipeline

### 6.1 Pipeline Integration Points

```
+------------------------------------------------------------------+
|                    Aria Compiler Pipeline                          |
+------------------------------------------------------------------+
|                                                                    |
|  Source Code (.aria)                                               |
|       |                                                            |
|       v                                                            |
|  [Lexer + Parser] -----> AST                                       |
|       |                                                            |
|       v                                                            |
|  [Type Checker] -----> Typed AST                                   |
|       |                                                            |
|       v                                                            |
|  [IR Generator] -----> Aria IR                                     |
|       |                                                            |
|       +---> [Traditional Optimization Passes]                      |
|       |              |                                             |
|       |              v                                             |
|       +---> [LLM Optimization] <-- INTEGRATION POINT               |
|       |              |                                             |
|       |              v                                             |
|       |     [Verification Framework]                               |
|       |              |                                             |
|       +<-------------+                                             |
|       |                                                            |
|       v                                                            |
|  [Backend Selection]                                               |
|       |                                                            |
|       +---> [LLVM Backend] -----> Native Binary                    |
|       |                                                            |
|       +---> [WASM Backend] -----> WebAssembly                      |
|       |                                                            |
|       +---> [Interpreter] -----> Direct Execution                  |
+------------------------------------------------------------------+
```

### 6.2 Optimization Annotation System

```aria
# User-facing optimization controls
@optimize(
  level: :aggressive,      # none | conservative | moderate | aggressive
  verify: :formal,         # none | testing | formal
  timeout: 30.seconds,     # verification timeout
  fallback: :original      # original | traditional | error
)
fn compute_intensive_function(data: Array[Float64]) -> Float64
  # Function body...
end

# Disable LLM optimization for security-critical code
@optimize(level: :none)
@security_critical
fn validate_signature(message: Bytes, signature: Bytes) -> Bool
  # Cryptographic verification...
end

# Allow aggressive optimization with explicit trust
@optimize(level: :aggressive, verify: :testing)
@trusted_optimization
fn numerical_computation(matrix: Matrix[Float64]) -> Matrix[Float64]
  # Hot path that benefits from optimization...
end
```

### 6.3 Compiler Configuration

```toml
# aria.toml - Project configuration
[compiler.llm-optimization]
enabled = true
default-level = "moderate"
default-verify = "formal"

[compiler.llm-optimization.model]
source = "local"  # local | cloud
path = "~/.aria/models/aria-opt-v1"
version = "1.2.3"
hash = "sha256:abc123..."

[compiler.llm-optimization.verification]
smt-timeout = "30s"
test-iterations = 10000
loop-bound = 8

[compiler.llm-optimization.cache]
enabled = true
path = "~/.aria/opt-cache"
remote = "https://aria-cache.example.com"

[compiler.llm-optimization.security]
require-model-signature = true
max-output-ratio = 2.0
sandbox-memory = "4GB"
```

---

## 7. Recommendations for Aria

### 7.1 Implementation Phases

| Phase | Focus | Timeline | Risk |
|-------|-------|----------|------|
| 1 | SMT verification for simple optimizations | Q2 2026 | Low |
| 2 | Property-based testing fallback | Q2 2026 | Low |
| 3 | Caching and determinism layer | Q3 2026 | Medium |
| 4 | Security hardening | Q3 2026 | Medium |
| 5 | Full pipeline integration | Q4 2026 | Medium |

### 7.2 Success Criteria

- [ ] SMT verification completes in <30s for 95% of functions
- [ ] Zero false positives in verification
- [ ] 100% deterministic builds with cached optimizations
- [ ] Security audit passed for threat model
- [ ] 5-20% performance improvement on benchmark suite

### 7.3 Key Metrics to Track

| Metric | Target | Measurement |
|--------|--------|-------------|
| Verification success rate | >90% | SMT checks passing |
| Average verification time | <5s | Time to verify |
| Cache hit rate | >80% | Repeated compilations |
| Optimization acceptance rate | >70% | LLM suggestions verified |
| Performance improvement | 5-20% | Benchmark comparison |

---

## 8. Key Resources

### Academic Papers
1. [Alive2: Bounded Translation Validation for LLVM](https://users.cs.utah.edu/~regehr/alive2-pldi21.pdf) - Lopes et al., PLDI 2021
2. [CompCert: Formal Verification of a Realistic Compiler](https://xavierleroy.org/publi/compcert-CACM.pdf) - Leroy, CACM 2009
3. [Compiler Optimization Testing Based on Optimization-Guided Equivalence Transformations](https://arxiv.org/abs/2504.04321) - FSE 2025
4. [VecTrans: LLM Transformation Framework](https://arxiv.org/html/2503.19449v1) - 2025

### Tools
- [Alive2](https://github.com/AliveToolkit/alive2) - LLVM translation validator
- [Z3 Theorem Prover](https://github.com/Z3Prover/z3) - SMT solver
- [Csmith](https://github.com/csmith-project/csmith) - Random program generator
- [SGLang](https://lmsys.org/blog/2025-09-22-sglang-deterministic/) - Deterministic LLM inference

### Security
- [OWASP LLM Top 10 2025](https://genai.owasp.org/llmrisk/llm032025-supply-chain/) - Supply chain risks
- [LLM Security Best Practices](https://www.oligo.security/academy/llm-security-in-2025-risks-examples-and-best-practices)

---

## 9. Open Questions

1. **Verification Scope**: How do we handle inter-procedural optimizations that Alive2 doesn't support?
2. **Effect Verification**: How do we verify optimizations preserve Aria's effect system?
3. **Contract Integration**: Can we use Aria contracts to strengthen verification?
4. **Incremental Verification**: Can we verify incrementally as the LLM constructs the optimization?
5. **User Experience**: How do we present verification failures meaningfully to developers?
6. **Performance Trade-offs**: What's the acceptable compilation time overhead?

---

## 10. Conclusion

This framework provides a comprehensive approach to verifying LLM-suggested optimizations for the Aria compiler. Key insights:

1. **Multi-layer verification** combining SMT solving, property testing, and metamorphic testing provides strong correctness guarantees
2. **Deterministic inference** is achievable with batch-invariant kernels and aggressive caching
3. **Security requires defense-in-depth** with sandboxing, provenance verification, and formal equivalence checking
4. **The verification layer makes LLM optimization safe** - untrusted suggestions are treated as potentially adversarial input that must pass formal verification

The framework positions Aria to be the first production compiler with verified LLM-assisted optimization, differentiating it significantly in the programming language landscape.
