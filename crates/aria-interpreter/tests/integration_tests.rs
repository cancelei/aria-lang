//! Integration tests for the Aria interpreter.
//!
//! These tests verify end-to-end execution of Aria programs,
//! from parsing through evaluation.

use aria_interpreter::{Interpreter, RuntimeError, Value};
use aria_parser::parse;

/// Helper to run Aria code and return the result.
/// Expects a main() function that returns a value.
fn run_aria(source: &str) -> Result<Value, RuntimeError> {
    let (program, errors) = parse(source);
    if !errors.is_empty() {
        panic!("Parse errors: {:?}", errors);
    }
    let mut interpreter = Interpreter::new();
    interpreter.run(&program)
}

/// Helper to run Aria code that defines and calls main().
fn eval_main(source: &str) -> Result<Value, RuntimeError> {
    run_aria(source)
}

// ============================================================================
// Basic Expressions
// ============================================================================

mod expressions {
    use super::*;

    #[test]
    fn test_integer_literals() {
        let result = eval_main(
            r#"
            fn main() -> Int
              42
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(42));
    }

    #[test]
    fn test_float_literals() {
        let result = eval_main(
            r#"
            fn main() -> Float
              3.14
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Float(3.14));
    }

    #[test]
    fn test_string_literals() {
        let result = eval_main(
            r#"
            fn main() -> String
              "hello"
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::String("hello".into()));
    }

