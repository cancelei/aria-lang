//! LLM provider trait and implementations.

use crate::{LlmError, Result, OptimizationRequest};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

/// Response from an LLM provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    /// Request ID this response is for
    pub request_id: String,

    /// Suggested optimizations
    pub suggestions: Vec<OptimizationSuggestion>,

    /// Model used
    pub model: String,

    /// Tokens used in request
    pub input_tokens: usize,

    /// Tokens used in response
    pub output_tokens: usize,

    /// Whether response was cached
    pub cached: bool,
}

/// A single optimization suggestion from the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationSuggestion {
    /// Unique identifier for this suggestion
    pub id: String,

    /// Optimized code
    pub optimized_code: String,

    /// Explanation of the optimization
    pub explanation: Option<String>,

    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,

    /// Expected speedup factor
    pub estimated_speedup: Option<f64>,

    /// Type of optimization applied
    pub optimization_type: String,

    /// Verification hints for the verifier
    pub verification_hints: Vec<VerificationHint>,
}

/// Hint for the verification system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationHint {
    /// Type of hint
    pub kind: VerificationHintKind,
    /// Additional data
    pub data: Option<String>,
}

/// Types of verification hints
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationHintKind {
    /// The transformation preserves semantics trivially
    TrivialEquivalence,
    /// Loop unrolling with factor
    LoopUnroll(usize),
    /// SIMD width used
    SimdWidth(usize),
    /// Mathematical identity used
    MathIdentity,
    /// Commutativity was applied
    Commutativity,
    /// Associativity was applied
    Associativity,
    /// Distribution was applied
    Distribution,
    /// Dead code was removed
    DeadCodeRemoval,
    /// Custom verification strategy needed
    Custom(String),
}

/// Trait for LLM providers
pub trait LlmProvider: Send + Sync {
    /// Get the name of this provider
    fn name(&self) -> &str;

    /// Check if the provider is available
    fn is_available(&self) -> bool;

    /// Send an optimization request and get suggestions
    fn optimize(&self, request: &OptimizationRequest) -> Result<LlmResponse>;

    /// Get the model being used
    fn model(&self) -> &str;
}

/// Mock LLM provider for testing
pub struct MockProvider {
    name: String,
    model: String,
    responses: Vec<LlmResponse>,
    response_idx: std::sync::atomic::AtomicUsize,
}

impl MockProvider {
    /// Create a new mock provider
    pub fn new() -> Self {
        Self {
            name: "mock".to_string(),
            model: "mock-model".to_string(),
            responses: vec![],
            response_idx: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Add a mock response
    pub fn with_response(mut self, response: LlmResponse) -> Self {
        self.responses.push(response);
        self
    }

    /// Create a simple mock response for testing
    pub fn simple_response(request_id: &str, optimized_code: String) -> LlmResponse {
        LlmResponse {
            request_id: request_id.to_string(),
            suggestions: vec![OptimizationSuggestion {
                id: format!("{}-1", request_id),
                optimized_code,
                explanation: Some("Mock optimization".to_string()),
                confidence: 0.9,
                estimated_speedup: Some(1.5),
                optimization_type: "mock".to_string(),
                verification_hints: vec![],
            }],
            model: "mock-model".to_string(),
            input_tokens: 100,
            output_tokens: 200,
            cached: false,
        }
    }
}

impl Default for MockProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl LlmProvider for MockProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn is_available(&self) -> bool {
        true
    }

    fn optimize(&self, request: &OptimizationRequest) -> Result<LlmResponse> {
        if self.responses.is_empty() {
            // Generate a default response
            Ok(Self::simple_response(&request.id, request.code.clone()))
        } else {
            let idx = self.response_idx.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let response_idx = idx % self.responses.len();
            Ok(self.responses[response_idx].clone())
        }
    }

    fn model(&self) -> &str {
        &self.model
    }
}

/// Placeholder for future OpenAI provider
#[allow(dead_code)]
pub struct OpenAiProvider {
    api_key: String,
    model: String,
    endpoint: String,
}

impl OpenAiProvider {
    /// Create a new OpenAI provider
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            endpoint: "https://api.openai.com/v1/chat/completions".to_string(),
        }
    }
}

impl LlmProvider for OpenAiProvider {
    fn name(&self) -> &str {
        "openai"
    }

    fn is_available(&self) -> bool {
        !self.api_key.is_empty()
    }

    fn optimize(&self, request: &OptimizationRequest) -> Result<LlmResponse> {
        // TODO: Implement actual OpenAI API call
        Err(LlmError::NotSupported("OpenAI provider not yet implemented".to_string()))
    }

    fn model(&self) -> &str {
        &self.model
    }
}

/// Placeholder for future Anthropic provider
#[allow(dead_code)]
pub struct AnthropicProvider {
    api_key: String,
    model: String,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider
    pub fn new(api_key: String, model: String) -> Self {
        Self { api_key, model }
    }
}

impl LlmProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    fn is_available(&self) -> bool {
        !self.api_key.is_empty()
    }

    fn optimize(&self, request: &OptimizationRequest) -> Result<LlmResponse> {
        // TODO: Implement actual Anthropic API call
        Err(LlmError::NotSupported("Anthropic provider not yet implemented".to_string()))
    }

    fn model(&self) -> &str {
        &self.model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_provider() {
        let provider = MockProvider::new();
        assert!(provider.is_available());
        assert_eq!(provider.name(), "mock");

        let request = OptimizationRequest::new("test", "code".to_string());
        let response = provider.optimize(&request).unwrap();
        assert!(!response.suggestions.is_empty());
    }

    #[test]
    fn test_mock_provider_with_responses() {
        let response = MockProvider::simple_response("req-1", "optimized".to_string());
        let provider = MockProvider::new().with_response(response);

        let request = OptimizationRequest::new("test", "code".to_string());
        let result = provider.optimize(&request).unwrap();
        assert_eq!(result.suggestions[0].optimized_code, "optimized");
    }
}
