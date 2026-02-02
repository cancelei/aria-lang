//! Diagnostic rendering for terminal and other outputs.
//!
//! This module provides the `DiagnosticRenderer` trait and implementations
//! for rendering diagnostics to various outputs, with the `TerminalRenderer`
//! being the primary implementation for CLI output.

use crate::{Diagnostic, DiagnosticSeverity};
use crate::span::{Label, LabelStyle, LineColumn, ResolvedSpan, SourceSpan};
use crate::suggestion::Suggestion;
use std::collections::HashMap;
use std::io::{self, Write};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

/// The color mode for diagnostic output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorMode {
    /// Standard colors for dark terminals.
    #[default]
    Standard,
    /// Colorblind-safe (CVD) mode.
    Cvd,
    /// High contrast mode.
    HighContrast,
}

/// Configuration for the diagnostic renderer.
#[derive(Debug, Clone)]
pub struct RenderConfig {
    /// Whether to use colors.
    pub use_color: bool,
    /// The color mode to use.
    pub color_mode: ColorMode,
    /// Maximum line width for output.
    pub max_width: usize,
    /// Whether to show documentation links.
    pub show_docs_links: bool,
    /// Base URL for documentation links.
    pub docs_base_url: String,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            use_color: true,
            color_mode: ColorMode::Standard,
            max_width: 100,
            show_docs_links: true,
            docs_base_url: "https://aria-lang.org/errors".to_string(),
        }
    }
}

/// A trait for rendering diagnostics to various outputs.
pub trait DiagnosticRenderer {
    /// Renders a single diagnostic.
    fn render(&self, diagnostic: &Diagnostic, sources: &SourceCache) -> io::Result<()>;

    /// Renders multiple diagnostics with a summary.
    fn render_all(&self, diagnostics: &[Diagnostic], sources: &SourceCache) -> io::Result<()> {
        for diagnostic in diagnostics {
            self.render(diagnostic, sources)?;
        }
        self.render_summary(diagnostics)?;
        Ok(())
    }

    /// Renders a summary of diagnostics.
    fn render_summary(&self, diagnostics: &[Diagnostic]) -> io::Result<()>;
}

/// A cache for source file contents.
///
/// This is used by renderers to look up source code for displaying
/// in error messages.
#[derive(Debug, Default)]
pub struct SourceCache {
    /// Map from file path to file contents.
    files: HashMap<String, String>,
}

impl SourceCache {
    /// Creates a new empty source cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a source file to the cache.
    pub fn add_source(&mut self, path: impl Into<String>, source: impl Into<String>) {
        self.files.insert(path.into(), source.into());
    }

    /// Gets the source for a file, if available.
    pub fn get_source(&self, path: &str) -> Option<&str> {
        self.files.get(path).map(|s| s.as_str())
    }

    /// Resolves a span to include line/column information and source text.
    pub fn resolve_span(&self, span: &SourceSpan) -> Option<ResolvedSpan> {
        let path = span.file.to_string_lossy();
        let source = self.get_source(&path)?;

        let (start_line, start_col) = offset_to_line_col(source, span.start);
        let (end_line, end_col) = offset_to_line_col(source, span.end);

        let source_lines: Vec<String> = source
            .lines()
            .skip(start_line.saturating_sub(1))
            .take(end_line - start_line + 1)
            .map(String::from)
            .collect();

        Some(ResolvedSpan {
            span: span.clone(),
            start: LineColumn::new(start_line, start_col),
            end: LineColumn::new(end_line, end_col),
            source_lines,
        })
    }
}

/// Converts a byte offset to a line and column number.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let offset = offset.min(source.len());
    let mut line = 1;
    let mut col = 1;
    let mut current_offset = 0;

    for ch in source.chars() {
        if current_offset >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
        current_offset += ch.len_utf8();
    }

    (line, col)
}

/// Terminal renderer for diagnostics.
///
/// This implements the canonical Aria error format from ARIA-PD-014,
/// with colored output, source context, and fix suggestions.
pub struct TerminalRenderer {
    config: RenderConfig,
    stream: StandardStream,
}

impl TerminalRenderer {
    /// Creates a new terminal renderer with default settings.
    pub fn new() -> Self {
        Self::with_config(RenderConfig::default())
    }

