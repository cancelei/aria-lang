//! Test Runner Infrastructure
//!
//! Provides the test runner trait and parallel execution infrastructure
//! as specified in ARIA-PD-011. This module includes:
//!
//! - `TestRunner` trait for running tests
//! - Parallel execution using std threads
//! - Test filtering and selection
//! - Progress reporting

use crate::{
    SuiteResult, TestCase, TestError, TestFilter, TestResult, TestStatus, TestSuite,
};
use rustc_hash::FxHashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

// ============================================================================
// Runner Configuration
// ============================================================================

/// Configuration for test execution
#[derive(Debug, Clone)]
pub struct RunnerConfig {
    /// Number of parallel threads (0 = auto-detect)
    pub parallelism: usize,

    /// Default timeout for each test
    pub default_timeout: Duration,

    /// Whether to fail fast (stop on first failure)
    pub fail_fast: bool,

    /// Whether to capture stdout/stderr
    pub capture_output: bool,

    /// Whether to show progress
    pub show_progress: bool,

    /// Whether to run tests in random order
    pub randomize: bool,

    /// Random seed for ordering (if randomize is true)
    pub seed: Option<u64>,

    /// Test filter
    pub filter: TestFilter,

    /// Retry count for flaky tests
    pub retries: u32,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            parallelism: 0, // Auto-detect
            default_timeout: Duration::from_secs(60),
            fail_fast: false,
            capture_output: true,
            show_progress: true,
            randomize: false,
            seed: None,
            filter: TestFilter::default(),
            retries: 0,
        }
    }
}

impl RunnerConfig {
    /// Create a new configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set parallelism level
    pub fn with_parallelism(mut self, n: usize) -> Self {
        self.parallelism = n;
        self
    }

    /// Set default timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout;
        self
    }

    /// Enable fail-fast mode
    pub fn fail_fast(mut self) -> Self {
        self.fail_fast = true;
        self
    }

    /// Disable output capture
    pub fn no_capture(mut self) -> Self {
        self.capture_output = false;
        self
    }

    /// Set the test filter
    pub fn with_filter(mut self, filter: TestFilter) -> Self {
        self.filter = filter;
        self
    }

    /// Enable random test order
    pub fn randomize(mut self, seed: Option<u64>) -> Self {
        self.randomize = true;
        self.seed = seed;
        self
    }

    /// Set retry count
    pub fn with_retries(mut self, n: u32) -> Self {
        self.retries = n;
        self
    }

    /// Get effective parallelism
    pub fn effective_parallelism(&self) -> usize {
        if self.parallelism == 0 {
            thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1)
        } else {
            self.parallelism
        }
    }
}

// ============================================================================
// Test Function Trait
// ============================================================================

/// Trait for test functions that can be executed
pub trait TestFn: Send + Sync {
    /// Execute the test
    fn run(&self) -> Result<(), TestError>;

    /// Get the test name
    fn name(&self) -> &str;
}

/// A boxed test function
pub type BoxedTestFn = Box<dyn TestFn>;

// ============================================================================
// Test Runner Trait
// ============================================================================

/// Trait for test runners
pub trait TestRunner {
    /// Run a single test case
    fn run_test(&self, test: &TestCase) -> TestResult;

    /// Run a test suite
    fn run_suite(&self, suite: &TestSuite) -> SuiteResult;

    /// Run all tests with filtering
    fn run_filtered(&self, suite: &TestSuite, filter: &TestFilter) -> SuiteResult;
}

// ============================================================================
// Sequential Runner
// ============================================================================

/// A simple sequential test runner
pub struct SequentialRunner {
    config: RunnerConfig,
}

impl SequentialRunner {
    /// Create a new sequential runner
    pub fn new(config: RunnerConfig) -> Self {
        Self { config }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(RunnerConfig::default())
    }

