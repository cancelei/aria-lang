//! String Interpolation Type Checking Tests
//!
//! Tests for type checking string interpolation expressions like "Hello, #{name}!".
//! These tests verify that:
//! 1. Interpolated strings always have type String
//! 2. Interpolated expressions are properly type-checked
//! 3. Interpolated values must implement Display or be convertible to String
//! 4. Function types and Channel types are rejected (don't implement Display)
//! 5. Format specifiers are supported

use aria_ast::{self as ast, Span};
use aria_types::{Type, TypeChecker, TypeEnv, TypeError};

#[test]
fn test_infer_interpolated_string_with_primitives() {
    let mut checker = TypeChecker::new();
    let mut env = TypeEnv::new();
    env.define_var("name".to_string(), Type::String);
    env.define_var("age".to_string(), Type::Int);

    // "Hello, #{name}! You are #{age} years old."
    let interp_expr = ast::Expr::new(
        ast::ExprKind::InterpolatedString(vec![
            ast::StringPart::Literal("Hello, ".into()),
            ast::StringPart::Expr(Box::new(ast::Expr::new(
                ast::ExprKind::Ident("name".into()),
                Span::dummy(),
            ))),
            ast::StringPart::Literal("! You are ".into()),
            ast::StringPart::Expr(Box::new(ast::Expr::new(
                ast::ExprKind::Ident("age".into()),
                Span::dummy(),
            ))),
            ast::StringPart::Literal(" years old.".into()),
        ]),
        Span::dummy(),
    );
    let ty = checker.infer_expr(&interp_expr, &env).unwrap();
    assert_eq!(ty, Type::String);
}

#[test]
fn test_infer_interpolated_string_with_bool_and_float() {
    let mut checker = TypeChecker::new();
    let mut env = TypeEnv::new();
    env.define_var("flag".to_string(), Type::Bool);
    env.define_var("pi".to_string(), Type::Float);

    // "Flag is #{flag}, pi is #{pi}"
    let interp_expr = ast::Expr::new(
        ast::ExprKind::InterpolatedString(vec![
            ast::StringPart::Literal("Flag is ".into()),
            ast::StringPart::Expr(Box::new(ast::Expr::new(
                ast::ExprKind::Ident("flag".into()),
                Span::dummy(),
            ))),
            ast::StringPart::Literal(", pi is ".into()),
            ast::StringPart::Expr(Box::new(ast::Expr::new(
                ast::ExprKind::Ident("pi".into()),
                Span::dummy(),
            ))),
        ]),
        Span::dummy(),
    );
    let ty = checker.infer_expr(&interp_expr, &env).unwrap();
    assert_eq!(ty, Type::String);
}

#[test]
fn test_infer_interpolated_string_with_expressions() {
    let mut checker = TypeChecker::new();
    let env = TypeEnv::new();

    // "Result: #{1 + 2}"
    let interp_expr = ast::Expr::new(
        ast::ExprKind::InterpolatedString(vec![
            ast::StringPart::Literal("Result: ".into()),
            ast::StringPart::Expr(Box::new(ast::Expr::new(
                ast::ExprKind::Binary {
                    op: ast::BinaryOp::Add,
                    left: Box::new(ast::Expr::new(
                        ast::ExprKind::Integer("1".into()),
                        Span::dummy(),
                    )),
                    right: Box::new(ast::Expr::new(
                        ast::ExprKind::Integer("2".into()),
                        Span::dummy(),
                    )),
                },
                Span::dummy(),
            ))),
        ]),
        Span::dummy(),
    );
    let ty = checker.infer_expr(&interp_expr, &env).unwrap();
    assert_eq!(ty, Type::String);
}

#[test]
fn test_interpolated_string_with_format_specifier() {
    let mut checker = TypeChecker::new();
    let mut env = TypeEnv::new();
    env.define_var("num".to_string(), Type::Int);

    // "Formatted: #{num:02d}"
    let interp_expr = ast::Expr::new(
        ast::ExprKind::InterpolatedString(vec![
            ast::StringPart::Literal("Formatted: ".into()),
            ast::StringPart::FormattedExpr {
                expr: Box::new(ast::Expr::new(
                    ast::ExprKind::Ident("num".into()),
                    Span::dummy(),
                )),
                format: "02d".into(),
            },
        ]),
        Span::dummy(),
    );
    let ty = checker.infer_expr(&interp_expr, &env).unwrap();
    assert_eq!(ty, Type::String);
}

