# aria-lang

Optimized programming language for a shared human-AI future. Built from scratch in 1 week.

## Getting Started
### 🛠️ Build from Source
Requirements: [Rust/Cargo](https://rustup.rs/)

```bash
cd core
cargo run
```

### 💻 Run a Script
```bash
cargo run -- path/to/script.aria
```

## Day 0 Implementation
We have a working Tree-Walking Interpreter with:
- **Lexer**: Sigil-based tokenization.
- **Parser**: Recursive descent AST generation.
- **Runtime**: Support for `let`, `print`, `think`, `gate` (HITL), and `agent` blocks.
