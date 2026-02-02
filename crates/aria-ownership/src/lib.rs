//! Aria Ownership Analyzer
//!
//! Implements the 80/15/5 hybrid ownership model:
//! - 80% inferred (move semantics, local borrowing)
//! - 15% explicit annotations (ref[L], mut ref[L])
//! - 5% ARC escape hatch (@shared, @weak)
//!
//! # Ownership Model
//!
//! Aria uses a three-tier ownership system:
//!
//! ## Tier 1: Inferred Ownership (80%)
//! - Single-owner patterns: automatic
//! - Move semantics: automatic
//! - Local borrowing: automatic
//! - Function-scoped references: automatic
//!
//! ## Tier 2: Explicit Annotations (15%)
//! - Multiple-source returns: `ref[L]` syntax
//! - Reference-holding structs: `[life L]` parameter
//! - Complex lifetime bounds: where clauses
//!
//! ## Tier 3: ARC Escape Hatch (5%)
//! - Cyclic structures: `@shared` class
//! - Observer patterns: `@weak` references
//! - Graph structures: reference counting

use aria_ast::{
    Block, Expr, ExprKind, FunctionBody, FunctionDecl, Pattern, PatternKind, Stmt,
    StmtKind,
};
use aria_lexer::Span;
use rustc_hash::FxHashMap;
use smol_str::SmolStr;
use std::fmt;
use thiserror::Error;

// ============================================================================
// Core Types
// ============================================================================

/// Lifetime identifier for tracking borrow scopes.
///
/// Each lifetime represents a scope during which a borrow is valid.
/// Lifetimes form a partial order based on outlives relationships.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LifetimeId(pub u32);

impl LifetimeId {
    /// The static lifetime - outlives all other lifetimes.
    pub const STATIC: LifetimeId = LifetimeId(0);

    /// Create a new lifetime with the given ID.
    pub fn new(id: u32) -> Self {
        LifetimeId(id)
    }
}

impl fmt::Display for LifetimeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *self == Self::STATIC {
            write!(f, "'static")
        } else {
            write!(f, "'L{}", self.0)
        }
    }
}

/// Ownership kind for a value.
///
/// Determines how a value can be used and when it must be dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ownership {
    /// Owned value (moved on transfer).
    ///
    /// The holder is responsible for dropping the value.
    /// When assigned to another variable, ownership transfers (move).
    Owned,

    /// Immutable reference (borrowed).
    ///
    /// Multiple immutable borrows can coexist.
    /// Cannot mutate the borrowed value.
    Borrowed { lifetime: LifetimeId },

    /// Mutable reference (exclusively borrowed).
    ///
    /// Only one mutable borrow can exist at a time.
    /// Can mutate the borrowed value.
    BorrowedMut { lifetime: LifetimeId },

    /// Reference-counted (ARC escape hatch).
    ///
    /// Used for shared ownership, particularly in cyclic data structures.
    /// Marked with `@shared` in source.
    Shared,

    /// Weak reference (non-owning).
    ///
    /// Does not prevent deallocation of the target.
    /// Must be upgraded to Shared before use.
    /// Marked with `@weak` in source.
    Weak,
}

impl Ownership {
    /// Check if this ownership kind requires a lifetime.
    pub fn has_lifetime(&self) -> bool {
        matches!(self, Ownership::Borrowed { .. } | Ownership::BorrowedMut { .. })
    }

    /// Get the lifetime if this is a borrowed ownership.
    pub fn lifetime(&self) -> Option<LifetimeId> {
        match self {
            Ownership::Borrowed { lifetime } | Ownership::BorrowedMut { lifetime } => {
                Some(*lifetime)
            }
            _ => None,
        }
    }

    /// Check if this is a mutable reference.
    pub fn is_mutable_borrow(&self) -> bool {
        matches!(self, Ownership::BorrowedMut { .. })
    }

    /// Check if this is any kind of borrow (mutable or immutable).
    pub fn is_borrow(&self) -> bool {
        matches!(self, Ownership::Borrowed { .. } | Ownership::BorrowedMut { .. })
    }
}

impl fmt::Display for Ownership {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ownership::Owned => write!(f, "owned"),
            Ownership::Borrowed { lifetime } => write!(f, "ref[{}]", lifetime),
            Ownership::BorrowedMut { lifetime } => write!(f, "mut ref[{}]", lifetime),
            Ownership::Shared => write!(f, "@shared"),
            Ownership::Weak => write!(f, "@weak"),
        }
    }
}

/// The kind of borrow: immutable or mutable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorrowKind {
    /// Immutable borrow - read-only access.
    Immutable,
    /// Mutable borrow - read-write access.
    Mutable,
}

impl fmt::Display for BorrowKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BorrowKind::Immutable => write!(f, "immutable"),
            BorrowKind::Mutable => write!(f, "mutable"),
        }
    }
}

