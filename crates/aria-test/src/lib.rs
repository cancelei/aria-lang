//! Aria Testing Framework
//!
//! A comprehensive testing framework for the Aria programming language as
//! specified in ARIA-PD-011. This crate provides:
//!
//! - **Unit Testing**: Test case representation, assertions, and test suites
//! - **Property-Based Testing**: Generator trait and arbitrary value generation
//! - **Test Runner**: Parallel execution and test filtering
//!
//! # Architecture
//!
//! The testing framework follows a four-layer architecture:
//!
//! 1. **Layer 1 - Unit Tests**: Explicit assertions with clear failure messages
//! 2. **Layer 2 - Property Tests**: Random input generation with shrinking
//! 3. **Layer 3 - Contract Tests**: Generated from requires/ensures clauses
//! 4. **Layer 4 - Fuzz Tests**: Coverage-guided mutation testing
//!
//! # Example
//!
//! ```rust
//! use aria_test::{TestCase, TestSuite, TestContext, TestResult};
//!
//! // Create a test case
//! let test = TestCase::new("addition works")
//!     .with_description("Verifies basic addition");
//!
//! // Create a test suite
//! let mut suite = TestSuite::new("Math tests");
//! suite.add_test(test);
//!
//! // Test context for assertions
//! let mut ctx = TestContext::new("example");
//! assert!(ctx.assert_eq(2 + 2, 4).is_ok());
//! ```

pub mod assertions;
pub mod generator;
pub mod property;
pub mod runner;

use rustc_hash::FxHashMap;
use std::time::Duration;
use thiserror::Error;

// Re-exports for convenience
pub use assertions::*;
pub use generator::*;
pub use property::*;
pub use runner::*;

// ============================================================================
// Test Errors
// ============================================================================

/// Errors that can occur during test execution
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TestError {
    /// An assertion failed
    #[error("Assertion failed: {message}")]
    AssertionFailed {
        message: String,
        expected: Option<String>,
        actual: Option<String>,
        location: Option<SourceLocation>,
    },

    /// Test timed out
    #[error("Test timed out after {0:?}")]
    Timeout(Duration),

    /// Test was skipped
    #[error("Test skipped: {0}")]
    Skipped(String),

    /// Test panicked
    #[error("Test panicked: {0}")]
    Panicked(String),

    /// Setup failed
    #[error("Setup failed: {0}")]
    SetupFailed(String),

    /// Teardown failed
    #[error("Teardown failed: {0}")]
    TeardownFailed(String),

    /// Property test found a counterexample
    #[error("Property falsified: {message}")]
    PropertyFalsified {
        message: String,
        counterexample: String,
        shrunk_counterexample: Option<String>,
        seed: u64,
    },

    /// Generator failed to produce a valid value
    #[error("Generator exhausted after {attempts} attempts")]
    GeneratorExhausted { attempts: usize },

    /// Internal test framework error
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Source location for error reporting
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

impl SourceLocation {
    pub fn new(file: impl Into<String>, line: u32, column: u32) -> Self {
        Self {
            file: file.into(),
            line,
            column,
        }
    }
}

impl std::fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}

// ============================================================================
// Test Status
// ============================================================================

/// Status of a test execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestStatus {
    /// Test passed successfully
    Passed,

    /// Test failed with an error
    Failed(TestError),

    /// Test was skipped
    Skipped(String),

    /// Test is pending (not yet implemented)
    Pending,

    /// Test is currently running
    Running,
}

impl TestStatus {
    /// Returns true if the test passed
    pub fn is_passed(&self) -> bool {
        matches!(self, TestStatus::Passed)
    }

    /// Returns true if the test failed
    pub fn is_failed(&self) -> bool {
        matches!(self, TestStatus::Failed(_))
    }

    /// Returns true if the test was skipped
    pub fn is_skipped(&self) -> bool {
        matches!(self, TestStatus::Skipped(_))
    }
}

// ============================================================================
// Test Annotations
// ============================================================================

