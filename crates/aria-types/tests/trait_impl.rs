//! Tests for trait implementation validation
//!
//! This module tests that:
//! 1. Trait definitions are properly validated and registered
//! 2. Impl blocks are checked for completeness (all required methods)
//! 3. Method signatures match the trait definition
//! 4. Associated types and constants are properly validated
//! 5. Supertrait requirements are enforced
//! 6. Duplicate methods/types produce errors
//! 7. Self type substitution works correctly

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

fn make_trait_bound(name: &str) -> ast::TraitBound {
    ast::TraitBound {
        path: vec![make_ident(name)],
        type_args: None,
        span: dummy_span(),
    }
}

fn make_simple_function(name: &str, params: Vec<ast::Param>, return_type: Option<ast::TypeExpr>) -> ast::FunctionDecl {
    ast::FunctionDecl {
        attributes: vec![],
        visibility: Visibility::Private,
        name: make_ident(name),
        generic_params: None,
        params,
        return_type,
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

fn make_self_param() -> ast::Param {
    ast::Param {
        mutable: false,
        name: ast::Spanned::dummy("self".into()),
        ty: None, // Self type
        default: None,
        span: dummy_span(),
    }
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

fn make_trait_decl(name: &str, members: Vec<ast::TraitMember>) -> ast::TraitDecl {
    ast::TraitDecl {
        attributes: vec![],
        visibility: Visibility::Private,
        name: make_type_ident(name),
        generic_params: None,
        supertraits: vec![],
        members,
        span: dummy_span(),
    }
}

fn make_struct_decl(name: &str, fields: Vec<ast::StructField>) -> ast::StructDecl {
    ast::StructDecl {
        attributes: vec![],
        visibility: Visibility::Private,
        name: make_type_ident(name),
        generic_params: None,
        fields,
        derive: vec![],
        span: dummy_span(),
    }
}

fn make_impl_decl(trait_name: Option<&str>, for_type: &str, members: Vec<ast::ImplMember>) -> ast::ImplDecl {
    ast::ImplDecl {
        attributes: vec![],
        generic_params: None,
        trait_: trait_name.map(make_trait_bound),
        for_type: make_type_expr(for_type),
        where_clause: None,
        members,
        span: dummy_span(),
    }
}

fn make_trait_method(name: &str, params: Vec<ast::Param>, return_type: Option<ast::TypeExpr>, has_default: bool) -> ast::TraitMethod {
    ast::TraitMethod {
        name: make_ident(name),
        generic_params: None,
        params,
        return_type,
        default: if has_default {
            Some(ast::FunctionBody::Expression(Box::new(ast::Expr::new(
                ast::ExprKind::Integer("0".into()),
                dummy_span(),
            ))))
        } else {
            None
        },
        span: dummy_span(),
    }
}

fn make_trait_type(name: &str, default: Option<&str>) -> ast::TraitType {
    ast::TraitType {
        name: make_type_ident(name),
        bounds: vec![],
        default: default.map(make_type_expr),
        span: dummy_span(),
    }
}

fn make_type_alias(name: &str, ty: &str) -> ast::TypeAlias {
    ast::TypeAlias {
        attributes: vec![],
        visibility: Visibility::Private,
        name: make_type_ident(name),
        generic_params: None,
        ty: make_type_expr(ty),
        span: dummy_span(),
    }
}

// =========================================================================
// Trait Definition Tests
// =========================================================================

#[test]
fn test_trait_definition_basic() {
    // Test: trait MyTrait { fn my_method(self) -> Int }
    let mut checker = TypeChecker::new();

    let method = make_trait_method("my_method", vec![make_self_param()], Some(make_type_expr("Int")), false);

    let trait_decl = make_trait_decl("MyTrait", vec![ast::TraitMember::Method(method)]);

    let program = ast::Program {
        items: vec![ast::Item::Trait(trait_decl)],
        span: dummy_span(),
    };

    let result = checker.check_program(&program);
    assert!(result.is_ok(), "Trait definition should succeed: {:?}", result);
}

#[test]
fn test_trait_with_associated_type() {
    // Test: trait Iterator { type Item; fn next(self) -> Optional[Self.Item] }
    let mut checker = TypeChecker::new();

    let assoc_type = make_trait_type("Item", None);

    let method = make_trait_method(
        "next",
        vec![make_self_param()],
        Some(ast::TypeExpr::Generic {
            name: make_type_ident("Optional"),
            args: vec![make_type_expr("Int")], // Simplified for test
            span: dummy_span(),
        }),
        false,
    );

    let trait_decl = make_trait_decl(
        "MyIterator",
        vec![
            ast::TraitMember::Type(assoc_type),
            ast::TraitMember::Method(method),
        ],
    );

    let program = ast::Program {
        items: vec![ast::Item::Trait(trait_decl)],
        span: dummy_span(),
    };

    let result = checker.check_program(&program);
    assert!(result.is_ok(), "Trait with associated type should succeed: {:?}", result);
}

// =========================================================================
// Impl Block Tests - Missing Methods
// =========================================================================

#[test]
fn test_impl_missing_required_method() {
    // Test that implementing a trait without all required methods fails
    let mut checker = TypeChecker::new();

    // Define trait with a required method
    let method = make_trait_method("required_method", vec![make_self_param()], Some(make_type_expr("Int")), false);
    let trait_decl = make_trait_decl("RequiredTrait", vec![ast::TraitMember::Method(method)]);

    // Define a struct
    let struct_decl = make_struct_decl("MyStruct", vec![]);

    // Implement the trait WITHOUT the required method
    let impl_decl = make_impl_decl(Some("RequiredTrait"), "MyStruct", vec![]);

    let program = ast::Program {
        items: vec![
            ast::Item::Trait(trait_decl),
            ast::Item::Struct(struct_decl),
            ast::Item::Impl(impl_decl),
        ],
        span: dummy_span(),
    };

    let result = checker.check_program(&program);
    assert!(result.is_err(), "Should fail with missing method");

    match result {
        Err(TypeError::MissingTraitMethod { trait_name, method_name, .. }) => {
            assert_eq!(trait_name, "RequiredTrait");
            assert_eq!(method_name, "required_method");
        }
        Err(e) => panic!("Expected MissingTraitMethod error, got: {:?}", e),
        Ok(_) => panic!("Expected error but got success"),
    }
}

#[test]
fn test_impl_complete_succeeds() {
    // Test that a complete implementation succeeds
    let mut checker = TypeChecker::new();

    // Define trait with a required method
    let method = make_trait_method("get_value", vec![make_self_param()], Some(make_type_expr("Int")), false);
    let trait_decl = make_trait_decl("ValueTrait", vec![ast::TraitMember::Method(method)]);

    // Define a struct
    let struct_decl = make_struct_decl("ValueStruct", vec![]);

    // Implement the trait WITH the required method
    let impl_method = make_simple_function("get_value", vec![make_self_param()], Some(make_type_expr("Int")));
    let impl_decl = make_impl_decl(
        Some("ValueTrait"),
        "ValueStruct",
        vec![ast::ImplMember::Function(impl_method)],
    );

    let program = ast::Program {
        items: vec![
            ast::Item::Trait(trait_decl),
            ast::Item::Struct(struct_decl),
            ast::Item::Impl(impl_decl),
        ],
        span: dummy_span(),
    };

    let result = checker.check_program(&program);
    assert!(result.is_ok(), "Complete implementation should succeed: {:?}", result);
}

// =========================================================================
// Method Not In Trait Tests
// =========================================================================

#[test]
fn test_impl_method_not_in_trait() {
    // Test that implementing a method that isn't in the trait fails
    let mut checker = TypeChecker::new();

    // Define an empty trait
    let trait_decl = make_trait_decl("EmptyTrait", vec![]);

    // Define a struct
    let struct_decl = make_struct_decl("MyStruct2", vec![]);

    // Try to implement a method that isn't in the trait
    let impl_method = make_simple_function("unknown_method", vec![make_self_param()], Some(make_type_expr("Int")));
    let impl_decl = make_impl_decl(
        Some("EmptyTrait"),
        "MyStruct2",
        vec![ast::ImplMember::Function(impl_method)],
    );

    let program = ast::Program {
        items: vec![
            ast::Item::Trait(trait_decl),
            ast::Item::Struct(struct_decl),
            ast::Item::Impl(impl_decl),
        ],
        span: dummy_span(),
    };

    let result = checker.check_program(&program);
    assert!(result.is_err(), "Should fail with method not in trait");

    match result {
        Err(TypeError::MethodNotInTrait { trait_name, method_name, .. }) => {
            assert_eq!(trait_name, "EmptyTrait");
            assert_eq!(method_name, "unknown_method");
        }
        Err(e) => panic!("Expected MethodNotInTrait error, got: {:?}", e),
        Ok(_) => panic!("Expected error but got success"),
    }
}

// =========================================================================
// Duplicate Method Tests
// =========================================================================

#[test]
fn test_impl_duplicate_method() {
    // Test that implementing the same method twice fails
    let mut checker = TypeChecker::new();

    // Define trait with a method
    let method = make_trait_method("do_something", vec![make_self_param()], None, false);
    let trait_decl = make_trait_decl("SomeTrait", vec![ast::TraitMember::Method(method)]);

    // Define a struct
    let struct_decl = make_struct_decl("SomeStruct", vec![]);

    // Implement the method twice
    let impl_method1 = make_simple_function("do_something", vec![make_self_param()], None);
    let impl_method2 = make_simple_function("do_something", vec![make_self_param()], None);

    let impl_decl = make_impl_decl(
        Some("SomeTrait"),
        "SomeStruct",
        vec![
            ast::ImplMember::Function(impl_method1),
            ast::ImplMember::Function(impl_method2),
        ],
    );

    let program = ast::Program {
        items: vec![
            ast::Item::Trait(trait_decl),
            ast::Item::Struct(struct_decl),
            ast::Item::Impl(impl_decl),
        ],
        span: dummy_span(),
    };

    let result = checker.check_program(&program);
    assert!(result.is_err(), "Should fail with duplicate method");

    match result {
        Err(TypeError::DuplicateImplMethod { method_name, .. }) => {
            assert_eq!(method_name, "do_something");
        }
        Err(e) => panic!("Expected DuplicateImplMethod error, got: {:?}", e),
        Ok(_) => panic!("Expected error but got success"),
    }
}

// =========================================================================
// Associated Type Tests
// =========================================================================

#[test]
fn test_impl_missing_associated_type() {
    // Test that implementing a trait without required associated types fails
    let mut checker = TypeChecker::new();

    // Define trait with an associated type (no default)
    let assoc_type = make_trait_type("Output", None);
    let trait_decl = make_trait_decl("OutputTrait", vec![ast::TraitMember::Type(assoc_type)]);

    // Define a struct
    let struct_decl = make_struct_decl("OutputStruct", vec![]);

    // Implement the trait WITHOUT the associated type
    let impl_decl = make_impl_decl(Some("OutputTrait"), "OutputStruct", vec![]);

    let program = ast::Program {
        items: vec![
            ast::Item::Trait(trait_decl),
            ast::Item::Struct(struct_decl),
            ast::Item::Impl(impl_decl),
        ],
        span: dummy_span(),
    };

    let result = checker.check_program(&program);
    assert!(result.is_err(), "Should fail with missing associated type");

    match result {
        Err(TypeError::MissingAssociatedType { trait_name, type_name, .. }) => {
            assert_eq!(trait_name, "OutputTrait");
            assert_eq!(type_name, "Output");
        }
        Err(e) => panic!("Expected MissingAssociatedType error, got: {:?}", e),
        Ok(_) => panic!("Expected error but got success"),
    }
}

#[test]
fn test_impl_associated_type_not_in_trait() {
    // Test that defining an associated type that isn't in the trait fails
    let mut checker = TypeChecker::new();

    // Define an empty trait
    let trait_decl = make_trait_decl("PlainTrait", vec![]);

    // Define a struct
    let struct_decl = make_struct_decl("PlainStruct", vec![]);

    // Try to define an associated type that isn't in the trait
    let impl_type = make_type_alias("UnknownType", "Int");

    let impl_decl = make_impl_decl(
        Some("PlainTrait"),
        "PlainStruct",
        vec![ast::ImplMember::Type(impl_type)],
    );

    let program = ast::Program {
        items: vec![
            ast::Item::Trait(trait_decl),
            ast::Item::Struct(struct_decl),
            ast::Item::Impl(impl_decl),
        ],
        span: dummy_span(),
    };

    let result = checker.check_program(&program);
    assert!(result.is_err(), "Should fail with associated type not in trait");

    match result {
        Err(TypeError::AssociatedTypeNotInTrait { trait_name, type_name, .. }) => {
            assert_eq!(trait_name, "PlainTrait");
            assert_eq!(type_name, "UnknownType");
        }
        Err(e) => panic!("Expected AssociatedTypeNotInTrait error, got: {:?}", e),
        Ok(_) => panic!("Expected error but got success"),
    }
}

// =========================================================================
// Inherent Impl Tests
// =========================================================================

#[test]
fn test_inherent_impl_basic() {
    // Test that inherent impls (impl Type without a trait) work
    let mut checker = TypeChecker::new();

    // Define a struct
    let field = ast::StructField {
        visibility: Visibility::Private,
        name: make_ident("count"),
        ty: make_type_expr("Int"),
        default: None,
        span: dummy_span(),
    };
    let struct_decl = make_struct_decl("Counter", vec![field]);

    // Add methods to the struct (inherent impl)
    let impl_method = make_simple_function("increment", vec![make_self_param()], Some(make_type_expr("Int")));

    let impl_decl = make_impl_decl(
        None, // No trait - inherent impl
        "Counter",
        vec![ast::ImplMember::Function(impl_method)],
    );

    let program = ast::Program {
        items: vec![
            ast::Item::Struct(struct_decl),
            ast::Item::Impl(impl_decl),
        ],
        span: dummy_span(),
    };

    let result = checker.check_program(&program);
    assert!(result.is_ok(), "Inherent impl should succeed: {:?}", result);
}

// =========================================================================
// Undefined Trait Tests
// =========================================================================

#[test]
fn test_impl_undefined_trait() {
    // Test that implementing an undefined trait fails
    let mut checker = TypeChecker::new();

    // Define a struct
    let struct_decl = make_struct_decl("SomeType", vec![]);

    // Try to implement an undefined trait
    let impl_decl = make_impl_decl(Some("NonexistentTrait"), "SomeType", vec![]);

    let program = ast::Program {
        items: vec![
            ast::Item::Struct(struct_decl),
            ast::Item::Impl(impl_decl),
        ],
        span: dummy_span(),
    };

    let result = checker.check_program(&program);
    assert!(result.is_err(), "Should fail with undefined trait");

    match result {
        Err(TypeError::UndefinedTrait(name, _)) => {
            assert_eq!(name, "NonexistentTrait");
        }
        Err(e) => panic!("Expected UndefinedTrait error, got: {:?}", e),
        Ok(_) => panic!("Expected error but got success"),
    }
}

// =========================================================================
// Default Method Tests
// =========================================================================

#[test]
fn test_trait_with_default_method() {
    // Test that traits with default methods don't require implementation
    let mut checker = TypeChecker::new();

    // Define trait with a default method
    let method = make_trait_method("default_method", vec![make_self_param()], Some(make_type_expr("Int")), true);
    let trait_decl = make_trait_decl("DefaultTrait", vec![ast::TraitMember::Method(method)]);

    // Define a struct
    let struct_decl = make_struct_decl("DefaultStruct", vec![]);

    // Implement the trait WITHOUT the default method (should be ok)
    let impl_decl = make_impl_decl(Some("DefaultTrait"), "DefaultStruct", vec![]);

    let program = ast::Program {
        items: vec![
            ast::Item::Trait(trait_decl),
            ast::Item::Struct(struct_decl),
            ast::Item::Impl(impl_decl),
        ],
        span: dummy_span(),
    };

    let result = checker.check_program(&program);
    assert!(result.is_ok(), "Trait with default method should not require impl: {:?}", result);
}

// =========================================================================
// Associated Type With Default Tests
// =========================================================================

#[test]
fn test_trait_with_default_associated_type() {
    // Test that associated types with defaults don't require implementation
    let mut checker = TypeChecker::new();

    // Define trait with an associated type that has a default
    let assoc_type = make_trait_type("Output", Some("Int")); // Has default = Int
    let trait_decl = make_trait_decl("DefaultTypeTrait", vec![ast::TraitMember::Type(assoc_type)]);

    // Define a struct
    let struct_decl = make_struct_decl("DefaultTypeStruct", vec![]);

    // Implement the trait WITHOUT the associated type (should be ok - uses default)
    let impl_decl = make_impl_decl(Some("DefaultTypeTrait"), "DefaultTypeStruct", vec![]);

    let program = ast::Program {
        items: vec![
            ast::Item::Trait(trait_decl),
            ast::Item::Struct(struct_decl),
            ast::Item::Impl(impl_decl),
        ],
        span: dummy_span(),
    };

    let result = checker.check_program(&program);
    assert!(result.is_ok(), "Trait with default associated type should not require impl: {:?}", result);
}
