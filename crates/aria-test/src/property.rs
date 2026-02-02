//! Property-Based Testing
//!
//! Provides property-based testing as specified in ARIA-PD-011.
//! Properties are universally quantified statements that should hold
//! for all inputs of a given type.
//!
//! # Example
//!
//! ```rust
//! use aria_test::property::{Property, PropertyConfig, PropertyResult};
//!
//! // Create a property test
//! let result = Property::<i32>::new("zero identity")
//!     .iterations(100)
//!     .check(|x| x + 0 == x);
//!
//! assert!(result.is_passed());
//! ```

use crate::generator::{Arbitrary, Generator, Seed, Size};
use rustc_hash::FxHashMap;
use std::time::{Duration, Instant};

// ============================================================================
// Property Configuration
// ============================================================================

/// Configuration for property-based testing
#[derive(Debug, Clone)]
pub struct PropertyConfig {
    /// Number of test iterations
    pub iterations: usize,

    /// Initial random seed (None for random)
    pub seed: Option<u64>,

    /// Maximum shrinking attempts
    pub max_shrinks: usize,

    /// Maximum shrinking depth
    pub shrink_depth: usize,

    /// Initial size parameter
    pub initial_size: usize,

    /// Maximum size parameter
    pub max_size: usize,

    /// Timeout for the entire property test
    pub timeout: Option<Duration>,

    /// Coverage requirements: (label, minimum_percentage)
    pub coverage_requirements: Vec<(String, f64)>,

    /// Whether to show progress during testing
    pub verbose: bool,
}

impl Default for PropertyConfig {
    fn default() -> Self {
        Self {
            iterations: 100,
            seed: None,
            max_shrinks: 1000,
            shrink_depth: 50,
            initial_size: 1,
            max_size: 100,
            timeout: None,
            coverage_requirements: Vec::new(),
            verbose: false,
        }
    }
}

impl PropertyConfig {
    /// Create a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the number of iterations
    pub fn with_iterations(mut self, n: usize) -> Self {
        self.iterations = n;
        self
    }

    /// Set the random seed
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Set the maximum shrinks
    pub fn with_max_shrinks(mut self, n: usize) -> Self {
        self.max_shrinks = n;
        self
    }

    /// Set the timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Add a coverage requirement
    pub fn with_coverage(mut self, label: impl Into<String>, min_percentage: f64) -> Self {
        self.coverage_requirements.push((label.into(), min_percentage));
        self
    }

    /// Enable verbose output
    pub fn verbose(mut self) -> Self {
        self.verbose = true;
        self
    }
}

// ============================================================================
// Property Status
// ============================================================================

/// Status of a property test
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyStatus {
    /// Property passed all iterations
    Passed,

    /// Property was falsified
    Failed {
        /// Reason for failure
        reason: String,
    },

    /// Property test gave up (couldn't generate enough valid inputs)
    GaveUp {
        /// Reason for giving up
        reason: String,
    },

    /// Property test timed out
    TimedOut,

    /// Coverage requirements not met
    InsufficientCoverage {
        /// Missing coverage requirements: (label, required, actual)
        missing: Vec<(String, f64, f64)>,
    },
}

impl PropertyStatus {
    /// Check if the property passed
    pub fn is_passed(&self) -> bool {
        matches!(self, PropertyStatus::Passed)
    }

    /// Check if the property failed
    pub fn is_failed(&self) -> bool {
        matches!(self, PropertyStatus::Failed { .. })
    }
}

// ============================================================================
// Property Result
// ============================================================================

/// Result of running a property test
#[derive(Debug, Clone)]
pub struct PropertyResult {
    /// Property name
    pub name: String,

    /// Final status
    pub status: PropertyStatus,

    /// Number of iterations completed
    pub iterations_run: usize,

    /// Original counterexample (if failed)
    pub counterexample: Option<String>,

    /// Shrunk counterexample (if failed and shrinking succeeded)
    pub shrunk_counterexample: Option<String>,

    /// Seed used for generation
    pub seed: u64,

    /// Coverage statistics: label -> count
    pub coverage: FxHashMap<String, usize>,

    /// Total duration
    pub duration: Duration,
}

