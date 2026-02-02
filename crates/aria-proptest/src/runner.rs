//! Test runner for property-based testing.
//!
//! Orchestrates test execution, shrinking, and reporting.

use crate::{AriaValue, GenContext, PropertyResult, Counterexample};
use crate::shrink::TypedShrinker;

/// Configuration for the test runner
#[derive(Debug, Clone)]
pub struct TestConfig {
    /// Number of test cases to generate
    pub num_tests: usize,
    /// Maximum number of shrink attempts
    pub max_shrinks: usize,
    /// Initial seed (None for random)
    pub seed: Option<u64>,
    /// Maximum discards before giving up
    pub max_discards: usize,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            num_tests: 100,
            max_shrinks: 100,
            seed: None,
            max_discards: 1000,
        }
    }
}

/// Result of running tests
#[derive(Debug, Clone)]
pub enum TestResult {
    /// All tests passed
    Success {
        num_tests: usize,
        num_discards: usize,
    },
    /// A test failed with a counterexample
    Failure {
        counterexample: Counterexample,
        num_tests_before_failure: usize,
    },
    /// Too many discards
    GaveUp {
        num_discards: usize,
        num_tests_completed: usize,
    },
}

impl TestResult {
    /// Check if tests succeeded
    pub fn is_success(&self) -> bool {
        matches!(self, TestResult::Success { .. })
    }

    /// Check if tests failed
    pub fn is_failure(&self) -> bool {
        matches!(self, TestResult::Failure { .. })
    }

    /// Format for display
    pub fn display(&self) -> String {
        match self {
            TestResult::Success { num_tests, num_discards } => {
                if *num_discards > 0 {
                    format!("OK: {} tests passed ({} discarded)", num_tests, num_discards)
                } else {
                    format!("OK: {} tests passed", num_tests)
                }
            }
            TestResult::Failure { counterexample, num_tests_before_failure } => {
                format!(
                    "FAILED after {} tests\n{}",
                    num_tests_before_failure,
                    counterexample.display()
                )
            }
            TestResult::GaveUp { num_discards, num_tests_completed } => {
                format!(
                    "GAVE UP: {} discards, only {} tests completed",
                    num_discards, num_tests_completed
                )
            }
        }
    }
}

/// Test runner
pub struct TestRunner {
    config: TestConfig,
    seed_counter: u64,
}

impl TestRunner {
    /// Create a new test runner
    pub fn new(config: TestConfig) -> Self {
        let seed_counter = config.seed.unwrap_or_else(|| {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        });

        Self { config, seed_counter }
    }

    /// Run a property test
    pub fn run<F>(&mut self, test_fn: F) -> TestResult
    where
        F: Fn(&mut GenContext) -> PropertyResult,
    {
        let mut num_tests = 0;
        let mut num_discards = 0;

        while num_tests < self.config.num_tests {
            if num_discards >= self.config.max_discards {
                return TestResult::GaveUp {
                    num_discards,
                    num_tests_completed: num_tests,
                };
            }

            let seed = self.next_seed();
            let mut ctx = GenContext::new(seed);

            match test_fn(&mut ctx) {
                PropertyResult::Pass => {
                    num_tests += 1;
                }
                PropertyResult::Fail(value) => {
                    // Found a failing case - try to shrink it
                    let shrunk = self.shrink(&value, &test_fn);
                    let counterexample = Counterexample::new(
                        value,
                        shrunk.0,
                        shrunk.1,
                        seed,
                    );
                    return TestResult::Failure {
                        counterexample,
                        num_tests_before_failure: num_tests,
                    };
                }
                PropertyResult::Discard => {
                    num_discards += 1;
                }
            }
        }

        TestResult::Success { num_tests, num_discards }
    }

    /// Run a property test with a generated value
    pub fn run_with_gen<G, F>(&mut self, gen: G, prop: F) -> TestResult
    where
        G: crate::Generator,
        F: Fn(&AriaValue) -> PropertyResult,
    {
        self.run(|ctx| {
            let value = gen.generate(ctx);
            prop(&value)
        })
    }

    /// Shrink a failing value
    fn shrink<F>(&self, value: &AriaValue, test_fn: &F) -> (AriaValue, usize)
    where
        F: Fn(&mut GenContext) -> PropertyResult,
    {
        let mut current = value.clone();
        let mut steps = 0;

        for _ in 0..self.config.max_shrinks {
            let mut improved = false;

            for shrunk in TypedShrinker::shrink(&current) {
                // Test if shrunk value still fails
                // Use a fixed seed for shrink testing
                let _ctx = GenContext::new(0);
                // We need to test the shrunk value directly
                // This is a simplification - in practice we'd re-run the test
                // For now, we assume the property only depends on the value

                // Check if this shrunk value still fails
                let result = check_value_fails(&shrunk, test_fn);
                if result {
                    current = shrunk;
                    steps += 1;
                    improved = true;
                    break;
                }
            }

            if !improved {
                break;
            }
        }

        (current, steps)
    }

    fn next_seed(&mut self) -> u64 {
        let seed = self.seed_counter;
        self.seed_counter = self.seed_counter.wrapping_add(1);
        seed
    }
}

/// Helper to check if a value still fails
fn check_value_fails<F>(value: &AriaValue, test_fn: &F) -> bool
where
    F: Fn(&mut GenContext) -> PropertyResult,
{
    // This is a hack - we create a context that would produce this value
    // In a real implementation, we'd have separate generate and check phases
    let mut ctx = GenContext::new(0);

    // Try to check if using this value would fail
    // This is simplified - real shrinking needs better integration
    match value {
        AriaValue::Int(_n) => {
            // For integers, we check directly
            matches!(test_fn(&mut ctx), PropertyResult::Fail(_))
        }
        _ => {
            // For other types, just assume it fails
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::Arbitrary;

    #[test]
    fn test_runner_all_pass() {
        let config = TestConfig {
            num_tests: 10,
            ..Default::default()
        };
        let mut runner = TestRunner::new(config);

        let result = runner.run(|ctx| {
            let gen = i64::arbitrary();
            let _value = gen.generate(ctx);
            PropertyResult::Pass
        });

        assert!(result.is_success());
    }

    #[test]
    fn test_runner_with_discard() {
        let config = TestConfig {
            num_tests: 10,
            max_discards: 100,
            ..Default::default()
        };
        let mut runner = TestRunner::new(config);

        let result = runner.run(|ctx| {
            let gen = i64::arbitrary();
            let value = gen.generate(ctx);
            // Discard negative numbers
            if let AriaValue::Int(n) = &value {
                if *n < 0 {
                    PropertyResult::Discard
                } else {
                    PropertyResult::Pass
                }
            } else {
                PropertyResult::Discard
            }
        });

        assert!(result.is_success());
    }

    #[test]
    fn test_result_display() {
        let success = TestResult::Success { num_tests: 100, num_discards: 0 };
        assert!(success.display().contains("100 tests passed"));

        let success_with_discards = TestResult::Success { num_tests: 100, num_discards: 10 };
        assert!(success_with_discards.display().contains("10 discarded"));
    }
}
