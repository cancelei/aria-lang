//! C Header Parser for Aria FFI
//!
//! A simple C header parser that extracts declarations without using libclang.
//! This parser handles common C constructs:
//! - Struct definitions
//! - Function declarations
//! - Typedefs
//! - Enum definitions
//! - Constant definitions (#define macros with simple values)
//!
//! ## Design Philosophy
//!
//! This is a pragmatic parser for common patterns, not a full C parser.
//! Complex macros and preprocessor directives are handled minimally.
//!
//! Based on ARIA-M09: C Interop System

use std::collections::HashMap;
use std::fmt;

// ============================================================================
// Token Types
// ============================================================================

/// Token types for the C lexer
#[derive(Debug, Clone, PartialEq)]
pub enum CToken {
    // Keywords
    Struct,
    Union,
    Enum,
    Typedef,
    Const,
    Volatile,
    Extern,
    Static,
    Inline,
    Unsigned,
    Signed,
    Void,
    Char,
    Short,
    Int,
    Long,
    Float,
    Double,
    Bool,
    SizeT,

    // Identifiers and literals
    Identifier(String),
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    CharLiteral(char),

    // Operators and punctuation
    Star,           // *
    Ampersand,      // &
    Semicolon,      // ;
    Comma,          // ,
    OpenBrace,      // {
    CloseBrace,     // }
    OpenParen,      // (
    CloseParen,     // )
    OpenBracket,    // [
    CloseBracket,   // ]
    Equals,         // =

    // Preprocessor
    Define,         // #define
    Include,        // #include
    Ifdef,          // #ifdef
    Ifndef,         // #ifndef
    Endif,          // #endif

    // Special
    Ellipsis,       // ...
    Arrow,          // ->

    // End of input
    Eof,
}

// ============================================================================
// C Type Representation
// ============================================================================

/// Represents a C type
#[derive(Debug, Clone, PartialEq)]
pub enum CType {
    /// Void type
    Void,
    /// Character type
    Char,
    /// Unsigned character
    UChar,
    /// Signed character
    SChar,
    /// Short integer
    Short,
    /// Unsigned short
    UShort,
    /// Integer
    Int,
    /// Unsigned integer
    UInt,
    /// Long integer
    Long,
    /// Unsigned long
    ULong,
    /// Long long integer
    LongLong,
    /// Unsigned long long
    ULongLong,
    /// Float
    Float,
    /// Double
    Double,
    /// Long double
    LongDouble,
    /// Bool (C99 _Bool)
    Bool,
    /// size_t
    SizeT,
    /// ssize_t
    SSizeT,
    /// intptr_t
    IntPtrT,
    /// uintptr_t
    UIntPtrT,
    /// ptrdiff_t
    PtrDiffT,
    /// Pointer to another type
    Pointer(Box<CType>),
    /// Const-qualified type
    Const(Box<CType>),
    /// Volatile-qualified type
    Volatile(Box<CType>),
    /// Array type with optional size
    Array(Box<CType>, Option<usize>),
    /// Function pointer: return type, parameter types, is_variadic
    FunctionPointer {
        return_type: Box<CType>,
        params: Vec<CType>,
        variadic: bool,
    },
    /// Reference to a named type (struct, union, enum, typedef)
    Named(String),
    /// Struct type with fields
    Struct {
        name: Option<String>,
        fields: Vec<CStructField>,
    },
    /// Union type
    Union {
        name: Option<String>,
        fields: Vec<CStructField>,
    },
    /// Enum type
    Enum {
        name: Option<String>,
        variants: Vec<CEnumVariant>,
    },
}

impl CType {
    /// Check if this type is a pointer type
    pub fn is_pointer(&self) -> bool {
        matches!(self, CType::Pointer(_))
    }

    /// Check if this type is const-qualified
    pub fn is_const(&self) -> bool {
        matches!(self, CType::Const(_))
    }

    /// Get the base type, stripping qualifiers and pointers
    pub fn base_type(&self) -> &CType {
        match self {
            CType::Pointer(inner) => inner.base_type(),
            CType::Const(inner) => inner.base_type(),
            CType::Volatile(inner) => inner.base_type(),
            CType::Array(inner, _) => inner.base_type(),
            _ => self,
        }
    }
}

impl fmt::Display for CType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CType::Void => write!(f, "void"),
            CType::Char => write!(f, "char"),
            CType::UChar => write!(f, "unsigned char"),
            CType::SChar => write!(f, "signed char"),
            CType::Short => write!(f, "short"),
            CType::UShort => write!(f, "unsigned short"),
            CType::Int => write!(f, "int"),
            CType::UInt => write!(f, "unsigned int"),
            CType::Long => write!(f, "long"),
            CType::ULong => write!(f, "unsigned long"),
            CType::LongLong => write!(f, "long long"),
            CType::ULongLong => write!(f, "unsigned long long"),
            CType::Float => write!(f, "float"),
            CType::Double => write!(f, "double"),
            CType::LongDouble => write!(f, "long double"),
            CType::Bool => write!(f, "_Bool"),
            CType::SizeT => write!(f, "size_t"),
            CType::SSizeT => write!(f, "ssize_t"),
            CType::IntPtrT => write!(f, "intptr_t"),
            CType::UIntPtrT => write!(f, "uintptr_t"),
            CType::PtrDiffT => write!(f, "ptrdiff_t"),
            CType::Pointer(inner) => write!(f, "{}*", inner),
            CType::Const(inner) => write!(f, "const {}", inner),
            CType::Volatile(inner) => write!(f, "volatile {}", inner),
            CType::Array(inner, Some(n)) => write!(f, "{}[{}]", inner, n),
            CType::Array(inner, None) => write!(f, "{}[]", inner),
            CType::FunctionPointer { return_type, params, variadic } => {
                write!(f, "{} (*)(", return_type)?;
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", param)?;
                }
                if *variadic {
                    if !params.is_empty() {
                        write!(f, ", ")?;
                    }
                    write!(f, "...")?;
                }
                write!(f, ")")
            }
            CType::Named(name) => write!(f, "{}", name),
            CType::Struct { name: Some(n), .. } => write!(f, "struct {}", n),
            CType::Struct { name: None, .. } => write!(f, "struct {{ ... }}"),
            CType::Union { name: Some(n), .. } => write!(f, "union {}", n),
            CType::Union { name: None, .. } => write!(f, "union {{ ... }}"),
            CType::Enum { name: Some(n), .. } => write!(f, "enum {}", n),
            CType::Enum { name: None, .. } => write!(f, "enum {{ ... }}"),
        }
    }
}

/// A field in a C struct or union
#[derive(Debug, Clone, PartialEq)]
pub struct CStructField {
    /// Field name
    pub name: String,
    /// Field type
    pub ty: CType,
    /// Bit field width (if applicable)
    pub bit_width: Option<u32>,
}

/// An enum variant
#[derive(Debug, Clone, PartialEq)]
pub struct CEnumVariant {
    /// Variant name
    pub name: String,
    /// Explicit value (if specified)
    pub value: Option<i64>,
}

// ============================================================================
// C Declarations
// ============================================================================

/// A C declaration (function, struct, typedef, etc.)
#[derive(Debug, Clone, PartialEq)]
pub enum CDeclaration {
    /// Function declaration
    Function(CFunctionDecl),
    /// Struct definition
    Struct(CStructDecl),
    /// Union definition
    Union(CUnionDecl),
    /// Enum definition
    Enum(CEnumDecl),
    /// Typedef
    Typedef(CTypedefDecl),
    /// Constant (from #define)
    Constant(CConstantDecl),
    /// Variable declaration (extern)
    Variable(CVariableDecl),
}

/// C function declaration
#[derive(Debug, Clone, PartialEq)]
pub struct CFunctionDecl {
    /// Function name
    pub name: String,
    /// Return type
    pub return_type: CType,
    /// Parameters
    pub params: Vec<CFunctionParam>,
    /// Is variadic (has ...)
    pub variadic: bool,
    /// Is inline
    pub is_inline: bool,
    /// Is static
    pub is_static: bool,
}

/// Function parameter
#[derive(Debug, Clone, PartialEq)]
pub struct CFunctionParam {
    /// Parameter name (may be empty for declarations)
    pub name: Option<String>,
    /// Parameter type
    pub ty: CType,
}

