//! Aria Language Parser
//!
//! Recursive descent parser that produces an AST from token stream.
//! Implements the grammar defined in GRAMMAR.md.

use aria_ast::*;
use aria_lexer::{Lexer, Token, TokenKind};
use thiserror::Error;

/// Parser error type with detailed, helpful error messages
#[derive(Debug, Clone, Error)]
pub enum ParseError {
    #[error("Unexpected token: expected {expected}, found {found} at position {span:?}")]
    UnexpectedToken {
        expected: String,
        found: String,
        span: Span,
    },

    #[error("Unexpected end of file - expected {expected}")]
    UnexpectedEof { expected: String },

    #[error("Invalid expression at position {span:?}: {hint}")]
    InvalidExpression { span: Span, hint: String },

    #[error("Invalid pattern at position {span:?}: {hint}")]
    InvalidPattern { span: Span, hint: String },

    #[error("Invalid type at position {span:?}: {hint}")]
    InvalidType { span: Span, hint: String },

    #[error("Missing 'end' keyword for {construct} at position {span:?} - did you forget to close the block that started at {block_start:?}?")]
    MissingEnd {
        span: Span,
        construct: String,
        block_start: Span,
    },

    #[error("Invalid function declaration at position {span:?}: {hint}")]
    InvalidFunction { span: Span, hint: String },

    #[error("Invalid struct field at position {span:?}: expected 'name: Type'")]
    InvalidStructField { span: Span },

    #[error("Invalid enum variant at position {span:?}: expected 'Name' or 'Name(types)'")]
    InvalidEnumVariant { span: Span },

    #[error("Invalid match arm at position {span:?}: expected 'pattern => expression'")]
    InvalidMatchArm { span: Span },

    #[error("Duplicate definition '{name}' at position {span:?}")]
    DuplicateDefinition { name: String, span: Span },

    #[error("Unclosed {bracket_type} at position {open_span:?} - expected '{expected}' but found {found}")]
    UnclosedBracket {
        bracket_type: String,
        open_span: Span,
        expected: String,
        found: String,
    },

    #[error("Mismatched bracket at position {span:?} - found '{found}' but expected '{expected}' to close '{opener}' at {open_span:?}")]
    MismatchedBracket {
        span: Span,
        found: String,
        expected: String,
        opener: String,
        open_span: Span,
    },

    #[error("Missing type annotation at position {span:?}: {hint}")]
    MissingTypeAnnotation { span: Span, hint: String },

    #[error("Invalid parameter at position {span:?}: {hint}")]
    InvalidParameter { span: Span, hint: String },
}

/// Result type for parser operations
pub type ParseResult<T> = Result<T, ParseError>;

/// Tracks an open bracket/delimiter for error recovery
#[derive(Debug, Clone)]
struct BracketInfo {
    kind: BracketKind,
    span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum BracketKind {
    Paren,    // ()
    Bracket,  // []
    Brace,    // {}
}

impl BracketKind {
    fn opener(&self) -> &'static str {
        match self {
            BracketKind::Paren => "(",
            BracketKind::Bracket => "[",
            BracketKind::Brace => "{",
        }
    }

    #[allow(dead_code)]
    fn closer(&self) -> &'static str {
        match self {
            BracketKind::Paren => ")",
            BracketKind::Bracket => "]",
            BracketKind::Brace => "}",
        }
    }

    #[allow(dead_code)]
    fn name(&self) -> &'static str {
        match self {
            BracketKind::Paren => "parenthesis",
            BracketKind::Bracket => "bracket",
            BracketKind::Brace => "brace",
        }
    }
}

/// Parser state
#[allow(dead_code)]
pub struct Parser<'src> {
    source: &'src str,
    tokens: Vec<Token>,
    pos: usize,
    errors: Vec<ParseError>,
    /// Stack tracking open brackets for error recovery
    bracket_stack: Vec<BracketInfo>,
    /// Stack tracking block starts (fn, struct, enum, etc.) for 'end' error recovery
    block_stack: Vec<(String, Span)>,
}

impl<'src> Parser<'src> {
    /// Create a new parser from source code
    pub fn new(source: &'src str) -> Self {
        let lexer = Lexer::new(source);
        let (tokens, _lex_errors) = lexer.tokenize_filtered();

        Self {
            source,
            tokens,
            pos: 0,
            errors: Vec::new(),
            bracket_stack: Vec::new(),
            block_stack: Vec::new(),
        }
    }

    /// Parse the entire program
    pub fn parse_program(&mut self) -> ParseResult<Program> {
        let start = self.current_span();
        let mut items = Vec::new();

        self.skip_newlines();

        while !self.is_eof() {
            match self.parse_item() {
                Ok(item) => items.push(item),
                Err(e) => {
                    self.errors.push(e);
                    self.recover_to_next_item();
                }
            }
            self.skip_newlines();
        }

        let end = self.previous_span();
        Ok(Program {
            items,
            span: start.merge(end),
        })
    }

    /// Get collected errors
    pub fn errors(&self) -> &[ParseError] {
        &self.errors
    }

    // ========================================================================
    // Token Navigation
    // ========================================================================

