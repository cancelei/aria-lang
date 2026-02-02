# AI Agent Instructions: aria-lang/eureka-vault

> Research Hub for the Aria Programming Language - Where Language Design Breakthroughs Are Born

## Purpose

This eureka-vault is the **strategic research hub** for the Aria programming language. It combines:
- **Deep language design research** - type systems, memory models, compiler theory
- **Competitive language analysis** - studying Rust, Zig, Nim, and emerging languages
- **Innovation exploration** - LLM-assisted optimization, contract systems, effect inference
- **Breakthrough discovery** - novel approaches to testability, performance, and developer experience

## Identity

- **Tone**: Research-first, academically rigorous, innovation-focused
- **Output style**: Structured research documents with citations, benchmarks, and prototypes
- **Integration**: Deep integration with WeDo protocol and Memory system
- **Scope**: `aria-lang` for project tasks/memories, `global` for cross-project findings

---

## Research Focus Areas

### 1. Type System Innovation (ARIA-TS-*)
- Dependent types for contracts
- Effect system design
- Type inference algorithms
- Gradual typing approaches
- Research directory: `research/type_systems/`

### 2. Memory Management (ARIA-MM-*)
- Ownership inference (vs explicit like Rust)
- Region-based memory
- Arena allocators
- Compile-time memory analysis
- Research directory: `research/memory_models/`

### 3. Compiler Architecture (ARIA-COMP-*)
- Multi-target compilation (Native, WASM, JS)
- Incremental compilation
- IR design
- Optimization passes
- Research directory: `research/compiler_architecture/`

### 4. LLM Integration (ARIA-LLM-*)
- Verified optimization suggestions
- Equivalence checking
- Deterministic builds with LLM
- Security model for LLM in compilation
- Research directory: `research/llm_integration/`

### 5. Contracts & Testing (ARIA-CT-*)
- Design by Contract implementation
- Property-based testing integration
- Formal verification approaches
- Test extraction from contracts
- Research directory: `research/contracts/`

### 6. FFI & Interoperability (ARIA-FFI-*)
- C header direct import (Zig-style)
- Python interop patterns
- WASM interface types
- Safe FFI design
- Research directory: `research/ffi/`

### 7. Concurrency Models (ARIA-CONC-*)
- Effect-inferred async
- Goroutine-style green threads
- Channel implementations
- Structured concurrency
- Research directory: `research/concurrency/`

---

## Directory Structure

```
aria-lang/eureka-vault/
├── CLAUDE.md                 # This file
├── README.md                 # Project overview
│
├── context/                  # Configuration and triggers
│   └── trigger_definitions.json
│
├── schedules/                # Research schedules
│   ├── daily_research.md
│   ├── weekly_deep_dive.md
│   └── monthly_synthesis.md
│
├── research/                 # Active research outputs
│   ├── daily_status/         # YYYY-MM-DD.md daily reports
│   ├── type_systems/         # Type system research
│   ├── memory_models/        # Memory management research
│   ├── compiler_architecture/# Compiler design research
│   ├── llm_integration/      # LLM optimization research
│   ├── contracts/            # Contract system research
│   ├── ffi/                  # FFI/interop research
│   ├── concurrency/          # Concurrency model research
│   ├── syntax/               # Syntax design research
│   ├── performance/          # Performance research
│   ├── wasm/                 # WebAssembly research
│   ├── tooling/              # Developer tooling research
│   ├── stdlib/               # Standard library research
│   ├── ide/                  # IDE integration research
│   ├── packaging/            # Package management research
│   └── debugging/            # Debug/profile research
│
├── docs/                     # Structured documentation
│   ├── patterns/             # Language design patterns
│   ├── decisions/            # Architecture decisions
│   └── designs/              # Detailed design docs
│
├── breakthroughs/            # Significant discoveries
│   └── [promoted research]
│
├── templates/                # Research templates
│   ├── language-feature-evaluation.json
│   ├── competitive-analysis.json
│   ├── prototype-experiment.json
│   └── breakthrough-synthesis.json
│
└── milestones/               # 20 Research Milestones
    ├── ARIA-M01-type-system-foundations.md
    ├── ARIA-M02-ownership-inference.md
    └── ... (20 total)
```

---

## WeDo Integration

### Task ID Prefixes

| Prefix | Purpose | Example |
|--------|---------|---------|
| ARIA-M01-* | Type System Foundations | ARIA-M01-01 |
| ARIA-M02-* | Ownership Inference | ARIA-M02-01 |
| ARIA-TS-* | Type system research | ARIA-TS-EFFECTS-01 |
| ARIA-MM-* | Memory model research | ARIA-MM-ARENA-01 |
| ARIA-COMP-* | Compiler research | ARIA-COMP-IR-01 |
| ARIA-LLM-* | LLM integration | ARIA-LLM-VERIFY-01 |
| ARIA-CT-* | Contracts/testing | ARIA-CT-PROPERTY-01 |
| ARIA-FFI-* | FFI research | ARIA-FFI-C-01 |
| ARIA-CONC-* | Concurrency research | ARIA-CONC-EFFECT-01 |
| ARIA-PERF-* | Performance research | ARIA-PERF-BENCH-01 |
| ARIA-WASM-* | WASM research | ARIA-WASM-SIMD-01 |
| ARIA-PROTO-* | Prototype experiments | ARIA-PROTO-LEXER-01 |