    /// Execute a test with timeout
    fn execute_with_timeout(&self, test: &TestCase) -> TestResult {
        let timeout = test
            .annotations
            .timeout
            .unwrap_or(self.config.default_timeout);
        let start = Instant::now();

        // Check if test should be skipped
        if let Some(reason) = test.should_skip() {
            return TestResult::skipped(test, reason);
        }

        // For now, we just simulate a passing test since we don't have
        // actual test functions yet. In a real implementation, this would
        // execute the test closure.
        let duration = start.elapsed();

        if duration > timeout {
            return TestResult::failed(test, TestError::Timeout(timeout), duration);
        }

        TestResult::passed(test, duration)
    }
}

impl TestRunner for SequentialRunner {
    fn run_test(&self, test: &TestCase) -> TestResult {
        self.execute_with_timeout(test)
    }

    fn run_suite(&self, suite: &TestSuite) -> SuiteResult {
        self.run_filtered(suite, &TestFilter::new())
    }

    fn run_filtered(&self, suite: &TestSuite, filter: &TestFilter) -> SuiteResult {
        let start = Instant::now();
        let mut result = SuiteResult::new(&suite.name);

        // Run tests in this suite
        for test in &suite.tests {
            if !filter.matches(test) {
                continue;
            }

            let test_result = self.run_test(test);

            // Check fail-fast
            if self.config.fail_fast && test_result.is_failed() {
                result.add_result(test_result);
                result.duration = start.elapsed();
                return result;
            }

            result.add_result(test_result);
        }

        // Run nested suites
        for nested in &suite.suites {
            let nested_result = self.run_filtered(nested, filter);

            // Check fail-fast
            if self.config.fail_fast && !nested_result.all_passed() {
                result.add_suite_result(nested_result);
                result.duration = start.elapsed();
                return result;
            }

            result.add_suite_result(nested_result);
        }

        result.duration = start.elapsed();
        result
    }
}

// ============================================================================
// Parallel Runner
// ============================================================================

/// Progress tracking for parallel execution
#[derive(Debug, Default)]
struct Progress {
    completed: AtomicUsize,
    passed: AtomicUsize,
    failed: AtomicUsize,
    skipped: AtomicUsize,
    total: AtomicUsize,
}

impl Progress {
    fn record(&self, status: &TestStatus) {
        self.completed.fetch_add(1, Ordering::SeqCst);
        match status {
            TestStatus::Passed => {
                self.passed.fetch_add(1, Ordering::SeqCst);
            }
            TestStatus::Failed(_) => {
                self.failed.fetch_add(1, Ordering::SeqCst);
            }
            TestStatus::Skipped(_) => {
                self.skipped.fetch_add(1, Ordering::SeqCst);
            }
            _ => {}
        }
    }
}

/// A parallel test runner using std threads
pub struct ParallelRunner {
    config: RunnerConfig,
}

impl ParallelRunner {
    /// Create a new parallel runner
    pub fn new(config: RunnerConfig) -> Self {
        Self { config }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(RunnerConfig::default())
    }

