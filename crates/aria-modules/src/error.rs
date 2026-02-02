//! Module system errors

use aria_parser::ParseError;
use std::path::PathBuf;
use thiserror::Error;
use crate::resolver::ModuleId;

/// Result type for module operations
pub type ModuleResult<T> = Result<T, ModuleError>;

/// Errors that can occur during module resolution and compilation
#[derive(Debug, Error)]
pub enum ModuleError {
    /// A module could not be found
    #[error("module not found: {0}")]
    ModuleNotFound(String),

    /// A file could not be found
    #[error("file not found: {}", .0.display())]
    FileNotFound(PathBuf),

    /// IO error while reading a module file
    #[error("IO error reading {}: {1}", .0.display())]
    IoError(PathBuf, #[source] std::io::Error),

    /// Parse errors in a module
    #[error("parse errors in {}: {} error(s)", .path.display(), .errors.len())]
    ParseError {
        path: PathBuf,
        errors: Vec<ParseError>,
    },

    /// Circular dependency detected (with module IDs only)
    #[error("circular dependency detected: {}", format_cycle_ids(.0))]
    CircularDependency(Vec<ModuleId>),

    /// Circular dependency detected (with module names for better error messages)
    #[error("circular dependency detected: {}", format_cycle_names(.0))]
    CircularDependencyNamed(Vec<String>),

    /// Import resolution failed
    #[error("failed to resolve import '{0}'")]
    ImportResolutionFailed(String),

    /// Imported item not found in module
    #[error("item '{item}' not found in module '{module}'")]
    ItemNotFound {
        module: String,
        item: String,
    },

    /// Attempted to import a private item
    #[error("item '{item}' is private in module '{module}'")]
    PrivateItem {
        module: String,
        item: String,
    },

    /// Conflicting imports
    #[error("conflicting imports for name '{0}'")]
    ConflictingImports(String),

    /// Module name conflict
    #[error("module name conflict: {0}")]
    NameConflict(String),
}

/// Format a cycle with module IDs for error messages
fn format_cycle_ids(cycle: &[ModuleId]) -> String {
    if cycle.is_empty() {
        return String::from("(empty cycle)");
    }

    let parts: Vec<_> = cycle.iter()
        .map(|id| format!("{:?}", id))
        .collect();

    parts.join(" -> ")
}

/// Format a cycle with module names for error messages
fn format_cycle_names(cycle: &[String]) -> String {
    if cycle.is_empty() {
        return String::from("(empty cycle)");
    }

    cycle.join(" -> ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_not_found_error() {
        let err = ModuleError::ModuleNotFound("std::collections".to_string());
        let msg = err.to_string();
        assert!(msg.contains("module not found"));
        assert!(msg.contains("std::collections"));
    }

    #[test]
    fn test_circular_dependency_error() {
        let cycle = vec![ModuleId(0), ModuleId(1), ModuleId(2), ModuleId(0)];
        let err = ModuleError::CircularDependency(cycle);
        let msg = err.to_string();
        assert!(msg.contains("circular dependency"));
    }

    #[test]
    fn test_private_item_error() {
        let err = ModuleError::PrivateItem {
            module: "utils".to_string(),
            item: "helper".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("private"));
        assert!(msg.contains("helper"));
        assert!(msg.contains("utils"));
    }

    #[test]
    fn test_format_cycle_ids() {
        let cycle = vec![ModuleId(0), ModuleId(1), ModuleId(0)];
        let formatted = format_cycle_ids(&cycle);
        assert!(formatted.contains("->"));
    }

    #[test]
    fn test_format_cycle_names() {
        let cycle = vec!["a".to_string(), "b".to_string(), "a".to_string()];
        let formatted = format_cycle_names(&cycle);
        assert_eq!(formatted, "a -> b -> a");
    }

    #[test]
    fn test_format_empty_cycle() {
        let cycle: Vec<ModuleId> = vec![];
        let formatted = format_cycle_ids(&cycle);
        assert_eq!(formatted, "(empty cycle)");
    }

    #[test]
    fn test_circular_dependency_named_error() {
        let cycle = vec!["utils".to_string(), "helpers".to_string(), "utils".to_string()];
        let err = ModuleError::CircularDependencyNamed(cycle);
        let msg = err.to_string();
        assert!(msg.contains("circular dependency"));
        assert!(msg.contains("utils -> helpers -> utils"));
    }
}