/// Information about an active borrow.
///
/// Tracks which variable is borrowed, how it's borrowed,
/// and the lifetime/scope of the borrow.
#[derive(Debug, Clone)]
pub struct BorrowInfo {
    /// The variable being borrowed.
    pub var: SmolStr,
    /// The kind of borrow (immutable or mutable).
    pub kind: BorrowKind,
    /// The lifetime of this borrow.
    pub lifetime: LifetimeId,
    /// Source location of the borrow.
    pub span: Span,
}

impl BorrowInfo {
    /// Create a new borrow info.
    pub fn new(var: SmolStr, kind: BorrowKind, lifetime: LifetimeId, span: Span) -> Self {
        Self {
            var,
            kind,
            lifetime,
            span,
        }
    }
}

// ============================================================================
// Variable State Tracking
// ============================================================================

/// State of a variable during ownership analysis.
#[derive(Debug, Clone)]
pub struct VarState {
    /// The current ownership of this variable.
    pub ownership: Ownership,
    /// Whether this variable is mutable.
    pub mutable: bool,
    /// Whether this variable has been moved (and cannot be used).
    pub moved: bool,
    /// Source location where this variable was defined.
    pub def_span: Span,
    /// Location where this variable was moved (if moved).
    pub move_span: Option<Span>,
}

impl VarState {
    /// Create a new owned variable state.
    pub fn owned(mutable: bool, span: Span) -> Self {
        Self {
            ownership: Ownership::Owned,
            mutable,
            moved: false,
            def_span: span,
            move_span: None,
        }
    }

    /// Create a new borrowed variable state.
    pub fn borrowed(lifetime: LifetimeId, mutable: bool, span: Span) -> Self {
        Self {
            ownership: if mutable {
                Ownership::BorrowedMut { lifetime }
            } else {
                Ownership::Borrowed { lifetime }
            },
            mutable,
            moved: false,
            def_span: span,
            move_span: None,
        }
    }

    /// Mark this variable as moved.
    pub fn mark_moved(&mut self, span: Span) {
        self.moved = true;
        self.move_span = Some(span);
    }
}

// ============================================================================
// Errors
// ============================================================================

/// Errors that can occur during ownership analysis.
#[derive(Debug, Clone, Error)]
pub enum OwnershipError {
    /// Attempted to use a value after it was moved.
    #[error("use of moved value `{var}` at {use_span:?}")]
    UseAfterMove {
        var: SmolStr,
        use_span: Span,
        move_span: Span,
        def_span: Span,
    },

    /// Attempted to mutate through an immutable reference.
    #[error("cannot mutate through immutable reference `{var}` at {span:?}")]
    MutationThroughImmutableRef { var: SmolStr, span: Span },

    /// Attempted to take a mutable reference while other references exist.
    #[error("cannot borrow `{var}` as mutable because it is already borrowed at {existing_span:?}")]
    ConflictingBorrow {
        var: SmolStr,
        new_span: Span,
        existing_span: Span,
        existing_kind: BorrowKind,
    },

    /// Attempted to use a value while it is mutably borrowed.
    #[error("cannot use `{var}` because it is mutably borrowed at {borrow_span:?}")]
    UseWhileMutablyBorrowed {
        var: SmolStr,
        use_span: Span,
        borrow_span: Span,
    },

    /// Attempted to assign to an immutable variable.
    #[error("cannot assign to immutable variable `{var}` at {span:?}")]
    AssignToImmutable { var: SmolStr, span: Span },

    /// Variable not found in scope.
    #[error("undefined variable `{var}` at {span:?}")]
    UndefinedVariable { var: SmolStr, span: Span },

    /// Attempted to return a reference that outlives its source.
    #[error("reference to `{var}` does not live long enough at {span:?}")]
    ReferenceTooShort { var: SmolStr, span: Span },
}

/// Result type for ownership analysis.
pub type OwnershipResult<T> = Result<T, OwnershipError>;

// ============================================================================
// Ownership Analyzer
// ============================================================================

/// Ownership analysis context.
///
/// Performs ownership inference for Aria code, implementing the
/// 80/15/5 hybrid model described in ARIA-PD-002.
pub struct OwnershipAnalyzer {
    /// Counter for generating unique lifetime IDs.
    next_lifetime: u32,
    /// Variable -> ownership state mapping.
    var_state: FxHashMap<SmolStr, VarState>,
    /// Active borrows for conflict detection.
    active_borrows: Vec<BorrowInfo>,
    /// Errors collected during analysis.
    errors: Vec<OwnershipError>,
    /// Current scope depth (for lifetime tracking).
    scope_depth: u32,
    /// Lifetime for the current scope.
    current_lifetime: LifetimeId,
}

