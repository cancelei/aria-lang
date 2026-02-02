# Milestone M12: Property Testing Integration

## Overview

Design Aria's built-in property-based testing system that works with contracts to automatically generate and shrink test cases.

## Research Questions

1. How do we generate values for custom types automatically?
2. What shrinking strategies work best?
3. How do we integrate with the contract system?
4. Can we use LLM to suggest better properties?

## Core Innovation Target

```ruby
fn sort<T: Ord>(arr: Array<T>) -> Array<T>
  # Properties automatically tested
  property "preserves length"
    forall arr: Array<Int>
      sort(arr).length == arr.length
  end

  property "output is sorted"
    forall arr: Array<Int>
      let result = sort(arr)
      result.windows(2).all? { |(a, b)| a <= b }
  end

  property "output is permutation"
    forall arr: Array<Int>
      sort(arr).sorted == arr.sorted  # Identity check
  end

  # Generator inferred from type
  # Shrinking automatic on failure
end
```

## Competitive Analysis Required

| Tool | Approach | Study Focus |
|------|----------|-------------|
| QuickCheck | Original | Foundation |
| Hypothesis | Python | Stateful testing |
| PropEr | Erlang | Type-driven |
| fast-check | TypeScript | Web testing |
| Hedgehog | Haskell | Integrated shrinking |

## Tasks

### ARIA-M12-01: Study QuickCheck architecture
- **Description**: Deep dive into original QuickCheck
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, testing, quickcheck, foundational
- **Deliverables**:
  - Generator patterns
  - Shrinking algorithms
  - Property composition

### ARIA-M12-02: Analyze Hypothesis approach
- **Description**: Study Hypothesis's innovations
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, testing, hypothesis, python
- **Deliverables**:
  - Database/caching
  - Stateful testing
  - Example saving

### ARIA-M12-03: Research type-driven generation
- **Description**: Study automatic generator derivation
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, testing, generation, types
- **Deliverables**:
  - Derivation rules
  - Custom generator escape hatches
  - Constraint handling

### ARIA-M12-04: Design property syntax
- **Description**: Design Aria's property testing syntax
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Blocked by**: ARIA-M12-01
- **Tags**: research, testing, syntax, design
- **Deliverables**:
  - Property block syntax
  - Quantifier syntax
  - Constraint syntax

### ARIA-M12-05: Design generator inference
- **Description**: Design automatic generator derivation
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Blocked by**: ARIA-M12-03
- **Tags**: research, testing, generators, design
- **Deliverables**:
  - Type to generator mapping
  - Constraint propagation
  - Custom generator API

### ARIA-M12-06: Design shrinking system
- **Description**: Design integrated shrinking
- **Dependency**: AGENT_CAPABLE
- **Priority**: normal
- **Blocked by**: ARIA-M12-01
- **Tags**: research, testing, shrinking, design
- **Deliverables**:
  - Shrinking strategies
  - Minimal counterexample
  - Performance optimization

## Implementation Progress

### Property Testing Core (COMPLETED - Jan 2026)
- [x] `crates/aria-proptest/` crate with QuickCheck-style testing
- [x] GenContext for randomness and size control
- [x] Generator trait and implementations for Int, Bool, String, Array, Tuple, Option
- [x] Arbitrary trait for type-driven generator inference
- [x] 19 unit tests passing

### Shrinking System (COMPLETED - Jan 2026)
- [x] TypedShrinker for automatic shrinking based on type
- [x] IntShrinkIter - shrinks towards 0
- [x] FloatShrinkIter - shrinks towards 0.0
- [x] BoolShrinkIter - shrinks true to false
- [x] StringShrinkIter - removes characters
- [x] ArrayShrinkIter - removes elements
- [x] TupleShrinkIter - shrinks elements
- [x] OptionShrinkIter - shrinks to None
- [x] ResultShrinkIter - shrinks inner values

### Test Runner (COMPLETED - Jan 2026)
- [x] TestRunner with configurable num_tests, max_shrinks, seed
- [x] TestResult: Success, Failure, GaveUp
- [x] Counterexample with original, shrunk, and seed
- [x] Automatic shrinking on failure

### Property Infrastructure (COMPLETED - Jan 2026)
- [x] PropertyResult: Pass, Fail, Discard
- [x] Property trait and FnProperty implementation
- [x] Property combinators (and, or)

### Remaining Work
- [ ] Parse `property` blocks in function definitions
- [ ] Integration with contract system (requires/ensures)
- [ ] LLM-suggested properties (M05 integration)
- [ ] Statistical coverage analysis

## Success Criteria

- [ ] Property syntax integrated in grammar
- [x] Generator inference working
- [x] Shrinking producing minimal examples
- [ ] Integration with contracts
- [x] Test runner integration

## Key Resources

1. "QuickCheck: A Lightweight Tool" - Claessen & Hughes
2. Hypothesis documentation
3. "How to Specify It!" - Lampropoulos & Pierce
4. Hedgehog source code
5. PropEr documentation

## Timeline

Target: Q2 2026

## Related Milestones

- **Depends on**: M01 (Types), M04 (Contracts)
- **Enables**: Reliable testing workflow
- **Synergy**: M05 (LLM) - LLM-suggested properties