    /// Collect all tests from a suite (flattened)
    fn collect_tests<'a>(&self, suite: &'a TestSuite, filter: &TestFilter) -> Vec<&'a TestCase> {
        let mut tests = Vec::new();

        for test in &suite.tests {
            if filter.matches(test) {
                tests.push(test);
            }
        }

        for nested in &suite.suites {
            tests.extend(self.collect_tests(nested, filter));
        }

        tests
    }

    /// Execute a single test
    fn execute_test(&self, test: &TestCase) -> TestResult {
        let timeout = test
            .annotations
            .timeout
            .unwrap_or(self.config.default_timeout);
        let start = Instant::now();

        // Check if test should be skipped
        if let Some(reason) = test.should_skip() {
            return TestResult::skipped(test, reason);
        }

        // Simulate test execution
        // In a real implementation, this would run the actual test function
        let duration = start.elapsed();

        if duration > timeout {
            return TestResult::failed(test, TestError::Timeout(timeout), duration);
        }

        TestResult::passed(test, duration)
    }

    /// Run tests in parallel
    fn run_parallel(&self, tests: Vec<&TestCase>) -> Vec<TestResult> {
        let num_threads = self.config.effective_parallelism();
        let tests: Vec<_> = tests.into_iter().cloned().collect();
        let total = tests.len();

        if total == 0 {
            return Vec::new();
        }

        let progress = Arc::new(Progress::default());
        progress.total.store(total, Ordering::SeqCst);

        let results = Arc::new(Mutex::new(Vec::with_capacity(total)));
        let fail_fast = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let test_queue = Arc::new(Mutex::new(tests.into_iter().enumerate().collect::<Vec<_>>()));

        let mut handles = Vec::with_capacity(num_threads);

        for _ in 0..num_threads {
            let queue = Arc::clone(&test_queue);
            let results = Arc::clone(&results);
            let progress = Arc::clone(&progress);
            let fail_fast = Arc::clone(&fail_fast);
            let config = self.config.clone();

            let handle = thread::spawn(move || {
                loop {
                    // Check fail-fast
                    if config.fail_fast && fail_fast.load(Ordering::SeqCst) {
                        break;
                    }

                    // Get next test
                    let test_item = {
                        let mut queue = queue.lock().unwrap();
                        queue.pop()
                    };

                    let (idx, test) = match test_item {
                        Some(item) => item,
                        None => break,
                    };

                    // Execute test
                    let timeout = test
                        .annotations
                        .timeout
                        .unwrap_or(config.default_timeout);
                    let start = Instant::now();

                    let result = if let Some(reason) = test.should_skip() {
                        TestResult::skipped(&test, reason)
                    } else {
                        let duration = start.elapsed();
                        if duration > timeout {
                            TestResult::failed(&test, TestError::Timeout(timeout), duration)
                        } else {
                            TestResult::passed(&test, duration)
                        }
                    };

                    // Record progress
                    progress.record(&result.status);

                    // Check for failure
                    if result.is_failed() && config.fail_fast {
                        fail_fast.store(true, Ordering::SeqCst);
                    }

                    // Store result
                    let mut results = results.lock().unwrap();
                    results.push((idx, result));
                }
            });

            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            let _ = handle.join();
        }

        // Sort results by original index and extract
        let mut results = Arc::try_unwrap(results).unwrap().into_inner().unwrap();
        results.sort_by_key(|(idx, _)| *idx);
        results.into_iter().map(|(_, r)| r).collect()
    }
}

impl TestRunner for ParallelRunner {
    fn run_test(&self, test: &TestCase) -> TestResult {
        self.execute_test(test)
    }

    fn run_suite(&self, suite: &TestSuite) -> SuiteResult {
        self.run_filtered(suite, &TestFilter::new())
    }

    fn run_filtered(&self, suite: &TestSuite, filter: &TestFilter) -> SuiteResult {
        let start = Instant::now();

        // Collect all matching tests
        let tests = self.collect_tests(suite, filter);

        // Run in parallel
        let results = self.run_parallel(tests);

        // Build result structure
        let mut suite_result = SuiteResult::new(&suite.name);
        for result in results {
            suite_result.add_result(result);
        }
        suite_result.duration = start.elapsed();

        suite_result
    }
}

// ============================================================================
// Test Discovery
// ============================================================================

/// Discovers tests from a module or file
pub struct TestDiscovery {
    /// Discovered test suites
    pub suites: Vec<TestSuite>,

    /// Discovered standalone tests
    pub tests: Vec<TestCase>,
}

impl TestDiscovery {
    /// Create a new empty discovery
    pub fn new() -> Self {
        Self {
            suites: Vec::new(),
            tests: Vec::new(),
        }
    }

    /// Add a test suite
    pub fn add_suite(&mut self, suite: TestSuite) {
        self.suites.push(suite);
    }

