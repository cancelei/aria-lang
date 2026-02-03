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
                // Check if it's a spawn statement
                if let Some(Ok(Token::VarIdent(name))) = &self.current {
                    let var_name = name.clone();
                    self.advance();
                    self.expect(Token::Assign)?;

                    // Check for spawn keyword
                    if let Some(Ok(Token::Spawn)) = &self.current {
                        return self.parse_spawn(var_name);
                    }

                    let value = self.parse_expr()?;
                    Ok(Statement::Let { name: var_name, value })
                } else {
                    return Err("Expected variable name after 'let'".to_string());
                }
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
                // Check if this is an agent definition or agent block
                self.advance();
                let name = if let Some(Ok(Token::Ident(name))) = &self.current {
                    let n = name.clone();
                    self.advance();
                    n
                } else {
                    "anonymous".to_string()
                };

                self.expect(Token::LBrace)?;

                // Try to parse as AgentDef (with allow/task)
                let mut allow_list = Vec::new();
                let mut tasks = Vec::new();
                let mut body = Vec::new();

                while let Some(Ok(token)) = &self.current {
                    if *token == Token::RBrace { break; }

                    match token {
                        Token::Allow => {
                            self.advance();
                            if let Some(Ok(Token::Ident(tool_name))) = &self.current {
                                allow_list.push(tool_name.clone());
                                self.advance();
                            } else {
                                return Err("Expected tool name after 'allow'".to_string());
                            }
                        }
                        Token::Task => {
                            tasks.push(self.parse_task_def()?);
                        }
                        _ => {
                            body.push(self.parse_statement()?);
                        }
                    }
                }

                self.expect(Token::RBrace)?;

                // If we have allow_list or tasks, return AgentDef, otherwise AgentBlock
                if !allow_list.is_empty() || !tasks.is_empty() {
                    Ok(Statement::AgentDef { name, allow_list, tasks, body })
                } else {
                    Ok(Statement::AgentBlock { name, body })
                }
            }
            Some(Ok(Token::Tool)) => self.parse_tool_def(),
            Some(Ok(Token::Task)) => {
                let task = self.parse_task_def()?;
                Ok(Statement::TaskDef(task))
            }
            Some(Ok(Token::Delegate)) => self.parse_delegate(),
            Some(Ok(Token::Main)) => self.parse_main(),
            Some(Ok(Token::Return)) => {
                self.advance();
                let value = self.parse_expr()?;
                Ok(Statement::Return(value))
            }
            _ => Err(format!("Unexpected token in statement: {:?}", self.current)),
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;

        // Check for function call or member access
        loop {
            match &self.current {
                Some(Ok(Token::LParen)) => {
                    // Parse function call
                    expr = self.parse_call(expr)?;
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
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
                let name = i.clone();
                self.advance();
                Ok(Expr::Var(name))
            }
            t => Err(format!("Expected expression, found {:?}", t)),
        }
    }

    // Step 3: Parse tool definitions
    fn parse_tool_def(&mut self) -> Result<Statement, String> {
        self.advance(); // consume 'tool'

        // Parse tool name
        let name = if let Some(Ok(Token::Ident(n))) = &self.current {
            let name = n.clone();
            self.advance();
            name
        } else {
            return Err("Expected tool name after 'tool'".to_string());
        };

        // Parse parameters
        self.expect(Token::LParen)?;
        let mut params = Vec::new();
        while let Some(Ok(token)) = &self.current {
            if *token == Token::RParen { break; }

            if let Token::Ident(param) = token {
                params.push(param.clone());
                self.advance();

                // Skip optional type annotation (e.g., : string)
                if let Some(Ok(Token::Colon)) = &self.current {
                    self.advance();
                    // Skip the type identifier
                    if let Some(Ok(Token::Ident(_))) = &self.current {
                        self.advance();
                    }
                }

                // Handle comma
                if let Some(Ok(Token::Comma)) = &self.current {
                    self.advance();
                }
            } else {
                break;
            }
        }
        self.expect(Token::RParen)?;

        // Parse body with permission and timeout
        self.expect(Token::LBrace)?;
        let mut permission = None;
        let mut timeout = None;

        while let Some(Ok(token)) = &self.current {
            if *token == Token::RBrace { break; }

            match token {
                Token::Permission => {
                    self.advance();
                    self.expect(Token::Colon)?;
                    if let Some(Ok(Token::String(perm))) = &self.current {
                        permission = Some(perm.clone());
                        self.advance();
                    }
                    // Skip optional comma
                    if let Some(Ok(Token::Comma)) = &self.current {
                        self.advance();
                    }
                }
                Token::Timeout => {
                    self.advance();
                    self.expect(Token::Colon)?;
                    if let Some(Ok(Token::Number(t))) = &self.current {
                        timeout = Some(*t);
                        self.advance();
                    }
                    // Skip optional comma
                    if let Some(Ok(Token::Comma)) = &self.current {
                        self.advance();
                    }
                }
                _ => {
                    return Err(format!("Unexpected token in tool body: {:?}", token));
                }
            }
        }

        self.expect(Token::RBrace)?;

        Ok(Statement::ToolDef {
            name,
            params,
            permission,
            timeout,
        })
    }

    // Step 4: Parse function calls
    fn parse_call(&mut self, callee: Expr) -> Result<Expr, String> {
        self.expect(Token::LParen)?;

        let mut args = Vec::new();
        while let Some(Ok(token)) = &self.current {
            if *token == Token::RParen { break; }

            args.push(self.parse_expr()?);

            // Handle comma
            if let Some(Ok(Token::Comma)) = &self.current {
                self.advance();
            }
        }

        self.expect(Token::RParen)?;

        // Extract the function name from the callee
        let name = match callee {
            Expr::Var(n) => n,
            Expr::MemberAccess { object, member } => {
                // For member access like $bot.cleanup_logs, we need to handle differently
                // For now, let's just concatenate them
                if let Expr::Var(obj) = *object {
                    format!("{}.{}", obj, member)
                } else {
                    return Err("Invalid function call".to_string());
                }
            }
            _ => return Err("Invalid function call".to_string()),
        };

        Ok(Expr::Call { name, args })
    }

    // Parse task definitions (used in Step 5)
    fn parse_task_def(&mut self) -> Result<crate::ast::TaskDef, String> {
        self.advance(); // consume 'task'

        // Parse task name
        let name = if let Some(Ok(Token::Ident(n))) = &self.current {
            let name = n.clone();
            self.advance();
            name
        } else {
            return Err("Expected task name after 'task'".to_string());
        };

        // Parse parameters
        self.expect(Token::LParen)?;
        let mut params = Vec::new();
        while let Some(Ok(token)) = &self.current {
            if *token == Token::RParen { break; }

            if let Token::Ident(param) = token {
                params.push(param.clone());
                self.advance();

                // Skip optional type annotation
                if let Some(Ok(Token::Colon)) = &self.current {
                    self.advance();
                    if let Some(Ok(Token::Ident(_))) = &self.current {
                        self.advance();
                    }
                }

                // Handle comma
                if let Some(Ok(Token::Comma)) = &self.current {
                    self.advance();
                }
            } else {
                break;
            }
        }
        self.expect(Token::RParen)?;

        // Parse body
        self.expect(Token::LBrace)?;
        let mut body = Vec::new();
        while let Some(Ok(token)) = &self.current {
            if *token == Token::RBrace { break; }
            body.push(self.parse_statement()?);
        }
        self.expect(Token::RBrace)?;

        Ok(crate::ast::TaskDef {
            name,
            params,
            body,
        })
    }

    // Step 6: Parse spawn statements
    fn parse_spawn(&mut self, var_name: String) -> Result<Statement, String> {
        self.advance(); // consume 'spawn'

        // Parse agent name
        let agent_name = if let Some(Ok(Token::Ident(name))) = &self.current {
            let n = name.clone();
            self.advance();
            n
        } else {
            return Err("Expected agent name after 'spawn'".to_string());
        };

        Ok(Statement::Spawn {
            var_name,
            agent_name,
        })
    }

    // Step 6: Parse delegate statements
    fn parse_delegate(&mut self) -> Result<Statement, String> {
        self.advance(); // consume 'delegate'

        // Parse the call expression (e.g., bot.cleanup_logs())
        let call = self.parse_delegate_call()?;

        Ok(Statement::Delegate { call })
    }

    fn parse_delegate_call(&mut self) -> Result<Expr, String> {
        // Parse variable (e.g., bot or $bot)
        let var_name = match &self.current {
            Some(Ok(Token::Ident(name))) => {
                let n = name.clone();
                self.advance();
                n
            }
            Some(Ok(Token::VarIdent(name))) => {
                let n = name.clone();
                self.advance();
                n
            }
            _ => return Err("Expected variable name in delegate".to_string()),
        };

        // Expect '.'
        self.expect(Token::Dot)?;

        let member = if let Some(Ok(Token::Ident(m))) = &self.current {
            let member = m.clone();
            self.advance();
            member
        } else {
            return Err("Expected member name after '.'".to_string());
        };

        // Parse the call
        self.expect(Token::LParen)?;
        let mut args = Vec::new();
        while let Some(Ok(token)) = &self.current {
            if *token == Token::RParen { break; }

            args.push(self.parse_expr()?);

            if let Some(Ok(Token::Comma)) = &self.current {
                self.advance();
            }
        }
        self.expect(Token::RParen)?;

        Ok(Expr::Call {
            name: format!("{}.{}", var_name, member),
            args,
        })
    }

    // Step 6: Parse main blocks
    fn parse_main(&mut self) -> Result<Statement, String> {
        self.advance(); // consume 'main'

        self.expect(Token::LBrace)?;
        let mut body = Vec::new();
        while let Some(Ok(token)) = &self.current {
            if *token == Token::RBrace { break; }
            body.push(self.parse_statement()?);
        }
        self.expect(Token::RBrace)?;

        Ok(Statement::Main { body })
    }
}

#[cfg(test)]
mod parser_tests;