#[test]
fn test_interpolated_string_rejects_function_type() {
    let mut checker = TypeChecker::new();
    let mut env = TypeEnv::new();
    env.define_var(
        "callback".to_string(),
        Type::Function {
            params: vec![Type::Int],
            return_type: Box::new(Type::Int),
        },
    );

    // "Callback: #{callback}" - should fail because functions don't implement Display
    let interp_expr = ast::Expr::new(
        ast::ExprKind::InterpolatedString(vec![
            ast::StringPart::Literal("Callback: ".into()),
            ast::StringPart::Expr(Box::new(ast::Expr::new(
                ast::ExprKind::Ident("callback".into()),
                Span::dummy(),
            ))),
        ]),
        Span::dummy(),
    );
    let result = checker.infer_expr(&interp_expr, &env);
    assert!(result.is_err());
    if let Err(TypeError::TraitNotImplemented { trait_name, .. }) = result {
        assert_eq!(trait_name, "Display");
    } else {
        panic!("Expected TraitNotImplemented error");
    }
}

#[test]
fn test_interpolated_string_rejects_channel_type() {
    let mut checker = TypeChecker::new();
    let mut env = TypeEnv::new();
    env.define_var(
        "ch".to_string(),
        Type::Channel(Box::new(Type::Int)),
    );

    // "Channel: #{ch}" - should fail because channels don't implement Display
    let interp_expr = ast::Expr::new(
        ast::ExprKind::InterpolatedString(vec![
            ast::StringPart::Literal("Channel: ".into()),
            ast::StringPart::Expr(Box::new(ast::Expr::new(
                ast::ExprKind::Ident("ch".into()),
                Span::dummy(),
            ))),
        ]),
        Span::dummy(),
    );
    let result = checker.infer_expr(&interp_expr, &env);
    assert!(result.is_err());
    if let Err(TypeError::TraitNotImplemented { trait_name, .. }) = result {
        assert_eq!(trait_name, "Display");
    } else {
        panic!("Expected TraitNotImplemented error");
    }
}

#[test]
fn test_can_convert_to_string() {
    let checker = TypeChecker::new();

    // Primitives should be convertible
    assert!(checker.can_convert_to_string(&Type::Int));
    assert!(checker.can_convert_to_string(&Type::Float));
    assert!(checker.can_convert_to_string(&Type::Bool));
    assert!(checker.can_convert_to_string(&Type::Char));
    assert!(checker.can_convert_to_string(&Type::String));
    assert!(checker.can_convert_to_string(&Type::Unit));

    // Compound types with convertible elements
    assert!(checker.can_convert_to_string(&Type::Array(Box::new(Type::Int))));
    assert!(checker.can_convert_to_string(&Type::Optional(Box::new(Type::String))));
    assert!(checker.can_convert_to_string(&Type::Tuple(vec![Type::Int, Type::String])));

    // Function types should NOT be convertible
    assert!(!checker.can_convert_to_string(&Type::Function {
        params: vec![Type::Int],
        return_type: Box::new(Type::Int),
    }));

    // Channel types should NOT be convertible
    assert!(!checker.can_convert_to_string(&Type::Channel(Box::new(Type::Int))));
}

#[test]
fn test_empty_interpolated_string() {
    let mut checker = TypeChecker::new();
    let env = TypeEnv::new();

    // An empty interpolated string with no parts
    let interp_expr = ast::Expr::new(
        ast::ExprKind::InterpolatedString(vec![]),
        Span::dummy(),
    );
    let ty = checker.infer_expr(&interp_expr, &env).unwrap();
    assert_eq!(ty, Type::String);
}

#[test]
fn test_interpolated_string_only_literals() {
    let mut checker = TypeChecker::new();
    let env = TypeEnv::new();

    // A string with only literal parts
    let interp_expr = ast::Expr::new(
        ast::ExprKind::InterpolatedString(vec![
            ast::StringPart::Literal("Hello ".into()),
            ast::StringPart::Literal("World".into()),
        ]),
        Span::dummy(),
    );
    let ty = checker.infer_expr(&interp_expr, &env).unwrap();
    assert_eq!(ty, Type::String);
}

