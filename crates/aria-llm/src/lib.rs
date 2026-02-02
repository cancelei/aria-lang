//! Aria LLM Optimization Pipeline
//!
//! This crate provides the infrastructure for LLM-assisted code optimization
//! in the Aria compiler. Key features:
//!
//! - **Verified Optimization**: LLM suggestions are verified for semantic equivalence
//! - **Deterministic Builds**: Optimizations are cached and versioned
//! - **Security Model**: Sandboxed execution with audit logging
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Aria Compiler                            │
//! │  ┌─────────┐    ┌──────────┐    ┌──────────┐   ┌─────────┐ │
//! │  │  Parse  │ -> │   MIR    │ -> │ LLM Opt  │ ->│ Codegen │ │
//! │  └─────────┘    └──────────┘    └──────────┘   └─────────┘ │
//! │                                       │                     │
//! │                        ┌──────────────┼──────────────┐      │
//! │                        │              │              │      │
//! │                        ▼              ▼              ▼      │
//! │                   ┌─────────┐   ┌──────────┐   ┌─────────┐ │
//! │                   │  Cache  │   │ Verifier │   │   LLM   │ │
//! │                   └─────────┘   └──────────┘   │ Provider│ │
//! │                                                └─────────┘ │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example Usage
//!
//! ```ignore
//! use aria_llm::{OptimizationPipeline, OptimizeLevel, OptimizeHint};
//!
//! let pipeline = OptimizationPipeline::new(config);
//! let optimized_mir = pipeline.optimize(mir_function, OptimizeLevel::Aggressive)?;
//! ```

mod cache;
mod config;
mod optimizer;
mod provider;
mod request;
mod security;
mod verifier;

pub use cache::{OptimizationCache, CacheEntry, CacheKey};
pub use config::{LlmConfig, OptimizeLevel, VerifyMode};
pub use optimizer::{OptimizationPipeline, OptimizationResult};
pub use provider::{LlmProvider, LlmResponse, MockProvider};
pub use request::{OptimizationRequest, OptimizationHint, OptimizationDomain};
pub use security::{SecurityPolicy, AuditLog, AuditEntry, SecurityViolation};
pub use verifier::{Verifier, VerificationResult, EquivalenceChecker};

/// Error types for LLM optimization
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("LLM provider error: {0}")]
    ProviderError(String),

    #[error("Verification failed: {0}")]
    VerificationFailed(String),

    #[error("Security violation: {0}")]
    SecurityViolation(String),

    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("Optimization timed out")]
    Timeout,

    #[error("Feature not supported: {0}")]
    NotSupported(String),

    #[error("Invalid configuration: {0}")]
    ConfigError(String),
}

pub type Result<T> = std::result::Result<T, LlmError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = LlmError::ProviderError("connection refused".to_string());
        assert!(err.to_string().contains("connection refused"));
    }
}
