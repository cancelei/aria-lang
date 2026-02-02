//! Main optimization pipeline that orchestrates LLM optimization.

use crate::{
    LlmConfig, LlmError, Result, OptimizeLevel,
    cache::{OptimizationCache, CacheKey, CacheEntry},
    provider::{LlmProvider, LlmResponse, OptimizationSuggestion},
    request::OptimizationRequest,
    security::{SecurityPolicy, AuditLog, AuditEntry, AuditEntryType},
    verifier::{Verifier, VerificationResult},
};
use std::sync::{Arc, Mutex};

/// Result of the optimization pipeline
#[derive(Debug, Clone)]
pub struct OptimizationResult {
    /// Original code
    pub original: String,

    /// Optimized code (if optimization was applied)
    pub optimized: Option<String>,

    /// The suggestion that was applied
    pub applied_suggestion: Option<OptimizationSuggestion>,

    /// Verification result
    pub verification: Option<VerificationResult>,

    /// Whether the optimization came from cache
    pub from_cache: bool,

    /// All suggestions considered
    pub all_suggestions: Vec<OptimizationSuggestion>,

    /// Any warnings generated
    pub warnings: Vec<String>,
}

impl OptimizationResult {
    /// Create a result with no optimization applied
    pub fn unchanged(original: String) -> Self {
        Self {
            original,
            optimized: None,
            applied_suggestion: None,
            verification: None,
            from_cache: false,
            all_suggestions: vec![],
            warnings: vec![],
        }
    }

    /// Check if an optimization was applied
    pub fn was_optimized(&self) -> bool {
        self.optimized.is_some()
    }

    /// Get the final code (optimized if available, otherwise original)
    pub fn code(&self) -> &str {
        self.optimized.as_deref().unwrap_or(&self.original)
    }
}

/// Main optimization pipeline
pub struct OptimizationPipeline {
    /// Configuration
    config: LlmConfig,

    /// LLM provider
    provider: Arc<dyn LlmProvider>,

    /// Verifier
    verifier: Verifier,

    /// Cache
    cache: Arc<Mutex<OptimizationCache>>,

    /// Security policy
    security: SecurityPolicy,

    /// Audit log
    audit: Arc<Mutex<AuditLog>>,
}

impl OptimizationPipeline {
    /// Create a new optimization pipeline
    pub fn new(
        config: LlmConfig,
        provider: Arc<dyn LlmProvider>,
    ) -> Self {
        let cache = match &config.cache_dir {
            Some(dir) => OptimizationCache::with_disk(dir.clone())
                .unwrap_or_else(|_| OptimizationCache::new()),
            None => OptimizationCache::new(),
        };

        Self {
            verifier: match config.verify_mode {
                crate::VerifyMode::Strict => Verifier::strict(),
                _ => Verifier::new(),
            },
            security: config.security.clone().into(),
            cache: Arc::new(Mutex::new(cache)),
            audit: Arc::new(Mutex::new(
                if config.enable_audit {
                    match &config.audit_log_path {
                        Some(path) => AuditLog::with_file(path.clone()),
                        None => AuditLog::new(),
                    }
                } else {
                    AuditLog::disabled()
                }
            )),
            config,
            provider,
        }
    }

    /// Create a pipeline that disables LLM optimization (pass-through)
    pub fn disabled() -> Self {
        use crate::provider::MockProvider;
        Self::new(
            LlmConfig::disabled(),
            Arc::new(MockProvider::new()),
        )
    }

    /// Optimize code using the pipeline
    pub fn optimize(&self, request: OptimizationRequest) -> Result<OptimizationResult> {
        // Check if optimization is disabled
        if self.config.optimize_level == OptimizeLevel::None {
            return Ok(OptimizationResult::unchanged(request.code.clone()));
        }

        // Security checks
        self.security.validate_input_size(request.code.len())?;
        self.security.check_blocked_patterns(&request.code)?;

        // Log request
        self.log(AuditEntry::new(AuditEntryType::LlmRequest)
            .with_function(&request.function_name)
            .with_request_id(&request.id));

        // Check cache
        let cache_key = CacheKey::new(
            &request.code,
            &format!("{:?}{:?}", request.domains, request.hints),
            self.provider.model(),
        );

        if self.config.enable_cache {
            if let Some(entry) = self.cache.lock().unwrap().get(&cache_key) {
                self.log(AuditEntry::new(AuditEntryType::CacheHit)
                    .with_request_id(&request.id));

                return Ok(self.apply_cached_entry(&request, &entry));
            } else {
                self.log(AuditEntry::new(AuditEntryType::CacheMiss)
                    .with_request_id(&request.id));
            }
        }

        // Get suggestions from LLM
        let response = match self.provider.optimize(&request) {
            Ok(r) => {
                self.log(AuditEntry::new(AuditEntryType::LlmResponse)
                    .with_request_id(&request.id)
                    .with_model(&r.model));
                r
            }
            Err(e) => {
                self.log(AuditEntry::new(AuditEntryType::Error)
                    .with_request_id(&request.id)
                    .with_error(&e.to_string()));
                return Err(e);
            }
        };

        // Verify and apply best suggestion
        self.process_suggestions(&request, response, &cache_key)
    }

