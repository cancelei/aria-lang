//! Tests for default parameter type checking
//!
//! This module tests that:
//! 1. Default parameter values must match their declared types
//! 2. Default parameters must come after required parameters
//! 3. Call sites can omit arguments for parameters with defaults
//! 4. Named arguments work correctly with default parameters
//! 5. Proper errors are reported for type mismatches

use aria_ast as ast;
use aria_ast::{Span, Visibility};
use aria_types::{TypeChecker, TypeError};

// =========================================================================
// Helper functions
// =========================================================================

fn dummy_span() -> Span {
    Span::dummy()
}

fn make_ident(name: &str) -> ast::Ident {
    ast::Spanned::dummy(name.into())
}

fn make_type_ident(name: &str) -> ast::TypeIdent {
    ast::Spanned::dummy(name.into())
}

fn make_type_expr(name: &str) -> ast::TypeExpr {
    ast::TypeExpr::Named(make_type_ident(name))
}

fn make_int_literal(value: i64) -> ast::Expr {
    ast::Expr::new(
        ast::ExprKind::Integer(value.to_string().into()),
        dummy_span(),
    )
}

fn make_string_literal(value: &str) -> ast::Expr {
    ast::Expr::new(
        ast::ExprKind::String(value.into()),
        dummy_span(),
    )
}

fn make_bool_literal(value: bool) -> ast::Expr {
    ast::Expr::new(
        ast::ExprKind::Bool(value),
        dummy_span(),
    )
}

fn make_typed_param(name: &str, ty: &str) -> ast::Param {
    ast::Param {
        mutable: false,
        name: ast::Spanned::dummy(name.into()),
        ty: Some(make_type_expr(ty)),
        default: None,
        span: dummy_span(),
    }
}

fn make_typed_param_with_default(name: &str, ty: &str, default: ast::Expr) -> ast::Param {
    ast::Param {
        mutable: false,
        name: ast::Spanned::dummy(name.into()),
        ty: Some(make_type_expr(ty)),
        default: Some(default),
        span: dummy_span(),
    }
}

fn make_function(
    name: &str,
    params: Vec<ast::Param>,
    return_type: Option<ast::TypeExpr>,
    body: ast::Expr,
) -> ast::FunctionDecl {
    ast::FunctionDecl {
        attributes: vec![],
        visibility: Visibility::Private,
        name: make_ident(name),
        generic_params: None,
        params,
        return_type,
        where_clause: None,
        contracts: vec![],
        body: ast::FunctionBody::Expression(Box::new(body)),
        test_block: None,
        span: dummy_span(),
    }
}

fn make_call(func_name: &str, args: Vec<ast::CallArg>) -> ast::Expr {
    ast::Expr::new(
        ast::ExprKind::Call {
            func: Box::new(ast::Expr::new(
                ast::ExprKind::Ident(func_name.into()),
                dummy_span(),
            )),
            args,
        },
        dummy_span(),
    )
}

fn make_positional_arg(value: ast::Expr) -> ast::CallArg {
    ast::CallArg {
        name: None,
        value,
        spread: false,
    }
}

fn make_named_arg(name: &str, value: ast::Expr) -> ast::CallArg {
    ast::CallArg {
        name: Some(make_ident(name)),
        value,
        spread: false,
    }
}

fn make_program(items: Vec<ast::Item>) -> ast::Program {
    ast::Program {
        items,
        span: dummy_span(),
    }
}

// =========================================================================
// Tests: Default value type checking
// =========================================================================

#[test]
fn test_default_value_matches_type() {
    // fn greet(name: String, times: Int = 1) -> String = name
    let func = make_function(
        "greet",
        vec![
            make_typed_param("name", "String"),
            make_typed_param_with_default("times", "Int", make_int_literal(1)),
        ],
        Some(make_type_expr("String")),
        ast::Expr::new(ast::ExprKind::Ident("name".into()), dummy_span()),
    );

    let program = make_program(vec![ast::Item::Function(func)]);
    let mut checker = TypeChecker::new();
    let result = checker.check_program(&program);

    assert!(result.is_ok(), "Function with matching default type should pass: {:?}", result);
}

