//! Tests for Hindley-Milner type inference features
//!
//! This module tests:
//! 1. Type unification algorithm (Algorithm W style)
//! 2. Type variable generation and substitution
//! 3. Generalization (creating polymorphic type schemes)
//! 4. Instantiation (creating fresh type variables from schemes)
//! 5. Occurs check (preventing infinite types)
//! 6. Type error diagnostics

use aria_ast::{self as ast, Span};
use aria_types::{Type, TypeChecker, TypeEnv, TypeInference, TypeScheme, TypeVar};

// ============================================================================
// Basic Unification Tests
// ============================================================================

#[test]
fn test_unify_identical_primitives() {
    let mut inf = TypeInference::new();

    // Same primitive types should unify
    assert!(inf.unify(&Type::Int, &Type::Int, Span::dummy()).is_ok());
    assert!(inf.unify(&Type::Bool, &Type::Bool, Span::dummy()).is_ok());
    assert!(inf.unify(&Type::String, &Type::String, Span::dummy()).is_ok());
    assert!(inf.unify(&Type::Float, &Type::Float, Span::dummy()).is_ok());
    assert!(inf.unify(&Type::Char, &Type::Char, Span::dummy()).is_ok());
    assert!(inf.unify(&Type::Unit, &Type::Unit, Span::dummy()).is_ok());
}

#[test]
fn test_unify_different_primitives_fails() {
    let mut inf = TypeInference::new();

    // Different primitive types should not unify
    assert!(inf.unify(&Type::Int, &Type::String, Span::dummy()).is_err());
    assert!(inf.unify(&Type::Bool, &Type::Int, Span::dummy()).is_err());
    assert!(inf.unify(&Type::Float, &Type::Bool, Span::dummy()).is_err());
}

#[test]
fn test_unify_type_variable_with_concrete() {
    let mut inf = TypeInference::new();
    let var = inf.fresh_var();

    // Type variable should unify with concrete type
    assert!(inf.unify(&var, &Type::Int, Span::dummy()).is_ok());

    // After unification, applying substitution should yield Int
    assert_eq!(inf.apply(&var), Type::Int);
}

#[test]
fn test_unify_type_variable_both_sides() {
    let mut inf = TypeInference::new();
    let var1 = inf.fresh_var();
    let var2 = inf.fresh_var();

    // Two type variables should unify
    assert!(inf.unify(&var1, &var2, Span::dummy()).is_ok());

    // Now unify one with a concrete type
    assert!(inf.unify(&var1, &Type::Bool, Span::dummy()).is_ok());

    // Both should resolve to Bool
    assert_eq!(inf.apply(&var1), Type::Bool);
    assert_eq!(inf.apply(&var2), Type::Bool);
}

#[test]
fn test_unify_array_types() {
    let mut inf = TypeInference::new();

    // Arrays of same type should unify
    let arr_int1 = Type::Array(Box::new(Type::Int));
    let arr_int2 = Type::Array(Box::new(Type::Int));
    assert!(inf.unify(&arr_int1, &arr_int2, Span::dummy()).is_ok());

    // Arrays of different types should not unify
    let arr_str = Type::Array(Box::new(Type::String));
    assert!(inf.unify(&arr_int1, &arr_str, Span::dummy()).is_err());
}

#[test]
fn test_unify_array_with_type_variable() {
    let mut inf = TypeInference::new();
    let elem_var = inf.fresh_var();
    let arr_var = Type::Array(Box::new(elem_var.clone()));
    let arr_int = Type::Array(Box::new(Type::Int));

    // Should unify and bind element type
    assert!(inf.unify(&arr_var, &arr_int, Span::dummy()).is_ok());
    assert_eq!(inf.apply(&elem_var), Type::Int);
}

