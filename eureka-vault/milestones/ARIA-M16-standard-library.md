# Milestone M16: Standard Library

## Overview

Design Aria's standard library with core types, collections, I/O, and essential utilities while maintaining the language's simplicity.

## Research Questions

1. What should be in std vs external packages?
2. How do we design APIs that work for both native and WASM?
3. What's the right abstraction level for I/O?
4. How do we handle platform-specific functionality?

## Core Standard Library Modules

```ruby
std::
├── prelude       # Auto-imported basics
├── collections   # Array, Map, Set, Deque
├── string        # String utilities
├── io            # Files, streams
├── net           # Networking
├── fs            # File system
├── time          # Date, time, duration
├── math          # Math functions
├── random        # Random generation
├── json          # JSON parsing
├── regex         # Regular expressions
├── process       # Process management
├── env           # Environment
├── fmt           # Formatting
├── iter          # Iterator utilities
├── sync          # Synchronization primitives
└── testing       # Test utilities
```

## Competitive Analysis Required

| Language | Stdlib Approach | Study Focus |
|----------|-----------------|-------------|
| Rust | Small, batteries | Extension pattern |
| Go | Batteries included | Completeness |
| Python | Very large | Discoverability |
| Zig | Minimal | Build vs runtime |
| Nim | Pragmatic | Balance |

## Tasks

### ARIA-M16-01: Survey stdlib approaches
- **Description**: Compare standard library philosophies
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, stdlib, comparison
- **Deliverables**:
  - Philosophy comparison
  - Size vs utility trade-offs
  - Maintenance considerations

### ARIA-M16-02: Design prelude module
- **Description**: Design auto-imported prelude
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, stdlib, prelude, design
- **Deliverables**:
  - Prelude contents
  - Import rules
  - Overriding mechanism

### ARIA-M16-03: Design collections module
- **Description**: Design core collection types
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, stdlib, collections, design
- **Deliverables**:
  - Collection trait hierarchy
  - Implementation choices
  - Performance characteristics

### ARIA-M16-04: Design I/O abstraction
- **Description**: Design cross-platform I/O
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, stdlib, io, design
- **Deliverables**:
  - Stream abstraction
  - File API
  - WASM compatibility

### ARIA-M16-05: Design string module
- **Description**: Design string handling
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, stdlib, string, design
- **Deliverables**:
  - UTF-8 handling
  - String interpolation
  - Formatting API

### ARIA-M16-06: Design networking module
- **Description**: Design network abstractions
- **Dependency**: AGENT_CAPABLE
- **Priority**: normal
- **Tags**: research, stdlib, net, design
- **Deliverables**:
  - TCP/UDP API
  - HTTP client
  - WASM/fetch integration

## Implementation Progress

### Implemented Modules (Jan 2026)
- [x] `std::prelude` - Core type aliases and re-exports
- [x] `std::io` - Basic I/O (print, println, File struct)
- [x] `std::string` - String manipulation methods
- [x] `std::collections` - List, Map, Set with common methods
- [x] `std::math` - Mathematical constants and functions
- [x] `std::option` - Option<T> with monadic operations
- [x] `std::result` - Result<T, E> with error handling
- [x] `std::iter` - Iterator trait and implementations
- [x] `std::time` - Duration, Instant, SystemTime, DateTime
- [x] `std::random` - Rng struct with random number generation
- [x] `std::fmt` - Display/Debug traits, formatting utilities, StringBuilder
- [x] `std::testing` - TestCase, TestSuite, assertions
- [x] `std::env` - Environment variables, platform info
- [x] `std::fs` - File system operations, Path utilities

### Remaining Modules
- [ ] `std::net` - Networking (TCP/UDP, HTTP client)
- [ ] `std::json` - JSON parsing/serialization
- [ ] `std::regex` - Regular expressions
- [ ] `std::process` - Process spawning and management
- [ ] `std::sync` - Synchronization primitives

### Infrastructure
- [x] Stdlib crate with embedded .aria sources
- [x] Module loader with parser integration
- [x] 14 modules available (8 original + 6 new)
- [x] 7 unit tests passing

## Success Criteria

- [x] Stdlib scope defined
- [x] Core module APIs designed
- [ ] Cross-platform strategy clear
- [ ] Performance targets set

## Key Resources

1. Rust std documentation
2. Go standard library
3. Python standard library
4. "API Design for C++" - Reddy
5. "The Design of Everyday Things" - Norman (UX principles)

## Timeline

Target: Q2-Q3 2026

## Related Milestones

- **Depends on**: M01 (Types), M02 (Ownership)
- **Enables**: Practical application development
- **Parallel**: M15 (Modules)
