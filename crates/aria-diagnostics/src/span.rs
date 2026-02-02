//! Source span and label types for tracking error locations.
//!
//! This module provides types for representing source locations (spans),
//! labels that annotate those spans, and multi-spans for errors that
//! involve multiple source locations.

use std::path::PathBuf;

/// A span representing a contiguous region in a source file.
///
/// Spans are used to track the location of errors, warnings, and other
/// diagnostics within source code. They use byte offsets rather than
/// line/column numbers for efficiency, but can be converted to line/column
/// when rendering.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceSpan {
    /// The file this span is located in.
    pub file: PathBuf,
    /// The starting byte offset (inclusive).
    pub start: usize,
    /// The ending byte offset (exclusive).
    pub end: usize,
}

impl SourceSpan {
    /// Creates a new source span.
    pub fn new(file: impl Into<PathBuf>, start: usize, end: usize) -> Self {
        Self {
            file: file.into(),
            start,
            end,
        }
    }

    /// Returns the length of the span in bytes.
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Returns true if this span has zero length.
    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    /// Merges two spans into one that covers both.
    ///
    /// Panics if the spans are from different files.
    pub fn merge(&self, other: &SourceSpan) -> SourceSpan {
        assert_eq!(
            self.file, other.file,
            "Cannot merge spans from different files"
        );
        SourceSpan {
            file: self.file.clone(),
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// Returns true if this span contains the given byte offset.
    pub fn contains(&self, offset: usize) -> bool {
        offset >= self.start && offset < self.end
    }

    /// Returns true if this span overlaps with another span.
    pub fn overlaps(&self, other: &SourceSpan) -> bool {
        self.file == other.file && self.start < other.end && other.start < self.end
    }
}

/// The visual style for a label's underline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LabelStyle {
    /// Primary label (error location) - uses `^^^` underline.
    #[default]
    Primary,
    /// Secondary label (related location) - uses `---` underline.
    Secondary,
}

/// A label that annotates a source span with a message.
///
/// Labels are used to point to specific locations in source code
/// and provide explanatory text about what happened at that location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    /// The span this label annotates.
    pub span: SourceSpan,
    /// The message to display with this label.
    pub message: String,
    /// The visual style for this label.
    pub style: LabelStyle,
}

impl Label {
    /// Creates a new primary label.
    pub fn primary(span: SourceSpan, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            style: LabelStyle::Primary,
        }
    }

    /// Creates a new secondary label.
    pub fn secondary(span: SourceSpan, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            style: LabelStyle::Secondary,
        }
    }

    /// Sets the style of this label.
    pub fn with_style(mut self, style: LabelStyle) -> Self {
        self.style = style;
        self
    }
}

/// A collection of spans for errors that involve multiple source locations.
///
/// MultiSpan is used when an error needs to reference multiple locations
/// in the source code, such as type mismatches where we want to show both
/// the expected type's definition and the actual expression.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MultiSpan {
    /// The primary span (where the error occurred).
    primary: Option<SourceSpan>,
    /// All labels attached to this multi-span.
    labels: Vec<Label>,
}

impl MultiSpan {
    /// Creates a new empty multi-span.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a multi-span from a single primary span.
    pub fn from_span(span: SourceSpan) -> Self {
        Self {
            primary: Some(span),
            labels: Vec::new(),
        }
    }

    /// Creates a multi-span from spans, using the first as primary.
    pub fn from_spans(spans: impl IntoIterator<Item = SourceSpan>) -> Self {
        let mut iter = spans.into_iter();
        let primary = iter.next();
        let labels = iter
            .map(|span| Label::secondary(span, ""))
            .collect();
        Self { primary, labels }
    }

    /// Returns the primary span, if any.
    pub fn primary_span(&self) -> Option<&SourceSpan> {
        self.primary.as_ref()
    }

