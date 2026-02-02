//! Core LSP Types for Aria Language Server
//!
//! This module provides Aria-specific LSP types and utilities for working
//! with positions, ranges, locations, and other fundamental LSP concepts.
//!
//! # Overview
//!
//! While `tower-lsp::lsp_types` provides standard LSP types, this module adds:
//!
//! - Aria-specific wrapper types with additional functionality
//! - Conversion utilities between byte offsets and LSP positions
//! - Helper types for span tracking across the compiler
//! - Builders for common LSP response types

use std::fmt;
use std::ops::Range;
use tower_lsp::lsp_types::{self, Url};

/// A position in a text document expressed as line and character offset.
///
/// Line and character are both zero-based. Character offset is measured in
/// UTF-16 code units (as per LSP specification).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Position {
    /// Zero-based line number.
    pub line: u32,
    /// Zero-based character offset (UTF-16 code units).
    pub character: u32,
}

impl Position {
    /// Creates a new position at the given line and character.
    pub fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }

    /// Creates a position at the start of a document (0, 0).
    pub fn start() -> Self {
        Self::new(0, 0)
    }

    /// Returns true if this position is before the other position.
    pub fn is_before(&self, other: &Position) -> bool {
        self.line < other.line || (self.line == other.line && self.character < other.character)
    }

    /// Returns true if this position is after the other position.
    pub fn is_after(&self, other: &Position) -> bool {
        other.is_before(self)
    }

    /// Returns true if this position is at the start of a line.
    pub fn is_line_start(&self) -> bool {
        self.character == 0
    }
}

impl From<lsp_types::Position> for Position {
    fn from(pos: lsp_types::Position) -> Self {
        Self {
            line: pos.line,
            character: pos.character,
        }
    }
}

impl From<Position> for lsp_types::Position {
    fn from(pos: Position) -> Self {
        lsp_types::Position {
            line: pos.line,
            character: pos.character,
        }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line + 1, self.character + 1)
    }
}

/// A range in a text document expressed as start and end positions.
///
/// The range is half-open: [start, end). The end position is exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TextRange {
    /// The start position (inclusive).
    pub start: Position,
    /// The end position (exclusive).
    pub end: Position,
}

impl TextRange {
    /// Creates a new range from start to end positions.
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    /// Creates a range spanning a single position (zero-width).
    pub fn point(pos: Position) -> Self {
        Self::new(pos, pos)
    }

    /// Creates a range spanning a single line.
    pub fn line(line: u32) -> Self {
        Self::new(
            Position::new(line, 0),
            Position::new(line, u32::MAX),
        )
    }

    /// Returns true if the range is empty (start == end).
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Returns true if this range contains the given position.
    pub fn contains(&self, pos: Position) -> bool {
        !pos.is_before(&self.start) && pos.is_before(&self.end)
    }

    /// Returns true if this range contains the other range entirely.
    pub fn contains_range(&self, other: &TextRange) -> bool {
        self.contains(other.start) && (self.contains(other.end) || self.end == other.end)
    }

    /// Returns true if this range overlaps with the other range.
    pub fn overlaps(&self, other: &TextRange) -> bool {
        // Adjacent ranges (where one ends exactly where the other starts) do NOT overlap
        other.start.is_before(&self.end) && self.start.is_before(&other.end)
    }

    /// Returns the intersection of this range with another, if any.
    pub fn intersection(&self, other: &TextRange) -> Option<TextRange> {
        if !self.overlaps(other) {
            return None;
        }

        let start = if self.start.is_before(&other.start) {
            other.start
        } else {
            self.start
        };

        let end = if self.end.is_before(&other.end) {
            self.end
        } else {
            other.end
        };

        Some(TextRange::new(start, end))
    }

    /// Extends this range to include the other range.
    pub fn union(&self, other: &TextRange) -> TextRange {
        let start = if self.start.is_before(&other.start) {
            self.start
        } else {
            other.start
        };

        let end = if self.end.is_after(&other.end) {
            self.end
        } else {
            other.end
        };

        TextRange::new(start, end)
    }
}

impl From<lsp_types::Range> for TextRange {
    fn from(range: lsp_types::Range) -> Self {
        Self {
            start: range.start.into(),
            end: range.end.into(),
        }
    }
}

impl From<TextRange> for lsp_types::Range {
    fn from(range: TextRange) -> Self {
        lsp_types::Range {
            start: range.start.into(),
            end: range.end.into(),
        }
    }
}

