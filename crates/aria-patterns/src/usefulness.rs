//! Usefulness predicate for pattern matching.
//!
//! A pattern is "useful" with respect to a pattern matrix if it can match
//! values that no pattern in the matrix matches.

use crate::{PatternMatrix, DeconstructedPattern, PatternType, ConstructorSet};

/// Check if a pattern vector is useful with respect to a matrix.
///
/// This is a simplified interface to the full algorithm.
pub fn is_useful(matrix: &PatternMatrix, pattern: &[DeconstructedPattern], types: &[PatternType]) -> bool {
    is_useful_impl(matrix, pattern, types)
}

fn is_useful_impl(
    matrix: &PatternMatrix,
    pattern: &[DeconstructedPattern],
    types: &[PatternType],
) -> bool {
    // Base case: empty pattern
    if pattern.is_empty() {
        return matrix.is_empty();
    }

    let first_type = types.first().cloned().unwrap_or(PatternType::Unknown);
    let first_pattern = pattern.first().unwrap();
    let rest_pattern = &pattern[1..];
    let rest_types = if types.len() > 1 { &types[1..] } else { &[] };

    let ctor_set = ConstructorSet::for_type(&first_type);
    let matrix_ctors = matrix.head_constructors();

    if first_pattern.ctor.is_wildcard() {
        // Wildcard case
        if ctor_set.is_infinite || matrix_ctors.is_empty() {
            // Check default matrix
            let default_matrix = matrix.default_matrix();
            is_useful_impl(&default_matrix, rest_pattern, rest_types)
        } else if ctor_set.is_exhaustive(&matrix_ctors) {
            // All constructors covered - split into each
            ctor_set.all_constructors().iter().any(|ctor| {
                let arity = ctor.arity(&first_type);
                let specialized = matrix.specialize(ctor, arity);

                let mut expanded: Vec<DeconstructedPattern> = Vec::new();
                for _ in 0..arity {
                    expanded.push(DeconstructedPattern::wildcard(aria_lexer::Span::new(0, 0)));
                }
                expanded.extend(rest_pattern.to_vec());

                let field_types = get_field_types_for_ctor(&first_type, ctor);
                let mut full_types = field_types;
                full_types.extend(rest_types.to_vec());

                is_useful_impl(&specialized, &expanded, &full_types)
            })
        } else {
            // Some constructors not covered - pattern is useful
            true
        }
    } else {
        // Specific constructor case
        let ctor = &first_pattern.ctor;
        let arity = ctor.arity(&first_type);
        let specialized = matrix.specialize(ctor, arity);

        let mut expanded: Vec<DeconstructedPattern> = first_pattern.fields.clone();
        expanded.extend(rest_pattern.to_vec());

        let field_types = get_field_types_for_ctor(&first_type, ctor);
        let mut full_types = field_types;
        full_types.extend(rest_types.to_vec());

        is_useful_impl(&specialized, &expanded, &full_types)
    }
}

fn get_field_types_for_ctor(ty: &PatternType, ctor: &crate::Constructor) -> Vec<PatternType> {
    match ty {
        PatternType::Tuple(types) => types.clone(),
        PatternType::Enum { variants, .. } => {
            if let crate::Constructor::Variant { index, .. } = ctor {
                variants.get(*index)
                    .map(|v| v.fields.clone())
                    .unwrap_or_default()
            } else {
                Vec::new()
            }
        }
        PatternType::Struct { fields, .. } => {
            fields.iter().map(|(_, t)| t.clone()).collect()
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PatternRow, Constructor};
    use aria_lexer::Span;

    #[test]
    fn test_useful_in_empty_matrix() {
        let matrix = PatternMatrix::new(vec![PatternType::Int]);
        let pattern = vec![DeconstructedPattern::wildcard(Span::new(0, 0))];
        assert!(is_useful(&matrix, &pattern, &[PatternType::Int]));
    }

    #[test]
    fn test_not_useful_after_wildcard() {
        let mut matrix = PatternMatrix::new(vec![PatternType::Int]);
        matrix.push_row(PatternRow {
            patterns: vec![DeconstructedPattern::wildcard(Span::new(0, 0))],
            arm_index: 0,
        });

        let pattern = vec![DeconstructedPattern {
            ctor: Constructor::Int(42),
            fields: Vec::new(),
            span: Span::new(0, 0),
        }];

        assert!(!is_useful(&matrix, &pattern, &[PatternType::Int]));
    }
}
