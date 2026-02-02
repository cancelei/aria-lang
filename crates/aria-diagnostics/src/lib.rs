//! Aria Diagnostics - Error message and diagnostic system for the Aria programming language.
//!
//! This crate provides a comprehensive diagnostic system implementing the
//! error message design from ARIA-PD-014. It includes:
//!
//! - `Diagnostic` - The core diagnostic type with code, severity, message, and spans
//! - `DiagnosticSeverity` - Error, Warning, Note, and Help levels
//! - `ErrorCode` - Registry of standardized error codes
//! - Source spans for tracking error locations
//! - Fix suggestions with applicability levels
//! - Terminal rendering with color support
//!
//! # Example
//!
//! ```rust
//! use aria_diagnostics::{Diagnostic, DiagnosticSeverity};
//! use aria_diagnostics::span::SourceSpan;
//! use aria_diagnostics::suggestion::{Suggestion, SuggestionEdit};
//!
//! // Create a type mismatch error
//! let span = SourceSpan::new("src/main.aria", 20, 22);
//! let diagnostic = Diagnostic::error("E0001", "type mismatch")
//!     .with_primary_span(span.clone(), "expected `String`, found `Int`")
//!     .with_suggestion(
//!         Suggestion::machine_applicable("convert to string")
//!             .with_edit(SuggestionEdit::from_span(&span, "42.to_string()"))
//!     );
//!
//! assert_eq!(diagnostic.severity, DiagnosticSeverity::Error);
//! assert_eq!(diagnostic.code, Some("E0001".to_string()));
//! ```

pub mod render;
pub mod span;
pub mod suggestion;

use span::{MultiSpan, SourceSpan};
use suggestion::Suggestion;
use thiserror::Error;

/// The severity level of a diagnostic.
///
/// This follows the four-level system from ARIA-PD-014:
/// - Error: Blocks compilation
/// - Warning: Compilation continues
/// - Note: Informational, attached to errors
/// - Help: Suggestion for fixing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DiagnosticSeverity {
    /// An error that prevents compilation.
    #[default]
    Error,
    /// A warning that doesn't prevent compilation.
    Warning,
    /// Informational note, usually attached to another diagnostic.
    Note,
    /// A suggestion for fixing an issue.
    Help,
}

impl DiagnosticSeverity {
    /// Returns the text prefix for this severity level.
    pub fn prefix(&self) -> &'static str {
        match self {
            DiagnosticSeverity::Error => "error",
            DiagnosticSeverity::Warning => "warning",
            DiagnosticSeverity::Note => "note",
            DiagnosticSeverity::Help => "help",
        }
    }

    /// Returns the underline character for this severity level.
    pub fn underline_char(&self) -> char {
        match self {
            DiagnosticSeverity::Error => '^',
            DiagnosticSeverity::Warning => '~',
            DiagnosticSeverity::Note => '-',
            DiagnosticSeverity::Help => '+',
        }
    }

    /// Returns true if this severity level blocks compilation.
    pub fn blocks_compilation(&self) -> bool {
        matches!(self, DiagnosticSeverity::Error)
    }

    /// Converts to LSP severity (1=Error, 2=Warning, 3=Info, 4=Hint).
    pub fn to_lsp_severity(&self) -> u8 {
        match self {
            DiagnosticSeverity::Error => 1,
            DiagnosticSeverity::Warning => 2,
            DiagnosticSeverity::Note => 3,
            DiagnosticSeverity::Help => 4,
        }
    }
}

/// A compiler diagnostic (error, warning, note, or help message).
///
/// Diagnostics are the primary way the compiler communicates issues
/// to users. They contain a severity level, optional error code,
/// message, source locations, and fix suggestions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// The error code (e.g., "E0001").
    pub code: Option<String>,
    /// The severity level.
    pub severity: DiagnosticSeverity,
    /// The main error message.
    pub message: String,
    /// Source locations related to this diagnostic.
    pub spans: MultiSpan,
    /// Suggested fixes for this diagnostic.
    pub suggestions: Vec<Suggestion>,
    /// Child diagnostics (notes, helps attached to this diagnostic).
    pub children: Vec<Diagnostic>,
    /// Whether this is a root cause or a cascading error.
    pub is_root_cause: bool,
    /// IDs of related errors (for cascade tracking).
    pub related_error_ids: Vec<String>,
}

