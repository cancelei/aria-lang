# ARIA-PD-016: Package Manager Design

**Decision ID**: ARIA-PD-016
**Status**: Approved
**Date**: 2026-01-15
**Author**: CARGO (Product Decision Agent)
**Research Inputs**:
- ARIA-M19-02: Package Management and Dependency Resolution (ATLAS)

---

## Executive Summary

This document defines Aria's package management system, synthesizing comprehensive research from ATLAS on package management across major ecosystems. The package manager is integrated into the unified `aria` CLI, providing dependency management, building, publishing, and workspace support with first-class effect system integration.

**Final Decisions**:
- **Manifest Format**: TOML-based `aria.toml` with effect declarations
- **CLI Commands**: Unified `aria` CLI with subcommands (add, build, publish, etc.)
- **Resolution Algorithm**: PubGrub-based with effect compatibility checking
- **Lock File**: Deterministic `aria.lock` with content hashes and effect metadata
- **Registry**: Federated design with official `aria.pkg` registry
- **Security**: Sigstore-based provenance, mandatory 2FA for publishers

---

## Table of Contents

1. [Design Philosophy](#1-design-philosophy)
2. [Manifest Format: aria.toml](#2-manifest-format-ariatoml)
3. [CLI Command Structure](#3-cli-command-structure)
4. [Dependency Resolution Algorithm](#4-dependency-resolution-algorithm)
5. [Lock File Format: aria.lock](#5-lock-file-format-arialock)
6. [Registry Design: aria.pkg](#6-registry-design-ariapkg)
7. [Workspace Support](#7-workspace-support)
8. [Effect Constraints in Dependencies](#8-effect-constraints-in-dependencies)
9. [Security Features](#9-security-features)
10. [Implementation Roadmap](#10-implementation-roadmap)

---

## 1. Design Philosophy

### 1.1 Core Principles

```
ARIA PACKAGE MANAGER PHILOSOPHY

Principle 1: Unified Experience
  - Single `aria` CLI for all operations
  - No separate package manager tool needed
  - Consistent with Rust's Cargo model

Principle 2: Effect-Aware Dependencies
  - Effects declared in manifests (required for publishing)
  - Effect compatibility verified at resolution time
  - Consumers can constrain dependency effects

Principle 3: Security by Default
  - Content hashes in lock files
  - Sigstore provenance attestations
  - 2FA required for publishing

Principle 4: Developer Ergonomics
  - TOML format with comments
  - Excellent error messages (PubGrub)
  - Inference where possible, explicit when needed
```

### 1.2 Comparison with Research Recommendations

| Research Finding | CARGO Decision | Rationale |
|------------------|----------------|-----------|
| ATLAS: TOML manifest (Cargo-style) | **Adopted** | Human-readable, comments supported, proven format |
| ATLAS: PubGrub resolver | **Adopted** | Fast, excellent errors, extensible for effects |
| ATLAS: Effect declarations in manifest | **Adopted** | First-class effect system integration |
| ATLAS: Sigstore provenance | **Adopted** | Modern supply chain security |
| ATLAS: Workspace support | **Adopted** | Native monorepo support |

---

## 2. Manifest Format: aria.toml

### 2.1 Complete Schema

```toml
#======================================================================
# aria.toml - Aria Package Manifest (Complete Reference)
#======================================================================

[package]
# Required fields
name = "my-package"              # Package name (lowercase, hyphens allowed)
version = "1.0.0"                # Semantic version (MAJOR.MINOR.PATCH)

# Required for publishing to aria.pkg
description = "A description of this package"
license = "MIT OR Apache-2.0"    # SPDX expression
authors = ["Author Name <author@example.com>"]
repository = "https://github.com/user/package"

# Optional metadata
readme = "README.md"
homepage = "https://package.dev"
documentation = "https://docs.package.dev"
keywords = ["keyword1", "keyword2"]  # Max 5 keywords
categories = ["category1"]

# Aria version requirement
aria-version = ">=1.0"

# Effect declarations (inferred locally, required for publishing)
effects = ["IO", "Async", "Exception[MyError]"]

# Effect handlers this package provides
provides-handlers = ["Database", "Cache"]

# Build script (optional)
build = "build.aria"

#======================================================================
# Dependencies
#======================================================================

[dependencies]
# Version specification formats:
#   "1.0"       - Caret: >=1.0.0, <2.0.0 (default)
#   "~1.5"      - Tilde: >=1.5.0, <1.6.0
#   "=1.2.3"    - Exact: exactly 1.2.3
#   ">=1.0, <2.0" - Range: explicit bounds

# Simple version (from default registry)
json = "1.5"

# With features enabled
http = { version = "2.0", features = ["server", "tls"] }

# Optional dependency (becomes a feature)
compression = { version = "1.0", optional = true }

# From specific registry
internal = { version = "1.0", registry = "company" }

# From git repository
experimental = {
    git = "https://github.com/user/experimental",
    branch = "main"
}

# From local path (for development)
local-lib = { path = "../local-lib" }

# Effect-constrained dependency
limited-io = { version = "1.0", max-effects = ["IO"] }

# Platform-specific dependency
native-bindings = { version = "1.0", target = "cfg(not(target_arch = \"wasm32\"))" }

[dev-dependencies]
testing = "1.0"
benchmark = "0.5"

[build-dependencies]
codegen = "1.0"

#======================================================================
# Features
#======================================================================

[features]
# Default features (enabled unless user opts out)
default = ["std"]

# Simple feature flags
std = []
no_std = []

# Feature that activates optional dependency
compression = ["dep:compression"]

# Feature with sub-features
full = ["std", "compression", "async"]

# Effect-modifying feature
async = {
    dependencies = ["aria-async"],
    effects = ["Async"]  # Adds Async effect when enabled
}

# Platform-specific feature
native = {
    dependencies = ["native-bindings"],
    target = "native"
}
wasm = {
    dependencies = ["wasm-bindings"],
    target = "wasm32"
}

#======================================================================
# Targets
#======================================================================

[targets]
default = "native"

[targets.native]
features = ["native"]

[targets.wasm]
target = "wasm32-unknown-unknown"
features = ["wasm"]

#======================================================================
# Binary and Library Targets
#======================================================================

[[bin]]
name = "my-app"
path = "src/main.aria"

[[bin]]
name = "my-tool"
path = "src/bin/tool.aria"
required-features = ["cli"]

[[example]]
name = "basic-usage"
path = "examples/basic.aria"

[[test]]
name = "integration"
path = "tests/integration.aria"

[[bench]]
name = "performance"
path = "benches/perf.aria"

#======================================================================
# Build Profiles
#======================================================================

[profile.dev]
opt-level = 0
debug = true
contracts = "runtime"     # Check contracts at runtime

[profile.release]
opt-level = 3
debug = false
contracts = "compile"     # Verify contracts at compile time
lto = true

[profile.test]
inherits = "dev"
contracts = "runtime"

#======================================================================
# Publishing Configuration
#======================================================================

[publish]
# Require provenance attestation (recommended)
provenance = true

# Allowed CI systems for provenance
allowed-builders = ["github-actions", "gitlab-ci"]

# Files to include in package (defaults: src/, aria.toml)
include = ["src/", "README.md", "LICENSE"]

# Files to exclude
exclude = ["tests/fixtures/large-file.bin"]

#======================================================================
# Workspace Configuration (root aria.toml only)
#======================================================================

# [workspace]
# members = ["core", "cli", "plugins/*"]
# exclude = ["experiments"]
#
# [workspace.package]
# version = "1.0.0"
# authors = ["Team <team@example.com>"]
# license = "MIT"
# aria-version = ">=1.0"
#
# [workspace.dependencies]
# json = "1.5"
# http = { version = "2.0", features = ["server"] }
#
# [workspace.effects]
# allowed = ["IO", "Async", "Console", "Exception"]
# denied = ["Unsafe"]
```

### 2.2 Minimal Application Example

```toml
# aria.toml - Minimal web application
[package]
name = "my-web-app"
version = "0.1.0"

[dependencies]
http = "2.0"
json = "1.5"

[[bin]]
name = "server"
path = "src/main.aria"
```

### 2.3 Library Example

```toml
# aria.toml - Library with features
[package]
name = "aria-http"
version = "2.3.1"
description = "HTTP client and server for Aria"
license = "MIT"
authors = ["Aria Team <team@aria-lang.org>"]
repository = "https://github.com/aria-lang/http"
keywords = ["http", "web", "async"]
categories = ["web-programming", "network-programming"]
aria-version = ">=1.0"
effects = ["IO", "Async", "Exception[HttpError]"]

[dependencies]
url = "3.0"
tls = { version = "1.5", optional = true }
zlib = { version = "1.0", optional = true }

[dev-dependencies]
testing = "1.0"

[features]
default = ["tls"]
tls = ["dep:tls"]
compression = ["dep:zlib"]
server = []
full = ["tls", "compression", "server"]
```

### 2.4 Version Range Syntax Reference

| Syntax | Meaning | Example |
|--------|---------|---------|
| `"1.2.3"` | Caret (default): `>=1.2.3, <2.0.0` | `"1.2"` means `>=1.2.0, <2.0.0` |
| `"~1.2.3"` | Tilde: `>=1.2.3, <1.3.0` | `"~1.2"` means `>=1.2.0, <1.3.0` |
| `"=1.2.3"` | Exact: exactly `1.2.3` | `"=1.0.0"` |
| `">=1.0, <2.0"` | Range: explicit bounds | `">=1.5.0, <1.8.0"` |
| `"1.2.*"` | Wildcard: any patch | `"1.*"` means any 1.x |
| `"*"` | Any version | (discouraged) |

---

## 3. CLI Command Structure

### 3.1 Command Overview

```bash
aria <COMMAND> [OPTIONS] [ARGS]

COMMANDS:
  Project Management:
    new         Create a new Aria project
    init        Initialize Aria in existing directory
    clean       Remove build artifacts

  Dependency Management:
    add         Add a dependency
    remove      Remove a dependency
    update      Update dependencies
    tree        Display dependency tree

  Building:
    build       Compile the project
    check       Type check without building
    run         Build and run a binary
    test        Run tests
    bench       Run benchmarks
    doc         Generate documentation

  Package Registry:
    search      Search for packages
    info        Display package information
    publish     Publish to registry
    yank        Yank a published version
    login       Authenticate with registry
    logout      Log out from registry
    owner       Manage package owners

  Effects and Security:
    effects     Display effect information
    audit       Security and effect audit

  Workspace:
    workspace   Workspace commands

  Misc:
    fmt         Format source files
    lint        Run linter
    fix         Auto-fix lint issues
    version     Display version information
    help        Display help
```

### 3.2 Project Management Commands

```bash
# Create new project
aria new my-project                    # Binary project
aria new my-lib --lib                  # Library project
aria new my-app --template=web         # From template

# Initialize in existing directory
aria init
aria init --lib

# Clean build artifacts
aria clean
aria clean --all                       # Include cached dependencies
```

### 3.3 Dependency Management Commands

```bash
# Add dependencies
aria add json                          # Add latest version
aria add http@2.0                      # Specific version
aria add http@^2.0                     # Version constraint
aria add http --features=server,tls    # With features
aria add testing --dev                 # Dev dependency
aria add codegen --build               # Build dependency
aria add --git https://github.com/user/repo  # From git
aria add --git https://... --branch main     # Git branch
aria add --git https://... --tag v1.0.0      # Git tag
aria add --git https://... --rev abc123      # Git commit
aria add --path ../local-lib           # Local path

# Remove dependencies
aria remove json
aria remove testing --dev

# Update dependencies
aria update                            # Update all (respecting constraints)
aria update json                       # Update specific package
aria update --aggressive               # Update to latest (may break semver)

# Display dependency tree
aria tree                              # Full tree
aria tree --depth 2                    # Limit depth
aria tree --duplicates                 # Show duplicate versions
aria tree --inverted json              # Show what depends on json
```

### 3.4 Build Commands

```bash
# Build project
aria build                             # Debug build
aria build --release                   # Release build
aria build --target wasm32             # Cross-compile

# Type check only
aria check
aria check --all-features

# Run binary
aria run                               # Default binary
aria run server                        # Named binary
aria run -- --port 8080                # Pass arguments to binary

# Run tests
aria test                              # All tests
aria test tests/unit                   # Specific tests
aria test --coverage                   # With coverage report
aria test --doc                        # Documentation tests

# Run benchmarks
aria bench
aria bench --baseline previous         # Compare to baseline

# Generate documentation
aria doc
aria doc --open                        # Open in browser
aria doc --no-deps                     # Skip dependencies
```

### 3.5 Registry Commands

```bash
# Search packages
aria search http                       # Search by name
aria search --effects IO,Async         # Filter by effects
aria search --features server          # Filter by features

# Package information
aria info http                         # Display package info
aria info http@2.0.0                   # Specific version
aria info http --versions              # List all versions

# Authentication
aria login                             # Interactive login
aria login --token                     # Token-based
aria logout

# Publishing
aria publish                           # Publish package
aria publish --dry-run                 # Validate without publishing
aria publish --registry internal       # Publish to private registry

# Version management
aria yank 1.0.0                        # Yank version
aria yank 1.0.0 --undo                 # Undo yank

# Owner management
aria owner list                        # List owners
aria owner add user@example.com        # Add owner
aria owner remove user@example.com     # Remove owner
```

### 3.6 Effect and Audit Commands

```bash
# Display effects
aria effects                           # Project effects
aria effects --deps                    # Include dependency effects
aria effects --transitive              # Full transitive closure

# Security and effect audit
aria audit                             # Full audit
aria audit --effects                   # Effect audit only
aria audit --security                  # Security audit only
aria audit --fix                       # Apply automatic fixes
```

### 3.7 Workspace Commands

```bash
# List workspace members
aria workspace list

# Display dependency graph
aria workspace graph

# Run command on specific member
aria -p core build                     # Build 'core' member
aria -p cli test                       # Test 'cli' member
aria -p "*" check                      # Check all members

# Build entire workspace
aria build --workspace
```

### 3.8 Formatting and Linting

```bash
# Format code
aria fmt                               # Format all
aria fmt --check                       # Check only (CI mode)

# Lint
aria lint
aria lint --fix                        # Auto-fix issues

# Combined fix
aria fix                               # Lint + format
```

---

## 4. Dependency Resolution Algorithm

### 4.1 PubGrub-Based Resolution

The resolver uses the PubGrub algorithm with extensions for effect compatibility checking.

```
ARIA DEPENDENCY RESOLUTION ALGORITHM

Phase 1: Standard PubGrub Resolution
  - Uses Conflict-Driven Clause Learning (CDCL)
  - Learns from conflicts to prune search space
  - Produces excellent error messages

Phase 2: Effect Metadata Collection
  - For each resolved package, fetch effect declarations
  - Build effect map: (package, version) -> effects

Phase 3: Effect Compatibility Verification
  - Check max-effects constraints
  - Verify transitive effect closure
  - Ensure effect handlers are available

Phase 4: Root Package Verification
  - Collect all transitive effects
  - Verify against allowed_effects (if specified)
  - Warn on unexpected effects
```

### 4.2 Resolution Algorithm Pseudocode

```aria
fn resolve_with_effects(manifest: Manifest) -> Result[Resolution, Error]
  # Phase 1: Standard PubGrub resolution
  let base_resolution = pubgrub_resolve(manifest.dependencies)?

  # Phase 2: Collect effect metadata
  let effect_map = Map.new()
  for (pkg, version) in base_resolution
    let pkg_effects = fetch_package_effects(pkg, version)
    effect_map.insert((pkg, version), pkg_effects)
  end

  # Phase 3: Verify effect compatibility
  for (pkg, version) in base_resolution
    let pkg_meta = manifest.get_dependency(pkg)

    # Check max-effects constraint
    if let Some(max) = pkg_meta.max_effects
      let actual = effect_map.get((pkg, version))
      if not actual.is_subset_of(max)
        return Err(EffectError.exceeded(pkg, actual, max))
      end
    end

    # Check transitive effect closure
    let transitive = compute_transitive_effects(pkg, version, effect_map)
    let declared = effect_map.get((pkg, version))
    if not transitive.is_subset_of(declared)
      return Err(EffectError.undeclared_transitive(pkg, transitive, declared))
    end
  end

  # Phase 4: Verify root package effects
  let all_effects = compute_transitive_effects(
    manifest.name,
    manifest.version,
    effect_map
  )

  if let Some(allowed) = manifest.allowed_effects
    if not all_effects.is_subset_of(allowed)
      let unexpected = all_effects.difference(allowed)
      return Err(EffectError.unexpected_in_deps(unexpected))
    end
  end

  # Phase 5: Verify effect handler availability
  for (pkg, version) in base_resolution
    let requires = effect_map.get((pkg, version)).requires_handlers
    for handler in requires
      if not handler_provided_by(handler, base_resolution, effect_map)
        return Err(EffectError.missing_handler(pkg, handler))
      end
    end
  end

  Ok(Resolution { packages: base_resolution, effects: effect_map })
end
```

### 4.3 Error Message Quality

PubGrub provides exceptional error messages:

```
Traditional Resolver:
  Error: Could not resolve dependencies

PubGrub Resolver:
  Because my-project depends on http >=2.0.0
    and http >=2.0.0 depends on tls >=1.2.0
    and tls >=1.2.0 requires aria >=1.1.0
    and you are using aria 1.0.0
  http >=2.0.0 cannot be installed.

  Suggestions:
    1. Upgrade to aria >=1.1.0
    2. Use http >=1.5.0, <2.0.0 which supports aria 1.0.0

Effect Resolution Error:
  Package http 2.0.0 requires effect handler "Async"
    but no package in the dependency graph provides it.

  Suggestions:
    1. Add aria-async as a dependency
    2. Implement Async handler in your application
    3. Use http-sync 2.0.0 (no Async requirement)

Effect Constraint Error:
  Package limited-io 1.0 declared max-effects = ["IO"]
    but http 2.0.0 declares effects = ["IO", "Async"]

  Suggestions:
    1. Remove max-effects constraint from limited-io
    2. Use http-sync 2.0.0 which only requires ["IO"]
```

---

## 5. Lock File Format: aria.lock

### 5.1 Lock File Purpose

1. **Reproducibility**: Same versions across machines and time
2. **Speed**: Skip resolution when lock is valid
3. **Security**: Content hashes verify integrity
4. **Auditability**: Track exactly what is installed
5. **Effect Tracking**: Record effect metadata for each package

### 5.2 Lock File Format

```toml
# aria.lock
# DO NOT EDIT - Generated by aria 1.0.0
#
# This file ensures reproducible builds by locking exact versions.
# Commit this file to version control for APPLICATIONS.
# Libraries should NOT commit aria.lock.

version = 2
aria-version = "1.0.0"

[metadata]
generated = "2026-01-15T10:30:00Z"
platform = "linux-x86_64"
resolver = "pubgrub-1.0"

# Root package
[root]
name = "my-project"
version = "1.0.0"
dependencies = [
    "json 1.5.2",
    "http 2.3.1",
]
effects = ["IO", "Async", "Exception[AppError]"]

# Locked packages
[[package]]
name = "json"
version = "1.5.2"
source = "registry+https://aria.pkg"
checksum = "sha256:abc123def456789012345678901234567890abcdef1234567890abcdef123456"
effects = []
provenance = {
    builder = "github-actions",
    repository = "https://github.com/aria-lang/json",
    commit = "abc123def456789012345678901234567890abcdef",
    log = "https://rekor.sigstore.dev/api/v1/log/entries/abc123"
}

[[package]]
name = "http"
version = "2.3.1"
source = "registry+https://aria.pkg"
checksum = "sha256:def456789012345678901234567890abcdef1234567890abcdef123456789012"
effects = ["IO", "Async"]
features = ["server", "tls"]
dependencies = [
    "url 3.0.0",
    "tls 1.5.0",
]
provenance = {
    builder = "github-actions",
    repository = "https://github.com/aria-lang/http",
    commit = "def456789012345678901234567890abcdef1234",
    log = "https://rekor.sigstore.dev/api/v1/log/entries/def456"
}

[[package]]
name = "url"
version = "3.0.0"
source = "registry+https://aria.pkg"
checksum = "sha256:789012345678901234567890abcdef1234567890abcdef12345678901234567890"
effects = []

[[package]]
name = "tls"
version = "1.5.0"
source = "registry+https://aria.pkg"
checksum = "sha256:012345678901234567890abcdef1234567890abcdef123456789012345678901234"
effects = ["IO"]

# Git dependency
[[package]]
name = "experimental"
version = "0.5.0"
source = "git+https://github.com/user/experimental?rev=abc123def456"
checksum = "sha256:345678901234567890abcdef1234567890abcdef12345678901234567890123456"
effects = ["IO", "Console"]

# Path dependency (for development)
[[package]]
name = "local-lib"
version = "0.1.0"
source = "path+../local-lib"
effects = ["IO"]

# Platform-specific package
[[package]]
name = "native-bindings"
version = "1.0.0"
source = "registry+https://aria.pkg"
checksum = "sha256:567890123456789012345678901234567890abcdef1234567890abcdef12345678"
effects = ["FFI"]
target = "cfg(not(target_arch = \"wasm32\"))"
```

### 5.3 Lock File Rules

| Situation | aria.lock Behavior |
|-----------|-------------------|
| `aria add <pkg>` | Update lock file |
| `aria update` | Regenerate lock file |
| `aria build` (lock exists) | Use exact versions from lock |
| `aria build` (no lock) | Resolve and generate lock |
| Lock conflicts with manifest | Re-resolve affected packages |
| Git dependency changes | Re-lock that dependency |

### 5.4 Lock File for Libraries vs Applications

**Applications**: SHOULD commit `aria.lock`
- Ensures reproducible deployments
- All team members get exact same versions

**Libraries**: SHOULD NOT commit `aria.lock`
- Allow consumers to resolve compatible versions
- Avoid version conflicts
- `.gitignore` should include `aria.lock`

---

## 6. Registry Design: aria.pkg

### 6.1 Architecture Overview

```
ARIA.PKG REGISTRY ARCHITECTURE

Index Repository (Git-based):
  aria-index/
  ├── config.toml           # Registry configuration
  ├── ar/ia/
  │   └── aria-http         # Package metadata (JSON lines)
  └── js/on/
      └── json              # Package metadata (JSON lines)

API Server:
  - RESTful API for package operations
  - CDN-backed package storage
  - Sigstore integration for provenance

Naming Convention (directory structure):
  - 1-char names: 1/{name}
  - 2-char names: 2/{name}
  - 3-char names: 3/{first-char}/{name}
  - 4+ char names: {first-two}/{next-two}/{name}
```

### 6.2 API Endpoints

```yaml
/api/v1:
  /packages:
    GET:
      description: "List/search packages"
      parameters:
        q: "search query"
        page: "page number"
        per_page: "results per page (max 100)"
        sort: "downloads | recent | relevance"
        effects: "filter by effects (e.g., IO,Async)"
      response: PackageList

    POST:
      description: "Publish package"
      auth: required (2FA)
      headers:
        Authorization: "Bearer <token>"
        X-Provenance: "<sigstore attestation>"
      body: multipart/form-data (package tarball + metadata)
      response: PublishResult

  /packages/{name}:
    GET:
      description: "Get package info"
      response: PackageInfo

  /packages/{name}/versions:
    GET:
      description: "List all versions"
      response: VersionList

  /packages/{name}/{version}:
    GET:
      description: "Get version info"
      response: VersionInfo

  /packages/{name}/{version}/download:
    GET:
      description: "Download package archive"
      response: application/gzip

  /packages/{name}/{version}/effects:
    GET:
      description: "Get effect declarations"
      response: EffectInfo

  /packages/{name}/{version}/provenance:
    GET:
      description: "Get provenance attestation"
      response: ProvenanceInfo

  /packages/{name}/{version}/yank:
    POST:
      description: "Yank version"
      auth: required (owner)
      response: YankResult

    DELETE:
      description: "Undo yank"
      auth: required (owner)
      response: YankResult

  /packages/{name}/owners:
    GET:
      description: "List package owners"
      response: OwnerList

    POST:
      description: "Add owner"
      auth: required (owner)

    DELETE:
      description: "Remove owner"
      auth: required (owner)

  /search:
    GET:
      description: "Search packages"
      parameters:
        q: "query"
        effects: "filter by effects"
        features: "filter by features"
      response: SearchResults

# Index endpoint (for fast resolution)
/index/{prefix}/{name}:
    GET:
      description: "Get package index entry (JSON lines)"
      response: IndexEntry
```

### 6.3 Package Index Format

Each package has a file in the index with JSON lines (one per version):

```json
{"name":"http","vers":"2.0.0","deps":[{"name":"url","req":"^3.0"},{"name":"tls","req":"^1.5","optional":true}],"effects":["IO"],"features":{"tls":["dep:tls"],"server":[]},"cksum":"sha256:..."}
{"name":"http","vers":"2.1.0","deps":[{"name":"url","req":"^3.0"},{"name":"tls","req":"^1.5","optional":true}],"effects":["IO","Async"],"features":{"tls":["dep:tls"],"server":[],"async":[]},"cksum":"sha256:..."}
{"name":"http","vers":"2.3.1","deps":[{"name":"url","req":"^3.0"},{"name":"tls","req":"^1.5","optional":true}],"effects":["IO","Async"],"features":{"tls":["dep:tls"],"server":[],"async":[]},"cksum":"sha256:..."}
```

### 6.4 Private Registry Configuration

```toml
# ~/.aria/config.toml
[registries]
# Official registry (default)
aria-pkg = { url = "https://aria.pkg", default = true }

# Company private registry
internal = {
    url = "https://packages.company.com",
    auth = "token"  # Use ARIA_REGISTRY_INTERNAL_TOKEN env var
}

# Air-gapped mirror
offline = {
    url = "file:///opt/aria-registry",
    readonly = true
}

[auth]
# Token configuration
internal = { token-cmd = "vault read -field=token secret/aria-registry" }
```

### 6.5 Registry Rules

| Rule | Description |
|------|-------------|
| Name uniqueness | Names are globally unique, first-come-first-served |
| Name squatting | Inactive packages may be reclaimed after 1 year |
| Immutability | Published versions cannot be modified |
| Yanking | Versions can be yanked (hidden from new resolves) but not deleted |
| 2FA required | All publishers must have 2FA enabled |
| Provenance | Sigstore attestations required for new packages |

---

## 7. Workspace Support

### 7.1 Workspace Configuration

```toml
# Root aria.toml for workspace
[workspace]
# Workspace members (glob patterns supported)
members = [
    "core",
    "cli",
    "web",
    "plugins/*"
]

# Exclude patterns
exclude = ["experiments", "scratch/*"]

# Shared package metadata (inherited by members)
[workspace.package]
version = "1.0.0"
authors = ["Aria Team <team@aria-lang.org>"]
license = "MIT"
aria-version = ">=1.0"
repository = "https://github.com/aria-lang/aria"

# Shared dependency versions
[workspace.dependencies]
json = "1.5"
http = { version = "2.0", features = ["server"] }
testing = "1.0"

# Workspace-wide effect constraints
[workspace.effects]
# All members can use these effects
allowed = ["IO", "Async", "Console", "Exception"]

# No member can use these effects
denied = ["Unsafe", "FFI"]

# Inheritance mode: strict = members cannot exceed, permissive = members can add
inheritance = "strict"
```

### 7.2 Member Package Configuration

```toml
# core/aria.toml
[package]
name = "aria-core"
version.workspace = true         # Inherit from workspace
authors.workspace = true
license.workspace = true
effects = []                     # Pure package

[dependencies]
# Uses workspace-defined version
json.workspace = true

# Member-specific dependency
parsing = "0.5"
```

```toml
# cli/aria.toml
[package]
name = "aria-cli"
version.workspace = true
effects = ["IO", "Console"]

[dependencies]
aria-core = { path = "../core" }
json.workspace = true
http.workspace = true

[[bin]]
name = "aria"
```

### 7.3 Workspace Commands

```bash
# List all workspace members
aria workspace list
# Output:
#   core        (aria-core v1.0.0)
#   cli         (aria-cli v1.0.0)
#   web         (aria-web v1.0.0)
#   plugins/sql (aria-sql v1.0.0)
#   plugins/json (aria-json v1.0.0)

# Build specific member
aria -p core build
aria -p cli build --release

# Build all members
aria build --workspace

# Test specific member
aria -p core test

# Test all members
aria test --workspace

# Publish all members (in dependency order)
aria publish --workspace
```

### 7.4 Workspace Dependency Graph

```bash
aria workspace graph

# Output:
# aria-cli
#   -> aria-core
#   -> aria-web
#        -> aria-core
# aria-web
#   -> aria-core
# aria-core (no deps)
# plugins/sql
#   -> aria-core
# plugins/json
#   -> aria-core
```

---

## 8. Effect Constraints in Dependencies

### 8.1 Effect Declaration in Manifests

Packages declare their effects in `aria.toml`:

```toml
[package]
name = "database-client"
version = "1.0.0"

# Effects this package uses (required for publishing)
effects = ["IO", "Async", "Exception[DbError]"]

# Effect handlers this package provides
provides-handlers = ["Database"]

# Effect handlers this package requires (must be provided by consumer)
requires-handlers = ["Async"]
```

### 8.2 Effect Constraints on Dependencies

Consumers can constrain the effects allowed by dependencies:

```toml
[dependencies]
# Constrain to specific effects
http = { version = "^2.0", max-effects = ["IO"] }

# This will fail resolution:
# http 2.0.0 declares effects = ["IO", "Async"]
# Consumer allows only ["IO"]
# Error: http 2.0.0 requires effect "Async" which exceeds max-effects
```

### 8.3 Effect Verification Flow

```
EFFECT VERIFICATION DURING RESOLUTION

1. Parse manifest dependencies
2. Run PubGrub to get version solution
3. For each (package, version) in solution:
   a. Fetch package effect metadata
   b. Check max-effects constraints from consumer
   c. Verify declared effects >= transitive effects

4. For root package:
   a. Compute transitive effect closure
   b. If allowed_effects specified, verify subset
   c. Warn on unexpected effects

5. Verify effect handlers:
   a. For each package requiring handlers
   b. Ensure handler is provided by some package in solution
   c. Error if handler unavailable
```

### 8.4 Effect Audit Command

```bash
aria effects
# Output:
# Package: my-app v1.0.0
# Direct effects: IO, Console
#
# Dependency effects:
#   http 2.3.1: IO, Async
#   json 1.5.2: (pure)
#   tls 1.5.0: IO
#
# Transitive closure: IO, Async, Console

aria effects --deps
# Output:
# Effect graph:
#   my-app
#     IO      <- direct
#     Console <- direct
#     Async   <- http 2.3.1
#   http 2.3.1
#     IO      <- direct
#     Async   <- direct
#     IO      <- tls 1.5.0
#   tls 1.5.0
#     IO      <- direct
#   json 1.5.2
#     (pure)

aria audit --effects
# Output:
# Effect Audit Report
# ===================
#
# Declared effects: IO, Async, Console
# Actual effects:   IO, Async, Console
# Status: OK
#
# Effect handlers:
#   Async: provided by aria-async 1.0.0
#   IO:    provided by stdlib
#   Console: provided by stdlib
# Status: OK
```

---

## 9. Security Features

### 9.1 Security Architecture

```
ARIA PACKAGE SECURITY MODEL

Layer 1: Authentication
  - API tokens for CLI access
  - 2FA required for all publishers
  - Scoped tokens (read-only, publish-only, admin)

Layer 2: Integrity
  - SHA-256 checksums for all packages
  - Checksums stored in lock file
  - Verification on every install

Layer 3: Provenance
  - Sigstore-based attestations
  - Ties package to source repository
  - Verifiable build chain

Layer 4: Effect Sandboxing
  - Effect constraints limit dependency capabilities
  - Audit trail of all effects
  - Block dangerous effects (Unsafe, FFI)
```

### 9.2 Sigstore Provenance

When publishing from CI, Sigstore attestations link the package to its source:

```yaml
# .github/workflows/publish.yml
name: Publish

on:
  push:
    tags: ['v*']

permissions:
  id-token: write  # Required for Sigstore
  contents: read

jobs:
  publish:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: aria-lang/setup-aria@v1

      - name: Publish with provenance
        run: aria publish
        env:
          ARIA_TOKEN: ${{ secrets.ARIA_TOKEN }}
```

The resulting provenance attestation:

```json
{
  "predicateType": "https://slsa.dev/provenance/v1",
  "subject": [
    {
      "name": "my-package",
      "digest": {
        "sha256": "abc123..."
      }
    }
  ],
  "predicate": {
    "buildType": "https://aria-lang.org/publish/v1",
    "builder": {
      "id": "https://github.com/actions/runner"
    },
    "invocation": {
      "configSource": {
        "uri": "git+https://github.com/user/my-package",
        "digest": {
          "sha1": "abc123def456..."
        },
        "entryPoint": ".github/workflows/publish.yml"
      }
    },
    "metadata": {
      "buildInvocationId": "https://github.com/user/my-package/actions/runs/12345",
      "buildStartedOn": "2026-01-15T10:30:00Z",
      "buildFinishedOn": "2026-01-15T10:35:00Z"
    }
  }
}
```

### 9.3 Security Configuration

```toml
# aria.toml security settings
[security]
# Effect sandboxing
max-effects = ["IO", "Console"]  # Limit dependency effects
deny-effects = ["Unsafe"]        # Block specific effects

# Dependency verification
require-checksums = true         # Always verify checksums
require-provenance = "optional"  # "required" | "optional" | "none"

# Registry trust
trusted-registries = ["aria.pkg"]
allow-git-deps = true            # Set false for locked-down environments

[publish]
provenance = true                # Require provenance for this package
allowed-builders = ["github-actions", "gitlab-ci"]
```

### 9.4 Security Audit

```bash
aria audit

# Output:
# Security Audit Report
# =====================
#
# Vulnerability Scan:
#   ARIA-SEC-2026-001 (HIGH): tls 1.4.x has known vulnerability
#     Recommendation: Upgrade to tls >=1.5.0
#     Status: Fixed by current lock file (tls 1.5.0)
#
# Effect Audit:
#   Transitive effects: IO, Async, Console
#   Dangerous effects: None detected
#   Status: OK
#
# Provenance Verification:
#   http 2.3.1: Verified (github-actions)
#   json 1.5.2: Verified (github-actions)
#   tls 1.5.0: Verified (github-actions)
#   url 3.0.0: No provenance (published before requirement)
#   Status: WARN - 1 package without provenance
#
# Checksum Verification:
#   All 4 packages verified
#   Status: OK
#
# Overall: PASS with 1 warning
```

### 9.5 Supply Chain Attack Mitigations

| Attack Vector | Mitigation |
|---------------|------------|
| Typosquatting | Reserved names, similarity check on publish |
| Dependency confusion | Namespace prefixes for internal packages |
| Account compromise | 2FA required, publish notifications |
| Malicious updates | Provenance verification, effect constraints |
| Man-in-the-middle | TLS + content hashes |

---

## 10. Implementation Roadmap

### 10.1 Priority Matrix

| Priority | Component | Effort | Impact | Dependencies |
|----------|-----------|--------|--------|--------------|
| P0 | aria.toml parser | Medium | Critical | None |
| P0 | PubGrub resolver | High | Critical | aria.toml parser |
| P0 | aria.lock generation | Medium | Critical | Resolver |
| P0 | Basic CLI (add, build) | Medium | Critical | Parser, resolver |
| P1 | Effect resolution | High | High | Resolver |
| P1 | Registry client | Medium | High | CLI |
| P1 | Full CLI commands | Medium | High | Registry client |
| P2 | Workspace support | Medium | Medium | Full CLI |
| P2 | Private registries | Medium | Medium | Registry client |
| P2 | aria.pkg registry server | High | Medium | Registry client |
| P3 | Provenance verification | Medium | Medium | Registry |
| P3 | Build scripts | High | Low | Full CLI |

### 10.2 Phase 1: Foundation (Weeks 1-4)

- Implement aria.toml parser
- Port/adapt pubgrub-rs for Aria
- Generate aria.lock from resolution
- Basic `aria add`, `aria build` commands
- Local path dependencies

### 10.3 Phase 2: Effects and Registry (Weeks 5-8)

- Effect metadata in manifests
- Effect compatibility checking in resolver
- Registry client (download, search)
- `aria publish` command
- `aria effects` and `aria audit` commands

### 10.4 Phase 3: Workspace and Security (Weeks 9-12)

- Workspace support
- Private registry configuration
- Sigstore provenance integration
- Security audit features
- Full CLI completion

### 10.5 Phase 4: Registry Server (Weeks 13-16)

- aria.pkg registry server implementation
- Git-based index
- CDN integration
- Authentication (2FA)
- Admin interface

---

## Appendix A: Complete CLI Reference

```
aria 1.0.0
Aria programming language toolchain

USAGE:
    aria [OPTIONS] <COMMAND>

OPTIONS:
    -v, --verbose       Enable verbose output
    -q, --quiet         Suppress output
    --color <WHEN>      Coloring: auto, always, never [default: auto]
    -p, --package <PKG> Run command on specific package
    -h, --help          Print help information
    -V, --version       Print version information

PROJECT COMMANDS:
    new <NAME>          Create a new Aria project
        --lib           Create library project
        --template <T>  Use template (web, cli, lib)
        --vcs <VCS>     Version control: git, none [default: git]

    init                Initialize Aria in existing directory
        --lib           Initialize as library

    clean               Remove build artifacts
        --all           Include cached dependencies

DEPENDENCY COMMANDS:
    add <SPEC>...       Add dependencies
        --dev           Add as dev dependency
        --build         Add as build dependency
        --features <F>  Enable features (comma-separated)
        --optional      Add as optional dependency
        --git <URL>     Add from git repository
        --branch <B>    Git branch
        --tag <T>       Git tag
        --rev <R>       Git revision
        --path <P>      Add from local path
        --registry <R>  Use specific registry

    remove <PKG>...     Remove dependencies
        --dev           Remove from dev dependencies
        --build         Remove from build dependencies

    update [PKG]...     Update dependencies
        --aggressive    Update to latest (may break semver)
        --dry-run       Show what would be updated

    tree                Display dependency tree
        --depth <N>     Maximum display depth
        --duplicates    Highlight duplicate versions
        --inverted <P>  Show what depends on package

BUILD COMMANDS:
    build               Build the project
        --release       Release profile
        --target <T>    Target triple
        --all-features  Enable all features
        --no-default-features
        --features <F>  Enable specific features

    check               Type check without building

    run [BIN]           Build and run binary
        -- <ARGS>...    Arguments to pass to binary

    test [FILTER]       Run tests
        --coverage      Generate coverage report
        --doc           Run documentation tests
        --lib           Test library only
        --bins          Test binaries only

    bench [FILTER]      Run benchmarks
        --baseline <N>  Compare to named baseline
        --save <N>      Save as named baseline

    doc                 Generate documentation
        --open          Open in browser
        --no-deps       Don't document dependencies

REGISTRY COMMANDS:
    search <QUERY>      Search for packages
        --effects <E>   Filter by effects
        --features <F>  Filter by features
        --limit <N>     Maximum results

    info <PKG>          Display package information
        --versions      Show all versions

    publish             Publish package to registry
        --dry-run       Validate without publishing
        --registry <R>  Target registry
        --allow-dirty   Allow uncommitted changes

    yank <VERSION>      Yank a version
        --undo          Undo yank

    login               Authenticate with registry
        --token         Use token (not interactive)

    logout              Log out from registry

    owner               Manage package owners
        list            List owners
        add <USER>      Add owner
        remove <USER>   Remove owner

EFFECT AND AUDIT COMMANDS:
    effects             Display effect information
        --deps          Include dependency effects
        --transitive    Full transitive closure

    audit               Security and effect audit
        --effects       Effect audit only
        --security      Security audit only
        --fix           Apply automatic fixes
        --json          Output as JSON

WORKSPACE COMMANDS:
    workspace           Workspace operations
        list            List workspace members
        graph           Display dependency graph

MISC COMMANDS:
    fmt                 Format source code
        --check         Check only (exit 1 if unformatted)

    lint                Run linter
        --fix           Auto-fix issues

    fix                 Apply all auto-fixes (lint + fmt)

    version             Display version information

    help [COMMAND]      Display help for command
```

---

## Appendix B: Error Message Examples

### B.1 Version Conflict

```
error: failed to select a version for `http`

Because my-project depends on json ^1.5.0
  and json ^1.5.0 depends on http ^1.0.0
  and my-project also depends on http ^2.0.0
version solving failed.

The conflict is caused by:
  - my-project requires http ^2.0.0
  - json 1.5.2 requires http ^1.0.0

Possible solutions:
  1. Upgrade json to ^2.0.0 (compatible with http ^2.0.0)
  2. Downgrade to http ^1.0.0
  3. Add http = ">=1.0, <3.0" to allow overlapping versions
```

### B.2 Effect Constraint Violation

```
error: effect constraint violated for `http`

http 2.3.1 declares effects:
  - IO
  - Async

But the dependency constraint specifies:
  max-effects = ["IO"]

The 'Async' effect is not allowed.

Possible solutions:
  1. Remove max-effects constraint
  2. Add 'Async' to max-effects: ["IO", "Async"]
  3. Use http-sync 2.3.1 which only requires ["IO"]
```

### B.3 Missing Effect Handler

```
error: missing effect handler for `Database`

Package database-client 1.0.0 requires handler: Database
  but no package in the dependency graph provides it.

The Database handler must be provided by a dependency or
implemented in your application.

Possible solutions:
  1. Add aria-database as a dependency (provides Database handler)
  2. Implement a Database handler in your application:

     handle database_operation() with
       Database.query(sql) => {
         # Your implementation
       }
       Database.execute(sql) => {
         # Your implementation
       }
     end
```

### B.4 Security Audit Failure

```
error: security audit failed

CRITICAL: 1 vulnerability found

ARIA-SEC-2026-042 (CRITICAL)
  Package: tls 1.2.3
  Title: Buffer overflow in certificate parsing
  Description: A malformed certificate can cause remote code execution
  Severity: CRITICAL (CVSS 9.8)

  Recommendation: Upgrade to tls >=1.5.0

  In aria.toml:
    [dependencies]
    - tls = "1.2"
    + tls = "1.5"

  Then run: aria update tls

Use --ignore-audit to bypass (not recommended)
```

---

**Document Status**: Approved
**Implementation Owner**: Aria Tooling Team
**Review Date**: 2026-01-15
**Next Review**: 2026-03-15 (post-Phase 1 completion)