    /// Creates a new terminal renderer with the given configuration.
    pub fn with_config(config: RenderConfig) -> Self {
        let color_choice = if config.use_color {
            ColorChoice::Auto
        } else {
            ColorChoice::Never
        };
        Self {
            config,
            stream: StandardStream::stderr(color_choice),
        }
    }

    /// Creates a renderer that outputs to stdout.
    pub fn stdout(config: RenderConfig) -> Self {
        let color_choice = if config.use_color {
            ColorChoice::Auto
        } else {
            ColorChoice::Never
        };
        Self {
            config,
            stream: StandardStream::stdout(color_choice),
        }
    }

    /// Gets the color for a severity level.
    fn severity_color(&self, severity: DiagnosticSeverity) -> Color {
        match (severity, self.config.color_mode) {
            // Standard mode
            (DiagnosticSeverity::Error, ColorMode::Standard) => Color::Red,
            (DiagnosticSeverity::Warning, ColorMode::Standard) => Color::Yellow,
            (DiagnosticSeverity::Note, ColorMode::Standard) => Color::Cyan,
            (DiagnosticSeverity::Help, ColorMode::Standard) => Color::Green,
            // CVD (colorblind-safe) mode
            (DiagnosticSeverity::Error, ColorMode::Cvd) => Color::Rgb(213, 94, 0),  // Orange
            (DiagnosticSeverity::Warning, ColorMode::Cvd) => Color::Rgb(240, 228, 66), // Yellow
            (DiagnosticSeverity::Note, ColorMode::Cvd) => Color::Rgb(0, 114, 178),  // Blue
            (DiagnosticSeverity::Help, ColorMode::Cvd) => Color::Rgb(0, 114, 178),  // Blue
            // High contrast mode - use bold with standard colors
            (DiagnosticSeverity::Error, ColorMode::HighContrast) => Color::Red,
            (DiagnosticSeverity::Warning, ColorMode::HighContrast) => Color::Yellow,
            (DiagnosticSeverity::Note, ColorMode::HighContrast) => Color::Cyan,
            (DiagnosticSeverity::Help, ColorMode::HighContrast) => Color::Green,
        }
    }

    /// Gets the underline character for a label style.
    fn underline_char(style: LabelStyle, severity: DiagnosticSeverity) -> char {
        match (style, severity) {
            (LabelStyle::Primary, DiagnosticSeverity::Error) => '^',
            (LabelStyle::Primary, DiagnosticSeverity::Warning) => '~',
            (LabelStyle::Primary, DiagnosticSeverity::Note) => '-',
            (LabelStyle::Primary, DiagnosticSeverity::Help) => '+',
            (LabelStyle::Secondary, _) => '-',
        }
    }

