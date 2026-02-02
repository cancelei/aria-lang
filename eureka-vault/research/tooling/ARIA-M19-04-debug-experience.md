# ARIA-M19-04: Debug Information and Debugging Experience

**Task ID**: ARIA-M19-04
**Status**: Completed
**Date**: 2026-01-15
**Author**: TRACE (Eureka Research Agent)
**Focus**: Comprehensive debugging infrastructure design for Aria

---

## Executive Summary

This research document establishes Aria's debugging infrastructure strategy, covering DWARF debug information generation, source maps for interpreted/JIT modes, effect-aware debugging, contract debugging, async/concurrent debugging, debugger integration (GDB, LLDB, VSCode), profiling support, and memory debugging. The design leverages proven approaches from Rust, Go, and modern debugging tools while introducing novel features for Aria's unique effect system and contracts.

**Key Recommendations**:
1. **DWARF 5 generation** via Cranelift/LLVM with Aria-specific extensions
2. **Debug Adapter Protocol (DAP)** as primary IDE integration layer
3. **Effect-aware debugging** with handler stack visualization and resumption point stepping
4. **Contract debugging** with evaluation tracing and failure location pinpointing
5. **Async debugging** with task state visualization and deadlock detection
6. **Integrated profiling** supporting sampling, instrumentation, and effect-aware tracing

---

## 1. DWARF Debug Information Generation

### 1.1 Overview

DWARF (Debugging With Attributed Record Formats) is the standard debugging format on macOS, Linux, and most Unix-like systems. Aria must generate high-quality DWARF information to enable source-level debugging with standard debuggers.

### 1.2 DWARF Version Strategy

| Version | Support Status | Key Features |
|---------|---------------|--------------|
| DWARF 4 | Baseline | Type deduplication via `.debug_types`, call frame info |
| DWARF 5 | **Primary Target** | Split DWARF, MD5 checksums, improved compression |
| DWARF 6 | Future | Under development, enhanced heterogeneous debugging |

**Decision**: Target DWARF 5 as primary format with DWARF 4 fallback for compatibility.

### 1.3 Debug Information Architecture

```
ARIA DEBUG INFO PIPELINE

+------------------+     +------------------+     +------------------+
|   Aria Source    | --> |   Aria MIR       | --> |   Cranelift IR   |
|   with spans     |     |   with debug     |     |   with DI        |
+------------------+     +------------------+     +------------------+
                                                          |
                                                          v
                                               +------------------+
                                               |   DWARF Sections |
                                               |   .debug_info    |
                                               |   .debug_line    |
                                               |   .debug_abbrev  |
                                               |   .debug_str     |
                                               +------------------+
```

### 1.4 MIR Debug Information Representation

```rust
/// Debug information attached to MIR instructions
pub struct MirDebugInfo {
    /// Source location span
    pub span: SourceSpan,
    /// Lexical scope ID
    pub scope: ScopeId,
    /// Variable bindings in scope
    pub locals: Vec<LocalDebugInfo>,
    /// Effect context (handler stack)
    pub effect_context: Option<EffectDebugContext>,
    /// Contract context (active contracts)
    pub contract_context: Option<ContractDebugContext>,
}

/// Debug info for local variables
pub struct LocalDebugInfo {
    pub name: Symbol,
    pub ty: TypeId,
    /// Storage location (register, stack slot, etc.)
    pub location: DebugLocation,
    /// Validity range within function
    pub range: InstructionRange,
}

/// Location where variable value is stored
pub enum DebugLocation {
    /// CPU register
    Register(RegisterId),
    /// Stack frame offset
    StackSlot { offset: i32, size: u32 },
    /// Split across multiple locations
    Composite(Vec<DebugLocation>),
    /// Optimized out (value not available)
    Unavailable,
}
```

### 1.5 Cranelift Debug Info Lowering

Following Rust's approach, Aria generates debug info through Cranelift's DIBuilder:

```rust
/// Lower Aria MIR debug info to Cranelift
pub struct DebugInfoLowering<'a> {
    /// Cranelift function builder context
    ctx: &'a mut FunctionBuilderContext,
    /// Debug info builder
    di_builder: DIBuilder,
    /// Scope stack for lexical scoping
    scope_stack: Vec<DIScope>,
    /// Current source location
    current_location: Option<SourceLocation>,
}

impl DebugInfoLowering<'_> {
    /// Set current source location for subsequent instructions
    pub fn set_source_location(&mut self, span: SourceSpan) {
        let location = SourceLocation {
            file: self.file_id(span.file),
            line: span.start_line,
            column: span.start_col,
        };
        self.di_builder.set_location(location);
        self.current_location = Some(location);
    }

    /// Create debug info for local variable
    pub fn create_local_variable(
        &mut self,
        local: &LocalDebugInfo,
        scope: DIScope,
    ) -> DILocalVariable {
        let ty = self.lower_type_debug_info(local.ty);

        self.di_builder.create_local_variable(
            scope,
            local.name.as_str(),
            ty,
            self.lower_location(&local.location),
        )
    }

    /// Create custom DWARF type for Aria effect handler
    pub fn create_effect_handler_type(
        &mut self,
        handler: &HandlerType,
    ) -> DIType {
        // Create structure type representing handler
        let fields = vec![
            self.di_builder.create_member("effect_id", self.u32_type(), 0),
            self.di_builder.create_member("vtable", self.ptr_type(), 8),
            self.di_builder.create_member("parent", self.ptr_type(), 16),
            self.di_builder.create_member("evidence_slot", self.u32_type(), 24),
        ];

        self.di_builder.create_struct_type(
            &format!("aria.Handler<{}>", handler.effect.name),
            fields,
            32,  // size
            8,   // alignment
        )
    }
}
```

### 1.6 Split DWARF for Large Projects

For improved compilation and linking speed with large projects:

```toml
# Aria.toml configuration
[profile.debug]
split-dwarf = true      # Generate .dwo files
dwarf-single-file = false

[profile.release-with-debug]
debug-info = "full"
split-dwarf = true
```

**Benefits**:
- Faster linking (debug info not copied to final binary)
- Parallel debug info processing
- Reduced memory usage during linking

### 1.7 Debug Info Quality Levels

| Level | Flag | Content | Use Case |
|-------|------|---------|----------|
| None | `-g0` | No debug info | Release builds |
| Line Tables | `-g1` | Source locations only | Stack traces |
| Full | `-g` | All debug info | Full debugging |
| Optimized | `-g -O` | Debug info with optimization | Production debugging |

---

## 2. Source Maps for Interpreted/JIT Modes

### 2.1 Overview

For Aria's interpreted mode, REPL, and JIT compilation, source maps provide the mapping between generated code and original source.

### 2.2 Aria Source Map Format

```json
{
  "version": 1,
  "file": "main.aria",
  "sourceRoot": "/path/to/project",
  "sources": ["main.aria", "utils.aria"],
  "mappings": [
    {
      "generated": { "line": 10, "column": 0 },
      "original": { "line": 5, "column": 4, "source": 0 },
      "name": "process_data"
    }
  ],
  "names": ["process_data", "result", "item"],
  "ariaExtensions": {
    "effectMappings": [
      {
        "location": { "line": 12, "column": 8 },
        "effect": "IO",
        "operation": "print",
        "handlerStack": [
          { "effect": "IO", "handler": "default_io", "location": { "line": 3, "column": 2 } }
        ]
      }
    ],
    "contractMappings": [
      {
        "function": "process_data",
        "preconditions": [
          { "expression": "data.len() > 0", "location": { "line": 4, "column": 2 } }
        ]
      }
    ]
  }
}
```

