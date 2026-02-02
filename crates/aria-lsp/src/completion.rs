//! Completion Provider Framework
//!
//! This module provides the framework for generating code completion suggestions
//! in the Aria language server.
//!
//! # Architecture
//!
//! The completion system uses a provider-based architecture:
//!
//! ```text
//! CompletionRequest
//!        |
//!        v
//!   CompletionEngine
//!        |
//!   +----+----+----+----+
//!   |    |    |    |    |
//!   v    v    v    v    v
//! Provider1 Provider2 Provider3 ...
//!        |
//!        v
//!   Merge & Sort
//!        |
//!        v
//!   CompletionResponse
//! ```
//!
//! # Aria-Specific Completions
//!
//! - Effect names with documentation
//! - Contract keywords with snippets
//! - Handler completions
//! - Type-aware member completions

use std::collections::HashSet;
use tower_lsp::lsp_types::{self, CompletionItemKind, InsertTextFormat};

use crate::types::Position;
use crate::state::Document;

/// The context in which completion is triggered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionContext {
    /// At a position where a keyword is expected (top-level or statement start).
    TopLevel,
    /// After `!` for effect annotations.
    EffectAnnotation,
    /// In type position (after `:`, in generics, etc.).
    TypePosition,
    /// After `.` for member access.
    MemberAccess {
        /// The expression before the dot (if available).
        receiver: Option<String>,
    },
    /// Inside a function body.
    Expression,
    /// Inside contract annotations.
    Contract,
    /// In pattern position (match arms, let bindings).
    Pattern,
    /// In import path (after `use`).
    ImportPath,
    /// In attribute position (before declarations).
    Attribute,
    /// Generic parameter list.
    GenericParams,
    /// Unknown or general context.
    Unknown,
}

/// A completion item with metadata for ranking.
#[derive(Debug, Clone)]
pub struct CompletionCandidate {
    /// The completion label (shown in the list).
    pub label: String,
    /// The kind of completion.
    pub kind: CompletionKind,
    /// Optional detail text (shown next to label).
    pub detail: Option<String>,
    /// Optional documentation.
    pub documentation: Option<String>,
    /// The text to insert (if different from label).
    pub insert_text: Option<String>,
    /// Whether to use snippet syntax.
    pub is_snippet: bool,
    /// Sort priority (lower = higher priority).
    pub priority: u32,
    /// Whether this item should be preselected.
    pub preselect: bool,
    /// Filter text (for fuzzy matching).
    pub filter_text: Option<String>,
    /// Additional data for resolve.
    pub data: Option<String>,
    /// Whether the item is deprecated.
    pub deprecated: bool,
    /// Tags for the item.
    pub tags: Vec<CompletionTag>,
}

impl CompletionCandidate {
    /// Creates a new completion candidate.
    pub fn new(label: impl Into<String>, kind: CompletionKind) -> Self {
        Self {
            label: label.into(),
            kind,
            detail: None,
            documentation: None,
            insert_text: None,
            is_snippet: false,
            priority: 100,
            preselect: false,
            filter_text: None,
            data: None,
            deprecated: false,
            tags: Vec::new(),
        }
    }

    /// Sets the detail text.
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Sets the documentation.
    pub fn with_documentation(mut self, doc: impl Into<String>) -> Self {
        self.documentation = Some(doc.into());
        self
    }

    /// Sets the insert text.
    pub fn with_insert_text(mut self, text: impl Into<String>) -> Self {
        self.insert_text = Some(text.into());
        self
    }

    /// Sets this as a snippet.
    pub fn as_snippet(mut self, snippet: impl Into<String>) -> Self {
        self.insert_text = Some(snippet.into());
        self.is_snippet = true;
        self
    }

    /// Sets the priority.
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// Marks as preselected.
    pub fn preselected(mut self) -> Self {
        self.preselect = true;
        self
    }

    /// Marks as deprecated.
    pub fn deprecated(mut self) -> Self {
        self.deprecated = true;
        self.tags.push(CompletionTag::Deprecated);
        self
    }

