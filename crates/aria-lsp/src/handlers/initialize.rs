//! Initialization and Initialized Handlers
//!
//! This module handles the LSP initialization handshake:
//!
//! 1. Client sends `initialize` request with its capabilities
//! 2. Server responds with its capabilities
//! 3. Client sends `initialized` notification
//! 4. Server is now ready to handle requests

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tracing::info;

use crate::capabilities::server_capabilities;
use crate::AriaLanguageServer;

/// Handles the `initialize` request.
///
/// This is the first message sent by the client. The server must respond
/// with its capabilities before any other requests can be processed.
///
/// # Arguments
///
/// * `server` - The language server instance
/// * `params` - Initialization parameters from the client
///
/// # Returns
///
/// The initialization result containing server capabilities
pub async fn handle_initialize(
    server: &AriaLanguageServer,
    params: InitializeParams,
) -> Result<InitializeResult> {
    info!("Initializing Aria Language Server");

    // Store client capabilities for later use
    if let Some(capabilities) = params.capabilities.clone().into() {
        server.state().set_client_capabilities(capabilities);
    }

    // Store workspace folders if provided
    if let Some(folders) = params.workspace_folders {
        info!("Workspace folders: {:?}", folders);
        server.state().set_workspace_folders(folders);
    }

    // Log client info if available
    if let Some(client_info) = &params.client_info {
        info!(
            "Client: {} {}",
            client_info.name,
            client_info.version.as_deref().unwrap_or("unknown")
        );
    }

    Ok(InitializeResult {
        capabilities: server_capabilities(),
        server_info: Some(ServerInfo {
            name: "aria-lsp".to_string(),
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
        }),
    })
}

/// Handles the `initialized` notification.
///
/// This notification is sent by the client after it has received the
/// `initialize` response. The server can now begin normal operation.
///
/// # Arguments
///
/// * `server` - The language server instance
/// * `_params` - Initialized parameters (currently empty)
pub async fn handle_initialized(server: &AriaLanguageServer, _params: InitializedParams) {
    info!("Aria Language Server initialized successfully");

    server.state().set_initialized(true);

    // TODO: Register for dynamic capabilities if needed
    // TODO: Start background indexing
    // TODO: Load workspace configuration
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::LspService;

    fn create_test_server() -> AriaLanguageServer {
        let (service, _) = LspService::build(|client| AriaLanguageServer::new(client)).finish();
        // Note: In tests, we'd need proper mocking. For now, this demonstrates the structure.
        drop(service);
        todo!("Proper test infrastructure needed")
    }

    #[test]
    #[ignore = "Requires proper LSP test infrastructure"]
    fn test_initialize_returns_capabilities() {
        let _server = create_test_server();
        // TODO: Add proper test once test infrastructure is in place
    }
}
