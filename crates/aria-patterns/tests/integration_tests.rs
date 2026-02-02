//! Integration tests for pattern matching enhancements

use aria_patterns::*;
use aria_lexer::Span;
use smol_str::SmolStr;

#[test]
fn test_exhaustiveness_bool() {
    let ty = PatternType::Bool;
    let mut matrix = PatternMatrix::new(vec![ty.clone()]);

    // Add both true and false
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

    let result = check_exhaustiveness(&matrix, &[ty]);
    assert!(result.is_exhaustive, "Bool match should be exhaustive");
    assert!(result.missing_patterns.is_empty());
}

#[test]
fn test_non_exhaustiveness_bool() {
    let ty = PatternType::Bool;
    let mut matrix = PatternMatrix::new(vec![ty.clone()]);

    // Only true - missing false
    matrix.push_row(PatternRow {
        patterns: vec![DeconstructedPattern {
            ctor: Constructor::Bool(true),
            fields: Vec::new(),
            span: Span::new(0, 0),
        }],
        arm_index: 0,
    });

    let result = check_exhaustiveness(&matrix, &[ty]);
    assert!(!result.is_exhaustive, "Incomplete Bool match should not be exhaustive");
    assert_eq!(result.missing_patterns.len(), 1);

    let witness = &result.missing_patterns[0];
    assert_eq!(witness.to_pattern_string(), "false");
}

#[test]
fn test_unreachable_pattern_detection() {
    let ty = PatternType::Bool;
    let mut matrix = PatternMatrix::new(vec![ty.clone()]);

    // Wildcard first - makes everything else unreachable
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
        arm_index: 1,
    });

    let result = check_exhaustiveness(&matrix, &[ty]);
    assert!(result.is_exhaustive);
    assert_eq!(result.redundant_arms.len(), 1);
    assert_eq!(result.redundant_arms[0], 1);
}

#[test]
fn test_enum_exhaustiveness() {
    let ty = PatternType::Enum {
        name: SmolStr::new("Option"),
        variants: vec![
            EnumVariant {
                name: SmolStr::new("Some"),
                fields: vec![PatternType::Int],
            },
            EnumVariant {
                name: SmolStr::new("None"),
                fields: vec![],
            },
        ],
    };

    let mut matrix = PatternMatrix::new(vec![ty.clone()]);

    // Match Some(_)
    matrix.push_row(PatternRow {
        patterns: vec![DeconstructedPattern {
            ctor: Constructor::Variant {
                name: SmolStr::new("Some"),
                index: 0,
            },
            fields: vec![DeconstructedPattern::wildcard(Span::new(0, 0))],
            span: Span::new(0, 0),
        }],
        arm_index: 0,
    });

    // Match None
    matrix.push_row(PatternRow {
        patterns: vec![DeconstructedPattern {
            ctor: Constructor::Variant {
                name: SmolStr::new("None"),
                index: 1,
            },
            fields: vec![],
            span: Span::new(0, 0),
        }],
        arm_index: 1,
    });

    let result = check_exhaustiveness(&matrix, &[ty]);
    assert!(result.is_exhaustive);
}

#[test]
fn test_nested_pattern_exhaustiveness() {
    let inner_enum = PatternType::Enum {
        name: SmolStr::new("Bool"),
        variants: vec![
            EnumVariant {
                name: SmolStr::new("True"),
                fields: vec![],
            },
            EnumVariant {
                name: SmolStr::new("False"),
                fields: vec![],
            },
        ],
    };

    let outer_enum = PatternType::Enum {
        name: SmolStr::new("Wrapper"),
        variants: vec![
            EnumVariant {
                name: SmolStr::new("Wrap"),
                fields: vec![inner_enum.clone()],
            },
        ],
    };

    let mut matrix = PatternMatrix::new(vec![outer_enum.clone()]);

    // Wrap(True)
    matrix.push_row(PatternRow {
        patterns: vec![DeconstructedPattern {
            ctor: Constructor::Variant {
                name: SmolStr::new("Wrap"),
                index: 0,
            },
            fields: vec![DeconstructedPattern {
                ctor: Constructor::Variant {
                    name: SmolStr::new("True"),
                    index: 0,
                },
                fields: vec![],
                span: Span::new(0, 0),
            }],
            span: Span::new(0, 0),
        }],
        arm_index: 0,
    });

    // Missing Wrap(False) - should not be exhaustive
    let result = check_exhaustiveness(&matrix, &[outer_enum]);
    assert!(!result.is_exhaustive);
    assert!(!result.missing_patterns.is_empty());
}

