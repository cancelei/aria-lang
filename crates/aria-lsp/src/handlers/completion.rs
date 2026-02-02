//! Completion Handler
//!
//! This module handles the `textDocument/completion` request, which provides
//! code completion suggestions as the user types.
//!
//! # Completion Triggers
//!
//! Completion is triggered by:
//! - `.` - Method/field access
//! - `:` - Type annotations, path separator
//! - `!` - Effect annotations
//! - Manual invocation (Ctrl+Space)
//!
//! # Aria-Specific Completions
//!
//! - Effect names after `!`
//! - Contract keywords (`requires`, `ensures`)
//! - Handler operations
//! - Type constructors

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tracing::debug;

use crate::AriaLanguageServer;

/// Handles the `textDocument/completion` request.
///
/// Provides completion suggestions at the given position.
///
/// # Arguments
///
/// * `server` - The language server instance
/// * `params` - The completion parameters containing position and trigger
///
/// # Returns
///
/// A list of completion items or None if no completions available
pub async fn handle_completion(
    server: &AriaLanguageServer,
    params: CompletionParams,
) -> Result<Option<CompletionResponse>> {
    let uri = &params.text_document_position.text_document.uri;
    let position = params.text_document_position.position;
    let trigger = params.context.as_ref().and_then(|c| c.trigger_character.as_deref());

    debug!(
        "Completion request at {}:{}:{} (trigger: {:?})",
        uri, position.line, position.character, trigger
    );

    // Get the document
    let doc = match server.state().get_document(uri) {
        Some(doc) => doc,
        None => {
            debug!("Document not found: {}", uri);
            return Ok(None);
        }
    };

    // Determine completion context
    let context = determine_completion_context(&doc, position, trigger);

    debug!("Completion context: {:?}", context);

    // Generate completions based on context
    let items = match context {
        CompletionContext::Keyword => keyword_completions(),
        CompletionContext::Effect => effect_completions(),
        CompletionContext::Type => type_completions(),
        CompletionContext::Member => member_completions(),
        CompletionContext::Contract => contract_completions(),
        CompletionContext::General => general_completions(&doc, position),
    };

    if items.is_empty() {
        return Ok(None);
    }

    Ok(Some(CompletionResponse::Array(items)))
}

/// The context in which completion was triggered.
#[derive(Debug, Clone, PartialEq)]
enum CompletionContext {
    /// At a position where a keyword is expected
    Keyword,
    /// After `!` for effect annotations
    Effect,
    /// In type position (after `:`, in generics, etc.)
    Type,
    /// After `.` for member access
    Member,
    /// Inside contract annotations
    Contract,
    /// General context (identifiers, etc.)
    General,
}

/// Determines the completion context from the document and position.
fn determine_completion_context(
    doc: &crate::state::Document,
    position: Position,
    trigger: Option<&str>,
) -> CompletionContext {
    // Check trigger character first
    match trigger {
        Some("!") => return CompletionContext::Effect,
        Some(".") => return CompletionContext::Member,
        Some(":") => return CompletionContext::Type,
        _ => {}
    }

    // Look at the text before the cursor
    let line = match doc.line(position.line as usize) {
        Some(line) => line,
        None => return CompletionContext::General,
    };

    let char_idx = position.character as usize;
    let before_cursor: String = line.chars().take(char_idx).collect();
    let trimmed = before_cursor.trim_end();

    // Check for specific patterns
    if trimmed.ends_with('!') {
        return CompletionContext::Effect;
    }

    if trimmed.ends_with('.') {
        return CompletionContext::Member;
    }

    if trimmed.ends_with(':') {
        return CompletionContext::Type;
    }

    // Check for contract context
    if trimmed.contains("requires") || trimmed.contains("ensures") {
        return CompletionContext::Contract;
    }

    // Check if we're at the start of a line (likely keyword position)
    if trimmed.is_empty() || trimmed.ends_with('{') || trimmed.ends_with(';') {
        return CompletionContext::Keyword;
    }

    CompletionContext::General
}