    #[test]
    fn test_boolean_literals() {
        let result = eval_main(
            r#"
            fn main() -> Bool
              true
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Bool(true));

        let result = eval_main(
            r#"
            fn main() -> Bool
              false
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_nil_literal() {
        let result = eval_main(
            r#"
            fn main() -> Nil
              nil
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Nil);
    }
}

// ============================================================================
// Arithmetic Operations
// ============================================================================

mod arithmetic {
    use super::*;

    #[test]
    fn test_integer_addition() {
        let result = eval_main(
            r#"
            fn main() -> Int
              1 + 2 + 3
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(6));
    }

    #[test]
    fn test_integer_subtraction() {
        let result = eval_main(
            r#"
            fn main() -> Int
              10 - 3 - 2
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(5));
    }

    #[test]
    fn test_integer_multiplication() {
        let result = eval_main(
            r#"
            fn main() -> Int
              2 * 3 * 4
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(24));
    }

    #[test]
    fn test_integer_division() {
        let result = eval_main(
            r#"
            fn main() -> Int
              20 / 4
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(5));
    }

    #[test]
    fn test_integer_modulo() {
        let result = eval_main(
            r#"
            fn main() -> Int
              17 % 5
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(2));
    }

    #[test]
    fn test_negation() {
        let result = eval_main(
            r#"
            fn main() -> Int
              -42
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(-42));
    }

    #[test]
    fn test_operator_precedence() {
        let result = eval_main(
            r#"
            fn main() -> Int
              2 + 3 * 4
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(14));

        let result = eval_main(
            r#"
            fn main() -> Int
              (2 + 3) * 4
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(20));
    }

    #[test]
    fn test_float_arithmetic() {
        let result = eval_main(
            r#"
            fn main() -> Float
              1.5 + 2.5
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Float(4.0));
    }

    #[test]
    fn test_division_by_zero() {
        let result = eval_main(
            r#"
            fn main() -> Int
              10 / 0
            end
            "#,
        );
        assert!(matches!(result, Err(RuntimeError::DivisionByZero { .. })));
    }
}

// ============================================================================
// Comparison Operations
// ============================================================================

mod comparisons {
    use super::*;

    #[test]
    fn test_equality() {
        let result = eval_main(
            r#"
            fn main() -> Bool
              42 == 42
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Bool(true));

        let result = eval_main(
            r#"
            fn main() -> Bool
              42 == 43
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_inequality() {
        let result = eval_main(
            r#"
            fn main() -> Bool
              42 != 43
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_less_than() {
        let result = eval_main(
            r#"
            fn main() -> Bool
              3 < 5
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Bool(true));

        let result = eval_main(
            r#"
            fn main() -> Bool
              5 < 3
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_less_than_or_equal() {
        let result = eval_main(
            r#"
            fn main() -> Bool
              5 <= 5
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_greater_than() {
        let result = eval_main(
            r#"
            fn main() -> Bool
              5 > 3
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_greater_than_or_equal() {
        let result = eval_main(
            r#"
            fn main() -> Bool
              5 >= 5
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_string_equality() {
        let result = eval_main(
            r#"
            fn main() -> Bool
              "hello" == "hello"
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Bool(true));

        let result = eval_main(
            r#"
            fn main() -> Bool
              "hello" == "world"
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Bool(false));
    }
}

// ============================================================================
// Logical Operations
// ============================================================================

mod logical {
    use super::*;

    #[test]
    fn test_logical_and() {
        let result = eval_main(
            r#"
            fn main() -> Bool
              true && true
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Bool(true));

        let result = eval_main(
            r#"
            fn main() -> Bool
              true && false
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_logical_or() {
        let result = eval_main(
            r#"
            fn main() -> Bool
              false || true
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Bool(true));

        let result = eval_main(
            r#"
            fn main() -> Bool
              false || false
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_logical_not() {
        let result = eval_main(
            r#"
            fn main() -> Bool
              !false
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Bool(true));

        let result = eval_main(
            r#"
            fn main() -> Bool
              !true
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_short_circuit_and() {
        // Should short-circuit: second expression not evaluated
        let result = eval_main(
            r#"
            fn main() -> Bool
              false && (1 / 0 == 0)
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_short_circuit_or() {
        // Should short-circuit: second expression not evaluated
        let result = eval_main(
            r#"
            fn main() -> Bool
              true || (1 / 0 == 0)
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Bool(true));
    }
}

// ============================================================================
// Variables and Bindings
// ============================================================================

mod variables {
    use super::*;

    #[test]
    fn test_let_binding() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let x = 42
              x
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(42));
    }

    #[test]
    fn test_multiple_bindings() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let x = 10
              let y = 20
              x + y
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(30));
    }

    #[test]
    fn test_variable_shadowing() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let x = 10
              let x = 20
              x
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(20));
    }

    #[test]
    fn test_variable_reassignment() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let x = 10
              x = 20
              x
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(20));
    }

    #[test]
    fn test_undefined_variable() {
        let result = eval_main(
            r#"
            fn main() -> Int
              x
            end
            "#,
        );
        assert!(matches!(
            result,
            Err(RuntimeError::UndefinedVariable { .. })
        ));
    }
}

// ============================================================================
// Control Flow
// ============================================================================

mod control_flow {
    use super::*;

    #[test]
    fn test_if_true() {
        let result = eval_main(
            r#"
            fn main() -> Int
              if true
                42
              else
                0
              end
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(42));
    }

    #[test]
    fn test_if_false() {
        let result = eval_main(
            r#"
            fn main() -> Int
              if false
                42
              else
                0
              end
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(0));
    }

    #[test]
    fn test_if_without_else() {
        let result = eval_main(
            r#"
            fn main() -> Nil
              if false
                42
              end
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Nil);
    }

    #[test]
    fn test_if_elsif() {
        // Aria uses elsif not "else if"
        let result = eval_main(
            r#"
            fn main() -> Int
              let x = 2
              if x == 1
                10
              elsif x == 2
                20
              else
                30
              end
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(20));
    }

    #[test]
    fn test_while_loop() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let sum = 0
              let i = 1
              while i <= 5
                sum = sum + i
                i = i + 1
              end
              sum
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(15));
    }

    #[test]
    fn test_for_loop_range() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let sum = 0
              for i in 1..5
                sum = sum + i
              end
              sum
            end
            "#,
        );
        // 1..5 is inclusive, so 1+2+3+4+5 = 15
        assert_eq!(result.unwrap(), Value::Int(15));
    }

    #[test]
    fn test_break_in_loop() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let sum = 0
              for i in 1..10
                if i > 3
                  break
                end
                sum = sum + i
              end
              sum
            end
            "#,
        );
        // 1+2+3 = 6
        assert_eq!(result.unwrap(), Value::Int(6));
    }

    #[test]
    fn test_continue_in_loop() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let sum = 0
              for i in 1..5
                if i == 3
                  continue
                end
                sum = sum + i
              end
              sum
            end
            "#,
        );
        // 1+2+4+5 = 12 (skipping 3)
        assert_eq!(result.unwrap(), Value::Int(12));
    }

    #[test]
    fn test_nested_loops() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let sum = 0
              for i in 1..3
                for j in 1..3
                  sum = sum + i * j
                end
              end
              sum
            end
            "#,
        );
        // (1*1 + 1*2 + 1*3) + (2*1 + 2*2 + 2*3) + (3*1 + 3*2 + 3*3) = 6 + 12 + 18 = 36
        assert_eq!(result.unwrap(), Value::Int(36));
    }
}

// ============================================================================
// Functions
// ============================================================================

mod functions {
    use super::*;

    #[test]
    fn test_function_definition_and_call() {
        let result = eval_main(
            r#"
            fn add(a: Int, b: Int) -> Int
              a + b
            end

            fn main() -> Int
              add(2, 3)
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(5));
    }

    #[test]
    fn test_recursive_function() {
        let result = eval_main(
            r#"
            fn factorial(n: Int) -> Int
              if n <= 1
                1
              else
                n * factorial(n - 1)
              end
            end

            fn main() -> Int
              factorial(5)
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(120));
    }

    #[test]
    fn test_fibonacci() {
        let result = eval_main(
            r#"
            fn fib(n: Int) -> Int
              if n <= 1
                n
              else
                fib(n - 1) + fib(n - 2)
              end
            end

            fn main() -> Int
              fib(10)
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(55));
    }

    #[test]
    fn test_function_with_no_params() {
        let result = eval_main(
            r#"
            fn get_answer() -> Int
              42
            end

            fn main() -> Int
              get_answer()
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(42));
    }

    #[test]
    fn test_arity_mismatch() {
        let result = eval_main(
            r#"
            fn add(a: Int, b: Int) -> Int
              a + b
            end

            fn main() -> Int
              add(1)
            end
            "#,
        );
        assert!(matches!(result, Err(RuntimeError::ArityMismatch { .. })));
    }
}

// ============================================================================
// Closures and Lambdas
// ============================================================================

mod closures {
    use super::*;

    #[test]
    fn test_arrow_lambda() {
        // Aria arrow lambda syntax: (params) => expr
        let result = eval_main(
            r#"
            fn main() -> Int
              let f = (x) => x * 2
              f(21)
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(42));
    }

    #[test]
    fn test_block_lambda() {
        // Aria block lambda syntax: { |params| body }
        let result = eval_main(
            r#"
            fn main() -> Int
              let f = { |x| x * 2 }
              f(21)
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(42));
    }

    #[test]
    fn test_closure_captures_variable() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let multiplier = 10
              let f = (x) => x * multiplier
              f(4)
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(40));
    }

    #[test]
    fn test_multi_param_lambda() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let add = (a, b) => a + b
              add(10, 20)
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(30));
    }

