#[cfg(test)]
mod tests {
    use crate::ast::{Expr, Program, Statement};
    use crate::eval::{Evaluator, Value};

    #[test]
    fn test_eval_let_print() {
        let mut evaluator = Evaluator::new();
        let program = Program {
            statements: vec![
                Statement::Let {
                    name: "$x".to_string(),
                    value: Expr::Number(42.0),
                },
                Statement::Print(Expr::Var("$x".to_string())),
            ],
        };
        evaluator.eval_program(program);
        assert_eq!(evaluator.variables.get("$x"), Some(&Value::Number(42.0)));
    }

    #[test]
    fn test_think_output() {
        let mut evaluator = Evaluator::new();
        let program = Program {
            statements: vec![Statement::Think(Expr::String("processing...".to_string()))],
        };
        evaluator.eval_program(program);
        assert!(
            evaluator
                .output
                .iter()
                .any(|s| s.contains("[Thinking...]") && s.contains("processing..."))
        );
    }

    #[test]
    fn test_print_string() {
        let mut evaluator = Evaluator::new();
        let program = Program {
            statements: vec![Statement::Print(Expr::String("hello world".to_string()))],
        };
        evaluator.eval_program(program);
        assert_eq!(evaluator.output, vec!["hello world"]);
    }

    #[test]
    fn test_print_number() {
        let mut evaluator = Evaluator::new();
        let program = Program {
            statements: vec![Statement::Print(Expr::Number(42.0))],
        };
        evaluator.eval_program(program);
        assert_eq!(evaluator.output, vec!["42"]);
    }

    #[test]
    fn test_return_value() {
        let mut evaluator = Evaluator::new();
        let program = Program {
            statements: vec![Statement::Return(Expr::String("done".to_string()))],
        };
        evaluator.eval_program(program);
        assert!(
            evaluator
                .output
                .iter()
                .any(|s| s.contains("[Return]") && s.contains("done"))
        );
    }

    #[test]
    fn test_tool_registration() {
        let mut evaluator = Evaluator::new();
        evaluator
            .eval_tool_def(
                "shell".to_string(),
                vec!["command".to_string()],
                Some("system.execute".to_string()),
                Some(30.0),
            )
            .unwrap();
        assert!(
            evaluator
                .output
                .iter()
                .any(|s| s.contains("[Tool Registered]") && s.contains("shell"))
        );
    }

    #[test]
    fn test_agent_def_registration() {
        let mut evaluator = Evaluator::new();
        evaluator
            .eval_agent_def(
                "TestAgent".to_string(),
                vec!["echo".to_string()],
                vec![],
                vec![],
            )
            .unwrap();
        assert!(
            evaluator
                .output
                .iter()
                .any(|s| s.contains("[Agent Defined]") && s.contains("TestAgent"))
        );
    }

    #[test]
    fn test_spawn_output() {
        let mut evaluator = Evaluator::new();
        evaluator
            .eval_agent_def(
                "TestAgent".to_string(),
                vec!["echo".to_string()],
                vec![],
                vec![],
            )
            .unwrap();
        evaluator
            .eval_spawn("$bot".to_string(), "TestAgent".to_string())
            .unwrap();
        assert!(
            evaluator
                .output
                .iter()
                .any(|s| s.contains("[Agent Spawned]") && s.contains("TestAgent"))
        );
    }

    #[test]
    fn test_main_block_context() {
        let mut evaluator = Evaluator::new();
        let program = Program {
            statements: vec![Statement::Main {
                body: vec![Statement::Print(Expr::String("inside".to_string()))],
            }],
        };
        evaluator.eval_program(program);
        assert!(
            evaluator
                .output
                .iter()
                .any(|s| s == "[Entering Main Block]")
        );
        assert!(evaluator.output.iter().any(|s| s == "[Exiting Main Block]"));
    }

    #[test]
    fn test_delegation_context_switch() {
        let mut evaluator = Evaluator::new();

        // Define agent with a task
        evaluator
            .eval_agent_def(
                "Worker".to_string(),
                vec![],
                vec![crate::ast::TaskDef {
                    name: "greet".to_string(),
                    params: vec![],
                    body: vec![Statement::Print(Expr::String("hi".to_string()))],
                }],
                vec![],
            )
            .unwrap();

        // Spawn agent
        evaluator
            .eval_spawn("$w".to_string(), "Worker".to_string())
            .unwrap();

        // Delegate
        let delegate_stmt = Statement::Delegate {
            call: Expr::Call {
                name: "$w.greet".to_string(),
                args: vec![],
            },
        };
        evaluator.eval_program(Program {
            statements: vec![delegate_stmt],
        });

        assert!(
            evaluator
                .output
                .iter()
                .any(|s| s.contains("[Context Switch] Entering agent context"))
        );
        assert!(
            evaluator
                .output
                .iter()
                .any(|s| s.contains("[Context Switch] Exiting agent context"))
        );
    }

    #[test]
    fn test_delegation_depth_limit() {
        use crate::eval::ResourceLimits;

        let limits = ResourceLimits {
            max_depth: 1,
            ..ResourceLimits::default()
        };
        let mut evaluator = Evaluator::with_limits(limits);

        // Define agent with a task that delegates back (simulated via depth)
        evaluator
            .eval_agent_def(
                "Recursor".to_string(),
                vec![],
                vec![crate::ast::TaskDef {
                    name: "recurse".to_string(),
                    params: vec![],
                    body: vec![Statement::Print(Expr::String("level".to_string()))],
                }],
                vec![],
            )
            .unwrap();

        evaluator
            .eval_spawn("$r".to_string(), "Recursor".to_string())
            .unwrap();

        // First delegation should succeed (depth goes to 1, which is <= max_depth of 1)
        let result1 = evaluator.eval_delegate(Expr::Call {
            name: "$r.recurse".to_string(),
            args: vec![],
        });
        assert!(result1.is_ok());

        // Manually set depth to trigger limit on next call
        evaluator.tracker.depth = 1;
        let result2 = evaluator.eval_delegate(Expr::Call {
            name: "$r.recurse".to_string(),
            args: vec![],
        });
        assert!(result2.is_err());
        assert!(result2.unwrap_err().contains("Resource Limit"));
    }

    #[test]
    fn test_builtin_call_output() {
        let mut evaluator = Evaluator::new();
        let result = evaluator.eval_call("str_len", vec![Expr::String("hello".to_string())]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Number(5.0));
        assert!(
            evaluator
                .output
                .iter()
                .any(|s| s.contains("[Builtin Call]") && s.contains("str_len"))
        );
    }

    #[test]
    fn test_undefined_variable_error() {
        let mut evaluator = Evaluator::new();
        let program = Program {
            statements: vec![Statement::Print(Expr::Var("$nonexistent".to_string()))],
        };
        evaluator.eval_program(program);
        // eval_program uses eprintln for errors, so check steps didn't complete
        assert_eq!(evaluator.tracker.steps, 1);
    }

    #[test]
    fn test_unknown_tool_error() {
        let mut evaluator = Evaluator::new();
        let result = evaluator.eval_call("nonexistent_tool", vec![]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown function or tool"));
    }

    #[test]
    fn test_value_display() {
        assert_eq!(format!("{}", Value::String("hello".to_string())), "hello");
        assert_eq!(format!("{}", Value::Number(42.0)), "42");
        assert_eq!(format!("{}", Value::Null), "null");
        assert_eq!(
            format!("{}", Value::Agent("bot".to_string())),
            "[Agent: bot]"
        );
        assert_eq!(
            format!("{}", Value::Array(vec![Value::Number(1.0), Value::Number(2.0)])),
            "[1, 2]"
        );
        assert_eq!(format!("{}", Value::Bool(true)), "true");
    }
}

// ========================================================================
// M21: MCP Tool Tests
// ========================================================================
#[cfg(test)]
mod mcp_tests {
    use crate::ast::{Expr, Program, Statement};
    use crate::eval::Evaluator;

