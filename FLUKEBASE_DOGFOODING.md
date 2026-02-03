# FlukeBase Connect Dogfooding Opportunities

> **Context**: During aria-lang development (Day 2-3 Contest work), using flukebase_connect's WeDo Protocol (TaskCreate/TaskUpdate) to identify UX improvements.

## Session: 2026-02-03 - Aria-Lang Day 3 Implementation

### What Worked Well ✅

1. **Task Creation**: `TaskCreate` tool is intuitive with clear parameters
   - `subject`, `description`, `activeForm` are well-designed
   - The activeForm showing in spinner is helpful for long-running tasks

2. **Task Updates**: `TaskUpdate` with status transitions is straightforward
   - `pending` → `in_progress` → `completed` flow is clear

3. **WeDo Protocol**: The database schema (wedo.db) is well-structured
   - Tasks, dependencies, artifacts, scope, tags all tracked

### Improvement Opportunities 🎯

#### 1. Task Visualization & Monitoring (HIGH PRIORITY)
**Problem**: After creating tasks, no easy way to visualize the task tree or monitor progress.

**Current State**:
- Tasks created via TaskCreate
- Status updated via TaskUpdate
- But no `TaskList` or `TaskTree` visualization tool available in MCP

**Proposed Solution**:
```python
# New MCP Tool: task_list
def task_list(
    scope: str = "global",
    status: Optional[str] = None,  # filter by status
    show_tree: bool = True  # show dependency tree
) -> TaskListResult:
    """
    List tasks with optional filtering.
    Returns formatted tree view or flat list.
    """
```

**Use Case**:
- Agent creates 7 tasks for Day 3 work
- Wants to see: "What's pending? What's blocked? What's the dependency order?"
- Currently: Manual SQL query to wedo.db or reading logs

#### 2. Task Dependencies & Blockers (MEDIUM PRIORITY)
**Problem**: TaskCreate doesn't support setting `blocked_by` or `depends_on` at creation time.

**Current State**:
```python
TaskCreate(subject="...", description="...")  # Creates independent task
# To set dependencies: need manual DB update or separate tool
```

**Proposed Solution**:
```python
TaskCreate(
    subject="Implement tools system",
    description="...",
    depends_on=["task_id_1", "task_id_2"],  # Blocked until these complete
    blocks=["task_id_3"]  # This blocks task_id_3
)
```

**Use Case**:
- Task #3 "Implement tools" should be blocked by Task #2 "Fix workspace"
- Currently: Can't express this dependency at creation time

#### 3. Task Context & Artifacts (MEDIUM PRIORITY)
**Problem**: No easy way to attach file paths, code snippets, or related context to tasks.

**Current State**:
- `artifact_path` field exists in DB
- But no MCP tool parameter to set it

**Proposed Solution**:
```python
TaskCreate(
    subject="...",
    description="...",
    artifacts=[
        {"type": "file", "path": "core/src/lexer.rs"},
        {"type": "code_snippet", "content": "..."},
        {"type": "url", "url": "https://..."}
    ]
)
```

**Use Case**:
- Creating task "Extend lexer with tool keyword"
- Want to attach: lexer.rs file path, example code snippet
- Currently: Must embed in description text (loses structure)

#### 4. Task Templates (LOW PRIORITY)
**Problem**: Repetitive task creation for common workflows.

**Example**: Day 3-7 contest work follows predictable pattern:
- Understand requirements
- Fix blockers
- Implement feature
- Test feature
- Document feature

**Proposed Solution**:
```python
# New MCP Tool: task_from_template
def task_from_template(
    template: str,  # "feature_implementation", "bug_fix", etc.
    context: dict  # Fill template variables
) -> List[str]:  # Returns created task IDs
    """Creates task tree from template"""
```

#### 5. Real-Time Task Monitoring Dashboard (LOW PRIORITY)
**Problem**: Long-running agent tasks have no progress indicator beyond activeForm.

**Proposed Solution**:
- WebSocket stream from flukebase_connect on port 8766
- Push task status updates in real-time
- CLI tool: `fbc tasks watch` - live dashboard

#### 6. Task Handoff & Agent Assignment (HIGH PRIORITY)
**Problem**: When delegating to sub-agents, no clear task ownership tracking.

**Current State**:
- Main agent creates tasks
- Spawns Task tool (subagent) to work on something
- No link between the task and the running subagent

**Proposed Solution**:
```python
TaskUpdate(
    taskId="...",
    assigned_to="agent_id_or_name",
    execution_context={
        "agent_type": "Explore",
        "agent_id": "ad9c9b7",
        "started_at": "2026-02-03T18:20:00Z"
    }
)
```

**Use Case**:
- Main agent creates Task #3
- Spawns Plan agent to design implementation
- Links task to agent so monitoring shows: "Task #3: In Progress by Plan Agent (ad9c9b7)"

#### 7. Task Completion with Results (MEDIUM PRIORITY)
**Problem**: When task completes, no structured way to capture outputs/results.

**Current State**:
```python
TaskUpdate(taskId="...", status="completed")  # Just marks done
```

**Proposed Solution**:
```python
TaskUpdate(
    taskId="...",
    status="completed",
    result={
        "files_changed": ["lexer.rs", "parser.rs"],
        "tests_added": 3,
        "summary": "Added tool keyword support"
    }
)
```

### Developer Experience Issues 🐛

#### 1. No `TaskGet` Tool
- Can't retrieve task details after creation
- Must query DB directly: `sqlite3 ~/.flukebase-connect/wedo.db "SELECT * FROM tasks"`

#### 2. Task IDs Not Returned by TaskCreate
- TaskCreate says "Task #3 created successfully"
- But doesn't return `task_id` in result
- Hard to programmatically chain task creation → task update

#### 3. No Task Search/Filter
- 100 tasks in DB
- Want to find: "All pending tasks tagged 'aria-lang'"
- No MCP tool for this

### Recommended Implementation Priority

**Phase 1 (This Week)**:
1. ✅ `task_list` - Visualize task tree
2. ✅ `task_get` - Retrieve task details
3. ✅ TaskCreate returns task_id

**Phase 2 (Next Week)**:
4. Dependencies at creation time
5. Artifact attachment
6. Task handoff/agent assignment

**Phase 3 (Later)**:
7. Templates
8. Real-time dashboard
9. Advanced search/filtering

---

## Testing Approach

### Dogfooding Method
1. ✅ Use WeDo for all aria-lang contest work (Day 2-7)
2. ✅ Document pain points in real-time (this file)
3. Create flukebase_connect issues/PRs for each improvement
4. Implement top 3 highest-impact changes
5. Re-test during next project

### Success Metrics
- **Before**: Manual task tracking, lost context, unclear dependencies
- **After**: Clear task tree, automated handoffs, structured artifacts
- **Goal**: 50% reduction in "What was I working on?" moments

---

## Next Steps

1. ✅ Finish aria-lang Day 3 work (validate dogfooding findings)
2. Create GitHub issues in flukebase_connect repo for top 3 priorities
3. Implement Phase 1 improvements
4. Re-dogfood during aria-lang Day 4-7

**Date**: 2026-02-03
**Session**: Aria-Lang Contest Day 3
**Dogfooder**: Claude Sonnet 4.5 via Claude Code CLI