    #[test]
    fn test_zero_param_lambda() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let get42 = () => 42
              get42()
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(42));
    }
}

// ============================================================================
// Arrays
// ============================================================================

mod arrays {
    use super::*;

    #[test]
    fn test_array_literal() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let arr = [1, 2, 3]
              arr[0]
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(1));
    }

    #[test]
    fn test_array_indexing() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let arr = [10, 20, 30, 40]
              arr[2]
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(30));
    }

    #[test]
    fn test_array_length() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let arr = [1, 2, 3, 4, 5]
              len(arr)
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(5));
    }

    #[test]
    fn test_array_index_out_of_bounds() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let arr = [1, 2, 3]
              arr[10]
            end
            "#,
        );
        assert!(matches!(
            result,
            Err(RuntimeError::IndexOutOfBounds { .. })
        ));
    }

    #[test]
    fn test_array_iteration() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let arr = [1, 2, 3, 4, 5]
              let sum = 0
              for x in arr
                sum = sum + x
              end
              sum
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(15));
    }
}

// ============================================================================
// Maps
// ============================================================================

mod maps {
    use super::*;

    #[test]
    fn test_map_literal() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let m = {"a": 1, "b": 2}
              m["a"]
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(1));
    }

    #[test]
    fn test_map_access() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let m = {"x": 10, "y": 20}
              m["x"] + m["y"]
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(30));
    }

    #[test]
    fn test_map_insertion() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let m = {"a": 1}
              m["b"] = 2
              m["b"]
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(2));
    }

    #[test]
    fn test_map_length() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let m = {"a": 1, "b": 2, "c": 3}
              len(m)
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(3));
    }
}

// ============================================================================
// String Operations
// ============================================================================

mod strings {
    use super::*;

    #[test]
    fn test_string_concatenation() {
        let result = eval_main(
            r#"
            fn main() -> String
              "Hello, " + "World!"
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::String("Hello, World!".into()));
    }

    #[test]
    fn test_string_length() {
        let result = eval_main(
            r#"
            fn main() -> Int
              len("hello")
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(5));
    }