/// Returns keyword completions.
fn keyword_completions() -> Vec<CompletionItem> {
    let keywords = [
        ("fn", "Define a function", "fn ${1:name}($2) {\n\t$0\n}"),
        ("let", "Declare a variable", "let ${1:name} = $0;"),
        ("mut", "Mutable binding", "mut "),
        ("if", "Conditional expression", "if ${1:condition} {\n\t$0\n}"),
        ("else", "Alternative branch", "else {\n\t$0\n}"),
        ("while", "While loop", "while ${1:condition} {\n\t$0\n}"),
        ("for", "For loop", "for ${1:item} in ${2:iter} {\n\t$0\n}"),
        ("return", "Return from function", "return $0;"),
        ("struct", "Define a struct", "struct ${1:Name} {\n\t$0\n}"),
        ("enum", "Define an enum", "enum ${1:Name} {\n\t$0\n}"),
        ("trait", "Define a trait", "trait ${1:Name} {\n\t$0\n}"),
        ("impl", "Implement methods", "impl ${1:Type} {\n\t$0\n}"),
        ("effect", "Define an effect", "effect ${1:Name} {\n\t$0\n}"),
        ("handle", "Handle an effect", "handle {\n\t$0\n}"),
        ("match", "Pattern matching", "match ${1:expr} {\n\t$0\n}"),
        ("use", "Import item", "use ${0:path};"),
        ("mod", "Define module", "mod ${0:name};"),
        ("pub", "Public visibility", "pub "),
    ];

    keywords
        .into_iter()
        .map(|(kw, detail, snippet)| CompletionItem {
            label: kw.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some(detail.to_string()),
            insert_text: Some(snippet.to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        })
        .collect()
}

/// Returns effect completions.
fn effect_completions() -> Vec<CompletionItem> {
    // TODO: Get these from the analysis of the current project
    let effects = [
        ("IO", "Input/output effect"),
        ("Console", "Console I/O effect"),
        ("Async", "Asynchronous computation effect"),
        ("Exception", "Exception handling effect"),
        ("State", "Mutable state effect"),
        ("Reader", "Reader monad effect"),
        ("Writer", "Writer monad effect"),
        ("NonDet", "Non-determinism effect"),
    ];

    effects
        .into_iter()
        .map(|(effect, detail)| CompletionItem {
            label: effect.to_string(),
            kind: Some(CompletionItemKind::INTERFACE), // Use interface for effects
            detail: Some(detail.to_string()),
            insert_text: Some(effect.to_string()),
            ..Default::default()
        })
        .collect()
}

/// Returns type completions.
fn type_completions() -> Vec<CompletionItem> {
    // TODO: Get these from scope analysis
    let types = [
        ("Int", "Integer type"),
        ("Float", "Floating point type"),
        ("Bool", "Boolean type"),
        ("String", "String type"),
        ("Char", "Character type"),
        ("Unit", "Unit type ()"),
        ("Option", "Optional value"),
        ("Result", "Result type for error handling"),
        ("List", "List collection"),
        ("Map", "Key-value map"),
        ("Set", "Set collection"),
    ];

    types
        .into_iter()
        .map(|(ty, detail)| CompletionItem {
            label: ty.to_string(),
            kind: Some(CompletionItemKind::CLASS),
            detail: Some(detail.to_string()),
            insert_text: Some(ty.to_string()),
            ..Default::default()
        })
        .collect()
}

/// Returns member access completions.
fn member_completions() -> Vec<CompletionItem> {
    // TODO: Get these from type inference
    // This is a placeholder that returns common method names
    vec![
        CompletionItem {
            label: "map".to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("Transform elements".to_string()),
            insert_text: Some("map(|${1:x}| ${0})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "filter".to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("Filter elements".to_string()),
            insert_text: Some("filter(|${1:x}| ${0})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "fold".to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("Fold/reduce elements".to_string()),
            insert_text: Some("fold(${1:init}, |${2:acc}, ${3:x}| ${0})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
    ]
}

/// Returns contract-related completions.
fn contract_completions() -> Vec<CompletionItem> {
    vec![
        CompletionItem {
            label: "requires".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Precondition".to_string()),
            insert_text: Some("requires ${0:condition}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "ensures".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Postcondition".to_string()),
            insert_text: Some("ensures ${0:condition}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "old".to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("Reference to pre-state value".to_string()),
            insert_text: Some("old(${0:expr})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "result".to_string(),
            kind: Some(CompletionItemKind::VARIABLE),
            detail: Some("Return value in ensures clause".to_string()),
            ..Default::default()
        },
    ]
}

/// Returns general completions based on context.
fn general_completions(doc: &crate::state::Document, position: Position) -> Vec<CompletionItem> {
    // Get the prefix being typed
    let line = match doc.line(position.line as usize) {
        Some(line) => line,
        None => return vec![],
    };

    let char_idx = position.character as usize;
    let before_cursor: String = line.chars().take(char_idx).collect();

    // Find the word being typed
    let prefix: String = before_cursor
        .chars()
        .rev()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>()
        .chars()
        .rev()
        .collect();

    if prefix.is_empty() {
        return keyword_completions();
    }

    // TODO: Get completions from scope analysis
    // For now, combine keywords and types that match the prefix
    let mut items = Vec::new();

    for item in keyword_completions() {
        if item.label.starts_with(&prefix) {
            items.push(item);
        }
    }

    for item in type_completions() {
        if item.label.to_lowercase().starts_with(&prefix.to_lowercase()) {
            items.push(item);
        }
    }

    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Document;

    fn test_uri() -> Url {
        Url::parse("file:///test.aria").unwrap()
    }

    #[test]
    fn test_determine_completion_context_effect() {
        let doc = Document::new(
            test_uri(),
            "fn foo() -> Int !".to_string(),
            1,
            "aria".to_string(),
        );

        let context = determine_completion_context(
            &doc,
            Position { line: 0, character: 17 },
            Some("!"),
        );

        assert_eq!(context, CompletionContext::Effect);
    }

    #[test]
    fn test_determine_completion_context_member() {
        let doc = Document::new(
            test_uri(),
            "let x = foo.".to_string(),
            1,
            "aria".to_string(),
        );

        let context = determine_completion_context(
            &doc,
            Position { line: 0, character: 12 },
            Some("."),
        );

        assert_eq!(context, CompletionContext::Member);
    }

    #[test]
    fn test_determine_completion_context_type() {
        let doc = Document::new(
            test_uri(),
            "let x:".to_string(),
            1,
            "aria".to_string(),
        );

        let context = determine_completion_context(
            &doc,
            Position { line: 0, character: 6 },
            Some(":"),
        );

        assert_eq!(context, CompletionContext::Type);
    }

    #[test]
    fn test_keyword_completions() {
        let completions = keyword_completions();

        assert!(!completions.is_empty());

        let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
        assert!(labels.contains(&"fn"));
        assert!(labels.contains(&"let"));
        assert!(labels.contains(&"effect"));
        assert!(labels.contains(&"handle"));
    }

    #[test]
    fn test_effect_completions() {
        let completions = effect_completions();

        assert!(!completions.is_empty());

        let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
        assert!(labels.contains(&"IO"));
        assert!(labels.contains(&"Console"));
        assert!(labels.contains(&"Async"));
    }

    #[test]
    fn test_type_completions() {
        let completions = type_completions();

        assert!(!completions.is_empty());

        let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
        assert!(labels.contains(&"Int"));
        assert!(labels.contains(&"String"));
        assert!(labels.contains(&"Option"));
    }

    #[test]
    fn test_contract_completions() {
        let completions = contract_completions();

        let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
        assert!(labels.contains(&"requires"));
        assert!(labels.contains(&"ensures"));
        assert!(labels.contains(&"old"));
        assert!(labels.contains(&"result"));
    }

    #[test]
    fn test_general_completions_with_prefix() {
        let doc = Document::new(
            test_uri(),
            "fn foo() {\n    le\n}".to_string(),
            1,
            "aria".to_string(),
        );

        let completions = general_completions(&doc, Position { line: 1, character: 6 });

        // Should include "let" since it starts with "le"
        let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
        assert!(labels.contains(&"let"));
    }
}