impl Default for OwnershipAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl OwnershipAnalyzer {
    /// Create a new ownership analyzer.
    pub fn new() -> Self {
        Self {
            next_lifetime: 1, // 0 is reserved for 'static
            var_state: FxHashMap::default(),
            active_borrows: Vec::new(),
            errors: Vec::new(),
            scope_depth: 0,
            current_lifetime: LifetimeId(1),
        }
    }

    /// Generate a fresh lifetime ID.
    pub fn fresh_lifetime(&mut self) -> LifetimeId {
        let id = self.next_lifetime;
        self.next_lifetime += 1;
        LifetimeId(id)
    }

    /// Enter a new scope.
    fn enter_scope(&mut self) {
        self.scope_depth += 1;
        self.current_lifetime = self.fresh_lifetime();
    }

    /// Exit the current scope, invalidating borrows in this scope.
    fn exit_scope(&mut self) {
        // Remove borrows that are scoped to the current lifetime
        let current = self.current_lifetime;
        self.active_borrows.retain(|b| b.lifetime != current);
        self.scope_depth = self.scope_depth.saturating_sub(1);
    }

    /// Get the errors collected during analysis.
    pub fn errors(&self) -> &[OwnershipError] {
        &self.errors
    }

    /// Take the errors, consuming them from the analyzer.
    pub fn take_errors(&mut self) -> Vec<OwnershipError> {
        std::mem::take(&mut self.errors)
    }

    /// Check if analysis completed without errors.
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get the ownership state of a variable.
    pub fn get_var_state(&self, name: &str) -> Option<&VarState> {
        self.var_state.get(name)
    }

    /// Get the ownership of a variable.
    pub fn get_ownership(&self, name: &str) -> Option<&Ownership> {
        self.var_state.get(name).map(|s| &s.ownership)
    }

    // ========================================================================
    // Borrow Checking
    // ========================================================================

    /// Check if a variable can be borrowed.
    fn check_can_borrow(
        &self,
        var: &SmolStr,
        kind: BorrowKind,
        span: Span,
    ) -> OwnershipResult<()> {
        // Check if the variable exists
        let state = self.var_state.get(var).ok_or_else(|| OwnershipError::UndefinedVariable {
            var: var.clone(),
            span,
        })?;

        // Check if the variable has been moved
        if state.moved {
            return Err(OwnershipError::UseAfterMove {
                var: var.clone(),
                use_span: span,
                move_span: state.move_span.unwrap_or(span),
                def_span: state.def_span,
            });
        }

        // Check for conflicting borrows
        for borrow in &self.active_borrows {
            if borrow.var == *var {
                // If we want a mutable borrow, any existing borrow is a conflict
                if kind == BorrowKind::Mutable {
                    return Err(OwnershipError::ConflictingBorrow {
                        var: var.clone(),
                        new_span: span,
                        existing_span: borrow.span,
                        existing_kind: borrow.kind,
                    });
                }
                // If we want an immutable borrow, an existing mutable borrow is a conflict
                if borrow.kind == BorrowKind::Mutable {
                    return Err(OwnershipError::ConflictingBorrow {
                        var: var.clone(),
                        new_span: span,
                        existing_span: borrow.span,
                        existing_kind: borrow.kind,
                    });
                }
            }
        }

        Ok(())
    }

    /// Record an immutable borrow of a variable.
    fn borrow(&mut self, var: SmolStr, span: Span) -> OwnershipResult<LifetimeId> {
        self.check_can_borrow(&var, BorrowKind::Immutable, span)?;

        let lifetime = self.current_lifetime;
        self.active_borrows.push(BorrowInfo::new(
            var,
            BorrowKind::Immutable,
            lifetime,
            span,
        ));

        Ok(lifetime)
    }

    /// Record a mutable borrow of a variable.
    #[allow(dead_code)] // Will be used for mutable reference handling
    fn borrow_mut(&mut self, var: SmolStr, span: Span) -> OwnershipResult<LifetimeId> {
        self.check_can_borrow(&var, BorrowKind::Mutable, span)?;

        // Also check that the variable is mutable
        if let Some(state) = self.var_state.get(&var) {
            if !state.mutable {
                return Err(OwnershipError::MutationThroughImmutableRef {
                    var: var.clone(),
                    span,
                });
            }
        }

        let lifetime = self.current_lifetime;
        self.active_borrows.push(BorrowInfo::new(
            var,
            BorrowKind::Mutable,
            lifetime,
            span,
        ));

        Ok(lifetime)
    }

    /// Mark a variable as moved.
    fn mark_moved(&mut self, var: &SmolStr, span: Span) -> OwnershipResult<()> {
        // Check for active borrows
        for borrow in &self.active_borrows {
            if borrow.var == *var {
                return Err(OwnershipError::UseWhileMutablyBorrowed {
                    var: var.clone(),
                    use_span: span,
                    borrow_span: borrow.span,
                });
            }
        }

        if let Some(state) = self.var_state.get_mut(var) {
            if state.moved {
                return Err(OwnershipError::UseAfterMove {
                    var: var.clone(),
                    use_span: span,
                    move_span: state.move_span.unwrap_or(span),
                    def_span: state.def_span,
                });
            }
            state.mark_moved(span);
        }

        Ok(())
    }