impl Diagnostic {
    /// Creates a new diagnostic with the given severity, code, and message.
    pub fn new(
        severity: DiagnosticSeverity,
        code: impl Into<Option<String>>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into().map(|s| s.to_string()),
            severity,
            message: message.into(),
            spans: MultiSpan::new(),
            suggestions: Vec::new(),
            children: Vec::new(),
            is_root_cause: true,
            related_error_ids: Vec::new(),
        }
    }

    /// Creates an error diagnostic.
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(DiagnosticSeverity::Error, Some(code.into()), message)
    }

    /// Creates a warning diagnostic.
    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(DiagnosticSeverity::Warning, Some(code.into()), message)
    }

    /// Creates a note diagnostic (usually attached to another diagnostic).
    pub fn note(message: impl Into<String>) -> Self {
        Self::new(DiagnosticSeverity::Note, None::<String>, message)
    }

    /// Creates a help diagnostic.
    pub fn help(message: impl Into<String>) -> Self {
        Self::new(DiagnosticSeverity::Help, None::<String>, message)
    }

    /// Sets the spans for this diagnostic.
    pub fn with_spans(mut self, spans: MultiSpan) -> Self {
        self.spans = spans;
        self
    }

    /// Adds a primary span with a label message.
    pub fn with_primary_span(mut self, span: SourceSpan, message: impl Into<String>) -> Self {
        self.spans.push_primary(span, message);
        self
    }

    /// Adds a secondary span with a label message.
    pub fn with_secondary_span(mut self, span: SourceSpan, message: impl Into<String>) -> Self {
        self.spans.push_secondary(span, message);
        self
    }

    /// Adds a suggestion to this diagnostic.
    pub fn with_suggestion(mut self, suggestion: Suggestion) -> Self {
        self.suggestions.push(suggestion);
        self
    }

    /// Adds multiple suggestions to this diagnostic.
    pub fn with_suggestions(mut self, suggestions: impl IntoIterator<Item = Suggestion>) -> Self {
        self.suggestions.extend(suggestions);
        self
    }

    /// Adds a child diagnostic (note or help).
    pub fn with_child(mut self, child: Diagnostic) -> Self {
        self.children.push(child);
        self
    }

    /// Marks this as a cascading (non-root-cause) error.
    pub fn as_cascade(mut self) -> Self {
        self.is_root_cause = false;
        self
    }

    /// Adds a related error ID for cascade tracking.
    pub fn with_related(mut self, error_id: impl Into<String>) -> Self {
        self.related_error_ids.push(error_id.into());
        self
    }

    /// Returns the documentation URL for this diagnostic.
    pub fn docs_url(&self, base_url: &str) -> Option<String> {
        self.code.as_ref().map(|code| format!("{}/{}", base_url, code))
    }

    /// Returns true if this diagnostic has any spans.
    pub fn has_spans(&self) -> bool {
        !self.spans.is_empty()
    }

    /// Returns true if this diagnostic has any suggestions.
    pub fn has_suggestions(&self) -> bool {
        !self.suggestions.is_empty()
    }

    /// Returns true if any suggestion can be auto-applied.
    pub fn has_auto_applicable_suggestions(&self) -> bool {
        self.suggestions.iter().any(|s| s.can_auto_apply())
    }
}

/// Error categories for the error code registry.
///
/// Error codes follow the pattern EXXXX where the first digit indicates
/// the category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCategory {
    /// E0XXX: Core/General errors (type mismatch, etc.)
    Core,
    /// E1XXX: Naming/Scope errors (undefined variable, etc.)
    Naming,
    /// E2XXX: Ownership/Borrowing errors
    Ownership,
    /// E3XXX: Syntax/Parsing errors
    Syntax,
    /// E4XXX: Pattern matching errors
    Pattern,
    /// E5XXX: Effect system errors
    Effects,
    /// E6XXX: Concurrency errors
    Concurrency,
    /// E7XXX: FFI/Interop errors
    Ffi,
    /// E8XXX: Contract errors
    Contracts,
    /// E9XXX: Internal compiler errors
    Internal,
}

