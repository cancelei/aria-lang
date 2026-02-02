//! Document Symbol Handler
//!
//! This module handles the textDocument/documentSymbol request,
//! which provides the outline/structure of a document for navigation.

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;

use crate::AriaLanguageServer;

/// Handles the textDocument/documentSymbol request.
///
/// Returns a hierarchical list of symbols in the document,
/// enabling features like outline views and breadcrumbs.
pub async fn handle_document_symbol(
    server: &AriaLanguageServer,
    params: DocumentSymbolParams,
) -> Result<Option<DocumentSymbolResponse>> {
    let uri = params.text_document.uri;

    let doc = match server.state().get_document(&uri) {
        Some(doc) => doc,
        None => return Ok(None),
    };

    let content = doc.text();
    let symbols = extract_document_symbols(&content, &uri);

    if symbols.is_empty() {
        Ok(None)
    } else {
        Ok(Some(DocumentSymbolResponse::Nested(symbols)))
    }
}

/// Extracts symbols from document content.
fn extract_document_symbols(content: &str, uri: &Url) -> Vec<DocumentSymbol> {
    let mut symbols = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Function definitions
        if let Some(sym) = extract_function_symbol(trimmed, line_num as u32, line) {
            symbols.push(sym);
        }
        // Struct definitions
        else if let Some(sym) = extract_struct_symbol(trimmed, line_num as u32, line, content, uri) {
            symbols.push(sym);
        }
        // Enum definitions
        else if let Some(sym) = extract_enum_symbol(trimmed, line_num as u32, line, content, uri) {
            symbols.push(sym);
        }
        // Effect definitions
        else if let Some(sym) = extract_effect_symbol(trimmed, line_num as u32, line, content, uri) {
            symbols.push(sym);
        }
        // Trait definitions
        else if let Some(sym) = extract_trait_symbol(trimmed, line_num as u32, line, content, uri) {
            symbols.push(sym);
        }
        // Constant definitions
        else if let Some(sym) = extract_const_symbol(trimmed, line_num as u32, line) {
            symbols.push(sym);
        }
        // Top-level let bindings
        else if let Some(sym) = extract_let_symbol(trimmed, line_num as u32, line) {
            symbols.push(sym);
        }
        // Module declarations
        else if let Some(sym) = extract_module_symbol(trimmed, line_num as u32, line) {
            symbols.push(sym);
        }
        // Import statements
        else if let Some(sym) = extract_import_symbol(trimmed, line_num as u32, line) {
            symbols.push(sym);
        }
        // Type aliases
        else if let Some(sym) = extract_type_alias_symbol(trimmed, line_num as u32, line) {
            symbols.push(sym);
        }
    }

    symbols
}

fn extract_function_symbol(trimmed: &str, line_num: u32, line: &str) -> Option<DocumentSymbol> {
    // Handle pub fn, async fn, pub async fn
    let line_to_check = trimmed
        .strip_prefix("pub ")
        .unwrap_or(trimmed);
    let line_to_check = line_to_check
        .strip_prefix("async ")
        .unwrap_or(line_to_check);

    if !line_to_check.starts_with("fn ") {
        return None;
    }

    let rest = line_to_check.strip_prefix("fn ")?;
    let name_end = rest.find(|c: char| c == '(' || c == '<' || c.is_whitespace())?;
    let name = rest[..name_end].to_string();

    if name.is_empty() {
        return None;
    }

    let start_char = line.find("fn ")? as u32;
    let is_async = trimmed.contains("async ");
    let detail = if is_async {
        Some("async function".to_string())
    } else {
        Some("function".to_string())
    };

    #[allow(deprecated)]
    Some(DocumentSymbol {
        name,
        detail,
        kind: SymbolKind::FUNCTION,
        tags: None,
        deprecated: None,
        range: Range {
            start: Position { line: line_num, character: 0 },
            end: Position { line: line_num, character: line.len() as u32 },
        },
        selection_range: Range {
            start: Position { line: line_num, character: start_char },
            end: Position { line: line_num, character: line.len() as u32 },
        },
        children: None,
    })
}

