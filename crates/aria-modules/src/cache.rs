//! Module cache
//!
//! Caches parsed modules to avoid re-parsing the same file multiple times.

use crate::resolver::ModuleId;
use crate::Module;
use rustc_hash::FxHashMap;

/// Cache for parsed modules
#[derive(Debug, Clone)]
pub struct ModuleCache {
    modules: FxHashMap<ModuleId, Module>,
}

impl ModuleCache {
    /// Create a new empty cache
    pub fn new() -> Self {
        Self {
            modules: FxHashMap::default(),
        }
    }

    /// Insert a module into the cache
    pub fn insert(&mut self, id: ModuleId, module: Module) {
        self.modules.insert(id, module);
    }

    /// Get a module from the cache
    pub fn get(&self, id: ModuleId) -> Option<&Module> {
        self.modules.get(&id)
    }

    /// Check if a module is in the cache
    pub fn contains(&self, id: ModuleId) -> bool {
        self.modules.contains_key(&id)
    }

    /// Remove a module from the cache
    pub fn remove(&mut self, id: ModuleId) -> Option<Module> {
        self.modules.remove(&id)
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.modules.clear();
    }

    /// Get the number of cached modules
    pub fn len(&self) -> usize {
        self.modules.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    /// Iterate over all cached modules
    pub fn iter(&self) -> impl Iterator<Item = (&ModuleId, &Module)> {
        self.modules.iter()
    }

    /// Get all module IDs in the cache
    pub fn module_ids(&self) -> impl Iterator<Item = ModuleId> + '_ {
        self.modules.keys().copied()
    }

    /// Get all modules in the cache
    pub fn modules(&self) -> impl Iterator<Item = &Module> + '_ {
        self.modules.values()
    }
}

impl Default for ModuleCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolver::ModuleId;
    use aria_ast::Program;
    use smol_str::SmolStr;
    use std::path::PathBuf;

    fn create_dummy_module(id: ModuleId, name: &str) -> Module {
        let program = Program {
            items: vec![],
            span: aria_lexer::Span::dummy(),
        };

        Module::new(
            id,
            program,
            PathBuf::from(format!("{}.aria", name)),
            SmolStr::new(name),
        )
    }

    #[test]
    fn test_cache_creation() {
        let cache = ModuleCache::new();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_insert_get() {
        let mut cache = ModuleCache::new();
        let id = ModuleId(0);
        let module = create_dummy_module(id, "test");

        cache.insert(id, module.clone());

        assert_eq!(cache.len(), 1);
        assert!(!cache.is_empty());
        assert!(cache.contains(id));

        let retrieved = cache.get(id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test");
    }

    #[test]
    fn test_cache_remove() {
        let mut cache = ModuleCache::new();
        let id = ModuleId(0);
        let module = create_dummy_module(id, "test");

        cache.insert(id, module);
        assert!(cache.contains(id));

        let removed = cache.remove(id);
        assert!(removed.is_some());
        assert!(!cache.contains(id));
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = ModuleCache::new();
        cache.insert(ModuleId(0), create_dummy_module(ModuleId(0), "a"));
        cache.insert(ModuleId(1), create_dummy_module(ModuleId(1), "b"));

        assert_eq!(cache.len(), 2);

        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_iter() {
        let mut cache = ModuleCache::new();
        cache.insert(ModuleId(0), create_dummy_module(ModuleId(0), "a"));
        cache.insert(ModuleId(1), create_dummy_module(ModuleId(1), "b"));

        let count = cache.iter().count();
        assert_eq!(count, 2);

        let names: Vec<_> = cache.modules()
            .map(|m| m.name.as_str())
            .collect();
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));
    }
}