    /// Converts to LSP CompletionItem.
    pub fn to_lsp_item(&self, sort_text_prefix: &str) -> lsp_types::CompletionItem {
        lsp_types::CompletionItem {
            label: self.label.clone(),
            kind: Some(self.kind.into()),
            detail: self.detail.clone(),
            documentation: self.documentation.clone().map(|d| {
                lsp_types::Documentation::MarkupContent(lsp_types::MarkupContent {
                    kind: lsp_types::MarkupKind::Markdown,
                    value: d,
                })
            }),
            deprecated: Some(self.deprecated),
            preselect: Some(self.preselect),
            sort_text: Some(format!("{}{:05}_{}", sort_text_prefix, self.priority, self.label)),
            filter_text: self.filter_text.clone(),
            insert_text: self.insert_text.clone(),
            insert_text_format: if self.is_snippet {
                Some(InsertTextFormat::SNIPPET)
            } else {
                Some(InsertTextFormat::PLAIN_TEXT)
            },
            tags: if self.tags.is_empty() {
                None
            } else {
                Some(self.tags.iter().map(|t| (*t).into()).collect())
            },
            data: self.data.clone().map(|d| serde_json::Value::String(d)),
            ..Default::default()
        }
    }
}

/// The kind of a completion item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompletionKind {
    /// A keyword.
    Keyword,
    /// A function.
    Function,
    /// A method.
    Method,
    /// A field.
    Field,
    /// A variable.
    Variable,
    /// A constant.
    Constant,
    /// A type (struct, enum, etc.).
    Type,
    /// A type parameter.
    TypeParameter,
    /// An enum variant.
    EnumVariant,
    /// A module.
    Module,
    /// A property.
    Property,
    /// An effect.
    Effect,
    /// A handler.
    Handler,
    /// A contract keyword.
    Contract,
    /// A snippet.
    Snippet,
    /// A file.
    File,
    /// A folder.
    Folder,
    /// An operator.
    Operator,
}

impl From<CompletionKind> for CompletionItemKind {
    fn from(kind: CompletionKind) -> Self {
        match kind {
            CompletionKind::Keyword => CompletionItemKind::KEYWORD,
            CompletionKind::Function => CompletionItemKind::FUNCTION,
            CompletionKind::Method => CompletionItemKind::METHOD,
            CompletionKind::Field => CompletionItemKind::FIELD,
            CompletionKind::Variable => CompletionItemKind::VARIABLE,
            CompletionKind::Constant => CompletionItemKind::CONSTANT,
            CompletionKind::Type => CompletionItemKind::CLASS,
            CompletionKind::TypeParameter => CompletionItemKind::TYPE_PARAMETER,
            CompletionKind::EnumVariant => CompletionItemKind::ENUM_MEMBER,
            CompletionKind::Module => CompletionItemKind::MODULE,
            CompletionKind::Property => CompletionItemKind::PROPERTY,
            CompletionKind::Effect => CompletionItemKind::INTERFACE,
            CompletionKind::Handler => CompletionItemKind::EVENT,
            CompletionKind::Contract => CompletionItemKind::KEYWORD,
            CompletionKind::Snippet => CompletionItemKind::SNIPPET,
            CompletionKind::File => CompletionItemKind::FILE,
            CompletionKind::Folder => CompletionItemKind::FOLDER,
            CompletionKind::Operator => CompletionItemKind::OPERATOR,
        }
    }
}

/// Tags for completion items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompletionTag {
    /// The item is deprecated.
    Deprecated,
}

impl From<CompletionTag> for lsp_types::CompletionItemTag {
    fn from(tag: CompletionTag) -> Self {
        match tag {
            CompletionTag::Deprecated => lsp_types::CompletionItemTag::DEPRECATED,
        }
    }
}

/// A trait for completion providers.
///
/// Implement this trait to add new sources of completion items.
pub trait CompletionProvider: Send + Sync {
    /// Returns the name of this provider for debugging.
    fn name(&self) -> &str;

    /// Returns whether this provider can handle the given context.
    fn can_handle(&self, context: &CompletionContext) -> bool;

