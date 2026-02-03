#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    String(String),
    Number(f64),
    Var(String),
    Agent(String),
    Call {
        name: String,
        args: Vec<Expr>,
    },
    MemberAccess {
        object: Box<Expr>,
        member: String,
    },
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
    ToolDef {
        name: String,
        params: Vec<String>,
        permission: Option<String>,
        timeout: Option<f64>,
    },
    AgentDef {
        name: String,
        allow_list: Vec<String>,
        tasks: Vec<TaskDef>,
        body: Vec<Statement>,
    },
    TaskDef(TaskDef),
    Spawn {
        var_name: String,
        agent_name: String,
    },
    Delegate {
        call: Expr,
    },
    Main {
        body: Vec<Statement>,
    },
    Return(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskDef {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Statement>,
}

pub struct Program {
    pub statements: Vec<Statement>,
}