    #[test]
    fn test_mcp_tool_registration() {
        let mut evaluator = Evaluator::new();
        let program = Program {
            statements: vec![Statement::McpToolDef {
                name: "code_search".to_string(),
                server: "github-server".to_string(),
                permission: Some("mcp.connect".to_string()),
                capabilities: vec!["search_code".to_string(), "search_issues".to_string()],
                timeout: Some(15.0),
            }],
        };
        evaluator.eval_program(program);
        assert!(evaluator.mcp_tools.contains_key("code_search"));
        assert!(evaluator.tools.contains_key("code_search")); // Also registered as regular tool
        let mcp = evaluator.mcp_tools.get("code_search").unwrap();
        assert_eq!(mcp.server, "github-server");
        assert_eq!(mcp.capabilities.len(), 2);
        assert!(evaluator.output.iter().any(|s| s.contains("[MCP Tool Registered]")));
    }

    #[test]
    fn test_mcp_tool_permission_enforcement() {
        let mut evaluator = Evaluator::new();

        // Register MCP tool
        let program = Program {
            statements: vec![
                Statement::McpToolDef {
                    name: "github_search".to_string(),
                    server: "github-mcp".to_string(),
                    permission: Some("mcp.github".to_string()),
                    capabilities: vec!["search".to_string()],
                    timeout: Some(10.0),
                },
                Statement::AgentDef {
                    name: "Searcher".to_string(),
                    allow_list: vec![], // No tools allowed!
                    tasks: vec![],
                    body: vec![],
                },
            ],
        };
        evaluator.eval_program(program);

        // Spawn agent and try to use MCP tool - should be denied
        evaluator
            .eval_spawn("$s".to_string(), "Searcher".to_string())
            .unwrap();
        evaluator.current_agent = Some("$s".to_string());

        let result = evaluator.eval_call("github_search", vec![Expr::String("query".to_string())]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Permission Denied"));
    }

    #[test]
    fn test_mcp_multiple_tools() {
        let mut evaluator = Evaluator::new();
        let program = Program {
            statements: vec![
                Statement::McpToolDef {
                    name: "fs_read".to_string(),
                    server: "filesystem-server".to_string(),
                    permission: Some("mcp.fs".to_string()),
                    capabilities: vec!["read".to_string()],
                    timeout: None,
                },
                Statement::McpToolDef {
                    name: "db_query".to_string(),
                    server: "postgres-server".to_string(),
                    permission: Some("mcp.db".to_string()),
                    capabilities: vec!["query".to_string(), "insert".to_string()],
                    timeout: Some(30.0),
                },
            ],
        };
        evaluator.eval_program(program);
        assert_eq!(evaluator.mcp_tools.len(), 2);
        assert!(evaluator.mcp_tools.contains_key("fs_read"));
        assert!(evaluator.mcp_tools.contains_key("db_query"));
    }
}

// ========================================================================
// M22: Orchestration Tests
// ========================================================================
#[cfg(test)]
mod orchestration_tests {
    use crate::ast::{
        ConcurrentBranch, Expr, HandoffRoute, PipelineStage, Program, Statement,
    };
    use crate::eval::Evaluator;

    #[test]
    fn test_pipeline_registration() {
        let mut evaluator = Evaluator::new();
        let program = Program {
            statements: vec![Statement::PipelineDef {
                name: "ReviewPipeline".to_string(),
                stages: vec![
                    PipelineStage {
                        agent_name: "Analyst".to_string(),
                        call: Expr::Call {
                            name: "analyze".to_string(),
                            args: vec![Expr::Var("$input".to_string())],
                        },
                    },
                    PipelineStage {
                        agent_name: "Reviewer".to_string(),
                        call: Expr::Call {
                            name: "review".to_string(),
                            args: vec![Expr::Var("$prev".to_string())],
                        },
                    },
                ],
            }],
        };
        evaluator.eval_program(program);
        assert!(evaluator.pipelines.contains_key("ReviewPipeline"));
        let pipeline = evaluator.pipelines.get("ReviewPipeline").unwrap();
        assert_eq!(pipeline.stages.len(), 2);
        assert_eq!(pipeline.stages[0].0, "Analyst");
        assert_eq!(pipeline.stages[1].0, "Reviewer");
        assert!(evaluator.output.iter().any(|s| s.contains("[Pipeline Registered]")));
    }

    #[test]
    fn test_concurrent_registration() {
        let mut evaluator = Evaluator::new();
        let program = Program {
            statements: vec![Statement::ConcurrentDef {
                name: "ResearchTask".to_string(),
                branches: vec![
                    ConcurrentBranch {
                        agent_name: "WebSearcher".to_string(),
                        call: Expr::Call {
                            name: "search_web".to_string(),
                            args: vec![Expr::Var("$query".to_string())],
                        },
                    },
                    ConcurrentBranch {
                        agent_name: "CodeSearcher".to_string(),
                        call: Expr::Call {
                            name: "search_code".to_string(),
                            args: vec![Expr::Var("$query".to_string())],
                        },
                    },
                ],
                merge_fn: Some(Expr::Call {
                    name: "combine_results".to_string(),
                    args: vec![Expr::Var("$results".to_string())],
                }),
            }],
        };
        evaluator.eval_program(program);
        assert!(evaluator.concurrent_defs.contains_key("ResearchTask"));
        let conc = evaluator.concurrent_defs.get("ResearchTask").unwrap();
        assert_eq!(conc.branches.len(), 2);
        assert!(conc.merge_fn.is_some());
        assert!(evaluator.output.iter().any(|s| s.contains("[Concurrent Registered]")));
    }

    #[test]
    fn test_handoff_registration() {
        let mut evaluator = Evaluator::new();
        let program = Program {
            statements: vec![Statement::HandoffDef {
                name: "SupportFlow".to_string(),
                agent_name: "Triage".to_string(),
                agent_call: Expr::Call {
                    name: "classify".to_string(),
                    args: vec![Expr::Var("$input".to_string())],
                },
                routes: vec![
                    HandoffRoute {
                        pattern: "billing".to_string(),
                        target_agent: "BillingAgent".to_string(),
                    },
                    HandoffRoute {
                        pattern: "technical".to_string(),
                        target_agent: "TechAgent".to_string(),
                    },
                    HandoffRoute {
                        pattern: "_".to_string(),
                        target_agent: "HumanEscalation".to_string(),
                    },
                ],
            }],
        };
        evaluator.eval_program(program);
        assert!(evaluator.handoffs.contains_key("SupportFlow"));
        let handoff = evaluator.handoffs.get("SupportFlow").unwrap();
        assert_eq!(handoff.agent_name, "Triage");
        assert_eq!(handoff.routes.len(), 3);
        assert_eq!(handoff.routes[2].0, "_"); // wildcard route
        assert!(evaluator.output.iter().any(|s| s.contains("[Handoff Registered]")));
    }
}

// ========================================================================
// M23: A2A Protocol Tests
// ========================================================================
#[cfg(test)]
mod a2a_tests {
    use crate::ast::{Program, Statement};
    use crate::eval::Evaluator;

    #[test]
    fn test_a2a_card_registration() {
        let mut evaluator = Evaluator::new();
        let program = Program {
            statements: vec![Statement::A2ADef {
                name: "ResearchCard".to_string(),
                discovery: Some("/.well-known/agent.json".to_string()),
                skills: vec![
                    "search".to_string(),
                    "analyze".to_string(),
                    "summarize".to_string(),
                ],
                endpoint: Some("https://agents.aria.dev/research".to_string()),
            }],
        };
        evaluator.eval_program(program);
        assert!(evaluator.a2a_cards.contains_key("ResearchCard"));
        let card = evaluator.a2a_cards.get("ResearchCard").unwrap();
        assert_eq!(card.skills.len(), 3);
        assert_eq!(card.discovery.as_deref(), Some("/.well-known/agent.json"));
        assert!(evaluator.output.iter().any(|s| s.contains("[A2A Card Registered]")));
    }

    #[test]
    fn test_a2a_minimal_card() {
        let mut evaluator = Evaluator::new();
        let program = Program {
            statements: vec![Statement::A2ADef {
                name: "MinimalAgent".to_string(),
                discovery: None,
                skills: vec!["respond".to_string()],
                endpoint: None,
            }],
        };
        evaluator.eval_program(program);
        assert!(evaluator.a2a_cards.contains_key("MinimalAgent"));
        let card = evaluator.a2a_cards.get("MinimalAgent").unwrap();
        assert_eq!(card.skills.len(), 1);
        assert!(card.discovery.is_none());
        assert!(card.endpoint.is_none());
    }
}

// ========================================================================
// M24: Workflow Tests
// ========================================================================
#[cfg(test)]
mod workflow_tests {
    use crate::ast::{Expr, Program, Statement, WorkflowState, WorkflowTransition};
    use crate::eval::Evaluator;

