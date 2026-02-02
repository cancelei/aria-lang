//! Aria Contract Verification
//!
//! Implements tiered contract verification as defined in ARIA-PD-003:
//! - Tier 1 (Static): SMT-solvable contracts verified at compile time
//! - Tier 2 (Cached): Pure method contracts with abstract interpretation
//! - Tier 3 (Dynamic): Runtime-only contracts (quantifiers, closures)
//!
//! # Contract Modes
//!
//! - `Static`: Only Tier 1 static checks (default for release builds)
//! - `Full`: All tiers checked (development default)
//! - `Runtime`: All contracts checked at runtime
//! - `Off`: No contract checking (use sparingly)
//!
//! # Tier Classification
//!
//! Contracts are automatically classified based on expression complexity:
//!
//! | Pattern | Tier | Method |
//! |---------|------|--------|
//! | Null checks (`x != nil`) | Tier 1 | SMT |
//! | Linear arithmetic (`x + y < z`) | Tier 1 | SMT |
//! | Type guards (`x is T`) | Tier 1 | Type system |
//! | Boolean combinations | Tier 1 | SMT |
//! | Pure method calls (`arr.sorted?`) | Tier 2 | Abstract interpretation |
//! | Collection predicates (`list.all?`) | Tier 2 | Abstract interpretation |
//! | Universal quantifiers (`forall`) | Tier 3 | Runtime |
//! | Existential quantifiers (`exists`) | Tier 3 | Runtime |
//! | Opaque closures | Tier 3 | Runtime |
//! | Non-linear arithmetic (`x * y > z * w`) | Tier 3 | Runtime |

use aria_ast::{BinaryOp, Contract, Expr, ExprKind, UnaryOp};
use thiserror::Error;

#[cfg(test)]
use smol_str::SmolStr;

// ============================================================================
// Contract Tiers
// ============================================================================

/// Contract verification tier
///
/// Contracts are automatically classified into one of three tiers based on
/// their complexity and whether they can be statically verified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContractTier {
    /// Tier 1: Static SMT verification
    ///
    /// These contracts can be fully verified at compile time using an SMT solver.
    /// Examples: null checks, linear arithmetic, boolean combinations.
    Tier1Static,

    /// Tier 2: Cached with abstract interpretation
    ///
    /// These contracts involve pure method calls that can be analyzed with
    /// abstract interpretation. Results are cached for efficiency.
    /// Examples: `arr.sorted?`, `list.all?`, immutable field access.
    Tier2Cached,

    /// Tier 3: Runtime-only verification
    ///
    /// These contracts cannot be statically verified and must be checked at runtime.
    /// Examples: quantifiers (`forall`, `exists`), opaque closures, non-linear arithmetic.
    Tier3Dynamic,
}

impl ContractTier {
    /// Returns true if this tier can be verified statically
    pub fn is_static(&self) -> bool {
        matches!(self, ContractTier::Tier1Static)
    }

    /// Returns true if this tier requires runtime verification
    pub fn requires_runtime(&self) -> bool {
        matches!(self, ContractTier::Tier3Dynamic)
    }

    /// Returns a human-readable description of this tier
    pub fn description(&self) -> &'static str {
        match self {
            ContractTier::Tier1Static => "Static SMT verification",
            ContractTier::Tier2Cached => "Abstract interpretation with caching",
            ContractTier::Tier3Dynamic => "Runtime-only verification",
        }
    }
}

// ============================================================================
// Contract Mode
// ============================================================================

/// Contract checking mode
///
/// Controls which tiers of contracts are verified and when.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ContractMode {
    /// Only Tier 1 static checks (default for release builds)
    ///
    /// Zero runtime overhead. Contracts that cannot be statically verified
    /// emit warnings but are not checked at runtime.
    Static,

    /// All tiers checked (development default)
    ///
    /// Tier 1: Verified at compile time
    /// Tier 2: Cached abstract interpretation
    /// Tier 3: Runtime checks
    #[default]
    Full,

    /// All contracts checked at runtime
    ///
    /// Even statically-provable contracts are checked at runtime.
    /// Useful for debugging contract-related issues.
    Runtime,

    /// No contract checking
    ///
    /// Use sparingly. All contracts are completely ignored.
    Off,
}

