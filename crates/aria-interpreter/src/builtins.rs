//! Built-in functions for the Aria interpreter.

use std::cell::RefCell;
use std::io::{self, Write};
use std::rc::Rc;

use smol_str::SmolStr;

use crate::environment::Environment;
use crate::value::{BuiltinFn, Value};
use crate::{Result, RuntimeError};
use aria_lexer::Span;

/// Register all built-in functions in the environment.
pub fn register(env: &mut Environment) {
    // I/O functions
    env.define("print".into(), make_builtin("print", None, builtin_print));
    env.define("println".into(), make_builtin("println", None, builtin_println));
    env.define("input".into(), make_builtin("input", Some(0), builtin_input));

    // Type functions
    env.define("type_of".into(), make_builtin("type_of", Some(1), builtin_type_of));
    env.define("to_string".into(), make_builtin("to_string", Some(1), builtin_to_string));
    env.define("to_int".into(), make_builtin("to_int", Some(1), builtin_to_int));
    env.define("to_float".into(), make_builtin("to_float", Some(1), builtin_to_float));

    // Collection functions
    env.define("len".into(), make_builtin("len", Some(1), builtin_len));
    env.define("range".into(), make_builtin("range", None, builtin_range));
    env.define("first".into(), make_builtin("first", Some(1), builtin_first));
    env.define("last".into(), make_builtin("last", Some(1), builtin_last));
    env.define("reverse".into(), make_builtin("reverse", Some(1), builtin_reverse));

    // Higher-order collection operations
    env.define("map".into(), make_builtin("map", Some(2), builtin_map));
    env.define("filter".into(), make_builtin("filter", Some(2), builtin_filter));
    env.define("reduce".into(), make_builtin("reduce", Some(3), builtin_reduce));
    env.define("find".into(), make_builtin("find", Some(2), builtin_find));
    env.define("any".into(), make_builtin("any", Some(2), builtin_any));
    env.define("all".into(), make_builtin("all", Some(2), builtin_all));
    env.define("slice".into(), make_builtin("slice", Some(3), builtin_slice));
    env.define("concat".into(), make_builtin("concat", Some(2), builtin_concat));

    // Math functions
    env.define("abs".into(), make_builtin("abs", Some(1), builtin_abs));
    env.define("min".into(), make_builtin("min", Some(2), builtin_min));
    env.define("max".into(), make_builtin("max", Some(2), builtin_max));

    // Assertion
    env.define("assert".into(), make_builtin("assert", None, builtin_assert));
}

fn make_builtin(
    name: &str,
    arity: Option<usize>,
    func: fn(Vec<Value>) -> Result<Value>,
) -> Value {
    Value::BuiltinFunction(BuiltinFn {
        name: SmolStr::new(name),
        arity,
        func,
    })
}

fn builtin_print(args: Vec<Value>) -> Result<Value> {
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            print!(" ");
        }
        match arg {
            Value::String(s) => print!("{}", s),
            v => print!("{}", v),
        }
    }
    io::stdout().flush().ok();
    Ok(Value::Nil)
}

fn builtin_println(args: Vec<Value>) -> Result<Value> {
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            print!(" ");
        }
        match arg {
            Value::String(s) => print!("{}", s),
            v => print!("{}", v),
        }
    }
    println!();
    Ok(Value::Nil)
}

fn builtin_input(_args: Vec<Value>) -> Result<Value> {
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| RuntimeError::General {
            message: format!("failed to read input: {}", e),
            span: Span::default(),
        })?;

    // Trim trailing newline
    if input.ends_with('\n') {
        input.pop();
        if input.ends_with('\r') {
            input.pop();
        }
    }

    Ok(Value::String(input.into()))
}

fn builtin_type_of(args: Vec<Value>) -> Result<Value> {
    Ok(Value::String(args[0].type_name().into()))
}

fn builtin_to_string(args: Vec<Value>) -> Result<Value> {
    Ok(Value::String(format!("{}", args[0]).into()))
}

fn builtin_to_int(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::Int(n) => Ok(Value::Int(*n)),
        Value::Float(n) => Ok(Value::Int(*n as i64)),
        Value::String(s) => s.parse::<i64>().map(Value::Int).map_err(|_| {
            RuntimeError::TypeError {
                message: format!("cannot convert '{}' to integer", s),
                span: Span::default(),
            }
        }),
        Value::Bool(b) => Ok(Value::Int(if *b { 1 } else { 0 })),
        v => Err(RuntimeError::TypeError {
            message: format!("cannot convert {} to integer", v.type_name()),
            span: Span::default(),
        }),
    }
}

