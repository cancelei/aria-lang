//! Text Document Synchronization Handlers
//!
//! This module handles document lifecycle events:
//!
//! - `didOpen`: Document opened in the editor
//! - `didChange`: Document content changed
//! - `didSave`: Document saved to disk
//! - `didClose`: Document closed in the editor
//!
//! These handlers maintain the in-memory document store and trigger
//! re-analysis as needed.

use tower_lsp::lsp_types::*;
use tracing::{debug, info};

use crate::AriaLanguageServer;

/// Handles the `textDocument/didOpen` notification.
///
/// Called when a document is opened in the editor. The full content
/// is provided and stored for subsequent analysis.
///
/// # Arguments
///
/// * `server` - The language server instance
/// * `params` - The open parameters containing document content
pub async fn handle_did_open(server: &AriaLanguageServer, params: DidOpenTextDocumentParams) {
    let uri = params.text_document.uri;
    let version = params.text_document.version;
    let language_id = params.text_document.language_id;
    let text = params.text_document.text;

    info!("Document opened: {}", uri);
    debug!(
        "Document version: {}, language: {}, length: {} bytes",
        version,
        language_id,
        text.len()
    );

    // Store the document
    server
        .state()
        .open_document(uri.clone(), text, version, language_id);

    // TODO: Trigger initial analysis
    // TODO: Publish diagnostics
    publish_diagnostics_placeholder(server, &uri).await;
}

/// Handles the `textDocument/didChange` notification.
///
/// Called when document content changes. We support incremental updates
/// for efficiency - only the changed portions are sent.
///
/// # Arguments
///
/// * `server` - The language server instance
/// * `params` - The change parameters containing incremental updates
pub async fn handle_did_change(server: &AriaLanguageServer, params: DidChangeTextDocumentParams) {
    let uri = params.text_document.uri;
    let version = params.text_document.version;

    debug!("Document changed: {} (version {})", uri, version);

    // Apply the changes
    let updated = server
        .state()
        .update_document(&uri, params.content_changes, version);

    if !updated {
        // Document wasn't in our store - this shouldn't happen
        tracing::warn!("Received change for unknown document: {}", uri);
        return;
    }

    // TODO: Trigger incremental re-analysis
    // TODO: Debounce diagnostics publishing
    // For now, we immediately publish (in production, this should be debounced)
}

/// Handles the `textDocument/didSave` notification.
///
/// Called when a document is saved. This can trigger additional
/// validation like contract checking (based on user configuration).
///
/// # Arguments
///
/// * `server` - The language server instance
/// * `params` - The save parameters, optionally containing the saved text
pub async fn handle_did_save(server: &AriaLanguageServer, params: DidSaveTextDocumentParams) {
    let uri = params.text_document.uri;

    info!("Document saved: {}", uri);

    // If text is provided, ensure our copy is in sync
    if let Some(text) = params.text {
        if let Some(doc) = server.state().get_document(&uri) {
            if doc.text() != text {
                debug!("Syncing document content from save");
                // Full replacement
                let changes = vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text,
                }];
                server.state().update_document(&uri, changes, doc.version + 1);
            }
        }
    }

    // TODO: Run contract checking if configured for on-save
    // TODO: Run full validation
    publish_diagnostics_placeholder(server, &uri).await;
}

/// Handles the `textDocument/didClose` notification.
///
/// Called when a document is closed in the editor. We remove it from
/// our in-memory store to free resources.
///
/// # Arguments
///
/// * `server` - The language server instance
/// * `params` - The close parameters
pub async fn handle_did_close(server: &AriaLanguageServer, params: DidCloseTextDocumentParams) {
    let uri = params.text_document.uri;

    info!("Document closed: {}", uri);

    // Remove the document from our store
    if server.state().close_document(&uri).is_none() {
        tracing::warn!("Closed document was not in store: {}", uri);
    }

    // Clear diagnostics for this document
    server
        .client()
        .publish_diagnostics(uri, vec![], None)
        .await;
}

/// Placeholder for publishing diagnostics.
///
/// This will be replaced with actual analysis once the semantic
/// analysis infrastructure is integrated.
async fn publish_diagnostics_placeholder(server: &AriaLanguageServer, uri: &Url) {
    // TODO: Actually analyze the document and produce diagnostics
    //
    // For now, we publish empty diagnostics to clear any stale ones.
    // Once the parser and type checker are integrated, this will be:
    //
    // 1. Get document content
    // 2. Parse with aria-parser
    // 3. Type check with aria-types
    // 4. Convert errors to LSP Diagnostics
    // 5. Publish

    let diagnostics: Vec<Diagnostic> = vec![];

    server
        .client()
        .publish_diagnostics(uri.clone(), diagnostics, None)
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::ServerState;
    use std::sync::Arc;

    #[test]
    fn test_document_open_and_change_flow() {
        // Test the state changes without the full LSP infrastructure
        let state = Arc::new(ServerState::new());
        let uri = Url::parse("file:///test.aria").unwrap();

        // Simulate open
        state.open_document(
            uri.clone(),
            "fn main() {}".to_string(),
            1,
            "aria".to_string(),
        );

        assert!(state.get_document(&uri).is_some());
        assert_eq!(state.get_document(&uri).unwrap().text(), "fn main() {}");

        // Simulate change
        let changes = vec![TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position {
                    line: 0,
                    character: 10,
                },
                end: Position {
                    line: 0,
                    character: 10,
                },
            }),
            range_length: None,
            text: "\n    println(\"hello\")\n".to_string(),
        }];

        state.update_document(&uri, changes, 2);

        let doc = state.get_document(&uri).unwrap();
        assert_eq!(doc.version, 2);
        assert!(doc.text().contains("println"));

        // Simulate close
        state.close_document(&uri);
        assert!(state.get_document(&uri).is_none());
    }
}