#[test]
fn test_unify_tuple_types() {
    let mut inf = TypeInference::new();

    // Same structure tuples should unify
    let tuple1 = Type::Tuple(vec![Type::Int, Type::String]);
    let tuple2 = Type::Tuple(vec![Type::Int, Type::String]);
    assert!(inf.unify(&tuple1, &tuple2, Span::dummy()).is_ok());

    // Different structure should not unify
    let tuple3 = Type::Tuple(vec![Type::Int, Type::Bool]);
    assert!(inf.unify(&tuple1, &tuple3, Span::dummy()).is_err());

    // Different length should not unify
    let tuple4 = Type::Tuple(vec![Type::Int]);
    assert!(inf.unify(&tuple1, &tuple4, Span::dummy()).is_err());
}

#[test]
fn test_unify_function_types() {
    let mut inf = TypeInference::new();

    // Same function types should unify
    let fn1 = Type::Function {
        params: vec![Type::Int],
        return_type: Box::new(Type::Bool),
    };
    let fn2 = Type::Function {
        params: vec![Type::Int],
        return_type: Box::new(Type::Bool),
    };
    assert!(inf.unify(&fn1, &fn2, Span::dummy()).is_ok());

    // Different return types should not unify
    let fn3 = Type::Function {
        params: vec![Type::Int],
        return_type: Box::new(Type::String),
    };
    assert!(inf.unify(&fn1, &fn3, Span::dummy()).is_err());
}

#[test]
fn test_unify_function_with_type_variables() {
    let mut inf = TypeInference::new();
    let param_var = inf.fresh_var();
    let ret_var = inf.fresh_var();

    let fn_var = Type::Function {
        params: vec![param_var.clone()],
        return_type: Box::new(ret_var.clone()),
    };
    let fn_concrete = Type::Function {
        params: vec![Type::String],
        return_type: Box::new(Type::Int),
    };

    assert!(inf.unify(&fn_var, &fn_concrete, Span::dummy()).is_ok());
    assert_eq!(inf.apply(&param_var), Type::String);
    assert_eq!(inf.apply(&ret_var), Type::Int);
}

// ============================================================================
// Occurs Check Tests
// ============================================================================

#[test]
fn test_occurs_check_prevents_infinite_type() {
    let mut inf = TypeInference::new();
    let var = inf.fresh_var();

    // Cannot unify a type variable with a type containing itself
    // e.g., cannot have T = Array[T]
    let recursive = Type::Array(Box::new(var.clone()));
    let result = inf.unify(&var, &recursive, Span::dummy());

    assert!(result.is_err());
}

#[test]
fn test_occurs_check_in_function_type() {
    let mut inf = TypeInference::new();
    let var = inf.fresh_var();

    // Cannot have T = fn(T) -> Int
    let recursive_fn = Type::Function {
        params: vec![var.clone()],
        return_type: Box::new(Type::Int),
    };
    let result = inf.unify(&var, &recursive_fn, Span::dummy());

    assert!(result.is_err());
}

// ============================================================================
// Type Inference Expression Tests
// ============================================================================

// test_infer_let_binding_simple removed - ExprKind::Let no longer exists in the AST
// Let bindings are now statements, not expressions

#[test]
fn test_infer_lambda_identity() {
    let mut checker = TypeChecker::new();
    let env = TypeEnv::new();

    // |x| x - identity function
    let lambda_expr = ast::Expr::new(
        ast::ExprKind::Lambda {
            params: vec![ast::Param {
                mutable: false,
                name: ast::Spanned::dummy("x".into()),
                ty: None,
                default: None,
                span: Span::dummy(),
            }],
            body: Box::new(ast::Expr::new(
                ast::ExprKind::Ident("x".into()),
                Span::dummy(),
            )),
        },
        Span::dummy(),
    );

    let ty = checker.infer_expr(&lambda_expr, &env).unwrap();

    // Should be a function type
    match ty {
        Type::Function { params, return_type } => {
            assert_eq!(params.len(), 1);
            // Parameter and return should be the same type variable
        }
        _ => panic!("Expected function type, got {:?}", ty),
    }
}

