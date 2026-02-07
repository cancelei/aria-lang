use crate::ast::{Expr, Program, Statement, TaskDef};
use crate::builtins::BuiltinRegistry;
use crate::tool_executor;
use std::collections::HashMap;

// Day 3: Tool definition storage
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Tool {
    pub name: String,
    pub params: Vec<String>,
    pub permission: Option<String>,
    pub timeout: Option<f64>,
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
    pub builtins: BuiltinRegistry,     // Day 6: Standard library functions
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Number(f64),
    Null,
    Agent(String), // NEW: Represents an agent instance
}

impl Evaluator {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            tools: HashMap::new(),
            agent_defs: HashMap::new(),
            agents: HashMap::new(),
            current_agent: None,              // Day 4: Start in main context
            builtins: BuiltinRegistry::new(), // Day 6: Initialize stdlib
        }
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
        match stmt {
            Statement::Let { name, value } => {
                let val = self.eval_expr(value)?;
                self.variables.insert(name, val);
            }
            Statement::Print(expr) => {
                let val = self.eval_expr(expr)?;
                match val {
                    Value::String(s) => println!("{}", s),
                    Value::Number(n) => println!("{}", n),
                    Value::Null => println!("null"),
                    Value::Agent(a) => println!("[Agent: {}]", a),
                }
            }
            Statement::Think(expr) => {
                let val = self.eval_expr(expr)?;
                println!("[Thinking...] {:?}", val);
            }
            Statement::Gate { prompt, body } => {
                let p = self.eval_expr(prompt)?;
                println!(
                    "[GATE] {}",
                    match p {
                        Value::String(s) => s,
                        _ => format!("{:?}", p),
                    }
                );
                println!("(Simulating Human Approval: Press Enter to Continue)");
                let mut input = String::new();
                std::io::stdin()
                    .read_line(&mut input)
                    .map_err(|e| e.to_string())?;

                for s in body {
                    self.eval_statement(s)?;
                }
            }
            Statement::AgentBlock { name, body } => {
                println!("[Entering Agent Context: {}]", name);
                for s in body {
                    self.eval_statement(s)?;
                }
                println!("[Exiting Agent Context: {}]", name);
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
                println!("[Entering Main Block]");
                // Day 4: Ensure we're in main context (unrestricted)
                let previous_agent = self.current_agent.clone();
                self.current_agent = None;

                for s in body {
                    self.eval_statement(s)?;
                }

                self.current_agent = previous_agent;
                println!("[Exiting Main Block]");
            }
            Statement::Return(expr) => {
                // TODO: Implement proper return handling with value propagation
                // For now, just evaluate the expression and log it
                let val = self.eval_expr(expr)?;
                println!("[Return] {:?}", val);
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

                println!(
                    "[Delegating] {}.{}() with {} args",
                    agent_var,
                    task_name,
                    args.len()
                );

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

                // Day 4: Save current execution context
                let previous_agent = self.current_agent.clone();

                // Day 4: Set agent context for task execution
                self.current_agent = Some(agent_var.clone());
                println!("[Context Switch] Entering agent context: {}", agent_var);

                // Execute task body with proper error handling
                for stmt in task.body {
                    if let Err(e) = self.eval_statement(stmt) {
                        self.current_agent = previous_agent; // RESTORE on error
                        self.variables = old_variables;
                        return Err(e);
                    }
                }

                // Day 4: Restore previous execution context
                self.current_agent = previous_agent;
                println!("[Context Switch] Exiting agent context: {}", agent_var);

                // Restore original scope
                // In a more sophisticated version, we'd handle return values
                self.variables = old_variables;

                Ok(())
            }
            _ => Err("Delegate expects call expression (agent.task())".to_string()),
        }
    }

    // Day 4: Function Calls with Permission Enforcement
    // Day 5: Now with real sandboxed execution
    // Day 6: Added builtin functions
    fn eval_call(&mut self, name: &str, args: Vec<Expr>) -> Result<Value, String> {
        // Evaluate arguments first (needed for both builtins and tools)
        let mut evaluated_args = Vec::new();
        for arg in args {
            evaluated_args.push(self.eval_expr(arg)?);
        }

        // Day 6: Check if it's a builtin function first
        if self.builtins.has(name) {
            println!("[Builtin Call] {} with {} args", name, evaluated_args.len());
            return self.builtins.call(name, evaluated_args);
        }

        // Check if tool is defined and get timeout
        let (permission, timeout) = {
            let tool = self
                .tools
                .get(name)
                .ok_or(format!("Unknown function or tool: '{}'", name))?;
            (tool.permission.clone(), tool.timeout)
        };

        // PERMISSION ENFORCEMENT
        if let Some(agent_name) = &self.current_agent {
            // Agent context - check permissions
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

            println!(
                "[Permission Check] Agent '{}' is ALLOWED to call '{}'",
                agent_name, name
            );
        } else {
            // Main context - unrestricted
            println!(
                "[Permission Check] Main context - tool '{}' allowed (unrestricted)",
                name
            );
        }

        // Day 5: Execute with sandboxing
        println!(
            "[Tool Call] {} with {} args (permission: {:?}, timeout: {:?}s)",
            name,
            evaluated_args.len(),
            permission,
            timeout
        );

        // Execute in sandbox
        tool_executor::execute_tool_command(name, &evaluated_args, timeout)
    }

    // Day 3 - Step 10: Agent Spawning (Instantiation)
    fn eval_spawn(&mut self, var_name: String, agent_name: String) -> Result<(), String> {
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

        println!(
            "[Agent Spawned] {} as {} (permissions: {:?})",
            agent_name, var_name, def.allow_list
        );
        Ok(())
    }

    // Day 3 - Step 9: Agent Definition Registration
    fn eval_agent_def(
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
        println!(
            "[Agent Defined] {} (allows {} tools)",
            name,
            allow_list.len()
        );
        Ok(())
    }

    // Day 3 - Step 8: Tool Registration
    fn eval_tool_def(
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
        println!(
            "[Tool Registered] {} with {} params",
            name,
            self.tools.get(&name).unwrap().params.len()
        );
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
        }
    }
}

#[cfg(test)]
mod eval_tests;