fn builtin_to_float(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::Int(n) => Ok(Value::Float(*n as f64)),
        Value::Float(n) => Ok(Value::Float(*n)),
        Value::String(s) => s.parse::<f64>().map(Value::Float).map_err(|_| {
            RuntimeError::TypeError {
                message: format!("cannot convert '{}' to float", s),
                span: Span::default(),
            }
        }),
        v => Err(RuntimeError::TypeError {
            message: format!("cannot convert {} to float", v.type_name()),
            span: Span::default(),
        }),
    }
}

fn builtin_len(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::String(s) => Ok(Value::Int(s.len() as i64)),
        Value::Array(arr) => Ok(Value::Int(arr.borrow().len() as i64)),
        Value::Map(map) => Ok(Value::Int(map.borrow().len() as i64)),
        Value::Tuple(t) => Ok(Value::Int(t.len() as i64)),
        v => Err(RuntimeError::TypeError {
            message: format!("{} has no length", v.type_name()),
            span: Span::default(),
        }),
    }
}

fn builtin_range(args: Vec<Value>) -> Result<Value> {
    match args.len() {
        1 => {
            // range(end) = 0..end
            let end = args[0].as_int().ok_or_else(|| RuntimeError::TypeError {
                message: "range argument must be an integer".into(),
                span: Span::default(),
            })?;
            Ok(Value::Range {
                start: 0,
                end,
                inclusive: false,
            })
        }
        2 => {
            // range(start, end) = start..end
            let start = args[0].as_int().ok_or_else(|| RuntimeError::TypeError {
                message: "range start must be an integer".into(),
                span: Span::default(),
            })?;
            let end = args[1].as_int().ok_or_else(|| RuntimeError::TypeError {
                message: "range end must be an integer".into(),
                span: Span::default(),
            })?;
            Ok(Value::Range {
                start,
                end,
                inclusive: false,
            })
        }
        3 => {
            // range(start, end, step) - for now just validate args
            let start = args[0].as_int().ok_or_else(|| RuntimeError::TypeError {
                message: "range start must be an integer".into(),
                span: Span::default(),
            })?;
            let end = args[1].as_int().ok_or_else(|| RuntimeError::TypeError {
                message: "range end must be an integer".into(),
                span: Span::default(),
            })?;
            // Step is ignored for now in basic range
            Ok(Value::Range {
                start,
                end,
                inclusive: false,
            })
        }
        _ => Err(RuntimeError::ArityMismatch {
            expected: 2,
            got: args.len(),
            span: Span::default(),
        }),
    }
}

fn builtin_abs(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::Int(n) => Ok(Value::Int(n.abs())),
        Value::Float(n) => Ok(Value::Float(n.abs())),
        v => Err(RuntimeError::TypeError {
            message: format!("abs expects a number, got {}", v.type_name()),
            span: Span::default(),
        }),
    }
}

fn builtin_min(args: Vec<Value>) -> Result<Value> {
    match (&args[0], &args[1]) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(*a.min(b))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.min(*b))),
        (Value::Int(a), Value::Float(b)) => Ok(Value::Float((*a as f64).min(*b))),
        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a.min(*b as f64))),
        (a, b) => Err(RuntimeError::TypeError {
            message: format!(
                "min expects numbers, got {} and {}",
                a.type_name(),
                b.type_name()
            ),
            span: Span::default(),
        }),
    }
}

fn builtin_max(args: Vec<Value>) -> Result<Value> {
    match (&args[0], &args[1]) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(*a.max(b))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.max(*b))),
        (Value::Int(a), Value::Float(b)) => Ok(Value::Float((*a as f64).max(*b))),
        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a.max(*b as f64))),
        (a, b) => Err(RuntimeError::TypeError {
            message: format!(
                "max expects numbers, got {} and {}",
                a.type_name(),
                b.type_name()
            ),
            span: Span::default(),
        }),
    }
}

fn builtin_assert(args: Vec<Value>) -> Result<Value> {
    if args.is_empty() {
        return Err(RuntimeError::ArityMismatch {
            expected: 1,
            got: 0,
            span: Span::default(),
        });
    }

    let condition = &args[0];
    let message = if args.len() > 1 {
        match &args[1] {
            Value::String(s) => s.to_string(),
            v => format!("{}", v),
        }
    } else {
        "assertion failed".into()
    };

    if !condition.is_truthy() {
        return Err(RuntimeError::AssertionFailed {
            message,
            span: Span::default(),
        });
    }

    Ok(Value::Nil)
}

