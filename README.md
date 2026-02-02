# Aria Lang

<div align="center">

**The Future of Safe, Expressive Programming**

[![Build Status](https://github.com/cancelei/aria-lang/workflows/CI/badge.svg)](https://github.com/cancelei/aria-lang/actions)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Discord](https://img.shields.io/discord/placeholder)](https://discord.gg/aria-lang)

*Combining Ruby's expressiveness with Rust's safety*

[Playground](https://aria-lang.dev) • [Documentation](./docs) • [Contributing](./community/CONTRIBUTING.md) • [Discord](https://discord.gg/aria-lang)

</div>

---

## What is Aria?

Aria is a next-generation programming language designed to make safe, high-performance code accessible and delightful to write. We believe you shouldn't have to choose between safety and expressiveness.

### Core Philosophy

- **Expressive**: Ruby-like syntax that reads like prose
- **Safe**: Rust-level memory safety without manual annotations
- **Fast**: Compiled to native code via LLVM/Cranelift
- **Agent-First**: Built for the age of AI and autonomous systems
- **Contract-Driven**: Design by Contract as a first-class feature

## Quick Start

### Installation

```bash
# Coming soon - currently in active development
curl -sSf https://get.aria-lang.dev | sh
```

### Hello World

```aria
fn main()
  print("Hello, Aria!")
end
```

### With Contracts

```aria
fn divide(a: Int, b: Int) -> Float
  requires b != 0 : "Division by zero"
  ensures result.finite?

  return a.to_f / b.to_f
end
```

### Concurrent Agents

```aria
fn main()
  let results = Channel<Int>.new

  spawn {
    results.send(compute_task_1())
  }

  spawn {
    results.send(compute_task_2())
  }

  print("Result 1: #{results.receive}")
  print("Result 2: #{results.receive}")
end
```

## Features

### 🎯 Design by Contract
Built-in `requires`, `ensures`, and `invariant` keywords for formal specifications.

### 🧠 Type Inference
Write less boilerplate with powerful Hindley-Milner-based type inference.

### 🔒 Memory Safety
Hybrid ownership model: inferred by default, explicit when needed.

### ⚡ Performance
Compiled to native code with zero-cost abstractions.

### 🤖 Agent-First
Designed for multi-agent systems and autonomous code.

### 🛠️ Great Tooling
First-class IDE support via LSP, excellent error messages, fast compilation.

## Project Structure

```
aria-lang/
├── aria-web/              # Cyberpunk-themed online playground
│   ├── frontend/          # Web interface
│   ├── playground/        # WASM-powered code editor
│   └── assets/themes/     # Cyberpunk theme files
│
├── contests/              # Community contests
│   ├── 01-agent-framework/   # Any language, agent-first
│   └── 02-aria-vision/       # Building Aria's vision
│
├── community/             # Community resources
│   ├── CONTRIBUTING.md    # Contribution guide
│   ├── CODE_OF_CONDUCT.md # Community standards
│   └── rfcs/              # Request for Comments
│
├── crates/                # Rust crates
│   ├── aria-compiler/     # Main compiler
│   ├── aria-runtime/      # Runtime library
│   ├── aria-stdlib/       # Standard library
│   └── aria-lsp/          # Language server
│
├── docs/                  # Documentation
│   ├── designs/           # Design documents
│   └── tutorials/         # Learning resources
│
├── examples/              # Example programs
│   └── showcases/         # Real-world applications
│
├── plugins/               # Editor plugins
│   ├── vscode/            # VSCode extension
│   ├── neovim/            # Neovim plugin
│   └── jetbrains/         # IntelliJ plugin
│
├── stdlib/                # Standard library implementations
├── PRD-v2.md             # Product requirements document
├── GRAMMAR.md            # Language grammar specification
└── ECOSYSTEM.md          # Ecosystem overview
```

## Development Status

Aria is in active development. See our [PRD](./PRD-v2.md) for the full vision and roadmap.

### Current Phase: Foundation (Phase 1-2)

- [x] Grammar specification
- [x] Project structure
- [x] Core type system design
- [ ] Parser implementation
- [ ] Type inference engine
- [ ] Basic compilation pipeline
- [ ] Ownership analysis

### Upcoming

- [ ] Standard library
- [ ] LSP server
- [ ] Package manager
- [ ] WASM backend
- [ ] Online playground

## Community

### Join the Contests

We're running two exciting contests:

1. **Agent Framework Challenge**: Build agent systems in ANY language
2. **Aria Vision Challenge**: Contribute to Aria's core goals

[Learn more about contests →](./contests/README.md)

### Get Involved

- 💬 [Join Discord](https://discord.gg/aria-lang)
- 🐛 [Report Issues](https://github.com/cancelei/aria-lang/issues)
- 📖 [Read the Docs](./docs)
- 🤝 [Contribute](./community/CONTRIBUTING.md)
- 🎓 [RFCs](./community/rfcs)

## Contributing

We welcome contributions! Whether you're:

- Writing code
- Improving documentation
- Reporting bugs
- Suggesting features
- Building tools
- Creating content

See our [Contributing Guide](./community/CONTRIBUTING.md) to get started.

## Example Projects

Check out the [examples](./examples) directory for:

- BioFlow: Multi-language bioinformatics examples
- Module system demonstrations
- Python interop examples
- Real-world applications

## Research & Design

Aria is backed by extensive research in:

- Type systems (Hindley-Milner, bidirectional checking)
- Memory models (ownership, borrowing, regions)
- Effect systems (algebraic effects, inference)
- Compiler architecture (LLVM, Cranelift, MIR)
- Contract verification (SMT, property testing)

See [eureka-vault](./eureka-vault) for research notes.

## Philosophy

> "We build the language we want to use."

Aria is designed to be:

- **Pragmatic**: Solve real problems elegantly
- **Safe**: Catch errors at compile time
- **Fast**: Competitive with C/C++/Rust
- **Joyful**: Delightful to write and read
- **Community-Driven**: Built together, not by decree

## Inspiration

Aria draws inspiration from:

- **Ruby**: Expressive syntax, blocks, readability
- **Rust**: Memory safety, ownership, zero-cost abstractions
- **Go**: Simplicity, concurrency, fast compilation
- **Kotlin**: Flow-sensitive typing, null safety
- **Eiffel**: Design by Contract

## License

Aria is dual-licensed under:

- MIT License ([LICENSE-MIT](LICENSE-MIT))
- Apache License 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

Choose the license that best suits your needs.

## Acknowledgments

Special thanks to:

- The Rust community for pioneering safe systems programming
- The Ruby community for beautiful, expressive syntax
- Research in type theory, formal verification, and language design
- All contributors who make Aria possible

## Links

- **Website**: https://aria-lang.dev (coming soon)
- **Playground**: https://play.aria-lang.dev (in development)
- **Documentation**: https://docs.aria-lang.dev (coming soon)
- **Blog**: https://blog.aria-lang.dev (coming soon)
- **Twitter**: @aria_lang
- **Discord**: [Join here](https://discord.gg/aria-lang)

---

<div align="center">

**Status**: Active Development | **Version**: 0.1.0-alpha | **License**: MIT/Apache-2.0

*"In the neon glow of tomorrow's code, safety and speed become one."*

[⭐ Star us on GitHub](https://github.com/cancelei/aria-lang) • [🐦 Follow on Twitter](https://twitter.com/aria_lang)

</div>
