//! Tests for type alias resolution
//!
//! This module tests that:
//! 1. Simple type aliases are properly registered and resolved
//! 2. Generic type aliases work with type parameter substitution
//! 3. Nested type aliases (aliases referencing other aliases) are expanded
//! 4. Type alias expansion works in function signatures, struct fields, etc.

use aria_ast as ast;
use aria_ast::{Span, Visibility};
use aria_types::{Type, TypeChecker};

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

fn make_generic_type_expr(name: &str, args: Vec<ast::TypeExpr>) -> ast::TypeExpr {
    ast::TypeExpr::Generic {
        name: make_type_ident(name),
        args,
        span: dummy_span(),
    }
}

fn make_tuple_type_expr(elements: Vec<ast::TypeExpr>) -> ast::TypeExpr {
    ast::TypeExpr::Tuple {
        elements,
        span: dummy_span(),
    }
}

fn make_optional_type_expr(inner: ast::TypeExpr) -> ast::TypeExpr {
    ast::TypeExpr::Optional {
        inner: Box::new(inner),
        span: dummy_span(),
    }
}

fn make_simple_type_alias(name: &str, ty: ast::TypeExpr) -> ast::TypeAlias {
    ast::TypeAlias {
        attributes: vec![],
        visibility: Visibility::Private,
        name: make_type_ident(name),
        generic_params: None,
        ty,
        span: dummy_span(),
    }
}

fn make_generic_type_alias(name: &str, params: Vec<&str>, ty: ast::TypeExpr) -> ast::TypeAlias {
    let generic_params = ast::GenericParams {
        params: params.iter().map(|p| ast::GenericParam {
            name: make_type_ident(p),
            bounds: vec![],
            span: dummy_span(),
        }).collect(),
        span: dummy_span(),
    };

    ast::TypeAlias {
        attributes: vec![],
        visibility: Visibility::Private,
        name: make_type_ident(name),
        generic_params: Some(generic_params),
        ty,
        span: dummy_span(),
    }
}

fn make_simple_function(name: &str, param_ty: Option<ast::TypeExpr>, return_ty: Option<ast::TypeExpr>) -> ast::FunctionDecl {
    let params = if let Some(ty) = param_ty {
        vec![ast::Param {
            mutable: false,
            name: make_ident("x"),
            ty: Some(ty),
            default: None,
            span: dummy_span(),
        }]
    } else {
        vec![]
    };

    ast::FunctionDecl {
        attributes: vec![],
        visibility: Visibility::Private,
        name: make_ident(name),
        generic_params: None,
        params,
        return_type: return_ty,
        where_clause: None,
        contracts: vec![],
        body: ast::FunctionBody::Expression(Box::new(ast::Expr::new(
            ast::ExprKind::Integer("0".into()),
            dummy_span(),
        ))),
        test_block: None,
        span: dummy_span(),
    }
}

// =========================================================================
// Simple Type Alias Tests
// =========================================================================

#[test]
fn test_simple_type_alias() {
    // Test: type UserId = Int
    let mut checker = TypeChecker::new();

    let alias = make_simple_type_alias("UserId", make_type_expr("Int"));

    let program = ast::Program {
        items: vec![ast::Item::TypeAlias(alias)],
        span: dummy_span(),
    };

    let result = checker.check_program(&program);
    assert!(result.is_ok(), "Simple type alias should succeed: {:?}", result);
}

#[test]
fn test_type_alias_to_string() {
    // Test: type Name = String
    let mut checker = TypeChecker::new();

    let alias = make_simple_type_alias("Name", make_type_expr("String"));

    let program = ast::Program {
        items: vec![ast::Item::TypeAlias(alias)],
        span: dummy_span(),
    };

    let result = checker.check_program(&program);
    assert!(result.is_ok(), "Type alias to String should succeed: {:?}", result);
}

#[test]
fn test_type_alias_to_bool() {
    // Test: type Flag = Bool
    let mut checker = TypeChecker::new();

    let alias = make_simple_type_alias("Flag", make_type_expr("Bool"));

    let program = ast::Program {
        items: vec![ast::Item::TypeAlias(alias)],
        span: dummy_span(),
    };

    let result = checker.check_program(&program);
    assert!(result.is_ok(), "Type alias to Bool should succeed: {:?}", result);
}

// =========================================================================
// Type Alias Usage Tests
// =========================================================================