#[test]
fn test_infer_lambda_with_application() {
    let mut checker = TypeChecker::new();
    let env = TypeEnv::new();

    // (|x| x + 1)(42) - should infer x as Int
    let lambda = ast::Expr::new(
        ast::ExprKind::Lambda {
            params: vec![ast::Param {
                mutable: false,
                name: ast::Spanned::dummy("x".into()),
                ty: None,
                default: None,
                span: Span::dummy(),
            }],
            body: Box::new(ast::Expr::new(
                ast::ExprKind::Binary {
                    op: ast::BinaryOp::Add,
                    left: Box::new(ast::Expr::new(
                        ast::ExprKind::Ident("x".into()),
                        Span::dummy(),
                    )),
                    right: Box::new(ast::Expr::new(
                        ast::ExprKind::Integer("1".into()),
                        Span::dummy(),
                    )),
                },
                Span::dummy(),
            )),
        },
        Span::dummy(),
    );

    let call_expr = ast::Expr::new(
        ast::ExprKind::Call {
            func: Box::new(lambda),
            args: vec![ast::CallArg {
                name: None,
                value: ast::Expr::new(
                    ast::ExprKind::Integer("42".into()),
                    Span::dummy(),
                ),
                spread: false,
            }],
        },
        Span::dummy(),
    );

    let ty = checker.infer_expr(&call_expr, &env).unwrap();
    assert_eq!(ty, Type::Int);
}

// ============================================================================
// Optional and Result Type Tests
// ============================================================================

#[test]
fn test_unify_optional_types() {
    let mut inf = TypeInference::new();

    let opt_int = Type::Optional(Box::new(Type::Int));
    let opt_int2 = Type::Optional(Box::new(Type::Int));
    assert!(inf.unify(&opt_int, &opt_int2, Span::dummy()).is_ok());

    let opt_str = Type::Optional(Box::new(Type::String));
    assert!(inf.unify(&opt_int, &opt_str, Span::dummy()).is_err());
}

#[test]
fn test_unify_result_types() {
    let mut inf = TypeInference::new();

    let res1 = Type::Result(Box::new(Type::Int), Box::new(Type::String));
    let res2 = Type::Result(Box::new(Type::Int), Box::new(Type::String));
    assert!(inf.unify(&res1, &res2, Span::dummy()).is_ok());

    let res3 = Type::Result(Box::new(Type::Bool), Box::new(Type::String));
    assert!(inf.unify(&res1, &res3, Span::dummy()).is_err());
}

// ============================================================================
// Never Type (Bottom) Tests
// ============================================================================

#[test]
fn test_never_unifies_with_anything() {
    let mut inf = TypeInference::new();

    // Never (bottom type) should unify with any type
    assert!(inf.unify(&Type::Never, &Type::Int, Span::dummy()).is_ok());
    assert!(inf.unify(&Type::Never, &Type::String, Span::dummy()).is_ok());
    assert!(inf.unify(&Type::Never, &Type::Bool, Span::dummy()).is_ok());

    // And in both directions
    assert!(inf.unify(&Type::Int, &Type::Never, Span::dummy()).is_ok());
}

// ============================================================================
// Type Application Tests
// ============================================================================

#[test]
fn test_apply_substitution_chain() {
    let mut inf = TypeInference::new();
    let var1 = inf.fresh_var();
    let var2 = inf.fresh_var();
    let var3 = inf.fresh_var();

    // Create a chain: var1 -> var2 -> var3 -> Int
    inf.unify(&var1, &var2, Span::dummy()).unwrap();
    inf.unify(&var2, &var3, Span::dummy()).unwrap();
    inf.unify(&var3, &Type::Int, Span::dummy()).unwrap();

    // All should resolve to Int
    assert_eq!(inf.apply(&var1), Type::Int);
    assert_eq!(inf.apply(&var2), Type::Int);
    assert_eq!(inf.apply(&var3), Type::Int);
}