    // ========================================================================
    // Analysis Entry Points
    // ========================================================================

    /// Analyze a function declaration for ownership.
    ///
    /// This is the main entry point for ownership analysis.
    /// Infers ownership for all variables and checks borrow rules.
    pub fn analyze_function(&mut self, func: &FunctionDecl) -> OwnershipResult<()> {
        // Enter function scope
        self.enter_scope();

        // Process parameters - they are owned by the function
        for param in &func.params {
            let name = param.name.node.clone();
            self.var_state.insert(
                name,
                VarState::owned(param.mutable, param.span),
            );
        }

        // Analyze the function body
        match &func.body {
            FunctionBody::Block(block) => {
                self.analyze_block(block)?;
            }
            FunctionBody::Expression(expr) => {
                self.analyze_expr(expr)?;
            }
        }

        // Exit function scope
        self.exit_scope();

        Ok(())
    }

    /// Analyze a block of statements.
    pub fn analyze_block(&mut self, block: &Block) -> OwnershipResult<()> {
        self.enter_scope();

        for stmt in &block.stmts {
            self.analyze_stmt(stmt)?;
        }

        self.exit_scope();
        Ok(())
    }

    /// Analyze a single statement.
    pub fn analyze_stmt(&mut self, stmt: &Stmt) -> OwnershipResult<()> {
        match &stmt.kind {
            StmtKind::Let { pattern, value, .. } => {
                // Analyze the initializer expression first
                self.analyze_expr(value)?;

                // Bind the pattern variables
                self.bind_pattern(pattern, false)?;
            }

            StmtKind::Var { name, value, .. } => {
                // Analyze the initializer
                self.analyze_expr(value)?;

                // Create mutable variable
                self.var_state.insert(
                    name.node.clone(),
                    VarState::owned(true, name.span),
                );
            }

            StmtKind::Const { name, value, .. } => {
                // Analyze the initializer
                self.analyze_expr(value)?;

                // Constants are immutable
                self.var_state.insert(
                    name.node.clone(),
                    VarState::owned(false, name.span),
                );
            }

            StmtKind::Assign { target, value, .. } => {
                // Analyze the value first
                self.analyze_expr(value)?;

                // Check if target is assignable
                self.analyze_assignment_target(target)?;
            }

            StmtKind::Expr(expr) => {
                self.analyze_expr(expr)?;
            }

            StmtKind::If {
                condition,
                then_branch,
                elsif_branches,
                else_branch,
            } => {
                self.analyze_expr(condition)?;
                self.analyze_block(then_branch)?;

                for (cond, block) in elsif_branches {
                    self.analyze_expr(cond)?;
                    self.analyze_block(block)?;
                }

                if let Some(else_block) = else_branch {
                    self.analyze_block(else_block)?;
                }
            }

            StmtKind::While { condition, body } => {
                self.analyze_expr(condition)?;
                self.analyze_block(body)?;
            }

            StmtKind::For {
                pattern,
                iterable,
                body,
            } => {
                // Analyze the iterable
                self.analyze_expr(iterable)?;

                // Enter loop scope
                self.enter_scope();

                // Bind the loop variable
                self.bind_pattern(pattern, false)?;

                // Analyze loop body
                for stmt in &body.stmts {
                    self.analyze_stmt(stmt)?;
                }

                self.exit_scope();
            }

            StmtKind::Loop { body } => {
                self.analyze_block(body)?;
            }

            StmtKind::Return(expr) => {
                if let Some(e) = expr {
                    self.analyze_expr(e)?;
                }
            }

            StmtKind::Break(expr) => {
                if let Some(e) = expr {
                    self.analyze_expr(e)?;
                }
            }

            StmtKind::Continue => {}

            StmtKind::Defer(expr) => {
                self.analyze_expr(expr)?;
            }

            StmtKind::Unless {
                condition,
                body,
                else_branch,
            } => {
                self.analyze_expr(condition)?;
                self.analyze_block(body)?;
                if let Some(else_block) = else_branch {
                    self.analyze_block(else_block)?;
                }
            }

            StmtKind::Match { scrutinee, arms } => {
                self.analyze_expr(scrutinee)?;
                for arm in arms {
                    self.enter_scope();
                    self.bind_pattern(&arm.pattern, false)?;
                    if let Some(guard) = &arm.guard {
                        self.analyze_expr(guard)?;
                    }
                    match &arm.body {
                        aria_ast::MatchArmBody::Expr(e) => { self.analyze_expr(e)?; }
                        aria_ast::MatchArmBody::Block(b) => { self.analyze_block(b)?; }
                    }
                    self.exit_scope();
                }
            }

            StmtKind::Unsafe(block) => {
                self.analyze_block(block)?;
            }

            StmtKind::Item(_) => {
                // Nested items are analyzed separately
            }
        }

        Ok(())
    }

