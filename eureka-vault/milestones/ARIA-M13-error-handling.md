# Milestone M13: Error Handling

## Overview

Design Aria's error handling system with Result types, automatic propagation via `?`, and integration with the effect system.

## Research Questions

1. How do we make error handling ergonomic without exceptions?
2. What's the relationship between effects and errors?
3. How do we handle recoverable vs unrecoverable errors?
4. Can we provide context automatically (backtraces, locations)?

## Core Innovation Target

```ruby
# Result types with propagation
fn read_config(path) -> Result<Config>
  content = File.read(path)?           # Propagates error
  json = JSON.parse(content)?          # Propagates error
  Config.from_json(json)?
end

# Pattern matching for handling
match read_config("app.json")
  Ok(config)          => start(config)
  Err(FileNotFound)   => use_defaults()
  Err(ParseError(e))  => log_and_exit(e)
  Err(e)              => raise e        # Re-raise unexpected
end

# Panic for unrecoverable
fn index<T>(arr: Array<T>, i: Int) -> T
  requires 0 <= i < arr.length          # Panic if violated
  arr.get_unchecked(i)
end
```

## Competitive Analysis Required

| Language | Approach | Study Focus |
|----------|----------|-------------|
| Rust | Result + ? | Ergonomic propagation |
| Go | Multiple return | Explicit checking |
| Swift | throws + try | Keyword-based |
| Elm | No exceptions | Pure functional |
| Zig | Error unions | Comptime checks |

## Tasks

### ARIA-M13-01: Compare error handling approaches
- **Description**: Comprehensive comparison of approaches
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, errors, comparison
- **Deliverables**:
  - Ergonomics comparison
  - Safety guarantees
  - Performance impact

### ARIA-M13-02: Study Rust's error handling
- **Description**: Deep dive into Rust's Result patterns
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, errors, rust
- **Deliverables**:
  - `?` operator mechanics
  - Error trait design
  - Backtrace capture

### ARIA-M13-03: Research error context
- **Description**: Study automatic error context approaches
- **Dependency**: AGENT_CAPABLE
- **Priority**: normal
- **Tags**: research, errors, context
- **Deliverables**:
  - Location tracking
  - Error chaining
  - Backtrace design

### ARIA-M13-04: Study effect-based errors
- **Description**: Research errors as effects
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, errors, effects
- **Deliverables**:
  - Error effect semantics
  - Handler patterns
  - Integration with other effects

### ARIA-M13-05: Design error handling system
- **Description**: Design Aria's error handling
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Blocked by**: ARIA-M13-01, ARIA-M13-02
- **Tags**: research, errors, design
- **Deliverables**:
  - Result type design
  - Propagation syntax
  - Panic semantics

### ARIA-M13-06: Design error messages
- **Description**: Design user-facing error messages
- **Dependency**: AGENT_CAPABLE
- **Priority**: normal
- **Blocked by**: ARIA-M13-03
- **Tags**: research, errors, ux, design
- **Deliverables**:
  - Error message format
  - Location display
  - Suggestion system

## Implementation Progress

### Phase 1: Core Result Type (COMPLETED - Jan 2026)
- [x] `Result<T, E>` type in aria-types with Ok/Err variants
- [x] `?` operator (Try) parsing in aria-parser
- [x] `?` operator type checking in aria-types
- [x] Standard library `std::result` module with combinators:
  - is_ok, is_err, unwrap, unwrap_err
  - unwrap_or, unwrap_or_else
  - map, map_err, and_then, or_else
  - ok, err, flatten, expect, expect_err
- [x] Exception effect in aria-effects with raise operation

### Phase 2: Error Propagation (IN PROGRESS)
- [x] Basic `?` propagation for Result and Optional
- [ ] Function return type validation for `?` usage
- [ ] Effect tracking for Exception effect

### Phase 3: Error Context (PENDING)
- [ ] Location tracking in errors
- [ ] Error chaining (cause tracking)
- [ ] Backtrace capture

## Success Criteria

- [x] Error handling syntax finalized
- [x] Result type integrated with type system
- [ ] Result type integrated with effect system
- [ ] Automatic context capture designed
- [x] Error messages helpful and clear

## Key Resources

1. "Error Handling in Rust" - Rust book
2. Elm error handling guide
3. "Exceptional Syntax" - OCaml paper
4. Swift error handling guide
5. Zig error union documentation

## Timeline

Target: Q2 2026

## Related Milestones

- **Depends on**: M01 (Types), M03 (Effects)
- **Enables**: Robust application development
- **Parallel**: M11 (Concurrency) - error propagation in async