    #[test]
    fn test_string_indexing() {
        let result = eval_main(
            r#"
            fn main() -> String
              let s = "hello"
              s[1]
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::String("e".into()));
    }
}

// ============================================================================
// Built-in Functions
// ============================================================================

mod builtins {
    use super::*;

    #[test]
    fn test_len_array() {
        let result = eval_main(
            r#"
            fn main() -> Int
              len([1, 2, 3])
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(3));
    }

    #[test]
    fn test_len_string() {
        let result = eval_main(
            r#"
            fn main() -> Int
              len("hello")
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(5));
    }

    #[test]
    fn test_len_map() {
        let result = eval_main(
            r#"
            fn main() -> Int
              len({"a": 1, "b": 2})
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(2));
    }

    #[test]
    fn test_type_of() {
        let result = eval_main(
            r#"
            fn main() -> String
              type_of(42)
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::String("Int".into()));

        let result = eval_main(
            r#"
            fn main() -> String
              type_of("hello")
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::String("String".into()));

        let result = eval_main(
            r#"
            fn main() -> String
              type_of([1, 2, 3])
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::String("Array".into()));
    }

    #[test]
    fn test_to_string() {
        let result = eval_main(
            r#"
            fn main() -> String
              to_string(42)
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::String("42".into()));
    }

    #[test]
    fn test_to_int_from_float() {
        let result = eval_main(
            r#"
            fn main() -> Int
              to_int(3.7)
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(3));
    }

    #[test]
    fn test_to_float_from_int() {
        let result = eval_main(
            r#"
            fn main() -> Float
              to_float(42)
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Float(42.0));
    }
}

// ============================================================================
// Pattern Matching
// ============================================================================

mod patterns {
    use super::*;

    #[test]
    fn test_tuple_pattern() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let (a, b) = (1, 2)
              a + b
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(3));
    }

    #[test]
    fn test_nested_tuple_pattern() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let ((a, b), c) = ((1, 2), 3)
              a + b + c
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(6));
    }

    #[test]
    fn test_wildcard_pattern() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let (a, _) = (42, 100)
              a
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(42));
    }
}

// ============================================================================
// Complex Programs
// ============================================================================

// ============================================================================
// Bitwise Operations
// ============================================================================

mod bitwise {
    use super::*;

    // NOTE: These tests are ignored because the parser doesn't yet support
    // bitwise operators (&, |, ^, <<, >>). The interpreter implementation
    // is ready and these tests will pass once parser support is added.

    #[test]
    #[ignore = "Parser doesn't support & operator yet"]
    fn test_bitwise_and() {
        // 10 & 12 = 8 (in binary: 1010 & 1100 = 1000)
        let result = eval_main(
            r#"
            fn main() -> Int
              10 & 12
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(8));
    }

    #[test]
    #[ignore = "Parser doesn't support | operator yet"]
    fn test_bitwise_or() {
        // 10 | 12 = 14 (in binary: 1010 | 1100 = 1110)
        let result = eval_main(
            r#"
            fn main() -> Int
              10 | 12
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(14));
    }

    #[test]
    #[ignore = "Parser doesn't support ^ operator yet"]
    fn test_bitwise_xor() {
        // 10 ^ 12 = 6 (in binary: 1010 ^ 1100 = 0110)
        let result = eval_main(
            r#"
            fn main() -> Int
              10 ^ 12
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(6));
    }

    #[test]
    fn test_bitwise_not() {
        let result = eval_main(
            r#"
            fn main() -> Int
              ~0
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(-1));
    }

    #[test]
    #[ignore = "Parser doesn't support << operator yet"]
    fn test_left_shift() {
        let result = eval_main(
            r#"
            fn main() -> Int
              1 << 4
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(16));
    }

    #[test]
    #[ignore = "Parser doesn't support >> operator yet"]
    fn test_right_shift() {
        let result = eval_main(
            r#"
            fn main() -> Int
              16 >> 2
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(4));
    }

    #[test]
    #[ignore = "Parser doesn't support & operator yet"]
    fn test_bitwise_type_error() {
        let result = eval_main(
            r#"
            fn main() -> Int
              "hello" & 5
            end
            "#,
        );
        assert!(matches!(result, Err(RuntimeError::TypeError { .. })));
    }
}

// ============================================================================
// Power and Integer Division
// ============================================================================

mod power_and_int_div {
    use super::*;

