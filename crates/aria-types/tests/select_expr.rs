//! Tests for select expression type checking
//!
//! This module tests:
//! 1. Select with receive arms (pattern binding)
//! 2. Select with send arms (channel type validation)
//! 3. Select with default arm
//! 4. Multiple default arm validation
//! 5. Select arm result type compatibility
//! 6. Type inference through select expressions

use aria_ast::{self as ast, Span};
use aria_types::{Type, TypeChecker, TypeEnv, TypeError, TypeInference};

// ============================================================================
// Helper functions to construct AST nodes
// ============================================================================

fn make_channel_expr(name: &str) -> ast::Expr {
    ast::Expr::new(
        ast::ExprKind::Ident(name.into()),
        Span::dummy(),
    )
}

fn make_int_expr(value: i64) -> ast::Expr {
    ast::Expr::new(
        ast::ExprKind::Integer(value.to_string().into()),
        Span::dummy(),
    )
}

fn make_string_expr(value: &str) -> ast::Expr {
    ast::Expr::new(
        ast::ExprKind::String(value.into()),
        Span::dummy(),
    )
}

fn make_bool_expr(value: bool) -> ast::Expr {
    ast::Expr::new(
        ast::ExprKind::Bool(value),
        Span::dummy(),
    )
}

fn make_ident_pattern(name: &str) -> ast::Pattern {
    ast::Pattern {
        kind: ast::PatternKind::Ident(name.into()),
        span: Span::dummy(),
    }
}

fn make_receive_arm(pattern: Option<ast::Pattern>, channel: ast::Expr, body: ast::Expr) -> ast::SelectArm {
    ast::SelectArm {
        kind: ast::SelectArmKind::Receive {
            pattern,
            channel: Box::new(channel),
        },
        body,
        span: Span::dummy(),
    }
}

fn make_send_arm(channel: ast::Expr, value: ast::Expr, body: ast::Expr) -> ast::SelectArm {
    ast::SelectArm {
        kind: ast::SelectArmKind::Send {
            channel: Box::new(channel),
            value: Box::new(value),
        },
        body,
        span: Span::dummy(),
    }
}

fn make_default_arm(body: ast::Expr) -> ast::SelectArm {
    ast::SelectArm {
        kind: ast::SelectArmKind::Default,
        body,
        span: Span::dummy(),
    }
}

fn make_select_expr(arms: Vec<ast::SelectArm>) -> ast::Expr {
    ast::Expr::new(
        ast::ExprKind::Select(arms),
        Span::dummy(),
    )
}

// ============================================================================
// Basic Select Expression Tests
// ============================================================================

#[test]
fn test_select_empty_returns_unit() {
    let mut checker = TypeChecker::new();
    let env = TypeEnv::new();

    let select_expr = make_select_expr(vec![]);
    let ty = checker.infer_expr(&select_expr, &env).unwrap();

    assert_eq!(ty, Type::Unit);
}

#[test]
fn test_select_single_receive_arm() {
    let mut checker = TypeChecker::new();
    let mut env = TypeEnv::new();

    // Define a channel of Int
    env.define_var("ch".to_string(), Type::Channel(Box::new(Type::Int)));

    // select { msg = <-ch => msg + 1 }
    let select_expr = make_select_expr(vec![
        make_receive_arm(
            Some(make_ident_pattern("msg")),
            make_channel_expr("ch"),
            ast::Expr::new(
                ast::ExprKind::Binary {
                    op: ast::BinaryOp::Add,
                    left: Box::new(ast::Expr::new(
                        ast::ExprKind::Ident("msg".into()),
                        Span::dummy(),
                    )),
                    right: Box::new(make_int_expr(1)),
                },
                Span::dummy(),
            ),
        ),
    ]);

    let ty = checker.infer_expr(&select_expr, &env).unwrap();
    assert_eq!(ty, Type::Int);
}