impl fmt::Display for TextRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}", self.start, self.end)
    }
}

/// A location in a specific document.
///
/// Combines a document URI with a range within that document.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Location {
    /// The document URI.
    pub uri: Url,
    /// The range within the document.
    pub range: TextRange,
}

impl Location {
    /// Creates a new location in the given document at the given range.
    pub fn new(uri: Url, range: TextRange) -> Self {
        Self { uri, range }
    }

    /// Creates a location at a single position in the document.
    pub fn point(uri: Url, pos: Position) -> Self {
        Self::new(uri, TextRange::point(pos))
    }
}

impl From<lsp_types::Location> for Location {
    fn from(loc: lsp_types::Location) -> Self {
        Self {
            uri: loc.uri,
            range: loc.range.into(),
        }
    }
}

impl From<Location> for lsp_types::Location {
    fn from(loc: Location) -> Self {
        lsp_types::Location {
            uri: loc.uri,
            range: loc.range.into(),
        }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.uri, self.range)
    }
}

/// A location link providing more detail than a simple location.
///
/// Used for go-to-definition to show the origin and target ranges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocationLink {
    /// The range in the origin document that was used to compute the link.
    pub origin_selection_range: Option<TextRange>,
    /// The target document URI.
    pub target_uri: Url,
    /// The full range of the target (e.g., the whole function definition).
    pub target_range: TextRange,
    /// The range to select/highlight (e.g., just the function name).
    pub target_selection_range: TextRange,
}

impl LocationLink {
    /// Creates a new location link.
    pub fn new(
        target_uri: Url,
        target_range: TextRange,
        target_selection_range: TextRange,
    ) -> Self {
        Self {
            origin_selection_range: None,
            target_uri,
            target_range,
            target_selection_range,
        }
    }

    /// Sets the origin selection range.
    pub fn with_origin(mut self, range: TextRange) -> Self {
        self.origin_selection_range = Some(range);
        self
    }
}

impl From<LocationLink> for lsp_types::LocationLink {
    fn from(link: LocationLink) -> Self {
        lsp_types::LocationLink {
            origin_selection_range: link.origin_selection_range.map(Into::into),
            target_uri: link.target_uri,
            target_range: link.target_range.into(),
            target_selection_range: link.target_selection_range.into(),
        }
    }
}

/// A text edit representing a change to a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    /// The range to replace.
    pub range: TextRange,
    /// The new text to insert.
    pub new_text: String,
}

impl TextEdit {
    /// Creates a new text edit.
    pub fn new(range: TextRange, new_text: String) -> Self {
        Self { range, new_text }
    }

    /// Creates an insert edit at the given position.
    pub fn insert(pos: Position, text: String) -> Self {
        Self::new(TextRange::point(pos), text)
    }

    /// Creates a delete edit for the given range.
    pub fn delete(range: TextRange) -> Self {
        Self::new(range, String::new())
    }

    /// Creates a replace edit.
    pub fn replace(range: TextRange, text: String) -> Self {
        Self::new(range, text)
    }
}

impl From<TextEdit> for lsp_types::TextEdit {
    fn from(edit: TextEdit) -> Self {
        lsp_types::TextEdit {
            range: edit.range.into(),
            new_text: edit.new_text,
        }
    }
}

impl From<lsp_types::TextEdit> for TextEdit {
    fn from(edit: lsp_types::TextEdit) -> Self {
        Self {
            range: edit.range.into(),
            new_text: edit.new_text,
        }
    }
}

/// A span in a source file, represented as byte offsets.
///
/// This is used internally for tracking locations in source text and
/// can be converted to LSP positions when needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    /// The start byte offset (inclusive).
    pub start: usize,
    /// The end byte offset (exclusive).
    pub end: usize,
}

impl Span {
    /// Creates a new span from start to end byte offsets.
    pub fn new(start: usize, end: usize) -> Self {
        debug_assert!(start <= end, "Span start must be <= end");
        Self { start, end }
    }

    /// Creates a span at a single point.
    pub fn point(offset: usize) -> Self {
        Self::new(offset, offset)
    }

    /// Creates a span from a Rust range.
    pub fn from_range(range: Range<usize>) -> Self {
        Self::new(range.start, range.end)
    }