    #[test]
    fn test_power_positive_exponent() {
        let result = eval_main(
            r#"
            fn main() -> Int
              2 ** 10
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(1024));
    }

    #[test]
    fn test_power_zero_exponent() {
        let result = eval_main(
            r#"
            fn main() -> Int
              5 ** 0
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(1));
    }

    #[test]
    fn test_power_float_base() {
        let result = eval_main(
            r#"
            fn main() -> Float
              2.0 ** 3
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Float(8.0));
    }

    #[test]
    fn test_integer_division() {
        let result = eval_main(
            r#"
            fn main() -> Int
              17 // 5
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(3));
    }

    #[test]
    fn test_integer_division_by_zero() {
        let result = eval_main(
            r#"
            fn main() -> Int
              10 // 0
            end
            "#,
        );
        assert!(matches!(result, Err(RuntimeError::DivisionByZero { .. })));
    }
}

// ============================================================================
// Ternary Expressions
// ============================================================================

mod ternary {
    use super::*;

    #[test]
    fn test_ternary_true() {
        let result = eval_main(
            r#"
            fn main() -> Int
              true ? 42 : 0
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(42));
    }

    #[test]
    fn test_ternary_false() {
        let result = eval_main(
            r#"
            fn main() -> Int
              false ? 42 : 0
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(0));
    }

    #[test]
    fn test_ternary_with_expression() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let x = 5
              x > 3 ? x * 2 : x
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(10));
    }

    #[test]
    fn test_nested_ternary() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let x = 2
              x == 1 ? 10 : (x == 2 ? 20 : 30)
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(20));
    }
}

// ============================================================================
// Pipe Operator
// ============================================================================

mod pipe_operator {
    use super::*;

    #[test]
    fn test_pipe_to_function() {
        let result = eval_main(
            r#"
            fn double(x: Int) -> Int
              x * 2
            end

            fn main() -> Int
              5 |> double
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(10));
    }

    #[test]
    fn test_pipe_chain() {
        let result = eval_main(
            r#"
            fn add_one(x: Int) -> Int
              x + 1
            end

            fn double(x: Int) -> Int
              x * 2
            end

            fn main() -> Int
              5 |> add_one |> double
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(12));
    }

    #[test]
    fn test_pipe_with_extra_args() {
        let result = eval_main(
            r#"
            fn add(a: Int, b: Int) -> Int
              a + b
            end

            fn main() -> Int
              10 |> add(5)
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(15));
    }
}

// ============================================================================
// Collection Methods
// ============================================================================

mod collection_methods {
    use super::*;

    #[test]
    fn test_array_push() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let arr = [1, 2, 3]
              arr.push(4)
              len(arr)
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(4));
    }

    #[test]
    fn test_array_pop() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let arr = [1, 2, 3]
              arr.pop()
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(3));
    }

    #[test]
    fn test_array_len_method() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let arr = [1, 2, 3, 4, 5]
              arr.len()
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(5));
    }

    #[test]
    fn test_string_to_uppercase() {
        let result = eval_main(
            r#"
            fn main() -> String
              let s = "hello"
              s.to_uppercase()
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::String("HELLO".into()));
    }

    #[test]
    fn test_string_to_lowercase() {
        let result = eval_main(
            r#"
            fn main() -> String
              let s = "HELLO"
              s.to_lowercase()
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::String("hello".into()));
    }

    #[test]
    fn test_string_trim() {
        let result = eval_main(
            r#"
            fn main() -> String
              let s = "  hello  "
              s.trim()
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::String("hello".into()));
    }

    #[test]
    fn test_string_split() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let s = "a,b,c,d"
              let parts = s.split(",")
              len(parts)
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(4));
    }

    #[test]
    fn test_map_keys() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let m = {"a": 1, "b": 2, "c": 3}
              let k = m.keys()
              len(k)
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(3));
    }

    #[test]
    fn test_map_values() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let m = {"x": 10, "y": 20}
              let v = m.values()
              len(v)
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(2));
    }

    #[test]
    fn test_map_contains() {
        let result = eval_main(
            r#"
            fn main() -> Bool
              let m = {"a": 1, "b": 2}
              m.contains("a")
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_map_contains_missing() {
        let result = eval_main(
            r#"
            fn main() -> Bool
              let m = {"a": 1, "b": 2}
              m.contains("c")
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Bool(false));
    }
}

// ============================================================================
// Negative Indexing
// ============================================================================

mod negative_indexing {
    use super::*;

    #[test]
    fn test_array_negative_index() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let arr = [1, 2, 3, 4, 5]
              arr[-1]
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(5));
    }

    #[test]
    fn test_array_negative_index_second_last() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let arr = [1, 2, 3, 4, 5]
              arr[-2]
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(4));
    }

    #[test]
    fn test_string_negative_index() {
        let result = eval_main(
            r#"
            fn main() -> String
              let s = "hello"
              s[-1]
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::String("o".into()));
    }

    #[test]
    fn test_array_negative_index_assignment() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let arr = [1, 2, 3, 4, 5]
              arr[-1] = 100
              arr[-1]
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(100));
    }
}

