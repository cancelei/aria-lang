//! Aria Language Type System
//!
//! Type representation, type checking, and type inference for Aria.
//!
//! ## Bidirectional Type Checking
//!
//! This module implements bidirectional type checking, which combines:
//! - **Synthesis mode**: Infer the type of an expression from its structure
//! - **Checking mode**: Verify an expression matches an expected type
//!
//! Bidirectional checking enables better error messages and allows type
//! information to flow into lambda parameters from their context.
//!
//! ## Flow-Sensitive Type Narrowing
//!
//! This module implements flow-sensitive typing, which allows the type checker
//! to narrow variable types based on control flow conditions. For example:
//!
//! ```aria
//! fn process(value: Int?) -> Int
//!   if value != nil
//!     # value automatically narrows from Int? to Int
//!     return value + 1
//!   end
//!   return 0
//! end
//! ```
//!
//! Supported narrowing patterns:
//! - **Nil checks**: `x != nil` narrows `T?` to `T`
//! - **Type guards**: `x is SomeType` narrows to `SomeType`
//! - **Logical combinations**: `a != nil and b != nil` narrows both
//! - **Negation**: `!(x == nil)` is equivalent to `x != nil`
//!
//! ## Performance Optimizations
//!
//! The type checker includes several performance optimizations:
//!
//! 1. **Static Primitive Type Lookup**: A lazily-initialized static hash map
//!    (`primitive_type_lookup()`) provides O(1) lookups for primitive types
//!    like `Int`, `Float`, `Bool`, etc., avoiding repeated string matching.
//!
//! 2. **Smart Substitution Application**: The `TypeInference::apply()` method
//!    uses a `needs_apply()` check to short-circuit when a type contains no
//!    type variables that need substitution, avoiding unnecessary allocations.
//!
//! 3. **Efficient Environment Scoping**: `TypeEnv::child_scope()` creates child
//!    scopes by sharing the parent through `Rc`, avoiding full clones when
//!    the parent environment is already reference-counted.

use aria_ast::{self as ast, Span};
use aria_contracts::{ContractVerifier, ContractTier, VerifierConfig};
use rustc_hash::FxHashMap;
use std::rc::Rc;
use std::sync::OnceLock;
use thiserror::Error;

// ============================================================================
// Compile-Time Constant Evaluation
// ============================================================================

/// Represents a compile-time constant value.
///
/// Const values are computed at compile time and can be used in contexts
/// that require compile-time evaluation, such as array sizes, const declarations,
/// and constant folding optimizations.
#[derive(Debug, Clone, PartialEq)]
pub enum ConstValue {
    /// Integer constant (stored as i128 to handle all integer types)
    Int(i128),
    /// Unsigned integer constant (stored as u128 to handle all unsigned types)
    UInt(u128),
    /// Floating-point constant (stored as f64 for maximum precision)
    Float(f64),
    /// Boolean constant
    Bool(bool),
    /// Character constant
    Char(char),
    /// String constant
    String(String),
    /// Unit value
    Unit,
    /// Array of constant values
    Array(Vec<ConstValue>),
    /// Tuple of constant values
    Tuple(Vec<ConstValue>),
}

impl ConstValue {
    /// Get the type of this constant value
    pub fn ty(&self) -> Type {
        match self {
            ConstValue::Int(_) => Type::Int,
            ConstValue::UInt(_) => Type::UInt,
            ConstValue::Float(_) => Type::Float,
            ConstValue::Bool(_) => Type::Bool,
            ConstValue::Char(_) => Type::Char,
            ConstValue::String(_) => Type::String,
            ConstValue::Unit => Type::Unit,
            ConstValue::Array(elems) => {
                if elems.is_empty() {
                    Type::Array(Box::new(Type::Any))
                } else {
                    Type::Array(Box::new(elems[0].ty()))
                }
            }
            ConstValue::Tuple(elems) => {
                Type::Tuple(elems.iter().map(|e| e.ty()).collect())
            }
        }
    }

    /// Try to convert this value to an i128
    pub fn as_int(&self) -> Option<i128> {
        match self {
            ConstValue::Int(n) => Some(*n),
            ConstValue::UInt(n) => i128::try_from(*n).ok(),
            _ => None,
        }
    }

    /// Try to convert this value to a u128
    pub fn as_uint(&self) -> Option<u128> {
        match self {
            ConstValue::UInt(n) => Some(*n),
            ConstValue::Int(n) if *n >= 0 => Some(*n as u128),
            _ => None,
        }
    }

    /// Try to convert this value to an f64
    pub fn as_float(&self) -> Option<f64> {
        match self {
            ConstValue::Float(f) => Some(*f),
            ConstValue::Int(n) => Some(*n as f64),
            ConstValue::UInt(n) => Some(*n as f64),
            _ => None,
        }
    }

    /// Try to convert this value to a bool
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ConstValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Try to convert this value to a usize (for array sizes)
    pub fn as_usize(&self) -> Option<usize> {
        match self {
            ConstValue::Int(n) if *n >= 0 => usize::try_from(*n).ok(),
            ConstValue::UInt(n) => usize::try_from(*n).ok(),
            _ => None,
        }
    }
}

impl std::fmt::Display for ConstValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConstValue::Int(n) => write!(f, "{}", n),
            ConstValue::UInt(n) => write!(f, "{}", n),
            ConstValue::Float(n) => write!(f, "{}", n),
            ConstValue::Bool(b) => write!(f, "{}", b),
            ConstValue::Char(c) => write!(f, "'{}'", c),
            ConstValue::String(s) => write!(f, "\"{}\"", s),
            ConstValue::Unit => write!(f, "()"),
            ConstValue::Array(elems) => {
                write!(f, "[")?;
                for (i, elem) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", elem)?;
                }
                write!(f, "]")
            }
            ConstValue::Tuple(elems) => {
                write!(f, "(")?;
                for (i, elem) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", elem)?;
                }
                write!(f, ")")
            }
        }
    }
}

// ============================================================================
// Static Primitive Type Lookup (Performance Optimization)
// ============================================================================

/// Get the static lookup table for primitive type names.
/// Using OnceLock for lazy initialization ensures this is only computed once.
fn primitive_type_lookup() -> &'static FxHashMap<&'static str, Type> {
    static PRIMITIVES: OnceLock<FxHashMap<&'static str, Type>> = OnceLock::new();
    PRIMITIVES.get_or_init(|| {
        let mut map = FxHashMap::default();
        map.insert("Int", Type::Int);
        map.insert("Int8", Type::Int8);
        map.insert("Int16", Type::Int16);
        map.insert("Int32", Type::Int32);
        map.insert("Int64", Type::Int64);
        map.insert("Int128", Type::Int128);
        map.insert("UInt", Type::UInt);
        map.insert("UInt8", Type::UInt8);
        map.insert("UInt16", Type::UInt16);
        map.insert("UInt32", Type::UInt32);
        map.insert("UInt64", Type::UInt64);
        map.insert("UInt128", Type::UInt128);
        map.insert("Float", Type::Float);
        map.insert("Float32", Type::Float32);
        map.insert("Float64", Type::Float64);
        map.insert("Bool", Type::Bool);
        map.insert("Char", Type::Char);
        map.insert("String", Type::String);
        map.insert("Bytes", Type::Bytes);
        map.insert("Unit", Type::Unit);
        map.insert("Never", Type::Never);
        map
    })
}

// ============================================================================
// Bidirectional Type Checking Infrastructure
// ============================================================================

/// Mode for type checking - determines whether we synthesize or check against expected type
#[derive(Debug, Clone, PartialEq)]
pub enum CheckMode {
    /// Synthesize type from expression (bottom-up inference)
    Synthesize,
    /// Check expression against expected type (top-down checking)
    Check {
        expected: Type,
        source: TypeSource,
    },
}

impl CheckMode {
    /// Create a new Check mode with the given expected type and source
    pub fn check(expected: Type, source: TypeSource) -> Self {
        CheckMode::Check { expected, source }
    }

    /// Create Synthesize mode
    pub fn synthesize() -> Self {
        CheckMode::Synthesize
    }

    /// Returns true if this is checking mode
    pub fn is_check(&self) -> bool {
        matches!(self, CheckMode::Check { .. })
    }

    /// Returns true if this is synthesize mode
    pub fn is_synthesize(&self) -> bool {
        matches!(self, CheckMode::Synthesize)
    }

    /// Get the expected type if in checking mode
    pub fn expected_type(&self) -> Option<&Type> {
        match self {
            CheckMode::Check { expected, .. } => Some(expected),
            CheckMode::Synthesize => None,
        }
    }

    /// Get the source if in checking mode
    pub fn source(&self) -> Option<&TypeSource> {
        match self {
            CheckMode::Check { source, .. } => Some(source),
            CheckMode::Synthesize => None,
        }
    }
}

/// Source of type expectation for enhanced error messages
///
/// Tracks where type expectations originate, enabling Elm-level error messages
/// that explain "I expected X because Y, but found Z from W".
#[derive(Debug, Clone, PartialEq)]
pub enum TypeSource {
    /// From explicit type annotation (e.g., `let x: Int = ...`)
    Annotation(Span),

    /// From function parameter (e.g., callback parameter in `map(|x| ...)`)
    Parameter {
        name: String,
        span: Span,
    },

    /// From function return type
    Return(Span),

    /// From surrounding context (e.g., array element, map value)
    Context {
        description: String,
        span: Span,
    },

    /// From assignment target
    Assignment(Span),

    /// From binary operator expectation
    BinaryOperator {
        op: String,
        side: BinaryOpSide,
        span: Span,
    },

    /// From conditional expression (if/ternary branches must match)
    ConditionalBranch(Span),

    /// Unknown or internal source
    Unknown,
}

/// Which side of a binary operator the expectation comes from
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOpSide {
    Left,
    Right,
}

// ============================================================================
// Flow-Sensitive Type Narrowing Infrastructure
// ============================================================================

/// Active type narrowing in current scope
///
/// Represents a narrowed type for a variable based on control flow analysis.
/// For example, after `if x != nil`, the variable `x` is narrowed from `T?` to `T`.
#[derive(Debug, Clone)]
pub struct Narrowing {
    /// The variable name being narrowed
    pub var_name: String,
    /// The narrowed type after the condition
    pub narrowed_type: Type,
    /// Source span of the condition that caused the narrowing
    pub condition_span: Span,
}

impl Narrowing {
    /// Create a new narrowing for a variable
    pub fn new(var_name: String, narrowed_type: Type, condition_span: Span) -> Self {
        Self {
            var_name,
            narrowed_type,
            condition_span,
        }
    }
}

/// Flow-sensitive type environment
///
/// Wraps a `TypeEnv` and tracks active type narrowings from control flow analysis.
/// This enables automatic type narrowing after guards like `if x != nil` or `x is SomeType`.
///
/// # Example
///
/// ```ignore
/// fn process(value: Int?) -> Int
///   if value != nil
///     # FlowTypeEnv narrows value from Int? to Int
///     return value + 1
///   end
///   return 0
/// end
/// ```
#[derive(Debug, Clone)]
pub struct FlowTypeEnv {
    /// Base type environment with standard variable bindings
    base: TypeEnv,
    /// Active narrowings in this scope (most recent takes precedence)
    narrowings: Vec<Narrowing>,
}

impl FlowTypeEnv {
    /// Create a new flow-sensitive environment from a base TypeEnv
    pub fn new(base: TypeEnv) -> Self {
        Self {
            base,
            narrowings: Vec::new(),
        }
    }

    /// Create a flow environment with a parent scope
    pub fn with_parent(parent: Rc<TypeEnv>) -> Self {
        Self {
            base: TypeEnv::with_parent(parent),
            narrowings: Vec::new(),
        }
    }

    /// Create a child flow environment inheriting narrowings from parent
    pub fn child_scope(&self) -> Self {
        Self {
            base: TypeEnv::with_parent(Rc::new(self.base.clone())),
            narrowings: self.narrowings.clone(),
        }
    }

    /// Narrow a variable's type in this scope
    ///
    /// Adds a narrowing that refines the variable's type. Multiple narrowings
    /// can be active for the same variable (most recent takes precedence).
    pub fn narrow(&mut self, var: String, ty: Type, span: Span) {
        self.narrowings.push(Narrowing::new(var, ty, span));
    }

    /// Look up a variable's type, considering any active narrowings
    ///
    /// Returns the narrowed type if one exists, otherwise falls back to the
    /// base environment's type for the variable.
    pub fn lookup_var(&self, name: &str) -> Option<Type> {
        // Check narrowings in reverse order (most recent first)
        for narrowing in self.narrowings.iter().rev() {
            if narrowing.var_name == name {
                return Some(narrowing.narrowed_type.clone());
            }
        }

        // Fall back to base environment
        self.base.lookup_var(name).cloned()
    }

    /// Look up type scheme in base environment
    pub fn lookup_type(&self, name: &str) -> Option<&TypeScheme> {
        self.base.lookup_type(name)
    }

    /// Define a new variable in this scope
    pub fn define_var(&mut self, name: String, ty: Type) {
        self.base.define_var(name, ty);
    }

    /// Define a new type in this scope
    pub fn define_type(&mut self, name: String, scheme: TypeScheme) {
        self.base.define_type(name, scheme);
    }

    /// Get a reference to the base TypeEnv
    pub fn base(&self) -> &TypeEnv {
        &self.base
    }

    /// Get a mutable reference to the base TypeEnv
    pub fn base_mut(&mut self) -> &mut TypeEnv {
        &mut self.base
    }

    /// Get the current narrowings
    pub fn narrowings(&self) -> &[Narrowing] {
        &self.narrowings
    }

    /// Clear all narrowings (useful when exiting a scope)
    pub fn clear_narrowings(&mut self) {
        self.narrowings.clear();
    }

    /// Remove narrowing for a specific variable (e.g., after reassignment)
    pub fn invalidate_narrowing(&mut self, var_name: &str) {
        self.narrowings.retain(|n| n.var_name != var_name);
    }

    /// Apply multiple narrowings at once
    pub fn apply_narrowings(&mut self, narrowings: Vec<Narrowing>) {
        self.narrowings.extend(narrowings);
    }
}

impl Default for FlowTypeEnv {
    fn default() -> Self {
        Self::new(TypeEnv::new())
    }
}

/// The kind of narrowing condition detected
#[derive(Debug, Clone, PartialEq)]
pub enum NarrowingKind {
    /// Nil check: `x != nil` narrows T? to T
    NilCheck {
        var_name: String,
        is_not_nil: bool,
    },
    /// Type guard: `x is SomeType` narrows to SomeType
    TypeGuard {
        var_name: String,
        target_type: Type,
        is_positive: bool,
    },
}

/// Extract narrowings from a condition expression
///
/// Analyzes a boolean condition and extracts any type narrowings that should
/// be applied if the condition is true.
///
/// # Supported patterns
///
/// - `x != nil` - narrows `T?` to `T` in the true branch
/// - `x == nil` - narrows to `T?` (no change), but narrowed to `T` in else branch
/// - `x is SomeType` - narrows to `SomeType` in the true branch
/// - `!(x is SomeType)` - narrows to exclude `SomeType` in the true branch
///
/// # Arguments
///
/// * `condition` - The condition expression to analyze
/// * `env` - The current type environment for looking up variable types
///
/// # Returns
///
/// A vector of `Narrowing` structs for narrowings active in the "true" branch.
/// To get narrowings for the "false" branch, use `extract_else_narrowings`.
pub fn extract_narrowings(condition: &ast::Expr, env: &TypeEnv) -> Vec<Narrowing> {
    let mut narrowings = Vec::new();

    match &condition.kind {
        // Pattern: x != nil
        ast::ExprKind::Binary {
            op: ast::BinaryOp::NotEq,
            left,
            right,
        } => {
            if let Some(narrowing) = check_nil_comparison(left, right, true, condition.span, env) {
                narrowings.push(narrowing);
            } else if let Some(narrowing) = check_nil_comparison(right, left, true, condition.span, env) {
                narrowings.push(narrowing);
            }
        }

        // Pattern: x == nil (for else branch analysis)
        ast::ExprKind::Binary {
            op: ast::BinaryOp::Eq,
            left: _,
            right: _,
        } => {
            // For `x == nil`, the "true" branch means x IS nil, so no narrowing there.
            // The narrowing happens in the else branch (handled by extract_else_narrowings)
            // But we record nothing for the true branch here.
        }

        // Pattern: x is SomeType
        ast::ExprKind::Binary {
            op: ast::BinaryOp::Is,
            left,
            right,
        } => {
            if let Some(narrowing) = check_type_guard(left, right, true, condition.span) {
                narrowings.push(narrowing);
            }
        }

        // Pattern: !expr (negation)
        ast::ExprKind::Unary {
            op: ast::UnaryOp::Not,
            operand,
        } => {
            // For negation, extract narrowings for the else branch of inner condition
            let inner_else_narrowings = extract_else_narrowings(operand, env);
            narrowings.extend(inner_else_narrowings);
        }

        // Pattern: expr1 and expr2 (both must be true)
        ast::ExprKind::Binary {
            op: ast::BinaryOp::And,
            left,
            right,
        } => {
            // Both conditions are true, so collect narrowings from both
            narrowings.extend(extract_narrowings(left, env));
            narrowings.extend(extract_narrowings(right, env));
        }

        _ => {}
    }

    narrowings
}

/// Extract narrowings for the "else" (false) branch of a condition
///
/// This is the complement of `extract_narrowings`. For example:
/// - `x == nil` in else branch means `x != nil`, so narrow `T?` to `T`
/// - `x is SomeType` in else branch means x is NOT SomeType
pub fn extract_else_narrowings(condition: &ast::Expr, env: &TypeEnv) -> Vec<Narrowing> {
    let mut narrowings = Vec::new();

    match &condition.kind {
        // Pattern: x == nil -> in else branch, x != nil
        ast::ExprKind::Binary {
            op: ast::BinaryOp::Eq,
            left,
            right,
        } => {
            if let Some(narrowing) = check_nil_comparison(left, right, true, condition.span, env) {
                narrowings.push(narrowing);
            } else if let Some(narrowing) = check_nil_comparison(right, left, true, condition.span, env) {
                narrowings.push(narrowing);
            }
        }

        // Pattern: x != nil -> in else branch, x == nil (no useful narrowing)
        ast::ExprKind::Binary {
            op: ast::BinaryOp::NotEq,
            ..
        } => {
            // In else branch, x could be nil, so no narrowing
        }

        // Pattern: x is SomeType -> in else branch, x is NOT SomeType
        ast::ExprKind::Binary {
            op: ast::BinaryOp::Is,
            left: _,
            right: _,
        } => {
            // For the else branch, we could track negative narrowing
            // For now, we don't narrow in the else branch of type guards
        }

        // Pattern: !expr -> else branch means expr was true
        ast::ExprKind::Unary {
            op: ast::UnaryOp::Not,
            operand,
        } => {
            narrowings.extend(extract_narrowings(operand, env));
        }

        // Pattern: expr1 or expr2 -> in else branch, BOTH are false
        ast::ExprKind::Binary {
            op: ast::BinaryOp::Or,
            left,
            right,
        } => {
            narrowings.extend(extract_else_narrowings(left, env));
            narrowings.extend(extract_else_narrowings(right, env));
        }

        _ => {}
    }

    narrowings
}

/// Check if a comparison is a nil check and return the narrowing if so
fn check_nil_comparison(
    var_expr: &ast::Expr,
    nil_expr: &ast::Expr,
    is_not_nil: bool,
    span: Span,
    env: &TypeEnv,
) -> Option<Narrowing> {
    // Check if nil_expr is actually nil
    if !matches!(nil_expr.kind, ast::ExprKind::Nil) {
        return None;
    }

    // Check if var_expr is an identifier
    let var_name = match &var_expr.kind {
        ast::ExprKind::Ident(name) => name.to_string(),
        _ => return None,
    };

    // Only narrow if the condition is "not nil"
    if !is_not_nil {
        return None;
    }

    // Look up the variable's type and narrow if it's Optional
    if let Some(var_type) = env.lookup_var(&var_name) {
        if let Type::Optional(inner) = var_type {
            // Narrow from T? to T
            return Some(Narrowing::new(var_name, (**inner).clone(), span));
        }
    }

    None
}

/// Check if an expression is a type guard and return the narrowing if so
fn check_type_guard(
    var_expr: &ast::Expr,
    type_expr: &ast::Expr,
    is_positive: bool,
    span: Span,
) -> Option<Narrowing> {
    // Only handle positive type guards for now
    if !is_positive {
        return None;
    }

    // Check if var_expr is an identifier
    let var_name = match &var_expr.kind {
        ast::ExprKind::Ident(name) => name.to_string(),
        _ => return None,
    };

    // Check if type_expr is a type identifier
    let target_type = match &type_expr.kind {
        ast::ExprKind::Ident(name) => {
            let name_str = name.as_str();
            // Fast path: check static primitive lookup table first
            if let Some(prim_type) = primitive_type_lookup().get(name_str) {
                prim_type.clone()
            } else {
                Type::Named {
                    name: name_str.to_string(),
                    type_args: Vec::new(),
                }
            }
        }
        _ => return None,
    };

    Some(Narrowing::new(var_name, target_type, span))
}

/// Type variable ID for inference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeVar(pub u32);

/// Effect row variable ID for effect inference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EffectRowVar(pub u32);

// ============================================================================
// Effect System Types
// ============================================================================

/// An effect represents a computational side effect that a function may perform.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    /// IO effect - file system, network operations
    IO,
    /// Console effect - print/read console operations
    Console,
    /// Async effect - async/await operations
    Async,
    /// Mutation effect - mutable state operations
    Mutation,
    /// Exception effect with error type
    Exception(Box<Type>),
    /// State effect with state type
    State(Box<Type>),
    /// Reader effect with environment type
    Reader(Box<Type>),
    /// Writer effect with output type
    Writer(Box<Type>),
    /// Custom user-defined effect
    Custom {
        name: String,
        type_args: Vec<Type>,
    },
}

impl std::fmt::Display for Effect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Effect::IO => write!(f, "IO"),
            Effect::Console => write!(f, "Console"),
            Effect::Async => write!(f, "Async"),
            Effect::Mutation => write!(f, "Mutation"),
            Effect::Exception(ty) => write!(f, "Exception[{}]", ty),
            Effect::State(ty) => write!(f, "State[{}]", ty),
            Effect::Reader(ty) => write!(f, "Reader[{}]", ty),
            Effect::Writer(ty) => write!(f, "Writer[{}]", ty),
            Effect::Custom { name, type_args } => {
                write!(f, "{}", name)?;
                if !type_args.is_empty() {
                    write!(f, "[")?;
                    for (i, arg) in type_args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", arg)?;
                    }
                    write!(f, "]")?;
                }
                Ok(())
            }
        }
    }
}

/// An effect row represents a set of effects, optionally open (polymorphic).
#[derive(Debug, Clone, PartialEq)]
pub struct EffectRow {
    /// The concrete effects in this row
    pub effects: Vec<Effect>,
    /// Optional row variable for polymorphism (open row)
    pub row_var: Option<EffectRowVar>,
}

impl EffectRow {
    /// Create an empty (pure) effect row
    pub fn pure() -> Self {
        Self {
            effects: Vec::new(),
            row_var: None,
        }
    }

    /// Create a closed effect row with the given effects
    pub fn closed(effects: Vec<Effect>) -> Self {
        Self {
            effects,
            row_var: None,
        }
    }

    /// Create an open effect row with a row variable
    pub fn open(effects: Vec<Effect>, row_var: EffectRowVar) -> Self {
        Self {
            effects,
            row_var: Some(row_var),
        }
    }

    /// Create an effect row with just a row variable (fully polymorphic)
    pub fn var(row_var: EffectRowVar) -> Self {
        Self {
            effects: Vec::new(),
            row_var: Some(row_var),
        }
    }

    /// Check if this is a pure (empty, closed) effect row
    pub fn is_pure(&self) -> bool {
        self.effects.is_empty() && self.row_var.is_none()
    }

    /// Check if this row is closed (no row variable)
    pub fn is_closed(&self) -> bool {
        self.row_var.is_none()
    }

    /// Check if this row contains a specific effect
    pub fn contains(&self, effect: &Effect) -> bool {
        self.effects.contains(effect)
    }

    /// Add an effect to this row (returns new row, preserves openness)
    pub fn with_effect(mut self, effect: Effect) -> Self {
        if !self.effects.contains(&effect) {
            self.effects.push(effect);
        }
        self
    }

    /// Merge two effect rows (union of effects)
    pub fn merge(&self, other: &EffectRow) -> EffectRow {
        let mut effects = self.effects.clone();
        for effect in &other.effects {
            if !effects.contains(effect) {
                effects.push(effect.clone());
            }
        }
        let row_var = self.row_var.or(other.row_var);
        EffectRow { effects, row_var }
    }

    /// Remove an effect from this row (for handler elimination)
    pub fn without_effect(&self, effect: &Effect) -> EffectRow {
        EffectRow {
            effects: self.effects.iter()
                .filter(|e| *e != effect)
                .cloned()
                .collect(),
            row_var: self.row_var,
        }
    }

    /// Check if this row is a subset of another (subtyping)
    pub fn is_subset_of(&self, other: &EffectRow) -> bool {
        if other.row_var.is_some() {
            return self.effects.iter().all(|e| other.effects.contains(e) || true);
        }
        self.effects.iter().all(|e| other.effects.contains(e))
    }
}

impl Default for EffectRow {
    fn default() -> Self {
        Self::pure()
    }
}

impl std::fmt::Display for EffectRow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "!{{")?;
        for (i, effect) in self.effects.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", effect)?;
        }
        if let Some(var) = self.row_var {
            if !self.effects.is_empty() {
                write!(f, " | ")?;
            }
            write!(f, "e{}", var.0)?;
        }
        write!(f, "}}")
    }
}


/// Aria types
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    // Primitive types
    Int,
    Int8,
    Int16,
    Int32,
    Int64,
    Int128,
    UInt,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    UInt128,
    Float,
    Float32,
    Float64,
    Bool,
    Char,
    String,
    Bytes,
    Unit,
    Never,

    // Compound types
    Array(Box<Type>),
    FixedArray(Box<Type>, usize),
    Map(Box<Type>, Box<Type>),
    Tuple(Vec<Type>),
    Optional(Box<Type>),
    Result(Box<Type>, Box<Type>),
    Reference { mutable: bool, inner: Box<Type> },
    Function { params: Vec<Type>, return_type: Box<Type> },

    /// Channel type for inter-task communication.
    ///
    /// Channels are typed, bounded message queues used for communication
    /// between concurrent tasks. A Channel[T] can send and receive values
    /// of type T.
    ///
    /// Properties:
    /// - Transfer: if T is Transfer (can send channel to another task)
    /// - Sharable: always (channels are designed for sharing between tasks)
    Channel(Box<Type>),

    /// Task type for concurrent computation.
    ///
    /// A Task[T] represents a spawned concurrent computation that will
    /// eventually produce a value of type T. Tasks can be awaited to
    /// retrieve their result.
    ///
    /// Properties:
    /// - Transfer: if T is Transfer (task handles can be sent between tasks)
    /// - Sharable: always (task handles are designed for sharing)
    ///
    /// Created by: spawn expressions
    /// Consumed by: await expressions
    Task(Box<Type>),

    /// Function type with effects.
    ///
    /// A function that may perform effects declared in its effect row.
    /// The syntax is: `fn(params) !{effects} -> return_type`
    ///
    /// Examples:
    /// - `fn(Int) !{IO} -> String` - function that performs IO
    /// - `fn() !{} -> Int` - pure function
    /// - `fn(T) !{e} -> T` - effect-polymorphic function
    EffectfulFunction {
        params: Vec<Type>,
        effects: EffectRow,
        return_type: Box<Type>,
    },

    // Named types
    Named {
        name: String,
        type_args: Vec<Type>,
    },

    /// Effect row variable for effect inference
    EffectVar(EffectRowVar),

    // Type variable (for inference)
    Var(TypeVar),

    // Error type (for error recovery)
    Error,

    // Any type (for polymorphic builtins)
    // Matches any other type during type checking
    Any,
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // Primitive types
            Type::Int => write!(f, "Int"),
            Type::Int8 => write!(f, "Int8"),
            Type::Int16 => write!(f, "Int16"),
            Type::Int32 => write!(f, "Int32"),
            Type::Int64 => write!(f, "Int64"),
            Type::Int128 => write!(f, "Int128"),
            Type::UInt => write!(f, "UInt"),
            Type::UInt8 => write!(f, "UInt8"),
            Type::UInt16 => write!(f, "UInt16"),
            Type::UInt32 => write!(f, "UInt32"),
            Type::UInt64 => write!(f, "UInt64"),
            Type::UInt128 => write!(f, "UInt128"),
            Type::Float => write!(f, "Float"),
            Type::Float32 => write!(f, "Float32"),
            Type::Float64 => write!(f, "Float64"),
            Type::Bool => write!(f, "Bool"),
            Type::Char => write!(f, "Char"),
            Type::String => write!(f, "String"),
            Type::Bytes => write!(f, "Bytes"),
            Type::Unit => write!(f, "()"),
            Type::Never => write!(f, "!"),

            // Compound types
            Type::Array(elem) => write!(f, "[{}]", elem),
            Type::FixedArray(elem, size) => write!(f, "[{}; {}]", elem, size),
            Type::Map(key, val) => write!(f, "Map<{}, {}>", key, val),
            Type::Tuple(types) => {
                write!(f, "(")?;
                for (i, ty) in types.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", ty)?;
                }
                write!(f, ")")
            }
            Type::Optional(inner) => write!(f, "{}?", inner),
            Type::Result(ok, err) => write!(f, "Result<{}, {}>", ok, err),
            Type::Reference { mutable, inner } => {
                if *mutable {
                    write!(f, "&mut {}", inner)
                } else {
                    write!(f, "&{}", inner)
                }
            }
            Type::Function { params, return_type } => {
                write!(f, "fn(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {}", return_type)
            }
            Type::Channel(elem) => write!(f, "Channel[{}]", elem),
            Type::Task(result) => write!(f, "Task[{}]", result),

            Type::EffectfulFunction { params, effects, return_type } => {
                write!(f, "fn(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ") ")?;
                if !effects.is_pure() {
                    write!(f, "{} ", effects)?;
                }
                write!(f, "-> {}", return_type)
            }

            // Named types
            Type::Named { name, type_args } => {
                write!(f, "{}", name)?;
                if !type_args.is_empty() {
                    write!(f, "<")?;
                    for (i, arg) in type_args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", arg)?;
                    }
                    write!(f, ">")?;
                }
                Ok(())
            }

            // Effect row variable
            Type::EffectVar(var) => write!(f, "!e{}", var.0),

            // Type variable
            Type::Var(var) => write!(f, "?{}", var.0),

            // Error type
            Type::Error => write!(f, "<error>"),

            // Any type
            Type::Any => write!(f, "Any"),
        }
    }
}

impl Type {
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            Type::Int
                | Type::Int8
                | Type::Int16
                | Type::Int32
                | Type::Int64
                | Type::Int128
                | Type::UInt
                | Type::UInt8
                | Type::UInt16
                | Type::UInt32
                | Type::UInt64
                | Type::UInt128
                | Type::Float
                | Type::Float32
                | Type::Float64
        )
    }

    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            Type::Int
                | Type::Int8
                | Type::Int16
                | Type::Int32
                | Type::Int64
                | Type::Int128
                | Type::UInt
                | Type::UInt8
                | Type::UInt16
                | Type::UInt32
                | Type::UInt64
                | Type::UInt128
        )
    }

    pub fn is_float(&self) -> bool {
        matches!(self, Type::Float | Type::Float32 | Type::Float64)
    }

    /// Check if this type implements the Transfer trait (can be safely moved between tasks).
    ///
    /// Transfer is Aria's equivalent of Rust's Send trait. Types that are Transfer can be
    /// safely transferred to another task/thread without data races.
    ///
    /// # Transfer types
    ///
    /// - All primitive types (Int, Float, Bool, Char, String, etc.)
    /// - Arrays and tuples of Transfer types
    /// - Optional and Result of Transfer types
    /// - Functions (closure captures must be checked separately)
    /// - Named types (assumed Transfer unless marked otherwise)
    ///
    /// # Non-Transfer types
    ///
    /// - Mutable references (could cause data races)
    /// - Type variables (unknown, conservatively non-Transfer)
    /// - Error types
    pub fn is_transfer(&self) -> bool {
        match self {
            // All primitives are Transfer
            Type::Int
            | Type::Int8
            | Type::Int16
            | Type::Int32
            | Type::Int64
            | Type::Int128
            | Type::UInt
            | Type::UInt8
            | Type::UInt16
            | Type::UInt32
            | Type::UInt64
            | Type::UInt128
            | Type::Float
            | Type::Float32
            | Type::Float64
            | Type::Bool
            | Type::Char
            | Type::String
            | Type::Bytes
            | Type::Unit
            | Type::Never => true,

            // Compound types are Transfer if their components are Transfer
            Type::Array(inner) => inner.is_transfer(),
            Type::FixedArray(inner, _) => inner.is_transfer(),
            Type::Map(key, value) => key.is_transfer() && value.is_transfer(),
            Type::Tuple(types) => types.iter().all(|t| t.is_transfer()),
            Type::Optional(inner) => inner.is_transfer(),
            Type::Result(ok, err) => ok.is_transfer() && err.is_transfer(),

            // Channels are Transfer if their element type is Transfer
            // (a channel handle can be sent to another task)
            Type::Channel(elem) => elem.is_transfer(),

            // Task handles are Transfer if their result type is Transfer
            Type::Task(result) => result.is_transfer(),

            // Mutable references are NOT Transfer (could cause data races)
            Type::Reference { mutable, inner } => {
                if *mutable {
                    false
                } else {
                    // Immutable references to Sharable types are Transfer
                    inner.is_sharable()
                }
            }

            // Functions are Transfer (but captures need separate checking)
            Type::Function { .. } => true,

            // Effectful functions are Transfer
            Type::EffectfulFunction { .. } => true,

            // Named types are assumed Transfer by default
            // TODO: Check against a registry of non-Transfer types
            Type::Named { .. } => true,

            // Effect row variables are Transfer (they represent effect rows, not values)
            Type::EffectVar(_) => true,

            // Type variables are unknown, conservatively non-Transfer
            Type::Var(_) => false,

            // Error types are not Transfer
            Type::Error => false,

            // Any type is considered Transfer (used for polymorphic builtins)
            Type::Any => true,
        }
    }

    /// Check if this type implements the Sharable trait (can be safely shared between tasks).
    ///
    /// Sharable is Aria's equivalent of Rust's Sync trait. Types that are Sharable can be
    /// safely accessed from multiple tasks through immutable references.
    ///
    /// # Sharable types
    ///
    /// - All primitive types (Int, Float, Bool, Char, String, etc.)
    /// - Arrays and tuples of Sharable types (with immutable access)
    /// - Immutable references to Sharable types
    ///
    /// # Non-Sharable types
    ///
    /// - Mutable references (concurrent mutation could cause data races)
    /// - Type variables (unknown, conservatively non-Sharable)
    /// - Error types
    pub fn is_sharable(&self) -> bool {
        match self {
            // All primitives are Sharable
            Type::Int
            | Type::Int8
            | Type::Int16
            | Type::Int32
            | Type::Int64
            | Type::Int128
            | Type::UInt
            | Type::UInt8
            | Type::UInt16
            | Type::UInt32
            | Type::UInt64
            | Type::UInt128
            | Type::Float
            | Type::Float32
            | Type::Float64
            | Type::Bool
            | Type::Char
            | Type::String
            | Type::Bytes
            | Type::Unit
            | Type::Never => true,

            // Compound types are Sharable if their components are Sharable
            Type::Array(inner) => inner.is_sharable(),
            Type::FixedArray(inner, _) => inner.is_sharable(),
            Type::Map(key, value) => key.is_sharable() && value.is_sharable(),
            Type::Tuple(types) => types.iter().all(|t| t.is_sharable()),
            Type::Optional(inner) => inner.is_sharable(),
            Type::Result(ok, err) => ok.is_sharable() && err.is_sharable(),

            // Channels are always Sharable - they are designed for sharing between tasks
            // (multiple tasks can hold references to the same channel)
            Type::Channel(_) => true,

            // Task handles are always Sharable - they can be awaited from any task
            Type::Task(_) => true,

            // Mutable references are NOT Sharable
            // Immutable references to Sharable types are Sharable
            Type::Reference { mutable, inner } => !mutable && inner.is_sharable(),

            // Functions are Sharable (captures need separate checking)
            Type::Function { .. } => true,

            // Effectful functions are Sharable
            Type::EffectfulFunction { .. } => true,

            // Named types are assumed Sharable by default
            // TODO: Check against a registry of non-Sharable types
            Type::Named { .. } => true,

            // Effect row variables are Sharable
            Type::EffectVar(_) => true,

            // Type variables are unknown, conservatively non-Sharable
            Type::Var(_) => false,

            // Error types are not Sharable
            Type::Error => false,

            // Any type is considered Sharable (used for polymorphic builtins)
            Type::Any => true,
        }
    }

    /// Check if this type can be safely captured by a spawned task.
    ///
    /// For a type to be capturable by spawn, it must be Transfer.
    /// This is the primary check used during spawn type checking.
    pub fn is_spawn_safe(&self) -> bool {
        self.is_transfer()
    }

    /// Check if this type implements the Copy trait (can be implicitly copied).
    ///
    /// Copy types can be duplicated by simple bitwise copying without running
    /// any user-defined code. When a Copy type is used, it is copied rather than moved.
    ///
    /// # Copy types
    ///
    /// - All numeric primitives (Int, Float, etc.)
    /// - Bool, Char, Unit, Never
    /// - References (copying the reference, not what it points to)
    /// - Tuples of Copy types
    /// - Fixed-size arrays of Copy types
    /// - Optional of Copy types
    /// - Result of Copy types (both Ok and Err must be Copy)
    ///
    /// # Non-Copy types (owned, require move semantics)
    ///
    /// - String (owns heap-allocated data)
    /// - Bytes (owns heap-allocated data)
    /// - Array (dynamic arrays own heap memory)
    /// - Map (owns heap-allocated data)
    /// - Channel (unique handle to communication endpoint)
    /// - Functions/closures (may capture owned state)
    /// - Named types (not Copy by default, require explicit @copy attribute)
    /// - Type variables (unknown, conservatively non-Copy)
    pub fn is_copy(&self) -> bool {
        match self {
            // All numeric primitives are Copy
            Type::Int
            | Type::Int8
            | Type::Int16
            | Type::Int32
            | Type::Int64
            | Type::Int128
            | Type::UInt
            | Type::UInt8
            | Type::UInt16
            | Type::UInt32
            | Type::UInt64
            | Type::UInt128
            | Type::Float
            | Type::Float32
            | Type::Float64
            | Type::Bool
            | Type::Char
            | Type::Unit
            | Type::Never => true,

            // String and Bytes own heap data - NOT Copy
            Type::String | Type::Bytes => false,

            // Dynamic arrays own heap memory - NOT Copy
            Type::Array(_) => false,

            // Fixed-size arrays are Copy if element type is Copy
            Type::FixedArray(inner, _) => inner.is_copy(),

            // Maps own heap memory - NOT Copy
            Type::Map(_, _) => false,

            // Tuples are Copy if all elements are Copy
            Type::Tuple(types) => types.iter().all(|t| t.is_copy()),

            // Optional is Copy if inner type is Copy
            Type::Optional(inner) => inner.is_copy(),

            // Result is Copy if both Ok and Err types are Copy
            Type::Result(ok, err) => ok.is_copy() && err.is_copy(),

            // References are Copy (we copy the reference itself, not the data)
            // This matches Rust's behavior where &T and &mut T are Copy
            Type::Reference { .. } => true,

            // Functions/closures may capture owned state - NOT Copy
            Type::Function { .. } => false,

            // Channels are unique handles - NOT Copy
            Type::Channel(_) => false,

            // Task handles are NOT Copy (they represent ownership of a computation)
            Type::Task(_) => false,

            // Effectful functions may capture owned state - NOT Copy
            Type::EffectfulFunction { .. } => false,

            // Named types are NOT Copy by default
            // Types can opt-in to Copy with @copy attribute
            // TODO: Check against a registry of Copy types
            Type::Named { .. } => false,

            // Effect row variables are not Copy (they're type-level constructs)
            Type::EffectVar(_) => false,

            // Type variables are unknown, conservatively non-Copy
            Type::Var(_) => false,

            // Error types are not Copy
            Type::Error => false,

            // Any type is NOT Copy (be conservative for polymorphic contexts)
            Type::Any => false,
        }
    }
}

/// Type error
#[derive(Debug, Clone, Error)]
pub enum TypeError {
    #[error("Type mismatch: expected `{expected}`, found `{found}`")]
    Mismatch {
        expected: String,
        found: String,
        span: Span,
        /// Optional context about where the expected type came from
        expected_source: Option<TypeSource>,
    },

    #[error("Undefined type: `{0}`")]
    UndefinedType(String, Span),

    #[error("Undefined variable: `{name}`")]
    UndefinedVariable {
        name: String,
        span: Span,
        /// Optional list of similar names for typo suggestions
        similar_names: Option<Vec<String>>,
    },

    #[error("Cannot infer type")]
    CannotInfer(Span),

    #[error("Recursive type detected")]
    RecursiveType(Span),

    #[error("Wrong number of type arguments: expected {expected}, found {found}")]
    WrongTypeArity {
        expected: usize,
        found: usize,
        span: Span,
    },

    #[error("Cannot spawn task capturing non-Transfer value `{var_name}` of type `{var_type}`")]
    NonTransferCapture {
        var_name: String,
        var_type: String,
        span: Span,
    },

    #[error("Cannot share non-Sharable value `{var_name}` of type `{var_type}` between tasks")]
    NonSharableShare {
        var_name: String,
        var_type: String,
        span: Span,
    },

    #[error("Cannot mutably capture immutable variable `{var_name}`")]
    MutableCaptureOfImmutable {
        var_name: String,
        span: Span,
    },

    #[error("Cannot mutably capture `{var_name}` in spawn - spawned closures cannot hold mutable borrows")]
    MutableCaptureInSpawn {
        var_name: String,
        span: Span,
    },

    #[error("Type `{ty}` does not implement trait `{trait_name}`")]
    MissingTraitImpl {
        ty: String,
        trait_name: String,
        span: Span,
    },

    #[error("The `?` operator can only be used on Result or Optional types, found `{found}`")]
    InvalidTryOperator {
        found: String,
        span: Span,
    },

    #[error("Cannot use `?` in a function that doesn't return Result or Optional")]
    TryInNonResultFunction {
        function_return: String,
        span: Span,
    },

    #[error("`await` can only be used in async context")]
    AwaitOutsideAsync {
        span: Span,
    },

    #[error("`await` expects a Task type, found `{found}`")]
    AwaitNonTask {
        found: String,
        span: Span,
    },

    #[error("Channel send expects a channel, found `{found}`")]
    SendOnNonChannel {
        found: String,
        span: Span,
    },

    #[error("Channel receive expects a channel, found `{found}`")]
    ReceiveOnNonChannel {
        found: String,
        span: Span,
    },

    #[error("Select expression cannot have multiple default arms")]
    MultipleDefaultArms {
        first_span: Span,
        second_span: Span,
    },

    #[error("Select arm result type mismatch: expected `{expected}`, found `{found}`")]
    SelectArmTypeMismatch {
        expected: String,
        found: String,
        arm_index: usize,
        span: Span,
    },

    // ============================================================================
    // Effect System Errors
    // ============================================================================

    #[error("Effect `{effect}` not declared in function signature")]
    UndeclaredEffect {
        effect: String,
        function_name: String,
        span: Span,
    },

    #[error("Cannot perform effect `{effect}` without a handler in scope")]
    UnhandledEffect {
        effect: String,
        span: Span,
    },

    #[error("Effect handler for `{effect}` has wrong type: expected `{expected}`, found `{found}`")]
    EffectHandlerTypeMismatch {
        effect: String,
        expected: String,
        found: String,
        span: Span,
    },

    #[error("Effect row mismatch: expected `{expected}`, found `{found}`")]
    EffectRowMismatch {
        expected: String,
        found: String,
        span: Span,
    },

    #[error("Cannot call function with effects `{callee_effects}` from pure context")]
    EffectfulCallInPureContext {
        callee_effects: String,
        span: Span,
    },

    #[error("Effect `{effect}` is not defined")]
    UndefinedEffect {
        effect: String,
        span: Span,
    },

    #[error("Duplicate effect declaration: `{effect}`")]
    DuplicateEffectDeclaration {
        effect: String,
        span: Span,
    },

    #[error("Resume outside of effect handler")]
    ResumeOutsideHandler {
        span: Span,
    },

    #[error("Resume type mismatch: expected `{expected}`, found `{found}`")]
    ResumeTypeMismatch {
        expected: String,
        found: String,
        span: Span,
    },

    #[error("Module not found: {0}")]
    ModuleNotFound(String, Span),

    #[error("Symbol `{symbol}` is not exported from module `{module}`")]
    ImportNotExported {
        symbol: String,
        module: String,
        span: Span,
    },

    #[error("Unresolved import: `{symbol}` from module `{module}`")]
    UnresolvedImport {
        symbol: String,
        module: String,
        span: Span,
    },

    #[error("Non-exhaustive patterns: {missing}")]
    NonExhaustivePatterns {
        missing: String,
        span: Span,
    },

    #[error("Unreachable pattern")]
    UnreachablePattern {
        span: Span,
    },

    #[error("Type `{type_name}` has no field `{field_name}`")]
    UndefinedField {
        type_name: String,
        field_name: String,
        span: Span,
    },

    #[error("Cannot access field on non-struct type `{type_name}`")]
    FieldAccessOnNonStruct {
        type_name: String,
        span: Span,
    },

    #[error("Return type mismatch: expected `{expected}`, found `{found}`")]
    ReturnTypeMismatch {
        expected: String,
        found: String,
        span: Span,
    },

    // ============================================================================
    // FFI Type Errors
    // ============================================================================

    #[error("Invalid C type in FFI declaration: `{c_type}` - {reason}")]
    InvalidFfiCType {
        c_type: String,
        reason: String,
        span: Span,
    },

    #[error("Extern function `{func_name}` parameter `{param_name}` uses non-FFI-safe type")]
    NonFfiSafeParameter {
        func_name: String,
        param_name: String,
        span: Span,
    },

    #[error("Extern function `{func_name}` returns non-FFI-safe type")]
    NonFfiSafeReturn {
        func_name: String,
        span: Span,
    },

    #[error("Extern struct `{struct_name}` field `{field_name}` uses non-FFI-safe type")]
    NonFfiSafeField {
        struct_name: String,
        field_name: String,
        span: Span,
    },

    #[error("Duplicate extern declaration: `{name}` was already declared")]
    DuplicateExternDeclaration {
        name: String,
        span: Span,
    },

    #[error("Missing type annotation on extern function parameter")]
    MissingExternParamType {
        func_name: String,
        span: Span,
    },

    #[error("Variadic functions are not supported in FFI: `{func_name}`")]
    VariadicFfiFunction {
        func_name: String,
        span: Span,
    },

    // ============================================================================
    // Tuple Type Errors
    // ============================================================================

    #[error("Tuple index {index} out of bounds for tuple of length {length}")]
    TupleIndexOutOfBounds {
        index: usize,
        length: usize,
        span: Span,
    },

    #[error("Cannot convert tuple to array: elements have different types ({types})")]
    TupleToArrayHeterogeneousTypes {
        types: String,
        span: Span,
    },

    // ============================================================================
    // Generic Type Bound Errors
    // ============================================================================

    #[error("Type `{ty}` does not implement trait `{trait_name}`")]
    TraitNotImplemented {
        ty: String,
        trait_name: String,
        span: Span,
    },

    #[error("Type argument `{type_arg}` does not satisfy bound `{bound}` for type parameter `{param}`")]
    BoundNotSatisfied {
        type_arg: String,
        param: String,
        bound: String,
        span: Span,
    },

    #[error("Undefined trait: `{0}`")]
    UndefinedTrait(String, Span),

    #[error("Trait `{trait_name}` expects {expected} type argument(s), found {found}")]
    WrongTraitArity {
        trait_name: String,
        expected: usize,
        found: usize,
        span: Span,
    },

    #[error("Conflicting implementations of trait `{trait_name}` for type `{for_type}`")]
    ConflictingImpl {
        trait_name: String,
        for_type: String,
        span: Span,
    },

    #[error("Where clause constraint `{constraint}` not satisfied")]
    WhereClauseNotSatisfied {
        constraint: String,
        span: Span,
    },

    // ============================================================================
    // Trait Implementation Errors
    // ============================================================================

    #[error("Missing method `{method_name}` in implementation of trait `{trait_name}`")]
    MissingTraitMethod {
        trait_name: String,
        method_name: String,
        span: Span,
    },

    #[error("Method `{method_name}` has wrong signature: expected `{expected}`, found `{found}`")]
    TraitMethodSignatureMismatch {
        method_name: String,
        expected: String,
        found: String,
        span: Span,
    },

    #[error("Missing associated type `{type_name}` in implementation of trait `{trait_name}`")]
    MissingAssociatedType {
        trait_name: String,
        type_name: String,
        span: Span,
    },

    #[error("Method `{method_name}` is not a member of trait `{trait_name}`")]
    MethodNotInTrait {
        trait_name: String,
        method_name: String,
        span: Span,
    },

    #[error("Associated type `{type_name}` is not defined in trait `{trait_name}`")]
    AssociatedTypeNotInTrait {
        trait_name: String,
        type_name: String,
        span: Span,
    },

    #[error("Trait `{trait_name}` requires implementing supertrait `{supertrait_name}`")]
    SupertraitNotImplemented {
        trait_name: String,
        supertrait_name: String,
        for_type: String,
        span: Span,
    },

    #[error("Self type mismatch in impl block: method `{method_name}` expects Self to be `{expected}`, found `{found}`")]
    SelfTypeMismatch {
        method_name: String,
        expected: String,
        found: String,
        span: Span,
    },

    #[error("Duplicate method `{method_name}` in impl block")]
    DuplicateImplMethod {
        method_name: String,
        span: Span,
    },

    #[error("Duplicate associated type `{type_name}` in impl block")]
    DuplicateAssociatedType {
        type_name: String,
        span: Span,
    },

    // ============================================================================
    // Const Expression Evaluation Errors
    // ============================================================================

    #[error("Expression is not a compile-time constant")]
    NotConstant {
        reason: String,
        span: Span,
    },

    #[error("Const evaluation error: {reason}")]
    ConstEvalError {
        reason: String,
        span: Span,
    },

    #[error("Integer overflow in const evaluation")]
    ConstOverflow {
        span: Span,
    },

    #[error("Division by zero in const evaluation")]
    ConstDivisionByZero {
        span: Span,
    },

    #[error("Undefined constant: {0}")]
    UndefinedConstant(String, Span),

    // ============================================================================
    // Defer Statement Errors
    // ============================================================================

    #[error("Deferred expression should return Unit, found `{found}`")]
    DeferNonUnit {
        found: String,
        span: Span,
    },

    #[error("Control flow statement `{statement}` cannot be used inside defer")]
    ControlFlowInDefer {
        statement: String,
        span: Span,
    },

    #[error("Variable `{var_name}` may not be valid when defer executes")]
    DeferCaptureInvalid {
        var_name: String,
        defer_span: Span,
        var_span: Span,
    },

    #[error("Cannot use `await` inside defer block")]
    AwaitInDefer {
        span: Span,
    },

    // ============================================================================
    // Default Parameter Errors
    // ============================================================================

    #[error("Default value type mismatch: parameter `{param_name}` has type `{param_type}`, but default value has type `{default_type}`")]
    DefaultValueTypeMismatch {
        param_name: String,
        param_type: String,
        default_type: String,
        span: Span,
    },

    #[error("Parameters with default values must come after required parameters: `{param_name}`")]
    DefaultAfterRequired {
        param_name: String,
        span: Span,
    },

    #[error("Too few arguments: expected at least {min_required} arguments, found {found}")]
    TooFewArguments {
        min_required: usize,
        found: usize,
        span: Span,
    },

    #[error("Too many arguments: expected at most {max_allowed} arguments, found {found}")]
    TooManyArguments {
        max_allowed: usize,
        found: usize,
        span: Span,
    },

    #[error("Named argument `{name}` does not match any parameter")]
    UnknownNamedArgument {
        name: String,
        span: Span,
    },

    #[error("Duplicate named argument: `{name}`")]
    DuplicateNamedArgument {
        name: String,
        span: Span,
    },

    #[error("Missing required argument: `{name}`")]
    MissingRequiredArgument {
        name: String,
        span: Span,
    },

    #[error("Positional arguments cannot follow named arguments")]
    PositionalAfterNamed {
        span: Span,
    },

    // ============================================================================
    // Spread Operator Errors
    // ============================================================================

    #[error("Spread operator requires an array type, found `{found}`")]
    SpreadOnNonArray {
        found: String,
        span: Span,
    },

    #[error("Spread argument element type `{spread_elem_type}` is not compatible with parameter type `{param_type}`")]
    SpreadElementTypeMismatch {
        spread_elem_type: String,
        param_type: String,
        span: Span,
    },

    #[error("Spread in array literal: element type `{spread_elem_type}` is not compatible with array element type `{array_elem_type}`")]
    SpreadArrayElementMismatch {
        spread_elem_type: String,
        array_elem_type: String,
        span: Span,
    },

    #[error("Spread in struct: source type `{source_type}` is not compatible with target struct `{target_struct}`")]
    SpreadStructTypeMismatch {
        source_type: String,
        target_struct: String,
        span: Span,
    },

    #[error("Cannot spread non-struct type `{found}` in struct initializer")]
    SpreadOnNonStruct {
        found: String,
        span: Span,
    },

    // ============================================================================
    // Loop Control Flow Errors
    // ============================================================================

    #[error("`break` cannot be used outside of a loop")]
    BreakOutsideLoop {
        span: Span,
    },

    #[error("`continue` cannot be used outside of a loop")]
    ContinueOutsideLoop {
        span: Span,
    },

    #[error("Break value type mismatch: expected `{expected}`, found `{found}`")]
    BreakTypeMismatch {
        expected: String,
        found: String,
        span: Span,
    },

    #[error("Type `{found}` is not iterable")]
    NotIterable {
        found: String,
        span: Span,
    },
}

/// Type result
pub type TypeResult<T> = Result<T, TypeError>;

/// Type environment (scope)
#[derive(Debug, Clone, Default)]
pub struct TypeEnv {
    /// Variable bindings: name -> type
    variables: FxHashMap<String, Type>,
    /// Type definitions: name -> type scheme
    types: FxHashMap<String, TypeScheme>,
    /// Parent scope
    parent: Option<Rc<TypeEnv>>,
}

impl TypeEnv {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_parent(parent: Rc<TypeEnv>) -> Self {
        Self {
            parent: Some(parent),
            ..Default::default()
        }
    }

    pub fn define_var(&mut self, name: String, ty: Type) {
        self.variables.insert(name, ty);
    }

    pub fn define_type(&mut self, name: String, scheme: TypeScheme) {
        self.types.insert(name, scheme);
    }

    pub fn lookup_var(&self, name: &str) -> Option<&Type> {
        self.variables
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|p| p.lookup_var(name)))
    }

    pub fn lookup_type(&self, name: &str) -> Option<&TypeScheme> {
        self.types
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|p| p.lookup_type(name)))
    }
}

/// A trait bound specification
/// Represents a constraint like `T: Display` or `T: Iterator<Item = U>`
#[derive(Debug, Clone, PartialEq)]
pub struct TypeBound {
    /// The trait name (possibly qualified path like "std::fmt::Display")
    pub trait_name: String,
    /// Type arguments for the trait (e.g., `Item = U` in `Iterator<Item = U>`)
    pub type_args: Vec<Type>,
}

impl TypeBound {
    pub fn new(trait_name: String) -> Self {
        Self {
            trait_name,
            type_args: Vec::new(),
        }
    }

    pub fn with_args(trait_name: String, type_args: Vec<Type>) -> Self {
        Self { trait_name, type_args }
    }
}

impl std::fmt::Display for TypeBound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.trait_name)?;
        if !self.type_args.is_empty() {
            write!(f, "<")?;
            for (i, arg) in self.type_args.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", arg)?;
            }
            write!(f, ">")?;
        }
        Ok(())
    }
}

/// Type parameter with optional bounds
#[derive(Debug, Clone)]
pub struct TypeParamDef {
    /// The type parameter name (e.g., "T")
    pub name: String,
    /// Bounds on this type parameter (e.g., "Display + Clone")
    pub bounds: Vec<TypeBound>,
}

// ============================================================================
// Function Signature with Default Parameters
// ============================================================================

/// Information about a function parameter, including default value info
#[derive(Debug, Clone)]
pub struct ParamInfo {
    /// Parameter name
    pub name: String,
    /// Parameter type
    pub ty: Type,
    /// Whether this parameter has a default value
    pub has_default: bool,
}

/// Extended function signature that tracks parameter names and defaults
///
/// This is used internally by the type checker to properly validate function
/// calls with default and named arguments. The standard `Type::Function` only
/// stores parameter types, not names or default info.
#[derive(Debug, Clone)]
pub struct FunctionSignature {
    /// Full parameter information
    pub params: Vec<ParamInfo>,
    /// Return type
    pub return_type: Type,
    /// Number of required parameters (those without defaults)
    pub required_count: usize,
}

impl FunctionSignature {
    /// Create a new function signature from parameter information
    pub fn new(params: Vec<ParamInfo>, return_type: Type) -> Self {
        let required_count = params.iter().take_while(|p| !p.has_default).count();
        Self {
            params,
            return_type,
            required_count,
        }
    }

    /// Create a simple signature from just types (no defaults, no names)
    pub fn from_types(param_types: Vec<Type>, return_type: Type) -> Self {
        let params: Vec<ParamInfo> = param_types
            .into_iter()
            .enumerate()
            .map(|(i, ty)| ParamInfo {
                name: format!("arg{}", i),
                ty,
                has_default: false,
            })
            .collect();
        Self {
            required_count: params.len(),
            params,
            return_type,
        }
    }

    /// Convert to standard function type (loses default/name info)
    pub fn to_function_type(&self) -> Type {
        Type::Function {
            params: self.params.iter().map(|p| p.ty.clone()).collect(),
            return_type: Box::new(self.return_type.clone()),
        }
    }

    /// Find a parameter by name
    pub fn find_param(&self, name: &str) -> Option<(usize, &ParamInfo)> {
        self.params.iter().enumerate().find(|(_, p)| p.name == name)
    }

    /// Check if all required parameters are satisfied by the given arguments
    pub fn min_args(&self) -> usize {
        self.required_count
    }

    /// Maximum number of arguments
    pub fn max_args(&self) -> usize {
        self.params.len()
    }
}

/// Extended function signature for generic functions
///
/// This stores the type parameter definitions along with the function signature,
/// enabling type argument inference at call sites.
#[derive(Debug, Clone)]
pub struct GenericFunctionInfo {
    /// Type parameter definitions (e.g., `T: Ord`, `U`)
    pub type_params: Vec<TypeParamDef>,
    /// Type parameter names in order
    pub type_param_names: Vec<String>,
    /// Parameter types (may contain Type::Var referencing type parameters)
    pub param_types: Vec<Type>,
    /// Return type (may contain Type::Var referencing type parameters)
    pub return_type: Type,
    /// Mapping from type parameter name to TypeVar ID
    pub type_param_vars: FxHashMap<String, TypeVar>,
}

impl GenericFunctionInfo {
    /// Create a new generic function info
    pub fn new(
        type_params: Vec<TypeParamDef>,
        param_types: Vec<Type>,
        return_type: Type,
        type_param_vars: Vec<(String, TypeVar)>,
    ) -> Self {
        let type_param_names = type_params.iter().map(|p| p.name.clone()).collect();
        let var_map = type_param_vars.into_iter().collect();
        Self {
            type_params,
            type_param_names,
            param_types,
            return_type,
            type_param_vars: var_map,
        }
    }

    /// Get the number of type parameters
    pub fn type_param_count(&self) -> usize {
        self.type_params.len()
    }

    /// Check if this function is generic
    pub fn is_generic(&self) -> bool {
        !self.type_params.is_empty()
    }
}

impl TypeParamDef {
    pub fn new(name: String) -> Self {
        Self { name, bounds: Vec::new() }
    }

    pub fn with_bounds(name: String, bounds: Vec<TypeBound>) -> Self {
        Self { name, bounds }
    }
}

/// Trait definition - stores the structure of a trait
#[derive(Debug, Clone)]
pub struct TraitDef {
    /// Trait name
    pub name: String,
    /// Type parameters for the trait itself
    pub type_params: Vec<TypeParamDef>,
    /// Required methods (name -> function type)
    pub methods: FxHashMap<String, Type>,
    /// Methods that have default implementations (don't need to be provided)
    pub default_methods: FxHashMap<String, Type>,
    /// Associated types
    pub associated_types: Vec<String>,
    /// Associated types that have default values
    pub default_associated_types: FxHashMap<String, Type>,
    /// Associated constants (name -> type)
    pub associated_consts: FxHashMap<String, Type>,
    /// Super traits that this trait extends
    pub supertraits: Vec<TypeBound>,
}

impl TraitDef {
    pub fn new(name: String) -> Self {
        Self {
            name,
            type_params: Vec::new(),
            methods: FxHashMap::default(),
            default_methods: FxHashMap::default(),
            associated_types: Vec::new(),
            default_associated_types: FxHashMap::default(),
            associated_consts: FxHashMap::default(),
            supertraits: Vec::new(),
        }
    }

    /// Add a required method to the trait
    pub fn add_method(&mut self, name: String, ty: Type) {
        self.methods.insert(name, ty);
    }

    /// Add a method with a default implementation
    pub fn add_default_method(&mut self, name: String, ty: Type) {
        self.default_methods.insert(name, ty);
    }

    /// Add an associated type requirement
    pub fn add_associated_type(&mut self, name: String) {
        self.associated_types.push(name);
    }

    /// Add an associated type with a default
    pub fn add_default_associated_type(&mut self, name: String, ty: Type) {
        self.default_associated_types.insert(name, ty);
    }

    /// Get all required methods (those without defaults)
    pub fn required_methods(&self) -> impl Iterator<Item = (&String, &Type)> {
        self.methods.iter()
    }

    /// Get all required associated types (those without defaults)
    pub fn required_associated_types(&self) -> impl Iterator<Item = &String> {
        self.associated_types.iter()
            .filter(|name| !self.default_associated_types.contains_key(*name))
    }

    /// Check if a method exists in this trait (either required or default)
    pub fn has_method(&self, name: &str) -> bool {
        self.methods.contains_key(name) || self.default_methods.contains_key(name)
    }

    /// Get a method's type (either required or default)
    pub fn get_method(&self, name: &str) -> Option<&Type> {
        self.methods.get(name).or_else(|| self.default_methods.get(name))
    }

    /// Check if an associated type is defined in this trait
    pub fn has_associated_type(&self, name: &str) -> bool {
        self.associated_types.contains(&name.to_string()) ||
        self.default_associated_types.contains_key(name)
    }
}

/// Record of a trait implementation
#[derive(Debug, Clone)]
pub struct TraitImpl {
    /// The trait being implemented
    pub trait_name: String,
    /// Type arguments for the trait
    pub trait_args: Vec<Type>,
    /// The type implementing the trait
    pub for_type: Type,
    /// Where clause constraints
    pub where_clause: Vec<(String, Vec<TypeBound>)>,
    /// Implemented methods (method_name -> function type)
    pub methods: FxHashMap<String, Type>,
    /// Defined associated types (type_name -> concrete type)
    pub associated_types: FxHashMap<String, Type>,
    /// Defined associated constants (const_name -> type)
    pub associated_consts: FxHashMap<String, Type>,
}

impl TraitImpl {
    /// Create a new trait implementation record
    pub fn new(trait_name: String, for_type: Type) -> Self {
        Self {
            trait_name,
            trait_args: Vec::new(),
            for_type,
            where_clause: Vec::new(),
            methods: FxHashMap::default(),
            associated_types: FxHashMap::default(),
            associated_consts: FxHashMap::default(),
        }
    }

    /// Add an implemented method
    pub fn add_method(&mut self, name: String, ty: Type) {
        self.methods.insert(name, ty);
    }

    /// Add an associated type definition
    pub fn add_associated_type(&mut self, name: String, ty: Type) {
        self.associated_types.insert(name, ty);
    }
}

/// Type scheme (for polymorphic types)
#[derive(Debug, Clone)]
pub struct TypeScheme {
    /// Type parameters (for backwards compatibility)
    pub type_params: Vec<String>,
    /// Type parameter definitions with bounds
    pub type_param_defs: Vec<TypeParamDef>,
    /// The underlying type
    pub ty: Type,
}

impl TypeScheme {
    pub fn mono(ty: Type) -> Self {
        Self {
            type_params: Vec::new(),
            type_param_defs: Vec::new(),
            ty,
        }
    }

    pub fn poly(type_params: Vec<String>, ty: Type) -> Self {
        // Convert to TypeParamDefs without bounds for backwards compatibility
        let type_param_defs = type_params.iter()
            .map(|name| TypeParamDef::new(name.clone()))
            .collect();
        Self { type_params, type_param_defs, ty }
    }

    /// Create a polymorphic type scheme with bounded type parameters
    pub fn poly_bounded(type_param_defs: Vec<TypeParamDef>, ty: Type) -> Self {
        let type_params = type_param_defs.iter().map(|p| p.name.clone()).collect();
        Self { type_params, type_param_defs, ty }
    }

    /// Get bounds for a type parameter by name
    pub fn get_bounds(&self, param_name: &str) -> Option<&[TypeBound]> {
        self.type_param_defs.iter()
            .find(|p| p.name == param_name)
            .map(|p| p.bounds.as_slice())
    }

    /// Check if this is a monomorphic type scheme (no type parameters)
    pub fn is_mono(&self) -> bool {
        self.type_params.is_empty()
    }

    /// Check if this is a polymorphic type scheme
    pub fn is_poly(&self) -> bool {
        !self.type_params.is_empty()
    }

    /// Get the arity (number of type parameters)
    pub fn arity(&self) -> usize {
        self.type_params.len()
    }
}

/// Type inference state
#[derive(Debug)]
pub struct TypeInference {
    /// Next type variable ID
    next_var: u32,
    /// Substitution map: TypeVar -> Type
    substitution: FxHashMap<TypeVar, Type>,
    /// Collected errors
    errors: Vec<TypeError>,
}

impl TypeInference {
    pub fn new() -> Self {
        Self {
            next_var: 0,
            substitution: FxHashMap::default(),
            errors: Vec::new(),
        }
    }

    /// Create a fresh type variable
    pub fn fresh_var(&mut self) -> Type {
        let var = TypeVar(self.next_var);
        self.next_var += 1;
        Type::Var(var)
    }

    /// Check if a type contains any type variables that need substitution.
    /// This allows us to skip expensive apply() calls when unnecessary.
    #[inline]
    fn needs_apply(&self, ty: &Type) -> bool {
        match ty {
            Type::Var(var) => self.substitution.contains_key(var),
            Type::Array(elem)
            | Type::FixedArray(elem, _)
            | Type::Optional(elem)
            | Type::Channel(elem)
            | Type::Task(elem) => self.needs_apply(elem),
            Type::Map(k, v) | Type::Result(k, v) => self.needs_apply(k) || self.needs_apply(v),
            Type::Tuple(ts) => ts.iter().any(|t| self.needs_apply(t)),
            Type::Reference { inner, .. } => self.needs_apply(inner),
            Type::Function { params, return_type } => {
                params.iter().any(|t| self.needs_apply(t)) || self.needs_apply(return_type)
            }
            Type::Named { type_args, .. } => type_args.iter().any(|t| self.needs_apply(t)),
            // Primitives and other types don't need substitution
            _ => false,
        }
    }

    /// Unify two types
    pub fn unify(&mut self, t1: &Type, t2: &Type, span: Span) -> TypeResult<()> {
        let t1 = self.apply(t1);
        let t2 = self.apply(t2);

        match (&t1, &t2) {
            // Same type
            _ if t1 == t2 => Ok(()),

            // Type variable on left
            (Type::Var(var), _) => {
                if self.occurs_check(*var, &t2) {
                    Err(TypeError::RecursiveType(span))
                } else {
                    self.substitution.insert(*var, t2.clone());
                    Ok(())
                }
            }

            // Type variable on right
            (_, Type::Var(var)) => {
                if self.occurs_check(*var, &t1) {
                    Err(TypeError::RecursiveType(span))
                } else {
                    self.substitution.insert(*var, t1.clone());
                    Ok(())
                }
            }

            // Array types
            (Type::Array(e1), Type::Array(e2)) => self.unify(e1, e2, span),

            // Optional types
            (Type::Optional(i1), Type::Optional(i2)) => self.unify(i1, i2, span),

            // Tuple types
            (Type::Tuple(ts1), Type::Tuple(ts2)) if ts1.len() == ts2.len() => {
                for (t1, t2) in ts1.iter().zip(ts2.iter()) {
                    self.unify(t1, t2, span)?;
                }
                Ok(())
            }

            // Function types
            (
                Type::Function {
                    params: p1,
                    return_type: r1,
                },
                Type::Function {
                    params: p2,
                    return_type: r2,
                },
            ) if p1.len() == p2.len() => {
                for (t1, t2) in p1.iter().zip(p2.iter()) {
                    self.unify(t1, t2, span)?;
                }
                self.unify(r1, r2, span)
            }

            // Map types
            (Type::Map(k1, v1), Type::Map(k2, v2)) => {
                self.unify(k1, k2, span)?;
                self.unify(v1, v2, span)
            }

            // Result types
            (Type::Result(ok1, err1), Type::Result(ok2, err2)) => {
                self.unify(ok1, ok2, span)?;
                self.unify(err1, err2, span)
            }

            // Channel types
            (Type::Channel(e1), Type::Channel(e2)) => self.unify(e1, e2, span),

            // Task types
            (Type::Task(r1), Type::Task(r2)) => self.unify(r1, r2, span),

            // Named types (same name, unify args)
            (
                Type::Named {
                    name: n1,
                    type_args: a1,
                },
                Type::Named {
                    name: n2,
                    type_args: a2,
                },
            ) if n1 == n2 && a1.len() == a2.len() => {
                for (t1, t2) in a1.iter().zip(a2.iter()) {
                    self.unify(t1, t2, span)?;
                }
                Ok(())
            }

            // Error types unify with anything
            (Type::Error, _) | (_, Type::Error) => Ok(()),

            // Any type unifies with anything (for polymorphic builtins)
            (Type::Any, _) | (_, Type::Any) => Ok(()),

            // Never type (bottom type) unifies with anything
            // Never is a subtype of all types - it represents computations that don't return
            (Type::Never, _) | (_, Type::Never) => Ok(()),

            // No match
            _ => Err(TypeError::Mismatch {
                expected: format!("{}", t1),
                found: format!("{}", t2),
                span,
                expected_source: None,
            }),
        }
    }

    /// Apply substitution to a type.
    ///
    /// Performance optimization: If the type contains no type variables that need
    /// substitution, we return a clone directly without recursing.
    pub fn apply(&self, ty: &Type) -> Type {
        // Fast path: if no substitution needed, just clone
        if !self.needs_apply(ty) {
            return ty.clone();
        }

        self.apply_inner(ty)
    }

    /// Internal apply implementation that does the actual substitution.
    /// Called only when needs_apply returns true.
    #[inline]
    fn apply_inner(&self, ty: &Type) -> Type {
        match ty {
            Type::Var(var) => self
                .substitution
                .get(var)
                .map(|t| self.apply(t))
                .unwrap_or_else(|| ty.clone()),
            Type::Array(elem) => Type::Array(Box::new(self.apply(elem))),
            Type::FixedArray(elem, size) => Type::FixedArray(Box::new(self.apply(elem)), *size),
            Type::Map(k, v) => Type::Map(Box::new(self.apply(k)), Box::new(self.apply(v))),
            Type::Tuple(ts) => Type::Tuple(ts.iter().map(|t| self.apply(t)).collect()),
            Type::Optional(inner) => Type::Optional(Box::new(self.apply(inner))),
            Type::Result(ok, err) => {
                Type::Result(Box::new(self.apply(ok)), Box::new(self.apply(err)))
            }
            Type::Reference { mutable, inner } => Type::Reference {
                mutable: *mutable,
                inner: Box::new(self.apply(inner)),
            },
            Type::Function {
                params,
                return_type,
            } => Type::Function {
                params: params.iter().map(|t| self.apply(t)).collect(),
                return_type: Box::new(self.apply(return_type)),
            },
            Type::Named { name, type_args } => Type::Named {
                name: name.clone(),
                type_args: type_args.iter().map(|t| self.apply(t)).collect(),
            },
            Type::Channel(elem) => Type::Channel(Box::new(self.apply(elem))),
            Type::Task(result) => Type::Task(Box::new(self.apply(result))),
            _ => ty.clone(),
        }
    }

    /// Check if type variable occurs in type (for recursive type detection)
    fn occurs_check(&self, var: TypeVar, ty: &Type) -> bool {
        match ty {
            Type::Var(v) => {
                if *v == var {
                    true
                } else if let Some(t) = self.substitution.get(v) {
                    self.occurs_check(var, t)
                } else {
                    false
                }
            }
            Type::Array(elem) => self.occurs_check(var, elem),
            Type::FixedArray(elem, _) => self.occurs_check(var, elem),
            Type::Map(k, v) => self.occurs_check(var, k) || self.occurs_check(var, v),
            Type::Tuple(ts) => ts.iter().any(|t| self.occurs_check(var, t)),
            Type::Optional(inner) => self.occurs_check(var, inner),
            Type::Result(ok, err) => self.occurs_check(var, ok) || self.occurs_check(var, err),
            Type::Reference { inner, .. } => self.occurs_check(var, inner),
            Type::Function {
                params,
                return_type,
            } => {
                params.iter().any(|t| self.occurs_check(var, t))
                    || self.occurs_check(var, return_type)
            }
            Type::Named { type_args, .. } => type_args.iter().any(|t| self.occurs_check(var, t)),
            Type::Channel(elem) => self.occurs_check(var, elem),
            Type::Task(result) => self.occurs_check(var, result),
            _ => false,
        }
    }

    /// Get collected errors
    pub fn errors(&self) -> &[TypeError] {
        &self.errors
    }

    /// Add an error
    pub fn add_error(&mut self, error: TypeError) {
        self.errors.push(error);
    }
}

/// Exported symbol from a module
#[derive(Debug, Clone)]
pub struct ModuleExport {
    /// The type of the exported symbol
    pub ty: Type,
    /// Whether this is a type definition (vs. a value)
    pub is_type: bool,
}

/// Module export table - symbols exported by a module
pub type ModuleExports = FxHashMap<String, ModuleExport>;

/// Represents the data associated with an enum variant
#[derive(Debug, Clone)]
pub enum VariantData {
    /// Unit variant: `Color::Red`
    Unit,
    /// Tuple variant: `Option::Some(T)` - stores the field types
    Tuple(Vec<Type>),
    /// Struct variant: `Message::Move { x: Int, y: Int }` - stores (field_name, field_type) pairs
    Struct(Vec<(String, Type)>),
}

/// Information about an enum's variants
#[derive(Debug, Clone)]
pub struct EnumVariantInfo {
    /// The name of the enum (e.g., "Option", "Result")
    pub enum_name: String,
    /// The type parameters of the enum (e.g., ["T"] for Option<T>)
    pub type_params: Vec<TypeParamDef>,
    /// Map from variant name to its data (e.g., "Some" -> Tuple([T]), "None" -> Unit)
    pub variants: FxHashMap<String, VariantData>,
    /// Map from type parameter names to their type variables
    /// Used for substituting type vars when pattern matching against concrete instantiations
    pub type_param_vars: FxHashMap<String, TypeVar>,
}

/// Type checker
pub struct TypeChecker {
    env: Rc<TypeEnv>,
    inference: TypeInference,
    /// Available modules and their exported symbols (module_name -> exports)
    module_exports: FxHashMap<String, ModuleExports>,
    /// Struct field definitions (struct_name -> vec of (field_name, field_type))
    struct_fields: FxHashMap<String, Vec<(String, Type)>>,
    /// Generic type parameter definitions for structs/enums (type_name -> type_param_defs)
    /// Stores bounds information for validation when generic types are instantiated
    generic_type_params: FxHashMap<String, Vec<TypeParamDef>>,
    /// Current function's expected return type (for validating return statements)
    current_return_type: Option<Type>,
    /// Trait definitions (trait_name -> TraitDef)
    trait_defs: FxHashMap<String, TraitDef>,
    /// Trait implementations (trait_name -> list of implementations)
    trait_impls: FxHashMap<String, Vec<TraitImpl>>,
    /// Whether we are currently inside an async context (async function or spawn block)
    /// This is used to validate that `await` is only used in async contexts.
    in_async_context: bool,
    /// Compile-time constant values (name -> ConstValue)
    /// Used for const propagation and compile-time evaluation
    const_values: FxHashMap<String, ConstValue>,
    /// Enum variant information (enum_name -> EnumVariantInfo)
    /// Stores variant names and their associated data types for type checking
    enum_variants: FxHashMap<String, EnumVariantInfo>,
    /// Whether we are currently inside a defer statement
    /// Used to validate that control flow statements (return, break, continue) are not used in defer
    in_defer_context: bool,
    /// Span of the current defer statement (for error reporting)
    current_defer_span: Option<Span>,
    /// Function signatures with default parameter information (func_name -> FunctionSignature)
    function_signatures: FxHashMap<String, FunctionSignature>,
    /// Stack of loop contexts for tracking expected break types
    /// Each entry represents a nested loop with its label (if any) and expected break value type
    loop_context_stack: Vec<LoopContext>,
    /// Stack of type parameter scopes for tracking generic type parameters
    /// Maps type parameter names (e.g., "T", "U") to their type variables
    /// This allows us to resolve type parameter references in function/struct bodies
    type_param_scopes: Vec<FxHashMap<String, TypeVar>>,
    /// Generic function information (func_name -> GenericFunctionInfo)
    /// Stores type parameter info for generic functions to enable type argument inference
    generic_functions: FxHashMap<String, GenericFunctionInfo>,
}

/// Context for a loop, tracking label and expected break value type
#[derive(Debug, Clone)]
pub struct LoopContext {
    /// Optional label for the loop (e.g., 'outer in `'outer: loop {}`)
    pub label: Option<String>,
    /// Expected type for break values (fresh type variable to be unified with break expressions)
    pub break_type: Type,
    /// Span of the loop for error reporting
    pub span: Span,
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut env = TypeEnv::new();

        // Define built-in types
        env.define_type("Int".to_string(), TypeScheme::mono(Type::Int));
        env.define_type("Int8".to_string(), TypeScheme::mono(Type::Int8));
        env.define_type("Int16".to_string(), TypeScheme::mono(Type::Int16));
        env.define_type("Int32".to_string(), TypeScheme::mono(Type::Int32));
        env.define_type("Int64".to_string(), TypeScheme::mono(Type::Int64));
        env.define_type("Float".to_string(), TypeScheme::mono(Type::Float));
        env.define_type("Float32".to_string(), TypeScheme::mono(Type::Float32));
        env.define_type("Float64".to_string(), TypeScheme::mono(Type::Float64));
        env.define_type("Bool".to_string(), TypeScheme::mono(Type::Bool));
        env.define_type("Char".to_string(), TypeScheme::mono(Type::Char));
        env.define_type("String".to_string(), TypeScheme::mono(Type::String));
        env.define_type("Bytes".to_string(), TypeScheme::mono(Type::Bytes));
        env.define_type("Unit".to_string(), TypeScheme::mono(Type::Unit));
        env.define_type("Never".to_string(), TypeScheme::mono(Type::Never));

        // Define built-in functions
        // I/O builtins - polymorphic, accept any type
        env.define_var(
            "print".to_string(),
            Type::Function {
                params: vec![Type::Any],
                return_type: Box::new(Type::Unit),
            },
        );
        env.define_var(
            "println".to_string(),
            Type::Function {
                params: vec![Type::Any],
                return_type: Box::new(Type::Unit),
            },
        );

        // Collection builtins
        env.define_var(
            "len".to_string(),
            Type::Function {
                params: vec![Type::Any],
                return_type: Box::new(Type::Int),
            },
        );
        env.define_var(
            "first".to_string(),
            Type::Function {
                params: vec![Type::Any],
                return_type: Box::new(Type::Any),
            },
        );
        env.define_var(
            "last".to_string(),
            Type::Function {
                params: vec![Type::Any],
                return_type: Box::new(Type::Any),
            },
        );
        env.define_var(
            "reverse".to_string(),
            Type::Function {
                params: vec![Type::Any],
                return_type: Box::new(Type::Any),
            },
        );
        env.define_var(
            "push".to_string(),
            Type::Function {
                params: vec![Type::Any, Type::Any],
                return_type: Box::new(Type::Unit),
            },
        );
        env.define_var(
            "pop".to_string(),
            Type::Function {
                params: vec![Type::Any],
                return_type: Box::new(Type::Any),
            },
        );

        // Type conversion builtins
        env.define_var(
            "to_string".to_string(),
            Type::Function {
                params: vec![Type::Any],
                return_type: Box::new(Type::String),
            },
        );
        env.define_var(
            "to_int".to_string(),
            Type::Function {
                params: vec![Type::Any],
                return_type: Box::new(Type::Int),
            },
        );
        env.define_var(
            "to_float".to_string(),
            Type::Function {
                params: vec![Type::Any],
                return_type: Box::new(Type::Float),
            },
        );
        env.define_var(
            "type_of".to_string(),
            Type::Function {
                params: vec![Type::Any],
                return_type: Box::new(Type::String),
            },
        );

        // String builtins
        env.define_var(
            "contains".to_string(),
            Type::Function {
                params: vec![Type::String, Type::String],
                return_type: Box::new(Type::Bool),
            },
        );
        env.define_var(
            "starts_with".to_string(),
            Type::Function {
                params: vec![Type::String, Type::String],
                return_type: Box::new(Type::Bool),
            },
        );
        env.define_var(
            "ends_with".to_string(),
            Type::Function {
                params: vec![Type::String, Type::String],
                return_type: Box::new(Type::Bool),
            },
        );
        env.define_var(
            "trim".to_string(),
            Type::Function {
                params: vec![Type::String],
                return_type: Box::new(Type::String),
            },
        );
        env.define_var(
            "replace".to_string(),
            Type::Function {
                params: vec![Type::String, Type::String, Type::String],
                return_type: Box::new(Type::String),
            },
        );
        env.define_var(
            "substring".to_string(),
            Type::Function {
                params: vec![Type::String, Type::Int, Type::Int],
                return_type: Box::new(Type::String),
            },
        );
        env.define_var(
            "char_at".to_string(),
            Type::Function {
                params: vec![Type::String, Type::Int],
                return_type: Box::new(Type::Int),
            },
        );
        env.define_var(
            "to_upper".to_string(),
            Type::Function {
                params: vec![Type::String],
                return_type: Box::new(Type::String),
            },
        );
        env.define_var(
            "to_lower".to_string(),
            Type::Function {
                params: vec![Type::String],
                return_type: Box::new(Type::String),
            },
        );

        // Math builtins
        env.define_var(
            "abs".to_string(),
            Type::Function {
                params: vec![Type::Any],
                return_type: Box::new(Type::Any),
            },
        );
        env.define_var(
            "min".to_string(),
            Type::Function {
                params: vec![Type::Any, Type::Any],
                return_type: Box::new(Type::Any),
            },
        );
        env.define_var(
            "max".to_string(),
            Type::Function {
                params: vec![Type::Any, Type::Any],
                return_type: Box::new(Type::Any),
            },
        );
        env.define_var(
            "sqrt".to_string(),
            Type::Function {
                params: vec![Type::Float],
                return_type: Box::new(Type::Float),
            },
        );
        env.define_var(
            "pow".to_string(),
            Type::Function {
                params: vec![Type::Float, Type::Float],
                return_type: Box::new(Type::Float),
            },
        );
        env.define_var(
            "sin".to_string(),
            Type::Function {
                params: vec![Type::Float],
                return_type: Box::new(Type::Float),
            },
        );
        env.define_var(
            "cos".to_string(),
            Type::Function {
                params: vec![Type::Float],
                return_type: Box::new(Type::Float),
            },
        );
        env.define_var(
            "tan".to_string(),
            Type::Function {
                params: vec![Type::Float],
                return_type: Box::new(Type::Float),
            },
        );
        env.define_var(
            "floor".to_string(),
            Type::Function {
                params: vec![Type::Float],
                return_type: Box::new(Type::Int),
            },
        );
        env.define_var(
            "ceil".to_string(),
            Type::Function {
                params: vec![Type::Float],
                return_type: Box::new(Type::Int),
            },
        );
        env.define_var(
            "round".to_string(),
            Type::Function {
                params: vec![Type::Float],
                return_type: Box::new(Type::Int),
            },
        );

        let mut trait_defs = FxHashMap::default();
        let mut trait_impls = FxHashMap::default();

        // Register built-in traits
        Self::register_builtin_traits(&mut trait_defs, &mut trait_impls);

        Self {
            env: Rc::new(env),
            inference: TypeInference::new(),
            module_exports: FxHashMap::default(),
            struct_fields: FxHashMap::default(),
            generic_type_params: FxHashMap::default(),
            current_return_type: None,
            trait_defs,
            trait_impls,
            in_async_context: false,
            const_values: FxHashMap::default(),
            enum_variants: FxHashMap::default(),
            in_defer_context: false,
            current_defer_span: None,
            function_signatures: FxHashMap::default(),
            loop_context_stack: Vec::new(),
            type_param_scopes: Vec::new(),
            generic_functions: FxHashMap::default(),
        }
    }

    /// Register built-in traits like Display, Debug, Clone, etc.
    fn register_builtin_traits(
        trait_defs: &mut FxHashMap<String, TraitDef>,
        trait_impls: &mut FxHashMap<String, Vec<TraitImpl>>,
    ) {
        // Display trait
        let display = TraitDef::new("Display".to_string());
        trait_defs.insert("Display".to_string(), display);

        // Debug trait
        let debug = TraitDef::new("Debug".to_string());
        trait_defs.insert("Debug".to_string(), debug);

        // Clone trait
        let clone = TraitDef::new("Clone".to_string());
        trait_defs.insert("Clone".to_string(), clone);

        // Copy trait (requires Clone)
        let mut copy = TraitDef::new("Copy".to_string());
        copy.supertraits.push(TypeBound::new("Clone".to_string()));
        trait_defs.insert("Copy".to_string(), copy);

        // Eq trait
        let eq = TraitDef::new("Eq".to_string());
        trait_defs.insert("Eq".to_string(), eq);

        // Ord trait (requires Eq)
        let mut ord = TraitDef::new("Ord".to_string());
        ord.supertraits.push(TypeBound::new("Eq".to_string()));
        trait_defs.insert("Ord".to_string(), ord);

        // Hash trait
        let hash = TraitDef::new("Hash".to_string());
        trait_defs.insert("Hash".to_string(), hash);

        // Default trait
        let default = TraitDef::new("Default".to_string());
        trait_defs.insert("Default".to_string(), default);

        // Iterator trait with associated Item type
        let mut iterator = TraitDef::new("Iterator".to_string());
        iterator.associated_types.push("Item".to_string());
        trait_defs.insert("Iterator".to_string(), iterator);

        // Numeric trait for numeric operations
        let numeric = TraitDef::new("Numeric".to_string());
        trait_defs.insert("Numeric".to_string(), numeric);

        // Register primitive type implementations
        let primitive_types = vec![
            Type::Int, Type::Int8, Type::Int16, Type::Int32, Type::Int64, Type::Int128,
            Type::UInt, Type::UInt8, Type::UInt16, Type::UInt32, Type::UInt64, Type::UInt128,
            Type::Float, Type::Float32, Type::Float64,
            Type::Bool, Type::Char, Type::String,
        ];

        for ty in &primitive_types {
            // All primitives implement Display, Debug, Clone, Copy, Eq
            for trait_name in &["Display", "Debug", "Clone", "Copy", "Eq"] {
                let impl_entry = TraitImpl {
                    trait_name: trait_name.to_string(),
                    trait_args: vec![],
                    for_type: ty.clone(),
                    where_clause: vec![],
                    methods: FxHashMap::default(),
                    associated_types: FxHashMap::default(),
                    associated_consts: FxHashMap::default(),
                };
                trait_impls.entry(trait_name.to_string())
                    .or_insert_with(Vec::new)
                    .push(impl_entry);
            }
        }

        // Numeric types also implement Ord, Hash, Numeric
        let numeric_types = vec![
            Type::Int, Type::Int8, Type::Int16, Type::Int32, Type::Int64, Type::Int128,
            Type::UInt, Type::UInt8, Type::UInt16, Type::UInt32, Type::UInt64, Type::UInt128,
            Type::Float, Type::Float32, Type::Float64,
        ];

        for ty in &numeric_types {
            for trait_name in &["Ord", "Hash", "Numeric"] {
                let impl_entry = TraitImpl {
                    trait_name: trait_name.to_string(),
                    trait_args: vec![],
                    for_type: ty.clone(),
                    where_clause: vec![],
                    methods: FxHashMap::default(),
                    associated_types: FxHashMap::default(),
                    associated_consts: FxHashMap::default(),
                };
                trait_impls.entry(trait_name.to_string())
                    .or_insert_with(Vec::new)
                    .push(impl_entry);
            }
        }
    }

    // ========================================================================
    // Testing Support Methods
    // ========================================================================

    /// Create a fresh type variable (for testing)
    pub fn fresh_var(&mut self) -> Type {
        self.inference.fresh_var()
    }

    /// Apply type substitutions to resolve a type (for testing)
    pub fn apply_substitutions(&self, ty: &Type) -> Type {
        self.inference.apply(ty)
    }

    // ========================================================================
    // Trait and Bound Checking
    // ========================================================================

    /// Check if a type implements a trait
    pub fn implements_trait(&self, ty: &Type, trait_name: &str) -> bool {
        // Handle special cases first
        match ty {
            Type::Error | Type::Any => return true,
            Type::Var(_) => return false, // Unknown type variable - conservative
            Type::Never => return true, // Never type implements everything
            _ => {}
        }

        // Check if there's a direct implementation
        if let Some(impls) = self.trait_impls.get(trait_name) {
            for impl_ in impls {
                if self.types_match(ty, &impl_.for_type) {
                    return true;
                }
            }
        }

        // Check for blanket implementations (e.g., Array<T> implements Clone if T: Clone)
        match ty {
            Type::Array(elem) | Type::Optional(elem) => {
                // Array<T>/Optional<T> implements trait if T does
                self.implements_trait(elem, trait_name)
            }
            Type::Tuple(elems) => {
                // Tuple implements trait if all elements do
                elems.iter().all(|e| self.implements_trait(e, trait_name))
            }
            Type::Result(ok, err) => {
                // Result<T, E> implements trait if both T and E do
                self.implements_trait(ok, trait_name) && self.implements_trait(err, trait_name)
            }
            Type::Map(k, v) => {
                // Map<K, V> implements trait if both K and V do
                self.implements_trait(k, trait_name) && self.implements_trait(v, trait_name)
            }
            _ => false,
        }
    }

    /// Check if a type can be converted to a string (for string interpolation).
    ///
    /// A type can be converted to a string if:
    /// 1. It is a primitive type that has built-in string conversion (Int, Float, Bool, Char, String)
    /// 2. It implements the Display trait
    /// 3. It is an Any, Error, or Never type (special handling)
    ///
    /// This is used for type checking string interpolation expressions like `"Hello, #{name}!"`.
    pub fn can_convert_to_string(&self, ty: &Type) -> bool {
        match ty {
            // Special types that can always be converted
            Type::Error | Type::Any | Type::Never => true,

            // Primitive types with built-in string conversion
            Type::Int | Type::Int8 | Type::Int16 | Type::Int32 | Type::Int64 | Type::Int128 |
            Type::UInt | Type::UInt8 | Type::UInt16 | Type::UInt32 | Type::UInt64 | Type::UInt128 |
            Type::Float | Type::Float32 | Type::Float64 |
            Type::Bool | Type::Char | Type::String => true,

            // Unit type can be displayed (as "()")
            Type::Unit => true,

            // Bytes can be displayed
            Type::Bytes => true,

            // Type variables - conservatively allow (will be resolved later)
            Type::Var(_) => true,

            // For compound types, check if Display is implemented
            // or if all elements can be converted
            Type::Array(elem) => self.can_convert_to_string(elem),
            Type::FixedArray(elem, _) => self.can_convert_to_string(elem),
            Type::Tuple(elems) => elems.iter().all(|e| self.can_convert_to_string(e)),
            Type::Optional(inner) => self.can_convert_to_string(inner),
            Type::Result(ok, err) => self.can_convert_to_string(ok) && self.can_convert_to_string(err),
            Type::Map(k, v) => self.can_convert_to_string(k) && self.can_convert_to_string(v),
            Type::Reference { inner, .. } => self.can_convert_to_string(inner),

            // Function types generally don't implement Display
            Type::Function { .. } => false,

            // Effectful function types don't implement Display
            Type::EffectfulFunction { .. } => false,

            // Channel types don't typically implement Display
            Type::Channel(_) => false,

            // Task types don't implement Display
            Type::Task(_) => false,

            // Effect row variables don't implement Display
            Type::EffectVar(_) => false,

            // Named types - check if they implement Display trait
            Type::Named { .. } => self.implements_trait(ty, "Display"),
        }
    }

    // ========================================================================
    // Type Parameter Scope Management
    // ========================================================================

    /// Enter a new type parameter scope with the given parameters
    ///
    /// Creates fresh type variables for each parameter and pushes them onto the scope stack.
    /// Type parameter references in the body will resolve to these variables.
    fn enter_type_param_scope(&mut self, params: &[ast::GenericParam]) -> Vec<(String, TypeVar)> {
        let mut scope = FxHashMap::default();
        let mut param_vars = Vec::new();

        for param in params {
            let name = param.name.node.to_string();
            // Create a fresh type variable ID
            let var_id = self.inference.next_var;
            self.inference.next_var += 1;
            let var = TypeVar(var_id);
            scope.insert(name.clone(), var);
            param_vars.push((name, var));
        }

        self.type_param_scopes.push(scope);
        param_vars
    }

    /// Exit the current type parameter scope
    fn exit_type_param_scope(&mut self) {
        self.type_param_scopes.pop();
    }

    /// Look up a type parameter in the current scopes (from innermost to outermost)
    fn lookup_type_param(&self, name: &str) -> Option<TypeVar> {
        for scope in self.type_param_scopes.iter().rev() {
            if let Some(&var) = scope.get(name) {
                return Some(var);
            }
        }
        None
    }

    /// Infer type arguments for a generic function call from the provided arguments.
    ///
    /// This method:
    /// 1. Creates fresh type variables for each type parameter
    /// 2. Unifies argument types with parameter types (substituting type params with fresh vars)
    /// 3. Resolves the type variables to get concrete type arguments
    /// 4. Validates trait bounds on the inferred types
    ///
    /// Returns the inferred concrete types for each type parameter, in order.
    fn infer_type_arguments(
        &mut self,
        generic_info: &GenericFunctionInfo,
        args: &[ast::CallArg],
        env: &TypeEnv,
        span: Span,
    ) -> TypeResult<Vec<Type>> {
        // Create fresh type variables for each type parameter
        let mut fresh_vars: FxHashMap<TypeVar, TypeVar> = FxHashMap::default();
        let mut type_param_to_fresh: FxHashMap<String, TypeVar> = FxHashMap::default();

        for (param_name, old_var) in &generic_info.type_param_vars {
            let fresh_var = TypeVar(self.inference.next_var);
            self.inference.next_var += 1;
            fresh_vars.insert(*old_var, fresh_var);
            type_param_to_fresh.insert(param_name.clone(), fresh_var);
        }

        // Helper function to substitute old type vars with fresh ones
        fn substitute_fresh(ty: &Type, fresh_vars: &FxHashMap<TypeVar, TypeVar>) -> Type {
            match ty {
                Type::Var(var) => {
                    if let Some(&fresh) = fresh_vars.get(var) {
                        Type::Var(fresh)
                    } else {
                        ty.clone()
                    }
                }
                Type::Array(elem) => Type::Array(Box::new(substitute_fresh(elem, fresh_vars))),
                Type::FixedArray(elem, size) => Type::FixedArray(Box::new(substitute_fresh(elem, fresh_vars)), *size),
                Type::Map(k, v) => Type::Map(
                    Box::new(substitute_fresh(k, fresh_vars)),
                    Box::new(substitute_fresh(v, fresh_vars)),
                ),
                Type::Tuple(elems) => Type::Tuple(elems.iter().map(|e| substitute_fresh(e, fresh_vars)).collect()),
                Type::Optional(inner) => Type::Optional(Box::new(substitute_fresh(inner, fresh_vars))),
                Type::Result(ok, err) => Type::Result(
                    Box::new(substitute_fresh(ok, fresh_vars)),
                    Box::new(substitute_fresh(err, fresh_vars)),
                ),
                Type::Reference { mutable, inner } => Type::Reference {
                    mutable: *mutable,
                    inner: Box::new(substitute_fresh(inner, fresh_vars)),
                },
                Type::Function { params, return_type } => Type::Function {
                    params: params.iter().map(|p| substitute_fresh(p, fresh_vars)).collect(),
                    return_type: Box::new(substitute_fresh(return_type, fresh_vars)),
                },
                Type::Named { name, type_args } => Type::Named {
                    name: name.clone(),
                    type_args: type_args.iter().map(|a| substitute_fresh(a, fresh_vars)).collect(),
                },
                Type::Channel(elem) => Type::Channel(Box::new(substitute_fresh(elem, fresh_vars))),
                Type::Task(result) => Type::Task(Box::new(substitute_fresh(result, fresh_vars))),
                _ => ty.clone(),
            }
        }

        // Substitute fresh vars into the parameter types
        let fresh_param_types: Vec<Type> = generic_info.param_types
            .iter()
            .map(|pt| substitute_fresh(pt, &fresh_vars))
            .collect();

        // Check argument count
        if args.len() != fresh_param_types.len() {
            return Err(TypeError::WrongTypeArity {
                expected: fresh_param_types.len(),
                found: args.len(),
                span,
            });
        }

        // Unify each argument with the corresponding parameter type
        for (arg, param_type) in args.iter().zip(fresh_param_types.iter()) {
            let arg_type = self.infer_expr(&arg.value, env)?;
            self.inference.unify(&arg_type, param_type, arg.value.span)?;
        }

        // Extract the inferred concrete types for each type parameter
        let mut inferred_types: Vec<Type> = Vec::new();
        for param_name in &generic_info.type_param_names {
            let fresh_var = type_param_to_fresh.get(param_name)
                .ok_or_else(|| TypeError::CannotInfer(span))?;
            let inferred = self.inference.apply(&Type::Var(*fresh_var));

            // Check if we actually inferred a concrete type
            if matches!(inferred, Type::Var(_)) {
                return Err(TypeError::CannotInfer(span));
            }
            inferred_types.push(inferred);
        }

        // Validate trait bounds on inferred types
        for (param_def, inferred_type) in generic_info.type_params.iter().zip(inferred_types.iter()) {
            for bound in &param_def.bounds {
                if !self.implements_trait(inferred_type, &bound.trait_name) {
                    return Err(TypeError::BoundNotSatisfied {
                        type_arg: format!("{}", inferred_type),
                        param: param_def.name.clone(),
                        bound: format!("{}", bound),
                        span,
                    });
                }
            }
        }

        Ok(inferred_types)
    }

    /// Instantiate a generic function's return type with inferred type arguments.
    ///
    /// Given the generic function info and inferred type arguments, substitutes the type
    /// parameters in the return type to produce the concrete return type.
    fn instantiate_return_type(
        &self,
        generic_info: &GenericFunctionInfo,
        inferred_types: &[Type],
    ) -> Type {
        // Build substitution map
        let mut substitution: FxHashMap<String, Type> = FxHashMap::default();
        for (param_name, inferred_type) in generic_info.type_param_names.iter().zip(inferred_types.iter()) {
            substitution.insert(param_name.clone(), inferred_type.clone());
        }

        // Build var to type substitution for Type::Var
        let mut var_subst: FxHashMap<TypeVar, Type> = FxHashMap::default();
        for (param_name, var) in &generic_info.type_param_vars {
            if let Some(ty) = substitution.get(param_name) {
                var_subst.insert(*var, ty.clone());
            }
        }

        // Substitute type vars in return type
        self.substitute_type_vars(&generic_info.return_type, &var_subst)
    }

    /// Substitute type variables with concrete types
    fn substitute_type_vars(&self, ty: &Type, var_subst: &FxHashMap<TypeVar, Type>) -> Type {
        match ty {
            Type::Var(var) => {
                if let Some(concrete) = var_subst.get(var) {
                    concrete.clone()
                } else {
                    ty.clone()
                }
            }
            Type::Array(elem) => Type::Array(Box::new(self.substitute_type_vars(elem, var_subst))),
            Type::FixedArray(elem, size) => Type::FixedArray(Box::new(self.substitute_type_vars(elem, var_subst)), *size),
            Type::Map(k, v) => Type::Map(
                Box::new(self.substitute_type_vars(k, var_subst)),
                Box::new(self.substitute_type_vars(v, var_subst)),
            ),
            Type::Tuple(elems) => Type::Tuple(elems.iter().map(|e| self.substitute_type_vars(e, var_subst)).collect()),
            Type::Optional(inner) => Type::Optional(Box::new(self.substitute_type_vars(inner, var_subst))),
            Type::Result(ok, err) => Type::Result(
                Box::new(self.substitute_type_vars(ok, var_subst)),
                Box::new(self.substitute_type_vars(err, var_subst)),
            ),
            Type::Reference { mutable, inner } => Type::Reference {
                mutable: *mutable,
                inner: Box::new(self.substitute_type_vars(inner, var_subst)),
            },
            Type::Function { params, return_type } => Type::Function {
                params: params.iter().map(|p| self.substitute_type_vars(p, var_subst)).collect(),
                return_type: Box::new(self.substitute_type_vars(return_type, var_subst)),
            },
            Type::Named { name, type_args } => Type::Named {
                name: name.clone(),
                type_args: type_args.iter().map(|a| self.substitute_type_vars(a, var_subst)).collect(),
            },
            Type::Channel(elem) => Type::Channel(Box::new(self.substitute_type_vars(elem, var_subst))),
            Type::Task(result) => Type::Task(Box::new(self.substitute_type_vars(result, var_subst))),
            _ => ty.clone(),
        }
    }

    /// Check if two types match (for impl lookup)
    fn types_match(&self, ty1: &Type, ty2: &Type) -> bool {
        match (ty1, ty2) {
            (Type::Any, _) | (_, Type::Any) => true,
            (Type::Error, _) | (_, Type::Error) => true,
            (Type::Int, Type::Int) => true,
            (Type::Int8, Type::Int8) => true,
            (Type::Int16, Type::Int16) => true,
            (Type::Int32, Type::Int32) => true,
            (Type::Int64, Type::Int64) => true,
            (Type::Int128, Type::Int128) => true,
            (Type::UInt, Type::UInt) => true,
            (Type::UInt8, Type::UInt8) => true,
            (Type::UInt16, Type::UInt16) => true,
            (Type::UInt32, Type::UInt32) => true,
            (Type::UInt64, Type::UInt64) => true,
            (Type::UInt128, Type::UInt128) => true,
            (Type::Float, Type::Float) => true,
            (Type::Float32, Type::Float32) => true,
            (Type::Float64, Type::Float64) => true,
            (Type::Bool, Type::Bool) => true,
            (Type::Char, Type::Char) => true,
            (Type::String, Type::String) => true,
            (Type::Bytes, Type::Bytes) => true,
            (Type::Unit, Type::Unit) => true,
            (Type::Never, Type::Never) => true,
            (Type::Array(e1), Type::Array(e2)) => self.types_match(e1, e2),
            (Type::Optional(e1), Type::Optional(e2)) => self.types_match(e1, e2),
            (Type::Tuple(t1), Type::Tuple(t2)) if t1.len() == t2.len() => {
                t1.iter().zip(t2.iter()).all(|(a, b)| self.types_match(a, b))
            }
            (Type::Named { name: n1, type_args: a1 }, Type::Named { name: n2, type_args: a2 }) => {
                n1 == n2 && a1.len() == a2.len() &&
                    a1.iter().zip(a2.iter()).all(|(a, b)| self.types_match(a, b))
            }
            _ => false,
        }
    }

    /// Check if a type satisfies a bound
    pub fn satisfies_bound(&self, ty: &Type, bound: &TypeBound, span: Span) -> TypeResult<()> {
        if !self.implements_trait(ty, &bound.trait_name) {
            return Err(TypeError::TraitNotImplemented {
                ty: format!("{}", ty),
                trait_name: bound.trait_name.clone(),
                span,
            });
        }

        // Check supertraits
        if let Some(trait_def) = self.trait_defs.get(&bound.trait_name) {
            for supertrait in &trait_def.supertraits {
                self.satisfies_bound(ty, supertrait, span)?;
            }
        }

        Ok(())
    }

    /// Validate that type arguments satisfy their bounds
    pub fn validate_type_args(
        &self,
        type_param_defs: &[TypeParamDef],
        type_args: &[Type],
        span: Span,
    ) -> TypeResult<()> {
        if type_param_defs.len() != type_args.len() {
            return Err(TypeError::WrongTypeArity {
                expected: type_param_defs.len(),
                found: type_args.len(),
                span,
            });
        }

        for (param_def, type_arg) in type_param_defs.iter().zip(type_args.iter()) {
            for bound in &param_def.bounds {
                if !self.implements_trait(type_arg, &bound.trait_name) {
                    return Err(TypeError::BoundNotSatisfied {
                        type_arg: format!("{}", type_arg),
                        param: param_def.name.clone(),
                        bound: format!("{}", bound),
                        span,
                    });
                }
            }
        }

        Ok(())
    }

    /// Expand a generic type alias by substituting type arguments
    ///
    /// Given a type scheme like `type Pair<A, B> = (A, B)` and arguments `[Int, String]`,
    /// this returns `(Int, String)`.
    fn expand_generic_type_alias(
        &self,
        scheme: &TypeScheme,
        type_args: &[Type],
        span: Span,
    ) -> TypeResult<Type> {
        // Validate arity first
        if scheme.type_params.len() != type_args.len() {
            return Err(TypeError::WrongTypeArity {
                expected: scheme.type_params.len(),
                found: type_args.len(),
                span,
            });
        }

        // Validate bounds if present
        self.validate_type_args(&scheme.type_param_defs, type_args, span)?;

        // Build substitution map: param name -> concrete type
        let substitution: FxHashMap<String, Type> = scheme
            .type_params
            .iter()
            .cloned()
            .zip(type_args.iter().cloned())
            .collect();

        // Apply substitution to the underlying type
        Ok(self.substitute_type(&scheme.ty, &substitution))
    }

    /// Substitute type parameters in a type with concrete types
    ///
    /// This recursively traverses the type and replaces any Named type
    /// whose name matches a key in the substitution map.
    fn substitute_type(&self, ty: &Type, substitution: &FxHashMap<String, Type>) -> Type {
        match ty {
            Type::Named { name, type_args } if type_args.is_empty() => {
                // This might be a type parameter reference
                if let Some(replacement) = substitution.get(name) {
                    return replacement.clone();
                }
                ty.clone()
            }
            Type::Named { name, type_args } => {
                // Named type with arguments - substitute in the arguments
                let new_args: Vec<Type> = type_args
                    .iter()
                    .map(|arg| self.substitute_type(arg, substitution))
                    .collect();
                Type::Named {
                    name: name.clone(),
                    type_args: new_args,
                }
            }
            Type::Array(inner) => {
                Type::Array(Box::new(self.substitute_type(inner, substitution)))
            }
            Type::FixedArray(inner, size) => {
                Type::FixedArray(Box::new(self.substitute_type(inner, substitution)), *size)
            }
            Type::Map(key, value) => Type::Map(
                Box::new(self.substitute_type(key, substitution)),
                Box::new(self.substitute_type(value, substitution)),
            ),
            Type::Tuple(elems) => {
                let new_elems: Vec<Type> = elems
                    .iter()
                    .map(|e| self.substitute_type(e, substitution))
                    .collect();
                Type::Tuple(new_elems)
            }
            Type::Optional(inner) => {
                Type::Optional(Box::new(self.substitute_type(inner, substitution)))
            }
            Type::Result(ok, err) => Type::Result(
                Box::new(self.substitute_type(ok, substitution)),
                Box::new(self.substitute_type(err, substitution)),
            ),
            Type::Reference { mutable, inner } => Type::Reference {
                mutable: *mutable,
                inner: Box::new(self.substitute_type(inner, substitution)),
            },
            Type::Function { params, return_type } => {
                let new_params: Vec<Type> = params
                    .iter()
                    .map(|p| self.substitute_type(p, substitution))
                    .collect();
                Type::Function {
                    params: new_params,
                    return_type: Box::new(self.substitute_type(return_type, substitution)),
                }
            }
            Type::Channel(inner) => {
                Type::Channel(Box::new(self.substitute_type(inner, substitution)))
            }
            Type::Task(inner) => {
                Type::Task(Box::new(self.substitute_type(inner, substitution)))
            }
            // Primitives and other types don't need substitution
            _ => ty.clone(),
        }
    }

    /// Validate where clause constraints
    pub fn validate_where_clause(
        &self,
        where_clause: &[(String, Vec<TypeBound>)],
        type_substitution: &FxHashMap<String, Type>,
        span: Span,
    ) -> TypeResult<()> {
        for (type_param, bounds) in where_clause {
            if let Some(concrete_type) = type_substitution.get(type_param) {
                for bound in bounds {
                    self.satisfies_bound(concrete_type, bound, span)?;
                }
            }
        }
        Ok(())
    }

    /// Register a trait definition
    pub fn register_trait(&mut self, trait_def: TraitDef) {
        self.trait_defs.insert(trait_def.name.clone(), trait_def);
    }

    /// Register a trait implementation
    pub fn register_trait_impl(&mut self, impl_: TraitImpl) -> TypeResult<()> {
        // Check for conflicting implementations
        if let Some(existing_impls) = self.trait_impls.get(&impl_.trait_name) {
            for existing in existing_impls {
                if self.types_match(&impl_.for_type, &existing.for_type) {
                    // Allow if the existing is more general (e.g., Any)
                    if !matches!(existing.for_type, Type::Any) {
                        return Err(TypeError::ConflictingImpl {
                            trait_name: impl_.trait_name.clone(),
                            for_type: format!("{}", impl_.for_type),
                            span: Span::dummy(),
                        });
                    }
                }
            }
        }

        self.trait_impls.entry(impl_.trait_name.clone())
            .or_insert_with(Vec::new)
            .push(impl_);
        Ok(())
    }

    /// Convert AST trait bounds to internal TypeBound representation
    pub fn resolve_trait_bounds(&self, ast_bounds: &[ast::TraitBound]) -> TypeResult<Vec<TypeBound>> {
        let mut bounds = Vec::new();
        for ast_bound in ast_bounds {
            let trait_name = ast_bound.path.iter()
                .map(|s| s.node.to_string())
                .collect::<Vec<_>>()
                .join("::");

            let type_args = if let Some(args) = &ast_bound.type_args {
                args.iter()
                    .map(|arg| self.resolve_type(arg))
                    .collect::<TypeResult<Vec<_>>>()?
            } else {
                Vec::new()
            };

            bounds.push(TypeBound::with_args(trait_name, type_args));
        }
        Ok(bounds)
    }

    /// Convert AST generic params to internal TypeParamDef representation
    pub fn resolve_generic_params(&self, ast_params: &ast::GenericParams) -> TypeResult<Vec<TypeParamDef>> {
        let mut params = Vec::new();
        for ast_param in &ast_params.params {
            let bounds = self.resolve_trait_bounds(&ast_param.bounds)?;
            params.push(TypeParamDef::with_bounds(
                ast_param.name.node.to_string(),
                bounds,
            ));
        }
        Ok(params)
    }

    /// Register a module's exports for cross-module type checking
    ///
    /// This should be called for each dependency module before type checking
    /// the module that imports from it.
    pub fn register_module_exports(&mut self, module_name: String, exports: ModuleExports) {
        self.module_exports.insert(module_name, exports);
    }

    /// Process imports and add imported symbols to the environment
    ///
    /// This resolves import statements against registered module exports
    /// and adds the imported symbols to the type environment.
    pub fn process_imports(&mut self, program: &ast::Program) -> TypeResult<()> {
        for item in &program.items {
            if let ast::Item::Import(import) = item {
                self.process_import(import)?;
            }
        }
        Ok(())
    }

    /// Process a single import declaration
    fn process_import(&mut self, import: &ast::ImportDecl) -> TypeResult<()> {
        // Get module name from import path
        let module_name = match &import.path {
            ast::ImportPath::Module(segments) => {
                segments.iter().map(|s| s.node.as_str()).collect::<Vec<_>>().join("::")
            }
            ast::ImportPath::String(path) => path.to_string(),
        };

        // Look up the module's exports
        let exports = self.module_exports.get(&module_name).cloned();
        let exports = match exports {
            Some(e) => e,
            None => {
                // Module might be a stdlib or external module - allow for now
                // In the future, we could warn about unknown modules
                return Ok(());
            }
        };

        // Need to make env mutable - get a mutable reference
        let env = Rc::make_mut(&mut self.env);

        // Process import selection
        match &import.selection {
            Some(ast::ImportSelection::All) => {
                // Import all exported symbols
                for (name, export) in &exports {
                    if export.is_type {
                        env.define_type(name.clone(), TypeScheme::mono(export.ty.clone()));
                    } else {
                        env.define_var(name.clone(), export.ty.clone());
                    }
                }
            }
            Some(ast::ImportSelection::Items(items)) => {
                // Import specific symbols
                for item in items {
                    let name = item.name.node.as_str();
                    let local_name = item.alias.as_ref()
                        .map(|a| a.node.to_string())
                        .unwrap_or_else(|| name.to_string());

                    match exports.get(name) {
                        Some(export) => {
                            if export.is_type {
                                env.define_type(local_name, TypeScheme::mono(export.ty.clone()));
                            } else {
                                env.define_var(local_name, export.ty.clone());
                            }
                        }
                        None => {
                            return Err(TypeError::ImportNotExported {
                                symbol: name.to_string(),
                                module: module_name.clone(),
                                span: item.name.span,
                            });
                        }
                    }
                }
            }
            None => {
                // Import the module as a namespace (aliased import)
                // For now, import all symbols prefixed with module name or alias
                let prefix = import.alias.as_ref()
                    .map(|a| a.node.to_string())
                    .unwrap_or_else(|| module_name.split("::").last().unwrap_or(&module_name).to_string());

                for (name, export) in &exports {
                    let qualified_name = format!("{}::{}", prefix, name);
                    if export.is_type {
                        env.define_type(qualified_name, TypeScheme::mono(export.ty.clone()));
                    } else {
                        env.define_var(qualified_name, export.ty.clone());
                    }
                }
            }
        }

        Ok(())
    }

    /// Extract exports from a type-checked program
    ///
    /// This should be called after type checking a module to collect
    /// its exported symbols for use by dependent modules.
    pub fn extract_exports(&self, program: &ast::Program) -> ModuleExports {
        let mut exports = ModuleExports::default();

        for item in &program.items {
            match item {
                ast::Item::Function(func) if func.visibility == ast::Visibility::Public => {
                    // Build function type
                    let params: Vec<Type> = func.params.iter().map(|p| {
                        p.ty.as_ref()
                            .and_then(|ty| self.resolve_type(ty).ok())
                            .unwrap_or(Type::Any)
                    }).collect();
                    let return_type = func.return_type.as_ref()
                        .and_then(|ty| self.resolve_type(ty).ok())
                        .unwrap_or(Type::Unit);

                    exports.insert(
                        func.name.node.to_string(),
                        ModuleExport {
                            ty: Type::Function {
                                params,
                                return_type: Box::new(return_type),
                            },
                            is_type: false,
                        },
                    );
                }
                ast::Item::Struct(s) if s.visibility == ast::Visibility::Public => {
                    exports.insert(
                        s.name.node.to_string(),
                        ModuleExport {
                            ty: Type::Named {
                                name: s.name.node.to_string(),
                                type_args: vec![],
                            },
                            is_type: true,
                        },
                    );
                }
                ast::Item::Enum(e) if e.visibility == ast::Visibility::Public => {
                    exports.insert(
                        e.name.node.to_string(),
                        ModuleExport {
                            ty: Type::Named {
                                name: e.name.node.to_string(),
                                type_args: vec![],
                            },
                            is_type: true,
                        },
                    );
                }
                ast::Item::Const(c) if c.visibility == ast::Visibility::Public => {
                    let ty = c.ty.as_ref()
                        .and_then(|ty| self.resolve_type(ty).ok())
                        .unwrap_or(Type::Any);
                    exports.insert(
                        c.name.node.to_string(),
                        ModuleExport {
                            ty,
                            is_type: false,
                        },
                    );
                }
                ast::Item::TypeAlias(t) if t.visibility == ast::Visibility::Public => {
                    let ty = self.resolve_type(&t.ty).unwrap_or(Type::Error);
                    exports.insert(
                        t.name.node.to_string(),
                        ModuleExport {
                            ty,
                            is_type: true,
                        },
                    );
                }
                _ => {}
            }
        }

        exports
    }

    /// Check a program
    pub fn check_program(&mut self, program: &ast::Program) -> TypeResult<()> {
        // First process imports to make imported symbols available
        self.process_imports(program)?;

        for item in &program.items {
            self.check_item(item)?;
        }
        Ok(())
    }

    /// Check an item
    fn check_item(&mut self, item: &ast::Item) -> TypeResult<()> {
        match item {
            ast::Item::Function(func) => self.check_function(func),
            ast::Item::Struct(s) => self.check_struct(s),
            ast::Item::Enum(e) => self.check_enum(e),
            ast::Item::Trait(t) => self.check_trait(t),
            ast::Item::Impl(i) => self.check_impl(i),
            ast::Item::Extern(extern_decl) => self.check_extern(extern_decl),
            ast::Item::TypeAlias(alias) => self.check_type_alias(alias),
            // TODO: implement other items
            _ => Ok(()),
        }
    }

    /// Check a type alias declaration
    ///
    /// Type aliases define alternative names for types:
    /// - Simple aliases: `type UserId = Int`
    /// - Generic aliases: `type Result<T> = Result<T, Error>`
    /// - Nested aliases: aliases that reference other aliases are resolved
    ///
    /// The alias is registered in the type environment for use in type resolution.
    fn check_type_alias(&mut self, alias: &ast::TypeAlias) -> TypeResult<()> {
        let alias_name = alias.name.node.to_string();

        // Process generic parameters if present
        let type_param_defs: Vec<TypeParamDef> = if let Some(ref generic_params) = alias.generic_params {
            self.resolve_generic_params(generic_params)?
        } else {
            Vec::new()
        };

        // Create a temporary scope with the generic parameters bound to placeholder types
        // This allows the aliased type to reference the generic parameters
        let type_param_names: Vec<String> = type_param_defs.iter()
            .map(|p| p.name.clone())
            .collect();

        // Temporarily define generic params as type placeholders for resolution
        let original_env = Rc::clone(&self.env);
        let mut temp_env = TypeEnv::with_parent(original_env.clone());
        for param_name in &type_param_names {
            // Create a Named type placeholder for each generic parameter
            temp_env.define_type(
                param_name.clone(),
                TypeScheme::mono(Type::Named {
                    name: param_name.clone(),
                    type_args: Vec::new(),
                }),
            );
        }
        self.env = Rc::new(temp_env);

        // Resolve the aliased type (this will expand nested aliases)
        let resolved_type = self.resolve_type(&alias.ty)?;

        // Restore the original environment
        self.env = original_env;

        // Create the type scheme for the alias
        let type_scheme = if type_param_defs.is_empty() {
            TypeScheme::mono(resolved_type)
        } else {
            TypeScheme::poly_bounded(type_param_defs, resolved_type)
        };

        // Register the type alias in the environment
        Rc::make_mut(&mut self.env).define_type(alias_name, type_scheme);

        Ok(())
    }

    // ========================================================================
    // Const Expression Evaluation
    // ========================================================================

    /// Check a const declaration and evaluate its value at compile time
    ///
    /// Const declarations define compile-time constant values:
    /// - `const PI = 3.14159`
    /// - `const MAX_SIZE: Int = 1024`
    /// - `const DOUBLED = MAX_SIZE * 2`  (const folding)
    ///
    /// The const value is evaluated and stored for const propagation.
    #[allow(dead_code)]
    fn check_const_decl(&mut self, decl: &ast::ConstDecl) -> TypeResult<()> {
        let const_name = decl.name.node.to_string();
        let env = TypeEnv::with_parent(Rc::clone(&self.env));

        // Type check the const expression
        let value_type = self.infer_expr(&decl.value, &env)?;

        // If type annotation present, verify it matches
        let const_type = if let Some(ref ty) = decl.ty {
            let resolved = self.resolve_type(ty)?;
            self.inference.unify(&value_type, &resolved, decl.span)?;
            resolved
        } else {
            self.inference.apply(&value_type)
        };

        // Try to evaluate the const expression at compile time
        if let Ok(const_val) = self.eval_const_expr(&decl.value) {
            // Verify the evaluated value's type matches
            let val_type = const_val.ty();
            if self.types_match(&val_type, &const_type) || matches!(const_type, Type::Any) {
                self.const_values.insert(const_name.clone(), const_val);
            }
        }

        // Register the const in the environment as a variable
        Rc::make_mut(&mut self.env).define_var(const_name, const_type);

        Ok(())
    }

    /// Evaluate an expression at compile time to get a constant value
    ///
    /// Returns `Ok(ConstValue)` if the expression can be evaluated at compile time,
    /// or `Err` if it contains runtime-only constructs.
    pub fn eval_const_expr(&self, expr: &ast::Expr) -> TypeResult<ConstValue> {
        match &expr.kind {
            // Literals are directly convertible to const values
            ast::ExprKind::Integer(s) => {
                let value: i128 = s.parse().map_err(|_| TypeError::ConstEvalError {
                    reason: format!("Invalid integer literal: {}", s),
                    span: expr.span,
                })?;
                Ok(ConstValue::Int(value))
            }

            ast::ExprKind::Float(s) => {
                let value: f64 = s.parse().map_err(|_| TypeError::ConstEvalError {
                    reason: format!("Invalid float literal: {}", s),
                    span: expr.span,
                })?;
                Ok(ConstValue::Float(value))
            }

            ast::ExprKind::Bool(b) => Ok(ConstValue::Bool(*b)),

            ast::ExprKind::Char(s) => {
                let c = s.chars().next().ok_or_else(|| TypeError::ConstEvalError {
                    reason: "Empty character literal".to_string(),
                    span: expr.span,
                })?;
                Ok(ConstValue::Char(c))
            }

            ast::ExprKind::String(s) => Ok(ConstValue::String(s.to_string())),

            // Identifiers - look up const values
            ast::ExprKind::Ident(name) => {
                let name_str = name.to_string();
                self.const_values
                    .get(&name_str)
                    .cloned()
                    .ok_or_else(|| TypeError::NotConstant {
                        reason: format!("Variable '{}' is not a compile-time constant", name_str),
                        span: expr.span,
                    })
            }

            // Tuples - evaluate all elements
            ast::ExprKind::Tuple(elements) => {
                let values: Vec<ConstValue> = elements
                    .iter()
                    .map(|e| self.eval_const_expr(e))
                    .collect::<TypeResult<Vec<_>>>()?;
                Ok(ConstValue::Tuple(values))
            }

            // Arrays - evaluate all elements
            ast::ExprKind::Array(elements) => {
                let values: Vec<ConstValue> = elements
                    .iter()
                    .map(|e| self.eval_const_expr(e))
                    .collect::<TypeResult<Vec<_>>>()?;
                Ok(ConstValue::Array(values))
            }

            // Unary operators
            ast::ExprKind::Unary { op, operand } => {
                let val = self.eval_const_expr(operand)?;
                self.eval_const_unary(*op, val, expr.span)
            }

            // Binary operators - const folding
            ast::ExprKind::Binary { op, left, right } => {
                let left_val = self.eval_const_expr(left)?;
                let right_val = self.eval_const_expr(right)?;
                self.eval_const_binary(*op, left_val, right_val, expr.span)
            }

            // Parenthesized expressions
            ast::ExprKind::Paren(inner) => self.eval_const_expr(inner),

            // If expressions with const conditions are not supported for now
            // as they use Block types which contain statements, not simple expressions
            ast::ExprKind::If { .. } => {
                Err(TypeError::NotConstant {
                    reason: "If expressions cannot be evaluated at compile time".to_string(),
                    span: expr.span,
                })
            }

            // Everything else is not a compile-time constant
            _ => Err(TypeError::NotConstant {
                reason: "Expression cannot be evaluated at compile time".to_string(),
                span: expr.span,
            }),
        }
    }

    /// Evaluate a unary operation on a const value
    fn eval_const_unary(&self, op: ast::UnaryOp, val: ConstValue, span: Span) -> TypeResult<ConstValue> {
        match (op, val) {
            // Numeric negation
            (ast::UnaryOp::Neg, ConstValue::Int(n)) => {
                n.checked_neg()
                    .map(ConstValue::Int)
                    .ok_or(TypeError::ConstOverflow { span })
            }
            (ast::UnaryOp::Neg, ConstValue::Float(f)) => Ok(ConstValue::Float(-f)),

            // Logical not
            (ast::UnaryOp::Not, ConstValue::Bool(b)) => Ok(ConstValue::Bool(!b)),

            // Bitwise not
            (ast::UnaryOp::BitNot, ConstValue::Int(n)) => Ok(ConstValue::Int(!n)),
            (ast::UnaryOp::BitNot, ConstValue::UInt(n)) => Ok(ConstValue::UInt(!n)),

            _ => Err(TypeError::ConstEvalError {
                reason: "Invalid unary operation for const evaluation".to_string(),
                span,
            }),
        }
    }

    /// Evaluate a binary operation on two const values (const folding)
    fn eval_const_binary(
        &self,
        op: ast::BinaryOp,
        left: ConstValue,
        right: ConstValue,
        span: Span,
    ) -> TypeResult<ConstValue> {
        use ast::BinaryOp::*;

        match (left, right) {
            // Integer arithmetic
            (ConstValue::Int(l), ConstValue::Int(r)) => match op {
                Add => l.checked_add(r).map(ConstValue::Int).ok_or(TypeError::ConstOverflow { span }),
                Sub => l.checked_sub(r).map(ConstValue::Int).ok_or(TypeError::ConstOverflow { span }),
                Mul => l.checked_mul(r).map(ConstValue::Int).ok_or(TypeError::ConstOverflow { span }),
                Div => {
                    if r == 0 {
                        Err(TypeError::ConstDivisionByZero { span })
                    } else {
                        l.checked_div(r).map(ConstValue::Int).ok_or(TypeError::ConstOverflow { span })
                    }
                }
                Mod => {
                    if r == 0 {
                        Err(TypeError::ConstDivisionByZero { span })
                    } else {
                        l.checked_rem(r).map(ConstValue::Int).ok_or(TypeError::ConstOverflow { span })
                    }
                }
                // Comparisons
                Eq => Ok(ConstValue::Bool(l == r)),
                NotEq => Ok(ConstValue::Bool(l != r)),
                Lt => Ok(ConstValue::Bool(l < r)),
                LtEq => Ok(ConstValue::Bool(l <= r)),
                Gt => Ok(ConstValue::Bool(l > r)),
                GtEq => Ok(ConstValue::Bool(l >= r)),
                // Bitwise operations
                BitAnd => Ok(ConstValue::Int(l & r)),
                BitOr => Ok(ConstValue::Int(l | r)),
                BitXor => Ok(ConstValue::Int(l ^ r)),
                Shl => {
                    let shift = u32::try_from(r).map_err(|_| TypeError::ConstEvalError {
                        reason: "Shift amount too large".to_string(),
                        span,
                    })?;
                    l.checked_shl(shift).map(ConstValue::Int).ok_or(TypeError::ConstOverflow { span })
                }
                Shr => {
                    let shift = u32::try_from(r).map_err(|_| TypeError::ConstEvalError {
                        reason: "Shift amount too large".to_string(),
                        span,
                    })?;
                    l.checked_shr(shift).map(ConstValue::Int).ok_or(TypeError::ConstOverflow { span })
                }
                // Power
                Pow => {
                    let exp = u32::try_from(r).map_err(|_| TypeError::ConstEvalError {
                        reason: "Exponent too large or negative".to_string(),
                        span,
                    })?;
                    l.checked_pow(exp).map(ConstValue::Int).ok_or(TypeError::ConstOverflow { span })
                }
                _ => Err(TypeError::ConstEvalError {
                    reason: format!("Unsupported binary operation {:?} for integers", op),
                    span,
                }),
            },

            // Unsigned integer arithmetic
            (ConstValue::UInt(l), ConstValue::UInt(r)) => match op {
                Add => l.checked_add(r).map(ConstValue::UInt).ok_or(TypeError::ConstOverflow { span }),
                Sub => l.checked_sub(r).map(ConstValue::UInt).ok_or(TypeError::ConstOverflow { span }),
                Mul => l.checked_mul(r).map(ConstValue::UInt).ok_or(TypeError::ConstOverflow { span }),
                Div => {
                    if r == 0 {
                        Err(TypeError::ConstDivisionByZero { span })
                    } else {
                        l.checked_div(r).map(ConstValue::UInt).ok_or(TypeError::ConstOverflow { span })
                    }
                }
                Mod => {
                    if r == 0 {
                        Err(TypeError::ConstDivisionByZero { span })
                    } else {
                        l.checked_rem(r).map(ConstValue::UInt).ok_or(TypeError::ConstOverflow { span })
                    }
                }
                // Comparisons
                Eq => Ok(ConstValue::Bool(l == r)),
                NotEq => Ok(ConstValue::Bool(l != r)),
                Lt => Ok(ConstValue::Bool(l < r)),
                LtEq => Ok(ConstValue::Bool(l <= r)),
                Gt => Ok(ConstValue::Bool(l > r)),
                GtEq => Ok(ConstValue::Bool(l >= r)),
                // Bitwise operations
                BitAnd => Ok(ConstValue::UInt(l & r)),
                BitOr => Ok(ConstValue::UInt(l | r)),
                BitXor => Ok(ConstValue::UInt(l ^ r)),
                _ => Err(TypeError::ConstEvalError {
                    reason: format!("Unsupported binary operation {:?} for unsigned integers", op),
                    span,
                }),
            },

            // Float arithmetic
            (ConstValue::Float(l), ConstValue::Float(r)) => match op {
                Add => Ok(ConstValue::Float(l + r)),
                Sub => Ok(ConstValue::Float(l - r)),
                Mul => Ok(ConstValue::Float(l * r)),
                Div => {
                    if r == 0.0 {
                        Err(TypeError::ConstDivisionByZero { span })
                    } else {
                        Ok(ConstValue::Float(l / r))
                    }
                }
                Mod => {
                    if r == 0.0 {
                        Err(TypeError::ConstDivisionByZero { span })
                    } else {
                        Ok(ConstValue::Float(l % r))
                    }
                }
                Pow => Ok(ConstValue::Float(l.powf(r))),
                // Comparisons
                Eq => Ok(ConstValue::Bool(l == r)),
                NotEq => Ok(ConstValue::Bool(l != r)),
                Lt => Ok(ConstValue::Bool(l < r)),
                LtEq => Ok(ConstValue::Bool(l <= r)),
                Gt => Ok(ConstValue::Bool(l > r)),
                GtEq => Ok(ConstValue::Bool(l >= r)),
                _ => Err(TypeError::ConstEvalError {
                    reason: format!("Unsupported binary operation {:?} for floats", op),
                    span,
                }),
            },

            // Mixed int/float - promote to float
            (ConstValue::Int(l), ConstValue::Float(r)) => {
                self.eval_const_binary(op, ConstValue::Float(l as f64), ConstValue::Float(r), span)
            }
            (ConstValue::Float(l), ConstValue::Int(r)) => {
                self.eval_const_binary(op, ConstValue::Float(l), ConstValue::Float(r as f64), span)
            }

            // Boolean logic
            (ConstValue::Bool(l), ConstValue::Bool(r)) => match op {
                And => Ok(ConstValue::Bool(l && r)),
                Or => Ok(ConstValue::Bool(l || r)),
                Eq => Ok(ConstValue::Bool(l == r)),
                NotEq => Ok(ConstValue::Bool(l != r)),
                _ => Err(TypeError::ConstEvalError {
                    reason: format!("Unsupported binary operation {:?} for booleans", op),
                    span,
                }),
            },

            // String concatenation
            (ConstValue::String(l), ConstValue::String(r)) => match op {
                Add => Ok(ConstValue::String(format!("{}{}", l, r))),
                Eq => Ok(ConstValue::Bool(l == r)),
                NotEq => Ok(ConstValue::Bool(l != r)),
                _ => Err(TypeError::ConstEvalError {
                    reason: format!("Unsupported binary operation {:?} for strings", op),
                    span,
                }),
            },

            _ => Err(TypeError::ConstEvalError {
                reason: "Type mismatch in const binary operation".to_string(),
                span,
            }),
        }
    }

    /// Look up a const value by name
    pub fn lookup_const(&self, name: &str) -> Option<&ConstValue> {
        self.const_values.get(name)
    }

    /// Try to evaluate an expression and return its const value if it's a compile-time constant
    ///
    /// This is useful for contexts that require compile-time values, like array sizes.
    pub fn try_eval_const(&self, expr: &ast::Expr) -> Option<ConstValue> {
        self.eval_const_expr(expr).ok()
    }

    /// Check a function
    fn check_function(&mut self, func: &ast::FunctionDecl) -> TypeResult<()> {
        // Enter type parameter scope if function is generic
        let type_param_vars = if let Some(ref generic_params) = func.generic_params {
            self.enter_type_param_scope(&generic_params.params)
        } else {
            Vec::new()
        };

        // Create new scope for function
        let mut local_env = TypeEnv::with_parent(Rc::clone(&self.env));

        // Validate and collect parameter information
        let mut param_infos: Vec<ParamInfo> = Vec::new();
        let mut seen_default = false;

        for param in &func.params {
            let param_type = if let Some(ty) = &param.ty {
                self.resolve_type(ty)?
            } else {
                self.inference.fresh_var()
            };

            let param_name = param.name.node.to_string();
            let has_default = param.default.is_some();

            // Check that default parameters come after required parameters
            if has_default {
                seen_default = true;
            } else if seen_default {
                return Err(TypeError::DefaultAfterRequired {
                    param_name: param_name.clone(),
                    span: param.span,
                });
            }

            // If there's a default value, type-check it and verify it matches param type
            if let Some(ref default_expr) = param.default {
                let default_type = self.infer_expr(default_expr, &local_env)?;
                let resolved_param_type = self.inference.apply(&param_type);
                let resolved_default_type = self.inference.apply(&default_type);

                // Try to unify default type with parameter type
                if self.inference.unify(&default_type, &param_type, default_expr.span).is_err() {
                    return Err(TypeError::DefaultValueTypeMismatch {
                        param_name: param_name.clone(),
                        param_type: format!("{}", resolved_param_type),
                        default_type: format!("{}", resolved_default_type),
                        span: default_expr.span,
                    });
                }
            }

            local_env.define_var(param_name.clone(), param_type.clone());
            param_infos.push(ParamInfo {
                name: param_name,
                ty: param_type,
                has_default,
            });
        }

        // Determine return type
        let return_type = if let Some(ty) = &func.return_type {
            self.resolve_type(ty)?
        } else {
            self.inference.fresh_var()
        };

        // Save the previous return type (for nested functions) and set the current one
        let prev_return_type = self.current_return_type.take();
        self.current_return_type = Some(return_type.clone());

        // Check function body and unify with return type
        let body_result = match &func.body {
            ast::FunctionBody::Block(block) => {
                let mut last_type = Type::Unit;
                for stmt in &block.stmts {
                    last_type = self.check_stmt(stmt, &mut local_env)?;
                }
                Ok(last_type)
            }
            ast::FunctionBody::Expression(expr) => {
                self.infer_expr(expr, &local_env)
            }
        };

        // Restore the previous return type
        self.current_return_type = prev_return_type;

        // Get the body type, propagating any errors
        let body_type = body_result?;

        // Unify body type with declared return type
        self.inference.unify(&body_type, &return_type, func.span)?;

        // Verify contracts
        self.verify_contracts(&func.contracts, &return_type, &local_env)?;

        // Register function signature with default parameter info
        let func_name = func.name.node.to_string();
        let signature = FunctionSignature::new(param_infos.clone(), return_type.clone());
        self.function_signatures.insert(func_name.clone(), signature);

        // Collect parameter types
        let param_types: Vec<Type> = param_infos.iter().map(|p| p.ty.clone()).collect();

        // Register generic function info if this is a generic function
        if let Some(ref generic_params) = func.generic_params {
            // Resolve type parameter definitions
            let type_param_defs = self.resolve_generic_params(generic_params)?;

            // Create GenericFunctionInfo with the parameter types (which may contain Type::Var)
            let generic_info = GenericFunctionInfo::new(
                type_param_defs,
                param_types.clone(),
                return_type.clone(),
                type_param_vars.clone(),
            );
            self.generic_functions.insert(func_name.clone(), generic_info);
        }

        let func_type = Type::Function {
            params: param_types,
            return_type: Box::new(return_type),
        };

        // Register the function in the environment for call resolution
        Rc::make_mut(&mut self.env).define_var(func_name, func_type);

        // Exit type parameter scope
        if func.generic_params.is_some() {
            self.exit_type_param_scope();
        }

        Ok(())
    }

    /// Verify function contracts
    ///
    /// Type-checks contract expressions and classifies them for verification tier.
    fn verify_contracts(
        &mut self,
        contracts: &[ast::Contract],
        return_type: &Type,
        env: &TypeEnv,
    ) -> TypeResult<()> {
        let verifier = ContractVerifier::new(VerifierConfig::development());

        for contract in contracts {
            match contract {
                ast::Contract::Requires(clause) => {
                    // Type-check the precondition - should be Bool
                    let cond_type = self.infer_expr(&clause.condition, env)?;
                    self.inference.unify(&cond_type, &Type::Bool, clause.span)?;

                    // Classify the contract for verification tier
                    let classification = verifier.classify(contract);
                    if classification.tier == ContractTier::Tier3Dynamic {
                        // Emit warning for runtime-only contracts
                        // TODO: Add warning infrastructure
                    }
                }
                ast::Contract::Ensures(clause) => {
                    // Type-check the postcondition with 'result' in scope
                    let mut ensures_env = TypeEnv::with_parent(Rc::new(env.clone()));
                    ensures_env.define_var("result".to_string(), return_type.clone());

                    let cond_type = self.infer_expr(&clause.condition, &ensures_env)?;
                    self.inference.unify(&cond_type, &Type::Bool, clause.span)?;

                    // Classify the contract
                    let classification = verifier.classify(contract);
                    if classification.tier == ContractTier::Tier3Dynamic {
                        // TODO: Emit warning for runtime-only contracts
                    }
                }
                ast::Contract::Invariant(clause) => {
                    // Type-check invariant - should be Bool
                    let cond_type = self.infer_expr(&clause.condition, env)?;
                    self.inference.unify(&cond_type, &Type::Bool, clause.span)?;
                }
            }
        }

        Ok(())
    }

    /// Check a struct
    fn check_struct(&mut self, s: &ast::StructDecl) -> TypeResult<()> {
        let struct_name = s.name.node.to_string();

        // Enter type parameter scope if struct is generic
        if let Some(ref generic_params) = s.generic_params {
            self.enter_type_param_scope(&generic_params.params);
        }

        // Process generic parameters if present
        let type_param_defs: Vec<TypeParamDef> = if let Some(ref generic_params) = s.generic_params {
            self.resolve_generic_params(generic_params)?
        } else {
            Vec::new()
        };

        // Store generic type parameter definitions for bound validation later
        if !type_param_defs.is_empty() {
            self.generic_type_params.insert(struct_name.clone(), type_param_defs.clone());
        }

        // Resolve and collect all field types
        let mut fields: Vec<(String, Type)> = Vec::new();
        for field in &s.fields {
            let field_type = self.resolve_type(&field.ty)?;
            fields.push((field.name.node.to_string(), field_type));
        }

        // Register struct fields for field access lookup
        self.struct_fields.insert(struct_name.clone(), fields);

        // Create proper type scheme - generic for parameterized structs, mono otherwise
        let type_args: Vec<Type> = type_param_defs.iter()
            .map(|p| Type::Named { name: p.name.clone(), type_args: vec![] })
            .collect();

        let struct_type = Type::Named {
            name: struct_name.clone(),
            type_args,
        };

        let type_scheme = if type_param_defs.is_empty() {
            TypeScheme::mono(struct_type)
        } else {
            TypeScheme::poly_bounded(type_param_defs, struct_type)
        };

        Rc::make_mut(&mut self.env).define_type(struct_name, type_scheme);

        // Exit type parameter scope
        if s.generic_params.is_some() {
            self.exit_type_param_scope();
        }

        Ok(())
    }

    /// Check an enum and register its variant constructors
    fn check_enum(&mut self, e: &ast::EnumDecl) -> TypeResult<()> {
        let enum_name = e.name.node.to_string();

        // Enter type parameter scope if enum is generic
        // Capture the type parameter variables for use in pattern matching substitution
        let type_param_vars: FxHashMap<String, TypeVar> = if let Some(ref generic_params) = e.generic_params {
            self.enter_type_param_scope(&generic_params.params).into_iter().collect()
        } else {
            FxHashMap::default()
        };

        // Process generic parameters if present
        let type_param_defs: Vec<TypeParamDef> = if let Some(ref generic_params) = e.generic_params {
            self.resolve_generic_params(generic_params)?
        } else {
            Vec::new()
        };

        // Store generic type parameter definitions for bound validation later
        if !type_param_defs.is_empty() {
            self.generic_type_params.insert(enum_name.clone(), type_param_defs.clone());
        }

        // Build variant information map
        let mut variants_map: FxHashMap<String, VariantData> = FxHashMap::default();

        // Verify all variant types exist and build variant data
        for variant in &e.variants {
            let variant_name = variant.name.node.to_string();
            let variant_data = match &variant.data {
                ast::EnumVariantData::Unit => VariantData::Unit,
                ast::EnumVariantData::Tuple(types) => {
                    let resolved: Vec<Type> = types
                        .iter()
                        .map(|ty| self.resolve_type(ty))
                        .collect::<TypeResult<Vec<_>>>()?;
                    VariantData::Tuple(resolved)
                }
                ast::EnumVariantData::Struct(fields) => {
                    let resolved: Vec<(String, Type)> = fields
                        .iter()
                        .map(|f| Ok((f.name.node.to_string(), self.resolve_type(&f.ty)?)))
                        .collect::<TypeResult<Vec<_>>>()?;
                    VariantData::Struct(resolved)
                }
                ast::EnumVariantData::Discriminant(_) => {
                    // Discriminant variants are treated like unit variants for type checking
                    VariantData::Unit
                }
            };
            variants_map.insert(variant_name, variant_data);
        }

        // Store enum variant information for pattern matching
        let enum_info = EnumVariantInfo {
            enum_name: enum_name.clone(),
            type_params: type_param_defs.clone(),
            variants: variants_map.clone(),
            type_param_vars: type_param_vars.clone(),
        };
        self.enum_variants.insert(enum_name.clone(), enum_info);

        // Create the enum type
        let type_args: Vec<Type> = type_param_defs.iter()
            .map(|p| Type::Named { name: p.name.clone(), type_args: vec![] })
            .collect();

        let enum_type = Type::Named {
            name: enum_name.clone(),
            type_args: type_args.clone(),
        };

        let type_scheme = if type_param_defs.is_empty() {
            TypeScheme::mono(enum_type.clone())
        } else {
            TypeScheme::poly_bounded(type_param_defs.clone(), enum_type.clone())
        };

        Rc::make_mut(&mut self.env).define_type(enum_name.clone(), type_scheme);

        // Register variant constructors for use in expressions
        for (variant_name, variant_data) in &variants_map {
            let qualified_name = format!("{}::{}", enum_name, variant_name);
            let constructor_type = match variant_data {
                VariantData::Unit => enum_type.clone(),
                VariantData::Tuple(field_types) => Type::Function {
                    params: field_types.clone(),
                    return_type: Box::new(enum_type.clone()),
                },
                VariantData::Struct(fields) => Type::Function {
                    params: fields.iter().map(|(_, ty)| ty.clone()).collect(),
                    return_type: Box::new(enum_type.clone()),
                },
            };

            // Register constructor with its type
            Rc::make_mut(&mut self.env).define_var(qualified_name, constructor_type);
        }

        // Exit type parameter scope
        if e.generic_params.is_some() {
            self.exit_type_param_scope();
        }

        Ok(())
    }

    /// Check a trait definition
    ///
    /// This validates:
    /// - Generic parameters are well-formed
    /// - Supertrait bounds exist and are valid
    /// - Method signatures are valid
    /// - Associated types and constants are valid
    fn check_trait(&mut self, trait_decl: &ast::TraitDecl) -> TypeResult<()> {
        let trait_name = trait_decl.name.node.to_string();

        // Process generic parameters if present
        let type_param_defs: Vec<TypeParamDef> = if let Some(ref generic_params) = trait_decl.generic_params {
            self.resolve_generic_params(generic_params)?
        } else {
            Vec::new()
        };

        // Build the TraitDef
        let mut trait_def = TraitDef::new(trait_name.clone());
        trait_def.type_params = type_param_defs;

        // Resolve and validate supertraits
        for supertrait in &trait_decl.supertraits {
            let supertrait_name = supertrait.path.iter()
                .map(|s| s.node.to_string())
                .collect::<Vec<_>>()
                .join("::");

            // Check that the supertrait exists
            if !self.trait_defs.contains_key(&supertrait_name) {
                return Err(TypeError::UndefinedTrait(supertrait_name, supertrait.span));
            }

            let type_args = if let Some(ref args) = supertrait.type_args {
                args.iter()
                    .map(|arg| self.resolve_type(arg))
                    .collect::<TypeResult<Vec<_>>>()?
            } else {
                Vec::new()
            };

            trait_def.supertraits.push(TypeBound::with_args(supertrait_name, type_args));
        }

        // Process trait members
        for member in &trait_decl.members {
            match member {
                ast::TraitMember::Method(method) => {
                    let param_types: Vec<Type> = method.params.iter()
                        .map(|p| {
                            if let Some(ref ty) = p.ty {
                                self.resolve_type(ty)
                            } else {
                                Ok(Type::Named { name: "Self".to_string(), type_args: vec![] })
                            }
                        })
                        .collect::<TypeResult<Vec<_>>>()?;

                    let return_type = if let Some(ref ty) = method.return_type {
                        self.resolve_type(ty)?
                    } else {
                        Type::Unit
                    };

                    let method_type = Type::Function {
                        params: param_types,
                        return_type: Box::new(return_type),
                    };

                    let method_name = method.name.node.to_string();

                    if method.default.is_some() {
                        trait_def.default_methods.insert(method_name, method_type);
                    } else {
                        trait_def.methods.insert(method_name, method_type);
                    }
                }
                ast::TraitMember::Const(const_decl) => {
                    let const_type = self.resolve_type(&const_decl.ty)?;
                    let const_name = const_decl.name.node.to_string();
                    trait_def.associated_consts.insert(const_name, const_type);
                }
                ast::TraitMember::Type(type_decl) => {
                    let type_name = type_decl.name.node.to_string();
                    if let Some(ref default_ty) = type_decl.default {
                        let resolved = self.resolve_type(default_ty)?;
                        trait_def.default_associated_types.insert(type_name.clone(), resolved);
                    }
                    trait_def.associated_types.push(type_name);
                }
            }
        }

        self.register_trait(trait_def);
        Ok(())
    }

    /// Check an impl block
    fn check_impl(&mut self, impl_decl: &ast::ImplDecl) -> TypeResult<()> {
        let for_type = self.resolve_type(&impl_decl.for_type)?;

        if let Some(ref trait_bound) = impl_decl.trait_ {
            self.check_trait_impl(impl_decl, trait_bound, &for_type)
        } else {
            self.check_inherent_impl(impl_decl, &for_type)
        }
    }

    /// Check a trait implementation (`impl Trait for Type`)
    fn check_trait_impl(
        &mut self,
        impl_decl: &ast::ImplDecl,
        trait_bound: &ast::TraitBound,
        for_type: &Type,
    ) -> TypeResult<()> {
        let trait_name = trait_bound.path.iter()
            .map(|s| s.node.to_string())
            .collect::<Vec<_>>()
            .join("::");

        let trait_def = self.trait_defs.get(&trait_name).cloned();
        let trait_def = match trait_def {
            Some(def) => def,
            None => return Err(TypeError::UndefinedTrait(trait_name.clone(), trait_bound.span)),
        };

        // Check supertraits are implemented
        for supertrait in &trait_def.supertraits {
            if !self.implements_trait(for_type, &supertrait.trait_name) {
                return Err(TypeError::SupertraitNotImplemented {
                    trait_name: trait_name.clone(),
                    supertrait_name: supertrait.trait_name.clone(),
                    for_type: format!("{}", for_type),
                    span: impl_decl.span,
                });
            }
        }

        let mut impl_methods: FxHashMap<String, Type> = FxHashMap::default();
        let mut impl_types: FxHashMap<String, Type> = FxHashMap::default();
        let mut impl_consts: FxHashMap<String, Type> = FxHashMap::default();
        let self_type = for_type.clone();

        for member in &impl_decl.members {
            match member {
                ast::ImplMember::Function(func) => {
                    let method_name = func.name.node.to_string();

                    if impl_methods.contains_key(&method_name) {
                        return Err(TypeError::DuplicateImplMethod {
                            method_name: method_name.clone(),
                            span: func.span,
                        });
                    }

                    let trait_method_type = trait_def.get_method(&method_name);
                    if trait_method_type.is_none() && !trait_def.default_methods.contains_key(&method_name) {
                        return Err(TypeError::MethodNotInTrait {
                            trait_name: trait_name.clone(),
                            method_name: method_name.clone(),
                            span: func.span,
                        });
                    }

                    let param_types: Vec<Type> = func.params.iter()
                        .map(|p| {
                            if let Some(ref ty) = p.ty {
                                self.resolve_type(ty)
                            } else {
                                Ok(self_type.clone())
                            }
                        })
                        .collect::<TypeResult<Vec<_>>>()?;

                    let return_type = if let Some(ref ty) = func.return_type {
                        self.resolve_type(ty)?
                    } else {
                        Type::Unit
                    };

                    let impl_method_type = Type::Function {
                        params: param_types,
                        return_type: Box::new(return_type),
                    };

                    if let Some(expected_type) = trait_method_type {
                        let expected_substituted = self.substitute_self_type(expected_type, for_type);
                        if !self.method_signatures_compatible(&expected_substituted, &impl_method_type) {
                            return Err(TypeError::TraitMethodSignatureMismatch {
                                method_name: method_name.clone(),
                                expected: format!("{}", expected_substituted),
                                found: format!("{}", impl_method_type),
                                span: func.span,
                            });
                        }
                    }

                    impl_methods.insert(method_name.clone(), impl_method_type);
                    self.check_function(func)?;
                }
                ast::ImplMember::Type(type_alias) => {
                    let type_name = type_alias.name.node.to_string();

                    if impl_types.contains_key(&type_name) {
                        return Err(TypeError::DuplicateAssociatedType {
                            type_name: type_name.clone(),
                            span: type_alias.span,
                        });
                    }

                    if !trait_def.has_associated_type(&type_name) {
                        return Err(TypeError::AssociatedTypeNotInTrait {
                            trait_name: trait_name.clone(),
                            type_name: type_name.clone(),
                            span: type_alias.span,
                        });
                    }

                    let resolved_type = self.resolve_type(&type_alias.ty)?;
                    impl_types.insert(type_name, resolved_type);
                }
                ast::ImplMember::Const(const_decl) => {
                    let const_name = const_decl.name.node.to_string();

                    if let Some(expected_type) = trait_def.associated_consts.get(&const_name) {
                        let const_type = if let Some(ref ty) = const_decl.ty {
                            self.resolve_type(ty)?
                        } else {
                            expected_type.clone()
                        };

                        if !self.types_match(&const_type, expected_type) {
                            return Err(TypeError::Mismatch {
                                expected: format!("{}", expected_type),
                                found: format!("{}", const_type),
                                span: const_decl.span,
                                expected_source: None,
                            });
                        }

                        impl_consts.insert(const_name, const_type);
                    }
                }
            }
        }

        // Check that all required methods are implemented
        for (method_name, _) in trait_def.required_methods() {
            if !impl_methods.contains_key(method_name) {
                return Err(TypeError::MissingTraitMethod {
                    trait_name: trait_name.clone(),
                    method_name: method_name.clone(),
                    span: impl_decl.span,
                });
            }
        }

        // Check that all required associated types are defined
        for type_name in trait_def.required_associated_types() {
            if !impl_types.contains_key(type_name) {
                return Err(TypeError::MissingAssociatedType {
                    trait_name: trait_name.clone(),
                    type_name: type_name.clone(),
                    span: impl_decl.span,
                });
            }
        }

        let mut trait_impl = TraitImpl::new(trait_name.clone(), for_type.clone());
        trait_impl.methods = impl_methods;
        trait_impl.associated_types = impl_types;
        trait_impl.associated_consts = impl_consts;

        self.register_trait_impl(trait_impl)?;
        Ok(())
    }

    /// Check an inherent implementation (`impl Type`)
    fn check_inherent_impl(
        &mut self,
        impl_decl: &ast::ImplDecl,
        _for_type: &Type,
    ) -> TypeResult<()> {
        for member in &impl_decl.members {
            match member {
                ast::ImplMember::Function(func) => {
                    self.check_function(func)?;
                }
                ast::ImplMember::Type(type_alias) => {
                    self.resolve_type(&type_alias.ty)?;
                }
                ast::ImplMember::Const(const_decl) => {
                    if let Some(ref ty) = const_decl.ty {
                        self.resolve_type(ty)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Substitute Self type in a type expression
    fn substitute_self_type(&self, ty: &Type, self_type: &Type) -> Type {
        match ty {
            Type::Named { name, .. } if name == "Self" => self_type.clone(),
            Type::Named { name, type_args } => {
                let new_args = type_args.iter()
                    .map(|arg| self.substitute_self_type(arg, self_type))
                    .collect();
                Type::Named { name: name.clone(), type_args: new_args }
            }
            Type::Array(elem) => Type::Array(Box::new(self.substitute_self_type(elem, self_type))),
            Type::Optional(inner) => Type::Optional(Box::new(self.substitute_self_type(inner, self_type))),
            Type::Result(ok, err) => Type::Result(
                Box::new(self.substitute_self_type(ok, self_type)),
                Box::new(self.substitute_self_type(err, self_type)),
            ),
            Type::Tuple(elems) => Type::Tuple(elems.iter()
                .map(|e| self.substitute_self_type(e, self_type))
                .collect()),
            Type::Map(k, v) => Type::Map(
                Box::new(self.substitute_self_type(k, self_type)),
                Box::new(self.substitute_self_type(v, self_type)),
            ),
            Type::Reference { mutable, inner } => Type::Reference {
                mutable: *mutable,
                inner: Box::new(self.substitute_self_type(inner, self_type)),
            },
            Type::Function { params, return_type } => Type::Function {
                params: params.iter()
                    .map(|p| self.substitute_self_type(p, self_type))
                    .collect(),
                return_type: Box::new(self.substitute_self_type(return_type, self_type)),
            },
            _ => ty.clone(),
        }
    }

    /// Check if two method signatures are compatible
    fn method_signatures_compatible(&self, expected: &Type, found: &Type) -> bool {
        match (expected, found) {
            (
                Type::Function { params: exp_params, return_type: exp_ret },
                Type::Function { params: found_params, return_type: found_ret },
            ) => {
                if exp_params.len() != found_params.len() {
                    return false;
                }
                for (exp_param, found_param) in exp_params.iter().zip(found_params.iter()) {
                    if !self.types_match(exp_param, found_param) {
                        return false;
                    }
                }
                self.types_match(exp_ret, found_ret)
            }
            _ => false,
        }
    }

    // ============================================================================
    // FFI / Extern Type Checking
    // ============================================================================

    /// Check an extern declaration for FFI safety
    ///
    /// This validates:
    /// - All C types are valid and FFI-safe
    /// - Parameter types are C-compatible
    /// - Return types are C-compatible
    /// - Extern struct fields are C-compatible
    fn check_extern(&mut self, extern_decl: &ast::ExternDecl) -> TypeResult<()> {
        match extern_decl {
            ast::ExternDecl::C(extern_c) => self.check_extern_c(extern_c),
            ast::ExternDecl::Python(extern_python) => self.check_extern_python(extern_python),
            ast::ExternDecl::Wasm(extern_wasm) => self.check_extern_wasm(extern_wasm),
        }
    }

    /// Check a C extern block
    fn check_extern_c(&mut self, extern_c: &ast::ExternC) -> TypeResult<()> {
        for item in &extern_c.items {
            match item {
                ast::ExternCItem::Function(func) => {
                    self.check_extern_c_function(func)?;
                }
                ast::ExternCItem::Struct(s) => {
                    self.check_extern_c_struct(s)?;
                }
                ast::ExternCItem::Const(c) => {
                    self.check_extern_c_const(c)?;
                }
                ast::ExternCItem::Type(_) => {
                    // Type declarations are opaque - no validation needed
                }
            }
        }
        Ok(())
    }

    /// Check a C extern function declaration
    fn check_extern_c_function(&mut self, func: &ast::ExternFunction) -> TypeResult<()> {
        let func_name = func.name.node.to_string();

        // Check all parameter types
        for (i, param) in func.params.iter().enumerate() {
            self.validate_c_type(&param.ty, func.span)?;

            // Check for non-FFI-safe patterns
            if !self.is_c_type_ffi_safe(&param.ty) {
                let param_name = param.name
                    .as_ref()
                    .map(|n| n.node.to_string())
                    .unwrap_or_else(|| format!("param{}", i));
                return Err(TypeError::NonFfiSafeParameter {
                    func_name,
                    param_name,
                    span: func.span,
                });
            }
        }

        // Check return type if present
        if let Some(return_ty) = &func.return_type {
            self.validate_c_type(return_ty, func.span)?;

            if !self.is_c_type_ffi_safe(return_ty) {
                return Err(TypeError::NonFfiSafeReturn {
                    func_name,
                    span: func.span,
                });
            }
        }

        // Register the extern function in the type environment
        let param_types: Vec<Type> = func.params
            .iter()
            .map(|p| self.c_type_to_aria_type(&p.ty))
            .collect();

        let return_type = func.return_type
            .as_ref()
            .map(|t| self.c_type_to_aria_type(t))
            .unwrap_or(Type::Unit);

        let func_type = Type::Function {
            params: param_types,
            return_type: Box::new(return_type),
        };

        Rc::make_mut(&mut self.env).define_var(func_name, func_type);

        Ok(())
    }

    /// Check a C extern struct declaration
    fn check_extern_c_struct(&mut self, s: &ast::ExternStruct) -> TypeResult<()> {
        let struct_name = s.name.node.to_string();

        // Validate all field types
        for field in &s.fields {
            self.validate_c_type(&field.ty, s.span)?;

            if !self.is_c_type_ffi_safe(&field.ty) {
                return Err(TypeError::NonFfiSafeField {
                    struct_name,
                    field_name: field.name.node.to_string(),
                    span: s.span,
                });
            }
        }

        // Register the extern struct as a named type
        let struct_type = Type::Named {
            name: struct_name.clone(),
            type_args: vec![],
        };
        Rc::make_mut(&mut self.env).define_type(struct_name.clone(), TypeScheme::mono(struct_type));

        // Register struct fields
        let fields: Vec<(String, Type)> = s.fields
            .iter()
            .map(|f| (f.name.node.to_string(), self.c_type_to_aria_type(&f.ty)))
            .collect();
        self.struct_fields.insert(struct_name, fields);

        Ok(())
    }

    /// Check a C extern constant declaration
    fn check_extern_c_const(&mut self, c: &ast::ExternConst) -> TypeResult<()> {
        self.validate_c_type(&c.ty, c.span)?;

        if !self.is_c_type_ffi_safe(&c.ty) {
            return Err(TypeError::InvalidFfiCType {
                c_type: format!("{:?}", c.ty),
                reason: "constant type must be FFI-safe".to_string(),
                span: c.span,
            });
        }

        // Register the constant
        let const_type = self.c_type_to_aria_type(&c.ty);
        Rc::make_mut(&mut self.env).define_var(c.name.node.to_string(), const_type);

        Ok(())
    }

    /// Validate that a C type is valid
    fn validate_c_type(&self, c_type: &ast::CType, span: Span) -> TypeResult<()> {
        match c_type {
            ast::CType::Int
            | ast::CType::UInt
            | ast::CType::Long
            | ast::CType::ULong
            | ast::CType::LongLong
            | ast::CType::Float
            | ast::CType::Double
            | ast::CType::Char
            | ast::CType::Void
            | ast::CType::SizeT
            | ast::CType::SSizeT => Ok(()),

            ast::CType::Pointer { pointee, .. } => {
                // Recursively validate pointee type
                self.validate_c_type(pointee, span)
            }

            ast::CType::Named(type_ident) => {
                // Named types should be previously declared extern types
                // For now, allow them as they may reference opaque types
                let _ = type_ident;
                Ok(())
            }
        }
    }

    /// Check if a C type is FFI-safe
    ///
    /// FFI-safe types are:
    /// - Primitive C types (int, long, float, etc.)
    /// - Pointers to FFI-safe types
    /// - void (for return types)
    /// - Named types (assumed to be extern struct/union)
    fn is_c_type_ffi_safe(&self, c_type: &ast::CType) -> bool {
        match c_type {
            // All primitive C types are FFI-safe
            ast::CType::Int
            | ast::CType::UInt
            | ast::CType::Long
            | ast::CType::ULong
            | ast::CType::LongLong
            | ast::CType::Float
            | ast::CType::Double
            | ast::CType::Char
            | ast::CType::Void
            | ast::CType::SizeT
            | ast::CType::SSizeT => true,

            // Pointers are FFI-safe if pointee is FFI-safe
            ast::CType::Pointer { pointee, .. } => {
                self.is_c_type_ffi_safe(pointee)
            }

            // Named types are assumed FFI-safe (extern struct/union)
            ast::CType::Named(_) => true,
        }
    }

    /// Convert a C type to an Aria type for type checking
    fn c_type_to_aria_type(&self, c_type: &ast::CType) -> Type {
        match c_type {
            ast::CType::Int => Type::Int32,
            ast::CType::UInt => Type::UInt32,
            ast::CType::Long => Type::Int64,
            ast::CType::ULong => Type::UInt64,
            ast::CType::LongLong => Type::Int64, // Typically 64-bit
            ast::CType::Float => Type::Float32,
            ast::CType::Double => Type::Float64,
            ast::CType::Char => Type::Int8, // C char is typically signed byte
            ast::CType::Void => Type::Unit,
            ast::CType::SizeT => Type::UInt64, // size_t is typically 64-bit
            ast::CType::SSizeT => Type::Int64, // ssize_t is typically 64-bit

            ast::CType::Pointer { pointee, .. } => {
                // Map pointers to references
                // void* becomes &UInt8 (raw bytes)
                // T* becomes &T
                let inner = if matches!(**pointee, ast::CType::Void) {
                    Type::UInt8
                } else {
                    self.c_type_to_aria_type(pointee)
                };
                Type::Reference {
                    mutable: true, // Assume mutable by default for C pointers
                    inner: Box::new(inner),
                }
            }

            ast::CType::Named(type_ident) => {
                Type::Named {
                    name: type_ident.node.to_string(),
                    type_args: vec![],
                }
            }
        }
    }

    /// Check a Python extern declaration
    fn check_extern_python(&mut self, _extern_python: &ast::ExternPython) -> TypeResult<()> {
        // Python FFI is more permissive - types are checked at runtime
        // For now, just accept all Python extern declarations
        Ok(())
    }

    /// Check a WASM extern declaration
    fn check_extern_wasm(&mut self, extern_wasm: &ast::ExternWasm) -> TypeResult<()> {
        // Validate WASM extern functions
        for func in &extern_wasm.items {
            self.check_extern_c_function(func)?;
        }
        Ok(())
    }

    /// Resolve an AST type to internal type representation
    fn resolve_type(&self, ty: &ast::TypeExpr) -> TypeResult<Type> {
        match ty {
            ast::TypeExpr::Named(name) => {
                let name_str = name.node.as_str();
                // Check if this is a type parameter first
                if let Some(var) = self.lookup_type_param(name_str) {
                    return Ok(Type::Var(var));
                }
                // Fast path: check static primitive lookup table
                if let Some(prim_type) = primitive_type_lookup().get(name_str) {
                    return Ok(prim_type.clone());
                }
                // Look up in environment
                let name_owned = name_str.to_string();
                if let Some(scheme) = self.env.lookup_type(&name_owned) {
                    Ok(scheme.ty.clone())
                } else {
                    Ok(Type::Named {
                        name: name_owned,
                        type_args: Vec::new(),
                    })
                }
            }
            ast::TypeExpr::Generic { name, args, span } => {
                let resolved_args: Vec<Type> = args
                    .iter()
                    .map(|a| self.resolve_type(a))
                    .collect::<TypeResult<Vec<_>>>()?;

                let name_str = name.node.to_string();
                match name_str.as_str() {
                    "Array" if resolved_args.len() == 1 => {
                        Ok(Type::Array(Box::new(resolved_args[0].clone())))
                    }
                    "Map" if resolved_args.len() == 2 => Ok(Type::Map(
                        Box::new(resolved_args[0].clone()),
                        Box::new(resolved_args[1].clone()),
                    )),
                    "Result" if resolved_args.len() == 2 => Ok(Type::Result(
                        Box::new(resolved_args[0].clone()),
                        Box::new(resolved_args[1].clone()),
                    )),
                    _ => {
                        // Check if this is a type alias with type parameters
                        if let Some(scheme) = self.env.lookup_type(&name_str) {
                            if !scheme.type_params.is_empty() {
                                // This is a generic type alias - expand it with substitution
                                return self.expand_generic_type_alias(scheme, &resolved_args, *span);
                            }
                        }

                        // Validate type arguments against bounds if we have type parameter definitions
                        if let Some(type_param_defs) = self.generic_type_params.get(&name_str) {
                            self.validate_type_args(type_param_defs, &resolved_args, *span)?;
                        }

                        Ok(Type::Named {
                            name: name_str,
                            type_args: resolved_args,
                        })
                    }
                }
            }
            ast::TypeExpr::Array { element, size, .. } => {
                let elem_type = self.resolve_type(element)?;
                if let Some(_size_expr) = size {
                    // TODO: Evaluate size expression
                    Ok(Type::FixedArray(Box::new(elem_type), 0))
                } else {
                    Ok(Type::Array(Box::new(elem_type)))
                }
            }
            ast::TypeExpr::Map { key, value, .. } => {
                let key_type = self.resolve_type(key)?;
                let value_type = self.resolve_type(value)?;
                Ok(Type::Map(Box::new(key_type), Box::new(value_type)))
            }
            ast::TypeExpr::Tuple { elements, .. } => {
                let elem_types: Vec<Type> = elements
                    .iter()
                    .map(|e| self.resolve_type(e))
                    .collect::<TypeResult<Vec<_>>>()?;
                Ok(Type::Tuple(elem_types))
            }
            ast::TypeExpr::Optional { inner, .. } => {
                let inner_type = self.resolve_type(inner)?;
                Ok(Type::Optional(Box::new(inner_type)))
            }
            ast::TypeExpr::Result { ok, err, .. } => {
                let ok_type = self.resolve_type(ok)?;
                let err_type = if let Some(e) = err {
                    self.resolve_type(e)?
                } else {
                    Type::Named {
                        name: "Error".to_string(),
                        type_args: Vec::new(),
                    }
                };
                Ok(Type::Result(Box::new(ok_type), Box::new(err_type)))
            }
            ast::TypeExpr::Reference { mutable, inner, .. } => {
                let inner_type = self.resolve_type(inner)?;
                Ok(Type::Reference {
                    mutable: *mutable,
                    inner: Box::new(inner_type),
                })
            }
            ast::TypeExpr::Function {
                params,
                return_type,
                ..
            } => {
                let param_types: Vec<Type> = params
                    .iter()
                    .map(|p| self.resolve_type(p))
                    .collect::<TypeResult<Vec<_>>>()?;
                let ret_type = if let Some(r) = return_type {
                    self.resolve_type(r)?
                } else {
                    Type::Unit
                };
                Ok(Type::Function {
                    params: param_types,
                    return_type: Box::new(ret_type),
                })
            }
            ast::TypeExpr::Path { segments, .. } => {
                // Build qualified name
                let name = segments
                    .iter()
                    .map(|s| s.node.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                Ok(Type::Named {
                    name,
                    type_args: Vec::new(),
                })
            }
            ast::TypeExpr::Inferred(span) => {
                // Return error or fresh type variable
                Err(TypeError::CannotInfer(*span))
            }
        }
    }

    // ========================================================================
    // Expression Type Inference
    // ========================================================================

    /// Infer the type of an expression
    pub fn infer_expr(&mut self, expr: &ast::Expr, env: &TypeEnv) -> TypeResult<Type> {
        match &expr.kind {
            // Literals
            ast::ExprKind::Integer(_) => Ok(Type::Int),
            ast::ExprKind::Float(_) => Ok(Type::Float),
            ast::ExprKind::String(_) => Ok(Type::String),
            ast::ExprKind::InterpolatedString(parts) => {
                // Type check each interpolated part
                for part in parts {
                    match part {
                        ast::StringPart::Literal(_) => {
                            // Literal strings are always valid
                        }
                        ast::StringPart::Expr(inner_expr) => {
                            // Type check the expression and verify it implements Display
                            let expr_type = self.infer_expr(inner_expr, env)?;
                            let resolved = self.inference.apply(&expr_type);
                            // Verify the type can be converted to string (implements Display)
                            if !self.can_convert_to_string(&resolved) {
                                return Err(TypeError::TraitNotImplemented {
                                    ty: format!("{}", resolved),
                                    trait_name: "Display".to_string(),
                                    span: inner_expr.span,
                                });
                            }
                        }
                        ast::StringPart::FormattedExpr { expr: inner_expr, format: _ } => {
                            // Type check the expression
                            let expr_type = self.infer_expr(inner_expr, env)?;
                            let resolved = self.inference.apply(&expr_type);
                            // Verify the type can be converted to string (implements Display)
                            if !self.can_convert_to_string(&resolved) {
                                return Err(TypeError::TraitNotImplemented {
                                    ty: format!("{}", resolved),
                                    trait_name: "Display".to_string(),
                                    span: inner_expr.span,
                                });
                            }
                            // TODO: Validate format specifier against the type
                            // e.g., numeric formats like "02d" should only work with numeric types
                        }
                    }
                }
                Ok(Type::String)
            }
            ast::ExprKind::Char(_) => Ok(Type::Char),
            ast::ExprKind::Bool(_) => Ok(Type::Bool),
            ast::ExprKind::Nil => Ok(Type::Optional(Box::new(self.inference.fresh_var()))),

            // Identifiers
            ast::ExprKind::Ident(name) => {
                let name_str = name.to_string();
                env.lookup_var(&name_str)
                    .cloned()
                    .or_else(|| self.env.lookup_var(&name_str).cloned())
                    .ok_or_else(|| TypeError::UndefinedVariable {
                        name: name_str,
                        span: expr.span,
                        similar_names: None, // TODO: Collect similar names from env
                    })
            }

            ast::ExprKind::SelfLower => {
                // self refers to the current instance - type depends on context
                env.lookup_var("self")
                    .cloned()
                    .ok_or_else(|| TypeError::UndefinedVariable {
                        name: "self".to_string(),
                        span: expr.span,
                        similar_names: None,
                    })
            }

            ast::ExprKind::SelfUpper => {
                // Self refers to the current type
                env.lookup_var("Self")
                    .cloned()
                    .ok_or_else(|| TypeError::UndefinedVariable {
                        name: "Self".to_string(),
                        span: expr.span,
                        similar_names: None,
                    })
            }

            // Collections
            ast::ExprKind::Array(elements) => {
                if elements.is_empty() {
                    Ok(Type::Array(Box::new(self.inference.fresh_var())))
                } else {
                    let elem_type = self.infer_expr(&elements[0], env)?;
                    for elem in &elements[1..] {
                        let t = self.infer_expr(elem, env)?;
                        self.inference.unify(&elem_type, &t, elem.span)?;
                    }
                    Ok(Type::Array(Box::new(self.inference.apply(&elem_type))))
                }
            }

            ast::ExprKind::Tuple(elements) => {
                let types: Vec<Type> = elements
                    .iter()
                    .map(|e| self.infer_expr(e, env))
                    .collect::<TypeResult<Vec<_>>>()?;
                Ok(Type::Tuple(types))
            }

            ast::ExprKind::Map(pairs) => {
                if pairs.is_empty() {
                    Ok(Type::Map(
                        Box::new(self.inference.fresh_var()),
                        Box::new(self.inference.fresh_var()),
                    ))
                } else {
                    let key_type = self.infer_expr(&pairs[0].0, env)?;
                    let val_type = self.infer_expr(&pairs[0].1, env)?;
                    for (k, v) in &pairs[1..] {
                        let kt = self.infer_expr(k, env)?;
                        let vt = self.infer_expr(v, env)?;
                        self.inference.unify(&key_type, &kt, k.span)?;
                        self.inference.unify(&val_type, &vt, v.span)?;
                    }
                    Ok(Type::Map(
                        Box::new(self.inference.apply(&key_type)),
                        Box::new(self.inference.apply(&val_type)),
                    ))
                }
            }

            // Struct initialization
            ast::ExprKind::StructInit { name, fields } => {
                let struct_name = name.node.to_string();

                // Look up the struct fields
                if let Some(expected_fields) = self.struct_fields.get(&struct_name).cloned() {
                    // Check each provided field
                    for field_init in fields {
                        // Handle spread fields: `Point { ...other, x: 10 }`
                        if field_init.spread {
                            // Spread field: value must be the same struct type
                            if let Some(ref value) = field_init.value {
                                let spread_type = self.infer_expr(value, env)?;
                                let resolved_spread_type = self.inference.apply(&spread_type);

                                // Check that spread source is compatible with target struct
                                match &resolved_spread_type {
                                    Type::Named { name: spread_name, .. } => {
                                        if spread_name != &struct_name {
                                            return Err(TypeError::SpreadStructTypeMismatch {
                                                source_type: spread_name.clone(),
                                                target_struct: struct_name.clone(),
                                                span: value.span,
                                            });
                                        }
                                        // Same struct type - spread is valid
                                    }
                                    Type::Var(_) => {
                                        // Type variable - unify with target struct type
                                        let expected_type = Type::Named {
                                            name: struct_name.clone(),
                                            type_args: vec![],
                                        };
                                        self.inference.unify(&spread_type, &expected_type, value.span)?;
                                    }
                                    _ => {
                                        return Err(TypeError::SpreadOnNonStruct {
                                            found: format!("{}", resolved_spread_type),
                                            span: value.span,
                                        });
                                    }
                                }
                            }
                            continue;
                        }

                        let field_name = field_init.name.node.to_string();

                        // Find the expected field type
                        let expected_type = expected_fields
                            .iter()
                            .find(|(n, _)| n == &field_name)
                            .map(|(_, ty)| ty.clone());

                        if let Some(expected_ty) = expected_type {
                            // Type check the field value
                            if let Some(ref value) = field_init.value {
                                let actual_type = self.infer_expr(value, env)?;
                                self.inference.unify(&expected_ty, &actual_type, value.span)?;
                            }
                            // If no value provided (shorthand), look up the variable
                            else {
                                let var_type = env.lookup_var(&field_name)
                                    .cloned()
                                    .or_else(|| self.env.lookup_var(&field_name).cloned())
                                    .ok_or_else(|| TypeError::UndefinedVariable {
                                        name: field_name.clone(),
                                        span: field_init.name.span,
                                        similar_names: None,
                                    })?;
                                self.inference.unify(&expected_ty, &var_type, field_init.name.span)?;
                            }
                        } else {
                            return Err(TypeError::UndefinedField {
                                type_name: struct_name.clone(),
                                field_name,
                                span: field_init.name.span,
                            });
                        }
                    }

                    // Return the struct type
                    Ok(Type::Named {
                        name: struct_name,
                        type_args: vec![],
                    })
                } else {
                    // Struct not found - check if it's a type we know about
                    Err(TypeError::UndefinedType(struct_name, name.span))
                }
            }

            // Binary operators
            ast::ExprKind::Binary { op, left, right } => {
                let left_type = self.infer_expr(left, env)?;
                let right_type = self.infer_expr(right, env)?;
                self.infer_binary_op(*op, &left_type, &right_type, expr.span)
            }

            // Unary operators
            ast::ExprKind::Unary { op, operand } => {
                let operand_type = self.infer_expr(operand, env)?;
                self.infer_unary_op(*op, &operand_type, expr.span)
            }

            // Function call
            ast::ExprKind::Call { func, args } => {
                // First, check if this is a call to a generic function
                let func_name = self.extract_func_name(func);

                // Try to handle as a generic function call
                if let Some(ref name) = func_name {
                    if let Some(generic_info) = self.generic_functions.get(name).cloned() {
                        // This is a generic function - use type argument inference
                        let inferred_types = self.infer_type_arguments(&generic_info, args, env, expr.span)?;

                        // Instantiate the return type with the inferred types
                        let return_type = self.instantiate_return_type(&generic_info, &inferred_types);
                        return Ok(return_type);
                    }
                }

                let func_type = self.infer_expr(func, env)?;
                match self.inference.apply(&func_type) {
                    Type::Function { params, return_type } => {
                        // Check if any argument uses spread
                        let has_spread = args.iter().any(|a| a.spread);

                        if has_spread {
                            // With spread, we need to expand array arguments
                            // For now, we support a single spread argument at the end
                            let mut param_idx = 0;
                            for arg in args.iter() {
                                let arg_type = self.infer_expr(&arg.value, env)?;
                                let resolved_arg_type = self.inference.apply(&arg_type);

                                if arg.spread {
                                    // Spread argument: must be an array type
                                    match &resolved_arg_type {
                                        Type::Array(elem_type) => {
                                            // All remaining params must match the element type
                                            while param_idx < params.len() {
                                                self.inference.unify(elem_type, &params[param_idx], arg.value.span)
                                                    .map_err(|_| TypeError::SpreadElementTypeMismatch {
                                                        spread_elem_type: format!("{}", elem_type),
                                                        param_type: format!("{}", params[param_idx]),
                                                        span: arg.value.span,
                                                    })?;
                                                param_idx += 1;
                                            }
                                        }
                                        Type::Var(_) => {
                                            // Unknown type - create fresh element type
                                            let elem_type = self.inference.fresh_var();
                                            let array_type = Type::Array(Box::new(elem_type.clone()));
                                            self.inference.unify(&arg_type, &array_type, arg.value.span)?;
                                            // Remaining params should match element type
                                            while param_idx < params.len() {
                                                self.inference.unify(&elem_type, &params[param_idx], arg.value.span)?;
                                                param_idx += 1;
                                            }
                                        }
                                        _ => {
                                            return Err(TypeError::SpreadOnNonArray {
                                                found: format!("{}", resolved_arg_type),
                                                span: arg.value.span,
                                            });
                                        }
                                    }
                                } else {
                                    // Regular argument
                                    if param_idx >= params.len() {
                                        return Err(TypeError::TooManyArguments {
                                            max_allowed: params.len(),
                                            found: args.len(),
                                            span: expr.span,
                                        });
                                    }
                                    self.inference.unify(&arg_type, &params[param_idx], arg.value.span)?;
                                    param_idx += 1;
                                }
                            }
                            // Note: With spread, we can't easily verify all params are covered
                            // The spread array could have exactly the right number of elements
                        } else {
                            // No spread - check with default parameter support
                            // Try to get function name for signature lookup
                            let func_name = self.extract_func_name(func);

                            // Check if we have signature info with defaults
                            if let Some(ref sig) = func_name.and_then(|n| self.function_signatures.get(&n).cloned()) {
                                // Use enhanced call checking with default/named argument support
                                self.check_call_with_signature(args, sig, env, expr.span)?;
                            } else {
                                // No signature info - fall back to standard checking
                                // Check for named arguments
                                let has_named = args.iter().any(|a| a.name.is_some());
                                if has_named {
                                    // Named arguments require signature info
                                    // For now, just check positional arguments in order
                                    let mut seen_named = false;
                                    for (i, arg) in args.iter().enumerate() {
                                        if arg.name.is_some() {
                                            seen_named = true;
                                        } else if seen_named {
                                            return Err(TypeError::PositionalAfterNamed {
                                                span: arg.value.span,
                                            });
                                        }

                                        if i < params.len() {
                                            let arg_type = self.infer_expr(&arg.value, env)?;
                                            self.inference.unify(&arg_type, &params[i], arg.value.span)?;
                                        }
                                    }
                                    // Check argument count
                                    if args.len() < params.len() {
                                        return Err(TypeError::TooFewArguments {
                                            min_required: params.len(),
                                            found: args.len(),
                                            span: expr.span,
                                        });
                                    } else if args.len() > params.len() {
                                        return Err(TypeError::TooManyArguments {
                                            max_allowed: params.len(),
                                            found: args.len(),
                                            span: expr.span,
                                        });
                                    }
                                } else {
                                    // Pure positional - standard check
                                    if args.len() != params.len() {
                                        return Err(TypeError::WrongTypeArity {
                                            expected: params.len(),
                                            found: args.len(),
                                            span: expr.span,
                                        });
                                    }
                                    for (arg, param_type) in args.iter().zip(params.iter()) {
                                        let arg_type = self.infer_expr(&arg.value, env)?;
                                        self.inference.unify(&arg_type, param_type, arg.value.span)?;
                                    }
                                }
                            }
                        }
                        Ok(*return_type)
                    }
                    Type::Var(_) => {
                        // Unknown function type - create constraints
                        // With spread, we can't determine exact param types
                        let has_spread = args.iter().any(|a| a.spread);
                        if has_spread {
                            // For spread with unknown function, infer what we can
                            let mut arg_types = Vec::new();
                            for arg in args.iter() {
                                let arg_type = self.infer_expr(&arg.value, env)?;
                                if arg.spread {
                                    // Spread expands to unknown number of elements
                                    let resolved = self.inference.apply(&arg_type);
                                    match &resolved {
                                        Type::Array(elem_type) => {
                                            // We don't know how many elements, so we can't build param types
                                            // For now, just add the element type once
                                            arg_types.push(elem_type.as_ref().clone());
                                        }
                                        Type::Var(_) => {
                                            let elem_type = self.inference.fresh_var();
                                            let array_type = Type::Array(Box::new(elem_type.clone()));
                                            self.inference.unify(&arg_type, &array_type, arg.value.span)?;
                                            arg_types.push(elem_type);
                                        }
                                        _ => {
                                            return Err(TypeError::SpreadOnNonArray {
                                                found: format!("{}", resolved),
                                                span: arg.value.span,
                                            });
                                        }
                                    }
                                } else {
                                    arg_types.push(arg_type);
                                }
                            }
                            let return_type = self.inference.fresh_var();
                            let expected_func_type = Type::Function {
                                params: arg_types,
                                return_type: Box::new(return_type.clone()),
                            };
                            self.inference.unify(&func_type, &expected_func_type, expr.span)?;
                            Ok(return_type)
                        } else {
                            let arg_types: Vec<Type> = args
                                .iter()
                                .map(|a| self.infer_expr(&a.value, env))
                                .collect::<TypeResult<Vec<_>>>()?;
                            let return_type = self.inference.fresh_var();
                            let expected_func_type = Type::Function {
                                params: arg_types,
                                return_type: Box::new(return_type.clone()),
                            };
                            self.inference.unify(&func_type, &expected_func_type, expr.span)?;
                            Ok(return_type)
                        }
                    }
                    _ => Err(TypeError::Mismatch {
                        expected: "function".to_string(),
                        found: format!("{}", func_type),
                        span: func.span,
                        expected_source: None,
                    }),
                }
            }

            // If expression
            ast::ExprKind::If {
                condition,
                then_branch,
                elsif_branches,
                else_branch,
            } => {
                let cond_type = self.infer_expr(condition, env)?;
                self.inference.unify(&cond_type, &Type::Bool, condition.span)?;

                let then_type = self.infer_block(then_branch, env)?;

                for (elsif_cond, elsif_body) in elsif_branches {
                    let elsif_cond_type = self.infer_expr(elsif_cond, env)?;
                    self.inference.unify(&elsif_cond_type, &Type::Bool, elsif_cond.span)?;
                    let elsif_type = self.infer_block(elsif_body, env)?;
                    self.inference.unify(&then_type, &elsif_type, elsif_body.span)?;
                }

                if let Some(else_body) = else_branch {
                    let else_type = self.infer_block(else_body, env)?;
                    self.inference.unify(&then_type, &else_type, else_body.span)?;
                }

                Ok(self.inference.apply(&then_type))
            }

            // Block expression
            ast::ExprKind::Block(block) => self.infer_block(block, env),

            // Parenthesized expression
            ast::ExprKind::Paren(inner) => self.infer_expr(inner, env),

            // Ternary expression
            ast::ExprKind::Ternary {
                condition,
                then_expr,
                else_expr,
            } => {
                let cond_type = self.infer_expr(condition, env)?;
                self.inference.unify(&cond_type, &Type::Bool, condition.span)?;

                let then_type = self.infer_expr(then_expr, env)?;
                let else_type = self.infer_expr(else_expr, env)?;
                self.inference.unify(&then_type, &else_type, else_expr.span)?;

                Ok(self.inference.apply(&then_type))
            }

            // Field access
            ast::ExprKind::Field { object, field } => {
                let obj_type = self.infer_expr(object, env)?;
                let resolved_type = self.inference.apply(&obj_type);
                let field_name = field.node.to_string();

                match &resolved_type {
                    Type::Named { name: type_name, .. } => {
                        // Look up struct fields
                        if let Some(fields) = self.struct_fields.get(type_name) {
                            // Find the field with matching name
                            for (name, ty) in fields {
                                if name == &field_name {
                                    return Ok(ty.clone());
                                }
                            }
                            // Field not found in struct
                            Err(TypeError::UndefinedField {
                                type_name: type_name.clone(),
                                field_name,
                                span: field.span,
                            })
                        } else {
                            // Struct not registered - might be external or undefined
                            // Fall back to fresh variable for now
                            Ok(self.inference.fresh_var())
                        }
                    }
                    Type::Var(_) => {
                        // Type not yet resolved, return fresh variable
                        Ok(self.inference.fresh_var())
                    }
                    // Tuple indexing: tuple.0, tuple.1, etc.
                    Type::Tuple(types) => {
                        // Try to parse field name as a numeric index
                        if let Ok(index) = field_name.parse::<usize>() {
                            if index < types.len() {
                                Ok(types[index].clone())
                            } else {
                                Err(TypeError::TupleIndexOutOfBounds {
                                    index,
                                    length: types.len(),
                                    span: field.span,
                                })
                            }
                        } else {
                            // Non-numeric field on tuple - tuples don't have named fields
                            Err(TypeError::FieldAccessOnNonStruct {
                                type_name: format!("{}", resolved_type),
                                span: expr.span,
                            })
                        }
                    }
                    _ => {
                        // Cannot access field on non-struct type
                        Err(TypeError::FieldAccessOnNonStruct {
                            type_name: format!("{}", resolved_type),
                            span: expr.span,
                        })
                    }
                }
            }

            // Index access
            ast::ExprKind::Index { object, index } => {
                let obj_type = self.infer_expr(object, env)?;
                let idx_type = self.infer_expr(index, env)?;

                match self.inference.apply(&obj_type) {
                    Type::Array(elem) => {
                        self.inference.unify(&idx_type, &Type::Int, index.span)?;
                        Ok(*elem)
                    }
                    Type::Map(key, val) => {
                        self.inference.unify(&idx_type, &key, index.span)?;
                        Ok(*val)
                    }
                    Type::String => {
                        self.inference.unify(&idx_type, &Type::Int, index.span)?;
                        Ok(Type::Char)
                    }
                    _ => Ok(self.inference.fresh_var()),
                }
            }

            // Lambda - synthesize mode (no expected type)
            // For bidirectional checking with expected type, use check_expr or infer_expr_with_expected
            ast::ExprKind::Lambda { params, body } => {
                self.infer_lambda(params, body, None, env)
            }

            // Range
            ast::ExprKind::Range { start, end, .. } => {
                if let Some(s) = start {
                    let st = self.infer_expr(s, env)?;
                    if !st.is_numeric() {
                        self.inference.unify(&st, &Type::Int, s.span)?;
                    }
                }
                if let Some(e) = end {
                    let et = self.infer_expr(e, env)?;
                    if !et.is_numeric() {
                        self.inference.unify(&et, &Type::Int, e.span)?;
                    }
                }
                Ok(Type::Named {
                    name: "Range".to_string(),
                    type_args: vec![Type::Int],
                })
            }

            // Try operator (? for early error propagation)
            ast::ExprKind::Try(inner) => {
                let inner_type = self.infer_expr(inner, env)?;
                let applied = self.inference.apply(&inner_type);
                match applied {
                    Type::Result(ok, _) => Ok(*ok),
                    Type::Optional(inner) => Ok(*inner),
                    Type::Var(_) => {
                        // Unknown type, might become Result or Optional later
                        // Return a fresh var that will be unified
                        Ok(self.inference.fresh_var())
                    }
                    other => Err(TypeError::InvalidTryOperator {
                        found: format!("{}", other),
                        span: expr.span,
                    }),
                }
            }

            // Unwrap operator
            ast::ExprKind::Unwrap(inner) => {
                let inner_type = self.infer_expr(inner, env)?;
                match self.inference.apply(&inner_type) {
                    Type::Optional(inner) => Ok(*inner),
                    Type::Result(ok, _) => Ok(*ok),
                    _ => Ok(self.inference.fresh_var()),
                }
            }

            // Contract expressions
            ast::ExprKind::Result => {
                // Result refers to the function's return value in ensures clauses
                // The "result" variable is bound when type-checking ensures clauses
                Ok(env.lookup_var("result")
                    .cloned()
                    .unwrap_or_else(|| self.inference.fresh_var()))
            }

            // Old expression: captures pre-state value in ensures clauses
            ast::ExprKind::Old(inner) => {
                // old(expr) has the same type as expr
                // It captures the value of expr at function entry
                self.infer_expr(inner, env)
            }

            // Universal quantifier: forall x: T where cond => body
            ast::ExprKind::Forall { var, ty, condition, body } => {
                // Create a new scope with the quantified variable
                let mut quant_env = TypeEnv::with_parent(Rc::new(env.clone()));
                let var_type = self.resolve_type(ty)?;
                quant_env.define_var(var.node.to_string(), var_type);

                // Type-check optional condition (must be Bool)
                if let Some(cond) = condition {
                    let cond_type = self.infer_expr(cond, &quant_env)?;
                    self.inference.unify(&cond_type, &Type::Bool, cond.span)?;
                }

                // Type-check body (must be Bool)
                let body_type = self.infer_expr(body, &quant_env)?;
                self.inference.unify(&body_type, &Type::Bool, body.span)?;

                // forall expressions evaluate to Bool
                Ok(Type::Bool)
            }

            // Existential quantifier: exists x: T where cond => body
            ast::ExprKind::Exists { var, ty, condition, body } => {
                // Create a new scope with the quantified variable
                let mut quant_env = TypeEnv::with_parent(Rc::new(env.clone()));
                let var_type = self.resolve_type(ty)?;
                quant_env.define_var(var.node.to_string(), var_type);

                // Type-check optional condition (must be Bool)
                if let Some(cond) = condition {
                    let cond_type = self.infer_expr(cond, &quant_env)?;
                    self.inference.unify(&cond_type, &Type::Bool, cond.span)?;
                }

                // Type-check body (must be Bool)
                let body_type = self.infer_expr(body, &quant_env)?;
                self.inference.unify(&body_type, &Type::Bool, body.span)?;

                // exists expressions evaluate to Bool
                Ok(Type::Bool)
            }

            // Match expression
            ast::ExprKind::Match { scrutinee, arms } => {
                let scrutinee_type = self.infer_expr(scrutinee, env)?;
                let mut result_type: Option<Type> = None;

                for arm in arms {
                    let mut arm_env = TypeEnv::with_parent(Rc::new(env.clone()));
                    self.check_pattern(&arm.pattern, &scrutinee_type, &mut arm_env)?;

                    if let Some(guard) = &arm.guard {
                        let guard_type = self.infer_expr(guard, &arm_env)?;
                        self.inference.unify(&guard_type, &Type::Bool, guard.span)?;
                    }

                    let arm_result = match &arm.body {
                        ast::MatchArmBody::Expr(expr) => self.infer_expr(expr, &arm_env)?,
                        ast::MatchArmBody::Block(block) => self.infer_block(block, &arm_env)?,
                    };

                    if let Some(ref prev_type) = result_type {
                        self.inference.unify(prev_type, &arm_result, arm.pattern.span)?;
                    } else {
                        result_type = Some(arm_result);
                    }
                }

                // Check for exhaustiveness
                let patterns: Vec<_> = arms.iter().map(|a| &a.pattern).collect();
                self.check_exhaustiveness(&scrutinee_type, &patterns, scrutinee.span)?;

                Ok(result_type.unwrap_or(Type::Unit))
            }

            // Method calls - handle built-in methods on Result and Optional types
            ast::ExprKind::MethodCall { object, method, args } => {
                let obj_type = self.infer_expr(object, env)?;
                let resolved_type = self.inference.apply(&obj_type);
                let method_name = method.node.as_str();

                match &resolved_type {
                    // Result type methods for error context/wrapping
                    Type::Result(ok_type, err_type) => {
                        match method_name {
                            // context(msg: String) -> Result<T, ContextError<E>>
                            // Wraps the error with a context message
                            "context" => {
                                if args.len() != 1 {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 1,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let arg_type = self.infer_expr(&args[0], env)?;
                                self.inference.unify(&arg_type, &Type::String, args[0].span)?;

                                // Return Result<T, ContextError<E>> - wraps the error with context
                                let context_error = Type::Named {
                                    name: "ContextError".to_string(),
                                    type_args: vec![(**err_type).clone()],
                                };
                                Ok(Type::Result(ok_type.clone(), Box::new(context_error)))
                            }

                            // with_context(f: () -> String) -> Result<T, ContextError<E>>
                            // Lazily computes context message only on error
                            "with_context" => {
                                if args.len() != 1 {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 1,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let arg_type = self.infer_expr(&args[0], env)?;
                                let expected_fn = Type::Function {
                                    params: vec![],
                                    return_type: Box::new(Type::String),
                                };
                                self.inference.unify(&arg_type, &expected_fn, args[0].span)?;

                                // Return Result<T, ContextError<E>>
                                let context_error = Type::Named {
                                    name: "ContextError".to_string(),
                                    type_args: vec![(**err_type).clone()],
                                };
                                Ok(Type::Result(ok_type.clone(), Box::new(context_error)))
                            }

                            // map_err(f: (E) -> E2) -> Result<T, E2>
                            // Transforms the error type using a function
                            "map_err" => {
                                if args.len() != 1 {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 1,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let arg_type = self.infer_expr(&args[0], env)?;
                                let new_err_type = self.inference.fresh_var();
                                let expected_fn = Type::Function {
                                    params: vec![(**err_type).clone()],
                                    return_type: Box::new(new_err_type.clone()),
                                };
                                self.inference.unify(&arg_type, &expected_fn, args[0].span)?;

                                Ok(Type::Result(ok_type.clone(), Box::new(self.inference.apply(&new_err_type))))
                            }

                            // map(f: (T) -> U) -> Result<U, E>
                            // Transforms the ok value using a function
                            "map" => {
                                if args.len() != 1 {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 1,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let arg_type = self.infer_expr(&args[0], env)?;
                                let new_ok_type = self.inference.fresh_var();
                                let expected_fn = Type::Function {
                                    params: vec![(**ok_type).clone()],
                                    return_type: Box::new(new_ok_type.clone()),
                                };
                                self.inference.unify(&arg_type, &expected_fn, args[0].span)?;

                                Ok(Type::Result(Box::new(self.inference.apply(&new_ok_type)), err_type.clone()))
                            }

                            // and_then(f: (T) -> Result<U, E>) -> Result<U, E>
                            // Chains Result operations (flatMap/bind)
                            "and_then" => {
                                if args.len() != 1 {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 1,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let arg_type = self.infer_expr(&args[0], env)?;
                                let new_ok_type = self.inference.fresh_var();
                                let expected_fn = Type::Function {
                                    params: vec![(**ok_type).clone()],
                                    return_type: Box::new(Type::Result(
                                        Box::new(new_ok_type.clone()),
                                        err_type.clone(),
                                    )),
                                };
                                self.inference.unify(&arg_type, &expected_fn, args[0].span)?;

                                Ok(Type::Result(Box::new(self.inference.apply(&new_ok_type)), err_type.clone()))
                            }

                            // unwrap_or(default: T) -> T
                            // Returns the ok value or a default
                            "unwrap_or" => {
                                if args.len() != 1 {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 1,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let arg_type = self.infer_expr(&args[0], env)?;
                                self.inference.unify(&arg_type, ok_type, args[0].span)?;
                                Ok((**ok_type).clone())
                            }

                            // unwrap_or_else(f: (E) -> T) -> T
                            // Returns the ok value or computes it from the error
                            "unwrap_or_else" => {
                                if args.len() != 1 {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 1,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let arg_type = self.infer_expr(&args[0], env)?;
                                let expected_fn = Type::Function {
                                    params: vec![(**err_type).clone()],
                                    return_type: ok_type.clone(),
                                };
                                self.inference.unify(&arg_type, &expected_fn, args[0].span)?;
                                Ok((**ok_type).clone())
                            }

                            // is_ok() -> Bool
                            "is_ok" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                Ok(Type::Bool)
                            }

                            // is_err() -> Bool
                            "is_err" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                Ok(Type::Bool)
                            }

                            // ok() -> T?
                            // Converts Result<T, E> to T?
                            "ok" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                Ok(Type::Optional(ok_type.clone()))
                            }

                            // err() -> E?
                            // Extracts the error as an optional
                            "err" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                Ok(Type::Optional(err_type.clone()))
                            }

                            _ => {
                                // Unknown method on Result - return fresh var for now
                                Ok(self.inference.fresh_var())
                            }
                        }
                    }

                    // Optional type methods
                    Type::Optional(inner_type) => {
                        match method_name {
                            // context(msg: String) -> Result<T, ContextError<()>>
                            // Converts None to an error with context
                            "context" => {
                                if args.len() != 1 {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 1,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let arg_type = self.infer_expr(&args[0], env)?;
                                self.inference.unify(&arg_type, &Type::String, args[0].span)?;

                                // Return Result<T, ContextError<()>> - None becomes an error
                                let context_error = Type::Named {
                                    name: "ContextError".to_string(),
                                    type_args: vec![Type::Unit],
                                };
                                Ok(Type::Result(inner_type.clone(), Box::new(context_error)))
                            }

                            // with_context(f: () -> String) -> Result<T, ContextError<()>>
                            "with_context" => {
                                if args.len() != 1 {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 1,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let arg_type = self.infer_expr(&args[0], env)?;
                                let expected_fn = Type::Function {
                                    params: vec![],
                                    return_type: Box::new(Type::String),
                                };
                                self.inference.unify(&arg_type, &expected_fn, args[0].span)?;

                                let context_error = Type::Named {
                                    name: "ContextError".to_string(),
                                    type_args: vec![Type::Unit],
                                };
                                Ok(Type::Result(inner_type.clone(), Box::new(context_error)))
                            }

                            // ok_or(err: E) -> Result<T, E>
                            // Converts T? to Result<T, E>
                            "ok_or" => {
                                if args.len() != 1 {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 1,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let err_type = self.infer_expr(&args[0], env)?;
                                Ok(Type::Result(inner_type.clone(), Box::new(err_type)))
                            }

                            // ok_or_else(f: () -> E) -> Result<T, E>
                            "ok_or_else" => {
                                if args.len() != 1 {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 1,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let arg_type = self.infer_expr(&args[0], env)?;
                                let err_type = self.inference.fresh_var();
                                let expected_fn = Type::Function {
                                    params: vec![],
                                    return_type: Box::new(err_type.clone()),
                                };
                                self.inference.unify(&arg_type, &expected_fn, args[0].span)?;
                                Ok(Type::Result(inner_type.clone(), Box::new(self.inference.apply(&err_type))))
                            }

                            // map(f: (T) -> U) -> U?
                            "map" => {
                                if args.len() != 1 {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 1,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let arg_type = self.infer_expr(&args[0], env)?;
                                let new_inner = self.inference.fresh_var();
                                let expected_fn = Type::Function {
                                    params: vec![(**inner_type).clone()],
                                    return_type: Box::new(new_inner.clone()),
                                };
                                self.inference.unify(&arg_type, &expected_fn, args[0].span)?;
                                Ok(Type::Optional(Box::new(self.inference.apply(&new_inner))))
                            }

                            // and_then(f: (T) -> U?) -> U?
                            "and_then" => {
                                if args.len() != 1 {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 1,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let arg_type = self.infer_expr(&args[0], env)?;
                                let new_inner = self.inference.fresh_var();
                                let expected_fn = Type::Function {
                                    params: vec![(**inner_type).clone()],
                                    return_type: Box::new(Type::Optional(Box::new(new_inner.clone()))),
                                };
                                self.inference.unify(&arg_type, &expected_fn, args[0].span)?;
                                Ok(Type::Optional(Box::new(self.inference.apply(&new_inner))))
                            }

                            // unwrap_or(default: T) -> T
                            "unwrap_or" => {
                                if args.len() != 1 {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 1,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let arg_type = self.infer_expr(&args[0], env)?;
                                self.inference.unify(&arg_type, inner_type, args[0].span)?;
                                Ok((**inner_type).clone())
                            }

                            // unwrap_or_else(f: () -> T) -> T
                            "unwrap_or_else" => {
                                if args.len() != 1 {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 1,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let arg_type = self.infer_expr(&args[0], env)?;
                                let expected_fn = Type::Function {
                                    params: vec![],
                                    return_type: inner_type.clone(),
                                };
                                self.inference.unify(&arg_type, &expected_fn, args[0].span)?;
                                Ok((**inner_type).clone())
                            }

                            // is_some() -> Bool
                            "is_some" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                Ok(Type::Bool)
                            }

                            // is_none() -> Bool
                            "is_none" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                Ok(Type::Bool)
                            }

                            _ => {
                                // Unknown method on Optional - return fresh var for now
                                Ok(self.inference.fresh_var())
                            }
                        }
                    }

                    // Array methods
                    Type::Array(elem_type) => {
                        match method_name {
                            "len" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                Ok(Type::Int)
                            }
                            "push" => {
                                if args.len() != 1 {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 1,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let arg_type = self.infer_expr(&args[0], env)?;
                                self.inference.unify(&arg_type, elem_type, args[0].span)?;
                                Ok(Type::Unit)
                            }
                            "pop" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                Ok(Type::Optional(elem_type.clone()))
                            }
                            "first" | "last" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                Ok(Type::Optional(elem_type.clone()))
                            }
                            "is_empty" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                Ok(Type::Bool)
                            }
                            "map" => {
                                if args.len() != 1 {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 1,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let arg_type = self.infer_expr(&args[0], env)?;
                                let new_elem = self.inference.fresh_var();
                                let expected_fn = Type::Function {
                                    params: vec![(**elem_type).clone()],
                                    return_type: Box::new(new_elem.clone()),
                                };
                                self.inference.unify(&arg_type, &expected_fn, args[0].span)?;
                                Ok(Type::Array(Box::new(self.inference.apply(&new_elem))))
                            }
                            "filter" => {
                                if args.len() != 1 {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 1,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let arg_type = self.infer_expr(&args[0], env)?;
                                let expected_fn = Type::Function {
                                    params: vec![(**elem_type).clone()],
                                    return_type: Box::new(Type::Bool),
                                };
                                self.inference.unify(&arg_type, &expected_fn, args[0].span)?;
                                Ok(Type::Array(elem_type.clone()))
                            }
                            // fold(init: U, f: (U, T) -> U) -> U
                            // Reduces the array to a single value using an accumulator
                            "fold" => {
                                if args.len() != 2 {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 2,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let init_type = self.infer_expr(&args[0], env)?;
                                let arg_type = self.infer_expr(&args[1], env)?;
                                let acc_type = self.inference.fresh_var();
                                let expected_fn = Type::Function {
                                    params: vec![acc_type.clone(), (**elem_type).clone()],
                                    return_type: Box::new(acc_type.clone()),
                                };
                                self.inference.unify(&arg_type, &expected_fn, args[1].span)?;
                                self.inference.unify(&init_type, &acc_type, args[0].span)?;
                                Ok(self.inference.apply(&acc_type))
                            }
                            // reduce(f: (T, T) -> T) -> T?
                            // Reduces the array using the first element as initial value
                            "reduce" => {
                                if args.len() != 1 {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 1,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let arg_type = self.infer_expr(&args[0], env)?;
                                let expected_fn = Type::Function {
                                    params: vec![(**elem_type).clone(), (**elem_type).clone()],
                                    return_type: elem_type.clone(),
                                };
                                self.inference.unify(&arg_type, &expected_fn, args[0].span)?;
                                Ok(Type::Optional(elem_type.clone()))
                            }
                            // find(f: (T) -> Bool) -> T?
                            // Returns the first element matching the predicate
                            "find" => {
                                if args.len() != 1 {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 1,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let arg_type = self.infer_expr(&args[0], env)?;
                                let expected_fn = Type::Function {
                                    params: vec![(**elem_type).clone()],
                                    return_type: Box::new(Type::Bool),
                                };
                                self.inference.unify(&arg_type, &expected_fn, args[0].span)?;
                                Ok(Type::Optional(elem_type.clone()))
                            }
                            // any(f: (T) -> Bool) -> Bool
                            // Returns true if any element matches the predicate
                            "any" => {
                                if args.len() != 1 {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 1,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let arg_type = self.infer_expr(&args[0], env)?;
                                let expected_fn = Type::Function {
                                    params: vec![(**elem_type).clone()],
                                    return_type: Box::new(Type::Bool),
                                };
                                self.inference.unify(&arg_type, &expected_fn, args[0].span)?;
                                Ok(Type::Bool)
                            }
                            // all(f: (T) -> Bool) -> Bool
                            // Returns true if all elements match the predicate
                            "all" => {
                                if args.len() != 1 {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 1,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let arg_type = self.infer_expr(&args[0], env)?;
                                let expected_fn = Type::Function {
                                    params: vec![(**elem_type).clone()],
                                    return_type: Box::new(Type::Bool),
                                };
                                self.inference.unify(&arg_type, &expected_fn, args[0].span)?;
                                Ok(Type::Bool)
                            }
                            _ => Ok(self.inference.fresh_var()),
                        }
                    }

                    // Map methods
                    Type::Map(key_type, value_type) => {
                        match method_name {
                            // get(key: K) -> V?
                            // Returns the value associated with the key, or None if not found
                            "get" => {
                                if args.len() != 1 {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 1,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let arg_type = self.infer_expr(&args[0], env)?;
                                self.inference.unify(&arg_type, key_type, args[0].span)?;
                                Ok(Type::Optional(value_type.clone()))
                            }

                            // contains_key(key: K) -> Bool
                            // Returns true if the map contains the key
                            "contains_key" => {
                                if args.len() != 1 {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 1,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let arg_type = self.infer_expr(&args[0], env)?;
                                self.inference.unify(&arg_type, key_type, args[0].span)?;
                                Ok(Type::Bool)
                            }

                            // keys() -> Array<K>
                            // Returns an array of all keys in the map
                            "keys" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                Ok(Type::Array(key_type.clone()))
                            }

                            // values() -> Array<V>
                            // Returns an array of all values in the map
                            "values" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                Ok(Type::Array(value_type.clone()))
                            }

                            // entries() -> Array<(K, V)>
                            // Returns an array of key-value tuples
                            "entries" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let entry_type = Type::Tuple(vec![
                                    (**key_type).clone(),
                                    (**value_type).clone(),
                                ]);
                                Ok(Type::Array(Box::new(entry_type)))
                            }

                            // insert(key: K, value: V) -> V?
                            // Inserts a key-value pair, returns the old value if key existed
                            "insert" => {
                                if args.len() != 2 {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 2,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let key_arg_type = self.infer_expr(&args[0], env)?;
                                let value_arg_type = self.infer_expr(&args[1], env)?;
                                self.inference.unify(&key_arg_type, key_type, args[0].span)?;
                                self.inference.unify(&value_arg_type, value_type, args[1].span)?;
                                Ok(Type::Optional(value_type.clone()))
                            }

                            // remove(key: K) -> V?
                            // Removes a key and returns its value if it existed
                            "remove" => {
                                if args.len() != 1 {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 1,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let arg_type = self.infer_expr(&args[0], env)?;
                                self.inference.unify(&arg_type, key_type, args[0].span)?;
                                Ok(Type::Optional(value_type.clone()))
                            }

                            // len() -> Int
                            // Returns the number of key-value pairs in the map
                            "len" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                Ok(Type::Int)
                            }

                            // is_empty() -> Bool
                            // Returns true if the map has no entries
                            "is_empty" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                Ok(Type::Bool)
                            }

                            _ => Ok(self.inference.fresh_var()),
                        }
                    }

                    // String methods
                    Type::String => {
                        match method_name {
                            "len" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                Ok(Type::Int)
                            }
                            "is_empty" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                Ok(Type::Bool)
                            }
                            "to_uppercase" | "to_lowercase" | "trim" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                Ok(Type::String)
                            }
                            "contains" | "starts_with" | "ends_with" => {
                                if args.len() != 1 {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 1,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let arg_type = self.infer_expr(&args[0], env)?;
                                self.inference.unify(&arg_type, &Type::String, args[0].span)?;
                                Ok(Type::Bool)
                            }
                            "split" => {
                                if args.len() != 1 {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 1,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let arg_type = self.infer_expr(&args[0], env)?;
                                self.inference.unify(&arg_type, &Type::String, args[0].span)?;
                                Ok(Type::Array(Box::new(Type::String)))
                            }
                            "replace" => {
                                if args.len() != 2 {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 2,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let arg1_type = self.infer_expr(&args[0], env)?;
                                let arg2_type = self.infer_expr(&args[1], env)?;
                                self.inference.unify(&arg1_type, &Type::String, args[0].span)?;
                                self.inference.unify(&arg2_type, &Type::String, args[1].span)?;
                                Ok(Type::String)
                            }
                            _ => Ok(self.inference.fresh_var()),
                        }
                    }

                    // Range methods
                    Type::Named { name, type_args } if name == "Range" => {
                        let elem_type = type_args.first().cloned().unwrap_or(Type::Int);
                        match method_name {
                            // contains(value: T) -> Bool
                            // Check if a value is within the range
                            "contains" => {
                                if args.len() != 1 {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 1,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let arg_type = self.infer_expr(&args[0], env)?;
                                self.inference.unify(&arg_type, &elem_type, args[0].span)?;
                                Ok(Type::Bool)
                            }

                            // start() -> T
                            // Get the start value of the range
                            "start" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                Ok(elem_type)
                            }

                            // end() -> T
                            // Get the end value of the range
                            "end" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                Ok(elem_type)
                            }

                            // is_empty() -> Bool
                            // Check if the range is empty (start >= end for exclusive, start > end for inclusive)
                            "is_empty" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                Ok(Type::Bool)
                            }

                            // len() -> Int
                            // Get the number of elements in the range (for integer ranges)
                            "len" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                Ok(Type::Int)
                            }

                            // step_by(n: Int) -> Range<T>
                            // Create a stepped range that yields every nth element
                            "step_by" => {
                                if args.len() != 1 {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 1,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let arg_type = self.infer_expr(&args[0], env)?;
                                self.inference.unify(&arg_type, &Type::Int, args[0].span)?;
                                Ok(resolved_type.clone())
                            }

                            // rev() -> Range<T>
                            // Create a reversed range that iterates in opposite order
                            "rev" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                Ok(resolved_type.clone())
                            }

                            _ => Ok(self.inference.fresh_var()),
                        }
                    }

                    // Tuple methods
                    Type::Tuple(types) => {
                        match method_name {
                            // first() -> T0
                            // Returns the first element of the tuple (compile-time known)
                            "first" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                if types.is_empty() {
                                    return Err(TypeError::TupleIndexOutOfBounds {
                                        index: 0,
                                        length: 0,
                                        span: expr.span,
                                    });
                                }
                                Ok(types[0].clone())
                            }

                            // last() -> TN
                            // Returns the last element of the tuple (compile-time known)
                            "last" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                if types.is_empty() {
                                    return Err(TypeError::TupleIndexOutOfBounds {
                                        index: 0,
                                        length: 0,
                                        span: expr.span,
                                    });
                                }
                                Ok(types[types.len() - 1].clone())
                            }

                            // len() -> Int
                            // Returns the length of the tuple (compile-time constant)
                            "len" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                Ok(Type::Int)
                            }

                            // to_array() -> Array<T>
                            // Converts tuple to array if all elements have the same type
                            "to_array" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                if types.is_empty() {
                                    // Empty tuple converts to empty array of any type
                                    Ok(Type::Array(Box::new(self.inference.fresh_var())))
                                } else {
                                    // Check all elements have the same type
                                    let first_type = &types[0];
                                    let mut all_same = true;
                                    for ty in types.iter().skip(1) {
                                        // Try to unify - if unification fails, types are different
                                        if self.inference.unify(first_type, ty, expr.span).is_err() {
                                            all_same = false;
                                            break;
                                        }
                                    }
                                    if all_same {
                                        Ok(Type::Array(Box::new(self.inference.apply(first_type))))
                                    } else {
                                        let type_strs: Vec<String> = types.iter()
                                            .map(|t| format!("{}", t))
                                            .collect();
                                        Err(TypeError::TupleToArrayHeterogeneousTypes {
                                            types: type_strs.join(", "),
                                            span: expr.span,
                                        })
                                    }
                                }
                            }

                            _ => Ok(self.inference.fresh_var()),
                        }
                    }

                    // Channel methods
                    Type::Channel(elem_type) => {
                        match method_name {
                            // send(value: T) -> Result<(), SendError>
                            // Sends a value through the channel
                            "send" => {
                                if args.len() != 1 {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 1,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let arg_type = self.infer_expr(&args[0], env)?;
                                // Value must match channel element type
                                self.inference.unify(&arg_type, elem_type, args[0].span)?;
                                // Check Transfer trait for channel values
                                let resolved_arg = self.inference.apply(&arg_type);
                                if !resolved_arg.is_transfer() {
                                    return Err(TypeError::NonTransferCapture {
                                        var_name: "channel value".to_string(),
                                        var_type: format!("{}", resolved_arg),
                                        span: args[0].span,
                                    });
                                }
                                // Returns Result<(), SendError>
                                Ok(Type::Result(
                                    Box::new(Type::Unit),
                                    Box::new(Type::Named {
                                        name: "SendError".to_string(),
                                        type_args: vec![],
                                    }),
                                ))
                            }

                            // try_send(value: T) -> Result<(), TrySendError>
                            // Attempts to send without blocking
                            "try_send" => {
                                if args.len() != 1 {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 1,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                let arg_type = self.infer_expr(&args[0], env)?;
                                self.inference.unify(&arg_type, elem_type, args[0].span)?;
                                let resolved_arg = self.inference.apply(&arg_type);
                                if !resolved_arg.is_transfer() {
                                    return Err(TypeError::NonTransferCapture {
                                        var_name: "channel value".to_string(),
                                        var_type: format!("{}", resolved_arg),
                                        span: args[0].span,
                                    });
                                }
                                Ok(Type::Result(
                                    Box::new(Type::Unit),
                                    Box::new(Type::Named {
                                        name: "TrySendError".to_string(),
                                        type_args: vec![],
                                    }),
                                ))
                            }

                            // recv() -> Result<T, RecvError>
                            // Receives a value from the channel
                            "recv" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                Ok(Type::Result(
                                    elem_type.clone(),
                                    Box::new(Type::Named {
                                        name: "RecvError".to_string(),
                                        type_args: vec![],
                                    }),
                                ))
                            }

                            // try_recv() -> Result<T, TryRecvError>
                            // Attempts to receive without blocking
                            "try_recv" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                Ok(Type::Result(
                                    elem_type.clone(),
                                    Box::new(Type::Named {
                                        name: "TryRecvError".to_string(),
                                        type_args: vec![],
                                    }),
                                ))
                            }

                            // close() -> ()
                            // Closes the channel
                            "close" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                Ok(Type::Unit)
                            }

                            // is_closed() -> Bool
                            // Checks if the channel is closed
                            "is_closed" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                Ok(Type::Bool)
                            }

                            // is_empty() -> Bool
                            // Checks if the channel has no pending messages
                            "is_empty" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                Ok(Type::Bool)
                            }

                            // is_full() -> Bool
                            // Checks if the channel is at capacity
                            "is_full" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                Ok(Type::Bool)
                            }

                            // len() -> Int
                            // Returns the number of messages in the channel
                            "len" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                Ok(Type::Int)
                            }

                            // capacity() -> Int?
                            // Returns the channel capacity (None for unbounded)
                            "capacity" => {
                                if !args.is_empty() {
                                    return Err(TypeError::WrongTypeArity {
                                        expected: 0,
                                        found: args.len(),
                                        span: expr.span,
                                    });
                                }
                                Ok(Type::Optional(Box::new(Type::Int)))
                            }

                            _ => Ok(self.inference.fresh_var()),
                        }
                    }

                    // For other types or type variables, return a fresh var
                    _ => Ok(self.inference.fresh_var()),
                }
            }

            // ================================================================
            // Concurrency expressions
            // ================================================================

            // Spawn: spawn expr -> Task[T] where T is the type of expr
            ast::ExprKind::Spawn(inner) => {
                // Save current async context and enter async for the spawn body
                let was_async = self.in_async_context;
                self.in_async_context = true;

                // Infer the type of the spawned expression
                let inner_type = self.infer_expr(inner, env)?;

                // Restore async context
                self.in_async_context = was_async;

                // The result is a Task containing the inner type
                Ok(Type::Task(Box::new(inner_type)))
            }

            // Await: await expr -> T where expr is Task[T]
            ast::ExprKind::Await(inner) => {
                // Check that we're not in a defer context - await cannot be used in defer
                if self.in_defer_context {
                    return Err(TypeError::AwaitInDefer { span: expr.span });
                }

                // Check that we're in an async context
                if !self.in_async_context {
                    return Err(TypeError::AwaitOutsideAsync { span: expr.span });
                }

                // Infer the type of the awaited expression
                let inner_type = self.infer_expr(inner, env)?;
                let resolved = self.inference.apply(&inner_type);

                match resolved {
                    Type::Task(result_type) => Ok(*result_type),
                    Type::Var(_) => {
                        let result_type = self.inference.fresh_var();
                        let task_type = Type::Task(Box::new(result_type.clone()));
                        self.inference.unify(&inner_type, &task_type, inner.span)?;
                        Ok(result_type)
                    }
                    other => Err(TypeError::AwaitNonTask {
                        found: format!("{}", other),
                        span: inner.span,
                    }),
                }
            }

            // Channel send: channel <- value -> Unit
            // The value must match the channel's element type AND implement Transfer
            ast::ExprKind::ChannelSend { channel, value } => {
                let channel_type = self.infer_expr(channel, env)?;
                let resolved = self.inference.apply(&channel_type);

                match resolved {
                    Type::Channel(elem_type) => {
                        let value_type = self.infer_expr(value, env)?;
                        // Unify value type with channel element type
                        self.inference.unify(&value_type, &elem_type, value.span)?;

                        // Check Transfer trait requirement for channel values
                        // Values sent through channels must be safely transferable between tasks
                        let resolved_value = self.inference.apply(&value_type);
                        if !resolved_value.is_transfer() {
                            return Err(TypeError::NonTransferCapture {
                                var_name: "channel value".to_string(),
                                var_type: format!("{}", resolved_value),
                                span: value.span,
                            });
                        }

                        Ok(Type::Unit)
                    }
                    Type::Var(_) => {
                        let value_type = self.infer_expr(value, env)?;

                        // Check Transfer trait for the value
                        let resolved_value = self.inference.apply(&value_type);
                        if !resolved_value.is_transfer() {
                            return Err(TypeError::NonTransferCapture {
                                var_name: "channel value".to_string(),
                                var_type: format!("{}", resolved_value),
                                span: value.span,
                            });
                        }

                        let channel_expected = Type::Channel(Box::new(value_type));
                        self.inference.unify(&channel_type, &channel_expected, channel.span)?;
                        Ok(Type::Unit)
                    }
                    other => Err(TypeError::SendOnNonChannel {
                        found: format!("{}", other),
                        span: channel.span,
                    }),
                }
            }

            // Channel receive: <- channel -> T
            ast::ExprKind::ChannelRecv { channel } => {
                let channel_type = self.infer_expr(channel, env)?;
                let resolved = self.inference.apply(&channel_type);

                match resolved {
                    Type::Channel(elem_type) => Ok(*elem_type),
                    Type::Var(_) => {
                        let elem_type = self.inference.fresh_var();
                        let channel_expected = Type::Channel(Box::new(elem_type.clone()));
                        self.inference.unify(&channel_type, &channel_expected, channel.span)?;
                        Ok(elem_type)
                    }
                    other => Err(TypeError::ReceiveOnNonChannel {
                        found: format!("{}", other),
                        span: channel.span,
                    }),
                }
            }

            // Select: select { arms } -> T
            ast::ExprKind::Select(arms) => {
                if arms.is_empty() {
                    return Ok(Type::Unit);
                }

                // Check for multiple default arms
                let mut default_arm_span: Option<Span> = None;
                for arm in arms {
                    if matches!(arm.kind, ast::SelectArmKind::Default) {
                        if let Some(first_span) = default_arm_span {
                            return Err(TypeError::MultipleDefaultArms {
                                first_span,
                                second_span: arm.span,
                            });
                        }
                        default_arm_span = Some(arm.span);
                    }
                }

                let mut result_type: Option<Type> = None;

                for (arm_index, arm) in arms.iter().enumerate() {
                    let mut arm_env = TypeEnv::with_parent(Rc::new(env.clone()));

                    match &arm.kind {
                        ast::SelectArmKind::Receive { pattern, channel } => {
                            let channel_type = self.infer_expr(channel, env)?;
                            let resolved = self.inference.apply(&channel_type);

                            let elem_type = match resolved {
                                Type::Channel(elem) => *elem,
                                Type::Var(_) => {
                                    let elem = self.inference.fresh_var();
                                    let expected = Type::Channel(Box::new(elem.clone()));
                                    self.inference.unify(&channel_type, &expected, channel.span)?;
                                    elem
                                }
                                other => {
                                    return Err(TypeError::ReceiveOnNonChannel {
                                        found: format!("{}", other),
                                        span: channel.span,
                                    });
                                }
                            };

                            // Bind pattern variables if present
                            if let Some(pat) = pattern {
                                self.check_pattern(pat, &elem_type, &mut arm_env)?;
                            }
                        }
                        ast::SelectArmKind::Send { channel, value } => {
                            let channel_type = self.infer_expr(channel, env)?;
                            let resolved = self.inference.apply(&channel_type);

                            match resolved {
                                Type::Channel(elem_type) => {
                                    let value_type = self.infer_expr(value, env)?;
                                    self.inference.unify(&value_type, &elem_type, value.span)?;
                                }
                                Type::Var(_) => {
                                    let value_type = self.infer_expr(value, env)?;
                                    let expected = Type::Channel(Box::new(value_type));
                                    self.inference.unify(&channel_type, &expected, channel.span)?;
                                }
                                other => {
                                    return Err(TypeError::SendOnNonChannel {
                                        found: format!("{}", other),
                                        span: channel.span,
                                    });
                                }
                            }
                        }
                        ast::SelectArmKind::Default => {}
                    }

                    // Type check the body (it's an Expr, not MatchArmBody)
                    let arm_result = self.infer_expr(&arm.body, &arm_env)?;

                    if let Some(ref prev_type) = result_type {
                        // Use more specific error for arm type mismatch
                        let prev_resolved = self.inference.apply(prev_type);
                        let arm_resolved = self.inference.apply(&arm_result);

                        if self.inference.unify(prev_type, &arm_result, arm.span).is_err() {
                            return Err(TypeError::SelectArmTypeMismatch {
                                expected: format!("{}", prev_resolved),
                                found: format!("{}", arm_resolved),
                                arm_index,
                                span: arm.span,
                            });
                        }
                    } else {
                        result_type = Some(arm_result);
                    }
                }

                Ok(result_type.unwrap_or(Type::Unit))
            }

            // Safe navigation: obj?.field -> T?
            ast::ExprKind::SafeNav { object, field } => {
                let obj_type = self.infer_expr(object, env)?;
                let resolved = self.inference.apply(&obj_type);

                match resolved {
                    Type::Optional(inner) => {
                        let field_name = field.node.to_string();
                        match inner.as_ref() {
                            Type::Named { name: type_name, .. } => {
                                if let Some(fields) = self.struct_fields.get(type_name) {
                                    for (name, ty) in fields {
                                        if name == &field_name {
                                            return Ok(Type::Optional(Box::new(ty.clone())));
                                        }
                                    }
                                    Err(TypeError::UndefinedField {
                                        type_name: type_name.clone(),
                                        field_name,
                                        span: field.span,
                                    })
                                } else {
                                    Ok(Type::Optional(Box::new(self.inference.fresh_var())))
                                }
                            }
                            _ => Ok(Type::Optional(Box::new(self.inference.fresh_var()))),
                        }
                    }
                    Type::Var(_) => Ok(Type::Optional(Box::new(self.inference.fresh_var()))),
                    _ => Ok(Type::Optional(Box::new(self.inference.fresh_var()))),
                }
            }

            // Pipe: left |> right -> right(left)
            ast::ExprKind::Pipe { left, right } => {
                let left_type = self.infer_expr(left, env)?;
                let right_type = self.infer_expr(right, env)?;

                match self.inference.apply(&right_type) {
                    Type::Function { params, return_type } => {
                        if params.len() != 1 {
                            return Err(TypeError::WrongTypeArity {
                                expected: 1,
                                found: params.len(),
                                span: right.span,
                            });
                        }
                        self.inference.unify(&left_type, &params[0], left.span)?;
                        Ok(*return_type)
                    }
                    Type::Var(_) => {
                        let return_type = self.inference.fresh_var();
                        let expected_fn = Type::Function {
                            params: vec![left_type],
                            return_type: Box::new(return_type.clone()),
                        };
                        self.inference.unify(&right_type, &expected_fn, right.span)?;
                        Ok(return_type)
                    }
                    _ => Err(TypeError::Mismatch {
                        expected: "function".to_string(),
                        found: format!("{}", right_type),
                        span: right.span,
                        expected_source: None,
                    }),
                }
            }

            // Path expression: module::item
            ast::ExprKind::Path(segments) => {
                if segments.is_empty() {
                    return Ok(self.inference.fresh_var());
                }

                if segments.len() >= 2 {
                    let module_name = segments[0].node.to_string();
                    let item_name = segments[1].node.to_string();

                    if let Some(exports) = self.module_exports.get(&module_name) {
                        if let Some(export) = exports.get(&item_name) {
                            return Ok(export.ty.clone());
                        }
                    }
                }

                Ok(self.inference.fresh_var())
            }

            // Handle expression: effect handler for try/catch
            // handle body with handlers end -> Result type based on handlers
            ast::ExprKind::Handle { body, handlers, return_clause } => {
                // Infer the type of the body being handled
                let body_type = self.infer_expr(body, env)?;

                // Track the result type - starts as body type, may be transformed by return clause
                let mut result_type = body_type.clone();

                // Type check each handler clause
                for handler in handlers {
                    // Create environment for handler with bound parameters
                    let mut handler_env = TypeEnv::with_parent(Rc::new(env.clone()));

                    // The effect.operation pattern tells us what effect is being handled
                    // For Exception.raise(e), e is bound to the exception type
                    let effect_name = handler.effect.node.to_string();
                    let operation_name = handler.operation.node.to_string();

                    // For Exception effect, the raise operation takes an error and returns Never
                    if effect_name == "Exception" && operation_name == "raise" {
                        // Bind the exception parameter
                        if let Some(param) = handler.params.first() {
                            // The exception type could be inferred or annotated
                            let exception_type = self.inference.fresh_var();
                            self.check_pattern(param, &exception_type, &mut handler_env)?;
                        }
                    } else {
                        // For other effects, bind parameters generically
                        for param in &handler.params {
                            let param_type = self.inference.fresh_var();
                            self.check_pattern(param, &param_type, &mut handler_env)?;
                        }
                    }

                    // Type check the handler body
                    let handler_body_type = match &handler.body {
                        ast::HandlerBody::Expr(e) => self.infer_expr(e, &handler_env)?,
                        ast::HandlerBody::Block(block) => self.infer_block(block, &handler_env)?,
                    };

                    // Handler body type should unify with result type
                    // (all handlers should return compatible types)
                    self.inference.unify(&result_type, &handler_body_type, handler.span)?;
                }

                // If there's a return clause, it transforms the final result
                if let Some(ret_clause) = return_clause {
                    let mut return_env = TypeEnv::with_parent(Rc::new(env.clone()));

                    // The return clause's pattern binds the body's result
                    self.check_pattern(&ret_clause.pattern, &body_type, &mut return_env)?;

                    // The return clause's body produces the final result
                    let return_body_type = match ret_clause.body.as_ref() {
                        ast::HandlerBody::Expr(e) => self.infer_expr(e, &return_env)?,
                        ast::HandlerBody::Block(block) => self.infer_block(block, &return_env)?,
                    };

                    result_type = return_body_type;
                }

                Ok(result_type)
            }

            // Raise expression: raise(error) -> Never
            // Raising an exception transfers control to the nearest handler
            ast::ExprKind::Raise { error, exception_type: type_annotation } => {
                // Infer the type of the error being raised
                let error_type = self.infer_expr(error, env)?;

                // If there's a type annotation, unify with it
                if let Some(type_expr) = type_annotation {
                    let annotated_type = self.resolve_type(type_expr)?;
                    self.inference.unify(&error_type, &annotated_type, expr.span)?;
                }

                // Raise has the Exception effect with the error type
                // The function's effect row should include Exception[error_type]
                // For now, we track this by returning Never (the raise never returns normally)
                Ok(Type::Never)
            }

            // Resume expression: resume(value) -> T (inside effect handlers)
            // Resume continues the suspended computation with the given value
            ast::ExprKind::Resume { value } => {
                // Infer the type of the resume value
                let value_type = self.infer_expr(value, env)?;

                // The resume's return type depends on the handler context
                // For now, return a fresh type variable that will be unified
                // with the continuation's expected type
                // TODO: Add proper handler context tracking
                let _ = value_type; // Used for effect handler semantics

                // Resume's type is the type expected by the continuation
                Ok(self.inference.fresh_var())
            }

            // Default case for any remaining unimplemented expressions
            _ => Ok(self.inference.fresh_var()),
        }
    }

    // ========================================================================
    // Bidirectional Type Checking
    // ========================================================================

    /// Check an expression against an expected type (top-down checking mode)
    ///
    /// This is the "check" direction of bidirectional type checking. Instead of
    /// synthesizing a type from the expression, we verify that the expression
    /// matches an expected type. This enables:
    ///
    /// 1. Better error messages ("expected X from Y, found Z")
    /// 2. Type propagation into lambdas (parameter types from context)
    /// 3. More precise type inference in ambiguous situations
    ///
    /// # Example
    ///
    /// ```ignore
    /// // When checking `|x| x + 1` against `(Int) -> Int`:
    /// // - Parameter `x` gets type `Int` from the expected function type
    /// // - Body is checked against return type `Int`
    /// ```
    pub fn check_expr(
        &mut self,
        expr: &ast::Expr,
        expected: &Type,
        source: TypeSource,
        env: &TypeEnv,
    ) -> TypeResult<()> {
        let expected = self.inference.apply(expected);

        match (&expr.kind, &expected) {
            // Lambda checking: propagate expected parameter and return types
            (ast::ExprKind::Lambda { params, body }, Type::Function { params: expected_params, return_type }) => {
                if params.len() != expected_params.len() {
                    return Err(TypeError::WrongTypeArity {
                        expected: expected_params.len(),
                        found: params.len(),
                        span: expr.span,
                    });
                }

                // Create lambda environment with parameter types from expected
                let mut lambda_env = TypeEnv::with_parent(Rc::new(env.clone()));

                for (param, expected_param_type) in params.iter().zip(expected_params.iter()) {
                    let param_type = if let Some(t) = &param.ty {
                        // Explicit annotation - resolve and unify with expected
                        let annotated = self.resolve_type(t)?;
                        self.inference.unify(&annotated, expected_param_type, param.name.span)?;
                        annotated
                    } else {
                        // No annotation - use expected type (bidirectional propagation!)
                        expected_param_type.clone()
                    };
                    lambda_env.define_var(param.name.node.to_string(), param_type);
                }

                // Check body against expected return type
                let return_source = TypeSource::Return(body.span);
                self.check_expr(body, return_type, return_source, &lambda_env)
            }

            // If expression checking: propagate expected type to branches
            (ast::ExprKind::If { condition, then_branch, elsif_branches, else_branch }, _) => {
                let cond_type = self.infer_expr(condition, env)?;
                self.inference.unify(&cond_type, &Type::Bool, condition.span)?;

                self.check_block(then_branch, &expected, source.clone(), env)?;

                for (elsif_cond, elsif_body) in elsif_branches {
                    let elsif_cond_type = self.infer_expr(elsif_cond, env)?;
                    self.inference.unify(&elsif_cond_type, &Type::Bool, elsif_cond.span)?;
                    self.check_block(elsif_body, &expected, source.clone(), env)?;
                }

                if let Some(else_body) = else_branch {
                    self.check_block(else_body, &expected, source.clone(), env)?;
                }

                Ok(())
            }

            // Ternary expression checking: propagate expected type to branches
            (ast::ExprKind::Ternary { condition, then_expr, else_expr }, _) => {
                let cond_type = self.infer_expr(condition, env)?;
                self.inference.unify(&cond_type, &Type::Bool, condition.span)?;

                let branch_source = TypeSource::ConditionalBranch(expr.span);
                self.check_expr(then_expr, &expected, branch_source.clone(), env)?;
                self.check_expr(else_expr, &expected, branch_source, env)?;
                Ok(())
            }

            // Block expression: check the block
            (ast::ExprKind::Block(block), _) => {
                self.check_block(block, &expected, source, env)
            }

            // Parenthesized expression: check inner
            (ast::ExprKind::Paren(inner), _) => {
                self.check_expr(inner, &expected, source, env)
            }

            // Default case: synthesize and unify
            // This handles most expression types where bidirectional checking
            // doesn't provide additional benefit over synthesis + unification
            _ => {
                let inferred = self.infer_expr(expr, env)?;
                self.inference.unify(&inferred, &expected, expr.span)?;
                Ok(())
            }
        }
    }

    /// Check a block against an expected type
    fn check_block(
        &mut self,
        block: &ast::Block,
        expected: &Type,
        source: TypeSource,
        env: &TypeEnv,
    ) -> TypeResult<()> {
        let mut block_env = TypeEnv::with_parent(Rc::new(env.clone()));

        // Check all statements except the last
        for stmt in block.stmts.iter().take(block.stmts.len().saturating_sub(1)) {
            self.check_stmt(stmt, &mut block_env)?;
        }

        // Check the last statement (the block's value) against expected type
        if let Some(last) = block.stmts.last() {
            match &last.kind {
                ast::StmtKind::Expr(expr) => {
                    self.check_expr(expr, expected, source, &block_env)?;
                }
                _ => {
                    // Non-expression statement - block returns Unit
                    self.check_stmt(last, &mut block_env)?;
                    self.inference.unify(&Type::Unit, expected, block.span)?;
                }
            }
        } else {
            // Empty block - returns Unit
            self.inference.unify(&Type::Unit, expected, block.span)?;
        }

        Ok(())
    }

    /// Infer a lambda expression with an optional expected function type
    ///
    /// When `expected_fn_type` is provided, parameter types without explicit
    /// annotations are taken from the expected type (bidirectional propagation).
    pub fn infer_lambda(
        &mut self,
        params: &[ast::Param],
        body: &ast::Expr,
        expected_fn_type: Option<&Type>,
        env: &TypeEnv,
    ) -> TypeResult<Type> {
        let mut lambda_env = TypeEnv::with_parent(Rc::new(env.clone()));

        // Extract expected parameter and return types if available
        let (expected_params, expected_return): (Option<&[Type]>, Option<&Type>) = match expected_fn_type {
            Some(Type::Function { params: p, return_type: r }) => {
                if p.len() == params.len() {
                    (Some(p.as_slice()), Some(r.as_ref()))
                } else {
                    (None, None)
                }
            }
            _ => (None, None),
        };

        // Determine parameter types
        let param_types: Vec<Type> = params
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let ty = if let Some(t) = &p.ty {
                    // Explicit type annotation
                    self.resolve_type(t).unwrap_or_else(|_| self.inference.fresh_var())
                } else if let Some(exp_params) = expected_params {
                    // Bidirectional: use expected parameter type
                    exp_params[i].clone()
                } else {
                    // No information available - fresh type variable
                    self.inference.fresh_var()
                };
                lambda_env.define_var(p.name.node.to_string(), ty.clone());
                ty
            })
            .collect();

        // Infer or check body type
        let return_type = if let Some(expected_ret) = expected_return {
            let source = TypeSource::Return(body.span);
            self.check_expr(body, expected_ret, source, &lambda_env)?;
            self.inference.apply(expected_ret)
        } else {
            self.infer_expr(body, &lambda_env)?
        };

        Ok(Type::Function {
            params: param_types,
            return_type: Box::new(return_type),
        })
    }

    /// Infer an expression with an optional expected type (unified bidirectional entry point)
    ///
    /// This method combines synthesis and checking modes:
    /// - If `expected` is `Some`, checks the expression against the expected type
    /// - If `expected` is `None`, synthesizes the type from the expression
    pub fn infer_expr_with_expected(
        &mut self,
        expr: &ast::Expr,
        expected: Option<&Type>,
        source: TypeSource,
        env: &TypeEnv,
    ) -> TypeResult<Type> {
        match expected {
            Some(expected_type) => {
                self.check_expr(expr, expected_type, source, env)?;
                Ok(self.inference.apply(expected_type))
            }
            None => self.infer_expr(expr, env),
        }
    }

    /// Extract function name from a call expression target.
    ///
    /// Returns Some(name) for simple identifier calls like `foo()`,
    /// None for complex expressions like `obj.method()` or `(|| x)()`.
    fn extract_func_name(&self, func_expr: &ast::Expr) -> Option<String> {
        match &func_expr.kind {
            ast::ExprKind::Ident(name) => Some(name.to_string()),
            ast::ExprKind::Path(segments) => {
                // Handle qualified paths like `Mod::func`
                Some(segments.iter().map(|s| s.node.to_string()).collect::<Vec<_>>().join("::"))
            }
            _ => None,
        }
    }

    /// Check a function call against a known signature with default parameter support.
    ///
    /// This method handles:
    /// - Positional arguments
    /// - Named arguments
    /// - Default parameter values (omitted arguments)
    /// - Mixed positional and named arguments
    fn check_call_with_signature(
        &mut self,
        args: &[ast::CallArg],
        sig: &FunctionSignature,
        env: &TypeEnv,
        call_span: Span,
    ) -> TypeResult<()> {
        let mut provided: Vec<Option<usize>> = vec![None; sig.params.len()];
        let mut seen_named = false;
        let mut positional_idx = 0;
        let mut seen_arg_names: Vec<String> = Vec::new();

        for (arg_idx, arg) in args.iter().enumerate() {
            if let Some(ref name_ident) = arg.name {
                // Named argument
                seen_named = true;
                let arg_name = name_ident.node.to_string();

                // Check for duplicate named argument
                if seen_arg_names.contains(&arg_name) {
                    return Err(TypeError::DuplicateNamedArgument {
                        name: arg_name,
                        span: name_ident.span,
                    });
                }
                seen_arg_names.push(arg_name.clone());

                // Find matching parameter
                if let Some((param_idx, param_info)) = sig.find_param(&arg_name) {
                    // Check if already provided
                    if provided[param_idx].is_some() {
                        return Err(TypeError::DuplicateNamedArgument {
                            name: arg_name,
                            span: name_ident.span,
                        });
                    }

                    // Type check the argument
                    let arg_type = self.infer_expr(&arg.value, env)?;
                    self.inference.unify(&arg_type, &param_info.ty, arg.value.span)?;
                    provided[param_idx] = Some(arg_idx);
                } else {
                    return Err(TypeError::UnknownNamedArgument {
                        name: arg_name,
                        span: name_ident.span,
                    });
                }
            } else {
                // Positional argument
                if seen_named {
                    return Err(TypeError::PositionalAfterNamed {
                        span: arg.value.span,
                    });
                }

                // Find next unprovided parameter position
                while positional_idx < sig.params.len() && provided[positional_idx].is_some() {
                    positional_idx += 1;
                }

                if positional_idx >= sig.params.len() {
                    return Err(TypeError::TooManyArguments {
                        max_allowed: sig.params.len(),
                        found: args.len(),
                        span: call_span,
                    });
                }

                // Type check the argument
                let arg_type = self.infer_expr(&arg.value, env)?;
                self.inference.unify(&arg_type, &sig.params[positional_idx].ty, arg.value.span)?;
                provided[positional_idx] = Some(arg_idx);
                positional_idx += 1;
            }
        }

        // Check that all required parameters are provided
        for (idx, param) in sig.params.iter().enumerate() {
            if provided[idx].is_none() && !param.has_default {
                return Err(TypeError::MissingRequiredArgument {
                    name: param.name.clone(),
                    span: call_span,
                });
            }
        }

        Ok(())
    }

    /// Infer result type of binary operation
    fn infer_binary_op(
        &mut self,
        op: ast::BinaryOp,
        left: &Type,
        right: &Type,
        span: Span,
    ) -> TypeResult<Type> {
        use ast::BinaryOp::*;
        match op {
            // Arithmetic: both operands must be numeric, result is same type
            Add | Sub | Mul | Div | IntDiv | Mod | Pow => {
                self.inference.unify(left, right, span)?;
                if left.is_numeric() || matches!(left, Type::Var(_)) {
                    Ok(self.inference.apply(left))
                } else if matches!(op, Add) && matches!(left, Type::String) {
                    // String concatenation
                    Ok(Type::String)
                } else {
                    Err(TypeError::Mismatch {
                        expected: "numeric type".to_string(),
                        found: format!("{}", left),
                        span,
                        expected_source: None,
                    })
                }
            }

            // Comparison: both operands same type, result is Bool
            Eq | NotEq | Lt | Gt | LtEq | GtEq | Spaceship => {
                self.inference.unify(left, right, span)?;
                Ok(Type::Bool)
            }

            // Logical: both operands Bool, result is Bool
            And | Or => {
                self.inference.unify(left, &Type::Bool, span)?;
                self.inference.unify(right, &Type::Bool, span)?;
                Ok(Type::Bool)
            }

            // Bitwise: both operands integer, result is same type
            BitAnd | BitOr | BitXor | Shl | Shr => {
                self.inference.unify(left, right, span)?;
                if left.is_integer() || matches!(left, Type::Var(_)) {
                    Ok(self.inference.apply(left))
                } else {
                    Err(TypeError::Mismatch {
                        expected: "integer type".to_string(),
                        found: format!("{}", left),
                        span,
                        expected_source: None,
                    })
                }
            }

            // Range operator
            Range | RangeExclusive => Ok(Type::Named {
                name: "Range".to_string(),
                type_args: vec![self.inference.apply(left)],
            }),

            // In/Is operators - result is Bool
            In | Is | ApproxEq => {
                Ok(Type::Bool)
            }

            // Assignment operators - these shouldn't appear in expressions
            Assign | AddAssign | SubAssign | MulAssign | DivAssign | IntDivAssign
            | ModAssign | BitAndAssign | BitOrAssign | BitXorAssign | ShlAssign | ShrAssign => {
                // Assignment returns Unit
                self.inference.unify(left, right, span)?;
                Ok(Type::Unit)
            }
        }
    }

    /// Infer result type of unary operation
    fn infer_unary_op(
        &mut self,
        op: ast::UnaryOp,
        operand: &Type,
        span: Span,
    ) -> TypeResult<Type> {
        use ast::UnaryOp::*;
        match op {
            Neg => {
                if operand.is_numeric() || matches!(operand, Type::Var(_)) {
                    Ok(operand.clone())
                } else {
                    Err(TypeError::Mismatch {
                        expected: "numeric type".to_string(),
                        found: format!("{}", operand),
                        span,
                        expected_source: None,
                    })
                }
            }
            Not => {
                self.inference.unify(operand, &Type::Bool, span)?;
                Ok(Type::Bool)
            }
            BitNot => {
                if operand.is_integer() || matches!(operand, Type::Var(_)) {
                    Ok(operand.clone())
                } else {
                    Err(TypeError::Mismatch {
                        expected: "integer type".to_string(),
                        found: format!("{}", operand),
                        span,
                        expected_source: None,
                    })
                }
            }
            Ref => Ok(Type::Reference {
                mutable: false,
                inner: Box::new(operand.clone()),
            }),
            Deref => match operand {
                Type::Reference { inner, .. } => Ok(*inner.clone()),
                _ => Ok(self.inference.fresh_var()),
            },
        }
    }

    // ========================================================================
    // Block and Statement Type Checking
    // ========================================================================

    /// Infer the type of a block (returns the type of the last expression, or Unit)
    pub fn infer_block(&mut self, block: &ast::Block, env: &TypeEnv) -> TypeResult<Type> {
        let mut block_env = TypeEnv::with_parent(Rc::new(env.clone()));
        let mut last_type = Type::Unit;

        for stmt in &block.stmts {
            last_type = self.check_stmt(stmt, &mut block_env)?;
        }

        Ok(last_type)
    }

    /// Type check a statement, returning its type (usually Unit, except for expr statements)
    pub fn check_stmt(&mut self, stmt: &ast::Stmt, env: &mut TypeEnv) -> TypeResult<Type> {
        match &stmt.kind {
            ast::StmtKind::Expr(expr) => self.infer_expr(expr, env),

            ast::StmtKind::Let { pattern, ty, value } => {
                let value_type = self.infer_expr(value, env)?;

                let expected_type = if let Some(t) = ty {
                    let resolved = self.resolve_type(t)?;
                    // Unify with expected type first so error shows "expected <annotation>, found <value>"
                    self.inference.unify(&resolved, &value_type, stmt.span)?;
                    resolved
                } else {
                    self.inference.apply(&value_type)
                };

                // Bind pattern variables
                self.bind_pattern(pattern, &expected_type, env)?;
                Ok(Type::Unit)
            }

            ast::StmtKind::Var { name, ty, value } => {
                let value_type = self.infer_expr(value, env)?;

                let var_type = if let Some(t) = ty {
                    let resolved = self.resolve_type(t)?;
                    self.inference.unify(&value_type, &resolved, stmt.span)?;
                    resolved
                } else {
                    self.inference.apply(&value_type)
                };

                env.define_var(name.node.to_string(), var_type);
                Ok(Type::Unit)
            }

            ast::StmtKind::Const { name, ty, value } => {
                let const_name = name.node.to_string();
                let value_type = self.infer_expr(value, env)?;

                let const_type = if let Some(t) = ty {
                    let resolved = self.resolve_type(t)?;
                    self.inference.unify(&value_type, &resolved, stmt.span)?;
                    resolved
                } else {
                    self.inference.apply(&value_type)
                };

                // Try to evaluate the const expression at compile time
                if let Ok(const_val) = self.eval_const_expr(value) {
                    // Store for const propagation
                    self.const_values.insert(const_name.clone(), const_val);
                }

                env.define_var(const_name, const_type);
                Ok(Type::Unit)
            }

            ast::StmtKind::Assign { target, op, value } => {
                let target_type = self.infer_expr(target, env)?;
                let value_type = self.infer_expr(value, env)?;

                if let Some(bin_op) = op {
                    // Compound assignment: x += y means x = x + y
                    let result_type = self.infer_binary_op(*bin_op, &target_type, &value_type, stmt.span)?;
                    self.inference.unify(&target_type, &result_type, stmt.span)?;
                } else {
                    self.inference.unify(&target_type, &value_type, stmt.span)?;
                }

                Ok(Type::Unit)
            }

            ast::StmtKind::Return(expr) => {
                // Check if we're in a defer context - return cannot be used in defer
                if self.in_defer_context {
                    return Err(TypeError::ControlFlowInDefer {
                        statement: "return".to_string(),
                        span: stmt.span,
                    });
                }

                let actual_type = if let Some(e) = expr {
                    self.infer_expr(e, env)?
                } else {
                    Type::Unit
                };

                // Validate return type against the enclosing function's declared return type
                if let Some(expected_type) = &self.current_return_type {
                    let expected_resolved = self.inference.apply(expected_type);
                    let actual_resolved = self.inference.apply(&actual_type);

                    // Try to unify - if it fails, produce a specialized return type error
                    if self.inference.unify(&actual_resolved, &expected_resolved, stmt.span).is_err() {
                        return Err(TypeError::ReturnTypeMismatch {
                            expected: expected_resolved.to_string(),
                            found: actual_resolved.to_string(),
                            span: stmt.span,
                        });
                    }
                }

                // Return statements produce the Never type (they don't continue)
                Ok(Type::Never)
            }

            ast::StmtKind::If {
                condition,
                then_branch,
                elsif_branches,
                else_branch,
            } => {
                let cond_type = self.infer_expr(condition, env)?;
                self.inference.unify(&cond_type, &Type::Bool, condition.span)?;

                self.infer_block(then_branch, env)?;

                for (elsif_cond, elsif_body) in elsif_branches {
                    let elsif_cond_type = self.infer_expr(elsif_cond, env)?;
                    self.inference.unify(&elsif_cond_type, &Type::Bool, elsif_cond.span)?;
                    self.infer_block(elsif_body, env)?;
                }

                if let Some(else_body) = else_branch {
                    self.infer_block(else_body, env)?;
                }

                Ok(Type::Unit)
            }

            ast::StmtKind::While { condition, body } => {
                // Type check the condition - must be Bool
                let cond_type = self.infer_expr(condition, env)?;
                self.inference.unify(&cond_type, &Type::Bool, condition.span)?;

                // Push loop context with Unit break type (while loops don't return values via break)
                self.loop_context_stack.push(LoopContext {
                    label: None,
                    break_type: Type::Unit,
                    span: stmt.span,
                });

                // Create a child scope for the loop body
                let loop_env = TypeEnv::with_parent(Rc::new(env.clone()));
                let result = self.infer_block(body, &loop_env);

                // Pop loop context
                self.loop_context_stack.pop();

                result?;
                Ok(Type::Unit)
            }

            ast::StmtKind::Loop { body } => {
                // Loop expressions can return a value via break
                // Create a fresh type variable for the break type
                let break_type = self.inference.fresh_var();

                // Push loop context
                self.loop_context_stack.push(LoopContext {
                    label: None,
                    break_type: break_type.clone(),
                    span: stmt.span,
                });

                // Create a child scope for the loop body
                let loop_env = TypeEnv::with_parent(Rc::new(env.clone()));
                let result = self.infer_block(body, &loop_env);

                // Pop loop context
                self.loop_context_stack.pop();

                result?;

                // The loop's type is the break type (or Never if it never breaks)
                // For now, return the break type (will be Unit if no break with value)
                let resolved = self.inference.apply(&break_type);
                if matches!(resolved, Type::Var(_)) {
                    // No break with value was found, default to Unit
                    Ok(Type::Unit)
                } else {
                    Ok(resolved)
                }
            }

            ast::StmtKind::For { pattern, iterable, body } => {
                let iter_type = self.infer_expr(iterable, env)?;
                let resolved_iter_type = self.inference.apply(&iter_type);

                // Determine element type from iterable
                let elem_type = match &resolved_iter_type {
                    Type::Array(elem) => (**elem).clone(),
                    Type::Named { name, type_args } if name == "Range" => {
                        type_args.first().cloned().unwrap_or(Type::Int)
                    }
                    // Map iteration yields (key, value) tuples
                    Type::Map(key, value) => {
                        Type::Tuple(vec![(**key).clone(), (**value).clone()])
                    }
                    // Allow type variables for generic iteration
                    Type::Var(_) => self.inference.fresh_var(),
                    // Other types are not iterable
                    other => {
                        return Err(TypeError::NotIterable {
                            found: format!("{}", other),
                            span: iterable.span,
                        });
                    }
                };

                // Push loop context (for loops don't return values via break)
                self.loop_context_stack.push(LoopContext {
                    label: None,
                    break_type: Type::Unit,
                    span: stmt.span,
                });

                // Create a child scope for the loop body and bind pattern
                let mut for_env = TypeEnv::with_parent(Rc::new(env.clone()));
                self.bind_pattern(pattern, &elem_type, &mut for_env)?;
                let result = self.infer_block(body, &for_env);

                // Pop loop context
                self.loop_context_stack.pop();

                result?;
                Ok(Type::Unit)
            }

            ast::StmtKind::Break(value) => {
                // Check if we're in a defer context
                if self.in_defer_context {
                    return Err(TypeError::ControlFlowInDefer {
                        statement: "break".to_string(),
                        span: stmt.span,
                    });
                }

                // Check if we're inside a loop
                if self.loop_context_stack.is_empty() {
                    return Err(TypeError::BreakOutsideLoop { span: stmt.span });
                }

                // Get the current loop's expected break type
                let loop_ctx = self.loop_context_stack.last().unwrap();
                let expected_break_type = loop_ctx.break_type.clone();

                // Type check the break value if present
                if let Some(val_expr) = value {
                    let value_type = self.infer_expr(val_expr, env)?;
                    // Unify with the expected break type
                    self.inference.unify(&value_type, &expected_break_type, val_expr.span)?;
                } else {
                    // break without value is equivalent to break ()
                    self.inference.unify(&Type::Unit, &expected_break_type, stmt.span)?;
                }

                Ok(Type::Never)
            }

            ast::StmtKind::Continue => {
                // Check if we're in a defer context
                if self.in_defer_context {
                    return Err(TypeError::ControlFlowInDefer {
                        statement: "continue".to_string(),
                        span: stmt.span,
                    });
                }

                // Check if we're inside a loop
                if self.loop_context_stack.is_empty() {
                    return Err(TypeError::ContinueOutsideLoop { span: stmt.span });
                }

                Ok(Type::Never)
            }

            ast::StmtKind::Defer(expr) => {
                // Save and set defer context
                let was_in_defer = self.in_defer_context;
                let prev_defer_span = self.current_defer_span;
                self.in_defer_context = true;
                self.current_defer_span = Some(stmt.span);

                // Type check the deferred expression
                let expr_type = self.infer_expr(expr, env)?;

                // Restore defer context
                self.in_defer_context = was_in_defer;
                self.current_defer_span = prev_defer_span;

                // Check that deferred expressions return Unit (warn if they don't)
                // The result of a defer expression is discarded, so non-Unit is likely a mistake
                let resolved_type = self.inference.apply(&expr_type);
                if !matches!(resolved_type, Type::Unit | Type::Never) {
                    // For now, we allow non-Unit but could emit a warning
                    // In strict mode, this would be an error:
                    // return Err(TypeError::DeferNonUnit {
                    //     found: resolved_type.to_string(),
                    //     span: expr.span,
                    // });
                }

                // Validate captured variables are in scope at defer point
                self.validate_defer_captures(expr, env, stmt.span)?;

                Ok(Type::Unit)
            }

            ast::StmtKind::Match { scrutinee, arms } => {
                let scrutinee_type = self.infer_expr(scrutinee, env)?;

                for arm in arms {
                    let mut arm_env = TypeEnv::with_parent(Rc::new(env.clone()));
                    self.check_pattern(&arm.pattern, &scrutinee_type, &mut arm_env)?;

                    if let Some(guard) = &arm.guard {
                        let guard_type = self.infer_expr(guard, &arm_env)?;
                        self.inference.unify(&guard_type, &Type::Bool, guard.span)?;
                    }

                    match &arm.body {
                        ast::MatchArmBody::Expr(expr) => {
                            self.infer_expr(expr, &arm_env)?;
                        }
                        ast::MatchArmBody::Block(block) => {
                            self.infer_block(block, &arm_env)?;
                        }
                    }
                }

                // TODO: Check for exhaustiveness (requires pattern analysis implementation)
                // let patterns: Vec<_> = arms.iter().map(|a| &a.pattern).collect();
                // self.check_exhaustiveness(&scrutinee_type, &patterns, scrutinee.span)?;

                Ok(Type::Unit)
            }

            ast::StmtKind::Unless { condition, body, else_branch } => {
                let cond_type = self.infer_expr(condition, env)?;
                self.inference.unify(&cond_type, &Type::Bool, condition.span)?;
                self.infer_block(body, env)?;
                if let Some(else_body) = else_branch {
                    self.infer_block(else_body, env)?;
                }
                Ok(Type::Unit)
            }

            ast::StmtKind::Unsafe(block) => self.infer_block(block, env),

            ast::StmtKind::Item(item) => {
                self.check_item(item)?;
                Ok(Type::Unit)
            }
        }
    }

    // ========================================================================
    // Pattern Type Checking
    // ========================================================================

    /// Bind pattern variables to their types in the environment
    ///
    /// This method traverses a pattern and binds any variables to their appropriate
    /// types in the environment. This enables guard expressions and match arm bodies
    /// to reference pattern-bound variables.
    pub fn bind_pattern(
        &mut self,
        pattern: &ast::Pattern,
        ty: &Type,
        env: &mut TypeEnv,
    ) -> TypeResult<()> {
        match &pattern.kind {
            ast::PatternKind::Wildcard => Ok(()),

            ast::PatternKind::Ident(name) => {
                env.define_var(name.to_string(), ty.clone());
                Ok(())
            }

            ast::PatternKind::Literal(expr) => {
                let lit_type = self.infer_expr(expr, env)?;
                self.inference.unify(ty, &lit_type, pattern.span)?;
                Ok(())
            }

            ast::PatternKind::Tuple(patterns) => {
                match self.inference.apply(ty) {
                    Type::Tuple(types) if types.len() == patterns.len() => {
                        for (pat, elem_ty) in patterns.iter().zip(types.iter()) {
                            self.bind_pattern(pat, elem_ty, env)?;
                        }
                        Ok(())
                    }
                    Type::Var(_) => {
                        // Create tuple type with fresh variables
                        let elem_types: Vec<Type> = patterns
                            .iter()
                            .map(|_| self.inference.fresh_var())
                            .collect();
                        let tuple_type = Type::Tuple(elem_types.clone());
                        self.inference.unify(ty, &tuple_type, pattern.span)?;

                        for (pat, elem_ty) in patterns.iter().zip(elem_types.iter()) {
                            self.bind_pattern(pat, elem_ty, env)?;
                        }
                        Ok(())
                    }
                    _ => Err(TypeError::Mismatch {
                        expected: "tuple".to_string(),
                        found: format!("{}", ty),
                        span: pattern.span,
                        expected_source: None,
                    }),
                }
            }

            ast::PatternKind::Array { elements, rest } => {
                let elem_type = match self.inference.apply(ty) {
                    Type::Array(elem) => *elem,
                    Type::Var(_) => {
                        let elem = self.inference.fresh_var();
                        self.inference.unify(ty, &Type::Array(Box::new(elem.clone())), pattern.span)?;
                        elem
                    }
                    _ => return Err(TypeError::Mismatch {
                        expected: "array".to_string(),
                        found: format!("{}", ty),
                        span: pattern.span,
                        expected_source: None,
                    }),
                };

                for pat in elements {
                    self.bind_pattern(pat, &elem_type, env)?;
                }

                if let Some(rest_pat) = rest {
                    self.bind_pattern(rest_pat, ty, env)?;
                }

                Ok(())
            }

            // Binding pattern: `x @ Pattern` binds both the name and inner pattern
            ast::PatternKind::Binding { name, pattern: inner } => {
                // Bind the name to the entire matched type
                env.define_var(name.node.to_string(), ty.clone());
                // Also bind any variables in the inner pattern
                self.bind_pattern(inner, ty, env)
            }

            // Typed pattern: `x: Type` checks the type annotation and binds the pattern
            ast::PatternKind::Typed { pattern: inner, ty: type_expr } => {
                let annotated_type = self.resolve_type(type_expr)?;
                self.inference.unify(ty, &annotated_type, pattern.span)?;
                self.bind_pattern(inner, &annotated_type, env)
            }

            // Or pattern: `A | B` - bind variables from the first alternative
            // (variables must be consistent across all alternatives in a well-formed program)
            ast::PatternKind::Or(patterns) => {
                if let Some(first) = patterns.first() {
                    self.bind_pattern(first, ty, env)?;
                }
                // Verify other alternatives have consistent types
                for pat in patterns.iter().skip(1) {
                    let mut check_env = TypeEnv::with_parent(Rc::new(env.clone()));
                    self.bind_pattern(pat, ty, &mut check_env)?;
                }
                Ok(())
            }

            // Guard pattern: `x if condition` - bind the inner pattern
            // (guard condition is type-checked separately at the match arm level)
            ast::PatternKind::Guard { pattern: inner, .. } => {
                self.bind_pattern(inner, ty, env)
            }

            // Rest pattern: `...` or `...name` - binds the rest to an array of remaining elements
            ast::PatternKind::Rest(opt_name) => {
                if let Some(name) = opt_name {
                    // The rest pattern captures remaining elements as an array
                    env.define_var(name.node.to_string(), ty.clone());
                }
                Ok(())
            }

            // Struct pattern: `Point { x, y }` or `{ x, y }`
            ast::PatternKind::Struct { name, fields } => {
                // Look up struct field types from the expected type
                let struct_fields_lookup: Option<Vec<(String, Type)>> = match self.inference.apply(ty) {
                    Type::Named { name: type_name, type_args } => {
                        self.struct_fields.get(&type_name).map(|flds| {
                            if type_args.is_empty() {
                                flds.clone()
                            } else if let Some(tpdefs) = self.generic_type_params.get(&type_name) {
                                let subst: FxHashMap<String, Type> = tpdefs.iter()
                                    .map(|p| p.name.clone())
                                    .zip(type_args.iter().cloned())
                                    .collect();
                                flds.iter().map(|(n, ft)| (n.clone(), self.substitute_type(ft, &subst))).collect()
                            } else {
                                flds.clone()
                            }
                        })
                    }
                    _ => name.as_ref().and_then(|n| self.struct_fields.get(&n.node.to_string()).cloned())
                };

                for field in fields {
                    let fname = field.name.node.to_string();
                    let ftype = struct_fields_lookup.as_ref()
                        .and_then(|sf| sf.iter().find(|(n, _)| n == &fname))
                        .map(|(_, t)| t.clone())
                        .unwrap_or_else(|| self.inference.fresh_var());

                    if let Some(ref fpat) = field.pattern {
                        self.bind_pattern(fpat, &ftype, env)?;
                    } else {
                        env.define_var(fname, ftype);
                    }
                }
                Ok(())
            }

            // Variant pattern: `Some(x)` or `None` or `Color::Red`
            ast::PatternKind::Variant { path, variant, fields } => {
                let vname = variant.node.to_string();

                // Find enum info from expected type
                let (enum_info, targs) = match self.inference.apply(ty) {
                    Type::Named { name: ename, type_args } => {
                        (self.enum_variants.get(&ename).cloned(), type_args)
                    }
                    Type::Optional(inner) => {
                        let mut vs = FxHashMap::default();
                        vs.insert("Some".to_string(), VariantData::Tuple(vec![(*inner).clone()]));
                        vs.insert("None".to_string(), VariantData::Unit);
                        let info = EnumVariantInfo {
                            enum_name: "Optional".to_string(),
                            type_params: vec![TypeParamDef { name: "T".to_string(), bounds: vec![] }],
                            variants: vs,
                            type_param_vars: FxHashMap::default(), // Not needed - variant data already has concrete types
                        };
                        (Some(info), vec![(*inner).clone()])
                    }
                    Type::Result(ok, err) => {
                        let mut vs = FxHashMap::default();
                        vs.insert("Ok".to_string(), VariantData::Tuple(vec![(*ok).clone()]));
                        vs.insert("Err".to_string(), VariantData::Tuple(vec![(*err).clone()]));
                        let info = EnumVariantInfo {
                            enum_name: "Result".to_string(),
                            type_params: vec![
                                TypeParamDef { name: "T".to_string(), bounds: vec![] },
                                TypeParamDef { name: "E".to_string(), bounds: vec![] },
                            ],
                            variants: vs,
                            type_param_vars: FxHashMap::default(), // Not needed - variant data already has concrete types
                        };
                        (Some(info), vec![(*ok).clone(), (*err).clone()])
                    }
                    _ => {
                        if !path.is_empty() {
                            (self.enum_variants.get(&path[0].node.to_string()).cloned(), vec![])
                        } else {
                            (None, vec![])
                        }
                    }
                };

                if let Some(info) = enum_info {
                    if let Some(vdata) = info.variants.get(&vname) {
                        // Build name-based substitution for Type::Named references
                        let subst: FxHashMap<String, Type> = info.type_params.iter()
                            .map(|p| p.name.clone())
                            .zip(targs.iter().cloned())
                            .collect();

                        // Build TypeVar-based substitution for Type::Var references
                        // This handles when generic enums store Type::Var in variant data
                        let var_subst: FxHashMap<TypeVar, Type> = info.type_param_vars.iter()
                            .filter_map(|(name, var)| {
                                subst.get(name).map(|ty| (*var, ty.clone()))
                            })
                            .collect();

                        match vdata {
                            VariantData::Unit => {
                                if fields.is_some() && !fields.as_ref().unwrap().is_empty() {
                                    return Err(TypeError::Mismatch {
                                        expected: format!("unit variant `{}`", vname),
                                        found: "pattern with fields".to_string(),
                                        span: pattern.span,
                                        expected_source: None,
                                    });
                                }
                            }
                            VariantData::Tuple(ftypes) => {
                                if let Some(fpats) = fields {
                                    if fpats.len() != ftypes.len() {
                                        return Err(TypeError::WrongTypeArity {
                                            expected: ftypes.len(),
                                            found: fpats.len(),
                                            span: pattern.span,
                                        });
                                    }
                                    for (fpat, fty) in fpats.iter().zip(ftypes.iter()) {
                                        // First substitute Type::Named references, then Type::Var references
                                        let intermediate = self.substitute_type(fty, &subst);
                                        let concrete = self.substitute_type_vars(&intermediate, &var_subst);
                                        self.bind_pattern(fpat, &concrete, env)?;
                                    }
                                }
                            }
                            VariantData::Struct(sfields) => {
                                if let Some(fpats) = fields {
                                    for (fpat, (_, fty)) in fpats.iter().zip(sfields.iter()) {
                                        // First substitute Type::Named references, then Type::Var references
                                        let intermediate = self.substitute_type(fty, &subst);
                                        let concrete = self.substitute_type_vars(&intermediate, &var_subst);
                                        self.bind_pattern(fpat, &concrete, env)?;
                                    }
                                }
                            }
                        }
                    } else if let Some(fpats) = fields {
                        for fpat in fpats {
                            let ftype = self.inference.fresh_var();
                            self.bind_pattern(fpat, &ftype, env)?;
                        }
                    }
                } else if let Some(fpats) = fields {
                    for fpat in fpats {
                        let ftype = self.inference.fresh_var();
                        self.bind_pattern(fpat, &ftype, env)?;
                    }
                }
                Ok(())
            }

            // Range pattern: `1..10` - no variable bindings
            ast::PatternKind::Range { start, end, .. } => {
                // Verify the range bounds match the expected type
                let start_type = self.infer_expr(start, env)?;
                let end_type = self.infer_expr(end, env)?;
                self.inference.unify(ty, &start_type, pattern.span)?;
                self.inference.unify(ty, &end_type, pattern.span)?;
                Ok(())
            }
        }
    }

    /// Check a pattern against an expected type
    fn check_pattern(
        &mut self,
        pattern: &ast::Pattern,
        expected: &Type,
        env: &mut TypeEnv,
    ) -> TypeResult<()> {
        self.bind_pattern(pattern, expected, env)
    }

    // =========================================================================
    // Exhaustiveness Checking
    // =========================================================================

    /// Check if a set of patterns exhaustively covers all possible values
    /// of the scrutinee type.
    fn check_exhaustiveness(
        &self,
        scrutinee_type: &Type,
        patterns: &[&ast::Pattern],
        span: Span,
    ) -> TypeResult<()> {
        // Check if any pattern is a catch-all (wildcard or identifier binding)
        if self.has_catchall_pattern(patterns) {
            return Ok(());
        }

        let resolved = self.inference.apply(scrutinee_type);

        match resolved {
            Type::Bool => self.check_bool_exhaustiveness(patterns, span),
            Type::Optional(_) => self.check_optional_exhaustiveness(patterns, span),
            Type::Result(_, _) => self.check_result_exhaustiveness(patterns, span),
            Type::Named { .. } => {
                // For named types (enums), we'd need to look up the variants
                // For now, pass without warning - full implementation would
                // require looking up enum definitions
                Ok(())
            }
            // For other types (Int, String, etc.), require a catch-all pattern
            // since they have infinite values
            Type::Int | Type::Float | Type::String | Type::Char => {
                Err(TypeError::NonExhaustivePatterns {
                    missing: format!("patterns of type `{}` are not exhaustive without a catch-all pattern (`_`)", resolved),
                    span,
                })
            }
            _ => Ok(()), // Other types pass for now
        }
    }

    /// Check if any pattern is a catch-all (wildcard, identifier, or Or containing catch-all)
    fn has_catchall_pattern(&self, patterns: &[&ast::Pattern]) -> bool {
        for pattern in patterns {
            if self.is_catchall_pattern(pattern) {
                return true;
            }
        }
        false
    }

    /// Check if a single pattern is a catch-all
    fn is_catchall_pattern(&self, pattern: &ast::Pattern) -> bool {
        match &pattern.kind {
            ast::PatternKind::Wildcard => true,
            ast::PatternKind::Ident(_) => true,
            ast::PatternKind::Binding { pattern: inner, .. } => {
                self.is_catchall_pattern(inner)
            }
            ast::PatternKind::Or(patterns) => {
                patterns.iter().any(|p| self.is_catchall_pattern(p))
            }
            ast::PatternKind::Typed { pattern: inner, .. } => {
                self.is_catchall_pattern(inner)
            }
            _ => false,
        }
    }

    /// Check exhaustiveness for Bool type (must cover true and false)
    fn check_bool_exhaustiveness(
        &self,
        patterns: &[&ast::Pattern],
        span: Span,
    ) -> TypeResult<()> {
        let mut has_true = false;
        let mut has_false = false;

        for pattern in patterns {
            self.check_bool_pattern(pattern, &mut has_true, &mut has_false);
        }

        if !has_true && !has_false {
            Err(TypeError::NonExhaustivePatterns {
                missing: "`true` and `false` not covered".to_string(),
                span,
            })
        } else if !has_true {
            Err(TypeError::NonExhaustivePatterns {
                missing: "`true` not covered".to_string(),
                span,
            })
        } else if !has_false {
            Err(TypeError::NonExhaustivePatterns {
                missing: "`false` not covered".to_string(),
                span,
            })
        } else {
            Ok(())
        }
    }

    /// Recursively check a pattern for boolean literal coverage
    fn check_bool_pattern(&self, pattern: &ast::Pattern, has_true: &mut bool, has_false: &mut bool) {
        match &pattern.kind {
            ast::PatternKind::Wildcard | ast::PatternKind::Ident(_) => {
                *has_true = true;
                *has_false = true;
            }
            ast::PatternKind::Literal(expr) => {
                if let ast::ExprKind::Bool(val) = &expr.kind {
                    if *val {
                        *has_true = true;
                    } else {
                        *has_false = true;
                    }
                }
            }
            ast::PatternKind::Or(patterns) => {
                for p in patterns {
                    self.check_bool_pattern(p, has_true, has_false);
                }
            }
            ast::PatternKind::Binding { pattern: inner, .. } => {
                self.check_bool_pattern(inner, has_true, has_false);
            }
            _ => {}
        }
    }

    /// Check exhaustiveness for Optional type (must cover Some and None)
    fn check_optional_exhaustiveness(
        &self,
        patterns: &[&ast::Pattern],
        span: Span,
    ) -> TypeResult<()> {
        let mut has_some = false;
        let mut has_none = false;

        for pattern in patterns {
            self.check_optional_pattern(pattern, &mut has_some, &mut has_none);
        }

        if !has_some && !has_none {
            Err(TypeError::NonExhaustivePatterns {
                missing: "`Some(_)` and `None` not covered".to_string(),
                span,
            })
        } else if !has_some {
            Err(TypeError::NonExhaustivePatterns {
                missing: "`Some(_)` not covered".to_string(),
                span,
            })
        } else if !has_none {
            Err(TypeError::NonExhaustivePatterns {
                missing: "`None` not covered".to_string(),
                span,
            })
        } else {
            Ok(())
        }
    }

    /// Recursively check a pattern for Optional variant coverage
    fn check_optional_pattern(&self, pattern: &ast::Pattern, has_some: &mut bool, has_none: &mut bool) {
        match &pattern.kind {
            ast::PatternKind::Wildcard | ast::PatternKind::Ident(_) => {
                *has_some = true;
                *has_none = true;
            }
            ast::PatternKind::Variant { variant, .. } => {
                let name_str = variant.node.to_string();
                if name_str == "Some" {
                    *has_some = true;
                } else if name_str == "None" {
                    *has_none = true;
                }
            }
            ast::PatternKind::Or(patterns) => {
                for p in patterns {
                    self.check_optional_pattern(p, has_some, has_none);
                }
            }
            ast::PatternKind::Binding { pattern: inner, .. } => {
                self.check_optional_pattern(inner, has_some, has_none);
            }
            _ => {}
        }
    }

    /// Check exhaustiveness for Result type (must cover Ok and Err)
    fn check_result_exhaustiveness(
        &self,
        patterns: &[&ast::Pattern],
        span: Span,
    ) -> TypeResult<()> {
        let mut has_ok = false;
        let mut has_err = false;

        for pattern in patterns {
            self.check_result_pattern(pattern, &mut has_ok, &mut has_err);
        }

        if !has_ok && !has_err {
            Err(TypeError::NonExhaustivePatterns {
                missing: "`Ok(_)` and `Err(_)` not covered".to_string(),
                span,
            })
        } else if !has_ok {
            Err(TypeError::NonExhaustivePatterns {
                missing: "`Ok(_)` not covered".to_string(),
                span,
            })
        } else if !has_err {
            Err(TypeError::NonExhaustivePatterns {
                missing: "`Err(_)` not covered".to_string(),
                span,
            })
        } else {
            Ok(())
        }
    }

    /// Recursively check a pattern for Result variant coverage
    fn check_result_pattern(&self, pattern: &ast::Pattern, has_ok: &mut bool, has_err: &mut bool) {
        match &pattern.kind {
            ast::PatternKind::Wildcard | ast::PatternKind::Ident(_) => {
                *has_ok = true;
                *has_err = true;
            }
            ast::PatternKind::Variant { variant, .. } => {
                let name_str = variant.node.to_string();
                if name_str == "Ok" {
                    *has_ok = true;
                } else if name_str == "Err" {
                    *has_err = true;
                }
            }
            ast::PatternKind::Or(patterns) => {
                for p in patterns {
                    self.check_result_pattern(p, has_ok, has_err);
                }
            }
            ast::PatternKind::Binding { pattern: inner, .. } => {
                self.check_result_pattern(inner, has_ok, has_err);
            }
            _ => {}
        }
    }

    /// Get inference errors
    pub fn errors(&self) -> &[TypeError] {
        self.inference.errors()
    }
}

// =========================================================================
// Closure Capture Analysis Types
// =========================================================================

/// How a variable is captured by a closure
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureMode {
    /// Variable is borrowed immutably (read-only access)
    Borrow,
    /// Variable is borrowed mutably (read-write access)
    BorrowMut,
    /// Variable is moved into the closure (ownership transfer)
    Move,
}

/// Information about a captured variable
#[derive(Debug, Clone, PartialEq)]
pub struct CaptureInfo {
    /// Name of the captured variable
    pub name: String,
    /// Type of the captured variable
    pub ty: Type,
    /// How the variable is captured
    pub mode: CaptureMode,
    /// Whether the original variable was declared mutable
    pub is_mutable: bool,
    /// Span where the variable is defined (for error reporting)
    pub def_span: Option<Span>,
}

impl CaptureInfo {
    /// Create a new CaptureInfo
    pub fn new(name: String, ty: Type, mode: CaptureMode, is_mutable: bool) -> Self {
        Self {
            name,
            ty,
            mode,
            is_mutable,
            def_span: None,
        }
    }

    /// Create with a definition span
    pub fn with_span(mut self, span: Span) -> Self {
        self.def_span = Some(span);
        self
    }

    /// Check if this capture is valid for spawn (must be Transfer)
    pub fn is_spawn_safe(&self) -> bool {
        self.ty.is_transfer()
    }

    /// Check if this is a mutable capture
    pub fn is_mut_capture(&self) -> bool {
        matches!(self.mode, CaptureMode::BorrowMut)
    }

    /// Check if this is a move capture
    pub fn is_move(&self) -> bool {
        matches!(self.mode, CaptureMode::Move)
    }

    /// Check if the capture requires ownership (move or mutable borrow)
    pub fn requires_ownership(&self) -> bool {
        matches!(self.mode, CaptureMode::Move | CaptureMode::BorrowMut)
    }
}

/// Information about a variable in the capture environment
#[derive(Debug, Clone)]
pub struct VarInfo {
    /// Type of the variable
    pub ty: Type,
    /// Whether the variable was declared as mutable (var vs let)
    pub is_mutable: bool,
    /// Span where the variable was defined
    pub def_span: Option<Span>,
}

impl VarInfo {
    pub fn new(ty: Type, is_mutable: bool) -> Self {
        Self {
            ty,
            is_mutable,
            def_span: None,
        }
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.def_span = Some(span);
        self
    }
}

/// Environment that tracks both types and mutability for capture analysis
#[derive(Debug, Clone, Default)]
pub struct CaptureEnv {
    /// Variables with their type and mutability info
    variables: std::collections::HashMap<String, VarInfo>,
    /// Parent scope
    parent: Option<Rc<CaptureEnv>>,
}

impl CaptureEnv {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_parent(parent: Rc<CaptureEnv>) -> Self {
        Self {
            parent: Some(parent),
            ..Default::default()
        }
    }

    /// Create from a TypeEnv, assuming all variables are immutable
    /// Use define_var_mut to override specific variables as mutable
    pub fn from_type_env(env: &TypeEnv) -> Self {
        let capture_env = Self::new();
        // Note: TypeEnv doesn't expose iteration, so this just creates an empty
        // capture env. Users should populate it manually for now.
        let _ = env;
        capture_env
    }

    pub fn define_var(&mut self, name: String, ty: Type, is_mutable: bool) {
        self.variables.insert(name, VarInfo::new(ty, is_mutable));
    }

    pub fn define_var_with_span(&mut self, name: String, ty: Type, is_mutable: bool, span: Span) {
        self.variables.insert(name, VarInfo::new(ty, is_mutable).with_span(span));
    }

    pub fn lookup_var(&self, name: &str) -> Option<&VarInfo> {
        self.variables
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|p| p.lookup_var(name)))
    }

    pub fn lookup_type(&self, name: &str) -> Option<Type> {
        self.lookup_var(name).map(|info| info.ty.clone())
    }

    pub fn is_mutable(&self, name: &str) -> bool {
        self.lookup_var(name).map(|info| info.is_mutable).unwrap_or(false)
    }
}

/// Tracks how a captured variable is used within a closure
#[derive(Debug, Clone, Default)]
struct CaptureUsage {
    /// Variable is read (used in an expression)
    is_read: bool,
    /// Variable is written to (assigned)
    is_written: bool,
    /// Variable is moved (passed by value to a function expecting ownership)
    is_moved: bool,
}

impl TypeChecker {
    // =========================================================================
    // Spawn Transfer Checking
    // =========================================================================

    /// Analyze a lambda expression and collect its captured variables.
    ///
    /// This identifies free variables in the lambda body that are captured
    /// from the enclosing scope. Used for spawn transfer checking.
    ///
    /// # Arguments
    ///
    /// * `lambda_params` - Parameters of the lambda
    /// * `lambda_body` - Body expression of the lambda
    /// * `env` - The type environment at the lambda definition site
    ///
    /// # Returns
    ///
    /// A vector of (variable_name, type) pairs for captured variables.
    pub fn collect_lambda_captures(
        &self,
        lambda_params: &[ast::Param],
        lambda_body: &ast::Expr,
        env: &TypeEnv,
    ) -> Vec<(String, Type)> {
        let mut captures = Vec::new();
        let param_names: std::collections::HashSet<_> = lambda_params
            .iter()
            .map(|p| p.name.node.to_string())
            .collect();

        // Collect free variables from the lambda body
        self.collect_free_vars(lambda_body, &param_names, env, &mut captures);

        captures
    }

    /// Recursively collect free variables from an expression.
    fn collect_free_vars(
        &self,
        expr: &ast::Expr,
        bound_vars: &std::collections::HashSet<String>,
        env: &TypeEnv,
        captures: &mut Vec<(String, Type)>,
    ) {
        match &expr.kind {
            ast::ExprKind::Ident(name) => {
                let name_str = name.to_string();
                if !bound_vars.contains(&name_str) {
                    // It's a free variable - check if it's in the environment
                    if let Some(ty) = env.lookup_var(&name_str) {
                        // Only add if not already captured
                        if !captures.iter().any(|(n, _)| n == &name_str) {
                            captures.push((name_str, ty.clone()));
                        }
                    }
                }
            }

            // Binary operations
            ast::ExprKind::Binary { left, right, .. } => {
                self.collect_free_vars(left, bound_vars, env, captures);
                self.collect_free_vars(right, bound_vars, env, captures);
            }

            // Unary operations
            ast::ExprKind::Unary { operand, .. } => {
                self.collect_free_vars(operand, bound_vars, env, captures);
            }

            // Arrays and tuples
            ast::ExprKind::Array(elements) | ast::ExprKind::Tuple(elements) => {
                for elem in elements {
                    self.collect_free_vars(elem, bound_vars, env, captures);
                }
            }

            // Map literals
            ast::ExprKind::Map(pairs) => {
                for (key, value) in pairs {
                    self.collect_free_vars(key, bound_vars, env, captures);
                    self.collect_free_vars(value, bound_vars, env, captures);
                }
            }

            // Field access
            ast::ExprKind::Field { object, .. } => {
                self.collect_free_vars(object, bound_vars, env, captures);
            }

            // Index access
            ast::ExprKind::Index { object, index } => {
                self.collect_free_vars(object, bound_vars, env, captures);
                self.collect_free_vars(index, bound_vars, env, captures);
            }

            // Method calls (args are Expr directly)
            ast::ExprKind::MethodCall { object, args, .. } => {
                self.collect_free_vars(object, bound_vars, env, captures);
                for arg in args {
                    self.collect_free_vars(arg, bound_vars, env, captures);
                }
            }

            // Function calls (args are CallArg with value field)
            ast::ExprKind::Call { func, args } => {
                self.collect_free_vars(func, bound_vars, env, captures);
                for arg in args {
                    self.collect_free_vars(&arg.value, bound_vars, env, captures);
                }
            }

            // Block expressions
            ast::ExprKind::Block(block) => {
                // Need to track new bindings in the block
                let mut block_bound = bound_vars.clone();
                for stmt in &block.stmts {
                    self.collect_free_vars_stmt(stmt, &mut block_bound, env, captures);
                }
            }

            // If expressions
            ast::ExprKind::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                self.collect_free_vars(condition, bound_vars, env, captures);
                let mut then_bound = bound_vars.clone();
                for stmt in &then_branch.stmts {
                    self.collect_free_vars_stmt(stmt, &mut then_bound, env, captures);
                }
                if let Some(else_block) = else_branch {
                    let mut else_bound = bound_vars.clone();
                    for stmt in &else_block.stmts {
                        self.collect_free_vars_stmt(stmt, &mut else_bound, env, captures);
                    }
                }
            }

            // Nested lambdas - traverse into their bodies but add params as bound
            ast::ExprKind::Lambda { params, body } => {
                let mut lambda_bound = bound_vars.clone();
                for p in params {
                    lambda_bound.insert(p.name.node.to_string());
                }
                self.collect_free_vars(body, &lambda_bound, env, captures);
            }

            ast::ExprKind::BlockLambda { params, body } => {
                let mut lambda_bound = bound_vars.clone();
                for p in params {
                    lambda_bound.insert(p.name.node.to_string());
                }
                for stmt in &body.stmts {
                    self.collect_free_vars_stmt(stmt, &mut lambda_bound, env, captures);
                }
            }

            // Literals and other expressions that don't capture
            _ => {}
        }
    }

    /// Collect free variables from a statement, updating bound variables.
    fn collect_free_vars_stmt(
        &self,
        stmt: &ast::Stmt,
        bound_vars: &mut std::collections::HashSet<String>,
        env: &TypeEnv,
        captures: &mut Vec<(String, Type)>,
    ) {
        match &stmt.kind {
            ast::StmtKind::Let { pattern, value, .. } => {
                // First process the value (which may reference outer variables)
                self.collect_free_vars(value, bound_vars, env, captures);
                // Then add the bound variable
                if let ast::PatternKind::Ident(name) = &pattern.kind {
                    bound_vars.insert(name.to_string());
                }
            }

            ast::StmtKind::Expr(expr) => {
                self.collect_free_vars(expr, bound_vars, env, captures);
            }

            ast::StmtKind::Return(Some(expr)) => {
                self.collect_free_vars(expr, bound_vars, env, captures);
            }

            _ => {}
        }
    }

    /// Check that all captured variables in a lambda are Transfer-safe for spawn.
    ///
    /// This should be called when type-checking a spawn expression to ensure
    /// that the spawned closure doesn't capture non-Transfer values.
    ///
    /// # Arguments
    ///
    /// * `captures` - The captured variables from `collect_lambda_captures`
    /// * `span` - The span of the spawn expression for error reporting
    ///
    /// # Returns
    ///
    /// Ok(()) if all captures are Transfer, or a TypeError for the first
    /// non-Transfer capture found.
    pub fn check_spawn_captures(
        &self,
        captures: &[(String, Type)],
        span: Span,
    ) -> TypeResult<()> {
        for (var_name, var_type) in captures {
            let resolved_type = self.inference.apply(var_type);
            if !resolved_type.is_transfer() {
                return Err(TypeError::NonTransferCapture {
                    var_name: var_name.clone(),
                    var_type: format!("{}", resolved_type),
                    span,
                });
            }
        }
        Ok(())
    }

    /// Check a spawn expression for Transfer safety.
    ///
    /// This is a convenience method that combines capture collection and checking.
    ///
    /// # Arguments
    ///
    /// * `lambda_params` - Parameters of the spawned lambda
    /// * `lambda_body` - Body of the spawned lambda
    /// * `env` - Type environment at the spawn site
    /// * `span` - Span of the spawn expression
    ///
    /// # Returns
    ///
    /// Ok(()) if the spawn is safe, or a TypeError if non-Transfer captures exist.
    pub fn check_spawn_lambda(
        &mut self,
        lambda_params: &[ast::Param],
        lambda_body: &ast::Expr,
        env: &TypeEnv,
        span: Span,
    ) -> TypeResult<()> {
        let captures = self.collect_lambda_captures(lambda_params, lambda_body, env);
        self.check_spawn_captures(&captures, span)
    }

    // =========================================================================
    // Enhanced Capture Mode Analysis
    // =========================================================================

    /// Analyze a lambda expression and collect captured variables with full mode information.
    ///
    /// This is an enhanced version of `collect_lambda_captures` that determines
    /// the appropriate capture mode (borrow, borrow_mut, or move) for each variable
    /// based on how it is used within the lambda body.
    ///
    /// # Arguments
    ///
    /// * `lambda_params` - Parameters of the lambda
    /// * `lambda_body` - Body expression of the lambda
    /// * `env` - The capture environment at the lambda definition site
    ///
    /// # Returns
    ///
    /// A vector of CaptureInfo structs with full capture mode analysis.
    pub fn analyze_lambda_captures(
        &self,
        lambda_params: &[ast::Param],
        lambda_body: &ast::Expr,
        env: &CaptureEnv,
    ) -> Vec<CaptureInfo> {
        let mut captures: std::collections::HashMap<String, CaptureUsage> = std::collections::HashMap::new();
        let param_names: std::collections::HashSet<_> = lambda_params
            .iter()
            .map(|p| p.name.node.to_string())
            .collect();

        // Collect free variables and analyze their usage
        self.analyze_capture_usage(lambda_body, &param_names, env, &mut captures);

        // Convert usage info to CaptureInfo with appropriate modes
        let mut result = Vec::new();
        for (name, usage) in captures {
            if let Some(var_info) = env.lookup_var(&name) {
                let mode = self.determine_capture_mode(&var_info.ty, &usage, var_info.is_mutable);
                let mut capture = CaptureInfo::new(
                    name,
                    var_info.ty.clone(),
                    mode,
                    var_info.is_mutable,
                );
                if let Some(span) = var_info.def_span {
                    capture = capture.with_span(span);
                }
                result.push(capture);
            }
        }

        result
    }

    /// Determine the capture mode based on usage and type characteristics.
    ///
    /// Rules for capture mode determination:
    /// 1. If the variable is written to, it must be captured mutably (BorrowMut)
    /// 2. If the variable is moved (non-Copy type used by value), it must be Move
    /// 3. Otherwise, immutable borrow (Borrow) is sufficient
    fn determine_capture_mode(&self, ty: &Type, usage: &CaptureUsage, _is_declared_mutable: bool) -> CaptureMode {
        // If the variable is written to, we need mutable access
        if usage.is_written {
            return CaptureMode::BorrowMut;
        }

        // If the variable is moved and it's not a Copy type, we must move it
        if usage.is_moved && !ty.is_copy() {
            return CaptureMode::Move;
        }

        // Default to immutable borrow for read-only access
        CaptureMode::Borrow
    }

    /// Recursively analyze capture usage in an expression.
    fn analyze_capture_usage(
        &self,
        expr: &ast::Expr,
        bound_vars: &std::collections::HashSet<String>,
        env: &CaptureEnv,
        captures: &mut std::collections::HashMap<String, CaptureUsage>,
    ) {
        match &expr.kind {
            ast::ExprKind::Ident(name) => {
                let name_str = name.to_string();
                if !bound_vars.contains(&name_str) {
                    // It's a free variable - check if it's in the environment
                    if env.lookup_var(&name_str).is_some() {
                        let usage = captures.entry(name_str).or_default();
                        usage.is_read = true;
                    }
                }
            }

            // Binary operations
            ast::ExprKind::Binary { left, right, .. } => {
                self.analyze_capture_usage(left, bound_vars, env, captures);
                self.analyze_capture_usage(right, bound_vars, env, captures);
            }

            // Unary operations
            ast::ExprKind::Unary { operand, .. } => {
                self.analyze_capture_usage(operand, bound_vars, env, captures);
            }

            // Arrays and tuples
            ast::ExprKind::Array(elements) | ast::ExprKind::Tuple(elements) => {
                for elem in elements {
                    self.analyze_capture_usage(elem, bound_vars, env, captures);
                }
            }

            // Map literals
            ast::ExprKind::Map(pairs) => {
                for (key, value) in pairs {
                    self.analyze_capture_usage(key, bound_vars, env, captures);
                    self.analyze_capture_usage(value, bound_vars, env, captures);
                }
            }

            // Field access
            ast::ExprKind::Field { object, .. } => {
                self.analyze_capture_usage(object, bound_vars, env, captures);
            }

            // Index access
            ast::ExprKind::Index { object, index } => {
                self.analyze_capture_usage(object, bound_vars, env, captures);
                self.analyze_capture_usage(index, bound_vars, env, captures);
            }

            // Method calls (check if receiver is modified)
            ast::ExprKind::MethodCall { object, args, method, .. } => {
                // Check if this is a mutating method call (heuristic: methods starting with set_, push_, etc.)
                let method_name = method.node.to_string();
                let is_mutating = method_name.starts_with("set_")
                    || method_name.starts_with("push")
                    || method_name.starts_with("pop")
                    || method_name.starts_with("insert")
                    || method_name.starts_with("remove")
                    || method_name.starts_with("clear")
                    || method_name.ends_with("_mut");

                // If it's a mutating method and the object is a captured variable, mark as written
                if is_mutating {
                    if let ast::ExprKind::Ident(name) = &object.kind {
                        let name_str = name.to_string();
                        if !bound_vars.contains(&name_str) && env.lookup_var(&name_str).is_some() {
                            let usage = captures.entry(name_str).or_default();
                            usage.is_written = true;
                        }
                    }
                }

                self.analyze_capture_usage(object, bound_vars, env, captures);
                for arg in args {
                    self.analyze_capture_usage(arg, bound_vars, env, captures);
                }
            }

            // Function calls - arguments may be moved
            ast::ExprKind::Call { func, args } => {
                self.analyze_capture_usage(func, bound_vars, env, captures);
                for arg in args {
                    // If the argument is a direct identifier, it might be moved
                    if let ast::ExprKind::Ident(name) = &arg.value.kind {
                        let name_str = name.to_string();
                        if !bound_vars.contains(&name_str) {
                            if let Some(var_info) = env.lookup_var(&name_str) {
                                // Mark as potentially moved if not Copy
                                if !var_info.ty.is_copy() {
                                    let usage = captures.entry(name_str).or_default();
                                    usage.is_moved = true;
                                }
                            }
                        }
                    }
                    self.analyze_capture_usage(&arg.value, bound_vars, env, captures);
                }
            }

            // Block expressions
            ast::ExprKind::Block(block) => {
                let mut block_bound = bound_vars.clone();
                for stmt in &block.stmts {
                    self.analyze_capture_usage_stmt(stmt, &mut block_bound, env, captures);
                }
            }

            // If expressions
            ast::ExprKind::If {
                condition,
                then_branch,
                else_branch,
                elsif_branches,
            } => {
                self.analyze_capture_usage(condition, bound_vars, env, captures);
                let mut then_bound = bound_vars.clone();
                for stmt in &then_branch.stmts {
                    self.analyze_capture_usage_stmt(stmt, &mut then_bound, env, captures);
                }
                for (elsif_cond, elsif_block) in elsif_branches {
                    self.analyze_capture_usage(elsif_cond, bound_vars, env, captures);
                    let mut elsif_bound = bound_vars.clone();
                    for stmt in &elsif_block.stmts {
                        self.analyze_capture_usage_stmt(stmt, &mut elsif_bound, env, captures);
                    }
                }
                if let Some(else_block) = else_branch {
                    let mut else_bound = bound_vars.clone();
                    for stmt in &else_block.stmts {
                        self.analyze_capture_usage_stmt(stmt, &mut else_bound, env, captures);
                    }
                }
            }

            // Nested lambdas
            ast::ExprKind::Lambda { params, body } => {
                let mut lambda_bound = bound_vars.clone();
                for p in params {
                    lambda_bound.insert(p.name.node.to_string());
                }
                self.analyze_capture_usage(body, &lambda_bound, env, captures);
            }

            ast::ExprKind::BlockLambda { params, body } => {
                let mut lambda_bound = bound_vars.clone();
                for p in params {
                    lambda_bound.insert(p.name.node.to_string());
                }
                for stmt in &body.stmts {
                    self.analyze_capture_usage_stmt(stmt, &mut lambda_bound, env, captures);
                }
            }

            // Match expressions
            ast::ExprKind::Match { scrutinee, arms } => {
                self.analyze_capture_usage(scrutinee, bound_vars, env, captures);
                for arm in arms {
                    let mut arm_bound = bound_vars.clone();
                    // Add pattern bindings
                    self.collect_pattern_bindings(&arm.pattern, &mut arm_bound);
                    if let Some(guard) = &arm.guard {
                        self.analyze_capture_usage(guard, &arm_bound, env, captures);
                    }
                    // Handle both Expr and Block variants of MatchArmBody
                    match &arm.body {
                        ast::MatchArmBody::Expr(expr) => {
                            self.analyze_capture_usage(expr, &arm_bound, env, captures);
                        }
                        ast::MatchArmBody::Block(block) => {
                            let mut block_bound = arm_bound.clone();
                            for stmt in &block.stmts {
                                self.analyze_capture_usage_stmt(stmt, &mut block_bound, env, captures);
                            }
                        }
                    }
                }
            }

            // Literals and other expressions that don't capture
            _ => {}
        }
    }

    /// Analyze capture usage in a statement.
    fn analyze_capture_usage_stmt(
        &self,
        stmt: &ast::Stmt,
        bound_vars: &mut std::collections::HashSet<String>,
        env: &CaptureEnv,
        captures: &mut std::collections::HashMap<String, CaptureUsage>,
    ) {
        match &stmt.kind {
            ast::StmtKind::Let { pattern, value, .. } => {
                // First process the value (which may reference outer variables)
                self.analyze_capture_usage(value, bound_vars, env, captures);
                // Then add the bound variable
                self.collect_pattern_bindings(pattern, bound_vars);
            }

            ast::StmtKind::Var { name, value, .. } => {
                // Process the value first
                self.analyze_capture_usage(value, bound_vars, env, captures);
                // Add the mutable variable binding
                bound_vars.insert(name.node.to_string());
            }

            ast::StmtKind::Assign { target, value, .. } => {
                // Check if the target is a captured variable being written to
                if let ast::ExprKind::Ident(name) = &target.kind {
                    let name_str = name.to_string();
                    if !bound_vars.contains(&name_str) && env.lookup_var(&name_str).is_some() {
                        let usage = captures.entry(name_str).or_default();
                        usage.is_written = true;
                    }
                }
                self.analyze_capture_usage(target, bound_vars, env, captures);
                self.analyze_capture_usage(value, bound_vars, env, captures);
            }

            ast::StmtKind::Expr(expr) => {
                self.analyze_capture_usage(expr, bound_vars, env, captures);
            }

            ast::StmtKind::Return(Some(expr)) => {
                self.analyze_capture_usage(expr, bound_vars, env, captures);
            }

            _ => {}
        }
    }

    /// Collect variable bindings from a pattern.
    fn collect_pattern_bindings(
        &self,
        pattern: &ast::Pattern,
        bound_vars: &mut std::collections::HashSet<String>,
    ) {
        match &pattern.kind {
            ast::PatternKind::Ident(name) => {
                bound_vars.insert(name.to_string());
            }
            ast::PatternKind::Tuple(patterns) => {
                for p in patterns {
                    self.collect_pattern_bindings(p, bound_vars);
                }
            }
            ast::PatternKind::Array { elements, rest } => {
                for p in elements {
                    self.collect_pattern_bindings(p, bound_vars);
                }
                if let Some(rest_pattern) = rest {
                    self.collect_pattern_bindings(rest_pattern, bound_vars);
                }
            }
            ast::PatternKind::Struct { fields, .. } => {
                for field in fields {
                    if let Some(p) = &field.pattern {
                        self.collect_pattern_bindings(p, bound_vars);
                    } else {
                        // Shorthand: field name is the binding
                        bound_vars.insert(field.name.node.to_string());
                    }
                }
            }
            ast::PatternKind::Variant { fields, .. } => {
                if let Some(patterns) = fields {
                    for p in patterns {
                        self.collect_pattern_bindings(p, bound_vars);
                    }
                }
            }
            ast::PatternKind::Binding { name, pattern } => {
                bound_vars.insert(name.node.to_string());
                self.collect_pattern_bindings(pattern, bound_vars);
            }
            ast::PatternKind::Or(patterns) => {
                // All branches should bind the same names
                if let Some(first) = patterns.first() {
                    self.collect_pattern_bindings(first, bound_vars);
                }
            }
            ast::PatternKind::Guard { pattern, .. } => {
                self.collect_pattern_bindings(pattern, bound_vars);
            }
            ast::PatternKind::Rest(opt_ident) => {
                // Rest pattern binds the identifier if present: `...rest`
                if let Some(ident) = opt_ident {
                    bound_vars.insert(ident.node.to_string());
                }
            }
            _ => {}
        }
    }

    // =========================================================================
    // Ownership and Borrowing Validation
    // =========================================================================

    /// Validate captured variables against ownership and borrowing rules.
    ///
    /// This checks that:
    /// 1. Mutable captures are only for mutable variables
    /// 2. Move captures don't violate ownership rules
    /// 3. Spawn captures don't include mutable borrows
    ///
    /// # Arguments
    ///
    /// * `captures` - The analyzed captures from `analyze_lambda_captures`
    /// * `is_spawn` - Whether this is a spawn context (stricter rules)
    /// * `span` - Span for error reporting
    ///
    /// # Returns
    ///
    /// Ok(()) if all captures are valid, or a TypeError for the first violation.
    pub fn validate_captures(
        &self,
        captures: &[CaptureInfo],
        is_spawn: bool,
        span: Span,
    ) -> TypeResult<()> {
        for capture in captures {
            // Check: Cannot mutably capture an immutable variable
            if capture.mode == CaptureMode::BorrowMut && !capture.is_mutable {
                return Err(TypeError::MutableCaptureOfImmutable {
                    var_name: capture.name.clone(),
                    span,
                });
            }

            // Check: Spawn cannot have mutable captures
            if is_spawn && capture.mode == CaptureMode::BorrowMut {
                return Err(TypeError::MutableCaptureInSpawn {
                    var_name: capture.name.clone(),
                    span,
                });
            }

            // Check: Spawn requires Transfer types for move captures
            if is_spawn && capture.mode == CaptureMode::Move {
                let resolved_type = self.inference.apply(&capture.ty);
                if !resolved_type.is_transfer() {
                    return Err(TypeError::NonTransferCapture {
                        var_name: capture.name.clone(),
                        var_type: format!("{}", resolved_type),
                        span,
                    });
                }
            }

            // Check: Spawn also requires Transfer for borrowed values (they will be cloned/moved)
            if is_spawn && capture.mode == CaptureMode::Borrow {
                let resolved_type = self.inference.apply(&capture.ty);
                if !resolved_type.is_transfer() {
                    return Err(TypeError::NonTransferCapture {
                        var_name: capture.name.clone(),
                        var_type: format!("{}", resolved_type),
                        span,
                    });
                }
            }
        }

        Ok(())
    }

    /// Convenience method to analyze captures and validate for spawn.
    ///
    /// # Arguments
    ///
    /// * `lambda_params` - Parameters of the spawned lambda
    /// * `lambda_body` - Body of the spawned lambda
    /// * `env` - Capture environment at the spawn site
    /// * `span` - Span of the spawn expression
    ///
    /// # Returns
    ///
    /// A tuple of (captures, validation_result)
    pub fn analyze_and_validate_spawn_captures(
        &self,
        lambda_params: &[ast::Param],
        lambda_body: &ast::Expr,
        env: &CaptureEnv,
        span: Span,
    ) -> (Vec<CaptureInfo>, TypeResult<()>) {
        let captures = self.analyze_lambda_captures(lambda_params, lambda_body, env);
        let validation = self.validate_captures(&captures, true, span);
        (captures, validation)
    }

    /// Convenience method to analyze captures and validate for regular closures.
    ///
    /// # Arguments
    ///
    /// * `lambda_params` - Parameters of the lambda
    /// * `lambda_body` - Body of the lambda
    /// * `env` - Capture environment at the definition site
    /// * `span` - Span of the lambda expression
    ///
    /// # Returns
    ///
    /// A tuple of (captures, validation_result)
    pub fn analyze_and_validate_closure_captures(
        &self,
        lambda_params: &[ast::Param],
        lambda_body: &ast::Expr,
        env: &CaptureEnv,
        span: Span,
    ) -> (Vec<CaptureInfo>, TypeResult<()>) {
        let captures = self.analyze_lambda_captures(lambda_params, lambda_body, env);
        let validation = self.validate_captures(&captures, false, span);
        (captures, validation)
    }

    /// Validate that variables captured by a defer expression are valid.
    ///
    /// Defer blocks have special requirements:
    /// - They cannot capture variables that will go out of scope before the defer executes
    /// - They should not mutate captured variables (defer runs at scope exit)
    ///
    /// For now, this is a basic validation that simply verifies the captured variables exist.
    fn validate_defer_captures(
        &self,
        _expr: &ast::Expr,
        _env: &TypeEnv,
        _defer_span: Span,
    ) -> TypeResult<()> {
        // TODO: Implement more thorough defer capture validation
        // For now, defer expressions are allowed to capture any in-scope variables
        // A more complete implementation would track variable lifetimes
        Ok(())
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unify_same_types() {
        let mut inf = TypeInference::new();
        assert!(inf.unify(&Type::Int, &Type::Int, Span::dummy()).is_ok());
        assert!(inf
            .unify(&Type::String, &Type::String, Span::dummy())
            .is_ok());
    }

    #[test]
    fn test_unify_type_var() {
        let mut inf = TypeInference::new();
        let var = inf.fresh_var();
        assert!(inf.unify(&var, &Type::Int, Span::dummy()).is_ok());
        assert_eq!(inf.apply(&var), Type::Int);
    }

    #[test]
    fn test_unify_mismatch() {
        let mut inf = TypeInference::new();
        assert!(inf.unify(&Type::Int, &Type::String, Span::dummy()).is_err());
    }

    #[test]
    fn test_infer_literal_types() {
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        // Integer literal
        let int_expr = ast::Expr::new(
            ast::ExprKind::Integer("42".into()),
            Span::dummy(),
        );
        let ty = checker.infer_expr(&int_expr, &env).unwrap();
        assert_eq!(ty, Type::Int);

        // Float literal
        let float_expr = ast::Expr::new(
            ast::ExprKind::Float("3.14".into()),
            Span::dummy(),
        );
        let ty = checker.infer_expr(&float_expr, &env).unwrap();
        assert_eq!(ty, Type::Float);

        // String literal
        let str_expr = ast::Expr::new(
            ast::ExprKind::String("hello".into()),
            Span::dummy(),
        );
        let ty = checker.infer_expr(&str_expr, &env).unwrap();
        assert_eq!(ty, Type::String);

        // Bool literal
        let bool_expr = ast::Expr::new(ast::ExprKind::Bool(true), Span::dummy());
        let ty = checker.infer_expr(&bool_expr, &env).unwrap();
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_infer_array_type() {
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        // Array of integers
        let arr_expr = ast::Expr::new(
            ast::ExprKind::Array(vec![
                ast::Expr::new(ast::ExprKind::Integer("1".into()), Span::dummy()),
                ast::Expr::new(ast::ExprKind::Integer("2".into()), Span::dummy()),
            ]),
            Span::dummy(),
        );
        let ty = checker.infer_expr(&arr_expr, &env).unwrap();
        assert_eq!(ty, Type::Array(Box::new(Type::Int)));
    }

    #[test]
    fn test_infer_tuple_type() {
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        // Tuple (Int, String)
        let tuple_expr = ast::Expr::new(
            ast::ExprKind::Tuple(vec![
                ast::Expr::new(ast::ExprKind::Integer("42".into()), Span::dummy()),
                ast::Expr::new(ast::ExprKind::String("hello".into()), Span::dummy()),
            ]),
            Span::dummy(),
        );
        let ty = checker.infer_expr(&tuple_expr, &env).unwrap();
        assert_eq!(ty, Type::Tuple(vec![Type::Int, Type::String]));
    }

    #[test]
    fn test_infer_binary_arithmetic() {
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        // 1 + 2
        let add_expr = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::Add,
                left: Box::new(ast::Expr::new(ast::ExprKind::Integer("1".into()), Span::dummy())),
                right: Box::new(ast::Expr::new(ast::ExprKind::Integer("2".into()), Span::dummy())),
            },
            Span::dummy(),
        );
        let ty = checker.infer_expr(&add_expr, &env).unwrap();
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn test_infer_binary_comparison() {
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        // 1 < 2
        let cmp_expr = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::Lt,
                left: Box::new(ast::Expr::new(ast::ExprKind::Integer("1".into()), Span::dummy())),
                right: Box::new(ast::Expr::new(ast::ExprKind::Integer("2".into()), Span::dummy())),
            },
            Span::dummy(),
        );
        let ty = checker.infer_expr(&cmp_expr, &env).unwrap();
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_infer_variable_lookup() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();
        env.define_var("x".to_string(), Type::Int);

        let var_expr = ast::Expr::new(
            ast::ExprKind::Ident("x".into()),
            Span::dummy(),
        );
        let ty = checker.infer_expr(&var_expr, &env).unwrap();
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn test_undefined_variable_error() {
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        let var_expr = ast::Expr::new(
            ast::ExprKind::Ident("undefined_var".into()),
            Span::dummy(),
        );
        let result = checker.infer_expr(&var_expr, &env);
        assert!(result.is_err());
    }

    // ========================================================================
    // Additional comprehensive tests for ARIA-IMPL-006
    // ========================================================================

    #[test]
    fn test_infer_char_type() {
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        let char_expr = ast::Expr::new(
            ast::ExprKind::Char("a".into()),
            Span::dummy(),
        );
        let ty = checker.infer_expr(&char_expr, &env).unwrap();
        assert_eq!(ty, Type::Char);
    }

    #[test]
    fn test_infer_nil_type() {
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        let nil_expr = ast::Expr::new(ast::ExprKind::Nil, Span::dummy());
        let ty = checker.infer_expr(&nil_expr, &env).unwrap();
        // Nil should produce an Optional type with a fresh type var
        assert!(matches!(ty, Type::Optional(_)));
    }

    #[test]
    fn test_infer_unary_negation() {
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        // -5
        let neg_expr = ast::Expr::new(
            ast::ExprKind::Unary {
                op: ast::UnaryOp::Neg,
                operand: Box::new(ast::Expr::new(ast::ExprKind::Integer("5".into()), Span::dummy())),
            },
            Span::dummy(),
        );
        let ty = checker.infer_expr(&neg_expr, &env).unwrap();
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn test_infer_unary_not() {
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        // !true
        let not_expr = ast::Expr::new(
            ast::ExprKind::Unary {
                op: ast::UnaryOp::Not,
                operand: Box::new(ast::Expr::new(ast::ExprKind::Bool(true), Span::dummy())),
            },
            Span::dummy(),
        );
        let ty = checker.infer_expr(&not_expr, &env).unwrap();
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_infer_logical_and() {
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        // true and false
        let and_expr = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::And,
                left: Box::new(ast::Expr::new(ast::ExprKind::Bool(true), Span::dummy())),
                right: Box::new(ast::Expr::new(ast::ExprKind::Bool(false), Span::dummy())),
            },
            Span::dummy(),
        );
        let ty = checker.infer_expr(&and_expr, &env).unwrap();
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_infer_logical_or() {
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        // true or false
        let or_expr = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::Or,
                left: Box::new(ast::Expr::new(ast::ExprKind::Bool(true), Span::dummy())),
                right: Box::new(ast::Expr::new(ast::ExprKind::Bool(false), Span::dummy())),
            },
            Span::dummy(),
        );
        let ty = checker.infer_expr(&or_expr, &env).unwrap();
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_infer_float_arithmetic() {
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        // 3.14 * 2.0
        let mul_expr = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::Mul,
                left: Box::new(ast::Expr::new(ast::ExprKind::Float("3.14".into()), Span::dummy())),
                right: Box::new(ast::Expr::new(ast::ExprKind::Float("2.0".into()), Span::dummy())),
            },
            Span::dummy(),
        );
        let ty = checker.infer_expr(&mul_expr, &env).unwrap();
        assert_eq!(ty, Type::Float);
    }

    #[test]
    fn test_infer_equality_comparison() {
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        // "a" == "b"
        let eq_expr = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::Eq,
                left: Box::new(ast::Expr::new(ast::ExprKind::String("a".into()), Span::dummy())),
                right: Box::new(ast::Expr::new(ast::ExprKind::String("b".into()), Span::dummy())),
            },
            Span::dummy(),
        );
        let ty = checker.infer_expr(&eq_expr, &env).unwrap();
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_infer_not_equal_comparison() {
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        // 1 != 2
        let ne_expr = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::NotEq,
                left: Box::new(ast::Expr::new(ast::ExprKind::Integer("1".into()), Span::dummy())),
                right: Box::new(ast::Expr::new(ast::ExprKind::Integer("2".into()), Span::dummy())),
            },
            Span::dummy(),
        );
        let ty = checker.infer_expr(&ne_expr, &env).unwrap();
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_infer_subtraction() {
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        // 10 - 3
        let sub_expr = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::Sub,
                left: Box::new(ast::Expr::new(ast::ExprKind::Integer("10".into()), Span::dummy())),
                right: Box::new(ast::Expr::new(ast::ExprKind::Integer("3".into()), Span::dummy())),
            },
            Span::dummy(),
        );
        let ty = checker.infer_expr(&sub_expr, &env).unwrap();
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn test_infer_division() {
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        // 10 / 2
        let div_expr = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::Div,
                left: Box::new(ast::Expr::new(ast::ExprKind::Integer("10".into()), Span::dummy())),
                right: Box::new(ast::Expr::new(ast::ExprKind::Integer("2".into()), Span::dummy())),
            },
            Span::dummy(),
        );
        let ty = checker.infer_expr(&div_expr, &env).unwrap();
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn test_infer_modulo() {
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        // 10 % 3
        let mod_expr = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::Mod,
                left: Box::new(ast::Expr::new(ast::ExprKind::Integer("10".into()), Span::dummy())),
                right: Box::new(ast::Expr::new(ast::ExprKind::Integer("3".into()), Span::dummy())),
            },
            Span::dummy(),
        );
        let ty = checker.infer_expr(&mod_expr, &env).unwrap();
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn test_infer_empty_array() {
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        // []
        let empty_arr = ast::Expr::new(
            ast::ExprKind::Array(vec![]),
            Span::dummy(),
        );
        let ty = checker.infer_expr(&empty_arr, &env).unwrap();
        // Empty array should have fresh type var element
        assert!(matches!(ty, Type::Array(_)));
    }

    #[test]
    fn test_infer_nested_binary_ops() {
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        // (1 + 2) * 3
        let nested_expr = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::Mul,
                left: Box::new(ast::Expr::new(
                    ast::ExprKind::Binary {
                        op: ast::BinaryOp::Add,
                        left: Box::new(ast::Expr::new(ast::ExprKind::Integer("1".into()), Span::dummy())),
                        right: Box::new(ast::Expr::new(ast::ExprKind::Integer("2".into()), Span::dummy())),
                    },
                    Span::dummy(),
                )),
                right: Box::new(ast::Expr::new(ast::ExprKind::Integer("3".into()), Span::dummy())),
            },
            Span::dummy(),
        );
        let ty = checker.infer_expr(&nested_expr, &env).unwrap();
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn test_type_env_scoping() {
        let mut parent_env = TypeEnv::new();
        parent_env.define_var("x".to_string(), Type::Int);

        let child_env = TypeEnv::with_parent(std::rc::Rc::new(parent_env));

        // Should be able to look up from parent
        let x_type = child_env.lookup_var("x");
        assert!(x_type.is_some());
        assert_eq!(x_type.unwrap(), &Type::Int);
    }

    #[test]
    fn test_type_env_shadowing() {
        let parent_env = TypeEnv::new();
        let mut child_env = TypeEnv::with_parent(std::rc::Rc::new(parent_env));

        // Define in parent would need to be done before creating child
        // But we can test that child definitions work
        child_env.define_var("y".to_string(), Type::String);

        let y_type = child_env.lookup_var("y");
        assert!(y_type.is_some());
        assert_eq!(y_type.unwrap(), &Type::String);
    }

    #[test]
    fn test_infer_map_literal() {
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        // {"a": 1}
        let map_expr = ast::Expr::new(
            ast::ExprKind::Map(vec![
                (
                    ast::Expr::new(ast::ExprKind::String("a".into()), Span::dummy()),
                    ast::Expr::new(ast::ExprKind::Integer("1".into()), Span::dummy()),
                ),
            ]),
            Span::dummy(),
        );
        let ty = checker.infer_expr(&map_expr, &env).unwrap();
        // Map type should have key and value types
        assert!(matches!(ty, Type::Map(_, _)));
    }

    #[test]
    fn test_unify_arrays() {
        let mut inf = TypeInference::new();
        let arr1 = Type::Array(Box::new(Type::Int));
        let arr2 = Type::Array(Box::new(Type::Int));
        assert!(inf.unify(&arr1, &arr2, Span::dummy()).is_ok());
    }

    #[test]
    fn test_unify_arrays_mismatch() {
        let mut inf = TypeInference::new();
        let arr1 = Type::Array(Box::new(Type::Int));
        let arr2 = Type::Array(Box::new(Type::String));
        assert!(inf.unify(&arr1, &arr2, Span::dummy()).is_err());
    }

    #[test]
    fn test_unify_tuples() {
        let mut inf = TypeInference::new();
        let tup1 = Type::Tuple(vec![Type::Int, Type::String]);
        let tup2 = Type::Tuple(vec![Type::Int, Type::String]);
        assert!(inf.unify(&tup1, &tup2, Span::dummy()).is_ok());
    }

    #[test]
    fn test_unify_tuples_length_mismatch() {
        let mut inf = TypeInference::new();
        let tup1 = Type::Tuple(vec![Type::Int, Type::String]);
        let tup2 = Type::Tuple(vec![Type::Int]);
        assert!(inf.unify(&tup1, &tup2, Span::dummy()).is_err());
    }

    #[test]
    fn test_unify_functions() {
        let mut inf = TypeInference::new();
        let fn1 = Type::Function {
            params: vec![Type::Int, Type::Int],
            return_type: Box::new(Type::Int),
        };
        let fn2 = Type::Function {
            params: vec![Type::Int, Type::Int],
            return_type: Box::new(Type::Int),
        };
        assert!(inf.unify(&fn1, &fn2, Span::dummy()).is_ok());
    }

    #[test]
    fn test_type_is_numeric() {
        assert!(Type::Int.is_numeric());
        assert!(Type::Float.is_numeric());
        assert!(Type::Int64.is_numeric());
        assert!(Type::Float32.is_numeric());
        assert!(!Type::String.is_numeric());
        assert!(!Type::Bool.is_numeric());
    }

    #[test]
    fn test_type_is_integer() {
        assert!(Type::Int.is_integer());
        assert!(Type::Int8.is_integer());
        assert!(Type::UInt64.is_integer());
        assert!(!Type::Float.is_integer());
        assert!(!Type::String.is_integer());
    }

    #[test]
    fn test_infer_greater_than() {
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        // 5 > 3
        let gt_expr = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::Gt,
                left: Box::new(ast::Expr::new(ast::ExprKind::Integer("5".into()), Span::dummy())),
                right: Box::new(ast::Expr::new(ast::ExprKind::Integer("3".into()), Span::dummy())),
            },
            Span::dummy(),
        );
        let ty = checker.infer_expr(&gt_expr, &env).unwrap();
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_infer_less_than_or_equal() {
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        // 5 <= 10
        let lte_expr = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::LtEq,
                left: Box::new(ast::Expr::new(ast::ExprKind::Integer("5".into()), Span::dummy())),
                right: Box::new(ast::Expr::new(ast::ExprKind::Integer("10".into()), Span::dummy())),
            },
            Span::dummy(),
        );
        let ty = checker.infer_expr(&lte_expr, &env).unwrap();
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_infer_greater_than_or_equal() {
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        // 10 >= 5
        let gte_expr = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::GtEq,
                left: Box::new(ast::Expr::new(ast::ExprKind::Integer("10".into()), Span::dummy())),
                right: Box::new(ast::Expr::new(ast::ExprKind::Integer("5".into()), Span::dummy())),
            },
            Span::dummy(),
        );
        let ty = checker.infer_expr(&gte_expr, &env).unwrap();
        assert_eq!(ty, Type::Bool);
    }

    // ========================================================================
    // Bidirectional Type Checking Tests
    // ========================================================================

    #[test]
    fn test_check_mode_constructors() {
        // Test CheckMode::Synthesize
        let synth = CheckMode::synthesize();
        assert!(synth.is_synthesize());
        assert!(!synth.is_check());
        assert!(synth.expected_type().is_none());

        // Test CheckMode::Check
        let check = CheckMode::check(Type::Int, TypeSource::Unknown);
        assert!(check.is_check());
        assert!(!check.is_synthesize());
        assert_eq!(check.expected_type(), Some(&Type::Int));
    }

    #[test]
    fn test_type_source_variants() {
        // Test TypeSource::Annotation
        let annot = TypeSource::Annotation(Span::dummy());
        assert!(matches!(annot, TypeSource::Annotation(_)));

        // Test TypeSource::Parameter
        let param = TypeSource::Parameter {
            name: "x".to_string(),
            span: Span::dummy(),
        };
        if let TypeSource::Parameter { name, .. } = param {
            assert_eq!(name, "x");
        } else {
            panic!("Expected TypeSource::Parameter");
        }

        // Test TypeSource::Return
        let ret = TypeSource::Return(Span::dummy());
        assert!(matches!(ret, TypeSource::Return(_)));

        // Test TypeSource::Context
        let ctx = TypeSource::Context {
            description: "array element".to_string(),
            span: Span::dummy(),
        };
        if let TypeSource::Context { description, .. } = ctx {
            assert_eq!(description, "array element");
        } else {
            panic!("Expected TypeSource::Context");
        }
    }

    #[test]
    fn test_check_expr_simple_literal() {
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        // Check 42 against Int - should succeed
        let int_expr = ast::Expr::new(
            ast::ExprKind::Integer("42".into()),
            Span::dummy(),
        );
        let result = checker.check_expr(
            &int_expr,
            &Type::Int,
            TypeSource::Annotation(Span::dummy()),
            &env,
        );
        assert!(result.is_ok());

        // Check 42 against String - should fail
        let mut checker2 = TypeChecker::new();
        let result2 = checker2.check_expr(
            &int_expr,
            &Type::String,
            TypeSource::Annotation(Span::dummy()),
            &env,
        );
        assert!(result2.is_err());
    }

    #[test]
    fn test_check_expr_type_mismatch_gives_error() {
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        // Try to check "hello" against Int
        let str_expr = ast::Expr::new(
            ast::ExprKind::String("hello".into()),
            Span::dummy(),
        );
        let result = checker.check_expr(
            &str_expr,
            &Type::Int,
            TypeSource::Context {
                description: "expected integer".to_string(),
                span: Span::dummy(),
            },
            &env,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_bidirectional_lambda_param_propagation() {
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        // Create lambda: |x| x (identity function)
        // When we don't have expected type, x gets fresh type var
        let lambda_expr = ast::Expr::new(
            ast::ExprKind::Lambda {
                params: vec![ast::Param {
                    mutable: false,
                    name: ast::Spanned::new("x".into(), Span::dummy()),
                    ty: None, // No explicit type annotation
                    default: None,
                    span: Span::dummy(),
                }],
                body: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("x".into()),
                    Span::dummy(),
                )),
            },
            Span::dummy(),
        );

        // Check lambda against (Int) -> Int
        // This should propagate Int to the parameter x
        let expected_fn_type = Type::Function {
            params: vec![Type::Int],
            return_type: Box::new(Type::Int),
        };

        let result = checker.check_expr(
            &lambda_expr,
            &expected_fn_type,
            TypeSource::Parameter {
                name: "callback".to_string(),
                span: Span::dummy(),
            },
            &env,
        );
        assert!(result.is_ok(), "Lambda should check against (Int) -> Int");
    }

    #[test]
    fn test_bidirectional_lambda_body_checking() {
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        // Create lambda: |x| x + 1
        // The body should be checked against the expected return type
        let lambda_expr = ast::Expr::new(
            ast::ExprKind::Lambda {
                params: vec![ast::Param {
                    mutable: false,
                    name: ast::Spanned::new("x".into(), Span::dummy()),
                    ty: None,
                    default: None,
                    span: Span::dummy(),
                }],
                body: Box::new(ast::Expr::new(
                    ast::ExprKind::Binary {
                        op: ast::BinaryOp::Add,
                        left: Box::new(ast::Expr::new(
                            ast::ExprKind::Ident("x".into()),
                            Span::dummy(),
                        )),
                        right: Box::new(ast::Expr::new(
                            ast::ExprKind::Integer("1".into()),
                            Span::dummy(),
                        )),
                    },
                    Span::dummy(),
                )),
            },
            Span::dummy(),
        );

        // Check against (Int) -> Int - should succeed
        let expected = Type::Function {
            params: vec![Type::Int],
            return_type: Box::new(Type::Int),
        };

        let result = checker.check_expr(
            &lambda_expr,
            &expected,
            TypeSource::Unknown,
            &env,
        );
        assert!(result.is_ok(), "Lambda |x| x + 1 should check against (Int) -> Int");
    }

    #[test]
    fn test_bidirectional_lambda_return_type_mismatch() {
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        // Create lambda: |x| x + 1 (returns Int)
        let lambda_expr = ast::Expr::new(
            ast::ExprKind::Lambda {
                params: vec![ast::Param {
                    mutable: false,
                    name: ast::Spanned::new("x".into(), Span::dummy()),
                    ty: None,
                    default: None,
                    span: Span::dummy(),
                }],
                body: Box::new(ast::Expr::new(
                    ast::ExprKind::Binary {
                        op: ast::BinaryOp::Add,
                        left: Box::new(ast::Expr::new(
                            ast::ExprKind::Ident("x".into()),
                            Span::dummy(),
                        )),
                        right: Box::new(ast::Expr::new(
                            ast::ExprKind::Integer("1".into()),
                            Span::dummy(),
                        )),
                    },
                    Span::dummy(),
                )),
            },
            Span::dummy(),
        );

        // Check against (Int) -> String - should fail (return type mismatch)
        let expected = Type::Function {
            params: vec![Type::Int],
            return_type: Box::new(Type::String),
        };

        let result = checker.check_expr(
            &lambda_expr,
            &expected,
            TypeSource::Unknown,
            &env,
        );
        assert!(result.is_err(), "Lambda returning Int should not check against (Int) -> String");
    }

    #[test]
    fn test_infer_expr_with_expected() {
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        // Test with expected type (checking mode)
        let int_expr = ast::Expr::new(
            ast::ExprKind::Integer("42".into()),
            Span::dummy(),
        );

        let result = checker.infer_expr_with_expected(
            &int_expr,
            Some(&Type::Int),
            TypeSource::Unknown,
            &env,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Type::Int);

        // Test without expected type (synthesize mode)
        let mut checker2 = TypeChecker::new();
        let result2 = checker2.infer_expr_with_expected(
            &int_expr,
            None,
            TypeSource::Unknown,
            &env,
        );
        assert!(result2.is_ok());
        assert_eq!(result2.unwrap(), Type::Int);
    }

    #[test]
    fn test_infer_lambda_with_expected_type() {
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        // Lambda without type annotations
        let params = vec![ast::Param {
            mutable: false,
            name: ast::Spanned::new("x".into(), Span::dummy()),
            ty: None,
            default: None,
            span: Span::dummy(),
        }];
        let body = ast::Expr::new(
            ast::ExprKind::Ident("x".into()),
            Span::dummy(),
        );

        // Infer with expected function type
        let expected = Type::Function {
            params: vec![Type::String],
            return_type: Box::new(Type::String),
        };

        let result = checker.infer_lambda(&params, &body, Some(&expected), &env);
        assert!(result.is_ok());

        let fn_type = result.unwrap();
        if let Type::Function { params: p, return_type: r } = fn_type {
            // Parameter should have been inferred as String from expected type
            assert_eq!(p.len(), 1);
            assert_eq!(p[0], Type::String);
            assert_eq!(*r, Type::String);
        } else {
            panic!("Expected function type");
        }
    }

    #[test]
    fn test_check_mode_equality() {
        let mode1 = CheckMode::check(Type::Int, TypeSource::Unknown);
        let mode2 = CheckMode::check(Type::Int, TypeSource::Unknown);
        let mode3 = CheckMode::check(Type::String, TypeSource::Unknown);
        let mode4 = CheckMode::Synthesize;

        assert_eq!(mode1, mode2);
        assert_ne!(mode1, mode3);
        assert_ne!(mode1, mode4);
        assert_eq!(mode4, CheckMode::Synthesize);
    }

    // ========================================================================
    // Flow-Sensitive Type Narrowing Tests
    // ========================================================================

    #[test]
    fn test_flow_type_env_basic() {
        let mut env = FlowTypeEnv::default();
        env.define_var("x".to_string(), Type::Optional(Box::new(Type::Int)));

        // Before narrowing
        let ty = env.lookup_var("x");
        assert!(matches!(ty, Some(Type::Optional(_))));

        // After narrowing
        env.narrow("x".to_string(), Type::Int, Span::dummy());
        let ty = env.lookup_var("x");
        assert_eq!(ty, Some(Type::Int));
    }

    #[test]
    fn test_flow_type_env_narrowing_precedence() {
        let mut env = FlowTypeEnv::default();
        env.define_var("x".to_string(), Type::Optional(Box::new(Type::Int)));

        // Multiple narrowings - most recent wins
        env.narrow("x".to_string(), Type::Int, Span::dummy());
        env.narrow("x".to_string(), Type::Int64, Span::dummy());

        let ty = env.lookup_var("x");
        assert_eq!(ty, Some(Type::Int64));
    }

    #[test]
    fn test_flow_type_env_invalidate_narrowing() {
        let mut env = FlowTypeEnv::default();
        env.define_var("x".to_string(), Type::Optional(Box::new(Type::Int)));
        env.narrow("x".to_string(), Type::Int, Span::dummy());

        assert_eq!(env.lookup_var("x"), Some(Type::Int));

        // Invalidate the narrowing (e.g., after reassignment)
        env.invalidate_narrowing("x");

        // Should fall back to original type
        let ty = env.lookup_var("x");
        assert!(matches!(ty, Some(Type::Optional(_))));
    }

    #[test]
    fn test_flow_type_env_child_scope() {
        let mut env = FlowTypeEnv::default();
        env.define_var("x".to_string(), Type::Optional(Box::new(Type::Int)));
        env.narrow("x".to_string(), Type::Int, Span::dummy());

        // Child scope inherits narrowings
        let child = env.child_scope();
        assert_eq!(child.lookup_var("x"), Some(Type::Int));
    }

    #[test]
    fn test_extract_narrowings_nil_check() {
        let mut env = TypeEnv::new();
        env.define_var("value".to_string(), Type::Optional(Box::new(Type::Int)));

        // value != nil
        let condition = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::NotEq,
                left: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("value".into()),
                    Span::dummy(),
                )),
                right: Box::new(ast::Expr::new(ast::ExprKind::Nil, Span::dummy())),
            },
            Span::dummy(),
        );

        let narrowings = extract_narrowings(&condition, &env);
        assert_eq!(narrowings.len(), 1);
        assert_eq!(narrowings[0].var_name, "value");
        assert_eq!(narrowings[0].narrowed_type, Type::Int);
    }

    #[test]
    fn test_extract_narrowings_nil_check_reversed() {
        let mut env = TypeEnv::new();
        env.define_var("value".to_string(), Type::Optional(Box::new(Type::String)));

        // nil != value (reversed order)
        let condition = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::NotEq,
                left: Box::new(ast::Expr::new(ast::ExprKind::Nil, Span::dummy())),
                right: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("value".into()),
                    Span::dummy(),
                )),
            },
            Span::dummy(),
        );

        let narrowings = extract_narrowings(&condition, &env);
        assert_eq!(narrowings.len(), 1);
        assert_eq!(narrowings[0].var_name, "value");
        assert_eq!(narrowings[0].narrowed_type, Type::String);
    }

    #[test]
    fn test_extract_else_narrowings_eq_nil() {
        let mut env = TypeEnv::new();
        env.define_var("value".to_string(), Type::Optional(Box::new(Type::Int)));

        // value == nil (in else branch, value is not nil)
        let condition = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::Eq,
                left: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("value".into()),
                    Span::dummy(),
                )),
                right: Box::new(ast::Expr::new(ast::ExprKind::Nil, Span::dummy())),
            },
            Span::dummy(),
        );

        // In true branch, no narrowing
        let true_narrowings = extract_narrowings(&condition, &env);
        assert_eq!(true_narrowings.len(), 0);

        // In else branch, value narrows to Int
        let else_narrowings = extract_else_narrowings(&condition, &env);
        assert_eq!(else_narrowings.len(), 1);
        assert_eq!(else_narrowings[0].var_name, "value");
        assert_eq!(else_narrowings[0].narrowed_type, Type::Int);
    }

    #[test]
    fn test_extract_narrowings_type_guard() {
        let env = TypeEnv::new();

        // x is Int
        let condition = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::Is,
                left: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("x".into()),
                    Span::dummy(),
                )),
                right: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("Int".into()),
                    Span::dummy(),
                )),
            },
            Span::dummy(),
        );

        let narrowings = extract_narrowings(&condition, &env);
        assert_eq!(narrowings.len(), 1);
        assert_eq!(narrowings[0].var_name, "x");
        assert_eq!(narrowings[0].narrowed_type, Type::Int);
    }

    #[test]
    fn test_extract_narrowings_type_guard_named_type() {
        let env = TypeEnv::new();

        // obj is SomeClass
        let condition = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::Is,
                left: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("obj".into()),
                    Span::dummy(),
                )),
                right: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("SomeClass".into()),
                    Span::dummy(),
                )),
            },
            Span::dummy(),
        );

        let narrowings = extract_narrowings(&condition, &env);
        assert_eq!(narrowings.len(), 1);
        assert_eq!(narrowings[0].var_name, "obj");
        assert_eq!(
            narrowings[0].narrowed_type,
            Type::Named {
                name: "SomeClass".to_string(),
                type_args: Vec::new(),
            }
        );
    }

    #[test]
    fn test_extract_narrowings_and_combination() {
        let mut env = TypeEnv::new();
        env.define_var("a".to_string(), Type::Optional(Box::new(Type::Int)));
        env.define_var("b".to_string(), Type::Optional(Box::new(Type::String)));

        // a != nil and b != nil
        let condition = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::And,
                left: Box::new(ast::Expr::new(
                    ast::ExprKind::Binary {
                        op: ast::BinaryOp::NotEq,
                        left: Box::new(ast::Expr::new(
                            ast::ExprKind::Ident("a".into()),
                            Span::dummy(),
                        )),
                        right: Box::new(ast::Expr::new(ast::ExprKind::Nil, Span::dummy())),
                    },
                    Span::dummy(),
                )),
                right: Box::new(ast::Expr::new(
                    ast::ExprKind::Binary {
                        op: ast::BinaryOp::NotEq,
                        left: Box::new(ast::Expr::new(
                            ast::ExprKind::Ident("b".into()),
                            Span::dummy(),
                        )),
                        right: Box::new(ast::Expr::new(ast::ExprKind::Nil, Span::dummy())),
                    },
                    Span::dummy(),
                )),
            },
            Span::dummy(),
        );

        let narrowings = extract_narrowings(&condition, &env);
        assert_eq!(narrowings.len(), 2);

        // Check both narrowings are present
        let a_narrowing = narrowings.iter().find(|n| n.var_name == "a");
        let b_narrowing = narrowings.iter().find(|n| n.var_name == "b");

        assert!(a_narrowing.is_some());
        assert!(b_narrowing.is_some());
        assert_eq!(a_narrowing.unwrap().narrowed_type, Type::Int);
        assert_eq!(b_narrowing.unwrap().narrowed_type, Type::String);
    }

    #[test]
    fn test_extract_narrowings_negation() {
        let mut env = TypeEnv::new();
        env.define_var("x".to_string(), Type::Optional(Box::new(Type::Int)));

        // !(x == nil) is equivalent to x != nil
        let condition = ast::Expr::new(
            ast::ExprKind::Unary {
                op: ast::UnaryOp::Not,
                operand: Box::new(ast::Expr::new(
                    ast::ExprKind::Binary {
                        op: ast::BinaryOp::Eq,
                        left: Box::new(ast::Expr::new(
                            ast::ExprKind::Ident("x".into()),
                            Span::dummy(),
                        )),
                        right: Box::new(ast::Expr::new(ast::ExprKind::Nil, Span::dummy())),
                    },
                    Span::dummy(),
                )),
            },
            Span::dummy(),
        );

        let narrowings = extract_narrowings(&condition, &env);
        assert_eq!(narrowings.len(), 1);
        assert_eq!(narrowings[0].var_name, "x");
        assert_eq!(narrowings[0].narrowed_type, Type::Int);
    }

    #[test]
    fn test_extract_else_narrowings_or_combination() {
        let mut env = TypeEnv::new();
        env.define_var("a".to_string(), Type::Optional(Box::new(Type::Int)));
        env.define_var("b".to_string(), Type::Optional(Box::new(Type::String)));

        // a == nil or b == nil
        // In else branch, BOTH a != nil AND b != nil
        let condition = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::Or,
                left: Box::new(ast::Expr::new(
                    ast::ExprKind::Binary {
                        op: ast::BinaryOp::Eq,
                        left: Box::new(ast::Expr::new(
                            ast::ExprKind::Ident("a".into()),
                            Span::dummy(),
                        )),
                        right: Box::new(ast::Expr::new(ast::ExprKind::Nil, Span::dummy())),
                    },
                    Span::dummy(),
                )),
                right: Box::new(ast::Expr::new(
                    ast::ExprKind::Binary {
                        op: ast::BinaryOp::Eq,
                        left: Box::new(ast::Expr::new(
                            ast::ExprKind::Ident("b".into()),
                            Span::dummy(),
                        )),
                        right: Box::new(ast::Expr::new(ast::ExprKind::Nil, Span::dummy())),
                    },
                    Span::dummy(),
                )),
            },
            Span::dummy(),
        );

        let else_narrowings = extract_else_narrowings(&condition, &env);
        assert_eq!(else_narrowings.len(), 2);

        let a_narrowing = else_narrowings.iter().find(|n| n.var_name == "a");
        let b_narrowing = else_narrowings.iter().find(|n| n.var_name == "b");

        assert!(a_narrowing.is_some());
        assert!(b_narrowing.is_some());
        assert_eq!(a_narrowing.unwrap().narrowed_type, Type::Int);
        assert_eq!(b_narrowing.unwrap().narrowed_type, Type::String);
    }

    #[test]
    fn test_narrowing_kind_equality() {
        let kind1 = NarrowingKind::NilCheck {
            var_name: "x".to_string(),
            is_not_nil: true,
        };
        let kind2 = NarrowingKind::NilCheck {
            var_name: "x".to_string(),
            is_not_nil: true,
        };
        let kind3 = NarrowingKind::TypeGuard {
            var_name: "x".to_string(),
            target_type: Type::Int,
            is_positive: true,
        };

        assert_eq!(kind1, kind2);
        assert_ne!(kind1, kind3);
    }

    #[test]
    fn test_flow_type_env_clear_narrowings() {
        let mut env = FlowTypeEnv::default();
        env.define_var("x".to_string(), Type::Optional(Box::new(Type::Int)));
        env.narrow("x".to_string(), Type::Int, Span::dummy());

        assert_eq!(env.narrowings().len(), 1);

        env.clear_narrowings();

        assert_eq!(env.narrowings().len(), 0);
        // Should fall back to original type
        assert!(matches!(env.lookup_var("x"), Some(Type::Optional(_))));
    }

    #[test]
    fn test_flow_type_env_apply_narrowings() {
        let mut env = FlowTypeEnv::default();
        env.define_var("x".to_string(), Type::Optional(Box::new(Type::Int)));
        env.define_var("y".to_string(), Type::Optional(Box::new(Type::String)));

        let narrowings = vec![
            Narrowing::new("x".to_string(), Type::Int, Span::dummy()),
            Narrowing::new("y".to_string(), Type::String, Span::dummy()),
        ];

        env.apply_narrowings(narrowings);

        assert_eq!(env.lookup_var("x"), Some(Type::Int));
        assert_eq!(env.lookup_var("y"), Some(Type::String));
    }

    #[test]
    fn test_no_narrowing_for_non_optional() {
        let mut env = TypeEnv::new();
        env.define_var("x".to_string(), Type::Int); // Not Optional!

        // x != nil
        let condition = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::NotEq,
                left: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("x".into()),
                    Span::dummy(),
                )),
                right: Box::new(ast::Expr::new(ast::ExprKind::Nil, Span::dummy())),
            },
            Span::dummy(),
        );

        let narrowings = extract_narrowings(&condition, &env);
        // Should not narrow since x is not Optional
        assert_eq!(narrowings.len(), 0);
    }

    // =========================================================================
    // Transfer and Sharable Trait Tests
    // =========================================================================

    #[test]
    fn test_primitives_are_transfer() {
        // All primitive types should be Transfer
        assert!(Type::Int.is_transfer());
        assert!(Type::Int8.is_transfer());
        assert!(Type::Int64.is_transfer());
        assert!(Type::Float.is_transfer());
        assert!(Type::Float64.is_transfer());
        assert!(Type::Bool.is_transfer());
        assert!(Type::Char.is_transfer());
        assert!(Type::String.is_transfer());
        assert!(Type::Bytes.is_transfer());
        assert!(Type::Unit.is_transfer());
        assert!(Type::Never.is_transfer());
    }

    #[test]
    fn test_primitives_are_sharable() {
        // All primitive types should be Sharable
        assert!(Type::Int.is_sharable());
        assert!(Type::Int8.is_sharable());
        assert!(Type::Int64.is_sharable());
        assert!(Type::Float.is_sharable());
        assert!(Type::Float64.is_sharable());
        assert!(Type::Bool.is_sharable());
        assert!(Type::Char.is_sharable());
        assert!(Type::String.is_sharable());
        assert!(Type::Bytes.is_sharable());
        assert!(Type::Unit.is_sharable());
        assert!(Type::Never.is_sharable());
    }

    #[test]
    fn test_compound_types_transfer() {
        // Arrays of Transfer types are Transfer
        assert!(Type::Array(Box::new(Type::Int)).is_transfer());

        // Tuples of Transfer types are Transfer
        assert!(Type::Tuple(vec![Type::Int, Type::String]).is_transfer());

        // Optional of Transfer type is Transfer
        assert!(Type::Optional(Box::new(Type::Int)).is_transfer());

        // Result of Transfer types is Transfer
        assert!(Type::Result(Box::new(Type::Int), Box::new(Type::String)).is_transfer());

        // Maps of Transfer types are Transfer
        assert!(Type::Map(Box::new(Type::String), Box::new(Type::Int)).is_transfer());

        // Fixed arrays of Transfer types are Transfer
        assert!(Type::FixedArray(Box::new(Type::Int), 10).is_transfer());
    }

    #[test]
    fn test_mutable_ref_not_transfer() {
        // Mutable references are NOT Transfer (could cause data races)
        let mut_ref = Type::Reference {
            mutable: true,
            inner: Box::new(Type::Int),
        };
        assert!(!mut_ref.is_transfer());
        assert!(!mut_ref.is_sharable());
    }

    #[test]
    fn test_immutable_ref_to_sharable_is_transfer() {
        // Immutable references to Sharable types ARE Transfer
        let imm_ref = Type::Reference {
            mutable: false,
            inner: Box::new(Type::Int),
        };
        assert!(imm_ref.is_transfer());
        assert!(imm_ref.is_sharable());
    }

    #[test]
    fn test_type_var_not_transfer() {
        // Type variables are conservatively non-Transfer (unknown type)
        let var = Type::Var(TypeVar(0));
        assert!(!var.is_transfer());
        assert!(!var.is_sharable());
    }

    #[test]
    fn test_error_type_not_transfer() {
        // Error types are not Transfer
        assert!(!Type::Error.is_transfer());
        assert!(!Type::Error.is_sharable());
    }

    #[test]
    fn test_function_type_is_transfer() {
        // Function types are Transfer (captures checked separately)
        let func = Type::Function {
            params: vec![Type::Int],
            return_type: Box::new(Type::String),
        };
        assert!(func.is_transfer());
        assert!(func.is_sharable());
    }

    #[test]
    fn test_named_type_is_transfer() {
        // Named types are assumed Transfer by default
        let named = Type::Named {
            name: "MyStruct".to_string(),
            type_args: vec![Type::Int],
        };
        assert!(named.is_transfer());
        assert!(named.is_sharable());
    }

    #[test]
    fn test_spawn_safe() {
        // is_spawn_safe should match is_transfer
        assert!(Type::Int.is_spawn_safe());
        assert!(Type::String.is_spawn_safe());
        assert!(Type::Array(Box::new(Type::Int)).is_spawn_safe());

        let mut_ref = Type::Reference {
            mutable: true,
            inner: Box::new(Type::Int),
        };
        assert!(!mut_ref.is_spawn_safe());

        assert!(!Type::Var(TypeVar(0)).is_spawn_safe());
    }

    #[test]
    fn test_nested_non_transfer() {
        // Array containing a mutable reference should not be Transfer
        let mut_ref = Type::Reference {
            mutable: true,
            inner: Box::new(Type::Int),
        };
        let array_of_mut_ref = Type::Array(Box::new(mut_ref.clone()));
        assert!(!array_of_mut_ref.is_transfer());

        // Tuple containing a mutable reference should not be Transfer
        let tuple_with_mut_ref = Type::Tuple(vec![Type::Int, mut_ref]);
        assert!(!tuple_with_mut_ref.is_transfer());
    }

    // =========================================================================
    // Spawn Capture Checking Tests
    // =========================================================================

    #[test]
    fn test_check_spawn_captures_all_transfer() {
        let checker = TypeChecker::new();

        // All Transfer types should pass
        let captures = vec![
            ("x".to_string(), Type::Int),
            ("y".to_string(), Type::String),
            ("z".to_string(), Type::Array(Box::new(Type::Int))),
        ];

        let result = checker.check_spawn_captures(&captures, Span::dummy());
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_spawn_captures_non_transfer() {
        let checker = TypeChecker::new();

        // A mutable reference is not Transfer
        let mut_ref = Type::Reference {
            mutable: true,
            inner: Box::new(Type::Int),
        };

        let captures = vec![
            ("x".to_string(), Type::Int),
            ("conn".to_string(), mut_ref),  // Non-Transfer!
        ];

        let result = checker.check_spawn_captures(&captures, Span::dummy());
        assert!(result.is_err());

        match result.unwrap_err() {
            TypeError::NonTransferCapture { var_name, .. } => {
                assert_eq!(var_name, "conn");
            }
            e => panic!("Expected NonTransferCapture error, got {:?}", e),
        }
    }

    #[test]
    fn test_check_spawn_captures_type_var() {
        let checker = TypeChecker::new();

        // Type variables are conservatively non-Transfer
        let captures = vec![
            ("x".to_string(), Type::Var(TypeVar(0))),
        ];

        let result = checker.check_spawn_captures(&captures, Span::dummy());
        assert!(result.is_err());
    }

    #[test]
    fn test_check_spawn_captures_empty() {
        let checker = TypeChecker::new();

        // No captures is always safe
        let captures: Vec<(String, Type)> = vec![];
        let result = checker.check_spawn_captures(&captures, Span::dummy());
        assert!(result.is_ok());
    }

    #[test]
    fn test_collect_lambda_captures_simple() {
        let checker = TypeChecker::new();
        let mut env = TypeEnv::new();
        env.define_var("outer_x".to_string(), Type::Int);
        env.define_var("outer_y".to_string(), Type::String);

        // Lambda: |a| a + outer_x
        let lambda_params = vec![
            ast::Param {
                mutable: false,
                name: ast::Spanned::dummy("a".into()),
                ty: None,
                default: None,
                span: Span::dummy(),
            },
        ];

        // Body: a + outer_x (outer_x is captured)
        let lambda_body = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::Add,
                left: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("a".into()),
                    Span::dummy(),
                )),
                right: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("outer_x".into()),
                    Span::dummy(),
                )),
            },
            Span::dummy(),
        );

        let captures = checker.collect_lambda_captures(&lambda_params, &lambda_body, &env);

        // Should capture outer_x but not 'a' (which is a parameter)
        assert_eq!(captures.len(), 1);
        assert_eq!(captures[0].0, "outer_x");
        assert_eq!(captures[0].1, Type::Int);
    }

    #[test]
    fn test_collect_lambda_captures_no_capture() {
        let checker = TypeChecker::new();
        let mut env = TypeEnv::new();
        env.define_var("outer".to_string(), Type::Int);

        // Lambda: |x| x * 2 (no captures)
        let lambda_params = vec![
            ast::Param {
                mutable: false,
                name: ast::Spanned::dummy("x".into()),
                ty: None,
                default: None,
                span: Span::dummy(),
            },
        ];

        let lambda_body = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::Mul,
                left: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("x".into()),
                    Span::dummy(),
                )),
                right: Box::new(ast::Expr::new(
                    ast::ExprKind::Integer("2".into()),
                    Span::dummy(),
                )),
            },
            Span::dummy(),
        );

        let captures = checker.collect_lambda_captures(&lambda_params, &lambda_body, &env);
        assert!(captures.is_empty());
    }

    // =========================================================================
    // Channel Type Tests
    // =========================================================================

    #[test]
    fn test_channel_type_display() {
        let ch = Type::Channel(Box::new(Type::Int));
        assert_eq!(format!("{}", ch), "Channel[Int]");

        let ch_string = Type::Channel(Box::new(Type::String));
        assert_eq!(format!("{}", ch_string), "Channel[String]");

        // Nested channel type
        let ch_nested = Type::Channel(Box::new(Type::Array(Box::new(Type::Int))));
        assert_eq!(format!("{}", ch_nested), "Channel[[Int]]");
    }

    #[test]
    fn test_channel_is_transfer_if_elem_is_transfer() {
        // Channel[Int] is Transfer because Int is Transfer
        let ch_int = Type::Channel(Box::new(Type::Int));
        assert!(ch_int.is_transfer());

        // Channel[String] is Transfer because String is Transfer
        let ch_string = Type::Channel(Box::new(Type::String));
        assert!(ch_string.is_transfer());

        // Channel[Array[Int]] is Transfer because Array[Int] is Transfer
        let ch_array = Type::Channel(Box::new(Type::Array(Box::new(Type::Int))));
        assert!(ch_array.is_transfer());
    }

    #[test]
    fn test_channel_not_transfer_if_elem_not_transfer() {
        // Channel[&mut Int] is NOT Transfer because &mut Int is not Transfer
        let ch_mut_ref = Type::Channel(Box::new(Type::Reference {
            mutable: true,
            inner: Box::new(Type::Int),
        }));
        assert!(!ch_mut_ref.is_transfer());

        // Channel[TypeVar] is NOT Transfer because TypeVar is not Transfer
        let ch_var = Type::Channel(Box::new(Type::Var(TypeVar(0))));
        assert!(!ch_var.is_transfer());
    }

    #[test]
    fn test_channel_is_always_sharable() {
        // Channels are always Sharable - they're designed for sharing between tasks

        // Channel[Int] is Sharable
        let ch_int = Type::Channel(Box::new(Type::Int));
        assert!(ch_int.is_sharable());

        // Channel[&mut Int] is still Sharable (channel handle can be shared)
        let ch_mut_ref = Type::Channel(Box::new(Type::Reference {
            mutable: true,
            inner: Box::new(Type::Int),
        }));
        assert!(ch_mut_ref.is_sharable());

        // Channel[TypeVar] is Sharable
        let ch_var = Type::Channel(Box::new(Type::Var(TypeVar(0))));
        assert!(ch_var.is_sharable());
    }

    #[test]
    fn test_channel_spawn_safe() {
        // Channel is spawn-safe if element is Transfer
        let ch_int = Type::Channel(Box::new(Type::Int));
        assert!(ch_int.is_spawn_safe());

        // Channel is NOT spawn-safe if element is not Transfer
        let ch_mut_ref = Type::Channel(Box::new(Type::Reference {
            mutable: true,
            inner: Box::new(Type::Int),
        }));
        assert!(!ch_mut_ref.is_spawn_safe());
    }

    #[test]
    fn test_channel_in_compound_types() {
        // Array of channels
        let array_of_ch = Type::Array(Box::new(Type::Channel(Box::new(Type::Int))));
        assert!(array_of_ch.is_transfer());
        assert!(array_of_ch.is_sharable());

        // Tuple containing a channel
        let tuple_with_ch = Type::Tuple(vec![
            Type::Int,
            Type::Channel(Box::new(Type::String)),
        ]);
        assert!(tuple_with_ch.is_transfer());
        assert!(tuple_with_ch.is_sharable());

        // Optional channel
        let opt_ch = Type::Optional(Box::new(Type::Channel(Box::new(Type::Int))));
        assert!(opt_ch.is_transfer());
        assert!(opt_ch.is_sharable());
    }

    #[test]
    fn test_copy_primitives() {
        // All numeric types are Copy
        assert!(Type::Int.is_copy());
        assert!(Type::Int8.is_copy());
        assert!(Type::Int16.is_copy());
        assert!(Type::Int32.is_copy());
        assert!(Type::Int64.is_copy());
        assert!(Type::Int128.is_copy());
        assert!(Type::UInt.is_copy());
        assert!(Type::UInt8.is_copy());
        assert!(Type::UInt16.is_copy());
        assert!(Type::UInt32.is_copy());
        assert!(Type::UInt64.is_copy());
        assert!(Type::UInt128.is_copy());
        assert!(Type::Float.is_copy());
        assert!(Type::Float32.is_copy());
        assert!(Type::Float64.is_copy());

        // Bool, Char, Unit, Never are Copy
        assert!(Type::Bool.is_copy());
        assert!(Type::Char.is_copy());
        assert!(Type::Unit.is_copy());
        assert!(Type::Never.is_copy());

        // String and Bytes are NOT Copy (own heap data)
        assert!(!Type::String.is_copy());
        assert!(!Type::Bytes.is_copy());
    }

    #[test]
    fn test_copy_compound_types() {
        // Dynamic array is NOT Copy
        let arr = Type::Array(Box::new(Type::Int));
        assert!(!arr.is_copy());

        // Fixed array of Copy type IS Copy
        let fixed_arr = Type::FixedArray(Box::new(Type::Int), 10);
        assert!(fixed_arr.is_copy());

        // Fixed array of non-Copy type is NOT Copy
        let fixed_arr_string = Type::FixedArray(Box::new(Type::String), 5);
        assert!(!fixed_arr_string.is_copy());

        // Map is NOT Copy
        let map = Type::Map(Box::new(Type::String), Box::new(Type::Int));
        assert!(!map.is_copy());

        // Tuple of Copy types IS Copy
        let tuple_copy = Type::Tuple(vec![Type::Int, Type::Bool, Type::Char]);
        assert!(tuple_copy.is_copy());

        // Tuple containing non-Copy type is NOT Copy
        let tuple_non_copy = Type::Tuple(vec![Type::Int, Type::String]);
        assert!(!tuple_non_copy.is_copy());

        // Empty tuple is Copy
        let empty_tuple = Type::Tuple(vec![]);
        assert!(empty_tuple.is_copy());
    }

    #[test]
    fn test_copy_optional_result() {
        // Optional of Copy type IS Copy
        let opt_int = Type::Optional(Box::new(Type::Int));
        assert!(opt_int.is_copy());

        // Optional of non-Copy type is NOT Copy
        let opt_string = Type::Optional(Box::new(Type::String));
        assert!(!opt_string.is_copy());

        // Result of Copy types IS Copy
        let result_copy = Type::Result(Box::new(Type::Int), Box::new(Type::Bool));
        assert!(result_copy.is_copy());

        // Result with non-Copy Ok is NOT Copy
        let result_non_copy_ok = Type::Result(Box::new(Type::String), Box::new(Type::Int));
        assert!(!result_non_copy_ok.is_copy());

        // Result with non-Copy Err is NOT Copy
        let result_non_copy_err = Type::Result(Box::new(Type::Int), Box::new(Type::String));
        assert!(!result_non_copy_err.is_copy());
    }

    #[test]
    fn test_copy_references() {
        // Immutable reference IS Copy (we copy the reference, not the data)
        let ref_int = Type::Reference {
            mutable: false,
            inner: Box::new(Type::Int),
        };
        assert!(ref_int.is_copy());

        // Mutable reference IS Copy (in Rust &mut T is Copy)
        let mut_ref_int = Type::Reference {
            mutable: true,
            inner: Box::new(Type::Int),
        };
        assert!(mut_ref_int.is_copy());

        // Reference to non-Copy type is still Copy (reference itself is just a pointer)
        let ref_string = Type::Reference {
            mutable: false,
            inner: Box::new(Type::String),
        };
        assert!(ref_string.is_copy());
    }

    #[test]
    fn test_copy_functions_channels() {
        // Functions are NOT Copy (may capture owned state)
        let func = Type::Function {
            params: vec![Type::Int],
            return_type: Box::new(Type::Bool),
        };
        assert!(!func.is_copy());

        // Channels are NOT Copy (unique handles)
        let channel = Type::Channel(Box::new(Type::Int));
        assert!(!channel.is_copy());
    }

    #[test]
    fn test_copy_named_and_special() {
        // Named types are NOT Copy by default
        let named = Type::Named {
            name: "MyStruct".to_string(),
            type_args: vec![],
        };
        assert!(!named.is_copy());

        // Type variables are NOT Copy (unknown)
        let var = Type::Var(TypeVar(0));
        assert!(!var.is_copy());

        // Error type is NOT Copy
        assert!(!Type::Error.is_copy());

        // Any type is NOT Copy (be conservative)
        assert!(!Type::Any.is_copy());
    }

    // =========================================================================
    // Contract Expression Type Checking Tests
    // =========================================================================

    #[test]
    fn test_result_expression_type_in_ensures() {
        // The `result` variable should have the function's return type
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        // Simulate an ensures clause context where result is bound to Int
        env.define_var("result".to_string(), Type::Int);

        let result_expr = ast::Expr::new(
            ast::ExprKind::Result,
            Span::dummy(),
        );

        let ty = checker.infer_expr(&result_expr, &env).unwrap();
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn test_old_expression_preserves_type() {
        // old(expr) should have the same type as expr
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();
        env.define_var("x".to_string(), Type::Int);

        let old_expr = ast::Expr::new(
            ast::ExprKind::Old(Box::new(ast::Expr::new(
                ast::ExprKind::Ident("x".into()),
                Span::dummy(),
            ))),
            Span::dummy(),
        );

        let ty = checker.infer_expr(&old_expr, &env).unwrap();
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn test_forall_expression_returns_bool() {
        // forall x: Int => x > 0 should be Bool
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        let forall_expr = ast::Expr::new(
            ast::ExprKind::Forall {
                var: ast::Spanned::dummy("x".into()),
                ty: ast::TypeExpr::Named(ast::Spanned::dummy("Int".into())),
                condition: None,
                body: Box::new(ast::Expr::new(
                    ast::ExprKind::Binary {
                        op: ast::BinaryOp::Gt,
                        left: Box::new(ast::Expr::new(
                            ast::ExprKind::Ident("x".into()),
                            Span::dummy(),
                        )),
                        right: Box::new(ast::Expr::new(
                            ast::ExprKind::Integer("0".into()),
                            Span::dummy(),
                        )),
                    },
                    Span::dummy(),
                )),
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&forall_expr, &env).unwrap();
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_exists_expression_returns_bool() {
        // exists x: Int => x == 0 should be Bool
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        let exists_expr = ast::Expr::new(
            ast::ExprKind::Exists {
                var: ast::Spanned::dummy("x".into()),
                ty: ast::TypeExpr::Named(ast::Spanned::dummy("Int".into())),
                condition: None,
                body: Box::new(ast::Expr::new(
                    ast::ExprKind::Binary {
                        op: ast::BinaryOp::Eq,
                        left: Box::new(ast::Expr::new(
                            ast::ExprKind::Ident("x".into()),
                            Span::dummy(),
                        )),
                        right: Box::new(ast::Expr::new(
                            ast::ExprKind::Integer("0".into()),
                            Span::dummy(),
                        )),
                    },
                    Span::dummy(),
                )),
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&exists_expr, &env).unwrap();
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_forall_with_condition() {
        // forall x: Int where x > 0 => x * 2 > x should be Bool
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        let forall_expr = ast::Expr::new(
            ast::ExprKind::Forall {
                var: ast::Spanned::dummy("x".into()),
                ty: ast::TypeExpr::Named(ast::Spanned::dummy("Int".into())),
                condition: Some(Box::new(ast::Expr::new(
                    ast::ExprKind::Binary {
                        op: ast::BinaryOp::Gt,
                        left: Box::new(ast::Expr::new(
                            ast::ExprKind::Ident("x".into()),
                            Span::dummy(),
                        )),
                        right: Box::new(ast::Expr::new(
                            ast::ExprKind::Integer("0".into()),
                            Span::dummy(),
                        )),
                    },
                    Span::dummy(),
                ))),
                body: Box::new(ast::Expr::new(
                    ast::ExprKind::Bool(true),
                    Span::dummy(),
                )),
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&forall_expr, &env).unwrap();
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_forall_body_must_be_bool() {
        // forall x: Int => x + 1 should fail (body is Int, not Bool)
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        let forall_expr = ast::Expr::new(
            ast::ExprKind::Forall {
                var: ast::Spanned::dummy("x".into()),
                ty: ast::TypeExpr::Named(ast::Spanned::dummy("Int".into())),
                condition: None,
                body: Box::new(ast::Expr::new(
                    ast::ExprKind::Binary {
                        op: ast::BinaryOp::Add,
                        left: Box::new(ast::Expr::new(
                            ast::ExprKind::Ident("x".into()),
                            Span::dummy(),
                        )),
                        right: Box::new(ast::Expr::new(
                            ast::ExprKind::Integer("1".into()),
                            Span::dummy(),
                        )),
                    },
                    Span::dummy(),
                )),
            },
            Span::dummy(),
        );

        let result = checker.infer_expr(&forall_expr, &env);
        assert!(result.is_err());
    }

    #[test]
    fn test_quantifier_variable_scoping() {
        // The quantified variable should be available in the body
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        // forall y: Int => y > 0 - y should be in scope
        let forall_expr = ast::Expr::new(
            ast::ExprKind::Forall {
                var: ast::Spanned::dummy("y".into()),
                ty: ast::TypeExpr::Named(ast::Spanned::dummy("Int".into())),
                condition: None,
                body: Box::new(ast::Expr::new(
                    ast::ExprKind::Binary {
                        op: ast::BinaryOp::Gt,
                        left: Box::new(ast::Expr::new(
                            ast::ExprKind::Ident("y".into()),
                            Span::dummy(),
                        )),
                        right: Box::new(ast::Expr::new(
                            ast::ExprKind::Integer("0".into()),
                            Span::dummy(),
                        )),
                    },
                    Span::dummy(),
                )),
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&forall_expr, &env).unwrap();
        assert_eq!(ty, Type::Bool);
    }

    // ========================================================================
    // Return Type Validation Tests (ARIA-M21-TYPEINF)
    // ========================================================================

    #[test]
    fn test_return_type_validation_correct_type() {
        // fn add(a: Int, b: Int) -> Int { return a + b }
        let mut checker = TypeChecker::new();

        let func = ast::FunctionDecl {
            attributes: vec![],
            visibility: ast::Visibility::Private,
            name: ast::Spanned::dummy("add".into()),
            generic_params: None,
            params: vec![
                ast::Param {
                    mutable: false,
                    name: ast::Spanned::dummy("a".into()),
                    ty: Some(ast::TypeExpr::Named(ast::Spanned::dummy("Int".into()))),
                    default: None,
                    span: Span::dummy(),
                },
                ast::Param {
                    mutable: false,
                    name: ast::Spanned::dummy("b".into()),
                    ty: Some(ast::TypeExpr::Named(ast::Spanned::dummy("Int".into()))),
                    default: None,
                    span: Span::dummy(),
                },
            ],
            return_type: Some(ast::TypeExpr::Named(ast::Spanned::dummy("Int".into()))),
            where_clause: None,
            contracts: vec![],
            body: ast::FunctionBody::Block(ast::Block {
                stmts: vec![ast::Stmt {
                    kind: ast::StmtKind::Return(Some(ast::Expr::new(
                        ast::ExprKind::Binary {
                            op: ast::BinaryOp::Add,
                            left: Box::new(ast::Expr::new(
                                ast::ExprKind::Ident("a".into()),
                                Span::dummy(),
                            )),
                            right: Box::new(ast::Expr::new(
                                ast::ExprKind::Ident("b".into()),
                                Span::dummy(),
                            )),
                        },
                        Span::dummy(),
                    ))),
                    span: Span::dummy(),
                }],
                span: Span::dummy(),
            }),
            test_block: None,
            span: Span::dummy(),
        };

        // Should succeed because return type matches
        let result = checker.check_function(&func);
        assert!(result.is_ok(), "Expected success, got {:?}", result);
    }

    #[test]
    fn test_return_type_validation_mismatch() {
        // fn bad() -> Int { return "hello" }
        let mut checker = TypeChecker::new();

        let func = ast::FunctionDecl {
            attributes: vec![],
            visibility: ast::Visibility::Private,
            name: ast::Spanned::dummy("bad".into()),
            generic_params: None,
            params: vec![],
            return_type: Some(ast::TypeExpr::Named(ast::Spanned::dummy("Int".into()))),
            where_clause: None,
            contracts: vec![],
            body: ast::FunctionBody::Block(ast::Block {
                stmts: vec![ast::Stmt {
                    kind: ast::StmtKind::Return(Some(ast::Expr::new(
                        ast::ExprKind::String("hello".into()),
                        Span::dummy(),
                    ))),
                    span: Span::dummy(),
                }],
                span: Span::dummy(),
            }),
            test_block: None,
            span: Span::dummy(),
        };

        // Should fail because returning String instead of Int
        let result = checker.check_function(&func);
        assert!(result.is_err(), "Expected error for type mismatch");

        // Verify it's a ReturnTypeMismatch error
        if let Err(TypeError::ReturnTypeMismatch { expected, found, .. }) = result {
            assert_eq!(expected, "Int");
            assert_eq!(found, "String");
        } else {
            panic!("Expected ReturnTypeMismatch error, got {:?}", result);
        }
    }

    #[test]
    fn test_return_type_validation_unit_return() {
        // fn greet() -> () { return }
        let mut checker = TypeChecker::new();

        let func = ast::FunctionDecl {
            attributes: vec![],
            visibility: ast::Visibility::Private,
            name: ast::Spanned::dummy("greet".into()),
            generic_params: None,
            params: vec![],
            return_type: Some(ast::TypeExpr::Named(ast::Spanned::dummy("Unit".into()))),
            where_clause: None,
            contracts: vec![],
            body: ast::FunctionBody::Block(ast::Block {
                stmts: vec![ast::Stmt {
                    kind: ast::StmtKind::Return(None), // return without expression
                    span: Span::dummy(),
                }],
                span: Span::dummy(),
            }),
            test_block: None,
            span: Span::dummy(),
        };

        // Should succeed because empty return matches Unit
        let result = checker.check_function(&func);
        assert!(result.is_ok(), "Expected success for Unit return, got {:?}", result);
    }

    #[test]
    fn test_return_type_validation_unit_mismatch() {
        // fn bad() -> Int { return }  // returning nothing when Int expected
        let mut checker = TypeChecker::new();

        let func = ast::FunctionDecl {
            attributes: vec![],
            visibility: ast::Visibility::Private,
            name: ast::Spanned::dummy("bad".into()),
            generic_params: None,
            params: vec![],
            return_type: Some(ast::TypeExpr::Named(ast::Spanned::dummy("Int".into()))),
            where_clause: None,
            contracts: vec![],
            body: ast::FunctionBody::Block(ast::Block {
                stmts: vec![ast::Stmt {
                    kind: ast::StmtKind::Return(None), // return without expression
                    span: Span::dummy(),
                }],
                span: Span::dummy(),
            }),
            test_block: None,
            span: Span::dummy(),
        };

        // Should fail because returning () instead of Int
        let result = checker.check_function(&func);
        assert!(result.is_err(), "Expected error for Unit/Int mismatch");
    }

    #[test]
    fn test_return_type_validation_multiple_returns() {
        // fn check(x: Int) -> Bool {
        //     if x > 0 {
        //         return true
        //     }
        //     return false
        // }
        let mut checker = TypeChecker::new();

        let func = ast::FunctionDecl {
            attributes: vec![],
            visibility: ast::Visibility::Private,
            name: ast::Spanned::dummy("check".into()),
            generic_params: None,
            params: vec![ast::Param {
                mutable: false,
                name: ast::Spanned::dummy("x".into()),
                ty: Some(ast::TypeExpr::Named(ast::Spanned::dummy("Int".into()))),
                default: None,
                span: Span::dummy(),
            }],
            return_type: Some(ast::TypeExpr::Named(ast::Spanned::dummy("Bool".into()))),
            where_clause: None,
            contracts: vec![],
            body: ast::FunctionBody::Block(ast::Block {
                stmts: vec![
                    ast::Stmt {
                        kind: ast::StmtKind::If {
                            condition: ast::Expr::new(
                                ast::ExprKind::Binary {
                                    op: ast::BinaryOp::Gt,
                                    left: Box::new(ast::Expr::new(
                                        ast::ExprKind::Ident("x".into()),
                                        Span::dummy(),
                                    )),
                                    right: Box::new(ast::Expr::new(
                                        ast::ExprKind::Integer("0".into()),
                                        Span::dummy(),
                                    )),
                                },
                                Span::dummy(),
                            ),
                            then_branch: ast::Block {
                                stmts: vec![ast::Stmt {
                                    kind: ast::StmtKind::Return(Some(ast::Expr::new(
                                        ast::ExprKind::Bool(true),
                                        Span::dummy(),
                                    ))),
                                    span: Span::dummy(),
                                }],
                                span: Span::dummy(),
                            },
                            elsif_branches: vec![],
                            else_branch: None,
                        },
                        span: Span::dummy(),
                    },
                    ast::Stmt {
                        kind: ast::StmtKind::Return(Some(ast::Expr::new(
                            ast::ExprKind::Bool(false),
                            Span::dummy(),
                        ))),
                        span: Span::dummy(),
                    },
                ],
                span: Span::dummy(),
            }),
            test_block: None,
            span: Span::dummy(),
        };

        // Should succeed because both returns are Bool
        let result = checker.check_function(&func);
        assert!(result.is_ok(), "Expected success for multiple matching returns, got {:?}", result);
    }

    #[test]
    fn test_return_type_validation_expression_body() {
        // fn double(x: Int) -> Int = x * 2
        let mut checker = TypeChecker::new();

        let func = ast::FunctionDecl {
            attributes: vec![],
            visibility: ast::Visibility::Private,
            name: ast::Spanned::dummy("double".into()),
            generic_params: None,
            params: vec![ast::Param {
                mutable: false,
                name: ast::Spanned::dummy("x".into()),
                ty: Some(ast::TypeExpr::Named(ast::Spanned::dummy("Int".into()))),
                default: None,
                span: Span::dummy(),
            }],
            return_type: Some(ast::TypeExpr::Named(ast::Spanned::dummy("Int".into()))),
            where_clause: None,
            contracts: vec![],
            body: ast::FunctionBody::Expression(Box::new(ast::Expr::new(
                ast::ExprKind::Binary {
                    op: ast::BinaryOp::Mul,
                    left: Box::new(ast::Expr::new(
                        ast::ExprKind::Ident("x".into()),
                        Span::dummy(),
                    )),
                    right: Box::new(ast::Expr::new(
                        ast::ExprKind::Integer("2".into()),
                        Span::dummy(),
                    )),
                },
                Span::dummy(),
            ))),
            test_block: None,
            span: Span::dummy(),
        };

        // Should succeed because expression body type matches return type
        let result = checker.check_function(&func);
        assert!(result.is_ok(), "Expected success for expression body, got {:?}", result);
    }

    #[test]
    fn test_return_type_error_message() {
        // Verify the ReturnTypeMismatch error produces helpful messages
        let error = TypeError::ReturnTypeMismatch {
            expected: "Int".to_string(),
            found: "String".to_string(),
            span: Span::dummy(),
        };

        let error_msg = format!("{}", error);
        assert!(error_msg.contains("Int"), "Error should mention expected type");
        assert!(error_msg.contains("String"), "Error should mention found type");
        assert!(error_msg.contains("Return type mismatch"), "Error should indicate return type mismatch");
    }

    // =========================================================================
    // Error Context/Wrapping Tests (ARIA-M13)
    // =========================================================================

    #[test]
    fn test_result_context_method() {
        // Result<T, E>.context("msg") -> Result<T, ContextError<E>>
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        // Define a Result<Int, String> variable
        env.define_var(
            "result".to_string(),
            Type::Result(Box::new(Type::Int), Box::new(Type::String)),
        );

        // Create: result.context("failed to do something")
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("result".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("context".into(), Span::dummy()),
                args: vec![ast::Expr::new(
                    ast::ExprKind::String("failed to do something".into()),
                    Span::dummy(),
                )],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();

        // Should be Result<Int, ContextError<String>>
        match ty {
            Type::Result(ok, err) => {
                assert_eq!(*ok, Type::Int);
                match *err {
                    Type::Named { name, type_args } => {
                        assert_eq!(name, "ContextError");
                        assert_eq!(type_args.len(), 1);
                        assert_eq!(type_args[0], Type::String);
                    }
                    _ => panic!("Expected ContextError named type, got {:?}", err),
                }
            }
            _ => panic!("Expected Result type, got {:?}", ty),
        }
    }

    #[test]
    fn test_result_context_requires_string_arg() {
        // context() should require a String argument
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "result".to_string(),
            Type::Result(Box::new(Type::Int), Box::new(Type::String)),
        );

        // Create: result.context(42) - wrong type
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("result".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("context".into(), Span::dummy()),
                args: vec![ast::Expr::new(
                    ast::ExprKind::Integer("42".into()),
                    Span::dummy(),
                )],
            },
            Span::dummy(),
        );

        let result = checker.infer_expr(&method_call, &env);
        assert!(result.is_err(), "context(Int) should fail type check");
    }

    #[test]
    fn test_result_with_context_method() {
        // Result<T, E>.with_context(|| "msg") -> Result<T, ContextError<E>>
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "result".to_string(),
            Type::Result(Box::new(Type::Int), Box::new(Type::String)),
        );

        // Create: result.with_context(|| "computed message")
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("result".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("with_context".into(), Span::dummy()),
                args: vec![ast::Expr::new(
                    ast::ExprKind::Lambda {
                        params: vec![],
                        body: Box::new(ast::Expr::new(
                            ast::ExprKind::String("computed message".into()),
                            Span::dummy(),
                        )),
                    },
                    Span::dummy(),
                )],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();

        // Should be Result<Int, ContextError<String>>
        match ty {
            Type::Result(ok, err) => {
                assert_eq!(*ok, Type::Int);
                match *err {
                    Type::Named { name, .. } => {
                        assert_eq!(name, "ContextError");
                    }
                    _ => panic!("Expected ContextError named type"),
                }
            }
            _ => panic!("Expected Result type, got {:?}", ty),
        }
    }

    #[test]
    fn test_result_map_err_method() {
        // Result<T, E>.map_err(|e| f(e)) -> Result<T, E2>
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "result".to_string(),
            Type::Result(Box::new(Type::Int), Box::new(Type::String)),
        );

        // Create: result.map_err(|e| 42) - transform String error to Int
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("result".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("map_err".into(), Span::dummy()),
                args: vec![ast::Expr::new(
                    ast::ExprKind::Lambda {
                        params: vec![ast::Param {
                            mutable: false,
                            name: ast::Ident::new("e".into(), Span::dummy()),
                            ty: None,
                            default: None,
                            span: Span::dummy(),
                        }],
                        body: Box::new(ast::Expr::new(
                            ast::ExprKind::Integer("42".into()),
                            Span::dummy(),
                        )),
                    },
                    Span::dummy(),
                )],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();

        // Should be Result<Int, Int> - error type transformed
        match ty {
            Type::Result(ok, err) => {
                assert_eq!(*ok, Type::Int);
                assert_eq!(*err, Type::Int);
            }
            _ => panic!("Expected Result type, got {:?}", ty),
        }
    }

    #[test]
    fn test_result_map_method() {
        // Result<T, E>.map(|t| f(t)) -> Result<U, E>
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "result".to_string(),
            Type::Result(Box::new(Type::Int), Box::new(Type::String)),
        );

        // Create: result.map(|x| true) - transform Int to Bool
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("result".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("map".into(), Span::dummy()),
                args: vec![ast::Expr::new(
                    ast::ExprKind::Lambda {
                        params: vec![ast::Param {
                            mutable: false,
                            name: ast::Ident::new("x".into(), Span::dummy()),
                            ty: None,
                            default: None,
                            span: Span::dummy(),
                        }],
                        body: Box::new(ast::Expr::new(
                            ast::ExprKind::Bool(true),
                            Span::dummy(),
                        )),
                    },
                    Span::dummy(),
                )],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();

        // Should be Result<Bool, String>
        match ty {
            Type::Result(ok, err) => {
                assert_eq!(*ok, Type::Bool);
                assert_eq!(*err, Type::String);
            }
            _ => panic!("Expected Result type, got {:?}", ty),
        }
    }

    #[test]
    fn test_result_is_ok_is_err() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "result".to_string(),
            Type::Result(Box::new(Type::Int), Box::new(Type::String)),
        );

        // result.is_ok()
        let is_ok_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("result".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("is_ok".into(), Span::dummy()),
                args: vec![],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&is_ok_call, &env).unwrap();
        assert_eq!(ty, Type::Bool);

        // result.is_err()
        let is_err_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("result".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("is_err".into(), Span::dummy()),
                args: vec![],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&is_err_call, &env).unwrap();
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_result_ok_err_methods() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "result".to_string(),
            Type::Result(Box::new(Type::Int), Box::new(Type::String)),
        );

        // result.ok() -> Int?
        let ok_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("result".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("ok".into(), Span::dummy()),
                args: vec![],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&ok_call, &env).unwrap();
        assert_eq!(ty, Type::Optional(Box::new(Type::Int)));

        // result.err() -> String?
        let err_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("result".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("err".into(), Span::dummy()),
                args: vec![],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&err_call, &env).unwrap();
        assert_eq!(ty, Type::Optional(Box::new(Type::String)));
    }

    #[test]
    fn test_result_unwrap_or() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "result".to_string(),
            Type::Result(Box::new(Type::Int), Box::new(Type::String)),
        );

        // result.unwrap_or(0) -> Int
        let unwrap_or_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("result".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("unwrap_or".into(), Span::dummy()),
                args: vec![ast::Expr::new(
                    ast::ExprKind::Integer("0".into()),
                    Span::dummy(),
                )],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&unwrap_or_call, &env).unwrap();
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn test_optional_context_method() {
        // T?.context("msg") -> Result<T, ContextError<()>>
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "opt".to_string(),
            Type::Optional(Box::new(Type::Int)),
        );

        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("opt".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("context".into(), Span::dummy()),
                args: vec![ast::Expr::new(
                    ast::ExprKind::String("value was none".into()),
                    Span::dummy(),
                )],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();

        // Should be Result<Int, ContextError<()>>
        match ty {
            Type::Result(ok, err) => {
                assert_eq!(*ok, Type::Int);
                match *err {
                    Type::Named { name, type_args } => {
                        assert_eq!(name, "ContextError");
                        assert_eq!(type_args.len(), 1);
                        assert_eq!(type_args[0], Type::Unit);
                    }
                    _ => panic!("Expected ContextError named type"),
                }
            }
            _ => panic!("Expected Result type, got {:?}", ty),
        }
    }

    #[test]
    fn test_optional_ok_or_method() {
        // T?.ok_or(err) -> Result<T, E>
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "opt".to_string(),
            Type::Optional(Box::new(Type::Int)),
        );

        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("opt".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("ok_or".into(), Span::dummy()),
                args: vec![ast::Expr::new(
                    ast::ExprKind::String("not found".into()),
                    Span::dummy(),
                )],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();

        // Should be Result<Int, String>
        match ty {
            Type::Result(ok, err) => {
                assert_eq!(*ok, Type::Int);
                assert_eq!(*err, Type::String);
            }
            _ => panic!("Expected Result type, got {:?}", ty),
        }
    }

    #[test]
    fn test_optional_map_method() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "opt".to_string(),
            Type::Optional(Box::new(Type::Int)),
        );

        // opt.map(|x| true) -> Bool?
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("opt".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("map".into(), Span::dummy()),
                args: vec![ast::Expr::new(
                    ast::ExprKind::Lambda {
                        params: vec![ast::Param {
                            mutable: false,
                            name: ast::Ident::new("x".into(), Span::dummy()),
                            ty: None,
                            default: None,
                            span: Span::dummy(),
                        }],
                        body: Box::new(ast::Expr::new(
                            ast::ExprKind::Bool(true),
                            Span::dummy(),
                        )),
                    },
                    Span::dummy(),
                )],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();
        assert_eq!(ty, Type::Optional(Box::new(Type::Bool)));
    }

    #[test]
    fn test_optional_is_some_is_none() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "opt".to_string(),
            Type::Optional(Box::new(Type::Int)),
        );

        // opt.is_some() -> Bool
        let is_some_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("opt".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("is_some".into(), Span::dummy()),
                args: vec![],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&is_some_call, &env).unwrap();
        assert_eq!(ty, Type::Bool);

        // opt.is_none() -> Bool
        let is_none_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("opt".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("is_none".into(), Span::dummy()),
                args: vec![],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&is_none_call, &env).unwrap();
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_array_len_method() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "arr".to_string(),
            Type::Array(Box::new(Type::Int)),
        );

        let len_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("arr".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("len".into(), Span::dummy()),
                args: vec![],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&len_call, &env).unwrap();
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn test_array_map_method() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "arr".to_string(),
            Type::Array(Box::new(Type::Int)),
        );

        // arr.map(|x| true) -> [Bool]
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("arr".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("map".into(), Span::dummy()),
                args: vec![ast::Expr::new(
                    ast::ExprKind::Lambda {
                        params: vec![ast::Param {
                            mutable: false,
                            name: ast::Ident::new("x".into(), Span::dummy()),
                            ty: None,
                            default: None,
                            span: Span::dummy(),
                        }],
                        body: Box::new(ast::Expr::new(
                            ast::ExprKind::Bool(true),
                            Span::dummy(),
                        )),
                    },
                    Span::dummy(),
                )],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();
        assert_eq!(ty, Type::Array(Box::new(Type::Bool)));
    }

    #[test]
    fn test_array_filter_method() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "arr".to_string(),
            Type::Array(Box::new(Type::Int)),
        );

        // arr.filter(|x| true) -> [Int]
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("arr".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("filter".into(), Span::dummy()),
                args: vec![ast::Expr::new(
                    ast::ExprKind::Lambda {
                        params: vec![ast::Param {
                            mutable: false,
                            name: ast::Ident::new("x".into(), Span::dummy()),
                            ty: None,
                            default: None,
                            span: Span::dummy(),
                        }],
                        body: Box::new(ast::Expr::new(
                            ast::ExprKind::Bool(true),
                            Span::dummy(),
                        )),
                    },
                    Span::dummy(),
                )],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();
        assert_eq!(ty, Type::Array(Box::new(Type::Int)));
    }

    #[test]
    fn test_array_fold_method() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "arr".to_string(),
            Type::Array(Box::new(Type::Int)),
        );

        // arr.fold(0, |acc, x| acc) -> Int
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("arr".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("fold".into(), Span::dummy()),
                args: vec![
                    ast::Expr::new(
                        ast::ExprKind::Integer("0".into()),
                        Span::dummy(),
                    ),
                    ast::Expr::new(
                        ast::ExprKind::Lambda {
                            params: vec![
                                ast::Param {
                                    mutable: false,
                                    name: ast::Ident::new("acc".into(), Span::dummy()),
                                    ty: None,
                                    default: None,
                                    span: Span::dummy(),
                                },
                                ast::Param {
                                    mutable: false,
                                    name: ast::Ident::new("x".into(), Span::dummy()),
                                    ty: None,
                                    default: None,
                                    span: Span::dummy(),
                                },
                            ],
                            body: Box::new(ast::Expr::new(
                                ast::ExprKind::Ident("acc".into()),
                                Span::dummy(),
                            )),
                        },
                        Span::dummy(),
                    ),
                ],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn test_array_reduce_method() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "arr".to_string(),
            Type::Array(Box::new(Type::Int)),
        );

        // arr.reduce(|a, b| a) -> Int?
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("arr".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("reduce".into(), Span::dummy()),
                args: vec![ast::Expr::new(
                    ast::ExprKind::Lambda {
                        params: vec![
                            ast::Param {
                                mutable: false,
                                name: ast::Ident::new("a".into(), Span::dummy()),
                                ty: None,
                                default: None,
                                span: Span::dummy(),
                            },
                            ast::Param {
                                mutable: false,
                                name: ast::Ident::new("b".into(), Span::dummy()),
                                ty: None,
                                default: None,
                                span: Span::dummy(),
                            },
                        ],
                        body: Box::new(ast::Expr::new(
                            ast::ExprKind::Ident("a".into()),
                            Span::dummy(),
                        )),
                    },
                    Span::dummy(),
                )],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();
        assert_eq!(ty, Type::Optional(Box::new(Type::Int)));
    }

    #[test]
    fn test_array_find_method() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "arr".to_string(),
            Type::Array(Box::new(Type::Int)),
        );

        // arr.find(|x| true) -> Int?
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("arr".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("find".into(), Span::dummy()),
                args: vec![ast::Expr::new(
                    ast::ExprKind::Lambda {
                        params: vec![ast::Param {
                            mutable: false,
                            name: ast::Ident::new("x".into(), Span::dummy()),
                            ty: None,
                            default: None,
                            span: Span::dummy(),
                        }],
                        body: Box::new(ast::Expr::new(
                            ast::ExprKind::Bool(true),
                            Span::dummy(),
                        )),
                    },
                    Span::dummy(),
                )],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();
        assert_eq!(ty, Type::Optional(Box::new(Type::Int)));
    }

    #[test]
    fn test_array_any_method() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "arr".to_string(),
            Type::Array(Box::new(Type::Int)),
        );

        // arr.any(|x| true) -> Bool
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("arr".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("any".into(), Span::dummy()),
                args: vec![ast::Expr::new(
                    ast::ExprKind::Lambda {
                        params: vec![ast::Param {
                            mutable: false,
                            name: ast::Ident::new("x".into(), Span::dummy()),
                            ty: None,
                            default: None,
                            span: Span::dummy(),
                        }],
                        body: Box::new(ast::Expr::new(
                            ast::ExprKind::Bool(true),
                            Span::dummy(),
                        )),
                    },
                    Span::dummy(),
                )],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_array_all_method() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "arr".to_string(),
            Type::Array(Box::new(Type::Int)),
        );

        // arr.all(|x| true) -> Bool
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("arr".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("all".into(), Span::dummy()),
                args: vec![ast::Expr::new(
                    ast::ExprKind::Lambda {
                        params: vec![ast::Param {
                            mutable: false,
                            name: ast::Ident::new("x".into(), Span::dummy()),
                            ty: None,
                            default: None,
                            span: Span::dummy(),
                        }],
                        body: Box::new(ast::Expr::new(
                            ast::ExprKind::Bool(true),
                            Span::dummy(),
                        )),
                    },
                    Span::dummy(),
                )],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_string_methods() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var("s".to_string(), Type::String);

        // s.len() -> Int
        let len_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("s".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("len".into(), Span::dummy()),
                args: vec![],
            },
            Span::dummy(),
        );
        let ty = checker.infer_expr(&len_call, &env).unwrap();
        assert_eq!(ty, Type::Int);

        // s.to_uppercase() -> String
        let upper_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("s".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("to_uppercase".into(), Span::dummy()),
                args: vec![],
            },
            Span::dummy(),
        );
        let ty = checker.infer_expr(&upper_call, &env).unwrap();
        assert_eq!(ty, Type::String);

        // s.contains("sub") -> Bool
        let contains_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("s".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("contains".into(), Span::dummy()),
                args: vec![ast::Expr::new(
                    ast::ExprKind::String("sub".into()),
                    Span::dummy(),
                )],
            },
            Span::dummy(),
        );
        let ty = checker.infer_expr(&contains_call, &env).unwrap();
        assert_eq!(ty, Type::Bool);

        // s.split(",") -> [String]
        let split_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("s".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("split".into(), Span::dummy()),
                args: vec![ast::Expr::new(
                    ast::ExprKind::String(",".into()),
                    Span::dummy(),
                )],
            },
            Span::dummy(),
        );
        let ty = checker.infer_expr(&split_call, &env).unwrap();
        assert_eq!(ty, Type::Array(Box::new(Type::String)));
    }

    #[test]
    fn test_method_wrong_arity() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "result".to_string(),
            Type::Result(Box::new(Type::Int), Box::new(Type::String)),
        );

        // result.is_ok(42) - is_ok takes no arguments
        let bad_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("result".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("is_ok".into(), Span::dummy()),
                args: vec![ast::Expr::new(
                    ast::ExprKind::Integer("42".into()),
                    Span::dummy(),
                )],
            },
            Span::dummy(),
        );

        let result = checker.infer_expr(&bad_call, &env);
        assert!(result.is_err(), "is_ok(42) should fail - wrong arity");
    }

    // ========================================================================
    // Range Method Tests
    // ========================================================================

    #[test]
    fn test_range_contains_method() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "r".to_string(),
            Type::Named {
                name: "Range".to_string(),
                type_args: vec![Type::Int],
            },
        );

        // r.contains(5) -> Bool
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("r".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("contains".into(), Span::dummy()),
                args: vec![ast::Expr::new(
                    ast::ExprKind::Integer("5".into()),
                    Span::dummy(),
                )],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_range_contains_wrong_type() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "r".to_string(),
            Type::Named {
                name: "Range".to_string(),
                type_args: vec![Type::Int],
            },
        );

        // r.contains("hello") should fail - wrong argument type
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("r".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("contains".into(), Span::dummy()),
                args: vec![ast::Expr::new(
                    ast::ExprKind::String("hello".into()),
                    Span::dummy(),
                )],
            },
            Span::dummy(),
        );

        let result = checker.infer_expr(&method_call, &env);
        assert!(result.is_err(), "contains with wrong type should fail");
    }

    #[test]
    fn test_range_start_method() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "r".to_string(),
            Type::Named {
                name: "Range".to_string(),
                type_args: vec![Type::Int],
            },
        );

        // r.start() -> Int
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("r".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("start".into(), Span::dummy()),
                args: vec![],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn test_range_end_method() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "r".to_string(),
            Type::Named {
                name: "Range".to_string(),
                type_args: vec![Type::Int],
            },
        );

        // r.end() -> Int
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("r".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("end".into(), Span::dummy()),
                args: vec![],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn test_range_is_empty_method() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "r".to_string(),
            Type::Named {
                name: "Range".to_string(),
                type_args: vec![Type::Int],
            },
        );

        // r.is_empty() -> Bool
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("r".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("is_empty".into(), Span::dummy()),
                args: vec![],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_range_len_method() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "r".to_string(),
            Type::Named {
                name: "Range".to_string(),
                type_args: vec![Type::Int],
            },
        );

        // r.len() -> Int
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("r".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("len".into(), Span::dummy()),
                args: vec![],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn test_range_step_by_method() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "r".to_string(),
            Type::Named {
                name: "Range".to_string(),
                type_args: vec![Type::Int],
            },
        );

        // r.step_by(2) -> Range<Int>
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("r".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("step_by".into(), Span::dummy()),
                args: vec![ast::Expr::new(
                    ast::ExprKind::Integer("2".into()),
                    Span::dummy(),
                )],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();
        assert_eq!(ty, Type::Named {
            name: "Range".to_string(),
            type_args: vec![Type::Int],
        });
    }

    #[test]
    fn test_range_step_by_wrong_type() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "r".to_string(),
            Type::Named {
                name: "Range".to_string(),
                type_args: vec![Type::Int],
            },
        );

        // r.step_by("hello") should fail - step must be Int
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("r".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("step_by".into(), Span::dummy()),
                args: vec![ast::Expr::new(
                    ast::ExprKind::String("hello".into()),
                    Span::dummy(),
                )],
            },
            Span::dummy(),
        );

        let result = checker.infer_expr(&method_call, &env);
        assert!(result.is_err(), "step_by with non-Int should fail");
    }

    #[test]
    fn test_range_rev_method() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "r".to_string(),
            Type::Named {
                name: "Range".to_string(),
                type_args: vec![Type::Int],
            },
        );

        // r.rev() -> Range<Int>
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("r".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("rev".into(), Span::dummy()),
                args: vec![],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();
        assert_eq!(ty, Type::Named {
            name: "Range".to_string(),
            type_args: vec![Type::Int],
        });
    }

    #[test]
    fn test_range_method_wrong_arity() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "r".to_string(),
            Type::Named {
                name: "Range".to_string(),
                type_args: vec![Type::Int],
            },
        );

        // r.start(42) should fail - start takes no arguments
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("r".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("start".into(), Span::dummy()),
                args: vec![ast::Expr::new(
                    ast::ExprKind::Integer("42".into()),
                    Span::dummy(),
                )],
            },
            Span::dummy(),
        );

        let result = checker.infer_expr(&method_call, &env);
        assert!(result.is_err(), "start(42) should fail - wrong arity");
    }

    #[test]
    fn test_range_in_for_loop() {
        let mut checker = TypeChecker::new();
        let env = TypeEnv::new();

        // for i in 1..10 { ... }
        // The range expression should type-check and i should be Int
        let range_expr = ast::Expr::new(
            ast::ExprKind::Range {
                start: Some(Box::new(ast::Expr::new(
                    ast::ExprKind::Integer("1".into()),
                    Span::dummy(),
                ))),
                end: Some(Box::new(ast::Expr::new(
                    ast::ExprKind::Integer("10".into()),
                    Span::dummy(),
                ))),
                inclusive: false,
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&range_expr, &env).unwrap();
        assert_eq!(ty, Type::Named {
            name: "Range".to_string(),
            type_args: vec![Type::Int],
        });
    }

    #[test]
    fn test_range_float_element_type() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        // A Range<Float> should have Float methods return Float
        env.define_var(
            "r".to_string(),
            Type::Named {
                name: "Range".to_string(),
                type_args: vec![Type::Float],
            },
        );

        // r.start() -> Float
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("r".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("start".into(), Span::dummy()),
                args: vec![],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();
        assert_eq!(ty, Type::Float);
    }

    // ========================================================================
    // Map Method Tests
    // ========================================================================

    #[test]
    fn test_map_get_method() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "map".to_string(),
            Type::Map(Box::new(Type::String), Box::new(Type::Int)),
        );

        // map.get("key") -> Int?
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("map".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("get".into(), Span::dummy()),
                args: vec![ast::Expr::new(
                    ast::ExprKind::String("key".into()),
                    Span::dummy(),
                )],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();
        assert_eq!(ty, Type::Optional(Box::new(Type::Int)));
    }

    #[test]
    fn test_map_contains_key_method() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "map".to_string(),
            Type::Map(Box::new(Type::String), Box::new(Type::Int)),
        );

        // map.contains_key("key") -> Bool
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("map".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("contains_key".into(), Span::dummy()),
                args: vec![ast::Expr::new(
                    ast::ExprKind::String("key".into()),
                    Span::dummy(),
                )],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_map_keys_method() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "map".to_string(),
            Type::Map(Box::new(Type::String), Box::new(Type::Int)),
        );

        // map.keys() -> [String]
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("map".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("keys".into(), Span::dummy()),
                args: vec![],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();
        assert_eq!(ty, Type::Array(Box::new(Type::String)));
    }

    #[test]
    fn test_map_values_method() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "map".to_string(),
            Type::Map(Box::new(Type::String), Box::new(Type::Int)),
        );

        // map.values() -> [Int]
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("map".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("values".into(), Span::dummy()),
                args: vec![],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();
        assert_eq!(ty, Type::Array(Box::new(Type::Int)));
    }

    #[test]
    fn test_map_entries_method() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "map".to_string(),
            Type::Map(Box::new(Type::String), Box::new(Type::Int)),
        );

        // map.entries() -> [(String, Int)]
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("map".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("entries".into(), Span::dummy()),
                args: vec![],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();
        let expected_entry = Type::Tuple(vec![Type::String, Type::Int]);
        assert_eq!(ty, Type::Array(Box::new(expected_entry)));
    }

    #[test]
    fn test_map_insert_method() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "map".to_string(),
            Type::Map(Box::new(Type::String), Box::new(Type::Int)),
        );

        // map.insert("key", 42) -> Int?
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("map".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("insert".into(), Span::dummy()),
                args: vec![
                    ast::Expr::new(
                        ast::ExprKind::String("key".into()),
                        Span::dummy(),
                    ),
                    ast::Expr::new(
                        ast::ExprKind::Integer("42".into()),
                        Span::dummy(),
                    ),
                ],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();
        assert_eq!(ty, Type::Optional(Box::new(Type::Int)));
    }

    #[test]
    fn test_map_remove_method() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "map".to_string(),
            Type::Map(Box::new(Type::String), Box::new(Type::Int)),
        );

        // map.remove("key") -> Int?
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("map".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("remove".into(), Span::dummy()),
                args: vec![ast::Expr::new(
                    ast::ExprKind::String("key".into()),
                    Span::dummy(),
                )],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();
        assert_eq!(ty, Type::Optional(Box::new(Type::Int)));
    }

    #[test]
    fn test_map_len_method() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "map".to_string(),
            Type::Map(Box::new(Type::String), Box::new(Type::Int)),
        );

        // map.len() -> Int
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("map".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("len".into(), Span::dummy()),
                args: vec![],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn test_map_is_empty_method() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "map".to_string(),
            Type::Map(Box::new(Type::String), Box::new(Type::Int)),
        );

        // map.is_empty() -> Bool
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("map".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("is_empty".into(), Span::dummy()),
                args: vec![],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_map_get_wrong_key_type() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "map".to_string(),
            Type::Map(Box::new(Type::String), Box::new(Type::Int)),
        );

        // map.get(42) - wrong key type (Int instead of String)
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("map".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("get".into(), Span::dummy()),
                args: vec![ast::Expr::new(
                    ast::ExprKind::Integer("42".into()),
                    Span::dummy(),
                )],
            },
            Span::dummy(),
        );

        let result = checker.infer_expr(&method_call, &env);
        assert!(result.is_err(), "map.get(Int) should fail when key type is String");
    }

    #[test]
    fn test_map_insert_wrong_value_type() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "map".to_string(),
            Type::Map(Box::new(Type::String), Box::new(Type::Int)),
        );

        // map.insert("key", "wrong") - wrong value type (String instead of Int)
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("map".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("insert".into(), Span::dummy()),
                args: vec![
                    ast::Expr::new(
                        ast::ExprKind::String("key".into()),
                        Span::dummy(),
                    ),
                    ast::Expr::new(
                        ast::ExprKind::String("wrong".into()),
                        Span::dummy(),
                    ),
                ],
            },
            Span::dummy(),
        );

        let result = checker.infer_expr(&method_call, &env);
        assert!(result.is_err(), "map.insert(String, String) should fail when value type is Int");
    }

    #[test]
    fn test_map_method_wrong_arity() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "map".to_string(),
            Type::Map(Box::new(Type::String), Box::new(Type::Int)),
        );

        // map.len(42) - len takes no arguments
        let bad_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("map".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("len".into(), Span::dummy()),
                args: vec![ast::Expr::new(
                    ast::ExprKind::Integer("42".into()),
                    Span::dummy(),
                )],
            },
            Span::dummy(),
        );

        let result = checker.infer_expr(&bad_call, &env);
        assert!(result.is_err(), "map.len(42) should fail - wrong arity");
    }

    // =========================================================================
    // Tuple Method Tests
    // =========================================================================

    #[test]
    fn test_tuple_first_method() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        // Define a tuple (Int, String, Bool)
        env.define_var(
            "tup".to_string(),
            Type::Tuple(vec![Type::Int, Type::String, Type::Bool]),
        );

        // tup.first() -> Int
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("tup".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("first".into(), Span::dummy()),
                args: vec![],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn test_tuple_last_method() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        // Define a tuple (Int, String, Bool)
        env.define_var(
            "tup".to_string(),
            Type::Tuple(vec![Type::Int, Type::String, Type::Bool]),
        );

        // tup.last() -> Bool
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("tup".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("last".into(), Span::dummy()),
                args: vec![],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_tuple_len_method() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        // Define a tuple (Int, String, Bool)
        env.define_var(
            "tup".to_string(),
            Type::Tuple(vec![Type::Int, Type::String, Type::Bool]),
        );

        // tup.len() -> Int
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("tup".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("len".into(), Span::dummy()),
                args: vec![],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn test_tuple_first_empty_tuple_error() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        // Define an empty tuple ()
        env.define_var("tup".to_string(), Type::Tuple(vec![]));

        // tup.first() should fail on empty tuple
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("tup".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("first".into(), Span::dummy()),
                args: vec![],
            },
            Span::dummy(),
        );

        let result = checker.infer_expr(&method_call, &env);
        assert!(result.is_err(), "first() on empty tuple should fail");
    }

    #[test]
    fn test_tuple_last_empty_tuple_error() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        // Define an empty tuple ()
        env.define_var("tup".to_string(), Type::Tuple(vec![]));

        // tup.last() should fail on empty tuple
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("tup".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("last".into(), Span::dummy()),
                args: vec![],
            },
            Span::dummy(),
        );

        let result = checker.infer_expr(&method_call, &env);
        assert!(result.is_err(), "last() on empty tuple should fail");
    }

    #[test]
    fn test_tuple_method_wrong_arity() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        env.define_var(
            "tup".to_string(),
            Type::Tuple(vec![Type::Int, Type::String]),
        );

        // tup.first(42) - first takes no arguments
        let bad_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("tup".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("first".into(), Span::dummy()),
                args: vec![ast::Expr::new(
                    ast::ExprKind::Integer("42".into()),
                    Span::dummy(),
                )],
            },
            Span::dummy(),
        );

        let result = checker.infer_expr(&bad_call, &env);
        assert!(result.is_err(), "tup.first(42) should fail - wrong arity");
    }

    #[test]
    fn test_tuple_numeric_indexing() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        // Define a tuple (Int, String, Bool)
        env.define_var(
            "tup".to_string(),
            Type::Tuple(vec![Type::Int, Type::String, Type::Bool]),
        );

        // tup.0 -> Int
        let field_access_0 = ast::Expr::new(
            ast::ExprKind::Field {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("tup".into()),
                    Span::dummy(),
                )),
                field: ast::Ident::new("0".into(), Span::dummy()),
            },
            Span::dummy(),
        );
        let ty0 = checker.infer_expr(&field_access_0, &env).unwrap();
        assert_eq!(ty0, Type::Int);

        // tup.1 -> String
        let field_access_1 = ast::Expr::new(
            ast::ExprKind::Field {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("tup".into()),
                    Span::dummy(),
                )),
                field: ast::Ident::new("1".into(), Span::dummy()),
            },
            Span::dummy(),
        );
        let ty1 = checker.infer_expr(&field_access_1, &env).unwrap();
        assert_eq!(ty1, Type::String);

        // tup.2 -> Bool
        let field_access_2 = ast::Expr::new(
            ast::ExprKind::Field {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("tup".into()),
                    Span::dummy(),
                )),
                field: ast::Ident::new("2".into(), Span::dummy()),
            },
            Span::dummy(),
        );
        let ty2 = checker.infer_expr(&field_access_2, &env).unwrap();
        assert_eq!(ty2, Type::Bool);
    }

    #[test]
    fn test_tuple_index_out_of_bounds() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        // Define a tuple (Int, String)
        env.define_var(
            "tup".to_string(),
            Type::Tuple(vec![Type::Int, Type::String]),
        );

        // tup.5 - index out of bounds (tuple only has 2 elements)
        let field_access = ast::Expr::new(
            ast::ExprKind::Field {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("tup".into()),
                    Span::dummy(),
                )),
                field: ast::Ident::new("5".into(), Span::dummy()),
            },
            Span::dummy(),
        );

        let result = checker.infer_expr(&field_access, &env);
        assert!(result.is_err(), "tup.5 should fail - index out of bounds");
    }

    #[test]
    fn test_tuple_non_numeric_field_error() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        // Define a tuple (Int, String)
        env.define_var(
            "tup".to_string(),
            Type::Tuple(vec![Type::Int, Type::String]),
        );

        // tup.name - tuples don't have named fields
        let field_access = ast::Expr::new(
            ast::ExprKind::Field {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("tup".into()),
                    Span::dummy(),
                )),
                field: ast::Ident::new("name".into(), Span::dummy()),
            },
            Span::dummy(),
        );

        let result = checker.infer_expr(&field_access, &env);
        assert!(result.is_err(), "tup.name should fail - tuples don't have named fields");
    }

    #[test]
    fn test_tuple_to_array_method() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        // Define a tuple of same types (Int, Int, Int)
        env.define_var(
            "tup".to_string(),
            Type::Tuple(vec![Type::Int, Type::Int, Type::Int]),
        );

        // tup.to_array() -> [Int]
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("tup".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("to_array".into(), Span::dummy()),
                args: vec![],
            },
            Span::dummy(),
        );

        let ty = checker.infer_expr(&method_call, &env).unwrap();
        assert_eq!(ty, Type::Array(Box::new(Type::Int)));
    }

    #[test]
    fn test_tuple_to_array_heterogeneous_error() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        // Define a tuple with different types (Int, String)
        env.define_var(
            "tup".to_string(),
            Type::Tuple(vec![Type::Int, Type::String]),
        );

        // tup.to_array() should fail for heterogeneous tuple
        let method_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("tup".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("to_array".into(), Span::dummy()),
                args: vec![],
            },
            Span::dummy(),
        );

        let result = checker.infer_expr(&method_call, &env);
        assert!(result.is_err(), "to_array() on heterogeneous tuple should fail");
    }

    #[test]
    fn test_tuple_single_element() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        // Define a single-element tuple (String,)
        env.define_var("tup".to_string(), Type::Tuple(vec![Type::String]));

        // tup.first() -> String
        let first_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("tup".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("first".into(), Span::dummy()),
                args: vec![],
            },
            Span::dummy(),
        );
        let ty = checker.infer_expr(&first_call, &env).unwrap();
        assert_eq!(ty, Type::String);

        // tup.last() -> String (same as first for single element)
        let last_call = ast::Expr::new(
            ast::ExprKind::MethodCall {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("tup".into()),
                    Span::dummy(),
                )),
                method: ast::Ident::new("last".into(), Span::dummy()),
                args: vec![],
            },
            Span::dummy(),
        );
        let ty = checker.infer_expr(&last_call, &env).unwrap();
        assert_eq!(ty, Type::String);

        // tup.0 -> String
        let field_access = ast::Expr::new(
            ast::ExprKind::Field {
                object: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("tup".into()),
                    Span::dummy(),
                )),
                field: ast::Ident::new("0".into(), Span::dummy()),
            },
            Span::dummy(),
        );
        let ty = checker.infer_expr(&field_access, &env).unwrap();
        assert_eq!(ty, Type::String);
    }

    // ========================================================================
    // Generic Type Parameter and Bounds Checking Tests
    // ========================================================================

    #[test]
    fn test_type_param_def_creation() {
        // Test creating type parameter definitions without bounds
        let param = TypeParamDef::new("T".to_string());
        assert_eq!(param.name, "T");
        assert!(param.bounds.is_empty());

        // Test creating type parameter definitions with bounds
        let bounds = vec![TypeBound::new("Display".to_string())];
        let bounded_param = TypeParamDef::with_bounds("T".to_string(), bounds);
        assert_eq!(bounded_param.name, "T");
        assert_eq!(bounded_param.bounds.len(), 1);
        assert_eq!(bounded_param.bounds[0].trait_name, "Display");
    }

    #[test]
    fn test_type_bound_display() {
        // Simple bound
        let bound = TypeBound::new("Clone".to_string());
        assert_eq!(format!("{}", bound), "Clone");

        // Bound with type arguments
        let bound_with_args = TypeBound::with_args(
            "Iterator".to_string(),
            vec![Type::Int],
        );
        assert_eq!(format!("{}", bound_with_args), "Iterator<Int>");
    }

    #[test]
    fn test_type_scheme_poly_bounded() {
        // Create a polymorphic type scheme with bounded parameters
        let type_param_defs = vec![
            TypeParamDef::with_bounds(
                "T".to_string(),
                vec![TypeBound::new("Clone".to_string())],
            ),
        ];

        let struct_type = Type::Named {
            name: "Container".to_string(),
            type_args: vec![Type::Named { name: "T".to_string(), type_args: vec![] }],
        };

        let scheme = TypeScheme::poly_bounded(type_param_defs, struct_type);

        assert_eq!(scheme.type_params, vec!["T".to_string()]);
        assert_eq!(scheme.type_param_defs.len(), 1);
        assert_eq!(scheme.type_param_defs[0].name, "T");
        assert_eq!(scheme.type_param_defs[0].bounds.len(), 1);
    }

    #[test]
    fn test_type_scheme_get_bounds() {
        let type_param_defs = vec![
            TypeParamDef::with_bounds(
                "T".to_string(),
                vec![TypeBound::new("Clone".to_string()), TypeBound::new("Debug".to_string())],
            ),
            TypeParamDef::new("U".to_string()),
        ];

        let scheme = TypeScheme::poly_bounded(type_param_defs, Type::Unit);

        // Get bounds for T
        let t_bounds = scheme.get_bounds("T").unwrap();
        assert_eq!(t_bounds.len(), 2);
        assert_eq!(t_bounds[0].trait_name, "Clone");
        assert_eq!(t_bounds[1].trait_name, "Debug");

        // Get bounds for U (no bounds)
        let u_bounds = scheme.get_bounds("U").unwrap();
        assert!(u_bounds.is_empty());

        // Get bounds for non-existent param
        assert!(scheme.get_bounds("V").is_none());
    }

    #[test]
    fn test_validate_type_args_arity_check() {
        let checker = TypeChecker::new();

        let type_param_defs = vec![
            TypeParamDef::new("T".to_string()),
            TypeParamDef::new("U".to_string()),
        ];

        // Wrong number of arguments
        let type_args = vec![Type::Int];
        let result = checker.validate_type_args(&type_param_defs, &type_args, Span::dummy());

        assert!(result.is_err());
        match result.unwrap_err() {
            TypeError::WrongTypeArity { expected, found, .. } => {
                assert_eq!(expected, 2);
                assert_eq!(found, 1);
            }
            other => panic!("Expected WrongTypeArity, got {:?}", other),
        }
    }

    #[test]
    fn test_validate_type_args_bound_satisfaction() {
        let checker = TypeChecker::new();

        // Type parameter T with Clone bound
        let type_param_defs = vec![
            TypeParamDef::with_bounds(
                "T".to_string(),
                vec![TypeBound::new("Clone".to_string())],
            ),
        ];

        // Int implements Clone (builtin)
        let result = checker.validate_type_args(
            &type_param_defs,
            &[Type::Int],
            Span::dummy(),
        );
        assert!(result.is_ok(), "Int should satisfy Clone bound");

        // String implements Clone (builtin)
        let result = checker.validate_type_args(
            &type_param_defs,
            &[Type::String],
            Span::dummy(),
        );
        assert!(result.is_ok(), "String should satisfy Clone bound");
    }

    #[test]
    fn test_validate_type_args_bound_not_satisfied() {
        let checker = TypeChecker::new();

        // Type parameter T with a non-implemented bound
        let type_param_defs = vec![
            TypeParamDef::with_bounds(
                "T".to_string(),
                vec![TypeBound::new("NonExistentTrait".to_string())],
            ),
        ];

        let result = checker.validate_type_args(
            &type_param_defs,
            &[Type::Int],
            Span::dummy(),
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            TypeError::BoundNotSatisfied { type_arg, param, bound, .. } => {
                assert_eq!(type_arg, "Int");
                assert_eq!(param, "T");
                assert_eq!(bound, "NonExistentTrait");
            }
            other => panic!("Expected BoundNotSatisfied, got {:?}", other),
        }
    }

    #[test]
    fn test_implements_trait_primitives() {
        let checker = TypeChecker::new();

        // Primitives implement common traits
        assert!(checker.implements_trait(&Type::Int, "Clone"));
        assert!(checker.implements_trait(&Type::Int, "Debug"));
        assert!(checker.implements_trait(&Type::Int, "Display"));
        assert!(checker.implements_trait(&Type::Int, "Eq"));
        assert!(checker.implements_trait(&Type::Int, "Hash"));

        assert!(checker.implements_trait(&Type::String, "Clone"));
        assert!(checker.implements_trait(&Type::String, "Debug"));
        assert!(checker.implements_trait(&Type::String, "Display"));

        assert!(checker.implements_trait(&Type::Bool, "Clone"));
        assert!(checker.implements_trait(&Type::Bool, "Eq"));
    }

    #[test]
    fn test_implements_trait_error_and_never() {
        let checker = TypeChecker::new();

        // Error type implements everything (for error recovery)
        assert!(checker.implements_trait(&Type::Error, "AnythingAtAll"));
        assert!(checker.implements_trait(&Type::Error, "NonExistent"));

        // Never type implements everything (bottom type)
        assert!(checker.implements_trait(&Type::Never, "AnythingAtAll"));

        // Any type implements everything
        assert!(checker.implements_trait(&Type::Any, "AnythingAtAll"));
    }

    #[test]
    fn test_implements_trait_compound_types() {
        let checker = TypeChecker::new();

        // Array<Int> implements Clone if Int does
        let array_int = Type::Array(Box::new(Type::Int));
        assert!(checker.implements_trait(&array_int, "Clone"));

        // Optional<String> implements Clone if String does
        let opt_string = Type::Optional(Box::new(Type::String));
        assert!(checker.implements_trait(&opt_string, "Clone"));

        // Tuple (Int, String) implements Clone if both do
        let tuple = Type::Tuple(vec![Type::Int, Type::String]);
        assert!(checker.implements_trait(&tuple, "Clone"));

        // Result<Int, String> implements Clone if both do
        let result = Type::Result(Box::new(Type::Int), Box::new(Type::String));
        assert!(checker.implements_trait(&result, "Clone"));
    }

    #[test]
    fn test_check_struct_with_generics() {
        use ast::{StructDecl, StructField, GenericParams, GenericParam, Visibility, Spanned};

        let mut checker = TypeChecker::new();

        // Define a generic struct: struct Container<T> { value: T }
        let struct_decl = StructDecl {
            attributes: vec![],
            visibility: Visibility::Public,
            name: Spanned::dummy("Container".into()),
            generic_params: Some(GenericParams {
                params: vec![GenericParam {
                    name: Spanned::dummy("T".into()),
                    bounds: vec![],
                    span: Span::dummy(),
                }],
                span: Span::dummy(),
            }),
            fields: vec![StructField {
                visibility: Visibility::Public,
                name: Spanned::dummy("value".into()),
                ty: ast::TypeExpr::Named(Spanned::dummy("T".into())),
                default: None,
                span: Span::dummy(),
            }],
            derive: vec![],
            span: Span::dummy(),
        };

        let result = checker.check_struct(&struct_decl);
        assert!(result.is_ok());

        // Check that type params were registered
        assert!(checker.generic_type_params.contains_key("Container"));
        let params = checker.generic_type_params.get("Container").unwrap();
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].name, "T");
    }

    #[test]
    fn test_check_struct_with_bounded_generics() {
        use ast::{StructDecl, StructField, GenericParams, GenericParam, TraitBound, Visibility, Spanned};

        let mut checker = TypeChecker::new();

        // Define a generic struct with bounds: struct Printable<T: Display> { value: T }
        let struct_decl = StructDecl {
            attributes: vec![],
            visibility: Visibility::Public,
            name: Spanned::dummy("Printable".into()),
            generic_params: Some(GenericParams {
                params: vec![GenericParam {
                    name: Spanned::dummy("T".into()),
                    bounds: vec![TraitBound {
                        path: vec![Spanned::dummy("Display".into())],
                        type_args: None,
                        span: Span::dummy(),
                    }],
                    span: Span::dummy(),
                }],
                span: Span::dummy(),
            }),
            fields: vec![StructField {
                visibility: Visibility::Public,
                name: Spanned::dummy("value".into()),
                ty: ast::TypeExpr::Named(Spanned::dummy("T".into())),
                default: None,
                span: Span::dummy(),
            }],
            derive: vec![],
            span: Span::dummy(),
        };

        let result = checker.check_struct(&struct_decl);
        assert!(result.is_ok());

        // Check that type params with bounds were registered
        assert!(checker.generic_type_params.contains_key("Printable"));
        let params = checker.generic_type_params.get("Printable").unwrap();
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].name, "T");
        assert_eq!(params[0].bounds.len(), 1);
        assert_eq!(params[0].bounds[0].trait_name, "Display");
    }

    #[test]
    fn test_check_enum_with_generics() {
        use ast::{EnumDecl, EnumVariant, EnumVariantData, GenericParams, GenericParam, Visibility, Spanned};

        let mut checker = TypeChecker::new();

        // Define a generic enum: enum Option<T> { Some(T), None }
        let enum_decl = EnumDecl {
            attributes: vec![],
            visibility: Visibility::Public,
            name: Spanned::dummy("MyOption".into()),
            generic_params: Some(GenericParams {
                params: vec![GenericParam {
                    name: Spanned::dummy("T".into()),
                    bounds: vec![],
                    span: Span::dummy(),
                }],
                span: Span::dummy(),
            }),
            variants: vec![
                EnumVariant {
                    name: Spanned::dummy("Some".into()),
                    data: EnumVariantData::Tuple(vec![
                        ast::TypeExpr::Named(Spanned::dummy("T".into())),
                    ]),
                    span: Span::dummy(),
                },
                EnumVariant {
                    name: Spanned::dummy("None".into()),
                    data: EnumVariantData::Unit,
                    span: Span::dummy(),
                },
            ],
            derive: vec![],
            span: Span::dummy(),
        };

        let result = checker.check_enum(&enum_decl);
        assert!(result.is_ok());

        // Check that type params were registered
        assert!(checker.generic_type_params.contains_key("MyOption"));
        let params = checker.generic_type_params.get("MyOption").unwrap();
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].name, "T");

        // Check that variant info was registered
        assert!(checker.enum_variants.contains_key("MyOption"));
        let variant_info = checker.enum_variants.get("MyOption").unwrap();
        assert_eq!(variant_info.variants.len(), 2);
        assert!(variant_info.variants.contains_key("Some"));
        assert!(variant_info.variants.contains_key("None"));
    }

    #[test]
    fn test_enum_unit_variant_type() {
        use ast::{EnumDecl, EnumVariant, EnumVariantData, Visibility, Spanned};

        let mut checker = TypeChecker::new();

        // Define enum Color { Red, Green, Blue }
        let enum_decl = EnumDecl {
            attributes: vec![],
            visibility: Visibility::Public,
            name: Spanned::dummy("Color".into()),
            generic_params: None,
            variants: vec![
                EnumVariant {
                    name: Spanned::dummy("Red".into()),
                    data: EnumVariantData::Unit,
                    span: Span::dummy(),
                },
                EnumVariant {
                    name: Spanned::dummy("Green".into()),
                    data: EnumVariantData::Unit,
                    span: Span::dummy(),
                },
                EnumVariant {
                    name: Spanned::dummy("Blue".into()),
                    data: EnumVariantData::Unit,
                    span: Span::dummy(),
                },
            ],
            derive: vec![],
            span: Span::dummy(),
        };

        let result = checker.check_enum(&enum_decl);
        assert!(result.is_ok());

        // Check that variant constructors are registered
        let red_ty = checker.env.lookup_var("Color::Red");
        assert!(red_ty.is_some(), "Color::Red should be defined");

        // Unit variants have the enum type directly
        match red_ty.unwrap() {
            Type::Named { name, .. } => assert_eq!(name, "Color"),
            _ => panic!("Expected Named type for unit variant"),
        }
    }

    #[test]
    fn test_enum_tuple_variant_type() {
        use ast::{EnumDecl, EnumVariant, EnumVariantData, GenericParams, GenericParam, Visibility, Spanned};

        let mut checker = TypeChecker::new();

        // Define enum Result<T, E> { Ok(T), Err(E) }
        let enum_decl = EnumDecl {
            attributes: vec![],
            visibility: Visibility::Public,
            name: Spanned::dummy("MyResult".into()),
            generic_params: Some(GenericParams {
                params: vec![
                    GenericParam {
                        name: Spanned::dummy("T".into()),
                        bounds: vec![],
                        span: Span::dummy(),
                    },
                    GenericParam {
                        name: Spanned::dummy("E".into()),
                        bounds: vec![],
                        span: Span::dummy(),
                    },
                ],
                span: Span::dummy(),
            }),
            variants: vec![
                EnumVariant {
                    name: Spanned::dummy("Ok".into()),
                    data: EnumVariantData::Tuple(vec![
                        ast::TypeExpr::Named(Spanned::dummy("T".into())),
                    ]),
                    span: Span::dummy(),
                },
                EnumVariant {
                    name: Spanned::dummy("Err".into()),
                    data: EnumVariantData::Tuple(vec![
                        ast::TypeExpr::Named(Spanned::dummy("E".into())),
                    ]),
                    span: Span::dummy(),
                },
            ],
            derive: vec![],
            span: Span::dummy(),
        };

        let result = checker.check_enum(&enum_decl);
        assert!(result.is_ok());

        // Check that variant constructors are registered as functions
        let ok_ty = checker.env.lookup_var("MyResult::Ok");
        assert!(ok_ty.is_some(), "MyResult::Ok should be defined");

        // Tuple variants are functions
        match ok_ty.unwrap() {
            Type::Function { params, return_type } => {
                assert_eq!(params.len(), 1);
                match return_type.as_ref() {
                    Type::Named { name, .. } => assert_eq!(name, "MyResult"),
                    _ => panic!("Expected Named return type"),
                }
            }
            _ => panic!("Expected Function type for tuple variant"),
        }
    }

    #[test]
    fn test_enum_struct_variant_type() {
        use ast::{EnumDecl, EnumVariant, EnumVariantData, DataField, Visibility, Spanned};

        let mut checker = TypeChecker::new();

        // Define enum Message { Move { x: Int, y: Int }, Quit }
        let enum_decl = EnumDecl {
            attributes: vec![],
            visibility: Visibility::Public,
            name: Spanned::dummy("Message".into()),
            generic_params: None,
            variants: vec![
                EnumVariant {
                    name: Spanned::dummy("Move".into()),
                    data: EnumVariantData::Struct(vec![
                        DataField {
                            name: Spanned::dummy("x".into()),
                            ty: ast::TypeExpr::Named(Spanned::dummy("Int".into())),
                            default: None,
                            span: Span::dummy(),
                        },
                        DataField {
                            name: Spanned::dummy("y".into()),
                            ty: ast::TypeExpr::Named(Spanned::dummy("Int".into())),
                            default: None,
                            span: Span::dummy(),
                        },
                    ]),
                    span: Span::dummy(),
                },
                EnumVariant {
                    name: Spanned::dummy("Quit".into()),
                    data: EnumVariantData::Unit,
                    span: Span::dummy(),
                },
            ],
            derive: vec![],
            span: Span::dummy(),
        };

        let result = checker.check_enum(&enum_decl);
        assert!(result.is_ok());

        // Check that variant info includes struct fields
        let variant_info = checker.enum_variants.get("Message").unwrap();
        let move_variant = variant_info.variants.get("Move").unwrap();

        match move_variant {
            VariantData::Struct(fields) => {
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].0, "x");
                assert_eq!(fields[1].0, "y");
            }
            _ => panic!("Expected Struct variant"),
        }
    }

    #[test]
    fn test_enum_variant_pattern_matching() {
        use ast::{EnumDecl, EnumVariant, EnumVariantData, GenericParams, GenericParam, Visibility, Spanned, Pattern, PatternKind};

        let mut checker = TypeChecker::new();

        // Define enum Option<T> { Some(T), None }
        let enum_decl = EnumDecl {
            attributes: vec![],
            visibility: Visibility::Public,
            name: Spanned::dummy("MyOption".into()),
            generic_params: Some(GenericParams {
                params: vec![GenericParam {
                    name: Spanned::dummy("T".into()),
                    bounds: vec![],
                    span: Span::dummy(),
                }],
                span: Span::dummy(),
            }),
            variants: vec![
                EnumVariant {
                    name: Spanned::dummy("Some".into()),
                    data: EnumVariantData::Tuple(vec![
                        ast::TypeExpr::Named(Spanned::dummy("T".into())),
                    ]),
                    span: Span::dummy(),
                },
                EnumVariant {
                    name: Spanned::dummy("None".into()),
                    data: EnumVariantData::Unit,
                    span: Span::dummy(),
                },
            ],
            derive: vec![],
            span: Span::dummy(),
        };

        checker.check_enum(&enum_decl).unwrap();

        // Create a pattern for Some(x) matching against MyOption<Int>
        let scrutinee_type = Type::Named {
            name: "MyOption".to_string(),
            type_args: vec![Type::Int],
        };

        let pattern = Pattern {
            kind: PatternKind::Variant {
                path: vec![],
                variant: Spanned::dummy("Some".into()),
                fields: Some(vec![
                    Pattern {
                        kind: PatternKind::Ident("x".into()),
                        span: Span::dummy(),
                    },
                ]),
            },
            span: Span::dummy(),
        };

        let mut env = TypeEnv::with_parent(Rc::clone(&checker.env));
        let result = checker.bind_pattern(&pattern, &scrutinee_type, &mut env);
        assert!(result.is_ok());

        // Check that x is bound to Int
        let x_type = env.lookup_var("x");
        assert!(x_type.is_some());
        assert_eq!(*x_type.unwrap(), Type::Int);
    }

    #[test]
    fn test_resolve_generic_type_validates_bounds() {
        use ast::{StructDecl, GenericParams, GenericParam, TraitBound, Visibility, Spanned};

        let mut checker = TypeChecker::new();

        // First register a struct with a bounded generic parameter
        let struct_decl = StructDecl {
            attributes: vec![],
            visibility: Visibility::Public,
            name: Spanned::dummy("BoundedContainer".into()),
            generic_params: Some(GenericParams {
                params: vec![GenericParam {
                    name: Spanned::dummy("T".into()),
                    bounds: vec![TraitBound {
                        path: vec![Spanned::dummy("Clone".into())],
                        type_args: None,
                        span: Span::dummy(),
                    }],
                    span: Span::dummy(),
                }],
                span: Span::dummy(),
            }),
            fields: vec![],
            derive: vec![],
            span: Span::dummy(),
        };

        checker.check_struct(&struct_decl).unwrap();

        // Now resolve BoundedContainer<Int> - should succeed (Int implements Clone)
        let type_expr = ast::TypeExpr::Generic {
            name: Spanned::dummy("BoundedContainer".into()),
            args: vec![ast::TypeExpr::Named(Spanned::dummy("Int".into()))],
            span: Span::dummy(),
        };

        let result = checker.resolve_type(&type_expr);
        assert!(result.is_ok(), "BoundedContainer<Int> should be valid");
    }

    #[test]
    fn test_resolve_generic_type_rejects_unsatisfied_bounds() {
        use ast::{StructDecl, GenericParams, GenericParam, TraitBound, Visibility, Spanned};

        let mut checker = TypeChecker::new();

        // Register a struct with a custom bound that won't be satisfied
        let struct_decl = StructDecl {
            attributes: vec![],
            visibility: Visibility::Public,
            name: Spanned::dummy("RequiresSpecialTrait".into()),
            generic_params: Some(GenericParams {
                params: vec![GenericParam {
                    name: Spanned::dummy("T".into()),
                    bounds: vec![TraitBound {
                        path: vec![Spanned::dummy("SpecialTrait".into())],
                        type_args: None,
                        span: Span::dummy(),
                    }],
                    span: Span::dummy(),
                }],
                span: Span::dummy(),
            }),
            fields: vec![],
            derive: vec![],
            span: Span::dummy(),
        };

        checker.check_struct(&struct_decl).unwrap();

        // Now try to resolve RequiresSpecialTrait<Int> - should fail
        let type_expr = ast::TypeExpr::Generic {
            name: Spanned::dummy("RequiresSpecialTrait".into()),
            args: vec![ast::TypeExpr::Named(Spanned::dummy("Int".into()))],
            span: Span::dummy(),
        };

        let result = checker.resolve_type(&type_expr);
        assert!(result.is_err(), "RequiresSpecialTrait<Int> should fail - Int doesn't implement SpecialTrait");

        match result.unwrap_err() {
            TypeError::BoundNotSatisfied { type_arg, param, bound, .. } => {
                assert_eq!(type_arg, "Int");
                assert_eq!(param, "T");
                assert_eq!(bound, "SpecialTrait");
            }
            other => panic!("Expected BoundNotSatisfied, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_trait_bounds() {
        use ast::{TraitBound, Spanned};

        let checker = TypeChecker::new();

        // Simple bound without type args
        let ast_bounds = vec![TraitBound {
            path: vec![Spanned::dummy("Clone".into())],
            type_args: None,
            span: Span::dummy(),
        }];

        let bounds = checker.resolve_trait_bounds(&ast_bounds).unwrap();
        assert_eq!(bounds.len(), 1);
        assert_eq!(bounds[0].trait_name, "Clone");
        assert!(bounds[0].type_args.is_empty());

        // Bound with type args
        let ast_bounds_with_args = vec![TraitBound {
            path: vec![Spanned::dummy("Iterator".into())],
            type_args: Some(vec![ast::TypeExpr::Named(Spanned::dummy("Int".into()))]),
            span: Span::dummy(),
        }];

        let bounds = checker.resolve_trait_bounds(&ast_bounds_with_args).unwrap();
        assert_eq!(bounds.len(), 1);
        assert_eq!(bounds[0].trait_name, "Iterator");
        assert_eq!(bounds[0].type_args.len(), 1);
        assert_eq!(bounds[0].type_args[0], Type::Int);
    }

    #[test]
    fn test_resolve_generic_params() {
        use ast::{GenericParams, GenericParam, TraitBound, Spanned};

        let checker = TypeChecker::new();

        let ast_params = GenericParams {
            params: vec![
                GenericParam {
                    name: Spanned::dummy("T".into()),
                    bounds: vec![
                        TraitBound {
                            path: vec![Spanned::dummy("Clone".into())],
                            type_args: None,
                            span: Span::dummy(),
                        },
                        TraitBound {
                            path: vec![Spanned::dummy("Debug".into())],
                            type_args: None,
                            span: Span::dummy(),
                        },
                    ],
                    span: Span::dummy(),
                },
                GenericParam {
                    name: Spanned::dummy("U".into()),
                    bounds: vec![],
                    span: Span::dummy(),
                },
            ],
            span: Span::dummy(),
        };

        let params = checker.resolve_generic_params(&ast_params).unwrap();

        assert_eq!(params.len(), 2);

        // First param with bounds
        assert_eq!(params[0].name, "T");
        assert_eq!(params[0].bounds.len(), 2);
        assert_eq!(params[0].bounds[0].trait_name, "Clone");
        assert_eq!(params[0].bounds[1].trait_name, "Debug");

        // Second param without bounds
        assert_eq!(params[1].name, "U");
        assert!(params[1].bounds.is_empty());
    }

    #[test]
    fn test_multiple_type_params_with_bounds() {
        let checker = TypeChecker::new();

        // Map<K: Eq + Hash, V: Clone>
        let type_param_defs = vec![
            TypeParamDef::with_bounds(
                "K".to_string(),
                vec![
                    TypeBound::new("Eq".to_string()),
                    TypeBound::new("Hash".to_string()),
                ],
            ),
            TypeParamDef::with_bounds(
                "V".to_string(),
                vec![TypeBound::new("Clone".to_string())],
            ),
        ];

        // Int satisfies Eq+Hash, String satisfies Clone
        let result = checker.validate_type_args(
            &type_param_defs,
            &[Type::Int, Type::String],
            Span::dummy(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_satisfies_bound_checks_supertraits() {
        let mut checker = TypeChecker::new();

        // Define a trait with a supertrait: trait Ordered: Eq { }
        let mut ordered_trait = TraitDef::new("Ordered".to_string());
        ordered_trait.supertraits = vec![TypeBound::new("Eq".to_string())];
        checker.register_trait(ordered_trait);

        // Register that Int implements Ordered
        let impl_ = TraitImpl {
            trait_name: "Ordered".to_string(),
            trait_args: vec![],
            for_type: Type::Int,
            where_clause: vec![],
            methods: FxHashMap::default(),
            associated_types: FxHashMap::default(),
            associated_consts: FxHashMap::default(),
        };
        checker.register_trait_impl(impl_).unwrap();

        // Int satisfies Ordered bound (and transitively Eq)
        let bound = TypeBound::new("Ordered".to_string());
        let result = checker.satisfies_bound(&Type::Int, &bound, Span::dummy());
        assert!(result.is_ok());
    }

    // =========================================================================
    // Enhanced Capture Mode Analysis Tests
    // =========================================================================

    #[test]
    fn test_capture_mode_borrow_for_read_only() {
        let checker = TypeChecker::new();
        let mut env = CaptureEnv::new();
        env.define_var("x".to_string(), Type::Int, false);
        env.define_var("y".to_string(), Type::String, false);

        // Lambda: |a| a + x (x is only read)
        let lambda_params = vec![
            ast::Param {
                mutable: false,
                name: ast::Spanned::dummy("a".into()),
                ty: None,
                default: None,
                span: Span::dummy(),
            },
        ];

        let lambda_body = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::Add,
                left: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("a".into()),
                    Span::dummy(),
                )),
                right: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("x".into()),
                    Span::dummy(),
                )),
            },
            Span::dummy(),
        );

        let captures = checker.analyze_lambda_captures(&lambda_params, &lambda_body, &env);

        assert_eq!(captures.len(), 1);
        assert_eq!(captures[0].name, "x");
        assert_eq!(captures[0].mode, CaptureMode::Borrow);
        assert!(!captures[0].is_mutable);
    }

    #[test]
    fn test_capture_mode_move_for_non_copy_in_call() {
        let checker = TypeChecker::new();
        let mut env = CaptureEnv::new();
        // String is not Copy, so passing it to a function causes a move
        env.define_var("msg".to_string(), Type::String, false);

        // Lambda: || send(msg)  - msg is passed to a function, so it's moved
        let lambda_params: Vec<ast::Param> = vec![];

        let lambda_body = ast::Expr::new(
            ast::ExprKind::Call {
                func: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("send".into()),
                    Span::dummy(),
                )),
                args: vec![ast::CallArg {
                    name: None,
                    value: ast::Expr::new(
                        ast::ExprKind::Ident("msg".into()),
                        Span::dummy(),
                    ),
                    spread: false,
                }],
            },
            Span::dummy(),
        );

        let captures = checker.analyze_lambda_captures(&lambda_params, &lambda_body, &env);

        assert_eq!(captures.len(), 1);
        assert_eq!(captures[0].name, "msg");
        assert_eq!(captures[0].mode, CaptureMode::Move);
    }

    #[test]
    fn test_capture_mode_borrow_for_copy_in_call() {
        let checker = TypeChecker::new();
        let mut env = CaptureEnv::new();
        // Int is Copy, so passing it to a function just copies it
        env.define_var("count".to_string(), Type::Int, false);

        // Lambda: || process(count)  - count is Copy, so just borrowed
        let lambda_params: Vec<ast::Param> = vec![];

        let lambda_body = ast::Expr::new(
            ast::ExprKind::Call {
                func: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("process".into()),
                    Span::dummy(),
                )),
                args: vec![ast::CallArg {
                    name: None,
                    value: ast::Expr::new(
                        ast::ExprKind::Ident("count".into()),
                        Span::dummy(),
                    ),
                    spread: false,
                }],
            },
            Span::dummy(),
        );

        let captures = checker.analyze_lambda_captures(&lambda_params, &lambda_body, &env);

        assert_eq!(captures.len(), 1);
        assert_eq!(captures[0].name, "count");
        // Copy types don't need to be moved
        assert_eq!(captures[0].mode, CaptureMode::Borrow);
    }

    #[test]
    fn test_validate_captures_mutable_capture_of_immutable_fails() {
        let checker = TypeChecker::new();

        // Create a capture that tries to mutably borrow an immutable variable
        let captures = vec![
            CaptureInfo::new("x".to_string(), Type::Int, CaptureMode::BorrowMut, false),
        ];

        let result = checker.validate_captures(&captures, false, Span::dummy());
        assert!(result.is_err());

        match result.unwrap_err() {
            TypeError::MutableCaptureOfImmutable { var_name, .. } => {
                assert_eq!(var_name, "x");
            }
            e => panic!("Expected MutableCaptureOfImmutable error, got {:?}", e),
        }
    }

    #[test]
    fn test_validate_captures_mutable_capture_of_mutable_succeeds() {
        let checker = TypeChecker::new();

        // Mutably borrowing a mutable variable is OK for non-spawn contexts
        let captures = vec![
            CaptureInfo::new("x".to_string(), Type::Int, CaptureMode::BorrowMut, true),
        ];

        let result = checker.validate_captures(&captures, false, Span::dummy());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_spawn_rejects_mutable_capture() {
        let checker = TypeChecker::new();

        // Spawn cannot have mutable captures
        let captures = vec![
            CaptureInfo::new("counter".to_string(), Type::Int, CaptureMode::BorrowMut, true),
        ];

        let result = checker.validate_captures(&captures, true, Span::dummy());
        assert!(result.is_err());

        match result.unwrap_err() {
            TypeError::MutableCaptureInSpawn { var_name, .. } => {
                assert_eq!(var_name, "counter");
            }
            e => panic!("Expected MutableCaptureInSpawn error, got {:?}", e),
        }
    }

    #[test]
    fn test_validate_spawn_rejects_non_transfer_move() {
        let checker = TypeChecker::new();

        // Create a non-Transfer type (mutable reference)
        let non_transfer = Type::Reference {
            mutable: true,
            inner: Box::new(Type::Int),
        };

        let captures = vec![
            CaptureInfo::new("ptr".to_string(), non_transfer, CaptureMode::Move, false),
        ];

        let result = checker.validate_captures(&captures, true, Span::dummy());
        assert!(result.is_err());

        match result.unwrap_err() {
            TypeError::NonTransferCapture { var_name, .. } => {
                assert_eq!(var_name, "ptr");
            }
            e => panic!("Expected NonTransferCapture error, got {:?}", e),
        }
    }

    #[test]
    fn test_validate_spawn_accepts_transfer_types() {
        let checker = TypeChecker::new();

        // Int, String, Array[Int] are all Transfer
        let captures = vec![
            CaptureInfo::new("x".to_string(), Type::Int, CaptureMode::Borrow, false),
            CaptureInfo::new("msg".to_string(), Type::String, CaptureMode::Move, false),
            CaptureInfo::new("data".to_string(), Type::Array(Box::new(Type::Int)), CaptureMode::Move, false),
        ];

        let result = checker.validate_captures(&captures, true, Span::dummy());
        assert!(result.is_ok());
    }

    #[test]
    fn test_capture_env_tracks_mutability() {
        let mut env = CaptureEnv::new();
        env.define_var("immutable_var".to_string(), Type::Int, false);
        env.define_var("mutable_var".to_string(), Type::Int, true);

        assert!(!env.is_mutable("immutable_var"));
        assert!(env.is_mutable("mutable_var"));
        assert!(!env.is_mutable("unknown_var")); // Unknown returns false
    }

    #[test]
    fn test_capture_info_methods() {
        let borrow_capture = CaptureInfo::new("a".to_string(), Type::Int, CaptureMode::Borrow, false);
        assert!(!borrow_capture.is_mut_capture());
        assert!(!borrow_capture.is_move());
        assert!(!borrow_capture.requires_ownership());

        let borrow_mut_capture = CaptureInfo::new("b".to_string(), Type::Int, CaptureMode::BorrowMut, true);
        assert!(borrow_mut_capture.is_mut_capture());
        assert!(!borrow_mut_capture.is_move());
        assert!(borrow_mut_capture.requires_ownership());

        let move_capture = CaptureInfo::new("c".to_string(), Type::String, CaptureMode::Move, false);
        assert!(!move_capture.is_mut_capture());
        assert!(move_capture.is_move());
        assert!(move_capture.requires_ownership());
    }

    #[test]
    fn test_capture_info_spawn_safe() {
        // Transfer types are spawn safe
        let int_capture = CaptureInfo::new("x".to_string(), Type::Int, CaptureMode::Borrow, false);
        assert!(int_capture.is_spawn_safe());

        let string_capture = CaptureInfo::new("s".to_string(), Type::String, CaptureMode::Move, false);
        assert!(string_capture.is_spawn_safe());

        // Mutable references are not spawn safe
        let mut_ref_capture = CaptureInfo::new(
            "ptr".to_string(),
            Type::Reference { mutable: true, inner: Box::new(Type::Int) },
            CaptureMode::Borrow,
            false
        );
        assert!(!mut_ref_capture.is_spawn_safe());
    }

    #[test]
    fn test_analyze_and_validate_closure_captures() {
        let checker = TypeChecker::new();
        let mut env = CaptureEnv::new();
        env.define_var("x".to_string(), Type::Int, false);

        // Simple lambda: |a| a + x
        let lambda_params = vec![
            ast::Param {
                mutable: false,
                name: ast::Spanned::dummy("a".into()),
                ty: None,
                default: None,
                span: Span::dummy(),
            },
        ];

        let lambda_body = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::Add,
                left: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("a".into()),
                    Span::dummy(),
                )),
                right: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("x".into()),
                    Span::dummy(),
                )),
            },
            Span::dummy(),
        );

        let (captures, result) = checker.analyze_and_validate_closure_captures(
            &lambda_params, &lambda_body, &env, Span::dummy()
        );

        assert_eq!(captures.len(), 1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_multiple_captures_with_different_modes() {
        let checker = TypeChecker::new();
        let mut env = CaptureEnv::new();
        env.define_var("read_only".to_string(), Type::Int, false);
        env.define_var("to_move".to_string(), Type::String, false);

        // Lambda that reads one var and moves another: || { let _ = read_only; send(to_move) }
        let lambda_params: Vec<ast::Param> = vec![];

        // Create a block body with two expressions
        let lambda_body = ast::Expr::new(
            ast::ExprKind::Block(ast::Block {
                stmts: vec![
                    ast::Stmt {
                        kind: ast::StmtKind::Expr(ast::Expr::new(
                            ast::ExprKind::Ident("read_only".into()),
                            Span::dummy(),
                        )),
                        span: Span::dummy(),
                    },
                    ast::Stmt {
                        kind: ast::StmtKind::Expr(ast::Expr::new(
                            ast::ExprKind::Call {
                                func: Box::new(ast::Expr::new(
                                    ast::ExprKind::Ident("send".into()),
                                    Span::dummy(),
                                )),
                                args: vec![ast::CallArg {
                                    name: None,
                                    value: ast::Expr::new(
                                        ast::ExprKind::Ident("to_move".into()),
                                        Span::dummy(),
                                    ),
                                    spread: false,
                                }],
                            },
                            Span::dummy(),
                        )),
                        span: Span::dummy(),
                    },
                ],
                span: Span::dummy(),
            }),
            Span::dummy(),
        );

        let captures = checker.analyze_lambda_captures(&lambda_params, &lambda_body, &env);

        assert_eq!(captures.len(), 2);

        // Find each capture by name and check its mode
        let read_capture = captures.iter().find(|c| c.name == "read_only").unwrap();
        assert_eq!(read_capture.mode, CaptureMode::Borrow);

        let move_capture = captures.iter().find(|c| c.name == "to_move").unwrap();
        assert_eq!(move_capture.mode, CaptureMode::Move);
    }

    // ========================================================================
    // Const Expression Evaluation Tests
    // ========================================================================

    #[test]
    fn test_const_eval_integer_literal() {
        let checker = TypeChecker::new();
        let expr = ast::Expr::new(ast::ExprKind::Integer("42".into()), Span::dummy());
        let result = checker.eval_const_expr(&expr).unwrap();
        assert_eq!(result, ConstValue::Int(42));
    }

    #[test]
    fn test_const_eval_float_literal() {
        let checker = TypeChecker::new();
        let expr = ast::Expr::new(ast::ExprKind::Float("3.14".into()), Span::dummy());
        let result = checker.eval_const_expr(&expr).unwrap();
        assert_eq!(result, ConstValue::Float(3.14));
    }

    #[test]
    fn test_const_eval_bool_literal() {
        let checker = TypeChecker::new();
        let true_expr = ast::Expr::new(ast::ExprKind::Bool(true), Span::dummy());
        let false_expr = ast::Expr::new(ast::ExprKind::Bool(false), Span::dummy());
        assert_eq!(checker.eval_const_expr(&true_expr).unwrap(), ConstValue::Bool(true));
        assert_eq!(checker.eval_const_expr(&false_expr).unwrap(), ConstValue::Bool(false));
    }

    #[test]
    fn test_const_eval_string_literal() {
        let checker = TypeChecker::new();
        let expr = ast::Expr::new(ast::ExprKind::String("hello".into()), Span::dummy());
        let result = checker.eval_const_expr(&expr).unwrap();
        assert_eq!(result, ConstValue::String("hello".to_string()));
    }

    #[test]
    fn test_const_eval_char_literal() {
        let checker = TypeChecker::new();
        let expr = ast::Expr::new(ast::ExprKind::Char("a".into()), Span::dummy());
        let result = checker.eval_const_expr(&expr).unwrap();
        assert_eq!(result, ConstValue::Char('a'));
    }

    #[test]
    fn test_const_eval_tuple() {
        let checker = TypeChecker::new();
        let expr = ast::Expr::new(
            ast::ExprKind::Tuple(vec![
                ast::Expr::new(ast::ExprKind::Integer("1".into()), Span::dummy()),
                ast::Expr::new(ast::ExprKind::Integer("2".into()), Span::dummy()),
            ]),
            Span::dummy(),
        );
        let result = checker.eval_const_expr(&expr).unwrap();
        assert_eq!(result, ConstValue::Tuple(vec![ConstValue::Int(1), ConstValue::Int(2)]));
    }

    #[test]
    fn test_const_eval_array() {
        let checker = TypeChecker::new();
        let expr = ast::Expr::new(
            ast::ExprKind::Array(vec![
                ast::Expr::new(ast::ExprKind::Integer("10".into()), Span::dummy()),
                ast::Expr::new(ast::ExprKind::Integer("20".into()), Span::dummy()),
                ast::Expr::new(ast::ExprKind::Integer("30".into()), Span::dummy()),
            ]),
            Span::dummy(),
        );
        let result = checker.eval_const_expr(&expr).unwrap();
        assert_eq!(
            result,
            ConstValue::Array(vec![ConstValue::Int(10), ConstValue::Int(20), ConstValue::Int(30)])
        );
    }

    #[test]
    fn test_const_eval_unary_neg() {
        let checker = TypeChecker::new();
        let expr = ast::Expr::new(
            ast::ExprKind::Unary {
                op: ast::UnaryOp::Neg,
                operand: Box::new(ast::Expr::new(ast::ExprKind::Integer("42".into()), Span::dummy())),
            },
            Span::dummy(),
        );
        let result = checker.eval_const_expr(&expr).unwrap();
        assert_eq!(result, ConstValue::Int(-42));
    }

    #[test]
    fn test_const_eval_unary_not() {
        let checker = TypeChecker::new();
        let expr = ast::Expr::new(
            ast::ExprKind::Unary {
                op: ast::UnaryOp::Not,
                operand: Box::new(ast::Expr::new(ast::ExprKind::Bool(true), Span::dummy())),
            },
            Span::dummy(),
        );
        let result = checker.eval_const_expr(&expr).unwrap();
        assert_eq!(result, ConstValue::Bool(false));
    }

    #[test]
    fn test_const_eval_binary_add() {
        let checker = TypeChecker::new();
        let expr = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::Add,
                left: Box::new(ast::Expr::new(ast::ExprKind::Integer("10".into()), Span::dummy())),
                right: Box::new(ast::Expr::new(ast::ExprKind::Integer("20".into()), Span::dummy())),
            },
            Span::dummy(),
        );
        let result = checker.eval_const_expr(&expr).unwrap();
        assert_eq!(result, ConstValue::Int(30));
    }

    #[test]
    fn test_const_eval_binary_mul() {
        let checker = TypeChecker::new();
        let expr = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::Mul,
                left: Box::new(ast::Expr::new(ast::ExprKind::Integer("6".into()), Span::dummy())),
                right: Box::new(ast::Expr::new(ast::ExprKind::Integer("7".into()), Span::dummy())),
            },
            Span::dummy(),
        );
        let result = checker.eval_const_expr(&expr).unwrap();
        assert_eq!(result, ConstValue::Int(42));
    }

    #[test]
    fn test_const_eval_division_by_zero() {
        let checker = TypeChecker::new();
        let expr = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::Div,
                left: Box::new(ast::Expr::new(ast::ExprKind::Integer("10".into()), Span::dummy())),
                right: Box::new(ast::Expr::new(ast::ExprKind::Integer("0".into()), Span::dummy())),
            },
            Span::dummy(),
        );
        let result = checker.eval_const_expr(&expr);
        assert!(matches!(result, Err(TypeError::ConstDivisionByZero { .. })));
    }

    #[test]
    fn test_const_eval_comparison_lt() {
        let checker = TypeChecker::new();
        let expr = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::Lt,
                left: Box::new(ast::Expr::new(ast::ExprKind::Integer("5".into()), Span::dummy())),
                right: Box::new(ast::Expr::new(ast::ExprKind::Integer("10".into()), Span::dummy())),
            },
            Span::dummy(),
        );
        let result = checker.eval_const_expr(&expr).unwrap();
        assert_eq!(result, ConstValue::Bool(true));
    }

    #[test]
    fn test_const_eval_boolean_and() {
        let checker = TypeChecker::new();
        let expr = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::And,
                left: Box::new(ast::Expr::new(ast::ExprKind::Bool(true), Span::dummy())),
                right: Box::new(ast::Expr::new(ast::ExprKind::Bool(false), Span::dummy())),
            },
            Span::dummy(),
        );
        let result = checker.eval_const_expr(&expr).unwrap();
        assert_eq!(result, ConstValue::Bool(false));
    }

    #[test]
    fn test_const_eval_string_concat() {
        let checker = TypeChecker::new();
        let expr = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::Add,
                left: Box::new(ast::Expr::new(ast::ExprKind::String("Hello, ".into()), Span::dummy())),
                right: Box::new(ast::Expr::new(ast::ExprKind::String("World!".into()), Span::dummy())),
            },
            Span::dummy(),
        );
        let result = checker.eval_const_expr(&expr).unwrap();
        assert_eq!(result, ConstValue::String("Hello, World!".to_string()));
    }

    #[test]
    fn test_const_eval_bitwise_and() {
        let checker = TypeChecker::new();
        let expr = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::BitAnd,
                left: Box::new(ast::Expr::new(ast::ExprKind::Integer("12".into()), Span::dummy())),
                right: Box::new(ast::Expr::new(ast::ExprKind::Integer("10".into()), Span::dummy())),
            },
            Span::dummy(),
        );
        let result = checker.eval_const_expr(&expr).unwrap();
        assert_eq!(result, ConstValue::Int(8)); // 1100 & 1010 = 1000
    }

    #[test]
    fn test_const_eval_power() {
        let checker = TypeChecker::new();
        let expr = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::Pow,
                left: Box::new(ast::Expr::new(ast::ExprKind::Integer("2".into()), Span::dummy())),
                right: Box::new(ast::Expr::new(ast::ExprKind::Integer("10".into()), Span::dummy())),
            },
            Span::dummy(),
        );
        let result = checker.eval_const_expr(&expr).unwrap();
        assert_eq!(result, ConstValue::Int(1024)); // 2^10 = 1024
    }

    #[test]
    fn test_const_eval_nested_expression() {
        let checker = TypeChecker::new();
        // (2 + 3) * 4 = 20
        let expr = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::Mul,
                left: Box::new(ast::Expr::new(
                    ast::ExprKind::Paren(Box::new(ast::Expr::new(
                        ast::ExprKind::Binary {
                            op: ast::BinaryOp::Add,
                            left: Box::new(ast::Expr::new(ast::ExprKind::Integer("2".into()), Span::dummy())),
                            right: Box::new(ast::Expr::new(ast::ExprKind::Integer("3".into()), Span::dummy())),
                        },
                        Span::dummy(),
                    ))),
                    Span::dummy(),
                )),
                right: Box::new(ast::Expr::new(ast::ExprKind::Integer("4".into()), Span::dummy())),
            },
            Span::dummy(),
        );
        let result = checker.eval_const_expr(&expr).unwrap();
        assert_eq!(result, ConstValue::Int(20));
    }

    #[test]
    fn test_const_eval_non_constant_variable() {
        let checker = TypeChecker::new();
        let expr = ast::Expr::new(ast::ExprKind::Ident("x".into()), Span::dummy());
        let result = checker.eval_const_expr(&expr);
        assert!(matches!(result, Err(TypeError::NotConstant { .. })));
    }

    #[test]
    fn test_const_propagation() {
        let mut checker = TypeChecker::new();
        // Manually add a const value for testing
        checker.const_values.insert("MAX_SIZE".to_string(), ConstValue::Int(1024));

        // Now evaluating MAX_SIZE * 2 should work
        let expr = ast::Expr::new(
            ast::ExprKind::Binary {
                op: ast::BinaryOp::Mul,
                left: Box::new(ast::Expr::new(ast::ExprKind::Ident("MAX_SIZE".into()), Span::dummy())),
                right: Box::new(ast::Expr::new(ast::ExprKind::Integer("2".into()), Span::dummy())),
            },
            Span::dummy(),
        );
        let result = checker.eval_const_expr(&expr).unwrap();
        assert_eq!(result, ConstValue::Int(2048));
    }

    #[test]
    fn test_const_value_type() {
        assert_eq!(ConstValue::Int(42).ty(), Type::Int);
        assert_eq!(ConstValue::Float(3.14).ty(), Type::Float);
        assert_eq!(ConstValue::Bool(true).ty(), Type::Bool);
        assert_eq!(ConstValue::Char('x').ty(), Type::Char);
        assert_eq!(ConstValue::String("test".to_string()).ty(), Type::String);
        assert_eq!(ConstValue::Unit.ty(), Type::Unit);
    }

    #[test]
    fn test_const_value_conversions() {
        let int_val = ConstValue::Int(42);
        assert_eq!(int_val.as_int(), Some(42));
        assert_eq!(int_val.as_float(), Some(42.0));
        assert_eq!(int_val.as_usize(), Some(42));
        assert_eq!(int_val.as_bool(), None);

        let float_val = ConstValue::Float(3.14);
        assert_eq!(float_val.as_float(), Some(3.14));
        assert_eq!(float_val.as_int(), None);

        let bool_val = ConstValue::Bool(true);
        assert_eq!(bool_val.as_bool(), Some(true));
        assert_eq!(bool_val.as_int(), None);

        let neg_int = ConstValue::Int(-5);
        assert_eq!(neg_int.as_usize(), None);
    }

    #[test]
    fn test_const_value_display() {
        assert_eq!(format!("{}", ConstValue::Int(42)), "42");
        assert_eq!(format!("{}", ConstValue::Float(3.14)), "3.14");
        assert_eq!(format!("{}", ConstValue::Bool(true)), "true");
        assert_eq!(format!("{}", ConstValue::Char('a')), "'a'");
        assert_eq!(format!("{}", ConstValue::String("hello".to_string())), "\"hello\"");
        assert_eq!(format!("{}", ConstValue::Unit), "()");
    }

    // ========================================================================
    // Defer Statement Type Checking Tests
    // ========================================================================

    #[test]
    fn test_defer_basic_expression() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();
        env.define_var("x".to_string(), Type::Int);

        // defer println(x) - should type check successfully
        let defer_stmt = ast::Stmt {
            kind: ast::StmtKind::Defer(ast::Expr::new(
                ast::ExprKind::Call {
                    func: Box::new(ast::Expr::new(
                        ast::ExprKind::Ident("println".into()),
                        Span::dummy(),
                    )),
                    args: vec![ast::CallArg {
                        name: None,
                        value: ast::Expr::new(ast::ExprKind::Ident("x".into()), Span::dummy()),
                        spread: false,
                    }],
                },
                Span::dummy(),
            )),
            span: Span::dummy(),
        };

        let result = checker.check_stmt(&defer_stmt, &mut env);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Type::Unit);
    }

    #[test]
    fn test_defer_return_in_defer_error() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        // First, set up a return type context
        checker.current_return_type = Some(Type::Int);

        // Simulate being in a defer context
        checker.in_defer_context = true;

        // return 42 inside defer should produce an error
        let return_stmt = ast::Stmt {
            kind: ast::StmtKind::Return(Some(ast::Expr::new(
                ast::ExprKind::Integer("42".into()),
                Span::dummy(),
            ))),
            span: Span::dummy(),
        };

        let result = checker.check_stmt(&return_stmt, &mut env);
        assert!(result.is_err());
        match result.unwrap_err() {
            TypeError::ControlFlowInDefer { statement, .. } => {
                assert_eq!(statement, "return");
            }
            other => panic!("Expected ControlFlowInDefer error, got {:?}", other),
        }
    }

    #[test]
    fn test_defer_break_in_defer_error() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        // Simulate being in a defer context
        checker.in_defer_context = true;

        // break inside defer should produce an error
        let break_stmt = ast::Stmt {
            kind: ast::StmtKind::Break(None),
            span: Span::dummy(),
        };

        let result = checker.check_stmt(&break_stmt, &mut env);
        assert!(result.is_err());
        match result.unwrap_err() {
            TypeError::ControlFlowInDefer { statement, .. } => {
                assert_eq!(statement, "break");
            }
            other => panic!("Expected ControlFlowInDefer error, got {:?}", other),
        }
    }

    #[test]
    fn test_defer_continue_in_defer_error() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        // Simulate being in a defer context
        checker.in_defer_context = true;

        // continue inside defer should produce an error
        let continue_stmt = ast::Stmt {
            kind: ast::StmtKind::Continue,
            span: Span::dummy(),
        };

        let result = checker.check_stmt(&continue_stmt, &mut env);
        assert!(result.is_err());
        match result.unwrap_err() {
            TypeError::ControlFlowInDefer { statement, .. } => {
                assert_eq!(statement, "continue");
            }
            other => panic!("Expected ControlFlowInDefer error, got {:?}", other),
        }
    }

    #[test]
    fn test_defer_context_reset() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();
        env.define_var("x".to_string(), Type::Int);

        // Initially not in defer context
        assert!(!checker.in_defer_context);

        // defer println(x)
        let defer_stmt = ast::Stmt {
            kind: ast::StmtKind::Defer(ast::Expr::new(
                ast::ExprKind::Call {
                    func: Box::new(ast::Expr::new(
                        ast::ExprKind::Ident("println".into()),
                        Span::dummy(),
                    )),
                    args: vec![ast::CallArg {
                        name: None,
                        value: ast::Expr::new(ast::ExprKind::Ident("x".into()), Span::dummy()),
                        spread: false,
                    }],
                },
                Span::dummy(),
            )),
            span: Span::dummy(),
        };

        let _ = checker.check_stmt(&defer_stmt, &mut env);

        // After processing defer, context should be reset
        assert!(!checker.in_defer_context);
    }

    #[test]
    fn test_defer_error_types() {
        // Test that defer-related error variants exist and display correctly
        let control_flow_err = TypeError::ControlFlowInDefer {
            statement: "return".to_string(),
            span: Span::dummy(),
        };
        assert!(format!("{}", control_flow_err).contains("return"));
        assert!(format!("{}", control_flow_err).contains("defer"));

        let await_err = TypeError::AwaitInDefer {
            span: Span::dummy(),
        };
        assert!(format!("{}", await_err).contains("await"));
        assert!(format!("{}", await_err).contains("defer"));

        let capture_err = TypeError::DeferCaptureInvalid {
            var_name: "x".to_string(),
            defer_span: Span::dummy(),
            var_span: Span::dummy(),
        };
        assert!(format!("{}", capture_err).contains("x"));
    }

    #[test]
    fn test_defer_valid_function_call() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        // Define a cleanup function
        env.define_var(
            "cleanup".to_string(),
            Type::Function {
                params: vec![],
                return_type: Box::new(Type::Unit),
            },
        );

        // defer cleanup() - should succeed
        let defer_stmt = ast::Stmt {
            kind: ast::StmtKind::Defer(ast::Expr::new(
                ast::ExprKind::Call {
                    func: Box::new(ast::Expr::new(
                        ast::ExprKind::Ident("cleanup".into()),
                        Span::dummy(),
                    )),
                    args: vec![],
                },
                Span::dummy(),
            )),
            span: Span::dummy(),
        };

        let result = checker.check_stmt(&defer_stmt, &mut env);
        assert!(result.is_ok());
    }

    // ========================================================================
    // Loop Type Checking Tests
    // ========================================================================

    #[test]
    fn test_while_loop_condition_must_be_bool() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        // while 42 { } - should fail (condition is Int, not Bool)
        let while_stmt = ast::Stmt {
            kind: ast::StmtKind::While {
                condition: ast::Expr::new(
                    ast::ExprKind::Integer("42".into()),
                    Span::dummy(),
                ),
                body: ast::Block { stmts: vec![], span: Span::dummy() },
            },
            span: Span::dummy(),
        };

        let result = checker.check_stmt(&while_stmt, &mut env);
        assert!(result.is_err());
    }

    #[test]
    fn test_while_loop_with_bool_condition() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        // while true { } - should succeed
        let while_stmt = ast::Stmt {
            kind: ast::StmtKind::While {
                condition: ast::Expr::new(
                    ast::ExprKind::Bool(true),
                    Span::dummy(),
                ),
                body: ast::Block { stmts: vec![], span: Span::dummy() },
            },
            span: Span::dummy(),
        };

        let result = checker.check_stmt(&while_stmt, &mut env);
        assert!(result.is_ok());
    }

    #[test]
    fn test_for_loop_over_array() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        // for x in [1, 2, 3] { }
        let for_stmt = ast::Stmt {
            kind: ast::StmtKind::For {
                pattern: ast::Pattern {
                    kind: ast::PatternKind::Ident("x".into()),
                    span: Span::dummy(),
                },
                iterable: ast::Expr::new(
                    ast::ExprKind::Array(vec![
                        ast::Expr::new(ast::ExprKind::Integer("1".into()), Span::dummy()),
                        ast::Expr::new(ast::ExprKind::Integer("2".into()), Span::dummy()),
                        ast::Expr::new(ast::ExprKind::Integer("3".into()), Span::dummy()),
                    ]),
                    Span::dummy(),
                ),
                body: ast::Block { stmts: vec![], span: Span::dummy() },
            },
            span: Span::dummy(),
        };

        let result = checker.check_stmt(&for_stmt, &mut env);
        assert!(result.is_ok());
    }

    #[test]
    fn test_for_loop_over_map() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        // Define a map variable
        env.define_var(
            "my_map".to_string(),
            Type::Map(Box::new(Type::String), Box::new(Type::Int)),
        );

        // for (k, v) in my_map { }
        let for_stmt = ast::Stmt {
            kind: ast::StmtKind::For {
                pattern: ast::Pattern {
                    kind: ast::PatternKind::Tuple(vec![
                        ast::Pattern {
                            kind: ast::PatternKind::Ident("k".into()),
                            span: Span::dummy(),
                        },
                        ast::Pattern {
                            kind: ast::PatternKind::Ident("v".into()),
                            span: Span::dummy(),
                        },
                    ]),
                    span: Span::dummy(),
                },
                iterable: ast::Expr::new(
                    ast::ExprKind::Ident("my_map".into()),
                    Span::dummy(),
                ),
                body: ast::Block { stmts: vec![], span: Span::dummy() },
            },
            span: Span::dummy(),
        };

        let result = checker.check_stmt(&for_stmt, &mut env);
        assert!(result.is_ok());
    }

    #[test]
    fn test_for_loop_not_iterable() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        // for x in 42 { } - should fail (Int is not iterable)
        let for_stmt = ast::Stmt {
            kind: ast::StmtKind::For {
                pattern: ast::Pattern {
                    kind: ast::PatternKind::Ident("x".into()),
                    span: Span::dummy(),
                },
                iterable: ast::Expr::new(
                    ast::ExprKind::Integer("42".into()),
                    Span::dummy(),
                ),
                body: ast::Block { stmts: vec![], span: Span::dummy() },
            },
            span: Span::dummy(),
        };

        let result = checker.check_stmt(&for_stmt, &mut env);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TypeError::NotIterable { .. }));
    }

    #[test]
    fn test_loop_with_break_value() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        // loop { break 42 }
        let loop_stmt = ast::Stmt {
            kind: ast::StmtKind::Loop {
                body: ast::Block {
                    stmts: vec![ast::Stmt {
                        kind: ast::StmtKind::Break(Some(ast::Expr::new(
                            ast::ExprKind::Integer("42".into()),
                            Span::dummy(),
                        ))),
                        span: Span::dummy(),
                    }],
                    span: Span::dummy(),
                },
            },
            span: Span::dummy(),
        };

        let result = checker.check_stmt(&loop_stmt, &mut env);
        assert!(result.is_ok());
        // The loop should return Int type
        assert_eq!(result.unwrap(), Type::Int);
    }

    #[test]
    fn test_loop_without_break_returns_unit() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        // loop { } - empty infinite loop
        let loop_stmt = ast::Stmt {
            kind: ast::StmtKind::Loop {
                body: ast::Block { stmts: vec![], span: Span::dummy() },
            },
            span: Span::dummy(),
        };

        let result = checker.check_stmt(&loop_stmt, &mut env);
        assert!(result.is_ok());
        // With no break, defaults to Unit
        assert_eq!(result.unwrap(), Type::Unit);
    }

    #[test]
    fn test_break_outside_loop_error() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        // break - outside any loop
        let break_stmt = ast::Stmt {
            kind: ast::StmtKind::Break(None),
            span: Span::dummy(),
        };

        let result = checker.check_stmt(&break_stmt, &mut env);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TypeError::BreakOutsideLoop { .. }));
    }

    #[test]
    fn test_continue_outside_loop_error() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        // continue - outside any loop
        let continue_stmt = ast::Stmt {
            kind: ast::StmtKind::Continue,
            span: Span::dummy(),
        };

        let result = checker.check_stmt(&continue_stmt, &mut env);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TypeError::ContinueOutsideLoop { .. }));
    }

    #[test]
    fn test_loop_variable_scoping() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        // for i in [1, 2, 3] { let y = i }
        // Variable 'i' should only be visible inside the loop body
        let for_stmt = ast::Stmt {
            kind: ast::StmtKind::For {
                pattern: ast::Pattern {
                    kind: ast::PatternKind::Ident("i".into()),
                    span: Span::dummy(),
                },
                iterable: ast::Expr::new(
                    ast::ExprKind::Array(vec![
                        ast::Expr::new(ast::ExprKind::Integer("1".into()), Span::dummy()),
                    ]),
                    Span::dummy(),
                ),
                body: ast::Block {
                    stmts: vec![ast::Stmt {
                        kind: ast::StmtKind::Let {
                            pattern: ast::Pattern {
                                kind: ast::PatternKind::Ident("y".into()),
                                span: Span::dummy(),
                            },
                            ty: None,
                            value: ast::Expr::new(
                                ast::ExprKind::Ident("i".into()),
                                Span::dummy(),
                            ),
                        },
                        span: Span::dummy(),
                    }],
                    span: Span::dummy(),
                },
            },
            span: Span::dummy(),
        };

        let result = checker.check_stmt(&for_stmt, &mut env);
        assert!(result.is_ok());

        // Verify 'i' is NOT in the outer environment
        assert!(env.lookup_var("i").is_none());
    }

    #[test]
    fn test_nested_loops() {
        let mut checker = TypeChecker::new();
        let mut env = TypeEnv::new();

        // while true { while true { break } }
        let nested_while = ast::Stmt {
            kind: ast::StmtKind::While {
                condition: ast::Expr::new(ast::ExprKind::Bool(true), Span::dummy()),
                body: ast::Block {
                    stmts: vec![ast::Stmt {
                        kind: ast::StmtKind::While {
                            condition: ast::Expr::new(ast::ExprKind::Bool(true), Span::dummy()),
                            body: ast::Block {
                                stmts: vec![ast::Stmt {
                                    kind: ast::StmtKind::Break(None),
                                    span: Span::dummy(),
                                }],
                                span: Span::dummy(),
                            },
                        },
                        span: Span::dummy(),
                    }],
                    span: Span::dummy(),
                },
            },
            span: Span::dummy(),
        };

        let result = checker.check_stmt(&nested_while, &mut env);
        assert!(result.is_ok());
    }
}
