use crate::ast::{Expr, Program, Statement, TaskDef};
use crate::builtins::BuiltinRegistry;
use crate::tool_executor;
use std::collections::HashMap;
use std::fmt;

// Day 5: Runtime resource limits - The Immune System
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub max_steps: u64,        // Max statement evaluations before abort
    pub max_depth: u32,        // Max delegation/recursion depth
    pub max_output_bytes: u64, // Max bytes from a single tool output
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_steps: 10_000,
            max_depth: 32,
            max_output_bytes: 1_048_576, // 1 MB
        }
    }
}

// Day 5: Tracks resource usage at runtime
#[derive(Debug, Default)]
pub struct ResourceTracker {
    pub steps: u64,
    pub depth: u32,
}

// Day 3: Tool definition storage
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Tool {
    pub name: String,
    pub params: Vec<String>,
    pub permission: Option<String>,
    pub timeout: Option<f64>,
}

// M21: MCP Tool definition
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct McpTool {
    pub name: String,
    pub server: String,
    pub permission: Option<String>,
    pub capabilities: Vec<String>,
    pub timeout: Option<f64>,
}

// M22: Orchestration definitions stored at runtime
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PipelineInstance {
    pub name: String,
    pub stages: Vec<(String, Expr)>, // (agent_name, call)
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ConcurrentInstance {
    pub name: String,
    pub branches: Vec<(String, Expr)>, // (agent_name, call)
    pub merge_fn: Option<Expr>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct HandoffInstance {
    pub name: String,
    pub agent_name: String,
    pub agent_call: Expr,
    pub routes: Vec<(String, String)>, // (pattern, target_agent)
}

// M23: A2A agent card
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct A2ACard {
    pub name: String,
    pub discovery: Option<String>,
    pub skills: Vec<String>,
    pub endpoint: Option<String>,
}

// M24: Workflow instance
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct WorkflowInstance {
    pub name: String,
    pub current_state: String,
    pub states: HashMap<String, WorkflowStateRuntime>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct WorkflowStateRuntime {
    pub name: String,
    pub transitions: Vec<(String, String)>, // (event, target_state)
    pub requires: Option<Expr>,
    pub ensures: Option<Expr>,
    pub body: Vec<Statement>,
}

// M25: Model definition
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ModelInstance {
    pub name: String,
    pub capability: Option<String>,
    pub provider: Option<String>,
    pub supports: Vec<String>,
}

// M26: Memory definition
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MemoryInstance {
    pub name: String,
    pub store: Option<String>,
    pub embedding_model: Option<String>,
    pub operations: Vec<String>,
    pub entries: Vec<(String, Value)>, // Simple in-memory key-value for runtime
}

// Day 3: Agent definition (the "type" of agent)
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AgentDef {
    pub name: String,
    pub allow_list: Vec<String>,
    pub tasks: Vec<TaskDef>,
    pub body: Vec<Statement>,
}

// Day 3: Agent instance (a running agent with scoped permissions)
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AgentInstance {
    pub name: String,
    pub agent_def_name: String,
    pub allowed_tools: Vec<String>,
    pub variables: HashMap<String, Value>,
}

pub struct Evaluator {
    pub variables: HashMap<String, Value>,
    pub tools: HashMap<String, Tool>,
    pub agent_defs: HashMap<String, AgentDef>,
    pub agents: HashMap<String, AgentInstance>,
    pub current_agent: Option<String>, // Day 4: Execution context tracking
    pub limits: ResourceLimits,        // Day 5: Configurable resource limits
    pub tracker: ResourceTracker,      // Day 5: Runtime resource usage tracking
    pub builtins: BuiltinRegistry,     // Day 6: Standard library functions
    pub output: Vec<String>,           // Captured runtime trace for testing

    // M21: MCP tools registry
    pub mcp_tools: HashMap<String, McpTool>,
    // M22: Orchestration registries
    pub pipelines: HashMap<String, PipelineInstance>,
    pub concurrent_defs: HashMap<String, ConcurrentInstance>,
    pub handoffs: HashMap<String, HandoffInstance>,
    // M23: A2A cards
    pub a2a_cards: HashMap<String, A2ACard>,
    // M24: Workflows
    pub workflows: HashMap<String, WorkflowInstance>,
    // M25: Models
    pub models: HashMap<String, ModelInstance>,
    // M26: Memories
    pub memories: HashMap<String, MemoryInstance>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Number(f64),
    Null,
    Agent(String),       // Represents an agent instance
    Array(Vec<Value>),   // M22: For concurrent results aggregation
    Bool(bool),          // M24: For contract evaluation
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::String(s) => write!(f, "{}", s),
            Value::Number(n) => write!(f, "{}", n),
            Value::Null => write!(f, "null"),
            Value::Agent(a) => write!(f, "[Agent: {}]", a),
            Value::Array(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            Value::Bool(b) => write!(f, "{}", b),
        }
    }
}

impl Evaluator {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            tools: HashMap::new(),
            agent_defs: HashMap::new(),
            agents: HashMap::new(),
            current_agent: None,
            limits: ResourceLimits::default(),
            tracker: ResourceTracker::default(),
            builtins: BuiltinRegistry::new(),
            output: Vec::new(),
            // M21-M26: Initialize new registries
            mcp_tools: HashMap::new(),
            pipelines: HashMap::new(),
            concurrent_defs: HashMap::new(),
            handoffs: HashMap::new(),
            a2a_cards: HashMap::new(),
            workflows: HashMap::new(),
            models: HashMap::new(),
            memories: HashMap::new(),
        }
    }

    /// Create an evaluator with custom resource limits
    #[cfg(test)]
    pub fn with_limits(limits: ResourceLimits) -> Self {
        Self {
            limits,
            ..Self::new()
        }
    }

    /// Emit a runtime trace message (captured in output vec and printed to stdout)
    fn emit(&mut self, msg: String) {
        println!("{}", msg);
        self.output.push(msg);
    }

    pub fn eval_program(&mut self, program: Program) {
        for stmt in program.statements {
            if let Err(e) = self.eval_statement(stmt) {
                eprintln!("[Runtime Error] {}", e);
                break;
            }
        }
    }

    fn eval_statement(&mut self, stmt: Statement) -> Result<(), String> {
        // Day 5: Step limit enforcement
        self.tracker.steps += 1;
        if self.tracker.steps > self.limits.max_steps {
            return Err(format!(
                "[Resource Limit] Exceeded max execution steps ({}) - possible infinite loop",
                self.limits.max_steps
            ));
        }

        match stmt {
            Statement::Let { name, value } => {
                let val = self.eval_expr(value)?;
                self.variables.insert(name, val);
            }
            Statement::Print(expr) => {
                let val = self.eval_expr(expr)?;
                self.emit(format!("{}", val));
            }
            Statement::Think(expr) => {
                let val = self.eval_expr(expr)?;
                self.emit(format!("[Thinking...] {:?}", val));
            }
            Statement::Gate { prompt, body } => {
                let p = self.eval_expr(prompt)?;
                self.emit(format!(
                    "[GATE] {}",
                    match p {
                        Value::String(s) => s,
                        _ => format!("{:?}", p),
                    }
                ));
                self.emit("(Simulating Human Approval: Press Enter to Continue)".to_string());
                let mut input = String::new();
                std::io::stdin()
                    .read_line(&mut input)
                    .map_err(|e| e.to_string())?;

                for s in body {
                    self.eval_statement(s)?;
                }
            }
            Statement::AgentBlock { name, body } => {
                self.emit(format!("[Entering Agent Context: {}]", name));
                for s in body {
                    self.eval_statement(s)?;
                }
                self.emit(format!("[Exiting Agent Context: {}]", name));
            }
            Statement::ToolDef {
                name,
                params,
                permission,
                timeout,
            } => {
                self.eval_tool_def(name, params, permission, timeout)?;
            }
            Statement::AgentDef {
                name,
                allow_list,
                tasks,
                body,
            } => {
                self.eval_agent_def(name, allow_list, tasks, body)?;
            }
            Statement::TaskDef(_) => {
                // TODO: Implement task definitions
                return Err("Task definitions not yet implemented".to_string());
            }
            Statement::Spawn {
                var_name,
                agent_name,
            } => {
                self.eval_spawn(var_name, agent_name)?;
            }
            Statement::Delegate { call } => {
                self.eval_delegate(call)?;
            }
            Statement::Main { body } => {
                self.emit("[Entering Main Block]".to_string());
                // Day 4: Ensure we're in main context (unrestricted)
                let previous_agent = self.current_agent.clone();
                self.current_agent = None;

                for s in body {
                    self.eval_statement(s)?;
                }

                self.current_agent = previous_agent;
                self.emit("[Exiting Main Block]".to_string());
            }
            Statement::Return(expr) => {
                // TODO: Implement proper return handling with value propagation
                // For now, just evaluate the expression and log it
                let val = self.eval_expr(expr)?;
                self.emit(format!("[Return] {:?}", val));
            }

            // ==============================================================
            // M21: MCP Tool Registration
            // ==============================================================
            Statement::McpToolDef {
                name,
                server,
                permission,
                capabilities,
                timeout,
            } => {
                let mcp_tool = McpTool {
                    name: name.clone(),
                    server: server.clone(),
                    permission: permission.clone(),
                    capabilities: capabilities.clone(),
                    timeout,
                };
                self.mcp_tools.insert(name.clone(), mcp_tool);
                // Also register as a regular tool so permission checks work
                let tool = Tool {
                    name: name.clone(),
                    params: vec![], // MCP tools have dynamic params
                    permission: permission.clone(),
                    timeout,
                };
                self.tools.insert(name.clone(), tool);
                self.emit(format!(
                    "[MCP Tool Registered] {} from server '{}' (capabilities: {:?}, permission: {:?})",
                    name, server, capabilities, permission
                ));
            }

            // ==============================================================
            // M22: Pipeline Orchestration
            // ==============================================================
            Statement::PipelineDef { name, stages } => {
                let stage_data: Vec<(String, Expr)> = stages
                    .into_iter()
                    .map(|s| (s.agent_name, s.call))
                    .collect();
                let stage_count = stage_data.len();
                let stage_names: Vec<String> = stage_data.iter().map(|(n, _)| n.clone()).collect();
                self.pipelines.insert(
                    name.clone(),
                    PipelineInstance {
                        name: name.clone(),
                        stages: stage_data,
                    },
                );
                self.emit(format!(
                    "[Pipeline Registered] {} with {} stages: {:?}",
                    name, stage_count, stage_names
                ));
            }

            // M22: Concurrent Orchestration
            Statement::ConcurrentDef {
                name,
                branches,
                merge_fn,
            } => {
                let branch_data: Vec<(String, Expr)> = branches
                    .into_iter()
                    .map(|b| (b.agent_name, b.call))
                    .collect();
                let branch_count = branch_data.len();
                let branch_names: Vec<String> = branch_data.iter().map(|(n, _)| n.clone()).collect();
                let has_merge = merge_fn.is_some();
                self.concurrent_defs.insert(
                    name.clone(),
                    ConcurrentInstance {
                        name: name.clone(),
                        branches: branch_data,
                        merge_fn,
                    },
                );
                self.emit(format!(
                    "[Concurrent Registered] {} with {} branches: {:?} (merge: {})",
                    name, branch_count, branch_names, has_merge
                ));
            }

            // M22: Handoff Orchestration
            Statement::HandoffDef {
                name,
                agent_name,
                agent_call,
                routes,
            } => {
                let route_data: Vec<(String, String)> = routes
                    .into_iter()
                    .map(|r| (r.pattern, r.target_agent))
                    .collect();
                let route_count = route_data.len();
                self.handoffs.insert(
                    name.clone(),
                    HandoffInstance {
                        name: name.clone(),
                        agent_name: agent_name.clone(),
                        agent_call,
                        routes: route_data,
                    },
                );
                self.emit(format!(
                    "[Handoff Registered] {} with classifier '{}' and {} routes",
                    name, agent_name, route_count
                ));
            }

            // ==============================================================
            // M23: A2A Protocol Registration
            // ==============================================================
            Statement::A2ADef {
                name,
                discovery,
                skills,
                endpoint,
            } => {
                let card = A2ACard {
                    name: name.clone(),
                    discovery: discovery.clone(),
                    skills: skills.clone(),
                    endpoint: endpoint.clone(),
                };
                self.a2a_cards.insert(name.clone(), card);
                self.emit(format!(
                    "[A2A Card Registered] {} (discovery: {:?}, skills: {:?}, endpoint: {:?})",
                    name, discovery, skills, endpoint
                ));
            }

            // ==============================================================
            // M24: Workflow State Machine Registration
            // ==============================================================
            Statement::WorkflowDef { name, states } => {
                let mut state_map = HashMap::new();
                let first_state = states.first().map(|s| s.name.clone()).unwrap_or_default();

                for state in &states {
                    let transitions: Vec<(String, String)> = state
                        .transitions
                        .iter()
                        .map(|t| (t.event.clone(), t.target_state.clone()))
                        .collect();
                    state_map.insert(
                        state.name.clone(),
                        WorkflowStateRuntime {
                            name: state.name.clone(),
                            transitions,
                            requires: state.requires.clone(),
                            ensures: state.ensures.clone(),
                            body: state.body.clone(),
                        },
                    );
                }

                let state_names: Vec<String> = states.iter().map(|s| s.name.clone()).collect();
                let transition_count: usize = states.iter().map(|s| s.transitions.len()).sum();
                self.workflows.insert(
                    name.clone(),
                    WorkflowInstance {
                        name: name.clone(),
                        current_state: first_state.clone(),
                        states: state_map,
                    },
                );
                self.emit(format!(
                    "[Workflow Registered] {} with {} states: {:?} ({} transitions, initial: '{}')",
                    name, state_names.len(), state_names, transition_count, first_state
                ));
            }

            // ==============================================================
            // M25: Model Declaration Registration
            // ==============================================================
            Statement::ModelDef {
                name,
                capability,
                provider,
                supports,
            } => {
                let model = ModelInstance {
                    name: name.clone(),
                    capability: capability.clone(),
                    provider: provider.clone(),
                    supports: supports.clone(),
                };
                self.models.insert(name.clone(), model);
                self.emit(format!(
                    "[Model Registered] {} (capability: {:?}, provider: {:?}, supports: {:?})",
                    name, capability, provider, supports
                ));
            }

            // ==============================================================
            // M26: Memory Definition Registration
            // ==============================================================
            Statement::MemoryDef {
                name,
                store,
                embedding_model,
                operations,
            } => {
                let mem = MemoryInstance {
                    name: name.clone(),
                    store: store.clone(),
                    embedding_model: embedding_model.clone(),
                    operations: operations.clone(),
                    entries: Vec::new(),
                };
                self.memories.insert(name.clone(), mem);
                self.emit(format!(
                    "[Memory Registered] {} (store: {:?}, embedding: {:?}, operations: {:?})",
                    name, store, embedding_model, operations
                ));
            }
        }
        Ok(())
    }

    // Day 3 - Step 12: Delegate Task Execution
    fn eval_delegate(&mut self, call: Expr) -> Result<(), String> {
        // Delegate expects a Call expression with name formatted as "agent_var.task_name"
        // The parser creates: Expr::Call { name: "bot.cleanup_logs", args: [...] }

        match call {
            Expr::Call { name, args } => {
                // Parse the name to extract agent variable and task name
                // Format: "agent_var.task_name"
                let parts: Vec<&str> = name.split('.').collect();
                if parts.len() != 2 {
                    return Err(format!(
                        "Invalid delegate call format: '{}'. Expected 'agent.task'",
                        name
                    ));
                }

                let mut agent_var = parts[0].to_string();
                let task_name = parts[1].to_string();

                // Try to find the agent instance - handle both "bot" and "$bot" formats
                // If the variable doesn't have a $ prefix and isn't found, try adding it
                let agent_instance = if let Some(instance) = self.agents.get(&agent_var) {
                    instance
                } else if !agent_var.starts_with('$') {
                    // Try with $ prefix
                    let prefixed = format!("${}", agent_var);
                    agent_var = prefixed.clone();
                    self.agents
                        .get(&prefixed)
                        .ok_or(format!("Agent instance '{}' not found", parts[0]))?
                } else {
                    return Err(format!("Agent instance '{}' not found", agent_var));
                };

                // Get agent definition
                let agent_def_name = agent_instance.agent_def_name.clone();
                let agent_def = self
                    .agent_defs
                    .get(&agent_def_name)
                    .ok_or(format!("Agent definition '{}' not found", agent_def_name))?;

                // Find the task in the agent definition
                let task = agent_def
                    .tasks
                    .iter()
                    .find(|t| t.name == task_name)
                    .ok_or(format!(
                        "Task '{}' not found in agent '{}'",
                        task_name, agent_def_name
                    ))?
                    .clone();

                self.emit(format!(
                    "[Delegating] {}.{}() with {} args",
                    agent_var,
                    task_name,
                    args.len()
                ));

                // Evaluate arguments
                let mut evaluated_args = Vec::new();
                for arg in args {
                    evaluated_args.push(self.eval_expr(arg)?);
                }

                // Execute task body in the context of the agent
                // Create a new scope for the task execution
                let old_variables = self.variables.clone();

                // Bind task parameters to evaluated arguments
                if task.params.len() != evaluated_args.len() {
                    return Err(format!(
                        "Task '{}' expects {} parameters but got {}",
                        task_name,
                        task.params.len(),
                        evaluated_args.len()
                    ));
                }

                for (param, value) in task.params.iter().zip(evaluated_args.iter()) {
                    self.variables.insert(param.clone(), value.clone());
                }

                // Day 5: Depth limit enforcement
                self.tracker.depth += 1;
                if self.tracker.depth > self.limits.max_depth {
                    self.tracker.depth -= 1;
                    return Err(format!(
                        "[Resource Limit] Exceeded max delegation depth ({}) - possible infinite delegation",
                        self.limits.max_depth
                    ));
                }

                // Day 4: Save current execution context
                let previous_agent = self.current_agent.clone();

                // Day 4: Set agent context for task execution
                self.current_agent = Some(agent_var.clone());
                self.emit(format!(
                    "[Context Switch] Entering agent context: {}",
                    agent_var
                ));

                // Execute task body with proper error handling
                for stmt in task.body {
                    if let Err(e) = self.eval_statement(stmt) {
                        self.tracker.depth -= 1; // Day 5: Restore depth on error
                        self.current_agent = previous_agent; // Restore on error
                        self.variables = old_variables;
                        return Err(e);
                    }
                }

                // Day 5: Restore depth on success
                self.tracker.depth -= 1;

                // Day 4: Restore previous execution context
                self.current_agent = previous_agent;
                self.emit(format!(
                    "[Context Switch] Exiting agent context: {}",
                    agent_var
                ));

                // Restore original scope
                self.variables = old_variables;

                Ok(())
            }
            _ => Err("Delegate expects call expression (agent.task())".to_string()),
        }
    }

    // Day 4: Function Calls with Permission Enforcement (THE PHYSICS!)
    // Day 6: Now checks builtins first
    pub fn eval_call(&mut self, name: &str, args: Vec<Expr>) -> Result<Value, String> {
        // Day 6: Check if it's a builtin function first (no permission check needed)
        if self.builtins.has(name) {
            let mut evaluated_args = Vec::new();
            for arg in args {
                evaluated_args.push(self.eval_expr(arg)?);
            }
            self.emit(format!(
                "[Builtin Call] {} with {} args",
                name,
                evaluated_args.len()
            ));
            return self.builtins.call(name, evaluated_args);
        }

        // Check if tool is defined and get its metadata
        let (permission, timeout) = {
            let tool = self
                .tools
                .get(name)
                .ok_or(format!("Unknown function or tool: '{}'", name))?;
            (tool.permission.clone(), tool.timeout)
        };

        // Day 4: PERMISSION ENFORCEMENT
        if let Some(agent_name) = &self.current_agent {
            // Agent context - check permissions against allow list
            let agent_instance = self
                .agents
                .get(agent_name)
                .ok_or(format!("[Internal Error] Agent '{}' not found", agent_name))?;

            if !agent_instance.allowed_tools.contains(&name.to_string()) {
                return Err(format!(
                    "[Permission Denied] Agent '{}' attempted to call tool '{}' but it is not in the allow list. Allowed tools: {:?}",
                    agent_name, name, agent_instance.allowed_tools
                ));
            }

            self.emit(format!(
                "[Permission Check] Agent '{}' is ALLOWED to call '{}'",
                agent_name, name
            ));
        } else {
            // Main context - unrestricted
            self.emit(format!(
                "[Permission Check] Main context - tool '{}' allowed (unrestricted)",
                name
            ));
        }

        // Evaluate arguments
        let mut evaluated_args = Vec::new();
        for arg in args {
            evaluated_args.push(self.eval_expr(arg)?);
        }

        // Day 4: Execute in sandbox
        self.emit(format!(
            "[Tool Call] {} with {} args (permission: {:?}, timeout: {:?}s)",
            name,
            evaluated_args.len(),
            permission,
            timeout
        ));

        tool_executor::execute_tool_command(
            name,
            &evaluated_args,
            timeout,
            self.limits.max_output_bytes,
        )
    }

    // Day 3 - Step 10: Agent Spawning (Instantiation)
    pub fn eval_spawn(&mut self, var_name: String, agent_name: String) -> Result<(), String> {
        let def = self
            .agent_defs
            .get(&agent_name)
            .ok_or(format!("Agent '{}' not defined", agent_name))?;

        let instance = AgentInstance {
            name: var_name.clone(),
            agent_def_name: agent_name.clone(),
            allowed_tools: def.allow_list.clone(),
            variables: HashMap::new(),
        };

        self.agents.insert(var_name.clone(), instance);
        self.variables
            .insert(var_name.clone(), Value::Agent(var_name.clone()));

        self.emit(format!(
            "[Agent Spawned] {} as {} (permissions: {:?})",
            agent_name, var_name, def.allow_list
        ));
        Ok(())
    }

    // Day 3 - Step 9: Agent Definition Registration
    pub fn eval_agent_def(
        &mut self,
        name: String,
        allow_list: Vec<String>,
        tasks: Vec<TaskDef>,
        body: Vec<Statement>,
    ) -> Result<(), String> {
        let agent_def = AgentDef {
            name: name.clone(),
            allow_list: allow_list.clone(),
            tasks,
            body,
        };
        self.agent_defs.insert(name.clone(), agent_def);
        self.emit(format!(
            "[Agent Defined] {} (allows {} tools)",
            name,
            allow_list.len()
        ));
        Ok(())
    }

    // Day 3 - Step 8: Tool Registration
    pub fn eval_tool_def(
        &mut self,
        name: String,
        params: Vec<String>,
        permission: Option<String>,
        timeout: Option<f64>,
    ) -> Result<(), String> {
        let tool = Tool {
            name: name.clone(),
            params,
            permission,
            timeout,
        };
        self.tools.insert(name.clone(), tool);
        self.emit(format!(
            "[Tool Registered] {} with {} params",
            name,
            self.tools.get(&name).unwrap().params.len()
        ));
        Ok(())
    }

    fn eval_expr(&mut self, expr: Expr) -> Result<Value, String> {
        match expr {
            Expr::String(s) => Ok(Value::String(s)),
            Expr::Number(n) => Ok(Value::Number(n)),
            Expr::Var(v) => self
                .variables
                .get(&v)
                .cloned()
                .ok_or(format!("Undefined variable: {}", v)),
            Expr::Agent(a) => Ok(Value::String(format!("Context:{}", a))),
            Expr::Call { name, args } => self.eval_call(&name, args),
            Expr::MemberAccess { .. } => {
                // TODO: Implement member access
                Err("Member access not yet implemented".to_string())
            }
            Expr::Array(elements) => {
                let mut values = Vec::new();
                for elem in elements {
                    values.push(self.eval_expr(elem)?);
                }
                Ok(Value::Array(values))
            }
        }
    }
}

#[cfg(test)]
mod eval_tests;
