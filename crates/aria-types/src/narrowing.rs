//! Flow-sensitive type narrowing.
//!
//! Implements type narrowing based on control flow analysis.
//! Supports nil checks (`x != nil`), type guards (`x is T`),
//! and logical combinations.

use crate::{Type, TypeEnv, TypeScheme, primitive_type_lookup};
use aria_ast::{self as ast, Span};
use std::rc::Rc;


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
