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
}