fn extract_struct_symbol(
    trimmed: &str,
    line_num: u32,
    line: &str,
    content: &str,
    _uri: &Url,
) -> Option<DocumentSymbol> {
    let line_to_check = trimmed.strip_prefix("pub ").unwrap_or(trimmed);
    if !line_to_check.starts_with("struct ") {
        return None;
    }

    let rest = line_to_check.strip_prefix("struct ")?;
    let name_end = rest.find(|c: char| !c.is_alphanumeric() && c != '_')
        .unwrap_or(rest.len());
    let name = rest[..name_end].to_string();

    if name.is_empty() {
        return None;
    }

    // Find the end of the struct (look for "end")
    let end_line = find_end_line(content, line_num as usize);

    // Extract fields as children
    let children = extract_struct_fields(content, line_num as usize, end_line);

    let start_char = line.find("struct ")? as u32;

    #[allow(deprecated)]
    Some(DocumentSymbol {
        name,
        detail: Some("struct".to_string()),
        kind: SymbolKind::STRUCT,
        tags: None,
        deprecated: None,
        range: Range {
            start: Position { line: line_num, character: 0 },
            end: Position { line: end_line as u32, character: 3 }, // "end"
        },
        selection_range: Range {
            start: Position { line: line_num, character: start_char },
            end: Position { line: line_num, character: line.len() as u32 },
        },
        children: if children.is_empty() { None } else { Some(children) },
    })
}

fn extract_enum_symbol(
    trimmed: &str,
    line_num: u32,
    line: &str,
    content: &str,
    _uri: &Url,
) -> Option<DocumentSymbol> {
    let line_to_check = trimmed.strip_prefix("pub ").unwrap_or(trimmed);
    if !line_to_check.starts_with("enum ") {
        return None;
    }

    let rest = line_to_check.strip_prefix("enum ")?;
    let name_end = rest.find(|c: char| !c.is_alphanumeric() && c != '_')
        .unwrap_or(rest.len());
    let name = rest[..name_end].to_string();

    if name.is_empty() {
        return None;
    }

    let end_line = find_end_line(content, line_num as usize);
    let children = extract_enum_variants(content, line_num as usize, end_line);

    let start_char = line.find("enum ")? as u32;

    #[allow(deprecated)]
    Some(DocumentSymbol {
        name,
        detail: Some("enum".to_string()),
        kind: SymbolKind::ENUM,
        tags: None,
        deprecated: None,
        range: Range {
            start: Position { line: line_num, character: 0 },
            end: Position { line: end_line as u32, character: 3 },
        },
        selection_range: Range {
            start: Position { line: line_num, character: start_char },
            end: Position { line: line_num, character: line.len() as u32 },
        },
        children: if children.is_empty() { None } else { Some(children) },
    })
}

fn extract_effect_symbol(
    trimmed: &str,
    line_num: u32,
    line: &str,
    content: &str,
    _uri: &Url,
) -> Option<DocumentSymbol> {
    let line_to_check = trimmed.strip_prefix("pub ").unwrap_or(trimmed);
    if !line_to_check.starts_with("effect ") {
        return None;
    }

    let rest = line_to_check.strip_prefix("effect ")?;
    let name_end = rest.find(|c: char| !c.is_alphanumeric() && c != '_')
        .unwrap_or(rest.len());
    let name = rest[..name_end].to_string();

    if name.is_empty() {
        return None;
    }

    let end_line = find_end_line(content, line_num as usize);
    let children = extract_effect_operations(content, line_num as usize, end_line);

    let start_char = line.find("effect ")? as u32;

    #[allow(deprecated)]
    Some(DocumentSymbol {
        name,
        detail: Some("effect".to_string()),
        kind: SymbolKind::INTERFACE, // Effects are similar to interfaces
        tags: None,
        deprecated: None,
        range: Range {
            start: Position { line: line_num, character: 0 },
            end: Position { line: end_line as u32, character: 3 },
        },
        selection_range: Range {
            start: Position { line: line_num, character: start_char },
            end: Position { line: line_num, character: line.len() as u32 },
        },
        children: if children.is_empty() { None } else { Some(children) },
    })
}

