//! Aria Language Lexer
//!
//! Tokenizes Aria source code according to GRAMMAR.md Section 1 (Lexical Structure).
//! Uses the `logos` crate for efficient lexing.

use logos::Logos;
use smol_str::SmolStr;
use std::fmt;
use std::ops::Range;

/// Source span representing a range in the source code
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn merge(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    pub fn dummy() -> Self {
        Self { start: 0, end: 0 }
    }
}

impl From<Range<usize>> for Span {
    fn from(range: Range<usize>) -> Self {
        Span::new(range.start, range.end)
    }
}

impl From<Span> for Range<usize> {
    fn from(span: Span) -> Self {
        span.start..span.end
    }
}

/// A token with its kind and source location
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// Callback to lex block comments: consumes input until closing ###
fn block_comment_callback(lex: &mut logos::Lexer<TokenKind>) -> Option<SmolStr> {
    let remainder = lex.remainder();
    if let Some(end_idx) = remainder.find("###") {
        let content = &remainder[..end_idx];
        lex.bump(end_idx + 3); // consume content + closing ###
        Some(SmolStr::new(&format!("###{content}###")))
    } else {
        // No closing ###, consume rest as unterminated comment
        let len = remainder.len();
        lex.bump(len);
        Some(SmolStr::new(&format!("###{remainder}")))
    }
}

