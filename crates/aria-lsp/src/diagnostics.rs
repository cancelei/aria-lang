//! Diagnostic Reporting Infrastructure
//!
//! This module provides the infrastructure for reporting diagnostics (errors,
//! warnings, hints) from the Aria compiler to IDE clients via LSP.
//!
//! # Architecture
//!
//! ```text
//! Compiler Errors/Warnings
//!         |
//!         v
//!   AriaDignostic (internal representation)
//!         |
//!         v
//!   DiagnosticConverter (converts to LSP format)
//!         |
//!         v
//!   LSP Diagnostic (sent to client)
//! ```
//!
//! # Aria-Specific Features
//!
//! - Effect inference errors with suggested handlers
//! - Contract violation diagnostics
//! - Ownership/borrowing errors with hints
//! - Type mismatch with detailed explanations

use std::collections::HashMap;
use std::sync::Arc;
use tower_lsp::lsp_types::{self, Url};

use crate::types::{Span, TextRange, LineIndex};

/// Diagnostic severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    /// An error that prevents compilation.
    Error,
    /// A warning that doesn't prevent compilation.
    Warning,
    /// Informational message.
    Information,
    /// A hint for improving code.
    Hint,
}

impl From<Severity> for lsp_types::DiagnosticSeverity {
    fn from(severity: Severity) -> Self {
        match severity {
            Severity::Error => lsp_types::DiagnosticSeverity::ERROR,
            Severity::Warning => lsp_types::DiagnosticSeverity::WARNING,
            Severity::Information => lsp_types::DiagnosticSeverity::INFORMATION,
            Severity::Hint => lsp_types::DiagnosticSeverity::HINT,
        }
    }
}

/// Diagnostic tags for additional categorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticTag {
    /// The code is unused or unnecessary.
    Unnecessary,
    /// The code is deprecated.
    Deprecated,
}

impl From<DiagnosticTag> for lsp_types::DiagnosticTag {
    fn from(tag: DiagnosticTag) -> Self {
        match tag {
            DiagnosticTag::Unnecessary => lsp_types::DiagnosticTag::UNNECESSARY,
            DiagnosticTag::Deprecated => lsp_types::DiagnosticTag::DEPRECATED,
        }
    }
}

/// The source of a diagnostic (which part of the compiler produced it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticSource {
    /// Lexer/scanner errors.
    Lexer,
    /// Parser errors.
    Parser,
    /// Name resolution errors.
    NameResolution,
    /// Type checking errors.
    TypeChecker,
    /// Effect inference errors.
    EffectChecker,
    /// Contract verification errors.
    ContractChecker,
    /// Borrow checking errors.
    BorrowChecker,
    /// General compiler errors.
    Compiler,
}

impl DiagnosticSource {
    /// Returns the source as a string for LSP.
    pub fn as_str(&self) -> &'static str {
        match self {
            DiagnosticSource::Lexer => "aria-lexer",
            DiagnosticSource::Parser => "aria-parser",
            DiagnosticSource::NameResolution => "aria-names",
            DiagnosticSource::TypeChecker => "aria-types",
            DiagnosticSource::EffectChecker => "aria-effects",
            DiagnosticSource::ContractChecker => "aria-contracts",
            DiagnosticSource::BorrowChecker => "aria-borrow",
            DiagnosticSource::Compiler => "aria",
        }
    }
}

/// A related location for a diagnostic.
///
/// Used to show additional relevant locations (e.g., "defined here",
/// "first occurrence", etc.).
#[derive(Debug, Clone)]
pub struct RelatedInformation {
    /// The location of the related information.
    pub uri: Url,
    /// The range within the document.
    pub range: TextRange,
    /// A message describing the relationship.
    pub message: String,
}

impl RelatedInformation {
    /// Creates new related information.
    pub fn new(uri: Url, range: TextRange, message: impl Into<String>) -> Self {
        Self {
            uri,
            range,
            message: message.into(),
        }
    }
}