    /// Analyze an expression.
    pub fn analyze_expr(&mut self, expr: &Expr) -> OwnershipResult<Ownership> {
        match &expr.kind {
            // Literals are always owned
            ExprKind::Integer(_)
            | ExprKind::Float(_)
            | ExprKind::String(_)
            | ExprKind::Char(_)
            | ExprKind::Bool(_)
            | ExprKind::Nil => Ok(Ownership::Owned),

            // Variable reference - check if valid and determine ownership
            ExprKind::Ident(name) => {
                let name = SmolStr::new(name.as_str());

                // Check if variable exists and is not moved
                if let Some(state) = self.var_state.get(&name) {
                    if state.moved {
                        self.errors.push(OwnershipError::UseAfterMove {
                            var: name.clone(),
                            use_span: expr.span,
                            move_span: state.move_span.unwrap_or(expr.span),
                            def_span: state.def_span,
                        });
                    }
                    Ok(state.ownership.clone())
                } else {
                    self.errors.push(OwnershipError::UndefinedVariable {
                        var: name,
                        span: expr.span,
                    });
                    Ok(Ownership::Owned)
                }
            }

            ExprKind::SelfLower | ExprKind::SelfUpper => {
                // `self` is always available in method context
                Ok(Ownership::Owned)
            }

            // Collections - analyze elements
            ExprKind::Array(elements) => {
                for elem in elements {
                    self.analyze_expr(elem)?;
                }
                Ok(Ownership::Owned)
            }

            ExprKind::Tuple(elements) => {
                for elem in elements {
                    self.analyze_expr(elem)?;
                }
                Ok(Ownership::Owned)
            }

            ExprKind::Map(pairs) => {
                for (key, value) in pairs {
                    self.analyze_expr(key)?;
                    self.analyze_expr(value)?;
                }
                Ok(Ownership::Owned)
            }

            // Binary operations - analyze both sides
            ExprKind::Binary { left, right, .. } => {
                self.analyze_expr(left)?;
                self.analyze_expr(right)?;
                Ok(Ownership::Owned)
            }

            // Unary operations
            ExprKind::Unary { operand, op } => {
                match op {
                    aria_ast::UnaryOp::Ref => {
                        // Reference creation - immutable borrow
                        if let ExprKind::Ident(name) = &operand.kind {
                            let name = SmolStr::new(name.as_str());
                            let lifetime = self.borrow(name, operand.span)?;
                            Ok(Ownership::Borrowed { lifetime })
                        } else {
                            self.analyze_expr(operand)?;
                            Ok(Ownership::Borrowed {
                                lifetime: self.current_lifetime,
                            })
                        }
                    }
                    aria_ast::UnaryOp::Deref => {
                        // Dereference
                        self.analyze_expr(operand)?;
                        Ok(Ownership::Owned)
                    }
                    _ => {
                        self.analyze_expr(operand)?;
                        Ok(Ownership::Owned)
                    }
                }
            }

            // Field access
            ExprKind::Field { object, .. } => {
                self.analyze_expr(object)?;
                Ok(Ownership::Borrowed {
                    lifetime: self.current_lifetime,
                })
            }

            // Index access
            ExprKind::Index { object, index } => {
                self.analyze_expr(object)?;
                self.analyze_expr(index)?;
                Ok(Ownership::Borrowed {
                    lifetime: self.current_lifetime,
                })
            }

            // Method call
            ExprKind::MethodCall { object, args, .. } => {
                self.analyze_expr(object)?;
                for arg in args {
                    self.analyze_expr(arg)?;
                }
                Ok(Ownership::Owned)
            }

            // Function call - arguments may be moved
            ExprKind::Call { func, args } => {
                self.analyze_expr(func)?;
                for arg in args {
                    self.analyze_expr(&arg.value)?;
                    // Arguments are moved by default (unless borrowed)
                    if let ExprKind::Ident(name) = &arg.value.kind {
                        let name = SmolStr::new(name.as_str());
                        // In Tier 1 inference, we assume function takes ownership
                        // unless the function signature indicates otherwise
                        if let Err(e) = self.mark_moved(&name, arg.value.span) {
                            self.errors.push(e);
                        }
                    }
                }
                Ok(Ownership::Owned)
            }

            // Control flow expressions
            ExprKind::If {
                condition,
                then_branch,
                elsif_branches,
                else_branch,
            } => {
                self.analyze_expr(condition)?;
                self.analyze_block(then_branch)?;
                for (cond, block) in elsif_branches {
                    self.analyze_expr(cond)?;
                    self.analyze_block(block)?;
                }
                if let Some(else_block) = else_branch {
                    self.analyze_block(else_block)?;
                }
                Ok(Ownership::Owned)
            }

            ExprKind::Match { scrutinee, arms } => {
                self.analyze_expr(scrutinee)?;
                for arm in arms {
                    self.enter_scope();
                    self.bind_pattern(&arm.pattern, false)?;
                    if let Some(guard) = &arm.guard {
                        self.analyze_expr(guard)?;
                    }
                    match &arm.body {
                        aria_ast::MatchArmBody::Expr(e) => { self.analyze_expr(e)?; }
                        aria_ast::MatchArmBody::Block(b) => { self.analyze_block(b)?; }
                    }
                    self.exit_scope();
                }
                Ok(Ownership::Owned)
            }

            ExprKind::Block(block) => {
                self.analyze_block(block)?;
                Ok(Ownership::Owned)
            }

            // Lambda expressions
            ExprKind::Lambda { params, body } => {
                self.enter_scope();
                for param in params {
                    self.var_state.insert(
                        param.name.node.clone(),
                        VarState::owned(param.mutable, param.span),
                    );
                }
                self.analyze_expr(body)?;
                self.exit_scope();
                Ok(Ownership::Owned)
            }

            ExprKind::BlockLambda { params, body } => {
                self.enter_scope();
                for param in params {
                    self.var_state.insert(
                        param.name.node.clone(),
                        VarState::owned(param.mutable, param.span),
                    );
                }
                self.analyze_block(body)?;
                self.exit_scope();
                Ok(Ownership::Owned)
            }

            // Comprehensions
            ExprKind::ArrayComprehension {
                element,
                pattern,
                iterable,
                condition,
            } => {
                self.analyze_expr(iterable)?;
                self.enter_scope();
                self.bind_pattern(pattern, false)?;
                if let Some(cond) = condition {
                    self.analyze_expr(cond)?;
                }
                self.analyze_expr(element)?;
                self.exit_scope();
                Ok(Ownership::Owned)
            }

            ExprKind::MapComprehension {
                key,
                value,
                pattern,
                iterable,
                condition,
            } => {
                self.analyze_expr(iterable)?;
                self.enter_scope();
                self.bind_pattern(pattern, false)?;
                if let Some(cond) = condition {
                    self.analyze_expr(cond)?;
                }
                self.analyze_expr(key)?;
                self.analyze_expr(value)?;
                self.exit_scope();
                Ok(Ownership::Owned)
            }

            // Special expressions
            ExprKind::Range { start, end, .. } => {
                if let Some(s) = start {
                    self.analyze_expr(s)?;
                }
                if let Some(e) = end {
                    self.analyze_expr(e)?;
                }
                Ok(Ownership::Owned)
            }

            ExprKind::Pipe { left, right } => {
                // Pipe: left |> right
                // The result of left is passed to right
                self.analyze_expr(left)?;
                self.analyze_expr(right)?;
                Ok(Ownership::Owned)
            }

            ExprKind::Try(inner) => {
                self.analyze_expr(inner)?;
                Ok(Ownership::Owned)
            }

            ExprKind::Unwrap(inner) => {
                self.analyze_expr(inner)?;
                Ok(Ownership::Owned)
            }

            ExprKind::SafeNav { object, .. } => {
                self.analyze_expr(object)?;
                Ok(Ownership::Owned)
            }

            ExprKind::StructInit { fields, .. } => {
                for field in fields {
                    if let Some(value) = &field.value {
                        self.analyze_expr(value)?;
                    }
                }
                Ok(Ownership::Owned)
            }

            // Concurrency
            ExprKind::Spawn(inner) => {
                // Spawned expression runs in a new context
                // Values must be moved into it
                self.analyze_expr(inner)?;
                Ok(Ownership::Owned)
            }

            ExprKind::Await(inner) => {
                self.analyze_expr(inner)?;
                Ok(Ownership::Owned)
            }

            ExprKind::Select(arms) => {
                use aria_ast::SelectArmKind;
                for arm in arms {
                    self.enter_scope();
                    match &arm.kind {
                        SelectArmKind::Receive { pattern, channel } => {
                            if let Some(pat) = pattern {
                                self.bind_pattern(pat, false)?;
                            }
                            self.analyze_expr(channel)?;
                        }
                        SelectArmKind::Send { channel, value } => {
                            self.analyze_expr(channel)?;
                            self.analyze_expr(value)?;
                        }
                        SelectArmKind::Default => {}
                    }
                    self.analyze_expr(&arm.body)?;
                    self.exit_scope();
                }
                Ok(Ownership::Owned)
            }

            // Contract expressions
            ExprKind::Old(inner) => {
                self.analyze_expr(inner)?;
                Ok(Ownership::Owned)
            }

            ExprKind::Result => Ok(Ownership::Owned),

            ExprKind::Forall { body, condition, .. } => {
                if let Some(cond) = condition {
                    self.analyze_expr(cond)?;
                }
                self.analyze_expr(body)?;
                Ok(Ownership::Owned)
            }

            ExprKind::Exists { body, condition, .. } => {
                if let Some(cond) = condition {
                    self.analyze_expr(cond)?;
                }
                self.analyze_expr(body)?;
                Ok(Ownership::Owned)
            }

            ExprKind::Paren(inner) => self.analyze_expr(inner),

            ExprKind::Ternary {
                condition,
                then_expr,
                else_expr,
            } => {
                self.analyze_expr(condition)?;
                self.analyze_expr(then_expr)?;
                self.analyze_expr(else_expr)?;
                Ok(Ownership::Owned)
            }

            ExprKind::Path(_) => Ok(Ownership::Owned),

            ExprKind::ChannelSend { channel, value } => {
                self.analyze_expr(channel)?;
                self.analyze_expr(value)?;
                Ok(Ownership::Owned)
            }

            ExprKind::ChannelRecv { channel } => {
                self.analyze_expr(channel)?;
                Ok(Ownership::Owned)
            }

            // Effect handling expressions
            ExprKind::Handle { body, handlers, return_clause } => {
                // Analyze the body being handled
                self.analyze_expr(body)?;

                // Analyze each handler clause body
                for handler in handlers {
                    match &handler.body {
                        aria_ast::HandlerBody::Expr(expr) => {
                            self.analyze_expr(expr)?;
                        }
                        aria_ast::HandlerBody::Block(block) => {
                            for stmt in &block.stmts {
                                self.analyze_stmt(stmt)?;
                            }
                        }
                    }
                }

                // Analyze return clause if present
                if let Some(ret_clause) = return_clause {
                    match ret_clause.body.as_ref() {
                        aria_ast::HandlerBody::Expr(expr) => {
                            self.analyze_expr(expr)?;
                        }
                        aria_ast::HandlerBody::Block(block) => {
                            for stmt in &block.stmts {
                                self.analyze_stmt(stmt)?;
                            }
                        }
                    }
                }

                Ok(Ownership::Owned)
            }

            ExprKind::Raise { error, .. } => {
                // Analyze the error value being raised
                self.analyze_expr(error)?;
                Ok(Ownership::Owned)
            }

            ExprKind::Resume { value } => {
                // Analyze the value being resumed with
                self.analyze_expr(value)?;
                Ok(Ownership::Owned)
            }

            ExprKind::Error => Ok(Ownership::Owned),

            // Interpolated strings - analyze each interpolated expression
            ExprKind::InterpolatedString(parts) => {
                for part in parts {
                    match part {
                        aria_ast::StringPart::Expr(expr) => {
                            self.analyze_expr(expr)?;
                        }
                        aria_ast::StringPart::FormattedExpr { expr, .. } => {
                            self.analyze_expr(expr)?;
                        }
                        aria_ast::StringPart::Literal(_) => {}
                    }
                }
                Ok(Ownership::Owned)
            }
        }
    }

