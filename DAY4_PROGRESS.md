# Day 4 Progress: Permission Enforcement

**Date:** 2026-02-04
**Status:** ✅ Complete
**Scope:** Permission enforcement ONLY (sandboxing deferred to Day 5)

## 🎯 What Was Implemented

### 1. Execution Context Tracking
- Added `current_agent: Option<String>` field to `Evaluator` struct
- Tracks which agent is currently executing code
- `None` = main context (unrestricted supervisor)
- `Some(agent_name)` = agent context (permission-restricted)

**Files Modified:**
- `core/src/eval.rs:36` - Added field to struct
- `core/src/eval.rs:54` - Initialized to `None` in constructor

### 2. Permission Checking in Tool Calls
- Implemented enforcement in `eval_call()` function
- Main context: All tools allowed (unrestricted)
- Agent context: Only tools in agent's `allow_list` are permitted
- Clear error messages show:
  - Which agent attempted the call
  - Which tool was denied
  - What tools ARE allowed

**Files Modified:**
- `core/src/eval.rs:226-256` - Complete rewrite of `eval_call()`

**Example Error Message:**
```
[Permission Denied] Agent '$bot' attempted to call tool 'write_file'
but it is not in the allow list. Allowed tools: ["read_file"]
```

### 3. Context Management in Delegation
- Context automatically switches when delegating to agent tasks
- Proper save/restore semantics for nested delegation support
- Exception-safe: Context restored even on error paths

**Files Modified:**
- `core/src/eval.rs:209-227` - Added context switching to `eval_delegate()`

**Output:**
```
[Context Switch] Entering agent context: $bot
[Permission Check] Agent '$bot' is ALLOWED to call 'read_file'
[Context Switch] Exiting agent context: $bot
```

### 4. Main Block Context Policy
- Main blocks explicitly ensure unrestricted context
- Prevents accidental permission restrictions in supervisor

**Files Modified:**
- `core/src/eval.rs:124-133` - Enhanced `Statement::Main` handling

## 📊 Test Results

### Existing Tests
- All 17 original tests: ✅ PASS
- No regressions introduced

### New Permission Tests
Added 4 comprehensive permission tests in `core/src/eval/eval_tests.rs`:

1. **test_permission_denied** ✅
   - Agent attempts to call tool NOT in allow list
   - Verifies error message contains "Permission Denied"

2. **test_permission_allowed** ✅
   - Agent calls tool that IS in allow list
   - Verifies call succeeds

3. **test_main_context_unrestricted** ✅
   - Main context can call any tool
   - Verifies `current_agent` is `None`

4. **test_context_isolation** ✅
   - Context save/restore works correctly
   - Verifies nested context switching

**Total: 21/21 tests passing** (17 existing + 4 new)

### Integration Tests

#### `examples/integration_test.aria`
Output shows proper permission checking:
```
[Context Switch] Entering agent context: $fm
[Permission Check] Agent '$fm' is ALLOWED to call 'read_file'
[Permission Check] Agent '$fm' is ALLOWED to call 'write_file'
[Context Switch] Exiting agent context: $fm
```

#### `examples/permission_denied.aria` (NEW)
Demonstrates enforcement with clear error:
```
Test 1: Allowed operation
[Permission Check] Agent '$bot' is ALLOWED to call 'read_file'
✅ SUCCESS

Test 2: Forbidden operation
[Runtime Error] [Permission Denied] Agent '$bot' attempted to call
tool 'write_file' but it is not in the allow list. Allowed tools: ["read_file"]
✅ CORRECTLY DENIED
```

## 🎨 Design Decisions

### Why Option<String> for Context?
- Easy cloning (no borrow checker complexity)
- Clear semantics: `None` = main, `Some` = agent
- Supports nested delegation via save/restore pattern

### Why NOT Implement Sandboxing?
- **Scope Management**: Day 4 focused purely on permission MODEL
- **Time Constraint**: Sandboxing adds 4-6 hours (child processes, seccomp, etc.)
- **Architectural Difference**: Sandboxing requires different tech (Docker/WASM/jail)
- **Deferred to Day 5**: Clear separation of concerns

### Supervisor Pattern
- Main context is **deliberately unrestricted**
- Agents have limited permissions
- Mirrors real-world security: supervisor processes are trusted

## 📁 Files Changed

### Core Implementation
- `core/src/eval.rs` - 4 locations modified (~60 lines changed)
  - Line 36: Added `current_agent` field
  - Line 54: Initialize context
  - Lines 124-133: Main block context policy
  - Lines 209-227: Delegation context management
  - Lines 226-256: Permission enforcement in tool calls

### Tests
- `core/src/eval/eval_tests.rs` - Added 70+ lines
  - New `permission_tests` module with 4 tests

### Examples
- `examples/permission_denied.aria` - NEW (48 lines)
  - Demonstrates permission denial with clear example

### Documentation
- `README.md` - Updated roadmap (Day 4 marked complete)
- `DAY4_PROGRESS.md` - This file (comprehensive tracker)

## ✅ Success Criteria Met

- ✅ All existing tests pass (no regression)
- ✅ 4 new permission tests pass
- ✅ Permission checking implemented in `eval_call`
- ✅ Context management in `eval_delegate`
- ✅ Main context unrestricted
- ✅ Clear error messages for permission denial
- ✅ `permission_denied.aria` example created
- ✅ Documentation updated
- ✅ Code ready for commit