/// Test annotations that modify test behavior
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TestAnnotations {
    /// Tags for test filtering
    pub tags: Vec<String>,

    /// Timeout override for this test
    pub timeout: Option<Duration>,

    /// Whether this test should be skipped
    pub skip: Option<String>,

    /// Whether this test should be run exclusively
    pub exclusive: bool,

    /// Whether this test is async
    pub is_async: bool,

    /// Parameterized test values
    pub parameters: Option<Vec<FxHashMap<String, String>>>,

    /// Retry count on failure
    pub retries: u32,

    /// Expected to fail (test succeeds if assertion fails)
    pub should_fail: bool,

    /// Fuzzing configuration
    pub fuzz_iterations: Option<usize>,

    /// Property test configuration
    pub property_iterations: Option<usize>,

    /// Shrink limit for property tests
    pub shrink_limit: Option<usize>,
}

impl TestAnnotations {
    /// Create empty annotations
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Mark as skipped
    pub fn skip(mut self, reason: impl Into<String>) -> Self {
        self.skip = Some(reason.into());
        self
    }

    /// Mark as async
    pub fn async_test(mut self) -> Self {
        self.is_async = true;
        self
    }

    /// Set retry count
    pub fn with_retries(mut self, count: u32) -> Self {
        self.retries = count;
        self
    }

    /// Mark as expected to fail
    pub fn should_fail(mut self) -> Self {
        self.should_fail = true;
        self
    }

    /// Set property test iterations
    pub fn with_property_iterations(mut self, count: usize) -> Self {
        self.property_iterations = Some(count);
        self
    }
}

// ============================================================================
// Test Case
// ============================================================================

/// A single test case
#[derive(Clone)]
pub struct TestCase {
    /// Unique identifier for this test
    pub id: String,

    /// Human-readable test name
    pub name: String,

    /// Optional description
    pub description: Option<String>,

    /// Test annotations
    pub annotations: TestAnnotations,

    /// The test function (stored as a trait object would require dyn, using placeholder)
    /// In practice, this would hold the actual test closure
    #[allow(dead_code)]
    test_fn_id: u64,
}

impl std::fmt::Debug for TestCase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestCase")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("description", &self.description)
            .field("annotations", &self.annotations)
            .finish()
    }
}

impl TestCase {
    /// Create a new test case
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        let id = name.replace(' ', "_").to_lowercase();
        Self {
            id,
            name,
            description: None,
            annotations: TestAnnotations::default(),
            test_fn_id: 0,
        }
    }

    /// Set the test description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set test annotations
    pub fn with_annotations(mut self, annotations: TestAnnotations) -> Self {
        self.annotations = annotations;
        self
    }

    /// Add a tag to this test
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.annotations.tags.push(tag.into());
        self
    }

    /// Set timeout for this test
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.annotations.timeout = Some(timeout);
        self
    }

    /// Check if this test should be skipped
    pub fn should_skip(&self) -> Option<&str> {
        self.annotations.skip.as_deref()
    }

    /// Check if this test matches a filter
    pub fn matches_filter(&self, filter: &TestFilter) -> bool {
        filter.matches(self)
    }
}

// ============================================================================
// Test Suite
// ============================================================================

/// A collection of related tests
#[derive(Debug, Clone)]
pub struct TestSuite {
    /// Suite name
    pub name: String,

    /// Suite description
    pub description: Option<String>,

    /// Tests in this suite
    pub tests: Vec<TestCase>,

    /// Nested suites
    pub suites: Vec<TestSuite>,

    /// Suite-level annotations (applied to all tests)
    pub annotations: TestAnnotations,

    /// Setup function identifier (placeholder)
    #[allow(dead_code)]
    setup_fn_id: Option<u64>,

    /// Teardown function identifier (placeholder)
    #[allow(dead_code)]
    teardown_fn_id: Option<u64>,
}

impl TestSuite {
    /// Create a new test suite
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            tests: Vec::new(),
            suites: Vec::new(),
            annotations: TestAnnotations::default(),
            setup_fn_id: None,
            teardown_fn_id: None,
        }
    }

    /// Set suite description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Add a test to this suite
    pub fn add_test(&mut self, test: TestCase) {
        self.tests.push(test);
    }

    /// Add a nested suite
    pub fn add_suite(&mut self, suite: TestSuite) {
        self.suites.push(suite);
    }

    /// Get all tests including from nested suites
    pub fn all_tests(&self) -> Vec<&TestCase> {
        let mut tests: Vec<&TestCase> = self.tests.iter().collect();
        for suite in &self.suites {
            tests.extend(suite.all_tests());
        }
        tests
    }

    /// Count total tests including nested suites
    pub fn test_count(&self) -> usize {
        self.tests.len() + self.suites.iter().map(|s| s.test_count()).sum::<usize>()
    }
}