/// All token types in the Aria language
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t]+")]  // Skip whitespace (but not newlines)
pub enum TokenKind {
    // ========== Keywords ==========
    #[token("fn")]
    Fn,
    #[token("let")]
    Let,
    #[token("mut")]
    Mut,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("elsif")]
    Elsif,
    #[token("unless")]
    Unless,
    #[token("match")]
    Match,
    #[token("for")]
    For,
    #[token("while")]
    While,
    #[token("loop")]
    Loop,
    #[token("break")]
    Break,
    #[token("continue")]
    Continue,
    #[token("return")]
    Return,
    #[token("struct")]
    Struct,
    #[token("data")]
    Data,
    #[token("enum")]
    Enum,
    #[token("trait")]
    Trait,
    #[token("impl")]
    Impl,
    #[token("module")]
    Module,
    #[token("import")]
    Import,
    #[token("export")]
    Export,
    #[token("use")]
    Use,
    #[token("from")]
    From,
    #[token("as")]
    As,
    #[token("extern")]
    Extern,
    #[token("unsafe")]
    Unsafe,
    #[token("defer")]
    Defer,
    #[token("spawn")]
    Spawn,
    #[token("await")]
    Await,
    #[token("select")]
    Select,
    #[token("requires")]
    Requires,
    #[token("ensures")]
    Ensures,
    #[token("invariant")]
    Invariant,
    #[token("examples")]
    Examples,
    #[token("property")]
    Property,
    #[token("test")]
    Test,
    #[token("forall")]
    Forall,
    #[token("exists")]
    Exists,
    #[token("true")]
    True,
    #[token("false")]
    False,
    #[token("nil")]
    Nil,
    #[token("self")]
    SelfLower,
    #[token("Self")]
    SelfUpper,
    #[token("super")]
    Super,
    #[token("and")]
    And,
    #[token("or")]
    Or,
    #[token("not")]
    Not,
    #[token("in")]
    In,
    #[token("is")]
    Is,
    #[token("ref")]
    Ref,
    #[token("move")]
    Move,
    #[token("copy")]
    Copy,
    #[token("pub")]
    Pub,
    #[token("priv")]
    Priv,
    #[token("end")]
    End,
    #[token("where")]
    Where,
    #[token("type")]
    Type,
    #[token("const")]
    Const,
    #[token("derive")]
    Derive,
    #[token("old")]
    Old,
    // TODO: Make 'result' context-sensitive (only a keyword in contracts)
    // For now, removed to allow 'result' as a variable name
    // #[token("result")]
    // Result,
    #[token("raises")]
    Raises,
    #[token("handle")]
    Handle,
    #[token("with")]
    With,
    #[token("resume")]
    Resume,
    #[token("raise")]
    Raise,
    #[token("then")]
    Then,
    #[token("default")]
    Default,

    // ========== Operators ==========
    // Arithmetic
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("//")]
    SlashSlash,
    #[token("%")]
    Percent,
    #[token("**")]
    StarStar,

    // Comparison
    #[token("==")]
    EqEq,
    #[token("!=")]
    NotEq,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("<=")]
    LtEq,
    #[token(">=")]
    GtEq,
    #[token("<=>")]
    Spaceship,

    // Logical (symbol forms)
    #[token("&&")]
    AmpAmp,
    #[token("||")]
    PipePipe,
    #[token("!")]
    Bang,

    // Bitwise
    #[token("&")]
    Amp,
    #[token("|")]
    Pipe,
    #[token("^")]
    Caret,
    #[token("~")]
    Tilde,
    #[token("<<")]
    LtLt,
    #[token(">>")]
    GtGt,

    // Assignment
    #[token("=")]
    Eq,
    #[token("+=")]
    PlusEq,
    #[token("-=")]
    MinusEq,
    #[token("*=")]
    StarEq,
    #[token("/=")]
    SlashEq,
    #[token("//=")]
    SlashSlashEq,
    #[token("%=")]
    PercentEq,
    #[token("&=")]
    AmpEq,
    #[token("|=")]
    PipeEq,
    #[token("^=")]
    CaretEq,
    #[token("<<=")]
    LtLtEq,
    #[token(">>=")]
    GtGtEq,

    // Special
    #[token("->")]
    Arrow,
    #[token("=>")]
    FatArrow,
    #[token("|>")]
    Pipe_,
    #[token("..")]
    DotDot,
    #[token("..<")]
    DotDotLt,
    #[token("..=")]
    DotDotEq,
    #[token("?")]
    Question,
    #[token("::")]
    ColonColon,
    #[token(".")]
    Dot,
    #[token("@")]
    At,
    #[token("@!")]
    AtBang,
    #[token("&:")]
    AmpColon,
    #[token("<-")]
    LeftArrow,
    #[token("...")]
    DotDotDot,
    #[token("~=")]
    TildeEq,

    // ========== Delimiters ==========
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[token(";")]
    Semi,
    #[token("?.")]
    QuestionDot,

    // ========== Literals ==========
    /// Integer literal (decimal, hex, octal, binary)
    #[regex(r"[0-9][0-9_]*(?:i8|i16|i32|i64|i128|u8|u16|u32|u64|u128|isize|usize)?", |lex| SmolStr::new(lex.slice()))]
    #[regex(r"0x[0-9a-fA-F][0-9a-fA-F_]*(?:i8|i16|i32|i64|i128|u8|u16|u32|u64|u128|isize|usize)?", |lex| SmolStr::new(lex.slice()))]
    #[regex(r"0o[0-7][0-7_]*(?:i8|i16|i32|i64|i128|u8|u16|u32|u64|u128|isize|usize)?", |lex| SmolStr::new(lex.slice()))]
    #[regex(r"0b[01][01_]*(?:i8|i16|i32|i64|i128|u8|u16|u32|u64|u128|isize|usize)?", |lex| SmolStr::new(lex.slice()))]
    Integer(SmolStr),

    /// Float literal
    #[regex(r"[0-9][0-9_]*\.[0-9][0-9_]*(?:[eE][+-]?[0-9][0-9_]*)?(?:f32|f64)?", |lex| SmolStr::new(lex.slice()))]
    #[regex(r"[0-9][0-9_]*[eE][+-]?[0-9][0-9_]*(?:f32|f64)?", |lex| SmolStr::new(lex.slice()))]
    Float(SmolStr),

    /// Simple string (single quotes)
    #[regex(r"'(?:[^'\\]|\\.)*'", |lex| SmolStr::new(lex.slice()))]
    SimpleString(SmolStr),

    /// Interpolated string (double quotes)
    #[regex(r#""(?:[^"\\]|\\.)*""#, |lex| SmolStr::new(lex.slice()))]
    InterpolatedString(SmolStr),

    /// Raw string
    #[regex(r#"r"[^"]*""#, |lex| SmolStr::new(lex.slice()))]
    RawString(SmolStr),

    /// Character literal (backtick)
    #[regex(r"`(?:[^`\\]|\\.)`", |lex| SmolStr::new(lex.slice()))]
    Char(SmolStr),

    // ========== Identifiers ==========
    /// Regular identifier (starts with lowercase or underscore)
    #[regex(r"[a-z_][a-zA-Z0-9_]*[?!]?", |lex| SmolStr::new(lex.slice()))]
    Identifier(SmolStr),

    /// Type identifier (starts with uppercase, mixed case like MyType)
    #[regex(r"[A-Z][a-zA-Z0-9]*", priority = 1, callback = |lex| SmolStr::new(lex.slice()))]
    TypeIdent(SmolStr),

    /// Constant identifier (all uppercase with underscores like MAX_SIZE)
    #[regex(r"[A-Z][A-Z0-9_]+", priority = 3, callback = |lex| SmolStr::new(lex.slice()))]
    ConstIdent(SmolStr),

    // ========== Comments ==========
    /// Line comment
    #[regex(r"#[^#\n][^\n]*", |lex| SmolStr::new(lex.slice()))]
    #[regex(r"#\n", |_| SmolStr::new("#"))]
    LineComment(SmolStr),

    /// Doc comment
    #[regex(r"##[^#][^\n]*", |lex| SmolStr::new(lex.slice()))]
    DocComment(SmolStr),

    /// Block comment (simplified - matches ### to closing ###)
    #[token("###", block_comment_callback)]
    BlockComment(SmolStr),

    // ========== Whitespace ==========
    /// Newline (significant in Aria)
    #[regex(r"\n|\r\n")]
    Newline,

    // ========== Error ==========
    /// Lexer error - unrecognized character
    Error,
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Fn => write!(f, "fn"),
            TokenKind::Let => write!(f, "let"),
            TokenKind::Mut => write!(f, "mut"),
            TokenKind::If => write!(f, "if"),
            TokenKind::Else => write!(f, "else"),
            TokenKind::Elsif => write!(f, "elsif"),
            TokenKind::Unless => write!(f, "unless"),
            TokenKind::Match => write!(f, "match"),
            TokenKind::For => write!(f, "for"),
            TokenKind::While => write!(f, "while"),
            TokenKind::Loop => write!(f, "loop"),
            TokenKind::Break => write!(f, "break"),
            TokenKind::Continue => write!(f, "continue"),
            TokenKind::Return => write!(f, "return"),
            TokenKind::Struct => write!(f, "struct"),
            TokenKind::Data => write!(f, "data"),
            TokenKind::Enum => write!(f, "enum"),
            TokenKind::Trait => write!(f, "trait"),
            TokenKind::Impl => write!(f, "impl"),
            TokenKind::Module => write!(f, "module"),
            TokenKind::Import => write!(f, "import"),
            TokenKind::Export => write!(f, "export"),
            TokenKind::Use => write!(f, "use"),
            TokenKind::From => write!(f, "from"),
            TokenKind::As => write!(f, "as"),
            TokenKind::Extern => write!(f, "extern"),
            TokenKind::Unsafe => write!(f, "unsafe"),
            TokenKind::Defer => write!(f, "defer"),
            TokenKind::Spawn => write!(f, "spawn"),
            TokenKind::Await => write!(f, "await"),
            TokenKind::Select => write!(f, "select"),
            TokenKind::Requires => write!(f, "requires"),
            TokenKind::Ensures => write!(f, "ensures"),
            TokenKind::Invariant => write!(f, "invariant"),
            TokenKind::Examples => write!(f, "examples"),
            TokenKind::Property => write!(f, "property"),
            TokenKind::Test => write!(f, "test"),
            TokenKind::Forall => write!(f, "forall"),
            TokenKind::Exists => write!(f, "exists"),
            TokenKind::True => write!(f, "true"),
            TokenKind::False => write!(f, "false"),
            TokenKind::Nil => write!(f, "nil"),
            TokenKind::SelfLower => write!(f, "self"),
            TokenKind::SelfUpper => write!(f, "Self"),
            TokenKind::Super => write!(f, "super"),
            TokenKind::And => write!(f, "and"),
            TokenKind::Or => write!(f, "or"),
            TokenKind::Not => write!(f, "not"),
            TokenKind::In => write!(f, "in"),
            TokenKind::Is => write!(f, "is"),
            TokenKind::Ref => write!(f, "ref"),
            TokenKind::Move => write!(f, "move"),
            TokenKind::Copy => write!(f, "copy"),
            TokenKind::Pub => write!(f, "pub"),
            TokenKind::Priv => write!(f, "priv"),
            TokenKind::End => write!(f, "end"),
            TokenKind::Where => write!(f, "where"),
            TokenKind::Type => write!(f, "type"),
            TokenKind::Const => write!(f, "const"),
            TokenKind::Derive => write!(f, "derive"),
            TokenKind::Old => write!(f, "old"),
            // TokenKind::Result => write!(f, "result"),
            TokenKind::Raises => write!(f, "raises"),
            TokenKind::Handle => write!(f, "handle"),
            TokenKind::With => write!(f, "with"),
            TokenKind::Resume => write!(f, "resume"),
            TokenKind::Raise => write!(f, "raise"),
            TokenKind::Then => write!(f, "then"),
            TokenKind::Default => write!(f, "default"),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Star => write!(f, "*"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::SlashSlash => write!(f, "//"),
            TokenKind::Percent => write!(f, "%"),
            TokenKind::StarStar => write!(f, "**"),
            TokenKind::EqEq => write!(f, "=="),
            TokenKind::NotEq => write!(f, "!="),
            TokenKind::Lt => write!(f, "<"),
            TokenKind::Gt => write!(f, ">"),
            TokenKind::LtEq => write!(f, "<="),
            TokenKind::GtEq => write!(f, ">="),
            TokenKind::Spaceship => write!(f, "<=>"),
            TokenKind::AmpAmp => write!(f, "&&"),
            TokenKind::PipePipe => write!(f, "||"),
            TokenKind::Bang => write!(f, "!"),
            TokenKind::Amp => write!(f, "&"),
            TokenKind::Pipe => write!(f, "|"),
            TokenKind::Caret => write!(f, "^"),
            TokenKind::Tilde => write!(f, "~"),
            TokenKind::LtLt => write!(f, "<<"),
            TokenKind::GtGt => write!(f, ">>"),
            TokenKind::Eq => write!(f, "="),
            TokenKind::PlusEq => write!(f, "+="),
            TokenKind::MinusEq => write!(f, "-="),
            TokenKind::StarEq => write!(f, "*="),
            TokenKind::SlashEq => write!(f, "/="),
            TokenKind::SlashSlashEq => write!(f, "//="),
            TokenKind::PercentEq => write!(f, "%="),
            TokenKind::AmpEq => write!(f, "&="),
            TokenKind::PipeEq => write!(f, "|="),
            TokenKind::CaretEq => write!(f, "^="),
            TokenKind::LtLtEq => write!(f, "<<="),
            TokenKind::GtGtEq => write!(f, ">>="),
            TokenKind::Arrow => write!(f, "->"),
            TokenKind::FatArrow => write!(f, "=>"),
            TokenKind::Pipe_ => write!(f, "|>"),
            TokenKind::DotDot => write!(f, ".."),
            TokenKind::DotDotLt => write!(f, "..<"),
            TokenKind::DotDotEq => write!(f, "..="),
            TokenKind::Question => write!(f, "?"),
            TokenKind::ColonColon => write!(f, "::"),
            TokenKind::Dot => write!(f, "."),
            TokenKind::At => write!(f, "@"),
            TokenKind::AtBang => write!(f, "@!"),
            TokenKind::AmpColon => write!(f, "&:"),
            TokenKind::LeftArrow => write!(f, "<-"),
            TokenKind::DotDotDot => write!(f, "..."),
            TokenKind::TildeEq => write!(f, "~="),
            TokenKind::LParen => write!(f, "("),
            TokenKind::RParen => write!(f, ")"),
            TokenKind::LBracket => write!(f, "["),
            TokenKind::RBracket => write!(f, "]"),
            TokenKind::LBrace => write!(f, "{{"),
            TokenKind::RBrace => write!(f, "}}"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::Semi => write!(f, ";"),
            TokenKind::QuestionDot => write!(f, "?."),
            TokenKind::Integer(s) => write!(f, "integer `{}`", s),
            TokenKind::Float(s) => write!(f, "float `{}`", s),
            TokenKind::SimpleString(s) => write!(f, "string {}", s),
            TokenKind::InterpolatedString(s) => write!(f, "string {}", s),
            TokenKind::RawString(s) => write!(f, "raw string {}", s),
            TokenKind::Char(s) => write!(f, "char {}", s),
            TokenKind::Identifier(s) => write!(f, "identifier `{}`", s),
            TokenKind::TypeIdent(s) => write!(f, "type `{}`", s),
            TokenKind::ConstIdent(s) => write!(f, "constant `{}`", s),
            TokenKind::LineComment(_) => write!(f, "comment"),
            TokenKind::DocComment(_) => write!(f, "doc comment"),
            TokenKind::BlockComment(_) => write!(f, "block comment"),
            TokenKind::Newline => write!(f, "newline"),
            TokenKind::Error => write!(f, "error"),
        }
    }
}