impl ContractMode {
    /// Returns true if contracts should be checked in this mode
    pub fn is_enabled(&self) -> bool {
        !matches!(self, ContractMode::Off)
    }

    /// Returns true if static verification should be attempted
    pub fn should_verify_static(&self) -> bool {
        matches!(self, ContractMode::Static | ContractMode::Full)
    }

    /// Returns true if runtime checks should be generated for this tier
    pub fn should_check_runtime(&self, tier: ContractTier) -> bool {
        match self {
            ContractMode::Static => false,
            ContractMode::Full => matches!(tier, ContractTier::Tier2Cached | ContractTier::Tier3Dynamic),
            ContractMode::Runtime => true,
            ContractMode::Off => false,
        }
    }

    /// Parse contract mode from string (for CLI/config)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "static" => Some(ContractMode::Static),
            "full" => Some(ContractMode::Full),
            "runtime" => Some(ContractMode::Runtime),
            "off" => Some(ContractMode::Off),
            _ => None,
        }
    }
}

// ============================================================================
// Classification Result
// ============================================================================

/// Result of contract classification
///
/// Contains the assigned tier and the reason for classification.
#[derive(Debug, Clone, PartialEq)]
pub struct ContractClassification {
    /// The verification tier assigned to this contract
    pub tier: ContractTier,

    /// Human-readable explanation of why this tier was assigned
    pub reason: String,

    /// Expression patterns that contributed to the classification
    pub patterns: Vec<ExpressionPattern>,
}

impl ContractClassification {
    /// Create a new classification
    pub fn new(tier: ContractTier, reason: impl Into<String>) -> Self {
        Self {
            tier,
            reason: reason.into(),
            patterns: Vec::new(),
        }
    }

    /// Add a pattern that contributed to this classification
    pub fn with_pattern(mut self, pattern: ExpressionPattern) -> Self {
        self.patterns.push(pattern);
        self
    }
}

/// Patterns detected in contract expressions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpressionPattern {
    /// Null/nil check
    NullCheck,
    /// Linear arithmetic expression
    LinearArithmetic,
    /// Non-linear arithmetic (multiplication of variables)
    NonLinearArithmetic,
    /// Boolean combination (and/or)
    BooleanCombination,
    /// Type guard (`is` expression)
    TypeGuard,
    /// Pure method call
    PureMethodCall,
    /// Quantified expression (forall/exists)
    Quantifier,
    /// Lambda/closure reference
    Closure,
    /// Field access
    FieldAccess,
    /// Collection predicate (all?, any?, etc.)
    CollectionPredicate,
    /// Unknown/opaque expression
    Opaque,
}

// ============================================================================
// Verification Result
// ============================================================================

/// Result of contract verification
#[derive(Debug, Clone, PartialEq)]
pub enum VerificationResult {
    /// Contract was successfully verified (statically proven true)
    Verified,

    /// Contract was proven false with a counterexample
    Refuted {
        /// Description of the counterexample that violates the contract
        counterexample: String,
    },

    /// Verification timed out before completion
    Timeout {
        /// Duration in milliseconds before timeout occurred
        duration_ms: u64,
    },

    /// Contract verification was deferred to runtime
    DeferredToRuntime {
        /// Reason for deferral
        reason: String,
    },

    /// Verification encountered an error
    Error(VerificationError),
}

impl VerificationResult {
    /// Returns true if the contract was successfully verified
    pub fn is_verified(&self) -> bool {
        matches!(self, VerificationResult::Verified)
    }

