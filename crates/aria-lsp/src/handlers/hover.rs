//! Hover Handler
//!
//! This module handles the `textDocument/hover` request, which provides
//! information when the user hovers over code elements.
//!
//! # Aria-Specific Hover Information
//!
//! For Aria, hover should show:
//! - Type information
//! - Effect annotations (e.g., `!Console`, `!Async`)
//! - Contract status (requires/ensures)
//! - Documentation comments
//! - Ownership/borrowing information

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tracing::debug;

use crate::AriaLanguageServer;

/// Handles the `textDocument/hover` request.
///
/// Provides hover information for the element at the given position.
///
/// # Arguments
///
/// * `server` - The language server instance
/// * `params` - The hover parameters containing position
///
/// # Returns
///
/// Hover information if available at the position
pub async fn handle_hover(
    server: &AriaLanguageServer,
    params: HoverParams,
) -> Result<Option<Hover>> {
    let uri = &params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    debug!("Hover request at {}:{}", uri, format!("{}:{}", position.line, position.character));

    // Get the document
    let doc = match server.state().get_document(uri) {
        Some(doc) => doc,
        None => {
            debug!("Document not found: {}", uri);
            return Ok(None);
        }
    };

    // Get the word at the position
    let word = get_word_at_position(&doc, position);

    if word.is_empty() {
        return Ok(None);
    }

    debug!("Hovering over: {}", word);

    // TODO: Integrate with actual analysis
    //
    // Once integrated, this should:
    // 1. Find the semantic element at this position (name resolution)
    // 2. Get type information from type inference
    // 3. Get effect information from effect inference
    // 4. Get contract information if applicable
    // 5. Get documentation from AST
    // 6. Format all information into markdown

    // For now, return a placeholder
    let hover = create_placeholder_hover(&word);

    Ok(Some(hover))
}

/// Gets the word at the given position in the document.
///
/// This extracts the identifier or keyword that the cursor is on.
fn get_word_at_position(doc: &crate::state::Document, position: Position) -> String {
    let line_idx = position.line as usize;
    let char_idx = position.character as usize;

    // Get the line
    let line = match doc.line(line_idx) {
        Some(line) => line,
        None => return String::new(),
    };

    // Find word boundaries
    let chars: Vec<char> = line.chars().collect();

    if char_idx >= chars.len() {
        return String::new();
    }

    // If we're on whitespace/non-word char, return empty
    if !is_word_char(chars[char_idx]) {
        return String::new();
    }

    // Find start of word (walk backwards)
    let mut start = char_idx;
    while start > 0 && is_word_char(chars[start - 1]) {
        start -= 1;
    }

    // Find end of word (walk forwards)
    let mut end = char_idx;
    while end < chars.len() && is_word_char(chars[end]) {
        end += 1;
    }

    chars[start..end].iter().collect()
}

/// Checks if a character can be part of an identifier.
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Creates a placeholder hover for development.
///
/// This will be replaced with actual semantic information.
fn create_placeholder_hover(word: &str) -> Hover {
    // Provide helpful placeholder info for keywords
    let content = match word {
        "fn" => "**Keyword**: `fn`\n\nDeclares a function.".to_string(),
        "let" => "**Keyword**: `let`\n\nDeclares an immutable variable binding.".to_string(),
        "mut" => "**Keyword**: `mut`\n\nMakes a variable binding mutable.".to_string(),
        "if" => "**Keyword**: `if`\n\nConditional expression.".to_string(),
        "else" => "**Keyword**: `else`\n\nAlternative branch for `if`.".to_string(),
        "while" => "**Keyword**: `while`\n\nLoop with condition.".to_string(),
        "for" => "**Keyword**: `for`\n\nIterator loop.".to_string(),
        "return" => "**Keyword**: `return`\n\nReturns from a function.".to_string(),
        "struct" => "**Keyword**: `struct`\n\nDeclares a struct type.".to_string(),
        "enum" => "**Keyword**: `enum`\n\nDeclares an enum type.".to_string(),
        "trait" => "**Keyword**: `trait`\n\nDeclares a trait.".to_string(),
        "impl" => "**Keyword**: `impl`\n\nImplements methods or traits.".to_string(),
        "effect" => "**Keyword**: `effect`\n\nDeclares an algebraic effect.".to_string(),
        "handle" => "**Keyword**: `handle`\n\nHandles an algebraic effect.".to_string(),
        "requires" => "**Keyword**: `requires`\n\nPrecondition contract.".to_string(),
        "ensures" => "**Keyword**: `ensures`\n\nPostcondition contract.".to_string(),
        _ => format!(
            "**`{}`**\n\n_Type information not yet available._\n\n\
            *Aria LSP is in early development. Semantic analysis will be integrated soon.*",
            word
        ),
    };

    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: content,
        }),
        range: None, // TODO: Provide exact range of the hovered element
    }
}