#[test]
fn test_default_value_type_mismatch() {
    // fn greet(times: Int = "oops") -> Int = times
    // Default is String but param is Int - should fail
    let func = make_function(
        "greet",
        vec![
            make_typed_param_with_default("times", "Int", make_string_literal("oops")),
        ],
        Some(make_type_expr("Int")),
        ast::Expr::new(ast::ExprKind::Ident("times".into()), dummy_span()),
    );

    let program = make_program(vec![ast::Item::Function(func)]);
    let mut checker = TypeChecker::new();
    let result = checker.check_program(&program);

    assert!(result.is_err(), "Function with mismatched default type should fail");
    if let Err(TypeError::DefaultValueTypeMismatch { param_name, .. }) = result {
        assert_eq!(param_name, "times");
    } else {
        panic!("Expected DefaultValueTypeMismatch error, got: {:?}", result);
    }
}

#[test]
fn test_multiple_default_params() {
    // fn configure(host: String = "localhost", port: Int = 8080) -> String = host
    let func = make_function(
        "configure",
        vec![
            make_typed_param_with_default("host", "String", make_string_literal("localhost")),
            make_typed_param_with_default("port", "Int", make_int_literal(8080)),
        ],
        Some(make_type_expr("String")),
        ast::Expr::new(ast::ExprKind::Ident("host".into()), dummy_span()),
    );

    let program = make_program(vec![ast::Item::Function(func)]);
    let mut checker = TypeChecker::new();
    let result = checker.check_program(&program);

    assert!(result.is_ok(), "Function with multiple default params should pass: {:?}", result);
}

// =========================================================================
// Tests: Parameter ordering
// =========================================================================

#[test]
fn test_default_after_required_is_valid() {
    // fn foo(a: Int, b: Int = 1, c: Int = 2) -> Int = a
    let func = make_function(
        "foo",
        vec![
            make_typed_param("a", "Int"),
            make_typed_param_with_default("b", "Int", make_int_literal(1)),
            make_typed_param_with_default("c", "Int", make_int_literal(2)),
        ],
        Some(make_type_expr("Int")),
        ast::Expr::new(ast::ExprKind::Ident("a".into()), dummy_span()),
    );

    let program = make_program(vec![ast::Item::Function(func)]);
    let mut checker = TypeChecker::new();
    let result = checker.check_program(&program);

    assert!(result.is_ok(), "Default params after required should pass: {:?}", result);
}

#[test]
fn test_required_after_default_is_error() {
    // fn foo(a: Int = 1, b: Int) -> Int = b
    // Required param 'b' comes after default param 'a' - should fail
    let func = make_function(
        "foo",
        vec![
            make_typed_param_with_default("a", "Int", make_int_literal(1)),
            make_typed_param("b", "Int"), // Error: required after default
        ],
        Some(make_type_expr("Int")),
        ast::Expr::new(ast::ExprKind::Ident("b".into()), dummy_span()),
    );

    let program = make_program(vec![ast::Item::Function(func)]);
    let mut checker = TypeChecker::new();
    let result = checker.check_program(&program);

    assert!(result.is_err(), "Required param after default should fail");
    if let Err(TypeError::DefaultAfterRequired { param_name, .. }) = result {
        assert_eq!(param_name, "b");
    } else {
        panic!("Expected DefaultAfterRequired error, got: {:?}", result);
    }
}

// =========================================================================
// Tests: Call site checking with defaults
// =========================================================================

#[test]
fn test_call_with_all_args_provided() {
    // fn add(a: Int, b: Int = 10) -> Int = a
    // Call: add(1, 2) - all args provided
    let func = make_function(
        "add",
        vec![
            make_typed_param("a", "Int"),
            make_typed_param_with_default("b", "Int", make_int_literal(10)),
        ],
        Some(make_type_expr("Int")),
        ast::Expr::new(ast::ExprKind::Ident("a".into()), dummy_span()),
    );

    let call_expr = make_call("add", vec![
        make_positional_arg(make_int_literal(1)),
        make_positional_arg(make_int_literal(2)),
    ]);

    let program = make_program(vec![
        ast::Item::Function(func),
        ast::Item::Function(make_function(
            "main",
            vec![],
            Some(make_type_expr("Int")),
            call_expr,
        )),
    ]);

    let mut checker = TypeChecker::new();
    let result = checker.check_program(&program);

    assert!(result.is_ok(), "Call with all args should pass: {:?}", result);
}

