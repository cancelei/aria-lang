//! Module resolution
//!
//! Responsible for finding and loading module files from the filesystem.

use crate::error::{ModuleError, ModuleResult};
use rustc_hash::FxHashMap;
use smol_str::SmolStr;
use std::path::{Path, PathBuf};
use std::fs;

/// Unique identifier for a module
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModuleId(pub usize);

/// A resolved module with its source code
#[derive(Debug, Clone)]
pub struct ResolvedModule {
    /// Module ID
    pub id: ModuleId,
    /// Full path to the module file
    pub path: PathBuf,
    /// Module name (e.g., "std::collections::HashMap" -> "HashMap")
    pub name: SmolStr,
    /// Source code
    pub source: String,
}

/// Trait for resolving module paths to actual files
pub trait ModuleResolver: Send {
    /// Resolve a module name to a module ID
    ///
    /// # Arguments
    /// * `name` - Module name (e.g., "std::collections::HashMap")
    /// * `current_path` - Optional path of the importing module (for relative imports)
    fn resolve(&mut self, name: &str, current_path: Option<&Path>) -> ModuleResult<ModuleId>;

    /// Resolve a file path to a module ID
    fn resolve_path(&mut self, path: &Path) -> ModuleResult<ModuleId>;

    /// Load a module by ID
    fn load(&mut self, id: ModuleId) -> ModuleResult<ResolvedModule>;
}

/// File system-based module resolver
pub struct FileSystemResolver {
    /// Root directories to search for modules
    search_paths: Vec<PathBuf>,
    /// Maps module IDs to file paths
    id_to_path: FxHashMap<ModuleId, PathBuf>,
    /// Maps file paths to module IDs
    path_to_id: FxHashMap<PathBuf, ModuleId>,
    /// Next module ID to assign
    next_id: usize,
}

impl FileSystemResolver {
    /// Create a new filesystem resolver
    pub fn new() -> Self {
        let mut search_paths = vec![PathBuf::from(".")];

        // Add stdlib path - try several common locations
        // 1. Relative to current executable (for installed version)
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let stdlib_path = exe_dir.join("../stdlib");
                if stdlib_path.exists() {
                    search_paths.push(stdlib_path);
                }
            }
        }

        // 2. Relative to project root (for development)
        let project_stdlib = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../stdlib");
        if project_stdlib.exists() {
            search_paths.push(project_stdlib);
        }

        // 3. System-wide installation
        let system_stdlib = PathBuf::from("/usr/local/lib/aria/stdlib");
        if system_stdlib.exists() {
            search_paths.push(system_stdlib);
        }

        Self {
            search_paths,
            id_to_path: FxHashMap::default(),
            path_to_id: FxHashMap::default(),
            next_id: 0,
        }
    }

    /// Add a search path
    pub fn add_search_path(&mut self, path: impl Into<PathBuf>) {
        self.search_paths.push(path.into());
    }

    /// Get or create a module ID for a path
    fn get_or_create_id(&mut self, path: PathBuf) -> ModuleId {
        if let Some(&id) = self.path_to_id.get(&path) {
            return id;
        }

        let id = ModuleId(self.next_id);
        self.next_id += 1;
        self.id_to_path.insert(id, path.clone());
        self.path_to_id.insert(path, id);
        id
    }

    /// Find a module file in the search paths
    fn find_module(&self, name: &str, current_path: Option<&Path>) -> ModuleResult<PathBuf> {
        // Convert module name to file path
        // e.g., "std::collections::HashMap" -> "std/collections/HashMap.aria"
        let file_path = name.replace("::", "/") + ".aria";

        // Try relative to current file first
        if let Some(current) = current_path {
            if let Some(parent) = current.parent() {
                let candidate = parent.join(&file_path);
                if candidate.exists() {
                    return Ok(candidate.canonicalize().map_err(|e| {
                        ModuleError::IoError(candidate.clone(), e)
                    })?);
                }
            }
        }

        // Try each search path
        for search_path in &self.search_paths {
            let candidate = search_path.join(&file_path);
            if candidate.exists() {
                return Ok(candidate.canonicalize().map_err(|e| {
                    ModuleError::IoError(candidate.clone(), e)
                })?);
            }

            // Also try as a directory with mod.aria
            let dir_candidate = search_path.join(name.replace("::", "/")).join("mod.aria");
            if dir_candidate.exists() {
                return Ok(dir_candidate.canonicalize().map_err(|e| {
                    ModuleError::IoError(dir_candidate.clone(), e)
                })?);
            }
        }

        Err(ModuleError::ModuleNotFound(name.to_string()))
    }

    /// Extract module name from a file path
    fn path_to_module_name(&self, path: &Path) -> SmolStr {
        // Get the file stem (filename without extension)
        let file_stem = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        // If it's "mod.aria", use the parent directory name
        if file_stem == "mod" {
            if let Some(parent) = path.parent() {
                if let Some(dir_name) = parent.file_name().and_then(|s| s.to_str()) {
                    return SmolStr::new(dir_name);
                }
            }
        }

        SmolStr::new(file_stem)
    }
}

impl Default for FileSystemResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleResolver for FileSystemResolver {
    fn resolve(&mut self, name: &str, current_path: Option<&Path>) -> ModuleResult<ModuleId> {
        let path = self.find_module(name, current_path)?;
        Ok(self.get_or_create_id(path))
    }

    fn resolve_path(&mut self, path: &Path) -> ModuleResult<ModuleId> {
        let canonical_path = path.canonicalize()
            .map_err(|e| ModuleError::IoError(path.to_path_buf(), e))?;

        if !canonical_path.exists() {
            return Err(ModuleError::FileNotFound(canonical_path));
        }

        Ok(self.get_or_create_id(canonical_path))
    }

    fn load(&mut self, id: ModuleId) -> ModuleResult<ResolvedModule> {
        let path = self.id_to_path.get(&id)
            .ok_or_else(|| ModuleError::ModuleNotFound(format!("ID {:?}", id)))?
            .clone();

        let source = fs::read_to_string(&path)
            .map_err(|e| ModuleError::IoError(path.clone(), e))?;

        let name = self.path_to_module_name(&path);

        Ok(ResolvedModule {
            id,
            path,
            name,
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_id_creation() {
        let id1 = ModuleId(0);
        let id2 = ModuleId(1);
        assert_ne!(id1, id2);
        assert_eq!(id1, ModuleId(0));
    }

    #[test]
    fn test_filesystem_resolver_creation() {
        let resolver = FileSystemResolver::new();
        assert_eq!(resolver.search_paths.len(), 1);
        assert_eq!(resolver.next_id, 0);
    }

    #[test]
    fn test_path_to_module_name() {
        let resolver = FileSystemResolver::new();

        let path = PathBuf::from("src/main.aria");
        assert_eq!(resolver.path_to_module_name(&path), "main");

        let mod_path = PathBuf::from("src/utils/mod.aria");
        assert_eq!(resolver.path_to_module_name(&mod_path), "utils");
    }
}
