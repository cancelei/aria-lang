use std::collections::HashMap;
use crate::ast::{Program, Statement, Expr, TaskDef};

// Day 3: Tool definition storage
#[derive(Debug, Clone)]
pub struct Tool {
    pub name: String,
    pub params: Vec<String>,
    pub permission: Option<String>,
    pub timeout: Option<f64>,
}

// Day 3: Agent definition (the "type" of agent)
#[derive(Debug, Clone)]
pub struct AgentDef {
    pub name: String,
    pub allow_list: Vec<String>,
    pub tasks: Vec<TaskDef>,
    pub body: Vec<Statement>,
}

// Day 3: Agent instance (a running agent with scoped permissions)
#[derive(Debug, Clone)]
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
                println!("[GATE] {}", match p {
                    Value::String(s) => s,
                    _ => format!("{:?}", p),
                });
                println!("(Simulating Human Approval: Press Enter to Continue)");
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).map_err(|e| e.to_string())?;
                
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
            Statement::ToolDef { name, params, permission, timeout } => {
                self.eval_tool_def(name, params, permission, timeout)?;
            }
            Statement::AgentDef { name, allow_list, tasks, body } => {
                self.eval_agent_def(name, allow_list, tasks, body)?;
            }
            Statement::TaskDef(_) => {
                // TODO: Implement task definitions
                return Err("Task definitions not yet implemented".to_string());
            }
            Statement::Spawn { var_name, agent_name } => {
                self.eval_spawn(var_name, agent_name)?;
            }
            Statement::Delegate { .. } => {
                // TODO: Implement delegate
                return Err("Delegate not yet implemented".to_string());
            }
            Statement::Main { body } => {
                println!("[Entering Main Block]");
                for s in body {
                    self.eval_statement(s)?;
                }
                println!("[Exiting Main Block]");
            }
            Statement::Return(_) => {
                // TODO: Implement return
                return Err("Return not yet implemented".to_string());
            }
        }
        Ok(())
    }

    // Day 3 - Step 11: Function Calls with Permission Checking (THE PHYSICS!)
    fn eval_call(&mut self, name: &str, args: Vec<Expr>) -> Result<Value, String> {
        // Check if tool is defined and clone permission (to avoid borrow issues)
        let permission = {
            let tool = self.tools.get(name)
                .ok_or(format!("Tool '{}' is not defined", name))?;
            tool.permission.clone()
        };

        // TODO: For now, we'll skip permission checking since we don't track "current agent context"
        // In a real implementation, we'd need to know which agent is making the call
        // For now, just log and execute

        // Evaluate arguments
        let mut evaluated_args = Vec::new();
        for arg in args {
            evaluated_args.push(self.eval_expr(arg)?);
        }

        // Log the call (in real implementation, this would actually execute)
        println!("[Tool Call] {} with {} args (permission: {:?})",
                 name, evaluated_args.len(), permission);

        // Return a dummy result
        Ok(Value::String(format!("Result from {}", name)))
    }

    // Day 3 - Step 10: Agent Spawning (Instantiation)
    fn eval_spawn(
        &mut self,
        var_name: String,
        agent_name: String,
    ) -> Result<(), String> {
        let def = self.agent_defs.get(&agent_name)
            .ok_or(format!("Agent '{}' not defined", agent_name))?;

        let instance = AgentInstance {
            name: var_name.clone(),
            agent_def_name: agent_name.clone(),
            allowed_tools: def.allow_list.clone(),
            variables: HashMap::new(),
        };

        self.agents.insert(var_name.clone(), instance);
        self.variables.insert(var_name.clone(), Value::Agent(var_name.clone()));

        println!("[Agent Spawned] {} as {} (permissions: {:?})",
                 agent_name, var_name, def.allow_list);
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
        println!("[Agent Defined] {} (allows {} tools)", name, allow_list.len());
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
        println!("[Tool Registered] {} with {} params", name, self.tools.get(&name).unwrap().params.len());
        Ok(())
    }

    fn eval_expr(&mut self, expr: Expr) -> Result<Value, String> {
        match expr {
            Expr::String(s) => Ok(Value::String(s)),
            Expr::Number(n) => Ok(Value::Number(n)),
            Expr::Var(v) => self.variables.get(&v).cloned().ok_or(format!("Undefined variable: {}", v)),
            Expr::Agent(a) => Ok(Value::String(format!("Context:{}", a))),
            Expr::Call { name, args } => {
                self.eval_call(&name, args)
            }
            Expr::MemberAccess { .. } => {
                // TODO: Implement member access
                Err("Member access not yet implemented".to_string())
            }
        }
    }
}

#[cfg(test)]
mod eval_tests;
