use logos::Logos;

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\n\f]+")] // Skip whitespace
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
}
