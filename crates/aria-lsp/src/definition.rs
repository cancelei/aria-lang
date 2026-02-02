//! Go-to-Definition Infrastructure
//!
//! This module provides the infrastructure for navigating to definitions
//! of symbols in Aria source code.
//!
//! # Architecture
//!
//! ```text
//! GotoDefinitionRequest
//!         |
//!         v
//!   DefinitionResolver
//!         |
//!   +-----+-----+
//!   |     |     |
//!   v     v     v
//! Local  Import  External
//! Scope  Lookup  Lookup
//!         |
//!         v
//!   Location(s) or LocationLink(s)
//! ```
//!
//! # Aria-Specific Features
//!
//! - Navigate to effect definitions
//! - Navigate to handler implementations
//! - Navigate to trait implementations
//! - Navigate to contract definitions

use std::collections::HashMap;
use tower_lsp::lsp_types::Url;

use crate::types::{Position, TextRange, Location, LocationLink, LineIndex};
use crate::state::Document;

/// The kind of symbol being looked up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    /// A variable binding.
    Variable,
    /// A function definition.
    Function,
    /// A type (struct, enum, type alias).
    Type,
    /// A trait definition.
    Trait,
    /// An effect definition.
    Effect,
    /// A module.
    Module,
    /// A field in a struct.
    Field,
    /// An enum variant.
    Variant,
    /// A type parameter.
    TypeParameter,
    /// A lifetime parameter.
    Lifetime,
    /// A constant.
    Constant,
    /// A static variable.
    Static,
    /// A macro.
    Macro,
}

/// Information about a symbol definition.
#[derive(Debug, Clone)]
pub struct SymbolDefinition {
    /// The name of the symbol.
    pub name: String,
    /// The kind of symbol.
    pub kind: SymbolKind,
    /// The document containing the definition.
    pub uri: Url,
    /// The full range of the definition (e.g., entire function).
    pub full_range: TextRange,
    /// The range of just the name (for highlighting).
    pub name_range: TextRange,
    /// Optional documentation.
    pub documentation: Option<String>,
    /// The parent symbol (for nested definitions).
    pub parent: Option<String>,
    /// Whether the symbol is public.
    pub is_public: bool,
}

impl SymbolDefinition {
    /// Creates a new symbol definition.
    pub fn new(
        name: impl Into<String>,
        kind: SymbolKind,
        uri: Url,
        full_range: TextRange,
        name_range: TextRange,
    ) -> Self {
        Self {
            name: name.into(),
            kind,
            uri,
            full_range,
            name_range,
            documentation: None,
            parent: None,
            is_public: false,
        }
    }

    /// Sets the documentation.
    pub fn with_documentation(mut self, doc: impl Into<String>) -> Self {
        self.documentation = Some(doc.into());
        self
    }

    /// Sets the parent symbol.
    pub fn with_parent(mut self, parent: impl Into<String>) -> Self {
        self.parent = Some(parent.into());
        self
    }

    /// Marks as public.
    pub fn public(mut self) -> Self {
        self.is_public = true;
        self
    }

    /// Converts to a Location.
    pub fn to_location(&self) -> Location {
        Location::new(self.uri.clone(), self.name_range)
    }

    /// Converts to a LocationLink with origin.
    pub fn to_location_link(&self, origin_range: Option<TextRange>) -> LocationLink {
        let mut link = LocationLink::new(
            self.uri.clone(),
            self.full_range,
            self.name_range,
        );
        if let Some(origin) = origin_range {
            link = link.with_origin(origin);
        }
        link
    }
}

/// A reference to a symbol.
#[derive(Debug, Clone)]
pub struct SymbolReference {
    /// The document containing the reference.
    pub uri: Url,
    /// The range of the reference.
    pub range: TextRange,
    /// Whether this is a write/modification.
    pub is_write: bool,
    /// Whether this is the definition itself.
    pub is_definition: bool,
}

impl SymbolReference {
    /// Creates a new read reference.
    pub fn read(uri: Url, range: TextRange) -> Self {
        Self {
            uri,
            range,
            is_write: false,
            is_definition: false,
        }
    }

    /// Creates a new write reference.
    pub fn write(uri: Url, range: TextRange) -> Self {
        Self {
            uri,
            range,
            is_write: true,
            is_definition: false,
        }
    }

    /// Creates a definition reference.
    pub fn definition(uri: Url, range: TextRange) -> Self {
        Self {
            uri,
            range,
            is_write: true,
            is_definition: true,
        }
    }
}

