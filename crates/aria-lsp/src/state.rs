//! Server State Management
//!
//! This module provides centralized state management for the Aria language server.
//! It handles document storage, configuration, and will eventually integrate with
//! the query-based analysis infrastructure.
//!
//! # Architecture
//!
//! The state is designed for concurrent access from multiple async tasks:
//!
//! - **Documents**: Thread-safe storage using `DashMap`
//! - **Configuration**: Atomic access patterns for settings
//! - **Analysis Cache**: Will integrate with Salsa-based query system

use dashmap::DashMap;
use ropey::Rope;
use std::sync::atomic::{AtomicBool, Ordering};
use tower_lsp::lsp_types::*;

/// A stored document with its content and metadata.
#[derive(Debug, Clone)]
pub struct Document {
    /// The document URI.
    pub uri: Url,

    /// The document content as a rope for efficient editing.
    pub content: Rope,

    /// The document version from the client.
    pub version: i32,

    /// The language ID (should be "aria").
    pub language_id: String,
}

impl Document {
    /// Creates a new document from the given parameters.
    pub fn new(uri: Url, content: String, version: i32, language_id: String) -> Self {
        Self {
            uri,
            content: Rope::from_str(&content),
            version,
            language_id,
        }
    }

    /// Returns the document content as a string.
    pub fn text(&self) -> String {
        self.content.to_string()
    }

    /// Returns the number of lines in the document.
    pub fn line_count(&self) -> usize {
        self.content.len_lines()
    }

    /// Returns the text of a specific line.
    pub fn line(&self, line_idx: usize) -> Option<String> {
        if line_idx < self.content.len_lines() {
            Some(self.content.line(line_idx).to_string())
        } else {
            None
        }
    }

    /// Converts an LSP position to a byte offset.
    pub fn position_to_offset(&self, position: Position) -> Option<usize> {
        let line_idx = position.line as usize;
        let char_idx = position.character as usize;

        if line_idx >= self.content.len_lines() {
            return None;
        }

        let line_start = self.content.line_to_char(line_idx);
        let line = self.content.line(line_idx);
        let line_len = line.len_chars();

        // Clamp character to line length
        let char_offset = char_idx.min(line_len);

        Some(self.content.char_to_byte(line_start + char_offset))
    }

    /// Converts a byte offset to an LSP position.
    pub fn offset_to_position(&self, offset: usize) -> Option<Position> {
        if offset > self.content.len_bytes() {
            return None;
        }

        let char_idx = self.content.byte_to_char(offset);
        let line_idx = self.content.char_to_line(char_idx);
        let line_start_char = self.content.line_to_char(line_idx);
        let character = char_idx - line_start_char;

        Some(Position {
            line: line_idx as u32,
            character: character as u32,
        })
    }

    /// Applies incremental text changes to the document.
    pub fn apply_changes(&mut self, changes: Vec<TextDocumentContentChangeEvent>, version: i32) {
        for change in changes {
            if let Some(range) = change.range {
                // Incremental change
                let start_offset = self.position_to_offset(range.start);
                let end_offset = self.position_to_offset(range.end);

                if let (Some(start), Some(end)) = (start_offset, end_offset) {
                    let start_char = self.content.byte_to_char(start);
                    let end_char = self.content.byte_to_char(end);

                    self.content.remove(start_char..end_char);
                    self.content.insert(start_char, &change.text);
                }
            } else {
                // Full document replacement
                self.content = Rope::from_str(&change.text);
            }
        }

        self.version = version;
    }
}

/// Server configuration settings.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Enable inlay hints.
    pub inlay_hints_enabled: bool,

    /// Effect display style: "inline", "hover", or "both".
    pub effect_display_style: String,

    /// Contract checking mode: "on-save", "on-type", or "manual".
    pub contract_checking: String,

    /// Maximum completion items to return.
    pub max_completion_items: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            inlay_hints_enabled: true,
            effect_display_style: "inline".to_string(),
            contract_checking: "on-save".to_string(),
            max_completion_items: 50,
        }
    }
}

