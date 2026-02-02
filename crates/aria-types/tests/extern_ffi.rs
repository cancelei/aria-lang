//! Tests for extern/FFI type checking
//!
//! This module tests that:
//! 1. Valid extern C declarations type-check successfully
//! 2. Extern functions are registered in the type environment
//! 3. Extern structs are registered with correct field types
//! 4. Invalid FFI types produce appropriate errors

use aria_ast as ast;
use aria_ast::Span;
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

// =========================================================================
// Extern C Function Tests
// =========================================================================

#[test]
fn test_extern_c_function_basic() {
    // Test: extern "C" "stdio.h" { fn printf(format: *const char) -> int }
    let mut checker = TypeChecker::new();

    let extern_func = ast::ExternFunction {
        name: make_ident("printf"),
        params: vec![
            ast::ExternParam {
                name: Some(make_ident("format")),
                ty: ast::CType::Pointer {
                    const_: true,
                    pointee: Box::new(ast::CType::Char),
                },
            },
        ],
        return_type: Some(ast::CType::Int),
        span: dummy_span(),
    };

    let extern_c = ast::ExternC {
        header: "stdio.h".into(),
        alias: None,
        items: vec![ast::ExternCItem::Function(extern_func)],
        span: dummy_span(),
    };

    let extern_decl = ast::ExternDecl::C(extern_c);

    // Should succeed
    let result = checker.check_program(&ast::Program {
        items: vec![ast::Item::Extern(extern_decl)],
        span: dummy_span(),
    });
    assert!(result.is_ok());
}

#[test]
fn test_extern_c_function_registers_type() {
    // Test that extern functions are registered in the type environment
    let mut checker = TypeChecker::new();

    // Use a simple function with no parameters to test registration
    let extern_func = ast::ExternFunction {
        name: make_ident("get_value"),
        params: vec![],
        return_type: Some(ast::CType::Int),
        span: dummy_span(),
    };

    let extern_c = ast::ExternC {
        header: "mylib.h".into(),
        alias: None,
        items: vec![ast::ExternCItem::Function(extern_func)],
        span: dummy_span(),
    };

    let extern_decl = ast::ExternDecl::C(extern_c);

    let program = ast::Program {
        items: vec![ast::Item::Extern(extern_decl)],
        span: dummy_span(),
    };

    checker.check_program(&program).unwrap();

    // Now test that calling get_value works
    // The function should be registered with type: fn() -> Int32
    let call_expr = ast::Expr::new(
        ast::ExprKind::Call {
            func: Box::new(ast::Expr::new(
                ast::ExprKind::Ident("get_value".into()),
                dummy_span(),
            )),
            args: vec![],
        },
        dummy_span(),
    );

    let env = aria_types::TypeEnv::new();
    // The call should type-check (returns Int32)
    let result = checker.infer_expr(&call_expr, &env);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Type::Int32);
}

#[test]
fn test_extern_c_function_void_return() {
    // Test extern function with void return type
    let mut checker = TypeChecker::new();

    let extern_func = ast::ExternFunction {
        name: make_ident("exit"),
        params: vec![
            ast::ExternParam {
                name: Some(make_ident("code")),
                ty: ast::CType::Int,
            },
        ],
        return_type: Some(ast::CType::Void),
        span: dummy_span(),
    };

    let extern_c = ast::ExternC {
        header: "stdlib.h".into(),
        alias: None,
        items: vec![ast::ExternCItem::Function(extern_func)],
        span: dummy_span(),
    };

    let extern_decl = ast::ExternDecl::C(extern_c);

    let result = checker.check_program(&ast::Program {
        items: vec![ast::Item::Extern(extern_decl)],
        span: dummy_span(),
    });
    assert!(result.is_ok());
}

#[test]
fn test_extern_c_function_no_return_type() {
    // Test extern function with no return type (implicitly void)
    let mut checker = TypeChecker::new();

    let extern_func = ast::ExternFunction {
        name: make_ident("do_something"),
        params: vec![],
        return_type: None,
        span: dummy_span(),
    };

    let extern_c = ast::ExternC {
        header: "mylib.h".into(),
        alias: None,
        items: vec![ast::ExternCItem::Function(extern_func)],
        span: dummy_span(),
    };

    let extern_decl = ast::ExternDecl::C(extern_c);

    let result = checker.check_program(&ast::Program {
        items: vec![ast::Item::Extern(extern_decl)],
        span: dummy_span(),
    });
    assert!(result.is_ok());
}

// =========================================================================
// Extern C Struct Tests
// =========================================================================

