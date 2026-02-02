//! Tests for pattern guard type checking
//!
//! This module tests that:
//! 1. Guard expressions are type-checked as Bool
//! 2. Pattern-bound variables are available in guard expressions
//! 3. Various pattern types properly bind their variables

use aria_ast as ast;
use aria_ast::Span;
use aria_types::{Type, TypeChecker, TypeEnv};

// =========================================================================
// Pattern Guard Tests
// =========================================================================

#[test]
fn test_match_with_simple_guard() {
    // Test: match x { n if n > 0 => n, _ => 0 }
    // The guard `n > 0` should type-check as Bool
    let mut checker = TypeChecker::new();
    let mut env = TypeEnv::new();
    env.define_var("x".to_string(), Type::Int);

    // Build the match expression
    let scrutinee = ast::Expr::new(ast::ExprKind::Ident("x".into()), Span::dummy());

    // Pattern: n (binds variable n to the matched value)
    let pattern1 = ast::Pattern {
        kind: ast::PatternKind::Ident("n".into()),
        span: Span::dummy(),
    };

    // Guard: n > 0 (should be Bool)
    let guard1 = ast::Expr::new(
        ast::ExprKind::Binary {
            op: ast::BinaryOp::Gt,
            left: Box::new(ast::Expr::new(
                ast::ExprKind::Ident("n".into()),
                Span::dummy(),
            )),
            right: Box::new(ast::Expr::new(
                ast::ExprKind::Integer("0".into()),
                Span::dummy(),
            )),
        },
        Span::dummy(),
    );

    // Body: n
    let body1 = ast::Expr::new(ast::ExprKind::Ident("n".into()), Span::dummy());

    let arm1 = ast::MatchArm {
        pattern: pattern1,
        guard: Some(guard1),
        body: ast::MatchArmBody::Expr(body1),
        span: Span::dummy(),
    };

    // Wildcard arm: _ => 0
    let pattern2 = ast::Pattern {
        kind: ast::PatternKind::Wildcard,
        span: Span::dummy(),
    };
    let body2 = ast::Expr::new(ast::ExprKind::Integer("0".into()), Span::dummy());
    let arm2 = ast::MatchArm {
        pattern: pattern2,
        guard: None,
        body: ast::MatchArmBody::Expr(body2),
        span: Span::dummy(),
    };

    let match_expr = ast::Expr::new(
        ast::ExprKind::Match {
            scrutinee: Box::new(scrutinee),
            arms: vec![arm1, arm2],
        },
        Span::dummy(),
    );

    let ty = checker.infer_expr(&match_expr, &env).unwrap();
    assert_eq!(ty, Type::Int);
}

#[test]
fn test_match_guard_with_tuple_pattern() {
    // Test: match pair { (a, b) if a > b => true, _ => false }
    let mut checker = TypeChecker::new();
    let mut env = TypeEnv::new();
    env.define_var(
        "pair".to_string(),
        Type::Tuple(vec![Type::Int, Type::Int]),
    );

    let scrutinee = ast::Expr::new(ast::ExprKind::Ident("pair".into()), Span::dummy());

    // Pattern: (a, b)
    let pattern1 = ast::Pattern {
        kind: ast::PatternKind::Tuple(vec![
            ast::Pattern {
                kind: ast::PatternKind::Ident("a".into()),
                span: Span::dummy(),
            },
            ast::Pattern {
                kind: ast::PatternKind::Ident("b".into()),
                span: Span::dummy(),
            },
        ]),
        span: Span::dummy(),
    };

    // Guard: a > b (both should be Int from the tuple pattern)
    let guard1 = ast::Expr::new(
        ast::ExprKind::Binary {
            op: ast::BinaryOp::Gt,
            left: Box::new(ast::Expr::new(
                ast::ExprKind::Ident("a".into()),
                Span::dummy(),
            )),
            right: Box::new(ast::Expr::new(
                ast::ExprKind::Ident("b".into()),
                Span::dummy(),
            )),
        },
        Span::dummy(),
    );

    let arm1 = ast::MatchArm {
        pattern: pattern1,
        guard: Some(guard1),
        body: ast::MatchArmBody::Expr(ast::Expr::new(ast::ExprKind::Bool(true), Span::dummy())),
        span: Span::dummy(),
    };

    // Wildcard arm
    let arm2 = ast::MatchArm {
        pattern: ast::Pattern {
            kind: ast::PatternKind::Wildcard,
            span: Span::dummy(),
        },
        guard: None,
        body: ast::MatchArmBody::Expr(ast::Expr::new(ast::ExprKind::Bool(false), Span::dummy())),
        span: Span::dummy(),
    };

    let match_expr = ast::Expr::new(
        ast::ExprKind::Match {
            scrutinee: Box::new(scrutinee),
            arms: vec![arm1, arm2],
        },
        Span::dummy(),
    );

    let ty = checker.infer_expr(&match_expr, &env).unwrap();
    assert_eq!(ty, Type::Bool);
}