    /// Add a standalone test
    pub fn add_test(&mut self, test: TestCase) {
        self.tests.push(test);
    }

    /// Convert to a single root suite
    pub fn into_suite(self, name: impl Into<String>) -> TestSuite {
        let mut root = TestSuite::new(name);
        root.tests = self.tests;
        root.suites = self.suites;
        root
    }

    /// Total test count
    pub fn test_count(&self) -> usize {
        self.tests.len() + self.suites.iter().map(|s| s.test_count()).sum::<usize>()
    }
}

impl Default for TestDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Reporter Trait
// ============================================================================

/// Trait for test result reporters
pub trait Reporter {
    /// Called when a test starts
    fn test_started(&mut self, test: &TestCase);

    /// Called when a test completes
    fn test_completed(&mut self, result: &TestResult);

    /// Called when a suite starts
    fn suite_started(&mut self, suite: &TestSuite);

    /// Called when a suite completes
    fn suite_completed(&mut self, result: &SuiteResult);

    /// Called at the end with final summary
    fn summary(&mut self, result: &SuiteResult);
}

/// A simple console reporter
#[derive(Default)]
pub struct ConsoleReporter {
    indent: usize,
    verbose: bool,
}

impl ConsoleReporter {
    /// Create a new console reporter
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable verbose output
    pub fn verbose(mut self) -> Self {
        self.verbose = true;
        self
    }

    fn indent_str(&self) -> String {
        "  ".repeat(self.indent)
    }
}

impl Reporter for ConsoleReporter {
    fn test_started(&mut self, test: &TestCase) {
        if self.verbose {
            println!("{}Running: {}", self.indent_str(), test.name);
        }
    }

    fn test_completed(&mut self, result: &TestResult) {
        let status = match &result.status {
            TestStatus::Passed => "PASS",
            TestStatus::Failed(_) => "FAIL",
            TestStatus::Skipped(_) => "SKIP",
            TestStatus::Pending => "PEND",
            TestStatus::Running => "RUN ",
        };

        if self.verbose || !result.is_passed() {
            println!(
                "{}{} {} ({:?})",
                self.indent_str(),
                status,
                result.test_name,
                result.duration
            );
        }

        if let TestStatus::Failed(err) = &result.status {
            println!("{}  Error: {}", self.indent_str(), err);
        }
    }

    fn suite_started(&mut self, suite: &TestSuite) {
        println!("{}{}", self.indent_str(), suite.name);
        self.indent += 1;
    }

    fn suite_completed(&mut self, _result: &SuiteResult) {
        self.indent = self.indent.saturating_sub(1);
    }

    fn summary(&mut self, result: &SuiteResult) {
        println!();
        println!("Test Results:");
        println!("  Total:   {}", result.total_count());
        println!("  Passed:  {}", result.passed_count());
        println!("  Failed:  {}", result.failed_count());
        println!("  Skipped: {}", result.skipped_count());
        println!("  Duration: {:?}", result.duration);

        if !result.all_passed() {
            println!();
            println!("Failed tests:");
            for failed in result.failed_results() {
                println!("  - {}", failed.test_name);
                if let TestStatus::Failed(err) = &failed.status {
                    println!("    {}", err);
                }
            }
        }
    }
}

// ============================================================================
// Test Execution Context
// ============================================================================

/// Context for the entire test run
pub struct ExecutionContext {
    /// Runner configuration
    pub config: RunnerConfig,

    /// Reporter for output
    pub reporter: Option<Box<dyn Reporter>>,

    /// Start time
    pub start_time: Option<Instant>,

    /// Custom metadata
    pub metadata: FxHashMap<String, String>,
}

impl ExecutionContext {
    /// Create a new execution context
    pub fn new(config: RunnerConfig) -> Self {
        Self {
            config,
            reporter: None,
            start_time: None,
            metadata: FxHashMap::default(),
        }
    }