#[test]
fn test_decision_tree_compilation() {
    let ty = PatternType::Bool;
    let mut matrix = PatternMatrix::new(vec![ty.clone()]);

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

    let tree = compile_decision_tree(&matrix, &[ty]);

    // Should create a switch with two cases
    match tree {
        DecisionTree::Switch { cases, default, .. } => {
            assert_eq!(cases.len(), 2);
            assert!(default.is_none()); // Exhaustive, no default needed
        }
        _ => panic!("Expected Switch node"),
    }
}

#[test]
fn test_decision_tree_optimization() {
    // Create a tree where all branches lead to same arm
    let tree = DecisionTree::Switch {
        place: TestPlace::root(),
        ty: PatternType::Bool,
        cases: vec![
            decision_tree::SwitchCase {
                constructor: Constructor::Bool(true),
                fields: Vec::new(),
                subtree: DecisionTree::Leaf {
                    arm_index: 0,
                    span: Span::new(0, 0),
                },
            },
            decision_tree::SwitchCase {
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

    // Should optimize to single leaf
    match optimized {
        DecisionTree::Leaf { arm_index, .. } => {
            assert_eq!(arm_index, 0);
        }
        _ => panic!("Expected optimization to collapse to leaf"),
    }
}

#[test]
fn test_tuple_pattern_exhaustiveness() {
    let ty = PatternType::Tuple(vec![PatternType::Bool, PatternType::Bool]);
    let mut matrix = PatternMatrix::new(vec![ty.clone()]);

    // (true, _)
    matrix.push_row(PatternRow {
        patterns: vec![DeconstructedPattern {
            ctor: Constructor::Tuple(2),
            fields: vec![
                DeconstructedPattern {
                    ctor: Constructor::Bool(true),
                    fields: Vec::new(),
                    span: Span::new(0, 0),
                },
                DeconstructedPattern::wildcard(Span::new(0, 0)),
            ],
            span: Span::new(0, 0),
        }],
        arm_index: 0,
    });

    // (false, _)
    matrix.push_row(PatternRow {
        patterns: vec![DeconstructedPattern {
            ctor: Constructor::Tuple(2),
            fields: vec![
                DeconstructedPattern {
                    ctor: Constructor::Bool(false),
                    fields: Vec::new(),
                    span: Span::new(0, 0),
                },
                DeconstructedPattern::wildcard(Span::new(0, 0)),
            ],
            span: Span::new(0, 0),
        }],
        arm_index: 1,
    });

    let result = check_exhaustiveness(&matrix, &[ty]);
    assert!(result.is_exhaustive);
}

#[test]
fn test_infinite_type_with_wildcard() {
    let ty = PatternType::Int;
    let mut matrix = PatternMatrix::new(vec![ty.clone()]);

    // Just a wildcard - should be exhaustive for infinite types
    matrix.push_row(PatternRow {
        patterns: vec![DeconstructedPattern::wildcard(Span::new(0, 0))],
        arm_index: 0,
    });

    let result = check_exhaustiveness(&matrix, &[ty]);
    assert!(result.is_exhaustive);
}

#[test]
fn test_infinite_type_without_wildcard() {
    let ty = PatternType::Int;
    let mut matrix = PatternMatrix::new(vec![ty.clone()]);

    // Only specific values - never exhaustive without wildcard
    matrix.push_row(PatternRow {
        patterns: vec![DeconstructedPattern {
            ctor: Constructor::Int(1),
            fields: Vec::new(),
            span: Span::new(0, 0),
        }],
        arm_index: 0,
    });

    matrix.push_row(PatternRow {
        patterns: vec![DeconstructedPattern {
            ctor: Constructor::Int(2),
            fields: Vec::new(),
            span: Span::new(0, 0),
        }],
        arm_index: 1,
    });

    let result = check_exhaustiveness(&matrix, &[ty]);
    assert!(!result.is_exhaustive);
}