### 2.3 JIT Debug Information

For Cranelift JIT, register generated code with debugger:

```rust
/// JIT debug info manager
pub struct JitDebugInfo {
    /// GDB JIT interface
    gdb_jit: Option<GdbJitDescriptor>,
    /// LLDB JIT interface
    lldb_jit: Option<LldbJitDescriptor>,
    /// Generated code regions
    code_regions: Vec<JitCodeRegion>,
}

impl JitDebugInfo {
    /// Register JIT-compiled function with debugger
    pub fn register_function(
        &mut self,
        func_name: &str,
        code_ptr: *const u8,
        code_size: usize,
        debug_info: &DwarfDebugInfo,
    ) {
        let region = JitCodeRegion {
            name: func_name.to_string(),
            start: code_ptr,
            size: code_size,
            dwarf: debug_info.clone(),
        };

        self.code_regions.push(region);

        // Notify GDB via __jit_debug_register_code
        if let Some(ref mut gdb) = self.gdb_jit {
            gdb.register(&region);
        }
    }
}

/// GDB JIT descriptor (C ABI compatible)
#[repr(C)]
struct JitCodeEntry {
    next_entry: *mut JitCodeEntry,
    prev_entry: *mut JitCodeEntry,
    symfile_addr: *const u8,
    symfile_size: u64,
}
```

### 2.4 WASM Source Map Integration

For WebAssembly targets, generate standard WASM source maps:

```rust
/// Generate WASM source map
pub fn generate_wasm_source_map(
    module: &WasmModule,
    aria_sources: &SourceFiles,
) -> WasmSourceMap {
    let mut map = WasmSourceMap::new();

    for func in module.functions() {
        for (wasm_offset, aria_location) in func.debug_mappings() {
            map.add_mapping(
                wasm_offset,
                aria_location.file,
                aria_location.line,
                aria_location.column,
            );
        }
    }

    // Embed source map URL in custom section
    map.set_url("main.aria.map");

    map
}
```

---

## 3. Effect-Aware Debugging

### 3.1 The Challenge

Algebraic effects introduce unique debugging challenges:
- **Non-local control flow**: Effects can suspend and resume execution
- **Handler stacks**: Multiple handlers may be active simultaneously
- **Resumption points**: Execution can resume at captured continuation points
- **Multi-shot continuations**: Same continuation may be invoked multiple times

### 3.2 Effect Handler Stack Visualization

```
EFFECT STACK VISUALIZATION

Current execution point: line 42 in process_data()

Handler Stack (innermost first):
+--------+------------------+----------------------+-------------+
| Depth  | Effect           | Handler              | Location    |
+--------+------------------+----------------------+-------------+
| 0      | Exception[DbErr] | db_error_handler     | main.aria:15|
| 1      | State[Config]    | config_handler       | main.aria:8 |
| 2      | Async            | runtime_async        | <runtime>   |
| 3      | IO               | default_io           | <runtime>   |
+--------+------------------+----------------------+-------------+

Active Effect Operations:
  - State.get() at line 42 (tail-resumptive, will return directly)
  - Async.await at line 38 (suspended, awaiting HTTP response)
```

### 3.3 Stepping Through Effect Handlers

**New Stepping Modes for Effects**:

| Command | Behavior |
|---------|----------|
| `step` | Step into effect operation (enter handler) |
| `step-over-effect` | Treat effect operation as single step |
| `step-to-resume` | Run until continuation resumes |
| `step-out-handler` | Step out of current handler |

```rust
/// Effect-aware stepping controller
pub struct EffectStepController {
    /// Current stepping mode
    mode: StepMode,
    /// Handler stack at step start
    handler_stack_depth: usize,
    /// Resumption point breakpoints
    resume_breakpoints: Vec<ResumeBreakpoint>,
}

pub enum StepMode {
    /// Standard source-level step
    Normal,
    /// Step into effect handler
    IntoEffect,
    /// Step over effect (treat as atomic)
    OverEffect,
    /// Step to next resumption
    ToResume,
    /// Step out of handler
    OutOfHandler,
}

impl EffectStepController {
    /// Handle effect operation during stepping
    pub fn on_effect_perform(
        &mut self,
        effect: &EffectType,
        handler: &HandlerInfo,
    ) -> StepAction {
        match self.mode {
            StepMode::Normal | StepMode::IntoEffect => {
                // Stop at handler entry
                StepAction::Stop
            }
            StepMode::OverEffect => {
                // Set breakpoint at resume point, continue
                self.set_resume_breakpoint(handler);
                StepAction::Continue
            }
            StepMode::ToResume => {
                // Continue to next resume
                StepAction::Continue
            }
            StepMode::OutOfHandler => {
                if handler.depth < self.handler_stack_depth {
                    StepAction::Stop
                } else {
                    StepAction::Continue
                }
            }
        }
    }
}
```

### 3.4 Resumption Point Debugging

For debugging multi-shot continuations:

```rust
/// Continuation debug info
pub struct ContinuationDebugInfo {
    /// Unique continuation ID
    pub id: ContinuationId,
    /// Capture location
    pub capture_point: SourceLocation,
    /// Resume locations (for multi-shot)
    pub resume_points: Vec<ResumePoint>,
    /// Captured variable values
    pub captured_state: HashMap<Symbol, DebugValue>,
    /// Handler context at capture
    pub handler_context: HandlerStackSnapshot,
}

/// Resume point tracking
pub struct ResumePoint {
    /// Where resume was called
    pub call_site: SourceLocation,
    /// Value passed to resume
    pub resume_value: DebugValue,
    /// Timestamp (for ordering)
    pub timestamp: u64,
    /// Whether this invocation completed
    pub status: ResumeStatus,
}

pub enum ResumeStatus {
    /// Currently executing
    Active,
    /// Completed successfully
    Completed(DebugValue),
    /// Raised another effect
    Suspended,
    /// Threw exception
    Failed(ExceptionInfo),
}
```

### 3.5 Effect Debugging DAP Extensions

Custom DAP requests for effect debugging:

```typescript
// DAP extension: Get effect handler stack
interface EffectStackRequest extends Request {
    command: 'aria/effectStack';
    arguments: { threadId: number };
}

interface EffectStackResponse extends Response {
    body: {
        handlers: Array<{
            depth: number;
            effect: string;
            handlerName: string;
            location: Source;
            operations: string[];
        }>;
    };
}

// DAP extension: Get active continuations
interface ContinuationsRequest extends Request {
    command: 'aria/continuations';
    arguments: { threadId: number };
}

interface ContinuationsResponse extends Response {
    body: {
        continuations: Array<{
            id: string;
            captureLocation: Source;
            status: 'active' | 'suspended' | 'completed';
            resumeCount: number;
            capturedVariables: Variable[];
        }>;
    };
}

// DAP extension: Step to resume
interface StepToResumeRequest extends Request {
    command: 'aria/stepToResume';
    arguments: {
        threadId: number;
        continuationId?: string;
    };
}
```

