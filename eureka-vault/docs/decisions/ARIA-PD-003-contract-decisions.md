# ARIA-PD-003: Contract System Product Decisions

**Decision ID**: ARIA-PD-003
**Status**: Approved
**Date**: 2026-01-15
**Author**: ARBITER (Product Decision Agent)
**Input**: ARIA-M04-04 Tiered Contract System Research (SENTINEL)

---

## Decision Summary

Aria adopts a **three-tier contract verification system** with a strict **zero-cost production default**. The system provides powerful compile-time guarantees for common cases while gracefully degrading to runtime checks only when explicitly requested by developers.

### Core Product Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Default Mode** | `:static` for `--release`, `:full` for development | Zero-cost in production, helpful in development |
| **Tier Classification** | Automatic, transparent | Reduce cognitive burden on developers |
| **Timeout Handling** | Graceful degradation with warnings | Never block compilation |
| **SMT Solver** | Z3 (bundled), with timeout limits | Industry standard, predictable behavior |
| **Quantified Contracts** | Always Tier 3 (runtime) | Theoretically undecidable |

---

## Tier Strategy

### Default Tier Classification

| Contract Pattern | Default Tier | Verification Method |
|------------------|--------------|---------------------|
| Null checks (`x != nil`) | Tier 1 | SMT - compile time |
| Linear arithmetic (`x + y < z`) | Tier 1 | SMT - compile time |
| Type guards (`x is T`) | Tier 1 | Type system - compile time |
| Boolean combinations | Tier 1 | SMT - compile time |
| Pure method calls (`arr.sorted?`) | Tier 2 | Abstract interpretation + cache |
| Immutable field access | Tier 2 | Dataflow analysis |
| Collection predicates (`list.all?`) | Tier 2 | Abstract interpretation |
| Universal quantifiers (`forall`) | Tier 3 | Runtime only |
| Existential quantifiers (`exists`) | Tier 3 | Runtime only |
| Opaque closures | Tier 3 | Runtime only |
| Non-linear arithmetic (`x * y > z * w`) | Tier 3 | Runtime only |
| IO/Effect-dependent contracts | Tier 3 | Runtime only |

### Tier Promotion Rules

Contracts may be **automatically promoted** to a higher tier (more static verification) when:

1. **Tier 3 -> Tier 2**: When a quantified contract has bounded domain known at compile time
2. **Tier 2 -> Tier 1**: When abstract interpretation can fully resolve pure expressions

Developers **cannot manually demote** a tier (e.g., force a Tier 1 contract to runtime). This prevents accidental loss of compile-time guarantees.

---

## User Control

### Contract Modes

Aria provides four contract checking modes, configurable at function, module, or project level:

```aria
# Function-level annotation
@contracts(:static)    # Tier 1 only, zero runtime cost
@contracts(:full)      # Tier 1 static, Tier 2/3 runtime (development default)
@contracts(:runtime)   # All contracts checked at runtime
@contracts(:off)       # No contract checking (use sparingly)
```

### Configuration Hierarchy

```
1. Function annotation     @contracts(:static)     <- Highest priority
2. Module annotation       @contracts(:full)
3. Project config          aria.toml
4. Build flag              aria build --contracts=static
5. Default                 :full (dev) / :static (release)  <- Lowest priority
```

### Project Configuration (aria.toml)

```toml
[contracts]
# Default mode for development builds
development_mode = "full"

# Default mode for release builds (--release flag)
production_mode = "static"

# SMT solver timeout in milliseconds
smt_timeout = 5000

# Maximum memory for contract verification
max_verification_memory = "64mb"

# Enable/disable contract warnings
show_tier_warnings = true

# Cache verification results across builds
persistent_cache = true
```

### Command-Line Overrides

```bash
# Development (default: :full)
aria build

# Production (default: :static)
aria build --release

# Force specific mode
aria build --contracts=runtime
aria build --contracts=off

# Testing (always full verification)
aria test  # Implicitly uses :full mode
```

---

## Performance Budget

### Compile-Time Limits

| Resource | Limit | Behavior on Exceed |
|----------|-------|--------------------|
| **SMT timeout per contract** | 5 seconds (configurable) | Degrade to Tier 3 + warning |
| **Total SMT time per file** | 30 seconds | Emit batch warning, continue |
| **Verification memory** | 64 MB | Fail verification, suggest simplification |
| **Cache size** | 256 MB per project | LRU eviction |

### Runtime Overhead Targets

| Mode | Tier 1 | Tier 2 | Tier 3 |
|------|--------|--------|--------|
| `:static` | 0% | 0% | 0% |
| `:full` | 0% | 0%* | varies |
| `:runtime` | <1% | varies | varies |
| `:off` | 0% | 0% | 0% |

*Tier 2 in `:full` mode may have negligible overhead for cache lookups.

### Incremental Verification

Only re-verify contracts when:
1. Function signature changes
2. Contract expression changes
3. Called function's contract changes
4. Type definitions used in contract change

Cache invalidation is fine-grained to minimize recomputation.

---

## Failure Modes

### SMT Timeout Handling

When SMT verification times out:

```
1. Emit warning (not error)
2. Degrade contract to Tier 3
3. Insert runtime check (if mode allows)
4. Continue compilation
5. Suggest simplification in warning message
```

**Warning Example:**
```
warning[W0410]: Contract verification timed out
  --> src/algorithm.aria:42:3
   |
42 |   requires complex_invariant(data)
   |            ^^^^^^^^^^^^^^^^^^^^^^^ SMT timeout after 5000ms
   |
   = Contract downgraded to runtime check
   = Suggestion: Break contract into simpler sub-conditions
   = Tip: Increase timeout with @contracts(:full, smt_timeout: 10000)
```