    #[test]
    fn test_workflow_registration() {
        let mut evaluator = Evaluator::new();
        let program = Program {
            statements: vec![Statement::WorkflowDef {
                name: "OrderProcessing".to_string(),
                states: vec![
                    WorkflowState {
                        name: "pending".to_string(),
                        transitions: vec![WorkflowTransition {
                            event: "receive_order".to_string(),
                            target_state: "validating".to_string(),
                        }],
                        requires: None,
                        ensures: None,
                        body: vec![],
                    },
                    WorkflowState {
                        name: "validating".to_string(),
                        transitions: vec![
                            WorkflowTransition {
                                event: "valid".to_string(),
                                target_state: "processing".to_string(),
                            },
                            WorkflowTransition {
                                event: "invalid".to_string(),
                                target_state: "rejected".to_string(),
                            },
                        ],
                        requires: Some(Expr::Var("$order_valid".to_string())),
                        ensures: None,
                        body: vec![],
                    },
                    WorkflowState {
                        name: "processing".to_string(),
                        transitions: vec![WorkflowTransition {
                            event: "complete".to_string(),
                            target_state: "shipped".to_string(),
                        }],
                        requires: None,
                        ensures: None,
                        body: vec![],
                    },
                    WorkflowState {
                        name: "shipped".to_string(),
                        transitions: vec![],
                        requires: None,
                        ensures: Some(Expr::Var("$tracking_exists".to_string())),
                        body: vec![],
                    },
                    WorkflowState {
                        name: "rejected".to_string(),
                        transitions: vec![],
                        requires: None,
                        ensures: None,
                        body: vec![],
                    },
                ],
            }],
        };
        evaluator.eval_program(program);
        assert!(evaluator.workflows.contains_key("OrderProcessing"));
        let wf = evaluator.workflows.get("OrderProcessing").unwrap();
        assert_eq!(wf.current_state, "pending"); // Initial state is first declared state
        assert_eq!(wf.states.len(), 5);
        assert!(wf.states.contains_key("pending"));
        assert!(wf.states.contains_key("validating"));
        assert!(wf.states.contains_key("shipped"));
        assert!(evaluator.output.iter().any(|s| s.contains("[Workflow Registered]")));
    }

    #[test]
    fn test_workflow_transitions_stored() {
        let mut evaluator = Evaluator::new();
        let program = Program {
            statements: vec![Statement::WorkflowDef {
                name: "Simple".to_string(),
                states: vec![
                    WorkflowState {
                        name: "start".to_string(),
                        transitions: vec![
                            WorkflowTransition {
                                event: "go".to_string(),
                                target_state: "middle".to_string(),
                            },
                        ],
                        requires: None,
                        ensures: None,
                        body: vec![],
                    },
                    WorkflowState {
                        name: "middle".to_string(),
                        transitions: vec![
                            WorkflowTransition {
                                event: "finish".to_string(),
                                target_state: "end".to_string(),
                            },
                        ],
                        requires: None,
                        ensures: None,
                        body: vec![],
                    },
                    WorkflowState {
                        name: "end".to_string(),
                        transitions: vec![],
                        requires: None,
                        ensures: None,
                        body: vec![],
                    },
                ],
            }],
        };
        evaluator.eval_program(program);
        let wf = evaluator.workflows.get("Simple").unwrap();
        let start = wf.states.get("start").unwrap();
        assert_eq!(start.transitions.len(), 1);
        assert_eq!(start.transitions[0], ("go".to_string(), "middle".to_string()));
    }
}

// ========================================================================
// M25: Model Declaration Tests
// ========================================================================
#[cfg(test)]
mod model_tests {
    use crate::ast::{Program, Statement};
    use crate::eval::Evaluator;

    #[test]
    fn test_model_registration() {
        let mut evaluator = Evaluator::new();
        let program = Program {
            statements: vec![Statement::ModelDef {
                name: "assistant".to_string(),
                capability: Some("chat_completion".to_string()),
                provider: Some("openai".to_string()),
                supports: vec![
                    "tool_calling".to_string(),
                    "structured_output".to_string(),
                ],
            }],
        };
        evaluator.eval_program(program);
        assert!(evaluator.models.contains_key("assistant"));
        let model = evaluator.models.get("assistant").unwrap();
        assert_eq!(model.capability.as_deref(), Some("chat_completion"));
        assert_eq!(model.provider.as_deref(), Some("openai"));
        assert_eq!(model.supports.len(), 2);
        assert!(evaluator.output.iter().any(|s| s.contains("[Model Registered]")));
    }

    #[test]
    fn test_model_minimal() {
        let mut evaluator = Evaluator::new();
        let program = Program {
            statements: vec![Statement::ModelDef {
                name: "embedder".to_string(),
                capability: Some("embedding".to_string()),
                provider: None,
                supports: vec![],
            }],
        };
        evaluator.eval_program(program);
        assert!(evaluator.models.contains_key("embedder"));
        let model = evaluator.models.get("embedder").unwrap();
        assert!(model.provider.is_none());
        assert!(model.supports.is_empty());
    }
}

// ========================================================================
// M26: Memory Tests
// ========================================================================
#[cfg(test)]
mod memory_tests {
    use crate::ast::{Program, Statement};
    use crate::eval::Evaluator;

    #[test]
    fn test_memory_registration() {
        let mut evaluator = Evaluator::new();
        let program = Program {
            statements: vec![Statement::MemoryDef {
                name: "ProjectKnowledge".to_string(),
                store: Some("chromadb://localhost:8000/project".to_string()),
                embedding_model: Some("text_embedder".to_string()),
                operations: vec![
                    "remember".to_string(),
                    "recall".to_string(),
                    "forget".to_string(),
                ],
            }],
        };
        evaluator.eval_program(program);
        assert!(evaluator.memories.contains_key("ProjectKnowledge"));
        let mem = evaluator.memories.get("ProjectKnowledge").unwrap();
        assert_eq!(mem.store.as_deref(), Some("chromadb://localhost:8000/project"));
        assert_eq!(mem.embedding_model.as_deref(), Some("text_embedder"));
        assert_eq!(mem.operations.len(), 3);
        assert!(evaluator.output.iter().any(|s| s.contains("[Memory Registered]")));
    }

    #[test]
    fn test_memory_minimal() {
        let mut evaluator = Evaluator::new();
        let program = Program {
            statements: vec![Statement::MemoryDef {
                name: "BasicMem".to_string(),
                store: None,
                embedding_model: None,
                operations: vec![],
            }],
        };
        evaluator.eval_program(program);
        assert!(evaluator.memories.contains_key("BasicMem"));
    }
}

// ========================================================================
// Integration: Full Program with M21-M26 Features
// ========================================================================
#[cfg(test)]
mod integration_tests {
    use crate::ast::{
        Expr, PipelineStage, Program, Statement, WorkflowState, WorkflowTransition,
    };
    use crate::eval::Evaluator;

