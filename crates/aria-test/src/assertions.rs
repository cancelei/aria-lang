//! Assertion Types and Utilities
//!
//! Provides a comprehensive assertion DSL as specified in ARIA-PD-011.
//! Supports fluent assertions, comparison assertions, collection assertions,
//! and exception assertions.

use crate::{SourceLocation, TestError};

// ============================================================================
// Assertion Types
// ============================================================================

/// Types of assertions available in the testing framework
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssertionKind {
    /// Basic truth assertion: `assert condition`
    Assert,

    /// Equality assertion: `assert_eq actual, expected`
    AssertEq,

    /// Inequality assertion: `assert_ne actual, expected`
    AssertNe,

    /// Less than assertion: `assert_lt actual, expected`
    AssertLt,

    /// Less than or equal assertion: `assert_le actual, expected`
    AssertLe,

    /// Greater than assertion: `assert_gt actual, expected`
    AssertGt,

    /// Greater than or equal assertion: `assert_ge actual, expected`
    AssertGe,

    /// Approximate equality for floats: `assert_approx actual, expected, epsilon`
    AssertApprox,

    /// Collection contains element: `assert_contains collection, element`
    AssertContains,

    /// Collection is empty: `assert_empty collection`
    AssertEmpty,

    /// Collection is not empty: `assert_not_empty collection`
    AssertNotEmpty,

    /// Value matches pattern: `assert_matches value, pattern`
    AssertMatches,

    /// Block raises exception: `assert_raises ExceptionType { block }`
    AssertRaises,

    /// Block does not raise: `assert_no_raise { block }`
    AssertNoRaise,

    /// Type assertion: `assert_type[T] value`
    AssertType,

    /// Option is Some: `assert_some option`
    AssertSome,

    /// Option is None: `assert_none option`
    AssertNone,

    /// Result is Ok: `assert_ok result`
    AssertOk,

    /// Result is Err: `assert_err result`
    AssertErr,

    /// Eventually true (for async): `assert_eventually { condition }`
    AssertEventually,
}

/// An assertion with its metadata
#[derive(Debug, Clone)]
pub struct Assertion {
    /// Kind of assertion
    pub kind: AssertionKind,

    /// Optional custom message
    pub message: Option<String>,

    /// Source location of the assertion
    pub location: Option<SourceLocation>,

    /// Expected value (for comparison assertions)
    pub expected: Option<String>,

    /// Actual value (for comparison assertions)
    pub actual: Option<String>,
}

impl Assertion {
    /// Create a new assertion
    pub fn new(kind: AssertionKind) -> Self {
        Self {
            kind,
            message: None,
            location: None,
            expected: None,
            actual: None,
        }
    }

    /// Set custom message
    pub fn with_message(mut self, msg: impl Into<String>) -> Self {
        self.message = Some(msg.into());
        self
    }

    /// Set source location
    pub fn with_location(mut self, loc: SourceLocation) -> Self {
        self.location = Some(loc);
        self
    }

    /// Set expected value
    pub fn with_expected(mut self, expected: impl Into<String>) -> Self {
        self.expected = Some(expected.into());
        self
    }

    /// Set actual value
    pub fn with_actual(mut self, actual: impl Into<String>) -> Self {
        self.actual = Some(actual.into());
        self
    }

    /// Convert to a TestError for a failed assertion
    pub fn to_error(&self) -> TestError {
        let default_message = match self.kind {
            AssertionKind::Assert => "Assertion failed",
            AssertionKind::AssertEq => "Values are not equal",
            AssertionKind::AssertNe => "Values should not be equal",
            AssertionKind::AssertLt => "Value is not less than expected",
            AssertionKind::AssertLe => "Value is not less than or equal to expected",
            AssertionKind::AssertGt => "Value is not greater than expected",
            AssertionKind::AssertGe => "Value is not greater than or equal to expected",
            AssertionKind::AssertApprox => "Values are not approximately equal",
            AssertionKind::AssertContains => "Collection does not contain element",
            AssertionKind::AssertEmpty => "Collection is not empty",
            AssertionKind::AssertNotEmpty => "Collection is empty",
            AssertionKind::AssertMatches => "Value does not match pattern",
            AssertionKind::AssertRaises => "Expected exception was not raised",
            AssertionKind::AssertNoRaise => "Unexpected exception was raised",
            AssertionKind::AssertType => "Value is not of expected type",
            AssertionKind::AssertSome => "Expected Some, got None",
            AssertionKind::AssertNone => "Expected None, got Some",
            AssertionKind::AssertOk => "Expected Ok, got Err",
            AssertionKind::AssertErr => "Expected Err, got Ok",
            AssertionKind::AssertEventually => "Condition was not satisfied in time",
        };

        TestError::AssertionFailed {
            message: self.message.clone().unwrap_or_else(|| default_message.to_string()),
            expected: self.expected.clone(),
            actual: self.actual.clone(),
            location: self.location.clone(),
        }
    }
}