impl ErrorCategory {
    /// Returns the numeric prefix for this category.
    pub fn prefix(&self) -> u16 {
        match self {
            ErrorCategory::Core => 0,
            ErrorCategory::Naming => 1,
            ErrorCategory::Ownership => 2,
            ErrorCategory::Syntax => 3,
            ErrorCategory::Pattern => 4,
            ErrorCategory::Effects => 5,
            ErrorCategory::Concurrency => 6,
            ErrorCategory::Ffi => 7,
            ErrorCategory::Contracts => 8,
            ErrorCategory::Internal => 9,
        }
    }

    /// Creates a category from an error code.
    pub fn from_code(code: &str) -> Option<Self> {
        if !code.starts_with('E') || code.len() < 2 {
            return None;
        }
        match code.chars().nth(1)? {
            '0' => Some(ErrorCategory::Core),
            '1' => Some(ErrorCategory::Naming),
            '2' => Some(ErrorCategory::Ownership),
            '3' => Some(ErrorCategory::Syntax),
            '4' => Some(ErrorCategory::Pattern),
            '5' => Some(ErrorCategory::Effects),
            '6' => Some(ErrorCategory::Concurrency),
            '7' => Some(ErrorCategory::Ffi),
            '8' => Some(ErrorCategory::Contracts),
            '9' => Some(ErrorCategory::Internal),
            _ => None,
        }
    }

    /// Returns a human-readable name for this category.
    pub fn name(&self) -> &'static str {
        match self {
            ErrorCategory::Core => "Core",
            ErrorCategory::Naming => "Naming/Scope",
            ErrorCategory::Ownership => "Ownership/Borrowing",
            ErrorCategory::Syntax => "Syntax/Parsing",
            ErrorCategory::Pattern => "Pattern Matching",
            ErrorCategory::Effects => "Effects",
            ErrorCategory::Concurrency => "Concurrency",
            ErrorCategory::Ffi => "FFI/Interop",
            ErrorCategory::Contracts => "Contracts",
            ErrorCategory::Internal => "Internal",
        }
    }
}

/// Information about a registered error code.
#[derive(Debug, Clone)]
pub struct ErrorCodeInfo {
    /// The error code (e.g., "E0001").
    pub code: String,
    /// The category this error belongs to.
    pub category: ErrorCategory,
    /// A brief description of this error.
    pub description: String,
    /// Whether this error is deprecated.
    pub deprecated: bool,
}

impl ErrorCodeInfo {
    /// Creates a new error code info.
    pub fn new(code: impl Into<String>, description: impl Into<String>) -> Option<Self> {
        let code = code.into();
        let category = ErrorCategory::from_code(&code)?;
        Some(Self {
            code,
            category,
            description: description.into(),
            deprecated: false,
        })
    }

    /// Marks this error code as deprecated.
    pub fn deprecated(mut self) -> Self {
        self.deprecated = true;
        self
    }
}

/// Registry of all known error codes.
///
/// This provides a centralized place to define and look up error codes,
/// their descriptions, and documentation.
#[derive(Debug, Default)]
pub struct ErrorCodeRegistry {
    codes: std::collections::HashMap<String, ErrorCodeInfo>,
}

impl ErrorCodeRegistry {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a registry with the standard Aria error codes.
    pub fn with_standard_codes() -> Self {
        let mut registry = Self::new();

        // E0XXX: Core errors
        registry.register("E0001", "type mismatch");

        // E1XXX: Naming errors
        registry.register("E1001", "undefined variable");
        registry.register("E1002", "name not in scope");
        registry.register("E1003", "method not found");

        // E2XXX: Ownership errors
        registry.register("E2001", "use after move");

        // E3XXX: Syntax errors
        registry.register("E3001", "unexpected token");

        // E4XXX: Pattern matching errors
        registry.register("E4001", "non-exhaustive patterns");

        // E5XXX: Effect errors
        registry.register("E5001", "unhandled effect");

        // E6XXX: Concurrency errors
        registry.register("E6001", "data race detected");

        // E7XXX: FFI errors
        registry.register("E7001", "invalid C type");

        // E8XXX: Contract errors
        registry.register("E8001", "precondition violated");

        // E9XXX: Internal errors
        registry.register("E9001", "internal compiler error");

        registry
    }

