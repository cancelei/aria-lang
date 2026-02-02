//! Security model for LLM optimization.
//!
//! Provides sandboxing, audit logging, and policy enforcement.

use crate::{LlmError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

/// Security policy for LLM operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    /// Allowed LLM endpoints
    pub allowed_endpoints: HashSet<String>,

    /// Blocked code patterns (regex)
    pub blocked_patterns: Vec<String>,

    /// Maximum input size in bytes
    pub max_input_size: usize,

    /// Maximum output size in bytes
    pub max_output_size: usize,

    /// Require verification before applying optimization
    pub require_verification: bool,

    /// Require proof witness for verification
    pub require_proof: bool,

    /// Allow network access
    pub allow_network: bool,

    /// Allow file system access
    pub allow_filesystem: bool,

    /// Sandbox mode (strict isolation)
    pub sandbox_mode: bool,

    /// Sign optimized output
    pub sign_output: bool,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            allowed_endpoints: HashSet::new(),
            blocked_patterns: vec![
                r"system\s*\(".to_string(),
                r"eval\s*\(".to_string(),
                r"exec\s*\(".to_string(),
                r"unsafe\s*\{".to_string(),
            ],
            max_input_size: 1024 * 1024, // 1MB
            max_output_size: 10 * 1024 * 1024, // 10MB
            require_verification: true,
            require_proof: false,
            allow_network: true,
            allow_filesystem: false,
            sandbox_mode: true,
            sign_output: false,
        }
    }
}

impl SecurityPolicy {
    /// Create a strict security policy
    pub fn strict() -> Self {
        Self {
            require_verification: true,
            require_proof: true,
            allow_network: false,
            allow_filesystem: false,
            sandbox_mode: true,
            sign_output: true,
            ..Default::default()
        }
    }

    /// Create a permissive security policy (for testing)
    pub fn permissive() -> Self {
        Self {
            allowed_endpoints: HashSet::new(),
            blocked_patterns: vec![],
            max_input_size: usize::MAX,
            max_output_size: usize::MAX,
            require_verification: false,
            require_proof: false,
            allow_network: true,
            allow_filesystem: true,
            sandbox_mode: false,
            sign_output: false,
        }
    }

    /// Add an allowed endpoint
    pub fn allow_endpoint(mut self, endpoint: &str) -> Self {
        self.allowed_endpoints.insert(endpoint.to_string());
        self
    }

    /// Check if an endpoint is allowed
    pub fn is_endpoint_allowed(&self, endpoint: &str) -> bool {
        if self.allowed_endpoints.is_empty() {
            return self.allow_network;
        }
        self.allowed_endpoints.contains(endpoint)
    }

    /// Check if code contains blocked patterns
    pub fn check_blocked_patterns(&self, code: &str) -> Result<()> {
        for pattern in &self.blocked_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if regex.is_match(code) {
                    return Err(LlmError::SecurityViolation(
                        format!("Code contains blocked pattern: {}", pattern)
                    ));
                }
            }
        }
        Ok(())
    }

    /// Validate input size
    pub fn validate_input_size(&self, size: usize) -> Result<()> {
        if size > self.max_input_size {
            return Err(LlmError::SecurityViolation(
                format!("Input size {} exceeds maximum {}", size, self.max_input_size)
            ));
        }
        Ok(())
    }

    /// Validate output size
    pub fn validate_output_size(&self, size: usize) -> Result<()> {
        if size > self.max_output_size {
            return Err(LlmError::SecurityViolation(
                format!("Output size {} exceeds maximum {}", size, self.max_output_size)
            ));
        }
        Ok(())
    }
}

/// Security violation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityViolation {
    /// Blocked pattern detected
    BlockedPattern { pattern: String, location: String },

    /// Size limit exceeded
    SizeLimit { limit: usize, actual: usize, kind: String },

    /// Unauthorized endpoint
    UnauthorizedEndpoint { endpoint: String },

    /// Verification required but skipped
    VerificationRequired,

    /// Proof required but not provided
    ProofRequired,

    /// Sandbox escape attempted
    SandboxEscape { details: String },

    /// Network access denied
    NetworkDenied,

    /// Filesystem access denied
    FilesystemDenied,

    /// Other violation
    Other { message: String },
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Timestamp (Unix epoch seconds)
    pub timestamp: u64,

    /// Entry type
    pub entry_type: AuditEntryType,

    /// Function being optimized
    pub function_name: Option<String>,

    /// LLM model used
    pub model: Option<String>,

    /// Request ID
    pub request_id: Option<String>,

    /// Result (success/failure)
    pub success: bool,

    /// Error message if failed
    pub error: Option<String>,

    /// Security violations detected
    pub violations: Vec<SecurityViolation>,

    /// Additional details
    pub details: Option<String>,
}

/// Types of audit log entries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEntryType {
    /// LLM request sent
    LlmRequest,
    /// LLM response received
    LlmResponse,
    /// Verification started
    VerificationStart,
    /// Verification completed
    VerificationComplete,
    /// Optimization applied
    OptimizationApplied,
    /// Security policy checked
    PolicyCheck,
    /// Cache hit
    CacheHit,
    /// Cache miss
    CacheMiss,
    /// Error occurred
    Error,
}