/// An Aria-specific diagnostic.
///
/// This is the internal representation of diagnostics that can be converted
/// to LSP format for sending to clients.
#[derive(Debug, Clone)]
pub struct AriaDiagnostic {
    /// The main message.
    pub message: String,
    /// The primary span of the diagnostic.
    pub span: Span,
    /// The severity level.
    pub severity: Severity,
    /// The source of the diagnostic.
    pub source: DiagnosticSource,
    /// An optional error code.
    pub code: Option<String>,
    /// Related information (additional locations).
    pub related: Vec<RelatedInformation>,
    /// Tags for additional categorization.
    pub tags: Vec<DiagnosticTag>,
    /// Optional fix suggestion.
    pub fix_suggestion: Option<FixSuggestion>,
}

impl AriaDiagnostic {
    /// Creates a new error diagnostic.
    pub fn error(message: impl Into<String>, span: Span, source: DiagnosticSource) -> Self {
        Self {
            message: message.into(),
            span,
            severity: Severity::Error,
            source,
            code: None,
            related: Vec::new(),
            tags: Vec::new(),
            fix_suggestion: None,
        }
    }

    /// Creates a new warning diagnostic.
    pub fn warning(message: impl Into<String>, span: Span, source: DiagnosticSource) -> Self {
        Self {
            message: message.into(),
            span,
            severity: Severity::Warning,
            source,
            code: None,
            related: Vec::new(),
            tags: Vec::new(),
            fix_suggestion: None,
        }
    }

    /// Creates a new hint diagnostic.
    pub fn hint(message: impl Into<String>, span: Span, source: DiagnosticSource) -> Self {
        Self {
            message: message.into(),
            span,
            severity: Severity::Hint,
            source,
            code: None,
            related: Vec::new(),
            tags: Vec::new(),
            fix_suggestion: None,
        }
    }

    /// Creates a new informational diagnostic.
    pub fn info(message: impl Into<String>, span: Span, source: DiagnosticSource) -> Self {
        Self {
            message: message.into(),
            span,
            severity: Severity::Information,
            source,
            code: None,
            related: Vec::new(),
            tags: Vec::new(),
            fix_suggestion: None,
        }
    }

    /// Sets the error code.
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Adds related information.
    pub fn with_related(mut self, info: RelatedInformation) -> Self {
        self.related.push(info);
        self
    }

    /// Adds a diagnostic tag.
    pub fn with_tag(mut self, tag: DiagnosticTag) -> Self {
        self.tags.push(tag);
        self
    }

    /// Adds a fix suggestion.
    pub fn with_fix(mut self, fix: FixSuggestion) -> Self {
        self.fix_suggestion = Some(fix);
        self
    }

    /// Marks the diagnostic as deprecated.
    pub fn deprecated(self) -> Self {
        self.with_tag(DiagnosticTag::Deprecated)
    }

    /// Marks the diagnostic as unnecessary.
    pub fn unnecessary(self) -> Self {
        self.with_tag(DiagnosticTag::Unnecessary)
    }
}

/// A suggested fix for a diagnostic.
#[derive(Debug, Clone)]
pub struct FixSuggestion {
    /// A short description of the fix.
    pub message: String,
    /// The text edits to apply.
    pub edits: Vec<FixEdit>,
}

impl FixSuggestion {
    /// Creates a new fix suggestion.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            edits: Vec::new(),
        }
    }

    /// Adds an edit to the fix.
    pub fn with_edit(mut self, span: Span, new_text: impl Into<String>) -> Self {
        self.edits.push(FixEdit {
            span,
            new_text: new_text.into(),
        });
        self
    }
}

/// A text edit for a fix suggestion.
#[derive(Debug, Clone)]
pub struct FixEdit {
    /// The span to replace.
    pub span: Span,
    /// The new text.
    pub new_text: String,
}

/// Converts Aria diagnostics to LSP diagnostics.
pub struct DiagnosticConverter {
    /// Line index for the document.
    line_index: LineIndex,
    /// The document URI.
    uri: Url,
}