fn builtin_first(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::Array(arr) => {
            let arr_ref = arr.borrow();
            arr_ref.first().cloned().ok_or_else(|| RuntimeError::General {
                message: "first() called on empty array".into(),
                span: Span::default(),
            })
        }
        v => Err(RuntimeError::TypeError {
            message: format!("first() requires an array, got {}", v.type_name()),
            span: Span::default(),
        }),
    }
}

fn builtin_last(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::Array(arr) => {
            let arr_ref = arr.borrow();
            arr_ref.last().cloned().ok_or_else(|| RuntimeError::General {
                message: "last() called on empty array".into(),
                span: Span::default(),
            })
        }
        v => Err(RuntimeError::TypeError {
            message: format!("last() requires an array, got {}", v.type_name()),
            span: Span::default(),
        }),
    }
}

fn builtin_reverse(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::Array(arr) => {
            let mut reversed = arr.borrow().clone();
            reversed.reverse();
            Ok(Value::Array(Rc::new(RefCell::new(reversed))))
        }
        v => Err(RuntimeError::TypeError {
            message: format!("reverse() requires an array, got {}", v.type_name()),
            span: Span::default(),
        }),
    }
}

// Helper function to call a function value with an argument
fn call_function(func: &Value, arg: Value) -> Result<Value> {
    match func {
        Value::BuiltinFunction(bf) => (bf.func)(vec![arg]),
        Value::Function(_) => {
            // For user-defined functions, we can't call them directly in builtins
            // This is a limitation we'll document
            Err(RuntimeError::General {
                message: "Higher-order functions with user-defined functions not yet supported in interpreter".into(),
                span: Span::default(),
            })
        }
        v => Err(RuntimeError::TypeError {
            message: format!("expected function, got {}", v.type_name()),
            span: Span::default(),
        }),
    }
}

fn builtin_map(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::Array(arr) => {
            let func = &args[1];
            let arr_ref = arr.borrow();
            let mut result = Vec::with_capacity(arr_ref.len());

            for item in arr_ref.iter() {
                result.push(call_function(func, item.clone())?);
            }

            Ok(Value::Array(Rc::new(RefCell::new(result))))
        }
        v => Err(RuntimeError::TypeError {
            message: format!("map() requires an array, got {}", v.type_name()),
            span: Span::default(),
        }),
    }
}

fn builtin_filter(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::Array(arr) => {
            let func = &args[1];
            let arr_ref = arr.borrow();
            let mut result = Vec::new();

            for item in arr_ref.iter() {
                let predicate_result = call_function(func, item.clone())?;
                if predicate_result.is_truthy() {
                    result.push(item.clone());
                }
            }

            Ok(Value::Array(Rc::new(RefCell::new(result))))
        }
        v => Err(RuntimeError::TypeError {
            message: format!("filter() requires an array, got {}", v.type_name()),
            span: Span::default(),
        }),
    }
}

fn builtin_reduce(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::Array(arr) => {
            let func = &args[1];
            let mut accumulator = args[2].clone();
            let arr_ref = arr.borrow();

            for item in arr_ref.iter() {
                // Call func with (accumulator, item)
                match func {
                    Value::BuiltinFunction(bf) => {
                        accumulator = (bf.func)(vec![accumulator, item.clone()])?;
                    }
                    Value::Function(_) => {
                        return Err(RuntimeError::General {
                            message: "reduce() with user-defined functions not yet supported in interpreter".into(),
                            span: Span::default(),
                        });
                    }
                    v => {
                        return Err(RuntimeError::TypeError {
                            message: format!("expected function, got {}", v.type_name()),
                            span: Span::default(),
                        });
                    }
                }
            }

            Ok(accumulator)
        }
        v => Err(RuntimeError::TypeError {
            message: format!("reduce() requires an array, got {}", v.type_name()),
            span: Span::default(),
        }),
    }
}

fn builtin_find(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::Array(arr) => {
            let func = &args[1];
            let arr_ref = arr.borrow();

            for item in arr_ref.iter() {
                let predicate_result = call_function(func, item.clone())?;
                if predicate_result.is_truthy() {
                    // Return Some(item) - represented as EnumVariant for now
                    return Ok(Value::EnumVariant {
                        enum_name: "Option".into(),
                        variant_name: "Some".into(),
                        fields: Some(vec![item.clone()]),
                    });
                }
            }

            // Return None
            Ok(Value::EnumVariant {
                enum_name: "Option".into(),
                variant_name: "None".into(),
                fields: None,
            })
        }
        v => Err(RuntimeError::TypeError {
            message: format!("find() requires an array, got {}", v.type_name()),
            span: Span::default(),
        }),
    }
}

