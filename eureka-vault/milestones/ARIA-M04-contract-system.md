# Milestone M04: Contract System

## Overview

Design Aria's built-in Design by Contract system that makes testing first-class: preconditions, postconditions, invariants, and property-based testing integrated into the language syntax.

## Research Questions

1. How do we verify contracts at compile-time vs runtime?
2. Can contracts drive test generation automatically?
3. How do we make contracts as natural as type annotations?
4. What verification depth is practical for a production language?

## Core Innovation Target

```ruby
fn binary_search(arr, target) -> Int?
  requires arr.sorted?                    # Compile-time when possible
  requires arr.length > 0
  ensures result.nil? or arr[result] == target
  ensures forall i: result.some? implies
          not exists j: arr[j] == target and j < result

  # Implementation...

  examples
    binary_search([1,2,3,4,5], 3) == Some(2)
    binary_search([1,2,3], 4) == None
  end

  property "finds all elements"
    forall arr: Array<Int>, x: Int
      arr.sorted? and arr.contains?(x) implies
        binary_search(arr, x).some?
  end
end
```

## Competitive Analysis Required

| Language | Approach | Study Focus |
|----------|----------|-------------|
| Eiffel | Design by Contract | Original DBC implementation |
| Dafny | Verification | Full program verification |
| Ada/SPARK | Contracts + proof | Industrial verification |
| Racket | Contracts | Dynamic contracts |
| D | Contracts | Pragmatic DBC |

## Tasks

### ARIA-M04-01: Study Eiffel's Design by Contract
- **Description**: Deep dive into original DBC implementation
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, contracts, eiffel, foundational
- **Deliverables**:
  - Contract semantics analysis
  - Inheritance and contracts
  - Runtime vs static checking

### ARIA-M04-02: Analyze Dafny's verification
- **Description**: Study Dafny's automated verification approach
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, contracts, verification, dafny
- **Deliverables**:
  - Z3 integration patterns
  - Annotation burden analysis
  - Verification limits

### ARIA-M04-03: Research property-based testing
- **Description**: Study QuickCheck-style property testing
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, testing, properties
- **Deliverables**:
  - Shrinking strategies
  - Generator design
  - Integration with contracts

### ARIA-M04-04: Design contract syntax and semantics
- **Description**: Design Aria's contract system
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Blocked by**: ARIA-M04-01, ARIA-M04-02
- **Tags**: research, contracts, design
- **Deliverables**:
  - Contract syntax specification
  - Static vs dynamic checking strategy
  - Test extraction rules

### ARIA-M04-05: Design property testing integration
- **Description**: Integrate property-based testing into contracts
- **Dependency**: AGENT_CAPABLE
- **Priority**: normal
- **Blocked by**: ARIA-M04-03, ARIA-M04-04
- **Tags**: research, testing, properties, design
- **Deliverables**:
  - Property syntax specification
  - Generator inference
  - Shrinking integration

### ARIA-M04-06: Prototype contract checker
- **Description**: Build contract verification prototype
- **Dependency**: AGENT_CAPABLE
- **Priority**: normal
- **Blocked by**: ARIA-M04-04
- **Tags**: prototype, contracts, implementation
- **Deliverables**:
  - Static contract verification (simple cases)
  - Runtime contract insertion
  - Test extraction demo

## Implementation Progress

### Contract Parsing (COMPLETED - Jan 2026)
- [x] `requires`, `ensures`, `invariant` keywords in lexer
- [x] Contract clause parsing in aria-parser
- [x] `forall`, `exists`, `old` expression keywords
- [x] AST Contract types (Contract, ContractClause)
- [x] Functions store contracts in AST

### Contract Verification Infrastructure (COMPLETED - Jan 2026)
- [x] `crates/aria-contracts/` crate with tiered verification
- [x] ContractTier: Tier1Static, Tier2Cached, Tier3Dynamic
- [x] ContractMode: Static, Full, Runtime, Off
- [x] Expression pattern classification (12 patterns)
- [x] ContractVerifier with classify() and verify_static()
- [x] 9 unit tests passing

### Type Checker Integration (COMPLETED - Jan 2026)
- [x] Contract verification in TypeChecker::check_function()
- [x] Preconditions (requires) type-checked as Bool
- [x] Postconditions (ensures) type-checked with `result` in scope
- [x] Invariants type-checked as Bool
- [x] Contract tier classification during type checking

### Remaining Work
- [ ] Generate runtime assertion code for Tier 2/3 contracts
- [ ] `examples` block syntax for test extraction
- [ ] `property` block syntax for property-based testing
- [ ] SMT solver integration (Z3) for Tier 1 static verification
- [ ] Contract violation error messages at runtime

## Success Criteria

- [x] Contract syntax integrated into parser (requires/ensures/invariant)
- [x] Static verification infrastructure (tier classification)
- [ ] Runtime checking with good error messages
- [ ] Automatic test extraction from examples
- [ ] Property-based testing integrated

## Key Papers/Resources

1. "Object-Oriented Software Construction" - Meyer (Eiffel)
2. "QuickCheck: A Lightweight Tool for Random Testing" - Claessen & Hughes
3. "Dafny: An Automatic Program Verifier" - Leino
4. "Contracts for Higher-Order Functions" - Findler & Felleisen
5. "Hypothesis: Property-Based Testing for Python" - MacIver

## Timeline

Target: Q1-Q2 2026

## Related Milestones

- **Depends on**: M01 (Type System)
- **Enables**: M12 (Property Testing)
- **Parallel**: M03 (Effects) - contracts can specify effect requirements