    /// Registers a new error code.
    pub fn register(&mut self, code: impl Into<String>, description: impl Into<String>) -> bool {
        let code = code.into();
        if let Some(info) = ErrorCodeInfo::new(code.clone(), description) {
            self.codes.insert(code, info);
            true
        } else {
            false
        }
    }

    /// Looks up an error code.
    pub fn get(&self, code: &str) -> Option<&ErrorCodeInfo> {
        self.codes.get(code)
    }

    /// Returns all registered error codes.
    pub fn all_codes(&self) -> impl Iterator<Item = &ErrorCodeInfo> {
        self.codes.values()
    }

    /// Returns all error codes in a category.
    pub fn codes_in_category(&self, category: ErrorCategory) -> impl Iterator<Item = &ErrorCodeInfo> {
        self.codes.values().filter(move |info| info.category == category)
    }
}

/// Result type for diagnostic operations.
pub type DiagnosticResult<T> = Result<T, DiagnosticError>;

/// Errors that can occur during diagnostic operations.
#[derive(Debug, Error)]
pub enum DiagnosticError {
    /// An I/O error occurred.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A source file could not be found.
    #[error("source file not found: {0}")]
    SourceNotFound(String),

    /// An invalid error code was used.
    #[error("invalid error code: {0}")]
    InvalidErrorCode(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_severity() {
        assert_eq!(DiagnosticSeverity::Error.prefix(), "error");
        assert_eq!(DiagnosticSeverity::Warning.prefix(), "warning");
        assert!(DiagnosticSeverity::Error.blocks_compilation());
        assert!(!DiagnosticSeverity::Warning.blocks_compilation());
    }

    #[test]
    fn test_diagnostic_creation() {
        let diag = Diagnostic::error("E0001", "type mismatch");
        assert_eq!(diag.severity, DiagnosticSeverity::Error);
        assert_eq!(diag.code, Some("E0001".to_string()));
        assert_eq!(diag.message, "type mismatch");
        assert!(diag.is_root_cause);
    }

    #[test]
    fn test_diagnostic_with_spans() {
        let span = SourceSpan::new("test.aria", 10, 20);
        let diag = Diagnostic::error("E0001", "type mismatch")
            .with_primary_span(span, "expected `Int`, found `String`");

        assert!(diag.has_spans());
        assert!(!diag.has_suggestions());
    }

    #[test]
    fn test_error_category() {
        assert_eq!(ErrorCategory::from_code("E0001"), Some(ErrorCategory::Core));
        assert_eq!(ErrorCategory::from_code("E1001"), Some(ErrorCategory::Naming));
        assert_eq!(ErrorCategory::from_code("E9001"), Some(ErrorCategory::Internal));
        assert_eq!(ErrorCategory::from_code("invalid"), None);
    }

    #[test]
    fn test_error_registry() {
        let registry = ErrorCodeRegistry::with_standard_codes();

        let e0001 = registry.get("E0001");
        assert!(e0001.is_some());
        assert_eq!(e0001.unwrap().description, "type mismatch");

        let e1001 = registry.get("E1001");
        assert!(e1001.is_some());
        assert_eq!(e1001.unwrap().category, ErrorCategory::Naming);
    }

    #[test]
    fn test_docs_url() {
        let diag = Diagnostic::error("E0001", "type mismatch");
        let url = diag.docs_url("https://aria-lang.org/errors");
        assert_eq!(url, Some("https://aria-lang.org/errors/E0001".to_string()));
    }

    #[test]
    fn test_diagnostic_builder_pattern() {
        let span1 = SourceSpan::new("test.aria", 10, 20);
        let span2 = SourceSpan::new("test.aria", 50, 60);

        let diag = Diagnostic::error("E0001", "type mismatch")
            .with_primary_span(span1, "expected `String`, found `Int`")
            .with_secondary_span(span2, "expected due to this annotation")
            .with_suggestion(Suggestion::machine_applicable("convert to string"))
            .with_child(Diagnostic::note("types must match"));

        assert!(diag.has_spans());
        assert!(diag.has_suggestions());
        assert_eq!(diag.children.len(), 1);
    }
}
