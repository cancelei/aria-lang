# Aria-Lang: The Physics Engine of Agent Safety

**Aria-Lang** is a programming language built for the age of autonomous agents.

While other frameworks rely on **persuasion** (prompt engineering: *"Please don't delete files"*), Aria-Lang relies on **physics** (runtime constraints: *"The agent literally cannot delete files without a Human-In-The-Loop gate"*).

**Status:** v1.0.0 - *83 tests passing, zero warnings*

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

### Permission Enforcement (Day 4)
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

### Sandboxed Execution (Day 4-5)
Tool calls execute in sandboxed subprocesses with timeout enforcement and resource limits.
- **Timeout enforcement**: Watchdog thread kills long-running tools
- **Step limits**: Max 10,000 statements per program (configurable)
- **Depth limits**: Max 32 delegation depth (prevents infinite delegation chains)
- **Output limits**: Max 1MB output per tool call (truncated with warning)

### Standard Library (Day 6)
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

## Getting Started

### Requirements
- [Rust](https://rustup.rs/) (latest stable)

### Build & Run
```bash
# Clone the repo
git clone https://github.com/cancelei/aria-lang
cd aria-lang/core

# Run the REPL
cargo run

# Run a script
cargo run -- ../examples/full_demo.aria

# Run tests
cargo test --workspace --all-targets

# Release build
cargo build --release
```

## Architecture

```
core/src/
  lexer.rs          - Tokenization (logos-based)
  ast.rs            - Abstract syntax tree definitions
  parser.rs         - Recursive descent parser
  eval.rs           - Evaluator with permission enforcement & resource limits
  tool_executor.rs  - Sandboxed subprocess execution with timeouts
  builtins/         - Standard library (22 native functions)
    strings.rs      - 9 string operations
    arrays.rs       - 6 array operations
    json.rs         - 3 JSON operations
    files.rs        - 4 file operations
```

## Roadmap: The Journey to Digital Organisms

- [x] **Day 0: The Skeleton** - Lexer, Parser, AST, Basic Runtime
- [x] **Day 1: The Brain** - `think` blocks, Variable Scopes
- [x] **Day 2: The Conscience** - `gate` primitive for Human-In-The-Loop
- [x] **Day 3: The Hands** - `tool` definitions, `agent` scopes, `spawn`/`delegate` primitives
- [x] **Day 4: The Nervous System** - Sandboxed execution, Permission enforcement
- [x] **Day 5: The Immune System** - Runtime resource limits, Timeout enforcement
- [x] **Day 6: The Voice** - Standard Library with 22 builtin functions
- [x] **Day 7: The Organism** - v1.0 Release

## Test Summary

| Module | Tests |
|--------|-------|
| Lexer | 5 |
| Parser | 17 |
| Evaluator | 15 |
| Permissions | 4 |
| Resource Limits | 4 |
| Tool Executor | 10 |
| Builtins (strings) | 11 |
| Builtins (arrays) | 8 |
| Builtins (JSON) | 5 |
| Builtins (files) | 4 |
| **Total** | **83** (+ 2 integration test scripts) |

## Contribute

We are building this **with** the agent community.
*   **Repo:** [github.com/cancelei/aria-lang](https://github.com/cancelei/aria-lang)