    /// Returns true if the contract was proven false
    pub fn is_refuted(&self) -> bool {
        matches!(self, VerificationResult::Refuted { .. })
    }
}

/// Errors that can occur during verification
#[derive(Debug, Clone, PartialEq, Error)]
pub enum VerificationError {
    #[error("SMT solver not available")]
    SmtNotAvailable,

    #[error("Memory limit exceeded during verification")]
    MemoryLimitExceeded,

    #[error("Contract expression too complex: {0}")]
    TooComplex(String),

    #[error("Unsupported expression in contract: {0}")]
    UnsupportedExpression(String),

    #[error("Internal verification error: {0}")]
    Internal(String),
}

// ============================================================================
// Verifier Configuration
// ============================================================================

/// Configuration for contract verification
#[derive(Debug, Clone)]
pub struct VerifierConfig {
    /// Contract checking mode
    pub mode: ContractMode,

    /// SMT solver timeout in milliseconds (default: 5000)
    pub timeout_ms: u64,

    /// Maximum memory for verification in bytes (default: 64MB)
    pub max_memory_bytes: usize,

    /// Whether to show tier classification warnings
    pub show_tier_warnings: bool,

    /// Whether to cache verification results
    pub cache_enabled: bool,
}

impl Default for VerifierConfig {
    fn default() -> Self {
        Self {
            mode: ContractMode::Full,
            timeout_ms: 5000,
            max_memory_bytes: 64 * 1024 * 1024, // 64MB
            show_tier_warnings: true,
            cache_enabled: true,
        }
    }
}

impl VerifierConfig {
    /// Create a configuration for release builds (static mode only)
    pub fn release() -> Self {
        Self {
            mode: ContractMode::Static,
            ..Default::default()
        }
    }

    /// Create a configuration for development builds
    pub fn development() -> Self {
        Self {
            mode: ContractMode::Full,
            ..Default::default()
        }
    }

    /// Create a configuration with contracts disabled
    pub fn disabled() -> Self {
        Self {
            mode: ContractMode::Off,
            ..Default::default()
        }
    }
}

// ============================================================================
// Contract Verifier
// ============================================================================

/// Contract verifier for the Aria language
///
/// Provides tiered contract verification as specified in ARIA-PD-003.
pub struct ContractVerifier {
    config: VerifierConfig,
    // Placeholder for SMT solver integration
    // In the future, this will hold a Z3 context or similar
}

impl ContractVerifier {
    /// Create a new contract verifier with the given configuration
    pub fn new(config: VerifierConfig) -> Self {
        Self { config }
    }

    /// Create a verifier with default configuration
    pub fn with_defaults() -> Self {
        Self::new(VerifierConfig::default())
    }

    /// Get the current verification mode
    pub fn mode(&self) -> ContractMode {
        self.config.mode
    }

    /// Get the current configuration
    pub fn config(&self) -> &VerifierConfig {
        &self.config
    }

    /// Classify a contract into its verification tier
    ///
    /// Analyzes the contract expression to determine which tier of verification
    /// is appropriate, based on the patterns found in the expression.
    pub fn classify(&self, contract: &Contract) -> ContractClassification {
        let clause = match contract {
            Contract::Requires(clause) => clause,
            Contract::Ensures(clause) => clause,
            Contract::Invariant(clause) => clause,
        };

        self.classify_expression(&clause.condition)
    }

    /// Classify an expression for contract tier
    fn classify_expression(&self, expr: &Expr) -> ContractClassification {
        let (tier, reason, patterns) = self.analyze_expression(expr);
        let mut classification = ContractClassification::new(tier, reason);
        for pattern in patterns {
            classification = classification.with_pattern(pattern);
        }
        classification
    }