    /// Generates completion candidates for the given context.
    fn provide(
        &self,
        context: &CompletionContext,
        prefix: &str,
        document: &Document,
        position: Position,
    ) -> Vec<CompletionCandidate>;
}

/// The main completion engine.
pub struct CompletionEngine {
    /// Registered providers.
    providers: Vec<Box<dyn CompletionProvider>>,
    /// Maximum number of items to return.
    max_items: usize,
}

impl CompletionEngine {
    /// Creates a new completion engine with default providers.
    pub fn new() -> Self {
        let mut engine = Self {
            providers: Vec::new(),
            max_items: 50,
        };

        // Register default providers
        engine.register(Box::new(KeywordProvider));
        engine.register(Box::new(EffectProvider));
        engine.register(Box::new(TypeProvider));
        engine.register(Box::new(ContractProvider));
        engine.register(Box::new(SnippetProvider));

        engine
    }

    /// Registers a completion provider.
    pub fn register(&mut self, provider: Box<dyn CompletionProvider>) {
        self.providers.push(provider);
    }

    /// Sets the maximum number of items to return.
    pub fn set_max_items(&mut self, max: usize) {
        self.max_items = max;
    }

    /// Computes completion candidates.
    pub fn complete(
        &self,
        context: CompletionContext,
        prefix: &str,
        document: &Document,
        position: Position,
    ) -> Vec<CompletionCandidate> {
        let mut candidates = Vec::new();
        let mut seen = HashSet::new();

        // Collect from all applicable providers
        for provider in &self.providers {
            if provider.can_handle(&context) {
                let items = provider.provide(&context, prefix, document, position);
                for item in items {
                    if seen.insert(item.label.clone()) {
                        candidates.push(item);
                    }
                }
            }
        }

        // Sort by priority and label
        candidates.sort_by(|a, b| {
            a.priority.cmp(&b.priority).then_with(|| a.label.cmp(&b.label))
        });

        // Limit results
        candidates.truncate(self.max_items);

        candidates
    }

    /// Converts candidates to LSP completion items.
    pub fn to_lsp_items(&self, candidates: Vec<CompletionCandidate>) -> Vec<lsp_types::CompletionItem> {
        candidates
            .into_iter()
            .enumerate()
            .map(|(i, c)| c.to_lsp_item(&format!("{:03}_", i)))
            .collect()
    }
}

impl Default for CompletionEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Provider for Aria keywords.
pub struct KeywordProvider;

impl CompletionProvider for KeywordProvider {
    fn name(&self) -> &str {
        "keywords"
    }

    fn can_handle(&self, context: &CompletionContext) -> bool {
        matches!(
            context,
            CompletionContext::TopLevel | CompletionContext::Expression | CompletionContext::Unknown
        )
    }

    fn provide(
        &self,
        _context: &CompletionContext,
        prefix: &str,
        _document: &Document,
        _position: Position,
    ) -> Vec<CompletionCandidate> {
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
            ("true", "Boolean true", "true"),
            ("false", "Boolean false", "false"),
            ("self", "Current instance", "self"),
            ("Self", "Current type", "Self"),
        ];

        keywords
            .iter()
            .filter(|(kw, _, _)| prefix.is_empty() || kw.starts_with(prefix))
            .map(|(kw, detail, snippet)| {
                CompletionCandidate::new(*kw, CompletionKind::Keyword)
                    .with_detail(*detail)
                    .as_snippet(*snippet)
                    .with_priority(10)
            })
            .collect()
    }
}

/// Provider for effect names.
pub struct EffectProvider;

impl CompletionProvider for EffectProvider {
    fn name(&self) -> &str {
        "effects"
    }

    fn can_handle(&self, context: &CompletionContext) -> bool {
        matches!(context, CompletionContext::EffectAnnotation)
    }

