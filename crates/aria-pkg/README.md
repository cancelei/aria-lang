# aria-pkg

The official package manager for the Aria programming language.

## Features

- **Project initialization**: Create new Aria projects with a single command
- **Dependency management**: Add, remove, and install dependencies
- **Version resolution**: Automatic dependency resolution with semver support
- **Lock file generation**: Deterministic builds with Aria.lock
- **Build system**: Integrated build and run commands
- **Registry support**: Placeholder for future package registry integration

## Installation

Build from the Aria workspace:

```bash
cargo build -p aria-pkg --release
```

The binary will be located at `target/release/aria-pkg`.

## Usage

### Initialize a new project

```bash
aria-pkg init my-project
cd my-project
```

This creates:
- `Aria.toml` - Package manifest
- `src/main.aria` - Entry point
- `.gitignore` - Git ignore file

### Add dependencies

```bash
# Add from registry with version
aria-pkg add some-lib --version "1.0.0"

# Add from git repository
aria-pkg add some-lib --git "https://github.com/user/some-lib"

# Add from local path
aria-pkg add some-lib --path "../some-lib"
```

### Remove dependencies

```bash
aria-pkg remove some-lib
```

### Install dependencies

```bash
aria-pkg install
```

This resolves dependencies and creates `Aria.lock`.

### Build the project

```bash
# Debug build
aria-pkg build

# Release build
aria-pkg build --release
```

### Run the project

```bash
# Debug run
aria-pkg run

# Release run
aria-pkg run --release

# Pass arguments
aria-pkg run -- --arg1 value1
```

### Publish a package

```bash
# Dry run
aria-pkg publish --dry-run

# Actual publish (not yet implemented)
aria-pkg publish
```

## Manifest Format (Aria.toml)

```toml
[package]
name = "my-project"
version = "0.1.0"
authors = ["Your Name <you@example.com>"]
description = "A description of your project"
license = "MIT OR Apache-2.0"
repository = "https://github.com/user/my-project"

[dependencies]
# Simple version constraint
some-lib = "1.0.0"

# Detailed specification
other-lib = { version = "2.1.0", git = "https://github.com/user/other-lib" }

# Local path
local-lib = { version = "*", path = "../local-lib" }
```

## Lock File Format (Aria.lock)

```toml
version = 1

[[packages]]
name = "some-lib"
version = "1.0.0"
source = "registry+https://pkg.aria-lang.org"
dependencies = ["dep1", "dep2"]

[[packages]]
name = "other-lib"
version = "2.1.0"
source = "git+https://github.com/user/other-lib"
dependencies = []
```

## Version Requirements

aria-pkg uses semantic versioning (semver) for version resolution:

- `1.0.0` - Exact version
- `^1.0.0` - Compatible with 1.0.0 (>=1.0.0, <2.0.0)
- `~1.2.3` - Reasonably close to 1.2.3 (>=1.2.3, <1.3.0)
- `>=1.0.0` - Greater than or equal to 1.0.0
- `*` - Any version

## Project Structure

```
crates/aria-pkg/
├── Cargo.toml           # Package configuration
├── README.md            # This file
└── src/
    ├── main.rs          # CLI implementation and commands
    ├── manifest.rs      # Aria.toml parsing and serialization
    └── resolver.rs      # Dependency resolution and lock file
```

## Architecture

### Components

1. **CLI** (`main.rs`): Command-line interface using clap
2. **Manifest** (`manifest.rs`): Parses and manages Aria.toml
3. **Resolver** (`resolver.rs`): Resolves dependencies and generates lock files

### Dependency Resolution

The resolver performs the following steps:

1. Parse `Aria.toml` to extract dependencies
2. For each dependency, determine the version constraint
3. Select compatible versions (currently uses simple strategy)
4. Generate a lock file with exact versions
5. Create dependency graph for topological sorting

### Future Enhancements

- [ ] Implement package registry client
- [ ] Add support for checksums and verification
- [ ] Implement actual package downloading
- [ ] Support for build scripts
- [ ] Workspace support for monorepos
- [ ] Cache management
- [ ] Parallel dependency downloads
- [ ] Better conflict resolution
- [ ] Support for optional dependencies
- [ ] Feature flags
- [ ] Build profiles

## Testing

Run tests:

```bash
cargo test -p aria-pkg
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
