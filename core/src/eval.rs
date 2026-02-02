use std::collections::HashMap;
use crate::ast::{Program, Statement, Expr};

pub struct Evaluator {
    variables: HashMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Number(f64),
    Null,
}

impl Evaluator {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
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
        }
        Ok(())
    }

    fn eval_expr(&self, expr: Expr) -> Result<Value, String> {
        match expr {
            Expr::String(s) => Ok(Value::String(s)),
            Expr::Number(n) => Ok(Value::Number(n)),
            Expr::Var(v) => self.variables.get(&v).cloned().ok_or(format!("Undefined variable: {}", v)),
            Expr::Agent(a) => Ok(Value::String(format!("Context:{}", a))),
        }
    }
}
