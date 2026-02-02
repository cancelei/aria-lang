# Milestone M17: Package Manager

## Overview

Design Aria's package manager for dependency management, versioning, and distribution.

## Research Questions

1. What version resolution algorithm to use?
2. How do we handle native dependencies?
3. What's the security model for packages?
4. How do we support multiple targets (native, WASM)?

## Core Features Target

```toml
# aria.toml
[package]
name = "myapp"
version = "1.0.0"
targets = ["native", "wasm"]

[dependencies]
http = "^2.0"
json = "^1.5"
crypto = { version = "^3.0", optional = true }

[dev-dependencies]
testing = "^1.0"

[features]
default = ["crypto"]
minimal = []

[target.wasm.dependencies]
web = "^1.0"
```

```bash
# CLI commands
aria new myapp           # Create project
aria build               # Build project
aria test                # Run tests
aria add http            # Add dependency
aria publish             # Publish package
```

## Competitive Analysis Required

| Tool | Approach | Study Focus |
|------|----------|-------------|
| Cargo | Rust | Best in class |
| npm | JavaScript | Ecosystem scale |
| pip | Python | Simplicity |
| Go modules | Go | Proxy system |
| Zig | Build system | Minimal |

## Tasks

### ARIA-M17-01: Study Cargo deeply
- **Description**: Deep analysis of Cargo's design
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, package-manager, cargo
- **Deliverables**:
  - Resolution algorithm
  - Lock file design
  - Registry protocol

### ARIA-M17-02: Research version resolution
- **Description**: Study SAT-solver based resolution
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, package-manager, resolution
- **Deliverables**:
  - PubGrub analysis
  - Performance characteristics
  - Error message generation

### ARIA-M17-03: Study security models
- **Description**: Research package security approaches
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Tags**: research, package-manager, security
- **Deliverables**:
  - Signing approaches
  - Supply chain security
  - Audit capabilities

### ARIA-M17-04: Design manifest format
- **Description**: Design aria.toml format
- **Dependency**: AGENT_CAPABLE
- **Priority**: high
- **Blocked by**: ARIA-M17-01
- **Tags**: research, package-manager, manifest, design
- **Deliverables**:
  - Manifest specification
  - Feature system
  - Target configuration

### ARIA-M17-05: Design registry protocol
- **Description**: Design package registry API
- **Dependency**: AGENT_CAPABLE
- **Priority**: normal
- **Tags**: research, package-manager, registry, design
- **Deliverables**:
  - API specification
  - Authentication
  - Mirroring support

### ARIA-M17-06: Design CLI interface
- **Description**: Design aria CLI commands
- **Dependency**: AGENT_CAPABLE
- **Priority**: normal
- **Tags**: research, package-manager, cli, design
- **Deliverables**:
  - Command structure
  - Output formatting
  - Error handling

## Implementation Progress

### Package Manager Core (COMPLETED - Jan 2026)
- [x] `crates/aria-pkg/` crate with full manifest parsing
- [x] AriaManifest with package, dependencies, dev-dependencies, build-dependencies
- [x] DependencySpec supporting simple and detailed specs (git, path, optional, features)
- [x] Features system with default features and recursive resolution
- [x] Target-specific dependencies (e.g., wasm-only deps)
- [x] LockFile generation and parsing
- [x] Resolver with version selection and source determination
- [x] DependencyGraph with topological sorting
- [x] 9 unit tests passing

### CLI Commands (COMPLETED - Jan 2026)
- [x] `aria-pkg init` - Initialize new project
- [x] `aria-pkg add` - Add dependency
- [x] `aria-pkg remove` - Remove dependency
- [x] `aria-pkg install` - Install all dependencies
- [x] `aria-pkg build` - Build project (integrated with aria-compiler)
- [x] `aria-pkg run` - Build and run project
- [x] `aria-pkg publish` - Publish package (placeholder)

### Remaining Work
- [ ] Registry protocol implementation
- [ ] Package signing and verification
- [ ] Transitive dependency resolution (PubGrub-style)
- [ ] Package caching
- [ ] Workspace support

## Success Criteria

- [x] Manifest format finalized
- [x] Resolution algorithm selected (basic, PubGrub planned)
- [ ] Security model defined
- [x] CLI design complete

## Key Resources

1. Cargo documentation
2. "PubGrub: Next-Generation Version Solving"
3. npm security documentation
4. Go module proxy documentation
5. "Software Supply Chain Security" - papers

## Timeline

Target: Q3 2026

## Related Milestones

- **Depends on**: M15 (Modules)
- **Enables**: Package ecosystem
- **Parallel**: M18 (IDE Integration)
