//! Witness generation for non-exhaustive patterns.
//!
//! A witness is a concrete pattern that demonstrates what values are not covered
//! by the match expression.

use crate::Constructor;
use std::fmt;

/// A witness to non-exhaustiveness.
/// Represents a pattern that is not covered by the match.
#[derive(Debug, Clone)]
pub struct Witness {
    /// The constructor at the head
    pub ctor: Option<Constructor>,
    /// Field witnesses (for constructors with fields)
    pub fields: Vec<Witness>,
}

impl Witness {
    /// Create an empty witness (represents completed matching)
    pub fn empty() -> Self {
        Self {
            ctor: None,
            fields: Vec::new(),
        }
    }

    /// Create a wildcard witness
    pub fn wildcard() -> Self {
        Self {
            ctor: Some(Constructor::Wildcard),
            fields: Vec::new(),
        }
    }

    /// Prepend a constructor to this witness
    pub fn prepend(&mut self, ctor: Constructor, field_witnesses: Vec<Witness>) {
        // The current witness becomes a field of the new one
        let old_self = std::mem::replace(self, Self {
            ctor: Some(ctor),
            fields: field_witnesses,
        });

        // If the old witness had content, add it as a continuation
        if old_self.ctor.is_some() {
            self.fields.push(old_self);
        }
    }

    /// Pop a field witness for expansion
    pub fn pop_field(&mut self) -> Option<Witness> {
        self.fields.pop()
    }

    /// Check if this witness is empty
    pub fn is_empty(&self) -> bool {
        self.ctor.is_none() && self.fields.is_empty()
    }

    /// Convert to a displayable pattern string
    pub fn to_pattern_string(&self) -> String {
        match &self.ctor {
            None => "_".to_string(),
            Some(Constructor::Wildcard) => "_".to_string(),
            Some(Constructor::Bool(b)) => b.to_string(),
            Some(Constructor::Int(n)) => n.to_string(),
            Some(Constructor::Float(bits)) => f64::from_bits(*bits).to_string(),
            Some(Constructor::String(s)) => format!("\"{}\"", s),
            Some(Constructor::Unit) => "()".to_string(),
            Some(Constructor::Tuple(n)) => {
                let fields: Vec<String> = self.fields.iter()
                    .take(*n)
                    .map(|w| w.to_pattern_string())
                    .collect();
                format!("({})", fields.join(", "))
            }
            Some(Constructor::Array(n)) => {
                let elements: Vec<String> = self.fields.iter()
                    .take(*n)
                    .map(|w| w.to_pattern_string())
                    .collect();
                format!("[{}]", elements.join(", "))
            }
            Some(Constructor::Variant { name, .. }) => {
                if self.fields.is_empty() {
                    name.to_string()
                } else {
                    let fields: Vec<String> = self.fields.iter()
                        .map(|w| w.to_pattern_string())
                        .collect();
                    format!("{}({})", name, fields.join(", "))
                }
            }
            Some(Constructor::Struct { name }) => {
                if self.fields.is_empty() {
                    format!("{} {{ }}", name)
                } else {
                    let fields: Vec<String> = self.fields.iter()
                        .map(|w| w.to_pattern_string())
                        .collect();
                    format!("{} {{ {} }}", name, fields.join(", "))
                }
            }
            Some(Constructor::Range { start, end, inclusive }) => {
                if *inclusive {
                    format!("{}..={}", start, end)
                } else {
                    format!("{}..{}", start, end)
                }
            }
            Some(Constructor::Missing) => "_".to_string(),
        }
    }
}

impl fmt::Display for Witness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_pattern_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wildcard_witness() {
        let w = Witness::wildcard();
        assert_eq!(w.to_pattern_string(), "_");
    }

    #[test]
    fn test_bool_witness() {
        let mut w = Witness::empty();
        w.prepend(Constructor::Bool(false), Vec::new());
        assert_eq!(w.to_pattern_string(), "false");
    }

    #[test]
    fn test_variant_witness() {
        let mut w = Witness::empty();
        let field = Witness::wildcard();
        w.prepend(
            Constructor::Variant {
                name: smol_str::SmolStr::new("Some"),
                index: 0,
            },
            vec![field],
        );
        assert_eq!(w.to_pattern_string(), "Some(_)");
    }

    #[test]
    fn test_tuple_witness() {
        let mut w = Witness::empty();
        w.prepend(
            Constructor::Tuple(2),
            vec![Witness::wildcard(), Witness::wildcard()],
        );
        assert_eq!(w.to_pattern_string(), "(_, _)");
    }
}