### Trigger-Based Task Creation

| Trigger | Task Type | Priority | Dependency |
|---------|-----------|----------|------------|
| Novel type system paper | ARIA-TS-* | high | AGENT_CAPABLE |
| Memory safety approach | ARIA-MM-* | high | AGENT_CAPABLE |
| Competitor language feature | ARIA-COMP-* | normal | AGENT_CAPABLE |
| LLM verification method | ARIA-LLM-* | high | AGENT_CAPABLE |
| Contract system innovation | ARIA-CT-* | normal | AGENT_CAPABLE |
| Performance breakthrough | ARIA-PERF-* | high | AGENT_CAPABLE |
| Security concern | ARIA-SEC-* | urgent | USER_REQUIRED |

---

## Research Methodology

### Eureka Discovery Process

```
1. EXPLORE    → Survey existing approaches in the domain
2. ANALYZE    → Deep dive into most promising approaches
3. SYNTHESIZE → Combine insights for Aria's unique approach
4. PROTOTYPE  → Build minimal proof-of-concept
5. VALIDATE   → Test against requirements and benchmarks
6. DOCUMENT   → Promote to breakthroughs/ if significant
```

### Research Depth Levels

| Level | Time | Output |
|-------|------|--------|
| Survey | 1-2 hours | Quick overview, key papers identified |
| Analysis | 4-8 hours | Detailed comparison, pros/cons |
| Deep Dive | 2-3 days | Implementation analysis, prototype |
| Breakthrough | 1+ weeks | Novel contribution, paper-worthy |

### Decision Tree: What to Research Next

```
Is there a blocking question for Aria's design?
├── Yes → Prioritize that research immediately
└── No → Check milestone progress
    ├── Milestone behind → Work on milestone tasks
    └── On track → Explore emerging opportunities
        ├── New paper/approach discovered → ARIA-TS/MM/etc task
        └── Competitor feature → Competitive analysis task
```

---

## Memory Integration

### When to Store Memories

| Finding Type | Memory Type | Scope |
|--------------|-------------|-------|
| Language design decision | decision | aria-lang |
| Discovered pattern from other language | fact | aria-lang |
| Implementation gotcha | gotcha | aria-lang |
| Naming/syntax convention | convention | aria-lang |
| Cross-project insight | fact | global |

### Memory Examples

```python
# Store a design decision
mem_remember(
    content="Aria will use inferred ownership like Swift ARC but with compile-time guarantees like Rust",
    type="decision",
    tags=["memory", "ownership", "design"],
    scope="aria-lang"
)

# Store a competitive insight
mem_remember(
    content="Zig's @cImport directly parses C headers at compile time - no binding generation needed",
    type="fact",
    tags=["ffi", "zig", "competitive"],
    scope="aria-lang"
)

# Store a gotcha
mem_remember(
    content="Dependent types require termination checking to prevent infinite loops in type computation",
    type="gotcha",
    tags=["type-system", "dependent-types"],
    scope="aria-lang"
)
```

---

## Session Startup

```python
# 1. Load context and pending tasks
wedo_continue(scope="aria-lang")

# 2. Recall recent research context
mem_recall(query="aria research recent", scope="aria-lang")

# 3. Check milestone progress
wedo_progress(scope="aria-lang")

# 4. Review any breakthroughs from last session
# Check breakthroughs/ directory for recent additions
```

---

## Competitive Languages to Study

| Language | Key Innovation | Aria Relevance |
|----------|----------------|----------------|
| Rust | Ownership/borrowing | Memory safety model |
| Zig | Comptime, C interop | FFI, compile-time eval |
| Nim | Multi-target, macros | Transpilation strategy |
| Swift | ARC, protocol extensions | Ownership inference |
| Kotlin | Null safety, coroutines | Effect handling |
| Elm | No runtime exceptions | Error handling |
| Idris | Dependent types | Contract system |
| Dafny | Built-in verification | Formal contracts |
| Vale | Region-based memory | Memory innovation |
| Roc | Fast compilation | Developer experience |

---

## Output Integration

Research findings feed into:
1. **WeDo tasks** - actionable implementation items
2. **Memories** - persistent knowledge for future sessions
3. **Breakthroughs** - significant discoveries
4. **GRAMMAR.md** - formal language specification
5. **Implementation** - actual compiler/tooling code