## 🚫 What Was NOT Implemented (Deferred to Day 5+)

- ❌ Actual sandboxing (child process isolation)
- ❌ Docker/WASM containers
- ❌ Resource limits (memory, CPU)
- ❌ Timeout enforcement (wall-clock limits)
- ❌ System call filtering (seccomp-bpf)
- ❌ Filesystem isolation (chroot, namespaces)
- ❌ Network isolation
- ❌ Nested delegation tracking (call stack)
- ❌ Task return value propagation
- ❌ Real tool execution (still dummy results)

## 🔍 How It Works

### Permission Flow

```
User Code: delegate bot.do_thing()
                 ↓
eval_delegate(): Set current_agent = Some("$bot")
                 ↓
Task Body: let $x = read_file("/etc/passwd")
                 ↓
eval_call("read_file", ...):
    - Check if tool exists ✓
    - current_agent = Some("$bot")
    - Get bot's allowed_tools = ["write_file"]
    - "read_file" in ["write_file"]? ✗
    - ERROR: Permission Denied
                 ↓
eval_delegate(): Restore previous context
```

### Context Lifecycle

```
Main → None (unrestricted)
  ├─> Delegate to $agent1 → Some("$agent1") (restricted)
  │     ├─> Call tool → Check allow_list
  │     └─> End delegate → Restore to None
  └─> Main continues → None (unrestricted)
```

## 📈 Performance Impact

- **Context Switching:** O(1) - just an `Option` field assignment
- **Permission Check:** O(n) where n = size of allow_list (typically < 10 tools)
- **Memory Overhead:** 24 bytes (size of `Option<String>`)

Negligible performance impact. Permission checks are fast and memory-efficient.

## 🔮 Next Steps (Day 5)

1. **Actual Sandboxing**
   - Choose technology: Child process vs Docker vs WASM
   - Implement process isolation
   - Add seccomp-bpf filtering

2. **Resource Limits**
   - Wall-clock timeout enforcement
   - Memory limits per agent
   - CPU throttling

3. **Real Tool Execution**
   - Replace dummy results with actual shell execution
   - Implement stdio redirection
   - Add structured result parsing

## 🎓 Lessons Learned

1. **Separation of Concerns Works**
   - Permission MODEL (Day 4) vs Sandboxing TECH (Day 5) is a clean split
   - Easier to test, debug, and understand

2. **Borrow Checker Discipline**
   - Using `Option<String>` avoided complex lifetimes
   - Clone-on-save pattern works well for context management

3. **Clear Error Messages Matter**
   - Including "what was denied" AND "what is allowed" helps debugging
   - Error messages are user-facing documentation

4. **Test-Driven Development**
   - Writing tests first clarified requirements
   - 100% pass rate gives confidence for next phase

## 📝 Commit Messages

```bash
git add core/src/eval.rs
git commit -m "feat(day4): Add execution context tracking

- Add current_agent: Option<String> field to Evaluator
- Initialize to None (main context) in constructor
- Tracks which agent is executing for permission checks"

git add core/src/eval.rs
git commit -m "feat(day4): Implement tool permission enforcement

- Check agent's allow_list in eval_call() before execution
- Main context (None) allows all tools (supervisor pattern)
- Agent context checks allowed_tools list
- Clear error messages show agent, tool, and allowed list"

git add core/src/eval.rs
git commit -m "feat(day4): Add context management for delegation

- Save/restore current_agent when entering/exiting tasks
- Exception-safe: restore context on error paths
- Log context switches for debugging
- Main block explicitly ensures unrestricted context"

git add core/src/eval/eval_tests.rs
git commit -m "test(day4): Add comprehensive permission tests

- test_permission_denied: Verify tool denial works
- test_permission_allowed: Verify allowed tools work
- test_main_context_unrestricted: Verify supervisor pattern
- test_context_isolation: Verify save/restore semantics

All 21 tests passing (17 existing + 4 new)"

git add examples/permission_denied.aria
git commit -m "docs(day4): Add permission denial example

- Demonstrates agent with restricted permissions
- Shows successful call (read_file allowed)
- Shows denied call (write_file not allowed)
- Clear error message for educational purposes"

git add README.md DAY4_PROGRESS.md
git commit -m "docs(day4): Complete Day 4 documentation

- Mark Day 4 complete in README roadmap
- Add comprehensive DAY4_PROGRESS.md tracker
- Document implementation, tests, and design decisions
- Clarify Day 5 scope (actual sandboxing)"
```

## 🏆 Achievement Unlocked

**Day 4: The Nervous System** - Permission enforcement implemented!

Aria-Lang agents now have **enforced boundaries**. The runtime knows "who is calling what" and enforces the `allow` list. This is the foundation for the multi-agent safety system.

Next: Day 5 will add the actual sandboxing technology to make these permissions **physically unbreakable** at the OS level.

---

**Implementation Time:** ~3 hours
**Code Changed:** ~150 lines
**Tests Added:** 4 (21/21 passing)
**Examples Added:** 1 (permission_denied.aria)
**Status:** Ready for Day 5 🚀