#[test]
fn test_select_single_send_arm() {
    let mut checker = TypeChecker::new();
    let mut env = TypeEnv::new();

    // Define a channel of Int
    env.define_var("ch".to_string(), Type::Channel(Box::new(Type::Int)));

    // select { ch <- 42 => true }
    let select_expr = make_select_expr(vec![
        make_send_arm(
            make_channel_expr("ch"),
            make_int_expr(42),
            make_bool_expr(true),
        ),
    ]);

    let ty = checker.infer_expr(&select_expr, &env).unwrap();
    assert_eq!(ty, Type::Bool);
}

#[test]
fn test_select_with_default_arm() {
    let mut checker = TypeChecker::new();
    let mut env = TypeEnv::new();

    // Define a channel of String
    env.define_var("ch".to_string(), Type::Channel(Box::new(Type::String)));

    // select { <-ch => "received", default => "nothing" }
    let select_expr = make_select_expr(vec![
        make_receive_arm(
            None,
            make_channel_expr("ch"),
            make_string_expr("received"),
        ),
        make_default_arm(make_string_expr("nothing")),
    ]);

    let ty = checker.infer_expr(&select_expr, &env).unwrap();
    assert_eq!(ty, Type::String);
}

// ============================================================================
// Multiple Arms Type Compatibility Tests
// ============================================================================

#[test]
fn test_select_multiple_arms_same_type() {
    let mut checker = TypeChecker::new();
    let mut env = TypeEnv::new();

    env.define_var("ch1".to_string(), Type::Channel(Box::new(Type::Int)));
    env.define_var("ch2".to_string(), Type::Channel(Box::new(Type::String)));

    // select { <-ch1 => 1, <-ch2 => 2, default => 3 }
    let select_expr = make_select_expr(vec![
        make_receive_arm(
            None,
            make_channel_expr("ch1"),
            make_int_expr(1),
        ),
        make_receive_arm(
            None,
            make_channel_expr("ch2"),
            make_int_expr(2),
        ),
        make_default_arm(make_int_expr(3)),
    ]);

    let ty = checker.infer_expr(&select_expr, &env).unwrap();
    assert_eq!(ty, Type::Int);
}

#[test]
fn test_select_arms_type_mismatch() {
    let mut checker = TypeChecker::new();
    let mut env = TypeEnv::new();

    env.define_var("ch".to_string(), Type::Channel(Box::new(Type::Int)));

    // select { <-ch => 1, default => "error" }
    let select_expr = make_select_expr(vec![
        make_receive_arm(
            None,
            make_channel_expr("ch"),
            make_int_expr(1),
        ),
        make_default_arm(make_string_expr("error")),
    ]);

    let result = checker.infer_expr(&select_expr, &env);
    assert!(result.is_err());

    // Should be a SelectArmTypeMismatch error
    match result.unwrap_err() {
        TypeError::SelectArmTypeMismatch { expected, found, arm_index, .. } => {
            assert_eq!(expected, "Int");
            assert_eq!(found, "String");
            assert_eq!(arm_index, 1);
        }
        err => panic!("Expected SelectArmTypeMismatch, got {:?}", err),
    }
}

// ============================================================================
// Multiple Default Arms Validation
// ============================================================================

#[test]
fn test_select_multiple_default_arms_error() {
    let mut checker = TypeChecker::new();
    let env = TypeEnv::new();

    // select { default => 1, default => 2 }
    let select_expr = make_select_expr(vec![
        make_default_arm(make_int_expr(1)),
        make_default_arm(make_int_expr(2)),
    ]);

    let result = checker.infer_expr(&select_expr, &env);
    assert!(result.is_err());

    match result.unwrap_err() {
        TypeError::MultipleDefaultArms { .. } => {
            // Expected error
        }
        err => panic!("Expected MultipleDefaultArms, got {:?}", err),
    }
}

// ============================================================================
// Channel Type Validation Tests
// ============================================================================

