#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    String(String),
    Number(f64),
    Var(String),
    Agent(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Let {
        name: String,
        value: Expr,
    },
    Print(Expr),
    Think(Expr),
    Gate {
        prompt: Expr,
        body: Vec<Statement>,
    },
    AgentBlock {
        name: String,
        body: Vec<Statement>,
    },
}

pub struct Program {
    pub statements: Vec<Statement>,
}