/// C struct declaration
#[derive(Debug, Clone, PartialEq)]
pub struct CStructDecl {
    /// Struct name
    pub name: String,
    /// Fields
    pub fields: Vec<CStructField>,
}

/// C union declaration
#[derive(Debug, Clone, PartialEq)]
pub struct CUnionDecl {
    /// Union name
    pub name: String,
    /// Fields
    pub fields: Vec<CStructField>,
}

/// C enum declaration
#[derive(Debug, Clone, PartialEq)]
pub struct CEnumDecl {
    /// Enum name
    pub name: String,
    /// Variants
    pub variants: Vec<CEnumVariant>,
}

/// C typedef declaration
#[derive(Debug, Clone, PartialEq)]
pub struct CTypedefDecl {
    /// New type name
    pub name: String,
    /// Underlying type
    pub underlying_type: CType,
}

/// C constant (#define NAME value)
#[derive(Debug, Clone, PartialEq)]
pub struct CConstantDecl {
    /// Constant name
    pub name: String,
    /// Constant value
    pub value: CConstantValue,
}

/// Constant value from #define
#[derive(Debug, Clone, PartialEq)]
pub enum CConstantValue {
    Integer(i64),
    Float(f64),
    String(String),
    Char(char),
    /// Expression we couldn't parse
    Unknown(String),
}

/// C variable declaration (extern variables)
#[derive(Debug, Clone, PartialEq)]
pub struct CVariableDecl {
    /// Variable name
    pub name: String,
    /// Variable type
    pub ty: CType,
    /// Is extern
    pub is_extern: bool,
    /// Is const
    pub is_const: bool,
}

// ============================================================================
// Lexer
// ============================================================================

/// C lexer for tokenizing C header files
pub struct CLexer<'a> {
    input: &'a str,
    position: usize,
    line: usize,
    column: usize,
}

impl<'a> CLexer<'a> {
    /// Create a new lexer
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            position: 0,
            line: 1,
            column: 1,
        }
    }

    /// Get the current position
    pub fn position(&self) -> (usize, usize) {
        (self.line, self.column)
    }

    /// Peek at the current character without consuming
    fn peek(&self) -> Option<char> {
        self.input[self.position..].chars().next()
    }

    /// Peek at the next character
    fn peek_next(&self) -> Option<char> {
        self.input[self.position..].chars().nth(1)
    }

    /// Advance and consume the current character
    fn advance(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.position += c.len_utf8();
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(c)
    }

    /// Skip whitespace and comments
    fn skip_whitespace_and_comments(&mut self) {
        loop {
            // Skip whitespace
            while self.peek().map_or(false, |c| c.is_whitespace()) {
                self.advance();
            }

            // Skip C-style comments
            if self.peek() == Some('/') {
                if self.peek_next() == Some('/') {
                    // Line comment
                    while self.peek().map_or(false, |c| c != '\n') {
                        self.advance();
                    }
                    continue;
                } else if self.peek_next() == Some('*') {
                    // Block comment
                    self.advance(); // /
                    self.advance(); // *
                    while !(self.peek() == Some('*') && self.peek_next() == Some('/')) {
                        if self.advance().is_none() {
                            break;
                        }
                    }
                    self.advance(); // *
                    self.advance(); // /
                    continue;
                }
            }

            break;
        }
    }

    /// Read an identifier
    fn read_identifier(&mut self) -> String {
        let mut ident = String::new();
        while self.peek().map_or(false, |c| c.is_alphanumeric() || c == '_') {
            ident.push(self.advance().unwrap());
        }
        ident
    }

    /// Read a number literal
    fn read_number(&mut self) -> CToken {
        let mut num_str = String::new();
        let mut is_float = false;
        let mut is_hex = false;

        // Check for hex prefix
        if self.peek() == Some('0') && self.peek_next().map_or(false, |c| c == 'x' || c == 'X') {
            num_str.push(self.advance().unwrap()); // 0
            num_str.push(self.advance().unwrap()); // x
            is_hex = true;
        }

        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || (is_hex && c.is_ascii_hexdigit()) {
                num_str.push(self.advance().unwrap());
            } else if c == '.' && !is_float && !is_hex {
                is_float = true;
                num_str.push(self.advance().unwrap());
            } else if (c == 'e' || c == 'E') && !is_hex {
                is_float = true;
                num_str.push(self.advance().unwrap());
                if self.peek() == Some('-') || self.peek() == Some('+') {
                    num_str.push(self.advance().unwrap());
                }
            } else {
                break;
            }
        }

        // Skip suffixes (L, LL, U, UL, ULL, F, etc.)
        while self.peek().map_or(false, |c| matches!(c, 'L' | 'l' | 'U' | 'u' | 'F' | 'f')) {
            self.advance();
        }

        if is_float {
            CToken::FloatLiteral(num_str.parse().unwrap_or(0.0))
        } else if is_hex {
            let hex_part = &num_str[2..]; // Skip "0x"
            CToken::IntLiteral(i64::from_str_radix(hex_part, 16).unwrap_or(0))
        } else {
            CToken::IntLiteral(num_str.parse().unwrap_or(0))
        }
    }

    /// Read a string literal
    fn read_string(&mut self) -> String {
        let mut s = String::new();
        self.advance(); // Opening quote
        while let Some(c) = self.peek() {
            if c == '"' {
                self.advance();
                break;
            } else if c == '\\' {
                self.advance();
                if let Some(escaped) = self.advance() {
                    match escaped {
                        'n' => s.push('\n'),
                        't' => s.push('\t'),
                        'r' => s.push('\r'),
                        '\\' => s.push('\\'),
                        '"' => s.push('"'),
                        '0' => s.push('\0'),
                        _ => s.push(escaped),
                    }
                }
            } else {
                s.push(self.advance().unwrap());
            }
        }
        s
    }

    /// Read a character literal
    fn read_char(&mut self) -> char {
        self.advance(); // Opening quote
        let c = if self.peek() == Some('\\') {
            self.advance();
            match self.advance() {
                Some('n') => '\n',
                Some('t') => '\t',
                Some('r') => '\r',
                Some('\\') => '\\',
                Some('\'') => '\'',
                Some('0') => '\0',
                Some(c) => c,
                None => '\0',
            }
        } else {
            self.advance().unwrap_or('\0')
        };
        self.advance(); // Closing quote
        c
    }

    /// Read a preprocessor directive
    fn read_preprocessor(&mut self) -> CToken {
        self.advance(); // #
        self.skip_whitespace_and_comments();
        let directive = self.read_identifier();

        match directive.as_str() {
            "define" => CToken::Define,
            "include" => CToken::Include,
            "ifdef" => CToken::Ifdef,
            "ifndef" => CToken::Ifndef,
            "endif" => CToken::Endif,
            _ => {
                // Skip the rest of the line for unknown preprocessor directives
                while self.peek().map_or(false, |c| c != '\n') {
                    self.advance();
                }
                self.next_token()
            }
        }
    }

    /// Get the next token
    pub fn next_token(&mut self) -> CToken {
        self.skip_whitespace_and_comments();

        match self.peek() {
            None => CToken::Eof,
            Some('#') => self.read_preprocessor(),
            Some('"') => CToken::StringLiteral(self.read_string()),
            Some('\'') => CToken::CharLiteral(self.read_char()),
            Some(c) if c.is_ascii_digit() => self.read_number(),
            Some(c) if c.is_alphabetic() || c == '_' => {
                let ident = self.read_identifier();
                match ident.as_str() {
                    "struct" => CToken::Struct,
                    "union" => CToken::Union,
                    "enum" => CToken::Enum,
                    "typedef" => CToken::Typedef,
                    "const" => CToken::Const,
                    "volatile" => CToken::Volatile,
                    "extern" => CToken::Extern,
                    "static" => CToken::Static,
                    "inline" => CToken::Inline,
                    "__inline" => CToken::Inline,
                    "__inline__" => CToken::Inline,
                    "unsigned" => CToken::Unsigned,
                    "signed" => CToken::Signed,
                    "void" => CToken::Void,
                    "char" => CToken::Char,
                    "short" => CToken::Short,
                    "int" => CToken::Int,
                    "long" => CToken::Long,
                    "float" => CToken::Float,
                    "double" => CToken::Double,
                    "_Bool" | "bool" => CToken::Bool,
                    "size_t" => CToken::SizeT,
                    _ => CToken::Identifier(ident),
                }
            }
            Some('*') => {
                self.advance();
                CToken::Star
            }
            Some('&') => {
                self.advance();
                CToken::Ampersand
            }
            Some(';') => {
                self.advance();
                CToken::Semicolon
            }
            Some(',') => {
                self.advance();
                CToken::Comma
            }
            Some('{') => {
                self.advance();
                CToken::OpenBrace
            }
            Some('}') => {
                self.advance();
                CToken::CloseBrace
            }
            Some('(') => {
                self.advance();
                CToken::OpenParen
            }
            Some(')') => {
                self.advance();
                CToken::CloseParen
            }
            Some('[') => {
                self.advance();
                CToken::OpenBracket
            }
            Some(']') => {
                self.advance();
                CToken::CloseBracket
            }
            Some('=') => {
                self.advance();
                CToken::Equals
            }
            Some('.') => {
                if self.peek_next() == Some('.') {
                    self.advance();
                    self.advance();
                    if self.peek() == Some('.') {
                        self.advance();
                        CToken::Ellipsis
                    } else {
                        // Just skip unknown tokens
                        self.next_token()
                    }
                } else {
                    self.advance();
                    self.next_token()
                }
            }
            Some('-') => {
                self.advance();
                if self.peek() == Some('>') {
                    self.advance();
                    CToken::Arrow
                } else {
                    self.next_token()
                }
            }
            Some(_) => {
                // Skip unknown characters
                self.advance();
                self.next_token()
            }
        }
    }

    /// Peek at the next token without consuming it
    pub fn peek_token(&mut self) -> CToken {
        let saved_position = self.position;
        let saved_line = self.line;
        let saved_column = self.column;
        let token = self.next_token();
        self.position = saved_position;
        self.line = saved_line;
        self.column = saved_column;
        token
    }
}