// ============================================================================
// Test Result
// ============================================================================

/// Result of running a single test
#[derive(Debug, Clone)]
pub struct TestResult {
    /// The test that was run
    pub test_id: String,

    /// Test name
    pub test_name: String,

    /// Result status
    pub status: TestStatus,

    /// Execution duration
    pub duration: Duration,

    /// Output captured during test execution
    pub output: String,

    /// Any additional metadata
    pub metadata: FxHashMap<String, String>,
}

impl TestResult {
    /// Create a new passing result
    pub fn passed(test: &TestCase, duration: Duration) -> Self {
        Self {
            test_id: test.id.clone(),
            test_name: test.name.clone(),
            status: TestStatus::Passed,
            duration,
            output: String::new(),
            metadata: FxHashMap::default(),
        }
    }

    /// Create a new failing result
    pub fn failed(test: &TestCase, error: TestError, duration: Duration) -> Self {
        Self {
            test_id: test.id.clone(),
            test_name: test.name.clone(),
            status: TestStatus::Failed(error),
            duration,
            output: String::new(),
            metadata: FxHashMap::default(),
        }
    }

    /// Create a skipped result
    pub fn skipped(test: &TestCase, reason: impl Into<String>) -> Self {
        Self {
            test_id: test.id.clone(),
            test_name: test.name.clone(),
            status: TestStatus::Skipped(reason.into()),
            duration: Duration::ZERO,
            output: String::new(),
            metadata: FxHashMap::default(),
        }
    }

    /// Add captured output
    pub fn with_output(mut self, output: impl Into<String>) -> Self {
        self.output = output.into();
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Check if this result represents a passed test
    pub fn is_passed(&self) -> bool {
        self.status.is_passed()
    }

    /// Check if this result represents a failed test
    pub fn is_failed(&self) -> bool {
        self.status.is_failed()
    }
}

// ============================================================================
// Suite Result
// ============================================================================

/// Result of running a test suite
#[derive(Debug, Clone)]
pub struct SuiteResult {
    /// Suite name
    pub suite_name: String,

    /// Individual test results
    pub results: Vec<TestResult>,

    /// Nested suite results
    pub suite_results: Vec<SuiteResult>,

    /// Total duration
    pub duration: Duration,
}

impl SuiteResult {
    /// Create a new suite result
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            suite_name: name.into(),
            results: Vec::new(),
            suite_results: Vec::new(),
            duration: Duration::ZERO,
        }
    }

    /// Add a test result
    pub fn add_result(&mut self, result: TestResult) {
        self.results.push(result);
    }

    /// Add a nested suite result
    pub fn add_suite_result(&mut self, result: SuiteResult) {
        self.suite_results.push(result);
    }

    /// Count passed tests
    pub fn passed_count(&self) -> usize {
        self.results.iter().filter(|r| r.is_passed()).count()
            + self
                .suite_results
                .iter()
                .map(|s| s.passed_count())
                .sum::<usize>()
    }

    /// Count failed tests
    pub fn failed_count(&self) -> usize {
        self.results.iter().filter(|r| r.is_failed()).count()
            + self
                .suite_results
                .iter()
                .map(|s| s.failed_count())
                .sum::<usize>()
    }

    /// Count skipped tests
    pub fn skipped_count(&self) -> usize {
        self.results
            .iter()
            .filter(|r| r.status.is_skipped())
            .count()
            + self
                .suite_results
                .iter()
                .map(|s| s.skipped_count())
                .sum::<usize>()
    }

    /// Total test count
    pub fn total_count(&self) -> usize {
        self.results.len()
            + self
                .suite_results
                .iter()
                .map(|s| s.total_count())
                .sum::<usize>()
    }

    /// Check if all tests passed
    pub fn all_passed(&self) -> bool {
        self.failed_count() == 0
    }

    /// Get all failed results
    pub fn failed_results(&self) -> Vec<&TestResult> {
        let mut failed: Vec<&TestResult> = self.results.iter().filter(|r| r.is_failed()).collect();
        for suite in &self.suite_results {
            failed.extend(suite.failed_results());
        }
        failed
    }
}