    /// Writes colored text.
    fn write_colored(&mut self, text: &str, color: Color, bold: bool) -> io::Result<()> {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(color));
        if bold || self.config.color_mode == ColorMode::HighContrast {
            spec.set_bold(true);
        }
        self.stream.set_color(&spec)?;
        write!(self.stream, "{}", text)?;
        self.stream.reset()?;
        Ok(())
    }

    /// Writes the severity prefix (e.g., "error[E0001]").
    fn write_severity_header(&mut self, diagnostic: &Diagnostic) -> io::Result<()> {
        let color = self.severity_color(diagnostic.severity);
        let prefix = diagnostic.severity.prefix();

        self.write_colored(prefix, color, true)?;

        if let Some(code) = &diagnostic.code {
            self.write_colored("[", color, true)?;
            self.write_colored(code, color, true)?;
            self.write_colored("]", color, true)?;
        }

        self.write_colored(": ", color, true)?;
        self.write_colored(&diagnostic.message, color, true)?;
        writeln!(self.stream)?;

        Ok(())
    }

    /// Writes the location arrow (e.g., " --> src/main.aria:10:5").
    fn write_location(&mut self, resolved: &ResolvedSpan) -> io::Result<()> {
        let path = resolved.span.file.display();
        write!(
            self.stream,
            " --> {}:{}:{}\n",
            path, resolved.start.line, resolved.start.column
        )?;
        Ok(())
    }

    /// Writes a source line with line number.
    fn write_source_line(&mut self, line_num: usize, line: &str, max_line_width: usize) -> io::Result<()> {
        let line_num_str = format!("{:>width$}", line_num, width = max_line_width);
        self.write_colored(&line_num_str, Color::Blue, false)?;
        write!(self.stream, " | ")?;

        // Truncate line if too long
        let display_line = if line.len() > self.config.max_width.saturating_sub(max_line_width + 4) {
            let max = self.config.max_width.saturating_sub(max_line_width + 7);
            format!("{}...", &line[..max.min(line.len())])
        } else {
            line.to_string()
        };

        writeln!(self.stream, "{}", display_line)?;
        Ok(())
    }

    /// Writes an underline for a label.
    fn write_underline(
        &mut self,
        label: &Label,
        resolved: &ResolvedSpan,
        severity: DiagnosticSeverity,
        max_line_width: usize,
    ) -> io::Result<()> {
        let color = self.severity_color(if label.style == LabelStyle::Primary {
            severity
        } else {
            DiagnosticSeverity::Note
        });

        // Padding for line number column
        write!(self.stream, "{:>width$} | ", "", width = max_line_width)?;

        // Spaces before the underline
        let start_col = resolved.start.column.saturating_sub(1);
        write!(self.stream, "{:>width$}", "", width = start_col)?;

        // The underline
        let underline_char = Self::underline_char(label.style, severity);
        let underline_len = (resolved.end.column - resolved.start.column).max(1);
        let underline: String = std::iter::repeat(underline_char).take(underline_len).collect();
        self.write_colored(&underline, color, false)?;

        // The label message
        if !label.message.is_empty() {
            write!(self.stream, " ")?;
            self.write_colored(&label.message, color, false)?;
        }

        writeln!(self.stream)?;
        Ok(())
    }

    /// Writes a suggestion.
    fn write_suggestion(&mut self, suggestion: &Suggestion, sources: &SourceCache) -> io::Result<()> {
        let color = self.severity_color(DiagnosticSeverity::Help);

        // Write "help: <message>"
        self.write_colored("  help: ", color, false)?;
        writeln!(self.stream, "{}", suggestion.full_message())?;

        // Write the suggested code if there are edits
        for edit in &suggestion.edits {
            let path = edit.file.to_string_lossy();
            if let Some(source) = sources.get_source(&path) {
                let (line, col) = offset_to_line_col(source, edit.start);

                // Get the original line
                if let Some(original_line) = source.lines().nth(line.saturating_sub(1)) {
                    let line_num_width = line.to_string().len();

                    // Show the suggested replacement
                    write!(self.stream, "   |\n")?;
                    write!(self.stream, "{:>width$} | ", line, width = line_num_width)?;

                    // Construct the fixed line
                    let before = &original_line[..col.saturating_sub(1).min(original_line.len())];
                    let after_start = col.saturating_sub(1) + (edit.end - edit.start);
                    let after = if after_start < original_line.len() {
                        &original_line[after_start.min(original_line.len())..]
                    } else {
                        ""
                    };

                    write!(self.stream, "{}", before)?;
                    self.write_colored(&edit.new_text, color, false)?;
                    writeln!(self.stream, "{}", after)?;

                    // Show the change markers
                    write!(self.stream, "{:>width$} | ", "", width = line_num_width)?;
                    write!(self.stream, "{:>width$}", "", width = col.saturating_sub(1))?;

                    if edit.is_insertion() {
                        let markers: String = std::iter::repeat('+').take(edit.new_text.len()).collect();
                        self.write_colored(&markers, color, false)?;
                    } else if edit.is_deletion() {
                        let markers: String = std::iter::repeat('-').take(edit.replaced_len()).collect();
                        self.write_colored(&markers, color, false)?;
                    } else {
                        let markers: String = std::iter::repeat('~').take(edit.new_text.len()).collect();
                        self.write_colored(&markers, color, false)?;
                    }

                    writeln!(self.stream)?;
                }
            }
        }

        Ok(())
    }

    /// Writes the documentation link.
    fn write_docs_link(&mut self, code: &str) -> io::Result<()> {
        if self.config.show_docs_links {
            let color = self.severity_color(DiagnosticSeverity::Note);
            self.write_colored("  docs: ", color, false)?;
            writeln!(self.stream, "{}/{}", self.config.docs_base_url, code)?;
        }
        Ok(())
    }
}