impl TokenKind {
    /// Check if this token is a keyword
    pub fn is_keyword(&self) -> bool {
        matches!(
            self,
            TokenKind::Fn
                | TokenKind::Let
                | TokenKind::Mut
                | TokenKind::If
                | TokenKind::Else
                | TokenKind::Elsif
                | TokenKind::Unless
                | TokenKind::Match
                | TokenKind::For
                | TokenKind::While
                | TokenKind::Loop
                | TokenKind::Break
                | TokenKind::Continue
                | TokenKind::Return
                | TokenKind::Struct
                | TokenKind::Data
                | TokenKind::Enum
                | TokenKind::Trait
                | TokenKind::Impl
                | TokenKind::Module
                | TokenKind::Import
                | TokenKind::Export
                | TokenKind::Use
                | TokenKind::From
                | TokenKind::As
                | TokenKind::Extern
                | TokenKind::Unsafe
                | TokenKind::Defer
                | TokenKind::Spawn
                | TokenKind::Await
                | TokenKind::Select
                | TokenKind::Requires
                | TokenKind::Ensures
                | TokenKind::Invariant
                | TokenKind::Examples
                | TokenKind::Property
                | TokenKind::Test
                | TokenKind::Forall
                | TokenKind::Exists
                | TokenKind::True
                | TokenKind::False
                | TokenKind::Nil
                | TokenKind::SelfLower
                | TokenKind::SelfUpper
                | TokenKind::Super
                | TokenKind::And
                | TokenKind::Or
                | TokenKind::Not
                | TokenKind::In
                | TokenKind::Is
                | TokenKind::Ref
                | TokenKind::Move
                | TokenKind::Copy
                | TokenKind::Pub
                | TokenKind::Priv
                | TokenKind::End
                | TokenKind::Where
                | TokenKind::Type
                | TokenKind::Const
                | TokenKind::Derive
                | TokenKind::Old
                // | TokenKind::Result
                | TokenKind::Raises
                | TokenKind::Handle
                | TokenKind::With
                | TokenKind::Resume
                | TokenKind::Raise
                | TokenKind::Then
                | TokenKind::Default
        )
    }

