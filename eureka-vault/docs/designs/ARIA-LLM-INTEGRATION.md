# Aria LLM Integration Points

**Version**: 1.0
**Date**: 2026-01-26
**Status**: Design Complete
**Enables**: M05 (LLM Optimization)

## Overview

This document defines where and how LLM optimization suggestions integrate with Aria's compilation pipeline. The design ensures safety through verification while enabling AI-assisted optimization.

## Integration Architecture

```
Source → AST → Type Check → MIR → [LLM Hook] → Optimize → Codegen
                                        ↓
                                 ┌─────────────┐
                                 │ LLM Service │
                                 └─────────────┘
                                        ↓
                              ┌─────────────────┐
                              │ Verification    │
                              │ - Equivalence   │
                              │ - Type Safety   │
                              │ - Effect Safety │
                              └─────────────────┘
```

## Hook Points

### 1. Post-Lowering Optimization Hook

**Location**: After AST→MIR lowering, before standard optimizations

**Purpose**: Allow LLM to suggest high-level transformations

**Interface**:
```rust
pub trait LlmOptimizer {
    /// Analyze MIR and suggest optimizations
    fn suggest_optimizations(&self, mir: &MirProgram) -> Vec<OptimizationSuggestion>;

    /// Apply a verified suggestion
    fn apply_suggestion(&self, mir: &mut MirProgram, suggestion: &OptimizationSuggestion) -> Result<()>;
}

pub struct OptimizationSuggestion {
    pub id: SuggestionId,
    pub kind: SuggestionKind,
    pub target: OptimizationTarget,
    pub confidence: f64,
    pub explanation: String,
    pub transformation: Transformation,
}
```

**Suggestion Kinds**:
- `InlineFunction` - Inline a function call
- `LoopUnroll` - Unroll a loop
- `VectorizeLoop` - Vectorize a loop
- `SpecializeGeneric` - Specialize a generic function
- `ReorderOperations` - Reorder independent operations
- `EliminateAllocation` - Stack allocate instead of heap
- `MergeLoops` - Fuse adjacent loops
- `HoistInvariant` - Move loop-invariant code out

### 2. Function-Level Analysis Hook

**Location**: Per-function analysis phase

**Purpose**: Allow LLM to analyze function complexity and suggest improvements

**Interface**:
```rust
pub trait FunctionAnalyzer {
    /// Analyze a function and return insights
    fn analyze_function(&self, func: &MirFunction) -> FunctionAnalysis;
}

pub struct FunctionAnalysis {
    pub complexity_score: u32,
    pub hotspot_blocks: Vec<BlockId>,
    pub optimization_opportunities: Vec<Opportunity>,
    pub potential_bugs: Vec<Warning>,
}
```

### 3. Peephole Optimization Hook

**Location**: During CFG simplification

**Purpose**: Pattern-based local optimizations

**Interface**:
```rust
pub trait PeepholeOptimizer {
    /// Match patterns in a basic block
    fn find_patterns(&self, block: &BasicBlock) -> Vec<PatternMatch>;

    /// Suggest replacement for a pattern
    fn suggest_replacement(&self, pattern: &PatternMatch) -> Option<Replacement>;
}
```

## Verification Requirements

All LLM suggestions MUST be verified before application.

### 1. Semantic Equivalence

The transformed MIR must produce the same observable behavior:

```rust
pub trait EquivalenceChecker {
    /// Verify two MIR programs are semantically equivalent
    fn check_equivalence(&self, original: &MirProgram, transformed: &MirProgram) -> VerificationResult;
}

pub enum VerificationResult {
    Equivalent,
    NotEquivalent { counterexample: Counterexample },
    Unknown { reason: String },
}
```

**Verification Strategies**:
1. **Symbolic Execution**: For small functions
2. **Abstract Interpretation**: For control flow changes
3. **Test-Based**: Run existing tests on transformed code
4. **Proof Checking**: For formally specified transformations

### 2. Type Safety

Transformations must preserve type safety:

```rust
fn verify_type_safety(original: &MirProgram, transformed: &MirProgram) -> bool {
    // All locals must have compatible types
    // All operations must be type-correct
    // All function signatures must match
}
```