// ============================================================================
// Parser
// ============================================================================

/// Parse error
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.column, self.message)
    }
}

impl std::error::Error for ParseError {}

/// C header parser
pub struct CHeaderParser<'a> {
    lexer: CLexer<'a>,
    current_token: CToken,
    /// Type aliases from typedefs
    type_aliases: HashMap<String, CType>,
}

impl<'a> CHeaderParser<'a> {
    /// Create a new parser
    pub fn new(input: &'a str) -> Self {
        let mut lexer = CLexer::new(input);
        let current_token = lexer.next_token();
        Self {
            lexer,
            current_token,
            type_aliases: HashMap::new(),
        }
    }

    /// Get all registered type aliases
    pub fn type_aliases(&self) -> &HashMap<String, CType> {
        &self.type_aliases
    }

    /// Advance to the next token
    fn advance(&mut self) {
        self.current_token = self.lexer.next_token();
    }

    /// Expect a specific token
    fn expect(&mut self, expected: CToken) -> Result<(), ParseError> {
        if std::mem::discriminant(&self.current_token) == std::mem::discriminant(&expected) {
            self.advance();
            Ok(())
        } else {
            let (line, column) = self.lexer.position();
            Err(ParseError {
                message: format!("Expected {:?}, found {:?}", expected, self.current_token),
                line,
                column,
            })
        }
    }

    /// Create an error at the current position
    fn error(&self, message: impl Into<String>) -> ParseError {
        let (line, column) = self.lexer.position();
        ParseError {
            message: message.into(),
            line,
            column,
        }
    }

    /// Parse the entire header file
    pub fn parse(&mut self) -> Result<Vec<CDeclaration>, ParseError> {
        let mut declarations = Vec::new();

        while self.current_token != CToken::Eof {
            match self.parse_declaration() {
                Ok(Some(decl)) => declarations.push(decl),
                Ok(None) => {} // Skipped declaration
                Err(e) => {
                    // Try to recover by skipping to the next semicolon or brace
                    self.skip_to_recovery_point();
                    // Log the error but continue parsing
                    eprintln!("Warning: {}", e);
                }
            }
        }

        Ok(declarations)
    }

    /// Skip to a recovery point (semicolon or closing brace)
    fn skip_to_recovery_point(&mut self) {
        let mut brace_depth = 0;
        loop {
            match &self.current_token {
                CToken::Eof => break,
                CToken::Semicolon if brace_depth == 0 => {
                    self.advance();
                    break;
                }
                CToken::OpenBrace => {
                    brace_depth += 1;
                    self.advance();
                }
                CToken::CloseBrace => {
                    if brace_depth > 0 {
                        brace_depth -= 1;
                    }
                    self.advance();
                    if brace_depth == 0 {
                        // Check for semicolon after closing brace
                        if self.current_token == CToken::Semicolon {
                            self.advance();
                        }
                        break;
                    }
                }
                _ => self.advance(),
            }
        }
    }

    /// Parse a single declaration
    fn parse_declaration(&mut self) -> Result<Option<CDeclaration>, ParseError> {
        match &self.current_token {
            CToken::Define => self.parse_define(),
            CToken::Include => {
                // Skip includes
                self.skip_line();
                Ok(None)
            }
            CToken::Ifdef | CToken::Ifndef | CToken::Endif => {
                // Skip conditional compilation
                self.skip_line();
                Ok(None)
            }
            CToken::Typedef => self.parse_typedef(),
            CToken::Struct => self.parse_struct_or_function(),
            CToken::Union => self.parse_union_decl(),
            CToken::Enum => self.parse_enum_decl(),
            CToken::Extern => self.parse_extern_decl(),
            CToken::Static | CToken::Inline => {
                // Skip static/inline and parse the rest
                self.advance();
                self.parse_declaration()
            }
            CToken::Identifier(_)
            | CToken::Void
            | CToken::Char
            | CToken::Short
            | CToken::Int
            | CToken::Long
            | CToken::Float
            | CToken::Double
            | CToken::Unsigned
            | CToken::Signed
            | CToken::Const
            | CToken::SizeT
            | CToken::Bool => self.parse_function_or_variable(),
            CToken::Eof => Ok(None),
            _ => {
                self.advance();
                Ok(None)
            }
        }
    }

    /// Skip to end of line (for preprocessor directives)
    fn skip_line(&mut self) {
        // For preprocessor directives, we need to handle line continuation
        loop {
            self.advance();
            match &self.current_token {
                CToken::Eof => break,
                // If we hit something that starts a new declaration, stop
                CToken::Define
                | CToken::Include
                | CToken::Ifdef
                | CToken::Ifndef
                | CToken::Endif
                | CToken::Typedef
                | CToken::Struct
                | CToken::Union
                | CToken::Enum
                | CToken::Extern => break,
                _ => continue,
            }
        }
    }

    /// Parse a #define
    fn parse_define(&mut self) -> Result<Option<CDeclaration>, ParseError> {
        self.advance(); // #define

        let name = match &self.current_token {
            CToken::Identifier(s) => s.clone(),
            _ => {
                // Not a valid define, but don't skip - let the main loop handle
                return Ok(None);
            }
        };
        self.advance();

        // Check if it's a macro with arguments
        if self.current_token == CToken::OpenParen {
            // Skip macro definitions with arguments
            self.skip_to_recovery_point();
            return Ok(None);
        }

        // Try to parse the value
        // If the next token is a preprocessor directive or declarative keyword,
        // this is a flag-style define (no value)
        let value = match &self.current_token {
            CToken::IntLiteral(n) => CConstantValue::Integer(*n),
            CToken::FloatLiteral(n) => CConstantValue::Float(*n),
            CToken::StringLiteral(s) => CConstantValue::String(s.clone()),
            CToken::CharLiteral(c) => CConstantValue::Char(*c),
            CToken::Identifier(s) => CConstantValue::Unknown(s.clone()),
            // Flag-style defines (no value) - just return None without skipping
            CToken::Define
            | CToken::Include
            | CToken::Ifdef
            | CToken::Ifndef
            | CToken::Endif
            | CToken::Typedef
            | CToken::Struct
            | CToken::Union
            | CToken::Enum
            | CToken::Extern
            | CToken::Eof => {
                return Ok(None);
            }
            _ => {
                // Unknown value type - return as unknown string
                return Ok(None);
            }
        };
        self.advance();

        Ok(Some(CDeclaration::Constant(CConstantDecl { name, value })))
    }