    /// Check if this token is a comment
    pub fn is_comment(&self) -> bool {
        matches!(
            self,
            TokenKind::LineComment(_) | TokenKind::DocComment(_) | TokenKind::BlockComment(_)
        )
    }

    /// Check if this token is trivia (comments or newlines)
    pub fn is_trivia(&self) -> bool {
        self.is_comment() || matches!(self, TokenKind::Newline)
    }
}

/// Lexer error type with detailed error messages
#[derive(Debug, Clone, thiserror::Error)]
pub enum LexerError {
    #[error("Unexpected character '{1}' at position {0}")]
    UnexpectedCharacter(usize, char),

    #[error("Unterminated string literal starting at position {0}")]
    UnterminatedString(usize),

    #[error("Invalid escape sequence '\\{1}' at position {0}")]
    InvalidEscape(usize, char),

    #[error("Unterminated block comment starting at position {0} - expected closing '###'")]
    UnterminatedBlockComment(usize),

    #[error("Invalid number literal at position {0}: {1}")]
    InvalidNumber(usize, String),

    #[error("Unterminated character literal at position {0}")]
    UnterminatedChar(usize),
}

/// Lexer for Aria source code
pub struct Lexer<'src> {
    source: &'src str,
    inner: logos::Lexer<'src, TokenKind>,
}

impl<'src> Lexer<'src> {
    /// Create a new lexer for the given source code
    pub fn new(source: &'src str) -> Self {
        Self {
            source,
            inner: TokenKind::lexer(source),
        }
    }

    /// Get the source code being lexed
    pub fn source(&self) -> &'src str {
        self.source
    }

    /// Tokenize the entire source into a vector of tokens
    pub fn tokenize(self) -> (Vec<Token>, Vec<LexerError>) {
        let mut tokens = Vec::new();
        let mut errors = Vec::new();

        for (result, span) in self.inner.spanned() {
            match result {
                Ok(kind) => {
                    tokens.push(Token::new(kind, Span::from(span)));
                }
                Err(_) => {
                    // Extract the problematic character for better error messages
                    let bad_char = self.source[span.clone()].chars().next().unwrap_or('?');
                    errors.push(LexerError::UnexpectedCharacter(span.start, bad_char));
                    tokens.push(Token::new(TokenKind::Error, Span::from(span)));
                }
            }
        }

        // Post-process to fix `ident?.` -> `ident` `?.`
        // When an identifier ends with `?` and is followed by `.`, split it
        let mut fixed_tokens = Vec::with_capacity(tokens.len());
        let mut i = 0;
        while i < tokens.len() {
            let token = &tokens[i];

            // Check if this is an identifier ending with `?` followed by `.`
            if let TokenKind::Identifier(name) = &token.kind {
                if name.ends_with('?') && i + 1 < tokens.len() {
                    if let TokenKind::Dot = tokens[i + 1].kind {
                        // Split: create identifier without `?` and QuestionDot
                        let name_without_q = &name[..name.len() - 1];
                        let ident_span = Span {
                            start: token.span.start,
                            end: token.span.end - 1,
                        };
                        let qdot_span = Span {
                            start: token.span.end - 1,
                            end: tokens[i + 1].span.end,
                        };

                        fixed_tokens.push(Token::new(
                            TokenKind::Identifier(SmolStr::new(name_without_q)),
                            ident_span,
                        ));
                        fixed_tokens.push(Token::new(TokenKind::QuestionDot, qdot_span));

                        i += 2; // Skip both the original identifier and the dot
                        continue;
                    }
                }
            }

            fixed_tokens.push(token.clone());
            i += 1;
        }

        (fixed_tokens, errors)
    }

    /// Tokenize, filtering out comments and collecting only non-trivia tokens
    pub fn tokenize_filtered(self) -> (Vec<Token>, Vec<LexerError>) {
        let (tokens, errors) = self.tokenize();
        let filtered: Vec<Token> = tokens
            .into_iter()
            .filter(|t| !t.kind.is_comment())
            .collect();
        (filtered, errors)
    }
}