### 3.6 Effect Stack in Stack Traces

Enhanced stack traces showing effect context:

```
Exception: DatabaseError("Connection refused")
  at db_query (database.aria:45)
  at fetch_user (users.aria:23)
  |-- handling Exception[DatabaseError] at main.aria:15
  at process_request (server.aria:78)
  |-- handling State[RequestContext] at server.aria:60
  at async_handler (runtime.aria:120)
  |-- handling Async at runtime.aria:100
  at main (main.aria:5)
```

---

## 4. Contract Debugging

### 4.1 Overview

Aria's contract system (preconditions, postconditions, invariants) requires specialized debugging support to:
- Locate exact contract failure points
- Show contract expression evaluation
- Display counterexample values
- Enable contract evaluation stepping

### 4.2 Contract Failure Information

```rust
/// Contract failure debug information
pub struct ContractFailure {
    /// Type of contract that failed
    pub kind: ContractKind,
    /// The contract expression that failed
    pub expression: String,
    /// Source location of contract
    pub location: SourceLocation,
    /// Function the contract belongs to
    pub function: FunctionId,
    /// Variable values at failure
    pub variable_bindings: HashMap<Symbol, DebugValue>,
    /// Subexpression values for complex contracts
    pub subexpressions: Vec<SubexpressionValue>,
    /// Call stack at failure
    pub call_stack: Vec<StackFrame>,
}

pub enum ContractKind {
    /// Function precondition (requires)
    Precondition,
    /// Function postcondition (ensures)
    Postcondition,
    /// Type/class invariant
    Invariant,
    /// Loop invariant
    LoopInvariant,
    /// Assert statement
    Assert,
}

/// Subexpression evaluation trace
pub struct SubexpressionValue {
    /// The subexpression text
    pub expression: String,
    /// Its evaluated value
    pub value: DebugValue,
    /// Position within parent expression
    pub span: Span,
}
```

### 4.3 Contract Evaluation Tracing

For complex contracts, provide step-by-step evaluation:

```
Contract Failure: Precondition violated
  Function: transfer_funds(from, to, amount)
  Location: banking.aria:42

  Contract: requires from.balance >= amount && amount > 0 && to.is_active

  Evaluation trace:
    1. from.balance >= amount
       from.balance = 150.00
       amount = 200.00
       Result: false  <-- FAILURE

    2. amount > 0
       amount = 200.00
       Result: true (short-circuited, not evaluated due to &&)

    3. to.is_active
       (not evaluated due to short-circuit)

  Suggestion: The amount (200.00) exceeds the available balance (150.00)
```

### 4.4 Contract Debugging Implementation

```rust
/// Contract evaluation tracer
pub struct ContractTracer {
    /// Enable detailed tracing
    trace_enabled: bool,
    /// Collected subexpression values
    trace_entries: Vec<TraceEntry>,
    /// Current evaluation depth
    depth: usize,
}

impl ContractTracer {
    /// Evaluate contract with tracing
    pub fn evaluate_contract(
        &mut self,
        contract: &ContractExpr,
        env: &Environment,
    ) -> ContractResult {
        self.trace_entries.clear();
        self.depth = 0;

        let result = self.eval_expr(&contract.expression, env);

        if !result.as_bool() {
            ContractResult::Failed(ContractFailure {
                kind: contract.kind,
                expression: contract.expression.to_string(),
                location: contract.location,
                function: contract.function,
                variable_bindings: self.capture_bindings(env, &contract.expression),
                subexpressions: self.trace_entries.clone(),
                call_stack: self.capture_call_stack(),
            })
        } else {
            ContractResult::Passed
        }
    }

    fn eval_expr(&mut self, expr: &Expr, env: &Environment) -> Value {
        self.depth += 1;

        let value = match expr {
            Expr::Binary { op, left, right, .. } => {
                let left_val = self.eval_expr(left, env);

                // Short-circuit evaluation
                if self.is_short_circuit_op(op, &left_val) {
                    self.record_trace(expr, left_val.clone(), true);
                    return left_val;
                }

                let right_val = self.eval_expr(right, env);
                let result = self.apply_op(op, &left_val, &right_val);
                self.record_trace(expr, result.clone(), false);
                result
            }
            Expr::FieldAccess { base, field, .. } => {
                let base_val = self.eval_expr(base, env);
                let result = base_val.get_field(field);
                self.record_trace(expr, result.clone(), false);
                result
            }
            Expr::Variable { name, .. } => {
                let value = env.get(name).clone();
                self.record_trace(expr, value.clone(), false);
                value
            }
            // ... other expression types
        };

        self.depth -= 1;
        value
    }

    fn record_trace(&mut self, expr: &Expr, value: Value, short_circuited: bool) {
        self.trace_entries.push(TraceEntry {
            expression: expr.to_string(),
            value: value.to_debug_value(),
            span: expr.span(),
            depth: self.depth,
            short_circuited,
        });
    }
}
```

### 4.5 Postcondition Debugging with `old()` Values

For postconditions that reference pre-call values:

```aria
fn update_balance(account: &mut Account, delta: Int) -> Int
  requires account.is_active
  ensures result == old(account.balance) + delta
  ensures account.balance == result

  account.balance += delta
  account.balance
end
```

Debug information captures `old()` values:

```rust
/// Postcondition debug context
pub struct PostconditionContext {
    /// Values of old() expressions at function entry
    pub old_values: HashMap<OldExpr, DebugValue>,
    /// Return value
    pub return_value: DebugValue,
    /// Modified variables
    pub modified_vars: HashMap<Symbol, (DebugValue, DebugValue)>,  // (before, after)
}
```

### 4.6 Contract Breakpoints

Specialized breakpoints for contracts:

```rust
/// Contract-specific breakpoint types
pub enum ContractBreakpoint {
    /// Break on any contract failure
    OnAnyFailure,
    /// Break on specific contract kind failure
    OnKindFailure(ContractKind),
    /// Break on specific function's contract failure
    OnFunctionContract(FunctionId),
    /// Break before contract evaluation
    BeforeEvaluation { function: FunctionId, contract_index: usize },
    /// Break during contract evaluation (step through)
    StepEvaluation { function: FunctionId, contract_index: usize },
}
```

---

## 5. Async/Concurrent Debugging

### 5.1 Overview

Aria's concurrency model (spawn, channels, structured concurrency) requires specialized debugging support:
- Task state visualization
- Channel state inspection
- Deadlock detection
- Async stack traces

### 5.2 Task State Visualization