    /// Parse a typedef
    fn parse_typedef(&mut self) -> Result<Option<CDeclaration>, ParseError> {
        self.advance(); // typedef

        // Handle typedef struct/union/enum
        match &self.current_token {
            CToken::Struct => {
                self.advance();
                return self.parse_typedef_struct();
            }
            CToken::Union => {
                self.advance();
                return self.parse_typedef_union();
            }
            CToken::Enum => {
                self.advance();
                return self.parse_typedef_enum();
            }
            _ => {}
        }

        // Parse the underlying type
        let underlying_type = self.parse_type()?;

        // Get the new name - it could be an identifier or a known type name being redefined
        let name = match &self.current_token {
            CToken::Identifier(s) => s.clone(),
            // Handle common type names that might be redefined
            CToken::SizeT => "size_t".to_string(),
            CToken::Bool => "bool".to_string(),
            _ => return Err(self.error("Expected typedef name")),
        };
        self.advance();

        // Handle array types in typedef
        let final_type = if self.current_token == CToken::OpenBracket {
            self.parse_array_suffix(underlying_type)?
        } else {
            underlying_type
        };

        self.expect(CToken::Semicolon)?;

        // Register the type alias
        self.type_aliases.insert(name.clone(), final_type.clone());

        Ok(Some(CDeclaration::Typedef(CTypedefDecl {
            name,
            underlying_type: final_type,
        })))
    }

    /// Parse typedef struct
    fn parse_typedef_struct(&mut self) -> Result<Option<CDeclaration>, ParseError> {
        // Optional struct name
        let struct_name = match &self.current_token {
            CToken::Identifier(s) => {
                let name = s.clone();
                self.advance();
                Some(name)
            }
            _ => None,
        };

        // Parse struct body
        let fields = if self.current_token == CToken::OpenBrace {
            self.parse_struct_body()?
        } else {
            Vec::new()
        };

        // Get the typedef name
        let typedef_name = match &self.current_token {
            CToken::Identifier(s) => s.clone(),
            _ => return Err(self.error("Expected typedef name")),
        };
        self.advance();

        self.expect(CToken::Semicolon)?;

        // Register type alias
        let struct_type = CType::Struct {
            name: struct_name.clone(),
            fields: fields.clone(),
        };
        self.type_aliases.insert(typedef_name.clone(), struct_type.clone());

        // If the struct has a name, also emit a struct declaration
        if let Some(name) = struct_name {
            return Ok(Some(CDeclaration::Struct(CStructDecl {
                name,
                fields,
            })));
        }

        Ok(Some(CDeclaration::Typedef(CTypedefDecl {
            name: typedef_name,
            underlying_type: struct_type,
        })))
    }

    /// Parse typedef union
    fn parse_typedef_union(&mut self) -> Result<Option<CDeclaration>, ParseError> {
        let union_name = match &self.current_token {
            CToken::Identifier(s) => {
                let name = s.clone();
                self.advance();
                Some(name)
            }
            _ => None,
        };

        let fields = if self.current_token == CToken::OpenBrace {
            self.parse_struct_body()?
        } else {
            Vec::new()
        };

        let typedef_name = match &self.current_token {
            CToken::Identifier(s) => s.clone(),
            _ => return Err(self.error("Expected typedef name")),
        };
        self.advance();

        self.expect(CToken::Semicolon)?;

        let union_type = CType::Union {
            name: union_name,
            fields,
        };
        self.type_aliases.insert(typedef_name.clone(), union_type.clone());

        Ok(Some(CDeclaration::Typedef(CTypedefDecl {
            name: typedef_name,
            underlying_type: union_type,
        })))
    }

    /// Parse typedef enum
    fn parse_typedef_enum(&mut self) -> Result<Option<CDeclaration>, ParseError> {
        let enum_name = match &self.current_token {
            CToken::Identifier(s) => {
                let name = s.clone();
                self.advance();
                Some(name)
            }
            _ => None,
        };

        let variants = if self.current_token == CToken::OpenBrace {
            self.parse_enum_body()?
        } else {
            Vec::new()
        };

        let typedef_name = match &self.current_token {
            CToken::Identifier(s) => s.clone(),
            _ => return Err(self.error("Expected typedef name")),
        };
        self.advance();

        self.expect(CToken::Semicolon)?;

        let enum_type = CType::Enum {
            name: enum_name,
            variants,
        };
        self.type_aliases.insert(typedef_name.clone(), enum_type.clone());

        Ok(Some(CDeclaration::Typedef(CTypedefDecl {
            name: typedef_name,
            underlying_type: enum_type,
        })))
    }

    /// Parse struct or function declaration starting with 'struct'
    fn parse_struct_or_function(&mut self) -> Result<Option<CDeclaration>, ParseError> {
        self.advance(); // struct

        let struct_name = match &self.current_token {
            CToken::Identifier(s) => s.clone(),
            CToken::OpenBrace => {
                // Anonymous struct - skip for now
                self.skip_to_recovery_point();
                return Ok(None);
            }
            _ => return Err(self.error("Expected struct name")),
        };
        self.advance();

        // Check if this is a struct definition or just a type specifier
        if self.current_token == CToken::OpenBrace {
            // Struct definition
            let fields = self.parse_struct_body()?;

            // Check for variable declaration after struct
            if matches!(self.current_token, CToken::Identifier(_)) {
                // Variable of this struct type
                self.skip_to_recovery_point();
            } else {
                self.expect(CToken::Semicolon)?;
            }

            return Ok(Some(CDeclaration::Struct(CStructDecl {
                name: struct_name,
                fields,
            })));
        }

        // This might be a function returning struct or a variable declaration
        // For now, skip it
        self.skip_to_recovery_point();
        Ok(None)
    }

    /// Parse struct body
    fn parse_struct_body(&mut self) -> Result<Vec<CStructField>, ParseError> {
        self.expect(CToken::OpenBrace)?;
        let mut fields = Vec::new();

        while self.current_token != CToken::CloseBrace && self.current_token != CToken::Eof {
            // Parse field type
            let field_type = self.parse_type()?;

            // Parse field name
            let field_name = match &self.current_token {
                CToken::Identifier(s) => s.clone(),
                _ => return Err(self.error("Expected field name")),
            };
            self.advance();

            // Handle arrays
            let final_type = if self.current_token == CToken::OpenBracket {
                self.parse_array_suffix(field_type)?
            } else {
                field_type
            };

            // Handle bit fields
            let bit_width = if self.current_token == CToken::Equals {
                // This is likely initialization, not bit field
                self.skip_to_recovery_point();
                continue;
            } else {
                None
            };

            fields.push(CStructField {
                name: field_name,
                ty: final_type,
                bit_width,
            });

            self.expect(CToken::Semicolon)?;
        }

        self.expect(CToken::CloseBrace)?;
        Ok(fields)
    }

    /// Parse union declaration
    fn parse_union_decl(&mut self) -> Result<Option<CDeclaration>, ParseError> {
        self.advance(); // union

        let union_name = match &self.current_token {
            CToken::Identifier(s) => s.clone(),
            _ => {
                self.skip_to_recovery_point();
                return Ok(None);
            }
        };
        self.advance();

        if self.current_token == CToken::OpenBrace {
            let fields = self.parse_struct_body()?;
            self.expect(CToken::Semicolon)?;
            return Ok(Some(CDeclaration::Union(CUnionDecl {
                name: union_name,
                fields,
            })));
        }

        self.skip_to_recovery_point();
        Ok(None)
    }

    /// Parse enum declaration
    fn parse_enum_decl(&mut self) -> Result<Option<CDeclaration>, ParseError> {
        self.advance(); // enum

        let enum_name = match &self.current_token {
            CToken::Identifier(s) => s.clone(),
            _ => {
                self.skip_to_recovery_point();
                return Ok(None);
            }
        };
        self.advance();

        if self.current_token == CToken::OpenBrace {
            let variants = self.parse_enum_body()?;
            self.expect(CToken::Semicolon)?;
            return Ok(Some(CDeclaration::Enum(CEnumDecl {
                name: enum_name,
                variants,
            })));
        }

        self.skip_to_recovery_point();
        Ok(None)
    }

