# ARIA-M19-02: Package Management and Dependency Resolution

**Task ID**: ARIA-M19-02
**Status**: Completed
**Date**: 2026-01-15
**Agent**: ATLAS (Eureka Research Agent)
**Focus**: Comprehensive package management design for Aria

---

## Executive Summary

This research document provides a comprehensive analysis of package management systems across major programming languages, with specific focus on how Aria's unique effect system and type safety features should influence its package manager design. The recommended package manager name is **`aria`** (the unified CLI tool), with the registry called **`aria.pkg`** or **`aria.dev`**.

**Key Recommendations:**
1. **Manifest Format**: TOML-based `aria.toml` with effect declarations
2. **Resolution Algorithm**: PubGrub-based with effect compatibility checking
3. **Lock File**: Deterministic `aria.lock` with content hashes and effect metadata
4. **Registry**: Federated design with official `aria.pkg` registry + private registry support
5. **Security**: Sigstore-based provenance, mandatory 2FA for publishers, effect-based sandboxing

---

## Table of Contents

1. [Package Manifest Format Comparison](#1-package-manifest-format-comparison)
2. [Dependency Resolution Algorithms](#2-dependency-resolution-algorithms)
3. [Lock File Format and Deterministic Builds](#3-lock-file-format-and-deterministic-builds)
4. [Package Registry Design](#4-package-registry-design)
5. [Workspace Support for Monorepos](#5-workspace-support-for-monorepos)
6. [Feature Flags and Optional Dependencies](#6-feature-flags-and-optional-dependencies)
7. [Security and Supply Chain](#7-security-and-supply-chain)
8. [Private Registries and Git Dependencies](#8-private-registries-and-git-dependencies)
9. [Effect System Integration](#9-effect-system-integration)
10. [Aria Package Manager Design](#10-aria-package-manager-design)

---

## 1. Package Manifest Format Comparison

### 1.1 Format Overview

| Ecosystem | File | Format | Key Characteristics |
|-----------|------|--------|---------------------|
| Rust | Cargo.toml | TOML | Unified, comprehensive, features system |
| JavaScript | package.json | JSON | Simple, scripts, multiple managers |
| Python | pyproject.toml | TOML | Build-backend agnostic, tool sections |
| Go | go.mod | Custom | Minimal, module paths, replace directives |
| Zig | build.zig.zon | Zon | Zig's DSL, content-addressed dependencies |

### 1.2 Cargo.toml (Rust) - Best in Class

```toml
[package]
name = "my-crate"
version = "1.2.3"
edition = "2021"
authors = ["Developer <dev@example.com>"]
description = "A great library"
license = "MIT OR Apache-2.0"
repository = "https://github.com/user/repo"
keywords = ["web", "async"]
categories = ["web-programming"]

[dependencies]
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", optional = true }
log = "0.4"

[dev-dependencies]
criterion = "0.5"

[build-dependencies]
cc = "1.0"

[features]
default = ["std"]
std = []
async = ["tokio", "dep:async-trait"]

[profile.release]
opt-level = 3
lto = true

[[bin]]
name = "mytool"
path = "src/bin/tool.rs"

[workspace]
members = ["crates/*"]
```

**Strengths:**
- Comprehensive metadata
- Feature system for conditional compilation
- Profile configuration
- Workspace support
- Clear dependency specification

### 1.3 package.json (JavaScript/Node.js)

```json
{
  "name": "@org/my-package",
  "version": "1.2.3",
  "description": "A great library",
  "main": "dist/index.js",
  "types": "dist/index.d.ts",
  "exports": {
    ".": {
      "import": "./dist/index.mjs",
      "require": "./dist/index.js"
    }
  },
  "scripts": {
    "build": "tsc",
    "test": "vitest"
  },
  "dependencies": {
    "lodash": "^4.17.21"
  },
  "devDependencies": {
    "typescript": "^5.0.0"
  },
  "peerDependencies": {
    "react": ">=18.0.0"
  },
  "optionalDependencies": {
    "fsevents": "^2.3.0"
  },
  "engines": {
    "node": ">=18.0.0"
  }
}
```

**Strengths:**
- Familiar JSON format
- Script system for tasks
- Peer dependencies for plugins
- Scoped packages (@org/name)

**Weaknesses:**
- No comments (JSON limitation)
- Schema sprawl
- Multiple package managers (npm/yarn/pnpm)

### 1.4 pyproject.toml (Python)

```toml
[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[project]
name = "my-package"
version = "1.2.3"
description = "A great library"
readme = "README.md"
license = "MIT"
authors = [
    { name = "Developer", email = "dev@example.com" }
]
requires-python = ">=3.10"
keywords = ["web", "async"]
classifiers = [
    "Development Status :: 4 - Beta",
    "Programming Language :: Python :: 3.10",
]
dependencies = [
    "requests>=2.28.0",
    "pydantic>=2.0",
]

[project.optional-dependencies]
dev = ["pytest", "mypy"]
docs = ["sphinx"]

[project.scripts]
mytool = "my_package:main"

[project.urls]
Homepage = "https://example.com"
Repository = "https://github.com/user/repo"

[tool.pytest.ini_options]
testpaths = ["tests"]

[tool.mypy]
strict = true
```

**Strengths:**
- Build-backend agnostic
- Tool configuration sections
- Optional dependency groups
- Standardized by PEP 517/518/621

### 1.5 Comparison Matrix

| Feature | Cargo.toml | package.json | pyproject.toml |
|---------|------------|--------------|----------------|
| Format | TOML | JSON | TOML |
| Comments | Yes | No | Yes |
| Features/Groups | Yes | Partial | Yes |
| Workspaces | Yes | Yes (pnpm) | Yes (uv) |
| Build Config | Yes | External | Build-backend |
| Tool Config | Limited | External | [tool.*] |
| Scripts | build.rs | scripts | entry points |

---

## 2. Dependency Resolution Algorithms

### 2.1 Algorithm Comparison

| Algorithm | Used By | Approach | Error Messages |
|-----------|---------|----------|----------------|
| **PubGrub** | Cargo, uv, Swift PM, Poetry | CDCL SAT | Excellent |
| **Backtracking** | pip (legacy), early Cargo | Naive search | Poor |
| **SAT Solver** | Composer, libsolv | Boolean SAT | Very poor |
| **Semver + Duplicates** | npm, pnpm | Allow multiple versions | N/A |

### 2.2 PubGrub Algorithm Deep Dive

PubGrub uses Conflict-Driven Clause Learning (CDCL), adapted from Boolean satisfiability solvers:

```
PubGrub Algorithm:

  1. UNIT PROPAGATION
     - Given incompatibility {A >= 1.0, B >= 2.0}
     - If A >= 1.0 is in solution
     - Derive: B < 2.0 must be true

  2. DECISION MAKING
     - Select undecided package
     - Choose version (prefer recent)
     - Add to partial solution

  3. CONFLICT RESOLUTION
     - When contradiction found:
       a) Analyze conflict root cause
       b) Learn new incompatibility
       c) Backjump to relevant decision
     - Prevents revisiting same failure

  4. TERMINATION
     - Solution found: return versions
     - Root conflict: explain failure
```

**Key Innovation**: Learning from conflicts to prune search space

### 2.3 Error Message Quality

**Traditional Resolver:**
```
Error: Could not resolve dependencies
```

**PubGrub:**
```
Because my-project depends on http >=2.0.0
  and http >=2.0.0 depends on tls >=1.2.0
  and tls >=1.2.0 requires OpenSSL >=1.1.1
  and your system has OpenSSL 1.0.2
http >=2.0.0 cannot be installed on your system.

Suggestions:
  1. Upgrade OpenSSL to >=1.1.1
  2. Use http >=1.5.0, <2.0.0 which supports older OpenSSL
```

### 2.4 Version Range Syntax Comparison

| Syntax | Cargo | npm | Poetry/pip |
|--------|-------|-----|------------|
| Exact | `=1.2.3` | `1.2.3` | `==1.2.3` |
| Caret (default) | `^1.2.3` or `1.2.3` | `^1.2.3` | `^1.2.3` |
| Tilde | `~1.2.3` | `~1.2.3` | `~=1.2.3` |
| Range | `>=1.0, <2.0` | `>=1.0.0 <2.0.0` | `>=1.0,<2.0` |
| Wildcard | `1.2.*` | `1.2.x` | `1.2.*` |
| Or | N/A | `1.0 \|\| 2.0` | N/A |

### 2.5 Recommendations for Aria

```aria
# Aria version syntax (Cargo-inspired)
dependencies = [
  "http" = "^2.0"           # >=2.0.0, <3.0.0 (caret default)
  "json" = "~1.5"           # >=1.5.0, <1.6.0 (tilde)
  "crypto" = ">=1.0, <2.0"  # Explicit range
  "legacy" = "=1.2.3"       # Exact version
]
```

---

## 3. Lock File Format and Deterministic Builds

### 3.1 Lock File Purposes

1. **Reproducibility**: Same versions across machines/time
2. **Speed**: Skip resolution when lock valid
3. **Security**: Content hashes verify integrity
4. **Auditability**: Track what was installed

### 3.2 Lock File Format Comparison

**Cargo.lock (TOML):**
```toml
# This file is automatically @generated by Cargo.
[[package]]
name = "serde"
version = "1.0.193"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "25dd9975e68d0cb5aa1120c288333fc98731bd1dd12f561e468ea4728c042b89"
dependencies = [
    "serde_derive",
]

[[package]]
name = "my-project"
version = "0.1.0"
dependencies = [
    "serde",
]
```

**package-lock.json (JSON):**
```json
{
  "name": "my-project",
  "version": "1.0.0",
  "lockfileVersion": 3,
  "requires": true,
  "packages": {
    "": {
      "name": "my-project",
      "version": "1.0.0",
      "dependencies": {
        "lodash": "^4.17.21"
      }
    },
    "node_modules/lodash": {
      "version": "4.17.21",
      "resolved": "https://registry.npmjs.org/lodash/-/lodash-4.17.21.tgz",
      "integrity": "sha512-v2kDE..."
    }
  }
}
```

**uv.lock (TOML, Python):**
```toml
version = 1
requires-python = ">=3.10"

[[package]]
name = "requests"
version = "2.31.0"
source = { registry = "https://pypi.org/simple" }
sdist = { url = "...", hash = "sha256:..." }
wheels = [
    { url = "...", hash = "sha256:..." }
]
dependencies = [
    "certifi>=2017.4.17",
    "charset-normalizer<4,>=2",
]
```

### 3.3 Lock File Features Comparison

| Feature | Cargo.lock | package-lock | uv.lock | pnpm-lock |
|---------|------------|--------------|---------|-----------|
| Format | TOML | JSON | TOML | YAML |
| Content Hash | SHA-256 | Subresource Integrity | SHA-256 | SHA-512 |
| Multiple Platforms | No | No | Yes | Yes |
| Multiple Python Versions | N/A | N/A | Yes | N/A |
| Git Commits | Yes | Yes | Yes | Yes |
| Registry URL | Yes | Yes | Yes | Yes |

### 3.4 Aria Lock File Design (aria.lock)

```toml
# aria.lock - DO NOT EDIT MANUALLY
# Generated by aria 1.0.0
version = 1
aria-version = ">=1.0.0"

# Resolution metadata
[resolution]
timestamp = "2026-01-15T10:30:00Z"
resolver = "pubgrub"
platform = "linux-x86_64"

# Packages
[[package]]
name = "http"
version = "2.3.1"
source = "registry+https://aria.pkg"
checksum = "sha256:abc123..."
effects = ["IO", "Async"]  # Effect metadata!
dependencies = [
    { name = "tls", version = "1.5.0" },
    { name = "url", version = "3.0.0" },
]

[[package]]
name = "json"
version = "1.5.2"
source = "registry+https://aria.pkg"
checksum = "sha256:def456..."
effects = []  # Pure package

[[package]]
name = "my-local-lib"
version = "0.1.0"
source = "path+../my-local-lib"
effects = ["IO", "Exception[ParseError]"]

[[package]]
name = "experimental"
version = "0.5.0"
source = "git+https://github.com/user/experimental?rev=abc123"
checksum = "sha256:ghi789..."
effects = ["IO", "Console"]
```

---

## 4. Package Registry Design

### 4.1 Registry Comparison

| Registry | Language | Architecture | Notable Features |
|----------|----------|--------------|------------------|
| **crates.io** | Rust | Centralized, Git index | Squatting prevention, yank |
| **npm** | JavaScript | Centralized | Scopes, provenance |
| **PyPI** | Python | Centralized | Multiple file types |
| **Go Proxy** | Go | Federated, proxies | Immutable, checksum DB |
| **GitHub Packages** | Multi | Federated | Org-scoped |

### 4.2 crates.io Architecture

```
crates.io Architecture:

  Index Repository (Git):
    crates.io-index/
    ├── config.json         # Registry metadata
    ├── 1/                   # Single-char package names
    │   └── a                # Package "a" versions
    ├── 2/                   # Two-char names
    │   └── ab               # Package "ab" versions
    ├── 3/s/                 # Three-char names
    │   └── ser              # Package "ser" versions
    └── se/rd/               # Four+ char names
        └── serde            # Package "serde" versions

  Package Metadata (JSON per package):
    {
      "name": "serde",
      "vers": "1.0.193",
      "deps": [...],
      "features": {...},
      "cksum": "sha256:..."
    }

  Crate Storage (S3-compatible):
    /crates/serde/serde-1.0.193.crate (tarball)
```

**Benefits:**
- Git clone for offline index
- Incremental updates via git fetch
- CDN-friendly crate storage
- Immutable once published

### 4.3 npm Registry Features

- **Scoped packages**: `@organization/package`
- **Provenance**: Sigstore attestations (2025+)
- **Deprecation**: Mark versions deprecated
- **Access control**: Public, restricted
- **Audit**: `npm audit` for vulnerabilities

### 4.4 Go Module Proxy Design

```
Go Module Proxy Protocol:

  GET /{module}/@v/list        → List versions
  GET /{module}/@v/{version}.info  → Version metadata
  GET /{module}/@v/{version}.mod   → go.mod file
  GET /{module}/@v/{version}.zip   → Source archive

  Checksum Database:
    sum.golang.org provides signed checksums
    Ensures global consistency of module content
```

**Key Innovation**: Federated proxies with single checksum database

### 4.5 Aria Registry Design (aria.pkg)

```
aria.pkg Registry Design:

  Index (Git-based, like crates.io):
    aria-index/
    ├── config.toml           # Registry configuration
    ├── ar/ia/
    │   └── aria-http         # Package metadata
    └── js/on/
        └── json              # Package metadata

  API Endpoints:
    GET  /api/v1/packages/{name}           # Package info
    GET  /api/v1/packages/{name}/{version} # Version info
    GET  /api/v1/packages/{name}/{version}/download  # Tarball
    POST /api/v1/packages                  # Publish (auth required)

  Package Metadata:
    {
      "name": "http",
      "version": "2.3.1",
      "effects": ["IO", "Async"],         # Effect declarations!
      "effect_handlers": ["Async"],       # Provides handlers for
      "checksum": "sha256:...",
      "dependencies": [...],
      "features": {...},
      "provenance": {...}                 # Sigstore attestation
    }

  Federation Support:
    - Primary: aria.pkg (official)
    - Private: company.aria.pkg
    - Mirror: proxy for air-gapped environments
```

---

## 5. Workspace Support for Monorepos

### 5.1 Workspace Feature Comparison

| Feature | Cargo | pnpm | uv (Python) | npm |
|---------|-------|------|-------------|-----|
| Root config | Cargo.toml | pnpm-workspace.yaml | pyproject.toml | package.json |
| Glob patterns | Yes | Yes | Yes | Yes |
| Shared deps | workspace.dependencies | Hoisting | workspace | Hoisting |
| Cross-references | `path = "../lib"` | `workspace:*` | Path deps | file: |
| Selective commands | `-p package` | `--filter` | `--package` | `-w` |
| Publish individual | Yes | Yes | Yes | Yes |

### 5.2 Cargo Workspaces (Best Practice)

```toml
# Root Cargo.toml
[workspace]
members = [
    "core",
    "cli",
    "web",
    "plugins/*"
]
exclude = ["experiments"]
resolver = "2"

[workspace.package]
version = "1.0.0"
authors = ["Team <team@example.com>"]
edition = "2021"
license = "MIT"

[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }
thiserror = "1.0"

# Member inherits from workspace
# crates/core/Cargo.toml
[package]
name = "my-core"
version.workspace = true
edition.workspace = true

[dependencies]
serde.workspace = true
my-utils = { path = "../utils" }
```

### 5.3 pnpm Workspaces

```yaml
# pnpm-workspace.yaml
packages:
  - 'packages/*'
  - 'apps/*'
  - '!**/test/**'
```

```json
// packages/shared/package.json
{
  "name": "@myorg/shared",
  "version": "1.0.0"
}

// apps/web/package.json
{
  "name": "@myorg/web",
  "dependencies": {
    "@myorg/shared": "workspace:*"
  }
}
```

### 5.4 Aria Workspace Design

```toml
# Root aria.toml
[workspace]
members = [
    "core",
    "stdlib",
    "cli",
    "plugins/*"
]
exclude = ["experiments"]

[workspace.package]
version = "1.0.0"
authors = ["Aria Team <team@aria-lang.org>"]
license = "MIT"
aria-version = ">=1.0"

[workspace.dependencies]
# Shared dependency versions
json = "1.5"
http = { version = "2.0", features = ["server"] }
testing = "1.0"

[workspace.effects]
# Effect allowlist for workspace
allowed = ["IO", "Async", "Console", "Exception"]
denied = ["Unsafe"]  # No packages can use Unsafe effect
```

```toml
# core/aria.toml
[package]
name = "aria-core"
version.workspace = true
effects = []  # Pure package

[dependencies]
# No external deps for core
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

---

## 6. Feature Flags and Optional Dependencies

### 6.1 Feature Systems Comparison

| System | Language | Compile-time | Runtime | Additive |
|--------|----------|--------------|---------|----------|
| Cargo features | Rust | Yes | No | Yes |
| package.json optionalDeps | JS | No | Yes | N/A |
| pyproject.toml extras | Python | No | Yes | N/A |
| Go build tags | Go | Yes | No | N/A |
| Zig comptime | Zig | Yes | No | N/A |

### 6.2 Cargo Features (Best Practice)

```toml
[features]
# Default features
default = ["std", "derive"]

# Feature flags
std = []                           # Enable std library usage
derive = ["serde/derive"]          # Enable dependency feature
async = ["tokio", "async-trait"]   # Enable optional deps
full = ["std", "derive", "async"]  # Convenience combo

# Weak dependencies (Rust 1.60+)
serde = { version = "1.0", optional = true }
tokio = { version = "1.0", optional = true }

# Feature-specific code
# In lib.rs:
# #[cfg(feature = "async")]
# pub mod async_client;
```

**Additive Property**: Features can only ADD capabilities, never remove.

### 6.3 Python Optional Dependencies

```toml
[project.optional-dependencies]
dev = [
    "pytest>=7.0",
    "mypy",
    "ruff",
]
docs = [
    "sphinx",
    "furo",
]
all = [
    "my-package[dev,docs]"
]
```

Usage: `pip install my-package[dev]`

### 6.4 Aria Feature System Design

```toml
# aria.toml
[package]
name = "http-client"
version = "2.0.0"
effects = ["IO"]  # Base effects

[features]
default = ["tls"]

# Simple flags
tls = []
http2 = []
compression = ["dep:zlib"]

# Effect-modifying features
async = {
    dependencies = ["aria-async"],
    effects = ["Async"]  # Adds Async effect when enabled
}

# Platform-specific
native-tls = {
    dependencies = ["native-tls"],
    target = "native"
}
rustls = {
    dependencies = ["rustls"],
    target = "any"
}

[dependencies]
url = "3.0"
zlib = { version = "1.0", optional = true }
aria-async = { version = "1.0", optional = true }
native-tls = { version = "1.0", optional = true }
rustls = { version = "1.0", optional = true }

# In Aria code:
# @when(feature = "async")
# fn fetch_async(url: String) -> Future[Response] !Async
#   ...
# end
```

---

## 7. Security and Supply Chain

### 7.1 Supply Chain Attack Vectors

| Attack | Description | Example |
|--------|-------------|---------|
| **Typosquatting** | Similar names | `lodash` vs `lodahs` |
| **Dependency Confusion** | Internal/external name clash | 2021 attacks |
| **Account Compromise** | Maintainer credentials stolen | event-stream 2018 |
| **Malicious Updates** | Legit package goes rogue | ua-parser-js 2021 |
| **Worm** | Self-replicating via postinstall | Shai-Hulud 2025 |

### 7.2 Security Measures by Ecosystem

| Measure | crates.io | npm | PyPI |
|---------|-----------|-----|------|
| 2FA | Optional | Default (2025+) | Required for critical |
| Sigstore/Provenance | Limited | Yes (2025) | Yes (trusted publishing) |
| Name squatting prevention | Yes (reserved) | Scopes | No |
| Publish rate limiting | Yes | Yes | Yes |
| Content scanning | Basic | Yes | Basic |
| Yank/Deprecation | Yank | Deprecate | Yank |

### 7.3 Sigstore and Provenance

```
Sigstore Provenance Flow:

  1. Developer pushes to GitHub
  2. CI/CD (GitHub Actions) builds package
  3. Build system signs with ephemeral key (Fulcio)
  4. Signature logged to transparency log (Rekor)
  5. Registry stores attestation with package
  6. Consumers can verify:
     - WHO built it (CI identity, not maintainer key)
     - WHERE it was built (GitHub repo)
     - WHAT went in (source commit hash)
```

**npm provenance example:**
```json
{
  "predicateType": "https://slsa.dev/provenance/v0.2",
  "predicate": {
    "buildType": "https://github.com/npm/cli/gha/v2",
    "builder": { "id": "https://github.com/actions/runner" },
    "invocation": {
      "configSource": {
        "uri": "git+https://github.com/user/package",
        "digest": { "sha1": "abc123..." },
        "entryPoint": ".github/workflows/publish.yml"
      }
    }
  }
}
```

### 7.4 Aria Security Design

```toml
# aria.toml security configuration
[package]
name = "my-package"
version = "1.0.0"

[publish]
# Require provenance for publishing
provenance = true
# Allowed CI providers for provenance
allowed-builders = ["github-actions", "gitlab-ci"]

[security]
# Effect sandboxing
max-effects = ["IO", "Console"]  # Limit dependency effects
deny-effects = ["Unsafe"]        # Block specific effects

# Dependency verification
require-checksums = true
require-provenance = "optional"  # "required" | "optional" | "none"

# Registry trust
trusted-registries = ["aria.pkg"]
allow-git-deps = false           # For locked-down environments
```

**aria.lock with provenance:**
```toml
[[package]]
name = "http"
version = "2.3.1"
source = "registry+https://aria.pkg"
checksum = "sha256:abc123..."
provenance = {
    builder = "github-actions",
    repository = "https://github.com/aria-pkg/http",
    commit = "abc123def456",
    transparency-log = "https://rekor.sigstore.dev/...",
    verified = true
}
```

### 7.5 Effect-Based Security

Aria's effect system provides unique security capabilities:

```aria
# Packages declare their effects in manifest
# aria.toml
[package]
effects = ["IO", "Console"]

# Consumer can verify effects are acceptable
# In application aria.toml
[dependencies.http]
version = "2.0"
max-effects = ["IO", "Async"]  # Fail if http uses more effects

# The resolver enforces effect compatibility:
# - http declares ["IO", "Async"]
# - Consumer allows ["IO", "Async"]
# - OK!

# If http used Console effect:
# Resolution error: http 2.0 requires effect "Console"
#   which is not allowed by consumer's max-effects
```

---

## 8. Private Registries and Git Dependencies

### 8.1 Private Registry Options

| Solution | Languages | Self-hosted | Cloud | Features |
|----------|-----------|-------------|-------|----------|
| **Verdaccio** | JS/npm | Yes | No | Proxy, cache |
| **Artifactory** | Multi | Yes | Yes | Enterprise |
| **GitHub Packages** | Multi | No | Yes | Org-scoped |
| **GitLab Package Registry** | Multi | Yes | Yes | CI integration |
| **crates.io alt registry** | Rust | Yes | - | Git index |

### 8.2 Cargo Alternative Registries

```toml
# .cargo/config.toml
[registries]
my-company = { index = "https://github.com/company/crate-index" }

# Cargo.toml
[dependencies]
internal-lib = { version = "1.0", registry = "my-company" }
```

### 8.3 Git Dependencies

**Cargo:**
```toml
[dependencies]
# From default branch
experimental = { git = "https://github.com/user/experimental" }

# Specific branch
experimental = { git = "...", branch = "develop" }

# Specific tag
experimental = { git = "...", tag = "v1.0.0" }

# Specific commit
experimental = { git = "...", rev = "abc123" }
```

**npm:**
```json
{
  "dependencies": {
    "package": "git+https://github.com/user/repo.git#v1.0.0"
  }
}
```

### 8.4 Aria Private Registry Design

```toml
# ~/.aria/config.toml (global config)
[registries]
# Official (default)
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
# Token storage (or use environment variables)
internal = { token-cmd = "vault read -field=token secret/aria-registry" }
```

```toml
# aria.toml
[dependencies]
# From official registry (default)
json = "1.5"

# From specific registry
internal-lib = { version = "2.0", registry = "internal" }

# From git
experimental = {
    git = "https://github.com/user/experimental",
    branch = "main",
    effects = ["IO"]  # Must declare expected effects
}

# From local path
local-dev = { path = "../local-dev" }
```

---

## 9. Effect System Integration

### 9.1 Effects in Package Metadata

Aria's effect system creates unique opportunities for package management:

```toml
# aria.toml
[package]
name = "database-client"
version = "1.0.0"

# Effect declarations (required for publishing)
effects = ["IO", "Async", "Exception[DbError]"]

# Effect handlers provided by this package
provides-handlers = ["Database"]

# Required effect handlers (must be provided by consumer)
requires-handlers = ["Async"]
```

### 9.2 Effect Compatibility Resolution

```
Effect Compatibility Algorithm:

1. For each package P in resolution:
   a) Read P's declared effects
   b) Read P's dependencies' effects
   c) Verify: deps_effects ⊆ P.effects (transitively)

2. For root package R:
   a) Collect all transitive effects
   b) Verify: effects ⊆ R.allowed_effects (if specified)
   c) Warn if unexpected effects detected

3. Effect handler resolution:
   a) Package declares requires-handlers
   b) Consumer must provide handler (or transitive dep)
   c) Error if handler not available
```

**Resolution Error Example:**
```
Effect Resolution Error:

  database-client 1.0.0 requires effect handler "Async"
  but no package in the dependency graph provides it.

  Suggestions:
    1. Add aria-async as a dependency
    2. Implement Async handler in your application
    3. Use database-client-sync 1.0.0 (no Async requirement)
```

### 9.3 Effect-Aware Dependency Selection

```toml
# aria.toml - consumer perspective
[dependencies]
# Constrain by effects
http = { version = "^2.0", max-effects = ["IO"] }

# This would fail:
# http 2.0 declares effects = ["IO", "Async"]
# Consumer allows only ["IO"]
# Resolution fails with clear error

# Solution: use sync version
http-sync = { version = "^2.0" }  # effects = ["IO"] only
```

### 9.4 Effect Inheritance in Workspaces

```toml
# Root aria.toml
[workspace]
members = ["core", "web", "cli"]

[workspace.effects]
# Packages can only use these effects
allowed = ["IO", "Async", "Console", "Exception"]

# Packages MUST NOT use these
denied = ["Unsafe", "FFI"]

# Effect inheritance rules
inheritance = "strict"  # Members cannot exceed workspace effects
```

```toml
# core/aria.toml
[package]
name = "core"
effects = []  # Pure - OK

# web/aria.toml
[package]
name = "web"
effects = ["IO", "Async"]  # OK - subset of allowed

# This would be rejected:
# [package]
# effects = ["IO", "FFI"]  # Error: FFI is denied
```

---

## 10. Aria Package Manager Design

### 10.1 CLI Tool: `aria`

The package manager is integrated into the main `aria` CLI:

```bash
# Project creation
aria new my-project            # Create new project
aria new my-lib --lib          # Create library
aria init                      # Initialize in existing directory

# Dependency management
aria add json                  # Add latest json package
aria add http@2.0              # Add specific version
aria add http --features=async # Add with features
aria add --dev testing         # Add dev dependency
aria add --git https://...     # Add from git
aria remove json               # Remove dependency
aria update                    # Update all dependencies
aria update json               # Update specific package

# Building and testing
aria build                     # Build project
aria build --release           # Release build
aria build --target=wasm       # Cross-compile
aria test                      # Run tests
aria test --coverage           # With coverage
aria check                     # Type check only
aria doc                       # Generate documentation

# Package management
aria publish                   # Publish to registry
aria publish --dry-run         # Check before publishing
aria yank 1.0.0                # Yank a version
aria owner add user            # Add package owner

# Registry interaction
aria search http               # Search packages
aria info http                 # Package details
aria login                     # Authenticate with registry

# Workspace commands
aria workspace list            # List workspace members
aria build --workspace         # Build all workspace members
aria -p core build             # Build specific member

# Effect inspection
aria effects                   # Show project's effects
aria effects --deps            # Show all dependency effects
aria audit                     # Security + effect audit
```

### 10.2 Manifest Format: `aria.toml`

```toml
#======================================================================
# aria.toml - Aria Package Manifest
#======================================================================

[package]
# Required fields
name = "my-awesome-project"
version = "1.0.0"

# Required for publishing
description = "An awesome Aria project"
license = "MIT OR Apache-2.0"
authors = ["Developer <dev@example.com>"]
repository = "https://github.com/user/my-awesome-project"

# Optional metadata
readme = "README.md"
keywords = ["web", "api", "async"]
categories = ["web-programming", "network-programming"]
homepage = "https://my-project.dev"
documentation = "https://docs.my-project.dev"

# Aria version requirement
aria-version = ">=1.0"

# Effect declarations (inferred if not specified, required for publishing)
effects = ["IO", "Async", "Exception[AppError]"]

# Effect handlers provided
provides-handlers = ["Database", "Cache"]

#======================================================================
# Dependencies
#======================================================================

[dependencies]
# Simple version (from default registry)
json = "1.5"

# With features
http = { version = "2.0", features = ["server", "tls"] }

# Optional dependency (becomes a feature)
compression = { version = "1.0", optional = true }

# From specific registry
internal = { version = "1.0", registry = "company" }

# From git
experimental = {
    git = "https://github.com/user/experimental",
    branch = "main"
}

# From local path
local-lib = { path = "../local-lib" }

# Effect-constrained
limited-io = { version = "1.0", max-effects = ["IO"] }

[dev-dependencies]
testing = "1.0"
benchmark = "0.5"

[build-dependencies]
codegen = "1.0"

#======================================================================
# Features
#======================================================================

[features]
default = ["std"]

# Simple flags
std = []
no_std = []

# Dependency activation
compression = ["dep:compression"]

# Effect-modifying features
async = {
    dependencies = ["aria-async"],
    effects = ["Async"]
}

# Platform-specific
native = {
    dependencies = ["native-bindings"],
    target = "native"
}
wasm = {
    dependencies = ["wasm-bindings"],
    target = "wasm32"
}

#======================================================================
# Build configuration
#======================================================================

[targets]
default = "native"

[targets.native]
features = ["native"]

[targets.wasm]
target = "wasm32-unknown-unknown"
features = ["wasm"]

# Binary targets
[[bin]]
name = "my-app"
path = "src/main.aria"

[[bin]]
name = "my-tool"
path = "src/bin/tool.aria"
required-features = ["cli"]

# Example targets (compiled but not published)
[[example]]
name = "basic-usage"
path = "examples/basic.aria"

# Benchmark targets
[[bench]]
name = "performance"
path = "benches/perf.aria"

#======================================================================
# Profiles
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
# Workspace (only in root)
#======================================================================

# [workspace]
# members = ["core", "cli", "plugins/*"]
# [workspace.package]
# version = "1.0.0"
# [workspace.dependencies]
# json = "1.5"
# [workspace.effects]
# allowed = ["IO", "Async", "Console"]

#======================================================================
# Publishing
#======================================================================

[publish]
# Require provenance attestation
provenance = true

# Allowed CI systems for provenance
allowed-builders = ["github-actions"]

# Files to include (defaults to src/ + aria.toml)
include = ["src/", "README.md", "LICENSE"]

# Files to exclude
exclude = ["tests/fixtures/large-file.bin"]
```

### 10.3 Lock File Format: `aria.lock`

```toml
# aria.lock
# DO NOT EDIT - Generated by aria 1.0.0
#
# This file ensures reproducible builds by locking exact versions.
# Commit this file to version control for applications.
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
checksum = "sha256:abc123def456..."
effects = []
provenance = {
    builder = "github-actions",
    repository = "https://github.com/aria-pkg/json",
    commit = "abc123",
    log = "https://rekor.sigstore.dev/api/v1/log/entries/..."
}

[[package]]
name = "http"
version = "2.3.1"
source = "registry+https://aria.pkg"
checksum = "sha256:def456ghi789..."
effects = ["IO", "Async"]
dependencies = [
    "url 3.0.0",
    "tls 1.5.0",
]
features = ["server", "tls"]
provenance = {
    builder = "github-actions",
    repository = "https://github.com/aria-pkg/http",
    commit = "def456",
    log = "https://rekor.sigstore.dev/api/v1/log/entries/..."
}

[[package]]
name = "url"
version = "3.0.0"
source = "registry+https://aria.pkg"
checksum = "sha256:ghi789jkl012..."
effects = []

[[package]]
name = "tls"
version = "1.5.0"
source = "registry+https://aria.pkg"
checksum = "sha256:jkl012mno345..."
effects = ["IO"]

[[package]]
name = "experimental"
version = "0.5.0"
source = "git+https://github.com/user/experimental?rev=abc123def456"
checksum = "sha256:mno345pqr678..."
effects = ["IO", "Console"]

# Platform-specific packages
[[package]]
name = "native-bindings"
version = "1.0.0"
source = "registry+https://aria.pkg"
checksum = "sha256:..."
effects = ["FFI"]
target = "cfg(not(target_arch = \"wasm32\"))"
```

### 10.4 Registry API Design

```yaml
# aria.pkg Registry API (OpenAPI-style)

/api/v1:
  /packages:
    GET:
      description: "List packages"
      parameters:
        - q: "search query"
        - page: "page number"
        - per_page: "results per page"
        - sort: "downloads | recent | relevance"
      response: PackageList

    POST:
      description: "Publish package"
      auth: required
      body: PackageUpload
      response: PublishResult

  /packages/{name}:
    GET:
      description: "Get package info"
      response: PackageInfo

  /packages/{name}/versions:
    GET:
      description: "List versions"
      response: VersionList

  /packages/{name}/{version}:
    GET:
      description: "Get version info"
      response: VersionInfo

  /packages/{name}/{version}/download:
    GET:
      description: "Download package archive"
      response: binary (application/gzip)

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

  /owners/{name}:
    GET:
      description: "List package owners"
      response: OwnerList
    POST:
      description: "Add owner"
      auth: required (owner)

  /search:
    GET:
      description: "Search packages"
      parameters:
        - q: "query"
        - effects: "filter by effects (e.g., effects=IO,Async)"
        - features: "filter by features"
      response: SearchResults

# Index endpoint (for fast resolution)
/index:
  GET /{prefix}/{name}:
    description: "Get package index entry"
    response: IndexEntry (JSON lines)
```

### 10.5 Effect Resolution Algorithm

```aria
# Pseudocode for effect-aware resolution

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

---

## 11. Comparison Summary

### 11.1 Feature Comparison Matrix

| Feature | Cargo | npm | pip/uv | Aria (Proposed) |
|---------|-------|-----|--------|-----------------|
| **Manifest Format** | TOML | JSON | TOML | TOML |
| **Resolver** | PubGrub | SAT | PubGrub | PubGrub + Effects |
| **Lock File** | TOML | JSON | TOML | TOML |
| **Workspaces** | Yes | pnpm | uv | Yes |
| **Features** | Additive | N/A | Extras | Additive + Effects |
| **Provenance** | Limited | Sigstore | Trusted Publishing | Sigstore |
| **Effect Tracking** | N/A | N/A | N/A | First-class |
| **Private Registries** | Yes | Yes | Yes | Yes |
| **Build Scripts** | Rust | JS | Python | Aria |

### 11.2 Resolution Algorithm Comparison

| Feature | Cargo PubGrub | uv PubGrub | Aria PubGrub |
|---------|---------------|------------|--------------|
| Core algorithm | CDCL | CDCL | CDCL |
| Error messages | Excellent | Excellent | Excellent |
| Effect awareness | No | No | Yes |
| Effect constraints | N/A | N/A | max-effects |
| Handler resolution | N/A | N/A | Yes |

### 11.3 Security Comparison

| Feature | crates.io | npm | PyPI | aria.pkg (Proposed) |
|---------|-----------|-----|------|---------------------|
| 2FA | Optional | Default | Critical pkgs | Required |
| Provenance | Limited | Sigstore | Trusted Pub | Sigstore |
| Effect sandboxing | N/A | N/A | N/A | Yes |
| Effect auditing | N/A | N/A | N/A | Built-in |

---

## 12. Recommendations Summary

### 12.1 Naming

| Component | Recommended Name | Alternatives |
|-----------|------------------|--------------|
| CLI Tool | `aria` (unified) | - |
| Manifest | `aria.toml` | - |
| Lock File | `aria.lock` | - |
| Registry | `aria.pkg` | `aria.dev`, `packages.aria-lang.org` |
| Index | `aria-index` | - |

### 12.2 Key Design Decisions

1. **Unified CLI**: Package management integrated into `aria` command (like Cargo)
2. **TOML Manifest**: Human-readable, comments supported, proven format
3. **PubGrub Resolution**: Fast, excellent errors, extensible for effects
4. **Effect Declarations**: Required for publishing, optional locally (inferred)
5. **Sigstore Provenance**: Mandatory for official registry, optional for private
6. **Workspace First**: Native monorepo support with shared settings
7. **Feature Additivity**: Features only add capabilities (Cargo model)

### 12.3 Unique Aria Innovations

1. **Effect-Aware Resolution**: Verify effect compatibility at resolution time
2. **Effect Constraints**: `max-effects` to limit dependency effects
3. **Handler Resolution**: Ensure required effect handlers are available
4. **Effect Auditing**: `aria audit` shows full effect graph
5. **Effect in Lock File**: Track effects in aria.lock for reproducibility

### 12.4 Implementation Priority

| Priority | Component | Effort | Impact |
|----------|-----------|--------|--------|
| P0 | aria.toml parser | Medium | Critical |
| P0 | PubGrub resolver | High | Critical |
| P0 | aria.lock generation | Medium | Critical |
| P1 | Effect resolution | High | High |
| P1 | CLI commands | Medium | High |
| P1 | Registry client | Medium | High |
| P2 | Workspace support | Medium | Medium |
| P2 | Private registries | Medium | Medium |
| P3 | Provenance verification | Medium | Medium |
| P3 | Build scripts | High | Low |

---

## 13. Open Questions

1. **Effect Inference vs Declaration**: Should effects be inferred for local development but required for publishing?

2. **Effect Versioning**: How do we handle breaking changes in effect declarations?

3. **Cross-Platform Effects**: Some effects (FFI, native IO) are platform-specific. How to handle?

4. **Effect Handler Distribution**: Should effect handlers be separate packages or bundled?

5. **Registry Federation**: Full federation (like Go) or primary with mirrors (like npm)?

6. **Effect Compatibility Semver**: Should effect changes be semver major/minor/patch?

---

## 14. Resources and References

### Academic Papers
- Leijen (2017): "Type Directed Compilation of Row-Typed Algebraic Effects"
- Stucki et al. (2021): "Polymorphic Effect Inference"
- [Package Management Papers](https://nesbitt.io/2025/11/13/package-management-papers.html)

### Documentation
- [Cargo Book](https://doc.rust-lang.org/cargo/)
- [npm Documentation](https://docs.npmjs.com/)
- [Python Packaging Guide](https://packaging.python.org/)
- [PubGrub Algorithm](https://nex3.medium.com/pubgrub-2fb6470504f)

### Implementations
- [pubgrub-rs](https://github.com/pubgrub-rs/pubgrub)
- [uv Resolver](https://docs.astral.sh/uv/)
- [Verdaccio](https://www.verdaccio.org/)

### Security
- [Sigstore](https://www.sigstore.dev/)
- [npm Provenance](https://docs.npmjs.com/generating-provenance-statements)
- [SLSA Framework](https://slsa.dev/)

---

## Appendix A: aria.toml Complete Schema

```toml
# Complete aria.toml schema reference

[package]
name = "string"                    # Required: package name
version = "semver"                 # Required: semantic version
description = "string"             # Required for publish
authors = ["string"]               # Required for publish
license = "SPDX"                   # Required for publish
repository = "url"                 # Optional
homepage = "url"                   # Optional
documentation = "url"              # Optional
readme = "path"                    # Optional
keywords = ["string"]              # Optional (max 5)
categories = ["string"]            # Optional
aria-version = "version-req"       # Optional: minimum Aria version
effects = ["Effect"]               # Optional (inferred if not set)
provides-handlers = ["Handler"]    # Optional
build = "path"                     # Optional: build script

[dependencies]
# Simple: name = "version"
# Full: name = { version, features, optional, registry, git, branch, tag, rev, path, max-effects, target }

[dev-dependencies]
# Same as [dependencies]

[build-dependencies]
# Same as [dependencies]

[features]
# name = []
# name = ["dep:name", "other-feature"]
# name = { dependencies = [], effects = [], target = "string" }

[[bin]]
name = "string"                    # Binary name
path = "path"                      # Source file
required-features = ["feature"]    # Optional

[[example]]
name = "string"
path = "path"

[[test]]
name = "string"
path = "path"

[[bench]]
name = "string"
path = "path"

[targets]
default = "string"                 # Default target

[targets.name]
target = "triple"                  # Target triple
features = ["feature"]             # Auto-enabled features

[profile.name]
opt-level = 0-3                    # Optimization level
debug = bool                       # Debug info
lto = bool                         # Link-time optimization
contracts = "disabled|runtime|compile"  # Contract checking

[workspace]
members = ["glob"]                 # Workspace members
exclude = ["glob"]                 # Excluded paths

[workspace.package]
# Inherited package fields

[workspace.dependencies]
# Shared dependency versions

[workspace.effects]
allowed = ["Effect"]               # Allowed effects
denied = ["Effect"]                # Denied effects

[publish]
provenance = bool                  # Require provenance
allowed-builders = ["string"]      # Allowed CI systems
include = ["glob"]                 # Include patterns
exclude = ["glob"]                 # Exclude patterns
```

---

## Appendix B: CLI Command Reference

```bash
# Project Management
aria new <name> [--lib] [--template <t>]
aria init
aria clean

# Dependencies
aria add <pkg>[@version] [--dev] [--features <f>] [--git <url>]
aria remove <pkg>
aria update [pkg]
aria tree [--depth <n>]

# Building
aria build [--release] [--target <t>]
aria check
aria test [--coverage] [--doc]
aria bench
aria doc [--open]
aria run [bin] [-- args]

# Package Registry
aria search <query>
aria info <pkg>
aria publish [--dry-run]
aria yank <version>
aria login
aria logout
aria owner <add|remove|list> [user]

# Effects
aria effects [--deps]
aria audit [--effects] [--security]

# Workspace
aria workspace <list|graph>
aria -p <pkg> <command>

# Misc
aria fmt [--check]
aria lint
aria fix
aria version
aria help [command]
```

---

**Document Status**: Completed
**Next Steps**: Implementation planning for aria package manager
**Owner**: Aria Tooling Team
**Reviewers**: ATLAS (Research), Core Compiler Team