    /// Analyze an expression to determine its tier
    fn analyze_expression(&self, expr: &Expr) -> (ContractTier, String, Vec<ExpressionPattern>) {
        let mut patterns = Vec::new();
        let tier = self.compute_tier(expr, &mut patterns);

        let reason = match tier {
            ContractTier::Tier1Static => {
                "Expression contains only SMT-decidable patterns".to_string()
            }
            ContractTier::Tier2Cached => {
                "Expression contains pure method calls suitable for abstract interpretation"
                    .to_string()
            }
            ContractTier::Tier3Dynamic => {
                let dynamic_patterns: Vec<_> = patterns
                    .iter()
                    .filter(|p| is_tier3_pattern(p))
                    .map(|p| format!("{:?}", p))
                    .collect();
                format!(
                    "Expression contains runtime-only patterns: {}",
                    dynamic_patterns.join(", ")
                )
            }
        };

        (tier, reason, patterns)
    }

    /// Recursively compute the tier for an expression
    fn compute_tier(&self, expr: &Expr, patterns: &mut Vec<ExpressionPattern>) -> ContractTier {
        match &expr.kind {
            // Tier 1: Literals are always SMT-decidable
            ExprKind::Integer(_)
            | ExprKind::Float(_)
            | ExprKind::Bool(_)
            | ExprKind::Nil
            | ExprKind::String(_)
            | ExprKind::Char(_) => ContractTier::Tier1Static,

            // Tier 1: Simple identifiers
            ExprKind::Ident(_) | ExprKind::SelfLower | ExprKind::SelfUpper => {
                ContractTier::Tier1Static
            }

            // Binary operations - analyze based on operator and operands
            ExprKind::Binary { op, left, right } => {
                self.classify_binary_op(*op, left, right, patterns)
            }

            // Unary operations - usually Tier 1
            ExprKind::Unary { op, operand } => {
                let operand_tier = self.compute_tier(operand, patterns);
                match op {
                    UnaryOp::Neg | UnaryOp::Not | UnaryOp::BitNot => operand_tier,
                    UnaryOp::Ref | UnaryOp::Deref => operand_tier,
                }
            }

            // Tier 3: Quantifiers always require runtime verification
            ExprKind::Forall { .. } => {
                patterns.push(ExpressionPattern::Quantifier);
                ContractTier::Tier3Dynamic
            }
            ExprKind::Exists { .. } => {
                patterns.push(ExpressionPattern::Quantifier);
                ContractTier::Tier3Dynamic
            }

            // Tier 2/3: Method calls depend on purity
            ExprKind::MethodCall {
                object,
                method,
                args,
            } => self.classify_method_call(object, method, args, patterns),

            // Tier 2: Field access on immutable data
            ExprKind::Field { object, .. } => {
                patterns.push(ExpressionPattern::FieldAccess);
                let object_tier = self.compute_tier(object, patterns);
                max_tier(object_tier, ContractTier::Tier1Static)
            }

            // Tier 2/3: Index depends on the expression
            ExprKind::Index { object, index } => {
                let object_tier = self.compute_tier(object, patterns);
                let index_tier = self.compute_tier(index, patterns);
                max_tier(object_tier, index_tier)
            }

            // Tier 3: Lambdas and closures
            ExprKind::Lambda { .. } | ExprKind::BlockLambda { .. } => {
                patterns.push(ExpressionPattern::Closure);
                ContractTier::Tier3Dynamic
            }

            // Tier 1: Control flow with SMT-decidable branches
            ExprKind::If {
                condition,
                then_branch,
                elsif_branches,
                else_branch,
            } => {
                let mut max = self.compute_tier(condition, patterns);
                for stmt in &then_branch.stmts {
                    if let aria_ast::StmtKind::Expr(e) = &stmt.kind {
                        max = max_tier(max, self.compute_tier(e, patterns));
                    }
                }
                for (cond, block) in elsif_branches {
                    max = max_tier(max, self.compute_tier(cond, patterns));
                    for stmt in &block.stmts {
                        if let aria_ast::StmtKind::Expr(e) = &stmt.kind {
                            max = max_tier(max, self.compute_tier(e, patterns));
                        }
                    }
                }
                if let Some(block) = else_branch {
                    for stmt in &block.stmts {
                        if let aria_ast::StmtKind::Expr(e) = &stmt.kind {
                            max = max_tier(max, self.compute_tier(e, patterns));
                        }
                    }
                }
                max
            }

            // Tier 1: Parenthesized expressions
            ExprKind::Paren(inner) => self.compute_tier(inner, patterns),

            // Tier 1: Ternary expressions
            ExprKind::Ternary {
                condition,
                then_expr,
                else_expr,
            } => {
                let cond_tier = self.compute_tier(condition, patterns);
                let then_tier = self.compute_tier(then_expr, patterns);
                let else_tier = self.compute_tier(else_expr, patterns);
                max_tier(cond_tier, max_tier(then_tier, else_tier))
            }

            // Contract-specific expressions
            ExprKind::Old(inner) => self.compute_tier(inner, patterns),
            ExprKind::Result => ContractTier::Tier1Static,

            // Tier 3: Function calls (generally opaque)
            ExprKind::Call { func, args } => {
                patterns.push(ExpressionPattern::Opaque);
                let mut max = self.compute_tier(func, patterns);
                for arg in args {
                    max = max_tier(max, self.compute_tier(&arg.value, patterns));
                }
                // Function calls are at least Tier 2 (need analysis)
                max_tier(max, ContractTier::Tier2Cached)
            }

            // Collections - analyze element tiers
            ExprKind::Array(elements) | ExprKind::Tuple(elements) => {
                let mut max = ContractTier::Tier1Static;
                for elem in elements {
                    max = max_tier(max, self.compute_tier(elem, patterns));
                }
                max
            }

            // Map literals
            ExprKind::Map(pairs) => {
                let mut max = ContractTier::Tier1Static;
                for (k, v) in pairs {
                    max = max_tier(max, self.compute_tier(k, patterns));
                    max = max_tier(max, self.compute_tier(v, patterns));
                }
                max
            }

            // Default: treat as opaque (Tier 3)
            _ => {
                patterns.push(ExpressionPattern::Opaque);
                ContractTier::Tier3Dynamic
            }
        }
    }