impl PropertyResult {
    /// Create a passing result
    pub fn passed(
        name: impl Into<String>,
        iterations: usize,
        seed: u64,
        coverage: FxHashMap<String, usize>,
        duration: Duration,
    ) -> Self {
        Self {
            name: name.into(),
            status: PropertyStatus::Passed,
            iterations_run: iterations,
            counterexample: None,
            shrunk_counterexample: None,
            seed,
            coverage,
            duration,
        }
    }

    /// Create a failed result
    pub fn failed(
        name: impl Into<String>,
        reason: impl Into<String>,
        iterations: usize,
        counterexample: impl Into<String>,
        shrunk_counterexample: Option<String>,
        seed: u64,
        coverage: FxHashMap<String, usize>,
        duration: Duration,
    ) -> Self {
        Self {
            name: name.into(),
            status: PropertyStatus::Failed {
                reason: reason.into(),
            },
            iterations_run: iterations,
            counterexample: Some(counterexample.into()),
            shrunk_counterexample,
            seed,
            coverage,
            duration,
        }
    }

    /// Check if the property passed
    pub fn is_passed(&self) -> bool {
        self.status.is_passed()
    }

    /// Get coverage percentage for a label
    pub fn coverage_percentage(&self, label: &str) -> f64 {
        let count = self.coverage.get(label).copied().unwrap_or(0);
        if self.iterations_run == 0 {
            0.0
        } else {
            (count as f64 / self.iterations_run as f64) * 100.0
        }
    }
}

// ============================================================================
// Property Context
// ============================================================================

/// Context for property execution, used for classification and reporting
#[derive(Debug, Default)]
pub struct PropertyContext {
    /// Classification counts
    classifications: FxHashMap<String, usize>,

    /// Labels applied to this test case
    labels: Vec<String>,

    /// Whether this test case should be discarded
    discard: bool,
}

impl PropertyContext {
    /// Create a new context
    pub fn new() -> Self {
        Self::default()
    }

    /// Classify the current test case
    pub fn classify(&mut self, condition: bool, label: impl Into<String>) {
        if condition {
            let label = label.into();
            *self.classifications.entry(label.clone()).or_insert(0) += 1;
            self.labels.push(label);
        }
    }

    /// Add a label to the current test case
    pub fn label(&mut self, label: impl Into<String>) {
        self.labels.push(label.into());
    }

    /// Discard the current test case (for filtering)
    pub fn discard(&mut self) {
        self.discard = true;
    }

    /// Check if this test case should be discarded
    pub fn is_discarded(&self) -> bool {
        self.discard
    }

    /// Get classification counts
    pub fn classifications(&self) -> &FxHashMap<String, usize> {
        &self.classifications
    }

    /// Merge another context into this one
    pub fn merge(&mut self, other: &PropertyContext) {
        for (label, count) in &other.classifications {
            *self.classifications.entry(label.clone()).or_insert(0) += count;
        }
    }
}

// ============================================================================
// Property Definition
// ============================================================================

/// A property-based test
pub struct Property<T> {
    /// Property name
    pub name: String,

    /// Generator for inputs
    generator: Generator<T>,

    /// Configuration
    pub config: PropertyConfig,
}

