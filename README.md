# Aria-Lang: The Physics Engine of Agent Safety

**Aria-Lang** is a programming language built for the age of autonomous agents.

While other frameworks rely on **persuasion** (prompt engineering: *"Please don't delete files"*), Aria-Lang relies on **physics** (runtime constraints: *"The agent literally cannot delete files without a Human-In-The-Loop gate"*).

**Status:** v1.2.0 — 953 tests passing, zero failures

## Why Aria-Lang?

We are building a Digital Organism Runtime where:
1.  **Safety is a Law, Not a Suggestion**: `gate` primitives pause the runtime. The AI cannot "hallucinate" its way past a hard stop.
2.  **Reasoning is Visible**: `think` blocks are first-class citizens, making the agent's thought process traceable and debuggable.
3.  **Permissions are Physical**: Capabilities like `filesystem` or `network` are granted to specific agent scopes, not the entire process.
4.  **Resources are Bounded**: Runtime step limits, delegation depth limits, and output size limits prevent runaway agents.

## Key Features

### `gate` - The Safety Valve
A native primitive that pauses execution until a human signal is received.
```aria
think { "I found 500GB of logs to delete." }

gate "Approve deletion of 500GB logs?" {
    shell("rm -rf /var/logs/*.old")
}
```

### `think` - Native Reasoning
Separate the "internal monologue" from the "external action."
```aria
think { "User asked for a poem. I should avoid cliches." }
print "Roses are red..."
```

### Permission Enforcement
Agents can only call tools they're explicitly allowed to use.
```aria
tool echo(text: string) {
    permission: "io.write",
    timeout: 5
}

agent Greeter {
    allow echo       // Can use echo
    // Cannot use shell, read_file, etc. - enforced at runtime!

    task greet(name: string) {
        let $msg = echo(name)
        return $msg
    }
}

main {
    let $bot = spawn Greeter
    delegate bot.greet("World")
}
```

### Sandboxed Execution
Tool calls execute in sandboxed subprocesses with timeout enforcement and resource limits.
- **Timeout enforcement**: Watchdog thread kills long-running tools
- **Step limits**: Max 10,000 statements per program (configurable)
- **Depth limits**: Max 32 delegation depth (prevents infinite delegation chains)
- **Output limits**: Max 1MB output per tool call (truncated with warning)

### Standard Library
22 builtin functions available without tool definitions:
```aria
// String operations
let $upper = str_upper("aria")           // "ARIA"
let $has = str_contains("hello", "ell")  // 1.0

// Array operations (JSON-based)
let $arr = arr_from_split("a,b,c", ",")  // ["a","b","c"]
let $len = arr_len($arr)                 // 3.0

// JSON operations
let $obj = json_parse("{\"key\":\"val\"}")
let $val = json_get($obj, "key")         // "val"

// File operations
let $content = file_read("/etc/hostname")
file_write("/tmp/out.txt", "hello")
```

### Effects-as-WASM-Capabilities (Industry First)
Aria's effect system maps directly to the WASM Component Model's capability system:
```
Aria Effect    →  WASI Capability       →  Physical Enforcement
!IO            →  wasi:io/streams       →  Only I/O access
!Console       →  wasi:cli/stdout       →  Only terminal access
!FileSystem    →  wasi:filesystem       →  Only file access
!Network       →  wasi:http             →  Only HTTP access
!{} (pure)     →  No imports            →  Zero capabilities
```
An agent declared with `!{Network, FileSystem}` compiles to a WASM component that **physically cannot** access anything else. Two-layer guarantee: compile-time verification + runtime sandbox enforcement.

### MIR Compiler Pipeline
Full compiler infrastructure with Mid-level Intermediate Representation:
- **Lexer** (logos-based) → **Parser** (recursive descent) → **AST** → **MIR lowering** → **Codegen**
- Type inference with Hindley-Milner unification
- Generic functions with type parameter inference
- String interpolation, lambdas/closures with capture analysis
- Array/map comprehensions, pattern matching, effect system
- Cranelift, WASM core module, and WASM Component Model codegen backends

## Getting Started

### Requirements
- [Rust](https://rustup.rs/) (latest stable)

### Build & Run
```bash
# Clone the repo
git clone https://github.com/cancelei/aria-lang
cd aria-lang

# Run the REPL
cargo run -p aria

# Run a script
cargo run -p aria -- examples/full_demo.aria

# Run all tests
cargo test --workspace

# Release build
cargo build --release
```

## Architecture

```
core/src/                     Runtime & interpreter
  lexer.rs                    Tokenization (logos-based)
  ast.rs                      Abstract syntax tree
  parser.rs                   Recursive descent parser
  eval.rs                     Evaluator with permissions & resource limits
  tool_executor.rs            Sandboxed subprocess execution
  mcp_client.rs               MCP protocol client
  builtins/                   Standard library (22 native functions)

crates/
  aria-ast/                   AST definitions (shared across compiler)
  aria-lexer/                 Lexer (shared)
  aria-types/                 Type system & Hindley-Milner inference
  aria-mir/                   MIR lowering, optimization, pretty-printing
  aria-codegen/               Cranelift, WASM core & Component Model backends
  aria-wit/                   WIT generation from effect declarations
  aria-compiler/              Compiler driver
  aria-contracts/             Contract verification
  aria-diagnostics/           Error reporting with suggestions
  aria-effects/               Algebraic effect system
  aria-channel/               Channel & select concurrency primitives
  aria-ffi/                   C FFI bridge
  aria-runtime/               Runtime support library
```

## Test Summary

| Area | Tests |
|------|-------|
| Core (lexer, parser, evaluator, permissions, builtins, tool executor) | 161 |
| Type system & inference | 241 |
| MIR lowering & optimization | 77 |
| MIR integration tests | 25 |
| Parser (crate) | 101 |
| AST | 51 |
| Diagnostics | 23 |
| Effects | 31 |
| Channels & concurrency | 26 |
| Contracts | 31 |
| FFI | 20 |
| Codegen (Cranelift + WASM core) | 27 |
| WASM Component Model | 7 |
| WIT generation | 15 |
| Feature-specific tests (pattern guards, async/await, traits, etc.) | 101 |
| Inlining optimization | 4 |
| Doc tests | 1 |
| **Total** | **953** |

## Roadmap

- [x] **Day 0-2**: Core language — Lexer, Parser, `think`, `gate`
- [x] **Day 3**: Agent primitives — `tool`, `agent`, `spawn`/`delegate`, MIR parser extensions
- [x] **Day 4**: Permission enforcement & sandboxed execution
- [x] **Day 5**: Runtime resource limits & timeout enforcement
- [x] **Day 6**: Standard library (22 builtins)
- [x] **Day 7**: v1.0 release
- [x] **Post-v1.0**: MIR compiler pipeline, type inference, MCP client, orchestration
- [x] **v1.2.0**: Effects-as-WASM-Capabilities pipeline, WIT generation, Component Model codegen
- [ ] **Next**: Compile-time gate coverage analysis, MCP native support, LSP server

## Contribute

We are building this **with** the agent community.
- **Repo:** [github.com/cancelei/aria-lang](https://github.com/cancelei/aria-lang)