/// A scope for name resolution.
#[derive(Debug, Clone)]
pub struct Scope {
    /// The parent scope (if any).
    parent: Option<Box<Scope>>,
    /// Symbols defined in this scope.
    symbols: HashMap<String, SymbolDefinition>,
}

impl Scope {
    /// Creates a new empty scope.
    pub fn new() -> Self {
        Self {
            parent: None,
            symbols: HashMap::new(),
        }
    }

    /// Creates a child scope.
    pub fn child(parent: Scope) -> Self {
        Self {
            parent: Some(Box::new(parent)),
            symbols: HashMap::new(),
        }
    }

    /// Defines a symbol in this scope.
    pub fn define(&mut self, symbol: SymbolDefinition) {
        self.symbols.insert(symbol.name.clone(), symbol);
    }

    /// Looks up a symbol by name, checking parent scopes.
    pub fn lookup(&self, name: &str) -> Option<&SymbolDefinition> {
        self.symbols.get(name).or_else(|| {
            self.parent.as_ref().and_then(|p| p.lookup(name))
        })
    }

    /// Returns all symbols in this scope (not including parent).
    pub fn symbols(&self) -> impl Iterator<Item = &SymbolDefinition> {
        self.symbols.values()
    }
}

impl Default for Scope {
    fn default() -> Self {
        Self::new()
    }
}

/// The result of a definition lookup.
#[derive(Debug, Clone)]
pub enum DefinitionResult {
    /// A single location.
    Single(Location),
    /// Multiple locations (e.g., for overloaded functions).
    Multiple(Vec<Location>),
    /// Location links with more detail.
    Links(Vec<LocationLink>),
    /// No definition found.
    NotFound,
}

impl DefinitionResult {
    /// Returns true if the result is empty/not found.
    pub fn is_empty(&self) -> bool {
        match self {
            DefinitionResult::NotFound => true,
            DefinitionResult::Single(_) => false,
            DefinitionResult::Multiple(locs) => locs.is_empty(),
            DefinitionResult::Links(links) => links.is_empty(),
        }
    }

    /// Converts to LSP response format.
    pub fn to_lsp(self) -> Option<tower_lsp::lsp_types::GotoDefinitionResponse> {
        use tower_lsp::lsp_types::GotoDefinitionResponse;

        match self {
            DefinitionResult::NotFound => None,
            DefinitionResult::Single(loc) => Some(GotoDefinitionResponse::Scalar(loc.into())),
            DefinitionResult::Multiple(locs) => {
                Some(GotoDefinitionResponse::Array(locs.into_iter().map(Into::into).collect()))
            }
            DefinitionResult::Links(links) => {
                Some(GotoDefinitionResponse::Link(links.into_iter().map(Into::into).collect()))
            }
        }
    }
}

/// Resolves definitions for symbols.
pub struct DefinitionResolver {
    /// Known symbols from the current document.
    local_symbols: HashMap<String, SymbolDefinition>,
    /// Imported symbols from other modules.
    imported_symbols: HashMap<String, SymbolDefinition>,
    /// Line index for position conversion.
    line_index: Option<LineIndex>,
}

impl DefinitionResolver {
    /// Creates a new resolver.
    pub fn new() -> Self {
        Self {
            local_symbols: HashMap::new(),
            imported_symbols: HashMap::new(),
            line_index: None,
        }
    }

    /// Sets the line index for position conversion.
    pub fn with_line_index(mut self, index: LineIndex) -> Self {
        self.line_index = Some(index);
        self
    }

    /// Registers a local symbol.
    pub fn register_local(&mut self, symbol: SymbolDefinition) {
        self.local_symbols.insert(symbol.name.clone(), symbol);
    }

    /// Registers an imported symbol.
    pub fn register_import(&mut self, symbol: SymbolDefinition) {
        self.imported_symbols.insert(symbol.name.clone(), symbol);
    }

    /// Resolves a name to its definition.
    pub fn resolve(&self, name: &str) -> Option<&SymbolDefinition> {
        self.local_symbols
            .get(name)
            .or_else(|| self.imported_symbols.get(name))
    }

