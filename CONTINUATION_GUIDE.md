# Day 3 Continuation Guide

**Status**: Parser Complete ✅ | Evaluator Pending ⏸️
**Last Session**: 2026-02-03 21:45 UTC
**Next Session**: Continue evaluator implementation

---

## 🎯 Where We Left Off

### ✅ Completed (Steps 1-6)
- Lexer with all Day 3 tokens
- AST with all new expression/statement types
- Parser with 100% Day 3 syntax support
- 17 tests passing (5 original + 12 new)
- 4 clean commits with documentation

### ⏸️ Next Phase (Steps 7-12)
**Evaluator Implementation** - Make the runtime execute code

**Current State**: Parser works, but evaluator stubs return "not yet implemented"

**Example**:
```bash
cd core && cargo run -- ../examples/parser_demo.aria
# Output: [Runtime Error] Tool definitions not yet implemented
```

This is **expected** - parser validated syntax, now evaluator needs to execute it.

---

## 🚀 Quick Start for Next Session

### 1. Review Current State
```bash
cd /home/cancelei/Projects/aria-lang
git log --oneline -5  # See recent commits
cargo test --quiet    # Verify 17 tests pass
```

### 2. Read the Plan
- **Primary**: `DAY3_IMPLEMENTATION_PLAN.md` (Steps 7-12)
- **Context**: `PARSER_IMPLEMENTATION.md` (what's already done)
- **Progress**: `DAY3_PROGRESS.md` (session tracking)

### 3. Key Files to Modify
- `/home/cancelei/Projects/aria-lang/core/src/eval.rs` - Main evaluator logic
- `/home/cancelei/Projects/aria-lang/core/src/eval/eval_tests.rs` - New tests

### 4. Target Example
- `/home/cancelei/Projects/aria-lang/examples/agentic_primitives.aria`
- **Goal**: Make this run successfully with permission checking

---

## 📋 Implementation Checklist

### Step 7: Data Structures (30 min)
```rust
// Add to eval.rs
struct Tool { name, params, permission, timeout }
struct AgentDef { name, allow_list, tasks }
struct AgentInstance { name, agent_def_name, allowed_tools, variables }

// Update Evaluator
pub struct Evaluator {
    variables: HashMap<String, Value>,
    tools: HashMap<String, Tool>,      // NEW
    agent_defs: HashMap<String, AgentDef>,  // NEW
    agents: HashMap<String, AgentInstance>, // NEW
}
```

### Step 8: Tool Registration (30 min)
```rust
fn eval_tool_def(&mut self, name, params, permission, timeout) {
    self.tools.insert(name, Tool { ... });
}
```

### Step 9: Agent Registration (45 min)
```rust
fn eval_agent_def(&mut self, name, allow_list, tasks, body) {
    self.agent_defs.insert(name, AgentDef { ... });
}
```

### Step 10: Spawn (45 min)
```rust
fn eval_spawn(&mut self, var_name, agent_name) {
    let def = self.agent_defs.get(agent_name)?;
    let instance = AgentInstance {
        name: var_name.clone(),
        agent_def_name: agent_name,
        allowed_tools: def.allow_list.clone(),
        variables: HashMap::new(),
    };
    self.agents.insert(var_name, instance);
}
```

### Step 11: Function Calls (1 hour)
```rust
fn eval_call(&mut self, name, args, agent_context) -> Result<Value> {
    // 1. Check if it's a tool or task
    if self.tools.contains_key(name) {
        return self.execute_tool(name, args, agent_context);
    }
    // 2. Otherwise look for task in agent context
}

fn execute_tool(&mut self, name, args, agent_context) -> Result<Value> {
    // 1. Get tool definition
    let tool = self.tools.get(name)?;

    // 2. **PHYSICS-BASED SAFETY**: Check permissions
    if let Some(agent) = agent_context {
        if !self.check_permission(agent, name) {
            return Err("Permission denied: agent not allowed to use this tool");
        }
    }

    // 3. For Day 3: Only implement 'shell' tool
    if name == "shell" {
        // Execute shell command (no sandbox yet - Day 4)
    }
}

fn check_permission(&self, agent_name: &str, tool_name: &str) -> bool {
    if let Some(agent) = self.agents.get(agent_name) {
        return agent.allowed_tools.contains(&tool_name.to_string());
    }
    false
}
```

### Step 12: Delegate (1 hour)
```rust
fn eval_delegate(&mut self, call: Expr) -> Result<Value> {
    // Parse call as member access (bot.method())
    // Look up agent instance
    // Look up task in agent def
    // Execute task body in agent's variable scope
}
```

---

## 🧪 Testing Strategy

### New Tests to Add (eval/eval_tests.rs)

```rust
#[test]
fn test_tool_registration() {
    let mut eval = Evaluator::new();
    // Register tool, verify it's in tools map
}

#[test]
fn test_spawn_agent() {
    let mut eval = Evaluator::new();
    // Define agent, spawn instance, verify created
}

#[test]
fn test_permission_check_allowed() {
    // Agent WITH permission can use tool
}

#[test]
fn test_permission_check_denied() {
    // Agent WITHOUT permission CANNOT use tool
    // This is the PHYSICS part!
}

#[test]
fn test_delegate_call() {
    // Spawn agent, delegate to task, verify execution
}

#[test]
fn test_shell_tool_execution() {
    // Execute shell("echo test"), verify output
}
```

---

## 🎯 Success Criteria

### Must Have
- [ ] All existing 17 tests still pass
- [ ] Minimum 6 new evaluator tests pass (total 23+)
- [ ] Tool definitions register successfully
- [ ] Agent spawning creates instances
- [ ] **Permission system blocks unauthorized tool access**
- [ ] Delegate calls agent tasks
- [ ] Shell tool executes commands

### Integration Test
- [ ] `examples/agentic_primitives.aria` runs successfully
- [ ] Permission denied when agent tries unauthorized tool
- [ ] Gate primitive still works in agent contexts

### Nice to Have
- [ ] `examples/cooperation.aria` also works
- [ ] String interpolation in shell commands
- [ ] Return values from tasks

---

## ⚡ Quick Implementation Path

### Phase 1: Foundation (1 hour)
1. Add data structures to eval.rs
2. Update Evaluator::new() to initialize new HashMaps
3. Add eval_tool_def and eval_agent_def
4. Wire them into eval_statement match

### Phase 2: Execution (2 hours)
1. Implement eval_spawn
2. Implement eval_call with permission checking
3. Add shell tool execution
4. Test tool + spawn working

### Phase 3: Orchestration (1 hour)
1. Implement eval_delegate
2. Test full agent workflow
3. Add remaining evaluator tests

### Phase 4: Integration (30 min)
1. Run agentic_primitives.aria
2. Debug any issues
3. Verify permission system works

### Phase 5: Polish (30 min)
1. Better error messages
2. Add comments
3. Update VISION.md roadmap
4. Commit with proper message

**Total Time**: 4-6 hours

---

## 📁 Key Files Reference

### Implementation Files
- `core/src/eval.rs` - Main evaluator (will modify heavily)
- `core/src/eval/eval_tests.rs` - Tests (will add 6-8 tests)
- `core/src/ast.rs` - AST definitions (reference only)

### Test Files
- `examples/agentic_primitives.aria` - Primary integration test
- `examples/parser_demo.aria` - Alternative demo
- `examples/step_by_step.aria` - Step-by-step examples

### Documentation
- `DAY3_IMPLEMENTATION_PLAN.md` - Complete roadmap
- `PARSER_IMPLEMENTATION.md` - Parser details (done)
- `SESSION_SUMMARY.md` - Last session achievements

---

## 🔧 Useful Commands

```bash
# Build and test
cd core
cargo test                    # Run all tests
cargo test --quiet           # Quiet mode
cargo run -- ../examples/agentic_primitives.aria  # Integration test

# Check specific test
cargo test test_permission_check -- --nocapture

# Format and lint
cargo fmt
cargo clippy

# Git
git log --oneline -5
git diff HEAD~1
git status
```

---

## 🎨 Physics-Based Safety Model

**Key Concept**: The evaluator enforces safety as a runtime constraint, not a prompt suggestion.

### Traditional Approach (Persuasion)
```python
# Prompt: "Please don't delete files without asking"
agent.execute("rm -rf /")  # Agent can ignore this!
```

### Aria Approach (Physics)
```aria
agent Bot {
    allow shell  // Explicit capability grant

    task work() {
        shell("rm -rf /")  // Gate will block this!
    }
}
```

**Implementation**:
```rust
fn execute_tool(&mut self, name: &str, args: Vec<Value>, agent: &str) -> Result<Value> {
    // PHYSICS: Runtime check that cannot be bypassed
    if !self.check_permission(agent, name) {
        return Err("Permission denied: physics violation");
    }
    // Tool execution only happens if permission check passes
}
```

---

## 🏁 Session Goal

**Primary**: Get agentic_primitives.aria running end-to-end
**Proof**: Shell commands execute, permissions enforce, gates block

**Secondary**: 23+ tests passing
**Proof**: All original tests + 6-8 new evaluator tests

**Tertiary**: Clean commit with documentation
**Proof**: Evaluator implementation committed, VISION.md updated

---

## 💡 Tips for Success

1. **Start with data structures** - Get the types right first
2. **Test incrementally** - Write test, implement, verify
3. **Focus on shell tool only** - Other tools can wait for Day 4
4. **Keep permission checking simple** - Just string matching for now
5. **Use existing gate code as reference** - Shows how to block execution
6. **Don't worry about perfect error handling** - Working prototype > perfection

---

## 🚨 Common Pitfalls

1. **Forgetting agent context in eval_call** - Need to pass which agent is calling
2. **Not scoping variables to agents** - Each agent instance needs its own HashMap
3. **Mixing AgentDef and AgentInstance** - Definition vs spawned instance
4. **Permission check after execution** - Check BEFORE, not after (physics!)
5. **Hardcoding agent names** - Use dynamic lookup from variables

---

## ✅ Pre-Flight Checklist

Before starting next session:
- [ ] Read this guide
- [ ] Review DAY3_IMPLEMENTATION_PLAN.md Steps 7-12
- [ ] Verify 17 tests passing
- [ ] Understand permission model (physics-based safety)
- [ ] Have examples/agentic_primitives.aria open for reference

**Estimated Session Duration**: 4-6 hours
**Complexity**: Medium-High (data structures + permission logic)
**Reward**: Full working agentic runtime with physics-based safety! 🎉

---

**Ready to build the physics engine of agent safety!** 🚀

*Last Updated: 2026-02-03 21:50 UTC*
*Next Session: Evaluator implementation (Steps 7-12)*