fn extract_trait_symbol(
    trimmed: &str,
    line_num: u32,
    line: &str,
    content: &str,
    _uri: &Url,
) -> Option<DocumentSymbol> {
    let line_to_check = trimmed.strip_prefix("pub ").unwrap_or(trimmed);
    if !line_to_check.starts_with("trait ") {
        return None;
    }

    let rest = line_to_check.strip_prefix("trait ")?;
    let name_end = rest.find(|c: char| !c.is_alphanumeric() && c != '_')
        .unwrap_or(rest.len());
    let name = rest[..name_end].to_string();

    if name.is_empty() {
        return None;
    }

    let end_line = find_end_line(content, line_num as usize);
    let children = extract_trait_members(content, line_num as usize, end_line);

    let start_char = line.find("trait ")? as u32;

    #[allow(deprecated)]
    Some(DocumentSymbol {
        name,
        detail: Some("trait".to_string()),
        kind: SymbolKind::INTERFACE,
        tags: None,
        deprecated: None,
        range: Range {
            start: Position { line: line_num, character: 0 },
            end: Position { line: end_line as u32, character: 3 },
        },
        selection_range: Range {
            start: Position { line: line_num, character: start_char },
            end: Position { line: line_num, character: line.len() as u32 },
        },
        children: if children.is_empty() { None } else { Some(children) },
    })
}

fn extract_const_symbol(trimmed: &str, line_num: u32, line: &str) -> Option<DocumentSymbol> {
    let line_to_check = trimmed.strip_prefix("pub ").unwrap_or(trimmed);
    if !line_to_check.starts_with("const ") {
        return None;
    }

    let rest = line_to_check.strip_prefix("const ")?;
    let name_end = rest.find(|c: char| c == ':' || c == '=' || c.is_whitespace())?;
    let name = rest[..name_end].to_string();

    if name.is_empty() {
        return None;
    }

    let start_char = line.find("const ")? as u32;

    #[allow(deprecated)]
    Some(DocumentSymbol {
        name,
        detail: Some("constant".to_string()),
        kind: SymbolKind::CONSTANT,
        tags: None,
        deprecated: None,
        range: Range {
            start: Position { line: line_num, character: 0 },
            end: Position { line: line_num, character: line.len() as u32 },
        },
        selection_range: Range {
            start: Position { line: line_num, character: start_char },
            end: Position { line: line_num, character: line.len() as u32 },
        },
        children: None,
    })
}

fn extract_let_symbol(trimmed: &str, line_num: u32, line: &str) -> Option<DocumentSymbol> {
    // Only extract top-level let bindings (those that don't have leading whitespace in original line)
    if !trimmed.starts_with("let ") {
        return None;
    }

    // Skip if this looks like it's inside a function (check indentation)
    let indent = line.len() - line.trim_start().len();
    if indent > 0 {
        return None;
    }

    let rest = trimmed.strip_prefix("let ")?;
    let rest = rest.strip_prefix("mut ").unwrap_or(rest);
    let name_end = rest.find(|c: char| c == ':' || c == '=' || c.is_whitespace())?;
    let name = rest[..name_end].to_string();

    if name.is_empty() {
        return None;
    }

    #[allow(deprecated)]
    Some(DocumentSymbol {
        name,
        detail: Some("variable".to_string()),
        kind: SymbolKind::VARIABLE,
        tags: None,
        deprecated: None,
        range: Range {
            start: Position { line: line_num, character: 0 },
            end: Position { line: line_num, character: line.len() as u32 },
        },
        selection_range: Range {
            start: Position { line: line_num, character: 0 },
            end: Position { line: line_num, character: line.len() as u32 },
        },
        children: None,
    })
}

fn extract_module_symbol(trimmed: &str, line_num: u32, line: &str) -> Option<DocumentSymbol> {
    let line_to_check = trimmed.strip_prefix("pub ").unwrap_or(trimmed);
    if !line_to_check.starts_with("mod ") {
        return None;
    }

    let rest = line_to_check.strip_prefix("mod ")?;
    let name_end = rest.find(|c: char| !c.is_alphanumeric() && c != '_')
        .unwrap_or(rest.len());
    let name = rest[..name_end].to_string();

    if name.is_empty() {
        return None;
    }

    let start_char = line.find("mod ")? as u32;

    #[allow(deprecated)]
    Some(DocumentSymbol {
        name,
        detail: Some("module".to_string()),
        kind: SymbolKind::MODULE,
        tags: None,
        deprecated: None,
        range: Range {
            start: Position { line: line_num, character: 0 },
            end: Position { line: line_num, character: line.len() as u32 },
        },
        selection_range: Range {
            start: Position { line: line_num, character: start_char },
            end: Position { line: line_num, character: line.len() as u32 },
        },
        children: None,
    })
}

