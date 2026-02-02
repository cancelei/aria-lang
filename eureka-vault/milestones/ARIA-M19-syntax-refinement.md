# Milestone M19: Syntax Refinement

## Overview

Refine Aria's syntax for maximum ergonomics, consistency, and learnability while maintaining the Ruby/Python feel.

## Research Questions

1. What syntax causes the most confusion for new users?
2. Where do we have unnecessary ceremony?
3. What Ruby/Python idioms should we preserve vs change?
4. How do we balance terseness with readability?

## Syntax Goals

```ruby
# Goal: Ruby simplicity + Rust safety + Python readability

# Clean function definition
fn greet(name) = "Hello, #{name}!"

fn complex_function(items)
  items
    .filter { |x| x.valid? }
    .map { |x| transform(x) }
    .sum
end

# Intuitive data structures
data Point(x: Float, y: Float)

struct User
  name: String
  age: Int
end

# Clear control flow
for item in items
  process(item) if item.ready?
end

# Obvious error handling
result = try_operation()?
```

## Evaluation Criteria

| Aspect | Weight | Evaluation Method |
|--------|--------|-------------------|
| Learnability | 30% | User studies |
| Readability | 25% | Code review studies |
| Writability | 20% | Typing effort metrics |
| Consistency | 15% | Rule count, exceptions |
| Expressiveness | 10% | Feature coverage |

## Tasks

### ARIA-M19-01: User research on syntax preferences
- **Description**: Study syntax preferences across communities
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, syntax, user-research
- **Deliverables**:
  - Ruby community preferences
  - Python community preferences
  - Rust community feedback

### ARIA-M19-02: Analyze syntax pain points
- **Description**: Identify syntax pain points in similar languages
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, syntax, pain-points
- **Deliverables**:
  - Common complaints
  - Confusion sources
  - Learning hurdles

### ARIA-M19-03: Study syntax error recovery
- **Description**: Research parser error recovery
- **Dependency**: AGENT_CAPABLE
- **Priority**: normal
- **Tags**: research, syntax, errors
- **Deliverables**:
  - Error recovery strategies
  - Error message quality
  - Suggestion generation

### ARIA-M19-04: Refine function syntax
- **Description**: Finalize function definition syntax
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Blocked by**: ARIA-M19-01
- **Tags**: research, syntax, functions, design
- **Deliverables**:
  - Short vs long form rules
  - Parameter syntax
  - Return type annotation

### ARIA-M19-05: Refine type annotation syntax
- **Description**: Finalize type annotation syntax
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, syntax, types, design
- **Deliverables**:
  - Annotation positions
  - Inference boundaries
  - Complex type syntax

### ARIA-M19-06: Consistency audit
- **Description**: Audit grammar for consistency
- **Dependency**: AGENT_CAPABLE
- **Priority**: normal
- **Blocked by**: ARIA-M19-04, ARIA-M19-05
- **Tags**: research, syntax, consistency
- **Deliverables**:
  - Rule consistency check
  - Exception documentation
  - Simplification opportunities

## Success Criteria

- [ ] GRAMMAR.md finalized
- [ ] User feedback positive
- [ ] Consistency verified
- [ ] Error messages helpful

## Key Resources

1. "Programming Language Pragmatics" - Scott
2. Python Enhancement Proposals (PEPs)
3. Rust RFCs on syntax
4. "The Design of Everyday Things" - Norman
5. User experience research methods

## Timeline

Target: Throughout development (iterative)

## Related Milestones

- **Depends on**: All feature milestones
- **Enables**: User adoption
- **Iterative**: Refined based on feedback
