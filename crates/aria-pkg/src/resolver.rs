use crate::manifest::{AriaManifest, DependencySpec};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockFile {
    pub version: u32,
    #[serde(default)]
    pub packages: Vec<LockedPackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedPackage {
    pub name: String,
    pub version: semver::Version,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
}

impl LockFile {
    pub fn new() -> Self {
        Self {
            version: 1,
            packages: vec![],
        }
    }

    // Future use: For reading existing lock files during incremental builds
    #[allow(dead_code)]
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        let lock_file = toml::from_str(&content)?;
        Ok(lock_file)
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn add_package(&mut self, package: LockedPackage) {
        // Remove any existing entry for this package
        self.packages.retain(|p| p.name != package.name);
        self.packages.push(package);
        // Keep packages sorted by name for deterministic output
        self.packages.sort_by(|a, b| a.name.cmp(&b.name));
    }
}

impl Default for LockFile {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Resolver {
    manifest: AriaManifest,
}

impl Resolver {
    pub fn new(manifest: AriaManifest) -> Self {
        Self { manifest }
    }

    pub fn resolve(&self) -> anyhow::Result<LockFile> {
        let mut lock_file = LockFile::new();
        let mut visited = HashSet::new();

        // For now, we do a simple flat resolution
        // In a real implementation, this would handle:
        // - Version conflict resolution
        // - Transitive dependencies
        // - Multiple version support
        // - Registry fetching

        for (name, spec) in &self.manifest.dependencies {
            if visited.contains(name) {
                continue;
            }
            visited.insert(name.clone());

            let version = self.select_version(name, spec)?;
            let source = self.determine_source(spec);

            lock_file.add_package(LockedPackage {
                name: name.clone(),
                version,
                source,
                checksum: None, // Would compute from downloaded package
                dependencies: vec![],
            });
        }

        Ok(lock_file)
    }

    fn select_version(
        &self,
        name: &str,
        spec: &DependencySpec,
    ) -> anyhow::Result<semver::Version> {
        let version_req = spec.version_req()?;

        // In a real implementation, this would:
        // 1. Query the registry for available versions
        // 2. Filter by the version requirement
        // 3. Select the newest compatible version
        // 4. Check for pre-existing locked versions

        // For now, we'll try to extract a specific version or use a default
        match spec {
            DependencySpec::Simple(v) => {
                // Try to parse as exact version first
                if let Ok(version) = semver::Version::parse(v) {
                    return Ok(version);
                }
                // Otherwise, create a compatible version from the requirement
                self.version_from_requirement(&version_req, name)
            }
            DependencySpec::Detailed { version, .. } => {
                if let Ok(v) = semver::Version::parse(version) {
                    Ok(v)
                } else {
                    self.version_from_requirement(&version_req, name)
                }
            }
        }
    }

    fn version_from_requirement(
        &self,
        req: &semver::VersionReq,
        name: &str,
    ) -> anyhow::Result<semver::Version> {
        // Try to extract a reasonable version from the requirement
        // This is a simplified approach
        let req_str = req.to_string();

        // Handle wildcard - use a default version
        if req_str == "*" {
            return Ok(semver::Version::new(0, 1, 0));
        }

        // Handle common patterns
        if req_str.starts_with('^') || req_str.starts_with('~') {
            let version_part = req_str.trim_start_matches('^').trim_start_matches('~');
            if let Ok(v) = semver::Version::parse(version_part) {
                return Ok(v);
            }
        } else if req_str.starts_with(">=") {
            let version_part = req_str.trim_start_matches(">=").trim();
            if let Ok(v) = semver::Version::parse(version_part) {
                return Ok(v);
            }
        }

        // Try direct parse
        if let Ok(v) = semver::Version::parse(&req_str) {
            return Ok(v);
        }

        // Default fallback
        anyhow::bail!("Cannot determine version for {} with requirement {}", name, req_str)
    }

    fn determine_source(&self, spec: &DependencySpec) -> Option<String> {
        match spec {
            DependencySpec::Simple(_) => Some("registry+https://pkg.aria-lang.org".to_string()),
            DependencySpec::Detailed { git, path, .. } => {
                if let Some(git_url) = git {
                    Some(format!("git+{}", git_url))
                } else if let Some(path_str) = path {
                    Some(format!("path+{}", path_str))
                } else {
                    Some("registry+https://pkg.aria-lang.org".to_string())
                }
            }
        }
    }
}

// Future use: For complex dependency resolution with multiple versions
#[allow(dead_code)]
pub struct DependencyGraph {
    nodes: HashMap<String, DependencyNode>,
}

#[derive(Debug)]
#[allow(dead_code)]
struct DependencyNode {
    name: String,
    version: semver::Version,
    dependencies: Vec<String>,
}

#[allow(dead_code)]
impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, name: String, version: semver::Version, dependencies: Vec<String>) {
        let node = DependencyNode {
            name: name.clone(),
            version,
            dependencies,
        };
        self.nodes.insert(name, node);
    }

    pub fn topological_sort(&self) -> anyhow::Result<Vec<String>> {
        let mut sorted = Vec::new();
        let mut visited = HashSet::new();
        let mut temp_mark = HashSet::new();

        for name in self.nodes.keys() {
            if !visited.contains(name) {
                self.visit(name, &mut visited, &mut temp_mark, &mut sorted)?;
            }
        }

        // DFS post-order traversal already gives us the correct order
        // (dependencies before dependents)
        Ok(sorted)
    }

    fn visit(
        &self,
        name: &str,
        visited: &mut HashSet<String>,
        temp_mark: &mut HashSet<String>,
        sorted: &mut Vec<String>,
    ) -> anyhow::Result<()> {
        if temp_mark.contains(name) {
            anyhow::bail!("Circular dependency detected involving {}", name);
        }

        if visited.contains(name) {
            return Ok(());
        }

        temp_mark.insert(name.to_string());

        if let Some(node) = self.nodes.get(name) {
            for dep in &node.dependencies {
                self.visit(dep, visited, temp_mark, sorted)?;
            }
        }

        temp_mark.remove(name);
        visited.insert(name.to_string());
        sorted.push(name.to_string());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::AriaManifest;
    use std::collections::BTreeMap;

    #[test]
    fn test_resolver_simple() {
        let mut manifest = AriaManifest::new(
            "test".to_string(),
            semver::Version::new(0, 1, 0),
        );

        manifest.add_dependency(
            "some-lib".to_string(),
            DependencySpec::Simple("1.0.0".to_string()),
        );

        let resolver = Resolver::new(manifest);
        let lock_file = resolver.resolve().unwrap();

        assert_eq!(lock_file.packages.len(), 1);
        assert_eq!(lock_file.packages[0].name, "some-lib");
    }

    #[test]
    fn test_dependency_graph() {
        let mut graph = DependencyGraph::new();
        graph.add_node("a".to_string(), semver::Version::new(1, 0, 0), vec!["b".to_string()]);
        graph.add_node("b".to_string(), semver::Version::new(1, 0, 0), vec![]);

        let sorted = graph.topological_sort().unwrap();
        println!("Sorted order: {:?}", sorted);
        // "b" should come before "a" since "a" depends on "b"
        let b_pos = sorted.iter().position(|x| x == "b").unwrap();
        let a_pos = sorted.iter().position(|x| x == "a").unwrap();
        assert!(b_pos < a_pos, "b should come before a in topological order, got {:?}", sorted);
    }
}
