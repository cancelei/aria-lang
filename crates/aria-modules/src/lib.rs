//! Aria Module System
//!
//! Provides module resolution, dependency tracking, and import/export handling.

pub mod resolver;
pub mod graph;
pub mod cache;
pub mod error;

pub use resolver::{ModuleResolver, FileSystemResolver, ModuleId, ResolvedModule};
pub use graph::{ModuleGraph, DependencyEdge};
pub use cache::ModuleCache;
pub use error::{ModuleError, ModuleResult};

use aria_ast::{Program, ImportDecl, Item, Visibility};
use rustc_hash::FxHashSet;
use smol_str::SmolStr;
use std::path::PathBuf;

/// Module compilation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilationMode {
    /// Compile as a library
    Library,
    /// Compile as a binary/executable
    Binary,
}

/// A fully resolved module with its dependencies
#[derive(Debug, Clone)]
pub struct Module {
    /// Unique identifier for this module
    pub id: ModuleId,
    /// The parsed AST
    pub ast: Program,
    /// Source file path
    pub path: PathBuf,
    /// Module name (derived from path)
    pub name: SmolStr,
    /// Direct dependencies (modules this one imports)
    pub dependencies: Vec<ModuleId>,
    /// Public items exported by this module
    pub exports: FxHashSet<SmolStr>,
    /// Private items (not exported)
    pub private_items: FxHashSet<SmolStr>,
}

impl Module {
    /// Create a new module from a parsed program
    pub fn new(id: ModuleId, ast: Program, path: PathBuf, name: SmolStr) -> Self {
        let mut exports = FxHashSet::default();
        let mut private_items = FxHashSet::default();

        // Collect exported and private items
        for item in &ast.items {
            match item {
                Item::Export(export) => {
                    // Handle export declarations
                    match &export.selection {
                        aria_ast::ExportSelection::All => {
                            // Export all public items
                            for item in &ast.items {
                                if let Some(name) = get_item_name(item) {
                                    if is_public_item(item) {
                                        exports.insert(name);
                                    }
                                }
                            }
                        }
                        aria_ast::ExportSelection::Items(items) => {
                            for item in items {
                                exports.insert(item.node.clone());
                            }
                        }
                    }
                }
                _ => {
                    if let Some(name) = get_item_name(item) {
                        if is_public_item(item) {
                            exports.insert(name.clone());
                        } else {
                            private_items.insert(name);
                        }
                    }
                }
            }
        }

        Self {
            id,
            ast,
            path,
            name,
            dependencies: Vec::new(),
            exports,
            private_items,
        }
    }

    /// Check if an item is exported
    pub fn is_exported(&self, name: &str) -> bool {
        self.exports.contains(name)
    }

    /// Get all imported modules
    pub fn get_imports(&self) -> Vec<ImportDecl> {
        self.ast
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Import(import) => Some(import.clone()),
                _ => None,
            })
            .collect()
    }
}

/// Get the name of an item if it has one
fn get_item_name(item: &Item) -> Option<SmolStr> {
    match item {
        Item::Function(f) => Some(f.name.node.clone()),
        Item::Struct(s) => Some(s.name.node.clone()),
        Item::Data(d) => Some(d.name.node.clone()),
        Item::Enum(e) => Some(e.name.node.clone()),
        Item::Trait(t) => Some(t.name.node.clone()),
        Item::Const(c) => Some(c.name.node.clone()),
        Item::TypeAlias(t) => Some(t.name.node.clone()),
        Item::Module(m) => m.path.last().map(|p| p.node.clone()),
        Item::Use(u) => {
            // For re-exports, use the alias if present, otherwise the last path segment
            u.alias.as_ref().map(|a| a.node.clone())
                .or_else(|| u.path.last().map(|p| p.node.clone()))
        }
        _ => None,
    }
}

/// Check if an item is public
fn is_public_item(item: &Item) -> bool {
    match item {
        Item::Function(f) => f.visibility == Visibility::Public,
        Item::Struct(s) => s.visibility == Visibility::Public,
        Item::Data(d) => d.visibility == Visibility::Public,
        Item::Enum(e) => e.visibility == Visibility::Public,
        Item::Trait(t) => t.visibility == Visibility::Public,
        Item::Const(c) => c.visibility == Visibility::Public,
        Item::TypeAlias(t) => t.visibility == Visibility::Public,
        Item::Use(u) => u.visibility == Visibility::Public,
        _ => false,
    }
}

/// Module compilation context
#[allow(dead_code)]
pub struct ModuleCompiler {
    resolver: Box<dyn ModuleResolver>,
    graph: ModuleGraph,
    cache: ModuleCache,
    mode: CompilationMode,
}