impl<'src> Iterator for Lexer<'src> {
    type Item = std::result::Result<Token, LexerError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|result| {
            let span = self.inner.span();
            let bad_char = self.source[span.clone()].chars().next().unwrap_or('?');
            result
                .map(|kind| Token::new(kind, Span::from(span.clone())))
                .map_err(|_| LexerError::UnexpectedCharacter(span.start, bad_char))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keywords() {
        let source = "fn let mut if else match for while loop end";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert_eq!(tokens.len(), 10);
        assert_eq!(tokens[0].kind, TokenKind::Fn);
        assert_eq!(tokens[1].kind, TokenKind::Let);
        assert_eq!(tokens[9].kind, TokenKind::End);
    }

    #[test]
    fn test_operators() {
        let source = "+ - * / == != <= >= -> => |>";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert_eq!(tokens[0].kind, TokenKind::Plus);
        assert_eq!(tokens[4].kind, TokenKind::EqEq);
        assert_eq!(tokens[8].kind, TokenKind::Arrow);
        assert_eq!(tokens[10].kind, TokenKind::Pipe_);
    }

    #[test]
    fn test_integers() {
        let source = "42 1_000_000 0xFF 0b1010 42u64";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert!(matches!(&tokens[0].kind, TokenKind::Integer(s) if s == "42"));
        assert!(matches!(&tokens[1].kind, TokenKind::Integer(s) if s == "1_000_000"));
        assert!(matches!(&tokens[2].kind, TokenKind::Integer(s) if s == "0xFF"));
        assert!(matches!(&tokens[3].kind, TokenKind::Integer(s) if s == "0b1010"));
        assert!(matches!(&tokens[4].kind, TokenKind::Integer(s) if s == "42u64"));
    }

    #[test]
    fn test_floats() {
        let source = "3.14 1.0e-10 2.5f32";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert!(matches!(&tokens[0].kind, TokenKind::Float(s) if s == "3.14"));
        assert!(matches!(&tokens[1].kind, TokenKind::Float(s) if s == "1.0e-10"));
        assert!(matches!(&tokens[2].kind, TokenKind::Float(s) if s == "2.5f32"));
    }

    #[test]
    fn test_strings() {
        let source = r#"'simple' "interpolated" r"raw""#;
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert!(matches!(&tokens[0].kind, TokenKind::SimpleString(_)));
        assert!(matches!(&tokens[1].kind, TokenKind::InterpolatedString(_)));
        assert!(matches!(&tokens[2].kind, TokenKind::RawString(_)));
    }

    #[test]
    fn test_identifiers() {
        let source = "name empty? save! User MAX_SIZE";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert!(matches!(&tokens[0].kind, TokenKind::Identifier(s) if s == "name"));
        assert!(matches!(&tokens[1].kind, TokenKind::Identifier(s) if s == "empty?"));
        assert!(matches!(&tokens[2].kind, TokenKind::Identifier(s) if s == "save!"));
        assert!(matches!(&tokens[3].kind, TokenKind::TypeIdent(s) if s == "User"));
        assert!(matches!(&tokens[4].kind, TokenKind::ConstIdent(s) if s == "MAX_SIZE"));
    }

    #[test]
    fn test_comments() {
        let source = "# line comment\n## doc comment\ncode";
        let lexer = Lexer::new(source);
        let (tokens, _) = lexer.tokenize();

        assert!(matches!(&tokens[0].kind, TokenKind::LineComment(_)));
        assert_eq!(tokens[1].kind, TokenKind::Newline);
        assert!(matches!(&tokens[2].kind, TokenKind::DocComment(_)));
    }

    #[test]
    fn test_function_declaration() {
        let source = r#"fn add(a: Int, b: Int) -> Int
  a + b
end"#;
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert_eq!(tokens[0].kind, TokenKind::Fn);
        assert!(matches!(&tokens[1].kind, TokenKind::Identifier(s) if s == "add"));
        assert_eq!(tokens[2].kind, TokenKind::LParen);
    }

    // ========================================================================
    // Additional comprehensive tests for ARIA-IMPL-006
    // ========================================================================

    #[test]
    fn test_all_keywords() {
        let source = "struct data enum trait impl module import export from as extern unsafe defer spawn await select requires ensures invariant examples property forall exists true false nil self Self super and or not in is ref move copy pub priv break continue return test";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty(), "Errors: {:?}", errors);
        assert_eq!(tokens[0].kind, TokenKind::Struct);
        assert_eq!(tokens[1].kind, TokenKind::Data);
        assert_eq!(tokens[2].kind, TokenKind::Enum);
        assert_eq!(tokens[3].kind, TokenKind::Trait);
        assert_eq!(tokens[4].kind, TokenKind::Impl);
        assert_eq!(tokens[5].kind, TokenKind::Module);
        assert_eq!(tokens[6].kind, TokenKind::Import);
        assert_eq!(tokens[7].kind, TokenKind::Export);
    }

    #[test]
    fn test_bitwise_operators() {
        let source = "& | ^ ~ << >>";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert_eq!(tokens[0].kind, TokenKind::Amp);
        assert_eq!(tokens[1].kind, TokenKind::Pipe);
        assert_eq!(tokens[2].kind, TokenKind::Caret);
        assert_eq!(tokens[3].kind, TokenKind::Tilde);
        assert_eq!(tokens[4].kind, TokenKind::LtLt);
        assert_eq!(tokens[5].kind, TokenKind::GtGt);
    }

    #[test]
    fn test_assignment_operators() {
        let source = "= += -= *= /= //= %= &= |= ^= <<= >>=";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert_eq!(tokens[0].kind, TokenKind::Eq);
        assert_eq!(tokens[1].kind, TokenKind::PlusEq);
        assert_eq!(tokens[2].kind, TokenKind::MinusEq);
        assert_eq!(tokens[3].kind, TokenKind::StarEq);
        assert_eq!(tokens[4].kind, TokenKind::SlashEq);
    }