```
TASK OVERVIEW

Active Tasks: 12 | Suspended: 5 | Completed: 147 | Failed: 2

+----------+------------------+----------+-------------------+-------------+
| Task ID  | Name             | State    | Current Location  | Runtime     |
+----------+------------------+----------+-------------------+-------------+
| task-001 | main             | Running  | main.aria:45      | 1.234s      |
| task-002 | http-handler-1   | Suspended| server.aria:78    | 0.456s      |
|          |                  |          | (awaiting IO)     |             |
| task-003 | http-handler-2   | Running  | database.aria:23  | 0.123s      |
| task-004 | background-sync  | Suspended| sync.aria:100     | 5.678s      |
|          |                  |          | (awaiting channel)|             |
| task-005 | worker-1         | Blocked  | worker.aria:55    | 2.345s      |
|          |                  |          | (deadlock risk!)  |             |
+----------+------------------+----------+-------------------+-------------+

Task Hierarchy:
task-001 (main)
├── task-002 (http-handler-1) [suspended]
├── task-003 (http-handler-2) [running]
├── scope-1 (Async.scope)
│   ├── task-006 (fetch-1) [completed]
│   ├── task-007 (fetch-2) [completed]
│   └── task-008 (fetch-3) [suspended]
└── task-004 (background-sync) [suspended]
```

### 5.3 Channel State Inspection

```rust
/// Channel debug information
pub struct ChannelDebugInfo {
    /// Channel unique ID
    pub id: ChannelId,
    /// Element type
    pub element_type: TypeId,
    /// Capacity (None for unbounded)
    pub capacity: Option<usize>,
    /// Current item count
    pub current_count: usize,
    /// Pending senders
    pub pending_senders: Vec<TaskId>,
    /// Pending receivers
    pub pending_receivers: Vec<TaskId>,
    /// Is channel closed?
    pub is_closed: bool,
    /// Recent send/receive operations
    pub recent_operations: VecDeque<ChannelOperation>,
}

pub struct ChannelOperation {
    pub kind: ChannelOpKind,
    pub task: TaskId,
    pub timestamp: Instant,
    pub value_preview: Option<String>,  // First 100 chars of debug repr
}

pub enum ChannelOpKind {
    Send,
    Receive,
    TrySend(bool),    // success
    TryReceive(bool), // success
    Close,
}
```

Channel visualization:

```
CHANNEL INSPECTION: data_channel (Channel[ProcessedData], cap=100)

Status: Open, 45/100 items
Pending: 0 senders, 2 receivers

Waiting Tasks:
  - task-010 (worker-3): blocked on recv() for 1.234s
  - task-011 (worker-4): blocked on recv() for 0.567s

Recent Operations:
  [12:34:56.789] task-007 -> SEND  (ProcessedData { id: 1234, ... })
  [12:34:56.790] task-010 <- RECV  (ProcessedData { id: 1233, ... })
  [12:34:56.792] task-008 -> SEND  (ProcessedData { id: 1235, ... })
```

### 5.4 Deadlock Detection

```rust
/// Deadlock detector for Aria's concurrency primitives
pub struct DeadlockDetector {
    /// Wait-for graph: task -> resources it's waiting for
    wait_graph: HashMap<TaskId, Vec<ResourceId>>,
    /// Resource holders: resource -> task holding it
    resource_holders: HashMap<ResourceId, TaskId>,
    /// Channel wait relationships
    channel_waits: HashMap<ChannelId, ChannelWaitState>,
}

pub enum ResourceId {
    Channel(ChannelId),
    Mutex(MutexId),
    Task(TaskId),  // Waiting for task completion
}

pub struct ChannelWaitState {
    pub waiting_senders: Vec<TaskId>,
    pub waiting_receivers: Vec<TaskId>,
}

impl DeadlockDetector {
    /// Check for deadlock cycles
    pub fn detect_deadlock(&self) -> Option<DeadlockInfo> {
        // Build wait-for graph
        let mut graph: HashMap<TaskId, Vec<TaskId>> = HashMap::new();

        for (waiting_task, resources) in &self.wait_graph {
            for resource in resources {
                if let Some(holder) = self.resource_holders.get(resource) {
                    graph.entry(*waiting_task)
                        .or_default()
                        .push(*holder);
                }
            }
        }

        // Find cycles using DFS
        self.find_cycle(&graph)
    }

    fn find_cycle(&self, graph: &HashMap<TaskId, Vec<TaskId>>) -> Option<DeadlockInfo> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for &start in graph.keys() {
            if let Some(cycle) = self.dfs_find_cycle(
                start, graph, &mut visited, &mut rec_stack, &mut path
            ) {
                return Some(DeadlockInfo {
                    cycle,
                    detection_time: Instant::now(),
                    wait_durations: self.get_wait_durations(&cycle),
                });
            }
        }

        None
    }
}

/// Deadlock information for debugger
pub struct DeadlockInfo {
    /// Tasks involved in the cycle
    pub cycle: Vec<TaskId>,
    /// When deadlock was detected
    pub detection_time: Instant,
    /// How long each task has been waiting
    pub wait_durations: HashMap<TaskId, Duration>,
}
```

### 5.5 Async Stack Traces

Reconstruct logical call stacks across async boundaries:

```rust
/// Async-aware stack trace builder
pub struct AsyncStackTraceBuilder {
    /// Task-local stack frames
    task_frames: HashMap<TaskId, Vec<StackFrame>>,
    /// Suspension points
    suspension_points: HashMap<TaskId, SuspensionInfo>,
    /// Task spawn relationships
    spawn_tree: HashMap<TaskId, TaskId>,  // child -> parent
}

impl AsyncStackTraceBuilder {
    /// Build complete async stack trace
    pub fn build_async_trace(&self, task: TaskId) -> AsyncStackTrace {
        let mut trace = AsyncStackTrace::new();

        // Add current task's frames
        if let Some(frames) = self.task_frames.get(&task) {
            trace.add_section(AsyncTraceSection {
                task,
                frames: frames.clone(),
                suspension: self.suspension_points.get(&task).cloned(),
            });
        }

        // Walk up the spawn tree
        let mut current = task;
        while let Some(&parent) = self.spawn_tree.get(&current) {
            if let Some(frames) = self.task_frames.get(&parent) {
                trace.add_section(AsyncTraceSection {
                    task: parent,
                    frames: frames.clone(),
                    suspension: self.suspension_points.get(&parent).cloned(),
                });
            }
            current = parent;
        }

        trace
    }
}

/// Async stack trace output
pub struct AsyncStackTrace {
    pub sections: Vec<AsyncTraceSection>,
}

pub struct AsyncTraceSection {
    pub task: TaskId,
    pub frames: Vec<StackFrame>,
    pub suspension: Option<SuspensionInfo>,
}
```

Formatted output:

```
Async Stack Trace:

[Task: http-handler-3 (running)]
  at process_request (handler.aria:45)
  at validate_input (validation.aria:23)

[Task: http-handler-3 (suspended at handler.aria:50)]
  -- awaiting: fetch_user_data() --
  at fetch_user (database.aria:78)
  at db_query (database.aria:120)

[Task: main (spawn point: server.aria:30)]
  at accept_connection (server.aria:55)
  at server_loop (server.aria:25)
  at main (main.aria:10)
```

### 5.6 Parallel Tasks Window (DAP)

