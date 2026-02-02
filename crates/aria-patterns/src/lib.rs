//! Pattern Matching Exhaustiveness Checking for Aria
//!
//! This module implements the exhaustiveness checking algorithm based on
//! Maranget's "Warnings for Pattern Matching" paper.
//!
//! The algorithm uses a matrix-based approach where:
//! - Rows represent pattern clauses
//! - Columns represent constructor positions
//!
//! A pattern matrix is exhaustive if the "useful" predicate returns false
//! for the wildcard pattern vector.

pub mod exhaustive;
pub mod constructor;
pub mod usefulness;
pub mod witness;
pub mod decision_tree;
pub mod or_pattern;

pub use exhaustive::{check_exhaustiveness, ExhaustivenessResult};
pub use constructor::{Constructor, ConstructorSet};
pub use usefulness::is_useful;
pub use witness::Witness;
pub use decision_tree::{DecisionTree, TestPlace, compile_decision_tree, optimize_tree};
pub use or_pattern::{expand_ast_or_pattern, contains_or_pattern};

use aria_ast::{Pattern, PatternKind};
use aria_lexer::Span;
use smol_str::SmolStr;
use rustc_hash::FxHashSet;

/// A pattern matrix for exhaustiveness checking.
/// Each row is a pattern vector, each column is a constructor position.
#[derive(Debug, Clone)]
pub struct PatternMatrix {
    /// The pattern rows
    pub rows: Vec<PatternRow>,
    /// The types at each column position
    pub column_types: Vec<PatternType>,
}

/// A single row in the pattern matrix
#[derive(Debug, Clone)]
pub struct PatternRow {
    /// The patterns in this row
    pub patterns: Vec<DeconstructedPattern>,
    /// Associated arm index (for error reporting)
    pub arm_index: usize,
}

/// A deconstructed pattern for analysis
#[derive(Debug, Clone)]
pub struct DeconstructedPattern {
    /// The constructor at the head of this pattern
    pub ctor: Constructor,
    /// Sub-patterns (fields/elements)
    pub fields: Vec<DeconstructedPattern>,
    /// Original span for error reporting
    pub span: Span,
}

/// Type information for pattern matching
#[derive(Debug, Clone, PartialEq)]
pub enum PatternType {
    /// Boolean type with two constructors
    Bool,
    /// Integer type (infinite constructors)
    Int,
    /// Float type (infinite constructors)
    Float,
    /// String type (infinite constructors)
    String,
    /// Unit type (single constructor)
    Unit,
    /// Tuple with element types
    Tuple(Vec<PatternType>),
    /// Array with element type
    Array(Box<PatternType>),
    /// Enum with variants
    Enum {
        name: SmolStr,
        variants: Vec<EnumVariant>,
    },
    /// Struct with fields
    Struct {
        name: SmolStr,
        fields: Vec<(SmolStr, PatternType)>,
    },
    /// Unknown/opaque type
    Unknown,
}

/// An enum variant
#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub name: SmolStr,
    pub fields: Vec<PatternType>,
}

impl PatternMatrix {
    /// Create a new empty pattern matrix
    pub fn new(column_types: Vec<PatternType>) -> Self {
        Self {
            rows: Vec::new(),
            column_types,
        }
    }

    /// Add a row to the matrix
    pub fn push_row(&mut self, row: PatternRow) {
        self.rows.push(row);
    }

    /// Check if the matrix is empty
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Get the number of columns
    pub fn num_columns(&self) -> usize {
        self.column_types.len()
    }

    /// Specialize the matrix by a constructor.
    /// Returns a new matrix where:
    /// - Rows with matching constructor have the constructor expanded
    /// - Rows with wildcard have the wildcard replicated
    /// - Rows with non-matching constructor are removed
    pub fn specialize(&self, ctor: &Constructor, ctor_arity: usize) -> PatternMatrix {
        let mut new_column_types = Vec::new();

        // Add types for constructor fields
        if let Some(first_type) = self.column_types.first() {
            match first_type {
                PatternType::Tuple(types) => {
                    new_column_types.extend(types.clone());
                }
                PatternType::Enum { variants, .. } => {
                    if let Constructor::Variant { index, .. } = ctor {
                        if let Some(variant) = variants.get(*index) {
                            new_column_types.extend(variant.fields.clone());
                        }
                    }
                }
                PatternType::Struct { fields, .. } => {
                    for (_, ty) in fields {
                        new_column_types.push(ty.clone());
                    }
                }
                _ => {
                    // For other types, no fields
                }
            }
        }

        // Add remaining column types
        if self.column_types.len() > 1 {
            new_column_types.extend(self.column_types[1..].to_vec());
        }

        let mut result = PatternMatrix::new(new_column_types);

        for row in &self.rows {
            if let Some(first_pat) = row.patterns.first() {
                if first_pat.ctor.covers(ctor) {
                    // Matching or wildcard - expand
                    let mut new_patterns: Vec<DeconstructedPattern> = Vec::new();

                    if first_pat.ctor.is_wildcard() {
                        // Replicate wildcard for each field
                        for _ in 0..ctor_arity {
                            new_patterns.push(DeconstructedPattern::wildcard(first_pat.span));
                        }
                    } else {
                        // Use actual fields
                        new_patterns.extend(first_pat.fields.clone());
                    }

                    // Add remaining patterns
                    if row.patterns.len() > 1 {
                        new_patterns.extend(row.patterns[1..].to_vec());
                    }

                    result.push_row(PatternRow {
                        patterns: new_patterns,
                        arm_index: row.arm_index,
                    });
                }
                // Non-matching constructors are dropped
            }
        }

        result
    }

