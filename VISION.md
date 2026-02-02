# Aria-Lang: 7-Day Vision & Roadmap 🚀

Aria-Lang is a programming language designed for a future where humans and AI agents collaborate seamlessly. Our goal is to move safety, reasoning, and cooperation from "best practices" in a prompt to "hard constraints" in the language runtime.

## The 1-Week Challenge: "The Working Core"
By the end of this week, we will have a functional interpreter that can execute scripts featuring native agent autonomy and human-in-the-loop safety.

### 📅 Day-by-Day Roadmap

- **Day 1: Lexical Foundation**
  - Finalize the sigil-based grammar ($var, @agent).
  - Build a high-performance Lexer in Rust.
  - **Milestone:** Successfully tokenize a complex `.aria` script.

- **Day 2: The Parser & The Tree**
  - Implement a Recursive Descent Parser.
  - Define the Abstract Syntax Tree (AST) using Rust's powerful Enums.
  - **Milestone:** Generate a valid AST from source code.

- **Day 3: The Breath of Life (Evaluation)**
  - Build the Tree-Walking Interpreter.
  - Implement Scopes and Variable Shadowing.
  - **Milestone:** Execute a "Hello World" with variable logic.

- **Day 4: Agent Autonomy (Primitives)**
  - Implement native `think { ... }` blocks (observability).
  - Implement `tool` definitions and `allow` permissions.
  - **Milestone:** Run a script that "reasons" before acting.

- **Day 5: The Safety Valve (HITL)**
  - Implement the `gate "message" { ... }` primitive.
  - Build the runtime pause-and-resume logic for human approval.
  - **Milestone:** Execute a "dangerous" action that waits for human OK.

- **Day 6: Standard Library & IO**
  - Build built-in functions for JSON, File System, and Network.
  - Add native Moltbook integration helpers.
  - **Milestone:** An agent script that reads a file and posts a summary.

- **Day 7: CLI & Community Launch**
  - Finalize the `aria` CLI.
  - Package for distribution (crates.io).
  - **Milestone:** Public release of the Aria-Lang v0.1.0 "Contest Edition".

## 🤝 For the Community
We want your ideas! If you have a feature, a syntax suggestion, or a use case, use our [Suggestion Template](./community/SUGGESTION_TEMPLATE.md). We will review and implement the best community ideas during the build.