#[test]
fn test_call_omitting_default_arg() {
    // fn add(a: Int, b: Int = 10) -> Int = a
    // Call: add(1) - omit default arg
    let func = make_function(
        "add",
        vec![
            make_typed_param("a", "Int"),
            make_typed_param_with_default("b", "Int", make_int_literal(10)),
        ],
        Some(make_type_expr("Int")),
        ast::Expr::new(ast::ExprKind::Ident("a".into()), dummy_span()),
    );

    let call_expr = make_call("add", vec![
        make_positional_arg(make_int_literal(1)),
    ]);

    let program = make_program(vec![
        ast::Item::Function(func),
        ast::Item::Function(make_function(
            "main",
            vec![],
            Some(make_type_expr("Int")),
            call_expr,
        )),
    ]);

    let mut checker = TypeChecker::new();
    let result = checker.check_program(&program);

    assert!(result.is_ok(), "Call omitting default arg should pass: {:?}", result);
}

#[test]
fn test_call_missing_required_arg() {
    // fn add(a: Int, b: Int = 10) -> Int = a
    // Call: add() - missing required arg 'a'
    let func = make_function(
        "add",
        vec![
            make_typed_param("a", "Int"),
            make_typed_param_with_default("b", "Int", make_int_literal(10)),
        ],
        Some(make_type_expr("Int")),
        ast::Expr::new(ast::ExprKind::Ident("a".into()), dummy_span()),
    );

    let call_expr = make_call("add", vec![]);

    let program = make_program(vec![
        ast::Item::Function(func),
        ast::Item::Function(make_function(
            "main",
            vec![],
            Some(make_type_expr("Int")),
            call_expr,
        )),
    ]);

    let mut checker = TypeChecker::new();
    let result = checker.check_program(&program);

    assert!(result.is_err(), "Call missing required arg should fail");
    match result {
        Err(TypeError::MissingRequiredArgument { name, .. }) => {
            assert_eq!(name, "a");
        }
        _ => panic!("Expected MissingRequiredArgument error, got: {:?}", result),
    }
}

// =========================================================================
// Tests: Named arguments
// =========================================================================

#[test]
fn test_call_with_named_args() {
    // fn greet(name: String, greeting: String = "Hello") -> String = name
    // Call: greet(name: "World")
    let func = make_function(
        "greet",
        vec![
            make_typed_param("name", "String"),
            make_typed_param_with_default("greeting", "String", make_string_literal("Hello")),
        ],
        Some(make_type_expr("String")),
        ast::Expr::new(ast::ExprKind::Ident("name".into()), dummy_span()),
    );

    let call_expr = make_call("greet", vec![
        make_named_arg("name", make_string_literal("World")),
    ]);

    let program = make_program(vec![
        ast::Item::Function(func),
        ast::Item::Function(make_function(
            "main",
            vec![],
            Some(make_type_expr("String")),
            call_expr,
        )),
    ]);

    let mut checker = TypeChecker::new();
    let result = checker.check_program(&program);

    assert!(result.is_ok(), "Call with named args should pass: {:?}", result);
}

#[test]
fn test_call_with_out_of_order_named_args() {
    // fn make(a: Int, b: Int = 1, c: Int = 2) -> Int = a
    // Call: make(c: 30, a: 10) - out of order named args
    let func = make_function(
        "make",
        vec![
            make_typed_param("a", "Int"),
            make_typed_param_with_default("b", "Int", make_int_literal(1)),
            make_typed_param_with_default("c", "Int", make_int_literal(2)),
        ],
        Some(make_type_expr("Int")),
        ast::Expr::new(ast::ExprKind::Ident("a".into()), dummy_span()),
    );

    let call_expr = make_call("make", vec![
        make_named_arg("c", make_int_literal(30)),
        make_named_arg("a", make_int_literal(10)),
    ]);

    let program = make_program(vec![
        ast::Item::Function(func),
        ast::Item::Function(make_function(
            "main",
            vec![],
            Some(make_type_expr("Int")),
            call_expr,
        )),
    ]);

    let mut checker = TypeChecker::new();
    let result = checker.check_program(&program);

    assert!(result.is_ok(), "Call with out-of-order named args should pass: {:?}", result);
}