    /// Bind variables from a pattern.
    fn bind_pattern(&mut self, pattern: &Pattern, mutable: bool) -> OwnershipResult<()> {
        match &pattern.kind {
            PatternKind::Wildcard => Ok(()),

            PatternKind::Ident(name) => {
                self.var_state.insert(
                    name.clone(),
                    VarState::owned(mutable, pattern.span),
                );
                Ok(())
            }

            PatternKind::Tuple(patterns) => {
                for p in patterns {
                    self.bind_pattern(p, mutable)?;
                }
                Ok(())
            }

            PatternKind::Array { elements, rest } => {
                for p in elements {
                    self.bind_pattern(p, mutable)?;
                }
                if let Some(rest_pattern) = rest {
                    self.bind_pattern(rest_pattern, mutable)?;
                }
                Ok(())
            }

            PatternKind::Struct { fields, .. } => {
                for field in fields {
                    if let Some(p) = &field.pattern {
                        self.bind_pattern(p, mutable)?;
                    } else {
                        // Shorthand: `Point { x, y }` binds x and y
                        self.var_state.insert(
                            field.name.node.clone(),
                            VarState::owned(mutable, field.span),
                        );
                    }
                }
                Ok(())
            }

            PatternKind::Variant { fields, .. } => {
                if let Some(patterns) = fields {
                    for p in patterns {
                        self.bind_pattern(p, mutable)?;
                    }
                }
                Ok(())
            }

            PatternKind::Or(patterns) => {
                // All branches must bind the same variables
                for p in patterns {
                    self.bind_pattern(p, mutable)?;
                }
                Ok(())
            }

            PatternKind::Guard { pattern, .. } => {
                self.bind_pattern(pattern, mutable)?;
                Ok(())
            }

            PatternKind::Binding { name, pattern } => {
                self.var_state.insert(
                    name.node.clone(),
                    VarState::owned(mutable, pattern.span),
                );
                self.bind_pattern(pattern, mutable)?;
                Ok(())
            }

            PatternKind::Typed { pattern, .. } => {
                self.bind_pattern(pattern, mutable)?;
                Ok(())
            }

            PatternKind::Rest(name) => {
                if let Some(n) = name {
                    self.var_state.insert(
                        n.node.clone(),
                        VarState::owned(mutable, pattern.span),
                    );
                }
                Ok(())
            }

            PatternKind::Literal(_) | PatternKind::Range { .. } => Ok(()),
        }
    }

