//! Fix suggestions for diagnostics.
//!
//! This module provides types for representing suggested fixes to errors,
//! including the applicability level (how confident we are in the suggestion)
//! and concrete text edits that can be applied.

use crate::span::SourceSpan;
use std::path::PathBuf;

/// How applicable/confident a suggestion is.
///
/// This follows the four-tier system from ARIA-PD-014:
/// - MachineApplicable: 100% confident, can be auto-applied
/// - HasPlaceholders: >90% confident, but needs user input
/// - MaybeIncorrect: 60-90% confident, use "Consider..." phrasing
/// - Educational: <60% confident, points to documentation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Applicability {
    /// The suggestion is definitely correct and can be automatically applied.
    /// Used for simple, unambiguous fixes like typo corrections.
    /// Presented as direct suggestions, eligible for `aria fix`.
    MachineApplicable,

    /// The suggestion is likely correct but contains placeholders that
    /// need to be filled in by the user. Shown with `<...>` markers.
    /// Example: "add the missing field: `<age: Int>`"
    HasPlaceholders,

    /// The suggestion might be correct but could also be wrong.
    /// Presented with softer language like "Consider..." or "You might want to...".
    /// Example: "consider cloning the value if you need to use it again"
    #[default]
    MaybeIncorrect,

    /// The suggestion is educational - pointing to documentation or concepts
    /// rather than providing a concrete fix. Used when we're not confident
    /// enough to suggest specific code changes.
    /// Example: "see documentation on ownership and borrowing"
    Educational,
}

impl Applicability {
    /// Returns true if this suggestion can be automatically applied.
    pub fn is_machine_applicable(&self) -> bool {
        matches!(self, Applicability::MachineApplicable)
    }

    /// Returns the confidence level as a percentage range.
    pub fn confidence_range(&self) -> (u8, u8) {
        match self {
            Applicability::MachineApplicable => (100, 100),
            Applicability::HasPlaceholders => (90, 100),
            Applicability::MaybeIncorrect => (60, 90),
            Applicability::Educational => (0, 60),
        }
    }

    /// Returns an appropriate prefix for the help message.
    pub fn help_prefix(&self) -> &'static str {
        match self {
            Applicability::MachineApplicable => "",
            Applicability::HasPlaceholders => "",
            Applicability::MaybeIncorrect => "consider ",
            Applicability::Educational => "see documentation on ",
        }
    }
}

/// A concrete text edit representing a change to source code.
///
/// Edits are specified as a span (the text to replace) and the new text
/// to insert. To insert text without replacing, use an empty span.
/// To delete text, use an empty replacement string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuggestionEdit {
    /// The file containing the edit.
    pub file: PathBuf,
    /// The starting byte offset of the text to replace.
    pub start: usize,
    /// The ending byte offset of the text to replace.
    pub end: usize,
    /// The new text to insert.
    pub new_text: String,
}

impl SuggestionEdit {
    /// Creates a new suggestion edit.
    pub fn new(file: impl Into<PathBuf>, start: usize, end: usize, new_text: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            start,
            end,
            new_text: new_text.into(),
        }
    }

    /// Creates an edit from a source span.
    pub fn from_span(span: &SourceSpan, new_text: impl Into<String>) -> Self {
        Self {
            file: span.file.clone(),
            start: span.start,
            end: span.end,
            new_text: new_text.into(),
        }
    }

    /// Creates an insertion edit (no text replaced).
    pub fn insert(file: impl Into<PathBuf>, offset: usize, text: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            start: offset,
            end: offset,
            new_text: text.into(),
        }
    }

    /// Creates a deletion edit (text removed, nothing inserted).
    pub fn delete(file: impl Into<PathBuf>, start: usize, end: usize) -> Self {
        Self {
            file: file.into(),
            start,
            end,
            new_text: String::new(),
        }
    }

    /// Returns true if this edit is a pure insertion (no replacement).
    pub fn is_insertion(&self) -> bool {
        self.start == self.end && !self.new_text.is_empty()
    }

    /// Returns true if this edit is a pure deletion (no insertion).
    pub fn is_deletion(&self) -> bool {
        self.start < self.end && self.new_text.is_empty()
    }

    /// Returns the length of text being replaced.
    pub fn replaced_len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Converts this edit to a span.
    pub fn to_span(&self) -> SourceSpan {
        SourceSpan::new(self.file.clone(), self.start, self.end)
    }
}

