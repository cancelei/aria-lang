# ARIA-PD-004: Developer Experience Product Decisions

**Decision ID**: ARIA-PD-004
**Status**: Approved
**Date**: 2026-01-15
**Authors**: CRAFTER (Product Decision Agent)
**Research Inputs**: ARIA-M18-02 (HERALD), ARIA-M18-03 (SAGE)

---

## Decision Summary

This document captures the final product decisions for Aria's developer experience (DX), synthesizing research from error message design (HERALD) and incremental compilation architecture (SAGE). These decisions prioritize **rapid feedback loops** and **educational error messages** as the two pillars of Aria's DX strategy.

### Core DX Principles (Ranked)

1. **Sub-100ms keystroke response** - IDE responsiveness is non-negotiable
2. **Errors that teach** - Every error is an opportunity to educate
3. **Fix suggestions that work** - MachineApplicable fixes are prioritized
4. **Fast iteration cycle** - Save to feedback in under 500ms

---

## 1. Error Philosophy

### Decision: Elm-Style with Aria Adaptations

**Chosen Approach**: First-person plural voice ("We expected...") combined with Rust's structured diagnostic architecture.

| Aspect | Elm | Rust | Aria Decision |
|--------|-----|------|---------------|
| Voice | "I see..." (first-person) | Impersonal ("expected X, found Y") | **"We expected..."** (team metaphor) |
| Structure | Prose-heavy | Structured with labels | **Structured + conversational help** |
| Suggestions | Contextual hints | Applicability levels | **Applicability levels (Rust model)** |
| Docs | Inline explanations | Error codes + index | **Error codes + inline help + docs URL** |

### Rationale

- First-person plural creates collaborative feel ("the compiler is on your team")
- Structured diagnostics enable IDE integration (JSON output, code actions)
- Applicability levels enable safe auto-fixing via `aria fix` command
- Documentation URLs enable deep-dive learning without cluttering terminal

### Error Message Template

```
error[E0001]: type mismatch
 --> src/main.aria:42:21
  |
42 |   let name: String = get_id()
  |       ----            ^^^^^^^^ expected `String`, found `Int`
  |       |
  |       expected due to this type annotation
  |
  help: Convert the integer to a string
  |
42 |   let name: String = get_id().to_s
  |                              +++++
  |
  docs: https://aria-lang.org/errors/E0001
```

### Trade-off: Verbosity vs Clarity

**Accepted**: Aria will produce longer error messages than minimal compilers. This is intentional. A 10-line helpful error is better than a 1-line cryptic error.

---

## 2. LSP Responsiveness Targets

### Decision: Tiered Latency Targets

Based on SAGE research and human perception thresholds:

| Operation | Target Latency | Rationale |
|-----------|----------------|-----------|
| **Keystroke echo** | <16ms | 60fps for smooth typing |
| **Syntax highlighting** | <30ms | Instant visual feedback |
| **Inline errors (squiggles)** | <50ms | Near-instant feedback on typos |
| **Go-to-definition** | <100ms | Perceived as "instant" |
| **Hover information** | <100ms | Same as go-to-definition |
| **Completions popup** | <200ms | Allows brief computation |
| **Full file diagnostics** | <500ms | After typing pause |
| **Workspace diagnostics** | <5s | Background task, can be slower |

### Implementation Strategy

1. **Prioritize hot paths**: Keystroke -> parse -> inline errors must be optimized
2. **Background analysis**: Full diagnostics run after 300ms typing pause
3. **Cancellation support**: All operations cancellable on new keystroke
4. **Durability optimization**: Standard library = HIGH durability (rarely revalidated)

### Trade-off: Accuracy vs Speed

**Accepted**: Inline errors may briefly show stale data during rapid typing. Full accuracy restored within 500ms of typing pause. Users prefer responsive-but-briefly-stale over accurate-but-laggy.

---

## 3. Incremental Compilation Strategy

### Decision: Query-Based Architecture (Salsa Model)

**Core Invariant**: "Editing a function body never invalidates global derived data."

### What Invalidates

| Change | Invalidates |
|--------|-------------|
| Edit function body | `parse(file)`, `infer_body(func)` only |
| Edit function signature | `fn_signature(func)`, `infer_body(callers)` |
| Add/remove function | `file_item_tree(file)`, `crate_def_map()` |
| Add/remove import | `imports(file)`, dependent resolutions |
| Edit type definition | `fn_signature(users)`, `infer_body(users)` |