    #[test]
    fn test_range_operators() {
        let source = ".. ..< ..=";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert_eq!(tokens[0].kind, TokenKind::DotDot);
        assert_eq!(tokens[1].kind, TokenKind::DotDotLt);
        assert_eq!(tokens[2].kind, TokenKind::DotDotEq);
    }

    #[test]
    fn test_special_operators() {
        let source = ":: ? @ ...";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert_eq!(tokens[0].kind, TokenKind::ColonColon);
        assert_eq!(tokens[1].kind, TokenKind::Question);
        assert_eq!(tokens[2].kind, TokenKind::At);
        assert_eq!(tokens[3].kind, TokenKind::DotDotDot);
    }

    #[test]
    fn test_octal_integers() {
        let source = "0o755 0o0 0o777";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert!(matches!(&tokens[0].kind, TokenKind::Integer(s) if s == "0o755"));
        assert!(matches!(&tokens[1].kind, TokenKind::Integer(s) if s == "0o0"));
        assert!(matches!(&tokens[2].kind, TokenKind::Integer(s) if s == "0o777"));
    }

    #[test]
    fn test_integer_suffixes() {
        let source = "42i8 100i16 200i32 300i64 400u8 500u16 600u32 700u64";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert!(matches!(&tokens[0].kind, TokenKind::Integer(s) if s == "42i8"));
        assert!(matches!(&tokens[4].kind, TokenKind::Integer(s) if s == "400u8"));
    }

    #[test]
    fn test_float_exponents() {
        let source = "1e10 1E10 1e+10 1e-10 1.5e10 2.5E-5";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert!(matches!(&tokens[0].kind, TokenKind::Float(s) if s == "1e10"));
        assert!(matches!(&tokens[2].kind, TokenKind::Float(s) if s == "1e+10"));
        assert!(matches!(&tokens[3].kind, TokenKind::Float(s) if s == "1e-10"));
    }

    #[test]
    fn test_character_literals() {
        let source = r#"`a` `\n` `\t` `\\`"#;
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert!(matches!(&tokens[0].kind, TokenKind::Char(_)));
        assert!(matches!(&tokens[1].kind, TokenKind::Char(_)));
    }

    #[test]
    fn test_block_comments() {
        let source = "### block comment ###\ncode";
        let lexer = Lexer::new(source);
        let (tokens, _) = lexer.tokenize();

        assert!(matches!(&tokens[0].kind, TokenKind::BlockComment(_)));
    }

    #[test]
    fn test_comparison_operators() {
        let source = "== != < > <= >= <=> ~=";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert_eq!(tokens[0].kind, TokenKind::EqEq);
        assert_eq!(tokens[1].kind, TokenKind::NotEq);
        assert_eq!(tokens[2].kind, TokenKind::Lt);
        assert_eq!(tokens[3].kind, TokenKind::Gt);
        assert_eq!(tokens[4].kind, TokenKind::LtEq);
        assert_eq!(tokens[5].kind, TokenKind::GtEq);
    }

    #[test]
    fn test_logical_operators() {
        let source = "and or not && || !";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert_eq!(tokens[0].kind, TokenKind::And);
        assert_eq!(tokens[1].kind, TokenKind::Or);
        assert_eq!(tokens[2].kind, TokenKind::Not);
        assert_eq!(tokens[3].kind, TokenKind::AmpAmp);
        assert_eq!(tokens[4].kind, TokenKind::PipePipe);
        assert_eq!(tokens[5].kind, TokenKind::Bang);
    }

    #[test]
    fn test_arithmetic_operators() {
        let source = "+ - * / // % **";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert_eq!(tokens[0].kind, TokenKind::Plus);
        assert_eq!(tokens[1].kind, TokenKind::Minus);
        assert_eq!(tokens[2].kind, TokenKind::Star);
        assert_eq!(tokens[3].kind, TokenKind::Slash);
        assert_eq!(tokens[4].kind, TokenKind::SlashSlash);
        assert_eq!(tokens[5].kind, TokenKind::Percent);
        assert_eq!(tokens[6].kind, TokenKind::StarStar);
    }

    #[test]
    fn test_brackets_and_delimiters() {
        let source = "( ) [ ] { } , ; :";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert_eq!(tokens[0].kind, TokenKind::LParen);
        assert_eq!(tokens[1].kind, TokenKind::RParen);
        assert_eq!(tokens[2].kind, TokenKind::LBracket);
        assert_eq!(tokens[3].kind, TokenKind::RBracket);
        assert_eq!(tokens[4].kind, TokenKind::LBrace);
        assert_eq!(tokens[5].kind, TokenKind::RBrace);
        assert_eq!(tokens[6].kind, TokenKind::Comma);
        assert_eq!(tokens[7].kind, TokenKind::Semi);
        assert_eq!(tokens[8].kind, TokenKind::Colon);
    }

    #[test]
    fn test_underscore_separated_numbers() {
        let source = "1_000_000 0xFF_AA_BB 0b1010_1010";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert!(matches!(&tokens[0].kind, TokenKind::Integer(s) if s == "1_000_000"));
        assert!(matches!(&tokens[1].kind, TokenKind::Integer(s) if s == "0xFF_AA_BB"));
        assert!(matches!(&tokens[2].kind, TokenKind::Integer(s) if s == "0b1010_1010"));
    }

    #[test]
    fn test_private_identifier() {
        let source = "_private __dunder";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert!(matches!(&tokens[0].kind, TokenKind::Identifier(s) if s == "_private"));
        assert!(matches!(&tokens[1].kind, TokenKind::Identifier(s) if s == "__dunder"));
    }

    // ========================================================================
    // M19 Syntax Refinement - Additional Comprehensive Tests
    // ========================================================================

