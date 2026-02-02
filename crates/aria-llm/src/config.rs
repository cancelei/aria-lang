//! Configuration for LLM optimization pipeline.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Optimization aggressiveness level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizeLevel {
    /// No LLM optimization (pass-through)
    None,
    /// Conservative optimizations (high confidence, verified)
    Conservative,
    /// Standard optimizations (balanced)
    Standard,
    /// Aggressive optimizations (more suggestions, longer verification)
    Aggressive,
}

impl Default for OptimizeLevel {
    fn default() -> Self {
        OptimizeLevel::Standard
    }
}

/// Verification mode for LLM suggestions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerifyMode {
    /// Skip verification (dangerous, for testing only)
    Skip,
    /// Quick heuristic verification
    Quick,
    /// Full formal verification (SMT-based)
    Formal,
    /// Strict: formal verification with proof witness
    Strict,
}

impl Default for VerifyMode {
    fn default() -> Self {
        VerifyMode::Formal
    }
}

/// Configuration for the LLM optimization pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// LLM provider endpoint URL
    pub endpoint: Option<String>,

    /// API key for LLM provider (stored securely)
    pub api_key_env: String,

    /// Model identifier to use
    pub model: String,

    /// Optimization aggressiveness
    pub optimize_level: OptimizeLevel,

    /// Verification mode
    pub verify_mode: VerifyMode,

    /// Timeout for LLM requests
    pub timeout: Duration,

    /// Maximum tokens in LLM response
    pub max_tokens: usize,

    /// Temperature for LLM sampling (0.0 for deterministic)
    pub temperature: f32,

    /// Enable caching of verified optimizations
    pub enable_cache: bool,

    /// Cache directory path
    pub cache_dir: Option<PathBuf>,

    /// Maximum cache size in MB
    pub max_cache_size_mb: u64,

    /// Enable audit logging
    pub enable_audit: bool,

    /// Audit log path
    pub audit_log_path: Option<PathBuf>,

    /// Optimization domains to enable
    pub enabled_domains: Vec<OptimizationDomainConfig>,

    /// Security policy
    pub security: SecurityConfig,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            endpoint: None, // Use default provider endpoint
            api_key_env: "ARIA_LLM_API_KEY".to_string(),
            model: "gpt-4".to_string(),
            optimize_level: OptimizeLevel::Standard,
            verify_mode: VerifyMode::Formal,
            timeout: Duration::from_secs(30),
            max_tokens: 2048,
            temperature: 0.0, // Deterministic by default
            enable_cache: true,
            cache_dir: None, // Use default
            max_cache_size_mb: 100,
            enable_audit: true,
            audit_log_path: None,
            enabled_domains: vec![
                OptimizationDomainConfig::default_simd(),
                OptimizationDomainConfig::default_loops(),
            ],
            security: SecurityConfig::default(),
        }
    }
}

impl LlmConfig {
    /// Create a new configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a configuration for deterministic/reproducible builds
    pub fn deterministic() -> Self {
        Self {
            temperature: 0.0,
            enable_cache: true,
            verify_mode: VerifyMode::Formal,
            ..Default::default()
        }
    }

    /// Create a configuration for maximum performance (less strict verification)
    pub fn performance() -> Self {
        Self {
            optimize_level: OptimizeLevel::Aggressive,
            verify_mode: VerifyMode::Quick,
            temperature: 0.0,
            ..Default::default()
        }
    }

    /// Create a configuration that disables LLM optimization
    pub fn disabled() -> Self {
        Self {
            optimize_level: OptimizeLevel::None,
            enable_cache: false,
            enable_audit: false,
            ..Default::default()
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.temperature < 0.0 || self.temperature > 2.0 {
            return Err("Temperature must be between 0.0 and 2.0".to_string());
        }

        if self.max_tokens == 0 {
            return Err("max_tokens must be greater than 0".to_string());
        }

        if self.verify_mode == VerifyMode::Skip && self.optimize_level != OptimizeLevel::None {
            // Warning: skipping verification with active optimization
            eprintln!("Warning: Skipping verification with active LLM optimization is dangerous");
        }

        Ok(())
    }
}

/// Configuration for a specific optimization domain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationDomainConfig {
    /// Domain name
    pub name: String,

    /// Whether this domain is enabled
    pub enabled: bool,

    /// Maximum transformations to consider
    pub max_suggestions: usize,

    /// Domain-specific prompt template
    pub prompt_template: Option<String>,
}

impl OptimizationDomainConfig {
    /// Default SIMD domain configuration
    pub fn default_simd() -> Self {
        Self {
            name: "simd".to_string(),
            enabled: true,
            max_suggestions: 3,
            prompt_template: None,
        }
    }

    /// Default loop optimization domain configuration
    pub fn default_loops() -> Self {
        Self {
            name: "loops".to_string(),
            enabled: true,
            max_suggestions: 5,
            prompt_template: None,
        }
    }

    /// Default memory optimization domain configuration
    pub fn default_memory() -> Self {
        Self {
            name: "memory".to_string(),
            enabled: false, // Disabled by default (more complex)
            max_suggestions: 3,
            prompt_template: None,
        }
    }
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Allow network access for LLM calls
    pub allow_network: bool,

    /// Allowed external endpoints (empty = all allowed)
    pub allowed_endpoints: Vec<String>,

    /// Maximum input size to LLM (bytes)
    pub max_input_size: usize,

    /// Sandbox LLM execution
    pub sandbox: bool,

    /// Require code signing for optimized output
    pub require_signing: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            allow_network: true,
            allowed_endpoints: vec![],
            max_input_size: 1024 * 1024, // 1MB
            sandbox: true,
            require_signing: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LlmConfig::default();
        assert_eq!(config.optimize_level, OptimizeLevel::Standard);
        assert_eq!(config.temperature, 0.0);
        assert!(config.enable_cache);
    }

    #[test]
    fn test_deterministic_config() {
        let config = LlmConfig::deterministic();
        assert_eq!(config.temperature, 0.0);
        assert!(config.enable_cache);
    }

    #[test]
    fn test_validate_config() {
        let mut config = LlmConfig::default();
        assert!(config.validate().is_ok());

        config.temperature = -1.0;
        assert!(config.validate().is_err());
    }
}
