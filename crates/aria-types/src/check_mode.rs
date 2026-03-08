//! Bidirectional type checking infrastructure.
//!
//! Defines `CheckMode` for synthesis vs checking mode,
//! and `TypeSource` for enhanced error messages.

use crate::Type;
use aria_ast::Span;

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