#[test]
fn test_type_alias_in_function_param() {
    // Test:
    // type UserId = Int
    // fn get_user(id: UserId) -> Int { 0 }
    let mut checker = TypeChecker::new();

    let alias = make_simple_type_alias("UserId", make_type_expr("Int"));
    let func = make_simple_function("get_user", Some(make_type_expr("UserId")), Some(make_type_expr("Int")));

    let program = ast::Program {
        items: vec![
            ast::Item::TypeAlias(alias),
            ast::Item::Function(func),
        ],
        span: dummy_span(),
    };

    let result = checker.check_program(&program);
    assert!(result.is_ok(), "Type alias in function parameter should succeed: {:?}", result);
}

#[test]
fn test_type_alias_in_function_return() {
    // Test:
    // type UserId = Int
    // fn create_id() -> UserId { 0 }
    let mut checker = TypeChecker::new();

    let alias = make_simple_type_alias("UserId", make_type_expr("Int"));
    let func = make_simple_function("create_id", None, Some(make_type_expr("UserId")));

    let program = ast::Program {
        items: vec![
            ast::Item::TypeAlias(alias),
            ast::Item::Function(func),
        ],
        span: dummy_span(),
    };

    let result = checker.check_program(&program);
    assert!(result.is_ok(), "Type alias in function return should succeed: {:?}", result);
}

// =========================================================================
// Nested Type Alias Tests
// =========================================================================

#[test]
fn test_nested_type_alias() {
    // Test:
    // type UserId = Int
    // type AdminId = UserId
    let mut checker = TypeChecker::new();

    let alias1 = make_simple_type_alias("UserId", make_type_expr("Int"));
    let alias2 = make_simple_type_alias("AdminId", make_type_expr("UserId"));

    let program = ast::Program {
        items: vec![
            ast::Item::TypeAlias(alias1),
            ast::Item::TypeAlias(alias2),
        ],
        span: dummy_span(),
    };

    let result = checker.check_program(&program);
    assert!(result.is_ok(), "Nested type aliases should succeed: {:?}", result);
}

#[test]
fn test_deeply_nested_type_alias() {
    // Test:
    // type A = Int
    // type B = A
    // type C = B
    let mut checker = TypeChecker::new();

    let alias_a = make_simple_type_alias("A", make_type_expr("Int"));
    let alias_b = make_simple_type_alias("B", make_type_expr("A"));
    let alias_c = make_simple_type_alias("C", make_type_expr("B"));

    let program = ast::Program {
        items: vec![
            ast::Item::TypeAlias(alias_a),
            ast::Item::TypeAlias(alias_b),
            ast::Item::TypeAlias(alias_c),
        ],
        span: dummy_span(),
    };

    let result = checker.check_program(&program);
    assert!(result.is_ok(), "Deeply nested type aliases should succeed: {:?}", result);
}

// =========================================================================
// Generic Type Alias Tests
// =========================================================================

#[test]
fn test_generic_type_alias_simple() {
    // Test: type Container<T> = Array<T>
    let mut checker = TypeChecker::new();

    let alias = make_generic_type_alias(
        "Container",
        vec!["T"],
        make_generic_type_expr("Array", vec![make_type_expr("T")])
    );

    let program = ast::Program {
        items: vec![ast::Item::TypeAlias(alias)],
        span: dummy_span(),
    };

    let result = checker.check_program(&program);
    assert!(result.is_ok(), "Generic type alias should succeed: {:?}", result);
}

#[test]
fn test_generic_type_alias_pair() {
    // Test: type Pair<A, B> = (A, B)
    let mut checker = TypeChecker::new();

    let alias = make_generic_type_alias(
        "Pair",
        vec!["A", "B"],
        make_tuple_type_expr(vec![make_type_expr("A"), make_type_expr("B")])
    );

    let program = ast::Program {
        items: vec![ast::Item::TypeAlias(alias)],
        span: dummy_span(),
    };

    let result = checker.check_program(&program);
    assert!(result.is_ok(), "Generic pair type alias should succeed: {:?}", result);
}

#[test]
fn test_generic_type_alias_optional() {
    // Test: type Maybe<T> = T?
    let mut checker = TypeChecker::new();

    let alias = make_generic_type_alias(
        "Maybe",
        vec!["T"],
        make_optional_type_expr(make_type_expr("T"))
    );

    let program = ast::Program {
        items: vec![ast::Item::TypeAlias(alias)],
        span: dummy_span(),
    };

    let result = checker.check_program(&program);
    assert!(result.is_ok(), "Generic optional type alias should succeed: {:?}", result);
}