#[test]
fn test_extern_c_struct_basic() {
    // Test: extern "C" "sys/types.h" { struct Point { x: int, y: int } }
    let mut checker = TypeChecker::new();

    let extern_struct = ast::ExternStruct {
        name: make_type_ident("Point"),
        fields: vec![
            ast::ExternField {
                name: make_ident("x"),
                ty: ast::CType::Int,
            },
            ast::ExternField {
                name: make_ident("y"),
                ty: ast::CType::Int,
            },
        ],
        span: dummy_span(),
    };

    let extern_c = ast::ExternC {
        header: "geometry.h".into(),
        alias: None,
        items: vec![ast::ExternCItem::Struct(extern_struct)],
        span: dummy_span(),
    };

    let extern_decl = ast::ExternDecl::C(extern_c);

    let result = checker.check_program(&ast::Program {
        items: vec![ast::Item::Extern(extern_decl)],
        span: dummy_span(),
    });
    assert!(result.is_ok());
}

#[test]
fn test_extern_c_struct_with_pointers() {
    // Test struct with pointer fields
    let mut checker = TypeChecker::new();

    let extern_struct = ast::ExternStruct {
        name: make_type_ident("Node"),
        fields: vec![
            ast::ExternField {
                name: make_ident("data"),
                ty: ast::CType::Int,
            },
            ast::ExternField {
                name: make_ident("next"),
                ty: ast::CType::Pointer {
                    const_: false,
                    pointee: Box::new(ast::CType::Named(make_type_ident("Node"))),
                },
            },
        ],
        span: dummy_span(),
    };

    let extern_c = ast::ExternC {
        header: "list.h".into(),
        alias: None,
        items: vec![ast::ExternCItem::Struct(extern_struct)],
        span: dummy_span(),
    };

    let extern_decl = ast::ExternDecl::C(extern_c);

    let result = checker.check_program(&ast::Program {
        items: vec![ast::Item::Extern(extern_decl)],
        span: dummy_span(),
    });
    assert!(result.is_ok());
}

// =========================================================================
// Extern C Const Tests
// =========================================================================

#[test]
fn test_extern_c_const_basic() {
    // Test: extern "C" "limits.h" { const INT_MAX: int }
    let mut checker = TypeChecker::new();

    let extern_const = ast::ExternConst {
        name: make_ident("INT_MAX"),
        ty: ast::CType::Int,
        span: dummy_span(),
    };

    let extern_c = ast::ExternC {
        header: "limits.h".into(),
        alias: None,
        items: vec![ast::ExternCItem::Const(extern_const)],
        span: dummy_span(),
    };

    let extern_decl = ast::ExternDecl::C(extern_c);

    let result = checker.check_program(&ast::Program {
        items: vec![ast::Item::Extern(extern_decl)],
        span: dummy_span(),
    });
    assert!(result.is_ok());
}

// =========================================================================
// C Type to Aria Type Conversion Tests
// =========================================================================

#[test]
fn test_c_type_int_to_int32() {
    // Test that C int maps to Aria Int32
    let mut checker = TypeChecker::new();

    let extern_func = ast::ExternFunction {
        name: make_ident("get_int"),
        params: vec![],
        return_type: Some(ast::CType::Int),
        span: dummy_span(),
    };

    let extern_c = ast::ExternC {
        header: "test.h".into(),
        alias: None,
        items: vec![ast::ExternCItem::Function(extern_func)],
        span: dummy_span(),
    };

    checker.check_program(&ast::Program {
        items: vec![ast::Item::Extern(ast::ExternDecl::C(extern_c))],
        span: dummy_span(),
    }).unwrap();

    // Verify the registered type
    let call = ast::Expr::new(
        ast::ExprKind::Call {
            func: Box::new(ast::Expr::new(
                ast::ExprKind::Ident("get_int".into()),
                dummy_span(),
            )),
            args: vec![],
        },
        dummy_span(),
    );

    let result = checker.infer_expr(&call, &aria_types::TypeEnv::new()).unwrap();
    assert_eq!(result, Type::Int32);
}