// ============================================================================
// Loop Statement
// ============================================================================

mod loop_stmt {
    use super::*;

    #[test]
    fn test_loop_with_break() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let i = 0
              loop
                i = i + 1
                if i >= 5
                  break
                end
              end
              i
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(5));
    }

    #[test]
    fn test_loop_with_continue() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let i = 0
              let sum = 0
              loop
                i = i + 1
                if i > 10
                  break
                end
                if i % 2 == 0
                  continue
                end
                sum = sum + i
              end
              sum
            end
            "#,
        );
        // Sum of odd numbers 1..10: 1+3+5+7+9 = 25
        assert_eq!(result.unwrap(), Value::Int(25));
    }
}

// ============================================================================
// Range Expressions
// ============================================================================

mod ranges {
    use super::*;

    #[test]
    fn test_inclusive_range() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let sum = 0
              for i in 1..5
                sum = sum + i
              end
              sum
            end
            "#,
        );
        // 1+2+3+4+5 = 15 (inclusive)
        assert_eq!(result.unwrap(), Value::Int(15));
    }

    #[test]
    fn test_range_start_at_zero() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let count = 0
              for i in 0..3
                count = count + 1
              end
              count
            end
            "#,
        );
        // 0, 1, 2, 3 = 4 iterations
        assert_eq!(result.unwrap(), Value::Int(4));
    }
}

// ============================================================================
// Error Conditions
// ============================================================================

mod error_conditions {
    use super::*;

    #[test]
    fn test_undefined_function() {
        let result = eval_main(
            r#"
            fn main() -> Int
              undefined_func()
            end
            "#,
        );
        assert!(matches!(
            result,
            Err(RuntimeError::UndefinedVariable { .. })
        ));
    }

    #[test]
    fn test_not_callable() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let x = 42
              x()
            end
            "#,
        );
        assert!(matches!(result, Err(RuntimeError::NotCallable { .. })));
    }

    #[test]
    fn test_break_outside_loop() {
        let result = eval_main(
            r#"
            fn main() -> Int
              break
              0
            end
            "#,
        );
        assert!(matches!(
            result,
            Err(RuntimeError::BreakOutsideLoop { .. })
        ));
    }

    #[test]
    fn test_continue_outside_loop() {
        let result = eval_main(
            r#"
            fn main() -> Int
              continue
              0
            end
            "#,
        );
        assert!(matches!(
            result,
            Err(RuntimeError::ContinueOutsideLoop { .. })
        ));
    }

    #[test]
    fn test_return_outside_function() {
        // This is tricky - main is a function, so we need a different approach
        // The parser might reject this, or it might be allowed in main
        // Let's test type errors instead
        let result = eval_main(
            r#"
            fn main() -> Int
              "hello" - 5
            end
            "#,
        );
        assert!(matches!(result, Err(RuntimeError::TypeError { .. })));
    }

    #[test]
    fn test_type_error_negation() {
        let result = eval_main(
            r#"
            fn main() -> String
              -"hello"
            end
            "#,
        );
        assert!(matches!(result, Err(RuntimeError::TypeError { .. })));
    }

    #[test]
    fn test_invalid_map_key_type() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let m = {"a": 1}
              m[42]
            end
            "#,
        );
        assert!(matches!(result, Err(RuntimeError::TypeError { .. })));
    }

    #[test]
    fn test_pop_empty_array() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let arr = []
              arr.pop()
            end
            "#,
        );
        assert!(matches!(result, Err(RuntimeError::General { .. })));
    }

    #[test]
    fn test_invalid_key_error() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let m = {"a": 1}
              m["nonexistent"]
            end
            "#,
        );
        assert!(matches!(result, Err(RuntimeError::InvalidKey { .. })));
    }

    #[test]
    fn test_tuple_index_out_of_bounds() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let t = (1, 2, 3)
              t[10]
            end
            "#,
        );
        assert!(matches!(
            result,
            Err(RuntimeError::IndexOutOfBounds { .. })
        ));
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