// ============================================================================
// Test Filter
// ============================================================================

/// Filter for selecting tests to run
#[derive(Debug, Clone, Default)]
pub struct TestFilter {
    /// Name patterns to include (supports glob-like matching)
    pub include_patterns: Vec<String>,

    /// Name patterns to exclude
    pub exclude_patterns: Vec<String>,

    /// Tags to include (test must have at least one)
    pub include_tags: Vec<String>,

    /// Tags to exclude (test must not have any)
    pub exclude_tags: Vec<String>,

    /// Only run tests marked as exclusive
    pub exclusive_only: bool,
}

impl TestFilter {
    /// Create a new empty filter (matches all tests)
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an include pattern
    pub fn include(mut self, pattern: impl Into<String>) -> Self {
        self.include_patterns.push(pattern.into());
        self
    }

    /// Add an exclude pattern
    pub fn exclude(mut self, pattern: impl Into<String>) -> Self {
        self.exclude_patterns.push(pattern.into());
        self
    }

    /// Include tests with a specific tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.include_tags.push(tag.into());
        self
    }

    /// Exclude tests with a specific tag
    pub fn without_tag(mut self, tag: impl Into<String>) -> Self {
        self.exclude_tags.push(tag.into());
        self
    }

    /// Only run exclusive tests
    pub fn exclusive_only(mut self) -> Self {
        self.exclusive_only = true;
        self
    }

    /// Check if a test matches this filter
    pub fn matches(&self, test: &TestCase) -> bool {
        // Check exclusive constraint
        if self.exclusive_only && !test.annotations.exclusive {
            return false;
        }

        // Check exclude patterns first
        for pattern in &self.exclude_patterns {
            if self.pattern_matches(pattern, &test.name) || self.pattern_matches(pattern, &test.id)
            {
                return false;
            }
        }

        // Check exclude tags
        for tag in &self.exclude_tags {
            if test.annotations.tags.contains(tag) {
                return false;
            }
        }

        // Check include patterns
        if !self.include_patterns.is_empty() {
            let matches_any = self.include_patterns.iter().any(|pattern| {
                self.pattern_matches(pattern, &test.name) || self.pattern_matches(pattern, &test.id)
            });
            if !matches_any {
                return false;
            }
        }

        // Check include tags
        if !self.include_tags.is_empty() {
            let has_any_tag = self
                .include_tags
                .iter()
                .any(|tag| test.annotations.tags.contains(tag));
            if !has_any_tag {
                return false;
            }
        }

        true
    }

    /// Simple glob-like pattern matching
    fn pattern_matches(&self, pattern: &str, text: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        if pattern.starts_with('*') && pattern.ends_with('*') {
            let inner = &pattern[1..pattern.len() - 1];
            return text.contains(inner);
        }

        if pattern.starts_with('*') {
            let suffix = &pattern[1..];
            return text.ends_with(suffix);
        }

        if pattern.ends_with('*') {
            let prefix = &pattern[..pattern.len() - 1];
            return text.starts_with(prefix);
        }

        pattern == text
    }
}

// ============================================================================
// Test Context
// ============================================================================

/// Context passed to test functions for assertions and utilities
#[derive(Debug)]
pub struct TestContext {
    /// Current test name
    pub test_name: String,

    /// Captured output
    output: Vec<String>,

    /// Assertion count
    assertion_count: usize,

    /// Metadata collected during test
    metadata: FxHashMap<String, String>,
}

impl TestContext {
    /// Create a new test context
    pub fn new(test_name: impl Into<String>) -> Self {
        Self {
            test_name: test_name.into(),
            output: Vec::new(),
            assertion_count: 0,
            metadata: FxHashMap::default(),
        }
    }

    /// Log output during test
    pub fn log(&mut self, message: impl Into<String>) {
        self.output.push(message.into());
    }

    /// Get captured output
    pub fn output(&self) -> String {
        self.output.join("\n")
    }

    /// Get assertion count
    pub fn assertion_count(&self) -> usize {
        self.assertion_count
    }

