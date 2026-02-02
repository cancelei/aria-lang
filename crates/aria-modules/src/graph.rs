//! Module dependency graph
//!
//! Tracks dependencies between modules and detects circular dependencies.

use crate::resolver::ModuleId;
use rustc_hash::{FxHashMap, FxHashSet};

/// Edge in the dependency graph
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyEdge {
    /// The importing module
    pub from: ModuleId,
    /// The imported module
    pub to: ModuleId,
}

/// Directed graph of module dependencies
#[derive(Debug, Clone)]
pub struct ModuleGraph {
    /// Adjacency list: module -> list of dependencies
    dependencies: FxHashMap<ModuleId, Vec<ModuleId>>,
    /// Reverse adjacency list: module -> list of dependents
    dependents: FxHashMap<ModuleId, Vec<ModuleId>>,
    /// All nodes in the graph
    nodes: FxHashSet<ModuleId>,
}

impl ModuleGraph {
    /// Create a new empty module graph
    pub fn new() -> Self {
        Self {
            dependencies: FxHashMap::default(),
            dependents: FxHashMap::default(),
            nodes: FxHashSet::default(),
        }
    }

    /// Add a module to the graph
    pub fn add_module(&mut self, id: ModuleId) {
        self.nodes.insert(id);
        self.dependencies.entry(id).or_default();
        self.dependents.entry(id).or_default();
    }

    /// Add a dependency edge
    pub fn add_dependency(&mut self, from: ModuleId, to: ModuleId) {
        self.add_module(from);
        self.add_module(to);

        self.dependencies.entry(from)
            .or_default()
            .push(to);

        self.dependents.entry(to)
            .or_default()
            .push(from);
    }