fn builtin_any(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::Array(arr) => {
            let func = &args[1];
            let arr_ref = arr.borrow();

            for item in arr_ref.iter() {
                let predicate_result = call_function(func, item.clone())?;
                if predicate_result.is_truthy() {
                    return Ok(Value::Bool(true));
                }
            }

            Ok(Value::Bool(false))
        }
        v => Err(RuntimeError::TypeError {
            message: format!("any() requires an array, got {}", v.type_name()),
            span: Span::default(),
        }),
    }
}

fn builtin_all(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::Array(arr) => {
            let func = &args[1];
            let arr_ref = arr.borrow();

            for item in arr_ref.iter() {
                let predicate_result = call_function(func, item.clone())?;
                if !predicate_result.is_truthy() {
                    return Ok(Value::Bool(false));
                }
            }

            Ok(Value::Bool(true))
        }
        v => Err(RuntimeError::TypeError {
            message: format!("all() requires an array, got {}", v.type_name()),
            span: Span::default(),
        }),
    }
}

fn builtin_slice(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::Array(arr) => {
            let start = args[1].as_int().ok_or_else(|| RuntimeError::TypeError {
                message: "slice() start index must be an integer".into(),
                span: Span::default(),
            })?;
            let end = args[2].as_int().ok_or_else(|| RuntimeError::TypeError {
                message: "slice() end index must be an integer".into(),
                span: Span::default(),
            })?;

            let arr_ref = arr.borrow();
            let len = arr_ref.len() as i64;

            // Handle negative indices
            let start_idx = if start < 0 { 0 } else { start.min(len) } as usize;
            let end_idx = if end < 0 { 0 } else { end.min(len) } as usize;

            if start_idx > end_idx {
                return Ok(Value::Array(Rc::new(RefCell::new(Vec::new()))));
            }

            let sliced = arr_ref[start_idx..end_idx].to_vec();
            Ok(Value::Array(Rc::new(RefCell::new(sliced))))
        }
        v => Err(RuntimeError::TypeError {
            message: format!("slice() requires an array, got {}", v.type_name()),
            span: Span::default(),
        }),
    }
}

fn builtin_concat(args: Vec<Value>) -> Result<Value> {
    match (&args[0], &args[1]) {
        (Value::Array(arr1), Value::Array(arr2)) => {
            let arr1_ref = arr1.borrow();
            let arr2_ref = arr2.borrow();

            let mut result = Vec::with_capacity(arr1_ref.len() + arr2_ref.len());
            result.extend(arr1_ref.iter().cloned());
            result.extend(arr2_ref.iter().cloned());

            Ok(Value::Array(Rc::new(RefCell::new(result))))
        }
        (v1, v2) => Err(RuntimeError::TypeError {
            message: format!(
                "concat() requires two arrays, got {} and {}",
                v1.type_name(),
                v2.type_name()
            ),
            span: Span::default(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn test_builtin_len() {
        assert_eq!(
            builtin_len(vec![Value::String("hello".into())]).unwrap(),
            Value::Int(5)
        );

        let arr = Value::Array(Rc::new(RefCell::new(vec![
            Value::Int(1),
            Value::Int(2),
            Value::Int(3),
        ])));
        assert_eq!(builtin_len(vec![arr]).unwrap(), Value::Int(3));
    }

    #[test]
    fn test_builtin_type_of() {
        assert_eq!(
            builtin_type_of(vec![Value::Int(42)]).unwrap(),
            Value::String("Int".into())
        );
        assert_eq!(
            builtin_type_of(vec![Value::String("hello".into())]).unwrap(),
            Value::String("String".into())
        );
    }

    #[test]
    fn test_builtin_abs() {
        assert_eq!(builtin_abs(vec![Value::Int(-42)]).unwrap(), Value::Int(42));
        assert_eq!(
            builtin_abs(vec![Value::Float(-3.14)]).unwrap(),
            Value::Float(3.14)
        );
    }

    #[test]
    fn test_builtin_min_max() {
        assert_eq!(
            builtin_min(vec![Value::Int(10), Value::Int(5)]).unwrap(),
            Value::Int(5)
        );
        assert_eq!(
            builtin_max(vec![Value::Int(10), Value::Int(5)]).unwrap(),
            Value::Int(10)
        );
    }
}