// ============================================================================
// Assertion Result
// ============================================================================

/// Result of an assertion check
#[derive(Debug, Clone)]
pub enum AssertionResult {
    /// Assertion passed
    Passed,

    /// Assertion failed
    Failed(Assertion),
}

impl AssertionResult {
    /// Check if the assertion passed
    pub fn is_passed(&self) -> bool {
        matches!(self, AssertionResult::Passed)
    }

    /// Check if the assertion failed
    pub fn is_failed(&self) -> bool {
        matches!(self, AssertionResult::Failed(_))
    }

    /// Convert to Result
    pub fn to_result(&self) -> Result<(), TestError> {
        match self {
            AssertionResult::Passed => Ok(()),
            AssertionResult::Failed(assertion) => Err(assertion.to_error()),
        }
    }
}

// ============================================================================
// Assertion Builder (Fluent API)
// ============================================================================

/// Fluent assertion builder for expect-style assertions
///
/// Example:
/// ```rust
/// use aria_test::assertions::Expect;
///
/// let result = 42;
/// Expect::that(result).to_equal(42);
/// ```
#[derive(Debug)]
pub struct Expect<T> {
    value: T,
    negated: bool,
    message: Option<String>,
}

impl<T> Expect<T> {
    /// Create a new expectation for a value
    pub fn that(value: T) -> Self {
        Self {
            value,
            negated: false,
            message: None,
        }
    }

    /// Negate the next assertion
    pub fn not(mut self) -> Self {
        self.negated = !self.negated;
        self
    }

    /// Set a custom failure message
    pub fn with_message(mut self, msg: impl Into<String>) -> Self {
        self.message = Some(msg.into());
        self
    }
}

impl<T: PartialEq + std::fmt::Debug> Expect<T> {
    /// Assert equality
    pub fn to_equal(self, expected: T) -> AssertionResult {
        let passed = if self.negated {
            self.value != expected
        } else {
            self.value == expected
        };

        if passed {
            AssertionResult::Passed
        } else {
            let kind = if self.negated {
                AssertionKind::AssertNe
            } else {
                AssertionKind::AssertEq
            };
            AssertionResult::Failed(
                Assertion::new(kind)
                    .with_expected(format!("{:?}", expected))
                    .with_actual(format!("{:?}", self.value)),
            )
        }
    }

    /// Alias for to_equal
    pub fn to_eq(self, expected: T) -> AssertionResult {
        self.to_equal(expected)
    }

    /// Alias for not().to_equal()
    pub fn to_not_equal(self, expected: T) -> AssertionResult {
        self.not().to_equal(expected)
    }
}

impl<T: PartialOrd + std::fmt::Debug> Expect<T> {
    /// Assert less than
    pub fn to_be_less_than(self, expected: T) -> AssertionResult {
        let passed = if self.negated {
            self.value >= expected
        } else {
            self.value < expected
        };

        if passed {
            AssertionResult::Passed
        } else {
            AssertionResult::Failed(
                Assertion::new(AssertionKind::AssertLt)
                    .with_expected(format!("< {:?}", expected))
                    .with_actual(format!("{:?}", self.value)),
            )
        }
    }

    /// Assert greater than
    pub fn to_be_greater_than(self, expected: T) -> AssertionResult {
        let passed = if self.negated {
            self.value <= expected
        } else {
            self.value > expected
        };

        if passed {
            AssertionResult::Passed
        } else {
            AssertionResult::Failed(
                Assertion::new(AssertionKind::AssertGt)
                    .with_expected(format!("> {:?}", expected))
                    .with_actual(format!("{:?}", self.value)),
            )
        }
    }

    /// Assert less than or equal
    pub fn to_be_at_most(self, expected: T) -> AssertionResult {
        let passed = if self.negated {
            self.value > expected
        } else {
            self.value <= expected
        };

        if passed {
            AssertionResult::Passed
        } else {
            AssertionResult::Failed(
                Assertion::new(AssertionKind::AssertLe)
                    .with_expected(format!("<= {:?}", expected))
                    .with_actual(format!("{:?}", self.value)),
            )
        }
    }

    /// Assert greater than or equal
    pub fn to_be_at_least(self, expected: T) -> AssertionResult {
        let passed = if self.negated {
            self.value < expected
        } else {
            self.value >= expected
        };

        if passed {
            AssertionResult::Passed
        } else {
            AssertionResult::Failed(
                Assertion::new(AssertionKind::AssertGe)
                    .with_expected(format!(">= {:?}", expected))
                    .with_actual(format!("{:?}", self.value)),
            )
        }
    }
}

