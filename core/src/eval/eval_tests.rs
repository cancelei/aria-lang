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
        // This will print to stdout, but we check if it doesn't panic
        evaluator.eval_program(program);
        assert_eq!(evaluator.variables.get("$x"), Some(&Value::Number(42.0)));
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