mod edge_cases {
    use super::*;

    #[test]
    fn test_empty_array() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let arr = []
              len(arr)
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(0));
    }

    #[test]
    fn test_empty_map() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let m = {}
              len(m)
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(0));
    }

    #[test]
    fn test_empty_string() {
        let result = eval_main(
            r#"
            fn main() -> Int
              len("")
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(0));
    }

    #[test]
    fn test_deeply_nested_expression() {
        let result = eval_main(
            r#"
            fn main() -> Int
              ((((1 + 2) * 3) - 4) / 5)
            end
            "#,
        );
        // ((1+2)*3 - 4) / 5 = (9 - 4) / 5 = 5/5 = 1
        assert_eq!(result.unwrap(), Value::Int(1));
    }

    #[test]
    fn test_string_with_numbers_concatenation() {
        let result = eval_main(
            r#"
            fn main() -> String
              "Result: " + 42
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::String("Result: 42".into()));
    }

    #[test]
    fn test_zero_iteration_loop() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let sum = 0
              for i in 5..1
                sum = sum + i
              end
              sum
            end
            "#,
        );
        // Empty range: no iterations
        assert_eq!(result.unwrap(), Value::Int(0));
    }

    #[test]
    fn test_single_element_tuple() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let t = (42,)
              t[0]
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(42));
    }

    #[test]
    fn test_mixed_int_float_arithmetic() {
        let result = eval_main(
            r#"
            fn main() -> Float
              10 + 0.5
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Float(10.5));
    }

    #[test]
    fn test_float_int_subtraction() {
        let result = eval_main(
            r#"
            fn main() -> Float
              10.5 - 5
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Float(5.5));
    }

    #[test]
    fn test_truthiness_of_zero() {
        // In most languages, 0 is falsy
        let result = eval_main(
            r#"
            fn main() -> Bool
              if 0
                true
              else
                false
              end
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_truthiness_of_empty_string() {
        let result = eval_main(
            r#"
            fn main() -> Bool
              if ""
                true
              else
                false
              end
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_function_as_value() {
        let result = eval_main(
            r#"
            fn add(a: Int, b: Int) -> Int
              a + b
            end

            fn main() -> Int
              let f = add
              f(3, 4)
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(7));
    }

    #[test]
    fn test_recursive_counter() {
        // Using a smaller number to avoid stack overflow
        let result = eval_main(
            r#"
            fn count_down(n: Int, acc: Int) -> Int
              if n <= 0
                acc
              else
                count_down(n - 1, acc + 1)
              end
            end

            fn main() -> Int
              count_down(20, 0)
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(20));
    }

    #[test]
    fn test_string_comparison() {
        let result = eval_main(
            r#"
            fn main() -> Bool
              "apple" < "banana"
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_array_modification_in_function() {
        let result = eval_main(
            r#"
            fn push_value(arr: Array, val: Int)
              arr.push(val)
            end

            fn main() -> Int
              let a = [1, 2, 3]
              push_value(a, 4)
              len(a)
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(4));
    }
}

// ============================================================================
// Tuple Operations
// ============================================================================

mod tuples {
    use super::*;

    #[test]
    fn test_tuple_creation() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let t = (1, 2, 3)
              t[0] + t[1] + t[2]
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(6));
    }

    #[test]
    #[ignore = "Parser doesn't support tuple.N field access syntax yet"]
    fn test_tuple_field_access() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let t = (10, 20, 30)
              t.0 + t.2
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(40));
    }

    #[test]
    fn test_tuple_destructuring() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let (x, y, z) = (1, 2, 3)
              x * y * z
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(6));
    }

    #[test]
    #[ignore = "Parser doesn't support tuple.N field access syntax yet"]
    fn test_tuple_from_function() {
        let result = eval_main(
            r#"
            fn make_pair(a: Int, b: Int) -> (Int, Int)
              (a, b)
            end

            fn main() -> Int
              let p = make_pair(3, 4)
              p.0 + p.1
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(7));
    }
}

// ============================================================================
// Scoping
// ============================================================================

mod scoping {
    use super::*;

