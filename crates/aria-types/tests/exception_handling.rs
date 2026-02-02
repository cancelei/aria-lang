//! Tests for exception handling (try/catch) type checking
//!
//! This module tests:
//! 1. Handle expression type inference
//! 2. Exception.raise handler clause binding
//! 3. Return clause transformation
//! 4. Handler body type compatibility
//! 5. Raise expression returns Never
//! 6. Resume expression type inference

use aria_ast::{self as ast, Span};
use aria_types::{Type, TypeChecker, TypeEnv};

// ============================================================================
// Helper functions to construct AST nodes
// ============================================================================

fn make_ident_expr(name: &str) -> ast::Expr {
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

fn make_ident_pattern(name: &str) -> ast::Pattern {
    ast::Pattern {
        kind: ast::PatternKind::Ident(name.into()),
        span: Span::dummy(),
    }
}

fn make_wildcard_pattern() -> ast::Pattern {
    ast::Pattern {
        kind: ast::PatternKind::Wildcard,
        span: Span::dummy(),
    }
}

fn make_type_ident(name: &str) -> ast::TypeIdent {
    ast::Spanned::new(name.into(), Span::dummy())
}

fn make_ident(name: &str) -> ast::Ident {
    ast::Spanned::new(name.into(), Span::dummy())
}

fn make_handler_clause(
    effect: &str,
    operation: &str,
    params: Vec<ast::Pattern>,
    body_expr: ast::Expr,
) -> ast::HandlerClause {
    ast::HandlerClause {
        effect: make_type_ident(effect),
        operation: make_ident(operation),
        params,
        body: ast::HandlerBody::Expr(Box::new(body_expr)),
        span: Span::dummy(),
    }
}

fn make_return_clause(pattern: ast::Pattern, body_expr: ast::Expr) -> ast::ReturnClause {
    ast::ReturnClause {
        pattern,
        body: Box::new(ast::HandlerBody::Expr(Box::new(body_expr))),
        span: Span::dummy(),
    }
}

fn make_handle_expr(
    body: ast::Expr,
    handlers: Vec<ast::HandlerClause>,
    return_clause: Option<ast::ReturnClause>,
) -> ast::Expr {
    ast::Expr::new(
        ast::ExprKind::Handle {
            body: Box::new(body),
            handlers,
            return_clause: return_clause.map(Box::new),
        },
        Span::dummy(),
    )
}

fn make_raise_expr(error: ast::Expr) -> ast::Expr {
    ast::Expr::new(
        ast::ExprKind::Raise {
            error: Box::new(error),
            exception_type: None,
        },
        Span::dummy(),
    )
}

fn make_resume_expr(value: ast::Expr) -> ast::Expr {
    ast::Expr::new(
        ast::ExprKind::Resume {
            value: Box::new(value),
        },
        Span::dummy(),
    )
}

// ============================================================================
// Handle Expression Tests
// ============================================================================

#[test]
fn test_handle_simple_body_type() {
    // handle
    //   42
    // with
    // end
    // Should return Int (the body type)
    let body = make_int_expr(42);
    let handle_expr = make_handle_expr(body, vec![], None);

    let mut checker = TypeChecker::new();
    let env = TypeEnv::new();

    let result = checker.infer_expr(&handle_expr, &env);
    assert!(result.is_ok(), "Handle expression should type-check: {:?}", result.err());

    let ty = result.unwrap();
    let resolved = checker.apply_substitutions(&ty);
    assert_eq!(resolved, Type::Int, "Handle body type should be Int");
}

#[test]
fn test_handle_with_exception_handler() {
    // handle
    //   risky_operation()
    // with
    //   Exception.raise(e) => "error"
    // end
    // Should return String (the handler return type)
    let body = make_ident_expr("risky_operation");
    let handler = make_handler_clause(
        "Exception",
        "raise",
        vec![make_ident_pattern("e")],
        make_string_expr("error"),
    );
    let handle_expr = make_handle_expr(body, vec![handler], None);

    let mut env = TypeEnv::new();
    // Define risky_operation as returning String (to match handler)
    env.define_var("risky_operation".to_string(), Type::String);

    let mut checker = TypeChecker::new();
    let result = checker.infer_expr(&handle_expr, &env);
    assert!(result.is_ok(), "Handle with exception handler should type-check: {:?}", result.err());

    let ty = result.unwrap();
    let resolved = checker.apply_substitutions(&ty);
    assert_eq!(resolved, Type::String, "Handle result should be String");
}

#[test]
fn test_handle_with_return_clause() {
    // handle
    //   42
    // with
    //   return(x) => x + 1
    // end
    // The return clause transforms the result
    let body = make_int_expr(42);
    let return_pattern = make_ident_pattern("x");

    // x + 1 - we'll use a simplified version that returns Int
    let return_body = ast::Expr::new(
        ast::ExprKind::Binary {
            op: ast::BinaryOp::Add,
            left: Box::new(make_ident_expr("x")),
            right: Box::new(make_int_expr(1)),
        },
        Span::dummy(),
    );

    let return_clause = make_return_clause(return_pattern, return_body);
    let handle_expr = make_handle_expr(body, vec![], Some(return_clause));

    let mut checker = TypeChecker::new();
    let env = TypeEnv::new();

    let result = checker.infer_expr(&handle_expr, &env);
    assert!(result.is_ok(), "Handle with return clause should type-check: {:?}", result.err());

    let ty = result.unwrap();
    let resolved = checker.apply_substitutions(&ty);
    assert_eq!(resolved, Type::Int, "Transformed result should be Int");
}

#[test]
fn test_handle_multiple_handlers_same_type() {
    // handle
    //   body
    // with
    //   Exception.raise(e) => 0
    //   IO.read(path) => 0
    // end
    // All handlers must return the same type
    let body = make_int_expr(42);
    let handler1 = make_handler_clause(
        "Exception",
        "raise",
        vec![make_ident_pattern("e")],
        make_int_expr(0),
    );
    let handler2 = make_handler_clause(
        "IO",
        "read",
        vec![make_ident_pattern("path")],
        make_int_expr(0),
    );
    let handle_expr = make_handle_expr(body, vec![handler1, handler2], None);

    let mut checker = TypeChecker::new();
    let env = TypeEnv::new();

    let result = checker.infer_expr(&handle_expr, &env);
    assert!(result.is_ok(), "Multiple handlers with same type should type-check: {:?}", result.err());
}

// ============================================================================
// Raise Expression Tests
// ============================================================================

#[test]
fn test_raise_returns_never() {
    // raise("error message")
    // Should return Never type
    let raise_expr = make_raise_expr(make_string_expr("error message"));

    let mut checker = TypeChecker::new();
    let env = TypeEnv::new();

    let result = checker.infer_expr(&raise_expr, &env);
    assert!(result.is_ok(), "Raise expression should type-check: {:?}", result.err());

    let ty = result.unwrap();
    assert_eq!(ty, Type::Never, "Raise should return Never type");
}

#[test]
fn test_raise_with_int_error() {
    // raise(404)
    // Error can be any type
    let raise_expr = make_raise_expr(make_int_expr(404));

    let mut checker = TypeChecker::new();
    let env = TypeEnv::new();

    let result = checker.infer_expr(&raise_expr, &env);
    assert!(result.is_ok(), "Raise with int error should type-check");
    assert_eq!(result.unwrap(), Type::Never);
}

// ============================================================================
// Resume Expression Tests
// ============================================================================

#[test]
fn test_resume_expression() {
    // resume(42)
    // Inside a handler, resume continues with the given value
    let resume_expr = make_resume_expr(make_int_expr(42));

    let mut checker = TypeChecker::new();
    let env = TypeEnv::new();

    // Resume should type-check (returns a fresh type variable)
    let result = checker.infer_expr(&resume_expr, &env);
    assert!(result.is_ok(), "Resume expression should type-check: {:?}", result.err());
}

#[test]
fn test_resume_with_string_value() {
    // resume("success")
    let resume_expr = make_resume_expr(make_string_expr("success"));

    let mut checker = TypeChecker::new();
    let env = TypeEnv::new();

    let result = checker.infer_expr(&resume_expr, &env);
    assert!(result.is_ok(), "Resume with string value should type-check");
}

// ============================================================================
// Handler with Resume Tests
// ============================================================================

#[test]
fn test_handler_with_resume() {
    // handle
    //   get_value()
    // with
    //   State.get() => resume(42)
    // end
    // The handler uses resume to continue the computation
    let body = make_ident_expr("get_value");
    let handler = make_handler_clause(
        "State",
        "get",
        vec![],
        make_resume_expr(make_int_expr(42)),
    );
    let handle_expr = make_handle_expr(body, vec![handler], None);

    let mut env = TypeEnv::new();
    env.define_var("get_value".to_string(), Type::Int);

    let mut checker = TypeChecker::new();
    let result = checker.infer_expr(&handle_expr, &env);
    assert!(result.is_ok(), "Handler with resume should type-check: {:?}", result.err());
}

// ============================================================================
// Exception Effect Type Tests
// ============================================================================

#[test]
fn test_exception_effect_with_error_type() {
    use aria_types::Effect;

    // Exception[String] effect
    let exc_string = Effect::Exception(Box::new(Type::String));
    assert_eq!(format!("{}", exc_string), "Exception[String]");

    // Exception[Int] effect (error codes)
    let exc_int = Effect::Exception(Box::new(Type::Int));
    assert_eq!(format!("{}", exc_int), "Exception[Int]");

    // Exception effects with different error types are different
    assert_ne!(exc_string, exc_int);
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_handle_exception_and_return_default() {
    // This is the classic try/catch pattern:
    // handle
    //   might_fail()
    // with
    //   Exception.raise(_) => default_value
    // end
    let body = make_ident_expr("might_fail");
    let handler = make_handler_clause(
        "Exception",
        "raise",
        vec![make_wildcard_pattern()],
        make_int_expr(0), // Default value on error
    );
    let handle_expr = make_handle_expr(body, vec![handler], None);

    let mut env = TypeEnv::new();
    env.define_var("might_fail".to_string(), Type::Int);

    let mut checker = TypeChecker::new();
    let result = checker.infer_expr(&handle_expr, &env);
    assert!(result.is_ok(), "Try/catch pattern should type-check: {:?}", result.err());

    let ty = result.unwrap();
    let resolved = checker.apply_substitutions(&ty);
    assert_eq!(resolved, Type::Int);
}