    #[test]
    fn test_newlines_preserved() {
        let source = "fn\nmain\n";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize();

        assert!(errors.is_empty());
        assert_eq!(tokens[0].kind, TokenKind::Fn);
        assert_eq!(tokens[1].kind, TokenKind::Newline);
        assert!(matches!(&tokens[2].kind, TokenKind::Identifier(s) if s == "main"));
        assert_eq!(tokens[3].kind, TokenKind::Newline);
    }

    #[test]
    fn test_question_dot_operator() {
        let source = "obj?.field";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert!(matches!(&tokens[0].kind, TokenKind::Identifier(s) if s == "obj"));
        assert_eq!(tokens[1].kind, TokenKind::QuestionDot);
        assert!(matches!(&tokens[2].kind, TokenKind::Identifier(s) if s == "field"));
    }

    #[test]
    fn test_left_arrow_operator() {
        let source = "x <- y";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert!(matches!(&tokens[0].kind, TokenKind::Identifier(_)));
        assert_eq!(tokens[1].kind, TokenKind::LeftArrow);
        assert!(matches!(&tokens[2].kind, TokenKind::Identifier(_)));
    }

    #[test]
    fn test_at_bang_operator() {
        let source = "@! unsafe_call";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert_eq!(tokens[0].kind, TokenKind::AtBang);
        assert!(matches!(&tokens[1].kind, TokenKind::Identifier(s) if s == "unsafe_call"));
    }

    #[test]
    fn test_amp_colon_operator() {
        let source = "&:callback";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert_eq!(tokens[0].kind, TokenKind::AmpColon);
        assert!(matches!(&tokens[1].kind, TokenKind::Identifier(s) if s == "callback"));
    }

    #[test]
    fn test_string_escape_sequences() {
        let source = r#"'hello\nworld' "tab\there""#;
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert!(matches!(&tokens[0].kind, TokenKind::SimpleString(_)));
        assert!(matches!(&tokens[1].kind, TokenKind::InterpolatedString(_)));
    }

    #[test]
    fn test_raw_string_no_escapes() {
        let source = r#"r"no \n escape""#;
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert!(matches!(&tokens[0].kind, TokenKind::RawString(s) if s.contains(r"\n")));
    }

    #[test]
    fn test_spaceship_operator() {
        let source = "a <=> b";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert!(matches!(&tokens[0].kind, TokenKind::Identifier(_)));
        assert_eq!(tokens[1].kind, TokenKind::Spaceship);
        assert!(matches!(&tokens[2].kind, TokenKind::Identifier(_)));
    }

    #[test]
    fn test_multi_char_assignment_ops() {
        let source = "+= -= *= /= //= %= &= |= ^= <<= >>=";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert_eq!(tokens[0].kind, TokenKind::PlusEq);
        assert_eq!(tokens[1].kind, TokenKind::MinusEq);
        assert_eq!(tokens[2].kind, TokenKind::StarEq);
        assert_eq!(tokens[3].kind, TokenKind::SlashEq);
        assert_eq!(tokens[4].kind, TokenKind::SlashSlashEq);
        assert_eq!(tokens[5].kind, TokenKind::PercentEq);
        assert_eq!(tokens[6].kind, TokenKind::AmpEq);
        assert_eq!(tokens[7].kind, TokenKind::PipeEq);
        assert_eq!(tokens[8].kind, TokenKind::CaretEq);
        assert_eq!(tokens[9].kind, TokenKind::LtLtEq);
        assert_eq!(tokens[10].kind, TokenKind::GtGtEq);
    }

    #[test]
    fn test_all_control_flow_keywords() {
        let source = "if else elsif unless match for while loop break continue return";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert_eq!(tokens[0].kind, TokenKind::If);
        assert_eq!(tokens[1].kind, TokenKind::Else);
        assert_eq!(tokens[2].kind, TokenKind::Elsif);
        assert_eq!(tokens[3].kind, TokenKind::Unless);
        assert_eq!(tokens[4].kind, TokenKind::Match);
        assert_eq!(tokens[5].kind, TokenKind::For);
        assert_eq!(tokens[6].kind, TokenKind::While);
        assert_eq!(tokens[7].kind, TokenKind::Loop);
        assert_eq!(tokens[8].kind, TokenKind::Break);
        assert_eq!(tokens[9].kind, TokenKind::Continue);
        assert_eq!(tokens[10].kind, TokenKind::Return);
    }

    #[test]
    fn test_contract_keywords() {
        // Note: 'result' was removed as a keyword to allow it as a variable name
        let source = "requires ensures invariant old raises";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert_eq!(tokens[0].kind, TokenKind::Requires);
        assert_eq!(tokens[1].kind, TokenKind::Ensures);
        assert_eq!(tokens[2].kind, TokenKind::Invariant);
        assert_eq!(tokens[3].kind, TokenKind::Old);
        assert_eq!(tokens[4].kind, TokenKind::Raises);
    }

    #[test]
    fn test_quantifier_keywords() {
        let source = "forall exists";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert_eq!(tokens[0].kind, TokenKind::Forall);
        assert_eq!(tokens[1].kind, TokenKind::Exists);
    }

    #[test]
    fn test_concurrency_keywords() {
        let source = "spawn await select";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert_eq!(tokens[0].kind, TokenKind::Spawn);
        assert_eq!(tokens[1].kind, TokenKind::Await);
        assert_eq!(tokens[2].kind, TokenKind::Select);
    }

    #[test]
    fn test_identifier_with_question_bang() {
        let source = "empty? valid? save! update!";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert!(matches!(&tokens[0].kind, TokenKind::Identifier(s) if s == "empty?"));
        assert!(matches!(&tokens[1].kind, TokenKind::Identifier(s) if s == "valid?"));
        assert!(matches!(&tokens[2].kind, TokenKind::Identifier(s) if s == "save!"));
        assert!(matches!(&tokens[3].kind, TokenKind::Identifier(s) if s == "update!"));
    }