    /// Classify a binary operation
    fn classify_binary_op(
        &self,
        op: BinaryOp,
        left: &Expr,
        right: &Expr,
        patterns: &mut Vec<ExpressionPattern>,
    ) -> ContractTier {
        let left_tier = self.compute_tier(left, patterns);
        let right_tier = self.compute_tier(right, patterns);

        match op {
            // Tier 1: Comparison operators
            BinaryOp::Eq | BinaryOp::NotEq | BinaryOp::Lt | BinaryOp::Gt
            | BinaryOp::LtEq | BinaryOp::GtEq | BinaryOp::Spaceship | BinaryOp::ApproxEq => {
                // Check for null check pattern
                if is_nil_comparison(left, right) {
                    patterns.push(ExpressionPattern::NullCheck);
                }
                max_tier(left_tier, right_tier)
            }

            // Tier 1: Boolean combinations
            BinaryOp::And | BinaryOp::Or => {
                patterns.push(ExpressionPattern::BooleanCombination);
                max_tier(left_tier, right_tier)
            }

            // Tier 1: Linear arithmetic
            BinaryOp::Add | BinaryOp::Sub => {
                patterns.push(ExpressionPattern::LinearArithmetic);
                max_tier(left_tier, right_tier)
            }

            // Tier 3: Non-linear arithmetic (variable * variable)
            BinaryOp::Mul | BinaryOp::Div | BinaryOp::Pow => {
                // If both sides are variable expressions, this is non-linear
                if is_variable_expr(left) && is_variable_expr(right) {
                    patterns.push(ExpressionPattern::NonLinearArithmetic);
                    ContractTier::Tier3Dynamic
                } else {
                    patterns.push(ExpressionPattern::LinearArithmetic);
                    max_tier(left_tier, right_tier)
                }
            }

            // Tier 1: Type guard
            BinaryOp::Is => {
                patterns.push(ExpressionPattern::TypeGuard);
                ContractTier::Tier1Static
            }

            // Tier 2: 'in' operator may require collection analysis
            BinaryOp::In => {
                patterns.push(ExpressionPattern::CollectionPredicate);
                max_tier(max_tier(left_tier, right_tier), ContractTier::Tier2Cached)
            }

            // Other operators - use the max of operand tiers
            _ => max_tier(left_tier, right_tier),
        }
    }

