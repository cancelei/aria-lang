#[cfg(test)]
mod tests {
    use crate::ast::{Expr, Statement};
    use crate::parser::Parser;

    #[test]
    fn test_parse_let() {
        let input = "let $x = 10";
        let mut parser = Parser::new(input);
        let program = parser.parse_program().unwrap();
        assert_eq!(program.statements.len(), 1);
        if let Statement::Let { name, value } = &program.statements[0] {
            assert_eq!(name, "$x");
            assert_eq!(value, &Expr::Number(10.0));
        } else {
            panic!("Expected Let statement");
        }
    }

    #[test]
    fn test_parse_gate() {
        let input = "gate \"test\" { print 1 }";
        let mut parser = Parser::new(input);
        let program = parser.parse_program().unwrap();
        assert_eq!(program.statements.len(), 1);
        if let Statement::Gate { prompt, body } = &program.statements[0] {
            assert_eq!(prompt, &Expr::String("test".to_string()));
            assert_eq!(body.len(), 1);
        } else {
            panic!("Expected Gate statement");
        }
    }

    // Step 3: Test tool definition parsing
    #[test]
    fn test_parse_tool_def() {
        let input = r#"tool shell(command: string) {
            permission: "system.execute",
            timeout: 30
        }"#;
        let mut parser = Parser::new(input);
        let program = parser.parse_program().unwrap();
        assert_eq!(program.statements.len(), 1);
        if let Statement::ToolDef {
            name,
            params,
            permission,
            timeout,
        } = &program.statements[0]
        {
            assert_eq!(name, "shell");
            assert_eq!(params, &vec!["command".to_string()]);
            assert_eq!(permission, &Some("system.execute".to_string()));
            assert_eq!(timeout, &Some(30.0));
        } else {
            panic!("Expected ToolDef statement");
        }
    }

    #[test]
    fn test_parse_tool_def_no_timeout() {
        let input = r#"tool fetch(url: string) {
            permission: "network.http"
        }"#;
        let mut parser = Parser::new(input);
        let program = parser.parse_program().unwrap();
        assert_eq!(program.statements.len(), 1);
        if let Statement::ToolDef {
            name,
            params,
            permission,
            timeout,
        } = &program.statements[0]
        {
            assert_eq!(name, "fetch");
            assert_eq!(params, &vec!["url".to_string()]);
            assert_eq!(permission, &Some("network.http".to_string()));
            assert_eq!(timeout, &None);
        } else {
            panic!("Expected ToolDef statement");
        }
    }

    // Step 4: Test function call parsing
    #[test]
    fn test_parse_call() {
        let input = r#"print shell("ls -la")"#;
        let mut parser = Parser::new(input);
        let program = parser.parse_program().unwrap();
        assert_eq!(program.statements.len(), 1);
        if let Statement::Print(expr) = &program.statements[0] {
            if let Expr::Call { name, args } = expr {
                assert_eq!(name, "shell");
                assert_eq!(args.len(), 1);
                if let Expr::String(s) = &args[0] {
                    assert_eq!(s, "ls -la");
                } else {
                    panic!("Expected string argument");
                }
            } else {
                panic!("Expected Call expression");
            }
        } else {
            panic!("Expected Print statement");
        }
    }

    #[test]
    fn test_parse_call_multiple_args() {
        let input = r#"print add(1, 2)"#;
        let mut parser = Parser::new(input);
        let program = parser.parse_program().unwrap();
        assert_eq!(program.statements.len(), 1);
        if let Statement::Print(expr) = &program.statements[0] {
            if let Expr::Call { name, args } = expr {
                assert_eq!(name, "add");
                assert_eq!(args.len(), 2);
            } else {
                panic!("Expected Call expression");
            }
        } else {
            panic!("Expected Print statement");
        }
    }

    // Step 5: Test agent definition with allow and tasks
    #[test]
    fn test_parse_agent_def_with_allow() {
        let input = r#"agent DevOpsAssistant {
            allow shell
        }"#;
        let mut parser = Parser::new(input);
        let program = parser.parse_program().unwrap();
        assert_eq!(program.statements.len(), 1);
        if let Statement::AgentDef {
            name,
            allow_list,
            tasks,
            body: _,
        } = &program.statements[0]
        {
            assert_eq!(name, "DevOpsAssistant");
            assert_eq!(allow_list, &vec!["shell".to_string()]);
            assert_eq!(tasks.len(), 0);
        } else {
            panic!("Expected AgentDef statement");
        }
    }

    #[test]
    fn test_parse_agent_def_with_task() {
        let input = r#"agent DevOpsAssistant {
            allow shell

            task cleanup_logs() {
                print "Cleaning logs"
            }
        }"#;
        let mut parser = Parser::new(input);
        let program = parser.parse_program().unwrap();
        assert_eq!(program.statements.len(), 1);
        if let Statement::AgentDef {
            name,
            allow_list,
            tasks,
            body: _,
        } = &program.statements[0]
        {
            assert_eq!(name, "DevOpsAssistant");
            assert_eq!(allow_list, &vec!["shell".to_string()]);
            assert_eq!(tasks.len(), 1);
            assert_eq!(tasks[0].name, "cleanup_logs");
            assert_eq!(tasks[0].params.len(), 0);
            assert_eq!(tasks[0].body.len(), 1);
        } else {
            panic!("Expected AgentDef statement");
        }
    }

    #[test]
    fn test_parse_agent_def_with_params() {
        let input = r#"agent Worker {
            task process(data: string) {
                print data
            }
        }"#;
        let mut parser = Parser::new(input);
        let program = parser.parse_program().unwrap();
        assert_eq!(program.statements.len(), 1);
        if let Statement::AgentDef {
            name,
            allow_list: _,
            tasks,
            body: _,
        } = &program.statements[0]
        {
            assert_eq!(name, "Worker");
            assert_eq!(tasks.len(), 1);
            assert_eq!(tasks[0].name, "process");
            assert_eq!(tasks[0].params, vec!["data".to_string()]);
        } else {
            panic!("Expected AgentDef statement");
        }
    }

    // Step 6: Test spawn, delegate, and main
    #[test]
    fn test_parse_spawn() {
        let input = r#"let $bot = spawn DevOpsAssistant"#;
        let mut parser = Parser::new(input);
        let program = parser.parse_program().unwrap();
        assert_eq!(program.statements.len(), 1);
        if let Statement::Spawn {
            var_name,
            agent_name,
        } = &program.statements[0]
        {
            assert_eq!(var_name, "$bot");
            assert_eq!(agent_name, "DevOpsAssistant");
        } else {
            panic!("Expected Spawn statement");
        }
    }

    #[test]
    fn test_parse_delegate() {
        let input = r#"delegate bot.cleanup_logs()"#;
        let mut parser = Parser::new(input);
        let program = parser.parse_program().unwrap();
        assert_eq!(program.statements.len(), 1);
        if let Statement::Delegate { call } = &program.statements[0] {
            if let Expr::Call { name, args } = call {
                assert_eq!(name, "bot.cleanup_logs");
                assert_eq!(args.len(), 0);
            } else {
                panic!("Expected Call expression in delegate");
            }
        } else {
            panic!("Expected Delegate statement");
        }
    }

    #[test]
    fn test_parse_delegate_with_args() {
        let input = r#"delegate bot.process("data")"#;
        let mut parser = Parser::new(input);
        let program = parser.parse_program().unwrap();
        assert_eq!(program.statements.len(), 1);
        if let Statement::Delegate { call } = &program.statements[0] {
            if let Expr::Call { name, args } = call {
                assert_eq!(name, "bot.process");
                assert_eq!(args.len(), 1);
            } else {
                panic!("Expected Call expression in delegate");
            }
        } else {
            panic!("Expected Delegate statement");
        }
    }

    #[test]
    fn test_parse_main() {
        let input = r#"main {
            print "Starting"
            let $x = 42
        }"#;
        let mut parser = Parser::new(input);
        let program = parser.parse_program().unwrap();
        assert_eq!(program.statements.len(), 1);
        if let Statement::Main { body } = &program.statements[0] {
            assert_eq!(body.len(), 2);
        } else {
            panic!("Expected Main statement");
        }
    }

    #[test]
    fn test_parse_complete_program() {
        let input = r#"
            tool shell(command: string) {
                permission: "system.execute",
                timeout: 30
            }

            agent DevOpsAssistant {
                allow shell

                task cleanup_logs() {
                    print shell("rm -rf /tmp/*.log")
                }
            }

            main {
                let $bot = spawn DevOpsAssistant
                delegate bot.cleanup_logs()
            }
        "#;
        let mut parser = Parser::new(input);
        let program = parser.parse_program().unwrap();
        assert_eq!(program.statements.len(), 3);

        // Verify tool def
        if let Statement::ToolDef { name, .. } = &program.statements[0] {
            assert_eq!(name, "shell");
        } else {
            panic!("Expected ToolDef statement");
        }

        // Verify agent def
        if let Statement::AgentDef { name, tasks, .. } = &program.statements[1] {
            assert_eq!(name, "DevOpsAssistant");
            assert_eq!(tasks.len(), 1);
        } else {
            panic!("Expected AgentDef statement");
        }

        // Verify main block
        if let Statement::Main { body } = &program.statements[2] {
            assert_eq!(body.len(), 2);
        } else {
            panic!("Expected Main statement");
        }
    }

    // Parser error tests
    #[test]
    fn test_parse_error_missing_var_name() {
        let input = "let = 10";
        let mut parser = Parser::new(input);
        let result = parser.parse_program();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Expected variable name"));
    }

    #[test]
    fn test_parse_error_missing_tool_name() {
        let input = "tool () {}";
        let mut parser = Parser::new(input);
        let result = parser.parse_program();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Expected tool name"));
    }

    #[test]
    fn test_parse_error_unclosed_brace() {
        let input = "main { print 1";
        let mut parser = Parser::new(input);
        let result = parser.parse_program();
        assert!(result.is_err());
    }

    // ====================================================================
    // M21: MCP Tool Definition Parser Tests
    // ====================================================================

    #[test]
    fn test_parse_mcp_tool_def() {
        let input = r#"tool code_search from mcp("github-server") {
            permission: "mcp.connect",
            capabilities: [search_code, search_issues],
            timeout: 15
        }"#;
        let mut parser = Parser::new(input);
        let program = parser.parse_program().unwrap();
        assert_eq!(program.statements.len(), 1);
        if let Statement::McpToolDef {
            name,
            server,
            permission,
            capabilities,
            timeout,
        } = &program.statements[0]
        {
            assert_eq!(name, "code_search");
            assert_eq!(server, "github-server");
            assert_eq!(permission, &Some("mcp.connect".to_string()));
            assert_eq!(capabilities, &vec!["search_code".to_string(), "search_issues".to_string()]);
            assert_eq!(timeout, &Some(15.0));
        } else {
            panic!("Expected McpToolDef statement");
        }
    }

    #[test]
    fn test_parse_mcp_tool_minimal() {
        let input = r#"tool weather from mcp("weather-server") {
            permission: "mcp.weather"
        }"#;
        let mut parser = Parser::new(input);
        let program = parser.parse_program().unwrap();
        assert_eq!(program.statements.len(), 1);
        if let Statement::McpToolDef {
            name,
            server,
            capabilities,
            ..
        } = &program.statements[0]
        {
            assert_eq!(name, "weather");
            assert_eq!(server, "weather-server");
            assert!(capabilities.is_empty());
        } else {
            panic!("Expected McpToolDef statement");
        }
    }

    // ====================================================================
    // M22: Orchestration Parser Tests
    // ====================================================================

    #[test]
    fn test_parse_pipeline() {
        let input = r#"pipeline ReviewPipeline {
            stage Analyst -> analyze($input)
            stage Reviewer -> review($prev)
            stage Summarizer -> summarize($prev)
        }"#;
        let mut parser = Parser::new(input);
        let program = parser.parse_program().unwrap();
        assert_eq!(program.statements.len(), 1);
        if let Statement::PipelineDef { name, stages } = &program.statements[0] {
            assert_eq!(name, "ReviewPipeline");
            assert_eq!(stages.len(), 3);
            assert_eq!(stages[0].agent_name, "Analyst");
            assert_eq!(stages[1].agent_name, "Reviewer");
            assert_eq!(stages[2].agent_name, "Summarizer");
        } else {
            panic!("Expected PipelineDef statement");
        }
    }

    #[test]
    fn test_parse_concurrent() {
        let input = r#"concurrent ResearchTask {
            agent WebSearcher -> search_web($query)
            agent CodeSearcher -> search_codebase($query)
            merge combine_results($results)
        }"#;
        let mut parser = Parser::new(input);
        let program = parser.parse_program().unwrap();
        assert_eq!(program.statements.len(), 1);
        if let Statement::ConcurrentDef {
            name,
            branches,
            merge_fn,
        } = &program.statements[0]
        {
            assert_eq!(name, "ResearchTask");
            assert_eq!(branches.len(), 2);
            assert_eq!(branches[0].agent_name, "WebSearcher");
            assert_eq!(branches[1].agent_name, "CodeSearcher");
            assert!(merge_fn.is_some());
        } else {
            panic!("Expected ConcurrentDef statement");
        }
    }

    #[test]
    fn test_parse_handoff() {
        let input = r#"handoff SupportFlow {
            agent Triage -> classify($input)
            route "billing" => BillingAgent
            route "technical" => TechAgent
            route _ => HumanEscalation
        }"#;
        let mut parser = Parser::new(input);
        let program = parser.parse_program().unwrap();
        assert_eq!(program.statements.len(), 1);
        if let Statement::HandoffDef {
            name,
            agent_name,
            routes,
            ..
        } = &program.statements[0]
        {
            assert_eq!(name, "SupportFlow");
            assert_eq!(agent_name, "Triage");
            assert_eq!(routes.len(), 3);
            assert_eq!(routes[0].pattern, "billing");
            assert_eq!(routes[0].target_agent, "BillingAgent");
            assert_eq!(routes[1].pattern, "technical");
            assert_eq!(routes[2].pattern, "_");
            assert_eq!(routes[2].target_agent, "HumanEscalation");
        } else {
            panic!("Expected HandoffDef statement");
        }
    }

    // ====================================================================
    // M23: A2A Protocol Parser Tests
    // ====================================================================

    #[test]
    fn test_parse_a2a() {
        let input = r#"a2a ResearchCard {
            discovery: "/.well-known/agent.json"
            skills: [search, analyze, summarize]
            endpoint: "https://agents.aria.dev/research"
        }"#;
        let mut parser = Parser::new(input);
        let program = parser.parse_program().unwrap();
        assert_eq!(program.statements.len(), 1);
        if let Statement::A2ADef {
            name,
            discovery,
            skills,
            endpoint,
        } = &program.statements[0]
        {
            assert_eq!(name, "ResearchCard");
            assert_eq!(discovery, &Some("/.well-known/agent.json".to_string()));
            assert_eq!(skills, &vec!["search".to_string(), "analyze".to_string(), "summarize".to_string()]);
            assert_eq!(endpoint, &Some("https://agents.aria.dev/research".to_string()));
        } else {
            panic!("Expected A2ADef statement");
        }
    }

    // ====================================================================
    // M24: Workflow Parser Tests
    // ====================================================================

    #[test]
    fn test_parse_workflow() {
        let input = r#"workflow OrderProcessing {
            state pending {
                on receive_order -> validating
            }
            state validating {
                requires $order_valid
                on valid -> processing
                on invalid -> rejected
            }
            state processing {
                on complete -> shipped
            }
            state shipped {
                ensures $tracking_exists
            }
            state rejected {
            }
        }"#;
        let mut parser = Parser::new(input);
        let program = parser.parse_program().unwrap();
        assert_eq!(program.statements.len(), 1);
        if let Statement::WorkflowDef { name, states } = &program.statements[0] {
            assert_eq!(name, "OrderProcessing");
            assert_eq!(states.len(), 5);
            assert_eq!(states[0].name, "pending");
            assert_eq!(states[0].transitions.len(), 1);
            assert_eq!(states[0].transitions[0].event, "receive_order");
            assert_eq!(states[0].transitions[0].target_state, "validating");

            assert_eq!(states[1].name, "validating");
            assert!(states[1].requires.is_some());
            assert_eq!(states[1].transitions.len(), 2);

            assert_eq!(states[3].name, "shipped");
            assert!(states[3].ensures.is_some());
        } else {
            panic!("Expected WorkflowDef statement");
        }
    }

    // ====================================================================
    // M25: Model Declaration Parser Tests
    // ====================================================================

    #[test]
    fn test_parse_model() {
        let input = r#"model assistant {
            capability: "chat_completion"
            provider: "openai"
            supports: [tool_calling, structured_output, vision]
        }"#;
        let mut parser = Parser::new(input);
        let program = parser.parse_program().unwrap();
        assert_eq!(program.statements.len(), 1);
        if let Statement::ModelDef {
            name,
            capability,
            provider,
            supports,
        } = &program.statements[0]
        {
            assert_eq!(name, "assistant");
            assert_eq!(capability, &Some("chat_completion".to_string()));
            assert_eq!(provider, &Some("openai".to_string()));
            assert_eq!(supports.len(), 3);
            assert!(supports.contains(&"tool_calling".to_string()));
            assert!(supports.contains(&"structured_output".to_string()));
            assert!(supports.contains(&"vision".to_string()));
        } else {
            panic!("Expected ModelDef statement");
        }
    }

    // ====================================================================
    // M26: Memory Parser Tests
    // ====================================================================

    #[test]
    fn test_parse_memory() {
        let input = r#"memory ProjectKnowledge {
            store: "chromadb://localhost:8000/project"
            embedding: "text_embedder"
            operations: [remember, recall, forget]
        }"#;
        let mut parser = Parser::new(input);
        let program = parser.parse_program().unwrap();
        assert_eq!(program.statements.len(), 1);
        if let Statement::MemoryDef {
            name,
            store,
            embedding_model,
            operations,
        } = &program.statements[0]
        {
            assert_eq!(name, "ProjectKnowledge");
            assert_eq!(store, &Some("chromadb://localhost:8000/project".to_string()));
            assert_eq!(embedding_model, &Some("text_embedder".to_string()));
            assert_eq!(operations.len(), 3);
        } else {
            panic!("Expected MemoryDef statement");
        }
    }

    // ====================================================================
    // Integration: Full program with all M21-M26 features
    // ====================================================================

    #[test]
    fn test_parse_full_program_with_all_milestones() {
        let input = r#"
            model gpt4 {
                capability: "chat_completion"
                provider: "openai"
                supports: [tool_calling]
            }

            tool code_search from mcp("github-mcp") {
                permission: "mcp.github",
                capabilities: [search_code],
                timeout: 10
            }

            memory CodeMemory {
                store: "chromadb://localhost/code"
                embedding: "ada"
                operations: [recall, remember]
            }

            tool analyze(code: string) {
                permission: "code.read",
                timeout: 30
            }

            agent CodeAnalyzer {
                allow analyze
                allow code_search

                task review(input: string) {
                    print "Reviewing code"
                }
            }

            pipeline ReviewPipeline {
                stage Fetcher -> fetch($url)
                stage Analyzer -> analyze($code)
            }

            concurrent MultiSearch {
                agent WebSearcher -> search_web($q)
                agent CodeSearcher -> search_code($q)
                merge combine($results)
            }

            handoff Support {
                agent Triage -> classify($issue)
                route "bug" => BugFixer
                route _ => General
            }

            a2a AnalyzerCard {
                discovery: "/.well-known/agent.json"
                skills: [analyze, review]
                endpoint: "https://api.example.com/analyze"
            }

            workflow CodeReview {
                state draft {
                    on submit -> review
                }
                state review {
                    on approve -> merged
                    on reject -> draft
                }
                state merged {
                }
            }

            main {
                let $bot = spawn CodeAnalyzer
                delegate bot.review("test")
            }
        "#;

        let mut parser = Parser::new(input);
        let program = parser.parse_program().unwrap();

        // Count all statement types
        let mut model_count = 0;
        let mut mcp_count = 0;
        let mut memory_count = 0;
        let mut tool_count = 0;
        let mut agent_count = 0;
        let mut pipeline_count = 0;
        let mut concurrent_count = 0;
        let mut handoff_count = 0;
        let mut a2a_count = 0;
        let mut workflow_count = 0;
        let mut main_count = 0;

        for stmt in &program.statements {
            match stmt {
                Statement::ModelDef { .. } => model_count += 1,
                Statement::McpToolDef { .. } => mcp_count += 1,
                Statement::MemoryDef { .. } => memory_count += 1,
                Statement::ToolDef { .. } => tool_count += 1,
                Statement::AgentDef { .. } => agent_count += 1,
                Statement::PipelineDef { .. } => pipeline_count += 1,
                Statement::ConcurrentDef { .. } => concurrent_count += 1,
                Statement::HandoffDef { .. } => handoff_count += 1,
                Statement::A2ADef { .. } => a2a_count += 1,
                Statement::WorkflowDef { .. } => workflow_count += 1,
                Statement::Main { .. } => main_count += 1,
                _ => {}
            }
        }

        assert_eq!(model_count, 1, "Expected 1 model definition");
        assert_eq!(mcp_count, 1, "Expected 1 MCP tool definition");
        assert_eq!(memory_count, 1, "Expected 1 memory definition");
        assert_eq!(tool_count, 1, "Expected 1 tool definition");
        assert_eq!(agent_count, 1, "Expected 1 agent definition");
        assert_eq!(pipeline_count, 1, "Expected 1 pipeline definition");
        assert_eq!(concurrent_count, 1, "Expected 1 concurrent definition");
        assert_eq!(handoff_count, 1, "Expected 1 handoff definition");
        assert_eq!(a2a_count, 1, "Expected 1 A2A definition");
        assert_eq!(workflow_count, 1, "Expected 1 workflow definition");
        assert_eq!(main_count, 1, "Expected 1 main block");
    }
}