### 3. Effect Safety

Effect annotations must be preserved:

```rust
fn verify_effect_safety(original: &MirFunction, transformed: &MirFunction) -> bool {
    // Effect row must be compatible
    // Evidence requirements must be met
    // Handler installations must be preserved
}
```

## Transformation Rules Format

LLM suggestions use a structured transformation format:

```rust
pub enum Transformation {
    /// Replace statements in a block
    ReplaceStatements {
        block: BlockId,
        start: usize,
        end: usize,
        replacement: Vec<Statement>,
    },

    /// Replace a terminator
    ReplaceTerminator {
        block: BlockId,
        replacement: Terminator,
    },

    /// Insert new blocks
    InsertBlocks {
        after: BlockId,
        blocks: Vec<BasicBlock>,
        rewire: Vec<(BlockId, BlockId)>,
    },

    /// Inline a function call
    InlineCall {
        block: BlockId,
        call_index: usize,
        inlined_body: Vec<BasicBlock>,
    },

    /// Specialize a generic
    SpecializeGeneric {
        function: FunctionId,
        type_args: Vec<MirType>,
        specialized_body: MirFunction,
    },
}
```

## Safety Constraints

### 1. Conservative Mode (Default)

Only apply transformations with formal proofs:
- Constant folding
- Dead code elimination
- Proven-equivalent rewrites

### 2. Aggressive Mode (Opt-in)

Allow heuristic transformations with testing:
- Inlining decisions
- Loop transformations
- Allocation optimizations

```toml
# aria.toml
[llm]
mode = "conservative"  # or "aggressive"
verify_all = true
max_suggestions_per_function = 10
```

### 3. Forbidden Transformations

Never allow:
- Removing error handling
- Changing observable side effects
- Modifying unsafe blocks
- Altering concurrency semantics

## LLM Integration Protocol

### Request Format

```json
{
    "version": "1.0",
    "function": {
        "name": "process_data",
        "mir": "<serialized MIR>",
        "context": {
            "call_frequency": "hot",
            "size_bytes": 1024,
            "effect_row": ["IO", "State"]
        }
    },
    "constraints": {
        "max_code_growth": 2.0,
        "preserve_effects": true,
        "target": "x86_64"
    }
}
```

### Response Format

```json
{
    "suggestions": [
        {
            "id": "opt-001",
            "kind": "inline_function",
            "confidence": 0.95,
            "explanation": "Inlining 'helper' reduces call overhead in hot loop",
            "transformation": {
                "type": "InlineCall",
                "block": 3,
                "call_index": 2,
                "expected_speedup": 1.15
            },
            "verification_hint": "pure_function"
        }
    ]
}
```

## Implementation Phases

### Phase 1: Hook Infrastructure
- [ ] Define trait interfaces
- [ ] Add hook points to compilation pipeline
- [ ] Implement no-op default implementations

### Phase 2: Verification Framework
- [ ] Implement type safety checker
- [ ] Implement effect safety checker
- [ ] Add test-based verification

### Phase 3: LLM Service Integration
- [ ] Define serialization format for MIR
- [ ] Implement request/response protocol
- [ ] Add rate limiting and caching

### Phase 4: Production Hardening
- [ ] Add telemetry for suggestions
- [ ] Implement rollback on verification failure
- [ ] Add audit logging

## Security Considerations

1. **LLM Output Sanitization**: Never execute LLM-generated code directly
2. **Rate Limiting**: Limit suggestions per compilation
3. **Audit Trail**: Log all applied transformations
4. **Opt-in Only**: LLM integration disabled by default
5. **Local Mode**: Support offline operation with cached patterns

## Related Documents

- [ARIA-MIR-SPECIFICATION.md](ARIA-MIR-SPECIFICATION.md) - MIR format details
- [M05 LLM Optimization Milestone](../../milestones/ARIA-M05-llm-optimization.md) - Full milestone spec
- [M06 Compiler IR Design](../../milestones/ARIA-M06-compiler-ir-design.md) - IR design decisions
