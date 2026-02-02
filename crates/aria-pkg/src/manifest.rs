use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AriaManifest {
    pub package: PackageInfo,
    #[serde(default)]
    pub dependencies: BTreeMap<String, DependencySpec>,
    #[serde(default, rename = "dev-dependencies")]
    pub dev_dependencies: BTreeMap<String, DependencySpec>,
    #[serde(default, rename = "build-dependencies")]
    pub build_dependencies: BTreeMap<String, DependencySpec>,
    #[serde(default)]
    pub features: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub target: BTreeMap<String, TargetConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: semver::Version,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub repository: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub readme: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub documentation: Option<String>,
    #[serde(default, rename = "default-features")]
    pub default_features: Option<Vec<String>>,
    #[serde(default)]
    pub targets: Vec<String>,
}

/// Target-specific configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TargetConfig {
    #[serde(default)]
    pub dependencies: BTreeMap<String, DependencySpec>,
    #[serde(default, rename = "dev-dependencies")]
    pub dev_dependencies: BTreeMap<String, DependencySpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DependencySpec {
    Simple(String),
    Detailed {
        version: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        git: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        branch: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tag: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        rev: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        optional: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        features: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none", rename = "default-features")]
        default_features: Option<bool>,
    },
}

impl DependencySpec {
    /// Check if this dependency is optional
    pub fn is_optional(&self) -> bool {
        match self {
            DependencySpec::Simple(_) => false,
            DependencySpec::Detailed { optional, .. } => optional.unwrap_or(false),
        }
    }

    /// Get the features required by this dependency
    pub fn get_features(&self) -> Vec<String> {
        match self {
            DependencySpec::Simple(_) => vec![],
            DependencySpec::Detailed { features, .. } => features.clone().unwrap_or_default(),
        }
    }

    /// Check if default features are enabled
    pub fn uses_default_features(&self) -> bool {
        match self {
            DependencySpec::Simple(_) => true,
            DependencySpec::Detailed { default_features, .. } => default_features.unwrap_or(true),
        }
    }
}

impl DependencySpec {
    pub fn version_req(&self) -> anyhow::Result<semver::VersionReq> {
        let version_str = match self {
            DependencySpec::Simple(v) => v,
            DependencySpec::Detailed { version, .. } => version,
        };
        Ok(semver::VersionReq::parse(version_str)?)
    }
}

impl AriaManifest {
    pub fn new(name: String, version: semver::Version) -> Self {
        Self {
            package: PackageInfo {
                name,
                version,
                authors: vec![],
                description: None,
                license: Some("MIT OR Apache-2.0".to_string()),
                repository: None,
                keywords: vec![],
                categories: vec![],
                readme: None,
                homepage: None,
                documentation: None,
                default_features: None,
                targets: vec![],
            },
            dependencies: BTreeMap::new(),
            dev_dependencies: BTreeMap::new(),
            build_dependencies: BTreeMap::new(),
            features: BTreeMap::new(),
            target: BTreeMap::new(),
        }
    }

    /// Add a dev dependency
    pub fn add_dev_dependency(&mut self, name: String, spec: DependencySpec) {
        self.dev_dependencies.insert(name, spec);
    }

    /// Add a build dependency
    pub fn add_build_dependency(&mut self, name: String, spec: DependencySpec) {
        self.build_dependencies.insert(name, spec);
    }

    /// Add a feature
    pub fn add_feature(&mut self, name: String, enables: Vec<String>) {
        self.features.insert(name, enables);
    }

    /// Get all dependencies for a specific target (including common dependencies)
    pub fn get_dependencies_for_target(&self, target: &str) -> BTreeMap<String, DependencySpec> {
        let mut deps = self.dependencies.clone();

        if let Some(target_config) = self.target.get(target) {
            for (name, spec) in &target_config.dependencies {
                deps.insert(name.clone(), spec.clone());
            }
        }

        deps
    }

    /// Get all dev dependencies for a specific target
    pub fn get_dev_dependencies_for_target(&self, target: &str) -> BTreeMap<String, DependencySpec> {
        let mut deps = self.dev_dependencies.clone();

        if let Some(target_config) = self.target.get(target) {
            for (name, spec) in &target_config.dev_dependencies {
                deps.insert(name.clone(), spec.clone());
            }
        }

        deps
    }

    /// Get enabled features (including default features if not disabled)
    pub fn get_enabled_features(&self, requested: &[String]) -> Vec<String> {
        let mut enabled = Vec::new();

        // Add default features if they exist
        if let Some(defaults) = &self.package.default_features {
            for feature in defaults {
                if !enabled.contains(feature) {
                    enabled.push(feature.clone());
                }
            }
        }

        // Add requested features
        for feature in requested {
            if !enabled.contains(feature) {
                enabled.push(feature.clone());
            }
        }

        // Recursively resolve feature dependencies
        let mut resolved = Vec::new();
        for feature in &enabled {
            self.resolve_feature(feature, &mut resolved);
        }

        resolved
    }

