# Milestone M09: C Interop System

## Overview

Design Aria's C interoperability system that enables direct C header import without binding generation, similar to Zig's @cImport.

## Research Questions

1. Can we parse C headers at compile time like Zig?
2. How do we handle C's type system differences safely?
3. What's the memory safety model for C interop?
4. How do we handle platform-specific C code?

## Core Innovation Target

```ruby
# Direct C header import - no binding generation
extern C from "sqlite3.h"
extern C from "openssl/ssl.h" as ssl

fn use_sqlite
  db = C.sqlite3_open("test.db")
  defer C.sqlite3_close(db)

  # Type-checked, memory-safe wrapper generated automatically
  C.sqlite3_exec(db, "SELECT * FROM users")
end
```

## Competitive Analysis Required

| Language | C Interop | Study Focus |
|----------|-----------|-------------|
| Zig | @cImport | Direct parsing |
| Rust | bindgen | Generated bindings |
| Nim | importc | Pragma-based |
| D | extern(C) | Direct declaration |
| Cython | cdef | Python bridge |

## Tasks

### ARIA-M09-01: Deep dive into Zig's @cImport
- **Description**: Analyze Zig's compile-time C parsing
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, ffi, zig, c-interop
- **Deliverables**:
  - Parsing implementation
  - Type mapping rules
  - Macro handling

### ARIA-M09-02: Study libclang for C parsing
- **Description**: Evaluate libclang for header parsing
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, ffi, libclang, tooling
- **Deliverables**:
  - libclang capabilities
  - Performance characteristics
  - Integration patterns

### ARIA-M09-03: Research C type mapping strategies
- **Description**: Study C to safe language type mappings
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, ffi, types, safety
- **Deliverables**:
  - Pointer safety wrappers
  - Struct layout compatibility
  - Union handling

### ARIA-M09-04: Design C header import system
- **Description**: Design Aria's C import mechanism
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Blocked by**: ARIA-M09-01, ARIA-M09-02
- **Tags**: research, ffi, design
- **Deliverables**:
  - Import syntax specification
  - Type conversion rules
  - Safety boundary design

### ARIA-M09-05: Design safe C wrapper generation
- **Description**: Design automatic safe wrapper generation
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Blocked by**: ARIA-M09-03
- **Tags**: research, ffi, safety, design
- **Deliverables**:
  - Wrapper generation rules
  - Memory management strategy
  - Error handling patterns

### ARIA-M09-06: Prototype C import
- **Description**: Build C header import prototype
- **Dependency**: AGENT_CAPABLE
- **Priority**: normal
- **Blocked by**: ARIA-M09-04
- **Tags**: prototype, ffi, implementation
- **Deliverables**:
  - Working C header parser
  - Type conversion demo
  - Basic safety wrappers

## Implementation Progress

### FFI Type System (COMPLETED - Jan 2026)
- [x] `crates/aria-ffi/` crate with comprehensive FFI support
- [x] C type mappings: CInt, CUInt, CShort, CLong, CChar, CFloat, CDouble
- [x] Size types: CSize, CSSize, CPtrDiff, CIntPtr, CUIntPtr
- [x] Pointer types: CPtr<T>, CPtrConst<T>, CPtrRestrict<T>, CVoidPtr
- [x] Array types: CArray<T, N>, CArrayView<'a, T>, CArrayMutView<'a, T>
- [x] String type: AriaString with owned/borrowed semantics
- [x] Function pointers: CFn<Args, Ret>
- [x] CCompatible trait for FFI-safe types
- [x] 25 unit tests passing

### Ownership Annotations (COMPLETED - Jan 2026)
- [x] `Owned<T>` - Aria owns returned memory, must free
- [x] `Borrowed<'a, T>` - Foreign code owns, Aria must not free
- [x] `Transfer<T>` - Ownership moves to foreign code
- [x] `BorrowedFrom<'owner, T>` - Lifetime tied to owner
- [x] SafetyLevel enum: Safe, Unsafe, Raw
- [x] NoPanic marker for FFI-safe functions

### Extern Declaration Parsing (COMPLETED - Jan 2026)
- [x] `extern C from "header.h" ... end` syntax parsing
- [x] `extern Python from "module" ... end` syntax parsing
- [x] `extern Wasm import/export ... end` syntax parsing
- [x] Extern function declarations with C types
- [x] Extern struct declarations
- [x] Extern const declarations
- [x] Extern type aliases
- [x] C type parsing: int, uint, long, float, double, char, void, size_t, ssize_t
- [x] Pointer types: `*char`, `*const int`, etc.

### Remaining Work
- [ ] C header parsing (libclang integration or custom parser)
- [ ] Automatic type conversion from C headers
- [ ] Safe wrapper generation for C functions
- [ ] Platform-specific C code handling (#ifdef)
- [ ] Type checking for extern declarations
- [ ] Code generation for extern function calls

## Success Criteria

- [x] C type mapping rules documented (in aria-ffi)
- [x] Safety model defined (ownership annotations)
- [ ] C header import designed
- [ ] Prototype demonstrates feasibility

## Key Resources

1. Zig compiler source (stage2/c_import.zig)
2. libclang documentation
3. "Mixed-Language Programming" papers
4. Rust bindgen source code
5. FFI safety research

## Timeline

Target: Q2 2026

## Related Milestones

- **Depends on**: M01 (Type System), M02 (Ownership)
- **Enables**: M10 (Python Interop)
- **Parallel**: M07/M08 (Backends)