impl Expect<bool> {
    /// Assert value is true
    pub fn to_be_true(self) -> AssertionResult {
        let passed = if self.negated { !self.value } else { self.value };

        if passed {
            AssertionResult::Passed
        } else {
            AssertionResult::Failed(
                Assertion::new(AssertionKind::Assert)
                    .with_expected("true".to_string())
                    .with_actual(format!("{}", self.value)),
            )
        }
    }

    /// Assert value is false
    pub fn to_be_false(self) -> AssertionResult {
        let passed = if self.negated { self.value } else { !self.value };

        if passed {
            AssertionResult::Passed
        } else {
            AssertionResult::Failed(
                Assertion::new(AssertionKind::Assert)
                    .with_expected("false".to_string())
                    .with_actual(format!("{}", self.value)),
            )
        }
    }
}

impl<T> Expect<Option<T>>
where
    T: std::fmt::Debug,
{
    /// Assert Option is Some
    pub fn to_be_some(self) -> AssertionResult {
        let passed = if self.negated {
            self.value.is_none()
        } else {
            self.value.is_some()
        };

        if passed {
            AssertionResult::Passed
        } else {
            AssertionResult::Failed(
                Assertion::new(AssertionKind::AssertSome)
                    .with_expected("Some(_)".to_string())
                    .with_actual("None".to_string()),
            )
        }
    }

    /// Assert Option is None
    pub fn to_be_none(self) -> AssertionResult {
        let passed = if self.negated {
            self.value.is_some()
        } else {
            self.value.is_none()
        };

        if passed {
            AssertionResult::Passed
        } else {
            AssertionResult::Failed(
                Assertion::new(AssertionKind::AssertNone)
                    .with_expected("None".to_string())
                    .with_actual(format!("{:?}", self.value)),
            )
        }
    }
}

impl<T, E> Expect<Result<T, E>>
where
    T: std::fmt::Debug,
    E: std::fmt::Debug,
{
    /// Assert Result is Ok
    pub fn to_be_ok(self) -> AssertionResult {
        let passed = if self.negated {
            self.value.is_err()
        } else {
            self.value.is_ok()
        };

        if passed {
            AssertionResult::Passed
        } else {
            AssertionResult::Failed(
                Assertion::new(AssertionKind::AssertOk)
                    .with_expected("Ok(_)".to_string())
                    .with_actual(format!("{:?}", self.value)),
            )
        }
    }

    /// Assert Result is Err
    pub fn to_be_err(self) -> AssertionResult {
        let passed = if self.negated {
            self.value.is_ok()
        } else {
            self.value.is_err()
        };

        if passed {
            AssertionResult::Passed
        } else {
            AssertionResult::Failed(
                Assertion::new(AssertionKind::AssertErr)
                    .with_expected("Err(_)".to_string())
                    .with_actual(format!("{:?}", self.value)),
            )
        }
    }
}

impl<T> Expect<Vec<T>>
where
    T: std::fmt::Debug,
{
    /// Assert collection is empty
    pub fn to_be_empty(self) -> AssertionResult {
        let passed = if self.negated {
            !self.value.is_empty()
        } else {
            self.value.is_empty()
        };

        if passed {
            AssertionResult::Passed
        } else {
            AssertionResult::Failed(
                Assertion::new(AssertionKind::AssertEmpty)
                    .with_expected("empty collection".to_string())
                    .with_actual(format!("{} elements", self.value.len())),
            )
        }
    }

    /// Assert collection has specific length
    pub fn to_have_length(self, expected: usize) -> AssertionResult {
        let passed = if self.negated {
            self.value.len() != expected
        } else {
            self.value.len() == expected
        };

        if passed {
            AssertionResult::Passed
        } else {
            AssertionResult::Failed(
                Assertion::new(AssertionKind::AssertEq)
                    .with_expected(format!("length {}", expected))
                    .with_actual(format!("length {}", self.value.len())),
            )
        }
    }
}

impl<T> Expect<Vec<T>>
where
    T: std::fmt::Debug + PartialEq,
{
    /// Assert collection contains element
    pub fn to_contain(self, element: T) -> AssertionResult {
        let contains = self.value.contains(&element);
        let passed = if self.negated { !contains } else { contains };

        if passed {
            AssertionResult::Passed
        } else {
            AssertionResult::Failed(
                Assertion::new(AssertionKind::AssertContains)
                    .with_expected(format!("contains {:?}", element))
                    .with_actual(format!("{:?}", self.value)),
            )
        }
    }
}

