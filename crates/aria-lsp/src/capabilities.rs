//! Server Capability Declarations
//!
//! This module defines the LSP capabilities that the Aria language server supports.
//! Capabilities are negotiated during the initialization handshake with the client.
//!
//! # Capability Tiers
//!
//! Based on ARIA-PD-012, capabilities are organized into tiers:
//!
//! - **Tier 1 (MVP)**: Diagnostics, Go to Definition, Hover, Document Symbols
//! - **Tier 2 (v1.1)**: Completion, Find References, Rename, Signature Help
//! - **Tier 3 (v1.2)**: Semantic Tokens, Inlay Hints, Code Actions, Folding
//! - **Tier 4 (v2.0)**: Type/Call Hierarchy, Workspace Symbols, Code Lens

use tower_lsp::lsp_types::*;

/// Creates the server capabilities to advertise to the client.
///
/// This function returns the full set of capabilities that the Aria
/// language server currently supports. Capabilities are added incrementally
/// as features are implemented.
pub fn server_capabilities() -> ServerCapabilities {
    ServerCapabilities {
        // Text document synchronization
        text_document_sync: Some(TextDocumentSyncCapability::Options(
            text_document_sync_options(),
        )),

        // Tier 1: Foundation (MVP)
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        definition_provider: Some(OneOf::Left(true)),
        document_symbol_provider: Some(OneOf::Left(true)),

        // Tier 2: Productivity
        completion_provider: Some(completion_options()),
        references_provider: Some(OneOf::Left(false)), // Placeholder - not yet implemented
        rename_provider: Some(OneOf::Left(false)), // Placeholder - not yet implemented
        signature_help_provider: None, // Placeholder - not yet implemented

        // Tier 3: Enhanced Experience
        semantic_tokens_provider: None, // Placeholder - not yet implemented
        inlay_hint_provider: None, // Placeholder - not yet implemented
        code_action_provider: None, // Placeholder - not yet implemented
        folding_range_provider: None, // Placeholder - not yet implemented

        // Tier 4: Advanced
        // Note: type_hierarchy_provider requires LSP 3.17+ which is not in tower-lsp 0.20
        call_hierarchy_provider: None, // Placeholder - not yet implemented
        workspace_symbol_provider: None, // Placeholder - not yet implemented
        code_lens_provider: None, // Placeholder - not yet implemented

        // Workspace capabilities
        workspace: Some(workspace_server_capabilities()),

        // Other capabilities
        position_encoding: None,
        selection_range_provider: None,
        execute_command_provider: None,
        document_formatting_provider: None,
        document_range_formatting_provider: None,
        document_on_type_formatting_provider: None,
        linked_editing_range_provider: None,
        moniker_provider: None,
        inline_value_provider: None,
        declaration_provider: None,
        type_definition_provider: None,
        implementation_provider: None,
        document_highlight_provider: None,
        document_link_provider: None,
        color_provider: None,
        diagnostic_provider: None,
        experimental: None,
    }
}

/// Text document synchronization options.
///
/// We use incremental sync for efficiency - only changed portions
/// of documents are sent from the client.
fn text_document_sync_options() -> TextDocumentSyncOptions {
    TextDocumentSyncOptions {
        // Open and close notifications
        open_close: Some(true),

        // Incremental sync for efficiency
        // Full sync would be: TextDocumentSyncKind::FULL
        change: Some(TextDocumentSyncKind::INCREMENTAL),

        // Receive save notifications with text content
        save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
            include_text: Some(true),
        })),

        // We don't use will_save notifications
        will_save: Some(false),
        will_save_wait_until: Some(false),
    }
}

/// Completion provider options.
///
/// Configures how the completion feature behaves.
fn completion_options() -> CompletionOptions {
    CompletionOptions {
        // Characters that trigger completion automatically
        trigger_characters: Some(vec![
            ".".to_string(),  // Method/field access
            ":".to_string(),  // Type annotations, path separator
            "!".to_string(),  // Effect annotations
        ]),

        // Characters that re-trigger completion while typing
        all_commit_characters: None,

        // We support resolving additional details for items
        resolve_provider: Some(true),

        // Work done progress support
        work_done_progress_options: WorkDoneProgressOptions {
            work_done_progress: Some(false),
        },

        // Completion item label details
        completion_item: None,
    }
}

/// Workspace server capabilities.
fn workspace_server_capabilities() -> WorkspaceServerCapabilities {
    WorkspaceServerCapabilities {
        workspace_folders: Some(WorkspaceFoldersServerCapabilities {
            supported: Some(true),
            change_notifications: Some(OneOf::Left(true)),
        }),
        file_operations: None,
    }
}