fn extract_import_symbol(trimmed: &str, line_num: u32, line: &str) -> Option<DocumentSymbol> {
    if !trimmed.starts_with("import ") && !trimmed.starts_with("use ") {
        return None;
    }

    let rest = trimmed.strip_prefix("import ")
        .or_else(|| trimmed.strip_prefix("use "))?;

    // Get the module path
    let path_end = rest.find(|c: char| c == '{' || c == '\n' || c == ';')
        .unwrap_or(rest.len());
    let name = rest[..path_end].trim().to_string();

    if name.is_empty() {
        return None;
    }

    #[allow(deprecated)]
    Some(DocumentSymbol {
        name,
        detail: Some("import".to_string()),
        kind: SymbolKind::NAMESPACE,
        tags: None,
        deprecated: None,
        range: Range {
            start: Position { line: line_num, character: 0 },
            end: Position { line: line_num, character: line.len() as u32 },
        },
        selection_range: Range {
            start: Position { line: line_num, character: 0 },
            end: Position { line: line_num, character: line.len() as u32 },
        },
        children: None,
    })
}

fn extract_type_alias_symbol(trimmed: &str, line_num: u32, line: &str) -> Option<DocumentSymbol> {
    let line_to_check = trimmed.strip_prefix("pub ").unwrap_or(trimmed);
    if !line_to_check.starts_with("type ") {
        return None;
    }

    let rest = line_to_check.strip_prefix("type ")?;
    let name_end = rest.find(|c: char| c == '=' || c == '<' || c.is_whitespace())?;
    let name = rest[..name_end].to_string();

    if name.is_empty() {
        return None;
    }

    let start_char = line.find("type ")? as u32;

    #[allow(deprecated)]
    Some(DocumentSymbol {
        name,
        detail: Some("type alias".to_string()),
        kind: SymbolKind::TYPE_PARAMETER,
        tags: None,
        deprecated: None,
        range: Range {
            start: Position { line: line_num, character: 0 },
            end: Position { line: line_num, character: line.len() as u32 },
        },
        selection_range: Range {
            start: Position { line: line_num, character: start_char },
            end: Position { line: line_num, character: line.len() as u32 },
        },
        children: None,
    })
}

/// Find the line number of the matching "end" keyword
fn find_end_line(content: &str, start_line: usize) -> usize {
    let lines: Vec<&str> = content.lines().collect();
    let mut depth = 1;

    for (i, line) in lines.iter().enumerate().skip(start_line + 1) {
        let trimmed = line.trim();

        // Track nesting depth
        if trimmed.starts_with("fn ")
            || trimmed.starts_with("struct ")
            || trimmed.starts_with("enum ")
            || trimmed.starts_with("effect ")
            || trimmed.starts_with("trait ")
            || trimmed.starts_with("if ")
            || trimmed.starts_with("match ")
            || trimmed.starts_with("for ")
            || trimmed.starts_with("while ")
            || trimmed.starts_with("handler ")
            || trimmed.starts_with("impl ")
            || trimmed.starts_with("pub fn ")
            || trimmed.starts_with("pub struct ")
            || trimmed.starts_with("pub enum ")
            || trimmed.starts_with("pub effect ")
            || trimmed.starts_with("pub trait ")
            || trimmed.starts_with("async fn ")
            || trimmed.starts_with("pub async fn ")
        {
            depth += 1;
        } else if trimmed == "end" {
            depth -= 1;
            if depth == 0 {
                return i;
            }
        }
    }

    // If no matching end found, return the last line
    lines.len().saturating_sub(1)
}

/// Extract struct fields as child symbols
fn extract_struct_fields(content: &str, start_line: usize, end_line: usize) -> Vec<DocumentSymbol> {
    let lines: Vec<&str> = content.lines().collect();
    let mut fields = Vec::new();

    for i in (start_line + 1)..end_line {
        if i >= lines.len() {
            break;
        }
        let line = lines[i];
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("#") {
            continue;
        }

        // Look for field pattern: name: Type
        if let Some(colon_pos) = trimmed.find(':') {
            let name = trimmed[..colon_pos].trim();
            let name = name.strip_prefix("pub ").unwrap_or(name);

            if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                #[allow(deprecated)]
                fields.push(DocumentSymbol {
                    name: name.to_string(),
                    detail: Some("field".to_string()),
                    kind: SymbolKind::FIELD,
                    tags: None,
                    deprecated: None,
                    range: Range {
                        start: Position { line: i as u32, character: 0 },
                        end: Position { line: i as u32, character: line.len() as u32 },
                    },
                    selection_range: Range {
                        start: Position { line: i as u32, character: 0 },
                        end: Position { line: i as u32, character: line.len() as u32 },
                    },
                    children: None,
                });
            }
        }
    }

    fields
}