impl DiagnosticConverter {
    /// Creates a new converter for the given document.
    pub fn new(uri: Url, text: &str) -> Self {
        Self {
            line_index: LineIndex::new(text),
            uri,
        }
    }

    /// Converts an Aria diagnostic to an LSP diagnostic.
    pub fn convert(&self, diag: &AriaDiagnostic) -> lsp_types::Diagnostic {
        let range = self.line_index.span_to_range(diag.span);

        let related_information = if diag.related.is_empty() {
            None
        } else {
            Some(
                diag.related
                    .iter()
                    .map(|r| lsp_types::DiagnosticRelatedInformation {
                        location: lsp_types::Location {
                            uri: r.uri.clone(),
                            range: r.range.into(),
                        },
                        message: r.message.clone(),
                    })
                    .collect(),
            )
        };

        let tags = if diag.tags.is_empty() {
            None
        } else {
            Some(diag.tags.iter().map(|t| (*t).into()).collect())
        };

        lsp_types::Diagnostic {
            range: range.into(),
            severity: Some(diag.severity.into()),
            code: diag.code.clone().map(lsp_types::NumberOrString::String),
            code_description: None,
            source: Some(diag.source.as_str().to_string()),
            message: diag.message.clone(),
            related_information,
            tags,
            data: None,
        }
    }

    /// Converts multiple diagnostics.
    pub fn convert_all(&self, diagnostics: &[AriaDiagnostic]) -> Vec<lsp_types::Diagnostic> {
        diagnostics.iter().map(|d| self.convert(d)).collect()
    }
}

/// A collection of diagnostics for multiple documents.
///
/// This is used to batch diagnostics and publish them efficiently.
#[derive(Debug, Default)]
pub struct DiagnosticCollection {
    /// Diagnostics keyed by document URI.
    diagnostics: HashMap<Url, Vec<AriaDiagnostic>>,
    /// Document contents for conversion.
    documents: HashMap<Url, Arc<String>>,
}

impl DiagnosticCollection {
    /// Creates a new empty collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a document with its content.
    pub fn register_document(&mut self, uri: Url, content: Arc<String>) {
        self.documents.insert(uri, content);
    }

    /// Adds a diagnostic for a document.
    pub fn add(&mut self, uri: Url, diagnostic: AriaDiagnostic) {
        self.diagnostics.entry(uri).or_default().push(diagnostic);
    }

    /// Clears all diagnostics for a document.
    pub fn clear(&mut self, uri: &Url) {
        self.diagnostics.remove(uri);
    }

    /// Clears all diagnostics.
    pub fn clear_all(&mut self) {
        self.diagnostics.clear();
    }

    /// Gets diagnostics for a document.
    pub fn get(&self, uri: &Url) -> Option<&[AriaDiagnostic]> {
        self.diagnostics.get(uri).map(|v| v.as_slice())
    }

    /// Gets all document URIs with diagnostics.
    pub fn uris(&self) -> impl Iterator<Item = &Url> {
        self.diagnostics.keys()
    }

    /// Converts diagnostics for a document to LSP format.
    pub fn to_lsp(&self, uri: &Url) -> Option<Vec<lsp_types::Diagnostic>> {
        let diagnostics = self.diagnostics.get(uri)?;
        let content = self.documents.get(uri)?;
        let converter = DiagnosticConverter::new(uri.clone(), content);
        Some(converter.convert_all(diagnostics))
    }

    /// Returns the total count of diagnostics.
    pub fn total_count(&self) -> usize {
        self.diagnostics.values().map(|v| v.len()).sum()
    }

    /// Returns the count of diagnostics for a document.
    pub fn count(&self, uri: &Url) -> usize {
        self.diagnostics.get(uri).map(|v| v.len()).unwrap_or(0)
    }

