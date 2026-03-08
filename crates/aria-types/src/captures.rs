//! Closure capture analysis types.
//!
//! Defines capture modes, capture information, variable info,
//! and the capture environment used for analyzing closures and
//! spawn expressions.

use crate::{Type, TypeEnv};
use aria_ast::Span;
use std::rc::Rc;

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