/// Extract enum variants as child symbols
fn extract_enum_variants(content: &str, start_line: usize, end_line: usize) -> Vec<DocumentSymbol> {
    let lines: Vec<&str> = content.lines().collect();
    let mut variants = Vec::new();

    for i in (start_line + 1)..end_line {
        if i >= lines.len() {
            break;
        }
        let line = lines[i];
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("#") {
            continue;
        }

        // Variant pattern: Name or Name(Type) or Name { field: Type }
        let name_end = trimmed.find(|c: char| c == '(' || c == '{' || c == ',' || c.is_whitespace())
            .unwrap_or(trimmed.len());
        let name = &trimmed[..name_end];

        if !name.is_empty() && name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
            #[allow(deprecated)]
            variants.push(DocumentSymbol {
                name: name.to_string(),
                detail: Some("variant".to_string()),
                kind: SymbolKind::ENUM_MEMBER,
                tags: None,
                deprecated: None,
                range: Range {
                    start: Position { line: i as u32, character: 0 },
                    end: Position { line: i as u32, character: line.len() as u32 },
                },
                selection_range: Range {
                    start: Position { line: i as u32, character: 0 },
                    end: Position { line: i as u32, character: line.len() as u32 },
                },
                children: None,
            });
        }
    }

    variants
}

/// Extract effect operations as child symbols
fn extract_effect_operations(content: &str, start_line: usize, end_line: usize) -> Vec<DocumentSymbol> {
    let lines: Vec<&str> = content.lines().collect();
    let mut operations = Vec::new();

    for i in (start_line + 1)..end_line {
        if i >= lines.len() {
            break;
        }
        let line = lines[i];
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("#") {
            continue;
        }

        // Operation pattern: fn name(...) -> Type
        if trimmed.starts_with("fn ") {
            let rest = &trimmed[3..];
            if let Some(name_end) = rest.find(|c: char| c == '(' || c == '<' || c.is_whitespace()) {
                let name = rest[..name_end].to_string();
                if !name.is_empty() {
                    #[allow(deprecated)]
                    operations.push(DocumentSymbol {
                        name,
                        detail: Some("operation".to_string()),
                        kind: SymbolKind::METHOD,
                        tags: None,
                        deprecated: None,
                        range: Range {
                            start: Position { line: i as u32, character: 0 },
                            end: Position { line: i as u32, character: line.len() as u32 },
                        },
                        selection_range: Range {
                            start: Position { line: i as u32, character: 0 },
                            end: Position { line: i as u32, character: line.len() as u32 },
                        },
                        children: None,
                    });
                }
            }
        }
    }

    operations
}