#[test]
fn test_select_receive_on_non_channel_error() {
    let mut checker = TypeChecker::new();
    let mut env = TypeEnv::new();

    // Define a non-channel variable
    env.define_var("not_channel".to_string(), Type::Int);

    // select { <-not_channel => 1 }
    let select_expr = make_select_expr(vec![
        make_receive_arm(
            None,
            make_channel_expr("not_channel"),
            make_int_expr(1),
        ),
    ]);

    let result = checker.infer_expr(&select_expr, &env);
    assert!(result.is_err());

    match result.unwrap_err() {
        TypeError::ReceiveOnNonChannel { found, .. } => {
            assert_eq!(found, "Int");
        }
        err => panic!("Expected ReceiveOnNonChannel, got {:?}", err),
    }
}

#[test]
fn test_select_send_on_non_channel_error() {
    let mut checker = TypeChecker::new();
    let mut env = TypeEnv::new();

    // Define a non-channel variable
    env.define_var("not_channel".to_string(), Type::String);

    // select { not_channel <- 42 => 1 }
    let select_expr = make_select_expr(vec![
        make_send_arm(
            make_channel_expr("not_channel"),
            make_int_expr(42),
            make_int_expr(1),
        ),
    ]);

    let result = checker.infer_expr(&select_expr, &env);
    assert!(result.is_err());

    match result.unwrap_err() {
        TypeError::SendOnNonChannel { found, .. } => {
            assert_eq!(found, "String");
        }
        err => panic!("Expected SendOnNonChannel, got {:?}", err),
    }
}

#[test]
fn test_select_send_type_mismatch() {
    let mut checker = TypeChecker::new();
    let mut env = TypeEnv::new();

    // Define a channel of Int
    env.define_var("ch".to_string(), Type::Channel(Box::new(Type::Int)));

    // select { ch <- "wrong type" => 1 }
    let select_expr = make_select_expr(vec![
        make_send_arm(
            make_channel_expr("ch"),
            make_string_expr("wrong type"),
            make_int_expr(1),
        ),
    ]);

    let result = checker.infer_expr(&select_expr, &env);
    assert!(result.is_err());
}

// ============================================================================
// Pattern Binding Tests
// ============================================================================

#[test]
fn test_select_receive_pattern_binds_channel_type() {
    let mut checker = TypeChecker::new();
    let mut env = TypeEnv::new();

    // Define a channel of String
    env.define_var("ch".to_string(), Type::Channel(Box::new(Type::String)));

    // select { msg = <-ch => msg.len() }
    // Note: We simulate this by using the bound variable in the body
    let select_expr = make_select_expr(vec![
        make_receive_arm(
            Some(make_ident_pattern("msg")),
            make_channel_expr("ch"),
            ast::Expr::new(
                ast::ExprKind::MethodCall {
                    object: Box::new(ast::Expr::new(
                        ast::ExprKind::Ident("msg".into()),
                        Span::dummy(),
                    )),
                    method: ast::Spanned::dummy("len".into()),
                    args: vec![],
                },
                Span::dummy(),
            ),
        ),
    ]);

    let ty = checker.infer_expr(&select_expr, &env).unwrap();
    assert_eq!(ty, Type::Int);
}

#[test]
fn test_select_receive_without_pattern() {
    let mut checker = TypeChecker::new();
    let mut env = TypeEnv::new();

    // Define a channel of Int
    env.define_var("ch".to_string(), Type::Channel(Box::new(Type::Int)));

    // select { <-ch => "received" }
    // No pattern binding, just consuming the value
    let select_expr = make_select_expr(vec![
        make_receive_arm(
            None,
            make_channel_expr("ch"),
            make_string_expr("received"),
        ),
    ]);

    let ty = checker.infer_expr(&select_expr, &env).unwrap();
    assert_eq!(ty, Type::String);
}

// ============================================================================
// Type Inference Tests
// ============================================================================