#[test]
fn test_match_guard_must_be_bool() {
    // Test that non-Bool guards produce an error
    // match x { n if n => ... } where n is Int should fail
    let mut checker = TypeChecker::new();
    let mut env = TypeEnv::new();
    env.define_var("x".to_string(), Type::Int);

    let scrutinee = ast::Expr::new(ast::ExprKind::Ident("x".into()), Span::dummy());

    // Pattern: n
    let pattern1 = ast::Pattern {
        kind: ast::PatternKind::Ident("n".into()),
        span: Span::dummy(),
    };

    // Invalid guard: n (Int, not Bool)
    let guard1 = ast::Expr::new(ast::ExprKind::Ident("n".into()), Span::dummy());

    let arm1 = ast::MatchArm {
        pattern: pattern1,
        guard: Some(guard1),
        body: ast::MatchArmBody::Expr(ast::Expr::new(
            ast::ExprKind::Integer("0".into()),
            Span::dummy(),
        )),
        span: Span::dummy(),
    };

    // Wildcard arm
    let arm2 = ast::MatchArm {
        pattern: ast::Pattern {
            kind: ast::PatternKind::Wildcard,
            span: Span::dummy(),
        },
        guard: None,
        body: ast::MatchArmBody::Expr(ast::Expr::new(
            ast::ExprKind::Integer("0".into()),
            Span::dummy(),
        )),
        span: Span::dummy(),
    };

    let match_expr = ast::Expr::new(
        ast::ExprKind::Match {
            scrutinee: Box::new(scrutinee),
            arms: vec![arm1, arm2],
        },
        Span::dummy(),
    );

    // Should fail because guard is Int, not Bool
    let result = checker.infer_expr(&match_expr, &env);
    assert!(result.is_err());
}

#[test]
fn test_match_guard_with_binding_pattern() {
    // Test: match x { all @ n if all > 0 => all, _ => 0 }
    // Both `all` and `n` should be available in the guard
    let mut checker = TypeChecker::new();
    let mut env = TypeEnv::new();
    env.define_var("x".to_string(), Type::Int);

    let scrutinee = ast::Expr::new(ast::ExprKind::Ident("x".into()), Span::dummy());

    // Pattern: all @ n (binding pattern)
    let inner_pattern = ast::Pattern {
        kind: ast::PatternKind::Ident("n".into()),
        span: Span::dummy(),
    };
    let pattern1 = ast::Pattern {
        kind: ast::PatternKind::Binding {
            name: ast::Spanned::dummy("all".into()),
            pattern: Box::new(inner_pattern),
        },
        span: Span::dummy(),
    };

    // Guard: all > 0 (all should be bound to Int)
    let guard1 = ast::Expr::new(
        ast::ExprKind::Binary {
            op: ast::BinaryOp::Gt,
            left: Box::new(ast::Expr::new(
                ast::ExprKind::Ident("all".into()),
                Span::dummy(),
            )),
            right: Box::new(ast::Expr::new(
                ast::ExprKind::Integer("0".into()),
                Span::dummy(),
            )),
        },
        Span::dummy(),
    );

    let arm1 = ast::MatchArm {
        pattern: pattern1,
        guard: Some(guard1),
        body: ast::MatchArmBody::Expr(ast::Expr::new(
            ast::ExprKind::Ident("all".into()),
            Span::dummy(),
        )),
        span: Span::dummy(),
    };

    // Wildcard arm
    let arm2 = ast::MatchArm {
        pattern: ast::Pattern {
            kind: ast::PatternKind::Wildcard,
            span: Span::dummy(),
        },
        guard: None,
        body: ast::MatchArmBody::Expr(ast::Expr::new(
            ast::ExprKind::Integer("0".into()),
            Span::dummy(),
        )),
        span: Span::dummy(),
    };

    let match_expr = ast::Expr::new(
        ast::ExprKind::Match {
            scrutinee: Box::new(scrutinee),
            arms: vec![arm1, arm2],
        },
        Span::dummy(),
    );

    let ty = checker.infer_expr(&match_expr, &env).unwrap();
    assert_eq!(ty, Type::Int);
}