    /// Gets the definition at a position in the document.
    pub fn definition_at(
        &self,
        doc: &Document,
        position: Position,
    ) -> DefinitionResult {
        // Get the word at the position
        let word = get_word_at_position(doc, position);
        if word.is_empty() {
            return DefinitionResult::NotFound;
        }

        // Look up the symbol
        if let Some(symbol) = self.resolve(&word) {
            let origin_range = get_word_range_at_position(doc, position);
            return DefinitionResult::Links(vec![symbol.to_location_link(origin_range)]);
        }

        DefinitionResult::NotFound
    }

    /// Finds all references to a symbol.
    pub fn find_references(&self, _name: &str, _include_definition: bool) -> Vec<SymbolReference> {
        // TODO: Implement full reference finding
        // This requires analyzing the entire workspace
        Vec::new()
    }
}

impl Default for DefinitionResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Gets the word at a position in a document.
pub fn get_word_at_position(doc: &Document, position: Position) -> String {
    let line_idx = position.line as usize;
    let char_idx = position.character as usize;

    let line = match doc.line(line_idx) {
        Some(line) => line,
        None => return String::new(),
    };

    let chars: Vec<char> = line.chars().collect();

    if char_idx >= chars.len() {
        return String::new();
    }

    if !is_identifier_char(chars[char_idx]) {
        return String::new();
    }

    // Find start of word
    let mut start = char_idx;
    while start > 0 && is_identifier_char(chars[start - 1]) {
        start -= 1;
    }

    // Find end of word
    let mut end = char_idx;
    while end < chars.len() && is_identifier_char(chars[end]) {
        end += 1;
    }

    chars[start..end].iter().collect()
}

/// Gets the range of a word at a position.
pub fn get_word_range_at_position(doc: &Document, position: Position) -> Option<TextRange> {
    let line_idx = position.line as usize;
    let char_idx = position.character as usize;

    let line = doc.line(line_idx)?;
    let chars: Vec<char> = line.chars().collect();

    if char_idx >= chars.len() || !is_identifier_char(chars[char_idx]) {
        return None;
    }

    // Find start of word
    let mut start = char_idx;
    while start > 0 && is_identifier_char(chars[start - 1]) {
        start -= 1;
    }

    // Find end of word
    let mut end = char_idx;
    while end < chars.len() && is_identifier_char(chars[end]) {
        end += 1;
    }

    Some(TextRange::new(
        Position::new(position.line, start as u32),
        Position::new(position.line, end as u32),
    ))
}

/// Checks if a character can be part of an identifier.
fn is_identifier_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Placeholder for extracting symbols from a document.
///
/// This will be replaced with proper AST-based extraction once
/// the parser is integrated.
pub fn extract_symbols_placeholder(doc: &Document) -> Vec<SymbolDefinition> {
    let mut symbols = Vec::new();
    let uri = doc.uri.clone();
    let text = doc.text();

    // Simple regex-like pattern matching for common definitions
    // This is a placeholder - real implementation uses the parser
    for (line_idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();

        // Match function definitions
        if let Some(name) = extract_fn_name(trimmed) {
            let start_char = line.find("fn ").unwrap_or(0) + 3;
            let end_char = start_char + name.len();

            symbols.push(SymbolDefinition::new(
                name,
                SymbolKind::Function,
                uri.clone(),
                TextRange::line(line_idx as u32),
                TextRange::new(
                    Position::new(line_idx as u32, start_char as u32),
                    Position::new(line_idx as u32, end_char as u32),
                ),
            ));
        }

        // Match struct definitions
        if let Some(name) = extract_struct_name(trimmed) {
            let start_char = line.find("struct ").unwrap_or(0) + 7;
            let end_char = start_char + name.len();

            symbols.push(SymbolDefinition::new(
                name,
                SymbolKind::Type,
                uri.clone(),
                TextRange::line(line_idx as u32),
                TextRange::new(
                    Position::new(line_idx as u32, start_char as u32),
                    Position::new(line_idx as u32, end_char as u32),
                ),
            ));
        }

        // Match enum definitions
        if let Some(name) = extract_enum_name(trimmed) {
            let start_char = line.find("enum ").unwrap_or(0) + 5;
            let end_char = start_char + name.len();

            symbols.push(SymbolDefinition::new(
                name,
                SymbolKind::Type,
                uri.clone(),
                TextRange::line(line_idx as u32),
                TextRange::new(
                    Position::new(line_idx as u32, start_char as u32),
                    Position::new(line_idx as u32, end_char as u32),
                ),
            ));
        }

        // Match effect definitions
        if let Some(name) = extract_effect_name(trimmed) {
            let start_char = line.find("effect ").unwrap_or(0) + 7;
            let end_char = start_char + name.len();

            symbols.push(SymbolDefinition::new(
                name,
                SymbolKind::Effect,
                uri.clone(),
                TextRange::line(line_idx as u32),
                TextRange::new(
                    Position::new(line_idx as u32, start_char as u32),
                    Position::new(line_idx as u32, end_char as u32),
                ),
            ));
        }

        // Match let bindings
        if let Some(name) = extract_let_name(trimmed) {
            let binding_start = if trimmed.starts_with("let mut ") {
                8
            } else {
                4
            };
            let start_char = line.find(if trimmed.starts_with("let mut ") { "let mut " } else { "let " }).unwrap_or(0) + binding_start;
            let end_char = start_char + name.len();

            symbols.push(SymbolDefinition::new(
                name,
                SymbolKind::Variable,
                uri.clone(),
                TextRange::line(line_idx as u32),
                TextRange::new(
                    Position::new(line_idx as u32, start_char as u32),
                    Position::new(line_idx as u32, end_char as u32),
                ),
            ));
        }
    }

    symbols
}