### Verification Memory Exceeded

```
error[E0411]: Contract verification memory limit exceeded
  --> src/large_data.aria:15:3
   |
15 |   requires data.all? |x| x.valid? end
   |            ^^^^^^^^^^^^^^^^^^^^^^^^^ Requires >64MB for verification
   |
   = Use @contracts(:runtime) for this function
   = Or simplify contract to reduce verification complexity
```

### Unsatisfiable Contract

```
error[E0401]: Contract is unsatisfiable
  --> src/impossible.aria:8:3
   |
 8 |   requires x > 0
 9 |   requires x < 0
   |            ^^^^^ Conflicts with constraint at line 8
   |
   = These conditions cannot both be true
   = Remove or correct one of the conflicting requirements
```

---

## Trade-offs Accepted

### Static vs Runtime Boundary

**Decision**: Aria accepts that some contracts cannot be statically verified.

| We Accept | We Do Not Accept |
|-----------|------------------|
| Quantified contracts are runtime-only | Blocking compilation on solver timeout |
| Non-linear arithmetic needs runtime checks | Requiring manual tier annotations |
| Opaque closures cannot be verified | Silent degradation (always warn) |
| IO-dependent contracts are dynamic | Runtime overhead in production by default |

### Verification Completeness vs Usability

**Decision**: Aria prioritizes usability over verification completeness.

| Usability Choice | Verification Trade-off |
|------------------|------------------------|
| Automatic tier classification | Less control for power users |
| Graceful timeout degradation | May miss provable contracts |
| Bounded analysis | Cannot verify unbounded properties |
| Incremental verification | May miss whole-program optimizations |

### Developer Experience vs Safety Guarantees

**Decision**: Aria prioritizes developer experience while maintaining safety for common cases.

| DX Choice | Safety Trade-off |
|-----------|------------------|
| `:off` mode available | Developers can disable all checks |
| Function-level overrides | Inconsistent checking possible |
| Configurable timeouts | Long timeouts slow builds |
| No mandatory annotations | Some contracts unclassifiable |

---

## Success Metrics

### Coverage Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Tier 1 coverage** | >80% of all contracts | Contracts proven statically |
| **Tier 2 coverage** | >10% of remaining | Contracts resolved with caching |
| **Tier 3 fallback** | <10% of all contracts | Contracts requiring runtime |
| **Timeout rate** | <1% of verification attempts | SMT timeouts per build |

### Developer Adoption Metrics

| Metric | Target | Notes |
|--------|--------|-------|
| **Contract usage** | >50% of public functions | In projects using contracts |
| **Mode overrides** | <5% of functions | Most use defaults |
| **`:off` usage** | <1% of functions | Strongly discouraged |
| **User complaints** | <10% about contract perf | Build time satisfaction |

### Performance Targets

| Metric | Target |
|--------|--------|
| **Verification overhead** | <20% of total compile time |
| **Cache hit rate** | >90% on incremental builds |
| **Runtime check overhead** | <5% in `:full` mode typical code |
| **Zero overhead guarantee** | 100% in `:static` mode |

---

## Implementation Phases

### Phase 1: Foundation (Milestone 4)
- Implement Tier 1 classification
- Basic SMT integration (Z3)
- `:static` and `:off` modes
- Contract syntax parsing

### Phase 2: Caching (Milestone 5)
- Implement Tier 2 classification
- Abstract interpretation engine
- Verification result caching
- `:full` mode support

### Phase 3: Runtime (Milestone 6)
- Implement Tier 3 runtime checks
- `:runtime` mode support
- Property-based testing integration
- Quantifier support

### Phase 4: Polish (Milestone 7-8)
- IDE integration (tier display)
- Enhanced error messages
- Performance optimization
- LLM-assisted suggestions (future)

---

## Risks and Mitigations

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| SMT solver too slow | Medium | High | Aggressive timeouts, caching, incremental |
| Developers confused by tiers | Low | Medium | Automatic classification, good errors |
| Too many runtime checks | Medium | Medium | Encourage Tier 1 patterns in docs |
| Cache invalidation bugs | Low | High | Conservative invalidation, testing |
| Z3 dependency issues | Low | High | Bundle Z3, provide fallback mode |

---

## Open Questions (Deferred)

1. **Cross-file contract verification**: Should contracts reference other modules?
2. **Generic contract templates**: Reusable contract patterns?
3. **Contract inheritance**: How do subtype contracts interact?
4. **Async contract timing**: When are contracts checked in async code?
5. **Contract documentation generation**: Auto-generate docs from contracts?

These questions are deferred to Phase 4 or later milestones.

---

## Appendix: Mode Quick Reference

```aria
# RECOMMENDED: Let Aria choose (default behavior)
fn example(x: Int)
  requires x > 0  # Tier 1, verified at compile time
end

# PRODUCTION: Maximum performance, static only
@contracts(:static)
fn hot_path(data: Data)
  requires data.size > 0  # Verified statically or warning
end

# DEVELOPMENT: Maximum safety
@contracts(:full)
fn debug_function(items: Array[Item])
  requires items.all? |i| i.valid? end  # Runtime check
end

# TESTING: Force all checks
@contracts(:runtime)
fn test_helper(input: Any)
  requires complex_invariant(input)  # Always runtime
end

# ESCAPE HATCH: No checks (use sparingly!)
@contracts(:off)
fn trusted_internal(raw: Pointer)
  # You're on your own here
end
```

---

## References

- ARIA-M04-04: Tiered Contract System Design (SENTINEL Research)
- ARIA-M04-01: Eiffel's Design by Contract Study
- ARIA-M04-02: Dafny's Verification Analysis
- Dafny Language Reference
- Z3 SMT Solver Documentation
- SPARK 2014 User's Guide