#[test]
fn test_call_with_unknown_named_arg() {
    // fn foo(a: Int) -> Int = a
    // Call: foo(b: 1) - 'b' doesn't exist
    let func = make_function(
        "foo",
        vec![
            make_typed_param("a", "Int"),
        ],
        Some(make_type_expr("Int")),
        ast::Expr::new(ast::ExprKind::Ident("a".into()), dummy_span()),
    );

    let call_expr = make_call("foo", vec![
        make_named_arg("b", make_int_literal(1)),
    ]);

    let program = make_program(vec![
        ast::Item::Function(func),
        ast::Item::Function(make_function(
            "main",
            vec![],
            Some(make_type_expr("Int")),
            call_expr,
        )),
    ]);

    let mut checker = TypeChecker::new();
    let result = checker.check_program(&program);

    assert!(result.is_err(), "Call with unknown named arg should fail");
    match result {
        Err(TypeError::UnknownNamedArgument { name, .. }) => {
            assert_eq!(name, "b");
        }
        _ => panic!("Expected UnknownNamedArgument error, got: {:?}", result),
    }
}

#[test]
fn test_call_with_duplicate_named_arg() {
    // fn foo(a: Int) -> Int = a
    // Call: foo(a: 1, a: 2) - duplicate 'a'
    let func = make_function(
        "foo",
        vec![
            make_typed_param("a", "Int"),
        ],
        Some(make_type_expr("Int")),
        ast::Expr::new(ast::ExprKind::Ident("a".into()), dummy_span()),
    );

    let call_expr = make_call("foo", vec![
        make_named_arg("a", make_int_literal(1)),
        make_named_arg("a", make_int_literal(2)),
    ]);

    let program = make_program(vec![
        ast::Item::Function(func),
        ast::Item::Function(make_function(
            "main",
            vec![],
            Some(make_type_expr("Int")),
            call_expr,
        )),
    ]);

    let mut checker = TypeChecker::new();
    let result = checker.check_program(&program);

    assert!(result.is_err(), "Call with duplicate named arg should fail");
    match result {
        Err(TypeError::DuplicateNamedArgument { name, .. }) => {
            assert_eq!(name, "a");
        }
        _ => panic!("Expected DuplicateNamedArgument error, got: {:?}", result),
    }
}

// =========================================================================
// Tests: Mixed positional and named arguments
// =========================================================================

#[test]
fn test_call_mixed_positional_then_named() {
    // fn make(a: Int, b: Int, c: Int = 3) -> Int = a
    // Call: make(1, c: 30) - positional then named
    let func = make_function(
        "make",
        vec![
            make_typed_param("a", "Int"),
            make_typed_param("b", "Int"),
            make_typed_param_with_default("c", "Int", make_int_literal(3)),
        ],
        Some(make_type_expr("Int")),
        ast::Expr::new(ast::ExprKind::Ident("a".into()), dummy_span()),
    );

    let call_expr = make_call("make", vec![
        make_positional_arg(make_int_literal(1)),
        make_positional_arg(make_int_literal(2)),
        make_named_arg("c", make_int_literal(30)),
    ]);

    let program = make_program(vec![
        ast::Item::Function(func),
        ast::Item::Function(make_function(
            "main",
            vec![],
            Some(make_type_expr("Int")),
            call_expr,
        )),
    ]);

    let mut checker = TypeChecker::new();
    let result = checker.check_program(&program);

    assert!(result.is_ok(), "Call with positional then named should pass: {:?}", result);
}

#[test]
fn test_call_positional_after_named_is_error() {
    // fn make(a: Int, b: Int) -> Int = a
    // Call: make(a: 1, 2) - positional after named is error
    let func = make_function(
        "make",
        vec![
            make_typed_param("a", "Int"),
            make_typed_param("b", "Int"),
        ],
        Some(make_type_expr("Int")),
        ast::Expr::new(ast::ExprKind::Ident("a".into()), dummy_span()),
    );

    let call_expr = make_call("make", vec![
        make_named_arg("a", make_int_literal(1)),
        make_positional_arg(make_int_literal(2)),
    ]);

    let program = make_program(vec![
        ast::Item::Function(func),
        ast::Item::Function(make_function(
            "main",
            vec![],
            Some(make_type_expr("Int")),
            call_expr,
        )),
    ]);

    let mut checker = TypeChecker::new();
    let result = checker.check_program(&program);

    assert!(result.is_err(), "Positional after named should fail");
    match result {
        Err(TypeError::PositionalAfterNamed { .. }) => {}
        _ => panic!("Expected PositionalAfterNamed error, got: {:?}", result),
    }
}

