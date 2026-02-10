use logos::Logos;

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\n\f]+")] // Skip whitespace
#[logos(skip r"//[^\n]*")] // Skip line comments
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

    // M21: MCP Integration keywords
    #[token("from")]
    From,
    #[token("mcp")]
    Mcp,
    #[token("capabilities")]
    Capabilities,
    #[token("operations")]
    Operations,

    // M22: Orchestration keywords
    #[token("pipeline")]
    Pipeline,
    #[token("stage")]
    Stage,
    #[token("concurrent")]
    Concurrent,
    #[token("merge")]
    Merge,
    #[token("handoff")]
    Handoff,
    #[token("route")]
    Route,

    // M23: A2A Protocol keywords
    #[token("a2a")]
    A2A,
    #[token("discovery")]
    Discovery,
    #[token("skills")]
    Skills,
    #[token("endpoint")]
    Endpoint,

    // M24: Workflow keywords
    #[token("workflow")]
    Workflow,
    #[token("state")]
    State,
    #[token("on")]
    On,
    #[token("requires")]
    Requires,
    #[token("ensures")]
    Ensures,

    // M25: Model declaration keywords
    #[token("model")]
    Model,
    #[token("capability")]
    Capability,
    #[token("supports")]
    Supports,
    #[token("uses")]
    Uses,
    #[token("provider")]
    Provider,

    // M26: Memory keywords
    #[token("memory")]
    Memory,
    #[token("store")]
    Store,
    #[token("embedding")]
    Embedding,

    // Operators
    #[token("=>")]
    FatArrow,
    #[token("_", priority = 10)]
    Wildcard,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token(">=")]
    GreaterEq,

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
        assert_eq!(
            lex.next(),
            Some(Ok(Token::AgentIdent("@agent".to_string())))
        );
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

    #[test]
    fn test_lexer_comments_skipped() {
        let input = "let // this is a comment\n$x";
        let mut lex = Token::lexer(input);
        assert_eq!(lex.next(), Some(Ok(Token::Let)));
        assert_eq!(lex.next(), Some(Ok(Token::VarIdent("$x".to_string()))));
        assert_eq!(lex.next(), None);
    }

    #[test]
    fn test_lexer_string_escapes() {
        let input = r#""hello \"world\"""#;
        let mut lex = Token::lexer(input);
        if let Some(Ok(Token::String(s))) = lex.next() {
            assert!(s.contains("\\\""));
        } else {
            panic!("Expected string token");
        }
    }

    #[test]
    fn test_lexer_dot_token() {
        let input = "bot.cleanup";
        let mut lex = Token::lexer(input);
        assert_eq!(lex.next(), Some(Ok(Token::Ident("bot".to_string()))));
        assert_eq!(lex.next(), Some(Ok(Token::Dot)));
        assert_eq!(lex.next(), Some(Ok(Token::Ident("cleanup".to_string()))));
    }

    // M21-M26: New keyword token tests
    #[test]
    fn test_m21_mcp_tokens() {
        let input = "tool from mcp capabilities operations";
        let mut lex = Token::lexer(input);
        assert_eq!(lex.next(), Some(Ok(Token::Tool)));
        assert_eq!(lex.next(), Some(Ok(Token::From)));
        assert_eq!(lex.next(), Some(Ok(Token::Mcp)));
        assert_eq!(lex.next(), Some(Ok(Token::Capabilities)));
        assert_eq!(lex.next(), Some(Ok(Token::Operations)));
    }

    #[test]
    fn test_m22_orchestration_tokens() {
        let input = "pipeline stage concurrent merge handoff route =>";
        let mut lex = Token::lexer(input);
        assert_eq!(lex.next(), Some(Ok(Token::Pipeline)));
        assert_eq!(lex.next(), Some(Ok(Token::Stage)));
        assert_eq!(lex.next(), Some(Ok(Token::Concurrent)));
        assert_eq!(lex.next(), Some(Ok(Token::Merge)));
        assert_eq!(lex.next(), Some(Ok(Token::Handoff)));
        assert_eq!(lex.next(), Some(Ok(Token::Route)));
        assert_eq!(lex.next(), Some(Ok(Token::FatArrow)));
    }

    #[test]
    fn test_m23_a2a_tokens() {
        let input = "a2a discovery skills endpoint";
        let mut lex = Token::lexer(input);
        assert_eq!(lex.next(), Some(Ok(Token::A2A)));
        assert_eq!(lex.next(), Some(Ok(Token::Discovery)));
        assert_eq!(lex.next(), Some(Ok(Token::Skills)));
        assert_eq!(lex.next(), Some(Ok(Token::Endpoint)));
    }

    #[test]
    fn test_m24_workflow_tokens() {
        let input = "workflow state on requires ensures";
        let mut lex = Token::lexer(input);
        assert_eq!(lex.next(), Some(Ok(Token::Workflow)));
        assert_eq!(lex.next(), Some(Ok(Token::State)));
        assert_eq!(lex.next(), Some(Ok(Token::On)));
        assert_eq!(lex.next(), Some(Ok(Token::Requires)));
        assert_eq!(lex.next(), Some(Ok(Token::Ensures)));
    }

    #[test]
    fn test_m25_model_tokens() {
        let input = "model capability supports uses provider";
        let mut lex = Token::lexer(input);
        assert_eq!(lex.next(), Some(Ok(Token::Model)));
        assert_eq!(lex.next(), Some(Ok(Token::Capability)));
        assert_eq!(lex.next(), Some(Ok(Token::Supports)));
        assert_eq!(lex.next(), Some(Ok(Token::Uses)));
        assert_eq!(lex.next(), Some(Ok(Token::Provider)));
    }

    #[test]
    fn test_m26_memory_tokens() {
        let input = "memory store embedding";
        let mut lex = Token::lexer(input);
        assert_eq!(lex.next(), Some(Ok(Token::Memory)));
        assert_eq!(lex.next(), Some(Ok(Token::Store)));
        assert_eq!(lex.next(), Some(Ok(Token::Embedding)));
    }

    #[test]
    fn test_bracket_and_wildcard_tokens() {
        let input = "[ ] _ >=";
        let mut lex = Token::lexer(input);
        assert_eq!(lex.next(), Some(Ok(Token::LBracket)));
        assert_eq!(lex.next(), Some(Ok(Token::RBracket)));
        assert_eq!(lex.next(), Some(Ok(Token::Wildcard)));
        assert_eq!(lex.next(), Some(Ok(Token::GreaterEq)));
    }
}
