# Milestone M08: WASM Backend

## Overview

Design Aria's WebAssembly backend for browser deployment, edge computing, and portable binaries.

## Research Questions

1. How do we achieve small WASM binary sizes?
2. What's the best approach for JS interop?
3. How do we handle WASM's limitations (no threads initially)?
4. How do we support WASI for server-side WASM?

## Target Capabilities

```ruby
# Same code targets browser and native
@target(:wasm, :native)
module MyApp
  fn main
    if wasm?
      # Browser-specific code
      Document.query("#app").render(App.new)
    else
      # Native CLI
      run_cli()
    end
  end
end
```

## Competitive Analysis Required

| Language | WASM Support | Study Focus |
|----------|--------------|-------------|
| Rust | wasm-pack | Mature tooling |
| Go | TinyGo | Size optimization |
| AssemblyScript | Native | TS-like for WASM |
| Zig | Built-in | Direct compilation |
| Grain | WASM-first | Language design |

## Implementation Progress

### WASM Bytecode Generation (COMPLETED - Jan 2026)
- [x] `crates/aria-codegen/src/wasm_backend.rs` with direct bytecode generation
- [x] MIR to WASM type mapping (i32, i64, f32, f64)
- [x] Function compilation with locals and parameters
- [x] Binary operations (arithmetic, bitwise, comparison)
- [x] Unary operations (negation, not, bitwise not)
- [x] LEB128 encoding for WASM binary format
- [x] Type section, function section, export section, code section
- [x] `compile_to_wasm()` function integrated in lib.rs
- [x] Unit tests for WASM compilation

### CLI Integration (COMPLETED - Jan 2026)
- [x] `aria build <file> --target wasm32` command
- [x] Automatic .wasm file extension
- [x] Usage hints for running WASM modules

### Current Limitations
- [ ] Control flow (if/else, loops) not yet implemented in WASM backend
- [ ] Complex types (strings, arrays, structs) need memory layout
- [ ] No WASI imports for IO operations
- [ ] No JS interop layer

## Tasks

### ARIA-M08-01: Study Rust's wasm-bindgen
- **Description**: Analyze Rust's JS interop approach
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, wasm, rust, interop
- **Deliverables**:
  - wasm-bindgen architecture
  - Type marshalling patterns
  - Performance characteristics

### ARIA-M08-02: Analyze TinyGo's size optimizations
- **Description**: Study TinyGo's WASM size reduction
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, wasm, size, tinygo
- **Deliverables**:
  - Dead code elimination
  - Runtime minimization
  - Feature stripping

### ARIA-M08-03: Research WASM Component Model
- **Description**: Study emerging WASM Component Model
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, wasm, component-model, future
- **Deliverables**:
  - Interface types analysis
  - Resource management
  - Cross-language interop

### ARIA-M08-04: Study WASI patterns
- **Description**: Research WASI for server-side WASM
- **Dependency**: AGENT_CAPABLE
- **Priority**: normal
- **Tags**: research, wasm, wasi, server
- **Deliverables**:
  - WASI capabilities
  - File system access
  - Network patterns

### ARIA-M08-05: Design JS interop layer
- **Description**: Design Aria's JS interop approach
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Blocked by**: ARIA-M08-01
- **Tags**: research, wasm, javascript, design
- **Deliverables**:
  - Type conversion rules
  - DOM access patterns
  - Event handling

### ARIA-M08-06: Design WASM code generation
- **Description**: Design WASM codegen from Aria IR
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Status**: COMPLETED (basic implementation)
- **Blocked by**: M06 (IR Design)
- **Tags**: research, wasm, codegen, design
- **Deliverables**:
  - ~~IR to WASM mapping~~ Implemented in wasm_backend.rs
  - Memory layout - needs work for complex types
  - Function calling conventions - basic support done

### ARIA-M08-07: Add WASM target to CLI
- **Description**: Add `aria build --target wasm32` command
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Status**: COMPLETED (Jan 2026)
- **Tags**: implementation, wasm, cli
- **Deliverables**:
  - [x] --target flag for build command (native, wasm32)
  - [x] .wasm file output
  - [ ] Basic WASI imports for IO (future)

## Success Criteria

- [ ] JS interop designed and documented
- [ ] WASM binary size targets defined
- [ ] WASI support scoped
- [ ] Component Model roadmap defined
- [x] Basic WASM codegen working (unit tests pass)

## Key Resources

1. WebAssembly specification
2. wasm-bindgen documentation
3. WASI documentation
4. Component Model proposal
5. Bytecode Alliance resources

## Timeline

Target: Q2-Q3 2026

## Related Milestones

- **Depends on**: M06 (IR Design)
- **Enables**: Browser deployment
- **Parallel**: M07 (Native Backend)