#[test]
fn test_c_type_double_to_float64() {
    // Test that C double maps to Aria Float64
    let mut checker = TypeChecker::new();

    let extern_func = ast::ExternFunction {
        name: make_ident("get_double"),
        params: vec![],
        return_type: Some(ast::CType::Double),
        span: dummy_span(),
    };

    let extern_c = ast::ExternC {
        header: "test.h".into(),
        alias: None,
        items: vec![ast::ExternCItem::Function(extern_func)],
        span: dummy_span(),
    };

    checker.check_program(&ast::Program {
        items: vec![ast::Item::Extern(ast::ExternDecl::C(extern_c))],
        span: dummy_span(),
    }).unwrap();

    let call = ast::Expr::new(
        ast::ExprKind::Call {
            func: Box::new(ast::Expr::new(
                ast::ExprKind::Ident("get_double".into()),
                dummy_span(),
            )),
            args: vec![],
        },
        dummy_span(),
    );

    let result = checker.infer_expr(&call, &aria_types::TypeEnv::new()).unwrap();
    assert_eq!(result, Type::Float64);
}

#[test]
fn test_c_type_size_t_to_uint64() {
    // Test that C size_t maps to Aria UInt64
    // Use a simple function without parameters to test the return type mapping
    let mut checker = TypeChecker::new();

    let extern_func = ast::ExternFunction {
        name: make_ident("get_size"),
        params: vec![],
        return_type: Some(ast::CType::SizeT),
        span: dummy_span(),
    };

    let extern_c = ast::ExternC {
        header: "string.h".into(),
        alias: None,
        items: vec![ast::ExternCItem::Function(extern_func)],
        span: dummy_span(),
    };

    checker.check_program(&ast::Program {
        items: vec![ast::Item::Extern(ast::ExternDecl::C(extern_c))],
        span: dummy_span(),
    }).unwrap();

    let call = ast::Expr::new(
        ast::ExprKind::Call {
            func: Box::new(ast::Expr::new(
                ast::ExprKind::Ident("get_size".into()),
                dummy_span(),
            )),
            args: vec![],
        },
        dummy_span(),
    );

    let result = checker.infer_expr(&call, &aria_types::TypeEnv::new()).unwrap();
    assert_eq!(result, Type::UInt64);
}

// =========================================================================
// Multiple Extern Items Tests
// =========================================================================

#[test]
fn test_extern_c_multiple_items() {
    // Test multiple items in one extern block
    let mut checker = TypeChecker::new();

    let extern_c = ast::ExternC {
        header: "combined.h".into(),
        alias: None,
        items: vec![
            // Function
            ast::ExternCItem::Function(ast::ExternFunction {
                name: make_ident("create_point"),
                params: vec![
                    ast::ExternParam {
                        name: Some(make_ident("x")),
                        ty: ast::CType::Int,
                    },
                    ast::ExternParam {
                        name: Some(make_ident("y")),
                        ty: ast::CType::Int,
                    },
                ],
                return_type: Some(ast::CType::Named(make_type_ident("Point"))),
                span: dummy_span(),
            }),
            // Struct
            ast::ExternCItem::Struct(ast::ExternStruct {
                name: make_type_ident("Point"),
                fields: vec![
                    ast::ExternField {
                        name: make_ident("x"),
                        ty: ast::CType::Int,
                    },
                    ast::ExternField {
                        name: make_ident("y"),
                        ty: ast::CType::Int,
                    },
                ],
                span: dummy_span(),
            }),
            // Const
            ast::ExternCItem::Const(ast::ExternConst {
                name: make_ident("ORIGIN_X"),
                ty: ast::CType::Int,
                span: dummy_span(),
            }),
        ],
        span: dummy_span(),
    };

    let result = checker.check_program(&ast::Program {
        items: vec![ast::Item::Extern(ast::ExternDecl::C(extern_c))],
        span: dummy_span(),
    });
    assert!(result.is_ok());
}

// =========================================================================
// Python Extern Tests
// =========================================================================

#[test]
fn test_extern_python_basic() {
    // Test: extern "Python" "numpy"
    let mut checker = TypeChecker::new();

    let extern_python = ast::ExternPython {
        module: "numpy".into(),
        alias: Some(make_ident("np")),
        span: dummy_span(),
    };

    let extern_decl = ast::ExternDecl::Python(extern_python);

    let result = checker.check_program(&ast::Program {
        items: vec![ast::Item::Extern(extern_decl)],
        span: dummy_span(),
    });
    assert!(result.is_ok());
}

// =========================================================================
// WASM Extern Tests
// =========================================================================