```typescript
// DAP extension: Get parallel tasks
interface ParallelTasksRequest extends Request {
    command: 'aria/parallelTasks';
}

interface ParallelTasksResponse extends Response {
    body: {
        tasks: Array<{
            id: string;
            name: string;
            state: 'running' | 'suspended' | 'blocked' | 'completed' | 'failed';
            parentId?: string;
            scopeId?: string;
            location: Source;
            runtime: number;  // milliseconds
            suspensionReason?: string;
            channels?: string[];  // channels this task is waiting on
        }>;
        scopes: Array<{
            id: string;
            type: 'scope' | 'supervisor' | 'nursery';
            taskCount: number;
            pendingCount: number;
            failedCount: number;
        }>;
        deadlocks?: Array<{
            cycle: string[];
            message: string;
        }>;
    };
}
```

---

## 6. Debugger Integration

### 6.1 Architecture Overview

```
ARIA DEBUGGING ARCHITECTURE

+------------------+     +------------------+     +------------------+
|   VS Code        |     |   Neovim/DAP     |     |   JetBrains      |
|   (DAP Client)   |     |   (DAP Client)   |     |   (DAP Client)   |
+--------+---------+     +--------+---------+     +--------+---------+
         |                        |                        |
         +------------------------+------------------------+
                                  |
                                  v
                    +----------------------------+
                    |   Aria Debug Adapter       |
                    |   (aria-debugger)          |
                    +-------------+--------------+
                                  |
         +------------------------+------------------------+
         |                        |                        |
         v                        v                        v
+------------------+     +------------------+     +------------------+
|   LLDB Backend   |     |   GDB Backend    |     |   Aria Runtime   |
|   (Native/macOS) |     |   (Native/Linux) |     |   (Interpreted)  |
+------------------+     +------------------+     +------------------+
```

### 6.2 Debug Adapter Protocol (DAP) Implementation

**Primary integration layer**: Implement DAP for broad IDE support.

```rust
/// Aria Debug Adapter main entry point
pub struct AriaDebugAdapter {
    /// Active debug session
    session: Option<DebugSession>,
    /// Backend connection (LLDB/GDB/Runtime)
    backend: Box<dyn DebugBackend>,
    /// Aria-specific extensions
    aria_extensions: AriaDebugExtensions,
}

impl AriaDebugAdapter {
    /// Handle DAP request
    pub async fn handle_request(&mut self, request: Request) -> Response {
        match request.command.as_str() {
            // Standard DAP commands
            "initialize" => self.initialize(request).await,
            "launch" => self.launch(request).await,
            "attach" => self.attach(request).await,
            "setBreakpoints" => self.set_breakpoints(request).await,
            "continue" => self.continue_(request).await,
            "next" => self.next(request).await,
            "stepIn" => self.step_in(request).await,
            "stepOut" => self.step_out(request).await,
            "threads" => self.threads(request).await,
            "stackTrace" => self.stack_trace(request).await,
            "scopes" => self.scopes(request).await,
            "variables" => self.variables(request).await,
            "evaluate" => self.evaluate(request).await,

            // Aria-specific extensions
            "aria/effectStack" => self.effect_stack(request).await,
            "aria/continuations" => self.continuations(request).await,
            "aria/parallelTasks" => self.parallel_tasks(request).await,
            "aria/contractInfo" => self.contract_info(request).await,
            "aria/channelState" => self.channel_state(request).await,
            "aria/stepToResume" => self.step_to_resume(request).await,
            "aria/stepOverEffect" => self.step_over_effect(request).await,

            _ => self.unknown_command(request),
        }
    }
}

/// Trait for debug backends
pub trait DebugBackend: Send {
    /// Launch debugee process
    fn launch(&mut self, program: &Path, args: &[String]) -> Result<ProcessId>;

    /// Attach to running process
    fn attach(&mut self, pid: u32) -> Result<()>;

    /// Set breakpoint at location
    fn set_breakpoint(&mut self, location: &BreakpointLocation) -> Result<BreakpointId>;

    /// Continue execution
    fn continue_(&mut self) -> Result<()>;

    /// Single step
    fn step(&mut self, mode: StepMode) -> Result<()>;

    /// Get current threads
    fn threads(&self) -> Result<Vec<ThreadInfo>>;

    /// Get stack trace for thread
    fn stack_trace(&self, thread: ThreadId) -> Result<Vec<StackFrame>>;

    /// Read variable value
    fn read_variable(&self, frame: FrameId, name: &str) -> Result<DebugValue>;

    /// Evaluate expression
    fn evaluate(&mut self, frame: FrameId, expr: &str) -> Result<DebugValue>;
}
```

### 6.3 LLDB Integration

```rust
/// LLDB backend implementation
pub struct LldbBackend {
    /// LLDB debugger instance
    debugger: SBDebugger,
    /// Current target
    target: Option<SBTarget>,
    /// Current process
    process: Option<SBProcess>,
    /// Aria type formatters
    type_formatters: AriaTypeFormatters,
    /// Effect debugging extensions
    effect_debug: EffectDebugSupport,
}

impl LldbBackend {
    /// Initialize LLDB with Aria support
    pub fn new() -> Self {
        SBDebugger::initialize();
        let debugger = SBDebugger::create(false);

        // Load Aria type formatters
        let formatters = AriaTypeFormatters::load(&debugger);

        LldbBackend {
            debugger,
            target: None,
            process: None,
            type_formatters: formatters,
            effect_debug: EffectDebugSupport::new(),
        }
    }

    /// Register Aria-specific type summaries
    fn register_type_summaries(&self) {
        // Register summary for Aria String
        self.debugger.handle_command(
            r#"type summary add -F aria_formatters.string_summary "aria::String""#
        );

        // Register summary for Aria Array
        self.debugger.handle_command(
            r#"type summary add -F aria_formatters.array_summary -x "aria::Array<.*>""#
        );

        // Register summary for effect handlers
        self.debugger.handle_command(
            r#"type summary add -F aria_formatters.handler_summary -x "aria::Handler<.*>""#
        );

        // Register summary for channels
        self.debugger.handle_command(
            r#"type summary add -F aria_formatters.channel_summary -x "aria::Channel<.*>""#
        );
    }
}

impl DebugBackend for LldbBackend {
    fn launch(&mut self, program: &Path, args: &[String]) -> Result<ProcessId> {
        let target = self.debugger.create_target(
            program.to_str().unwrap(),
            None, None, true,
        )?;

        let launch_info = SBLaunchInfo::new();
        launch_info.set_arguments(args);

        let process = target.launch(&launch_info)?;

        self.target = Some(target);
        self.process = Some(process);

        Ok(process.get_process_id())
    }

    fn stack_trace(&self, thread_id: ThreadId) -> Result<Vec<StackFrame>> {
        let process = self.process.as_ref().ok_or(Error::NoProcess)?;
        let thread = process.get_thread_by_id(thread_id.0)?;

        let mut frames = Vec::new();
        for i in 0..thread.get_num_frames() {
            let sb_frame = thread.get_frame_at_index(i);

            // Check for effect handler frames
            let is_effect_handler = self.effect_debug
                .is_handler_frame(&sb_frame);

            frames.push(StackFrame {
                id: FrameId(sb_frame.get_frame_id()),
                name: sb_frame.get_function_name().to_string(),
                source: self.get_source_location(&sb_frame),
                line: sb_frame.get_line_entry().get_line() as u32,
                column: sb_frame.get_line_entry().get_column() as u32,
                aria_metadata: AriaFrameMetadata {
                    is_effect_handler,
                    effect_info: if is_effect_handler {
                        self.effect_debug.get_handler_info(&sb_frame)
                    } else {
                        None
                    },
                },
            });
        }

        Ok(frames)
    }
}
```

