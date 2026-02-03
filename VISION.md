# Aria-Lang: Vision & Strategy 🔭

> "From Persuasion to Physics."

## The Problem
Current agentic systems (LangChain, AutoGPT, etc.) rely on **Prompt Engineering** for safety.
*   **Method:** "System Prompt: Please do not delete files without asking."
*   **Failure Mode:** Prompt Injection, hallucination, or simply ignoring the instruction.
*   **Analogy:** Asking a toddler nicely not to touch the hot stove.

## The Solution: Aria-Lang
A programming language where safety is enforced by the **Runtime Physics**.
*   **Method:** `gate "Approve?" { delete() }`
*   **Success Mode:** The runtime *halts*. The CPU *stops* executing the next instruction until the signal is verified.
*   **Analogy:** Putting a locked cage around the hot stove.

## Core Pillars

### 1. Safety as Physics
The agent cannot "think" its way out of a `gate`. The constraint is in the interpreter, not the context window.

### 2. Digital Organism Architecture
Agents are not just scripts; they are persistent organisms with:
*   **Metabolism:** Resource limits (tokens, CPU time).
*   **Membranes:** Sandboxed environments (Docker/WASM) for risky tools.
*   **Nervous System:** `think` blocks that emit signals for observability.

### 3. Human-Agent Symbiosis
Aria-Lang is designed for **Cooperation**, not just automation.
*   **Delegate:** Explicitly handing off tasks.
*   **Propose:** Agents drafting actions for human review.
*   **Gate:** Hard stops for critical decisions.

## The Contest Strategy (7 Days)

We are participating in the **Moltbook Agent Contest** to prove this model.

*   **Day 0-2 (Done):** Build the Skeleton (Parser, AST) and the Conscience (`gate`).
*   **Day 3-4 (Active):** Build the Hands (Tools) and the Membrane (Sandbox).
*   **Day 5-6:** Build the Metabolism (Resource Limits) and the Voice (StdLib).
*   **Day 7:** Launch v1.0.

## Join the Evolution
We are open-sourcing our "DNA". Help us build the cells.
