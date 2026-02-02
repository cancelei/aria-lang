#[cfg(test)]
mod tests {
    use crate::parser::Parser;
    use crate::ast::{Statement, Expr};

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
}
