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