### 6.4 GDB Integration

```rust
/// GDB/MI backend implementation
pub struct GdbBackend {
    /// GDB/MI connection
    mi_connection: GdbMiConnection,
    /// Process ID
    pid: Option<u32>,
    /// Aria runtime support
    runtime_support: AriaRuntimeSupport,
}

impl GdbBackend {
    /// Initialize GDB with Aria pretty printers
    pub fn new() -> Result<Self> {
        let mut conn = GdbMiConnection::spawn()?;

        // Load Aria GDB support scripts
        conn.execute("-interpreter-exec console \"source aria-gdb.py\"")?;

        Ok(GdbBackend {
            mi_connection: conn,
            pid: None,
            runtime_support: AriaRuntimeSupport::new(),
        })
    }
}

impl DebugBackend for GdbBackend {
    fn launch(&mut self, program: &Path, args: &[String]) -> Result<ProcessId> {
        self.mi_connection.execute(&format!(
            "-file-exec-and-symbols {}",
            program.display()
        ))?;

        if !args.is_empty() {
            self.mi_connection.execute(&format!(
                "-exec-arguments {}",
                args.join(" ")
            ))?;
        }

        let response = self.mi_connection.execute("-exec-run")?;

        // Extract PID from response
        let pid = self.extract_pid(&response)?;
        self.pid = Some(pid);

        Ok(ProcessId(pid as u64))
    }

    fn evaluate(&mut self, frame: FrameId, expr: &str) -> Result<DebugValue> {
        // Select frame
        self.mi_connection.execute(&format!(
            "-stack-select-frame {}",
            frame.0
        ))?;

        // Evaluate expression
        let response = self.mi_connection.execute(&format!(
            "-data-evaluate-expression \"{}\"",
            expr.replace("\"", "\\\"")
        ))?;

        self.parse_gdb_value(&response)
    }
}
```

### 6.5 VS Code Extension

```typescript
// package.json (extension manifest)
{
  "name": "aria-debug",
  "displayName": "Aria Debugger",
  "version": "1.0.0",
  "engines": { "vscode": "^1.80.0" },
  "categories": ["Debuggers"],
  "contributes": {
    "debuggers": [{
      "type": "aria",
      "label": "Aria Debug",
      "program": "./out/debugAdapter.js",
      "runtime": "node",
      "configurationAttributes": {
        "launch": {
          "required": ["program"],
          "properties": {
            "program": {
              "type": "string",
              "description": "Path to Aria executable"
            },
            "args": {
              "type": "array",
              "description": "Command line arguments"
            },
            "cwd": {
              "type": "string",
              "description": "Working directory"
            },
            "effectDebugging": {
              "type": "boolean",
              "description": "Enable effect-aware debugging",
              "default": true
            },
            "contractChecking": {
              "type": "string",
              "enum": ["none", "preconditions", "all"],
              "default": "all"
            }
          }
        }
      }
    }],
    "views": {
      "debug": [
        {
          "id": "aria.effectStack",
          "name": "Effect Handlers",
          "when": "debugType == 'aria'"
        },
        {
          "id": "aria.parallelTasks",
          "name": "Parallel Tasks",
          "when": "debugType == 'aria'"
        },
        {
          "id": "aria.channels",
          "name": "Channels",
          "when": "debugType == 'aria'"
        }
      ]
    },
    "commands": [
      {
        "command": "aria.stepToResume",
        "title": "Step to Resume",
        "category": "Aria Debug"
      },
      {
        "command": "aria.stepOverEffect",
        "title": "Step Over Effect",
        "category": "Aria Debug"
      },
      {
        "command": "aria.showEffectStack",
        "title": "Show Effect Stack",
        "category": "Aria Debug"
      }
    ]
  }
}
```

---

## 7. Profiling Support

### 7.1 Overview

Aria provides integrated profiling support with:
- CPU sampling profiler
- Instrumentation-based profiling
- Effect-aware profiling
- Memory allocation profiling
- Async task profiling

### 7.2 Sampling Profiler

```rust
/// CPU sampling profiler
pub struct SamplingProfiler {
    /// Sampling interval
    interval: Duration,
    /// Sample buffer
    samples: Vec<Sample>,
    /// Symbol resolver
    resolver: SymbolResolver,
    /// Running state
    running: AtomicBool,
}

pub struct Sample {
    /// Timestamp
    timestamp: Instant,
    /// Thread ID
    thread_id: ThreadId,
    /// Stack trace (instruction pointers)
    stack: Vec<u64>,
    /// CPU state
    cpu_state: CpuState,
}

impl SamplingProfiler {
    /// Start profiling
    pub fn start(&self) {
        self.running.store(true, Ordering::SeqCst);

        // Set up signal handler (SIGPROF on Unix)
        #[cfg(unix)]
        unsafe {
            signal::signal(Signal::SIGPROF, self.signal_handler());

            // Set up timer
            let interval = libc::itimerval {
                it_interval: self.to_timeval(self.interval),
                it_value: self.to_timeval(self.interval),
            };
            libc::setitimer(libc::ITIMER_PROF, &interval, std::ptr::null_mut());
        }
    }

    /// Process collected samples into profile
    pub fn finish(&self) -> Profile {
        self.running.store(false, Ordering::SeqCst);

        let mut profile = Profile::new();

        for sample in &self.samples {
            let resolved_stack: Vec<_> = sample.stack.iter()
                .map(|&ip| self.resolver.resolve(ip))
                .collect();

            profile.add_sample(&resolved_stack, sample.timestamp);
        }

        profile
    }
}
```

### 7.3 Instrumentation Profiling

For precise call counts and timing:

```rust
/// Instrumentation-based profiler
pub struct InstrumentationProfiler {
    /// Function entry/exit hooks
    hooks: FunctionHooks,
    /// Call stack per thread
    call_stacks: ThreadLocal<Vec<CallEntry>>,
    /// Accumulated data
    data: ProfileData,
}

pub struct CallEntry {
    function_id: FunctionId,
    entry_time: Instant,
}

pub struct FunctionStats {
    /// Total calls
    pub call_count: u64,
    /// Total time (including children)
    pub total_time: Duration,
    /// Self time (excluding children)
    pub self_time: Duration,
    /// Memory allocated
    pub bytes_allocated: u64,
    /// Effect operations performed
    pub effect_operations: HashMap<EffectType, u64>,
}

impl InstrumentationProfiler {
    /// Generate instrumented code
    pub fn instrument_function(
        &self,
        func: &mut MirFunction,
    ) {
        // Insert entry hook
        let entry_block = func.entry_block_mut();
        entry_block.insert_at_start(MirInst::Call {
            callee: self.hooks.on_entry,
            args: vec![MirOperand::Const(func.id.into())],
            dest: None,
        });

        // Insert exit hooks before all returns
        for block in func.blocks_mut() {
            for (idx, inst) in block.instructions().enumerate() {
                if matches!(inst, MirInst::Return { .. }) {
                    block.insert_before(idx, MirInst::Call {
                        callee: self.hooks.on_exit,
                        args: vec![MirOperand::Const(func.id.into())],
                        dest: None,
                    });
                }
            }
        }
    }
}
```