    fn provide(
        &self,
        _context: &CompletionContext,
        prefix: &str,
        _document: &Document,
        _position: Position,
    ) -> Vec<CompletionCandidate> {
        // TODO: Get these from project analysis
        let effects = [
            ("IO", "General input/output effect", "Represents any I/O operation."),
            ("Console", "Console I/O effect", "Reading from stdin or writing to stdout/stderr."),
            ("Async", "Asynchronous computation", "Represents async/await operations."),
            ("Exception", "Exception effect", "Represents operations that may throw."),
            ("State", "Mutable state effect", "Represents mutable state operations."),
            ("Reader", "Reader effect", "Environment reading operations."),
            ("Writer", "Writer effect", "Logging/output accumulation."),
            ("NonDet", "Non-determinism", "Non-deterministic choice operations."),
            ("Abort", "Abort effect", "Early termination with a value."),
            ("Amb", "Ambiguity effect", "Multiple possible values."),
        ];

        effects
            .iter()
            .filter(|(name, _, _)| prefix.is_empty() || name.to_lowercase().starts_with(&prefix.to_lowercase()))
            .map(|(name, detail, doc)| {
                CompletionCandidate::new(*name, CompletionKind::Effect)
                    .with_detail(*detail)
                    .with_documentation(*doc)
                    .with_priority(5)
            })
            .collect()
    }
}

/// Provider for type names.
pub struct TypeProvider;

impl CompletionProvider for TypeProvider {
    fn name(&self) -> &str {
        "types"
    }

    fn can_handle(&self, context: &CompletionContext) -> bool {
        matches!(
            context,
            CompletionContext::TypePosition | CompletionContext::GenericParams
        )
    }

    fn provide(
        &self,
        _context: &CompletionContext,
        prefix: &str,
        _document: &Document,
        _position: Position,
    ) -> Vec<CompletionCandidate> {
        // TODO: Get these from project analysis
        let types = [
            ("Int", "Integer type (64-bit signed)"),
            ("Int8", "8-bit signed integer"),
            ("Int16", "16-bit signed integer"),
            ("Int32", "32-bit signed integer"),
            ("Int64", "64-bit signed integer"),
            ("UInt", "Unsigned integer"),
            ("UInt8", "8-bit unsigned integer"),
            ("UInt16", "16-bit unsigned integer"),
            ("UInt32", "32-bit unsigned integer"),
            ("UInt64", "64-bit unsigned integer"),
            ("Float", "Floating point (64-bit)"),
            ("Float32", "32-bit floating point"),
            ("Float64", "64-bit floating point"),
            ("Bool", "Boolean type"),
            ("Char", "Unicode character"),
            ("String", "UTF-8 string"),
            ("Unit", "Unit type ()"),
            ("Never", "Never type (!)"),
            ("Option", "Optional value"),
            ("Result", "Result type for error handling"),
            ("List", "List collection"),
            ("Map", "Key-value map"),
            ("Set", "Set collection"),
            ("Vec", "Vector/array type"),
            ("Box", "Heap-allocated box"),
            ("Rc", "Reference counted pointer"),
            ("Arc", "Atomic reference counted pointer"),
        ];

        types
            .iter()
            .filter(|(ty, _)| prefix.is_empty() || ty.to_lowercase().starts_with(&prefix.to_lowercase()))
            .map(|(ty, detail)| {
                CompletionCandidate::new(*ty, CompletionKind::Type)
                    .with_detail(*detail)
                    .with_priority(15)
            })
            .collect()
    }
}

/// Provider for contract keywords.
pub struct ContractProvider;

impl CompletionProvider for ContractProvider {
    fn name(&self) -> &str {
        "contracts"
    }

    fn can_handle(&self, context: &CompletionContext) -> bool {
        matches!(context, CompletionContext::Contract | CompletionContext::TopLevel)
    }