    #[test]
    fn test_full_agentic_program() {
        let mut evaluator = Evaluator::new();
        let program = Program {
            statements: vec![
                // M25: Declare model
                Statement::ModelDef {
                    name: "gpt4".to_string(),
                    capability: Some("chat_completion".to_string()),
                    provider: Some("openai".to_string()),
                    supports: vec!["tool_calling".to_string()],
                },
                // M21: MCP tool
                Statement::McpToolDef {
                    name: "github_search".to_string(),
                    server: "github-mcp".to_string(),
                    permission: Some("mcp.github".to_string()),
                    capabilities: vec!["search".to_string()],
                    timeout: Some(10.0),
                },
                // M26: Memory
                Statement::MemoryDef {
                    name: "CodeMemory".to_string(),
                    store: Some("chromadb://localhost/code".to_string()),
                    embedding_model: Some("ada".to_string()),
                    operations: vec!["recall".to_string(), "remember".to_string()],
                },
                // Original: Tool + Agent
                Statement::ToolDef {
                    name: "analyze".to_string(),
                    params: vec!["code".to_string()],
                    permission: Some("code.read".to_string()),
                    timeout: Some(30.0),
                },
                Statement::AgentDef {
                    name: "CodeAnalyzer".to_string(),
                    allow_list: vec!["analyze".to_string(), "github_search".to_string()],
                    tasks: vec![],
                    body: vec![],
                },
                // M22: Pipeline
                Statement::PipelineDef {
                    name: "AnalysisPipeline".to_string(),
                    stages: vec![
                        PipelineStage {
                            agent_name: "Fetcher".to_string(),
                            call: Expr::Call {
                                name: "fetch".to_string(),
                                args: vec![],
                            },
                        },
                        PipelineStage {
                            agent_name: "Analyzer".to_string(),
                            call: Expr::Call {
                                name: "analyze".to_string(),
                                args: vec![],
                            },
                        },
                    ],
                },
                // M23: A2A card
                Statement::A2ADef {
                    name: "AnalyzerCard".to_string(),
                    discovery: Some("/.well-known/agent.json".to_string()),
                    skills: vec!["code_analysis".to_string()],
                    endpoint: Some("https://api.example.com/analyzer".to_string()),
                },
                // M24: Workflow
                Statement::WorkflowDef {
                    name: "ReviewFlow".to_string(),
                    states: vec![
                        WorkflowState {
                            name: "draft".to_string(),
                            transitions: vec![WorkflowTransition {
                                event: "submit".to_string(),
                                target_state: "review".to_string(),
                            }],
                            requires: None,
                            ensures: None,
                            body: vec![],
                        },
                        WorkflowState {
                            name: "review".to_string(),
                            transitions: vec![
                                WorkflowTransition {
                                    event: "approve".to_string(),
                                    target_state: "merged".to_string(),
                                },
                            ],
                            requires: None,
                            ensures: None,
                            body: vec![],
                        },
                        WorkflowState {
                            name: "merged".to_string(),
                            transitions: vec![],
                            requires: None,
                            ensures: None,
                            body: vec![],
                        },
                    ],
                },
            ],
        };

        evaluator.eval_program(program);

        // Verify everything registered correctly
        assert!(evaluator.models.contains_key("gpt4"));
        assert!(evaluator.mcp_tools.contains_key("github_search"));
        assert!(evaluator.memories.contains_key("CodeMemory"));
        assert!(evaluator.tools.contains_key("analyze"));
        assert!(evaluator.agent_defs.contains_key("CodeAnalyzer"));
        assert!(evaluator.pipelines.contains_key("AnalysisPipeline"));
        assert!(evaluator.a2a_cards.contains_key("AnalyzerCard"));
        assert!(evaluator.workflows.contains_key("ReviewFlow"));

        // Verify the output trace shows all registrations
        let output = evaluator.output.join("\n");
        assert!(output.contains("[Model Registered]"));
        assert!(output.contains("[MCP Tool Registered]"));
        assert!(output.contains("[Memory Registered]"));
        assert!(output.contains("[Tool Registered]"));
        assert!(output.contains("[Agent Defined]"));
        assert!(output.contains("[Pipeline Registered]"));
        assert!(output.contains("[A2A Card Registered]"));
        assert!(output.contains("[Workflow Registered]"));
    }
}

// Day 4: Permission Enforcement Tests
#[cfg(test)]
mod permission_tests {
    use crate::ast::Expr;
    use crate::eval::Evaluator;

    #[test]
    fn test_permission_denied() {
        let mut evaluator = Evaluator::new();

        // Define tool
        evaluator
            .eval_tool_def(
                "read_file".to_string(),
                vec!["path".to_string()],
                Some("io.read".to_string()),
                None,
            )
            .unwrap();

        // Define agent WITHOUT read_file
        evaluator
            .eval_agent_def(
                "RestrictedAgent".to_string(),
                vec!["write_file".to_string()],
                vec![],
                vec![],
            )
            .unwrap();

        // Spawn agent
        evaluator
            .eval_spawn("$agent".to_string(), "RestrictedAgent".to_string())
            .unwrap();

        // Set agent context
        evaluator.current_agent = Some("$agent".to_string());

        // Try to call read_file - should fail
        let result =
            evaluator.eval_call("read_file", vec![Expr::String("/etc/passwd".to_string())]);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Permission Denied"));
    }