    /// Parse enum body
    fn parse_enum_body(&mut self) -> Result<Vec<CEnumVariant>, ParseError> {
        self.expect(CToken::OpenBrace)?;
        let mut variants = Vec::new();

        while self.current_token != CToken::CloseBrace && self.current_token != CToken::Eof {
            let name = match &self.current_token {
                CToken::Identifier(s) => s.clone(),
                _ => {
                    self.advance();
                    continue;
                }
            };
            self.advance();

            let value = if self.current_token == CToken::Equals {
                self.advance();
                match &self.current_token {
                    CToken::IntLiteral(n) => {
                        let v = Some(*n);
                        self.advance();
                        v
                    }
                    _ => {
                        // Skip complex expressions
                        while self.current_token != CToken::Comma
                            && self.current_token != CToken::CloseBrace
                        {
                            self.advance();
                        }
                        None
                    }
                }
            } else {
                None
            };

            variants.push(CEnumVariant { name, value });

            if self.current_token == CToken::Comma {
                self.advance();
            }
        }

        self.expect(CToken::CloseBrace)?;
        Ok(variants)
    }

    /// Parse extern declaration
    fn parse_extern_decl(&mut self) -> Result<Option<CDeclaration>, ParseError> {
        self.advance(); // extern

        // Check for extern "C"
        if let CToken::StringLiteral(s) = &self.current_token {
            if s == "C" {
                self.advance();
                // Handle extern "C" { ... } block
                if self.current_token == CToken::OpenBrace {
                    // Parse multiple declarations inside
                    // For simplicity, skip the block
                    self.skip_to_recovery_point();
                    return Ok(None);
                }
            }
        }

        // Parse the type and name
        self.parse_function_or_variable()
    }

    /// Parse function or variable declaration
    fn parse_function_or_variable(&mut self) -> Result<Option<CDeclaration>, ParseError> {
        let mut is_static = false;
        let mut is_inline = false;
        let mut is_const = false;

        // Parse storage class and qualifiers
        loop {
            match &self.current_token {
                CToken::Static => {
                    is_static = true;
                    self.advance();
                }
                CToken::Inline => {
                    is_inline = true;
                    self.advance();
                }
                CToken::Const => {
                    is_const = true;
                    self.advance();
                }
                CToken::Extern => {
                    self.advance();
                }
                _ => break,
            }
        }

        // Parse return type
        let return_type = self.parse_type()?;

        // Handle const after type
        if self.current_token == CToken::Const {
            is_const = true;
            self.advance();
        }

        // Parse name
        let name = match &self.current_token {
            CToken::Identifier(s) => s.clone(),
            _ => {
                self.skip_to_recovery_point();
                return Ok(None);
            }
        };
        self.advance();

        // Check if function or variable
        if self.current_token == CToken::OpenParen {
            // Function declaration
            self.advance();
            let (params, variadic) = self.parse_function_params()?;
            self.expect(CToken::CloseParen)?;

            // Skip function body if present
            if self.current_token == CToken::OpenBrace {
                self.skip_to_recovery_point();
            } else {
                self.expect(CToken::Semicolon)?;
            }

            return Ok(Some(CDeclaration::Function(CFunctionDecl {
                name,
                return_type,
                params,
                variadic,
                is_inline,
                is_static,
            })));
        }

        // Variable declaration
        let final_type = if self.current_token == CToken::OpenBracket {
            self.parse_array_suffix(return_type)?
        } else {
            return_type
        };

        // Skip initialization
        if self.current_token == CToken::Equals {
            while self.current_token != CToken::Semicolon && self.current_token != CToken::Eof {
                self.advance();
            }
        }

        self.expect(CToken::Semicolon)?;

        Ok(Some(CDeclaration::Variable(CVariableDecl {
            name,
            ty: final_type,
            is_extern: true,
            is_const,
        })))
    }

    /// Parse function parameters
    fn parse_function_params(&mut self) -> Result<(Vec<CFunctionParam>, bool), ParseError> {
        let mut params = Vec::new();
        let mut variadic = false;

        // Handle (void) case
        if self.current_token == CToken::Void {
            let next = self.lexer.peek_token();
            if next == CToken::CloseParen {
                self.advance();
                return Ok((params, false));
            }
        }

        while self.current_token != CToken::CloseParen && self.current_token != CToken::Eof {
            // Check for ellipsis
            if self.current_token == CToken::Ellipsis {
                variadic = true;
                self.advance();
                break;
            }

            // Parse parameter type
            let param_type = self.parse_type()?;

            // Parse parameter name (optional)
            let param_name = match &self.current_token {
                CToken::Identifier(s) => {
                    let name = s.clone();
                    self.advance();
                    Some(name)
                }
                _ => None,
            };

            // Handle array parameters
            let final_type = if self.current_token == CToken::OpenBracket {
                // Arrays decay to pointers in function parameters
                self.advance();
                while self.current_token != CToken::CloseBracket && self.current_token != CToken::Eof {
                    self.advance();
                }
                self.expect(CToken::CloseBracket)?;
                CType::Pointer(Box::new(param_type))
            } else {
                param_type
            };

            params.push(CFunctionParam {
                name: param_name,
                ty: final_type,
            });

            if self.current_token == CToken::Comma {
                self.advance();
            } else {
                break;
            }
        }

        Ok((params, variadic))
    }

    /// Parse a C type
    fn parse_type(&mut self) -> Result<CType, ParseError> {
        let mut is_const = false;
        let mut is_volatile = false;
        let mut is_unsigned = false;
        let mut is_signed = false;

        // Parse qualifiers and signedness
        loop {
            match &self.current_token {
                CToken::Const => {
                    is_const = true;
                    self.advance();
                }
                CToken::Volatile => {
                    is_volatile = true;
                    self.advance();
                }
                CToken::Unsigned => {
                    is_unsigned = true;
                    self.advance();
                }
                CToken::Signed => {
                    is_signed = true;
                    self.advance();
                }
                _ => break,
            }
        }

        // Parse base type
        let base_type = match &self.current_token {
            CToken::Void => {
                self.advance();
                CType::Void
            }
            CToken::Char => {
                self.advance();
                if is_unsigned {
                    CType::UChar
                } else if is_signed {
                    CType::SChar
                } else {
                    CType::Char
                }
            }
            CToken::Short => {
                self.advance();
                // Skip optional 'int'
                if self.current_token == CToken::Int {
                    self.advance();
                }
                if is_unsigned {
                    CType::UShort
                } else {
                    CType::Short
                }
            }
            CToken::Int => {
                self.advance();
                if is_unsigned {
                    CType::UInt
                } else {
                    CType::Int
                }
            }
            CToken::Long => {
                self.advance();
                // Check for 'long long' or 'long int' or 'long double'
                match &self.current_token {
                    CToken::Long => {
                        self.advance();
                        // Skip optional 'int'
                        if self.current_token == CToken::Int {
                            self.advance();
                        }
                        if is_unsigned {
                            CType::ULongLong
                        } else {
                            CType::LongLong
                        }
                    }
                    CToken::Double => {
                        self.advance();
                        CType::LongDouble
                    }
                    CToken::Int => {
                        self.advance();
                        if is_unsigned {
                            CType::ULong
                        } else {
                            CType::Long
                        }
                    }
                    _ => {
                        if is_unsigned {
                            CType::ULong
                        } else {
                            CType::Long
                        }
                    }
                }
            }
            CToken::Float => {
                self.advance();
                CType::Float
            }
            CToken::Double => {
                self.advance();
                CType::Double
            }
            CToken::Bool => {
                self.advance();
                CType::Bool
            }
            CToken::SizeT => {
                self.advance();
                CType::SizeT
            }
            CToken::Struct => {
                self.advance();
                let name = match &self.current_token {
                    CToken::Identifier(s) => s.clone(),
                    _ => return Err(self.error("Expected struct name")),
                };
                self.advance();
                CType::Named(format!("struct {}", name))
            }
            CToken::Union => {
                self.advance();
                let name = match &self.current_token {
                    CToken::Identifier(s) => s.clone(),
                    _ => return Err(self.error("Expected union name")),
                };
                self.advance();
                CType::Named(format!("union {}", name))
            }
            CToken::Enum => {
                self.advance();
                let name = match &self.current_token {
                    CToken::Identifier(s) => s.clone(),
                    _ => return Err(self.error("Expected enum name")),
                };
                self.advance();
                CType::Named(format!("enum {}", name))
            }
            CToken::Identifier(name) => {
                // Check if it's a known typedef
                let name = name.clone();
                self.advance();

                // Check for common standard types
                match name.as_str() {
                    "size_t" => CType::SizeT,
                    "ssize_t" => CType::SSizeT,
                    "intptr_t" => CType::IntPtrT,
                    "uintptr_t" => CType::UIntPtrT,
                    "ptrdiff_t" => CType::PtrDiffT,
                    "int8_t" => CType::SChar,
                    "int16_t" => CType::Short,
                    "int32_t" => CType::Int,
                    "int64_t" => CType::LongLong,
                    "uint8_t" => CType::UChar,
                    "uint16_t" => CType::UShort,
                    "uint32_t" => CType::UInt,
                    "uint64_t" => CType::ULongLong,
                    _ => {
                        if let Some(aliased) = self.type_aliases.get(&name) {
                            aliased.clone()
                        } else {
                            CType::Named(name)
                        }
                    }
                }
            }
            _ => {
                // If we have unsigned/signed without a base type, it means 'int'
                if is_unsigned {
                    CType::UInt
                } else if is_signed {
                    CType::Int
                } else {
                    return Err(self.error(format!(
                        "Expected type, found {:?}",
                        self.current_token
                    )));
                }
            }
        };

        // Parse pointer qualifiers
        let mut result = base_type;
        while self.current_token == CToken::Star {
            self.advance();
            result = CType::Pointer(Box::new(result));

            // Handle const/volatile after *
            while matches!(self.current_token, CToken::Const | CToken::Volatile) {
                if self.current_token == CToken::Const {
                    is_const = true;
                } else {
                    is_volatile = true;
                }
                self.advance();
            }
        }

        // Apply qualifiers
        if is_volatile {
            result = CType::Volatile(Box::new(result));
        }
        if is_const {
            result = CType::Const(Box::new(result));
        }

        Ok(result)
    }