    /// Returns counts by severity.
    pub fn counts_by_severity(&self) -> HashMap<Severity, usize> {
        let mut counts = HashMap::new();
        for diagnostics in self.diagnostics.values() {
            for diag in diagnostics {
                *counts.entry(diag.severity).or_insert(0) += 1;
            }
        }
        counts
    }
}

/// Builder for creating common diagnostic patterns.
pub struct DiagnosticBuilder {
    source: DiagnosticSource,
}

impl DiagnosticBuilder {
    /// Creates a new builder for the given source.
    pub fn new(source: DiagnosticSource) -> Self {
        Self { source }
    }

    /// Creates an "undefined name" error.
    pub fn undefined_name(&self, name: &str, span: Span) -> AriaDiagnostic {
        AriaDiagnostic::error(
            format!("undefined name `{}`", name),
            span,
            self.source,
        )
        .with_code("E0001")
    }

    /// Creates a "type mismatch" error.
    pub fn type_mismatch(
        &self,
        expected: &str,
        found: &str,
        span: Span,
    ) -> AriaDiagnostic {
        AriaDiagnostic::error(
            format!("type mismatch: expected `{}`, found `{}`", expected, found),
            span,
            self.source,
        )
        .with_code("E0002")
    }

    /// Creates a "missing effect handler" error.
    pub fn missing_handler(&self, effect: &str, span: Span) -> AriaDiagnostic {
        AriaDiagnostic::error(
            format!("effect `{}` is not handled in this scope", effect),
            span,
            self.source,
        )
        .with_code("E0003")
    }

    /// Creates a "contract violation" error.
    pub fn contract_violation(
        &self,
        contract_type: &str,
        condition: &str,
        span: Span,
    ) -> AriaDiagnostic {
        AriaDiagnostic::error(
            format!("{} contract violated: {}", contract_type, condition),
            span,
            self.source,
        )
        .with_code("E0004")
    }

    /// Creates an "unused variable" warning.
    pub fn unused_variable(&self, name: &str, span: Span) -> AriaDiagnostic {
        AriaDiagnostic::warning(
            format!("unused variable: `{}`", name),
            span,
            self.source,
        )
        .with_code("W0001")
        .unnecessary()
    }

    /// Creates an "unreachable code" warning.
    pub fn unreachable_code(&self, span: Span) -> AriaDiagnostic {
        AriaDiagnostic::warning("unreachable code", span, self.source)
            .with_code("W0002")
            .unnecessary()
    }

    /// Creates a "deprecated" warning.
    pub fn deprecated(&self, name: &str, span: Span, replacement: Option<&str>) -> AriaDiagnostic {
        let message = match replacement {
            Some(repl) => format!("`{}` is deprecated, use `{}` instead", name, repl),
            None => format!("`{}` is deprecated", name),
        };
        AriaDiagnostic::warning(message, span, self.source)
            .with_code("W0003")
            .deprecated()
    }

    /// Creates a "syntax error" error.
    pub fn syntax_error(&self, message: &str, span: Span) -> AriaDiagnostic {
        AriaDiagnostic::error(message, span, self.source).with_code("E0100")
    }