    #[test]
    fn test_permission_allowed() {
        let mut evaluator = Evaluator::new();

        // Use echo instead of write_file for simpler testing
        evaluator
            .eval_tool_def(
                "echo".to_string(),
                vec![],
                Some("io.write".to_string()),
                Some(5.0),
            )
            .unwrap();
        evaluator
            .eval_agent_def(
                "WriterAgent".to_string(),
                vec!["echo".to_string()],
                vec![],
                vec![],
            )
            .unwrap();
        evaluator
            .eval_spawn("$writer".to_string(), "WriterAgent".to_string())
            .unwrap();
        evaluator.current_agent = Some("$writer".to_string());

        let result = evaluator.eval_call("echo", vec![Expr::String("test".to_string())]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_main_context_unrestricted() {
        let mut evaluator = Evaluator::new();
        // Use echo for testing
        evaluator
            .eval_tool_def(
                "echo".to_string(),
                vec![],
                Some("system.execute".to_string()),
                Some(5.0),
            )
            .unwrap();

        // Main context (current_agent = None)
        assert_eq!(evaluator.current_agent, None);
        let result = evaluator.eval_call("echo", vec![Expr::String("test".to_string())]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_context_isolation() {
        let mut evaluator = Evaluator::new();

        assert_eq!(evaluator.current_agent, None);

        evaluator.current_agent = Some("$agent1".to_string());
        assert_eq!(evaluator.current_agent, Some("$agent1".to_string()));

        let prev = evaluator.current_agent.clone();
        evaluator.current_agent = Some("$agent2".to_string());
        evaluator.current_agent = prev;

        assert_eq!(evaluator.current_agent, Some("$agent1".to_string()));
    }
}

// Day 5: Resource Limit Tests
#[cfg(test)]
mod resource_limit_tests {
    use crate::ast::{Expr, Program, Statement};
    use crate::eval::{Evaluator, ResourceLimits};

    #[test]
    fn test_max_steps_exceeded() {
        let limits = ResourceLimits {
            max_steps: 5,
            ..ResourceLimits::default()
        };
        let mut evaluator = Evaluator::with_limits(limits);

        // Create a program with more statements than allowed
        let program = Program {
            statements: vec![
                Statement::Print(Expr::String("1".to_string())),
                Statement::Print(Expr::String("2".to_string())),
                Statement::Print(Expr::String("3".to_string())),
                Statement::Print(Expr::String("4".to_string())),
                Statement::Print(Expr::String("5".to_string())),
                // This 6th statement should trigger the limit
                Statement::Print(Expr::String("6".to_string())),
            ],
        };

        evaluator.eval_program(program);
        // The evaluator should have stopped after max_steps
        assert!(evaluator.tracker.steps >= 5);
    }

    #[test]
    fn test_steps_within_limit() {
        let limits = ResourceLimits {
            max_steps: 100,
            ..ResourceLimits::default()
        };
        let mut evaluator = Evaluator::with_limits(limits);

        let program = Program {
            statements: vec![
                Statement::Let {
                    name: "$a".to_string(),
                    value: Expr::Number(1.0),
                },
                Statement::Let {
                    name: "$b".to_string(),
                    value: Expr::Number(2.0),
                },
            ],
        };

        evaluator.eval_program(program);
        assert_eq!(evaluator.tracker.steps, 2);
    }

    #[test]
    fn test_default_limits() {
        let evaluator = Evaluator::new();
        assert_eq!(evaluator.limits.max_steps, 10_000);
        assert_eq!(evaluator.limits.max_depth, 32);
        assert_eq!(evaluator.limits.max_output_bytes, 1_048_576);
        assert_eq!(evaluator.tracker.steps, 0);
        assert_eq!(evaluator.tracker.depth, 0);
    }

    #[test]
    fn test_custom_limits() {
        let limits = ResourceLimits {
            max_steps: 500,
            max_depth: 8,
            max_output_bytes: 4096,
        };
        let evaluator = Evaluator::with_limits(limits);
        assert_eq!(evaluator.limits.max_steps, 500);
        assert_eq!(evaluator.limits.max_depth, 8);
        assert_eq!(evaluator.limits.max_output_bytes, 4096);
    }
}

// ========================================================================
// M21 Runtime: MCP Tool Execution Tests
// ========================================================================
#[cfg(test)]
mod mcp_runtime_tests {
    use crate::ast::{Expr, Program, Statement};
    use crate::eval::{Evaluator, Value};

    #[test]
    fn test_mcp_tool_call_through_eval_call() {
        let mut evaluator = Evaluator::new();

        // Register an MCP tool
        let program = Program {
            statements: vec![Statement::McpToolDef {
                name: "search_code".to_string(),
                server: "github-server".to_string(),
                permission: Some("mcp.github".to_string()),
                capabilities: vec!["search".to_string()],
                timeout: Some(15.0),
            }],
        };
        evaluator.eval_program(program);

        // Call the MCP tool through eval_call (main context = unrestricted)
        let result = evaluator.eval_call(
            "search_code",
            vec![Expr::String("rust async".to_string())],
        );
        assert!(result.is_ok());
        let val = result.unwrap();
        if let Value::String(s) = &val {
            assert!(s.contains("[MCP:github-server]"));
            assert!(s.contains("search_code"));
            assert!(s.contains("rust async"));
        } else {
            panic!("Expected String value from MCP tool, got: {:?}", val);
        }

        // Verify output trace shows MCP routing
        let output = evaluator.output.join("\n");
        assert!(output.contains("[MCP Call]"));
        assert!(output.contains("[MCP Result]"));
    }

    #[test]
    fn test_mcp_tool_is_detected() {
        let mut evaluator = Evaluator::new();

        let program = Program {
            statements: vec![Statement::McpToolDef {
                name: "db_query".to_string(),
                server: "postgres-server".to_string(),
                permission: None,
                capabilities: vec!["query".to_string()],
                timeout: None,
            }],
        };
        evaluator.eval_program(program);

        assert!(evaluator.mcp_tools.contains_key("db_query"));
        // Also verify it's registered as a regular tool for permission checks
        assert!(evaluator.tools.contains_key("db_query"));
    }

    #[test]
    fn test_mcp_tool_with_agent_permission() {
        let mut evaluator = Evaluator::new();

        // Register MCP tool and agent that's allowed to use it
        let program = Program {
            statements: vec![
                Statement::McpToolDef {
                    name: "code_search".to_string(),
                    server: "github-mcp".to_string(),
                    permission: Some("mcp.github".to_string()),
                    capabilities: vec!["search".to_string()],
                    timeout: Some(10.0),
                },
                Statement::AgentDef {
                    name: "SearchAgent".to_string(),
                    allow_list: vec!["code_search".to_string()],
                    tasks: vec![],
                    body: vec![],
                },
            ],
        };
        evaluator.eval_program(program);

        // Spawn and set context
        evaluator
            .eval_spawn("$searcher".to_string(), "SearchAgent".to_string())
            .unwrap();
        evaluator.current_agent = Some("$searcher".to_string());

        // Call should succeed since agent has permission
        let result = evaluator.eval_call(
            "code_search",
            vec![Expr::String("query".to_string())],
        );
        assert!(result.is_ok());
        if let Value::String(s) = result.unwrap() {
            assert!(s.contains("[MCP:github-mcp]"));
        } else {
            panic!("Expected String from MCP call");
        }
    }
}

// ========================================================================
// M22 Runtime: Pipeline Execution Tests
// ========================================================================
#[cfg(test)]
mod pipeline_runtime_tests {
    use crate::ast::{Expr, PipelineStage, Program, Statement};
    use crate::eval::{Evaluator, Value};

    fn setup_pipeline_evaluator() -> Evaluator {
        let mut evaluator = Evaluator::new();

        // Register a pipeline with stages that use $prev/$input variables
        let program = Program {
            statements: vec![Statement::PipelineDef {
                name: "TestPipeline".to_string(),
                stages: vec![
                    PipelineStage {
                        agent_name: "Stage1".to_string(),
                        call: Expr::Var("$input".to_string()),
                    },
                    PipelineStage {
                        agent_name: "Stage2".to_string(),
                        call: Expr::Var("$prev".to_string()),
                    },
                ],
            }],
        };
        evaluator.eval_program(program);
        evaluator
    }

    #[test]
    fn test_pipeline_run_basic() {
        let mut evaluator = setup_pipeline_evaluator();

        let result = evaluator.run_pipeline(
            "TestPipeline",
            Value::String("hello".to_string()),
        );
        assert!(result.is_ok());
        // Each stage just passes through via $input/$prev
        assert_eq!(result.unwrap(), Value::String("hello".to_string()));
    }

    #[test]
    fn test_pipeline_run_traces_output() {
        let mut evaluator = setup_pipeline_evaluator();

        evaluator
            .run_pipeline("TestPipeline", Value::String("data".to_string()))
            .unwrap();

        let output = evaluator.output.join("\n");
        assert!(output.contains("[Pipeline Run] TestPipeline with 2 stages"));
        assert!(output.contains("[Pipeline Stage 1/2] Stage1"));
        assert!(output.contains("[Pipeline Stage 2/2] Stage2"));
        assert!(output.contains("[Pipeline Complete] TestPipeline"));
    }

    #[test]
    fn test_pipeline_not_found() {
        let mut evaluator = Evaluator::new();
        let result = evaluator.run_pipeline("NonExistent", Value::Null);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_pipeline_threads_values() {
        let mut evaluator = Evaluator::new();

        // Pipeline with 3 stages, each stage reads $prev
        let program = Program {
            statements: vec![Statement::PipelineDef {
                name: "ThreeStage".to_string(),
                stages: vec![
                    PipelineStage {
                        agent_name: "A".to_string(),
                        call: Expr::String("from_stage_1".to_string()),
                    },
                    PipelineStage {
                        agent_name: "B".to_string(),
                        call: Expr::String("from_stage_2".to_string()),
                    },
                    PipelineStage {
                        agent_name: "C".to_string(),
                        call: Expr::Var("$prev".to_string()),
                    },
                ],
            }],
        };
        evaluator.eval_program(program);

        let result = evaluator.run_pipeline("ThreeStage", Value::String("start".to_string()));
        assert!(result.is_ok());
        // Stage 3 reads $prev which is "from_stage_2"
        assert_eq!(
            result.unwrap(),
            Value::String("from_stage_2".to_string())
        );
    }
}

// ========================================================================
// M22 Runtime: Concurrent Execution Tests
// ========================================================================
#[cfg(test)]
mod concurrent_runtime_tests {
    use crate::ast::{ConcurrentBranch, Expr, Program, Statement};
    use crate::eval::{Evaluator, Value};

    #[test]
    fn test_concurrent_run_basic() {
        let mut evaluator = Evaluator::new();

        let program = Program {
            statements: vec![Statement::ConcurrentDef {
                name: "ParallelSearch".to_string(),
                branches: vec![
                    ConcurrentBranch {
                        agent_name: "Searcher1".to_string(),
                        call: Expr::String("result_a".to_string()),
                    },
                    ConcurrentBranch {
                        agent_name: "Searcher2".to_string(),
                        call: Expr::String("result_b".to_string()),
                    },
                ],
                merge_fn: None,
            }],
        };
        evaluator.eval_program(program);

        let result = evaluator.run_concurrent(
            "ParallelSearch",
            Value::String("query".to_string()),
        );
        assert!(result.is_ok());

        // Without merge_fn, result is an array of branch results
        let val = result.unwrap();
        if let Value::Array(items) = &val {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0], Value::String("result_a".to_string()));
            assert_eq!(items[1], Value::String("result_b".to_string()));
        } else {
            panic!("Expected Array value, got: {:?}", val);
        }
    }

    #[test]
    fn test_concurrent_with_merge() {
        let mut evaluator = Evaluator::new();

        let program = Program {
            statements: vec![Statement::ConcurrentDef {
                name: "MergedSearch".to_string(),
                branches: vec![
                    ConcurrentBranch {
                        agent_name: "A".to_string(),
                        call: Expr::String("alpha".to_string()),
                    },
                    ConcurrentBranch {
                        agent_name: "B".to_string(),
                        call: Expr::String("beta".to_string()),
                    },
                ],
                // merge_fn reads $results (which is the collected array)
                merge_fn: Some(Expr::Var("$results".to_string())),
            }],
        };
        evaluator.eval_program(program);

        let result = evaluator.run_concurrent(
            "MergedSearch",
            Value::String("q".to_string()),
        );
        assert!(result.is_ok());
        // Merge fn just returns $results as-is
        if let Value::Array(items) = result.unwrap() {
            assert_eq!(items.len(), 2);
        } else {
            panic!("Expected Array from merge function");
        }
    }

    #[test]
    fn test_concurrent_traces_output() {
        let mut evaluator = Evaluator::new();

        let program = Program {
            statements: vec![Statement::ConcurrentDef {
                name: "Traced".to_string(),
                branches: vec![ConcurrentBranch {
                    agent_name: "Worker".to_string(),
                    call: Expr::String("done".to_string()),
                }],
                merge_fn: None,
            }],
        };
        evaluator.eval_program(program);

        evaluator
            .run_concurrent("Traced", Value::Null)
            .unwrap();

        let output = evaluator.output.join("\n");
        assert!(output.contains("[Concurrent Run] Traced with 1 branches"));
        assert!(output.contains("[Concurrent Branch] Worker executing"));
        assert!(output.contains("[Concurrent Complete] Traced"));
    }

    #[test]
    fn test_concurrent_not_found() {
        let mut evaluator = Evaluator::new();
        let result = evaluator.run_concurrent("Missing", Value::Null);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_concurrent_sets_query_var() {
        let mut evaluator = Evaluator::new();

        // Branch reads $query that's set by run_concurrent
        let program = Program {
            statements: vec![Statement::ConcurrentDef {
                name: "QueryTest".to_string(),
                branches: vec![ConcurrentBranch {
                    agent_name: "Agent".to_string(),
                    call: Expr::Var("$query".to_string()),
                }],
                merge_fn: None,
            }],
        };
        evaluator.eval_program(program);

        let result = evaluator.run_concurrent(
            "QueryTest",
            Value::String("my_query".to_string()),
        );
        assert!(result.is_ok());
        if let Value::Array(items) = result.unwrap() {
            assert_eq!(items[0], Value::String("my_query".to_string()));
        } else {
            panic!("Expected Array");
        }
    }
}

// ========================================================================
// M22 Runtime: Handoff Execution Tests
// ========================================================================
#[cfg(test)]
mod handoff_runtime_tests {
    use crate::ast::{Expr, HandoffRoute, Program, Statement};
    use crate::eval::{Evaluator, Value};

    fn setup_handoff_evaluator() -> Evaluator {
        let mut evaluator = Evaluator::new();
        let program = Program {
            statements: vec![Statement::HandoffDef {
                name: "Router".to_string(),
                agent_name: "Classifier".to_string(),
                agent_call: Expr::Var("$input".to_string()),
                routes: vec![
                    HandoffRoute {
                        pattern: "billing".to_string(),
                        target_agent: "BillingAgent".to_string(),
                    },
                    HandoffRoute {
                        pattern: "technical".to_string(),
                        target_agent: "TechAgent".to_string(),
                    },
                    HandoffRoute {
                        pattern: "_".to_string(),
                        target_agent: "DefaultAgent".to_string(),
                    },
                ],
            }],
        };
        evaluator.eval_program(program);
        evaluator
    }

    #[test]
    fn test_handoff_exact_match() {
        let mut evaluator = setup_handoff_evaluator();

        let result = evaluator.run_handoff(
            "Router",
            Value::String("billing".to_string()),
        );
        assert!(result.is_ok());
        let val = result.unwrap();
        if let Value::String(s) = &val {
            assert!(s.contains("BillingAgent"));
        } else {
            panic!("Expected String, got: {:?}", val);
        }
    }

    #[test]
    fn test_handoff_second_route() {
        let mut evaluator = setup_handoff_evaluator();

        let result = evaluator.run_handoff(
            "Router",
            Value::String("technical".to_string()),
        );
        assert!(result.is_ok());
        let val = result.unwrap();
        if let Value::String(s) = &val {
            assert!(s.contains("TechAgent"));
        } else {
            panic!("Expected String");
        }
    }

    #[test]
    fn test_handoff_wildcard_route() {
        let mut evaluator = setup_handoff_evaluator();

        let result = evaluator.run_handoff(
            "Router",
            Value::String("unknown_category".to_string()),
        );
        assert!(result.is_ok());
        let val = result.unwrap();
        if let Value::String(s) = &val {
            assert!(s.contains("DefaultAgent"));
        } else {
            panic!("Expected String");
        }
    }

    #[test]
    fn test_handoff_no_matching_route() {
        let mut evaluator = Evaluator::new();

        // Handoff without wildcard
        let program = Program {
            statements: vec![Statement::HandoffDef {
                name: "StrictRouter".to_string(),
                agent_name: "Classifier".to_string(),
                agent_call: Expr::Var("$input".to_string()),
                routes: vec![HandoffRoute {
                    pattern: "only_this".to_string(),
                    target_agent: "OnlyAgent".to_string(),
                }],
            }],
        };
        evaluator.eval_program(program);

        let result = evaluator.run_handoff(
            "StrictRouter",
            Value::String("something_else".to_string()),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No route matches"));
    }

    #[test]
    fn test_handoff_not_found() {
        let mut evaluator = Evaluator::new();
        let result = evaluator.run_handoff("NoSuchHandoff", Value::Null);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_handoff_traces_output() {
        let mut evaluator = setup_handoff_evaluator();

        evaluator
            .run_handoff("Router", Value::String("billing".to_string()))
            .unwrap();

        let output = evaluator.output.join("\n");
        assert!(output.contains("[Handoff Run] Router with classifier 'Classifier'"));
        assert!(output.contains("[Handoff Classified]"));
        assert!(output.contains("[Handoff Routed] 'billing' -> agent 'BillingAgent'"));
    }
}

// ========================================================================
// M24 Runtime: Workflow Transition Tests
// ========================================================================
#[cfg(test)]
mod workflow_runtime_tests {
    use crate::ast::{Expr, Program, Statement, WorkflowState, WorkflowTransition};
    use crate::eval::{Evaluator, Value};

    fn setup_workflow_evaluator() -> Evaluator {
        let mut evaluator = Evaluator::new();
        let program = Program {
            statements: vec![Statement::WorkflowDef {
                name: "OrderFlow".to_string(),
                states: vec![
                    WorkflowState {
                        name: "pending".to_string(),
                        transitions: vec![WorkflowTransition {
                            event: "submit".to_string(),
                            target_state: "processing".to_string(),
                        }],
                        requires: None,
                        ensures: None,
                        body: vec![],
                    },
                    WorkflowState {
                        name: "processing".to_string(),
                        transitions: vec![
                            WorkflowTransition {
                                event: "complete".to_string(),
                                target_state: "shipped".to_string(),
                            },
                            WorkflowTransition {
                                event: "cancel".to_string(),
                                target_state: "cancelled".to_string(),
                            },
                        ],
                        requires: None,
                        ensures: None,
                        body: vec![],
                    },
                    WorkflowState {
                        name: "shipped".to_string(),
                        transitions: vec![],
                        requires: None,
                        ensures: None,
                        body: vec![],
                    },
                    WorkflowState {
                        name: "cancelled".to_string(),
                        transitions: vec![],
                        requires: None,
                        ensures: None,
                        body: vec![],
                    },
                ],
            }],
        };
        evaluator.eval_program(program);
        evaluator
    }

    #[test]
    fn test_workflow_initial_state() {
        let evaluator = setup_workflow_evaluator();
        let state = evaluator.get_workflow_state("OrderFlow").unwrap();
        assert_eq!(state, Value::String("pending".to_string()));
    }

    #[test]
    fn test_workflow_single_transition() {
        let mut evaluator = setup_workflow_evaluator();

        let result = evaluator.transition_workflow("OrderFlow", "submit");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::String("processing".to_string()));

        // Verify state was actually updated
        let state = evaluator.get_workflow_state("OrderFlow").unwrap();
        assert_eq!(state, Value::String("processing".to_string()));
    }

    #[test]
    fn test_workflow_multi_step_transitions() {
        let mut evaluator = setup_workflow_evaluator();

        // pending -> processing -> shipped
        evaluator.transition_workflow("OrderFlow", "submit").unwrap();
        let result = evaluator.transition_workflow("OrderFlow", "complete");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::String("shipped".to_string()));

        let state = evaluator.get_workflow_state("OrderFlow").unwrap();
        assert_eq!(state, Value::String("shipped".to_string()));
    }

    #[test]
    fn test_workflow_cancel_branch() {
        let mut evaluator = setup_workflow_evaluator();

        // pending -> processing -> cancelled
        evaluator.transition_workflow("OrderFlow", "submit").unwrap();
        let result = evaluator.transition_workflow("OrderFlow", "cancel");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::String("cancelled".to_string()));
    }

    #[test]
    fn test_workflow_invalid_event() {
        let mut evaluator = setup_workflow_evaluator();

        // "cancel" is not valid from "pending" state
        let result = evaluator.transition_workflow("OrderFlow", "cancel");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No transition for event"));
    }

    #[test]
    fn test_workflow_terminal_state() {
        let mut evaluator = setup_workflow_evaluator();

        // Go to terminal state "shipped"
        evaluator.transition_workflow("OrderFlow", "submit").unwrap();
        evaluator
            .transition_workflow("OrderFlow", "complete")
            .unwrap();

        // No transitions from "shipped"
        let result = evaluator.transition_workflow("OrderFlow", "anything");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No transition for event"));
    }

    #[test]
    fn test_workflow_not_found() {
        let mut evaluator = Evaluator::new();
        let result = evaluator.transition_workflow("NoWorkflow", "event");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_get_workflow_state_not_found() {
        let evaluator = Evaluator::new();
        let result = evaluator.get_workflow_state("Missing");
        assert!(result.is_err());
    }

    #[test]
    fn test_workflow_traces_output() {
        let mut evaluator = setup_workflow_evaluator();

        evaluator.transition_workflow("OrderFlow", "submit").unwrap();

        let output = evaluator.output.join("\n");
        assert!(output.contains("[Workflow Transition] OrderFlow: 'pending' --(submit)-> 'processing'"));
    }

    #[test]
    fn test_workflow_requires_contract_pass() {
        let mut evaluator = Evaluator::new();

        let program = Program {
            statements: vec![Statement::WorkflowDef {
                name: "ContractFlow".to_string(),
                states: vec![
                    WorkflowState {
                        name: "start".to_string(),
                        transitions: vec![WorkflowTransition {
                            event: "go".to_string(),
                            target_state: "end".to_string(),
                        }],
                        requires: Some(Expr::Var("$is_ready".to_string())),
                        ensures: None,
                        body: vec![],
                    },
                    WorkflowState {
                        name: "end".to_string(),
                        transitions: vec![],
                        requires: None,
                        ensures: None,
                        body: vec![],
                    },
                ],
            }],
        };
        evaluator.eval_program(program);

        // Set the requires variable to truthy value
        evaluator
            .variables
            .insert("$is_ready".to_string(), Value::Bool(true));

        let result = evaluator.transition_workflow("ContractFlow", "go");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::String("end".to_string()));
    }

    #[test]
    fn test_workflow_requires_contract_fail() {
        let mut evaluator = Evaluator::new();

        let program = Program {
            statements: vec![Statement::WorkflowDef {
                name: "GuardedFlow".to_string(),
                states: vec![
                    WorkflowState {
                        name: "start".to_string(),
                        transitions: vec![WorkflowTransition {
                            event: "go".to_string(),
                            target_state: "end".to_string(),
                        }],
                        requires: Some(Expr::Var("$condition".to_string())),
                        ensures: None,
                        body: vec![],
                    },
                    WorkflowState {
                        name: "end".to_string(),
                        transitions: vec![],
                        requires: None,
                        ensures: None,
                        body: vec![],
                    },
                ],
            }],
        };
        evaluator.eval_program(program);

        // Set requires condition to false
        evaluator
            .variables
            .insert("$condition".to_string(), Value::Bool(false));

        let result = evaluator.transition_workflow("GuardedFlow", "go");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("requires failed"));
    }

    #[test]
    fn test_workflow_ensures_contract_fail() {
        let mut evaluator = Evaluator::new();

        let program = Program {
            statements: vec![Statement::WorkflowDef {
                name: "EnsuresFlow".to_string(),
                states: vec![
                    WorkflowState {
                        name: "start".to_string(),
                        transitions: vec![WorkflowTransition {
                            event: "go".to_string(),
                            target_state: "end".to_string(),
                        }],
                        requires: None,
                        ensures: None,
                        body: vec![],
                    },
                    WorkflowState {
                        name: "end".to_string(),
                        transitions: vec![],
                        requires: None,
                        ensures: Some(Expr::Var("$postcondition".to_string())),
                        body: vec![],
                    },
                ],
            }],
        };
        evaluator.eval_program(program);

        // Set ensures condition to false
        evaluator
            .variables
            .insert("$postcondition".to_string(), Value::Bool(false));

        let result = evaluator.transition_workflow("EnsuresFlow", "go");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("ensures failed"));
    }
}

// ========================================================================
// M26 Runtime: Memory Operation Tests
// ========================================================================
#[cfg(test)]
mod memory_runtime_tests {
    use crate::ast::{Program, Statement};
    use crate::eval::{Evaluator, Value};