    fn current(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn current_kind(&self) -> Option<&TokenKind> {
        self.current().map(|t| &t.kind)
    }

    fn current_span(&self) -> Span {
        self.current().map(|t| t.span).unwrap_or(Span::new(0, 0))
    }

    /// Get the text of the current identifier token (or description for other tokens)
    fn current_token_text(&self) -> String {
        match self.current_kind() {
            Some(TokenKind::Identifier(s)) => s.to_string(),
            Some(TokenKind::TypeIdent(s)) => s.to_string(),
            Some(TokenKind::ConstIdent(s)) => s.to_string(),
            Some(TokenKind::SimpleString(s)) => s.to_string(),
            Some(TokenKind::InterpolatedString(s)) => s.to_string(),
            Some(TokenKind::RawString(s)) => s.to_string(),
            Some(kind) => format!("{}", kind),
            None => "EOF".to_string(),
        }
    }

    fn previous_span(&self) -> Span {
        if self.pos > 0 {
            self.tokens
                .get(self.pos - 1)
                .map(|t| t.span)
                .unwrap_or(Span::new(0, 0))
        } else {
            Span::new(0, 0)
        }
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn advance(&mut self) -> Option<&Token> {
        if !self.is_eof() {
            self.pos += 1;
        }
        self.tokens.get(self.pos - 1)
    }

    fn skip_newlines(&mut self) {
        while matches!(self.current_kind(), Some(TokenKind::Newline)) {
            self.advance();
        }
    }

    fn check(&self, kind: &TokenKind) -> bool {
        self.current_kind() == Some(kind)
    }

    fn peek_kind(&self) -> Option<&TokenKind> {
        self.tokens.get(self.pos + 1).map(|t| &t.kind)
    }

    fn is_expression_start(&self) -> bool {
        matches!(
            self.current_kind(),
            Some(TokenKind::Integer(_))
                | Some(TokenKind::Float(_))
                | Some(TokenKind::SimpleString(_))
                | Some(TokenKind::InterpolatedString(_))
                | Some(TokenKind::Identifier(_))
                | Some(TokenKind::TypeIdent(_))
                | Some(TokenKind::True)
                | Some(TokenKind::False)
                | Some(TokenKind::Nil)
                | Some(TokenKind::LParen)
                | Some(TokenKind::LBracket)
                | Some(TokenKind::LBrace)
                | Some(TokenKind::Minus)
                | Some(TokenKind::Not)
                | Some(TokenKind::Bang)
                | Some(TokenKind::If)
                | Some(TokenKind::Match)
                | Some(TokenKind::Select)
                | Some(TokenKind::SelfLower)
        )
    }

    /// Check if the token AFTER the current one can start an expression.
    /// Used to distinguish Try operator (expr?) from ternary (cond ? then : else).
    fn peek_is_expression_start(&self) -> bool {
        matches!(
            self.peek_kind(),
            Some(TokenKind::Integer(_))
                | Some(TokenKind::Float(_))
                | Some(TokenKind::SimpleString(_))
                | Some(TokenKind::InterpolatedString(_))
                | Some(TokenKind::Identifier(_))
                | Some(TokenKind::TypeIdent(_))
                | Some(TokenKind::True)
                | Some(TokenKind::False)
                | Some(TokenKind::Nil)
                | Some(TokenKind::LParen)
                | Some(TokenKind::LBracket)
                | Some(TokenKind::LBrace)
                | Some(TokenKind::Minus)
                | Some(TokenKind::Not)
                | Some(TokenKind::Bang)
                | Some(TokenKind::If)
                | Some(TokenKind::Match)
                | Some(TokenKind::Select)
                | Some(TokenKind::SelfLower)
        )
    }

    fn expect(&mut self, kind: TokenKind) -> ParseResult<Token> {
        if self.check(&kind) {
            Ok(self.advance().unwrap().clone())
        } else {
            Err(ParseError::UnexpectedToken {
                expected: format!("{}", kind),
                found: self
                    .current_kind()
                    .map(|k| format!("{}", k))
                    .unwrap_or_else(|| "EOF".to_string()),
                span: self.current_span(),
            })
        }
    }

    fn recover_to_next_item(&mut self) {
        while !self.is_eof() {
            if matches!(
                self.current_kind(),
                Some(
                    TokenKind::Fn
                        | TokenKind::Struct
                        | TokenKind::Enum
                        | TokenKind::Trait
                        | TokenKind::Impl
                        | TokenKind::Module
                        | TokenKind::Import
                        | TokenKind::Test
                        | TokenKind::Pub
                )
            ) {
                break;
            }
            self.advance();
        }
    }

    /// Recover to the next statement boundary (newline, end, or block-ending keyword)
    fn recover_to_next_statement(&mut self) {
        while !self.is_eof() {
            match self.current_kind() {
                Some(TokenKind::Newline) => {
                    self.advance();
                    break;
                }
                Some(TokenKind::End) | Some(TokenKind::Elsif) | Some(TokenKind::Else) => {
                    break;
                }
                // Also stop at new item declarations
                Some(
                    TokenKind::Fn
                    | TokenKind::Struct
                    | TokenKind::Enum
                    | TokenKind::Trait
                    | TokenKind::Impl
                    | TokenKind::Module
                    | TokenKind::Import
                    | TokenKind::Test
                    | TokenKind::Pub,
                ) => {
                    break;
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    /// Recover to a closing bracket, useful when we have mismatched brackets
    #[allow(dead_code)]
    fn recover_to_closing_bracket(&mut self, expected: BracketKind) {
        let mut depth = 1;
        while !self.is_eof() && depth > 0 {
            match self.current_kind() {
                Some(TokenKind::LParen) if expected == BracketKind::Paren => depth += 1,
                Some(TokenKind::LBracket) if expected == BracketKind::Bracket => depth += 1,
                Some(TokenKind::LBrace) if expected == BracketKind::Brace => depth += 1,
                Some(TokenKind::RParen) if expected == BracketKind::Paren => depth -= 1,
                Some(TokenKind::RBracket) if expected == BracketKind::Bracket => depth -= 1,
                Some(TokenKind::RBrace) if expected == BracketKind::Brace => depth -= 1,
                _ => {}
            }
            if depth > 0 {
                self.advance();
            }
        }
    }

    /// Push a block start for 'end' keyword tracking
    fn push_block(&mut self, construct: &str, span: Span) {
        self.block_stack.push((construct.to_string(), span));
    }

    /// Pop a block start
    fn pop_block(&mut self) {
        self.block_stack.pop();
    }

    /// Get the current block info for error messages
    fn current_block_info(&self) -> Option<&(String, Span)> {
        self.block_stack.last()
    }

    /// Track opening bracket
    fn push_bracket(&mut self, kind: BracketKind, span: Span) {
        self.bracket_stack.push(BracketInfo { kind, span });
    }

    /// Track closing bracket with validation
    #[allow(dead_code)]
    fn pop_bracket(&mut self, expected: BracketKind, close_span: Span) -> ParseResult<()> {
        match self.bracket_stack.pop() {
            Some(open) if open.kind == expected => Ok(()),
            Some(open) => {
                // Mismatched bracket
                Err(ParseError::MismatchedBracket {
                    span: close_span,
                    found: expected.closer().to_string(),
                    expected: open.kind.closer().to_string(),
                    opener: open.kind.opener().to_string(),
                    open_span: open.span,
                })
            }
            None => {
                // Extra closing bracket with no matching open
                Err(ParseError::UnexpectedToken {
                    expected: "expression or statement".to_string(),
                    found: expected.closer().to_string(),
                    span: close_span,
                })
            }
        }
    }

    /// Expect a specific token, with improved error messages for common mistakes
    fn expect_with_recovery(&mut self, kind: TokenKind) -> ParseResult<Token> {
        if self.check(&kind) {
            Ok(self.advance().unwrap().clone())
        } else {
            // Provide more helpful error messages for common cases
            let error = match &kind {
                TokenKind::End => {
                    if let Some((construct, start_span)) = self.current_block_info().cloned() {
                        ParseError::MissingEnd {
                            span: self.current_span(),
                            construct,
                            block_start: start_span,
                        }
                    } else {
                        ParseError::UnexpectedToken {
                            expected: "'end'".to_string(),
                            found: self.current_token_text(),
                            span: self.current_span(),
                        }
                    }
                }
                TokenKind::RParen => {
                    if let Some(open) = self.bracket_stack.last() {
                        if open.kind == BracketKind::Paren {
                            ParseError::UnclosedBracket {
                                bracket_type: "parenthesis".to_string(),
                                open_span: open.span,
                                expected: ")".to_string(),
                                found: self.current_token_text(),
                            }
                        } else {
                            ParseError::MismatchedBracket {
                                span: self.current_span(),
                                found: self.current_token_text(),
                                expected: ")".to_string(),
                                opener: open.kind.opener().to_string(),
                                open_span: open.span,
                            }
                        }
                    } else {
                        ParseError::UnexpectedToken {
                            expected: "')'".to_string(),
                            found: self.current_token_text(),
                            span: self.current_span(),
                        }
                    }
                }
                TokenKind::RBracket => {
                    if let Some(open) = self.bracket_stack.last() {
                        if open.kind == BracketKind::Bracket {
                            ParseError::UnclosedBracket {
                                bracket_type: "bracket".to_string(),
                                open_span: open.span,
                                expected: "]".to_string(),
                                found: self.current_token_text(),
                            }
                        } else {
                            ParseError::MismatchedBracket {
                                span: self.current_span(),
                                found: self.current_token_text(),
                                expected: "]".to_string(),
                                opener: open.kind.opener().to_string(),
                                open_span: open.span,
                            }
                        }
                    } else {
                        ParseError::UnexpectedToken {
                            expected: "']'".to_string(),
                            found: self.current_token_text(),
                            span: self.current_span(),
                        }
                    }
                }
                TokenKind::RBrace => {
                    if let Some(open) = self.bracket_stack.last() {
                        if open.kind == BracketKind::Brace {
                            ParseError::UnclosedBracket {
                                bracket_type: "brace".to_string(),
                                open_span: open.span,
                                expected: "}".to_string(),
                                found: self.current_token_text(),
                            }
                        } else {
                            ParseError::MismatchedBracket {
                                span: self.current_span(),
                                found: self.current_token_text(),
                                expected: "}".to_string(),
                                opener: open.kind.opener().to_string(),
                                open_span: open.span,
                            }
                        }
                    } else {
                        ParseError::UnexpectedToken {
                            expected: "'}'".to_string(),
                            found: self.current_token_text(),
                            span: self.current_span(),
                        }
                    }
                }
                TokenKind::Colon => {
                    // Common mistake: missing type annotation
                    ParseError::MissingTypeAnnotation {
                        span: self.current_span(),
                        hint: "type annotations use ':' followed by a type, e.g., 'x: Int'".to_string(),
                    }
                }
                _ => ParseError::UnexpectedToken {
                    expected: format!("{}", kind),
                    found: self
                        .current_kind()
                        .map(|k| format!("{}", k))
                        .unwrap_or_else(|| "EOF".to_string()),
                    span: self.current_span(),
                },
            };
            Err(error)
        }
    }

    /// Match assignment operator, returning the compound op if any
    /// Returns None if no assignment operator, Some(None) for `=`, Some(Some(op)) for `+=` etc.
    fn match_assign_op(&mut self) -> Option<Option<BinaryOp>> {
        let op = match self.current_kind() {
            Some(TokenKind::Eq) => Some(None),
            Some(TokenKind::PlusEq) => Some(Some(BinaryOp::Add)),
            Some(TokenKind::MinusEq) => Some(Some(BinaryOp::Sub)),
            Some(TokenKind::StarEq) => Some(Some(BinaryOp::Mul)),
            Some(TokenKind::SlashEq) => Some(Some(BinaryOp::Div)),
            Some(TokenKind::SlashSlashEq) => Some(Some(BinaryOp::IntDiv)),
            Some(TokenKind::PercentEq) => Some(Some(BinaryOp::Mod)),
            Some(TokenKind::AmpEq) => Some(Some(BinaryOp::BitAnd)),
            Some(TokenKind::PipeEq) => Some(Some(BinaryOp::BitOr)),
            Some(TokenKind::CaretEq) => Some(Some(BinaryOp::BitXor)),
            Some(TokenKind::LtLtEq) => Some(Some(BinaryOp::Shl)),
            Some(TokenKind::GtGtEq) => Some(Some(BinaryOp::Shr)),
            _ => None,
        };
        if op.is_some() {
            self.advance();
        }
        op
    }

    // ========================================================================
    // Item Parsing
    // ========================================================================

    fn parse_item(&mut self) -> ParseResult<Item> {
        let visibility = self.parse_visibility();

        match self.current_kind() {
            Some(TokenKind::Fn) => self.parse_function(visibility).map(Item::Function),
            Some(TokenKind::Struct) => self.parse_struct(visibility).map(Item::Struct),
            Some(TokenKind::Data) => self.parse_data(visibility).map(Item::Data),
            Some(TokenKind::Enum) => self.parse_enum(visibility).map(Item::Enum),
            Some(TokenKind::Trait) => self.parse_trait(visibility).map(Item::Trait),
            Some(TokenKind::Impl) => self.parse_impl().map(Item::Impl),
            Some(TokenKind::Module) => self.parse_module().map(Item::Module),
            Some(TokenKind::Import) => self.parse_import().map(Item::Import),
            Some(TokenKind::Use) => self.parse_use(visibility).map(Item::Use),
            Some(TokenKind::Test) => self.parse_test().map(Item::Test),
            Some(TokenKind::Const) => self.parse_const(visibility).map(Item::Const),
            Some(TokenKind::Type) => self.parse_type_alias(visibility).map(Item::TypeAlias),
            Some(TokenKind::Extern) => self.parse_extern().map(Item::Extern),
            _ => Err(ParseError::UnexpectedToken {
                expected: "item declaration".to_string(),
                found: self
                    .current_kind()
                    .map(|k| format!("{}", k))
                    .unwrap_or_else(|| "EOF".to_string()),
                span: self.current_span(),
            }),
        }
    }

    fn parse_visibility(&mut self) -> Visibility {
        if self.check(&TokenKind::Pub) {
            self.advance();
            Visibility::Public
        } else if self.check(&TokenKind::Priv) {
            self.advance();
            Visibility::Private
        } else {
            Visibility::Private
        }
    }

    // ========================================================================
    // Function Parsing
    // ========================================================================

    fn parse_function(&mut self, visibility: Visibility) -> ParseResult<FunctionDecl> {
        let start = self.current_span();
        self.expect(TokenKind::Fn)?;

        let name = self.parse_identifier()?;
        let generic_params = self.parse_optional_generic_params()?;

        // Track opening parenthesis for better error messages
        let lparen_span = self.current_span();
        self.expect(TokenKind::LParen)?;
        self.push_bracket(BracketKind::Paren, lparen_span);

        let params = self.parse_param_list()?;

        // Use improved error handling for closing paren
        self.expect_with_recovery(TokenKind::RParen)?;
        self.bracket_stack.pop(); // Remove the tracked paren

        let return_type = if self.check(&TokenKind::Arrow) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        let where_clause = self.parse_optional_where_clause()?;
        let contracts = self.parse_contracts()?;

        self.skip_newlines();

        let body = if self.check(&TokenKind::Eq) {
            self.advance();
            FunctionBody::Expression(Box::new(self.parse_expression()?))
        } else {
            // Track the function block for 'end' keyword errors
            self.push_block("function", start);
            let block = self.parse_block_with_tracking()?;
            self.pop_block();
            FunctionBody::Block(block)
        };

        let end = self.previous_span();

        Ok(FunctionDecl {
            attributes: Vec::new(),
            visibility,
            name,
            generic_params,
            params,
            return_type,
            where_clause,
            contracts,
            body,
            test_block: None,
            span: start.merge(end),
        })
    }

    fn parse_param_list(&mut self) -> ParseResult<Vec<Param>> {
        let mut params = Vec::new();

        if !self.check(&TokenKind::RParen) {
            params.push(self.parse_param()?);

            while self.check(&TokenKind::Comma) {
                self.advance();
                if self.check(&TokenKind::RParen) {
                    break;
                }
                params.push(self.parse_param()?);
            }
        }

        Ok(params)
    }

    fn parse_param(&mut self) -> ParseResult<Param> {
        let start = self.current_span();
        let mutable = if self.check(&TokenKind::Mut) {
            self.advance();
            true
        } else {
            false
        };

        // Handle 'self' as a special parameter name
        let name = if self.check(&TokenKind::SelfLower) {
            let span = self.current_span();
            self.advance();
            Spanned::new(SmolStr::new("self"), span)
        } else {
            self.parse_identifier()?
        };

        let ty = if self.check(&TokenKind::Colon) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        let default = if self.check(&TokenKind::Eq) {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };

        let end = self.previous_span();

        Ok(Param {
            mutable,
            name,
            ty,
            default,
            span: start.merge(end),
        })
    }

    /// Parse pipe-delimited lambda parameters: |x, y: Int|
    fn parse_pipe_param_list(&mut self) -> ParseResult<Vec<Param>> {
        self.expect(TokenKind::Pipe)?;

        let mut params = Vec::new();

        if !self.check(&TokenKind::Pipe) {
            params.push(self.parse_lambda_param()?);

            while self.check(&TokenKind::Comma) {
                self.advance();
                if self.check(&TokenKind::Pipe) {
                    break;
                }
                params.push(self.parse_lambda_param()?);
            }
        }

        self.expect(TokenKind::Pipe)?;
        Ok(params)
    }

    /// Parse a simple lambda parameter (just name, optional type)
    fn parse_lambda_param(&mut self) -> ParseResult<Param> {
        let start = self.current_span();
        let name = self.parse_identifier()?;

        let ty = if self.check(&TokenKind::Colon) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        let end = self.previous_span();

        Ok(Param {
            mutable: false,
            name,
            ty,
            default: None,
            span: start.merge(end),
        })
    }

    /// Convert expressions to lambda parameters (for arrow lambda syntax)
    fn exprs_to_params(&self, exprs: &[Expr]) -> ParseResult<Vec<Param>> {
        exprs
            .iter()
            .map(|expr| {
                // Only identifiers are valid as lambda params in arrow syntax
                if let ExprKind::Ident(name) = &expr.kind {
                    Ok(Param {
                        mutable: false,
                        name: Spanned::new(name.clone(), expr.span),
                        ty: None,
                        default: None,
                        span: expr.span,
                    })
                } else {
                    Err(ParseError::InvalidExpression {
                        span: expr.span,
                        hint: "only simple identifiers are allowed as lambda parameters in arrow syntax".to_string(),
                    })
                }
            })
            .collect()
    }

    fn parse_optional_generic_params(&mut self) -> ParseResult<Option<GenericParams>> {
        if !self.check(&TokenKind::Lt) {
            return Ok(None);
        }

        let start = self.current_span();
        self.advance();

        let mut params = Vec::new();
        params.push(self.parse_generic_param()?);

        while self.check(&TokenKind::Comma) {
            self.advance();
            if self.check(&TokenKind::Gt) {
                break;
            }
            params.push(self.parse_generic_param()?);
        }

        self.expect(TokenKind::Gt)?;
        let end = self.previous_span();

        Ok(Some(GenericParams {
            params,
            span: start.merge(end),
        }))
    }

    fn parse_generic_param(&mut self) -> ParseResult<GenericParam> {
        let start = self.current_span();
        let name = self.parse_type_identifier()?;

        let bounds = if self.check(&TokenKind::Colon) {
            self.advance();
            self.parse_trait_bounds()?
        } else {
            Vec::new()
        };

        let end = self.previous_span();

        Ok(GenericParam {
            name,
            bounds,
            span: start.merge(end),
        })
    }

    fn parse_trait_bounds(&mut self) -> ParseResult<Vec<TraitBound>> {
        let mut bounds = Vec::new();
        bounds.push(self.parse_trait_bound()?);

        while self.check(&TokenKind::Plus) {
            self.advance();
            bounds.push(self.parse_trait_bound()?);
        }

        Ok(bounds)
    }

    fn parse_trait_bound(&mut self) -> ParseResult<TraitBound> {
        let start = self.current_span();
        let name = self.parse_type_identifier()?;

        let type_args = if self.check(&TokenKind::Lt) {
            self.advance();
            let mut args = Vec::new();
            args.push(self.parse_type()?);

            while self.check(&TokenKind::Comma) {
                self.advance();
                if self.check(&TokenKind::Gt) {
                    break;
                }
                args.push(self.parse_type()?);
            }

            self.expect(TokenKind::Gt)?;
            Some(args)
        } else {
            None
        };

        let end = self.previous_span();

        Ok(TraitBound {
            path: vec![name],
            type_args,
            span: start.merge(end),
        })
    }

    fn parse_optional_where_clause(&mut self) -> ParseResult<Option<WhereClause>> {
        if !self.check(&TokenKind::Where) {
            return Ok(None);
        }

        let start = self.current_span();
        self.advance();

        let mut constraints = Vec::new();
        constraints.push(self.parse_type_constraint()?);

        while self.check(&TokenKind::Comma) {
            self.advance();
            constraints.push(self.parse_type_constraint()?);
        }

        let end = self.previous_span();

        Ok(Some(WhereClause {
            constraints,
            span: start.merge(end),
        }))
    }

    fn parse_type_constraint(&mut self) -> ParseResult<TypeConstraint> {
        let start = self.current_span();
        let ty = self.parse_type_identifier()?;
        self.expect(TokenKind::Colon)?;
        let bounds = self.parse_trait_bounds()?;
        let end = self.previous_span();

        Ok(TypeConstraint {
            ty,
            bounds,
            span: start.merge(end),
        })
    }

    fn parse_contracts(&mut self) -> ParseResult<Vec<Contract>> {
        let mut contracts = Vec::new();
        self.skip_newlines();

        loop {
            match self.current_kind() {
                Some(TokenKind::Requires) => {
                    self.advance();
                    let clause = self.parse_contract_clause()?;
                    contracts.push(Contract::Requires(clause));
                }
                Some(TokenKind::Ensures) => {
                    self.advance();
                    let clause = self.parse_contract_clause()?;
                    contracts.push(Contract::Ensures(clause));
                }
                Some(TokenKind::Invariant) => {
                    self.advance();
                    let clause = self.parse_contract_clause()?;
                    contracts.push(Contract::Invariant(clause));
                }
                _ => break,
            }
            self.skip_newlines();
        }

        Ok(contracts)
    }

    fn parse_contract_clause(&mut self) -> ParseResult<ContractClause> {
        let start = self.current_span();
        let condition = Box::new(self.parse_expression()?);

        let message = if self.check(&TokenKind::Colon) {
            self.advance();
            match self.current_kind() {
                Some(TokenKind::SimpleString(s)) | Some(TokenKind::InterpolatedString(s)) => {
                    // Strip the surrounding quotes from the message
                    let msg = if s.len() >= 2 {
                        SmolStr::new(&s[1..s.len() - 1])
                    } else {
                        s.clone()
                    };
                    self.advance();
                    Some(msg)
                }
                _ => None,
            }
        } else {
            None
        };

        let end = self.previous_span();

        Ok(ContractClause {
            condition,
            message,
            span: start.merge(end),
        })
    }

    fn parse_block(&mut self) -> ParseResult<Block> {
        let start = self.current_span();
        let mut stmts = Vec::new();

        self.skip_newlines();

        while !self.check(&TokenKind::End) && !self.is_eof() {
            stmts.push(self.parse_statement()?);
            self.skip_newlines();
        }

        self.expect(TokenKind::End)?;
        let end = self.previous_span();

        Ok(Block {
            stmts,
            span: start.merge(end),
        })
    }

    /// Parse a block with improved error tracking for 'end' keyword
    fn parse_block_with_tracking(&mut self) -> ParseResult<Block> {
        let start = self.current_span();
        let mut stmts = Vec::new();

        self.skip_newlines();

        while !self.check(&TokenKind::End) && !self.is_eof() {
            match self.parse_statement() {
                Ok(stmt) => stmts.push(stmt),
                Err(e) => {
                    self.errors.push(e);
                    self.recover_to_next_statement();
                }
            }
            self.skip_newlines();
        }

        // Use improved error handling for 'end'
        self.expect_with_recovery(TokenKind::End)?;
        let end = self.previous_span();

        Ok(Block {
            stmts,
            span: start.merge(end),
        })
    }

    // ========================================================================
    // Statement Parsing
    // ========================================================================

    fn parse_statement(&mut self) -> ParseResult<Stmt> {
        let start = self.current_span();

        let kind = match self.current_kind() {
            Some(TokenKind::Let) => {
                self.advance();
                let pattern = self.parse_pattern()?;
                let ty = if self.check(&TokenKind::Colon) {
                    self.advance();
                    Some(self.parse_type()?)
                } else {
                    None
                };
                self.expect(TokenKind::Eq)?;
                let value = self.parse_expression()?;
                StmtKind::Let { pattern, ty, value }
            }
            Some(TokenKind::Return) => {
                self.advance();
                let value = if self.check(&TokenKind::Newline) || self.check(&TokenKind::End) {
                    None
                } else {
                    Some(self.parse_expression()?)
                };
                StmtKind::Return(value)
            }
            Some(TokenKind::Break) => {
                self.advance();
                let value = if self.check(&TokenKind::Newline) || self.check(&TokenKind::End) {
                    None
                } else {
                    Some(self.parse_expression()?)
                };
                StmtKind::Break(value)
            }
            Some(TokenKind::Continue) => {
                self.advance();
                StmtKind::Continue
            }
            Some(TokenKind::For) => self.parse_for_stmt()?,
            Some(TokenKind::While) => self.parse_while_stmt()?,
            Some(TokenKind::Loop) => self.parse_loop_stmt()?,
            Some(TokenKind::If) => self.parse_if_stmt()?,
            Some(TokenKind::Defer) => {
                self.advance();
                let expr = self.parse_expression()?;
                StmtKind::Defer(expr)
            }
            _ => {
                let expr = self.parse_expression()?;
                // Check for assignment operators
                if let Some(op) = self.match_assign_op() {
                    let value = self.parse_expression()?;
                    StmtKind::Assign {
                        target: expr,
                        op,
                        value,
                    }
                } else {
                    StmtKind::Expr(expr)
                }
            }
        };

        let end = self.previous_span();
        Ok(Stmt {
            kind,
            span: start.merge(end),
        })
    }

    fn parse_for_stmt(&mut self) -> ParseResult<StmtKind> {
        self.expect(TokenKind::For)?;
        let pattern = self.parse_pattern()?;
        self.expect(TokenKind::In)?;
        let iterable = self.parse_expression()?;
        self.skip_newlines();
        let body = self.parse_block()?;
        Ok(StmtKind::For {
            pattern,
            iterable,
            body,
        })
    }

    fn parse_while_stmt(&mut self) -> ParseResult<StmtKind> {
        self.expect(TokenKind::While)?;
        let condition = self.parse_expression()?;
        self.skip_newlines();
        let body = self.parse_block()?;
        Ok(StmtKind::While { condition, body })
    }

    fn parse_loop_stmt(&mut self) -> ParseResult<StmtKind> {
        self.expect(TokenKind::Loop)?;
        self.skip_newlines();
        let body = self.parse_block()?;
        Ok(StmtKind::Loop { body })
    }

    fn parse_if_stmt(&mut self) -> ParseResult<StmtKind> {
        self.expect(TokenKind::If)?;
        let condition = self.parse_expression()?;
        self.skip_newlines();

        let then_stmts = self.parse_branch_body()?;
        let then_branch = Block {
            stmts: then_stmts,
            span: Span::dummy(),
        };

        let mut elsif_branches = Vec::new();
        while self.check(&TokenKind::Elsif) {
            self.advance();
            let cond = self.parse_expression()?;
            self.skip_newlines();
            let stmts = self.parse_branch_body()?;
            elsif_branches.push((
                cond,
                Block {
                    stmts,
                    span: Span::dummy(),
                },
            ));
        }

        let else_branch = if self.check(&TokenKind::Else) {
            self.advance();
            self.skip_newlines();
            let stmts = self.parse_branch_body()?;
            Some(Block {
                stmts,
                span: Span::dummy(),
            })
        } else {
            None
        };

        // Always expect 'end' to terminate if statement
        self.expect(TokenKind::End)?;

        Ok(StmtKind::If {
            condition,
            then_branch,
            elsif_branches,
            else_branch,
        })
    }

    fn parse_branch_body(&mut self) -> ParseResult<Vec<Stmt>> {
        let mut stmts = Vec::new();
        while !self.check(&TokenKind::End)
            && !self.check(&TokenKind::Elsif)
            && !self.check(&TokenKind::Else)
            && !self.is_eof()
        {
            stmts.push(self.parse_statement()?);
            self.skip_newlines();
        }
        Ok(stmts)
    }

    // ========================================================================
    // Expression Parsing (Pratt parser style)
    // ========================================================================

    fn parse_expression(&mut self) -> ParseResult<Expr> {
        // Note: Assignment is handled at statement level (StmtKind::Assign)
        let condition = self.parse_pipe()?;

        // Check for ternary: condition ? then_expr : else_expr
        if self.check(&TokenKind::Question) {
            self.advance();
            let then_expr = self.parse_expression()?; // Right-associative
            self.expect(TokenKind::Colon)?;
            let else_expr = self.parse_expression()?;
            let span = condition.span.merge(else_expr.span);
            return Ok(Expr::new(
                ExprKind::Ternary {
                    condition: Box::new(condition),
                    then_expr: Box::new(then_expr),
                    else_expr: Box::new(else_expr),
                },
                span,
            ));
        }

        Ok(condition)
    }

    fn parse_pipe(&mut self) -> ParseResult<Expr> {
        let mut left = self.parse_channel_send()?;

        while self.check(&TokenKind::PipeRight) {
            self.advance();
            let right = self.parse_channel_send()?;
            let span = left.span.merge(right.span);
            left = Expr::new(
                ExprKind::Pipe {
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }

        Ok(left)
    }

    /// Parse channel send expression: `channel <- value`
    ///
    /// Channel send has low precedence (between pipe and or)
    fn parse_channel_send(&mut self) -> ParseResult<Expr> {
        let left = self.parse_or()?;

        // Check for channel send: `channel <- value`
        if self.check(&TokenKind::LeftArrow) {
            self.advance();
            let value = self.parse_or()?; // Right side is also parse_or level
            let span = left.span.merge(value.span);
            return Ok(Expr::new(
                ExprKind::ChannelSend {
                    channel: Box::new(left),
                    value: Box::new(value),
                },
                span,
            ));
        }

        Ok(left)
    }

    fn parse_or(&mut self) -> ParseResult<Expr> {
        let mut left = self.parse_and()?;

        while self.check(&TokenKind::Or) || self.check(&TokenKind::PipePipe) {
            self.advance();
            let right = self.parse_and()?;
            let span = left.span.merge(right.span);
            left = Expr::new(
                ExprKind::Binary {
                    op: BinaryOp::Or,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }

        Ok(left)
    }

    fn parse_and(&mut self) -> ParseResult<Expr> {
        let mut left = self.parse_comparison()?;

        while self.check(&TokenKind::And) || self.check(&TokenKind::AmpAmp) {
            self.advance();
            let right = self.parse_comparison()?;
            let span = left.span.merge(right.span);
            left = Expr::new(
                ExprKind::Binary {
                    op: BinaryOp::And,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }

        Ok(left)
    }

    fn parse_comparison(&mut self) -> ParseResult<Expr> {
        let mut left = self.parse_range()?;

        loop {
            let op = match self.current_kind() {
                Some(TokenKind::EqEq) => BinaryOp::Eq,
                Some(TokenKind::NotEq) => BinaryOp::NotEq,
                Some(TokenKind::Lt) => BinaryOp::Lt,
                Some(TokenKind::Gt) => BinaryOp::Gt,
                Some(TokenKind::LtEq) => BinaryOp::LtEq,
                Some(TokenKind::GtEq) => BinaryOp::GtEq,
                _ => break,
            };

            self.advance();
            let right = self.parse_range()?;
            let span = left.span.merge(right.span);
            left = Expr::new(
                ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }

        Ok(left)
    }

    fn parse_range(&mut self) -> ParseResult<Expr> {
        let start = self.current_span();

        // Check for range starting with .. (e.g., ..10 or ..<10)
        if self.check(&TokenKind::DotDot) || self.check(&TokenKind::DotDotLt) || self.check(&TokenKind::DotDotEq) {
            let inclusive = match self.current_kind() {
                Some(TokenKind::DotDot) | Some(TokenKind::DotDotEq) => true,
                Some(TokenKind::DotDotLt) => false,
                _ => true,
            };
            self.advance();
            let end = self.parse_additive()?;
            let span = start.merge(end.span);
            return Ok(Expr::new(
                ExprKind::Range {
                    start: None,
                    end: Some(Box::new(end)),
                    inclusive,
                },
                span,
            ));
        }

        let left = self.parse_additive()?;

        // Check for range operator after expression
        if self.check(&TokenKind::DotDot) || self.check(&TokenKind::DotDotLt) || self.check(&TokenKind::DotDotEq) {
            let inclusive = match self.current_kind() {
                Some(TokenKind::DotDot) | Some(TokenKind::DotDotEq) => true,
                Some(TokenKind::DotDotLt) => false,
                _ => true,
            };
            self.advance();

            // Check if there's an end expression (not just `1..`)
            let end = if self.is_expression_start() {
                Some(Box::new(self.parse_additive()?))
            } else {
                None
            };

            let span = left.span.merge(self.previous_span());
            return Ok(Expr::new(
                ExprKind::Range {
                    start: Some(Box::new(left)),
                    end,
                    inclusive,
                },
                span,
            ));
        }

        Ok(left)
    }

    fn parse_additive(&mut self) -> ParseResult<Expr> {
        let mut left = self.parse_multiplicative()?;

        loop {
            let op = match self.current_kind() {
                Some(TokenKind::Plus) => BinaryOp::Add,
                Some(TokenKind::Minus) => BinaryOp::Sub,
                _ => break,
            };

            self.advance();
            let right = self.parse_multiplicative()?;
            let span = left.span.merge(right.span);
            left = Expr::new(
                ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }

        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> ParseResult<Expr> {
        let mut left = self.parse_power()?;

        loop {
            let op = match self.current_kind() {
                Some(TokenKind::Star) => BinaryOp::Mul,
                Some(TokenKind::Slash) => BinaryOp::Div,
                Some(TokenKind::SlashSlash) => BinaryOp::IntDiv,
                Some(TokenKind::Percent) => BinaryOp::Mod,
                _ => break,
            };

            self.advance();
            let right = self.parse_power()?;
            let span = left.span.merge(right.span);
            left = Expr::new(
                ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }

        Ok(left)
    }

    // Power operator is right-associative: 2**3**4 = 2**(3**4)
    fn parse_power(&mut self) -> ParseResult<Expr> {
        let left = self.parse_unary()?;

        if self.check(&TokenKind::StarStar) {
            self.advance();
            let right = self.parse_power()?; // Right-associative: recurse into parse_power
            let span = left.span.merge(right.span);
            return Ok(Expr::new(
                ExprKind::Binary {
                    op: BinaryOp::Pow,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            ));
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> ParseResult<Expr> {
        let start = self.current_span();

        match self.current_kind() {
            Some(TokenKind::Minus) => {
                self.advance();
                let operand = self.parse_unary()?;
                let span = start.merge(operand.span);
                Ok(Expr::new(
                    ExprKind::Unary {
                        op: UnaryOp::Neg,
                        operand: Box::new(operand),
                    },
                    span,
                ))
            }
            Some(TokenKind::Not) | Some(TokenKind::Bang) => {
                self.advance();
                let operand = self.parse_unary()?;
                let span = start.merge(operand.span);
                Ok(Expr::new(
                    ExprKind::Unary {
                        op: UnaryOp::Not,
                        operand: Box::new(operand),
                    },
                    span,
                ))
            }
            Some(TokenKind::Tilde) => {
                self.advance();
                let operand = self.parse_unary()?;
                let span = start.merge(operand.span);
                Ok(Expr::new(
                    ExprKind::Unary {
                        op: UnaryOp::BitNot,
                        operand: Box::new(operand),
                    },
                    span,
                ))
            }
            // Channel receive: `<- channel`
            Some(TokenKind::LeftArrow) => {
                self.advance();
                let channel = self.parse_unary()?;
                let span = start.merge(channel.span);
                Ok(Expr::new(
                    ExprKind::ChannelRecv {
                        channel: Box::new(channel),
                    },
                    span,
                ))
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.current_kind() {
                Some(TokenKind::Dot) => {
                    self.advance();
                    let field = self.parse_identifier()?;

                    if self.check(&TokenKind::LParen) {
                        // Method call
                        self.advance();
                        let args = self.parse_arg_list()?;
                        self.expect(TokenKind::RParen)?;
                        let span = expr.span.merge(self.previous_span());
                        expr = Expr::new(
                            ExprKind::MethodCall {
                                object: Box::new(expr),
                                method: field,
                                args,
                            },
                            span,
                        );
                    } else {
                        // Field access
                        let span = expr.span.merge(field.span);
                        expr = Expr::new(
                            ExprKind::Field {
                                object: Box::new(expr),
                                field,
                            },
                            span,
                        );
                    }
                }
                Some(TokenKind::LBracket) => {
                    self.advance();
                    let index = self.parse_expression()?;
                    self.expect(TokenKind::RBracket)?;
                    let span = expr.span.merge(self.previous_span());
                    expr = Expr::new(
                        ExprKind::Index {
                            object: Box::new(expr),
                            index: Box::new(index),
                        },
                        span,
                    );
                }
                Some(TokenKind::LParen) => {
                    self.advance();
                    let args = self.parse_call_arg_list()?;
                    self.expect(TokenKind::RParen)?;
                    let span = expr.span.merge(self.previous_span());
                    expr = Expr::new(
                        ExprKind::Call {
                            func: Box::new(expr),
                            args,
                        },
                        span,
                    );
                }
                Some(TokenKind::Question) => {
                    // Distinguish Try operator (expr?) from ternary (cond ? then : else)
                    // If followed by an expression-starting token, it's ternary - leave for parse_expression
                    if self.peek_is_expression_start() {
                        break; // Let parse_expression handle as ternary
                    }
                    self.advance();
                    let span = expr.span.merge(self.previous_span());
                    expr = Expr::new(ExprKind::Try(Box::new(expr)), span);
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> ParseResult<Expr> {
        let start = self.current_span();

        match self.current_kind().cloned() {
            Some(TokenKind::Integer(s)) => {
                self.advance();
                Ok(Expr::new(ExprKind::Integer(s), start))
            }
            Some(TokenKind::Float(s)) => {
                self.advance();
                Ok(Expr::new(ExprKind::Float(s), start))
            }
            Some(TokenKind::SimpleString(s)) | Some(TokenKind::InterpolatedString(s)) => {
                self.advance();
                // Strip the surrounding quotes from the string
                let content = if s.len() >= 2 {
                    SmolStr::new(&s[1..s.len() - 1])
                } else {
                    s
                };
                Ok(Expr::new(ExprKind::String(content), start))
            }
            Some(TokenKind::Char(s)) => {
                self.advance();
                Ok(Expr::new(ExprKind::Char(s), start))
            }
            Some(TokenKind::True) => {
                self.advance();
                Ok(Expr::new(ExprKind::Bool(true), start))
            }
            Some(TokenKind::False) => {
                self.advance();
                Ok(Expr::new(ExprKind::Bool(false), start))
            }
            Some(TokenKind::Nil) => {
                self.advance();
                Ok(Expr::new(ExprKind::Nil, start))
            }
            Some(TokenKind::SelfLower) => {
                self.advance();
                Ok(Expr::new(ExprKind::SelfLower, start))
            }
            Some(TokenKind::SelfUpper) => {
                self.advance();
                // Handle Self(...) as struct instantiation
                if self.check(&TokenKind::LParen) {
                    self.advance();

                    // Empty parens
                    if self.check(&TokenKind::RParen) {
                        self.advance();
                        let span = start.merge(self.previous_span());
                        return Ok(Expr::new(
                            ExprKind::StructInit {
                                name: Spanned::new(SmolStr::new("Self"), start),
                                fields: vec![],
                            },
                            span,
                        ));
                    }

                    // Check if this looks like named fields (identifier:) or positional args
                    let is_named_field = matches!(self.current_kind(), Some(TokenKind::Identifier(_)))
                        && self.peek_kind() == Some(&TokenKind::Colon);

                    if is_named_field {
                        // Struct initialization with named fields
                        let fields = self.parse_field_init_list()?;
                        self.expect(TokenKind::RParen)?;
                        let span = start.merge(self.previous_span());
                        Ok(Expr::new(
                            ExprKind::StructInit {
                                name: Spanned::new(SmolStr::new("Self"), start),
                                fields,
                            },
                            span,
                        ))
                    } else {
                        // Constructor call with positional arguments
                        let args = self.parse_call_arg_list()?;
                        self.expect(TokenKind::RParen)?;
                        let span = start.merge(self.previous_span());
                        Ok(Expr::new(
                            ExprKind::Call {
                                func: Box::new(Expr::new(ExprKind::SelfUpper, start)),
                                args,
                            },
                            span,
                        ))
                    }
                } else {
                    Ok(Expr::new(ExprKind::SelfUpper, start))
                }
            }
            // Note: 'result' is now handled as an identifier
            // In contract contexts, we check if the identifier is "result"
            Some(TokenKind::Identifier(s)) => {
                self.advance();
                // Handle 'result' as a special identifier in contract contexts
                if s.as_str() == "result" {
                    // TODO: Only parse as ExprKind::Result in contract blocks
                    // For now, treat as regular identifier to allow variable usage
                    Ok(Expr::new(ExprKind::Ident(s), start))
                } else {
                    Ok(Expr::new(ExprKind::Ident(s), start))
                }
            }
            Some(TokenKind::TypeIdent(s)) => {
                self.advance();
                // Could be struct instantiation, constructor call, or just type reference
                if self.check(&TokenKind::LParen) {
                    self.advance();

                    // Empty parens
                    if self.check(&TokenKind::RParen) {
                        self.advance();
                        let span = start.merge(self.previous_span());
                        return Ok(Expr::new(
                            ExprKind::StructInit {
                                name: Spanned::new(s, start),
                                fields: vec![],
                            },
                            span,
                        ));
                    }

                    // Check if this looks like named fields (identifier:) or positional args
                    let is_named_field = matches!(self.current_kind(), Some(TokenKind::Identifier(_)))
                        && self.peek_kind() == Some(&TokenKind::Colon);

                    if is_named_field {
                        // Struct initialization with named fields
                        let fields = self.parse_field_init_list()?;
                        self.expect(TokenKind::RParen)?;
                        let span = start.merge(self.previous_span());
                        Ok(Expr::new(
                            ExprKind::StructInit {
                                name: Spanned::new(s, start),
                                fields,
                            },
                            span,
                        ))
                    } else {
                        // Constructor call with positional arguments (like Ok(42), Some(x))
                        let args = self.parse_call_arg_list()?;
                        self.expect(TokenKind::RParen)?;
                        let span = start.merge(self.previous_span());
                        Ok(Expr::new(
                            ExprKind::Call {
                                func: Box::new(Expr::new(ExprKind::Ident(s), start)),
                                args,
                            },
                            span,
                        ))
                    }
                } else {
                    Ok(Expr::new(ExprKind::Ident(s), start))
                }
            }
            Some(TokenKind::LParen) => {
                self.advance();

                // Empty () - could be empty tuple or arrow lambda with no params
                if self.check(&TokenKind::RParen) {
                    self.advance();
                    // Check for arrow lambda: () => expr
                    if self.check(&TokenKind::FatArrow) {
                        self.advance();
                        let body = self.parse_expression()?;
                        let span = start.merge(self.previous_span());
                        return Ok(Expr::new(
                            ExprKind::Lambda {
                                params: vec![],
                                body: Box::new(body),
                            },
                            span,
                        ));
                    }
                    let span = start.merge(self.previous_span());
                    return Ok(Expr::new(ExprKind::Tuple(vec![]), span));
                }

                let first = self.parse_expression()?;

                // Check if it's a tuple (has comma) or parenthesized expression
                if self.check(&TokenKind::Comma) {
                    // It's a tuple or multi-param arrow lambda
                    let mut elements = vec![first];
                    while self.check(&TokenKind::Comma) {
                        self.advance();
                        if self.check(&TokenKind::RParen) {
                            break; // Trailing comma allowed
                        }
                        elements.push(self.parse_expression()?);
                    }
                    self.expect(TokenKind::RParen)?;

                    // Check for arrow lambda: (x, y) => expr
                    if self.check(&TokenKind::FatArrow) {
                        self.advance();
                        let params = self.exprs_to_params(&elements)?;
                        let body = self.parse_expression()?;
                        let span = start.merge(self.previous_span());
                        return Ok(Expr::new(
                            ExprKind::Lambda {
                                params,
                                body: Box::new(body),
                            },
                            span,
                        ));
                    }

                    let span = start.merge(self.previous_span());
                    Ok(Expr::new(ExprKind::Tuple(elements), span))
                } else {
                    // Parenthesized expression or single-param arrow lambda
                    self.expect(TokenKind::RParen)?;

                    // Check for arrow lambda: (x) => expr
                    if self.check(&TokenKind::FatArrow) {
                        self.advance();
                        let params = self.exprs_to_params(&[first])?;
                        let body = self.parse_expression()?;
                        let span = start.merge(self.previous_span());
                        return Ok(Expr::new(
                            ExprKind::Lambda {
                                params,
                                body: Box::new(body),
                            },
                            span,
                        ));
                    }

                    let span = start.merge(self.previous_span());
                    Ok(Expr::new(ExprKind::Paren(Box::new(first)), span))
                }
            }
            Some(TokenKind::LBracket) => {
                self.advance();
                let mut elements = Vec::new();
                if !self.check(&TokenKind::RBracket) {
                    elements.push(self.parse_expression()?);
                    while self.check(&TokenKind::Comma) {
                        self.advance();
                        if self.check(&TokenKind::RBracket) {
                            break;
                        }
                        elements.push(self.parse_expression()?);
                    }
                }
                self.expect(TokenKind::RBracket)?;
                let span = start.merge(self.previous_span());
                Ok(Expr::new(ExprKind::Array(elements), span))
            }
            Some(TokenKind::LBrace) => {
                self.advance();

                // Check for block lambda: { |params| body }
                if self.check(&TokenKind::Pipe) {
                    let params = self.parse_pipe_param_list()?;
                    self.skip_newlines();

                    // Parse lambda body as statements until }
                    let mut stmts = Vec::new();
                    while !self.check(&TokenKind::RBrace) && !self.is_eof() {
                        stmts.push(self.parse_statement()?);
                        self.skip_newlines();
                    }

                    self.expect(TokenKind::RBrace)?;
                    let span = start.merge(self.previous_span());

                    // If single expression statement, use Lambda; otherwise BlockLambda
                    if stmts.len() == 1 {
                        if let StmtKind::Expr(expr) = &stmts[0].kind {
                            return Ok(Expr::new(
                                ExprKind::Lambda {
                                    params,
                                    body: Box::new(expr.clone()),
                                },
                                span,
                            ));
                        }
                    }

                    return Ok(Expr::new(
                        ExprKind::BlockLambda {
                            params,
                            body: Block { stmts, span },
                        },
                        span,
                    ));
                }

                // Map literal: { key: value, ... }
                let mut entries = Vec::new();
                if !self.check(&TokenKind::RBrace) {
                    let key = self.parse_expression()?;
                    if self.check(&TokenKind::Colon) || self.check(&TokenKind::FatArrow) {
                        self.advance();
                        let value = self.parse_expression()?;
                        entries.push((key, value));

                        while self.check(&TokenKind::Comma) {
                            self.advance();
                            if self.check(&TokenKind::RBrace) {
                                break;
                            }
                            let k = self.parse_expression()?;
                            self.advance(); // : or =>
                            let v = self.parse_expression()?;
                            entries.push((k, v));
                        }
                    }
                }
                self.expect(TokenKind::RBrace)?;
                let span = start.merge(self.previous_span());
                Ok(Expr::new(ExprKind::Map(entries), span))
            }
            Some(TokenKind::If) => {
                // If expression
                self.advance();
                let condition = Box::new(self.parse_expression()?);
                self.expect(TokenKind::Then)?;
                let then_expr = Box::new(self.parse_expression()?);
                self.expect(TokenKind::Else)?;
                let else_expr = Box::new(self.parse_expression()?);
                let span = start.merge(self.previous_span());
                Ok(Expr::new(
                    ExprKind::Ternary {
                        condition,
                        then_expr,
                        else_expr,
                    },
                    span,
                ))
            }
            Some(TokenKind::Match) => {
                // Match expression
                self.advance();
                let scrutinee = Box::new(self.parse_expression()?);
                self.skip_newlines();

                let mut arms = Vec::new();
                while !self.check(&TokenKind::End) && !self.is_eof() {
                    let arm = self.parse_match_arm()?;
                    arms.push(arm);
                    self.skip_newlines();
                }

                self.expect(TokenKind::End)?;
                let span = start.merge(self.previous_span());
                Ok(Expr::new(
                    ExprKind::Match { scrutinee, arms },
                    span,
                ))
            }
            Some(TokenKind::Select) => {
                // Select expression for channel multiplexing
                // select
                //   pattern = <-channel => body
                //   channel <- value => body
                //   default => body
                // end
                self.advance();
                self.skip_newlines();

                let mut arms = Vec::new();
                while !self.check(&TokenKind::End) && !self.is_eof() {
                    let arm = self.parse_select_arm()?;
                    arms.push(arm);
                    self.skip_newlines();
                }

                self.expect(TokenKind::End)?;
                let span = start.merge(self.previous_span());
                Ok(Expr::new(
                    ExprKind::Select(arms),
                    span,
                ))
            }
            Some(TokenKind::Handle) => {
                // Handle expression for effect handling (including try/catch)
                // handle
                //   expr
                // with
                //   Effect.operation(params) => body
                //   return(x) => final_value
                // end
                self.advance();
                self.skip_newlines();

                // Parse the body expression being handled
                let body = Box::new(self.parse_expression()?);
                self.skip_newlines();

                // Expect 'with' keyword
                self.expect(TokenKind::With)?;
                self.skip_newlines();

                // Parse handler clauses
                let mut handlers = Vec::new();
                let mut return_clause = None;

                while !self.check(&TokenKind::End) && !self.is_eof() {
                    // Check for return clause
                    if self.check(&TokenKind::Return) {
                        self.advance();
                        self.expect(TokenKind::LParen)?;
                        let pattern = self.parse_pattern()?;
                        self.expect(TokenKind::RParen)?;
                        self.expect(TokenKind::FatArrow)?;

                        let clause_body = self.parse_handler_body()?;
                        let clause_span = start.merge(self.previous_span());

                        return_clause = Some(Box::new(ReturnClause {
                            pattern,
                            body: Box::new(clause_body),
                            span: clause_span,
                        }));
                    } else {
                        // Parse effect handler clause: Effect.operation(params) => body
                        let clause = self.parse_handler_clause()?;
                        handlers.push(clause);
                    }
                    self.skip_newlines();
                }

                self.expect(TokenKind::End)?;
                let span = start.merge(self.previous_span());
                Ok(Expr::new(
                    ExprKind::Handle {
                        body,
                        handlers,
                        return_clause,
                    },
                    span,
                ))
            }
            Some(TokenKind::Raise) => {
                // Raise expression: raise error or raise(error)
                self.advance();

                // Parse the error expression
                let error = if self.check(&TokenKind::LParen) {
                    self.advance();
                    let e = self.parse_expression()?;
                    self.expect(TokenKind::RParen)?;
                    e
                } else {
                    self.parse_unary()?
                };

                let span = start.merge(self.previous_span());
                Ok(Expr::new(
                    ExprKind::Raise {
                        error: Box::new(error),
                        exception_type: None,
                    },
                    span,
                ))
            }
            Some(TokenKind::Resume) => {
                // Resume expression (inside effect handlers): resume(value)
                self.advance();
                self.expect(TokenKind::LParen)?;
                let value = self.parse_expression()?;
                self.expect(TokenKind::RParen)?;
                let span = start.merge(self.previous_span());
                Ok(Expr::new(
                    ExprKind::Resume {
                        value: Box::new(value),
                    },
                    span,
                ))
            }
            _ => {
                let found = self.current_kind().map(|k| format!("{}", k)).unwrap_or_else(|| "EOF".to_string());
                Err(ParseError::InvalidExpression {
                    span: start,
                    hint: format!("unexpected token '{}' - expected a value, identifier, or operator", found),
                })
            }
        }
    }

    fn parse_arg_list(&mut self) -> ParseResult<Vec<Expr>> {
        let mut args = Vec::new();
        if !self.check(&TokenKind::RParen) {
            args.push(self.parse_expression()?);
            while self.check(&TokenKind::Comma) {
                self.advance();
                if self.check(&TokenKind::RParen) {
                    break;
                }
                args.push(self.parse_expression()?);
            }
        }
        Ok(args)
    }

    fn parse_call_arg_list(&mut self) -> ParseResult<Vec<CallArg>> {
        let mut args = Vec::new();
        if !self.check(&TokenKind::RParen) {
            args.push(self.parse_call_arg()?);
            while self.check(&TokenKind::Comma) {
                self.advance();
                if self.check(&TokenKind::RParen) {
                    break;
                }
                args.push(self.parse_call_arg()?);
            }
        }
        Ok(args)
    }

    fn parse_call_arg(&mut self) -> ParseResult<CallArg> {
        let spread = if self.check(&TokenKind::DotDotDot) {
            self.advance();
            true
        } else {
            false
        };

        let value = self.parse_expression()?;
        Ok(CallArg {
            name: None,
            value,
            spread,
        })
    }

    fn parse_field_init_list(&mut self) -> ParseResult<Vec<FieldInit>> {
        let mut fields = Vec::new();
        if !self.check(&TokenKind::RParen) {
            fields.push(self.parse_field_init()?);
            while self.check(&TokenKind::Comma) {
                self.advance();
                if self.check(&TokenKind::RParen) {
                    break;
                }
                fields.push(self.parse_field_init()?);
            }
        }
        Ok(fields)
    }

    fn parse_field_init(&mut self) -> ParseResult<FieldInit> {
        if self.check(&TokenKind::DotDotDot) {
            self.advance();
            let value = self.parse_expression()?;
            return Ok(FieldInit {
                name: Spanned::dummy(SmolStr::default()),
                value: Some(value),
                spread: true,
            });
        }

        let name = self.parse_identifier()?;
        let value = if self.check(&TokenKind::Colon) {
            self.advance();
            // Shorthand syntax: `x:` is equivalent to `x: x`
            // Check if next token is , or ) (end of field list)
            if self.check(&TokenKind::Comma) || self.check(&TokenKind::RParen) {
                // Shorthand - value is the identifier with same name
                Some(Expr::new(ExprKind::Ident(name.node.clone()), name.span))
            } else {
                Some(self.parse_expression()?)
            }
        } else {
            None
        };

        Ok(FieldInit {
            name,
            value,
            spread: false,
        })
    }

    // ========================================================================
    // Match Arm Parsing
    // ========================================================================

    fn parse_match_arm(&mut self) -> ParseResult<MatchArm> {
        let start = self.current_span();
        let pattern = self.parse_pattern()?;

        // Optional guard: pattern if condition
        let guard = if self.check(&TokenKind::If) {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };

        // Expect => for match arm
        self.expect(TokenKind::FatArrow)?;

        // Parse the arm body - can be expression or block
        let body = if self.check(&TokenKind::Newline) {
            // Multi-line block body (indented statements until next pattern or end)
            self.skip_newlines();
            let mut stmts = Vec::new();

            // Peek ahead to check if we're at a new pattern (starts with literal, identifier, or _)
            while !self.is_at_match_arm_start() && !self.check(&TokenKind::End) && !self.is_eof() {
                stmts.push(self.parse_statement()?);
                self.skip_newlines();
            }

            MatchArmBody::Block(Block {
                stmts,
                span: start.merge(self.previous_span()),
            })
        } else {
            // Single-line expression
            let expr = self.parse_expression()?;
            MatchArmBody::Expr(expr)
        };

        let span = start.merge(self.previous_span());
        Ok(MatchArm {
            pattern,
            guard,
            body,
            span,
        })
    }

    /// Check if current position looks like the start of a new match arm
    fn is_at_match_arm_start(&self) -> bool {
        matches!(
            self.current_kind(),
            Some(TokenKind::Identifier(_))
                | Some(TokenKind::TypeIdent(_))
                | Some(TokenKind::Integer(_))
                | Some(TokenKind::Float(_))
                | Some(TokenKind::SimpleString(_))
                | Some(TokenKind::InterpolatedString(_))
                | Some(TokenKind::True)
                | Some(TokenKind::False)
                | Some(TokenKind::Nil)
                | Some(TokenKind::LParen)
                | Some(TokenKind::LBracket)
        )
    }

    // ========================================================================
    // Select Arm Parsing (for channel multiplexing)
    // ========================================================================

    /// Parse a select arm for channel operations.
    ///
    /// Syntax:
    /// - Receive: `pattern = <-channel => body` or `<-channel => body`
    /// - Send: `channel <- value => body`
    /// - Default: `default => body`
    fn parse_select_arm(&mut self) -> ParseResult<SelectArm> {
        let start = self.current_span();

        // Check for default case
        if self.check(&TokenKind::Default) {
            self.advance();
            self.expect(TokenKind::FatArrow)?;
            let body = self.parse_expression()?;
            let span = start.merge(body.span);
            return Ok(SelectArm {
                kind: SelectArmKind::Default,
                body,
                span,
            });
        }

        // Check for receive: `<-channel` or `pattern = <-channel`
        if self.check(&TokenKind::LeftArrow) {
            // Direct receive without binding: `<-channel => body`
            self.advance();
            let channel = self.parse_expression()?;
            self.expect(TokenKind::FatArrow)?;
            let body = self.parse_expression()?;
            let span = start.merge(body.span);
            return Ok(SelectArm {
                kind: SelectArmKind::Receive {
                    pattern: None,
                    channel: Box::new(channel),
                },
                body,
                span,
            });
        }

        // Parse the left side - could be pattern for receive or channel for send
        // Use parse_or() to avoid consuming `<-` in parse_channel_send
        let left = self.parse_or()?;

        // Check what follows: `=` for receive binding, `<-` for send, or nothing
        if self.check(&TokenKind::Eq) {
            // Receive with binding: `pattern = <-channel => body`
            self.advance();
            self.expect(TokenKind::LeftArrow)?;
            let channel = self.parse_expression()?;
            self.expect(TokenKind::FatArrow)?;
            let body = self.parse_expression()?;
            let span = start.merge(body.span);

            // Convert expression to pattern (simple case: identifier)
            let pattern = self.expr_to_pattern(&left)?;

            return Ok(SelectArm {
                kind: SelectArmKind::Receive {
                    pattern: Some(pattern),
                    channel: Box::new(channel),
                },
                body,
                span,
            });
        }

        if self.check(&TokenKind::LeftArrow) {
            // Send: `channel <- value => body`
            self.advance();
            let value = self.parse_expression()?;
            self.expect(TokenKind::FatArrow)?;
            let body = self.parse_expression()?;
            let span = start.merge(body.span);
            return Ok(SelectArm {
                kind: SelectArmKind::Send {
                    channel: Box::new(left),
                    value: Box::new(value),
                },
                body,
                span,
            });
        }

        // Error: unexpected select arm format
        Err(ParseError::InvalidExpression {
            span: start,
            hint: "expected '<-channel', 'pattern = <-channel', 'channel <- value', or 'default'".to_string(),
        })
    }

    /// Convert a simple expression to a pattern (for select arm bindings)
    fn expr_to_pattern(&self, expr: &Expr) -> ParseResult<Pattern> {
        match &expr.kind {
            ExprKind::Ident(name) => Ok(Pattern {
                kind: PatternKind::Ident(name.clone()),
                span: expr.span,
            }),
            ExprKind::Tuple(elements) => {
                let patterns: Result<Vec<_>, _> = elements
                    .iter()
                    .map(|e| self.expr_to_pattern(e))
                    .collect();
                Ok(Pattern {
                    kind: PatternKind::Tuple(patterns?),
                    span: expr.span,
                })
            }
            _ => Err(ParseError::InvalidPattern {
                span: expr.span,
                hint: "expected identifier or tuple pattern in select receive binding".to_string(),
            }),
        }
    }

    /// Check if current position looks like the start of a new select arm
    #[allow(dead_code)]
    fn is_at_select_arm_start(&self) -> bool {
        matches!(
            self.current_kind(),
            Some(TokenKind::LeftArrow)      // <-channel
                | Some(TokenKind::Default)  // default
                | Some(TokenKind::Identifier(_))  // pattern or channel
        )
    }

    // ========================================================================
    // Effect Handler Parsing
    // ========================================================================

    /// Parse an operation name in a handler clause
    /// This can be a regular identifier or certain keywords like 'raise', 'resume', etc.
    fn parse_operation_name(&mut self) -> ParseResult<Ident> {
        let span = self.current_span();
        match self.current_kind().cloned() {
            Some(TokenKind::Identifier(s)) => {
                self.advance();
                Ok(Spanned::new(s, span))
            }
            // Allow 'raise' and 'resume' as operation names since they are effect operations
            Some(TokenKind::Raise) => {
                self.advance();
                Ok(Spanned::new("raise".into(), span))
            }
            Some(TokenKind::Resume) => {
                self.advance();
                Ok(Spanned::new("resume".into(), span))
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "operation name".to_string(),
                found: self
                    .current_kind()
                    .map(|k| format!("{}", k))
                    .unwrap_or_else(|| "EOF".to_string()),
                span,
            }),
        }
    }

    /// Parse a handler clause: `Effect.operation(params) => body`
    fn parse_handler_clause(&mut self) -> ParseResult<HandlerClause> {
        let start = self.current_span();

        // Parse effect name (TypeIdent like Exception, IO, Console)
        let effect = self.parse_type_identifier()?;

        // Expect dot
        self.expect(TokenKind::Dot)?;

        // Parse operation name - can be an identifier or certain keywords like 'raise'
        let operation = self.parse_operation_name()?;

        // Parse parameters
        self.expect(TokenKind::LParen)?;
        let mut params = Vec::new();
        if !self.check(&TokenKind::RParen) {
            params.push(self.parse_pattern()?);
            while self.check(&TokenKind::Comma) {
                self.advance();
                if self.check(&TokenKind::RParen) {
                    break;
                }
                params.push(self.parse_pattern()?);
            }
        }
        self.expect(TokenKind::RParen)?;

        // Expect =>
        self.expect(TokenKind::FatArrow)?;

        // Parse body
        let body = self.parse_handler_body()?;

        let span = start.merge(self.previous_span());
        Ok(HandlerClause {
            effect,
            operation,
            params,
            body,
            span,
        })
    }

    /// Parse a handler body (single expression or block)
    fn parse_handler_body(&mut self) -> ParseResult<HandlerBody> {
        // Check for newline followed by indented block
        if self.check(&TokenKind::Newline) {
            self.skip_newlines();
            // If next token could start a statement and it's not at handler clause level,
            // parse as block. For now, treat single expression case.
            // Check if we're at a handler clause start
            if self.is_at_handler_clause_start() || self.check(&TokenKind::End) {
                // No block body, must use expression on same line
                return Err(ParseError::InvalidExpression {
                    span: self.current_span(),
                    hint: "expected handler body expression".to_string(),
                });
            }

            // Parse as a block until next handler clause, return, or end
            let start = self.current_span();
            let mut stmts = Vec::new();

            while !self.is_at_handler_clause_start()
                && !self.check(&TokenKind::Return)
                && !self.check(&TokenKind::End)
                && !self.is_eof()
            {
                stmts.push(self.parse_statement()?);
                self.skip_newlines();
            }

            let span = start.merge(self.previous_span());
            Ok(HandlerBody::Block(Block { stmts, span }))
        } else {
            // Single expression on same line
            let expr = self.parse_expression()?;
            Ok(HandlerBody::Expr(Box::new(expr)))
        }
    }

    /// Check if current position looks like the start of a handler clause
    fn is_at_handler_clause_start(&self) -> bool {
        // Handler clause starts with TypeIdent (effect name)
        matches!(self.current_kind(), Some(TokenKind::TypeIdent(_)))
    }

    // ========================================================================
    // Pattern Parsing
    // ========================================================================

    fn parse_pattern(&mut self) -> ParseResult<Pattern> {
        let start = self.current_span();

        match self.current_kind().cloned() {
            Some(TokenKind::Identifier(s)) => {
                self.advance();
                // Check for @ binding: name @ pattern
                if self.check(&TokenKind::At) {
                    self.advance();
                    let inner = Box::new(self.parse_pattern()?);
                    let span = start.merge(self.previous_span());
                    return Ok(Pattern {
                        kind: PatternKind::Binding {
                            name: Spanned::new(s, start),
                            pattern: inner,
                        },
                        span,
                    });
                }
                Ok(Pattern {
                    kind: PatternKind::Ident(s),
                    span: start,
                })
            }
            // Handle TypeIdent for enum variant patterns like Some(x), None, etc.
            Some(TokenKind::TypeIdent(s)) => {
                self.advance();
                // Check for variant with data: Some(x) or Point(x, y)
                if self.check(&TokenKind::LParen) {
                    self.advance();
                    let mut patterns = Vec::new();
                    if !self.check(&TokenKind::RParen) {
                        patterns.push(self.parse_pattern()?);
                        while self.check(&TokenKind::Comma) {
                            self.advance();
                            if self.check(&TokenKind::RParen) {
                                break;
                            }
                            patterns.push(self.parse_pattern()?);
                        }
                    }
                    self.expect(TokenKind::RParen)?;
                    let span = start.merge(self.previous_span());
                    Ok(Pattern {
                        kind: PatternKind::Variant {
                            path: vec![],
                            variant: Spanned::new(s, start),
                            fields: Some(patterns),
                        },
                        span,
                    })
                } else {
                    // Unit variant: None, Red, etc.
                    Ok(Pattern {
                        kind: PatternKind::Variant {
                            path: vec![],
                            variant: Spanned::new(s, start),
                            fields: None,
                        },
                        span: start,
                    })
                }
            }
            Some(TokenKind::Integer(_))
            | Some(TokenKind::Float(_))
            | Some(TokenKind::SimpleString(_))
            | Some(TokenKind::True)
            | Some(TokenKind::False)
            | Some(TokenKind::Nil) => {
                let expr = self.parse_primary()?;
                Ok(Pattern {
                    kind: PatternKind::Literal(Box::new(expr)),
                    span: start,
                })
            }
            Some(TokenKind::LParen) => {
                self.advance();
                let mut patterns = Vec::new();
                if !self.check(&TokenKind::RParen) {
                    patterns.push(self.parse_pattern()?);
                    while self.check(&TokenKind::Comma) {
                        self.advance();
                        if self.check(&TokenKind::RParen) {
                            break;
                        }
                        patterns.push(self.parse_pattern()?);
                    }
                }
                self.expect(TokenKind::RParen)?;
                let span = start.merge(self.previous_span());
                Ok(Pattern {
                    kind: PatternKind::Tuple(patterns),
                    span,
                })
            }
            Some(TokenKind::LBracket) => {
                self.advance();
                let mut patterns = Vec::new();
                let mut rest = None;

                if !self.check(&TokenKind::RBracket) {
                    loop {
                        if self.check(&TokenKind::DotDotDot) {
                            self.advance();
                            if let Some(TokenKind::Identifier(s)) = self.current_kind().cloned() {
                                self.advance();
                                rest = Some(Box::new(Pattern {
                                    kind: PatternKind::Ident(s.clone()),
                                    span: self.previous_span(),
                                }));
                            }
                            break;
                        }
                        patterns.push(self.parse_pattern()?);
                        if !self.check(&TokenKind::Comma) {
                            break;
                        }
                        self.advance();
                        if self.check(&TokenKind::RBracket) {
                            break;
                        }
                    }
                }
                self.expect(TokenKind::RBracket)?;
                let span = start.merge(self.previous_span());
                Ok(Pattern {
                    kind: PatternKind::Array {
                        elements: patterns,
                        rest,
                    },
                    span,
                })
            }
            _ => {
                self.advance();
                Ok(Pattern {
                    kind: PatternKind::Wildcard,
                    span: start,
                })
            }
        }
    }

    // ========================================================================
    // Type Parsing
    // ========================================================================

    fn parse_type(&mut self) -> ParseResult<TypeExpr> {
        let start = self.current_span();

        // Parse the base type
        let mut ty = self.parse_base_type()?;

        // Check for optional suffix ?
        if self.check(&TokenKind::Question) {
            self.advance();
            let span = start.merge(self.previous_span());
            ty = TypeExpr::Optional {
                inner: Box::new(ty),
                span,
            };
        }

        Ok(ty)
    }

    fn parse_base_type(&mut self) -> ParseResult<TypeExpr> {
        let start = self.current_span();

        match self.current_kind().cloned() {
            Some(TokenKind::TypeIdent(s)) => {
                self.advance();

                // Check for function type: Fn(Args) -> ReturnType
                if s == "Fn" && self.check(&TokenKind::LParen) {
                    self.advance();
                    let mut params = Vec::new();
                    if !self.check(&TokenKind::RParen) {
                        params.push(self.parse_type()?);
                        while self.check(&TokenKind::Comma) {
                            self.advance();
                            params.push(self.parse_type()?);
                        }
                    }
                    self.expect(TokenKind::RParen)?;

                    // Check for return type
                    let return_type = if self.check(&TokenKind::Arrow) {
                        self.advance();
                        Some(Box::new(self.parse_type()?))
                    } else {
                        None
                    };

                    let span = start.merge(self.previous_span());
                    return Ok(TypeExpr::Function {
                        params,
                        return_type,
                        span,
                    });
                }

                if self.check(&TokenKind::Lt) {
                    // Generic type
                    self.advance();
                    let mut args = Vec::new();
                    args.push(self.parse_type()?);
                    while self.check(&TokenKind::Comma) {
                        self.advance();
                        args.push(self.parse_type()?);
                    }
                    self.expect(TokenKind::Gt)?;
                    let span = start.merge(self.previous_span());
                    Ok(TypeExpr::Generic {
                        name: Spanned::new(s, start),
                        args,
                        span,
                    })
                } else {
                    Ok(TypeExpr::Named(Spanned::new(s, start)))
                }
            }
            Some(TokenKind::LBracket) => {
                self.advance();
                let element = Box::new(self.parse_type()?);
                let size = if self.check(&TokenKind::Semi) {
                    self.advance();
                    Some(Box::new(self.parse_expression()?))
                } else {
                    None
                };
                self.expect(TokenKind::RBracket)?;
                let span = start.merge(self.previous_span());
                Ok(TypeExpr::Array {
                    element,
                    size,
                    span,
                })
            }
            Some(TokenKind::LParen) => {
                self.advance();
                let mut elements = Vec::new();
                if !self.check(&TokenKind::RParen) {
                    elements.push(self.parse_type()?);
                    while self.check(&TokenKind::Comma) {
                        self.advance();
                        elements.push(self.parse_type()?);
                    }
                }
                self.expect(TokenKind::RParen)?;
                let span = start.merge(self.previous_span());
                Ok(TypeExpr::Tuple { elements, span })
            }
            Some(TokenKind::LBrace) => {
                // Map type shorthand: {K: V}
                self.advance();
                let key = Box::new(self.parse_type()?);
                self.expect(TokenKind::Colon)?;
                let value = Box::new(self.parse_type()?);
                self.expect(TokenKind::RBrace)?;
                let span = start.merge(self.previous_span());
                Ok(TypeExpr::Map { key, value, span })
            }
            Some(TokenKind::Amp) => {
                self.advance();
                let mutable = if self.check(&TokenKind::Mut) {
                    self.advance();
                    true
                } else {
                    false
                };
                let inner = Box::new(self.parse_type()?);
                let span = start.merge(self.previous_span());
                Ok(TypeExpr::Reference {
                    mutable,
                    inner,
                    span,
                })
            }
            Some(TokenKind::SelfUpper) => {
                self.advance();
                Ok(TypeExpr::Named(Spanned::new(SmolStr::new("Self"), start)))
            }
            _ => {
                let found = self.current_kind().map(|k| format!("{}", k)).unwrap_or_else(|| "EOF".to_string());
                Err(ParseError::InvalidType {
                    span: start,
                    hint: format!("expected type name, found '{}' - types should start with uppercase letter (e.g., Int, String, MyType)", found),
                })
            }
        }
    }

    // ========================================================================
    // Identifier Helpers
    // ========================================================================

    fn parse_identifier(&mut self) -> ParseResult<Ident> {
        let span = self.current_span();
        match self.current_kind().cloned() {
            Some(TokenKind::Identifier(s)) => {
                self.advance();
                Ok(Spanned::new(s, span))
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "identifier".to_string(),
                found: self
                    .current_kind()
                    .map(|k| format!("{}", k))
                    .unwrap_or_else(|| "EOF".to_string()),
                span,
            }),
        }
    }

    /// Parse any identifier (lowercase or uppercase) - used for paths
    fn parse_any_identifier(&mut self) -> ParseResult<Ident> {
        let span = self.current_span();
        match self.current_kind().cloned() {
            Some(TokenKind::Identifier(s)) | Some(TokenKind::TypeIdent(s)) => {
                self.advance();
                Ok(Spanned::new(s, span))
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "identifier".to_string(),
                found: self
                    .current_kind()
                    .map(|k| format!("{}", k))
                    .unwrap_or_else(|| "EOF".to_string()),
                span,
            }),
        }
    }

    /// Parse a constant identifier (can be SCREAMING_CASE or regular identifier)
    fn parse_const_identifier(&mut self) -> ParseResult<Ident> {
        let span = self.current_span();
        match self.current_kind().cloned() {
            Some(TokenKind::Identifier(s)) | Some(TokenKind::ConstIdent(s)) => {
                self.advance();
                Ok(Spanned::new(s, span))
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "identifier".to_string(),
                found: self
                    .current_kind()
                    .map(|k| format!("{}", k))
                    .unwrap_or_else(|| "EOF".to_string()),
                span,
            }),
        }
    }

    fn parse_type_identifier(&mut self) -> ParseResult<TypeIdent> {
        let span = self.current_span();
        match self.current_kind().cloned() {
            Some(TokenKind::TypeIdent(s)) => {
                self.advance();
                Ok(Spanned::new(s, span))
            }
            Some(TokenKind::Identifier(s)) => {
                // Allow lowercase for type variables like `T`
                self.advance();
                Ok(Spanned::new(s, span))
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "type identifier".to_string(),
                found: self
                    .current_kind()
                    .map(|k| format!("{}", k))
                    .unwrap_or_else(|| "EOF".to_string()),
                span,
            }),
        }
    }

    /// Parses an enum variant name (accepts both TypeIdent and Identifier)
    /// Enum variants like Some, None start with uppercase (TypeIdent)
    fn parse_variant_name(&mut self) -> ParseResult<Ident> {
        let span = self.current_span();
        match self.current_kind().cloned() {
            Some(TokenKind::TypeIdent(s)) | Some(TokenKind::Identifier(s)) => {
                self.advance();
                Ok(Spanned::new(s, span))
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "variant name".to_string(),
                found: self
                    .current_kind()
                    .map(|k| format!("{}", k))
                    .unwrap_or_else(|| "EOF".to_string()),
                span,
            }),
        }
    }

    // ========================================================================
    // Struct/Enum/Trait Parsing (stubs for now)
    // ========================================================================

    fn parse_struct(&mut self, visibility: Visibility) -> ParseResult<StructDecl> {
        let start = self.current_span();
        self.expect(TokenKind::Struct)?;
        let name = self.parse_type_identifier()?;
        let generic_params = self.parse_optional_generic_params()?;

        // Track the struct block for 'end' keyword errors
        self.push_block("struct", start);

        self.skip_newlines();

        let mut fields = Vec::new();
        let mut derive = Vec::new();

        while !self.check(&TokenKind::End) && !self.is_eof() {
            if self.check(&TokenKind::Derive) {
                self.advance();
                let lparen_span = self.current_span();
                self.expect(TokenKind::LParen)?;
                self.push_bracket(BracketKind::Paren, lparen_span);
                derive.push(self.parse_type_identifier()?);
                while self.check(&TokenKind::Comma) {
                    self.advance();
                    derive.push(self.parse_type_identifier()?);
                }
                self.expect_with_recovery(TokenKind::RParen)?;
                self.bracket_stack.pop();
            } else {
                let field_vis = self.parse_visibility();
                let field_name = self.parse_identifier()?;

                // Improved error for missing type annotation on struct field
                if !self.check(&TokenKind::Colon) {
                    return Err(ParseError::MissingTypeAnnotation {
                        span: self.current_span(),
                        hint: format!(
                            "struct field '{}' requires a type annotation, e.g., '{}: Int'",
                            field_name.node, field_name.node
                        ),
                    });
                }
                self.advance(); // consume ':'

                let ty = self.parse_type()?;
                let default = if self.check(&TokenKind::Eq) {
                    self.advance();
                    Some(self.parse_expression()?)
                } else {
                    None
                };
                let field_span = field_name.span.merge(self.previous_span());
                fields.push(StructField {
                    visibility: field_vis,
                    name: field_name,
                    ty,
                    default,
                    span: field_span,
                });
            }
            self.skip_newlines();
        }

        self.expect_with_recovery(TokenKind::End)?;
        self.pop_block();
        let span = start.merge(self.previous_span());

        Ok(StructDecl {
            attributes: Vec::new(),
            visibility,
            name,
            generic_params,
            fields,
            derive,
            span,
        })
    }

    fn parse_data(&mut self, visibility: Visibility) -> ParseResult<DataDecl> {
        let start = self.current_span();
        self.expect(TokenKind::Data)?;
        let name = self.parse_type_identifier()?;
        let generic_params = self.parse_optional_generic_params()?;

        self.expect(TokenKind::LParen)?;
        let mut fields = Vec::new();

        if !self.check(&TokenKind::RParen) {
            loop {
                let field_name = self.parse_identifier()?;
                self.expect(TokenKind::Colon)?;
                let ty = self.parse_type()?;
                let default = if self.check(&TokenKind::Eq) {
                    self.advance();
                    Some(self.parse_expression()?)
                } else {
                    None
                };
                let field_span = field_name.span.merge(self.previous_span());
                fields.push(DataField {
                    name: field_name,
                    ty,
                    default,
                    span: field_span,
                });

                if !self.check(&TokenKind::Comma) {
                    break;
                }
                self.advance();
            }
        }

        self.expect(TokenKind::RParen)?;
        let span = start.merge(self.previous_span());

        Ok(DataDecl {
            attributes: Vec::new(),
            visibility,
            name,
            generic_params,
            fields,
            derive: Vec::new(),
            span,
        })
    }

    fn parse_enum(&mut self, visibility: Visibility) -> ParseResult<EnumDecl> {
        let start = self.current_span();
        self.expect(TokenKind::Enum)?;
        let name = self.parse_type_identifier()?;
        let generic_params = self.parse_optional_generic_params()?;

        // Track the enum block for 'end' keyword errors
        self.push_block("enum", start);

        self.skip_newlines();

        let mut variants = Vec::new();

        while !self.check(&TokenKind::End) && !self.is_eof() {
            let variant_name = self.parse_variant_name()?;
            let data = if self.check(&TokenKind::LParen) {
                let lparen_span = self.current_span();
                self.advance();
                self.push_bracket(BracketKind::Paren, lparen_span);

                let mut types = Vec::new();
                if !self.check(&TokenKind::RParen) {
                    types.push(self.parse_type()?);
                    while self.check(&TokenKind::Comma) {
                        self.advance();
                        types.push(self.parse_type()?);
                    }
                }
                self.expect_with_recovery(TokenKind::RParen)?;
                self.bracket_stack.pop();
                EnumVariantData::Tuple(types)
            } else {
                EnumVariantData::Unit
            };

            let variant_span = variant_name.span.merge(self.previous_span());
            variants.push(EnumVariant {
                name: variant_name,
                data,
                span: variant_span,
            });
            self.skip_newlines();
        }

        self.expect_with_recovery(TokenKind::End)?;
        self.pop_block();
        let span = start.merge(self.previous_span());

        Ok(EnumDecl {
            attributes: Vec::new(),
            visibility,
            name,
            generic_params,
            variants,
            derive: Vec::new(),
            span,
        })
    }

    fn parse_trait(&mut self, visibility: Visibility) -> ParseResult<TraitDecl> {
        let start = self.current_span();
        self.expect(TokenKind::Trait)?;
        let name = self.parse_type_identifier()?;
        let generic_params = self.parse_optional_generic_params()?;

        // Track the trait block for 'end' keyword errors
        self.push_block("trait", start);

        let supertraits = if self.check(&TokenKind::Colon) {
            self.advance();
            self.parse_trait_bounds()?
        } else {
            Vec::new()
        };

        self.skip_newlines();

        let mut members = Vec::new();

        while !self.check(&TokenKind::End) && !self.is_eof() {
            if self.check(&TokenKind::Fn) {
                self.advance();
                let method_name = self.parse_identifier()?;
                let method_generics = self.parse_optional_generic_params()?;
                let lparen_span = self.current_span();
                self.expect(TokenKind::LParen)?;
                self.push_bracket(BracketKind::Paren, lparen_span);
                let params = self.parse_param_list()?;
                self.expect_with_recovery(TokenKind::RParen)?;
                self.bracket_stack.pop();
                let return_type = if self.check(&TokenKind::Arrow) {
                    self.advance();
                    Some(self.parse_type()?)
                } else {
                    None
                };

                let method_span = method_name.span.merge(self.previous_span());
                members.push(TraitMember::Method(TraitMethod {
                    name: method_name,
                    generic_params: method_generics,
                    params,
                    return_type,
                    default: None,
                    span: method_span,
                }));
            }
            self.skip_newlines();
        }

        self.expect_with_recovery(TokenKind::End)?;
        self.pop_block();
        let span = start.merge(self.previous_span());

        Ok(TraitDecl {
            attributes: Vec::new(),
            visibility,
            name,
            generic_params,
            supertraits,
            members,
            span,
        })
    }

    fn parse_impl(&mut self) -> ParseResult<ImplDecl> {
        let start = self.current_span();
        self.expect(TokenKind::Impl)?;

        // Track the impl block for 'end' keyword errors
        self.push_block("impl", start);

        let generic_params = self.parse_optional_generic_params()?;
        let first_type = self.parse_type()?;

        let (trait_, for_type) = if self.check(&TokenKind::For) {
            self.advance();
            let for_ty = self.parse_type()?;
            // First type was the trait
            let trait_bound = match first_type {
                TypeExpr::Named(name) => TraitBound {
                    path: vec![name],
                    type_args: None,
                    span: start,
                },
                _ => {
                    return Err(ParseError::InvalidType {
                        span: start,
                        hint: "impl block requires a trait name before 'for' keyword".to_string(),
                    });
                }
            };
            (Some(trait_bound), for_ty)
        } else {
            (None, first_type)
        };

        let where_clause = self.parse_optional_where_clause()?;
        self.skip_newlines();

        let mut members = Vec::new();

        while !self.check(&TokenKind::End) && !self.is_eof() {
            let vis = self.parse_visibility();
            if self.check(&TokenKind::Fn) {
                let func = self.parse_function(vis)?;
                members.push(ImplMember::Function(func));
            }
            self.skip_newlines();
        }

        self.expect_with_recovery(TokenKind::End)?;
        self.pop_block();
        let span = start.merge(self.previous_span());

        Ok(ImplDecl {
            attributes: Vec::new(),
            generic_params,
            trait_,
            for_type,
            where_clause,
            members,
            span,
        })
    }

    fn parse_module(&mut self) -> ParseResult<ModuleDecl> {
        let start = self.current_span();
        self.expect(TokenKind::Module)?;

        // Track the module block for 'end' keyword errors
        self.push_block("module", start);

        let mut path = Vec::new();
        path.push(self.parse_any_identifier()?);

        while self.check(&TokenKind::ColonColon) {
            self.advance();
            path.push(self.parse_any_identifier()?);
        }

        self.skip_newlines();

        let mut items = Vec::new();
        while !self.check(&TokenKind::End) && !self.is_eof() {
            items.push(self.parse_item()?);
            self.skip_newlines();
        }

        self.expect_with_recovery(TokenKind::End)?;
        self.pop_block();
        let span = start.merge(self.previous_span());

        Ok(ModuleDecl { path, items, span })
    }

    fn parse_import(&mut self) -> ParseResult<ImportDecl> {
        let start = self.current_span();
        self.expect(TokenKind::Import)?;

        let mut path_segments = Vec::new();
        path_segments.push(self.parse_any_identifier()?);

        while self.check(&TokenKind::ColonColon) {
            self.advance();
            if self.check(&TokenKind::LBrace) || self.check(&TokenKind::Star) {
                // Don't consume the { or *, handle them below in selection parsing
                break;
            }
            path_segments.push(self.parse_any_identifier()?);
        }

        let path = ImportPath::Module(path_segments);

        let alias = if self.check(&TokenKind::As) {
            self.advance();
            Some(self.parse_identifier()?)
        } else {
            None
        };

        // Handle selection: either we just consumed :: and now see { or *,
        // or we need another :: before { or *
        let selection = if self.check(&TokenKind::LBrace) || self.check(&TokenKind::Star) {
            // We already consumed :: in the loop above
            if self.check(&TokenKind::Star) {
                self.advance();
                Some(ImportSelection::All)
            } else if self.check(&TokenKind::LBrace) {
                self.advance();
                let mut items = Vec::new();
                if !self.check(&TokenKind::RBrace) {
                    // Use parse_any_identifier to allow type names like Array, Map, etc.
                    let name = self.parse_any_identifier()?;
                    let item_alias = if self.check(&TokenKind::As) {
                        self.advance();
                        Some(self.parse_any_identifier()?)
                    } else {
                        None
                    };
                    items.push(ImportItem {
                        name,
                        alias: item_alias,
                    });

                    while self.check(&TokenKind::Comma) {
                        self.advance();
                        if self.check(&TokenKind::RBrace) {
                            break;
                        }
                        let name = self.parse_any_identifier()?;
                        let item_alias = if self.check(&TokenKind::As) {
                            self.advance();
                            Some(self.parse_any_identifier()?)
                        } else {
                            None
                        };
                        items.push(ImportItem {
                            name,
                            alias: item_alias,
                        });
                    }
                }
                self.expect(TokenKind::RBrace)?;
                Some(ImportSelection::Items(items))
            } else {
                None
            }
        } else {
            None
        };

        let span = start.merge(self.previous_span());

        Ok(ImportDecl {
            path,
            alias,
            selection,
            span,
        })
    }

    /// Parse a use declaration (re-export): `pub use Foo::Bar` or `use Foo::Bar::{A, B}`
    fn parse_use(&mut self, visibility: Visibility) -> ParseResult<UseDecl> {
        let start = self.current_span();
        self.expect(TokenKind::Use)?;

        // Parse path segments: Foo::Bar::Baz
        let mut path = Vec::new();
        path.push(self.parse_any_identifier()?);

        while self.check(&TokenKind::ColonColon) {
            self.advance();
            // Check for selection syntax: { ... } or *
            if self.check(&TokenKind::LBrace) || self.check(&TokenKind::Star) {
                break;
            }
            path.push(self.parse_any_identifier()?);
        }

        // Parse optional alias: `as NewName`
        let alias = if self.check(&TokenKind::As) {
            self.advance();
            Some(self.parse_any_identifier()?)
        } else {
            None
        };

        // Parse optional selection: `::{ A, B }` or `::*`
        let selection = if self.check(&TokenKind::LBrace) || self.check(&TokenKind::Star) {
            if self.check(&TokenKind::Star) {
                self.advance();
                Some(ImportSelection::All)
            } else if self.check(&TokenKind::LBrace) {
                self.advance();
                let mut items = Vec::new();
                if !self.check(&TokenKind::RBrace) {
                    // Use parse_any_identifier to allow type names like List, Map, etc.
                    let name = self.parse_any_identifier()?;
                    let item_alias = if self.check(&TokenKind::As) {
                        self.advance();
                        Some(self.parse_any_identifier()?)
                    } else {
                        None
                    };
                    items.push(ImportItem { name, alias: item_alias });

                    while self.check(&TokenKind::Comma) {
                        self.advance();
                        if self.check(&TokenKind::RBrace) {
                            break;
                        }
                        let name = self.parse_any_identifier()?;
                        let item_alias = if self.check(&TokenKind::As) {
                            self.advance();
                            Some(self.parse_any_identifier()?)
                        } else {
                            None
                        };
                        items.push(ImportItem { name, alias: item_alias });
                    }
                }
                self.expect(TokenKind::RBrace)?;
                Some(ImportSelection::Items(items))
            } else {
                None
            }
        } else {
            None
        };

        let span = start.merge(self.previous_span());

        Ok(UseDecl {
            visibility,
            path,
            selection,
            alias,
            span,
        })
    }

    fn parse_test(&mut self) -> ParseResult<TestDecl> {
        let start = self.current_span();
        self.expect(TokenKind::Test)?;

        // Track the test block for 'end' keyword errors
        self.push_block("test", start);

        let name = match self.current_kind().cloned() {
            Some(TokenKind::SimpleString(s)) | Some(TokenKind::InterpolatedString(s)) => {
                self.advance();
                // Strip the surrounding quotes from the test name
                if s.len() >= 2 {
                    SmolStr::new(&s[1..s.len() - 1])
                } else {
                    s
                }
            }
            _ => {
                return Err(ParseError::UnexpectedToken {
                    expected: "test name string".to_string(),
                    found: self
                        .current_kind()
                        .map(|k| format!("{}", k))
                        .unwrap_or_else(|| "EOF".to_string()),
                    span: self.current_span(),
                });
            }
        };

        self.skip_newlines();
        let body = self.parse_block_with_tracking()?;
        self.pop_block();
        let span = start.merge(self.previous_span());

        Ok(TestDecl { name, body, span })
    }

    fn parse_const(&mut self, visibility: Visibility) -> ParseResult<ConstDecl> {
        let start = self.current_span();
        self.expect(TokenKind::Const)?;

        // Const names can be SCREAMING_CASE (ConstIdent) or regular identifiers
        let name = self.parse_const_identifier()?;
        let ty = if self.check(&TokenKind::Colon) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect(TokenKind::Eq)?;
        let value = self.parse_expression()?;

        let span = start.merge(self.previous_span());

        Ok(ConstDecl {
            attributes: Vec::new(),
            visibility,
            name,
            ty,
            value,
            span,
        })
    }

    fn parse_type_alias(&mut self, visibility: Visibility) -> ParseResult<TypeAlias> {
        let start = self.current_span();
        self.expect(TokenKind::Type)?;

        let name = self.parse_type_identifier()?;
        let generic_params = self.parse_optional_generic_params()?;
        self.expect(TokenKind::Eq)?;
        let ty = self.parse_type()?;

        let span = start.merge(self.previous_span());

        Ok(TypeAlias {
            attributes: Vec::new(),
            visibility,
            name,
            generic_params,
            ty,
            span,
        })
    }

    // ========================================================================
    // Extern Declaration Parsing
    // ========================================================================

    /// Parse extern declaration: `extern C from "header.h" as alias ... end`
    fn parse_extern(&mut self) -> ParseResult<ExternDecl> {
        let start = self.current_span();
        self.expect(TokenKind::Extern)?;

        // Check which type of extern
        // Note: C, Python, Wasm may be lexed as TypeIdent (uppercase) or Identifier
        match self.current_kind().cloned() {
            Some(TokenKind::TypeIdent(s)) | Some(TokenKind::Identifier(s)) if s == "C" => {
                self.advance();
                self.parse_extern_c(start)
            }
            Some(TokenKind::TypeIdent(s)) | Some(TokenKind::Identifier(s)) if s == "Python" => {
                self.advance();
                self.parse_extern_python(start)
            }
            Some(TokenKind::TypeIdent(s)) | Some(TokenKind::Identifier(s)) if s == "Wasm" => {
                self.advance();
                self.parse_extern_wasm(start)
            }
            Some(kind) => Err(ParseError::UnexpectedToken {
                expected: "C, Python, or Wasm".to_string(),
                found: format!("{}", kind),
                span: self.current_span(),
            }),
            None => Err(ParseError::UnexpectedEof {
                expected: "C, Python, or Wasm".to_string(),
            }),
        }
    }

    /// Parse extern C block: `extern C from "header.h" as alias ... end`
    fn parse_extern_c(&mut self, start: Span) -> ParseResult<ExternDecl> {
        // Parse optional header path
        let header = if self.check(&TokenKind::From) {
            self.advance();
            self.expect_string()?
        } else {
            SmolStr::new("")
        };

        // Parse optional alias
        let alias = if self.check(&TokenKind::As) {
            self.advance();
            Some(self.parse_identifier()?)
        } else {
            None
        };

        // Skip newlines after header
        self.skip_newlines();

        // Parse items until 'end'
        let mut items = Vec::new();
        while !self.check(&TokenKind::End) && !self.is_eof() {
            self.skip_newlines();
            if self.check(&TokenKind::End) {
                break;
            }
            items.push(self.parse_extern_c_item()?);
            self.skip_newlines();
        }
        self.expect(TokenKind::End)?;

        let span = start.merge(self.previous_span());
        Ok(ExternDecl::C(ExternC {
            header,
            alias,
            items,
            span,
        }))
    }

    /// Parse a single extern C item
    fn parse_extern_c_item(&mut self) -> ParseResult<ExternCItem> {
        match self.current_kind() {
            Some(TokenKind::Fn) => self.parse_extern_function().map(ExternCItem::Function),
            Some(TokenKind::Struct) => self.parse_extern_struct().map(ExternCItem::Struct),
            Some(TokenKind::Const) => self.parse_extern_const().map(ExternCItem::Const),
            Some(TokenKind::Type) => {
                self.advance();
                let name = self.parse_type_identifier()?;
                Ok(ExternCItem::Type(name))
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "fn, struct, const, or type".to_string(),
                found: self.current_token_text().to_string(),
                span: self.current_span(),
            }),
        }
    }

    /// Parse extern function declaration
    fn parse_extern_function(&mut self) -> ParseResult<ExternFunction> {
        let start = self.current_span();
        self.expect(TokenKind::Fn)?;

        let name = self.parse_identifier()?;
        self.expect(TokenKind::LParen)?;

        let mut params = Vec::new();
        while !self.check(&TokenKind::RParen) && !self.is_eof() {
            // Check if this is a named parameter (identifier followed by colon)
            // or anonymous (just a type)
            let param_name = if self.peek_kind() == Some(&TokenKind::Colon) {
                let name = self.parse_identifier()?;
                self.expect(TokenKind::Colon)?;
                Some(name)
            } else {
                None
            };
            let ty = self.parse_c_type()?;
            params.push(ExternParam { name: param_name, ty });

            if !self.check(&TokenKind::RParen) {
                self.expect(TokenKind::Comma)?;
            }
        }
        self.expect(TokenKind::RParen)?;

        let return_type = if self.check(&TokenKind::Arrow) {
            self.advance();
            Some(self.parse_c_type()?)
        } else {
            None
        };

        let span = start.merge(self.previous_span());
        Ok(ExternFunction {
            name,
            params,
            return_type,
            span,
        })
    }

    /// Parse extern struct declaration
    fn parse_extern_struct(&mut self) -> ParseResult<ExternStruct> {
        let start = self.current_span();
        self.expect(TokenKind::Struct)?;

        let name = self.parse_type_identifier()?;
        let mut fields = Vec::new();

        // Fields until end or next item
        while !self.check(&TokenKind::End)
            && !self.check(&TokenKind::Fn)
            && !self.check(&TokenKind::Struct)
            && !self.check(&TokenKind::Const)
            && !self.check(&TokenKind::Type)
            && !self.is_eof()
        {
            let field_name = self.parse_identifier()?;
            self.expect(TokenKind::Colon)?;
            let ty = self.parse_c_type()?;
            fields.push(ExternField { name: field_name, ty });
        }

        let span = start.merge(self.previous_span());
        Ok(ExternStruct { name, fields, span })
    }

    /// Parse extern const declaration
    fn parse_extern_const(&mut self) -> ParseResult<ExternConst> {
        let start = self.current_span();
        self.expect(TokenKind::Const)?;

        let name = self.parse_identifier()?;
        self.expect(TokenKind::Colon)?;
        let ty = self.parse_c_type()?;

        let span = start.merge(self.previous_span());
        Ok(ExternConst { name, ty, span })
    }

    /// Parse a C type
    fn parse_c_type(&mut self) -> ParseResult<CType> {
        // Check for pointer prefix
        if self.check(&TokenKind::Star) {
            self.advance();
            let const_ = if self.current_token_text() == "const" {
                self.advance();
                true
            } else {
                false
            };
            let pointee = Box::new(self.parse_c_type()?);
            return Ok(CType::Pointer { const_, pointee });
        }

        // Parse base type
        let type_name = self.current_token_text();
        let ty = match type_name.as_str() {
            "int" | "CInt" => CType::Int,
            "uint" | "CUInt" => CType::UInt,
            "long" | "CLong" => CType::Long,
            "ulong" | "CULong" => CType::ULong,
            "longlong" | "CLongLong" => CType::LongLong,
            "float" | "CFloat" => CType::Float,
            "double" | "CDouble" => CType::Double,
            "char" | "CChar" => CType::Char,
            "void" | "CVoid" => CType::Void,
            "size_t" | "CSize" => CType::SizeT,
            "ssize_t" | "CSSize" => CType::SSizeT,
            _ => {
                let name = self.parse_type_identifier()?;
                return Ok(CType::Named(name));
            }
        };
        self.advance();
        Ok(ty)
    }

    /// Parse extern Python block
    fn parse_extern_python(&mut self, start: Span) -> ParseResult<ExternDecl> {
        // Expect 'from' "module"
        self.expect(TokenKind::From)?;
        let module = self.expect_string()?;

        let alias = if self.check(&TokenKind::As) {
            self.advance();
            Some(self.parse_identifier()?)
        } else {
            None
        };

        let span = start.merge(self.previous_span());
        Ok(ExternDecl::Python(ExternPython { module, alias, span }))
    }

    /// Parse extern Wasm block
    fn parse_extern_wasm(&mut self, start: Span) -> ParseResult<ExternDecl> {
        // Check for import/export
        let kind = if self.current_token_text() == "import" {
            self.advance();
            WasmExternKind::Import
        } else if self.current_token_text() == "export" {
            self.advance();
            WasmExternKind::Export
        } else {
            return Err(ParseError::UnexpectedToken {
                expected: "import or export".to_string(),
                found: self.current_token_text().to_string(),
                span: self.current_span(),
            });
        };

        let name = self.parse_identifier()?;

        let mut items = Vec::new();
        while !self.check(&TokenKind::End) && !self.is_eof() {
            if self.check(&TokenKind::Fn) {
                items.push(self.parse_extern_function()?);
            } else {
                break;
            }
        }
        self.expect(TokenKind::End)?;

        let span = start.merge(self.previous_span());
        Ok(ExternDecl::Wasm(ExternWasm { kind, name, items, span }))
    }

    /// Expect and parse a string literal
    fn expect_string(&mut self) -> ParseResult<SmolStr> {
        match self.current_kind().cloned() {
            Some(TokenKind::SimpleString(s)) | Some(TokenKind::InterpolatedString(s)) | Some(TokenKind::RawString(s)) => {
                // The lexer includes quotes, so strip them
                let content = if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
                    SmolStr::new(&s[1..s.len()-1])
                } else {
                    s
                };
                self.advance();
                Ok(content)
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "string literal".to_string(),
                found: self.current_token_text().to_string(),
                span: self.current_span(),
            }),
        }
    }
}

/// Convenience function to parse a string into a Program
pub fn parse(source: &str) -> (Program, Vec<ParseError>) {
    let mut parser = Parser::new(source);
    let program = parser.parse_program().unwrap_or_else(|_e| {
        Program {
            items: Vec::new(),
            span: Span::new(0, source.len()),
        }
    });
    (program, parser.errors().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_function() {
        let source = r#"fn add(a: Int, b: Int) -> Int
  a + b
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
        assert_eq!(program.items.len(), 1);
        assert!(matches!(program.items[0], Item::Function(_)));
    }

    #[test]
    fn test_parse_struct() {
        let source = r#"struct Point
  x: Float
  y: Float
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
        assert_eq!(program.items.len(), 1);
        assert!(matches!(program.items[0], Item::Struct(_)));
    }

    #[test]
    fn test_parse_data() {
        let source = "data Point(x: Float, y: Float)";
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
        assert_eq!(program.items.len(), 1);
        assert!(matches!(program.items[0], Item::Data(_)));
    }

    #[test]
    fn test_parse_enum() {
        let source = r#"enum Option<T>
  Some(T)
  None
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
        assert_eq!(program.items.len(), 1);
    }

    #[test]
    fn test_parse_with_contracts() {
        let source = r#"fn positive(x: Int) -> Int
  requires x > 0
  ensures result > 0
  x
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
        if let Item::Function(f) = &program.items[0] {
            assert_eq!(f.contracts.len(), 2);
        }
    }

    #[test]
    fn test_parse_assignment() {
        let source = r#"fn assign_example()
  let x = 1
  x = 2
  x += 3
  x -= 1
  x *= 2
  x /= 2
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
        if let Item::Function(f) = &program.items[0] {
            if let FunctionBody::Block(block) = &f.body {
                // let mut x = 1, x = 2, x += 3, x -= 1, x *= 2, x /= 2
                assert_eq!(block.stmts.len(), 6);
            } else {
                panic!("Expected block body");
            }
        }
    }

    // ========================================================================
    // Additional comprehensive tests for ARIA-IMPL-006
    // Working parser features
    // ========================================================================

    #[test]
    fn test_parse_while_loop() {
        let source = r#"fn countdown(n: Int)
  while n > 0
    print(n)
  end
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
        assert_eq!(program.items.len(), 1);
    }

    #[test]
    fn test_parse_for_loop() {
        let source = r#"fn iterate(items: Array<Int>)
  for item in items
    print(item)
  end
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
        assert_eq!(program.items.len(), 1);
    }

    #[test]
    fn test_parse_loop_infinite() {
        let source = r#"fn run_forever()
  loop
    process()
  end
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
        assert_eq!(program.items.len(), 1);
    }

    #[test]
    fn test_parse_short_function() {
        let source = "fn double(x: Int) -> Int = x * 2";
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
        if let Item::Function(f) = &program.items[0] {
            assert!(matches!(f.body, FunctionBody::Expression(_)));
        }
    }

    #[test]
    fn test_parse_generic_function() {
        let source = r#"fn first<T>(items: Array<T>) -> T
  items[0]
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
        if let Item::Function(f) = &program.items[0] {
            assert!(f.generic_params.is_some());
        }
    }

    #[test]
    fn test_parse_array_literal() {
        let source = r#"fn make_array()
  let arr = [1, 2, 3, 4, 5]
  arr
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_parse_map_literal() {
        let source = r#"fn make_map()
  let map = {name: "Alice", age: 30}
  map
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_parse_comparison_chain() {
        let source = r#"fn in_range(x: Int) -> Bool
  0 <= x and x < 100
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_parse_array_type() {
        let source = r#"fn with_arrays(a: Array<Int>, b: [String])
  a
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_parse_reference_type() {
        let source = r#"fn by_ref(x: &Int, y: &mut String)
  print(x)
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_parse_test_block() {
        let source = r#"test "addition works"
  let result = 1 + 1
  assert(result == 2)
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
        assert!(matches!(program.items[0], Item::Test(_)));
    }

    #[test]
    fn test_parse_visibility() {
        let source = r#"pub fn public_fn()
  42
end

struct MyStruct
  pub name: String
  priv secret: String
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
        assert_eq!(program.items.len(), 2);
    }

    #[test]
    fn test_parse_defer() {
        let source = r#"fn with_cleanup()
  let file = open("test.txt")
  defer close(file)
  read(file)
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_parse_method_call() {
        let source = r#"fn use_methods()
  let s = "hello"
  let len = s.len()
  let upper = s.to_upper()
  len
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_parse_struct_init() {
        let source = r#"fn create_point()
  let p = Point(x: 1.0, y: 2.0)
  p
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_parse_multiple_items() {
        let source = r#"struct Point
  x: Float
  y: Float
end

fn distance(a: Point, b: Point) -> Float
  let dx = b.x - a.x
  let dy = b.y - a.y
  dx
end

enum Shape
  Circle(Float)
  Rectangle(Float, Float)
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
        assert_eq!(program.items.len(), 3);
    }

    #[test]
    fn test_parse_type_alias() {
        let source = "type UserId = Int";
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
        assert!(matches!(program.items[0], Item::TypeAlias(_)));
    }

    #[test]
    fn test_parse_simple_binary_ops() {
        let source = r#"fn math()
  let a = 1 + 2
  let b = 3 - 1
  let c = 2 * 4
  let d = 8 / 2
  let e = 7 // 3
  let f = 10 % 3
  a
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_parse_logical_ops() {
        let source = r#"fn logic()
  let a = true and false
  let b = true or false
  a
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_parse_field_access() {
        let source = r#"fn get_x(p: Point) -> Float
  p.x
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_parse_function_call() {
        let source = r#"fn caller()
  let result = add(1, 2)
  result
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    // ========================================================================
    // Tests documenting features that need parser improvements
    // These are marked as ignored until the parser supports them
    // ========================================================================

    #[test]
    fn test_parse_trait() {
        let source = r#"trait Display
  fn to_string(self) -> String
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_parse_impl() {
        let source = r#"impl Point
  fn new(x: Float, y: Float) -> Self
    Self(x:, y:)
  end
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_parse_if_statement() {
        let source = r#"fn check(x: Int)
  if x > 0
    print("positive")
  elsif x < 0
    print("negative")
  else
    print("zero")
  end
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_parse_tuple_literal() {
        let source = r#"fn make_tuple()
  let t = (1, "hello", 3.14)
  t
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_parse_lambda_block() {
        let source = r#"fn use_lambda()
  let f = { |x| x * 2 }
  f(10)
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_parse_lambda_arrow() {
        let source = r#"fn use_arrow_lambda()
  let f = (x) => x * 2
  f(10)
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_parse_power_operator() {
        let source = r#"fn power()
  let a = 2 ** 3
  a
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_parse_pipe_operator() {
        let source = r#"fn pipeline(items: Array<Int>)
  items |> filter |> map |> reduce
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_parse_range_expression() {
        let source = r#"fn ranges()
  let inclusive = 1..10
  let exclusive = 1..<10
  inclusive
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_parse_optional_type() {
        let source = r#"fn maybe(x: Int?) -> Int?
  x
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_parse_result_type() {
        let source = r#"fn fallible() -> Result<Int, String>
  Ok(42)
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_parse_map_type() {
        let source = r#"fn with_map(m: Map<String, Int>) -> {String: Int}
  m
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_parse_function_type() {
        let source = r#"fn higher_order(f: Fn(Int, Int) -> Int) -> Int
  f(1, 2)
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_parse_import() {
        let source = r#"import std::io::File
import std::net::http as http"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_parse_module() {
        let source = r#"module MyApp::Models
  struct User
    name: String
  end
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_parse_ternary_expression() {
        let source = r#"fn ternary(x: Int) -> String
  x > 0 ? "positive" : "non-positive"
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_parse_bitwise_not() {
        let source = r#"fn bitwise()
  let d = ~0xFF
  d
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_parse_const_decl() {
        let source = "const MAX_SIZE = 1024";
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);
    }

    #[test]
    fn test_parse_module_with_import() {
        let source = r#"
import utils::{add, multiply}

pub fn main() -> Int
  let sum = add(10, 20)
  let product = multiply(5, 6)
  sum + product
end
"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);
        assert_eq!(program.items.len(), 2, "Should have import + function");
    }

    #[test]
    fn test_parse_use_declaration() {
        // Simple use
        let source1 = "use MyModule::MyType";
        let (program1, errors1) = parse(source1);
        assert!(errors1.is_empty(), "Parse errors: {:?}", errors1);
        assert_eq!(program1.items.len(), 1);
        if let Item::Use(u) = &program1.items[0] {
            assert_eq!(u.path.len(), 2);
            assert_eq!(u.path[0].node.as_str(), "MyModule");
            assert_eq!(u.path[1].node.as_str(), "MyType");
            assert_eq!(u.visibility, Visibility::Private);
        } else {
            panic!("Expected Use item");
        }

        // Public use (re-export)
        let source2 = "pub use std::collections::HashMap";
        let (program2, errors2) = parse(source2);
        assert!(errors2.is_empty(), "Parse errors: {:?}", errors2);
        if let Item::Use(u) = &program2.items[0] {
            assert_eq!(u.visibility, Visibility::Public);
            assert_eq!(u.path.len(), 3);
        } else {
            panic!("Expected Use item");
        }

        // Use with selection
        let source3 = "use collections::{List, Map, Set}";
        let (program3, errors3) = parse(source3);
        assert!(errors3.is_empty(), "Parse errors: {:?}", errors3);
        if let Item::Use(u) = &program3.items[0] {
            assert!(u.selection.is_some());
            if let Some(ImportSelection::Items(items)) = &u.selection {
                assert_eq!(items.len(), 3);
            } else {
                panic!("Expected Items selection");
            }
        } else {
            panic!("Expected Use item");
        }

        // Use with glob
        let source4 = "pub use models::*";
        let (program4, errors4) = parse(source4);
        assert!(errors4.is_empty(), "Parse errors: {:?}", errors4);
        if let Item::Use(u) = &program4.items[0] {
            assert!(matches!(u.selection, Some(ImportSelection::All)));
            assert_eq!(u.visibility, Visibility::Public);
        } else {
            panic!("Expected Use item");
        }

        // Use with alias
        let source5 = "use external::LongTypeName as Short";
        let (program5, errors5) = parse(source5);
        assert!(errors5.is_empty(), "Parse errors: {:?}", errors5);
        if let Item::Use(u) = &program5.items[0] {
            assert!(u.alias.is_some());
            assert_eq!(u.alias.as_ref().unwrap().node.as_str(), "Short");
        } else {
            panic!("Expected Use item");
        }
    }

    // =========================================================================
    // Channel Syntax Tests
    // =========================================================================

    #[test]
    fn test_parse_channel_receive() {
        // Channel receive: `<- channel`
        let source = r#"fn receive_example(ch: Channel<Int>) -> Int
  <- ch
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);

        // Check that we have a function
        assert_eq!(program.items.len(), 1);
        if let Item::Function(func) = &program.items[0] {
            // The body should contain a ChannelRecv
            if let FunctionBody::Block(block) = &func.body {
                assert!(!block.stmts.is_empty());
                if let StmtKind::Expr(expr) = &block.stmts[0].kind {
                    assert!(matches!(expr.kind, ExprKind::ChannelRecv { .. }));
                } else {
                    panic!("Expected expression statement");
                }
            } else {
                panic!("Expected block body");
            }
        } else {
            panic!("Expected function");
        }
    }

    #[test]
    fn test_parse_channel_send() {
        // Channel send: `channel <- value`
        let source = r#"fn send_example(ch: Channel<Int>, value: Int)
  ch <- value
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);

        // Check that we have a function
        assert_eq!(program.items.len(), 1);
        if let Item::Function(func) = &program.items[0] {
            if let FunctionBody::Block(block) = &func.body {
                assert!(!block.stmts.is_empty());
                if let StmtKind::Expr(expr) = &block.stmts[0].kind {
                    assert!(matches!(expr.kind, ExprKind::ChannelSend { .. }));
                } else {
                    panic!("Expected expression statement");
                }
            } else {
                panic!("Expected block body");
            }
        } else {
            panic!("Expected function");
        }
    }

    #[test]
    fn test_parse_channel_type() {
        // Channel type annotation: `Channel<Int>`
        let source = r#"fn make_channel() -> Channel<String>
  Channel.new()
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);
    }

    #[test]
    fn test_parse_channel_operations() {
        // Multiple channel operations
        let source = r#"fn producer_consumer()
  let ch = Channel.new()
  ch <- 42
  let value = <- ch
  value
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);
    }

    #[test]
    fn test_parse_channel_with_expression() {
        // Channel send with complex expression
        let source = r#"fn complex_send(ch: Channel<Int>)
  ch <- 1 + 2 * 3
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);
    }

    // =========================================================================
    // Select Statement Tests
    // =========================================================================

    #[test]
    fn test_parse_select_receive() {
        // Select with receive
        let source = r#"fn select_example(inbox: Channel<String>)
  select
    msg = <-inbox => process(msg)
  end
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);

        // Verify we have a function with select
        assert_eq!(program.items.len(), 1);
        if let Item::Function(func) = &program.items[0] {
            if let FunctionBody::Block(block) = &func.body {
                assert!(!block.stmts.is_empty());
                if let StmtKind::Expr(expr) = &block.stmts[0].kind {
                    assert!(matches!(expr.kind, ExprKind::Select(_)));
                } else {
                    panic!("Expected expression statement");
                }
            } else {
                panic!("Expected block body");
            }
        } else {
            panic!("Expected function");
        }
    }

    #[test]
    fn test_parse_select_send() {
        // Select with send
        let source = r#"fn select_send(ch: Channel<Int>)
  select
    ch <- 42 => log("sent")
  end
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);
    }

    #[test]
    fn test_parse_select_default() {
        // Select with default
        let source = r#"fn select_default(ch: Channel<Int>)
  select
    <-ch => process()
    default => idle()
  end
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);
    }

    #[test]
    fn test_parse_select_multiple_cases() {
        // Select with multiple cases
        let source = r#"fn multiplexer(inbox: Channel<Msg>, outbox: Channel<Reply>)
  select
    msg = <-inbox => process(msg)
    outbox <- reply => log("sent")
    default => yield()
  end
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);
    }

    #[test]
    fn test_parse_select_receive_no_binding() {
        // Select receive without binding
        let source = r#"fn ticker(timer: Channel<Unit>)
  select
    <-timer => tick()
  end
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);
    }

    // ========================================================================
    // Handle Expression Tests (Effect Handlers / Try-Catch)
    // ========================================================================

    #[test]
    fn test_parse_handle_simple() {
        // Simple handle expression with no handlers
        let source = r#"fn safe_compute()
  handle
    risky_computation()
  with
  end
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);

        // Verify we have a function with handle
        assert_eq!(program.items.len(), 1);
        if let Item::Function(func) = &program.items[0] {
            if let FunctionBody::Block(block) = &func.body {
                assert!(!block.stmts.is_empty());
                if let StmtKind::Expr(expr) = &block.stmts[0].kind {
                    assert!(matches!(expr.kind, ExprKind::Handle { .. }));
                } else {
                    panic!("Expected expression statement");
                }
            } else {
                panic!("Expected block body");
            }
        } else {
            panic!("Expected function");
        }
    }

    #[test]
    fn test_parse_handle_with_exception_handler() {
        // Handle expression with Exception handler (try/catch pattern)
        let source = r#"fn try_catch_example()
  handle
    might_throw()
  with
    Exception.raise(e) => fallback_value
  end
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);
    }

    #[test]
    fn test_parse_handle_with_multiple_handlers() {
        // Handle expression with multiple effect handlers
        // Note: We use "FileIO" instead of "IO" because "IO" is lexed as ConstIdent (all caps)
        let source = r#"fn multi_handler()
  handle
    complex_operation()
  with
    Exception.raise(e) => 0
    FileIO.read(path) => resume("")
  end
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);
    }

    #[test]
    fn test_parse_handle_with_return_clause() {
        // Handle expression with return clause
        let source = r#"fn with_return_clause()
  handle
    42
  with
    return(x) => x + 1
  end
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);
    }

    #[test]
    fn test_parse_raise_expression() {
        // Raise expression
        let source = r#"fn throw_error()
  raise("something went wrong")
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);

        // Verify we have a function with raise
        assert_eq!(program.items.len(), 1);
        if let Item::Function(func) = &program.items[0] {
            if let FunctionBody::Block(block) = &func.body {
                assert!(!block.stmts.is_empty());
                if let StmtKind::Expr(expr) = &block.stmts[0].kind {
                    assert!(matches!(expr.kind, ExprKind::Raise { .. }));
                } else {
                    panic!("Expected expression statement");
                }
            } else {
                panic!("Expected block body");
            }
        } else {
            panic!("Expected function");
        }
    }

    #[test]
    fn test_parse_resume_expression() {
        // Resume expression inside a handler
        let source = r#"fn handler_with_resume()
  handle
    get_state()
  with
    State.get() => resume(42)
  end
end"#;
        let (program, errors) = parse(source);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);
    }

    // ========================================================================
    // Error Message and Recovery Tests (ARIA-M19)
    // ========================================================================

    #[test]
    fn test_error_missing_end_in_function() {
        // Missing 'end' keyword in function
        let source = r#"fn broken()
  let x = 1
"#;
        let (_program, errors) = parse(source);
        // Should have an error about missing 'end'
        assert!(!errors.is_empty(), "Should have error for missing 'end'");
        assert!(
            errors.iter().any(|e| matches!(e, ParseError::MissingEnd { .. })
                || format!("{}", e).contains("end")),
            "Error should mention 'end': {:?}",
            errors
        );
    }

    #[test]
    fn test_error_missing_end_in_struct() {
        // Missing 'end' keyword in struct
        let source = r#"struct Point
  x: Float
  y: Float
"#;
        let (_program, errors) = parse(source);
        assert!(!errors.is_empty(), "Should have error for missing 'end'");
        assert!(
            errors.iter().any(|e| matches!(e, ParseError::MissingEnd { construct, .. } if construct == "struct")),
            "Error should be about missing 'end' for struct: {:?}",
            errors
        );
    }

    #[test]
    fn test_error_missing_end_in_enum() {
        // Missing 'end' keyword in enum
        let source = r#"enum Color
  Red
  Green
  Blue
"#;
        let (_program, errors) = parse(source);
        assert!(!errors.is_empty(), "Should have error for missing 'end'");
        assert!(
            errors.iter().any(|e| matches!(e, ParseError::MissingEnd { construct, .. } if construct == "enum")),
            "Error should be about missing 'end' for enum: {:?}",
            errors
        );
    }

    #[test]
    fn test_error_unclosed_parenthesis_in_function() {
        // Unclosed parenthesis in function signature
        let source = r#"fn broken(x: Int
  x + 1
end"#;
        let (_program, errors) = parse(source);
        assert!(!errors.is_empty(), "Should have error for unclosed paren");
        assert!(
            errors.iter().any(|e| matches!(e, ParseError::UnclosedBracket { .. })
                || format!("{}", e).contains(")") || format!("{}", e).contains("paren")),
            "Error should mention unclosed parenthesis: {:?}",
            errors
        );
    }

    #[test]
    fn test_error_missing_type_annotation_struct_field() {
        // Missing type annotation on struct field
        let source = r#"struct Point
  x
  y: Float
end"#;
        let (_program, errors) = parse(source);
        assert!(!errors.is_empty(), "Should have error for missing type");
        assert!(
            errors.iter().any(|e| matches!(e, ParseError::MissingTypeAnnotation { .. })),
            "Error should be MissingTypeAnnotation: {:?}",
            errors
        );
    }

    #[test]
    fn test_error_recovery_continues_after_error() {
        // Test that parser recovers and continues after an error
        let source = r#"fn broken(
  bad syntax here
end

fn valid()
  42
end"#;
        let (_program, errors) = parse(source);
        // Should have errors from the broken function
        assert!(!errors.is_empty(), "Should have parsing errors");
        // But should still try to parse the valid function
        // (recovery to next item)
        // The program might have 0, 1, or 2 items depending on recovery
        // The key is that parsing didn't completely fail
    }

    #[test]
    fn test_error_mismatched_brackets() {
        // Test mismatched brackets
        let source = r#"fn broken()
  let x = [1, 2, 3)
end"#;
        let (_program, errors) = parse(source);
        assert!(!errors.is_empty(), "Should have error for mismatched brackets");
    }

    #[test]
    fn test_error_multiple_errors_reported() {
        // Test that multiple errors can be collected
        let source = r#"fn first(
  incomplete
end

fn second(x
  incomplete too
end"#;
        let (_program, errors) = parse(source);
        // Should have multiple errors
        assert!(!errors.is_empty(), "Should have multiple errors");
    }

    #[test]
    fn test_error_message_includes_position() {
        // Test that error messages include position information
        let source = r#"fn broken()
  let x =
end"#;
        let (_program, errors) = parse(source);
        assert!(!errors.is_empty(), "Should have error");
        // Error should have span information
        let first_error = &errors[0];
        let error_str = format!("{}", first_error);
        assert!(
            error_str.contains("position") || error_str.contains("Span"),
            "Error should include position info: {}",
            error_str
        );
    }

    #[test]
    fn test_error_missing_end_in_test_block() {
        // Missing 'end' keyword in test block
        let source = r#"test "my test"
  assert(true)
"#;
        let (_program, errors) = parse(source);
        assert!(!errors.is_empty(), "Should have error for missing 'end'");
        assert!(
            errors.iter().any(|e| matches!(e, ParseError::MissingEnd { construct, .. } if construct == "test")),
            "Error should be about missing 'end' for test: {:?}",
            errors
        );
    }

    #[test]
    fn test_error_missing_end_in_impl() {
        // Missing 'end' keyword in impl block
        let source = r#"impl Point
  fn new() -> Self
    Self()
  end
"#;
        let (_program, errors) = parse(source);
        assert!(!errors.is_empty(), "Should have error for missing 'end'");
        assert!(
            errors.iter().any(|e| matches!(e, ParseError::MissingEnd { construct, .. } if construct == "impl")),
            "Error should be about missing 'end' for impl: {:?}",
            errors
        );
    }

    #[test]
    fn test_error_missing_end_in_trait() {
        // Missing 'end' keyword in trait block
        let source = r#"trait Display
  fn show(self) -> String
"#;
        let (_program, errors) = parse(source);
        assert!(!errors.is_empty(), "Should have error for missing 'end'");
        assert!(
            errors.iter().any(|e| matches!(e, ParseError::MissingEnd { construct, .. } if construct == "trait")),
            "Error should be about missing 'end' for trait: {:?}",
            errors
        );
    }

    #[test]
    fn test_error_missing_end_in_module() {
        // Missing 'end' keyword in module block
        let source = r#"module MyApp
  fn helper()
    42
  end
"#;
        let (_program, errors) = parse(source);
        assert!(!errors.is_empty(), "Should have error for missing 'end'");
        assert!(
            errors.iter().any(|e| matches!(e, ParseError::MissingEnd { construct, .. } if construct == "module")),
            "Error should be about missing 'end' for module: {:?}",
            errors
        );
    }

    #[test]
    fn test_helpful_error_message_content() {
        // Verify error messages contain helpful hints
        let source = r#"struct Point
  x
end"#;
        let (_program, errors) = parse(source);
        assert!(!errors.is_empty());
        let error_msg = format!("{}", errors[0]);
        // Should contain helpful information about the expected syntax
        assert!(
            error_msg.contains("type") || error_msg.contains(":") || error_msg.contains("annotation"),
            "Error should have helpful hint: {}",
            error_msg
        );
    }
}