    /// Default matrix: keep only rows starting with wildcard.
    /// Used when a constructor is not covered by any head constructor.
    pub fn default_matrix(&self) -> PatternMatrix {
        let new_column_types = if self.column_types.len() > 1 {
            self.column_types[1..].to_vec()
        } else {
            Vec::new()
        };

        let mut result = PatternMatrix::new(new_column_types);

        for row in &self.rows {
            if let Some(first_pat) = row.patterns.first() {
                if first_pat.ctor.is_wildcard() {
                    let new_patterns = if row.patterns.len() > 1 {
                        row.patterns[1..].to_vec()
                    } else {
                        Vec::new()
                    };
                    result.push_row(PatternRow {
                        patterns: new_patterns,
                        arm_index: row.arm_index,
                    });
                }
            }
        }

        result
    }

    /// Get all head constructors in the first column
    pub fn head_constructors(&self) -> FxHashSet<Constructor> {
        let mut ctors = FxHashSet::default();
        for row in &self.rows {
            if let Some(first_pat) = row.patterns.first() {
                if !first_pat.ctor.is_wildcard() {
                    ctors.insert(first_pat.ctor.clone());
                }
            }
        }
        ctors
    }
}

impl DeconstructedPattern {
    /// Create a wildcard pattern
    pub fn wildcard(span: Span) -> Self {
        Self {
            ctor: Constructor::Wildcard,
            fields: Vec::new(),
            span,
        }
    }