// ============================================================================
// Assertion Macros (for Rust usage)
// ============================================================================

/// Assert a condition is true
#[macro_export]
macro_rules! assert_test {
    ($cond:expr) => {
        if !$cond {
            return Err($crate::TestError::AssertionFailed {
                message: format!("Assertion failed: {}", stringify!($cond)),
                expected: Some("true".to_string()),
                actual: Some("false".to_string()),
                location: Some($crate::SourceLocation::new(file!(), line!(), column!())),
            });
        }
    };
    ($cond:expr, $msg:expr) => {
        if !$cond {
            return Err($crate::TestError::AssertionFailed {
                message: $msg.to_string(),
                expected: Some("true".to_string()),
                actual: Some("false".to_string()),
                location: Some($crate::SourceLocation::new(file!(), line!(), column!())),
            });
        }
    };
}

/// Assert two values are equal
#[macro_export]
macro_rules! assert_eq_test {
    ($left:expr, $right:expr) => {
        if $left != $right {
            return Err($crate::TestError::AssertionFailed {
                message: format!(
                    "Assertion failed: {} == {}",
                    stringify!($left),
                    stringify!($right)
                ),
                expected: Some(format!("{:?}", $right)),
                actual: Some(format!("{:?}", $left)),
                location: Some($crate::SourceLocation::new(file!(), line!(), column!())),
            });
        }
    };
}

/// Assert two values are not equal
#[macro_export]
macro_rules! assert_ne_test {
    ($left:expr, $right:expr) => {
        if $left == $right {
            return Err($crate::TestError::AssertionFailed {
                message: format!(
                    "Assertion failed: {} != {}",
                    stringify!($left),
                    stringify!($right)
                ),
                expected: Some(format!("not {:?}", $right)),
                actual: Some(format!("{:?}", $left)),
                location: Some($crate::SourceLocation::new(file!(), line!(), column!())),
            });
        }
    };
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assertion_kinds() {
        let assertion = Assertion::new(AssertionKind::AssertEq)
            .with_expected("42")
            .with_actual("43");

        let error = assertion.to_error();
        match error {
            TestError::AssertionFailed {
                expected, actual, ..
            } => {
                assert_eq!(expected, Some("42".to_string()));
                assert_eq!(actual, Some("43".to_string()));
            }
            _ => panic!("Expected AssertionFailed"),
        }
    }

    #[test]
    fn test_expect_equality() {
        assert!(Expect::that(42).to_equal(42).is_passed());
        assert!(Expect::that(42).to_equal(43).is_failed());
        assert!(Expect::that(42).not().to_equal(43).is_passed());
        assert!(Expect::that(42).not().to_equal(42).is_failed());
    }

    #[test]
    fn test_expect_comparisons() {
        assert!(Expect::that(5).to_be_less_than(10).is_passed());
        assert!(Expect::that(10).to_be_less_than(5).is_failed());
        assert!(Expect::that(10).to_be_greater_than(5).is_passed());
        assert!(Expect::that(5).to_be_greater_than(10).is_failed());
        assert!(Expect::that(5).to_be_at_most(5).is_passed());
        assert!(Expect::that(5).to_be_at_least(5).is_passed());
    }

    #[test]
    fn test_expect_boolean() {
        assert!(Expect::that(true).to_be_true().is_passed());
        assert!(Expect::that(false).to_be_true().is_failed());
        assert!(Expect::that(false).to_be_false().is_passed());
        assert!(Expect::that(true).to_be_false().is_failed());
    }

    #[test]
    fn test_expect_option() {
        assert!(Expect::that(Some(42)).to_be_some().is_passed());
        assert!(Expect::that(None::<i32>).to_be_some().is_failed());
        assert!(Expect::that(None::<i32>).to_be_none().is_passed());
        assert!(Expect::that(Some(42)).to_be_none().is_failed());
    }

    #[test]
    fn test_expect_result() {
        let ok: Result<i32, &str> = Ok(42);
        let err: Result<i32, &str> = Err("error");

        assert!(Expect::that(ok).to_be_ok().is_passed());
        assert!(Expect::that(err.clone()).to_be_ok().is_failed());
        assert!(Expect::that(err).to_be_err().is_passed());
    }

    #[test]
    fn test_expect_collection() {
        let empty: Vec<i32> = vec![];
        let items = vec![1, 2, 3];

        assert!(Expect::that(empty.clone()).to_be_empty().is_passed());
        assert!(Expect::that(items.clone()).to_be_empty().is_failed());
        assert!(Expect::that(items.clone()).to_have_length(3).is_passed());
        assert!(Expect::that(items.clone()).to_contain(2).is_passed());
        assert!(Expect::that(items).to_contain(5).is_failed());
    }
}
