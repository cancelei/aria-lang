//! Aria Language Server Protocol Implementation
//!
//! This crate provides the LSP server for the Aria programming language,
//! enabling rich IDE features like hover, completion, diagnostics, and more.
//!
//! # Architecture
//!
//! The LSP server follows a query-based architecture inspired by rust-analyzer:
//!
//! - **State Management**: Centralized server state with document storage
//! - **Capabilities**: Negotiated feature set with the client
//! - **Handlers**: Request/notification handlers for LSP protocol
//!
//! # Modules
//!
//! - [`capabilities`]: Server capability declarations
//! - [`handlers`]: LSP request and notification handlers
//! - [`state`]: Server state and document management

pub mod capabilities;
pub mod completion;
pub mod definition;
pub mod diagnostics;
pub mod handlers;
pub mod state;
pub mod types;

use std::sync::Arc;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tracing::{info, instrument};

use crate::state::ServerState;

/// The Aria Language Server.
///
/// This is the main entry point for the LSP implementation.
/// It handles all LSP protocol messages and coordinates
/// between the client and the Aria compiler infrastructure.
pub struct AriaLanguageServer {
    /// The LSP client handle for sending notifications.
    client: Client,
    /// The server state containing documents and analysis data.
    state: Arc<ServerState>,
}

impl AriaLanguageServer {
    /// Creates a new Aria Language Server instance.
    pub fn new(client: Client) -> Self {
        Self {
            client,
            state: Arc::new(ServerState::new()),
        }
    }

    /// Returns a reference to the server state.
    pub fn state(&self) -> &ServerState {
        &self.state
    }

    /// Returns a reference to the LSP client.
    pub fn client(&self) -> &Client {
        &self.client
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for AriaLanguageServer {
    #[instrument(skip(self))]
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        handlers::initialize::handle_initialize(self, params).await
    }

    #[instrument(skip(self))]
    async fn initialized(&self, params: InitializedParams) {
        handlers::initialize::handle_initialized(self, params).await
    }

    #[instrument(skip(self))]
    async fn shutdown(&self) -> Result<()> {
        handlers::shutdown::handle_shutdown(self).await
    }

    #[instrument(skip(self))]
    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        handlers::document::handle_did_open(self, params).await
    }

    #[instrument(skip(self))]
    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        handlers::document::handle_did_change(self, params).await
    }

    #[instrument(skip(self))]
    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        handlers::document::handle_did_close(self, params).await
    }

    #[instrument(skip(self))]
    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        handlers::document::handle_did_save(self, params).await
    }

    #[instrument(skip(self))]
    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        handlers::hover::handle_hover(self, params).await
    }

    #[instrument(skip(self))]
    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        handlers::completion::handle_completion(self, params).await
    }

    #[instrument(skip(self))]
    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        handlers::definition::handle_goto_definition(self, params).await
    }

    #[instrument(skip(self))]
    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        handlers::document_symbol::handle_document_symbol(self, params).await
    }
}

/// Creates the LSP service and IO transport.
///
/// This function sets up the tower-lsp infrastructure for the Aria language server.
pub fn create_server() -> (LspService<AriaLanguageServer>, tower_lsp::ClientSocket) {
    LspService::build(|client| AriaLanguageServer::new(client))
        .finish()
}

/// Runs the Aria Language Server over stdio.
///
/// This is the main entry point for the language server binary.
///
/// # Example
///
/// ```ignore
/// #[tokio::main]
/// async fn main() {
///     aria_lsp::run_server().await;
/// }
/// ```
pub async fn run_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = create_server();

    info!("Aria Language Server starting...");

    Server::new(stdin, stdout, socket)
        .serve(service)
        .await;

    info!("Aria Language Server stopped.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        // Basic test to ensure server can be created
        let (_service, _socket) = create_server();
    }
}
