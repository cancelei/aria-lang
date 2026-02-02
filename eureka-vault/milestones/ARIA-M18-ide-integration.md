# Milestone M18: IDE Integration

## Overview

Design Aria's IDE integration via Language Server Protocol (LSP) for code completion, navigation, refactoring, and debugging.

## Research Questions

1. How do we achieve fast completions with full type inference?
2. What incremental compilation strategy for IDE responsiveness?
3. How do we integrate debugging across platforms?
4. What refactoring operations should we support?

## Core Features Target

```
IDE Features:
├── Code Completion
│   ├── Type-aware completions
│   ├── Import suggestions
│   └── Snippet expansions
├── Navigation
│   ├── Go to definition
│   ├── Find references
│   └── Symbol search
├── Diagnostics
│   ├── Type errors
│   ├── Contract violations
│   └── Suggestions
├── Refactoring
│   ├── Rename
│   ├── Extract function
│   └── Inline
└── Debugging
    ├── Breakpoints
    ├── Variable inspection
    └── Hot reload
```

## Competitive Analysis Required

| Language | LSP | Study Focus |
|----------|-----|-------------|
| Rust | rust-analyzer | Best practices |
| TypeScript | tsserver | Incremental |
| Go | gopls | Simplicity |
| Kotlin | Kotlin LSP | IDE-first |
| Zig | ZLS | Lightweight |

## Tasks

### ARIA-M18-01: Study rust-analyzer architecture
- **Description**: Deep dive into rust-analyzer design
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, ide, rust-analyzer
- **Deliverables**:
  - Incremental architecture
  - Completion engine
  - Performance strategies

### ARIA-M18-02: Research LSP protocol
- **Description**: Study LSP specification thoroughly
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, ide, lsp
- **Deliverables**:
  - Protocol capabilities
  - Extension points
  - Client compatibility

### ARIA-M18-03: Study incremental compilation for IDE
- **Description**: Research incremental compilation strategies
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, ide, incremental
- **Deliverables**:
  - Salsa-style incremental
  - Dependency tracking
  - Cache invalidation

### ARIA-M18-04: Research debugging protocols
- **Description**: Study DAP and debugging approaches
- **Dependency**: AGENT_CAPABLE
- **Priority**: normal
- **Tags**: research, ide, debugging, dap
- **Deliverables**:
  - DAP specification
  - DWARF debug info
  - Source maps (WASM)

### ARIA-M18-05: Design language server architecture
- **Description**: Design Aria's language server
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Blocked by**: ARIA-M18-01, ARIA-M18-03
- **Tags**: research, ide, lsp, design
- **Deliverables**:
  - Server architecture
  - Query system
  - Incremental strategy

### ARIA-M18-06: Design debugging support
- **Description**: Design debugging experience
- **Dependency**: AGENT_CAPABLE
- **Priority**: normal
- **Blocked by**: ARIA-M18-04
- **Tags**: research, ide, debugging, design
- **Deliverables**:
  - Debug info generation
  - DAP integration
  - WASM debugging

## Success Criteria

- [ ] LSP architecture designed
- [ ] Completion strategy defined
- [ ] Incremental compilation designed
- [ ] Debugging approach documented

## Key Resources

1. rust-analyzer architecture docs
2. LSP specification
3. "Responsive Compilers" - matklad
4. DAP specification
5. "Salsa: Incremental Computation"

## Timeline

Target: Q3 2026

## Related Milestones

- **Depends on**: M06 (IR), M15 (Modules)
- **Enables**: Developer experience
- **Parallel**: M17 (Package Manager)