impl<T: std::fmt::Debug + Clone + PartialEq + Arbitrary + 'static> Property<T> {
    /// Create a new property with automatic generator
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            generator: T::arbitrary(),
            config: PropertyConfig::default(),
        }
    }

    /// Create a property with a custom generator
    pub fn with_generator(name: impl Into<String>, generator: Generator<T>) -> Self {
        Self {
            name: name.into(),
            generator,
            config: PropertyConfig::default(),
        }
    }

    /// Set the configuration
    pub fn configure(mut self, config: PropertyConfig) -> Self {
        self.config = config;
        self
    }

    /// Set the number of iterations
    pub fn iterations(mut self, n: usize) -> Self {
        self.config.iterations = n;
        self
    }

    /// Set the random seed
    pub fn seed(mut self, seed: u64) -> Self {
        self.config.seed = Some(seed);
        self
    }

    /// Run the property with a predicate
    pub fn check<P>(self, predicate: P) -> PropertyResult
    where
        P: Fn(T) -> bool,
    {
        let start = Instant::now();
        let seed = self.config.seed.unwrap_or_else(|| Seed::random().state());
        let mut current_seed = Seed::new(seed);
        let coverage = FxHashMap::default();
        // Discards are for filtered generators - placeholder for future use
        let _discarded = 0;
        let _max_discards = self.config.iterations * 10;

        for i in 0..self.config.iterations {
            // Check timeout
            if let Some(timeout) = self.config.timeout {
                if start.elapsed() > timeout {
                    return PropertyResult {
                        name: self.name,
                        status: PropertyStatus::TimedOut,
                        iterations_run: i,
                        counterexample: None,
                        shrunk_counterexample: None,
                        seed,
                        coverage,
                        duration: start.elapsed(),
                    };
                }
            }

            // Calculate size that grows with iterations
            let size = Size::new(
                self.config.initial_size
                    + (i * (self.config.max_size - self.config.initial_size)) / self.config.iterations.max(1),
            );

            let (s1, s2) = current_seed.split();
            current_seed = s2;

            let input = self.generator.generate(s1, size);

            if !predicate(input.clone()) {
                // Found counterexample - try to shrink
                let shrunk = self.shrink_counterexample(&input, &predicate);
                let shrunk_str = if shrunk != input {
                    Some(format!("{:?}", shrunk))
                } else {
                    None
                };

                return PropertyResult::failed(
                    self.name,
                    "Property falsified",
                    i + 1,
                    format!("{:?}", input),
                    shrunk_str,
                    seed,
                    coverage,
                    start.elapsed(),
                );
            }
        }

        // Check coverage requirements
        let missing_coverage: Vec<_> = self
            .config
            .coverage_requirements
            .iter()
            .filter_map(|(label, required)| {
                let count = coverage.get(label).copied().unwrap_or(0);
                let actual = (count as f64 / self.config.iterations as f64) * 100.0;
                if actual < *required {
                    Some((label.clone(), *required, actual))
                } else {
                    None
                }
            })
            .collect();

        if !missing_coverage.is_empty() {
            return PropertyResult {
                name: self.name,
                status: PropertyStatus::InsufficientCoverage {
                    missing: missing_coverage,
                },
                iterations_run: self.config.iterations,
                counterexample: None,
                shrunk_counterexample: None,
                seed,
                coverage,
                duration: start.elapsed(),
            };
        }

        PropertyResult::passed(
            self.name,
            self.config.iterations,
            seed,
            coverage,
            start.elapsed(),
        )
    }

    /// Shrink a counterexample to a minimal case
    fn shrink_counterexample<P>(&self, value: &T, predicate: &P) -> T
    where
        P: Fn(T) -> bool,
    {
        let mut current = value.clone();
        let mut shrink_count = 0;

        while shrink_count < self.config.max_shrinks {
            let candidates = T::shrink(current.clone());
            if candidates.is_empty() {
                break;
            }

            let mut improved = false;
            for candidate in candidates {
                if !predicate(candidate.clone()) {
                    current = candidate;
                    improved = true;
                    break;
                }
            }

            if !improved {
                break;
            }
            shrink_count += 1;
        }

        current
    }
}

// ============================================================================
// Property Runner
// ============================================================================

/// Runner for multiple properties
pub struct PropertyRunner {
    /// Configuration applied to all properties
    pub default_config: PropertyConfig,

    /// Results from property runs
    pub results: Vec<PropertyResult>,
}

impl PropertyRunner {
    /// Create a new runner with default configuration
    pub fn new() -> Self {
        Self {
            default_config: PropertyConfig::default(),
            results: Vec::new(),
        }
    }

    /// Create a runner with custom configuration
    pub fn with_config(config: PropertyConfig) -> Self {
        Self {
            default_config: config,
            results: Vec::new(),
        }
    }

    /// Run a property and store the result
    pub fn run<T, P>(&mut self, property: Property<T>, predicate: P) -> &PropertyResult
    where
        T: std::fmt::Debug + Clone + PartialEq + Arbitrary + 'static,
        P: Fn(T) -> bool,
    {
        let result = property.check(predicate);
        self.results.push(result);
        self.results.last().unwrap()
    }

