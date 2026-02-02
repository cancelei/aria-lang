//! Exhaustiveness checking algorithm.
//!
//! Based on Maranget's "Warnings for Pattern Matching" algorithm.
//! Checks whether a set of patterns exhaustively covers all possible values.

use crate::{
    PatternMatrix, DeconstructedPattern, PatternType,
    Constructor, ConstructorSet, Witness,
};
use aria_lexer::Span;

/// Result of exhaustiveness checking
#[derive(Debug, Clone)]
pub struct ExhaustivenessResult {
    /// Whether the patterns are exhaustive
    pub is_exhaustive: bool,
    /// Missing patterns (witnesses to non-exhaustiveness)
    pub missing_patterns: Vec<Witness>,
    /// Redundant pattern arms (unreachable)
    pub redundant_arms: Vec<usize>,
}

impl ExhaustivenessResult {
    /// Create an exhaustive result
    pub fn exhaustive() -> Self {
        Self {
            is_exhaustive: true,
            missing_patterns: Vec::new(),
            redundant_arms: Vec::new(),
        }
    }

    /// Create a non-exhaustive result with missing patterns
    pub fn non_exhaustive(missing: Vec<Witness>) -> Self {
        Self {
            is_exhaustive: false,
            missing_patterns: missing,
            redundant_arms: Vec::new(),
        }
    }
}

/// Check exhaustiveness of a pattern matrix.
///
/// Returns information about whether patterns are exhaustive and what's missing.
pub fn check_exhaustiveness(matrix: &PatternMatrix, types: &[PatternType]) -> ExhaustivenessResult {
    // Create a wildcard pattern vector to test exhaustiveness
    let wildcard_row: Vec<DeconstructedPattern> = types.iter()
        .map(|_| DeconstructedPattern::wildcard(Span::new(0, 0)))
        .collect();

    // Check if wildcard is useful (meaning patterns aren't exhaustive)
    let (useful, witnesses) = is_useful_with_witnesses(matrix, &wildcard_row, types);

    if useful {
        // Wildcard is useful, so patterns are not exhaustive
        ExhaustivenessResult::non_exhaustive(witnesses)
    } else {
        // Check for redundant arms
        let redundant = find_redundant_arms(matrix, types);
        ExhaustivenessResult {
            is_exhaustive: true,
            missing_patterns: Vec::new(),
            redundant_arms: redundant,
        }
    }
}