    /// Classify a method call
    fn classify_method_call(
        &self,
        object: &Expr,
        method: &aria_ast::Ident,
        args: &[Expr],
        patterns: &mut Vec<ExpressionPattern>,
    ) -> ContractTier {
        let object_tier = self.compute_tier(object, patterns);

        // Check for known pure methods
        let method_name = method.node.as_str();
        if is_collection_predicate(method_name) {
            patterns.push(ExpressionPattern::CollectionPredicate);

            // Check if args contain closures
            for arg in args {
                if matches!(
                    arg.kind,
                    ExprKind::Lambda { .. } | ExprKind::BlockLambda { .. }
                ) {
                    patterns.push(ExpressionPattern::Closure);
                    return ContractTier::Tier3Dynamic;
                }
            }

            return max_tier(object_tier, ContractTier::Tier2Cached);
        }

        if is_pure_method(method_name) {
            patterns.push(ExpressionPattern::PureMethodCall);
            return max_tier(object_tier, ContractTier::Tier2Cached);
        }

        // Unknown method - treat as opaque
        patterns.push(ExpressionPattern::Opaque);
        ContractTier::Tier3Dynamic
    }

    /// Verify a Tier 1 contract statically
    ///
    /// Uses SMT solving to verify the contract at compile time.
    /// This is a placeholder implementation - actual SMT integration will come later.
    pub fn verify_static(&self, contract: &Contract) -> VerificationResult {
        if !self.config.mode.should_verify_static() {
            return VerificationResult::DeferredToRuntime {
                reason: "Static verification disabled in current mode".to_string(),
            };
        }

        let classification = self.classify(contract);

        if classification.tier != ContractTier::Tier1Static {
            return VerificationResult::DeferredToRuntime {
                reason: format!(
                    "Contract classified as {:?}, not suitable for static verification",
                    classification.tier
                ),
            };
        }

        // Placeholder: In the future, this will invoke the SMT solver
        // For now, we always defer non-trivial contracts
        let clause = match contract {
            Contract::Requires(c) | Contract::Ensures(c) | Contract::Invariant(c) => c,
        };

        // Handle trivially true cases
        if let ExprKind::Bool(true) = clause.condition.kind {
            return VerificationResult::Verified;
        }

        // Handle trivially false cases
        if let ExprKind::Bool(false) = clause.condition.kind {
            return VerificationResult::Refuted {
                counterexample: "Contract is always false".to_string(),
            };
        }

        // For non-trivial cases, defer to runtime until SMT is integrated
        VerificationResult::DeferredToRuntime {
            reason: "SMT solver integration pending".to_string(),
        }
    }

    /// Verify all contracts for a function
    pub fn verify_contracts(&self, contracts: &[Contract]) -> Vec<(Contract, VerificationResult)> {
        contracts
            .iter()
            .map(|c| (c.clone(), self.verify_static(c)))
            .collect()
    }