#[test]
fn test_generic_type_alias_instantiation() {
    // Test:
    // type Container<T> = Array<T>
    // fn get_first(items: Container<Int>) -> Int { 0 }
    let mut checker = TypeChecker::new();

    let alias = make_generic_type_alias(
        "Container",
        vec!["T"],
        make_generic_type_expr("Array", vec![make_type_expr("T")])
    );

    let func = make_simple_function(
        "get_first",
        Some(make_generic_type_expr("Container", vec![make_type_expr("Int")])),
        Some(make_type_expr("Int"))
    );

    let program = ast::Program {
        items: vec![
            ast::Item::TypeAlias(alias),
            ast::Item::Function(func),
        ],
        span: dummy_span(),
    };

    let result = checker.check_program(&program);
    assert!(result.is_ok(), "Generic type alias instantiation should succeed: {:?}", result);
}

#[test]
fn test_generic_type_alias_pair_instantiation() {
    // Test:
    // type Pair<A, B> = (A, B)
    // fn make_pair() -> Pair<Int, String> { ... }
    let mut checker = TypeChecker::new();

    let alias = make_generic_type_alias(
        "Pair",
        vec!["A", "B"],
        make_tuple_type_expr(vec![make_type_expr("A"), make_type_expr("B")])
    );

    // Create a function that returns a tuple (Int, String) which matches Pair<Int, String>
    let func = ast::FunctionDecl {
        attributes: vec![],
        visibility: Visibility::Private,
        name: make_ident("make_pair"),
        generic_params: None,
        params: vec![],
        return_type: Some(make_generic_type_expr("Pair", vec![make_type_expr("Int"), make_type_expr("String")])),
        where_clause: None,
        contracts: vec![],
        body: ast::FunctionBody::Expression(Box::new(ast::Expr::new(
            ast::ExprKind::Tuple(vec![
                ast::Expr::new(ast::ExprKind::Integer("0".into()), dummy_span()),
                ast::Expr::new(ast::ExprKind::String("hello".into()), dummy_span()),
            ]),
            dummy_span(),
        ))),
        test_block: None,
        span: dummy_span(),
    };

    let program = ast::Program {
        items: vec![
            ast::Item::TypeAlias(alias),
            ast::Item::Function(func),
        ],
        span: dummy_span(),
    };

    let result = checker.check_program(&program);
    assert!(result.is_ok(), "Generic pair type alias instantiation should succeed: {:?}", result);
}

// =========================================================================
// Complex Type Alias Tests
// =========================================================================

#[test]
fn test_type_alias_to_array() {
    // Test: type Numbers = [Int]
    let mut checker = TypeChecker::new();

    let alias = make_simple_type_alias(
        "Numbers",
        ast::TypeExpr::Array {
            element: Box::new(make_type_expr("Int")),
            size: None,
            span: dummy_span(),
        }
    );

    let program = ast::Program {
        items: vec![ast::Item::TypeAlias(alias)],
        span: dummy_span(),
    };

    let result = checker.check_program(&program);
    assert!(result.is_ok(), "Type alias to array should succeed: {:?}", result);
}

#[test]
fn test_type_alias_to_map() {
    // Test: type StringMap = {String: Int}
    let mut checker = TypeChecker::new();

    let alias = make_simple_type_alias(
        "StringMap",
        ast::TypeExpr::Map {
            key: Box::new(make_type_expr("String")),
            value: Box::new(make_type_expr("Int")),
            span: dummy_span(),
        }
    );

    let program = ast::Program {
        items: vec![ast::Item::TypeAlias(alias)],
        span: dummy_span(),
    };

    let result = checker.check_program(&program);
    assert!(result.is_ok(), "Type alias to map should succeed: {:?}", result);
}

#[test]
fn test_type_alias_to_tuple() {
    // Test: type Point = (Int, Int)
    let mut checker = TypeChecker::new();

    let alias = make_simple_type_alias(
        "Point",
        make_tuple_type_expr(vec![make_type_expr("Int"), make_type_expr("Int")])
    );

    let program = ast::Program {
        items: vec![ast::Item::TypeAlias(alias)],
        span: dummy_span(),
    };

    let result = checker.check_program(&program);
    assert!(result.is_ok(), "Type alias to tuple should succeed: {:?}", result);
}