/// Check if a pattern vector is useful (can match something not covered by matrix).
/// Returns (is_useful, witnesses).
fn is_useful_with_witnesses(
    matrix: &PatternMatrix,
    pattern_vec: &[DeconstructedPattern],
    types: &[PatternType],
) -> (bool, Vec<Witness>) {
    // Base case: empty pattern vector
    if pattern_vec.is_empty() {
        // Useful iff no rows in matrix (nothing matched yet)
        return (matrix.is_empty(), if matrix.is_empty() {
            vec![Witness::empty()]
        } else {
            Vec::new()
        });
    }

    // Get first column type
    let first_type = types.first().cloned().unwrap_or(PatternType::Unknown);
    let first_pattern = pattern_vec.first().unwrap();
    let rest_pattern = &pattern_vec[1..];
    let rest_types = if types.len() > 1 { &types[1..] } else { &[] };

    // Get constructor set for the type
    let ctor_set = ConstructorSet::for_type(&first_type);

    // Get head constructors from matrix
    let matrix_ctors = matrix.head_constructors();

    if first_pattern.ctor.is_wildcard() {
        // Case 1: Pattern starts with wildcard

        // Check if matrix has any wildcards in first column
        let has_wildcard = matrix.rows.iter()
            .any(|row| row.patterns.first().map_or(false, |p| p.ctor.is_wildcard()));

        if has_wildcard {
            // Matrix has a wildcard, so our wildcard pattern is not useful
            // (the matrix already matches everything)
            return (false, Vec::new());
        }

        if ctor_set.is_infinite {
            // For infinite types, check default matrix
            let default_matrix = matrix.default_matrix();
            let (useful, mut witnesses) = is_useful_with_witnesses(
                &default_matrix,
                rest_pattern,
                rest_types,
            );
            if useful {
                // Prepend wildcard to witnesses
                for w in &mut witnesses {
                    w.prepend(Constructor::Wildcard, Vec::new());
                }
            }
            (useful, witnesses)
        } else if matrix_ctors.is_empty() {
            // No constructors in matrix, check default
            let default_matrix = matrix.default_matrix();
            let (useful, mut witnesses) = is_useful_with_witnesses(
                &default_matrix,
                rest_pattern,
                rest_types,
            );
            if useful {
                for w in &mut witnesses {
                    // Use first constructor as witness
                    if let Some(ctor) = ctor_set.all_constructors().first() {
                        w.prepend(ctor.clone(), Vec::new());
                    }
                }
            }
            (useful, witnesses)
        } else {
            // Check if all constructors are covered
            let missing_ctors = ctor_set.missing(&matrix_ctors);

            if missing_ctors.is_empty() {
                // All constructors covered - check each specialization
                let mut all_witnesses = Vec::new();
                for ctor in ctor_set.all_constructors() {
                    let arity = ctor.arity(&first_type);
                    let specialized = matrix.specialize(ctor, arity);

                    // Expand pattern with wildcards for constructor fields
                    let mut expanded: Vec<DeconstructedPattern> = Vec::new();
                    for _ in 0..arity {
                        expanded.push(DeconstructedPattern::wildcard(Span::new(0, 0)));
                    }
                    expanded.extend(rest_pattern.to_vec());

                    let expanded_types = get_field_types(&first_type, ctor);
                    let mut full_types = expanded_types;
                    full_types.extend(rest_types.to_vec());

                    let (useful, mut witnesses) = is_useful_with_witnesses(
                        &specialized,
                        &expanded,
                        &full_types,
                    );

                    if useful {
                        for w in &mut witnesses {
                            // Construct witness with this constructor
                            let field_witnesses: Vec<Witness> = (0..arity)
                                .map(|_| w.pop_field().unwrap_or_else(Witness::empty))
                                .collect();
                            w.prepend(ctor.clone(), field_witnesses);
                        }
                        all_witnesses.extend(witnesses);
                    }
                }
                (!all_witnesses.is_empty(), all_witnesses)
            } else {
                // Some constructors not covered - useful with missing constructors
                let default_matrix = matrix.default_matrix();
                let (useful, mut witnesses) = is_useful_with_witnesses(
                    &default_matrix,
                    rest_pattern,
                    rest_types,
                );

                if !missing_ctors.is_empty() {
                    // We have missing constructors - create witnesses for them
                    let mut all_witnesses = Vec::new();
                    for ctor in missing_ctors {
                        let mut w = Witness::empty();
                        let arity = ctor.arity(&first_type);
                        let field_witnesses: Vec<Witness> = (0..arity)
                            .map(|_| Witness::wildcard())
                            .collect();
                        w.prepend(ctor, field_witnesses);
                        all_witnesses.push(w);
                    }
                    (true, all_witnesses)
                } else if useful {
                    // No missing constructors but default is useful
                    // This means wildcard patterns don't cover everything
                    for w in &mut witnesses {
                        // Prepend wildcard to witnesses from default
                        w.prepend(Constructor::Wildcard, Vec::new());
                    }
                    (true, witnesses)
                } else {
                    (false, Vec::new())
                }
            }
        }
    } else {
        // Case 2: Pattern starts with a specific constructor
        let ctor = &first_pattern.ctor;
        let arity = ctor.arity(&first_type);
        let specialized = matrix.specialize(ctor, arity);

        // Expand pattern with constructor's fields
        let mut expanded: Vec<DeconstructedPattern> = first_pattern.fields.clone();
        expanded.extend(rest_pattern.to_vec());

        let expanded_types = get_field_types(&first_type, ctor);
        let mut full_types = expanded_types;
        full_types.extend(rest_types.to_vec());

        let (useful, mut witnesses) = is_useful_with_witnesses(
            &specialized,
            &expanded,
            &full_types,
        );

        if useful {
            for w in &mut witnesses {
                let field_witnesses: Vec<Witness> = (0..arity)
                    .map(|_| w.pop_field().unwrap_or_else(Witness::empty))
                    .collect();
                w.prepend(ctor.clone(), field_witnesses);
            }
        }
        (useful, witnesses)
    }
}