/// Extract trait members as child symbols
fn extract_trait_members(content: &str, start_line: usize, end_line: usize) -> Vec<DocumentSymbol> {
    let lines: Vec<&str> = content.lines().collect();
    let mut members = Vec::new();

    for i in (start_line + 1)..end_line {
        if i >= lines.len() {
            break;
        }
        let line = lines[i];
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("#") {
            continue;
        }

        // Method pattern: fn name(...)
        if trimmed.starts_with("fn ") {
            let rest = &trimmed[3..];
            if let Some(name_end) = rest.find(|c: char| c == '(' || c == '<' || c.is_whitespace()) {
                let name = rest[..name_end].to_string();
                if !name.is_empty() {
                    #[allow(deprecated)]
                    members.push(DocumentSymbol {
                        name,
                        detail: Some("method".to_string()),
                        kind: SymbolKind::METHOD,
                        tags: None,
                        deprecated: None,
                        range: Range {
                            start: Position { line: i as u32, character: 0 },
                            end: Position { line: i as u32, character: line.len() as u32 },
                        },
                        selection_range: Range {
                            start: Position { line: i as u32, character: 0 },
                            end: Position { line: i as u32, character: line.len() as u32 },
                        },
                        children: None,
                    });
                }
            }
        }
        // Associated type pattern: type Name
        else if trimmed.starts_with("type ") {
            let rest = &trimmed[5..];
            let name_end = rest.find(|c: char| c == '=' || c == ':' || c.is_whitespace())
                .unwrap_or(rest.len());
            let name = rest[..name_end].to_string();
            if !name.is_empty() {
                #[allow(deprecated)]
                members.push(DocumentSymbol {
                    name,
                    detail: Some("associated type".to_string()),
                    kind: SymbolKind::TYPE_PARAMETER,
                    tags: None,
                    deprecated: None,
                    range: Range {
                        start: Position { line: i as u32, character: 0 },
                        end: Position { line: i as u32, character: line.len() as u32 },
                    },
                    selection_range: Range {
                        start: Position { line: i as u32, character: 0 },
                        end: Position { line: i as u32, character: line.len() as u32 },
                    },
                    children: None,
                });
            }
        }
    }

    members
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_uri() -> Url {
        Url::parse("file:///test.aria").unwrap()
    }

    #[test]
    fn test_extract_function_symbols() {
        let content = r#"
fn simple()
    pass
end

pub fn public_fn(x: Int) -> Int
    x
end

async fn async_fn()
    pass
end
"#;

        let symbols = extract_document_symbols(content, &test_uri());
        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();

        assert!(names.contains(&"simple"));
        assert!(names.contains(&"public_fn"));
        assert!(names.contains(&"async_fn"));
    }

    #[test]
    fn test_extract_struct_symbols() {
        let content = r#"
struct Point
    x: Int
    y: Int
end

pub struct Person
    name: String
    age: Int
end
"#;

        let symbols = extract_document_symbols(content, &test_uri());

        let point = symbols.iter().find(|s| s.name == "Point");
        assert!(point.is_some());

        let point = point.unwrap();
        assert_eq!(point.kind, SymbolKind::STRUCT);

        // Check for field children
        let children = point.children.as_ref();
        assert!(children.is_some());
        let children = children.unwrap();
        assert!(children.iter().any(|c| c.name == "x"));
        assert!(children.iter().any(|c| c.name == "y"));
    }

    #[test]
    fn test_extract_enum_symbols() {
        let content = r#"
enum Color
    Red
    Green
    Blue
end
"#;

        let symbols = extract_document_symbols(content, &test_uri());

        let color = symbols.iter().find(|s| s.name == "Color");
        assert!(color.is_some());

        let color = color.unwrap();
        assert_eq!(color.kind, SymbolKind::ENUM);

        let children = color.children.as_ref().unwrap();
        assert!(children.iter().any(|c| c.name == "Red"));
        assert!(children.iter().any(|c| c.name == "Green"));
        assert!(children.iter().any(|c| c.name == "Blue"));
    }

    #[test]
    fn test_extract_effect_symbols() {
        let content = r#"
effect Console
    fn print(msg: String)
    fn read_line() -> String
end
"#;

        let symbols = extract_document_symbols(content, &test_uri());

        let console = symbols.iter().find(|s| s.name == "Console");
        assert!(console.is_some());

        let console = console.unwrap();
        assert_eq!(console.kind, SymbolKind::INTERFACE);

        let children = console.children.as_ref().unwrap();
        assert!(children.iter().any(|c| c.name == "print"));
        assert!(children.iter().any(|c| c.name == "read_line"));
    }

    #[test]
    fn test_extract_trait_symbols() {
        let content = r#"
trait Drawable
    type Color
    fn draw(self)
end
"#;

        let symbols = extract_document_symbols(content, &test_uri());

        let drawable = symbols.iter().find(|s| s.name == "Drawable");
        assert!(drawable.is_some());

        let drawable = drawable.unwrap();
        let children = drawable.children.as_ref().unwrap();
        assert!(children.iter().any(|c| c.name == "Color"));
        assert!(children.iter().any(|c| c.name == "draw"));
    }

    #[test]
    fn test_extract_const_and_type_alias() {
        let content = r#"
const MAX_SIZE = 100
type StringList = List[String]
"#;

        let symbols = extract_document_symbols(content, &test_uri());

        assert!(symbols.iter().any(|s| s.name == "MAX_SIZE" && s.kind == SymbolKind::CONSTANT));
        assert!(symbols.iter().any(|s| s.name == "StringList" && s.kind == SymbolKind::TYPE_PARAMETER));
    }
}