    /// Analyze an assignment target.
    fn analyze_assignment_target(&mut self, target: &Expr) -> OwnershipResult<()> {
        match &target.kind {
            ExprKind::Ident(name) => {
                let name = SmolStr::new(name.as_str());

                if let Some(state) = self.var_state.get(&name) {
                    if !state.mutable {
                        self.errors.push(OwnershipError::AssignToImmutable {
                            var: name,
                            span: target.span,
                        });
                    }
                } else {
                    self.errors.push(OwnershipError::UndefinedVariable {
                        var: name,
                        span: target.span,
                    });
                }
            }

            ExprKind::Field { object, .. } => {
                // Check that we have mutable access to the object
                self.analyze_expr(object)?;
            }

            ExprKind::Index { object, index } => {
                self.analyze_expr(object)?;
                self.analyze_expr(index)?;
            }

            _ => {
                // Invalid assignment target
                self.analyze_expr(target)?;
            }
        }

        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lifetime_id() {
        let l1 = LifetimeId::new(1);
        let l2 = LifetimeId::new(2);
        assert_ne!(l1, l2);
        assert_eq!(l1, LifetimeId(1));
        assert_eq!(format!("{}", LifetimeId::STATIC), "'static");
        assert_eq!(format!("{}", l1), "'L1");
    }

    #[test]
    fn test_ownership_display() {
        assert_eq!(format!("{}", Ownership::Owned), "owned");
        assert_eq!(
            format!("{}", Ownership::Borrowed { lifetime: LifetimeId(1) }),
            "ref['L1]"
        );
        assert_eq!(
            format!("{}", Ownership::BorrowedMut { lifetime: LifetimeId(2) }),
            "mut ref['L2]"
        );
        assert_eq!(format!("{}", Ownership::Shared), "@shared");
        assert_eq!(format!("{}", Ownership::Weak), "@weak");
    }

    #[test]
    fn test_ownership_methods() {
        let owned = Ownership::Owned;
        let borrowed = Ownership::Borrowed { lifetime: LifetimeId(1) };
        let borrowed_mut = Ownership::BorrowedMut { lifetime: LifetimeId(2) };

        assert!(!owned.has_lifetime());
        assert!(borrowed.has_lifetime());
        assert!(borrowed_mut.has_lifetime());

        assert_eq!(owned.lifetime(), None);
        assert_eq!(borrowed.lifetime(), Some(LifetimeId(1)));
        assert_eq!(borrowed_mut.lifetime(), Some(LifetimeId(2)));

        assert!(!owned.is_borrow());
        assert!(borrowed.is_borrow());
        assert!(borrowed_mut.is_borrow());

        assert!(!owned.is_mutable_borrow());
        assert!(!borrowed.is_mutable_borrow());
        assert!(borrowed_mut.is_mutable_borrow());
    }

    #[test]
    fn test_analyzer_fresh_lifetime() {
        let mut analyzer = OwnershipAnalyzer::new();

        let l1 = analyzer.fresh_lifetime();
        let l2 = analyzer.fresh_lifetime();
        let l3 = analyzer.fresh_lifetime();

        assert_eq!(l1, LifetimeId(1));
        assert_eq!(l2, LifetimeId(2));
        assert_eq!(l3, LifetimeId(3));
    }

    #[test]
    fn test_var_state() {
        let span = Span::dummy();

        let owned = VarState::owned(true, span);
        assert!(matches!(owned.ownership, Ownership::Owned));
        assert!(owned.mutable);
        assert!(!owned.moved);

        let borrowed = VarState::borrowed(LifetimeId(1), false, span);
        assert!(matches!(borrowed.ownership, Ownership::Borrowed { .. }));
        assert!(!borrowed.mutable);
    }

    #[test]
    fn test_borrow_info() {
        let info = BorrowInfo::new(
            SmolStr::new("x"),
            BorrowKind::Immutable,
            LifetimeId(1),
            Span::dummy(),
        );

        assert_eq!(info.var, "x");
        assert_eq!(info.kind, BorrowKind::Immutable);
        assert_eq!(info.lifetime, LifetimeId(1));
    }

    #[test]
    fn test_analyzer_default() {
        let analyzer = OwnershipAnalyzer::default();
        assert!(analyzer.is_ok());
        assert!(analyzer.errors().is_empty());
    }
}