/// The server state containing all runtime data.
///
/// This struct is designed to be wrapped in an `Arc` for shared access
/// across async tasks. All fields use interior mutability patterns
/// for thread-safe concurrent access.
pub struct ServerState {
    /// Open documents, keyed by URI.
    documents: DashMap<Url, Document>,

    /// Server configuration.
    config: parking_lot::RwLock<ServerConfig>,

    /// Whether the server has been initialized.
    initialized: AtomicBool,

    /// Client capabilities received during initialization.
    client_capabilities: parking_lot::RwLock<Option<ClientCapabilities>>,

    /// Workspace folders.
    workspace_folders: parking_lot::RwLock<Vec<WorkspaceFolder>>,
}

impl ServerState {
    /// Creates a new server state.
    pub fn new() -> Self {
        Self {
            documents: DashMap::new(),
            config: parking_lot::RwLock::new(ServerConfig::default()),
            initialized: AtomicBool::new(false),
            client_capabilities: parking_lot::RwLock::new(None),
            workspace_folders: parking_lot::RwLock::new(Vec::new()),
        }
    }

    /// Marks the server as initialized.
    pub fn set_initialized(&self, initialized: bool) {
        self.initialized.store(initialized, Ordering::SeqCst);
    }

    /// Returns whether the server has been initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
    }

    /// Sets the client capabilities.
    pub fn set_client_capabilities(&self, capabilities: ClientCapabilities) {
        *self.client_capabilities.write() = Some(capabilities);
    }

    /// Returns the client capabilities.
    pub fn client_capabilities(&self) -> Option<ClientCapabilities> {
        self.client_capabilities.read().clone()
    }

    /// Sets the workspace folders.
    pub fn set_workspace_folders(&self, folders: Vec<WorkspaceFolder>) {
        *self.workspace_folders.write() = folders;
    }

    /// Returns the workspace folders.
    pub fn workspace_folders(&self) -> Vec<WorkspaceFolder> {
        self.workspace_folders.read().clone()
    }

    /// Opens a document and stores it.
    pub fn open_document(&self, uri: Url, text: String, version: i32, language_id: String) {
        let doc = Document::new(uri.clone(), text, version, language_id);
        self.documents.insert(uri, doc);
    }

    /// Updates a document with incremental changes.
    pub fn update_document(
        &self,
        uri: &Url,
        changes: Vec<TextDocumentContentChangeEvent>,
        version: i32,
    ) -> bool {
        if let Some(mut doc) = self.documents.get_mut(uri) {
            doc.apply_changes(changes, version);
            true
        } else {
            false
        }
    }

    /// Closes a document and removes it from storage.
    pub fn close_document(&self, uri: &Url) -> Option<Document> {
        self.documents.remove(uri).map(|(_, doc)| doc)
    }

    /// Gets a document by URI.
    pub fn get_document(&self, uri: &Url) -> Option<Document> {
        self.documents.get(uri).map(|r| r.clone())
    }

    /// Returns the number of open documents.
    pub fn document_count(&self) -> usize {
        self.documents.len()
    }

    /// Returns all open document URIs.
    pub fn document_uris(&self) -> Vec<Url> {
        self.documents.iter().map(|r| r.key().clone()).collect()
    }

    /// Gets the current configuration.
    pub fn config(&self) -> ServerConfig {
        self.config.read().clone()
    }

    /// Updates the configuration.
    pub fn update_config(&self, config: ServerConfig) {
        *self.config.write() = config;
    }
}