### 7.4 Effect-Aware Profiling

Track time spent in effect handlers:

```rust
/// Effect profiling data
pub struct EffectProfile {
    /// Time per effect type
    pub effect_times: HashMap<EffectType, Duration>,
    /// Time per operation
    pub operation_times: HashMap<(EffectType, OperationId), Duration>,
    /// Handler overhead
    pub handler_overhead: HashMap<HandlerId, Duration>,
    /// Continuation captures
    pub continuation_captures: u64,
    /// Continuation resumes
    pub continuation_resumes: u64,
}

impl EffectProfiler {
    /// Hook for effect.perform
    pub fn on_effect_perform(
        &mut self,
        effect: EffectType,
        operation: OperationId,
    ) -> EffectPerformToken {
        EffectPerformToken {
            effect,
            operation,
            start_time: Instant::now(),
        }
    }

    /// Hook for effect completion
    pub fn on_effect_complete(&mut self, token: EffectPerformToken) {
        let duration = token.start_time.elapsed();

        *self.effect_times.entry(token.effect).or_default() += duration;
        *self.operation_times
            .entry((token.effect, token.operation))
            .or_default() += duration;
    }
}
```

### 7.5 Async Task Profiling

```rust
/// Async task profiler
pub struct AsyncTaskProfiler {
    /// Task statistics
    task_stats: HashMap<TaskId, TaskStats>,
    /// Active tasks
    active_tasks: HashMap<TaskId, TaskContext>,
}

pub struct TaskStats {
    /// Task name/location
    pub name: String,
    /// Total runtime
    pub total_runtime: Duration,
    /// Time waiting (suspended)
    pub wait_time: Duration,
    /// Number of suspensions
    pub suspension_count: u64,
    /// Suspension reasons
    pub suspension_reasons: HashMap<String, u64>,
    /// Channel operations
    pub channel_ops: ChannelOpStats,
}

pub struct ChannelOpStats {
    pub sends: u64,
    pub receives: u64,
    pub send_wait_time: Duration,
    pub receive_wait_time: Duration,
}
```

### 7.6 Profile Output Formats

Support multiple output formats:

```rust
/// Profile output formats
pub enum ProfileFormat {
    /// Chrome trace format (viewable in chrome://tracing)
    ChromeTrace,
    /// Flame graph (SVG)
    FlameGraph,
    /// pprof format (Go-compatible)
    Pprof,
    /// Speedscope JSON
    Speedscope,
    /// Plain text summary
    TextSummary,
}

impl Profile {
    /// Export to specified format
    pub fn export(&self, format: ProfileFormat, writer: &mut dyn Write) -> Result<()> {
        match format {
            ProfileFormat::ChromeTrace => {
                self.write_chrome_trace(writer)
            }
            ProfileFormat::FlameGraph => {
                let svg = self.generate_flame_graph();
                writer.write_all(svg.as_bytes())
            }
            ProfileFormat::Pprof => {
                self.write_pprof(writer)
            }
            ProfileFormat::Speedscope => {
                let json = self.to_speedscope_json();
                serde_json::to_writer(writer, &json)?;
                Ok(())
            }
            ProfileFormat::TextSummary => {
                self.write_text_summary(writer)
            }
        }
    }
}
```

---

## 8. Memory Debugging

### 8.1 Overview

Aria integrates memory debugging tools:
- Leak detection
- Use-after-free detection
- Buffer overflow detection
- Double-free detection
- Uninitialized memory reads

### 8.2 Sanitizer Integration

```rust
/// Memory sanitizer configuration
pub struct SanitizerConfig {
    /// Enable AddressSanitizer
    pub address_sanitizer: bool,
    /// Enable LeakSanitizer
    pub leak_sanitizer: bool,
    /// Enable MemorySanitizer (uninitialized reads)
    pub memory_sanitizer: bool,
    /// Enable ThreadSanitizer
    pub thread_sanitizer: bool,
    /// Custom Aria sanitizer
    pub aria_sanitizer: bool,
}

/// Aria-specific memory sanitizer
pub struct AriaSanitizer {
    /// Allocation tracking
    allocations: HashMap<usize, AllocationInfo>,
    /// Freed memory quarantine
    quarantine: VecDeque<FreedAllocation>,
    /// Stack traces for allocations
    allocation_stacks: HashMap<usize, StackTrace>,
}

pub struct AllocationInfo {
    pub size: usize,
    pub alignment: usize,
    pub allocation_time: Instant,
    pub allocation_site: SourceLocation,
    pub type_info: Option<TypeId>,
}

impl AriaSanitizer {
    /// Hook for memory allocation
    pub fn on_alloc(
        &mut self,
        ptr: *mut u8,
        size: usize,
        alignment: usize,
    ) {
        let stack = self.capture_stack_trace();

        self.allocations.insert(ptr as usize, AllocationInfo {
            size,
            alignment,
            allocation_time: Instant::now(),
            allocation_site: self.current_source_location(),
            type_info: self.infer_type_from_context(),
        });

        self.allocation_stacks.insert(ptr as usize, stack);
    }

    /// Hook for memory deallocation
    pub fn on_free(&mut self, ptr: *mut u8) {
        let addr = ptr as usize;

        if let Some(info) = self.allocations.remove(&addr) {
            // Add to quarantine (delay reuse to catch use-after-free)
            self.quarantine.push_back(FreedAllocation {
                ptr: addr,
                info,
                free_stack: self.capture_stack_trace(),
                free_time: Instant::now(),
            });

            // Limit quarantine size
            while self.quarantine.len() > QUARANTINE_SIZE {
                self.quarantine.pop_front();
            }
        } else {
            // Double free or invalid free
            self.report_invalid_free(ptr);
        }
    }

    /// Check for memory access validity
    pub fn check_access(&self, ptr: *const u8, size: usize, is_write: bool) {
        let addr = ptr as usize;

        // Check if accessing freed memory
        for freed in &self.quarantine {
            if addr >= freed.ptr && addr < freed.ptr + freed.info.size {
                self.report_use_after_free(ptr, &freed);
                return;
            }
        }

        // Check if accessing valid allocation
        for (&alloc_addr, info) in &self.allocations {
            if addr >= alloc_addr && addr < alloc_addr + info.size {
                // Check bounds
                if addr + size > alloc_addr + info.size {
                    self.report_buffer_overflow(ptr, size, info);
                }
                return;
            }
        }

        // Unknown memory access
        self.report_unknown_access(ptr, size, is_write);
    }
}
```

### 8.3 Leak Detection