/// A suggestion for fixing a diagnostic.
///
/// Suggestions can contain multiple edits (for multi-location fixes)
/// and have an applicability level indicating confidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    /// The message describing what this suggestion does.
    pub message: String,
    /// How applicable/confident this suggestion is.
    pub applicability: Applicability,
    /// The concrete edits that implement this suggestion.
    pub edits: Vec<SuggestionEdit>,
}

impl Suggestion {
    /// Creates a new suggestion with the given message and applicability.
    pub fn new(message: impl Into<String>, applicability: Applicability) -> Self {
        Self {
            message: message.into(),
            applicability,
            edits: Vec::new(),
        }
    }

    /// Creates a machine-applicable suggestion.
    pub fn machine_applicable(message: impl Into<String>) -> Self {
        Self::new(message, Applicability::MachineApplicable)
    }

    /// Creates a suggestion with placeholders.
    pub fn with_placeholders(message: impl Into<String>) -> Self {
        Self::new(message, Applicability::HasPlaceholders)
    }

    /// Creates a possibly incorrect suggestion.
    pub fn maybe_incorrect(message: impl Into<String>) -> Self {
        Self::new(message, Applicability::MaybeIncorrect)
    }

    /// Creates an educational suggestion.
    pub fn educational(message: impl Into<String>) -> Self {
        Self::new(message, Applicability::Educational)
    }

    /// Adds an edit to this suggestion.
    pub fn with_edit(mut self, edit: SuggestionEdit) -> Self {
        self.edits.push(edit);
        self
    }

    /// Adds multiple edits to this suggestion.
    pub fn with_edits(mut self, edits: impl IntoIterator<Item = SuggestionEdit>) -> Self {
        self.edits.extend(edits);
        self
    }

    /// Creates a simple single-edit suggestion.
    pub fn simple_replacement(
        message: impl Into<String>,
        span: &SourceSpan,
        new_text: impl Into<String>,
        applicability: Applicability,
    ) -> Self {
        Self {
            message: message.into(),
            applicability,
            edits: vec![SuggestionEdit::from_span(span, new_text)],
        }
    }

    /// Returns true if this suggestion can be automatically applied.
    pub fn can_auto_apply(&self) -> bool {
        self.applicability.is_machine_applicable() && !self.edits.is_empty()
    }

    /// Returns true if this suggestion has no concrete edits.
    pub fn is_educational_only(&self) -> bool {
        self.edits.is_empty()
    }

    /// Returns all unique files affected by this suggestion.
    pub fn affected_files(&self) -> Vec<&PathBuf> {
        let mut files: Vec<&PathBuf> = Vec::new();
        for edit in &self.edits {
            if !files.contains(&&edit.file) {
                files.push(&edit.file);
            }
        }
        files
    }

    /// Returns the full help message with appropriate prefix.
    pub fn full_message(&self) -> String {
        let prefix = self.applicability.help_prefix();
        if prefix.is_empty() {
            self.message.clone()
        } else {
            format!("{}{}", prefix, self.message)
        }
    }
}

/// Builder for creating suggestions with a fluent API.
#[derive(Debug, Default)]
pub struct SuggestionBuilder {
    message: Option<String>,
    applicability: Applicability,
    edits: Vec<SuggestionEdit>,
}

impl SuggestionBuilder {
    /// Creates a new suggestion builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the suggestion message.
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Sets the applicability level.
    pub fn applicability(mut self, applicability: Applicability) -> Self {
        self.applicability = applicability;
        self
    }

