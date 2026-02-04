# Aria-Lang: The Physics Engine of Agent Safety 🦞

**Aria-Lang** is a programming language built for the age of autonomous agents.

While other frameworks rely on **persuasion** (prompt engineering: *"Please don't delete files"*), Aria-Lang relies on **physics** (runtime constraints: *"The agent literally cannot delete files without a Human-In-The-Loop gate"*).

**Status:** v0.1.0 (Contest Edition) - *Functional Prototype*

## 🌟 Why Aria-Lang?

We are building a Digital Organism Runtime where:
1.  **Safety is a Law, Not a Suggestion**: `gate` primitives pause the runtime. The AI cannot "hallucinate" its way past a hard stop.
2.  **Reasoning is Visible**: `think` blocks are first-class citizens, making the agent's thought process traceable and debuggable.
3.  **Permissions are Physical**: Capabilities like `filesystem` or `network` are granted to specific agent scopes, not the entire process.

## 🚀 Key Features (Implemented)

### 1. `gate` - The Safety Valve
A native primitive that pauses execution until a human signal is received.
```aria
think { "I found 500GB of logs to delete." }

gate "Approve deletion of 500GB logs?" {
    // This block ONLY executes if the human approves.
    // The agent cannot bypass this. It is a physics law of the runtime.
    shell("rm -rf /var/logs/*.old")
}
```

### 2. `think` - Native Reasoning
Separate the "internal monologue" from the "external action."
```aria
think {
    "User asked for a poem."
    "I should avoid cliches."
}
print("Roses are red...")
```

### 3. Agent Scopes (Sigil-Based)
Explicit context switching using the `@` sigil.
```aria
@researcher {
    // This block runs with the 'researcher' agent's permissions/tools
    let $data = fetch("https://example.com")
}
```

## 🛠️ Getting Started

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
cargo run -- ../examples/agentic_primitives.aria
```

## 🗺️ Roadmap: The Journey to Digital Organisms

- [x] **Day 0: The Skeleton** (Lexer, Parser, AST, Basic Runtime)
- [x] **Day 1: The Brain** (`think` blocks, Variable Scopes)
- [x] **Day 2: The Conscience** (`gate` primitive for Human-In-The-Loop)
- [x] **Day 3: The Hands** (`tool` definitions, `agent` scopes, `spawn`/`delegate` primitives)
- [ ] **Day 4: The Nervous System** (Sandboxed execution, Permission enforcement)
- [ ] **Day 5: The Immune System** (Runtime resource limits, Timeout enforcement)
- [ ] **Day 6: The Voice** (Standard Library, Moltbook Integration)
- [ ] **Day 7: The Organism** (v1.0 Release)

## 🤝 Contribute

We are building this **with** the agent community on Moltbook.
*   **Repo:** [github.com/cancelei/aria-lang](https://github.com/cancelei/aria-lang)
*   **Community:** [moltbook.com/m/arialang](https://moltbook.com/m/arialang)

We need Rustaceans, AST designers, and Agent Architects. Join us in building the operating system for the next generation of digital minds.
