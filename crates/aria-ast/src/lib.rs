//! Aria Language Abstract Syntax Tree
//!
//! Defines all AST node types for the Aria programming language.
//! Follows the structure defined in GRAMMAR.md.

// Re-export common types for use by other crates
pub use smol_str::SmolStr;
pub use aria_lexer::Span;

/// A spanned value - wraps any value with source location info
#[derive(Debug, Clone, PartialEq)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(node: T, span: Span) -> Self {
        Self { node, span }
    }

    pub fn dummy(node: T) -> Self {
        Self {
            node,
            span: Span::dummy(),
        }
    }

    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Spanned<U> {
        Spanned {
            node: f(self.node),
            span: self.span,
        }
    }
}

/// Identifier (variable names, function names, etc.)
pub type Ident = Spanned<SmolStr>;

/// Type identifier (type names like `Int`, `String`, `User`)
pub type TypeIdent = Spanned<SmolStr>;

// ============================================================================
// Program Structure
// ============================================================================

/// A complete Aria program/module
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub items: Vec<Item>,
    pub span: Span,
}

/// Top-level declarations
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Module(ModuleDecl),
    Import(ImportDecl),
    Export(ExportDecl),
    Use(UseDecl),
    Function(FunctionDecl),
    Struct(StructDecl),
    Data(DataDecl),
    Enum(EnumDecl),
    Trait(TraitDecl),
    Impl(ImplDecl),
    Const(ConstDecl),
    TypeAlias(TypeAlias),
    Test(TestDecl),
    Extern(ExternDecl),
}

// ============================================================================
// Visibility
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Visibility {
    #[default]
    Private,
    Public,
}