### What Preserves

| Stable Under | Reason |
|--------------|--------|
| Body edits in other files | No cross-function body dependencies |
| Whitespace/comment changes | ItemTree ignores formatting |
| Standard library | HIGH durability, never revalidated |
| Dependency crates | MEDIUM durability, rare revalidation |

### Durability Classification

```rust
// Aria durability assignment
Durability::HIGH   // Standard library, dependencies
Durability::MEDIUM // Project config, Aria.toml
Durability::LOW    // User source files
```

### Trade-off: Memory vs Speed

**Accepted**: Salsa caches results aggressively. Memory usage will be higher than a non-incremental compiler. For a 100k LOC project, expect ~200-500MB cache. This is acceptable for IDE use.

---

## 4. IDE Feature Priority

### MVP (Milestone 18-19)

| Feature | Priority | Depends On |
|---------|----------|------------|
| Syntax highlighting | P0 | Lexer |
| Inline error diagnostics | P0 | Parser + TypeChecker |
| Go-to-definition | P0 | Name resolution |
| Hover type info | P0 | Type inference |
| Basic completions | P1 | Scope analysis |
| Document symbols | P1 | ItemTree |
| Find references | P1 | Name resolution |

### Post-MVP (Milestone 20+)

| Feature | Priority | Depends On |
|---------|----------|------------|
| Rename symbol | P2 | Find references |
| Code actions (quick fixes) | P2 | Suggestions with applicability |
| Signature help | P2 | Function signatures |
| Inlay hints | P2 | Type inference |
| Semantic highlighting | P3 | Full semantic analysis |
| Call hierarchy | P3 | Cross-reference analysis |
| Type hierarchy | P3 | Trait/interface analysis |

### Future (Post-1.0)

| Feature | Priority | Depends On |
|---------|----------|------------|
| Contract visualization | P4 | Contract system |
| Effect annotations | P4 | Effect system |
| Ownership flow hints | P4 | Ownership inference |
| AI-assisted completions | P5 | LLM integration |

### Trade-off: Feature Breadth vs Depth

**Accepted**: MVP focuses on 7 core features done excellently rather than 20 features done poorly. Each MVP feature must meet latency targets before shipping.

---

## 5. Suggestion Applicability Standards

### Decision: Four-Tier Applicability Model (Rust-Derived)

| Level | Aria Usage | Auto-Apply | Example |
|-------|------------|------------|---------|
| **MachineApplicable** | Safe auto-fix | Yes (`aria fix`) | `Int` to `String`: `.to_s` |
| **HasPlaceholders** | Template with user input | No | Type annotation: `: <type>` |
| **MaybeIncorrect** | Possible fix, needs review | No | Typo suggestions |
| **Unspecified** | Informational only | No | General hints |

### MachineApplicable Requirements

A suggestion is MachineApplicable only if:
1. The fix is provably correct (type-checked)
2. The fix does not change program semantics unexpectedly
3. The fix is complete (no placeholders)
4. The fix compiles successfully after application

### Prioritized MachineApplicable Fixes

| Error Type | Fix | Applicability |
|------------|-----|---------------|
| Type mismatch: Int -> String | `.to_s` | MachineApplicable |
| Unused variable | Prefix with `_` | MachineApplicable |
| Missing semicolon | Insert `;` | MachineApplicable |
| Non-exhaustive match | Add `_ => ...` | HasPlaceholders |
| Undefined variable (typo) | Rename | MaybeIncorrect |

### Trade-off: Safety vs Convenience

**Accepted**: Aria will be conservative with MachineApplicable. A false positive auto-fix that breaks code is worse than requiring manual intervention. When in doubt, downgrade to MaybeIncorrect.

---

## 6. Error Code System

### Decision: Category-Prefixed Numeric Codes

| Prefix | Category | Range |
|--------|----------|-------|
| E0xxx | Type Errors | E0001-E0999 |
| E1xxx | Ownership Errors | E1001-E1999 |
| E2xxx | Contract Errors | E2001-E2999 |
| E3xxx | Effect Errors | E3001-E3999 |
| E4xxx | Pattern Errors | E4001-E4999 |
| W0xxx | Warnings | W0001-W0999 |
| S0xxx | Suggestions | S0001-S0999 |