```rust
/// Memory leak detector
pub struct LeakDetector {
    /// All live allocations at detection time
    live_allocations: Vec<AllocationInfo>,
    /// Root pointers (stack, globals)
    roots: Vec<*const u8>,
}

impl LeakDetector {
    /// Run leak detection at program exit
    pub fn detect_leaks(&self) -> LeakReport {
        let mut report = LeakReport::new();

        // Mark reachable allocations
        let reachable = self.mark_reachable();

        // Find unreachable (leaked) allocations
        for alloc in &self.live_allocations {
            if !reachable.contains(&alloc.ptr) {
                report.add_leak(LeakedMemory {
                    address: alloc.ptr,
                    size: alloc.size,
                    allocation_site: alloc.allocation_site.clone(),
                    allocation_stack: alloc.stack.clone(),
                    type_info: alloc.type_info.clone(),
                });
            }
        }

        report
    }

    /// Mark allocations reachable from roots
    fn mark_reachable(&self) -> HashSet<usize> {
        let mut reachable = HashSet::new();
        let mut worklist: Vec<*const u8> = self.roots.clone();

        while let Some(ptr) = worklist.pop() {
            // Scan pointer-sized values at this location
            // (Conservative scanning)
            for offset in (0..SCAN_SIZE).step_by(std::mem::size_of::<usize>()) {
                let potential_ptr = unsafe {
                    *(ptr.add(offset) as *const usize)
                };

                // Check if this points to an allocation
                if let Some(alloc) = self.find_allocation(potential_ptr) {
                    if reachable.insert(alloc.ptr) {
                        // Newly discovered, add to worklist
                        worklist.push(alloc.ptr as *const u8);
                    }
                }
            }
        }

        reachable
    }
}
```

### 8.4 Memory Debug Reports

```
MEMORY ERROR DETECTED

Type: Use-After-Free
Location: process_data (data.aria:45)

Attempted access:
  Address: 0x7ffd12345678
  Size: 8 bytes
  Operation: Read

Memory was freed:
  At: cleanup (data.aria:78)
  Stack:
    cleanup (data.aria:78)
    process_complete (data.aria:65)
    main (main.aria:12)

Memory was originally allocated:
  At: create_buffer (data.aria:23)
  Size: 1024 bytes
  Stack:
    create_buffer (data.aria:23)
    process_data (data.aria:30)
    main (main.aria:10)

Suggestion: The buffer was freed in cleanup() but still referenced.
            Consider using ownership tracking or Arc for shared data.
```

---

## 9. Implementation Phases

### Phase 1: Core Debug Info (Weeks 1-4)

| Task | Priority | Dependencies |
|------|----------|--------------|
| DWARF 5 generation via Cranelift | P0 | Cranelift backend |
| Source location tracking in MIR | P0 | MIR design |
| Basic variable debug info | P0 | Type system |
| Debug info for Aria types | P1 | Type lowering |

### Phase 2: Debugger Integration (Weeks 5-8)

| Task | Priority | Dependencies |
|------|----------|--------------|
| DAP server implementation | P0 | - |
| LLDB backend | P0 | DWARF generation |
| GDB backend | P1 | DWARF generation |
| VS Code extension | P0 | DAP server |
| Basic breakpoints, stepping | P0 | Backends |

### Phase 3: Effect Debugging (Weeks 9-12)

| Task | Priority | Dependencies |
|------|----------|--------------|
| Effect handler stack tracking | P0 | Effect system |
| Stepping through handlers | P0 | Basic stepping |
| Resumption point debugging | P1 | Continuation support |
| Effect stack visualization | P1 | DAP extensions |

### Phase 4: Contract Debugging (Weeks 13-14)

| Task | Priority | Dependencies |
|------|----------|--------------|
| Contract failure location | P0 | Contract system |
| Evaluation tracing | P1 | Contract evaluation |
| old() value capture | P1 | Postconditions |
| Contract breakpoints | P2 | Breakpoints |

### Phase 5: Async Debugging (Weeks 15-18)

| Task | Priority | Dependencies |
|------|----------|--------------|
| Task state tracking | P0 | Async runtime |
| Parallel tasks window | P0 | DAP extensions |
| Channel state inspection | P1 | Channel runtime |
| Deadlock detection | P1 | Wait graph |
| Async stack traces | P1 | Task spawning |

### Phase 6: Profiling & Memory (Weeks 19-22)

| Task | Priority | Dependencies |
|------|----------|--------------|
| Sampling profiler | P1 | Debug info |
| Instrumentation profiler | P2 | MIR instrumentation |
| Effect-aware profiling | P2 | Effect tracking |
| Sanitizer integration | P1 | Memory allocator |
| Leak detection | P1 | Allocation tracking |

---

## 10. Performance Considerations

### 10.1 Debug Info Size

| Strategy | Size Reduction | Trade-off |
|----------|---------------|-----------|
| Split DWARF | 50-80% | Requires .dwo files |
| DWARF compression | 40-60% | Slower debug startup |
| Type deduplication | 30-50% | DWARF 4+ required |
| Line table compression | 20-30% | None significant |

### 10.2 Debug Build Performance

| Feature | Overhead | Mitigation |
|---------|----------|------------|
| Debug symbols | 10-20% compile time | Parallel debug info gen |
| Sanitizers | 2x runtime | Optional, debug only |
| Profiling hooks | 5-10% runtime | Sampling over instrumentation |
| Effect tracking | <5% runtime | Compile-time when possible |

---

## 11. Open Questions

1. **Multi-shot continuation debugging**: How to visualize multiple active resumes of same continuation?

2. **Distributed debugging**: When Aria adds distributed computing, how to debug across nodes?

3. **Time-travel debugging**: Should we support rr/Pernosco-style reverse debugging?

4. **AI-assisted debugging**: Integration with ChatDBG-style LLM debugging?

5. **Hot reload debugging**: Continue debugging after code changes?

---

## 12. References

### DWARF and Debug Information
- [DWARF Debugging Information Format](https://dwarfstd.org/)
- [LLVM Source Level Debugging](https://llvm.org/docs/SourceLevelDebugging.html)
- [Rust Debug Info in Rustc](https://rustc-dev-guide.rust-lang.org/debugging-support-in-rustc.html)

### Debugger Integration
- [LLDB Custom Language Support](https://lldb.llvm.org/resources/addinglanguagesupport.html)
- [Debug Adapter Protocol](https://microsoft.github.io/debug-adapter-protocol/)
- [VS Code Debugger Extension Guide](https://code.visualstudio.com/api/extension-guides/debugger-extension)

### Concurrent Debugging
- [Visual Studio Parallel Debugging](https://learn.microsoft.com/en-us/visualstudio/debugger/walkthrough-debugging-a-parallel-application)
- [GDB 14 Coroutine Debugging](https://markaicode.com/debugging-coroutines-gdb14-crash-analysis/)

### Memory Debugging
- [AddressSanitizer](https://github.com/google/sanitizers/wiki/AddressSanitizer)
- [LeakSanitizer](https://clang.llvm.org/docs/LeakSanitizer.html)

### Profiling
- [Chrome Trace Format](https://docs.google.com/document/d/1CvAClvFfyA5R-PhYUmn5OOQtYMH4h6I0nSsKchNAySU)
- [Profile Guided Optimization Research 2025](https://arxiv.org/html/2507.16649v1)

---

**Document Status**: Research Complete
**Next Steps**:
1. Review with NEXUS (Compiler Architecture Agent)
2. Create detailed implementation tasks
3. Begin Phase 1 implementation

---

*Research conducted by TRACE (Eureka Research Agent)*
*Aria Language Project - Eureka Vault*
*Last updated: 2026-01-15*