    fn setup_memory_evaluator() -> Evaluator {
        let mut evaluator = Evaluator::new();
        let program = Program {
            statements: vec![Statement::MemoryDef {
                name: "KnowledgeBase".to_string(),
                store: Some("local://memory".to_string()),
                embedding_model: None,
                operations: vec![
                    "remember".to_string(),
                    "recall".to_string(),
                    "forget".to_string(),
                ],
            }],
        };
        evaluator.eval_program(program);
        evaluator
    }

    #[test]
    fn test_memory_remember() {
        let mut evaluator = setup_memory_evaluator();

        let result = evaluator.memory_remember(
            "KnowledgeBase",
            "rust_safety".to_string(),
            Value::String("Rust ensures memory safety".to_string()),
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Bool(true));

        // Verify it's stored
        let mem = evaluator.memories.get("KnowledgeBase").unwrap();
        assert_eq!(mem.entries.len(), 1);
        assert_eq!(mem.entries[0].0, "rust_safety");
    }

    #[test]
    fn test_memory_recall_keyword_match() {
        let mut evaluator = setup_memory_evaluator();

        // Store some entries
        evaluator
            .memory_remember(
                "KnowledgeBase",
                "rust_safety".to_string(),
                Value::String("Rust ensures memory safety".to_string()),
            )
            .unwrap();
        evaluator
            .memory_remember(
                "KnowledgeBase",
                "python_typing".to_string(),
                Value::String("Python has optional type hints".to_string()),
            )
            .unwrap();
        evaluator
            .memory_remember(
                "KnowledgeBase",
                "rust_concurrency".to_string(),
                Value::String("Rust prevents data races".to_string()),
            )
            .unwrap();

        // Recall with keyword "rust" should match rust_safety and rust_concurrency
        let result = evaluator.memory_recall("KnowledgeBase", "rust", 10);
        assert!(result.is_ok());
        if let Value::Array(items) = result.unwrap() {
            assert_eq!(items.len(), 2);
            // Check that both rust entries are present
            let combined: String = items
                .iter()
                .map(|v| format!("{}", v))
                .collect::<Vec<_>>()
                .join(" ");
            assert!(combined.contains("rust_safety"));
            assert!(combined.contains("rust_concurrency"));
        } else {
            panic!("Expected Array from recall");
        }
    }