    #[test]
    fn test_mixed_case_type_identifier() {
        let source = "MyClass HashMap LinkedList";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert!(matches!(&tokens[0].kind, TokenKind::TypeIdent(s) if s == "MyClass"));
        assert!(matches!(&tokens[1].kind, TokenKind::TypeIdent(s) if s == "HashMap"));
        assert!(matches!(&tokens[2].kind, TokenKind::TypeIdent(s) if s == "LinkedList"));
    }

    #[test]
    fn test_error_token_for_invalid_char() {
        let source = "valid $ invalid";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert_eq!(errors.len(), 1);
        // Should still have tokens for valid parts plus the error token
        assert!(matches!(&tokens[0].kind, TokenKind::Identifier(s) if s == "valid"));
        assert!(matches!(&tokens[1].kind, TokenKind::Error));
        assert!(matches!(&tokens[2].kind, TokenKind::Identifier(s) if s == "invalid"));
    }

    #[test]
    fn test_nested_block_comment() {
        // Note: Block comments in Aria use ###...###
        let source = "before ### comment text ### after";
        let lexer = Lexer::new(source);
        let (tokens, _) = lexer.tokenize();

        // Should have: before, block comment, after
        assert!(matches!(&tokens[0].kind, TokenKind::Identifier(s) if s == "before"));
        assert!(matches!(&tokens[1].kind, TokenKind::BlockComment(_)));
        assert!(matches!(&tokens[2].kind, TokenKind::Identifier(s) if s == "after"));
    }

    #[test]
    fn test_consecutive_operators() {
        // Test that operators next to each other tokenize correctly
        let source = "a+-b";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert!(matches!(&tokens[0].kind, TokenKind::Identifier(s) if s == "a"));
        assert_eq!(tokens[1].kind, TokenKind::Plus);
        assert_eq!(tokens[2].kind, TokenKind::Minus);
        assert!(matches!(&tokens[3].kind, TokenKind::Identifier(s) if s == "b"));
    }

    #[test]
    fn test_float_with_leading_zero() {
        let source = "0.5 0.123 0.0";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert!(matches!(&tokens[0].kind, TokenKind::Float(s) if s == "0.5"));
        assert!(matches!(&tokens[1].kind, TokenKind::Float(s) if s == "0.123"));
        assert!(matches!(&tokens[2].kind, TokenKind::Float(s) if s == "0.0"));
    }

    #[test]
    fn test_i128_u128_suffixes() {
        let source = "42i128 42u128";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert!(matches!(&tokens[0].kind, TokenKind::Integer(s) if s == "42i128"));
        assert!(matches!(&tokens[1].kind, TokenKind::Integer(s) if s == "42u128"));
    }

    #[test]
    fn test_isize_usize_suffixes() {
        let source = "100isize 200usize";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert!(matches!(&tokens[0].kind, TokenKind::Integer(s) if s == "100isize"));
        assert!(matches!(&tokens[1].kind, TokenKind::Integer(s) if s == "200usize"));
    }

    #[test]
    fn test_doc_comment_format() {
        let source = "## This is a doc comment\nfn foo()";
        let lexer = Lexer::new(source);
        let (tokens, _) = lexer.tokenize();

        assert!(matches!(&tokens[0].kind, TokenKind::DocComment(s) if s.contains("This is a doc comment")));
    }

    #[test]
    fn test_then_keyword() {
        let source = "if condition then value else other";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert_eq!(tokens[0].kind, TokenKind::If);
        assert!(matches!(&tokens[1].kind, TokenKind::Identifier(s) if s == "condition"));
        assert_eq!(tokens[2].kind, TokenKind::Then);
        assert!(matches!(&tokens[3].kind, TokenKind::Identifier(s) if s == "value"));
        assert_eq!(tokens[4].kind, TokenKind::Else);
        assert!(matches!(&tokens[5].kind, TokenKind::Identifier(s) if s == "other"));
    }

    #[test]
    fn test_default_keyword() {
        let source = "default case";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert_eq!(tokens[0].kind, TokenKind::Default);
        assert!(matches!(&tokens[1].kind, TokenKind::Identifier(s) if s == "case"));
    }

    #[test]
    fn test_tilde_eq_operator() {
        let source = "a ~= b";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        assert!(matches!(&tokens[0].kind, TokenKind::Identifier(_)));
        assert_eq!(tokens[1].kind, TokenKind::TildeEq);
        assert!(matches!(&tokens[2].kind, TokenKind::Identifier(_)));
    }

    #[test]
    fn test_token_display() {
        // Ensure Display trait works for various tokens
        assert_eq!(format!("{}", TokenKind::Fn), "fn");
        assert_eq!(format!("{}", TokenKind::Plus), "+");
        assert_eq!(format!("{}", TokenKind::Arrow), "->");
        assert_eq!(format!("{}", TokenKind::FatArrow), "=>");
        assert_eq!(format!("{}", TokenKind::ColonColon), "::");
        assert_eq!(format!("{}", TokenKind::Pipe_), "|>");
    }

    #[test]
    fn test_span_correctness() {
        let source = "let x = 42";
        let lexer = Lexer::new(source);
        let (tokens, errors) = lexer.tokenize_filtered();

        assert!(errors.is_empty());
        // "let" should span 0..3
        assert_eq!(tokens[0].span.start, 0);
        assert_eq!(tokens[0].span.end, 3);
        // "x" should span 4..5
        assert_eq!(tokens[1].span.start, 4);
        assert_eq!(tokens[1].span.end, 5);
        // "=" should span 6..7
        assert_eq!(tokens[2].span.start, 6);
        assert_eq!(tokens[2].span.end, 7);
        // "42" should span 8..10
        assert_eq!(tokens[3].span.start, 8);
        assert_eq!(tokens[3].span.end, 10);
    }
}
