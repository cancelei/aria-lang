//! End-to-end integration tests for MIR lowering.
//!
//! These tests parse Aria source code and lower it through the full pipeline
//! to MIR, verifying that the lowering succeeds and produces expected output.
//!
//! Note: The aria-parser uses `end` keyword to close blocks (not `{}`).

use aria_mir::{lower_program, MirType, pretty_print};

/// Parse Aria source code and lower to MIR, returning the pretty-printed result.
fn lower_source(source: &str) -> Result<String, String> {
    let (program, errors) = aria_parser::parse(source);
    if !errors.is_empty() {
        return Err(format!("Parse errors: {:?}", errors));
    }
    match lower_program(&program) {
        Ok(mir) => Ok(pretty_print(&mir)),
        Err(e) => Err(format!("Lowering error: {}", e)),
    }
}

/// Parse and lower, asserting success
fn assert_lowers(source: &str) -> String {
    match lower_source(source) {
        Ok(mir) => mir,
        Err(e) => panic!("Failed to lower:\n{}\nError: {}", source, e),
    }
}

/// Parse and lower, asserting failure
fn assert_lower_fails(source: &str) {
    match lower_source(source) {
        Ok(_) => panic!("Expected lowering to fail for:\n{}", source),
        Err(_) => {} // Expected
    }
}

// ============================================================================
// Basic Function Lowering
// ============================================================================

#[test]
fn test_lower_empty_function() {
    let mir = assert_lowers("fn main()\nend");
    assert!(mir.contains("fn main"));
}

#[test]
fn test_lower_function_with_return() {
    let mir = assert_lowers("fn add(a: Int, b: Int) -> Int\n  return a + b\nend");
    assert!(mir.contains("fn add"));
}

#[test]
fn test_lower_let_binding() {
    let mir = assert_lowers("fn main()\n  let x = 42\nend");
    assert!(mir.contains("42"));
}

#[test]
fn test_lower_arithmetic() {
    let mir = assert_lowers("fn main()\n  let x = 1 + 2 * 3\nend");
    // Pretty printer uses operator symbols: +, *, etc.
    assert!(mir.contains("+") || mir.contains("*"));
}

// ============================================================================
// String Interpolation
// ============================================================================

#[test]
fn test_lower_string_literal() {
    let mir = assert_lowers("fn main()\n  let s = \"hello\"\nend");
    // Strings are interned, shown as str#N in pretty printer
    assert!(mir.contains("str#"));
}

#[test]
fn test_lower_string_interpolation_simple() {
    let source = r#"fn greet(name: String) -> String
  let msg = "Hello, #{name}!"
  return msg
end"#;
    let mir = assert_lowers(source);
    assert!(mir.contains("fn greet"));
}

#[test]
fn test_lower_string_interpolation_with_expression() {
    let source = r#"fn show(x: Int) -> String
  return "value: #{x + 1}"
end"#;
    let mir = assert_lowers(source);
    assert!(mir.contains("fn show"));
}

// ============================================================================
// Lambda / Closure
// ============================================================================

#[test]
fn test_lower_simple_lambda() {
    // Arrow syntax: (x) => expr
    let source = "fn main()\n  let f = (x) => x + 1\nend";
    let mir = assert_lowers(source);
    assert!(mir.contains("__lambda_"));
}

#[test]
fn test_lower_lambda_with_capture() {
    // Brace-pipe syntax: { |x| body }
    let source = "fn main()\n  let offset = 10\n  let f = { |x| x + offset }\nend";
    let mir = assert_lowers(source);
    assert!(mir.contains("__lambda_"));
}

// ============================================================================
// Control Flow
// ============================================================================

#[test]
fn test_lower_if_else() {
    let source = r#"fn max(a: Int, b: Int) -> Int
  if a > b
    return a
  else
    return b
  end
end"#;
    let mir = assert_lowers(source);
    assert!(mir.contains("fn max"));
}

#[test]
fn test_lower_while_loop() {
    let source = r#"fn count()
  let i = 0
  while i < 10
    i = i + 1
  end
end"#;
    let mir = assert_lowers(source);
    assert!(mir.contains("fn count"));
}

#[test]
fn test_lower_for_loop() {
    let source = r#"fn iterate()
  let arr = [1, 2, 3]
  for x in arr
    print(x)
  end
end"#;
    let mir = assert_lowers(source);
    assert!(mir.contains("fn iterate"));
}

#[test]
fn test_lower_match_with_literals() {
    let source = r#"fn classify(x: Int) -> String
  match x
    0 => "zero"
    1 => "one"
    _ => "other"
  end
end"#;
    let mir = assert_lowers(source);
    assert!(mir.contains("fn classify"));
}