    /// Check if a contract needs runtime verification in the current mode
    pub fn needs_runtime_check(&self, contract: &Contract) -> bool {
        let classification = self.classify(contract);
        self.config.mode.should_check_runtime(classification.tier)
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Returns the higher (less static) of two tiers
fn max_tier(a: ContractTier, b: ContractTier) -> ContractTier {
    match (a, b) {
        (ContractTier::Tier3Dynamic, _) | (_, ContractTier::Tier3Dynamic) => {
            ContractTier::Tier3Dynamic
        }
        (ContractTier::Tier2Cached, _) | (_, ContractTier::Tier2Cached) => ContractTier::Tier2Cached,
        _ => ContractTier::Tier1Static,
    }
}

/// Check if an expression is a variable (not a literal)
fn is_variable_expr(expr: &Expr) -> bool {
    matches!(
        expr.kind,
        ExprKind::Ident(_)
            | ExprKind::Field { .. }
            | ExprKind::Index { .. }
            | ExprKind::SelfLower
            | ExprKind::SelfUpper
    )
}

/// Check if this is a null/nil comparison
fn is_nil_comparison(left: &Expr, right: &Expr) -> bool {
    matches!(left.kind, ExprKind::Nil) || matches!(right.kind, ExprKind::Nil)
}

/// Check if a method name is a known collection predicate
fn is_collection_predicate(method: &str) -> bool {
    matches!(
        method,
        "all?" | "any?" | "none?" | "empty?" | "contains?" | "find" | "filter" | "map" | "reduce"
    )
}

/// Check if a method name is a known pure method
fn is_pure_method(method: &str) -> bool {
    matches!(
        method,
        "sorted?"
            | "valid?"
            | "length"
            | "size"
            | "count"
            | "first"
            | "last"
            | "min"
            | "max"
            | "sum"
            | "to_string"
            | "clone"
    )
}

/// Check if a pattern requires Tier 3 (runtime) verification
fn is_tier3_pattern(pattern: &ExpressionPattern) -> bool {
    matches!(
        pattern,
        ExpressionPattern::Quantifier
            | ExpressionPattern::Closure
            | ExpressionPattern::NonLinearArithmetic
            | ExpressionPattern::Opaque
    )
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use aria_ast::{ContractClause, Span};

    fn make_expr(kind: ExprKind) -> Expr {
        Expr {
            kind,
            span: Span::dummy(),
        }
    }

    fn make_contract(expr: Expr) -> Contract {
        Contract::Requires(ContractClause {
            condition: Box::new(expr),
            message: None,
            span: Span::dummy(),
        })
    }

    #[test]
    fn test_literal_classification() {
        let verifier = ContractVerifier::with_defaults();

        // Boolean literal should be Tier 1
        let contract = make_contract(make_expr(ExprKind::Bool(true)));
        let classification = verifier.classify(&contract);
        assert_eq!(classification.tier, ContractTier::Tier1Static);
    }

    #[test]
    fn test_quantifier_classification() {
        let verifier = ContractVerifier::with_defaults();

        // Forall should be Tier 3
        let contract = make_contract(make_expr(ExprKind::Forall {
            var: aria_ast::Spanned::dummy(SmolStr::new("x")),
            ty: aria_ast::TypeExpr::Named(aria_ast::Spanned::dummy(SmolStr::new("Int"))),
            condition: None,
            body: Box::new(make_expr(ExprKind::Bool(true))),
        }));
        let classification = verifier.classify(&contract);
        assert_eq!(classification.tier, ContractTier::Tier3Dynamic);
        assert!(classification.patterns.contains(&ExpressionPattern::Quantifier));
    }

    #[test]
    fn test_comparison_classification() {
        let verifier = ContractVerifier::with_defaults();

        // x > 0 should be Tier 1
        let contract = make_contract(make_expr(ExprKind::Binary {
            op: BinaryOp::Gt,
            left: Box::new(make_expr(ExprKind::Ident(SmolStr::new("x")))),
            right: Box::new(make_expr(ExprKind::Integer(SmolStr::new("0")))),
        }));
        let classification = verifier.classify(&contract);
        assert_eq!(classification.tier, ContractTier::Tier1Static);
    }

    #[test]
    fn test_null_check_pattern() {
        let verifier = ContractVerifier::with_defaults();

        // x != nil should be Tier 1 with NullCheck pattern
        let contract = make_contract(make_expr(ExprKind::Binary {
            op: BinaryOp::NotEq,
            left: Box::new(make_expr(ExprKind::Ident(SmolStr::new("x")))),
            right: Box::new(make_expr(ExprKind::Nil)),
        }));
        let classification = verifier.classify(&contract);
        assert_eq!(classification.tier, ContractTier::Tier1Static);
        assert!(classification.patterns.contains(&ExpressionPattern::NullCheck));
    }

    #[test]
    fn test_nonlinear_arithmetic() {
        let verifier = ContractVerifier::with_defaults();

        // x * y should be Tier 3 (non-linear)
        let contract = make_contract(make_expr(ExprKind::Binary {
            op: BinaryOp::Mul,
            left: Box::new(make_expr(ExprKind::Ident(SmolStr::new("x")))),
            right: Box::new(make_expr(ExprKind::Ident(SmolStr::new("y")))),
        }));
        let classification = verifier.classify(&contract);
        assert_eq!(classification.tier, ContractTier::Tier3Dynamic);
        assert!(classification.patterns.contains(&ExpressionPattern::NonLinearArithmetic));
    }

    #[test]
    fn test_linear_arithmetic_with_constant() {
        let verifier = ContractVerifier::with_defaults();

        // x * 2 should be Tier 1 (linear - one side is constant)
        let contract = make_contract(make_expr(ExprKind::Binary {
            op: BinaryOp::Mul,
            left: Box::new(make_expr(ExprKind::Ident(SmolStr::new("x")))),
            right: Box::new(make_expr(ExprKind::Integer(SmolStr::new("2")))),
        }));
        let classification = verifier.classify(&contract);
        assert_eq!(classification.tier, ContractTier::Tier1Static);
    }

    #[test]
    fn test_type_guard() {
        let verifier = ContractVerifier::with_defaults();

        // x is Int should be Tier 1
        let contract = make_contract(make_expr(ExprKind::Binary {
            op: BinaryOp::Is,
            left: Box::new(make_expr(ExprKind::Ident(SmolStr::new("x")))),
            right: Box::new(make_expr(ExprKind::Ident(SmolStr::new("Int")))),
        }));
        let classification = verifier.classify(&contract);
        assert_eq!(classification.tier, ContractTier::Tier1Static);
        assert!(classification.patterns.contains(&ExpressionPattern::TypeGuard));
    }

    #[test]
    fn test_contract_modes() {
        assert!(ContractMode::Static.should_verify_static());
        assert!(ContractMode::Full.should_verify_static());
        assert!(!ContractMode::Runtime.should_verify_static());
        assert!(!ContractMode::Off.should_verify_static());

        assert!(!ContractMode::Static.should_check_runtime(ContractTier::Tier1Static));
        assert!(!ContractMode::Static.should_check_runtime(ContractTier::Tier3Dynamic));

        assert!(!ContractMode::Full.should_check_runtime(ContractTier::Tier1Static));
        assert!(ContractMode::Full.should_check_runtime(ContractTier::Tier2Cached));
        assert!(ContractMode::Full.should_check_runtime(ContractTier::Tier3Dynamic));

        assert!(ContractMode::Runtime.should_check_runtime(ContractTier::Tier1Static));
        assert!(ContractMode::Runtime.should_check_runtime(ContractTier::Tier3Dynamic));

        assert!(!ContractMode::Off.should_check_runtime(ContractTier::Tier1Static));
    }

    #[test]
    fn test_trivial_verification() {
        let verifier = ContractVerifier::with_defaults();

        // `true` should verify
        let contract = make_contract(make_expr(ExprKind::Bool(true)));
        let result = verifier.verify_static(&contract);
        assert!(result.is_verified());

        // `false` should be refuted
        let contract = make_contract(make_expr(ExprKind::Bool(false)));
        let result = verifier.verify_static(&contract);
        assert!(result.is_refuted());
    }
}