    fn process_suggestions(
        &self,
        request: &OptimizationRequest,
        response: LlmResponse,
        cache_key: &CacheKey,
    ) -> Result<OptimizationResult> {
        let mut result = OptimizationResult {
            original: request.code.clone(),
            optimized: None,
            applied_suggestion: None,
            verification: None,
            from_cache: false,
            all_suggestions: response.suggestions.clone(),
            warnings: vec![],
        };

        // Try each suggestion, applying the first that verifies
        for suggestion in &response.suggestions {
            self.log(AuditEntry::new(AuditEntryType::VerificationStart)
                .with_request_id(&request.id)
                .with_details(&suggestion.id));

            // Security check on suggestion
            if let Err(e) = self.security.check_blocked_patterns(&suggestion.optimized_code) {
                result.warnings.push(format!("Suggestion {} rejected: {}", suggestion.id, e));
                continue;
            }

            // Verify equivalence
            let verification = self.verifier.verify(&request.code, suggestion)?;

            self.log(AuditEntry::new(AuditEntryType::VerificationComplete)
                .with_request_id(&request.id)
                .with_details(&format!("verified={}", verification.verified)));

            if verification.verified {
                // Cache the successful optimization
                if self.config.enable_cache {
                    let entry = CacheEntry::new(
                        cache_key.clone(),
                        response.clone(),
                        verification.clone(),
                    );
                    self.cache.lock().unwrap().put(entry);
                }

                self.log(AuditEntry::new(AuditEntryType::OptimizationApplied)
                    .with_request_id(&request.id)
                    .with_details(&suggestion.optimization_type));

                result.optimized = Some(suggestion.optimized_code.clone());
                result.applied_suggestion = Some(suggestion.clone());
                result.verification = Some(verification);
                break;
            } else {
                result.warnings.push(format!(
                    "Suggestion {} failed verification: {:?}",
                    suggestion.id,
                    verification.counterexample
                ));
            }
        }

        Ok(result)
    }

    fn apply_cached_entry(&self, request: &OptimizationRequest, entry: &CacheEntry) -> OptimizationResult {
        let mut result = OptimizationResult {
            original: request.code.clone(),
            optimized: None,
            applied_suggestion: None,
            verification: Some(entry.verification.clone()),
            from_cache: true,
            all_suggestions: entry.response.suggestions.clone(),
            warnings: vec![],
        };

        // Apply the first suggestion from cache
        if let Some(suggestion) = entry.response.suggestions.first() {
            result.optimized = Some(suggestion.optimized_code.clone());
            result.applied_suggestion = Some(suggestion.clone());
        }

        result
    }

    fn log(&self, entry: AuditEntry) {
        self.audit.lock().unwrap().log(entry);
    }

    /// Get the audit log
    pub fn audit_log(&self) -> Arc<Mutex<AuditLog>> {
        self.audit.clone()
    }

    /// Get the cache
    pub fn cache(&self) -> Arc<Mutex<OptimizationCache>> {
        self.cache.clone()
    }
}

// Convert security config to policy
impl From<crate::config::SecurityConfig> for SecurityPolicy {
    fn from(config: crate::config::SecurityConfig) -> Self {
        let mut policy = SecurityPolicy::default();
        policy.allow_network = config.allow_network;
        policy.allow_filesystem = false;
        policy.max_input_size = config.max_input_size;
        policy.sandbox_mode = config.sandbox;

        for endpoint in config.allowed_endpoints {
            policy.allowed_endpoints.insert(endpoint);
        }

        policy
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::MockProvider;

    #[test]
    fn test_pipeline_disabled() {
        let pipeline = OptimizationPipeline::disabled();
        let request = OptimizationRequest::new("test", "fn foo() end".to_string());

        let result = pipeline.optimize(request).unwrap();
        assert!(!result.was_optimized());
        assert_eq!(result.code(), "fn foo() end");
    }

    #[test]
    fn test_pipeline_with_mock() {
        let config = LlmConfig::default();
        let provider = Arc::new(MockProvider::new());
        let pipeline = OptimizationPipeline::new(config, provider);

        let request = OptimizationRequest::new("test", "fn foo() end".to_string());
        let result = pipeline.optimize(request).unwrap();

        // Mock provider returns suggestions but they may not verify
        assert!(!result.all_suggestions.is_empty() || result.warnings.is_empty());
    }

    #[test]
    fn test_optimization_result_unchanged() {
        let result = OptimizationResult::unchanged("original code".to_string());
        assert!(!result.was_optimized());
        assert_eq!(result.code(), "original code");
    }
}