    /// Get direct dependencies of a module
    pub fn get_dependencies(&self, id: ModuleId) -> &[ModuleId] {
        self.dependencies.get(&id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get modules that depend on this module
    pub fn get_dependents(&self, id: ModuleId) -> &[ModuleId] {
        self.dependents.get(&id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Detect circular dependencies using DFS
    pub fn detect_cycles(&self) -> Option<Vec<ModuleId>> {
        let mut visited = FxHashSet::default();
        let mut rec_stack = FxHashSet::default();
        let mut path = Vec::new();

        for &node in &self.nodes {
            if !visited.contains(&node) {
                if let Some(cycle) = self.dfs_cycle_detection(
                    node,
                    &mut visited,
                    &mut rec_stack,
                    &mut path,
                ) {
                    return Some(cycle);
                }
            }
        }

        None
    }

    /// DFS-based cycle detection
    fn dfs_cycle_detection(
        &self,
        node: ModuleId,
        visited: &mut FxHashSet<ModuleId>,
        rec_stack: &mut FxHashSet<ModuleId>,
        path: &mut Vec<ModuleId>,
    ) -> Option<Vec<ModuleId>> {
        visited.insert(node);
        rec_stack.insert(node);
        path.push(node);

        if let Some(deps) = self.dependencies.get(&node) {
            for &dep in deps {
                if !visited.contains(&dep) {
                    if let Some(cycle) = self.dfs_cycle_detection(dep, visited, rec_stack, path) {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(&dep) {
                    // Found a cycle! Extract it from the path
                    let cycle_start = path.iter().position(|&id| id == dep).unwrap();
                    let mut cycle = path[cycle_start..].to_vec();
                    cycle.push(dep); // Close the cycle
                    return Some(cycle);
                }
            }
        }

        path.pop();
        rec_stack.remove(&node);
        None
    }

    /// Topologically sort modules (dependencies first)
    /// Uses Kahn's algorithm with reversed edges (to get dependencies before dependents)
    pub fn topological_sort(&self) -> Vec<ModuleId> {
        let mut out_degree: FxHashMap<ModuleId, usize> = FxHashMap::default();
        let mut result = Vec::new();
        let mut queue = Vec::new();

        // Calculate out-degrees (number of dependencies each node has)
        for &node in &self.nodes {
            let deps_count = self.dependencies.get(&node).map_or(0, |d| d.len());
            out_degree.insert(node, deps_count);
        }

        // Find nodes with out-degree 0 (no dependencies - these come first)
        for (&node, &degree) in &out_degree {
            if degree == 0 {
                queue.push(node);
            }
        }

        // Process queue - use dependents to propagate
        while let Some(node) = queue.pop() {
            result.push(node);

            // For each node that depends on us, reduce their out-degree
            if let Some(dependents) = self.dependents.get(&node) {
                for &dependent in dependents {
                    if let Some(degree) = out_degree.get_mut(&dependent) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push(dependent);
                        }
                    }
                }
            }
        }

        // If we didn't process all nodes, there's a cycle
        // But we should have caught this in detect_cycles()
        result
    }

    /// Get all modules in the graph
    pub fn modules(&self) -> impl Iterator<Item = ModuleId> + '_ {
        self.nodes.iter().copied()
    }

    /// Get the number of modules in the graph
    pub fn module_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get the number of edges in the graph
    pub fn edge_count(&self) -> usize {
        self.dependencies.values().map(|v| v.len()).sum()
    }

    /// Check if there's a path from one module to another
    pub fn has_path(&self, from: ModuleId, to: ModuleId) -> bool {
        let mut visited = FxHashSet::default();
        let mut queue = vec![from];

        while let Some(current) = queue.pop() {
            if current == to {
                return true;
            }

            if !visited.insert(current) {
                continue;
            }

            if let Some(deps) = self.dependencies.get(&current) {
                queue.extend(deps);
            }
        }

        false
    }

    /// Get all transitive dependencies of a module (in dependency order)
    pub fn transitive_dependencies(&self, id: ModuleId) -> Vec<ModuleId> {
        let mut result = Vec::new();
        let mut visited = FxHashSet::default();
        self.collect_transitive_deps(id, &mut visited, &mut result);
        result
    }

    fn collect_transitive_deps(
        &self,
        id: ModuleId,
        visited: &mut FxHashSet<ModuleId>,
        result: &mut Vec<ModuleId>,
    ) {
        if !visited.insert(id) {
            return;
        }

        if let Some(deps) = self.dependencies.get(&id) {
            for &dep in deps {
                self.collect_transitive_deps(dep, visited, result);
            }
        }

        result.push(id);
    }
}

impl Default for ModuleGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_graph() {
        let graph = ModuleGraph::new();
        assert_eq!(graph.module_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_add_module() {
        let mut graph = ModuleGraph::new();
        let id = ModuleId(0);
        graph.add_module(id);
        assert_eq!(graph.module_count(), 1);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_add_dependency() {
        let mut graph = ModuleGraph::new();
        let id1 = ModuleId(0);
        let id2 = ModuleId(1);
        graph.add_dependency(id1, id2);
        assert_eq!(graph.module_count(), 2);
        assert_eq!(graph.edge_count(), 1);
        assert_eq!(graph.get_dependencies(id1), &[id2]);
        assert_eq!(graph.get_dependents(id2), &[id1]);
    }

    #[test]
    fn test_detect_cycle() {
        let mut graph = ModuleGraph::new();
        let a = ModuleId(0);
        let b = ModuleId(1);
        let c = ModuleId(2);

        // a -> b -> c -> a (cycle)
        graph.add_dependency(a, b);
        graph.add_dependency(b, c);
        graph.add_dependency(c, a);

        let cycle = graph.detect_cycles();
        assert!(cycle.is_some());
        let cycle = cycle.unwrap();
        assert!(cycle.len() >= 3);
    }

    #[test]
    fn test_no_cycle() {
        let mut graph = ModuleGraph::new();
        let a = ModuleId(0);
        let b = ModuleId(1);
        let c = ModuleId(2);

        // a -> b -> c (no cycle)
        graph.add_dependency(a, b);
        graph.add_dependency(b, c);

        assert!(graph.detect_cycles().is_none());
    }

    #[test]
    fn test_topological_sort() {
        let mut graph = ModuleGraph::new();
        let a = ModuleId(0);
        let b = ModuleId(1);
        let c = ModuleId(2);

        // a -> b -> c
        graph.add_dependency(a, b);
        graph.add_dependency(b, c);

        let sorted = graph.topological_sort();

        // c should come before b, and b before a (dependencies first)
        let pos_a = sorted.iter().position(|&x| x == a).unwrap();
        let pos_b = sorted.iter().position(|&x| x == b).unwrap();
        let pos_c = sorted.iter().position(|&x| x == c).unwrap();

        assert!(pos_c < pos_b);
        assert!(pos_b < pos_a);
    }

    #[test]
    fn test_has_path() {
        let mut graph = ModuleGraph::new();
        let a = ModuleId(0);
        let b = ModuleId(1);
        let c = ModuleId(2);

        graph.add_dependency(a, b);
        graph.add_dependency(b, c);

        assert!(graph.has_path(a, c));
        assert!(!graph.has_path(c, a));
    }

    #[test]
    fn test_transitive_dependencies() {
        let mut graph = ModuleGraph::new();
        let a = ModuleId(0);
        let b = ModuleId(1);
        let c = ModuleId(2);

        graph.add_dependency(a, b);
        graph.add_dependency(b, c);

        let deps = graph.transitive_dependencies(a);
        assert!(deps.contains(&b));
        assert!(deps.contains(&c));
    }

    #[test]
    fn test_self_cycle() {
        // Module that imports itself
        let mut graph = ModuleGraph::new();
        let a = ModuleId(0);

        graph.add_dependency(a, a);

        let cycle = graph.detect_cycles();
        assert!(cycle.is_some());
        let cycle = cycle.unwrap();
        // Self-cycle should be detected
        assert!(cycle.contains(&a));
    }

    #[test]
    fn test_two_node_cycle() {
        // a -> b -> a
        let mut graph = ModuleGraph::new();
        let a = ModuleId(0);
        let b = ModuleId(1);

        graph.add_dependency(a, b);
        graph.add_dependency(b, a);

        let cycle = graph.detect_cycles();
        assert!(cycle.is_some());
        let cycle = cycle.unwrap();
        assert!(cycle.contains(&a));
        assert!(cycle.contains(&b));
    }

    #[test]
    fn test_diamond_dependency_no_cycle() {
        // Diamond: a -> b, a -> c, b -> d, c -> d
        let mut graph = ModuleGraph::new();
        let a = ModuleId(0);
        let b = ModuleId(1);
        let c = ModuleId(2);
        let d = ModuleId(3);

        graph.add_dependency(a, b);
        graph.add_dependency(a, c);
        graph.add_dependency(b, d);
        graph.add_dependency(c, d);

        // Diamond has no cycle
        assert!(graph.detect_cycles().is_none());

        // But a has a path to d through both b and c
        assert!(graph.has_path(a, d));
    }

    #[test]
    fn test_cycle_in_subgraph() {
        // a -> b -> c, d -> e -> d (separate cycle in d-e)
        let mut graph = ModuleGraph::new();
        let a = ModuleId(0);
        let b = ModuleId(1);
        let c = ModuleId(2);
        let d = ModuleId(3);
        let e = ModuleId(4);

        // Acyclic chain
        graph.add_dependency(a, b);
        graph.add_dependency(b, c);

        // Cycle in separate component
        graph.add_dependency(d, e);
        graph.add_dependency(e, d);

        let cycle = graph.detect_cycles();
        assert!(cycle.is_some());
        let cycle = cycle.unwrap();
        // The cycle should be in d-e, not a-b-c
        assert!(cycle.contains(&d) || cycle.contains(&e));
    }

    #[test]
    fn test_topological_sort_diamond() {
        // Diamond: a -> b, a -> c, b -> d, c -> d
        let mut graph = ModuleGraph::new();
        let a = ModuleId(0);
        let b = ModuleId(1);
        let c = ModuleId(2);
        let d = ModuleId(3);

        graph.add_dependency(a, b);
        graph.add_dependency(a, c);
        graph.add_dependency(b, d);
        graph.add_dependency(c, d);

        let sorted = graph.topological_sort();
        assert_eq!(sorted.len(), 4);

        // d must come before b and c
        // b and c must come before a
        let pos_a = sorted.iter().position(|&x| x == a).unwrap();
        let pos_b = sorted.iter().position(|&x| x == b).unwrap();
        let pos_c = sorted.iter().position(|&x| x == c).unwrap();
        let pos_d = sorted.iter().position(|&x| x == d).unwrap();

        assert!(pos_d < pos_b);
        assert!(pos_d < pos_c);
        assert!(pos_b < pos_a);
        assert!(pos_c < pos_a);
    }

    #[test]
    fn test_isolated_node() {
        let mut graph = ModuleGraph::new();
        let a = ModuleId(0);
        let b = ModuleId(1);

        graph.add_module(a);
        graph.add_module(b);

        // No dependencies, so no cycles
        assert!(graph.detect_cycles().is_none());

        // Isolated nodes should be in topological sort
        let sorted = graph.topological_sort();
        assert_eq!(sorted.len(), 2);
        assert!(sorted.contains(&a));
        assert!(sorted.contains(&b));
    }

    #[test]
    fn test_duplicate_dependency() {
        // Adding same dependency twice
        let mut graph = ModuleGraph::new();
        let a = ModuleId(0);
        let b = ModuleId(1);

        graph.add_dependency(a, b);
        graph.add_dependency(a, b); // Duplicate

        // Module count should still be 2
        assert_eq!(graph.module_count(), 2);
        // Edge count will be 2 (duplicates are stored)
        assert_eq!(graph.edge_count(), 2);

        // But no cycle should be detected
        assert!(graph.detect_cycles().is_none());
    }
}