/// Get field types for a constructor
fn get_field_types(ty: &PatternType, ctor: &Constructor) -> Vec<PatternType> {
    match (ty, ctor) {
        (PatternType::Tuple(types), Constructor::Tuple(_)) => types.clone(),
        (PatternType::Enum { variants, .. }, Constructor::Variant { index, .. }) => {
            variants.get(*index)
                .map(|v| v.fields.clone())
                .unwrap_or_default()
        }
        (PatternType::Struct { fields, .. }, Constructor::Struct { .. }) => {
            fields.iter().map(|(_, t)| t.clone()).collect()
        }
        (PatternType::Array(inner), Constructor::Array(n)) => {
            vec![inner.as_ref().clone(); *n]
        }
        _ => Vec::new(),
    }
}

/// Find redundant (unreachable) pattern arms
fn find_redundant_arms(matrix: &PatternMatrix, types: &[PatternType]) -> Vec<usize> {
    let mut redundant = Vec::new();
    let mut checked_matrix = PatternMatrix::new(matrix.column_types.clone());

    for row in &matrix.rows {
        // Check if this row is useful given the rows before it
        let (useful, _) = is_useful_with_witnesses(&checked_matrix, &row.patterns, types);
        if !useful {
            redundant.push(row.arm_index);
        }
        checked_matrix.push_row(row.clone());
    }

    redundant
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EnumVariant, PatternRow, DeconstructedPattern, Constructor};
    use smol_str::SmolStr;

    #[test]
    fn test_empty_matrix_useful() {
        let matrix = PatternMatrix::new(vec![PatternType::Bool]);
        let wildcard = vec![DeconstructedPattern::wildcard(Span::new(0, 0))];
        let (useful, _) = is_useful_with_witnesses(&matrix, &wildcard, &[PatternType::Bool]);
        assert!(useful);
    }

    #[test]
    fn test_full_bool_not_useful() {
        let mut matrix = PatternMatrix::new(vec![PatternType::Bool]);
        matrix.push_row(PatternRow {
            patterns: vec![DeconstructedPattern {
                ctor: Constructor::Bool(true),
                fields: Vec::new(),
                span: Span::new(0, 0),
            }],
            arm_index: 0,
        });
        matrix.push_row(PatternRow {
            patterns: vec![DeconstructedPattern {
                ctor: Constructor::Bool(false),
                fields: Vec::new(),
                span: Span::new(0, 0),
            }],
            arm_index: 1,
        });

        let wildcard = vec![DeconstructedPattern::wildcard(Span::new(0, 0))];
        let (useful, _) = is_useful_with_witnesses(&matrix, &wildcard, &[PatternType::Bool]);
        assert!(!useful);
    }

    #[test]
    fn test_redundant_arm_detection() {
        let mut matrix = PatternMatrix::new(vec![PatternType::Bool]);
        matrix.push_row(PatternRow {
            patterns: vec![DeconstructedPattern::wildcard(Span::new(0, 0))],
            arm_index: 0,
        });
        matrix.push_row(PatternRow {
            patterns: vec![DeconstructedPattern {
                ctor: Constructor::Bool(true),
                fields: Vec::new(),
                span: Span::new(0, 0),
            }],
            arm_index: 1, // This is redundant
        });

        let redundant = find_redundant_arms(&matrix, &[PatternType::Bool]);
        assert_eq!(redundant, vec![1]);
    }
}
