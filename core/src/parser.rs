use crate::lexer::Token;
use crate::ast::{Expr, Statement, Program};
use logos::{Lexer, Logos};

pub struct Parser<'a> {
    lexer: Lexer<'a, Token>,
    current: Option<Result<Token, ()>>,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut lexer = Token::lexer(input);
        let current = lexer.next();
        Self { lexer, current }
    }

    fn advance(&mut self) {
        self.current = self.lexer.next();
    }

    fn expect(&mut self, expected: Token) -> Result<(), String> {
        if let Some(Ok(token)) = &self.current {
            if *token == expected {
                self.advance();
                return Ok(());
            }
        }
        Err(format!("Expected {:?}, found {:?}", expected, self.current))
    }

    pub fn parse_program(&mut self) -> Result<Program, String> {
        let mut statements = Vec::new();
        while self.current.is_some() {
            statements.push(self.parse_statement()?);
        }
        Ok(Program { statements })
    }

    fn parse_statement(&mut self) -> Result<Statement, String> {
        match &self.current {
            Some(Ok(Token::Let)) => {
                self.advance();
                let name = if let Some(Ok(Token::VarIdent(name))) = &self.current {
                    let n = name.clone();
                    self.advance();
                    n
                } else {
                    return Err("Expected variable name after 'let'".to_string());
                };
                self.expect(Token::Assign)?;
                let value = self.parse_expr()?;
                Ok(Statement::Let { name, value })
            }
            Some(Ok(Token::Print)) => {
                self.advance();
                let value = self.parse_expr()?;
                Ok(Statement::Print(value))
            }
            Some(Ok(Token::Think)) => {
                self.advance();
                self.expect(Token::LBrace)?;
                let value = self.parse_expr()?;
                self.expect(Token::RBrace)?;
                Ok(Statement::Think(value))
            }
            Some(Ok(Token::Gate)) => {
                self.advance();
                let prompt = self.parse_expr()?;
                self.expect(Token::LBrace)?;
                let mut body = Vec::new();
                while let Some(Ok(token)) = &self.current {
                    if *token == Token::RBrace { break; }
                    body.push(self.parse_statement()?);
                }
                self.expect(Token::RBrace)?;
                Ok(Statement::Gate { prompt, body })
            }
            Some(Ok(Token::Agent)) => {
                self.advance();
                let name = if let Some(Ok(Token::Ident(name))) = &self.current {
                    let n = name.clone();
                    self.advance();
                    n
                } else {
                    "anonymous".to_string()
                };
                self.expect(Token::LBrace)?;
                let mut body = Vec::new();
                while let Some(Ok(token)) = &self.current {
                    if *token == Token::RBrace { break; }
                    body.push(self.parse_statement()?);
                }
                self.expect(Token::RBrace)?;
                Ok(Statement::AgentBlock { name, body })
            }
            _ => Err(format!("Unexpected token in statement: {:?}", self.current)),
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        let token = self.current.clone();
        match token {
            Some(Ok(Token::String(s))) => {
                self.advance();
                Ok(Expr::String(s))
            }
            Some(Ok(Token::Number(n))) => {
                self.advance();
                Ok(Expr::Number(n))
            }
            Some(Ok(Token::VarIdent(v))) => {
                self.advance();
                Ok(Expr::Var(v))
            }
            Some(Ok(Token::AgentIdent(a))) => {
                self.advance();
                Ok(Expr::Agent(a))
            }
            Some(Ok(Token::Ident(i))) => {
                self.advance();
                Ok(Expr::Var(format!("${}", i)))
            }
            t => Err(format!("Expected expression, found {:?}", t)),
        }
    }
}

#[cfg(test)]
mod parser_tests;