    /// Parse array suffix [N] or []
    fn parse_array_suffix(&mut self, element_type: CType) -> Result<CType, ParseError> {
        self.expect(CToken::OpenBracket)?;

        let size = match &self.current_token {
            CToken::IntLiteral(n) => {
                let size = Some(*n as usize);
                self.advance();
                size
            }
            CToken::CloseBracket => None,
            _ => {
                // Skip complex size expressions
                while self.current_token != CToken::CloseBracket && self.current_token != CToken::Eof {
                    self.advance();
                }
                None
            }
        };

        self.expect(CToken::CloseBracket)?;

        // Check for multi-dimensional arrays
        if self.current_token == CToken::OpenBracket {
            let inner = self.parse_array_suffix(element_type)?;
            Ok(CType::Array(Box::new(inner), size))
        } else {
            Ok(CType::Array(Box::new(element_type), size))
        }
    }
}

// ============================================================================
// Type Conversion to Aria FFI Types
// ============================================================================

/// Aria FFI type representation for code generation
#[derive(Debug, Clone, PartialEq)]
pub enum AriaFfiType {
    /// Maps to `()`
    Void,
    /// Maps to CInt (i32 on most platforms)
    CInt,
    /// Maps to CUInt
    CUInt,
    /// Maps to CShort
    CShort,
    /// Maps to CUShort
    CUShort,
    /// Maps to CLong
    CLong,
    /// Maps to CULong
    CULong,
    /// Maps to CLongLong
    CLongLong,
    /// Maps to CULongLong
    CULongLong,
    /// Maps to CChar
    CChar,
    /// Maps to CUChar
    CUChar,
    /// Maps to CSChar
    CSChar,
    /// Maps to CFloat
    CFloat,
    /// Maps to CDouble
    CDouble,
    /// Maps to CBool
    CBool,
    /// Maps to CSize
    CSize,
    /// Maps to CSSize
    CSSize,
    /// Maps to CIntPtr
    CIntPtr,
    /// Maps to CUIntPtr
    CUIntPtr,
    /// Maps to CPtrDiff
    CPtrDiff,
    /// Pointer type: CPtr<T>
    Pointer(Box<AriaFfiType>),
    /// Const pointer: CPtrConst<T>
    ConstPointer(Box<AriaFfiType>),
    /// Array type: CArray<T, N>
    Array(Box<AriaFfiType>, usize),
    /// Function pointer type
    FunctionPointer {
        return_type: Box<AriaFfiType>,
        params: Vec<AriaFfiType>,
    },
    /// Named type (struct, enum, typedef)
    Named(String),
}

impl fmt::Display for AriaFfiType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AriaFfiType::Void => write!(f, "()"),
            AriaFfiType::CInt => write!(f, "CInt"),
            AriaFfiType::CUInt => write!(f, "CUInt"),
            AriaFfiType::CShort => write!(f, "CShort"),
            AriaFfiType::CUShort => write!(f, "CUShort"),
            AriaFfiType::CLong => write!(f, "CLong"),
            AriaFfiType::CULong => write!(f, "CULong"),
            AriaFfiType::CLongLong => write!(f, "CLongLong"),
            AriaFfiType::CULongLong => write!(f, "CULongLong"),
            AriaFfiType::CChar => write!(f, "CChar"),
            AriaFfiType::CUChar => write!(f, "CUChar"),
            AriaFfiType::CSChar => write!(f, "CSChar"),
            AriaFfiType::CFloat => write!(f, "CFloat"),
            AriaFfiType::CDouble => write!(f, "CDouble"),
            AriaFfiType::CBool => write!(f, "CBool"),
            AriaFfiType::CSize => write!(f, "CSize"),
            AriaFfiType::CSSize => write!(f, "CSSize"),
            AriaFfiType::CIntPtr => write!(f, "CIntPtr"),
            AriaFfiType::CUIntPtr => write!(f, "CUIntPtr"),
            AriaFfiType::CPtrDiff => write!(f, "CPtrDiff"),
            AriaFfiType::Pointer(inner) => write!(f, "CPtr<{}>", inner),
            AriaFfiType::ConstPointer(inner) => write!(f, "CPtrConst<{}>", inner),
            AriaFfiType::Array(inner, size) => write!(f, "CArray<{}, {}>", inner, size),
            AriaFfiType::FunctionPointer { return_type, params } => {
                write!(f, "CFn<(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, "), {}>", return_type)
            }
            AriaFfiType::Named(name) => write!(f, "{}", name),
        }
    }
}

/// Convert a C type to an Aria FFI type
pub fn c_type_to_aria(c_type: &CType) -> AriaFfiType {
    match c_type {
        CType::Void => AriaFfiType::Void,
        CType::Char => AriaFfiType::CChar,
        CType::UChar => AriaFfiType::CUChar,
        CType::SChar => AriaFfiType::CSChar,
        CType::Short => AriaFfiType::CShort,
        CType::UShort => AriaFfiType::CUShort,
        CType::Int => AriaFfiType::CInt,
        CType::UInt => AriaFfiType::CUInt,
        CType::Long => AriaFfiType::CLong,
        CType::ULong => AriaFfiType::CULong,
        CType::LongLong => AriaFfiType::CLongLong,
        CType::ULongLong => AriaFfiType::CULongLong,
        CType::Float => AriaFfiType::CFloat,
        CType::Double | CType::LongDouble => AriaFfiType::CDouble,
        CType::Bool => AriaFfiType::CBool,
        CType::SizeT => AriaFfiType::CSize,
        CType::SSizeT => AriaFfiType::CSSize,
        CType::IntPtrT => AriaFfiType::CIntPtr,
        CType::UIntPtrT => AriaFfiType::CUIntPtr,
        CType::PtrDiffT => AriaFfiType::CPtrDiff,
        CType::Pointer(inner) => {
            let inner_aria = c_type_to_aria(inner);
            AriaFfiType::Pointer(Box::new(inner_aria))
        }
        CType::Const(inner) => {
            // const T* -> CPtrConst<T>
            if let CType::Pointer(ptr_inner) = inner.as_ref() {
                let inner_aria = c_type_to_aria(ptr_inner);
                AriaFfiType::ConstPointer(Box::new(inner_aria))
            } else {
                // const T -> T (const values are just values)
                c_type_to_aria(inner)
            }
        }
        CType::Volatile(inner) => {
            // Volatile doesn't change the Aria type representation
            c_type_to_aria(inner)
        }
        CType::Array(inner, Some(size)) => {
            let inner_aria = c_type_to_aria(inner);
            AriaFfiType::Array(Box::new(inner_aria), *size)
        }
        CType::Array(inner, None) => {
            // Unsized arrays become pointers
            let inner_aria = c_type_to_aria(inner);
            AriaFfiType::Pointer(Box::new(inner_aria))
        }
        CType::FunctionPointer {
            return_type,
            params,
            variadic: _,
        } => {
            let ret = c_type_to_aria(return_type);
            let params_aria: Vec<_> = params.iter().map(c_type_to_aria).collect();
            AriaFfiType::FunctionPointer {
                return_type: Box::new(ret),
                params: params_aria,
            }
        }
        CType::Named(name) => AriaFfiType::Named(name.clone()),
        CType::Struct { name, .. } => {
            AriaFfiType::Named(name.clone().unwrap_or_else(|| "AnonymousStruct".to_string()))
        }
        CType::Union { name, .. } => {
            AriaFfiType::Named(name.clone().unwrap_or_else(|| "AnonymousUnion".to_string()))
        }
        CType::Enum { name, .. } => {
            AriaFfiType::Named(name.clone().unwrap_or_else(|| "AnonymousEnum".to_string()))
        }
    }
}