    /// Creates an "unexpected token" error.
    pub fn unexpected_token(
        &self,
        expected: &str,
        found: &str,
        span: Span,
    ) -> AriaDiagnostic {
        AriaDiagnostic::error(
            format!("expected {}, found `{}`", expected, found),
            span,
            self.source,
        )
        .with_code("E0101")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_conversion() {
        assert_eq!(
            lsp_types::DiagnosticSeverity::from(Severity::Error),
            lsp_types::DiagnosticSeverity::ERROR
        );
        assert_eq!(
            lsp_types::DiagnosticSeverity::from(Severity::Warning),
            lsp_types::DiagnosticSeverity::WARNING
        );
    }

    #[test]
    fn test_diagnostic_builder() {
        let builder = DiagnosticBuilder::new(DiagnosticSource::TypeChecker);

        let diag = builder.undefined_name("foo", Span::new(0, 3));
        assert_eq!(diag.severity, Severity::Error);
        assert!(diag.message.contains("foo"));
        assert_eq!(diag.code, Some("E0001".to_string()));
    }

    #[test]
    fn test_diagnostic_with_related() {
        let uri = Url::parse("file:///test.aria").unwrap();
        let diag = AriaDiagnostic::error("duplicate definition", Span::new(10, 15), DiagnosticSource::NameResolution)
            .with_related(RelatedInformation::new(
                uri,
                TextRange::new(
                    crate::types::Position::new(0, 0),
                    crate::types::Position::new(0, 5),
                ),
                "first defined here",
            ));

        assert_eq!(diag.related.len(), 1);
        assert_eq!(diag.related[0].message, "first defined here");
    }

    #[test]
    fn test_diagnostic_converter() {
        let uri = Url::parse("file:///test.aria").unwrap();
        let text = "let x = 42;\nlet y = x + 1;";
        let converter = DiagnosticConverter::new(uri, text);

        let diag = AriaDiagnostic::error("test error", Span::new(4, 5), DiagnosticSource::TypeChecker);
        let lsp_diag = converter.convert(&diag);

        assert_eq!(lsp_diag.message, "test error");
        assert_eq!(lsp_diag.severity, Some(lsp_types::DiagnosticSeverity::ERROR));
        assert_eq!(lsp_diag.range.start.line, 0);
        assert_eq!(lsp_diag.range.start.character, 4);
    }

    #[test]
    fn test_diagnostic_collection() {
        let mut collection = DiagnosticCollection::new();
        let uri = Url::parse("file:///test.aria").unwrap();
        let content = Arc::new("let x = 42;".to_string());

        collection.register_document(uri.clone(), content);
        collection.add(
            uri.clone(),
            AriaDiagnostic::error("error 1", Span::new(0, 3), DiagnosticSource::Parser),
        );
        collection.add(
            uri.clone(),
            AriaDiagnostic::warning("warning 1", Span::new(4, 5), DiagnosticSource::TypeChecker),
        );

        assert_eq!(collection.count(&uri), 2);
        assert_eq!(collection.total_count(), 2);

        let counts = collection.counts_by_severity();
        assert_eq!(counts.get(&Severity::Error), Some(&1));
        assert_eq!(counts.get(&Severity::Warning), Some(&1));

        let lsp_diags = collection.to_lsp(&uri).unwrap();
        assert_eq!(lsp_diags.len(), 2);
    }

    #[test]
    fn test_fix_suggestion() {
        let fix = FixSuggestion::new("Add missing semicolon")
            .with_edit(Span::new(10, 10), ";");

        assert_eq!(fix.message, "Add missing semicolon");
        assert_eq!(fix.edits.len(), 1);
        assert_eq!(fix.edits[0].new_text, ";");
    }

    #[test]
    fn test_diagnostic_tags() {
        let diag = AriaDiagnostic::warning("unused", Span::new(0, 5), DiagnosticSource::Compiler)
            .unnecessary()
            .deprecated();

        assert_eq!(diag.tags.len(), 2);
        assert!(diag.tags.contains(&DiagnosticTag::Unnecessary));
        assert!(diag.tags.contains(&DiagnosticTag::Deprecated));
    }

    #[test]
    fn test_type_mismatch_diagnostic() {
        let builder = DiagnosticBuilder::new(DiagnosticSource::TypeChecker);
        let diag = builder.type_mismatch("Int", "String", Span::new(10, 20));

        assert!(diag.message.contains("Int"));
        assert!(diag.message.contains("String"));
        assert_eq!(diag.code, Some("E0002".to_string()));
    }

    #[test]
    fn test_missing_handler_diagnostic() {
        let builder = DiagnosticBuilder::new(DiagnosticSource::EffectChecker);
        let diag = builder.missing_handler("Console", Span::new(5, 15));

        assert!(diag.message.contains("Console"));
        assert!(diag.message.contains("not handled"));
    }
}
