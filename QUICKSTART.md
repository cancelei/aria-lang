# Aria-Lang Quickstart Guide

**Get started with Aria in 10 minutes.**

## What is Aria-Lang?

Aria is a programming language for autonomous agents with **physics-based safety**. Unlike traditional frameworks that rely on prompt engineering ("Please don't delete files"), Aria enforces safety through runtime constraints that agents literally cannot bypass.

## Installation

### Prerequisites
- Rust 1.70+ ([install here](https://rustup.rs/))
- Git
- Linux/macOS (Windows via WSL)

### Build from Source

```bash
# Clone the repository
git clone https://github.com/cancelei/aria-lang
cd aria-lang

# Build the project
cargo build --release

# Verify installation
./target/release/aria --version
```

Expected output: `Aria-Lang v0.1.0 (Contest Edition)`

## Your First Program

Create a file called `hello.aria`:

```aria
main {
    print "Hello, Aria!"
    
    think { "This is my first program" }
    
    let $name = "World"
    let $greeting = str_concat("Hello, ", $name)
    print $greeting
}
```

Run it:

```bash
cargo run -- hello.aria
```

Expected output:
```
[Entering Main Block]
Hello, Aria!
[Thinking...] "This is my first program"
[Builtin Call] str_concat with 2 args
Hello, World
[Exiting Main Block]
```

## Core Concepts in 5 Minutes

### 1. Variables (let)

```aria
let $name = "Aria"
let $age = 42
let $pi = 3.14159
```

All variables start with `$`.

### 2. Printing Output

```aria
print "Simple message"
print $variable
```

### 3. String Operations

```aria
let $upper = str_upper("aria")        // "ARIA"
let $len = str_len("hello")            // 5
let $joined = str_concat("Hi", " there")  // "Hi there"
```

See [API_REFERENCE.md](API_REFERENCE.md) for all 24 builtin functions.

### 4. Arrays (JSON-based)

```aria
let $arr = arr_from_split("a,b,c", ",")  // ["a","b","c"]
let $len = arr_len($arr)                  // 3
let $first = arr_get($arr, 0)             // "a"
```

### 5. Thinking Blocks (Observability)

```aria
think { "Planning my next move" }
think { "Analyzing the data" }
```

Thinking blocks are first-class citizens - the agent's internal monologue is traceable.

### 6. Tool Definitions

```aria
tool echo(message: string) {
    timeout: 5
}

let $result = echo("Hello from a tool!")
```

Tools execute in sandboxed child processes with timeout enforcement.

## The REPL

Start an interactive session:

```bash
cargo run
```

Try these commands:

```
aria> let $x = str_upper("hello")
aria> print $x
aria> let $arr = arr_from_split("a,b,c", ",")
aria> print arr_len($arr)
aria> exit
```

## Next Steps

### Learn More
- **[TUTORIAL.md](TUTORIAL.md)** - Structured 1-hour learning path
- **[API_REFERENCE.md](API_REFERENCE.md)** - Complete builtin function reference
- **[examples/](examples/)** - 7+ example programs

### Explore Examples

```bash
# Basic features
cargo run -- examples/quickstart.aria

# Standard library showcase
cargo run -- examples/stdlib_demo.aria

# Multi-agent workflow
cargo run -- examples/multi_agent_workflow.aria

# Permission enforcement
cargo run -- examples/permission_denied.aria
```

### Key Features to Explore

1. **gate** primitive - Human-in-the-loop safety
   ```aria
   gate "Approve this operation?" {
       // Code here only runs if human approves
   }
   ```

2. **agent** definitions - Scoped permissions
   ```aria
   agent Bot {
       allow read_file
       allow echo
       
       task process() {
           // Bot can only use allowed tools
       }
   }
   ```

3. **Sandboxed execution** - Process isolation
   - Every tool runs in a separate child process
   - Timeout enforcement (wall-clock)
   - Permission checking (runtime)

## Troubleshooting

### Build Errors

```bash
# Clean and rebuild
cargo clean
cargo build --release
```

### Example Won't Run

Make sure you're in the project root:
```bash
pwd  # Should show /path/to/aria-lang
cargo run -- examples/quickstart.aria
```

### Permission Denied Error

This is expected! It means the permission system is working:

```
[Permission Denied] Agent '$bot' attempted to call tool 'write_file'
but it is not in the allow list. Allowed tools: ["read_file"]
```

Check the agent's `allow` list in the source code.

## Getting Help

- **Documentation**: Read [TUTORIAL.md](TUTORIAL.md) for detailed learning
- **Examples**: Check `examples/examples_README.md` for all examples
- **Community**: Visit [moltbook.com/m/arialang](https://moltbook.com/m/arialang)
- **Issues**: Report bugs at [github.com/cancelei/aria-lang/issues](https://github.com/cancelei/aria-lang/issues)

## What Makes Aria Different?

**Traditional Approach (Prompt Engineering):**
```
System Prompt: "Please don't delete files without asking."
Agent: *deletes files anyway*
```

**Aria Approach (Physics-Based Safety):**
```aria
gate "Delete files?" {
    delete_files()  // Runtime pauses until human approves
}
```

The `gate` blocks execution. The agent cannot "think" its way past it. Safety is a law of physics, not a suggestion.

---

**Ready to build safe autonomous agents?** Continue to [TUTORIAL.md](TUTORIAL.md) →