// ============================================================================
// Wrapper Generation
// ============================================================================

/// A safe wrapper for a C function
#[derive(Debug, Clone)]
pub struct SafeWrapper {
    /// Original C function name
    pub c_name: String,
    /// Aria function name (may be different)
    pub aria_name: String,
    /// Parameter conversions
    pub params: Vec<SafeWrapperParam>,
    /// Return type handling
    pub return_type: SafeReturnType,
    /// Pre-call checks
    pub pre_checks: Vec<String>,
    /// Post-call operations
    pub post_ops: Vec<String>,
}

/// Parameter in a safe wrapper
#[derive(Debug, Clone)]
pub struct SafeWrapperParam {
    /// Parameter name
    pub name: String,
    /// Aria-side type
    pub aria_type: AriaFfiType,
    /// Conversion to C type
    pub conversion: ParameterConversion,
}

/// How a parameter is converted for FFI
#[derive(Debug, Clone)]
pub enum ParameterConversion {
    /// No conversion needed
    Direct,
    /// String to CString (AriaString)
    StringToCString,
    /// Slice to pointer + length
    SliceToPointerLength,
    /// Owned to raw pointer
    OwnedToPtr,
    /// Reference to const pointer
    RefToConstPtr,
    /// Mutable reference to pointer
    MutRefToPtr,
}

/// Return type handling in safe wrapper
#[derive(Debug, Clone)]
pub enum SafeReturnType {
    /// No return (void)
    Void,
    /// Direct return of primitive
    Direct(AriaFfiType),
    /// Wrap in Result for error handling
    Result(AriaFfiType),
    /// Wrap in Option for nullable
    Option(AriaFfiType),
    /// Owned wrapper (caller must free)
    Owned(AriaFfiType),
    /// Borrowed wrapper (do not free)
    Borrowed(AriaFfiType),
}

/// Generate safe wrappers for C declarations
pub fn generate_safe_wrappers(declarations: &[CDeclaration]) -> Vec<SafeWrapper> {
    declarations
        .iter()
        .filter_map(|decl| {
            if let CDeclaration::Function(func) = decl {
                Some(generate_function_wrapper(func))
            } else {
                None
            }
        })
        .collect()
}

/// Generate a safe wrapper for a C function
fn generate_function_wrapper(func: &CFunctionDecl) -> SafeWrapper {
    let params: Vec<SafeWrapperParam> = func
        .params
        .iter()
        .enumerate()
        .map(|(i, param)| {
            let name = param.name.clone().unwrap_or_else(|| format!("arg{}", i));
            let aria_type = c_type_to_aria(&param.ty);
            let conversion = determine_param_conversion(&param.ty);
            SafeWrapperParam {
                name,
                aria_type,
                conversion,
            }
        })
        .collect();

    let return_type = determine_return_type(&func.return_type, &func.name);

    let mut pre_checks = Vec::new();

    // Add null checks for pointer parameters
    for param in &params {
        if matches!(param.aria_type, AriaFfiType::Pointer(_)) {
            pre_checks.push(format!(
                "if {}.is_null() {{ return Err(FfiError::null_pointer(\"{}\")); }}",
                param.name, param.name
            ));
        }
    }

    SafeWrapper {
        c_name: func.name.clone(),
        aria_name: to_snake_case(&func.name),
        params,
        return_type,
        pre_checks,
        post_ops: Vec::new(),
    }
}

/// Determine parameter conversion strategy
fn determine_param_conversion(ty: &CType) -> ParameterConversion {
    match ty {
        CType::Pointer(inner) => match inner.as_ref() {
            CType::Char => ParameterConversion::StringToCString,
            CType::Const(inner_const) => {
                if matches!(inner_const.as_ref(), CType::Char) {
                    ParameterConversion::StringToCString
                } else {
                    ParameterConversion::RefToConstPtr
                }
            }
            _ => ParameterConversion::MutRefToPtr,
        },
        CType::Const(inner) => {
            if let CType::Pointer(_) = inner.as_ref() {
                ParameterConversion::RefToConstPtr
            } else {
                ParameterConversion::Direct
            }
        }
        _ => ParameterConversion::Direct,
    }
}

/// Determine return type handling
fn determine_return_type(ty: &CType, func_name: &str) -> SafeReturnType {
    let aria_type = c_type_to_aria(ty);

    match ty {
        CType::Void => SafeReturnType::Void,
        CType::Pointer(inner) => {
            // Check for common patterns
            let name_lower = func_name.to_lowercase();
            if name_lower.contains("alloc")
                || name_lower.contains("create")
                || name_lower.contains("new")
                || name_lower.contains("open")
            {
                SafeReturnType::Owned(aria_type)
            } else if name_lower.contains("get")
                || name_lower.contains("find")
                || name_lower.contains("lookup")
            {
                SafeReturnType::Option(c_type_to_aria(inner))
            } else {
                SafeReturnType::Option(c_type_to_aria(inner))
            }
        }
        CType::Int | CType::Long => {
            // Many C functions return int for success/failure
            let name_lower = func_name.to_lowercase();
            if name_lower.contains("init")
                || name_lower.contains("close")
                || name_lower.contains("set")
                || name_lower.contains("write")
                || name_lower.contains("read")
            {
                SafeReturnType::Result(aria_type)
            } else {
                SafeReturnType::Direct(aria_type)
            }
        }
        _ => SafeReturnType::Direct(aria_type),
    }
}

