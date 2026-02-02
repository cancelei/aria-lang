# Milestone M14: Pattern Matching

## Overview

Design Aria's pattern matching system with exhaustiveness checking, optimization, and Ruby-like ergonomics.

## Research Questions

1. How do we optimize pattern matching for performance?
2. What exhaustiveness algorithm to use?
3. How do we handle complex nested patterns?
4. Can we support active patterns/views?

## Core Innovation Target

```ruby
# Rich pattern matching
match user
  User(name: "admin", role:)        => admin_panel(role)
  User(name:, age:) if age >= 18    => adult_content(name)
  User(name:, verified: true)       => verified_user(name)
  User(name:, _)                    => guest_view(name)
end

# Array patterns
match items
  []                    => "empty"
  [single]              => "one: #{single}"
  [first, ...rest]      => "first: #{first}, #{rest.length} more"
end

# Or patterns
match status
  Ok(value) | Cached(value) => process(value)
  Err(NotFound | Timeout)   => retry()
  Err(e)                    => fail(e)
end
```

## Competitive Analysis Required

| Language | Pattern Matching | Study Focus |
|----------|------------------|-------------|
| Rust | match + if let | Optimization |
| Scala | case classes | Extractors |
| F# | Active patterns | Views |
| Swift | switch | Exhaustiveness |
| Elixir | Pattern matching | Simplicity |

## Tasks

### ARIA-M14-01: Study Rust pattern compilation
- **Description**: Analyze Rust's pattern compilation
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, patterns, rust, optimization
- **Deliverables**:
  - Decision tree generation
  - Exhaustiveness checking
  - Performance analysis

### ARIA-M14-02: Research exhaustiveness algorithms
- **Description**: Study exhaustiveness checking algorithms
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, patterns, exhaustiveness, academic
- **Deliverables**:
  - Algorithm comparison
  - Error message generation
  - Performance characteristics

### ARIA-M14-03: Study active patterns
- **Description**: Research F#'s active patterns
- **Dependency**: AGENT_CAPABLE
- **Priority**: normal
- **Tags**: research, patterns, active, fsharp
- **Deliverables**:
  - Active pattern mechanics
  - Use cases
  - Implementation cost

### ARIA-M14-04: Design pattern syntax
- **Description**: Design Aria's pattern syntax
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Blocked by**: ARIA-M14-01
- **Tags**: research, patterns, syntax, design
- **Deliverables**:
  - Pattern syntax specification
  - Destructuring rules
  - Guard syntax

### ARIA-M14-05: Design exhaustiveness checker
- **Description**: Design exhaustiveness checking system
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Blocked by**: ARIA-M14-02
- **Tags**: research, patterns, exhaustiveness, design
- **Deliverables**:
  - Checking algorithm
  - Error messages
  - Suggestion generation

### ARIA-M14-06: Design pattern optimization
- **Description**: Design pattern compilation optimization
- **Dependency**: AGENT_CAPABLE
- **Priority**: normal
- **Blocked by**: ARIA-M14-01
- **Tags**: research, patterns, optimization, design
- **Deliverables**:
  - Decision tree compilation
  - Jump table generation
  - Performance targets

## Implementation Progress

### Exhaustiveness Checking (COMPLETED - Jan 2026)
- [x] `crates/aria-patterns/` crate with exhaustiveness algorithm
- [x] Maranget-style pattern matrix representation
- [x] Constructor types: Bool, Int, Float, String, Unit, Tuple, Array, Variant, Struct
- [x] ConstructorSet for type-based constructor enumeration
- [x] Usefulness predicate for pattern matching
- [x] Witness generation for missing pattern counterexamples
- [x] Redundant arm detection
- [x] 18 unit tests passing

### Pattern Lowering (COMPLETED - Jan 2026)
- [x] `crates/aria-mir/src/lower_pattern.rs` with comprehensive pattern lowering
- [x] Wildcard, identifier, literal patterns
- [x] Tuple, array, struct destructuring
- [x] Enum variant patterns with field extraction
- [x] Range patterns
- [x] Or patterns
- [x] Guard patterns with runtime checks
- [x] @ binding patterns
- [x] Type-annotated patterns

### Pattern AST (COMPLETED - Previous)
- [x] PatternKind enum with all pattern types
- [x] FieldPattern for struct destructuring
- [x] Rest patterns for arrays

### Remaining Work
- [ ] Integrate exhaustiveness checker with type checker
- [ ] Error messages for non-exhaustive matches
- [ ] Decision tree optimization for performance
- [ ] Active patterns / views (optional advanced feature)

## Success Criteria

- [x] Pattern syntax finalized
- [x] Exhaustiveness checking working
- [ ] Optimization strategy defined
- [ ] Error messages helpful

## Key Resources

1. "Compiling Pattern Matching" - Maranget
2. Rust MIR pattern compilation
3. "Warnings for Pattern Matching" - Maranget
4. F# active patterns documentation
5. Scala extractors documentation

## Timeline

Target: Q1-Q2 2026

## Related Milestones

- **Depends on**: M01 (Types)
- **Enables**: M13 (Error Handling patterns)
- **Parallel**: M04 (Contracts) - pattern contracts