/// Format type information for hover display.
///
/// This will be used once type inference is integrated.
#[allow(dead_code)]
fn format_type_info(type_str: &str) -> String {
    format!("```aria\n{}\n```", type_str)
}

/// Format effect information for hover display.
///
/// This will be used once effect inference is integrated.
#[allow(dead_code)]
fn format_effect_info(effects: &[String]) -> String {
    if effects.is_empty() {
        return String::new();
    }

    let effect_list = effects.join(", ");
    format!("\n\n**Effects**: `!{}`", effect_list)
}

/// Format contract information for hover display.
///
/// This will be used once contract checking is integrated.
#[allow(dead_code)]
fn format_contract_info(requires: &[String], ensures: &[String]) -> String {
    let mut result = String::new();

    if !requires.is_empty() {
        result.push_str("\n\n**Requires**:\n");
        for req in requires {
            result.push_str(&format!("- `{}`\n", req));
        }
    }

    if !ensures.is_empty() {
        result.push_str("\n\n**Ensures**:\n");
        for ens in ensures {
            result.push_str(&format!("- `{}`\n", ens));
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Document;

    fn test_uri() -> Url {
        Url::parse("file:///test.aria").unwrap()
    }

    #[test]
    fn test_get_word_at_position() {
        let doc = Document::new(
            test_uri(),
            "fn hello_world() {\n    let x = 42;\n}".to_string(),
            1,
            "aria".to_string(),
        );

        // "fn" keyword
        assert_eq!(
            get_word_at_position(&doc, Position { line: 0, character: 0 }),
            "fn"
        );
        assert_eq!(
            get_word_at_position(&doc, Position { line: 0, character: 1 }),
            "fn"
        );

        // "hello_world" identifier
        assert_eq!(
            get_word_at_position(&doc, Position { line: 0, character: 3 }),
            "hello_world"
        );
        assert_eq!(
            get_word_at_position(&doc, Position { line: 0, character: 10 }),
            "hello_world"
        );

        // "let" keyword on second line
        assert_eq!(
            get_word_at_position(&doc, Position { line: 1, character: 4 }),
            "let"
        );

        // "x" identifier
        assert_eq!(
            get_word_at_position(&doc, Position { line: 1, character: 8 }),
            "x"
        );

        // "42" number
        assert_eq!(
            get_word_at_position(&doc, Position { line: 1, character: 12 }),
            "42"
        );
    }

    #[test]
    fn test_get_word_at_position_whitespace() {
        let doc = Document::new(
            test_uri(),
            "fn foo() {}".to_string(),
            1,
            "aria".to_string(),
        );

        // Space between fn and foo
        assert_eq!(
            get_word_at_position(&doc, Position { line: 0, character: 2 }),
            ""
        );
    }

    #[test]
    fn test_is_word_char() {
        assert!(is_word_char('a'));
        assert!(is_word_char('Z'));
        assert!(is_word_char('_'));
        assert!(is_word_char('0'));
        assert!(is_word_char('9'));

        assert!(!is_word_char(' '));
        assert!(!is_word_char('('));
        assert!(!is_word_char(')'));
        assert!(!is_word_char('.'));
        assert!(!is_word_char(':'));
    }

    #[test]
    fn test_placeholder_hover_keywords() {
        let hover = create_placeholder_hover("fn");
        if let HoverContents::Markup(markup) = hover.contents {
            assert!(markup.value.contains("function"));
        } else {
            panic!("Expected markup content");
        }

        let hover = create_placeholder_hover("effect");
        if let HoverContents::Markup(markup) = hover.contents {
            assert!(markup.value.contains("algebraic effect"));
        } else {
            panic!("Expected markup content");
        }
    }

    #[test]
    fn test_format_effect_info() {
        let effects = vec!["Console".to_string(), "Async".to_string()];
        let formatted = format_effect_info(&effects);
        assert!(formatted.contains("Effects"));
        assert!(formatted.contains("Console, Async"));

        let empty: Vec<String> = vec![];
        assert!(format_effect_info(&empty).is_empty());
    }
}