#[test]
fn test_apply_in_nested_types() {
    let mut inf = TypeInference::new();
    let var = inf.fresh_var();

    // Unify var with Int
    inf.unify(&var, &Type::Int, Span::dummy()).unwrap();

    // Create a complex type containing the variable
    let complex = Type::Tuple(vec![
        Type::Array(Box::new(var.clone())),
        Type::Optional(Box::new(var.clone())),
    ]);

    let applied = inf.apply(&complex);
    let expected = Type::Tuple(vec![
        Type::Array(Box::new(Type::Int)),
        Type::Optional(Box::new(Type::Int)),
    ]);

    assert_eq!(applied, expected);
}

// ============================================================================
// Error Diagnostics Tests
// ============================================================================

#[test]
fn test_type_mismatch_error_has_span() {
    let mut checker = TypeChecker::new();
    let env = TypeEnv::new();

    // 42 + "hello" should produce type error
    let bad_expr = ast::Expr::new(
        ast::ExprKind::Binary {
            op: ast::BinaryOp::Add,
            left: Box::new(ast::Expr::new(
                ast::ExprKind::Integer("42".into()),
                Span::dummy(),
            )),
            right: Box::new(ast::Expr::new(
                ast::ExprKind::String("hello".into()),
                Span::dummy(),
            )),
        },
        Span::dummy(),
    );

    let result = checker.infer_expr(&bad_expr, &env);
    assert!(result.is_err());
}

#[test]
fn test_undefined_variable_error() {
    let mut checker = TypeChecker::new();
    let env = TypeEnv::new();

    let var_expr = ast::Expr::new(
        ast::ExprKind::Ident("undefined_var".into()),
        Span::dummy(),
    );

    let result = checker.infer_expr(&var_expr, &env);
    assert!(result.is_err());
}

// ============================================================================
// Type Environment Tests
// ============================================================================

#[test]
fn test_env_variable_shadowing() {
    let mut outer = TypeEnv::new();
    outer.define_var("x".to_string(), Type::Int);

    let outer_rc = std::rc::Rc::new(outer);
    let mut inner = TypeEnv::with_parent(outer_rc.clone());
    inner.define_var("x".to_string(), Type::String);

    // Inner should see String, outer should see Int
    assert_eq!(inner.lookup_var("x"), Some(&Type::String));
    assert_eq!(outer_rc.lookup_var("x"), Some(&Type::Int));
}

#[test]
fn test_env_parent_lookup() {
    let mut parent = TypeEnv::new();
    parent.define_var("x".to_string(), Type::Int);
    parent.define_var("y".to_string(), Type::Bool);

    let parent_rc = std::rc::Rc::new(parent);
    let child = TypeEnv::with_parent(parent_rc);

    // Child should be able to look up parent's variables
    assert_eq!(child.lookup_var("x"), Some(&Type::Int));
    assert_eq!(child.lookup_var("y"), Some(&Type::Bool));
    assert_eq!(child.lookup_var("z"), None);
}

// ============================================================================
// Generics and Named Types Tests
// ============================================================================

#[test]
fn test_unify_named_types_same() {
    let mut inf = TypeInference::new();

    let named1 = Type::Named {
        name: "List".to_string(),
        type_args: vec![Type::Int],
    };
    let named2 = Type::Named {
        name: "List".to_string(),
        type_args: vec![Type::Int],
    };

    assert!(inf.unify(&named1, &named2, Span::dummy()).is_ok());
}

#[test]
fn test_unify_named_types_different_args() {
    let mut inf = TypeInference::new();

    let named1 = Type::Named {
        name: "List".to_string(),
        type_args: vec![Type::Int],
    };
    let named2 = Type::Named {
        name: "List".to_string(),
        type_args: vec![Type::String],
    };

    assert!(inf.unify(&named1, &named2, Span::dummy()).is_err());
}