    fn provide(
        &self,
        context: &CompletionContext,
        prefix: &str,
        _document: &Document,
        _position: Position,
    ) -> Vec<CompletionCandidate> {
        let mut items = Vec::new();

        let contract_keywords = [
            ("requires", "Precondition", "requires ${0:condition}"),
            ("ensures", "Postcondition", "ensures ${0:condition}"),
            ("invariant", "Loop/type invariant", "invariant ${0:condition}"),
            ("assert", "Runtime assertion", "assert ${0:condition}"),
            ("assume", "Compiler assumption", "assume ${0:condition}"),
        ];

        for (kw, detail, snippet) in contract_keywords {
            if prefix.is_empty() || kw.starts_with(prefix) {
                items.push(
                    CompletionCandidate::new(kw, CompletionKind::Contract)
                        .with_detail(detail)
                        .as_snippet(snippet)
                        .with_priority(20),
                );
            }
        }

        // In contract context, also provide special identifiers
        if matches!(context, CompletionContext::Contract) {
            let special = [
                ("old", "Pre-state value", "old(${0:expr})"),
                ("result", "Return value", "result"),
                ("forall", "Universal quantifier", "forall ${1:x} in ${2:range} => ${0:condition}"),
                ("exists", "Existential quantifier", "exists ${1:x} in ${2:range} => ${0:condition}"),
            ];

            for (name, detail, snippet) in special {
                if prefix.is_empty() || name.starts_with(prefix) {
                    items.push(
                        CompletionCandidate::new(name, CompletionKind::Function)
                            .with_detail(detail)
                            .as_snippet(snippet)
                            .with_priority(25),
                    );
                }
            }
        }

        items
    }
}

/// Provider for code snippets.
pub struct SnippetProvider;

impl CompletionProvider for SnippetProvider {
    fn name(&self) -> &str {
        "snippets"
    }

    fn can_handle(&self, context: &CompletionContext) -> bool {
        matches!(
            context,
            CompletionContext::TopLevel | CompletionContext::Expression
        )
    }

    fn provide(
        &self,
        _context: &CompletionContext,
        prefix: &str,
        _document: &Document,
        _position: Position,
    ) -> Vec<CompletionCandidate> {
        let snippets = [
            (
                "fn main",
                "Main function",
                "fn main() {\n\t$0\n}",
                "Entry point for the program",
            ),
            (
                "fn test",
                "Test function",
                "#[test]\nfn test_${1:name}() {\n\t$0\n}",
                "Unit test function",
            ),
            (
                "struct new",
                "Struct with constructor",
                "struct ${1:Name} {\n\t${2:field}: ${3:Type},\n}\n\nimpl ${1:Name} {\n\tfn new(${2:field}: ${3:Type}) -> Self {\n\t\tSelf { ${2:field} }\n\t}\n}",
                "Struct with new constructor",
            ),
            (
                "match option",
                "Match on Option",
                "match ${1:opt} {\n\tSome(${2:value}) => $0,\n\tNone => todo!(),\n}",
                "Pattern match on Option type",
            ),
            (
                "match result",
                "Match on Result",
                "match ${1:result} {\n\tOk(${2:value}) => $0,\n\tErr(${3:err}) => todo!(),\n}",
                "Pattern match on Result type",
            ),
            (
                "effect handler",
                "Effect handler block",
                "handle {\n\t${1:effectful_expr}\n} with {\n\t${2:Effect}::${3:operation}(${4:args}) => $0,\n}",
                "Handle algebraic effects",
            ),
            (
                "impl trait",
                "Trait implementation",
                "impl ${1:Trait} for ${2:Type} {\n\t$0\n}",
                "Implement a trait for a type",
            ),
        ];

        snippets
            .iter()
            .filter(|(label, _, _, _)| prefix.is_empty() || label.to_lowercase().contains(&prefix.to_lowercase()))
            .map(|(label, detail, snippet, doc)| {
                CompletionCandidate::new(*label, CompletionKind::Snippet)
                    .with_detail(*detail)
                    .with_documentation(*doc)
                    .as_snippet(*snippet)
                    .with_priority(50)
            })
            .collect()
    }
}