/// Convert CamelCase or mixedCase to snake_case
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_basic_tokens() {
        let mut lexer = CLexer::new("int foo;");
        assert_eq!(lexer.next_token(), CToken::Int);
        assert_eq!(lexer.next_token(), CToken::Identifier("foo".to_string()));
        assert_eq!(lexer.next_token(), CToken::Semicolon);
        assert_eq!(lexer.next_token(), CToken::Eof);
    }

    #[test]
    fn test_lexer_numbers() {
        let mut lexer = CLexer::new("42 0x1F 3.14");
        assert_eq!(lexer.next_token(), CToken::IntLiteral(42));
        assert_eq!(lexer.next_token(), CToken::IntLiteral(31)); // 0x1F = 31
        assert_eq!(lexer.next_token(), CToken::FloatLiteral(3.14));
    }

    #[test]
    fn test_lexer_comments() {
        let mut lexer = CLexer::new("int /* comment */ foo // line comment\n;");
        assert_eq!(lexer.next_token(), CToken::Int);
        assert_eq!(lexer.next_token(), CToken::Identifier("foo".to_string()));
        assert_eq!(lexer.next_token(), CToken::Semicolon);
    }

    #[test]
    fn test_parse_simple_function() {
        let mut parser = CHeaderParser::new("int add(int a, int b);");
        let decls = parser.parse().unwrap();

        assert_eq!(decls.len(), 1);
        if let CDeclaration::Function(func) = &decls[0] {
            assert_eq!(func.name, "add");
            assert_eq!(func.params.len(), 2);
            assert_eq!(func.return_type, CType::Int);
        } else {
            panic!("Expected function declaration");
        }
    }

    #[test]
    fn test_parse_struct() {
        let input = r#"
            struct Point {
                int x;
                int y;
            };
        "#;
        let mut parser = CHeaderParser::new(input);
        let decls = parser.parse().unwrap();

        assert_eq!(decls.len(), 1);
        if let CDeclaration::Struct(s) = &decls[0] {
            assert_eq!(s.name, "Point");
            assert_eq!(s.fields.len(), 2);
            assert_eq!(s.fields[0].name, "x");
            assert_eq!(s.fields[1].name, "y");
        } else {
            panic!("Expected struct declaration");
        }
    }

    #[test]
    fn test_parse_typedef_struct() {
        let input = "typedef struct { int x; int y; } Point;";
        let mut parser = CHeaderParser::new(input);
        let decls = parser.parse().unwrap();

        assert_eq!(decls.len(), 1);
        if let CDeclaration::Typedef(td) = &decls[0] {
            assert_eq!(td.name, "Point");
        } else {
            panic!("Expected typedef declaration");
        }
    }

    #[test]
    fn test_parse_enum() {
        let input = r#"
            enum Color {
                RED = 0,
                GREEN = 1,
                BLUE = 2
            };
        "#;
        let mut parser = CHeaderParser::new(input);
        let decls = parser.parse().unwrap();

        assert_eq!(decls.len(), 1);
        if let CDeclaration::Enum(e) = &decls[0] {
            assert_eq!(e.name, "Color");
            assert_eq!(e.variants.len(), 3);
            assert_eq!(e.variants[0].name, "RED");
            assert_eq!(e.variants[0].value, Some(0));
        } else {
            panic!("Expected enum declaration");
        }
    }

    #[test]
    fn test_parse_typedef() {
        let input = "typedef unsigned long size_t;";
        let mut parser = CHeaderParser::new(input);
        let decls = parser.parse().unwrap();

        assert_eq!(decls.len(), 1);
        if let CDeclaration::Typedef(td) = &decls[0] {
            assert_eq!(td.name, "size_t");
            assert_eq!(td.underlying_type, CType::ULong);
        } else {
            panic!("Expected typedef declaration");
        }
    }

    #[test]
    fn test_parse_function_with_pointers() {
        let input = "void* malloc(size_t size);";
        let mut parser = CHeaderParser::new(input);
        let decls = parser.parse().unwrap();

        assert_eq!(decls.len(), 1);
        if let CDeclaration::Function(func) = &decls[0] {
            assert_eq!(func.name, "malloc");
            assert!(matches!(func.return_type, CType::Pointer(_)));
        } else {
            panic!("Expected function declaration");
        }
    }

    #[test]
    fn test_parse_const_pointer() {
        let input = "int strlen(const char* s);";
        let mut parser = CHeaderParser::new(input);
        let decls = parser.parse().unwrap();

        assert_eq!(decls.len(), 1);
        if let CDeclaration::Function(func) = &decls[0] {
            assert_eq!(func.name, "strlen");
            assert_eq!(func.params.len(), 1);
        } else {
            panic!("Expected function declaration");
        }
    }

    #[test]
    fn test_parse_define_constant() {
        let input = "#define MAX_SIZE 1024";
        let mut parser = CHeaderParser::new(input);
        let decls = parser.parse().unwrap();

        assert_eq!(decls.len(), 1);
        if let CDeclaration::Constant(c) = &decls[0] {
            assert_eq!(c.name, "MAX_SIZE");
            assert_eq!(c.value, CConstantValue::Integer(1024));
        } else {
            panic!("Expected constant declaration");
        }
    }

    #[test]
    fn test_parse_variadic_function() {
        let input = "int printf(const char* fmt, ...);";
        let mut parser = CHeaderParser::new(input);
        let decls = parser.parse().unwrap();

        assert_eq!(decls.len(), 1);
        if let CDeclaration::Function(func) = &decls[0] {
            assert_eq!(func.name, "printf");
            assert!(func.variadic);
        } else {
            panic!("Expected function declaration");
        }
    }

    #[test]
    fn test_c_type_to_aria() {
        assert_eq!(c_type_to_aria(&CType::Int), AriaFfiType::CInt);
        assert_eq!(c_type_to_aria(&CType::UInt), AriaFfiType::CUInt);
        assert_eq!(c_type_to_aria(&CType::Void), AriaFfiType::Void);
        assert_eq!(c_type_to_aria(&CType::SizeT), AriaFfiType::CSize);

        let ptr_int = CType::Pointer(Box::new(CType::Int));
        assert_eq!(
            c_type_to_aria(&ptr_int),
            AriaFfiType::Pointer(Box::new(AriaFfiType::CInt))
        );
    }

    #[test]
    fn test_aria_ffi_type_display() {
        assert_eq!(format!("{}", AriaFfiType::CInt), "CInt");
        assert_eq!(
            format!("{}", AriaFfiType::Pointer(Box::new(AriaFfiType::CChar))),
            "CPtr<CChar>"
        );
        assert_eq!(
            format!("{}", AriaFfiType::Array(Box::new(AriaFfiType::CInt), 10)),
            "CArray<CInt, 10>"
        );
    }

    #[test]
    fn test_generate_safe_wrapper() {
        let func = CFunctionDecl {
            name: "malloc".to_string(),
            return_type: CType::Pointer(Box::new(CType::Void)),
            params: vec![CFunctionParam {
                name: Some("size".to_string()),
                ty: CType::SizeT,
            }],
            variadic: false,
            is_inline: false,
            is_static: false,
        };

        let wrapper = generate_function_wrapper(&func);
        assert_eq!(wrapper.c_name, "malloc");
        assert_eq!(wrapper.aria_name, "malloc");
        assert!(matches!(wrapper.return_type, SafeReturnType::Owned(_)));
    }

    #[test]
    fn test_parse_complex_header() {
        let input = r#"
            #ifndef MY_HEADER_H
            #define MY_HEADER_H

            #define BUFFER_SIZE 256

            typedef struct {
                char name[64];
                int age;
            } Person;

            Person* person_create(const char* name, int age);
            void person_free(Person* p);
            const char* person_get_name(Person* p);

            #endif
        "#;

        let mut parser = CHeaderParser::new(input);
        let decls = parser.parse().unwrap();

        // Should have: BUFFER_SIZE constant, Person typedef, and 3 functions
        let constants: Vec<_> = decls.iter().filter(|d| matches!(d, CDeclaration::Constant(_))).collect();
        let typedefs: Vec<_> = decls.iter().filter(|d| matches!(d, CDeclaration::Typedef(_))).collect();
        let functions: Vec<_> = decls.iter().filter(|d| matches!(d, CDeclaration::Function(_))).collect();

        assert_eq!(constants.len(), 1);
        assert_eq!(typedefs.len(), 1);
        assert_eq!(functions.len(), 3);
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("camelCase"), "camel_case");
        assert_eq!(to_snake_case("PascalCase"), "pascal_case");
        assert_eq!(to_snake_case("already_snake"), "already_snake");
        assert_eq!(to_snake_case("XMLParser"), "x_m_l_parser");
    }

    #[test]
    fn test_parse_fixed_width_types() {
        let input = r#"
            int32_t read_int32(void);
            uint64_t read_uint64(void);
        "#;

        let mut parser = CHeaderParser::new(input);
        let decls = parser.parse().unwrap();

        assert_eq!(decls.len(), 2);
        if let CDeclaration::Function(func) = &decls[0] {
            assert_eq!(func.return_type, CType::Int);
        }
        if let CDeclaration::Function(func) = &decls[1] {
            assert_eq!(func.return_type, CType::ULongLong);
        }
    }

    #[test]
    fn test_parse_union() {
        let input = r#"
            union Data {
                int i;
                float f;
                char str[20];
            };
        "#;

        let mut parser = CHeaderParser::new(input);
        let decls = parser.parse().unwrap();

        assert_eq!(decls.len(), 1);
        if let CDeclaration::Union(u) = &decls[0] {
            assert_eq!(u.name, "Data");
            assert_eq!(u.fields.len(), 3);
        } else {
            panic!("Expected union declaration");
        }
    }
}