// ============================================================================
// Structs
// ============================================================================

#[test]
fn test_lower_struct_definition() {
    let source = r#"struct Point
  x: Float
  y: Float
end

fn origin() -> Point
  return Point(x: 0.0, y: 0.0)
end"#;
    let mir = assert_lowers(source);
    assert!(mir.contains("Point"));
}

// ============================================================================
// Pipe Operator
// ============================================================================

#[test]
fn test_lower_pipe_operator() {
    let source = r#"fn double(x: Int) -> Int
  return x * 2
end

fn main()
  let result = 5 |> double
end"#;
    let mir = assert_lowers(source);
    assert!(mir.contains("fn main"));
    assert!(mir.contains("fn double"));
}

// ============================================================================
// Boolean Operators
// ============================================================================

#[test]
fn test_lower_short_circuit_and() {
    let source = "fn both(a: Bool, b: Bool) -> Bool\n  return a && b\nend";
    let mir = assert_lowers(source);
    assert!(mir.contains("fn both"));
}

#[test]
fn test_lower_short_circuit_or() {
    let source = "fn either(a: Bool, b: Bool) -> Bool\n  return a || b\nend";
    let mir = assert_lowers(source);
    assert!(mir.contains("fn either"));
}

// ============================================================================
// Collections
// ============================================================================

#[test]
fn test_lower_array_literal() {
    let source = "fn make_array()\n  let arr = [1, 2, 3, 4, 5]\nend";
    let mir = assert_lowers(source);
    assert!(mir.contains("fn make_array"));
}

#[test]
fn test_lower_tuple_literal() {
    let source = "fn make_tuple()\n  let t = (1, true)\nend";
    let mir = assert_lowers(source);
    assert!(mir.contains("fn make_tuple"));
}

// ============================================================================
// Error Cases
// ============================================================================

#[test]
fn test_lower_undefined_variable_fails() {
    assert_lower_fails("fn main()\n  return undefined_var\nend");
}

#[test]
fn test_lower_undefined_function_fails() {
    assert_lower_fails("fn main()\n  let x = nonexistent_function(42)\nend");
}

// ============================================================================
// MIR Type Correctness
// ============================================================================

#[test]
fn test_lower_preserves_function_return_types() {
    let source = r#"fn get_int() -> Int
  return 42
end

fn get_bool() -> Bool
  return true
end

fn get_float() -> Float
  return 3.14
end

fn get_string() -> String
  return "hello"
end"#;

    let (program, errors) = aria_parser::parse(source);
    assert!(errors.is_empty(), "Parse errors: {:?}", errors);

    let mir = lower_program(&program).expect("Lowering should succeed");

    for (_, func) in &mir.functions {
        match func.name.as_str() {
            "get_int" => assert_eq!(func.return_ty, MirType::Int),
            "get_bool" => assert_eq!(func.return_ty, MirType::Bool),
            "get_float" => assert_eq!(func.return_ty, MirType::Float),
            "get_string" => assert_eq!(func.return_ty, MirType::String),
            _ => {} // builtins
        }
    }
}

#[test]
fn test_lower_function_params_have_correct_types() {
    let source = "fn add(a: Int, b: Int) -> Int\n  return a + b\nend";

    let (program, errors) = aria_parser::parse(source);
    assert!(errors.is_empty(), "Parse errors: {:?}", errors);

    let mir = lower_program(&program).expect("Lowering should succeed");

    for (_, func) in &mir.functions {
        if func.name.as_str() == "add" {
            assert_eq!(func.return_ty, MirType::Int);
            assert_eq!(func.params.len(), 2);
        }
    }
}

// ============================================================================
// Feature Interactions
// ============================================================================

#[test]
fn test_lower_nested_if_match() {
    let source = r#"fn complex(x: Int) -> String
  if x > 0
    match x
      1 => "one"
      _ => "positive"
    end
  else
    return "non-positive"
  end
end"#;
    let mir = assert_lowers(source);
    assert!(mir.contains("fn complex"));
}

#[test]
fn test_lower_multiple_functions() {
    let source = r#"fn square(x: Int) -> Int
  return x * x
end

fn cube(x: Int) -> Int
  return x * x * x
end

fn main()
  let a = square(3)
  let b = cube(2)
end"#;
    let mir = assert_lowers(source);
    assert!(mir.contains("fn square"));
    assert!(mir.contains("fn cube"));
    assert!(mir.contains("fn main"));
}