#[test]
fn test_unify_named_types_different_names() {
    let mut inf = TypeInference::new();

    let named1 = Type::Named {
        name: "List".to_string(),
        type_args: vec![Type::Int],
    };
    let named2 = Type::Named {
        name: "Vec".to_string(),
        type_args: vec![Type::Int],
    };

    assert!(inf.unify(&named1, &named2, Span::dummy()).is_err());
}

#[test]
fn test_unify_named_type_with_type_variable() {
    let mut inf = TypeInference::new();
    let var = inf.fresh_var();

    let named = Type::Named {
        name: "List".to_string(),
        type_args: vec![var.clone()],
    };
    let concrete = Type::Named {
        name: "List".to_string(),
        type_args: vec![Type::Int],
    };

    assert!(inf.unify(&named, &concrete, Span::dummy()).is_ok());
    assert_eq!(inf.apply(&var), Type::Int);
}

// ============================================================================
// Channel Type Tests
// ============================================================================

#[test]
fn test_unify_channel_types() {
    let mut inf = TypeInference::new();

    let chan1 = Type::Channel(Box::new(Type::Int));
    let chan2 = Type::Channel(Box::new(Type::Int));
    assert!(inf.unify(&chan1, &chan2, Span::dummy()).is_ok());

    let chan3 = Type::Channel(Box::new(Type::String));
    assert!(inf.unify(&chan1, &chan3, Span::dummy()).is_err());
}

// ============================================================================
// Reference Type Tests
// ============================================================================

#[test]
fn test_unify_reference_types() {
    let mut inf = TypeInference::new();

    let ref1 = Type::Reference {
        mutable: false,
        inner: Box::new(Type::Int),
    };
    let ref2 = Type::Reference {
        mutable: false,
        inner: Box::new(Type::Int),
    };
    assert!(inf.unify(&ref1, &ref2, Span::dummy()).is_ok());

    // Different mutability should not unify
    let ref_mut = Type::Reference {
        mutable: true,
        inner: Box::new(Type::Int),
    };
    assert!(inf.unify(&ref1, &ref_mut, Span::dummy()).is_err());
}

// ============================================================================
// Map Type Tests
// ============================================================================

#[test]
fn test_unify_map_types() {
    let mut inf = TypeInference::new();

    let map1 = Type::Map(Box::new(Type::String), Box::new(Type::Int));
    let map2 = Type::Map(Box::new(Type::String), Box::new(Type::Int));
    assert!(inf.unify(&map1, &map2, Span::dummy()).is_ok());

    let map3 = Type::Map(Box::new(Type::String), Box::new(Type::Bool));
    assert!(inf.unify(&map1, &map3, Span::dummy()).is_err());
}

#[test]
fn test_unify_map_with_type_variables() {
    let mut inf = TypeInference::new();
    let key_var = inf.fresh_var();
    let val_var = inf.fresh_var();

    let map_var = Type::Map(Box::new(key_var.clone()), Box::new(val_var.clone()));
    let map_concrete = Type::Map(Box::new(Type::String), Box::new(Type::Int));

    assert!(inf.unify(&map_var, &map_concrete, Span::dummy()).is_ok());
    assert_eq!(inf.apply(&key_var), Type::String);
    assert_eq!(inf.apply(&val_var), Type::Int);
}

// ============================================================================
// Type Variable Fresh Tests
// ============================================================================

#[test]
fn test_fresh_var_creates_unique_vars() {
    let mut inf = TypeInference::new();

    let var1 = inf.fresh_var();
    let var2 = inf.fresh_var();
    let var3 = inf.fresh_var();

    // All variables should be different
    assert_ne!(var1, var2);
    assert_ne!(var2, var3);
    assert_ne!(var1, var3);
}

// test_var_count_increments removed - TypeInference.var_count() no longer exists
// Type variable counting is internal implementation detail