/// Extracts function name from a line.
fn extract_fn_name(line: &str) -> Option<String> {
    let line = line.strip_prefix("pub ")?.trim();
    extract_fn_name_inner(line)
}

fn extract_fn_name_inner(line: &str) -> Option<String> {
    let rest = line.strip_prefix("fn ")?;
    let name_end = rest.find(|c: char| !c.is_alphanumeric() && c != '_')?;
    Some(rest[..name_end].to_string())
}

/// Extracts struct name from a line.
fn extract_struct_name(line: &str) -> Option<String> {
    let line = if line.starts_with("pub ") {
        &line[4..]
    } else {
        line
    };
    let rest = line.strip_prefix("struct ")?;
    let name_end = rest.find(|c: char| !c.is_alphanumeric() && c != '_')
        .unwrap_or(rest.len());
    if name_end == 0 {
        return None;
    }
    Some(rest[..name_end].to_string())
}

/// Extracts enum name from a line.
fn extract_enum_name(line: &str) -> Option<String> {
    let line = if line.starts_with("pub ") {
        &line[4..]
    } else {
        line
    };
    let rest = line.strip_prefix("enum ")?;
    let name_end = rest.find(|c: char| !c.is_alphanumeric() && c != '_')
        .unwrap_or(rest.len());
    if name_end == 0 {
        return None;
    }
    Some(rest[..name_end].to_string())
}

/// Extracts effect name from a line.
fn extract_effect_name(line: &str) -> Option<String> {
    let line = if line.starts_with("pub ") {
        &line[4..]
    } else {
        line
    };
    let rest = line.strip_prefix("effect ")?;
    let name_end = rest.find(|c: char| !c.is_alphanumeric() && c != '_')
        .unwrap_or(rest.len());
    if name_end == 0 {
        return None;
    }
    Some(rest[..name_end].to_string())
}

