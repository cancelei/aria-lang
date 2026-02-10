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
    #[allow(dead_code)]
    MemberAccess {
        object: Box<Expr>,
        member: String,
    },
    /// Array literal expression: [a, b, c]
    Array(Vec<Expr>),
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

    // ========================================================================
    // M21: MCP Tool Definitions
    // ========================================================================
    /// Tool sourced from an MCP server
    /// e.g., tool code_search from mcp("github-server") { permission: "mcp.connect", ... }
    McpToolDef {
        name: String,
        server: String,
        permission: Option<String>,
        capabilities: Vec<String>,
        timeout: Option<f64>,
    },

    // ========================================================================
    // M22: Multi-Agent Orchestration
    // ========================================================================
    /// Sequential pipeline: stages execute in order, each receives previous output
    /// pipeline ReviewPipeline { stage Analyst -> analyze($input) ... }
    PipelineDef {
        name: String,
        stages: Vec<PipelineStage>,
    },
    /// Concurrent fan-out: agents work in parallel, results merged
    /// concurrent ResearchTask { agent A -> do_a($q) ... merge combine($results) }
    ConcurrentDef {
        name: String,
        branches: Vec<ConcurrentBranch>,
        merge_fn: Option<Expr>,
    },
    /// Handoff: route to different agents based on conditions
    /// handoff SupportFlow { agent Triage -> classify($input) route "x" => AgentX ... }
    HandoffDef {
        name: String,
        agent_name: String,
        agent_call: Expr,
        routes: Vec<HandoffRoute>,
    },

    // ========================================================================
    // M23: A2A Protocol
    // ========================================================================
    /// Declare A2A capabilities for an agent
    /// a2a AgentCard { discovery: "/.well-known/agent.json", skills: [...], endpoint: "..." }
    A2ADef {
        name: String,
        discovery: Option<String>,
        skills: Vec<String>,
        endpoint: Option<String>,
    },

    // ========================================================================
    // M24: Workflow State Machines
    // ========================================================================
    /// Stateful workflow with typed transitions
    /// workflow OrderProcessing { state pending { on receive_order -> validating } ... }
    WorkflowDef {
        name: String,
        states: Vec<WorkflowState>,
    },

    // ========================================================================
    // M25: Model Declarations
    // ========================================================================
    /// Declarative model requirements
    /// model assistant { capability: "chat_completion", supports: [...] }
    ModelDef {
        name: String,
        capability: Option<String>,
        provider: Option<String>,
        supports: Vec<String>,
    },

    // ========================================================================
    // M26: Agent Memory
    // ========================================================================
    /// Type-safe agent memory with vector store
    /// memory ProjectKnowledge { store: "chromadb://...", embedding: model text_embedder }
    MemoryDef {
        name: String,
        store: Option<String>,
        embedding_model: Option<String>,
        operations: Vec<String>,
    },
}

// ============================================================================
// Supporting types
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct TaskDef {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PipelineStage {
    pub agent_name: String,
    pub call: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConcurrentBranch {
    pub agent_name: String,
    pub call: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HandoffRoute {
    pub pattern: String,     // match pattern (string literal or "_" for default)
    pub target_agent: String, // agent to route to
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowState {
    pub name: String,
    pub transitions: Vec<WorkflowTransition>,
    pub requires: Option<Expr>,
    pub ensures: Option<Expr>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowTransition {
    pub event: String,
    pub target_state: String,
}

#[derive(Debug)]
pub struct Program {
    pub statements: Vec<Statement>,
}