#[test]
fn test_select_infers_channel_type_from_send() {
    let mut checker = TypeChecker::new();
    let mut env = TypeEnv::new();
    let mut inf = TypeInference::new();

    // Define an untyped channel (type variable)
    let elem_var = inf.fresh_var();
    env.define_var("ch".to_string(), Type::Channel(Box::new(elem_var.clone())));

    // select { ch <- 42 => true }
    // This should infer ch is Channel[Int]
    let select_expr = make_select_expr(vec![
        make_send_arm(
            make_channel_expr("ch"),
            make_int_expr(42),
            make_bool_expr(true),
        ),
    ]);

    let ty = checker.infer_expr(&select_expr, &env).unwrap();
    assert_eq!(ty, Type::Bool);

    // Note: The type inference happens inside the checker, so we can't
    // directly verify it here without accessing private fields.
    // The fact that the type check succeeds is the primary validation.
}

// ============================================================================
// Mixed Arms Tests
// ============================================================================

#[test]
fn test_select_mixed_send_and_receive_arms() {
    let mut checker = TypeChecker::new();
    let mut env = TypeEnv::new();

    env.define_var("inbox".to_string(), Type::Channel(Box::new(Type::String)));
    env.define_var("outbox".to_string(), Type::Channel(Box::new(Type::Int)));

    // select {
    //     msg = <-inbox => msg.len(),
    //     outbox <- 42 => 0,
    //     default => -1
    // }
    let select_expr = make_select_expr(vec![
        make_receive_arm(
            Some(make_ident_pattern("msg")),
            make_channel_expr("inbox"),
            ast::Expr::new(
                ast::ExprKind::MethodCall {
                    object: Box::new(ast::Expr::new(
                        ast::ExprKind::Ident("msg".into()),
                        Span::dummy(),
                    )),
                    method: ast::Spanned::dummy("len".into()),
                    args: vec![],
                },
                Span::dummy(),
            ),
        ),
        make_send_arm(
            make_channel_expr("outbox"),
            make_int_expr(42),
            make_int_expr(0),
        ),
        make_default_arm(ast::Expr::new(
            ast::ExprKind::Unary {
                op: ast::UnaryOp::Neg,
                operand: Box::new(make_int_expr(1)),
            },
            Span::dummy(),
        )),
    ]);

    let ty = checker.infer_expr(&select_expr, &env).unwrap();
    assert_eq!(ty, Type::Int);
}

// ============================================================================
// Error Message Tests
// ============================================================================

#[test]
fn test_select_arm_mismatch_error_message() {
    use aria_types::TypeError;

    let err = TypeError::SelectArmTypeMismatch {
        expected: "Int".to_string(),
        found: "String".to_string(),
        arm_index: 2,
        span: Span::dummy(),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("Int"));
    assert!(msg.contains("String"));
    assert!(msg.contains("mismatch"));
}

#[test]
fn test_multiple_default_error_message() {
    use aria_types::TypeError;

    let err = TypeError::MultipleDefaultArms {
        first_span: Span::dummy(),
        second_span: Span::dummy(),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("default"));
    assert!(msg.contains("multiple"));
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_select_with_only_default_arm() {
    let mut checker = TypeChecker::new();
    let env = TypeEnv::new();

    // select { default => 42 }
    let select_expr = make_select_expr(vec![
        make_default_arm(make_int_expr(42)),
    ]);

    let ty = checker.infer_expr(&select_expr, &env).unwrap();
    assert_eq!(ty, Type::Int);
}

#[test]
fn test_select_unit_result_type() {
    let mut checker = TypeChecker::new();
    let mut env = TypeEnv::new();

    env.define_var("ch".to_string(), Type::Channel(Box::new(Type::Int)));

    // select { <-ch => (), default => () }
    // Using empty tuple () for unit type
    let select_expr = make_select_expr(vec![
        make_receive_arm(
            None,
            make_channel_expr("ch"),
            ast::Expr::new(ast::ExprKind::Tuple(vec![]), Span::dummy()),
        ),
        make_default_arm(ast::Expr::new(ast::ExprKind::Tuple(vec![]), Span::dummy())),
    ]);

    let ty = checker.infer_expr(&select_expr, &env).unwrap();
    // Empty tuple represents unit
    assert_eq!(ty, Type::Tuple(vec![]));
}
