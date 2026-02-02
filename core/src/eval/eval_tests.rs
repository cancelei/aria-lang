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
