use logos::Logos;

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\n\f]+")] // Skip whitespace
#[logos(skip r"//[^\n]*")]  // Skip line comments
pub enum Token {
    // Keywords
    #[token("let")]
    Let,
    #[token("think")]
    Think,
    #[token("gate")]
    Gate,
    #[token("print")]
    Print,
    #[token("agent")]
    Agent,
    #[token("tool")]
    Tool,
    #[token("task")]
    Task,
    #[token("allow")]
    Allow,
    #[token("spawn")]
    Spawn,
    #[token("delegate")]
    Delegate,
    #[token("permission")]
    Permission,
    #[token("timeout")]
    Timeout,
    #[token("main")]
    Main,
    #[token("return")]
    Return,
    #[token("else")]
    Else,

    // Sigils
    #[regex(r"\$[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    VarIdent(String),
    
    #[regex(r"@[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    AgentIdent(String),

    // Literals
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Ident(String),

    #[regex(r#""([^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        s[1..s.len()-1].to_string()
    })]
    String(String),

    #[regex(r"[0-9]+(\.[0-9]+)?", |lex| lex.slice().parse::<f64>().ok())]
    Number(f64),

    // Symbols
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("=")]
    Assign,
    #[token(":")]
    Colon,
    #[token(",")]
    Comma,
    #[token("->")]
    Arrow,
    #[token(".")]
    Dot,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer() {
        let input = r#"
            let $name = "Aria"
            @agent {
                think { "processing..." }
                print $name
            }
        "#;
        let mut lex = Token::lexer(input);

        assert_eq!(lex.next(), Some(Ok(Token::Let)));
        assert_eq!(lex.next(), Some(Ok(Token::VarIdent("$name".to_string()))));
        assert_eq!(lex.next(), Some(Ok(Token::Assign)));
        assert_eq!(lex.next(), Some(Ok(Token::String("Aria".to_string()))));
        assert_eq!(lex.next(), Some(Ok(Token::AgentIdent("@agent".to_string()))));
        assert_eq!(lex.next(), Some(Ok(Token::LBrace)));
    }

    #[test]
    fn test_new_tokens() {
        let input = r#"
            tool task allow spawn delegate permission timeout main return else
            : , ->
        "#;
        let mut lex = Token::lexer(input);

        assert_eq!(lex.next(), Some(Ok(Token::Tool)));
        assert_eq!(lex.next(), Some(Ok(Token::Task)));
        assert_eq!(lex.next(), Some(Ok(Token::Allow)));
        assert_eq!(lex.next(), Some(Ok(Token::Spawn)));
        assert_eq!(lex.next(), Some(Ok(Token::Delegate)));
        assert_eq!(lex.next(), Some(Ok(Token::Permission)));
        assert_eq!(lex.next(), Some(Ok(Token::Timeout)));
        assert_eq!(lex.next(), Some(Ok(Token::Main)));
        assert_eq!(lex.next(), Some(Ok(Token::Return)));
        assert_eq!(lex.next(), Some(Ok(Token::Else)));
        assert_eq!(lex.next(), Some(Ok(Token::Colon)));
        assert_eq!(lex.next(), Some(Ok(Token::Comma)));
        assert_eq!(lex.next(), Some(Ok(Token::Arrow)));
        assert_eq!(lex.next(), None);
    }
}
