use crate::ast::{
    ConcurrentBranch, Expr, HandoffRoute, PipelineStage, Program, Statement, WorkflowState,
    WorkflowTransition,
};
use crate::lexer::Token;
use logos::{Lexer, Logos};

pub struct Parser<'a> {
    lexer: Lexer<'a, Token>,
    current: Option<Result<Token, ()>>,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut lexer = Token::lexer(input);
        let current = lexer.next();
        Self { lexer, current }
    }

    fn advance(&mut self) {
        self.current = self.lexer.next();
    }

    fn expect(&mut self, expected: Token) -> Result<(), String> {
        if let Some(Ok(token)) = &self.current
            && *token == expected
        {
            self.advance();
            return Ok(());
        }
        Err(format!("Expected {:?}, found {:?}", expected, self.current))
    }

    pub fn parse_program(&mut self) -> Result<Program, String> {
        let mut statements = Vec::new();
        while self.current.is_some() {
            statements.push(self.parse_statement()?);
        }
        Ok(Program { statements })
    }

    fn parse_statement(&mut self) -> Result<Statement, String> {
        match &self.current {
            Some(Ok(Token::Let)) => {
                self.advance();
                // Check if it's a spawn statement
                if let Some(Ok(Token::VarIdent(name))) = &self.current {
                    let var_name = name.clone();
                    self.advance();
                    self.expect(Token::Assign)?;

                    // Check for spawn keyword
                    if let Some(Ok(Token::Spawn)) = &self.current {
                        return self.parse_spawn(var_name);
                    }

                    let value = self.parse_expr()?;
                    Ok(Statement::Let {
                        name: var_name,
                        value,
                    })
                } else {
                    Err("Expected variable name after 'let'".to_string())
                }
            }
            Some(Ok(Token::Print)) => {
                self.advance();
                let value = self.parse_expr()?;
                Ok(Statement::Print(value))
            }
            Some(Ok(Token::Think)) => {
                self.advance();
                self.expect(Token::LBrace)?;
                let value = self.parse_expr()?;
                self.expect(Token::RBrace)?;
                Ok(Statement::Think(value))
            }
            Some(Ok(Token::Gate)) => {
                self.advance();
                let prompt = self.parse_expr()?;
                self.expect(Token::LBrace)?;
                let mut body = Vec::new();
                while let Some(Ok(token)) = &self.current {
                    if *token == Token::RBrace {
                        break;
                    }
                    body.push(self.parse_statement()?);
                }
                self.expect(Token::RBrace)?;
                Ok(Statement::Gate { prompt, body })
            }
            Some(Ok(Token::Agent)) => {
                // Check if this is an agent definition or agent block
                self.advance();
                let name = if let Some(Ok(Token::Ident(name))) = &self.current {
                    let n = name.clone();
                    self.advance();
                    n
                } else {
                    "anonymous".to_string()
                };

                self.expect(Token::LBrace)?;

                // Try to parse as AgentDef (with allow/task)
                let mut allow_list = Vec::new();
                let mut tasks = Vec::new();
                let mut body = Vec::new();

                while let Some(Ok(token)) = &self.current {
                    if *token == Token::RBrace {
                        break;
                    }

                    match token {
                        Token::Allow => {
                            self.advance();
                            if let Some(Ok(Token::Ident(tool_name))) = &self.current {
                                allow_list.push(tool_name.clone());
                                self.advance();
                            } else {
                                return Err("Expected tool name after 'allow'".to_string());
                            }
                        }
                        Token::Task => {
                            tasks.push(self.parse_task_def()?);
                        }
                        _ => {
                            body.push(self.parse_statement()?);
                        }
                    }
                }

                self.expect(Token::RBrace)?;

                // If we have allow_list or tasks, return AgentDef, otherwise AgentBlock
                if !allow_list.is_empty() || !tasks.is_empty() {
                    Ok(Statement::AgentDef {
                        name,
                        allow_list,
                        tasks,
                        body,
                    })
                } else {
                    Ok(Statement::AgentBlock { name, body })
                }
            }
            Some(Ok(Token::Tool)) => self.parse_tool_def(),
            Some(Ok(Token::Task)) => {
                let task = self.parse_task_def()?;
                Ok(Statement::TaskDef(task))
            }
            Some(Ok(Token::Delegate)) => self.parse_delegate(),
            Some(Ok(Token::Main)) => self.parse_main(),
            Some(Ok(Token::Return)) => {
                self.advance();
                let value = self.parse_expr()?;
                Ok(Statement::Return(value))
            }
            // M21: MCP tool definitions (tool ... from mcp(...))
            // handled inside parse_tool_def via 'from' lookahead

            // M22: Orchestration primitives
            Some(Ok(Token::Pipeline)) => self.parse_pipeline(),
            Some(Ok(Token::Concurrent)) => self.parse_concurrent(),
            Some(Ok(Token::Handoff)) => self.parse_handoff(),

            // M23: A2A protocol
            Some(Ok(Token::A2A)) => self.parse_a2a(),

            // M24: Workflow state machines
            Some(Ok(Token::Workflow)) => self.parse_workflow(),

            // M25: Model declarations
            Some(Ok(Token::Model)) => self.parse_model(),

            // M26: Memory definitions
            Some(Ok(Token::Memory)) => self.parse_memory(),

            _ => Err(format!("Unexpected token in statement: {:?}", self.current)),
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;

        // Check for function call or member access
        while let Some(Ok(Token::LParen)) = &self.current {
            expr = self.parse_call(expr)?;
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        let token = self.current.clone();
        match token {
            Some(Ok(Token::String(s))) => {
                self.advance();
                Ok(Expr::String(s))
            }
            Some(Ok(Token::Number(n))) => {
                self.advance();
                Ok(Expr::Number(n))
            }
            Some(Ok(Token::VarIdent(v))) => {
                self.advance();
                Ok(Expr::Var(v))
            }
            Some(Ok(Token::AgentIdent(a))) => {
                self.advance();
                Ok(Expr::Agent(a))
            }
            Some(Ok(Token::Ident(i))) => {
                let name = i.clone();
                self.advance();
                Ok(Expr::Var(name))
            }
            // Array literal: [a, b, c]
            Some(Ok(Token::LBracket)) => {
                self.advance();
                let mut elements = Vec::new();
                while let Some(Ok(token)) = &self.current {
                    if *token == Token::RBracket {
                        break;
                    }
                    elements.push(self.parse_expr()?);
                    if let Some(Ok(Token::Comma)) = &self.current {
                        self.advance();
                    }
                }
                self.expect(Token::RBracket)?;
                Ok(Expr::Array(elements))
            }
            t => Err(format!("Expected expression, found {:?}", t)),
        }
    }

    // ========================================================================
    // Tool definition parsing (original + MCP extension)
    // ========================================================================

    fn parse_tool_def(&mut self) -> Result<Statement, String> {
        self.advance(); // consume 'tool'

        // Parse tool name
        let name = if let Some(Ok(Token::Ident(n))) = &self.current {
            let name = n.clone();
            self.advance();
            name
        } else {
            return Err("Expected tool name after 'tool'".to_string());
        };

        // M21: Check for 'from mcp(...)' — MCP tool definition
        if let Some(Ok(Token::From)) = &self.current {
            return self.parse_mcp_tool_def(name);
        }

        // Parse parameters
        self.expect(Token::LParen)?;
        let mut params = Vec::new();
        while let Some(Ok(token)) = &self.current {
            if *token == Token::RParen {
                break;
            }

            if let Token::Ident(param) = token {
                params.push(param.clone());
                self.advance();

                // Skip optional type annotation (e.g., : string)
                if let Some(Ok(Token::Colon)) = &self.current {
                    self.advance();
                    // Skip the type identifier
                    if let Some(Ok(Token::Ident(_))) = &self.current {
                        self.advance();
                    }
                }

                // Handle comma
                if let Some(Ok(Token::Comma)) = &self.current {
                    self.advance();
                }
            } else {
                break;
            }
        }
        self.expect(Token::RParen)?;

        // Parse body with permission and timeout
        self.expect(Token::LBrace)?;
        let mut permission = None;
        let mut timeout = None;

        while let Some(Ok(token)) = &self.current {
            if *token == Token::RBrace {
                break;
            }

            match token {
                Token::Permission => {
                    self.advance();
                    self.expect(Token::Colon)?;
                    if let Some(Ok(Token::String(perm))) = &self.current {
                        permission = Some(perm.clone());
                        self.advance();
                    }
                    // Skip optional comma
                    if let Some(Ok(Token::Comma)) = &self.current {
                        self.advance();
                    }
                }
                Token::Timeout => {
                    self.advance();
                    self.expect(Token::Colon)?;
                    if let Some(Ok(Token::Number(t))) = &self.current {
                        timeout = Some(*t);
                        self.advance();
                    }
                    // Skip optional comma
                    if let Some(Ok(Token::Comma)) = &self.current {
                        self.advance();
                    }
                }
                _ => {
                    return Err(format!("Unexpected token in tool body: {:?}", token));
                }
            }
        }

        self.expect(Token::RBrace)?;

        Ok(Statement::ToolDef {
            name,
            params,
            permission,
            timeout,
        })
    }

    // ========================================================================
    // M21: MCP Tool Definition
    // tool code_search from mcp("github-server") {
    //     permission: "mcp.connect",
    //     capabilities: [search_code, search_issues],
    //     timeout: 15
    // }
    // ========================================================================

    fn parse_mcp_tool_def(&mut self, name: String) -> Result<Statement, String> {
        self.advance(); // consume 'from'
        self.expect(Token::Mcp)?;
        self.expect(Token::LParen)?;

        let server = if let Some(Ok(Token::String(s))) = &self.current {
            let srv = s.clone();
            self.advance();
            srv
        } else {
            return Err("Expected MCP server name string after 'mcp('".to_string());
        };

        self.expect(Token::RParen)?;
        self.expect(Token::LBrace)?;

        let mut permission = None;
        let mut capabilities = Vec::new();
        let mut timeout = None;

        while let Some(Ok(token)) = &self.current {
            if *token == Token::RBrace {
                break;
            }

            match token {
                Token::Permission => {
                    self.advance();
                    self.expect(Token::Colon)?;
                    if let Some(Ok(Token::String(perm))) = &self.current {
                        permission = Some(perm.clone());
                        self.advance();
                    }
                    if let Some(Ok(Token::Comma)) = &self.current {
                        self.advance();
                    }
                }
                Token::Capabilities => {
                    self.advance();
                    self.expect(Token::Colon)?;
                    self.expect(Token::LBracket)?;
                    while let Some(Ok(token)) = &self.current {
                        if *token == Token::RBracket {
                            break;
                        }
                        if let Token::Ident(cap) = token {
                            capabilities.push(cap.clone());
                            self.advance();
                        } else {
                            return Err(format!(
                                "Expected identifier in capabilities list, found {:?}",
                                token
                            ));
                        }
                        if let Some(Ok(Token::Comma)) = &self.current {
                            self.advance();
                        }
                    }
                    self.expect(Token::RBracket)?;
                    if let Some(Ok(Token::Comma)) = &self.current {
                        self.advance();
                    }
                }
                Token::Timeout => {
                    self.advance();
                    self.expect(Token::Colon)?;
                    if let Some(Ok(Token::Number(t))) = &self.current {
                        timeout = Some(*t);
                        self.advance();
                    }
                    if let Some(Ok(Token::Comma)) = &self.current {
                        self.advance();
                    }
                }
                _ => {
                    return Err(format!(
                        "Unexpected token in MCP tool body: {:?}",
                        token
                    ));
                }
            }
        }

        self.expect(Token::RBrace)?;

        Ok(Statement::McpToolDef {
            name,
            server,
            permission,
            capabilities,
            timeout,
        })
    }

    // ========================================================================
    // M22: Pipeline Orchestration
    // pipeline ReviewPipeline {
    //     stage Analyst -> analyze($input)
    //     stage Reviewer -> review($prev)
    // }
    // ========================================================================

    fn parse_pipeline(&mut self) -> Result<Statement, String> {
        self.advance(); // consume 'pipeline'

        let name = if let Some(Ok(Token::Ident(n))) = &self.current {
            let name = n.clone();
            self.advance();
            name
        } else {
            return Err("Expected pipeline name".to_string());
        };

        self.expect(Token::LBrace)?;

        let mut stages = Vec::new();
        while let Some(Ok(token)) = &self.current {
            if *token == Token::RBrace {
                break;
            }

            if *token == Token::Stage {
                self.advance(); // consume 'stage'
                let agent_name = if let Some(Ok(Token::Ident(n))) = &self.current {
                    let n = n.clone();
                    self.advance();
                    n
                } else {
                    return Err("Expected agent name after 'stage'".to_string());
                };

                self.expect(Token::Arrow)?;
                let call = self.parse_expr()?;

                stages.push(PipelineStage { agent_name, call });
            } else {
                return Err(format!(
                    "Expected 'stage' in pipeline body, found {:?}",
                    token
                ));
            }
        }

        self.expect(Token::RBrace)?;

        Ok(Statement::PipelineDef { name, stages })
    }

    // ========================================================================
    // M22: Concurrent Orchestration
    // concurrent ResearchTask {
    //     agent WebSearcher -> search_web($query)
    //     agent CodeSearcher -> search_codebase($query)
    //     merge combine_results($results)
    // }
    // ========================================================================

    fn parse_concurrent(&mut self) -> Result<Statement, String> {
        self.advance(); // consume 'concurrent'

        let name = if let Some(Ok(Token::Ident(n))) = &self.current {
            let name = n.clone();
            self.advance();
            name
        } else {
            return Err("Expected concurrent block name".to_string());
        };

        self.expect(Token::LBrace)?;

        let mut branches = Vec::new();
        let mut merge_fn = None;

        while let Some(Ok(token)) = &self.current {
            if *token == Token::RBrace {
                break;
            }

            match token {
                Token::Agent => {
                    self.advance(); // consume 'agent'
                    let agent_name = if let Some(Ok(Token::Ident(n))) = &self.current {
                        let n = n.clone();
                        self.advance();
                        n
                    } else {
                        return Err("Expected agent name after 'agent'".to_string());
                    };

                    self.expect(Token::Arrow)?;
                    let call = self.parse_expr()?;

                    branches.push(ConcurrentBranch { agent_name, call });
                }
                Token::Merge => {
                    self.advance(); // consume 'merge'
                    merge_fn = Some(self.parse_expr()?);
                }
                _ => {
                    return Err(format!(
                        "Expected 'agent' or 'merge' in concurrent body, found {:?}",
                        token
                    ));
                }
            }
        }

        self.expect(Token::RBrace)?;

        Ok(Statement::ConcurrentDef {
            name,
            branches,
            merge_fn,
        })
    }

    // ========================================================================
    // M22: Handoff Orchestration
    // handoff SupportFlow {
    //     agent Triage -> classify($input)
    //     route "billing" => BillingAgent
    //     route "technical" => TechAgent
    //     route _ => HumanEscalation
    // }
    // ========================================================================

    fn parse_handoff(&mut self) -> Result<Statement, String> {
        self.advance(); // consume 'handoff'

        let name = if let Some(Ok(Token::Ident(n))) = &self.current {
            let name = n.clone();
            self.advance();
            name
        } else {
            return Err("Expected handoff name".to_string());
        };

        self.expect(Token::LBrace)?;

        // Parse the initial agent classification
        let mut agent_name = String::new();
        let mut agent_call = Expr::String("".to_string());
        let mut routes = Vec::new();

        while let Some(Ok(token)) = &self.current {
            if *token == Token::RBrace {
                break;
            }

            match token {
                Token::Agent => {
                    self.advance();
                    agent_name = if let Some(Ok(Token::Ident(n))) = &self.current {
                        let n = n.clone();
                        self.advance();
                        n
                    } else {
                        return Err("Expected agent name in handoff".to_string());
                    };

                    self.expect(Token::Arrow)?;
                    agent_call = self.parse_expr()?;
                }
                Token::Route => {
                    self.advance();
                    // Parse pattern: string literal or _ (wildcard)
                    let pattern = match &self.current {
                        Some(Ok(Token::String(s))) => {
                            let p = s.clone();
                            self.advance();
                            p
                        }
                        Some(Ok(Token::Wildcard)) => {
                            self.advance();
                            "_".to_string()
                        }
                        _ => return Err("Expected string or '_' after 'route'".to_string()),
                    };

                    self.expect(Token::FatArrow)?;

                    let target_agent = if let Some(Ok(Token::Ident(n))) = &self.current {
                        let n = n.clone();
                        self.advance();
                        n
                    } else {
                        return Err("Expected target agent after '=>'".to_string());
                    };

                    routes.push(HandoffRoute {
                        pattern,
                        target_agent,
                    });
                }
                _ => {
                    return Err(format!(
                        "Expected 'agent' or 'route' in handoff body, found {:?}",
                        token
                    ));
                }
            }
        }

        self.expect(Token::RBrace)?;

        Ok(Statement::HandoffDef {
            name,
            agent_name,
            agent_call,
            routes,
        })
    }

    // ========================================================================
    // M23: A2A Protocol Definition
    // a2a ResearchCard {
    //     discovery: "/.well-known/agent.json"
    //     skills: [search, analyze, summarize]
    //     endpoint: "https://agents.aria.dev/research"
    // }
    // ========================================================================

    fn parse_a2a(&mut self) -> Result<Statement, String> {
        self.advance(); // consume 'a2a'

        let name = if let Some(Ok(Token::Ident(n))) = &self.current {
            let name = n.clone();
            self.advance();
            name
        } else {
            return Err("Expected A2A definition name".to_string());
        };

        self.expect(Token::LBrace)?;

        let mut discovery = None;
        let mut skills = Vec::new();
        let mut endpoint = None;

        while let Some(Ok(token)) = &self.current {
            if *token == Token::RBrace {
                break;
            }

            match token {
                Token::Discovery => {
                    self.advance();
                    self.expect(Token::Colon)?;
                    if let Some(Ok(Token::String(s))) = &self.current {
                        discovery = Some(s.clone());
                        self.advance();
                    }
                    if let Some(Ok(Token::Comma)) = &self.current {
                        self.advance();
                    }
                }
                Token::Skills => {
                    self.advance();
                    self.expect(Token::Colon)?;
                    self.expect(Token::LBracket)?;
                    while let Some(Ok(token)) = &self.current {
                        if *token == Token::RBracket {
                            break;
                        }
                        if let Token::Ident(skill) = token {
                            skills.push(skill.clone());
                            self.advance();
                        } else {
                            return Err(format!(
                                "Expected identifier in skills list, found {:?}",
                                token
                            ));
                        }
                        if let Some(Ok(Token::Comma)) = &self.current {
                            self.advance();
                        }
                    }
                    self.expect(Token::RBracket)?;
                    if let Some(Ok(Token::Comma)) = &self.current {
                        self.advance();
                    }
                }
                Token::Endpoint => {
                    self.advance();
                    self.expect(Token::Colon)?;
                    if let Some(Ok(Token::String(s))) = &self.current {
                        endpoint = Some(s.clone());
                        self.advance();
                    }
                    if let Some(Ok(Token::Comma)) = &self.current {
                        self.advance();
                    }
                }
                _ => {
                    return Err(format!(
                        "Unexpected token in A2A body: {:?}",
                        token
                    ));
                }
            }
        }

        self.expect(Token::RBrace)?;

        Ok(Statement::A2ADef {
            name,
            discovery,
            skills,
            endpoint,
        })
    }

    // ========================================================================
    // M24: Workflow State Machine
    // workflow OrderProcessing {
    //     state pending {
    //         on receive_order -> validating
    //     }
    //     state validating {
    //         requires $order.is_valid
    //         on valid -> processing
    //         on invalid -> rejected
    //     }
    // }
    // ========================================================================

    fn parse_workflow(&mut self) -> Result<Statement, String> {
        self.advance(); // consume 'workflow'

        let name = if let Some(Ok(Token::Ident(n))) = &self.current {
            let name = n.clone();
            self.advance();
            name
        } else {
            return Err("Expected workflow name".to_string());
        };

        self.expect(Token::LBrace)?;

        let mut states = Vec::new();

        while let Some(Ok(token)) = &self.current {
            if *token == Token::RBrace {
                break;
            }

            if *token == Token::State {
                self.advance(); // consume 'state'

                let state_name = if let Some(Ok(Token::Ident(n))) = &self.current {
                    let n = n.clone();
                    self.advance();
                    n
                } else {
                    return Err("Expected state name".to_string());
                };

                self.expect(Token::LBrace)?;

                let mut transitions = Vec::new();
                let mut requires = None;
                let mut ensures = None;
                let mut body = Vec::new();

                while let Some(Ok(token)) = &self.current {
                    if *token == Token::RBrace {
                        break;
                    }

                    match token {
                        Token::On => {
                            self.advance();
                            let event = if let Some(Ok(Token::Ident(e))) = &self.current {
                                let e = e.clone();
                                self.advance();
                                e
                            } else {
                                return Err("Expected event name after 'on'".to_string());
                            };

                            self.expect(Token::Arrow)?;

                            let target = if let Some(Ok(Token::Ident(t))) = &self.current {
                                let t = t.clone();
                                self.advance();
                                t
                            } else {
                                return Err("Expected target state after '->'".to_string());
                            };

                            transitions.push(WorkflowTransition {
                                event,
                                target_state: target,
                            });
                        }
                        Token::Requires => {
                            self.advance();
                            requires = Some(self.parse_expr()?);
                        }
                        Token::Ensures => {
                            self.advance();
                            ensures = Some(self.parse_expr()?);
                        }
                        Token::Gate => {
                            body.push(self.parse_statement()?);
                        }
                        Token::Print => {
                            body.push(self.parse_statement()?);
                        }
                        Token::Delegate => {
                            body.push(self.parse_statement()?);
                        }
                        _ => {
                            body.push(self.parse_statement()?);
                        }
                    }
                }

                self.expect(Token::RBrace)?;

                states.push(WorkflowState {
                    name: state_name,
                    transitions,
                    requires,
                    ensures,
                    body,
                });
            } else {
                return Err(format!(
                    "Expected 'state' in workflow body, found {:?}",
                    token
                ));
            }
        }

        self.expect(Token::RBrace)?;

        Ok(Statement::WorkflowDef { name, states })
    }

    // ========================================================================
    // M25: Model Declaration
    // model assistant {
    //     capability: "chat_completion"
    //     provider: "openai"
    //     supports: [tool_calling, structured_output]
    // }
    // ========================================================================

    fn parse_model(&mut self) -> Result<Statement, String> {
        self.advance(); // consume 'model'

        let name = if let Some(Ok(Token::Ident(n))) = &self.current {
            let name = n.clone();
            self.advance();
            name
        } else {
            return Err("Expected model name".to_string());
        };

        self.expect(Token::LBrace)?;

        let mut capability = None;
        let mut provider = None;
        let mut supports = Vec::new();

        while let Some(Ok(token)) = &self.current {
            if *token == Token::RBrace {
                break;
            }

            match token {
                Token::Capability => {
                    self.advance();
                    self.expect(Token::Colon)?;
                    if let Some(Ok(Token::String(s))) = &self.current {
                        capability = Some(s.clone());
                        self.advance();
                    }
                    if let Some(Ok(Token::Comma)) = &self.current {
                        self.advance();
                    }
                }
                Token::Provider => {
                    self.advance();
                    self.expect(Token::Colon)?;
                    if let Some(Ok(Token::String(s))) = &self.current {
                        provider = Some(s.clone());
                        self.advance();
                    }
                    if let Some(Ok(Token::Comma)) = &self.current {
                        self.advance();
                    }
                }
                Token::Supports => {
                    self.advance();
                    self.expect(Token::Colon)?;
                    self.expect(Token::LBracket)?;
                    while let Some(Ok(token)) = &self.current {
                        if *token == Token::RBracket {
                            break;
                        }
                        if let Token::Ident(s) = token {
                            supports.push(s.clone());
                            self.advance();
                        } else {
                            return Err(format!(
                                "Expected identifier in supports list, found {:?}",
                                token
                            ));
                        }
                        if let Some(Ok(Token::Comma)) = &self.current {
                            self.advance();
                        }
                    }
                    self.expect(Token::RBracket)?;
                    if let Some(Ok(Token::Comma)) = &self.current {
                        self.advance();
                    }
                }
                _ => {
                    return Err(format!(
                        "Unexpected token in model body: {:?}",
                        token
                    ));
                }
            }
        }

        self.expect(Token::RBrace)?;

        Ok(Statement::ModelDef {
            name,
            capability,
            provider,
            supports,
        })
    }

    // ========================================================================
    // M26: Memory Definition
    // memory ProjectKnowledge {
    //     store: "chromadb://localhost:8000/project"
    //     embedding: "text_embedder"
    //     operations: [remember, recall, forget]
    // }
    // ========================================================================

    fn parse_memory(&mut self) -> Result<Statement, String> {
        self.advance(); // consume 'memory'

        let name = if let Some(Ok(Token::Ident(n))) = &self.current {
            let name = n.clone();
            self.advance();
            name
        } else {
            return Err("Expected memory name".to_string());
        };

        self.expect(Token::LBrace)?;

        let mut store = None;
        let mut embedding_model = None;
        let mut operations = Vec::new();

        while let Some(Ok(token)) = &self.current {
            if *token == Token::RBrace {
                break;
            }

            match token {
                Token::Store => {
                    self.advance();
                    self.expect(Token::Colon)?;
                    if let Some(Ok(Token::String(s))) = &self.current {
                        store = Some(s.clone());
                        self.advance();
                    }
                    if let Some(Ok(Token::Comma)) = &self.current {
                        self.advance();
                    }
                }
                Token::Embedding => {
                    self.advance();
                    self.expect(Token::Colon)?;
                    if let Some(Ok(Token::String(s))) = &self.current {
                        embedding_model = Some(s.clone());
                        self.advance();
                    }
                    if let Some(Ok(Token::Comma)) = &self.current {
                        self.advance();
                    }
                }
                Token::Operations => {
                    self.advance();
                    self.expect(Token::Colon)?;
                    self.expect(Token::LBracket)?;
                    while let Some(Ok(token)) = &self.current {
                        if *token == Token::RBracket {
                            break;
                        }
                        if let Token::Ident(op) = token {
                            operations.push(op.clone());
                            self.advance();
                        } else {
                            return Err(format!(
                                "Expected identifier in operations list, found {:?}",
                                token
                            ));
                        }
                        if let Some(Ok(Token::Comma)) = &self.current {
                            self.advance();
                        }
                    }
                    self.expect(Token::RBracket)?;
                    if let Some(Ok(Token::Comma)) = &self.current {
                        self.advance();
                    }
                }
                _ => {
                    return Err(format!(
                        "Unexpected token in memory body: {:?}",
                        token
                    ));
                }
            }
        }

        self.expect(Token::RBrace)?;

        Ok(Statement::MemoryDef {
            name,
            store,
            embedding_model,
            operations,
        })
    }

    // ========================================================================
    // Existing parse helpers
    // ========================================================================

    // Step 4: Parse function calls
    fn parse_call(&mut self, callee: Expr) -> Result<Expr, String> {
        self.expect(Token::LParen)?;

        let mut args = Vec::new();
        while let Some(Ok(token)) = &self.current {
            if *token == Token::RParen {
                break;
            }

            args.push(self.parse_expr()?);

            // Handle comma
            if let Some(Ok(Token::Comma)) = &self.current {
                self.advance();
            }
        }

        self.expect(Token::RParen)?;

        // Extract the function name from the callee
        let name = match callee {
            Expr::Var(n) => n,
            Expr::MemberAccess { object, member } => {
                // For member access like $bot.cleanup_logs, we need to handle differently
                // For now, let's just concatenate them
                if let Expr::Var(obj) = *object {
                    format!("{}.{}", obj, member)
                } else {
                    return Err("Invalid function call".to_string());
                }
            }
            _ => return Err("Invalid function call".to_string()),
        };

        Ok(Expr::Call { name, args })
    }

    // Parse task definitions (used in Step 5)
    fn parse_task_def(&mut self) -> Result<crate::ast::TaskDef, String> {
        self.advance(); // consume 'task'

        // Parse task name
        let name = if let Some(Ok(Token::Ident(n))) = &self.current {
            let name = n.clone();
            self.advance();
            name
        } else {
            return Err("Expected task name after 'task'".to_string());
        };

        // Parse parameters
        self.expect(Token::LParen)?;
        let mut params = Vec::new();
        while let Some(Ok(token)) = &self.current {
            if *token == Token::RParen {
                break;
            }

            if let Token::Ident(param) = token {
                params.push(param.clone());
                self.advance();

                // Skip optional type annotation
                if let Some(Ok(Token::Colon)) = &self.current {
                    self.advance();
                    if let Some(Ok(Token::Ident(_))) = &self.current {
                        self.advance();
                    }
                }

                // Handle comma
                if let Some(Ok(Token::Comma)) = &self.current {
                    self.advance();
                }
            } else {
                break;
            }
        }
        self.expect(Token::RParen)?;

        // Parse body
        self.expect(Token::LBrace)?;
        let mut body = Vec::new();
        while let Some(Ok(token)) = &self.current {
            if *token == Token::RBrace {
                break;
            }
            body.push(self.parse_statement()?);
        }
        self.expect(Token::RBrace)?;

        Ok(crate::ast::TaskDef { name, params, body })
    }

    // Step 6: Parse spawn statements
    fn parse_spawn(&mut self, var_name: String) -> Result<Statement, String> {
        self.advance(); // consume 'spawn'

        // Parse agent name
        let agent_name = if let Some(Ok(Token::Ident(name))) = &self.current {
            let n = name.clone();
            self.advance();
            n
        } else {
            return Err("Expected agent name after 'spawn'".to_string());
        };

        Ok(Statement::Spawn {
            var_name,
            agent_name,
        })
    }

    // Step 6: Parse delegate statements
    fn parse_delegate(&mut self) -> Result<Statement, String> {
        self.advance(); // consume 'delegate'

        // Parse the call expression (e.g., bot.cleanup_logs())
        let call = self.parse_delegate_call()?;

        Ok(Statement::Delegate { call })
    }

    fn parse_delegate_call(&mut self) -> Result<Expr, String> {
        // Parse variable (e.g., bot or $bot)
        let var_name = match &self.current {
            Some(Ok(Token::Ident(name))) => {
                let n = name.clone();
                self.advance();
                n
            }
            Some(Ok(Token::VarIdent(name))) => {
                let n = name.clone();
                self.advance();
                n
            }
            _ => return Err("Expected variable name in delegate".to_string()),
        };

        // Expect '.'
        self.expect(Token::Dot)?;

        let member = if let Some(Ok(Token::Ident(m))) = &self.current {
            let member = m.clone();
            self.advance();
            member
        } else {
            return Err("Expected member name after '.'".to_string());
        };

        // Parse the call
        self.expect(Token::LParen)?;
        let mut args = Vec::new();
        while let Some(Ok(token)) = &self.current {
            if *token == Token::RParen {
                break;
            }

            args.push(self.parse_expr()?);

            if let Some(Ok(Token::Comma)) = &self.current {
                self.advance();
            }
        }
        self.expect(Token::RParen)?;

        Ok(Expr::Call {
            name: format!("{}.{}", var_name, member),
            args,
        })
    }

    // Step 6: Parse main blocks
    fn parse_main(&mut self) -> Result<Statement, String> {
        self.advance(); // consume 'main'

        self.expect(Token::LBrace)?;
        let mut body = Vec::new();
        while let Some(Ok(token)) = &self.current {
            if *token == Token::RBrace {
                break;
            }
            body.push(self.parse_statement()?);
        }
        self.expect(Token::RBrace)?;

        Ok(Statement::Main { body })
    }
}

#[cfg(test)]
mod parser_tests;