    /// Returns the length of the span in bytes.
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    /// Returns true if the span is empty.
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Returns true if this span contains the given offset.
    pub fn contains(&self, offset: usize) -> bool {
        offset >= self.start && offset < self.end
    }

    /// Returns true if this span overlaps with the other span.
    pub fn overlaps(&self, other: &Span) -> bool {
        self.start < other.end && other.start < self.end
    }

    /// Extends this span to include the other span.
    pub fn union(&self, other: &Span) -> Span {
        Span::new(self.start.min(other.start), self.end.max(other.end))
    }

    /// Returns this span as a Rust range.
    pub fn as_range(&self) -> Range<usize> {
        self.start..self.end
    }
}

impl From<Range<usize>> for Span {
    fn from(range: Range<usize>) -> Self {
        Self::from_range(range)
    }
}

impl From<Span> for Range<usize> {
    fn from(span: Span) -> Self {
        span.as_range()
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

/// A spanned value that carries its source location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spanned<T> {
    /// The value.
    pub value: T,
    /// The source span.
    pub span: Span,
}

impl<T> Spanned<T> {
    /// Creates a new spanned value.
    pub fn new(value: T, span: Span) -> Self {
        Self { value, span }
    }

    /// Maps the inner value while preserving the span.
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Spanned<U> {
        Spanned {
            value: f(self.value),
            span: self.span,
        }
    }
}

impl<T> std::ops::Deref for Spanned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

/// Utility for converting between byte offsets and LSP positions.
pub struct LineIndex {
    /// Line start byte offsets (index = line number).
    line_starts: Vec<usize>,
    /// Total length in bytes.
    len: usize,
}

impl LineIndex {
    /// Creates a new line index for the given text.
    pub fn new(text: &str) -> Self {
        let mut line_starts = vec![0];

        for (i, c) in text.char_indices() {
            if c == '\n' {
                line_starts.push(i + 1);
            }
        }

        Self {
            line_starts,
            len: text.len(),
        }
    }

    /// Returns the number of lines.
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// Returns the byte offset of the start of the given line.
    pub fn line_start(&self, line: u32) -> Option<usize> {
        self.line_starts.get(line as usize).copied()
    }

    /// Converts a byte offset to an LSP position.
    pub fn offset_to_position(&self, offset: usize) -> Position {
        let offset = offset.min(self.len);

        // Binary search for the line containing this offset
        let line = match self.line_starts.binary_search(&offset) {
            Ok(line) => line,
            Err(line) => line.saturating_sub(1),
        };

        let line_start = self.line_starts[line];
        let character = offset - line_start;

        Position::new(line as u32, character as u32)
    }

    /// Converts an LSP position to a byte offset.
    pub fn position_to_offset(&self, pos: Position) -> Option<usize> {
        let line_start = self.line_start(pos.line)?;
        let offset = line_start + pos.character as usize;

        if offset <= self.len {
            Some(offset)
        } else {
            Some(self.len)
        }
    }

    /// Converts a span to an LSP range.
    pub fn span_to_range(&self, span: Span) -> TextRange {
        TextRange::new(
            self.offset_to_position(span.start),
            self.offset_to_position(span.end),
        )
    }