    /// Check if all properties passed
    pub fn all_passed(&self) -> bool {
        self.results.iter().all(|r| r.is_passed())
    }

    /// Get failed properties
    pub fn failed(&self) -> Vec<&PropertyResult> {
        self.results.iter().filter(|r| !r.is_passed()).collect()
    }

    /// Get summary statistics
    pub fn summary(&self) -> PropertySummary {
        PropertySummary {
            total: self.results.len(),
            passed: self.results.iter().filter(|r| r.is_passed()).count(),
            failed: self.results.iter().filter(|r| r.status.is_failed()).count(),
            total_duration: self.results.iter().map(|r| r.duration).sum(),
        }
    }
}

impl Default for PropertyRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary of property test results
#[derive(Debug, Clone)]
pub struct PropertySummary {
    /// Total properties tested
    pub total: usize,

    /// Properties that passed
    pub passed: usize,

    /// Properties that failed
    pub failed: usize,

    /// Total duration
    pub total_duration: Duration,
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Check a property with default configuration
pub fn check_property<T, P>(name: impl Into<String>, predicate: P) -> PropertyResult
where
    T: std::fmt::Debug + Clone + PartialEq + Arbitrary + 'static,
    P: Fn(T) -> bool,
{
    Property::<T>::new(name).check(predicate)
}

/// Check a property with custom iterations
pub fn check_property_n<T, P>(name: impl Into<String>, iterations: usize, predicate: P) -> PropertyResult
where
    T: std::fmt::Debug + Clone + PartialEq + Arbitrary + 'static,
    P: Fn(T) -> bool,
{
    Property::<T>::new(name).iterations(iterations).check(predicate)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_property_passes() {
        let result = Property::<i32>::new("zero identity")
            .iterations(100)
            .check(|x| x + 0 == x);

        assert!(result.is_passed());
        assert_eq!(result.iterations_run, 100);
    }

    #[test]
    fn test_property_fails() {
        let result = Property::<i32>::new("all positive")
            .iterations(100)
            .check(|x| x > 0);

        assert!(!result.is_passed());
        assert!(result.counterexample.is_some());
    }

    #[test]
    fn test_property_shrinking() {
        let result = Property::<i32>::new("less than ten")
            .seed(12345)
            .iterations(100)
            .check(|x| x < 10);

        assert!(!result.is_passed());

        // Should shrink to exactly 10 (minimal counterexample)
        if let Some(shrunk) = &result.shrunk_counterexample {
            assert!(shrunk.contains("10"), "Expected shrunk to 10, got {}", shrunk);
        }
    }

    #[test]
    fn test_property_with_tuples() {
        let result = Property::<(i32, i32)>::new("addition commutes")
            .iterations(100)
            .check(|(a, b)| a + b == b + a);

        assert!(result.is_passed());
    }

    #[test]
    fn test_property_with_vec() {
        let result = Property::<Vec<i32>>::new("reverse twice is identity")
            .iterations(50)
            .check(|v| {
                let reversed: Vec<_> = v.iter().rev().rev().cloned().collect();
                reversed == v
            });

        assert!(result.is_passed());
    }

    #[test]
    fn test_property_config() {
        let config = PropertyConfig::new()
            .with_iterations(50)
            .with_seed(42)
            .with_max_shrinks(500);

        assert_eq!(config.iterations, 50);
        assert_eq!(config.seed, Some(42));
        assert_eq!(config.max_shrinks, 500);
    }

    #[test]
    fn test_property_runner() {
        let mut runner = PropertyRunner::new();

        runner.run(Property::<i32>::new("identity").iterations(10), |x| x == x);
        runner.run(Property::<i32>::new("zero").iterations(10), |x| x + 0 == x);

        let summary = runner.summary();
        assert_eq!(summary.total, 2);
        assert_eq!(summary.passed, 2);
        assert!(runner.all_passed());
    }

    #[test]
    fn test_check_property_convenience() {
        let result = check_property::<i32, _>("abs non-negative", |x| x.abs() >= 0);
        assert!(result.is_passed());
    }
}