impl ModuleCompiler {
    /// Create a new module compiler with the given resolver
    pub fn new(resolver: Box<dyn ModuleResolver>, mode: CompilationMode) -> Self {
        Self {
            resolver,
            graph: ModuleGraph::new(),
            cache: ModuleCache::new(),
            mode,
        }
    }

    /// Compile a module and all its dependencies
    pub fn compile(&mut self, entry_point: &PathBuf) -> ModuleResult<Vec<Module>> {
        // Resolve the entry point
        let entry_id = self.resolver.resolve_path(entry_point)?;
        let resolved = self.resolver.load(entry_id)?;

        // Parse and process the entry module
        let entry_module = self.process_module(entry_id, resolved)?;

        // Build dependency graph
        self.build_dependency_graph(&entry_module)?;

        // Detect circular dependencies
        if let Some(cycle) = self.graph.detect_cycles() {
            // Convert module IDs to names for better error messages
            let cycle_names: Vec<String> = cycle.iter()
                .filter_map(|id| self.cache.get(*id).map(|m| m.name.to_string()))
                .collect();

            if cycle_names.len() == cycle.len() {
                return Err(ModuleError::CircularDependencyNamed(cycle_names));
            } else {
                // Fallback to ID-based error if names couldn't be resolved
                return Err(ModuleError::CircularDependency(cycle));
            }
        }

        // Topologically sort modules (dependencies first)
        let sorted_ids = self.graph.topological_sort();

        // Collect all modules in dependency order
        let mut modules = Vec::new();
        for id in sorted_ids {
            if let Some(module) = self.cache.get(id) {
                modules.push(module.clone());
            }
        }

        Ok(modules)
    }

    /// Process a single module
    fn process_module(&mut self, id: ModuleId, resolved: ResolvedModule) -> ModuleResult<Module> {
        // Check cache first
        if let Some(cached) = self.cache.get(id) {
            return Ok(cached.clone());
        }

        // Parse the module
        let (ast, parse_errors) = aria_parser::parse(&resolved.source);

        if !parse_errors.is_empty() {
            return Err(ModuleError::ParseError {
                path: resolved.path.clone(),
                errors: parse_errors,
            });
        }

        // Create module
        let module = Module::new(id, ast, resolved.path, resolved.name);

        // Cache it
        self.cache.insert(id, module.clone());

        Ok(module)
    }

    /// Build the complete dependency graph
    fn build_dependency_graph(&mut self, entry: &Module) -> ModuleResult<()> {
        let mut to_process = vec![entry.id];
        let mut processed = FxHashSet::default();

        // Always add the entry module to the graph
        self.graph.add_module(entry.id);

        while let Some(current_id) = to_process.pop() {
            if !processed.insert(current_id) {
                continue;
            }

            let current = self.cache.get(current_id)
                .ok_or_else(|| ModuleError::ModuleNotFound(format!("Module {:?}", current_id)))?
                .clone();

            // Process imports
            for import in current.get_imports() {
                let dep_id = self.resolve_import(&current, &import)?;

                // Add edge to graph
                self.graph.add_dependency(current_id, dep_id);

                // Load and process the dependency
                if !self.cache.contains(dep_id) {
                    let resolved = self.resolver.load(dep_id)?;
                    self.process_module(dep_id, resolved)?;
                }

                // Queue for processing
                to_process.push(dep_id);
            }
        }

        Ok(())
    }

    /// Resolve an import declaration to a module ID
    fn resolve_import(&mut self, current: &Module, import: &ImportDecl) -> ModuleResult<ModuleId> {
        match &import.path {
            aria_ast::ImportPath::Module(segments) => {
                // Convert path segments to module name
                let module_name: Vec<_> = segments.iter()
                    .map(|s| s.node.as_str())
                    .collect();
                let module_path = module_name.join("/");

                self.resolver.resolve(&module_path, Some(&current.path))
            }
            aria_ast::ImportPath::String(path) => {
                self.resolver.resolve(path.as_str(), Some(&current.path))
            }
        }
    }

    /// Get the module graph
    pub fn graph(&self) -> &ModuleGraph {
        &self.graph
    }

    /// Get the module cache
    pub fn cache(&self) -> &ModuleCache {
        &self.cache
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_creation() {
        let source = r#"
            pub fn add(x: Int, y: Int) -> Int = x + y
            fn private_helper() = 42
        "#;

        let (ast, errors) = aria_parser::parse(source);
        assert!(errors.is_empty());

        let module = Module::new(
            ModuleId(0),
            ast,
            PathBuf::from("test.aria"),
            SmolStr::new("test"),
        );

        assert_eq!(module.name, "test");
        assert_eq!(module.id, ModuleId(0));
    }
}