    /// Converts an LSP range to a span.
    pub fn range_to_span(&self, range: TextRange) -> Option<Span> {
        let start = self.position_to_offset(range.start)?;
        let end = self.position_to_offset(range.end)?;
        Some(Span::new(start, end))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_comparison() {
        let pos1 = Position::new(0, 5);
        let pos2 = Position::new(0, 10);
        let pos3 = Position::new(1, 0);

        assert!(pos1.is_before(&pos2));
        assert!(pos2.is_after(&pos1));
        assert!(pos2.is_before(&pos3));
        assert!(!pos1.is_before(&pos1));
    }

    #[test]
    fn test_position_display() {
        let pos = Position::new(0, 5);
        assert_eq!(format!("{}", pos), "1:6"); // 1-based display
    }

    #[test]
    fn test_range_contains() {
        let range = TextRange::new(Position::new(0, 5), Position::new(0, 15));

        assert!(range.contains(Position::new(0, 5)));
        assert!(range.contains(Position::new(0, 10)));
        assert!(!range.contains(Position::new(0, 15))); // exclusive
        assert!(!range.contains(Position::new(0, 4)));
        assert!(!range.contains(Position::new(1, 0)));
    }

    #[test]
    fn test_range_overlaps() {
        let range1 = TextRange::new(Position::new(0, 0), Position::new(0, 10));
        let range2 = TextRange::new(Position::new(0, 5), Position::new(0, 15));
        let range3 = TextRange::new(Position::new(0, 10), Position::new(0, 20));

        assert!(range1.overlaps(&range2));
        assert!(range2.overlaps(&range1));
        assert!(!range1.overlaps(&range3)); // adjacent, not overlapping
        assert!(range2.overlaps(&range3));
    }

    #[test]
    fn test_range_union() {
        let range1 = TextRange::new(Position::new(0, 0), Position::new(0, 10));
        let range2 = TextRange::new(Position::new(0, 5), Position::new(0, 15));

        let union = range1.union(&range2);
        assert_eq!(union.start, Position::new(0, 0));
        assert_eq!(union.end, Position::new(0, 15));
    }

    #[test]
    fn test_span_operations() {
        let span1 = Span::new(0, 10);
        let span2 = Span::new(5, 15);

        assert_eq!(span1.len(), 10);
        assert!(span1.contains(5));
        assert!(!span1.contains(10)); // exclusive
        assert!(span1.overlaps(&span2));

        let union = span1.union(&span2);
        assert_eq!(union.start, 0);
        assert_eq!(union.end, 15);
    }

    #[test]
    fn test_line_index_simple() {
        let text = "hello\nworld\n";
        let index = LineIndex::new(text);

        assert_eq!(index.line_count(), 3); // empty line after trailing newline
        assert_eq!(index.line_start(0), Some(0));
        assert_eq!(index.line_start(1), Some(6));
        assert_eq!(index.line_start(2), Some(12));
    }

    #[test]
    fn test_line_index_offset_to_position() {
        let text = "abc\ndef\nghi";
        let index = LineIndex::new(text);

        assert_eq!(index.offset_to_position(0), Position::new(0, 0));
        assert_eq!(index.offset_to_position(1), Position::new(0, 1));
        assert_eq!(index.offset_to_position(3), Position::new(0, 3)); // newline
        assert_eq!(index.offset_to_position(4), Position::new(1, 0));
        assert_eq!(index.offset_to_position(8), Position::new(2, 0));
    }

    #[test]
    fn test_line_index_position_to_offset() {
        let text = "abc\ndef\nghi";
        let index = LineIndex::new(text);

        assert_eq!(index.position_to_offset(Position::new(0, 0)), Some(0));
        assert_eq!(index.position_to_offset(Position::new(0, 2)), Some(2));
        assert_eq!(index.position_to_offset(Position::new(1, 0)), Some(4));
        assert_eq!(index.position_to_offset(Position::new(2, 2)), Some(10));
    }

    #[test]
    fn test_line_index_roundtrip() {
        let text = "line one\nline two\nline three";
        let index = LineIndex::new(text);

        for offset in 0..text.len() {
            let pos = index.offset_to_position(offset);
            let back = index.position_to_offset(pos).unwrap();
            assert_eq!(back, offset, "Roundtrip failed at offset {}", offset);
        }
    }

    #[test]
    fn test_text_edit_types() {
        let insert = TextEdit::insert(Position::new(0, 0), "hello".to_string());
        assert!(insert.range.is_empty());
        assert_eq!(insert.new_text, "hello");

        let delete = TextEdit::delete(TextRange::new(Position::new(0, 0), Position::new(0, 5)));
        assert!(delete.new_text.is_empty());

        let replace = TextEdit::replace(
            TextRange::new(Position::new(0, 0), Position::new(0, 5)),
            "world".to_string(),
        );
        assert_eq!(replace.new_text, "world");
    }

    #[test]
    fn test_location_display() {
        let uri = Url::parse("file:///test.aria").unwrap();
        let loc = Location::new(
            uri,
            TextRange::new(Position::new(0, 5), Position::new(0, 10)),
        );
        assert!(format!("{}", loc).contains("test.aria"));
    }

    #[test]
    fn test_spanned() {
        let spanned = Spanned::new("hello", Span::new(0, 5));
        assert_eq!(*spanned, "hello");
        assert_eq!(spanned.span.len(), 5);

        let mapped = spanned.map(|s| s.len());
        assert_eq!(*mapped, 5);
        assert_eq!(mapped.span, Span::new(0, 5));
    }
}
