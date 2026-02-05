# Aria-Lang Examples

## Working Examples (v1.0)

### Getting Started
- **quickstart.aria** - Your first Aria program. Demonstrates variables, string operations, and tool calls.
- **stdlib_demo.aria** - Comprehensive showcase of all 24 builtin functions (strings, arrays, JSON, files).

### Core Features
- **integration_test.aria** - Agent definitions, spawning, delegation, and permissions.
- **multi_agent_workflow.aria** - Multi-agent cooperation with permission isolation.
- **sandbox_test.aria** - Sandboxed tool execution and timeout enforcement.
- **permission_denied.aria** - Permission enforcement demonstration.

### Safety Primitives
- **hitl_approval.aria** - Human-in-the-loop gate primitive (requires interaction).

## How to Run

```bash
# Build the project
cargo build --release

# Run an example
cargo run -- examples/quickstart.aria

# Or use the built binary
./target/release/aria examples/quickstart.aria
```

## Expected Behavior

All examples should run without errors (except hitl_approval.aria which waits for user input at the gate).

Output includes debug traces showing:
- `[Builtin Call]` - Standard library function executions
- `[Tool Call]` - Sandboxed tool executions
- `[Sandbox]` - Command execution details
- `[Permission Check]` - Permission enforcement
- `[Context Switch]` - Agent context changes
- `[Thinking...]` - Agent reasoning blocks

## Example Categories

### Level 1: Basics
Start here if you're new to Aria.
- quickstart.aria (5 minutes)

### Level 2: Standard Library
Learn the builtin functions.
- stdlib_demo.aria (10 minutes)

### Level 3: Agentic Features
Understand agents, tools, and safety.
- integration_test.aria (10 minutes)
- multi_agent_workflow.aria (10 minutes)
- sandbox_test.aria (15 minutes)
- permission_denied.aria (5 minutes)

### Level 4: Safety Primitives
See physics-based safety in action.
- hitl_approval.aria (5 minutes, interactive)

## Future Examples (Coming Post-v1.0)

These examples use features not yet implemented:
- HTTP client examples
- Regular expression examples
- Advanced array operations (map, filter, fold)
- Module system examples
- Type system examples