/// Determines the completion context from document and position.
pub fn determine_context(
    doc: &Document,
    position: Position,
    trigger_char: Option<&str>,
) -> CompletionContext {
    // Check trigger character first
    match trigger_char {
        Some("!") => return CompletionContext::EffectAnnotation,
        Some(".") => return CompletionContext::MemberAccess { receiver: None },
        Some(":") => return CompletionContext::TypePosition,
        _ => {}
    }

    // Get the line content
    let line = match doc.line(position.line as usize) {
        Some(line) => line,
        None => return CompletionContext::Unknown,
    };

    let char_idx = position.character as usize;
    let before_cursor: String = line.chars().take(char_idx).collect();
    let trimmed = before_cursor.trim_end();

    // Check for specific patterns
    if trimmed.ends_with('!') {
        return CompletionContext::EffectAnnotation;
    }

    if trimmed.ends_with('.') {
        // Try to extract receiver
        let receiver = extract_receiver(trimmed);
        return CompletionContext::MemberAccess { receiver };
    }

    if trimmed.ends_with(':') && !trimmed.ends_with("::") {
        return CompletionContext::TypePosition;
    }

    // Check for contract context
    if trimmed.contains("requires") || trimmed.contains("ensures") || trimmed.contains("invariant") {
        return CompletionContext::Contract;
    }

    // Check for use/import
    if trimmed.starts_with("use ") {
        return CompletionContext::ImportPath;
    }

    // Check for pattern context
    if trimmed.contains("match ") || (trimmed.contains("let ") && !trimmed.contains('=')) {
        return CompletionContext::Pattern;
    }

    // Check if we're at the start of a line (likely keyword position)
    if trimmed.is_empty() {
        return CompletionContext::TopLevel;
    }

    // Check for generic context
    if trimmed.ends_with('<') || (trimmed.contains('<') && !trimmed.contains('>')) {
        return CompletionContext::GenericParams;
    }

    CompletionContext::Expression
}

/// Extracts the receiver expression before a dot.
fn extract_receiver(before_dot: &str) -> Option<String> {
    let without_dot = before_dot.trim_end_matches('.');
    let chars: Vec<char> = without_dot.chars().collect();

    // Walk backwards to find the start of the receiver
    let end = chars.len();
    let mut depth = 0;

    for i in (0..chars.len()).rev() {
        match chars[i] {
            ')' | ']' | '}' => depth += 1,
            '(' | '[' | '{' => depth -= 1,
            // Stop at dot when at depth 0 - we want just the immediate receiver
            '.' if depth == 0 => {
                let receiver: String = chars[i + 1..end].iter().collect();
                return if receiver.is_empty() {
                    None
                } else {
                    Some(receiver.trim().to_string())
                };
            }
            ' ' | '\t' | ',' | ';' | '=' if depth == 0 => {
                let receiver: String = chars[i + 1..end].iter().collect();
                return if receiver.is_empty() {
                    None
                } else {
                    Some(receiver.trim().to_string())
                };
            }
            _ => {}
        }
    }

    // Entire string is the receiver
    let receiver: String = chars[..end].iter().collect();
    if receiver.trim().is_empty() {
        None
    } else {
        Some(receiver.trim().to_string())
    }
}