/// Semantic token types supported by Aria.
///
/// These extend the standard LSP semantic token types with Aria-specific
/// tokens for effects, contracts, and handlers.
#[allow(dead_code)]
pub fn semantic_token_types() -> Vec<SemanticTokenType> {
    vec![
        // Standard token types
        SemanticTokenType::NAMESPACE,
        SemanticTokenType::TYPE,
        SemanticTokenType::CLASS,
        SemanticTokenType::ENUM,
        SemanticTokenType::INTERFACE,
        SemanticTokenType::STRUCT,
        SemanticTokenType::TYPE_PARAMETER,
        SemanticTokenType::PARAMETER,
        SemanticTokenType::VARIABLE,
        SemanticTokenType::PROPERTY,
        SemanticTokenType::ENUM_MEMBER,
        SemanticTokenType::FUNCTION,
        SemanticTokenType::METHOD,
        SemanticTokenType::MACRO,
        SemanticTokenType::KEYWORD,
        SemanticTokenType::COMMENT,
        SemanticTokenType::STRING,
        SemanticTokenType::NUMBER,
        SemanticTokenType::OPERATOR,
        // Aria-specific token types (registered as custom types)
        SemanticTokenType::new("effect"),
        SemanticTokenType::new("handler"),
        SemanticTokenType::new("contract"),
        SemanticTokenType::new("lifetime"),
    ]
}

/// Semantic token modifiers supported by Aria.
///
/// These extend the standard LSP semantic token modifiers with Aria-specific
/// modifiers for effectful functions, pure functions, and contracts.
#[allow(dead_code)]
pub fn semantic_token_modifiers() -> Vec<SemanticTokenModifier> {
    vec![
        // Standard modifiers
        SemanticTokenModifier::DECLARATION,
        SemanticTokenModifier::DEFINITION,
        SemanticTokenModifier::READONLY,
        SemanticTokenModifier::STATIC,
        SemanticTokenModifier::DEPRECATED,
        SemanticTokenModifier::ABSTRACT,
        SemanticTokenModifier::ASYNC,
        SemanticTokenModifier::MODIFICATION,
        SemanticTokenModifier::DOCUMENTATION,
        SemanticTokenModifier::DEFAULT_LIBRARY,
        // Aria-specific modifiers
        SemanticTokenModifier::new("effectful"),
        SemanticTokenModifier::new("pure"),
        SemanticTokenModifier::new("contracted"),
        SemanticTokenModifier::new("unsafe"),
        SemanticTokenModifier::new("inferred"),
        SemanticTokenModifier::new("handler"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_capabilities_has_basic_features() {
        let caps = server_capabilities();

        // Text sync should be enabled
        assert!(caps.text_document_sync.is_some());

        // Hover should be enabled
        assert!(caps.hover_provider.is_some());

        // Completion should be enabled
        assert!(caps.completion_provider.is_some());
    }

    #[test]
    fn test_text_document_sync_is_incremental() {
        let options = text_document_sync_options();

        assert_eq!(options.change, Some(TextDocumentSyncKind::INCREMENTAL));
        assert_eq!(options.open_close, Some(true));
    }

    #[test]
    fn test_completion_trigger_characters() {
        let options = completion_options();

        let triggers = options.trigger_characters.unwrap();
        assert!(triggers.contains(&".".to_string()));
        assert!(triggers.contains(&":".to_string()));
        assert!(triggers.contains(&"!".to_string()));
    }

    #[test]
    fn test_semantic_token_types_includes_aria_specific() {
        let types = semantic_token_types();

        // Check for standard types
        assert!(types.contains(&SemanticTokenType::FUNCTION));
        assert!(types.contains(&SemanticTokenType::VARIABLE));

        // Check for Aria-specific types
        assert!(types.contains(&SemanticTokenType::new("effect")));
        assert!(types.contains(&SemanticTokenType::new("handler")));
        assert!(types.contains(&SemanticTokenType::new("contract")));
    }

    #[test]
    fn test_semantic_token_modifiers_includes_aria_specific() {
        let modifiers = semantic_token_modifiers();

        // Check for standard modifiers
        assert!(modifiers.contains(&SemanticTokenModifier::READONLY));
        assert!(modifiers.contains(&SemanticTokenModifier::ASYNC));

        // Check for Aria-specific modifiers
        assert!(modifiers.contains(&SemanticTokenModifier::new("effectful")));
        assert!(modifiers.contains(&SemanticTokenModifier::new("pure")));
        assert!(modifiers.contains(&SemanticTokenModifier::new("contracted")));
    }
}