#[test]
fn test_extern_wasm_import() {
    // Test: extern "WASM" import "env" { fn log(x: int) }
    let mut checker = TypeChecker::new();

    let extern_wasm = ast::ExternWasm {
        kind: ast::WasmExternKind::Import,
        name: make_ident("env"),
        items: vec![
            ast::ExternFunction {
                name: make_ident("log"),
                params: vec![
                    ast::ExternParam {
                        name: Some(make_ident("x")),
                        ty: ast::CType::Int,
                    },
                ],
                return_type: None,
                span: dummy_span(),
            },
        ],
        span: dummy_span(),
    };

    let extern_decl = ast::ExternDecl::Wasm(extern_wasm);

    let result = checker.check_program(&ast::Program {
        items: vec![ast::Item::Extern(extern_decl)],
        span: dummy_span(),
    });
    assert!(result.is_ok());
}

#[test]
fn test_extern_wasm_export() {
    // Test: extern "WASM" export "math" { fn add(a: int, b: int) -> int }
    let mut checker = TypeChecker::new();

    let extern_wasm = ast::ExternWasm {
        kind: ast::WasmExternKind::Export,
        name: make_ident("math"),
        items: vec![
            ast::ExternFunction {
                name: make_ident("add"),
                params: vec![
                    ast::ExternParam {
                        name: Some(make_ident("a")),
                        ty: ast::CType::Int,
                    },
                    ast::ExternParam {
                        name: Some(make_ident("b")),
                        ty: ast::CType::Int,
                    },
                ],
                return_type: Some(ast::CType::Int),
                span: dummy_span(),
            },
        ],
        span: dummy_span(),
    };

    let extern_decl = ast::ExternDecl::Wasm(extern_wasm);

    let result = checker.check_program(&ast::Program {
        items: vec![ast::Item::Extern(extern_decl)],
        span: dummy_span(),
    });
    assert!(result.is_ok());
}

// =========================================================================
// All C Types Validation Tests
// =========================================================================

#[test]
fn test_all_c_primitive_types() {
    // Test all primitive C types are accepted
    let mut checker = TypeChecker::new();

    let c_types = vec![
        ("c_int", ast::CType::Int),
        ("c_uint", ast::CType::UInt),
        ("c_long", ast::CType::Long),
        ("c_ulong", ast::CType::ULong),
        ("c_longlong", ast::CType::LongLong),
        ("c_float", ast::CType::Float),
        ("c_double", ast::CType::Double),
        ("c_char", ast::CType::Char),
        ("c_void", ast::CType::Void),
        ("c_size_t", ast::CType::SizeT),
        ("c_ssize_t", ast::CType::SSizeT),
    ];

    let functions: Vec<ast::ExternCItem> = c_types
        .into_iter()
        .map(|(name, ty)| {
            ast::ExternCItem::Function(ast::ExternFunction {
                name: make_ident(name),
                params: vec![],
                return_type: Some(ty),
                span: dummy_span(),
            })
        })
        .collect();

    let extern_c = ast::ExternC {
        header: "all_types.h".into(),
        alias: None,
        items: functions,
        span: dummy_span(),
    };

    let result = checker.check_program(&ast::Program {
        items: vec![ast::Item::Extern(ast::ExternDecl::C(extern_c))],
        span: dummy_span(),
    });
    assert!(result.is_ok());
}

#[test]
fn test_nested_pointers() {
    // Test: int** (pointer to pointer)
    let mut checker = TypeChecker::new();

    let extern_func = ast::ExternFunction {
        name: make_ident("get_ptr_ptr"),
        params: vec![],
        return_type: Some(ast::CType::Pointer {
            const_: false,
            pointee: Box::new(ast::CType::Pointer {
                const_: false,
                pointee: Box::new(ast::CType::Int),
            }),
        }),
        span: dummy_span(),
    };

    let extern_c = ast::ExternC {
        header: "ptrs.h".into(),
        alias: None,
        items: vec![ast::ExternCItem::Function(extern_func)],
        span: dummy_span(),
    };

    let result = checker.check_program(&ast::Program {
        items: vec![ast::Item::Extern(ast::ExternDecl::C(extern_c))],
        span: dummy_span(),
    });
    assert!(result.is_ok());
}

#[test]
fn test_void_pointer_to_reference() {
    // Test that void* converts properly to &mut UInt8
    // Use a simple function without parameters to test the return type mapping
    let mut checker = TypeChecker::new();

    let extern_func = ast::ExternFunction {
        name: make_ident("get_buffer"),
        params: vec![],
        return_type: Some(ast::CType::Pointer {
            const_: false,
            pointee: Box::new(ast::CType::Void),
        }),
        span: dummy_span(),
    };

    let extern_c = ast::ExternC {
        header: "stdlib.h".into(),
        alias: None,
        items: vec![ast::ExternCItem::Function(extern_func)],
        span: dummy_span(),
    };

    checker.check_program(&ast::Program {
        items: vec![ast::Item::Extern(ast::ExternDecl::C(extern_c))],
        span: dummy_span(),
    }).unwrap();

    // Check that get_buffer returns &mut UInt8 (void* -> mutable ref to bytes)
    let call = ast::Expr::new(
        ast::ExprKind::Call {
            func: Box::new(ast::Expr::new(
                ast::ExprKind::Ident("get_buffer".into()),
                dummy_span(),
            )),
            args: vec![],
        },
        dummy_span(),
    );

    let result = checker.infer_expr(&call, &aria_types::TypeEnv::new()).unwrap();
    assert_eq!(result, Type::Reference {
        mutable: true,
        inner: Box::new(Type::UInt8),
    });
}