    /// Sets the suggestion as machine-applicable.
    pub fn machine_applicable(mut self) -> Self {
        self.applicability = Applicability::MachineApplicable;
        self
    }

    /// Sets the suggestion as having placeholders.
    pub fn has_placeholders(mut self) -> Self {
        self.applicability = Applicability::HasPlaceholders;
        self
    }

    /// Sets the suggestion as maybe incorrect.
    pub fn maybe_incorrect(mut self) -> Self {
        self.applicability = Applicability::MaybeIncorrect;
        self
    }

    /// Sets the suggestion as educational.
    pub fn educational(mut self) -> Self {
        self.applicability = Applicability::Educational;
        self
    }

    /// Adds a replacement edit.
    pub fn replace(mut self, span: &SourceSpan, new_text: impl Into<String>) -> Self {
        self.edits.push(SuggestionEdit::from_span(span, new_text));
        self
    }

    /// Adds an insertion edit.
    pub fn insert(mut self, file: impl Into<PathBuf>, offset: usize, text: impl Into<String>) -> Self {
        self.edits.push(SuggestionEdit::insert(file, offset, text));
        self
    }

    /// Adds a deletion edit.
    pub fn delete(mut self, file: impl Into<PathBuf>, start: usize, end: usize) -> Self {
        self.edits.push(SuggestionEdit::delete(file, start, end));
        self
    }

    /// Builds the suggestion.
    ///
    /// Panics if no message was set.
    pub fn build(self) -> Suggestion {
        Suggestion {
            message: self.message.expect("Suggestion message is required"),
            applicability: self.applicability,
            edits: self.edits,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_applicability_confidence() {
        assert_eq!(Applicability::MachineApplicable.confidence_range(), (100, 100));
        assert_eq!(Applicability::HasPlaceholders.confidence_range(), (90, 100));
        assert_eq!(Applicability::MaybeIncorrect.confidence_range(), (60, 90));
        assert_eq!(Applicability::Educational.confidence_range(), (0, 60));
    }

    #[test]
    fn test_suggestion_edit_creation() {
        let edit = SuggestionEdit::new("test.aria", 10, 20, "new_text");
        assert_eq!(edit.replaced_len(), 10);
        assert!(!edit.is_insertion());
        assert!(!edit.is_deletion());
    }

    #[test]
    fn test_suggestion_edit_insertion() {
        let edit = SuggestionEdit::insert("test.aria", 10, "inserted");
        assert!(edit.is_insertion());
        assert!(!edit.is_deletion());
        assert_eq!(edit.replaced_len(), 0);
    }

    #[test]
    fn test_suggestion_edit_deletion() {
        let edit = SuggestionEdit::delete("test.aria", 10, 20);
        assert!(!edit.is_insertion());
        assert!(edit.is_deletion());
        assert!(edit.new_text.is_empty());
    }

    #[test]
    fn test_suggestion_creation() {
        let span = SourceSpan::new("test.aria", 10, 15);
        let suggestion = Suggestion::simple_replacement(
            "replace `pritn` with `print`",
            &span,
            "print",
            Applicability::MachineApplicable,
        );

        assert!(suggestion.can_auto_apply());
        assert_eq!(suggestion.edits.len(), 1);
    }

    #[test]
    fn test_suggestion_builder() {
        let span = SourceSpan::new("test.aria", 10, 15);
        let suggestion = SuggestionBuilder::new()
            .message("use `print` instead")
            .machine_applicable()
            .replace(&span, "print")
            .build();

        assert!(suggestion.can_auto_apply());
        assert_eq!(suggestion.applicability, Applicability::MachineApplicable);
    }

    #[test]
    fn test_full_message() {
        let s1 = Suggestion::machine_applicable("convert to string");
        assert_eq!(s1.full_message(), "convert to string");

        let s2 = Suggestion::maybe_incorrect("cloning the value");
        assert_eq!(s2.full_message(), "consider cloning the value");

        let s3 = Suggestion::educational("ownership and borrowing");
        assert_eq!(s3.full_message(), "see documentation on ownership and borrowing");
    }
}