#[test]
fn test_interpolated_string_nested_expression() {
    let mut checker = TypeChecker::new();
    let mut env = TypeEnv::new();
    env.define_var("items".to_string(), Type::Array(Box::new(Type::Int)));

    // "Count: #{len(items)}" - nested function call in interpolation
    let interp_expr = ast::Expr::new(
        ast::ExprKind::InterpolatedString(vec![
            ast::StringPart::Literal("Count: ".into()),
            ast::StringPart::Expr(Box::new(ast::Expr::new(
                ast::ExprKind::Call {
                    func: Box::new(ast::Expr::new(
                        ast::ExprKind::Ident("len".into()),
                        Span::dummy(),
                    )),
                    args: vec![ast::CallArg {
                        name: None,
                        value: ast::Expr::new(
                            ast::ExprKind::Ident("items".into()),
                            Span::dummy(),
                        ),
                        spread: false,
                    }],
                },
                Span::dummy(),
            ))),
        ]),
        Span::dummy(),
    );
    let ty = checker.infer_expr(&interp_expr, &env).unwrap();
    assert_eq!(ty, Type::String);
}

#[test]
fn test_interpolated_string_with_all_numeric_types() {
    let mut checker = TypeChecker::new();
    let mut env = TypeEnv::new();

    // Define various numeric types
    env.define_var("i8_val".to_string(), Type::Int8);
    env.define_var("i16_val".to_string(), Type::Int16);
    env.define_var("i32_val".to_string(), Type::Int32);
    env.define_var("i64_val".to_string(), Type::Int64);
    env.define_var("u8_val".to_string(), Type::UInt8);
    env.define_var("f32_val".to_string(), Type::Float32);
    env.define_var("f64_val".to_string(), Type::Float64);

    // All numeric types should be displayable
    for var_name in ["i8_val", "i16_val", "i32_val", "i64_val", "u8_val", "f32_val", "f64_val"] {
        let interp_expr = ast::Expr::new(
            ast::ExprKind::InterpolatedString(vec![
                ast::StringPart::Literal("Value: ".into()),
                ast::StringPart::Expr(Box::new(ast::Expr::new(
                    ast::ExprKind::Ident(var_name.into()),
                    Span::dummy(),
                ))),
            ]),
            Span::dummy(),
        );
        let ty = checker.infer_expr(&interp_expr, &env).unwrap();
        assert_eq!(ty, Type::String, "Failed for {}", var_name);
    }
}

#[test]
fn test_interpolated_string_with_optional() {
    let mut checker = TypeChecker::new();
    let mut env = TypeEnv::new();
    env.define_var("maybe_int".to_string(), Type::Optional(Box::new(Type::Int)));

    // "Value: #{maybe_int}"
    let interp_expr = ast::Expr::new(
        ast::ExprKind::InterpolatedString(vec![
            ast::StringPart::Literal("Value: ".into()),
            ast::StringPart::Expr(Box::new(ast::Expr::new(
                ast::ExprKind::Ident("maybe_int".into()),
                Span::dummy(),
            ))),
        ]),
        Span::dummy(),
    );
    let ty = checker.infer_expr(&interp_expr, &env).unwrap();
    assert_eq!(ty, Type::String);
}

#[test]
fn test_interpolated_string_with_tuple() {
    let mut checker = TypeChecker::new();
    let mut env = TypeEnv::new();
    env.define_var("point".to_string(), Type::Tuple(vec![Type::Int, Type::Int]));

    // "Point: #{point}"
    let interp_expr = ast::Expr::new(
        ast::ExprKind::InterpolatedString(vec![
            ast::StringPart::Literal("Point: ".into()),
            ast::StringPart::Expr(Box::new(ast::Expr::new(
                ast::ExprKind::Ident("point".into()),
                Span::dummy(),
            ))),
        ]),
        Span::dummy(),
    );
    let ty = checker.infer_expr(&interp_expr, &env).unwrap();
    assert_eq!(ty, Type::String);
}
