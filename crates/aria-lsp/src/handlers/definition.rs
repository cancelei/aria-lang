//! Go-to-Definition Handler
//!
//! This module handles the textDocument/definition request,
//! which allows users to navigate to the definition of a symbol.

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;

use crate::AriaLanguageServer;
use crate::definition::{DefinitionResolver, extract_symbols_placeholder};
use crate::types::Position;

/// Handles the textDocument/definition request.
///
/// This request is sent when a user wants to navigate to the definition
/// of a symbol (e.g., by Ctrl+Click or F12 in most editors).
pub async fn handle_goto_definition(
    server: &AriaLanguageServer,
    params: GotoDefinitionParams,
) -> Result<Option<GotoDefinitionResponse>> {
    let uri = params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    // Get the document
    let doc = match server.state().get_document(&uri) {
        Some(doc) => doc,
        None => return Ok(None),
    };

    // Build the definition resolver from the document's symbols
    let mut resolver = DefinitionResolver::new();

    // Extract symbols from the current document
    let symbols = extract_symbols_placeholder(&doc);
    for symbol in symbols {
        resolver.register_local(symbol);
    }

    // TODO: Also register symbols from imported modules
    // This requires workspace-wide analysis

    // Convert LSP position to our internal position type
    let pos = Position::new(position.line, position.character);

    // Look up the definition
    let result = resolver.definition_at(&doc, pos);

    // Convert to LSP response
    Ok(result.to_lsp())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Document;
    use tower_lsp::lsp_types::Url;

    fn test_doc(content: &str) -> Document {
        Document::new(
            Url::parse("file:///test.aria").unwrap(),
            content.to_string(),
            1,
            "aria".to_string(),
        )
    }

    #[test]
    fn test_extract_symbols_for_definition() {
        let doc = test_doc(r#"
fn greet(name: String) -> String
    "Hello, " + name
end

struct Point
    x: Int
    y: Int
end
"#);

        let symbols = extract_symbols_placeholder(&doc);
        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();

        // Should find struct definition
        assert!(names.contains(&"Point"));
    }
}