    /// Create a pattern from an AST pattern
    pub fn from_ast(pattern: &Pattern, ty: &PatternType) -> Self {
        match &pattern.kind {
            PatternKind::Wildcard | PatternKind::Ident(_) => {
                Self::wildcard(pattern.span)
            }

            PatternKind::Literal(expr) => {
                // Extract literal value
                let ctor = Constructor::from_literal_expr(expr);
                Self {
                    ctor,
                    fields: Vec::new(),
                    span: pattern.span,
                }
            }

            PatternKind::Tuple(pats) => {
                let field_types = if let PatternType::Tuple(types) = ty {
                    types.clone()
                } else {
                    vec![PatternType::Unknown; pats.len()]
                };

                let fields: Vec<_> = pats.iter()
                    .zip(field_types.iter())
                    .map(|(p, t)| Self::from_ast(p, t))
                    .collect();

                Self {
                    ctor: Constructor::Tuple(pats.len()),
                    fields,
                    span: pattern.span,
                }
            }

            PatternKind::Variant { variant, fields, .. } => {
                // Look up variant index from type
                let (variant_idx, field_types) = if let PatternType::Enum { variants, .. } = ty {
                    variants.iter()
                        .enumerate()
                        .find(|(_, v)| v.name == variant.node)
                        .map(|(i, v)| (i, v.fields.clone()))
                        .unwrap_or((0, Vec::new()))
                } else {
                    (0, Vec::new())
                };

                let sub_fields: Vec<_> = fields.as_ref()
                    .map(|fs| fs.iter()
                        .enumerate()
                        .map(|(i, p)| Self::from_ast(p, field_types.get(i).unwrap_or(&PatternType::Unknown)))
                        .collect())
                    .unwrap_or_default();

                Self {
                    ctor: Constructor::Variant {
                        name: variant.node.clone(),
                        index: variant_idx,
                    },
                    fields: sub_fields,
                    span: pattern.span,
                }
            }

            PatternKind::Or(pats) => {
                // Or patterns need special handling for exhaustiveness.
                // For now, we treat the first alternative as representative.
                // A complete implementation would expand or-patterns into multiple rows.
                // Note: Or-patterns with guards are treated conservatively.
                if let Some(first) = pats.first() {
                    Self::from_ast(first, ty)
                } else {
                    Self::wildcard(pattern.span)
                }
            }

            PatternKind::Guard { pattern: _inner, .. } => {
                // Guards are runtime conditions that cannot be checked statically.
                // For exhaustiveness checking, we must treat guarded patterns conservatively:
                // they might fail at runtime, so they don't contribute to exhaustiveness.
                // This is why `x if p(x) => ...` still requires a catch-all case.
                Self::wildcard(pattern.span)
            }

            PatternKind::Binding { pattern: inner, .. } => {
                // @ bindings: the inner pattern determines exhaustiveness
                Self::from_ast(inner, ty)
            }

            PatternKind::Range { start, end, inclusive } => {
                // Range patterns for numeric types
                // For exhaustiveness, ranges are complex - treat conservatively
                use aria_ast::ExprKind;
                let start_val = match &start.kind {
                    ExprKind::Integer(s) => s.parse::<i64>().unwrap_or(0),
                    _ => 0,
                };
                let end_val = match &end.kind {
                    ExprKind::Integer(s) => s.parse::<i64>().unwrap_or(0),
                    _ => 0,
                };

                Self {
                    ctor: Constructor::Range {
                        start: start_val,
                        end: end_val,
                        inclusive: *inclusive,
                    },
                    fields: Vec::new(),
                    span: pattern.span,
                }
            }

            PatternKind::Typed { pattern: inner, .. } => {
                // Type annotations don't affect pattern structure
                Self::from_ast(inner, ty)
            }

            _ => Self::wildcard(pattern.span),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_matrix_creation() {
        let matrix = PatternMatrix::new(vec![PatternType::Bool]);
        assert!(matrix.is_empty());
        assert_eq!(matrix.num_columns(), 1);
    }

    #[test]
    fn test_bool_exhaustiveness() {
        let ty = PatternType::Bool;
        let mut matrix = PatternMatrix::new(vec![ty.clone()]);

        // Add `true` pattern
        matrix.push_row(PatternRow {
            patterns: vec![DeconstructedPattern {
                ctor: Constructor::Bool(true),
                fields: Vec::new(),
                span: Span::new(0, 0),
            }],
            arm_index: 0,
        });

        // Add `false` pattern
        matrix.push_row(PatternRow {
            patterns: vec![DeconstructedPattern {
                ctor: Constructor::Bool(false),
                fields: Vec::new(),
                span: Span::new(0, 0),
            }],
            arm_index: 1,
        });

        let result = check_exhaustiveness(&matrix, &[ty]);
        assert!(result.is_exhaustive);
    }

    #[test]
    fn test_bool_non_exhaustive() {
        let ty = PatternType::Bool;
        let mut matrix = PatternMatrix::new(vec![ty.clone()]);

        // Only `true` pattern - not exhaustive
        matrix.push_row(PatternRow {
            patterns: vec![DeconstructedPattern {
                ctor: Constructor::Bool(true),
                fields: Vec::new(),
                span: Span::new(0, 0),
            }],
            arm_index: 0,
        });

        let result = check_exhaustiveness(&matrix, &[ty]);
        assert!(!result.is_exhaustive);
        assert!(!result.missing_patterns.is_empty());
    }

    #[test]
    fn test_wildcard_exhaustive() {
        let ty = PatternType::Int;
        let mut matrix = PatternMatrix::new(vec![ty.clone()]);

        // Wildcard covers everything
        matrix.push_row(PatternRow {
            patterns: vec![DeconstructedPattern::wildcard(Span::new(0, 0))],
            arm_index: 0,
        });

        let result = check_exhaustiveness(&matrix, &[ty]);
        assert!(result.is_exhaustive);
    }

    #[test]
    fn test_enum_exhaustiveness() {
        let ty = PatternType::Enum {
            name: SmolStr::new("Option"),
            variants: vec![
                EnumVariant { name: SmolStr::new("Some"), fields: vec![PatternType::Int] },
                EnumVariant { name: SmolStr::new("None"), fields: vec![] },
            ],
        };

        let mut matrix = PatternMatrix::new(vec![ty.clone()]);

        // Add Some(_)
        matrix.push_row(PatternRow {
            patterns: vec![DeconstructedPattern {
                ctor: Constructor::Variant { name: SmolStr::new("Some"), index: 0 },
                fields: vec![DeconstructedPattern::wildcard(Span::new(0, 0))],
                span: Span::new(0, 0),
            }],
            arm_index: 0,
        });

        // Add None
        matrix.push_row(PatternRow {
            patterns: vec![DeconstructedPattern {
                ctor: Constructor::Variant { name: SmolStr::new("None"), index: 1 },
                fields: vec![],
                span: Span::new(0, 0),
            }],
            arm_index: 1,
        });

        let result = check_exhaustiveness(&matrix, &[ty]);
        assert!(result.is_exhaustive);
    }
}