    /// Sets the primary span.
    pub fn set_primary(&mut self, span: SourceSpan) {
        self.primary = Some(span);
    }

    /// Adds a label to this multi-span.
    pub fn push_label(&mut self, label: Label) {
        self.labels.push(label);
    }

    /// Adds a primary label with a message.
    pub fn push_primary(&mut self, span: SourceSpan, message: impl Into<String>) {
        // If there's no primary span yet, set it
        if self.primary.is_none() {
            self.primary = Some(span.clone());
        }
        self.labels.push(Label::primary(span, message));
    }

    /// Adds a secondary label with a message.
    pub fn push_secondary(&mut self, span: SourceSpan, message: impl Into<String>) {
        self.labels.push(Label::secondary(span, message));
    }

    /// Returns all labels.
    pub fn labels(&self) -> &[Label] {
        &self.labels
    }

    /// Returns true if this multi-span has no spans.
    pub fn is_empty(&self) -> bool {
        self.primary.is_none() && self.labels.is_empty()
    }

    /// Returns all unique files referenced by this multi-span.
    pub fn files(&self) -> Vec<&PathBuf> {
        let mut files: Vec<&PathBuf> = Vec::new();
        if let Some(primary) = &self.primary {
            files.push(&primary.file);
        }
        for label in &self.labels {
            if !files.contains(&&label.span.file) {
                files.push(&label.span.file);
            }
        }
        files
    }
}

/// Position information for a span (line and column).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LineColumn {
    /// 1-indexed line number.
    pub line: usize,
    /// 1-indexed column number (in characters, not bytes).
    pub column: usize,
}

impl LineColumn {
    /// Creates a new line/column position.
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

/// A resolved span with line/column information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSpan {
    /// The original source span.
    pub span: SourceSpan,
    /// The starting line/column.
    pub start: LineColumn,
    /// The ending line/column.
    pub end: LineColumn,
    /// The source lines covered by this span.
    pub source_lines: Vec<String>,
}

impl ResolvedSpan {
    /// Returns true if this span covers multiple lines.
    pub fn is_multiline(&self) -> bool {
        self.start.line != self.end.line
    }

    /// Returns the number of lines this span covers.
    pub fn line_count(&self) -> usize {
        self.end.line - self.start.line + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_creation() {
        let span = SourceSpan::new("test.aria", 10, 20);
        assert_eq!(span.file, PathBuf::from("test.aria"));
        assert_eq!(span.start, 10);
        assert_eq!(span.end, 20);
        assert_eq!(span.len(), 10);
        assert!(!span.is_empty());
    }

    #[test]
    fn test_span_merge() {
        let span1 = SourceSpan::new("test.aria", 10, 20);
        let span2 = SourceSpan::new("test.aria", 15, 30);
        let merged = span1.merge(&span2);
        assert_eq!(merged.start, 10);
        assert_eq!(merged.end, 30);
    }

    #[test]
    fn test_span_overlaps() {
        let span1 = SourceSpan::new("test.aria", 10, 20);
        let span2 = SourceSpan::new("test.aria", 15, 25);
        let span3 = SourceSpan::new("test.aria", 25, 35);
        assert!(span1.overlaps(&span2));
        assert!(!span1.overlaps(&span3));
    }

    #[test]
    fn test_label_creation() {
        let span = SourceSpan::new("test.aria", 10, 20);
        let label = Label::primary(span.clone(), "error here");
        assert_eq!(label.style, LabelStyle::Primary);
        assert_eq!(label.message, "error here");
    }

    #[test]
    fn test_multi_span() {
        let span1 = SourceSpan::new("test.aria", 10, 20);
        let span2 = SourceSpan::new("test.aria", 30, 40);

        let mut multi = MultiSpan::new();
        multi.push_primary(span1, "primary error");
        multi.push_secondary(span2, "related");

        assert_eq!(multi.labels().len(), 2);
        assert!(multi.primary_span().is_some());
    }
}