### Error Code Stability Policy

- **Major version**: Codes are stable within a major version
- **Minor version**: New codes may be added, existing codes preserved
- **Deprecation**: Codes retired after 2 major versions with redirect

### Trade-off: Code Stability vs Evolution

**Accepted**: Error codes may change between major versions. Tooling should use structured JSON output, not parse error codes for automation.

---

## 7. Trade-offs Accepted

### 7.1 Compile Speed vs Error Quality

| Scenario | Choice | Rationale |
|----------|--------|-----------|
| Initial compile | Quality first | Users wait anyway |
| Incremental (body edit) | Speed first (<100ms) | Hot path |
| Incremental (signature) | Balance | Moderate impact |
| Full rebuild | Quality first | Infrequent operation |

**Decision**: Error quality is never sacrificed, but error computation is staged. Fast path shows basic errors; full diagnostics arrive within 500ms.

### 7.2 Memory Usage vs Responsiveness

**Decision**: IDE mode prioritizes responsiveness over memory. Accept 200-500MB cache for instant feedback. Batch compile mode uses less memory.

### 7.3 Feature Completeness vs Time-to-Market

**Decision**: Ship MVP with 7 excellent features rather than 20 mediocre features. Each feature must meet latency targets.

### 7.4 Error Verbosity vs Terminal Clutter

**Decision**: Accept longer error messages. Provide `--compact-errors` flag for experienced users who want minimal output.

---

## 8. Success Metrics

### Quantitative Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| Keystroke latency P95 | <100ms | LSP telemetry |
| Go-to-definition P95 | <150ms | LSP telemetry |
| Completion P95 | <300ms | LSP telemetry |
| Error accuracy | >99% | Test suite |
| MachineApplicable success rate | 100% | Applied fixes must compile |

### Qualitative Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| Error message clarity | "Helpful" rating >80% | User surveys |
| Learning from errors | "I understood the fix" >90% | User surveys |
| IDE satisfaction | NPS >50 | Quarterly survey |
| "Compiler as teacher" perception | >70% agreement | User interviews |

### Tracking Cadence

- **Latency metrics**: Continuous (opt-in telemetry)
- **Error accuracy**: Every release (test suite)
- **User surveys**: Quarterly
- **NPS**: Quarterly

---

## 9. Implementation Priorities

### Immediate (Milestone 18)

1. Implement `TypeDiagnostic` struct with full metadata
2. Wire up error rendering with colors and source context
3. Add JSON output for IDE integration
4. Implement 5 MachineApplicable suggestions

### Near-term (Milestone 19)

1. Integrate salsa for query-based compilation
2. Implement per-function type inference isolation
3. Add cancellation support to all queries
4. Meet <100ms keystroke latency target

### Medium-term (Milestone 20)

1. Full LSP feature set (MVP complete)
2. `aria fix` command for auto-applying suggestions
3. Error documentation website launch
4. Telemetry opt-in for latency tracking

---

## 10. Open Questions for Future Decision

1. **Localization**: Should error messages support multiple languages? (Defer to post-1.0)
2. **AI suggestions**: Should Aria suggest fixes using LLM? (Research in ARIA-LLM track)
3. **Contract errors**: How verbose should contract violation errors be? (Depends on contract system design)
4. **Effect errors**: How to explain effect mismatches to users? (Depends on effect system design)

---

## Appendix A: Research References

- **ARIA-M18-02**: Error Message Design Research (HERALD)
- **ARIA-M18-03**: Incremental Compilation Architecture (SAGE)
- [Elm Compiler Errors for Humans](https://elm-lang.org/news/compiler-errors-for-humans)
- [Rust Diagnostic Structs Guide](https://rustc-dev-guide.rust-lang.org/diagnostics/diagnostic-structs.html)
- [rust-analyzer Architecture](https://rust-analyzer.github.io/book/contributing/architecture.html)

---

## Appendix B: Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-01-15 | First-person plural voice | Collaborative feel without being cutesy |
| 2026-01-15 | <100ms keystroke target | Human perception threshold |
| 2026-01-15 | Salsa-based incrementality | Proven at scale in rust-analyzer |
| 2026-01-15 | 4-tier applicability | Enables safe auto-fix |
| 2026-01-15 | 7-feature MVP | Depth over breadth |

---

*This document represents binding product decisions for Aria's developer experience. Changes require Product Decision review.*