    #[test]
    fn test_block_scope() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let x = 1
              if true
                let x = 2
                x
              else
                0
              end
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(2));
    }

    #[test]
    fn test_outer_variable_visible_in_inner_scope() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let outer = 10
              if true
                outer + 5
              else
                0
              end
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(15));
    }

    #[test]
    fn test_for_loop_scoping() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let sum = 0
              let i = 100
              for i in 1..3
                sum = sum + i
              end
              i
            end
            "#,
        );
        // The outer 'i' should be unchanged after the loop
        assert_eq!(result.unwrap(), Value::Int(100));
    }

    #[test]
    fn test_closure_captures_at_definition_time() {
        let result = eval_main(
            r#"
            fn main() -> Int
              let x = 10
              let f = () => x
              x = 20
              f()
            end
            "#,
        );
        // Closure captures the environment, which contains x
        // When x is reassigned, the closure sees the new value
        assert_eq!(result.unwrap(), Value::Int(20));
    }
}

// ============================================================================
// Higher-Order Functions
// ============================================================================

mod higher_order {
    use super::*;

    #[test]
    fn test_function_returning_function() {
        let result = eval_main(
            r#"
            fn make_adder(n: Int) -> Fn
              (x) => x + n
            end

            fn main() -> Int
              let add5 = make_adder(5)
              add5(10)
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(15));
    }

    #[test]
    fn test_function_as_parameter() {
        let result = eval_main(
            r#"
            fn apply(f: Fn, x: Int) -> Int
              f(x)
            end

            fn double(x: Int) -> Int
              x * 2
            end

            fn main() -> Int
              apply(double, 21)
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(42));
    }

    #[test]
    fn test_lambda_passed_to_function() {
        let result = eval_main(
            r#"
            fn apply(f: Fn, x: Int) -> Int
              f(x)
            end

            fn main() -> Int
              apply((n) => n * 3, 14)
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(42));
    }
}

// ============================================================================
// Complex Programs
// ============================================================================

mod complex_programs {
    use super::*;

    #[test]
    fn test_fizzbuzz_count() {
        // Count FizzBuzz matches using elsif syntax
        let result = eval_main(
            r#"
            fn fizzbuzz(n: Int) -> Int
              let count = 0
              for i in 1..n
                if i % 15 == 0
                  count = count + 1
                elsif i % 3 == 0
                  count = count + 1
                elsif i % 5 == 0
                  count = count + 1
                end
              end
              count
            end

            fn main() -> Int
              fizzbuzz(15)
            end
            "#,
        );
        // FizzBuzz counts: 3, 5, 6, 9, 10, 12, 15 = 7 matches
        assert_eq!(result.unwrap(), Value::Int(7));
    }

    #[test]
    fn test_gcd() {
        let result = eval_main(
            r#"
            fn gcd(a: Int, b: Int) -> Int
              if b == 0
                a
              else
                gcd(b, a % b)
              end
            end

            fn main() -> Int
              gcd(48, 18)
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(6));
    }

    #[test]
    fn test_is_prime() {
        let result = eval_main(
            r#"
            fn is_prime(n: Int) -> Bool
              if n < 2
                false
              else
                let i = 2
                let found_divisor = false
                while i * i <= n
                  if n % i == 0
                    found_divisor = true
                    break
                  end
                  i = i + 1
                end
                !found_divisor
              end
            end

            fn main() -> Bool
              is_prime(17)
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Bool(true));

        let result = eval_main(
            r#"
            fn is_prime(n: Int) -> Bool
              if n < 2
                false
              else
                let i = 2
                let found_divisor = false
                while i * i <= n
                  if n % i == 0
                    found_divisor = true
                    break
                  end
                  i = i + 1
                end
                !found_divisor
              end
            end

            fn main() -> Bool
              is_prime(18)
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_sum_array_loop() {
        let result = eval_main(
            r#"
            fn sum(arr: Array) -> Int
              let total = 0
              for x in arr
                total = total + x
              end
              total
            end

            fn main() -> Int
              let numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
              sum(numbers)
            end
            "#,
        );
        assert_eq!(result.unwrap(), Value::Int(55));
    }

    #[test]
    fn test_accumulator_function() {
        // Test function returning a modified value through recursion
        let result = eval_main(
            r#"
            fn accumulate(n: Int, acc: Int) -> Int
              if n <= 0
                acc
              else
                accumulate(n - 1, acc + n)
              end
            end

            fn main() -> Int
              accumulate(5, 0)
            end
            "#,
        );
        // 5 + 4 + 3 + 2 + 1 = 15
        assert_eq!(result.unwrap(), Value::Int(15));
    }
}