    /// Add metadata
    pub fn set_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    /// Get metadata
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Basic assertion
    pub fn assert(&mut self, condition: bool, message: &str) -> Result<(), TestError> {
        self.assertion_count += 1;
        if condition {
            Ok(())
        } else {
            Err(TestError::AssertionFailed {
                message: message.to_string(),
                expected: None,
                actual: None,
                location: None,
            })
        }
    }

    /// Assert equality
    pub fn assert_eq<T: PartialEq + std::fmt::Debug>(
        &mut self,
        actual: T,
        expected: T,
    ) -> Result<(), TestError> {
        self.assertion_count += 1;
        if actual == expected {
            Ok(())
        } else {
            Err(TestError::AssertionFailed {
                message: "Values are not equal".to_string(),
                expected: Some(format!("{:?}", expected)),
                actual: Some(format!("{:?}", actual)),
                location: None,
            })
        }
    }

    /// Assert inequality
    pub fn assert_ne<T: PartialEq + std::fmt::Debug>(
        &mut self,
        actual: T,
        expected: T,
    ) -> Result<(), TestError> {
        self.assertion_count += 1;
        if actual != expected {
            Ok(())
        } else {
            Err(TestError::AssertionFailed {
                message: "Values should not be equal".to_string(),
                expected: Some(format!("not {:?}", expected)),
                actual: Some(format!("{:?}", actual)),
                location: None,
            })
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_case_creation() {
        let test = TestCase::new("my test case")
            .with_description("A test description")
            .with_tag("unit");

        assert_eq!(test.id, "my_test_case");
        assert_eq!(test.name, "my test case");
        assert_eq!(test.description, Some("A test description".to_string()));
        assert!(test.annotations.tags.contains(&"unit".to_string()));
    }

    #[test]
    fn test_suite_creation() {
        let mut suite = TestSuite::new("Math tests");
        suite.add_test(TestCase::new("addition"));
        suite.add_test(TestCase::new("subtraction"));

        let mut nested = TestSuite::new("Advanced");
        nested.add_test(TestCase::new("multiplication"));

        suite.add_suite(nested);

        assert_eq!(suite.test_count(), 3);
        assert_eq!(suite.all_tests().len(), 3);
    }

    #[test]
    fn test_filter_matching() {
        let test = TestCase::new("user creation test")
            .with_tag("database")
            .with_tag("slow");

        // Match by name pattern
        let filter = TestFilter::new().include("user*");
        assert!(filter.matches(&test));

        // Match by tag
        let filter = TestFilter::new().with_tag("database");
        assert!(filter.matches(&test));

        // Exclude by tag
        let filter = TestFilter::new().without_tag("slow");
        assert!(!filter.matches(&test));

        // Exclude by pattern
        let filter = TestFilter::new().exclude("*creation*");
        assert!(!filter.matches(&test));
    }

    #[test]
    fn test_result_statistics() {
        let test1 = TestCase::new("test1");
        let test2 = TestCase::new("test2");
        let test3 = TestCase::new("test3");

        let mut suite_result = SuiteResult::new("main");
        suite_result.add_result(TestResult::passed(&test1, Duration::from_millis(10)));
        suite_result.add_result(TestResult::failed(
            &test2,
            TestError::AssertionFailed {
                message: "failed".to_string(),
                expected: None,
                actual: None,
                location: None,
            },
            Duration::from_millis(20),
        ));
        suite_result.add_result(TestResult::skipped(&test3, "not implemented"));

        assert_eq!(suite_result.passed_count(), 1);
        assert_eq!(suite_result.failed_count(), 1);
        assert_eq!(suite_result.skipped_count(), 1);
        assert_eq!(suite_result.total_count(), 3);
        assert!(!suite_result.all_passed());
    }

    #[test]
    fn test_context_assertions() {
        let mut ctx = TestContext::new("test");

        assert!(ctx.assert(true, "should pass").is_ok());
        assert!(ctx.assert(false, "should fail").is_err());
        assert!(ctx.assert_eq(42, 42).is_ok());
        assert!(ctx.assert_eq(42, 43).is_err());
        assert!(ctx.assert_ne(42, 43).is_ok());
        assert!(ctx.assert_ne(42, 42).is_err());

        assert_eq!(ctx.assertion_count(), 6);
    }
}