impl AuditEntry {
    /// Create a new audit entry
    pub fn new(entry_type: AuditEntryType) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            timestamp,
            entry_type,
            function_name: None,
            model: None,
            request_id: None,
            success: true,
            error: None,
            violations: vec![],
            details: None,
        }
    }

    pub fn with_function(mut self, name: &str) -> Self {
        self.function_name = Some(name.to_string());
        self
    }

    pub fn with_model(mut self, model: &str) -> Self {
        self.model = Some(model.to_string());
        self
    }

    pub fn with_request_id(mut self, id: &str) -> Self {
        self.request_id = Some(id.to_string());
        self
    }

    pub fn with_error(mut self, error: &str) -> Self {
        self.success = false;
        self.error = Some(error.to_string());
        self
    }

    pub fn with_violation(mut self, violation: SecurityViolation) -> Self {
        self.violations.push(violation);
        self
    }

    pub fn with_details(mut self, details: &str) -> Self {
        self.details = Some(details.to_string());
        self
    }
}

/// Audit log manager
pub struct AuditLog {
    /// In-memory entries
    entries: Vec<AuditEntry>,

    /// Log file path (optional)
    log_file: Option<PathBuf>,

    /// Maximum in-memory entries
    max_entries: usize,

    /// Whether logging is enabled
    enabled: bool,
}

impl AuditLog {
    /// Create a new in-memory audit log
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            log_file: None,
            max_entries: 10000,
            enabled: true,
        }
    }

    /// Create an audit log with file persistence
    pub fn with_file(path: PathBuf) -> Self {
        Self {
            entries: Vec::new(),
            log_file: Some(path),
            max_entries: 10000,
            enabled: true,
        }
    }

    /// Disable logging
    pub fn disabled() -> Self {
        Self {
            entries: Vec::new(),
            log_file: None,
            max_entries: 0,
            enabled: false,
        }
    }

    /// Log an entry
    pub fn log(&mut self, entry: AuditEntry) {
        if !self.enabled {
            return;
        }

        // Rotate if necessary
        if self.entries.len() >= self.max_entries {
            self.rotate();
        }

        // Write to file if configured
        if let Some(path) = &self.log_file {
            let _ = self.append_to_file(path, &entry);
        }

        self.entries.push(entry);
    }

    /// Get all entries
    pub fn entries(&self) -> &[AuditEntry] {
        &self.entries
    }

    /// Get entries for a specific request
    pub fn entries_for_request(&self, request_id: &str) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|e| e.request_id.as_deref() == Some(request_id))
            .collect()
    }

    /// Get entries with violations
    pub fn entries_with_violations(&self) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|e| !e.violations.is_empty())
            .collect()
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    fn rotate(&mut self) {
        // Keep only the most recent half
        let keep = self.max_entries / 2;
        if self.entries.len() > keep {
            self.entries.drain(0..self.entries.len() - keep);
        }
    }

    fn append_to_file(&self, path: &PathBuf, entry: &AuditEntry) -> Result<()> {
        use std::fs::OpenOptions;
        use std::io::Write;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| LlmError::SecurityViolation(format!("Cannot write audit log: {}", e)))?;

        let json = serde_json::to_string(entry)
            .map_err(|e| LlmError::SecurityViolation(format!("Serialization error: {}", e)))?;

        writeln!(file, "{}", json)
            .map_err(|e| LlmError::SecurityViolation(format!("Write error: {}", e)))?;

        Ok(())
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_policy_default() {
        let policy = SecurityPolicy::default();
        assert!(policy.require_verification);
        assert!(policy.allow_network);
        assert!(policy.sandbox_mode);
    }

    #[test]
    fn test_security_policy_strict() {
        let policy = SecurityPolicy::strict();
        assert!(policy.require_proof);
        assert!(!policy.allow_network);
        assert!(policy.sign_output);
    }

    #[test]
    fn test_blocked_patterns() {
        let policy = SecurityPolicy::default();

        // Should fail
        assert!(policy.check_blocked_patterns("system('rm -rf')").is_err());
        assert!(policy.check_blocked_patterns("eval(code)").is_err());

        // Should pass
        assert!(policy.check_blocked_patterns("let x = 1").is_ok());
    }

    #[test]
    fn test_input_size_validation() {
        let policy = SecurityPolicy::default();

        assert!(policy.validate_input_size(100).is_ok());
        assert!(policy.validate_input_size(policy.max_input_size + 1).is_err());
    }

    #[test]
    fn test_audit_log() {
        let mut log = AuditLog::new();

        log.log(AuditEntry::new(AuditEntryType::LlmRequest)
            .with_function("matrix_multiply")
            .with_request_id("req-123"));

        assert_eq!(log.entries().len(), 1);

        let entries = log.entries_for_request("req-123");
        assert_eq!(entries.len(), 1);
    }
}