    /// Set the reporter
    pub fn with_reporter(mut self, reporter: impl Reporter + 'static) -> Self {
        self.reporter = Some(Box::new(reporter));
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Execute tests using the appropriate runner
    pub fn execute(&mut self, suite: &TestSuite) -> SuiteResult {
        self.start_time = Some(Instant::now());

        let runner: Box<dyn TestRunner> = if self.config.parallelism == 1 {
            Box::new(SequentialRunner::new(self.config.clone()))
        } else {
            Box::new(ParallelRunner::new(self.config.clone()))
        };

        let result = runner.run_filtered(suite, &self.config.filter);

        if let Some(reporter) = &mut self.reporter {
            reporter.summary(&result);
        }

        result
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sequential_runner() {
        let runner = SequentialRunner::with_defaults();
        let mut suite = TestSuite::new("test suite");
        suite.add_test(TestCase::new("test 1"));
        suite.add_test(TestCase::new("test 2"));

        let result = runner.run_suite(&suite);

        assert_eq!(result.total_count(), 2);
        assert_eq!(result.passed_count(), 2);
        assert!(result.all_passed());
    }

    #[test]
    fn test_parallel_runner() {
        let config = RunnerConfig::new().with_parallelism(2);
        let runner = ParallelRunner::new(config);

        let mut suite = TestSuite::new("parallel suite");
        for i in 0..10 {
            suite.add_test(TestCase::new(format!("test {}", i)));
        }

        let result = runner.run_suite(&suite);

        assert_eq!(result.total_count(), 10);
        assert!(result.all_passed());
    }

    #[test]
    fn test_filtering() {
        let runner = SequentialRunner::with_defaults();
        let mut suite = TestSuite::new("filtered suite");
        suite.add_test(TestCase::new("user test").with_tag("user"));
        suite.add_test(TestCase::new("admin test").with_tag("admin"));
        suite.add_test(TestCase::new("guest test").with_tag("guest"));

        let filter = TestFilter::new().with_tag("user");
        let result = runner.run_filtered(&suite, &filter);

        assert_eq!(result.total_count(), 1);
        assert_eq!(result.passed_count(), 1);
    }

    #[test]
    fn test_skipped_tests() {
        let runner = SequentialRunner::with_defaults();
        let mut suite = TestSuite::new("skip suite");

        let skipped = TestCase::new("skipped test")
            .with_annotations(crate::TestAnnotations::new().skip("not implemented"));
        suite.add_test(skipped);
        suite.add_test(TestCase::new("normal test"));

        let result = runner.run_suite(&suite);

        assert_eq!(result.skipped_count(), 1);
        assert_eq!(result.passed_count(), 1);
    }

    #[test]
    fn test_runner_config() {
        let config = RunnerConfig::new()
            .with_parallelism(4)
            .with_timeout(Duration::from_secs(30))
            .fail_fast()
            .no_capture();

        assert_eq!(config.parallelism, 4);
        assert_eq!(config.default_timeout, Duration::from_secs(30));
        assert!(config.fail_fast);
        assert!(!config.capture_output);
    }

    #[test]
    fn test_discovery() {
        let mut discovery = TestDiscovery::new();
        discovery.add_test(TestCase::new("standalone"));

        let mut suite = TestSuite::new("discovered suite");
        suite.add_test(TestCase::new("suite test"));
        discovery.add_suite(suite);

        assert_eq!(discovery.test_count(), 2);

        let root = discovery.into_suite("root");
        assert_eq!(root.test_count(), 2);
    }

    #[test]
    fn test_execution_context() {
        let config = RunnerConfig::new().with_parallelism(1);
        let mut ctx = ExecutionContext::new(config).with_metadata("version", "1.0");

        let mut suite = TestSuite::new("context test");
        suite.add_test(TestCase::new("test"));

        let result = ctx.execute(&suite);
        assert!(result.all_passed());
    }
}