#[test]
fn test_type_alias_to_optional() {
    // Test: type MaybeInt = Int?
    let mut checker = TypeChecker::new();

    let alias = make_simple_type_alias(
        "MaybeInt",
        make_optional_type_expr(make_type_expr("Int"))
    );

    let program = ast::Program {
        items: vec![ast::Item::TypeAlias(alias)],
        span: dummy_span(),
    };

    let result = checker.check_program(&program);
    assert!(result.is_ok(), "Type alias to optional should succeed: {:?}", result);
}

#[test]
fn test_type_alias_to_function() {
    // Test: type Callback = fn(Int) -> Int
    let mut checker = TypeChecker::new();

    let alias = make_simple_type_alias(
        "Callback",
        ast::TypeExpr::Function {
            params: vec![make_type_expr("Int")],
            return_type: Some(Box::new(make_type_expr("Int"))),
            span: dummy_span(),
        }
    );

    let program = ast::Program {
        items: vec![ast::Item::TypeAlias(alias)],
        span: dummy_span(),
    };

    let result = checker.check_program(&program);
    assert!(result.is_ok(), "Type alias to function should succeed: {:?}", result);
}

// =========================================================================
// Type Alias with Structs Tests
// =========================================================================

#[test]
fn test_type_alias_with_struct_field() {
    // Test:
    // type UserId = Int
    // struct User { id: UserId }
    let mut checker = TypeChecker::new();

    let alias = make_simple_type_alias("UserId", make_type_expr("Int"));

    let struct_decl = ast::StructDecl {
        attributes: vec![],
        visibility: Visibility::Private,
        name: make_type_ident("User"),
        generic_params: None,
        fields: vec![ast::StructField {
            visibility: Visibility::Private,
            name: make_ident("id"),
            ty: make_type_expr("UserId"),
            default: None,
            span: dummy_span(),
        }],
        derive: vec![],
        span: dummy_span(),
    };

    let program = ast::Program {
        items: vec![
            ast::Item::TypeAlias(alias),
            ast::Item::Struct(struct_decl),
        ],
        span: dummy_span(),
    };

    let result = checker.check_program(&program);
    assert!(result.is_ok(), "Type alias in struct field should succeed: {:?}", result);
}

// =========================================================================
// Multiple Type Aliases Tests
// =========================================================================

#[test]
fn test_multiple_type_aliases() {
    // Test multiple unrelated type aliases
    let mut checker = TypeChecker::new();

    let alias1 = make_simple_type_alias("UserId", make_type_expr("Int"));
    let alias2 = make_simple_type_alias("Name", make_type_expr("String"));
    let alias3 = make_simple_type_alias("Active", make_type_expr("Bool"));

    let program = ast::Program {
        items: vec![
            ast::Item::TypeAlias(alias1),
            ast::Item::TypeAlias(alias2),
            ast::Item::TypeAlias(alias3),
        ],
        span: dummy_span(),
    };

    let result = checker.check_program(&program);
    assert!(result.is_ok(), "Multiple type aliases should succeed: {:?}", result);
}

#[test]
fn test_generic_alias_with_nested_alias() {
    // Test:
    // type UserId = Int
    // type Container<T> = Array<T>
    // fn get_users() -> Container<UserId> { [] }
    let mut checker = TypeChecker::new();

    let user_alias = make_simple_type_alias("UserId", make_type_expr("Int"));
    let container_alias = make_generic_type_alias(
        "Container",
        vec!["T"],
        make_generic_type_expr("Array", vec![make_type_expr("T")])
    );

    // Create a function that returns an empty array which matches Container<UserId> = Array<Int>
    let func = ast::FunctionDecl {
        attributes: vec![],
        visibility: Visibility::Private,
        name: make_ident("get_users"),
        generic_params: None,
        params: vec![],
        return_type: Some(make_generic_type_expr("Container", vec![make_type_expr("UserId")])),
        where_clause: None,
        contracts: vec![],
        body: ast::FunctionBody::Expression(Box::new(ast::Expr::new(
            ast::ExprKind::Array(vec![]),
            dummy_span(),
        ))),
        test_block: None,
        span: dummy_span(),
    };

    let program = ast::Program {
        items: vec![
            ast::Item::TypeAlias(user_alias),
            ast::Item::TypeAlias(container_alias),
            ast::Item::Function(func),
        ],
        span: dummy_span(),
    };

    let result = checker.check_program(&program);
    assert!(result.is_ok(), "Generic alias with nested alias should succeed: {:?}", result);
}