impl Default for ServerState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_uri() -> Url {
        Url::parse("file:///test.aria").unwrap()
    }

    #[test]
    fn test_document_creation() {
        let uri = test_uri();
        let content = "fn main() {\n    println(\"Hello\")\n}".to_string();
        let doc = Document::new(uri.clone(), content.clone(), 1, "aria".to_string());

        assert_eq!(doc.uri, uri);
        assert_eq!(doc.text(), content);
        assert_eq!(doc.version, 1);
        assert_eq!(doc.line_count(), 3);
    }

    #[test]
    fn test_document_line_access() {
        let doc = Document::new(
            test_uri(),
            "line 0\nline 1\nline 2".to_string(),
            1,
            "aria".to_string(),
        );

        assert_eq!(doc.line(0), Some("line 0\n".to_string()));
        assert_eq!(doc.line(1), Some("line 1\n".to_string()));
        assert_eq!(doc.line(2), Some("line 2".to_string()));
        assert_eq!(doc.line(3), None);
    }

    #[test]
    fn test_position_to_offset() {
        let doc = Document::new(
            test_uri(),
            "abc\ndef\nghi".to_string(),
            1,
            "aria".to_string(),
        );

        // Start of document
        assert_eq!(doc.position_to_offset(Position { line: 0, character: 0 }), Some(0));

        // Middle of first line
        assert_eq!(doc.position_to_offset(Position { line: 0, character: 1 }), Some(1));

        // Start of second line
        assert_eq!(doc.position_to_offset(Position { line: 1, character: 0 }), Some(4));

        // Start of third line
        assert_eq!(doc.position_to_offset(Position { line: 2, character: 0 }), Some(8));
    }

    #[test]
    fn test_offset_to_position() {
        let doc = Document::new(
            test_uri(),
            "abc\ndef\nghi".to_string(),
            1,
            "aria".to_string(),
        );

        // Start of document
        assert_eq!(
            doc.offset_to_position(0),
            Some(Position { line: 0, character: 0 })
        );

        // Middle of first line
        assert_eq!(
            doc.offset_to_position(1),
            Some(Position { line: 0, character: 1 })
        );

        // Start of second line
        assert_eq!(
            doc.offset_to_position(4),
            Some(Position { line: 1, character: 0 })
        );
    }

    #[test]
    fn test_apply_incremental_changes() {
        let mut doc = Document::new(test_uri(), "hello world".to_string(), 1, "aria".to_string());

        // Replace "world" with "aria"
        let changes = vec![TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position { line: 0, character: 6 },
                end: Position { line: 0, character: 11 },
            }),
            range_length: None,
            text: "aria".to_string(),
        }];

        doc.apply_changes(changes, 2);

        assert_eq!(doc.text(), "hello aria");
        assert_eq!(doc.version, 2);
    }

    #[test]
    fn test_apply_full_document_change() {
        let mut doc = Document::new(test_uri(), "old content".to_string(), 1, "aria".to_string());

        let changes = vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "new content".to_string(),
        }];

        doc.apply_changes(changes, 2);

        assert_eq!(doc.text(), "new content");
        assert_eq!(doc.version, 2);
    }

    #[test]
    fn test_server_state_document_lifecycle() {
        let state = ServerState::new();
        let uri = test_uri();

        // Initially no documents
        assert_eq!(state.document_count(), 0);
        assert!(state.get_document(&uri).is_none());

        // Open document
        state.open_document(uri.clone(), "content".to_string(), 1, "aria".to_string());
        assert_eq!(state.document_count(), 1);
        assert!(state.get_document(&uri).is_some());

        // Update document
        let changes = vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "new content".to_string(),
        }];
        assert!(state.update_document(&uri, changes, 2));

        let doc = state.get_document(&uri).unwrap();
        assert_eq!(doc.text(), "new content");
        assert_eq!(doc.version, 2);

        // Close document
        let closed = state.close_document(&uri);
        assert!(closed.is_some());
        assert_eq!(state.document_count(), 0);
    }

    #[test]
    fn test_server_state_initialization() {
        let state = ServerState::new();

        assert!(!state.is_initialized());

        state.set_initialized(true);
        assert!(state.is_initialized());

        state.set_initialized(false);
        assert!(!state.is_initialized());
    }

    #[test]
    fn test_server_state_config() {
        let state = ServerState::new();

        let config = state.config();
        assert!(config.inlay_hints_enabled);
        assert_eq!(config.max_completion_items, 50);

        let new_config = ServerConfig {
            inlay_hints_enabled: false,
            max_completion_items: 100,
            ..Default::default()
        };

        state.update_config(new_config);

        let config = state.config();
        assert!(!config.inlay_hints_enabled);
        assert_eq!(config.max_completion_items, 100);
    }
}
