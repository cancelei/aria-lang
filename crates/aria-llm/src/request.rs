//! Optimization request structures.

use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

/// Domain of optimization to perform
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OptimizationDomain {
    /// SIMD vectorization
    Simd,
    /// Loop transformations (tiling, unrolling, fusion)
    Loops,
    /// Memory layout optimization
    Memory,
    /// Algorithm replacement (e.g., faster sort)
    Algorithm,
    /// Parallelization
    Parallel,
    /// Constant folding and propagation
    Constants,
    /// Dead code elimination
    DeadCode,
    /// Inlining decisions
    Inlining,
    /// Generic optimization (let LLM decide)
    General,
}

impl OptimizationDomain {
    /// Get all available domains
    pub fn all() -> Vec<OptimizationDomain> {
        vec![
            OptimizationDomain::Simd,
            OptimizationDomain::Loops,
            OptimizationDomain::Memory,
            OptimizationDomain::Algorithm,
            OptimizationDomain::Parallel,
            OptimizationDomain::Constants,
            OptimizationDomain::DeadCode,
            OptimizationDomain::Inlining,
            OptimizationDomain::General,
        ]
    }

    /// Get the name of this domain
    pub fn name(&self) -> &'static str {
        match self {
            OptimizationDomain::Simd => "simd",
            OptimizationDomain::Loops => "loops",
            OptimizationDomain::Memory => "memory",
            OptimizationDomain::Algorithm => "algorithm",
            OptimizationDomain::Parallel => "parallel",
            OptimizationDomain::Constants => "constants",
            OptimizationDomain::DeadCode => "dead_code",
            OptimizationDomain::Inlining => "inlining",
            OptimizationDomain::General => "general",
        }
    }
}

/// Hints for the optimization request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationHint {
    /// The type of hint
    pub kind: OptimizationHintKind,
    /// Additional context
    pub context: Option<String>,
}

/// Types of optimization hints
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizationHintKind {
    /// Prefer speed over size
    PreferSpeed,
    /// Prefer size over speed
    PreferSize,
    /// Target specific architecture
    TargetArch(String),
    /// Hot path (frequently executed)
    HotPath,
    /// Cold path (rarely executed)
    ColdPath,
    /// Numeric computation
    Numeric,
    /// String processing
    StringProcessing,
    /// Collection operations
    Collections,
    /// I/O bound
    IoBound,
    /// Memory constrained
    MemoryConstrained,
    /// Custom hint
    Custom(String),
}

/// Request for LLM optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRequest {
    /// Unique identifier for this request
    pub id: String,

    /// Function name being optimized
    pub function_name: SmolStr,

    /// Source code or MIR representation
    pub code: String,

    /// Code format (source, mir, ast)
    pub format: CodeFormat,

    /// Domains to consider for optimization
    pub domains: Vec<OptimizationDomain>,

    /// Optimization hints
    pub hints: Vec<OptimizationHint>,

    /// Type information for the function
    pub type_info: Option<TypeContext>,

    /// Contract information (pre/post conditions)
    pub contracts: Option<ContractInfo>,

    /// Maximum number of suggestions to return
    pub max_suggestions: usize,

    /// Whether to include explanation with suggestions
    pub include_explanation: bool,
}

/// Format of the code in the request
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CodeFormat {
    /// Aria source code
    Source,
    /// MIR representation
    Mir,
    /// AST JSON
    Ast,
}

/// Type context for the function
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TypeContext {
    /// Parameter types
    pub parameters: Vec<TypeInfo>,
    /// Return type
    pub return_type: Option<TypeInfo>,
    /// Generic type parameters
    pub generics: Vec<String>,
}

/// Type information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeInfo {
    /// Type name
    pub name: SmolStr,
    /// Is it a primitive type
    pub is_primitive: bool,
    /// Is it an array/collection
    pub is_collection: bool,
    /// Element type (for collections)
    pub element_type: Option<Box<TypeInfo>>,
    /// Size hint (if known)
    pub size_hint: Option<usize>,
}

/// Contract information for the function
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContractInfo {
    /// Preconditions (requires clauses)
    pub preconditions: Vec<String>,
    /// Postconditions (ensures clauses)
    pub postconditions: Vec<String>,
    /// Invariants
    pub invariants: Vec<String>,
}

impl OptimizationRequest {
    /// Create a new optimization request
    pub fn new(function_name: impl Into<SmolStr>, code: String) -> Self {
        Self {
            id: generate_request_id(),
            function_name: function_name.into(),
            code,
            format: CodeFormat::Source,
            domains: vec![OptimizationDomain::General],
            hints: vec![],
            type_info: None,
            contracts: None,
            max_suggestions: 3,
            include_explanation: true,
        }
    }

    /// Set the code format
    pub fn with_format(mut self, format: CodeFormat) -> Self {
        self.format = format;
        self
    }

    /// Set the domains to consider
    pub fn with_domains(mut self, domains: Vec<OptimizationDomain>) -> Self {
        self.domains = domains;
        self
    }

    /// Add an optimization hint
    pub fn with_hint(mut self, hint: OptimizationHint) -> Self {
        self.hints.push(hint);
        self
    }

    /// Set type context
    pub fn with_types(mut self, type_info: TypeContext) -> Self {
        self.type_info = Some(type_info);
        self
    }

    /// Set contract information
    pub fn with_contracts(mut self, contracts: ContractInfo) -> Self {
        self.contracts = Some(contracts);
        self
    }

    /// Compute a cache key for this request
    pub fn cache_key(&self) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&self.code);
        hasher.update(format!("{:?}", self.domains));
        hasher.update(format!("{:?}", self.hints));
        format!("{:x}", hasher.finalize())
    }
}

fn generate_request_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("opt-{:x}", timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimization_request() {
        let request = OptimizationRequest::new("matrix_multiply", "fn matrix_multiply(a, b) ... end".to_string())
            .with_domains(vec![OptimizationDomain::Simd, OptimizationDomain::Loops])
            .with_hint(OptimizationHint {
                kind: OptimizationHintKind::Numeric,
                context: Some("matrix operations".to_string()),
            });

        assert_eq!(request.function_name.as_str(), "matrix_multiply");
        assert_eq!(request.domains.len(), 2);
        assert_eq!(request.hints.len(), 1);
    }

    #[test]
    fn test_cache_key() {
        let request1 = OptimizationRequest::new("foo", "code".to_string());
        let request2 = OptimizationRequest::new("foo", "code".to_string());
        let request3 = OptimizationRequest::new("foo", "different code".to_string());

        assert_eq!(request1.cache_key(), request2.cache_key());
        assert_ne!(request1.cache_key(), request3.cache_key());
    }
}