    fn resolve_feature(&self, feature: &str, resolved: &mut Vec<String>) {
        if resolved.contains(&feature.to_string()) {
            return;
        }

        resolved.push(feature.to_string());

        if let Some(enables) = self.features.get(feature) {
            for enabled in enables {
                // Check if it's enabling another feature or a dependency/feature
                if enabled.contains('/') {
                    // It's dep/feature format
                    resolved.push(enabled.clone());
                } else if self.features.contains_key(enabled) {
                    // It's another feature
                    self.resolve_feature(enabled, resolved);
                } else {
                    // It might be a dependency name (optional dep)
                    resolved.push(enabled.clone());
                }
            }
        }
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        let manifest = toml::from_str(&content)?;
        Ok(manifest)
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn add_dependency(&mut self, name: String, spec: DependencySpec) {
        self.dependencies.insert(name, spec);
    }

    pub fn remove_dependency(&mut self, name: &str) -> bool {
        self.dependencies.remove(name).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_manifest() {
        let manifest = AriaManifest::new(
            "test-project".to_string(),
            semver::Version::new(0, 1, 0),
        );
        let toml_str = toml::to_string_pretty(&manifest).unwrap();
        assert!(toml_str.contains("[package]"));
        assert!(toml_str.contains("name = \"test-project\""));
    }

    #[test]
    fn test_parse_manifest() {
        let toml_str = r#"
[package]
name = "test-project"
version = "0.1.0"
authors = ["Test Author"]

[dependencies]
some-lib = "1.0.0"
"#;
        let manifest: AriaManifest = toml::from_str(toml_str).unwrap();
        assert_eq!(manifest.package.name, "test-project");
        assert_eq!(manifest.dependencies.len(), 1);
    }

    #[test]
    fn test_parse_manifest_with_features() {
        let toml_str = r#"
[package]
name = "test-project"
version = "0.1.0"
targets = ["native", "wasm"]

[dependencies]
http = "^2.0"
json = "^1.5"
crypto = { version = "^3.0", optional = true }

[dev-dependencies]
testing = "^1.0"

[features]
default = ["crypto"]
minimal = []
full = ["crypto", "http/tls"]

[target.wasm.dependencies]
web = "^1.0"
"#;
        let manifest: AriaManifest = toml::from_str(toml_str).unwrap();
        assert_eq!(manifest.package.name, "test-project");
        assert_eq!(manifest.package.targets.len(), 2);
        assert_eq!(manifest.dependencies.len(), 3);
        assert_eq!(manifest.dev_dependencies.len(), 1);
        assert_eq!(manifest.features.len(), 3);
        assert!(manifest.features.contains_key("default"));
        assert!(manifest.target.contains_key("wasm"));
    }

    #[test]
    fn test_dependency_spec_optional() {
        let simple = DependencySpec::Simple("1.0.0".to_string());
        assert!(!simple.is_optional());

        let optional = DependencySpec::Detailed {
            version: "1.0.0".to_string(),
            git: None,
            branch: None,
            tag: None,
            rev: None,
            path: None,
            optional: Some(true),
            features: None,
            default_features: None,
        };
        assert!(optional.is_optional());
    }

    #[test]
    fn test_dependency_spec_features() {
        let with_features = DependencySpec::Detailed {
            version: "1.0.0".to_string(),
            git: None,
            branch: None,
            tag: None,
            rev: None,
            path: None,
            optional: None,
            features: Some(vec!["async".to_string(), "tls".to_string()]),
            default_features: Some(false),
        };
        assert_eq!(with_features.get_features(), vec!["async", "tls"]);
        assert!(!with_features.uses_default_features());
    }

    #[test]
    fn test_get_dependencies_for_target() {
        let toml_str = r#"
[package]
name = "test"
version = "0.1.0"

[dependencies]
common = "1.0.0"

[target.wasm.dependencies]
wasm-specific = "2.0.0"
"#;
        let manifest: AriaManifest = toml::from_str(toml_str).unwrap();

        let native_deps = manifest.get_dependencies_for_target("native");
        assert_eq!(native_deps.len(), 1);
        assert!(native_deps.contains_key("common"));

        let wasm_deps = manifest.get_dependencies_for_target("wasm");
        assert_eq!(wasm_deps.len(), 2);
        assert!(wasm_deps.contains_key("common"));
        assert!(wasm_deps.contains_key("wasm-specific"));
    }

    #[test]
    fn test_feature_resolution() {
        let mut manifest = AriaManifest::new("test".to_string(), semver::Version::new(0, 1, 0));
        manifest.package.default_features = Some(vec!["default_feature".to_string()]);
        manifest.features.insert("default_feature".to_string(), vec!["sub_feature".to_string()]);
        manifest.features.insert("sub_feature".to_string(), vec![]);
        manifest.features.insert("extra".to_string(), vec![]);

        let enabled = manifest.get_enabled_features(&["extra".to_string()]);
        assert!(enabled.contains(&"default_feature".to_string()));
        assert!(enabled.contains(&"sub_feature".to_string()));
        assert!(enabled.contains(&"extra".to_string()));
    }
}
