//! Tree-walking interpreter for the Aria programming language.
//!
//! This crate provides the runtime evaluation of Aria programs by walking
//! the AST and executing each node directly.

use aria_lexer::Span;
use smol_str::SmolStr;
use thiserror::Error;

mod value;
mod environment;
mod eval;
mod builtins;

pub use value::Value;
pub use environment::Environment;
pub use eval::Interpreter;

/// Runtime errors that can occur during interpretation.
#[derive(Error, Debug, Clone)]
pub enum RuntimeError {
    #[error("undefined variable: {name}")]
    UndefinedVariable { name: SmolStr, span: Span },

    #[error("type error: {message}")]
    TypeError { message: String, span: Span },

    #[error("division by zero")]
    DivisionByZero { span: Span },

    #[error("index out of bounds: {index} (length {length})")]
    IndexOutOfBounds { index: i64, length: usize, span: Span },

    #[error("invalid key: {key}")]
    InvalidKey { key: String, span: Span },

    #[error("not callable: {value_type}")]
    NotCallable { value_type: String, span: Span },

    #[error("arity mismatch: expected {expected}, got {got}")]
    ArityMismatch { expected: usize, got: usize, span: Span },

    #[error("undefined field: {field} on {struct_name}")]
    UndefinedField { struct_name: SmolStr, field: SmolStr, span: Span },

    #[error("break outside loop")]
    BreakOutsideLoop { span: Span },

    #[error("continue outside loop")]
    ContinueOutsideLoop { span: Span },

    #[error("return outside function")]
    ReturnOutsideFunction { span: Span },

    #[error("assertion failed: {message}")]
    AssertionFailed { message: String, span: Span },

    #[error("runtime error: {message}")]
    General { message: String, span: Span },
}

impl RuntimeError {
    pub fn span(&self) -> Span {
        match self {
            RuntimeError::UndefinedVariable { span, .. } => *span,
            RuntimeError::TypeError { span, .. } => *span,
            RuntimeError::DivisionByZero { span } => *span,
            RuntimeError::IndexOutOfBounds { span, .. } => *span,
            RuntimeError::InvalidKey { span, .. } => *span,
            RuntimeError::NotCallable { span, .. } => *span,
            RuntimeError::ArityMismatch { span, .. } => *span,
            RuntimeError::UndefinedField { span, .. } => *span,
            RuntimeError::BreakOutsideLoop { span } => *span,
            RuntimeError::ContinueOutsideLoop { span } => *span,
            RuntimeError::ReturnOutsideFunction { span } => *span,
            RuntimeError::AssertionFailed { span, .. } => *span,
            RuntimeError::General { span, .. } => *span,
        }
    }
}

/// Result type for interpreter operations.
pub type Result<T> = std::result::Result<T, RuntimeError>;

/// Control flow signals for break, continue, and return.
#[derive(Debug, Clone)]
pub enum ControlFlow {
    Break,
    Continue,
    Return(Value),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_display() {
        assert_eq!(format!("{}", Value::Nil), "nil");
        assert_eq!(format!("{}", Value::Bool(true)), "true");
        assert_eq!(format!("{}", Value::Int(42)), "42");
        assert_eq!(format!("{}", Value::Float(3.14)), "3.14");
        assert_eq!(format!("{}", Value::String("hello".into())), "\"hello\"");
    }

    #[test]
    fn test_value_equality() {
        assert_eq!(Value::Int(42), Value::Int(42));
        assert_ne!(Value::Int(42), Value::Int(43));
        assert_eq!(Value::Bool(true), Value::Bool(true));
        assert_eq!(Value::Nil, Value::Nil);
    }
}
