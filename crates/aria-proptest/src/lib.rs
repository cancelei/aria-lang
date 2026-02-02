//! Property-Based Testing for Aria
//!
//! This module provides QuickCheck-style property testing with:
//! - Automatic value generation based on types
//! - Shrinking to find minimal counterexamples
//! - Integration with the contract system
//!
//! Based on the ideas from QuickCheck, Hypothesis, and Hedgehog.

pub mod generator;
pub mod shrink;
pub mod property;
pub mod runner;

pub use generator::{Generator, GenContext, Arbitrary};
pub use shrink::{Shrinker, ShrinkIterator};
pub use property::{Property, PropertyResult, Counterexample};
pub use runner::{TestRunner, TestConfig, TestResult};

use smol_str::SmolStr;

/// A generated value with shrinking capability
#[derive(Debug, Clone)]
pub struct Generated<T> {
    /// The generated value
    pub value: T,
    /// The seed used for generation (for reproducibility)
    pub seed: u64,
    /// Shrink history (for debugging)
    pub shrink_steps: usize,
}

impl<T> Generated<T> {
    pub fn new(value: T, seed: u64) -> Self {
        Self {
            value,
            seed,
            shrink_steps: 0,
        }
    }
}

/// Type information for generator inference
#[derive(Debug, Clone, PartialEq)]
pub enum AriaType {
    Int,
    Float,
    Bool,
    String,
    Char,
    Unit,
    Array(Box<AriaType>),
    Tuple(Vec<AriaType>),
    Option(Box<AriaType>),
    Result(Box<AriaType>, Box<AriaType>),
    Custom(SmolStr),
}

/// Value representation for testing
#[derive(Debug, Clone, PartialEq)]
pub enum AriaValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Char(char),
    Unit,
    Array(Vec<AriaValue>),
    Tuple(Vec<AriaValue>),
    Option(Option<Box<AriaValue>>),
    Result(Result<Box<AriaValue>, Box<AriaValue>>),
}

impl AriaValue {
    /// Display value for error messages
    pub fn display(&self) -> String {
        match self {
            AriaValue::Int(n) => n.to_string(),
            AriaValue::Float(f) => f.to_string(),
            AriaValue::Bool(b) => b.to_string(),
            AriaValue::String(s) => format!("\"{}\"", s),
            AriaValue::Char(c) => format!("'{}'", c),
            AriaValue::Unit => "()".to_string(),
            AriaValue::Array(elems) => {
                let inner: Vec<_> = elems.iter().map(|v| v.display()).collect();
                format!("[{}]", inner.join(", "))
            }
            AriaValue::Tuple(elems) => {
                let inner: Vec<_> = elems.iter().map(|v| v.display()).collect();
                format!("({})", inner.join(", "))
            }
            AriaValue::Option(Some(v)) => format!("Some({})", v.display()),
            AriaValue::Option(None) => "None".to_string(),
            AriaValue::Result(Ok(v)) => format!("Ok({})", v.display()),
            AriaValue::Result(Err(e)) => format!("Err({})", e.display()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use generator::GenContext;

    #[test]
    fn test_value_display() {
        assert_eq!(AriaValue::Int(42).display(), "42");
        assert_eq!(AriaValue::Bool(true).display(), "true");
        assert_eq!(AriaValue::String("hello".into()).display(), "\"hello\"");
        assert_eq!(
            AriaValue::Array(vec![AriaValue::Int(1), AriaValue::Int(2)]).display(),
            "[1, 2]"
        );
    }

    #[test]
    fn test_generator_int() {
        let mut ctx = GenContext::new(12345);
        let gen = i64::arbitrary();
        let value = gen.generate(&mut ctx);
        // Should generate some integer
        assert!(matches!(value, AriaValue::Int(_)));
    }

    #[test]
    fn test_generator_bool() {
        let mut ctx = GenContext::new(12345);
        let gen = bool::arbitrary();
        let value = gen.generate(&mut ctx);
        assert!(matches!(value, AriaValue::Bool(_)));
    }

    #[test]
    fn test_generator_array() {
        let mut ctx = GenContext::new(12345);
        let gen = Vec::<i64>::arbitrary();
        let value = gen.generate(&mut ctx);
        assert!(matches!(value, AriaValue::Array(_)));
    }

    #[test]
    fn test_shrink_int() {
        let shrinker = i64::shrinker(100);
        let shrunk: Vec<_> = shrinker.take(5).collect();
        // Should shrink towards 0
        assert!(!shrunk.is_empty());
        for v in &shrunk {
            if let AriaValue::Int(n) = v {
                assert!(n.abs() < 100);
            }
        }
    }

    #[test]
    fn test_property_runner() {
        let config = TestConfig::default();
        let mut runner = TestRunner::new(config);

        // Property: all integers are less than 1000000
        let result = runner.run(|ctx| {
            let gen = i64::arbitrary();
            let value = gen.generate(ctx);
            if let AriaValue::Int(n) = &value {
                if n.abs() < 1_000_000 {
                    PropertyResult::Pass
                } else {
                    PropertyResult::Fail(value)
                }
            } else {
                PropertyResult::Pass
            }
        });

        // This should pass (default range is smaller)
        assert!(matches!(result, TestResult::Success { .. }));
    }
}