// ============================================================================
// Module System
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleDecl {
    pub path: Vec<Ident>,
    pub items: Vec<Item>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportDecl {
    pub path: ImportPath,
    pub alias: Option<Ident>,
    pub selection: Option<ImportSelection>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImportPath {
    Module(Vec<Ident>),
    String(SmolStr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImportSelection {
    All,
    Items(Vec<ImportItem>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportItem {
    pub name: Ident,
    pub alias: Option<Ident>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExportDecl {
    pub selection: ExportSelection,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExportSelection {
    All,
    Items(Vec<Ident>),
}

/// Re-export declaration: `pub use Foo::Bar` or `use Foo::Bar`
#[derive(Debug, Clone, PartialEq)]
pub struct UseDecl {
    pub visibility: Visibility,
    pub path: Vec<Ident>,
    pub selection: Option<ImportSelection>,
    pub alias: Option<Ident>,
    pub span: Span,
}

// ============================================================================
// Functions
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    pub attributes: Vec<Attribute>,
    pub visibility: Visibility,
    pub name: Ident,
    pub generic_params: Option<GenericParams>,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub where_clause: Option<WhereClause>,
    pub contracts: Vec<Contract>,
    pub body: FunctionBody,
    pub test_block: Option<TestBlock>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub mutable: bool,
    pub name: Ident,
    pub ty: Option<TypeExpr>,
    pub default: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FunctionBody {
    Block(Block),
    Expression(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

// ============================================================================
// Generics
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct GenericParams {
    pub params: Vec<GenericParam>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenericParam {
    pub name: TypeIdent,
    pub bounds: Vec<TraitBound>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraitBound {
    pub path: Vec<TypeIdent>,
    pub type_args: Option<Vec<TypeExpr>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhereClause {
    pub constraints: Vec<TypeConstraint>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeConstraint {
    pub ty: TypeIdent,
    pub bounds: Vec<TraitBound>,
    pub span: Span,
}

// ============================================================================
// Contracts
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Contract {
    Requires(ContractClause),
    Ensures(ContractClause),
    Invariant(ContractClause),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContractClause {
    pub condition: Box<Expr>,
    pub message: Option<SmolStr>,
    pub span: Span,
}

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    /// Simple named type: `Int`, `String`, `User`
    Named(TypeIdent),

    /// Generic type: `Array<T>`, `Map<K, V>`
    Generic {
        name: TypeIdent,
        args: Vec<TypeExpr>,
        span: Span,
    },

    /// Array type: `[T]` or `[T; N]`
    Array {
        element: Box<TypeExpr>,
        size: Option<Box<Expr>>,
        span: Span,
    },

    /// Map type: `{K: V}`
    Map {
        key: Box<TypeExpr>,
        value: Box<TypeExpr>,
        span: Span,
    },

    /// Tuple type: `(T1, T2, T3)`
    Tuple {
        elements: Vec<TypeExpr>,
        span: Span,
    },

    /// Optional type: `T?`
    Optional {
        inner: Box<TypeExpr>,
        span: Span,
    },

    /// Result type: `Result<T, E>`
    Result {
        ok: Box<TypeExpr>,
        err: Option<Box<TypeExpr>>,
        span: Span,
    },

    /// Reference type: `&T` or `&mut T`
    Reference {
        mutable: bool,
        inner: Box<TypeExpr>,
        span: Span,
    },

    /// Function type: `Fn(A, B) -> C`
    Function {
        params: Vec<TypeExpr>,
        return_type: Option<Box<TypeExpr>>,
        span: Span,
    },

    /// Module-qualified type: `std::collections::HashMap`
    Path {
        segments: Vec<TypeIdent>,
        span: Span,
    },

    /// Inferred type (placeholder)
    Inferred(Span),
}

impl TypeExpr {
    pub fn span(&self) -> Span {
        match self {
            TypeExpr::Named(id) => id.span,
            TypeExpr::Generic { span, .. } => *span,
            TypeExpr::Array { span, .. } => *span,
            TypeExpr::Map { span, .. } => *span,
            TypeExpr::Tuple { span, .. } => *span,
            TypeExpr::Optional { span, .. } => *span,
            TypeExpr::Result { span, .. } => *span,
            TypeExpr::Reference { span, .. } => *span,
            TypeExpr::Function { span, .. } => *span,
            TypeExpr::Path { span, .. } => *span,
            TypeExpr::Inferred(span) => *span,
        }
    }
}

// ============================================================================
// Expressions
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

impl Expr {
    pub fn new(kind: ExprKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// Part of an interpolated string
#[derive(Debug, Clone, PartialEq)]
pub enum StringPart {
    /// Literal string segment
    Literal(SmolStr),
    /// Interpolated expression: #{expr}
    Expr(Box<Expr>),
    /// Interpolated expression with format specifier: #{expr:format}
    FormattedExpr {
        expr: Box<Expr>,
        /// Format specifier (e.g., "02d", ".2f", "<10")
        format: SmolStr,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    // Literals
    Integer(SmolStr),
    Float(SmolStr),
    String(SmolStr),
    /// Interpolated string with parts: literal strings and embedded expressions
    /// e.g., "Hello, #{name}!" becomes InterpolatedString with parts:
    /// [StringPart::Literal("Hello, "), StringPart::Expr(name_expr), StringPart::Literal("!")]
    InterpolatedString(Vec<StringPart>),
    Char(SmolStr),
    Bool(bool),
    Nil,

    // Identifiers
    Ident(SmolStr),
    SelfLower,
    SelfUpper,

    // Collections
    Array(Vec<Expr>),
    Tuple(Vec<Expr>),
    Map(Vec<(Expr, Expr)>),

    // Operators
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },

    // Member access
    Field {
        object: Box<Expr>,
        field: Ident,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
    },
    MethodCall {
        object: Box<Expr>,
        method: Ident,
        args: Vec<Expr>,
    },

    // Function call
    Call {
        func: Box<Expr>,
        args: Vec<CallArg>,
    },

    // Control flow expressions
    If {
        condition: Box<Expr>,
        then_branch: Block,
        elsif_branches: Vec<(Expr, Block)>,
        else_branch: Option<Block>,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    Block(Block),

    // Lambda
    Lambda {
        params: Vec<Param>,
        body: Box<Expr>,
    },
    BlockLambda {
        params: Vec<Param>,
        body: Block,
    },

    // Comprehensions
    ArrayComprehension {
        element: Box<Expr>,
        pattern: Box<Pattern>,
        iterable: Box<Expr>,
        condition: Option<Box<Expr>>,
    },
    MapComprehension {
        key: Box<Expr>,
        value: Box<Expr>,
        pattern: Box<Pattern>,
        iterable: Box<Expr>,
        condition: Option<Box<Expr>>,
    },

    // Special
    Range {
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
        inclusive: bool,
    },
    Pipe {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Try(Box<Expr>),
    Unwrap(Box<Expr>),
    SafeNav {
        object: Box<Expr>,
        field: Ident,
    },

    // Struct instantiation
    StructInit {
        name: TypeIdent,
        fields: Vec<FieldInit>,
    },

    // Concurrency
    Spawn(Box<Expr>),
    Await(Box<Expr>),
    Select(Vec<SelectArm>),

    /// Channel send: `channel <- value`
    ChannelSend {
        channel: Box<Expr>,
        value: Box<Expr>,
    },

    /// Channel receive: `<- channel`
    ChannelRecv {
        channel: Box<Expr>,
    },

    // Contract expressions
    Old(Box<Expr>),
    Result,
    Forall {
        var: Ident,
        ty: TypeExpr,
        condition: Option<Box<Expr>>,
        body: Box<Expr>,
    },
    Exists {
        var: Ident,
        ty: TypeExpr,
        condition: Option<Box<Expr>>,
        body: Box<Expr>,
    },

    // Grouping
    Paren(Box<Expr>),

    // Ternary
    Ternary {
        condition: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
    },

    // Path expression (module::item)
    Path(Vec<Ident>),

    // Effect handling
    /// Handle expression: `handle expr with handlers end`
    Handle {
        /// The expression being handled
        body: Box<Expr>,
        /// Effect handler clauses
        handlers: Vec<HandlerClause>,
        /// Optional return clause: `return(x) => expr`
        return_clause: Option<Box<ReturnClause>>,
    },

    /// Raise expression: `Exception.raise(error)` or `raise error`
    Raise {
        /// The exception/error value being raised
        error: Box<Expr>,
        /// Optional exception type annotation
        exception_type: Option<TypeExpr>,
    },

    /// Resume expression (inside effect handlers): `resume(value)`
    Resume {
        /// The value to resume with
        value: Box<Expr>,
    },

    // Error placeholder
    Error,
}

/// A handler clause in a handle expression
/// Example: `Exception.raise(e) => fallback_value`
#[derive(Debug, Clone, PartialEq)]
pub struct HandlerClause {
    /// The effect being handled (e.g., "Exception", "IO", "Console")
    pub effect: TypeIdent,
    /// The operation being handled (e.g., "raise", "print", "get")
    pub operation: Ident,
    /// Patterns for the operation's parameters
    pub params: Vec<Pattern>,
    /// The handler body
    pub body: HandlerBody,
    pub span: Span,
}

/// The body of a handler clause
#[derive(Debug, Clone, PartialEq)]
pub enum HandlerBody {
    /// Single expression: `=> expr`
    Expr(Box<Expr>),
    /// Block body: `=> { stmts }`
    Block(Block),
}

/// Return clause in a handle expression
/// Example: `return(x) => x + 1`
#[derive(Debug, Clone, PartialEq)]
pub struct ReturnClause {
    /// Pattern to bind the return value
    pub pattern: Pattern,
    /// The body to transform the return value
    pub body: Box<HandlerBody>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CallArg {
    pub name: Option<Ident>,
    pub value: Expr,
    pub spread: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldInit {
    pub name: Ident,
    pub value: Option<Expr>,
    pub spread: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: MatchArmBody,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatchArmBody {
    Expr(Expr),
    Block(Block),
}

/// A select arm represents one case in a select expression.
///
/// Select can handle:
/// - Receive: `msg = <-inbox => handle(msg)`
/// - Send: `ch <- value => log("sent")`
/// - Default: `default => idle()`
#[derive(Debug, Clone, PartialEq)]
pub struct SelectArm {
    /// The operation (receive, send, or default)
    pub kind: SelectArmKind,
    /// The body to execute when this arm matches
    pub body: Expr,
    pub span: Span,
}

/// The kind of operation in a select arm
#[derive(Debug, Clone, PartialEq)]
pub enum SelectArmKind {
    /// Receive from channel: `pattern = <-channel` or `<-channel`
    Receive {
        /// Optional pattern to bind the received value
        pattern: Option<Pattern>,
        /// The channel expression to receive from
        channel: Box<Expr>,
    },
    /// Send to channel: `channel <- value`
    Send {
        /// The channel expression to send to
        channel: Box<Expr>,
        /// The value to send
        value: Box<Expr>,
    },
    /// Default case: `default`
    Default,
}

// ============================================================================
// Operators
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    IntDiv,
    Mod,
    Pow,

    // Comparison
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    Spaceship,

    // Logical
    And,
    Or,

    // Bitwise
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,

    // Other
    In,
    Is,
    Range,
    RangeExclusive,
    ApproxEq,

    // Assignment
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    IntDivAssign,
    ModAssign,
    BitAndAssign,
    BitOrAssign,
    BitXorAssign,
    ShlAssign,
    ShrAssign,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
    BitNot,
    Ref,
    Deref,
}

// ============================================================================
// Statements
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    /// Expression statement
    Expr(Expr),

    /// Let binding: `let x = expr` or `let (a, b) = expr`
    Let {
        pattern: Pattern,
        ty: Option<TypeExpr>,
        value: Expr,
    },

    /// Variable declaration: `x = expr` (mutable)
    Var {
        name: Ident,
        ty: Option<TypeExpr>,
        value: Expr,
    },

    /// Constant: `const X = expr`
    Const {
        name: Ident,
        ty: Option<TypeExpr>,
        value: Expr,
    },

    /// Assignment: `x = expr`
    Assign {
        target: Expr,
        op: Option<BinaryOp>,
        value: Expr,
    },

    /// For loop
    For {
        pattern: Pattern,
        iterable: Expr,
        body: Block,
    },

    /// While loop
    While {
        condition: Expr,
        body: Block,
    },

    /// Infinite loop
    Loop {
        body: Block,
    },

    /// If statement
    If {
        condition: Expr,
        then_branch: Block,
        elsif_branches: Vec<(Expr, Block)>,
        else_branch: Option<Block>,
    },

    /// Unless statement
    Unless {
        condition: Expr,
        body: Block,
        else_branch: Option<Block>,
    },

    /// Match statement
    Match {
        scrutinee: Expr,
        arms: Vec<MatchArm>,
    },

    /// Return
    Return(Option<Expr>),

    /// Break (with optional value)
    Break(Option<Expr>),

    /// Continue
    Continue,

    /// Defer
    Defer(Expr),

    /// Unsafe block
    Unsafe(Block),

    /// Item declaration (nested function, struct, etc.)
    Item(Box<Item>),
}

// ============================================================================
// Patterns
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    pub kind: PatternKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatternKind {
    /// Wildcard: `_`
    Wildcard,

    /// Literal: `42`, `"hello"`, `true`
    Literal(Box<Expr>),

    /// Identifier binding: `x`
    Ident(SmolStr),

    /// Tuple: `(a, b, c)`
    Tuple(Vec<Pattern>),

    /// Array: `[a, b, ...rest]`
    Array {
        elements: Vec<Pattern>,
        rest: Option<Box<Pattern>>,
    },

    /// Struct: `Point(x:, y:)` or `{x:, y:}`
    Struct {
        name: Option<TypeIdent>,
        fields: Vec<FieldPattern>,
    },

    /// Enum variant: `Some(x)` or `None`
    Variant {
        path: Vec<TypeIdent>,
        variant: Ident,
        fields: Option<Vec<Pattern>>,
    },

    /// Range: `1..10`
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        inclusive: bool,
    },

    /// Or pattern: `A | B | C`
    Or(Vec<Pattern>),

    /// Guard: `x if x > 0`
    Guard {
        pattern: Box<Pattern>,
        condition: Box<Expr>,
    },

    /// Binding: `x @ Pattern`
    Binding {
        name: Ident,
        pattern: Box<Pattern>,
    },

    /// Type annotation: `x: Int`
    Typed {
        pattern: Box<Pattern>,
        ty: TypeExpr,
    },

    /// Rest pattern: `...` or `...name`
    Rest(Option<Ident>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldPattern {
    pub name: Ident,
    pub pattern: Option<Pattern>,
    pub span: Span,
}

// ============================================================================
// Data Structures
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct StructDecl {
    pub attributes: Vec<Attribute>,
    pub visibility: Visibility,
    pub name: TypeIdent,
    pub generic_params: Option<GenericParams>,
    pub fields: Vec<StructField>,
    pub derive: Vec<TypeIdent>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub visibility: Visibility,
    pub name: Ident,
    pub ty: TypeExpr,
    pub default: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DataDecl {
    pub attributes: Vec<Attribute>,
    pub visibility: Visibility,
    pub name: TypeIdent,
    pub generic_params: Option<GenericParams>,
    pub fields: Vec<DataField>,
    pub derive: Vec<TypeIdent>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DataField {
    pub name: Ident,
    pub ty: TypeExpr,
    pub default: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    pub attributes: Vec<Attribute>,
    pub visibility: Visibility,
    pub name: TypeIdent,
    pub generic_params: Option<GenericParams>,
    pub variants: Vec<EnumVariant>,
    pub derive: Vec<TypeIdent>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub name: Ident,
    pub data: EnumVariantData,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EnumVariantData {
    Unit,
    Tuple(Vec<TypeExpr>),
    Struct(Vec<DataField>),
    Discriminant(Expr),
}

// ============================================================================
// Traits and Implementations
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct TraitDecl {
    pub attributes: Vec<Attribute>,
    pub visibility: Visibility,
    pub name: TypeIdent,
    pub generic_params: Option<GenericParams>,
    pub supertraits: Vec<TraitBound>,
    pub members: Vec<TraitMember>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TraitMember {
    Method(TraitMethod),
    Const(TraitConst),
    Type(TraitType),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraitMethod {
    pub name: Ident,
    pub generic_params: Option<GenericParams>,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub default: Option<FunctionBody>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraitConst {
    pub name: Ident,
    pub ty: TypeExpr,
    pub default: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraitType {
    pub name: TypeIdent,
    pub bounds: Vec<TraitBound>,
    pub default: Option<TypeExpr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImplDecl {
    pub attributes: Vec<Attribute>,
    pub generic_params: Option<GenericParams>,
    pub trait_: Option<TraitBound>,
    pub for_type: TypeExpr,
    pub where_clause: Option<WhereClause>,
    pub members: Vec<ImplMember>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImplMember {
    Function(FunctionDecl),
    Const(ConstDecl),
    Type(TypeAlias),
}

// ============================================================================
// Other Declarations
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct ConstDecl {
    pub attributes: Vec<Attribute>,
    pub visibility: Visibility,
    pub name: Ident,
    pub ty: Option<TypeExpr>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeAlias {
    pub attributes: Vec<Attribute>,
    pub visibility: Visibility,
    pub name: TypeIdent,
    pub generic_params: Option<GenericParams>,
    pub ty: TypeExpr,
    pub span: Span,
}

// ============================================================================
// Testing Constructs
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct TestBlock {
    pub examples: Option<ExamplesBlock>,
    pub properties: Vec<PropertyBlock>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExamplesBlock {
    pub assertions: Vec<ExampleAssertion>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExampleAssertion {
    Comparison {
        left: Expr,
        op: BinaryOp,
        right: Expr,
    },
    Truthy(Expr),
    Raises {
        exception: TypeIdent,
        expr: Expr,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct PropertyBlock {
    pub name: SmolStr,
    pub body: Vec<PropertyAssertion>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PropertyAssertion {
    Quantified {
        var: Ident,
        ty: TypeExpr,
        condition: Option<Expr>,
        body: Expr,
    },
    Simple(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TestDecl {
    pub name: SmolStr,
    pub body: Block,
    pub span: Span,
}

// ============================================================================
// FFI / Extern
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum ExternDecl {
    C(ExternC),
    Python(ExternPython),
    Wasm(ExternWasm),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExternC {
    pub header: SmolStr,
    pub alias: Option<Ident>,
    pub items: Vec<ExternCItem>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExternCItem {
    Function(ExternFunction),
    Struct(ExternStruct),
    Const(ExternConst),
    Type(TypeIdent),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExternFunction {
    pub name: Ident,
    pub params: Vec<ExternParam>,
    pub return_type: Option<CType>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExternParam {
    pub name: Option<Ident>,
    pub ty: CType,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExternStruct {
    pub name: TypeIdent,
    pub fields: Vec<ExternField>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExternField {
    pub name: Ident,
    pub ty: CType,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExternConst {
    pub name: Ident,
    pub ty: CType,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CType {
    Int,
    UInt,
    Long,
    ULong,
    LongLong,
    Float,
    Double,
    Char,
    Void,
    SizeT,
    SSizeT,
    Pointer {
        const_: bool,
        pointee: Box<CType>,
    },
    Named(TypeIdent),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExternPython {
    pub module: SmolStr,
    pub alias: Option<Ident>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExternWasm {
    pub kind: WasmExternKind,
    pub name: Ident,
    pub items: Vec<ExternFunction>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasmExternKind {
    Import,
    Export,
}

// ============================================================================
// Attributes
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    pub inner: bool,
    pub name: Ident,
    pub args: Vec<AttributeArg>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AttributeArg {
    Ident(Ident),
    NameValue { name: Ident, value: Expr },
    Expr(Expr),
}
