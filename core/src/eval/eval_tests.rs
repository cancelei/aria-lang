#[cfg(test)]
mod eval_tests {
    use crate::eval::{Evaluator, Value};
    use crate::ast::{Expr, Statement, Program};

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
    use crate::eval::{Evaluator, Value};
    use crate::ast::Expr;

    #[test]
    fn test_permission_denied() {
        let mut evaluator = Evaluator::new();

        // Define tool
        evaluator.eval_tool_def(
            "read_file".to_string(),
            vec!["path".to_string()],
            Some("io.read".to_string()),
            None,
        ).unwrap();

        // Define agent WITHOUT read_file
        evaluator.eval_agent_def(
            "RestrictedAgent".to_string(),
            vec!["write_file".to_string()],
            vec![], vec![],
        ).unwrap();

        // Spawn agent
        evaluator.eval_spawn("$agent".to_string(), "RestrictedAgent".to_string()).unwrap();

        // Set agent context
        evaluator.current_agent = Some("$agent".to_string());

        // Try to call read_file - should fail
        let result = evaluator.eval_call("read_file", vec![Expr::String("/etc/passwd".to_string())]);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Permission Denied"));
    }

    #[test]
    fn test_permission_allowed() {
        let mut evaluator = Evaluator::new();

        // Use echo instead of write_file for simpler testing
        evaluator.eval_tool_def("echo".to_string(), vec![], Some("io.write".to_string()), Some(5.0)).unwrap();
        evaluator.eval_agent_def("WriterAgent".to_string(), vec!["echo".to_string()], vec![], vec![]).unwrap();
        evaluator.eval_spawn("$writer".to_string(), "WriterAgent".to_string()).unwrap();
        evaluator.current_agent = Some("$writer".to_string());

        let result = evaluator.eval_call("echo", vec![Expr::String("test".to_string())]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_main_context_unrestricted() {
        let mut evaluator = Evaluator::new();
        // Use echo for testing
        evaluator.eval_tool_def("echo".to_string(), vec![], Some("system.execute".to_string()), Some(5.0)).unwrap();

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