#[test]
fn test_match_guard_with_array_pattern() {
    // Test: match arr { [first, ...rest] if first > 0 => first, _ => 0 }
    let mut checker = TypeChecker::new();
    let mut env = TypeEnv::new();
    env.define_var("arr".to_string(), Type::Array(Box::new(Type::Int)));

    let scrutinee = ast::Expr::new(ast::ExprKind::Ident("arr".into()), Span::dummy());

    // Pattern: [first, ...rest]
    let rest_pattern = ast::Pattern {
        kind: ast::PatternKind::Rest(Some(ast::Spanned::dummy("rest".into()))),
        span: Span::dummy(),
    };
    let pattern1 = ast::Pattern {
        kind: ast::PatternKind::Array {
            elements: vec![ast::Pattern {
                kind: ast::PatternKind::Ident("first".into()),
                span: Span::dummy(),
            }],
            rest: Some(Box::new(rest_pattern)),
        },
        span: Span::dummy(),
    };

    // Guard: first > 0
    let guard1 = ast::Expr::new(
        ast::ExprKind::Binary {
            op: ast::BinaryOp::Gt,
            left: Box::new(ast::Expr::new(
                ast::ExprKind::Ident("first".into()),
                Span::dummy(),
            )),
            right: Box::new(ast::Expr::new(
                ast::ExprKind::Integer("0".into()),
                Span::dummy(),
            )),
        },
        Span::dummy(),
    );

    let arm1 = ast::MatchArm {
        pattern: pattern1,
        guard: Some(guard1),
        body: ast::MatchArmBody::Expr(ast::Expr::new(
            ast::ExprKind::Ident("first".into()),
            Span::dummy(),
        )),
        span: Span::dummy(),
    };

    // Wildcard arm
    let arm2 = ast::MatchArm {
        pattern: ast::Pattern {
            kind: ast::PatternKind::Wildcard,
            span: Span::dummy(),
        },
        guard: None,
        body: ast::MatchArmBody::Expr(ast::Expr::new(
            ast::ExprKind::Integer("0".into()),
            Span::dummy(),
        )),
        span: Span::dummy(),
    };

    let match_expr = ast::Expr::new(
        ast::ExprKind::Match {
            scrutinee: Box::new(scrutinee),
            arms: vec![arm1, arm2],
        },
        Span::dummy(),
    );

    let ty = checker.infer_expr(&match_expr, &env).unwrap();
    assert_eq!(ty, Type::Int);
}