// =========================================================================
// Opaque Type Tests
// =========================================================================

#[test]
fn test_opaque_type_declaration() {
    // Test: extern "C" "file.h" { type FILE }
    let mut checker = TypeChecker::new();

    let extern_c = ast::ExternC {
        header: "stdio.h".into(),
        alias: None,
        items: vec![ast::ExternCItem::Type(make_type_ident("FILE"))],
        span: dummy_span(),
    };

    let result = checker.check_program(&ast::Program {
        items: vec![ast::Item::Extern(ast::ExternDecl::C(extern_c))],
        span: dummy_span(),
    });
    assert!(result.is_ok());
}

#[test]
fn test_opaque_type_pointer() {
    // Test functions using opaque type pointers
    let mut checker = TypeChecker::new();

    let extern_c = ast::ExternC {
        header: "stdio.h".into(),
        alias: None,
        items: vec![
            // Opaque type
            ast::ExternCItem::Type(make_type_ident("FILE")),
            // Function returning pointer to opaque type
            ast::ExternCItem::Function(ast::ExternFunction {
                name: make_ident("fopen"),
                params: vec![
                    ast::ExternParam {
                        name: Some(make_ident("path")),
                        ty: ast::CType::Pointer {
                            const_: true,
                            pointee: Box::new(ast::CType::Char),
                        },
                    },
                    ast::ExternParam {
                        name: Some(make_ident("mode")),
                        ty: ast::CType::Pointer {
                            const_: true,
                            pointee: Box::new(ast::CType::Char),
                        },
                    },
                ],
                return_type: Some(ast::CType::Pointer {
                    const_: false,
                    pointee: Box::new(ast::CType::Named(make_type_ident("FILE"))),
                }),
                span: dummy_span(),
            }),
            // Function taking pointer to opaque type
            ast::ExternCItem::Function(ast::ExternFunction {
                name: make_ident("fclose"),
                params: vec![
                    ast::ExternParam {
                        name: Some(make_ident("stream")),
                        ty: ast::CType::Pointer {
                            const_: false,
                            pointee: Box::new(ast::CType::Named(make_type_ident("FILE"))),
                        },
                    },
                ],
                return_type: Some(ast::CType::Int),
                span: dummy_span(),
            }),
        ],
        span: dummy_span(),
    };

    let result = checker.check_program(&ast::Program {
        items: vec![ast::Item::Extern(ast::ExternDecl::C(extern_c))],
        span: dummy_span(),
    });
    assert!(result.is_ok());
}

// =========================================================================
// Unnamed Parameter Tests
// =========================================================================

#[test]
fn test_unnamed_parameters() {
    // Test function with unnamed parameters (common in C headers)
    let mut checker = TypeChecker::new();

    let extern_func = ast::ExternFunction {
        name: make_ident("memcpy"),
        params: vec![
            ast::ExternParam {
                name: None, // dest
                ty: ast::CType::Pointer {
                    const_: false,
                    pointee: Box::new(ast::CType::Void),
                },
            },
            ast::ExternParam {
                name: None, // src
                ty: ast::CType::Pointer {
                    const_: true,
                    pointee: Box::new(ast::CType::Void),
                },
            },
            ast::ExternParam {
                name: None, // n
                ty: ast::CType::SizeT,
            },
        ],
        return_type: Some(ast::CType::Pointer {
            const_: false,
            pointee: Box::new(ast::CType::Void),
        }),
        span: dummy_span(),
    };

    let extern_c = ast::ExternC {
        header: "string.h".into(),
        alias: None,
        items: vec![ast::ExternCItem::Function(extern_func)],
        span: dummy_span(),
    };

    let result = checker.check_program(&ast::Program {
        items: vec![ast::Item::Extern(ast::ExternDecl::C(extern_c))],
        span: dummy_span(),
    });
    assert!(result.is_ok());
}