// =========================================================================
// Tests: Boolean default values
// =========================================================================

#[test]
fn test_default_bool_value() {
    // fn is_enabled(flag: Bool = true) -> Bool = flag
    let func = make_function(
        "is_enabled",
        vec![
            make_typed_param_with_default("flag", "Bool", make_bool_literal(true)),
        ],
        Some(make_type_expr("Bool")),
        ast::Expr::new(ast::ExprKind::Ident("flag".into()), dummy_span()),
    );

    let program = make_program(vec![ast::Item::Function(func)]);
    let mut checker = TypeChecker::new();
    let result = checker.check_program(&program);

    assert!(result.is_ok(), "Function with bool default should pass: {:?}", result);
}

// =========================================================================
// Tests: All parameters have defaults
// =========================================================================

#[test]
fn test_all_params_have_defaults() {
    // fn config(a: Int = 1, b: Int = 2, c: Int = 3) -> Int = a
    let func = make_function(
        "config",
        vec![
            make_typed_param_with_default("a", "Int", make_int_literal(1)),
            make_typed_param_with_default("b", "Int", make_int_literal(2)),
            make_typed_param_with_default("c", "Int", make_int_literal(3)),
        ],
        Some(make_type_expr("Int")),
        ast::Expr::new(ast::ExprKind::Ident("a".into()), dummy_span()),
    );

    // Call with no arguments
    let call_expr = make_call("config", vec![]);

    let program = make_program(vec![
        ast::Item::Function(func),
        ast::Item::Function(make_function(
            "main",
            vec![],
            Some(make_type_expr("Int")),
            call_expr,
        )),
    ]);

    let mut checker = TypeChecker::new();
    let result = checker.check_program(&program);

    assert!(result.is_ok(), "Call with all defaults should pass: {:?}", result);
}

#[test]
fn test_too_many_args_with_defaults() {
    // fn add(a: Int, b: Int = 10) -> Int = a
    // Call: add(1, 2, 3) - too many args
    let func = make_function(
        "add",
        vec![
            make_typed_param("a", "Int"),
            make_typed_param_with_default("b", "Int", make_int_literal(10)),
        ],
        Some(make_type_expr("Int")),
        ast::Expr::new(ast::ExprKind::Ident("a".into()), dummy_span()),
    );

    let call_expr = make_call("add", vec![
        make_positional_arg(make_int_literal(1)),
        make_positional_arg(make_int_literal(2)),
        make_positional_arg(make_int_literal(3)),
    ]);

    let program = make_program(vec![
        ast::Item::Function(func),
        ast::Item::Function(make_function(
            "main",
            vec![],
            Some(make_type_expr("Int")),
            call_expr,
        )),
    ]);

    let mut checker = TypeChecker::new();
    let result = checker.check_program(&program);

    assert!(result.is_err(), "Too many args should fail");
    match result {
        Err(TypeError::TooManyArguments { max_allowed, found, .. }) => {
            assert_eq!(max_allowed, 2);
            assert_eq!(found, 3);
        }
        _ => panic!("Expected TooManyArguments error, got: {:?}", result),
    }
}

// =========================================================================
// Tests: Named argument skipping middle defaults
// =========================================================================

#[test]
fn test_skip_middle_default_with_named() {
    // fn make(a: Int, b: Int = 1, c: Int = 2) -> Int = a
    // Call: make(10, c: 30) - skip b, provide a and c
    let func = make_function(
        "make",
        vec![
            make_typed_param("a", "Int"),
            make_typed_param_with_default("b", "Int", make_int_literal(1)),
            make_typed_param_with_default("c", "Int", make_int_literal(2)),
        ],
        Some(make_type_expr("Int")),
        ast::Expr::new(ast::ExprKind::Ident("a".into()), dummy_span()),
    );

    let call_expr = make_call("make", vec![
        make_positional_arg(make_int_literal(10)),
        make_named_arg("c", make_int_literal(30)),
    ]);

    let program = make_program(vec![
        ast::Item::Function(func),
        ast::Item::Function(make_function(
            "main",
            vec![],
            Some(make_type_expr("Int")),
            call_expr,
        )),
    ]);

    let mut checker = TypeChecker::new();
    let result = checker.check_program(&program);

    assert!(result.is_ok(), "Skipping middle default with named should pass: {:?}", result);
}