impl Default for TerminalRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl DiagnosticRenderer for TerminalRenderer {
    fn render(&self, diagnostic: &Diagnostic, sources: &SourceCache) -> io::Result<()> {
        // We need mutable access to write, so we'll create a new renderer
        // This is a bit awkward but necessary with the trait design
        let mut renderer = TerminalRenderer::with_config(self.config.clone());
        renderer.render_impl(diagnostic, sources)
    }

    fn render_summary(&self, diagnostics: &[Diagnostic]) -> io::Result<()> {
        let mut renderer = TerminalRenderer::with_config(self.config.clone());
        renderer.render_summary_impl(diagnostics)
    }
}

impl TerminalRenderer {
    fn render_impl(&mut self, diagnostic: &Diagnostic, sources: &SourceCache) -> io::Result<()> {
        // Write severity header
        self.write_severity_header(diagnostic)?;

        // Write location and source context for each label
        if let Some(primary_span) = diagnostic.spans.primary_span() {
            if let Some(resolved) = sources.resolve_span(primary_span) {
                self.write_location(&resolved)?;
                write!(self.stream, "  |\n")?;

                let max_line_width = resolved.end.line.to_string().len().max(2);

                // Write source lines
                for (i, line) in resolved.source_lines.iter().enumerate() {
                    let line_num = resolved.start.line + i;
                    self.write_source_line(line_num, line, max_line_width)?;
                }

                // Write underlines for labels
                for label in diagnostic.spans.labels() {
                    if let Some(label_resolved) = sources.resolve_span(&label.span) {
                        self.write_underline(label, &label_resolved, diagnostic.severity, max_line_width)?;
                    }
                }
            }
        }

        // Write suggestions
        for suggestion in &diagnostic.suggestions {
            self.write_suggestion(suggestion, sources)?;
        }

        // Write documentation link
        if let Some(code) = &diagnostic.code {
            self.write_docs_link(code)?;
        }

        writeln!(self.stream)?;
        Ok(())
    }

    fn render_summary_impl(&mut self, diagnostics: &[Diagnostic]) -> io::Result<()> {
        let error_count = diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Error)
            .count();
        let warning_count = diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Warning)
            .count();

        if error_count > 0 {
            let color = self.severity_color(DiagnosticSeverity::Error);
            self.write_colored("error", color, true)?;
            write!(self.stream, ": aborting due to ")?;

            if error_count == 1 {
                writeln!(self.stream, "1 error")?;
            } else {
                writeln!(self.stream, "{} errors", error_count)?;
            }
        } else if warning_count > 0 {
            let color = self.severity_color(DiagnosticSeverity::Warning);
            self.write_colored("warning", color, true)?;
            write!(self.stream, ": compilation succeeded with ")?;

            if warning_count == 1 {
                writeln!(self.stream, "1 warning")?;
            } else {
                writeln!(self.stream, "{} warnings", warning_count)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offset_to_line_col() {
        let source = "line 1\nline 2\nline 3";
        assert_eq!(offset_to_line_col(source, 0), (1, 1));
        assert_eq!(offset_to_line_col(source, 5), (1, 6));
        assert_eq!(offset_to_line_col(source, 7), (2, 1));
        assert_eq!(offset_to_line_col(source, 14), (3, 1));
    }

    #[test]
    fn test_source_cache() {
        let mut cache = SourceCache::new();
        cache.add_source("test.aria", "let x = 42\nlet y = x + 1");

        assert!(cache.get_source("test.aria").is_some());
        assert!(cache.get_source("other.aria").is_none());
    }

    #[test]
    fn test_resolve_span() {
        let mut cache = SourceCache::new();
        cache.add_source("test.aria", "let x = 42\nlet y = x + 1");

        let span = SourceSpan::new("test.aria", 4, 5);
        let resolved = cache.resolve_span(&span).unwrap();

        assert_eq!(resolved.start.line, 1);
        assert_eq!(resolved.start.column, 5);
        assert_eq!(resolved.source_lines.len(), 1);
    }

    #[test]
    fn test_render_config_default() {
        let config = RenderConfig::default();
        assert!(config.use_color);
        assert_eq!(config.color_mode, ColorMode::Standard);
        assert_eq!(config.max_width, 100);
        assert!(config.show_docs_links);
    }
}
