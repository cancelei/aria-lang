//! Or-pattern expansion and handling.
//!
//! Or-patterns like `A | B | C` need special handling for exhaustiveness checking
//! and decision tree compilation. This module provides utilities for expanding
//! or-patterns into multiple pattern rows.

use crate::{DeconstructedPattern, PatternMatrix, PatternRow, PatternType};
use aria_ast::{Pattern, PatternKind};

/// Expand or-patterns in a pattern matrix.
///
/// Each row containing an or-pattern is expanded into multiple rows,
/// one for each alternative.
pub fn expand_or_patterns(matrix: &PatternMatrix) -> PatternMatrix {
    let mut expanded = PatternMatrix::new(matrix.column_types.clone());

    for row in &matrix.rows {
        let expanded_rows = expand_row(row, &matrix.column_types);
        for expanded_row in expanded_rows {
            expanded.push_row(expanded_row);
        }
    }

    expanded
}

/// Expand a single row that may contain or-patterns.
fn expand_row(row: &PatternRow, types: &[PatternType]) -> Vec<PatternRow> {
    // Find the first or-pattern in the row
    for (i, pat) in row.patterns.iter().enumerate() {
        if has_or_pattern(pat) {
            // Expand this position
            let alternatives = extract_or_alternatives(pat, types.get(i));
            let mut result = Vec::new();

            for alt in alternatives {
                let mut new_patterns = row.patterns.clone();
                new_patterns[i] = alt;

                // Create new row and recursively expand remaining or-patterns
                let new_row = PatternRow {
                    patterns: new_patterns,
                    arm_index: row.arm_index,
                };

                result.extend(expand_row(&new_row, types));
            }

            return result;
        }
    }

    // No or-patterns found, return as-is
    vec![row.clone()]
}

/// Check if a pattern contains an or-pattern at the top level
fn has_or_pattern(_pat: &DeconstructedPattern) -> bool {
    // In the current implementation, or-patterns are flattened during construction
    // This is a conservative check for future expansion
    false
}

/// Extract alternatives from an or-pattern
fn extract_or_alternatives(
    pat: &DeconstructedPattern,
    _ty: Option<&PatternType>,
) -> Vec<DeconstructedPattern> {
    // Currently, or-patterns are already flattened in DeconstructedPattern::from_ast
    // This function is a placeholder for more sophisticated handling
    vec![pat.clone()]
}

/// Expand or-patterns from AST patterns into multiple patterns.
///
/// This is useful before constructing a pattern matrix.
pub fn expand_ast_or_pattern(pattern: &Pattern) -> Vec<Pattern> {
    match &pattern.kind {
        PatternKind::Or(pats) => {
            let mut result = Vec::new();
            for pat in pats {
                result.extend(expand_ast_or_pattern(pat));
            }
            result
        }
        PatternKind::Guard { pattern: inner, condition } => {
            // Guards don't expand, but inner patterns might
            let expanded = expand_ast_or_pattern(inner);
            expanded.into_iter()
                .map(|p| Pattern {
                    kind: PatternKind::Guard {
                        pattern: Box::new(p),
                        condition: condition.clone(),
                    },
                    span: pattern.span,
                })
                .collect()
        }
        PatternKind::Binding { name, pattern: inner } => {
            let expanded = expand_ast_or_pattern(inner);
            expanded.into_iter()
                .map(|p| Pattern {
                    kind: PatternKind::Binding {
                        name: name.clone(),
                        pattern: Box::new(p),
                    },
                    span: pattern.span,
                })
                .collect()
        }
        PatternKind::Typed { pattern: inner, ty } => {
            let expanded = expand_ast_or_pattern(inner);
            expanded.into_iter()
                .map(|p| Pattern {
                    kind: PatternKind::Typed {
                        pattern: Box::new(p),
                        ty: ty.clone(),
                    },
                    span: pattern.span,
                })
                .collect()
        }
        _ => vec![pattern.clone()],
    }
}

/// Check if a pattern contains nested or-patterns
pub fn contains_or_pattern(pattern: &Pattern) -> bool {
    match &pattern.kind {
        PatternKind::Or(_) => true,
        PatternKind::Guard { pattern: inner, .. } |
        PatternKind::Binding { pattern: inner, .. } |
        PatternKind::Typed { pattern: inner, .. } => {
            contains_or_pattern(inner)
        }
        PatternKind::Tuple(pats) => pats.iter().any(contains_or_pattern),
        PatternKind::Array { elements, rest } => {
            elements.iter().any(contains_or_pattern) ||
            rest.as_ref().map_or(false, |r| contains_or_pattern(r))
        }
        PatternKind::Struct { fields, .. } => {
            fields.iter().any(|f| {
                f.pattern.as_ref().map_or(false, contains_or_pattern)
            })
        }
        PatternKind::Variant { fields: Some(pats), .. } => {
            pats.iter().any(contains_or_pattern)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smol_str::SmolStr;
    use aria_lexer::Span;

    #[test]
    fn test_expand_simple_or_pattern() {
        let or_pat = Pattern {
            kind: PatternKind::Or(vec![
                Pattern {
                    kind: PatternKind::Ident(SmolStr::new("A")),
                    span: Span::new(0, 0),
                },
                Pattern {
                    kind: PatternKind::Ident(SmolStr::new("B")),
                    span: Span::new(0, 0),
                },
            ]),
            span: Span::new(0, 0),
        };

        let expanded = expand_ast_or_pattern(&or_pat);
        assert_eq!(expanded.len(), 2);
    }

    #[test]
    fn test_contains_or_pattern() {
        let or_pat = Pattern {
            kind: PatternKind::Or(vec![
                Pattern {
                    kind: PatternKind::Wildcard,
                    span: Span::new(0, 0),
                },
            ]),
            span: Span::new(0, 0),
        };

        assert!(contains_or_pattern(&or_pat));

        let simple_pat = Pattern {
            kind: PatternKind::Wildcard,
            span: Span::new(0, 0),
        };

        assert!(!contains_or_pattern(&simple_pat));
    }

    #[test]
    fn test_nested_or_pattern_with_guard() {
        let pattern = Pattern {
            kind: PatternKind::Guard {
                pattern: Box::new(Pattern {
                    kind: PatternKind::Or(vec![
                        Pattern {
                            kind: PatternKind::Ident(SmolStr::new("x")),
                            span: Span::new(0, 0),
                        },
                        Pattern {
                            kind: PatternKind::Ident(SmolStr::new("y")),
                            span: Span::new(0, 0),
                        },
                    ]),
                    span: Span::new(0, 0),
                }),
                condition: Box::new(aria_ast::Expr {
                    kind: aria_ast::ExprKind::Bool(true),
                    span: Span::new(0, 0),
                }),
            },
            span: Span::new(0, 0),
        };

        assert!(contains_or_pattern(&pattern));

        let expanded = expand_ast_or_pattern(&pattern);
        assert_eq!(expanded.len(), 2);

        // Both should be guards
        for exp in expanded {
            assert!(matches!(exp.kind, PatternKind::Guard { .. }));
        }
    }
}