#[test]
fn test_match_guard_with_logical_condition() {
    // Test: match x { n if n > 0 && n < 100 => n, _ => 0 }
    let mut checker = TypeChecker::new();
    let mut env = TypeEnv::new();
    env.define_var("x".to_string(), Type::Int);

    let scrutinee = ast::Expr::new(ast::ExprKind::Ident("x".into()), Span::dummy());

    // Pattern: n
    let pattern1 = ast::Pattern {
        kind: ast::PatternKind::Ident("n".into()),
        span: Span::dummy(),
    };

    // Guard: n > 0 && n < 100
    let left_cmp = ast::Expr::new(
        ast::ExprKind::Binary {
            op: ast::BinaryOp::Gt,
            left: Box::new(ast::Expr::new(
                ast::ExprKind::Ident("n".into()),
                Span::dummy(),
            )),
            right: Box::new(ast::Expr::new(
                ast::ExprKind::Integer("0".into()),
                Span::dummy(),
            )),
        },
        Span::dummy(),
    );
    let right_cmp = ast::Expr::new(
        ast::ExprKind::Binary {
            op: ast::BinaryOp::Lt,
            left: Box::new(ast::Expr::new(
                ast::ExprKind::Ident("n".into()),
                Span::dummy(),
            )),
            right: Box::new(ast::Expr::new(
                ast::ExprKind::Integer("100".into()),
                Span::dummy(),
            )),
        },
        Span::dummy(),
    );
    let guard1 = ast::Expr::new(
        ast::ExprKind::Binary {
            op: ast::BinaryOp::And,
            left: Box::new(left_cmp),
            right: Box::new(right_cmp),
        },
        Span::dummy(),
    );

    let arm1 = ast::MatchArm {
        pattern: pattern1,
        guard: Some(guard1),
        body: ast::MatchArmBody::Expr(ast::Expr::new(
            ast::ExprKind::Ident("n".into()),
            Span::dummy(),
        )),
        span: Span::dummy(),
    };

    // Wildcard arm
    let arm2 = ast::MatchArm {
        pattern: ast::Pattern {
            kind: ast::PatternKind::Wildcard,
            span: Span::dummy(),
        },
        guard: None,
        body: ast::MatchArmBody::Expr(ast::Expr::new(
            ast::ExprKind::Integer("0".into()),
            Span::dummy(),
        )),
        span: Span::dummy(),
    };

    let match_expr = ast::Expr::new(
        ast::ExprKind::Match {
            scrutinee: Box::new(scrutinee),
            arms: vec![arm1, arm2],
        },
        Span::dummy(),
    );

    let ty = checker.infer_expr(&match_expr, &env).unwrap();
    assert_eq!(ty, Type::Int);
}

#[test]
fn test_bind_pattern_binding() {
    // Test that Binding pattern (x @ pattern) properly binds both x and inner pattern vars
    let mut checker = TypeChecker::new();
    let mut env = TypeEnv::new();

    let inner = ast::Pattern {
        kind: ast::PatternKind::Ident("inner".into()),
        span: Span::dummy(),
    };
    let binding_pattern = ast::Pattern {
        kind: ast::PatternKind::Binding {
            name: ast::Spanned::dummy("outer".into()),
            pattern: Box::new(inner),
        },
        span: Span::dummy(),
    };

    checker
        .bind_pattern(&binding_pattern, &Type::Int, &mut env)
        .unwrap();

    // Both outer and inner should be bound to Int
    assert_eq!(env.lookup_var("outer"), Some(&Type::Int));
    assert_eq!(env.lookup_var("inner"), Some(&Type::Int));
}

#[test]
fn test_bind_pattern_or() {
    // Test that Or pattern binds variables from first alternative
    let mut checker = TypeChecker::new();
    let mut env = TypeEnv::new();

    let or_pattern = ast::Pattern {
        kind: ast::PatternKind::Or(vec![
            ast::Pattern {
                kind: ast::PatternKind::Ident("x".into()),
                span: Span::dummy(),
            },
            ast::Pattern {
                kind: ast::PatternKind::Ident("y".into()),
                span: Span::dummy(),
            },
        ]),
        span: Span::dummy(),
    };

    checker
        .bind_pattern(&or_pattern, &Type::Int, &mut env)
        .unwrap();

    // First alternative's variable should be bound
    assert_eq!(env.lookup_var("x"), Some(&Type::Int));
}

#[test]
fn test_bind_pattern_rest() {
    // Test that Rest pattern with name binds the variable
    let mut checker = TypeChecker::new();
    let mut env = TypeEnv::new();

    let rest_pattern = ast::Pattern {
        kind: ast::PatternKind::Rest(Some(ast::Spanned::dummy("rest".into()))),
        span: Span::dummy(),
    };

    checker
        .bind_pattern(
            &rest_pattern,
            &Type::Array(Box::new(Type::Int)),
            &mut env,
        )
        .unwrap();

    // rest should be bound to the array type
    assert_eq!(
        env.lookup_var("rest"),
        Some(&Type::Array(Box::new(Type::Int)))
    );
}