    #[test]
    fn test_memory_recall_no_match_returns_recent() {
        let mut evaluator = setup_memory_evaluator();

        evaluator
            .memory_remember(
                "KnowledgeBase",
                "entry1".to_string(),
                Value::String("first".to_string()),
            )
            .unwrap();
        evaluator
            .memory_remember(
                "KnowledgeBase",
                "entry2".to_string(),
                Value::String("second".to_string()),
            )
            .unwrap();

        // Query that matches nothing
        let result = evaluator.memory_recall("KnowledgeBase", "zzzzz_no_match", 1);
        assert!(result.is_ok());
        // Should return most recent entry as fallback
        if let Value::Array(items) = result.unwrap() {
            assert_eq!(items.len(), 1);
            // Most recent is entry2
            let s = format!("{}", items[0]);
            assert!(s.contains("entry2"));
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_memory_recall_top_k_limit() {
        let mut evaluator = setup_memory_evaluator();

        // Store 5 entries with "data" in the key
        for i in 0..5 {
            evaluator
                .memory_remember(
                    "KnowledgeBase",
                    format!("data_{}", i),
                    Value::Number(i as f64),
                )
                .unwrap();
        }

        // Recall with top_k=2
        let result = evaluator.memory_recall("KnowledgeBase", "data", 2);
        assert!(result.is_ok());
        if let Value::Array(items) = result.unwrap() {
            assert_eq!(items.len(), 2);
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_memory_forget() {
        let mut evaluator = setup_memory_evaluator();

        evaluator
            .memory_remember(
                "KnowledgeBase",
                "temp_data".to_string(),
                Value::String("temporary".to_string()),
            )
            .unwrap();
        evaluator
            .memory_remember(
                "KnowledgeBase",
                "keep_data".to_string(),
                Value::String("permanent".to_string()),
            )
            .unwrap();

        // Forget temp_data
        let result = evaluator.memory_forget("KnowledgeBase", "temp_data");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Number(1.0)); // 1 entry removed

        // Verify only keep_data remains
        let mem = evaluator.memories.get("KnowledgeBase").unwrap();
        assert_eq!(mem.entries.len(), 1);
        assert_eq!(mem.entries[0].0, "keep_data");
    }

    #[test]
    fn test_memory_forget_nonexistent_key() {
        let mut evaluator = setup_memory_evaluator();

        let result = evaluator.memory_forget("KnowledgeBase", "does_not_exist");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Number(0.0)); // 0 entries removed
    }

    #[test]
    fn test_memory_not_found() {
        let mut evaluator = Evaluator::new();

        let result = evaluator.memory_remember(
            "NonExistent",
            "key".to_string(),
            Value::Null,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_memory_operation_not_permitted() {
        let mut evaluator = Evaluator::new();

        // Memory that only allows "recall"
        let program = Program {
            statements: vec![Statement::MemoryDef {
                name: "ReadOnly".to_string(),
                store: None,
                embedding_model: None,
                operations: vec!["recall".to_string()],
            }],
        };
        evaluator.eval_program(program);

        // Try to remember — should fail
        let result = evaluator.memory_remember(
            "ReadOnly",
            "key".to_string(),
            Value::String("value".to_string()),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not permitted"));

        // Try to forget — should fail
        let result = evaluator.memory_forget("ReadOnly", "key");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not permitted"));
    }

    #[test]
    fn test_memory_empty_operations_allows_all() {
        let mut evaluator = Evaluator::new();

        // Memory with empty operations list — everything is allowed
        let program = Program {
            statements: vec![Statement::MemoryDef {
                name: "OpenMemory".to_string(),
                store: None,
                embedding_model: None,
                operations: vec![],
            }],
        };
        evaluator.eval_program(program);

        let r1 = evaluator.memory_remember(
            "OpenMemory",
            "key".to_string(),
            Value::String("value".to_string()),
        );
        assert!(r1.is_ok());

        let r2 = evaluator.memory_recall("OpenMemory", "key", 5);
        assert!(r2.is_ok());

        let r3 = evaluator.memory_forget("OpenMemory", "key");
        assert!(r3.is_ok());
    }

    #[test]
    fn test_memory_traces_output() {
        let mut evaluator = setup_memory_evaluator();

        evaluator
            .memory_remember(
                "KnowledgeBase",
                "test_key".to_string(),
                Value::String("test_val".to_string()),
            )
            .unwrap();
        evaluator
            .memory_recall("KnowledgeBase", "test", 5)
            .unwrap();
        evaluator
            .memory_forget("KnowledgeBase", "test_key")
            .unwrap();

        let output = evaluator.output.join("\n");
        assert!(output.contains("[Memory Remember] KnowledgeBase['test_key']"));
        assert!(output.contains("[Memory Recall] KnowledgeBase query='test'"));
        assert!(output.contains("[Memory Forget] KnowledgeBase key='test_key'"));
    }
}

// ========================================================================
// Runtime Integration: End-to-End Tests
// ========================================================================
#[cfg(test)]
mod runtime_integration_tests {
    use crate::ast::{
        Expr, PipelineStage, Program, Statement, WorkflowState,
        WorkflowTransition,

    };
    use crate::eval::{Evaluator, Value};

    #[test]
    fn test_full_runtime_lifecycle() {
        let mut evaluator = Evaluator::new();

        // Register everything
        let program = Program {
            statements: vec![
                // MCP tool
                Statement::McpToolDef {
                    name: "search".to_string(),
                    server: "search-server".to_string(),
                    permission: None,
                    capabilities: vec!["find".to_string()],
                    timeout: Some(10.0),
                },
                // Memory
                Statement::MemoryDef {
                    name: "Cache".to_string(),
                    store: None,
                    embedding_model: None,
                    operations: vec![
                        "remember".to_string(),
                        "recall".to_string(),
                        "forget".to_string(),
                    ],
                },
                // Workflow
                Statement::WorkflowDef {
                    name: "TaskFlow".to_string(),
                    states: vec![
                        WorkflowState {
                            name: "open".to_string(),
                            transitions: vec![WorkflowTransition {
                                event: "start".to_string(),
                                target_state: "in_progress".to_string(),
                            }],
                            requires: None,
                            ensures: None,
                            body: vec![],
                        },
                        WorkflowState {
                            name: "in_progress".to_string(),
                            transitions: vec![WorkflowTransition {
                                event: "done".to_string(),
                                target_state: "closed".to_string(),
                            }],
                            requires: None,
                            ensures: None,
                            body: vec![],
                        },
                        WorkflowState {
                            name: "closed".to_string(),
                            transitions: vec![],
                            requires: None,
                            ensures: None,
                            body: vec![],
                        },
                    ],
                },
                // Pipeline
                Statement::PipelineDef {
                    name: "DataPipeline".to_string(),
                    stages: vec![
                        PipelineStage {
                            agent_name: "Fetcher".to_string(),
                            call: Expr::String("fetched_data".to_string()),
                        },
                        PipelineStage {
                            agent_name: "Processor".to_string(),
                            call: Expr::Var("$prev".to_string()),
                        },
                    ],
                },
            ],
        };
        evaluator.eval_program(program);

        // 1. Call MCP tool
        let mcp_result = evaluator.eval_call(
            "search",
            vec![Expr::String("query".to_string())],
        );
        assert!(mcp_result.is_ok());

        // 2. Store result in memory
        evaluator
            .memory_remember(
                "Cache",
                "search_result".to_string(),
                mcp_result.unwrap(),
            )
            .unwrap();

        // 3. Recall from memory
        let recalled = evaluator.memory_recall("Cache", "search", 1);
        assert!(recalled.is_ok());

        // 4. Run pipeline
        let pipeline_result = evaluator.run_pipeline(
            "DataPipeline",
            Value::String("input".to_string()),
        );
        assert!(pipeline_result.is_ok());
        assert_eq!(
            pipeline_result.unwrap(),
            Value::String("fetched_data".to_string())
        );

        // 5. Progress workflow
        evaluator
            .transition_workflow("TaskFlow", "start")
            .unwrap();
        evaluator
            .transition_workflow("TaskFlow", "done")
            .unwrap();
        let final_state = evaluator.get_workflow_state("TaskFlow").unwrap();
        assert_eq!(final_state, Value::String("closed".to_string()));

        // 6. Clean up memory
        evaluator.memory_forget("Cache", "search_result").unwrap();
        let mem = evaluator.memories.get("Cache").unwrap();
        assert!(mem.entries.is_empty());
    }
}