/// Extracts let binding name from a line.
fn extract_let_name(line: &str) -> Option<String> {
    let rest = if line.starts_with("let mut ") {
        &line[8..]
    } else if line.starts_with("let ") {
        &line[4..]
    } else {
        return None;
    };

    let name_end = rest.find(|c: char| !c.is_alphanumeric() && c != '_')?;
    if name_end == 0 {
        return None;
    }
    Some(rest[..name_end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_doc(content: &str) -> Document {
        Document::new(
            Url::parse("file:///test.aria").unwrap(),
            content.to_string(),
            1,
            "aria".to_string(),
        )
    }

    #[test]
    fn test_get_word_at_position() {
        let doc = test_doc("fn hello_world() {}");

        assert_eq!(get_word_at_position(&doc, Position::new(0, 0)), "fn");
        assert_eq!(get_word_at_position(&doc, Position::new(0, 3)), "hello_world");
        assert_eq!(get_word_at_position(&doc, Position::new(0, 10)), "hello_world");
    }

    #[test]
    fn test_get_word_at_position_whitespace() {
        let doc = test_doc("fn foo() {}");

        // Space between fn and foo
        assert_eq!(get_word_at_position(&doc, Position::new(0, 2)), "");
    }

    #[test]
    fn test_get_word_range() {
        let doc = test_doc("let variable = 42");

        let range = get_word_range_at_position(&doc, Position::new(0, 6)).unwrap();
        assert_eq!(range.start.character, 4);
        assert_eq!(range.end.character, 12);
    }

    #[test]
    fn test_symbol_definition() {
        let uri = Url::parse("file:///test.aria").unwrap();
        let symbol = SymbolDefinition::new(
            "foo",
            SymbolKind::Function,
            uri.clone(),
            TextRange::new(Position::new(0, 0), Position::new(2, 1)),
            TextRange::new(Position::new(0, 3), Position::new(0, 6)),
        )
        .with_documentation("A test function")
        .public();

        assert_eq!(symbol.name, "foo");
        assert!(symbol.is_public);
        assert!(symbol.documentation.is_some());
    }

    #[test]
    fn test_scope_lookup() {
        let uri = Url::parse("file:///test.aria").unwrap();
        let mut parent = Scope::new();
        parent.define(SymbolDefinition::new(
            "x",
            SymbolKind::Variable,
            uri.clone(),
            TextRange::default(),
            TextRange::default(),
        ));

        let mut child = Scope::child(parent);
        child.define(SymbolDefinition::new(
            "y",
            SymbolKind::Variable,
            uri.clone(),
            TextRange::default(),
            TextRange::default(),
        ));

        // Can find both local and parent symbols
        assert!(child.lookup("y").is_some());
        assert!(child.lookup("x").is_some());
        assert!(child.lookup("z").is_none());
    }

    #[test]
    fn test_definition_resolver() {
        let uri = Url::parse("file:///test.aria").unwrap();
        let mut resolver = DefinitionResolver::new();

        resolver.register_local(SymbolDefinition::new(
            "my_func",
            SymbolKind::Function,
            uri.clone(),
            TextRange::new(Position::new(5, 0), Position::new(10, 1)),
            TextRange::new(Position::new(5, 3), Position::new(5, 10)),
        ));

        let symbol = resolver.resolve("my_func").unwrap();
        assert_eq!(symbol.kind, SymbolKind::Function);
    }

    #[test]
    fn test_extract_fn_name() {
        assert_eq!(extract_fn_name("pub fn hello() {}"), Some("hello".to_string()));
        assert_eq!(extract_fn_name_inner("fn world()"), Some("world".to_string()));
        assert_eq!(extract_fn_name_inner("let x = 1"), None);
    }

    #[test]
    fn test_extract_struct_name() {
        assert_eq!(extract_struct_name("struct Point { x: Int }"), Some("Point".to_string()));
        assert_eq!(extract_struct_name("pub struct Vector {}"), Some("Vector".to_string()));
    }

    #[test]
    fn test_extract_effect_name() {
        assert_eq!(extract_effect_name("effect Console { ... }"), Some("Console".to_string()));
    }

    #[test]
    fn test_extract_let_name() {
        assert_eq!(extract_let_name("let x = 42"), Some("x".to_string()));
        assert_eq!(extract_let_name("let mut counter = 0"), Some("counter".to_string()));
        assert_eq!(extract_let_name("fn foo()"), None);
    }

    #[test]
    fn test_extract_symbols_placeholder() {
        let doc = test_doc(
            r#"
fn main() {
    let x = 42;
}

struct Point {
    x: Int,
    y: Int,
}

effect Console {
    fn print(s: String);
}
"#,
        );

        let symbols = extract_symbols_placeholder(&doc);

        // Should find main, Point, Console, and x
        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"x"));
        assert!(names.contains(&"Point"));
        assert!(names.contains(&"Console"));
    }

    #[test]
    fn test_definition_result() {
        let uri = Url::parse("file:///test.aria").unwrap();

        // Test NotFound
        assert!(DefinitionResult::NotFound.is_empty());

        // Test Single
        let single = DefinitionResult::Single(Location::new(
            uri.clone(),
            TextRange::default(),
        ));
        assert!(!single.is_empty());

        // Test conversion to LSP
        let lsp = single.to_lsp();
        assert!(lsp.is_some());
    }

    #[test]
    fn test_symbol_reference() {
        let uri = Url::parse("file:///test.aria").unwrap();

        let read_ref = SymbolReference::read(uri.clone(), TextRange::default());
        assert!(!read_ref.is_write);
        assert!(!read_ref.is_definition);

        let write_ref = SymbolReference::write(uri.clone(), TextRange::default());
        assert!(write_ref.is_write);

        let def_ref = SymbolReference::definition(uri, TextRange::default());
        assert!(def_ref.is_definition);
    }
}
