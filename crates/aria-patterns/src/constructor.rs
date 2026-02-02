//! Pattern constructors for exhaustiveness checking.
//!
//! A constructor represents a way to construct a value of a type.
//! For example, `true` and `false` are constructors for Bool.

use aria_ast::Expr;
use smol_str::SmolStr;
use rustc_hash::FxHashSet;

use crate::PatternType;

/// A constructor for pattern matching.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Constructor {
    /// Wildcard matches any value
    Wildcard,
    /// Boolean literal
    Bool(bool),
    /// Integer literal
    Int(i64),
    /// Float literal (stored as bits for hashing)
    Float(u64),
    /// String literal
    String(SmolStr),
    /// Unit value ()
    Unit,
    /// Tuple with given arity
    Tuple(usize),
    /// Array with given length
    Array(usize),
    /// Enum variant with name and index
    Variant { name: SmolStr, index: usize },
    /// Struct constructor
    Struct { name: SmolStr },
    /// Range constructor (for numeric ranges)
    Range { start: i64, end: i64, inclusive: bool },
    /// Missing constructor (represents all non-covered cases)
    Missing,
}

impl Constructor {
    /// Check if this constructor is a wildcard
    pub fn is_wildcard(&self) -> bool {
        matches!(self, Constructor::Wildcard)
    }

    /// Check if this constructor covers another constructor.
    /// Wildcard covers everything.
    pub fn covers(&self, other: &Constructor) -> bool {
        match self {
            Constructor::Wildcard => true,
            _ => self == other,
        }
    }

    /// Get the arity (number of fields) of this constructor
    pub fn arity(&self, ty: &PatternType) -> usize {
        match self {
            Constructor::Wildcard => 0,
            Constructor::Bool(_) => 0,
            Constructor::Int(_) => 0,
            Constructor::Float(_) => 0,
            Constructor::String(_) => 0,
            Constructor::Unit => 0,
            Constructor::Tuple(n) => *n,
            Constructor::Array(n) => *n,
            Constructor::Variant { index, .. } => {
                if let PatternType::Enum { variants, .. } = ty {
                    variants.get(*index).map(|v| v.fields.len()).unwrap_or(0)
                } else {
                    0
                }
            }
            Constructor::Struct { .. } => {
                if let PatternType::Struct { fields, .. } = ty {
                    fields.len()
                } else {
                    0
                }
            }
            Constructor::Range { .. } => 0,
            Constructor::Missing => 0,
        }
    }

    /// Create a constructor from a literal expression
    pub fn from_literal_expr(expr: &Expr) -> Self {
        use aria_ast::ExprKind;
        match &expr.kind {
            ExprKind::Integer(s) => {
                s.parse::<i64>().map(Constructor::Int).unwrap_or(Constructor::Wildcard)
            }
            ExprKind::Float(s) => {
                s.parse::<f64>().map(|f| Constructor::Float(f.to_bits())).unwrap_or(Constructor::Wildcard)
            }
            ExprKind::String(s) => Constructor::String(s.clone()),
            ExprKind::Bool(b) => Constructor::Bool(*b),
            ExprKind::Char(s) => {
                s.chars().next().map(|c| Constructor::Int(c as i64)).unwrap_or(Constructor::Wildcard)
            }
            ExprKind::Nil => Constructor::Unit,
            _ => Constructor::Wildcard,
        }
    }
}

/// A set of constructors for a type.
#[derive(Debug, Clone)]
pub struct ConstructorSet {
    /// The type this set is for
    pub ty: PatternType,
    /// All possible constructors
    constructors: Vec<Constructor>,
    /// Whether the type has infinite constructors
    pub is_infinite: bool,
}

impl ConstructorSet {
    /// Create a constructor set for a type
    pub fn for_type(ty: &PatternType) -> Self {
        match ty {
            PatternType::Bool => Self {
                ty: ty.clone(),
                constructors: vec![Constructor::Bool(true), Constructor::Bool(false)],
                is_infinite: false,
            },
            PatternType::Unit => Self {
                ty: ty.clone(),
                constructors: vec![Constructor::Unit],
                is_infinite: false,
            },
            PatternType::Int | PatternType::Float | PatternType::String => Self {
                ty: ty.clone(),
                constructors: Vec::new(), // Infinite, can't enumerate
                is_infinite: true,
            },
            PatternType::Tuple(types) => Self {
                ty: ty.clone(),
                constructors: vec![Constructor::Tuple(types.len())],
                is_infinite: false,
            },
            PatternType::Array(_) => Self {
                ty: ty.clone(),
                constructors: Vec::new(), // Infinite lengths
                is_infinite: true,
            },
            PatternType::Enum { variants, .. } => Self {
                ty: ty.clone(),
                constructors: variants.iter()
                    .enumerate()
                    .map(|(i, v)| Constructor::Variant {
                        name: v.name.clone(),
                        index: i,
                    })
                    .collect(),
                is_infinite: false,
            },
            PatternType::Struct { name, .. } => Self {
                ty: ty.clone(),
                constructors: vec![Constructor::Struct { name: name.clone() }],
                is_infinite: false,
            },
            PatternType::Unknown => Self {
                ty: ty.clone(),
                constructors: Vec::new(),
                is_infinite: true,
            },
        }
    }

    /// Get constructors not covered by the given set
    pub fn missing(&self, covered: &FxHashSet<Constructor>) -> Vec<Constructor> {
        if self.is_infinite {
            // For infinite types, if no wildcard is present, report Missing
            if !covered.iter().any(|c| c.is_wildcard()) && covered.is_empty() {
                vec![Constructor::Missing]
            } else {
                Vec::new()
            }
        } else {
            self.constructors.iter()
                .filter(|c| !covered.contains(*c))
                .cloned()
                .collect()
        }
    }

    /// Check if all constructors are covered
    pub fn is_exhaustive(&self, covered: &FxHashSet<Constructor>) -> bool {
        if covered.iter().any(|c| c.is_wildcard()) {
            return true;
        }
        if self.is_infinite {
            return false; // Can't exhaust infinite types without wildcard
        }
        self.constructors.iter().all(|c| covered.contains(c))
    }

    /// Get all constructors
    pub fn all_constructors(&self) -> &[Constructor] {
        &self.constructors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bool_constructor_set() {
        let set = ConstructorSet::for_type(&PatternType::Bool);
        assert!(!set.is_infinite);
        assert_eq!(set.constructors.len(), 2);
    }

    #[test]
    fn test_int_is_infinite() {
        let set = ConstructorSet::for_type(&PatternType::Int);
        assert!(set.is_infinite);
    }

    #[test]
    fn test_enum_constructor_set() {
        let ty = PatternType::Enum {
            name: SmolStr::new("Option"),
            variants: vec![
                crate::EnumVariant { name: SmolStr::new("Some"), fields: vec![PatternType::Int] },
                crate::EnumVariant { name: SmolStr::new("None"), fields: vec![] },
            ],
        };
        let set = ConstructorSet::for_type(&ty);
        assert!(!set.is_infinite);
        assert_eq!(set.constructors.len(), 2);
    }

    #[test]
    fn test_missing_constructors() {
        let set = ConstructorSet::for_type(&PatternType::Bool);
        let mut covered = FxHashSet::default();
        covered.insert(Constructor::Bool(true));

        let missing = set.missing(&covered);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0], Constructor::Bool(false));
    }
}
