//! Property definitions for testing.
//!
//! Properties are boolean predicates that should hold for all inputs.

use crate::AriaValue;

/// The result of checking a property
#[derive(Debug, Clone)]
pub enum PropertyResult {
    /// Property passed
    Pass,
    /// Property failed with counterexample
    Fail(AriaValue),
    /// Property was discarded (precondition failed)
    Discard,
}

impl PropertyResult {
    /// Check if the property passed
    pub fn is_pass(&self) -> bool {
        matches!(self, PropertyResult::Pass)
    }

    /// Check if the property failed
    pub fn is_fail(&self) -> bool {
        matches!(self, PropertyResult::Fail(_))
    }

    /// Check if the property was discarded
    pub fn is_discard(&self) -> bool {
        matches!(self, PropertyResult::Discard)
    }
}

/// A counterexample to a property
#[derive(Debug, Clone)]
pub struct Counterexample {
    /// The original failing input
    pub original: AriaValue,
    /// The shrunk (minimal) failing input
    pub shrunk: AriaValue,
    /// Number of shrink steps
    pub shrink_steps: usize,
    /// The seed used
    pub seed: u64,
}

impl Counterexample {
    /// Create a new counterexample
    pub fn new(original: AriaValue, shrunk: AriaValue, shrink_steps: usize, seed: u64) -> Self {
        Self {
            original,
            shrunk,
            shrink_steps,
            seed,
        }
    }

    /// Format for display
    pub fn display(&self) -> String {
        format!(
            "Counterexample found:\n  Original: {}\n  Shrunk:   {}\n  Shrink steps: {}\n  Seed: {}",
            self.original.display(),
            self.shrunk.display(),
            self.shrink_steps,
            self.seed
        )
    }
}

/// A property that can be tested
pub trait Property {
    /// Check the property for a given input
    fn check(&self, input: &AriaValue) -> PropertyResult;
}

/// A property defined by a closure
pub struct FnProperty<F>
where
    F: Fn(&AriaValue) -> PropertyResult,
{
    check_fn: F,
}

impl<F> FnProperty<F>
where
    F: Fn(&AriaValue) -> PropertyResult,
{
    pub fn new(check_fn: F) -> Self {
        Self { check_fn }
    }
}

impl<F> Property for FnProperty<F>
where
    F: Fn(&AriaValue) -> PropertyResult,
{
    fn check(&self, input: &AriaValue) -> PropertyResult {
        (self.check_fn)(input)
    }
}

/// Property combinators
impl PropertyResult {
    /// Combine with another property result (both must pass)
    pub fn and(self, other: PropertyResult) -> PropertyResult {
        match (self, other) {
            (PropertyResult::Pass, PropertyResult::Pass) => PropertyResult::Pass,
            (PropertyResult::Fail(v), _) => PropertyResult::Fail(v),
            (_, PropertyResult::Fail(v)) => PropertyResult::Fail(v),
            (PropertyResult::Discard, _) => PropertyResult::Discard,
            (_, PropertyResult::Discard) => PropertyResult::Discard,
        }
    }

    /// Combine with another property result (either can pass)
    pub fn or(self, other: PropertyResult) -> PropertyResult {
        match (self, other) {
            (PropertyResult::Pass, _) => PropertyResult::Pass,
            (_, PropertyResult::Pass) => PropertyResult::Pass,
            (PropertyResult::Fail(v), PropertyResult::Fail(_)) => PropertyResult::Fail(v),
            (PropertyResult::Discard, other) => other,
            (other, PropertyResult::Discard) => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_property_result_and() {
        assert!(PropertyResult::Pass.and(PropertyResult::Pass).is_pass());
        assert!(PropertyResult::Pass.and(PropertyResult::Fail(AriaValue::Unit)).is_fail());
        assert!(PropertyResult::Fail(AriaValue::Unit).and(PropertyResult::Pass).is_fail());
    }

    #[test]
    fn test_property_result_or() {
        assert!(PropertyResult::Pass.or(PropertyResult::Fail(AriaValue::Unit)).is_pass());
        assert!(PropertyResult::Fail(AriaValue::Unit).or(PropertyResult::Pass).is_pass());
        assert!(PropertyResult::Fail(AriaValue::Unit).or(PropertyResult::Fail(AriaValue::Unit)).is_fail());
    }

    #[test]
    fn test_fn_property() {
        let prop = FnProperty::new(|v| {
            if let AriaValue::Int(n) = v {
                if *n > 0 {
                    PropertyResult::Pass
                } else {
                    PropertyResult::Fail(v.clone())
                }
            } else {
                PropertyResult::Discard
            }
        });

        assert!(prop.check(&AriaValue::Int(5)).is_pass());
        assert!(prop.check(&AriaValue::Int(-5)).is_fail());
        assert!(prop.check(&AriaValue::Bool(true)).is_discard());
    }
}
