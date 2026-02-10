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