/// Gets the prefix being typed at the cursor position.
pub fn get_prefix(doc: &Document, position: Position) -> String {
    let line = match doc.line(position.line as usize) {
        Some(line) => line,
        None => return String::new(),
    };

    let char_idx = position.character as usize;
    let before_cursor: String = line.chars().take(char_idx).collect();

    // Find the word being typed
    before_cursor
        .chars()
        .rev()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>()
        .chars()
        .rev()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn test_completion_candidate_to_lsp() {
        let candidate = CompletionCandidate::new("test", CompletionKind::Function)
            .with_detail("A test function")
            .as_snippet("test($0)")
            .with_priority(10);

        let lsp_item = candidate.to_lsp_item("001_");

        assert_eq!(lsp_item.label, "test");
        assert_eq!(lsp_item.kind, Some(CompletionItemKind::FUNCTION));
        assert_eq!(lsp_item.insert_text, Some("test($0)".to_string()));
        assert_eq!(lsp_item.insert_text_format, Some(InsertTextFormat::SNIPPET));
    }

    #[test]
    fn test_determine_context_effect() {
        let doc = test_doc("fn foo() -> Int !");
        let context = determine_context(&doc, Position::new(0, 17), Some("!"));
        assert_eq!(context, CompletionContext::EffectAnnotation);
    }

    #[test]
    fn test_determine_context_member_access() {
        let doc = test_doc("let x = foo.");
        let context = determine_context(&doc, Position::new(0, 12), Some("."));
        assert!(matches!(context, CompletionContext::MemberAccess { .. }));
    }

    #[test]
    fn test_determine_context_type() {
        let doc = test_doc("let x:");
        let context = determine_context(&doc, Position::new(0, 6), Some(":"));
        assert_eq!(context, CompletionContext::TypePosition);
    }

    #[test]
    fn test_determine_context_contract() {
        let doc = test_doc("fn foo() requires ");
        let context = determine_context(&doc, Position::new(0, 18), None);
        assert_eq!(context, CompletionContext::Contract);
    }

    #[test]
    fn test_get_prefix() {
        let doc = test_doc("let some_var");
        let prefix = get_prefix(&doc, Position::new(0, 12));
        assert_eq!(prefix, "some_var");

        let doc = test_doc("let som");
        let prefix = get_prefix(&doc, Position::new(0, 7));
        assert_eq!(prefix, "som");
    }

    #[test]
    fn test_keyword_provider() {
        let provider = KeywordProvider;
        let doc = test_doc("");

        assert!(provider.can_handle(&CompletionContext::TopLevel));
        assert!(!provider.can_handle(&CompletionContext::EffectAnnotation));

        let items = provider.provide(&CompletionContext::TopLevel, "fn", &doc, Position::new(0, 0));
        assert!(!items.is_empty());
        assert!(items.iter().any(|i| i.label == "fn"));
    }

    #[test]
    fn test_effect_provider() {
        let provider = EffectProvider;
        let doc = test_doc("");

        assert!(provider.can_handle(&CompletionContext::EffectAnnotation));
        assert!(!provider.can_handle(&CompletionContext::TopLevel));

        let items = provider.provide(&CompletionContext::EffectAnnotation, "", &doc, Position::new(0, 0));
        assert!(!items.is_empty());
        assert!(items.iter().any(|i| i.label == "IO"));
        assert!(items.iter().any(|i| i.label == "Console"));
    }

    #[test]
    fn test_type_provider() {
        let provider = TypeProvider;
        let doc = test_doc("");

        let items = provider.provide(&CompletionContext::TypePosition, "Int", &doc, Position::new(0, 0));
        assert!(items.iter().any(|i| i.label == "Int"));
        assert!(items.iter().any(|i| i.label == "Int32"));
    }

    #[test]
    fn test_completion_engine() {
        let engine = CompletionEngine::new();
        let doc = test_doc("");

        let candidates = engine.complete(CompletionContext::TopLevel, "", &doc, Position::new(0, 0));
        assert!(!candidates.is_empty());

        // Should include keywords
        assert!(candidates.iter().any(|c| c.label == "fn"));
        assert!(candidates.iter().any(|c| c.label == "let"));
    }

    #[test]
    fn test_extract_receiver() {
        assert_eq!(extract_receiver("foo."), Some("foo".to_string()));
        assert_eq!(extract_receiver("x.y."), Some("y".to_string()));
        assert_eq!(extract_receiver("func()."), Some("func()".to_string()));
        assert_eq!(extract_receiver("let a = b."), Some("b".to_string()));
    }

    #[test]
    fn test_contract_provider() {
        let provider = ContractProvider;
        let doc = test_doc("");

        let items = provider.provide(&CompletionContext::Contract, "", &doc, Position::new(0, 0));
        assert!(items.iter().any(|i| i.label == "requires"));
        assert!(items.iter().any(|i| i.label == "ensures"));
        assert!(items.iter().any(|i| i.label == "old")); // special identifier in contract context
    }

    #[test]
    fn test_snippet_provider() {
        let provider = SnippetProvider;
        let doc = test_doc("");

        let items = provider.provide(&CompletionContext::TopLevel, "", &doc, Position::new(0, 0));
        assert!(items.iter().any(|i| i.label.contains("main")));
    }

    #[test]
    fn test_completion_candidate_deprecated() {
        let candidate = CompletionCandidate::new("old_func", CompletionKind::Function)
            .deprecated();

        assert!(candidate.deprecated);
        assert!(candidate.tags.contains(&CompletionTag::Deprecated));
    }
}
