// Array operations for Aria standard library
// Arrays are represented as JSON array strings internally

use crate::eval::Value;

/// Create an array by splitting a string
pub fn arr_from_split(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!(
            "arr_from_split expects 2 arguments, got {}",
            args.len()
        ));
    }
    let s = match &args[0] {
        Value::String(s) => s,
        _ => return Err("arr_from_split expects string arguments".to_string()),
    };
    let delim = match &args[1] {
        Value::String(s) => s,
        _ => return Err("arr_from_split expects string arguments".to_string()),
    };

    let parts: Vec<String> = s.split(delim.as_str()).map(|s| s.to_string()).collect();
    let json =
        serde_json::to_string(&parts).map_err(|e| format!("Failed to serialize array: {}", e))?;
    Ok(Value::String(json))
}

/// Get the length of an array
pub fn arr_len(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("arr_len expects 1 argument, got {}", args.len()));
    }
    let arr_str = match &args[0] {
        Value::String(s) => s,
        _ => return Err("arr_len expects a string (JSON array) argument".to_string()),
    };
    let arr: Vec<serde_json::Value> =
        serde_json::from_str(arr_str).map_err(|e| format!("Invalid JSON array: {}", e))?;
    Ok(Value::Number(arr.len() as f64))
}

/// Get an element from an array by index
pub fn arr_get(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!("arr_get expects 2 arguments, got {}", args.len()));
    }
    let arr_str = match &args[0] {
        Value::String(s) => s,
        _ => return Err("arr_get expects first argument to be a string (JSON array)".to_string()),
    };
    let index = match &args[1] {
        Value::Number(n) => *n as usize,
        _ => return Err("arr_get expects second argument to be a number".to_string()),
    };

    let arr: Vec<serde_json::Value> =
        serde_json::from_str(arr_str).map_err(|e| format!("Invalid JSON array: {}", e))?;

    if index >= arr.len() {
        return Err(format!(
            "Index {} out of bounds (array length: {})",
            index,
            arr.len()
        ));
    }

    match &arr[index] {
        serde_json::Value::String(s) => Ok(Value::String(s.clone())),
        serde_json::Value::Number(n) => Ok(Value::Number(n.as_f64().unwrap_or(0.0))),
        serde_json::Value::Bool(b) => Ok(Value::Number(if *b { 1.0 } else { 0.0 })),
        serde_json::Value::Null => Ok(Value::Null),
        other => Ok(Value::String(other.to_string())),
    }
}

/// Join array elements into a string
pub fn arr_join(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!("arr_join expects 2 arguments, got {}", args.len()));
    }
    let arr_str = match &args[0] {
        Value::String(s) => s,
        _ => return Err("arr_join expects first argument to be a string (JSON array)".to_string()),
    };
    let delim = match &args[1] {
        Value::String(s) => s,
        _ => return Err("arr_join expects second argument to be a string".to_string()),
    };

    let arr: Vec<String> =
        serde_json::from_str(arr_str).map_err(|e| format!("Invalid JSON array: {}", e))?;
    Ok(Value::String(arr.join(delim)))
}

/// Push an element to the end of an array (returns new array)
pub fn arr_push(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!("arr_push expects 2 arguments, got {}", args.len()));
    }
    let arr_str = match &args[0] {
        Value::String(s) => s,
        _ => return Err("arr_push expects first argument to be a string (JSON array)".to_string()),
    };
    let item = match &args[1] {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Null => "null".to_string(),
        Value::Agent(a) => a.clone(),
        Value::Array(items) => {
            let json_items: Vec<serde_json::Value> = items.iter().map(|v| match v {
                Value::String(s) => serde_json::Value::String(s.clone()),
                Value::Number(n) => serde_json::Number::from_f64(*n)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null),
                Value::Bool(b) => serde_json::Value::Bool(*b),
                Value::Null => serde_json::Value::Null,
                other => serde_json::Value::String(format!("{}", other)),
            }).collect();
            serde_json::to_string(&json_items).unwrap_or_else(|_| "[]".to_string())
        }
        Value::Bool(b) => b.to_string(),
    };

    let mut arr: Vec<String> =
        serde_json::from_str(arr_str).map_err(|e| format!("Invalid JSON array: {}", e))?;
    arr.push(item);
    let json =
        serde_json::to_string(&arr).map_err(|e| format!("Failed to serialize array: {}", e))?;
    Ok(Value::String(json))
}

/// Pop an element from the end of an array (returns new array)
pub fn arr_pop(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("arr_pop expects 1 argument, got {}", args.len()));
    }
    let arr_str = match &args[0] {
        Value::String(s) => s,
        _ => return Err("arr_pop expects a string (JSON array) argument".to_string()),
    };

    let mut arr: Vec<String> =
        serde_json::from_str(arr_str).map_err(|e| format!("Invalid JSON array: {}", e))?;

    if arr.is_empty() {
        return Err("Cannot pop from empty array".to_string());
    }
    arr.pop();
    let json =
        serde_json::to_string(&arr).map_err(|e| format!("Failed to serialize array: {}", e))?;
    Ok(Value::String(json))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arr_from_split() {
        let result = arr_from_split(vec![
            Value::String("a,b,c".to_string()),
            Value::String(",".to_string()),
        ]);
        assert!(result.is_ok());
        if let Ok(Value::String(s)) = result {
            assert_eq!(s, r#"["a","b","c"]"#);
        }
    }

    #[test]
    fn test_arr_len() {
        let result = arr_len(vec![Value::String(r#"["a","b","c"]"#.to_string())]);
        assert_eq!(result, Ok(Value::Number(3.0)));
    }

    #[test]
    fn test_arr_get() {
        let result = arr_get(vec![
            Value::String(r#"["a","b","c"]"#.to_string()),
            Value::Number(1.0),
        ]);
        assert_eq!(result, Ok(Value::String("b".to_string())));
    }

    #[test]
    fn test_arr_join() {
        let result = arr_join(vec![
            Value::String(r#"["a","b","c"]"#.to_string()),
            Value::String("-".to_string()),
        ]);
        assert_eq!(result, Ok(Value::String("a-b-c".to_string())));
    }

    #[test]
    fn test_arr_push() {
        let result = arr_push(vec![
            Value::String(r#"["a","b"]"#.to_string()),
            Value::String("c".to_string()),
        ]);
        if let Ok(Value::String(s)) = result {
            assert_eq!(s, r#"["a","b","c"]"#);
        } else {
            panic!("Expected Ok(String)");
        }
    }

    #[test]
    fn test_arr_pop() {
        let result = arr_pop(vec![Value::String(r#"["a","b","c"]"#.to_string())]);
        if let Ok(Value::String(s)) = result {
            assert_eq!(s, r#"["a","b"]"#);
        } else {
            panic!("Expected Ok(String)");
        }
    }

    #[test]
    fn test_arr_get_out_of_bounds() {
        let result = arr_get(vec![
            Value::String(r#"["a","b"]"#.to_string()),
            Value::Number(5.0),
        ]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("out of bounds"));
    }

    #[test]
    fn test_arr_pop_empty() {
        let result = arr_pop(vec![Value::String("[]".to_string())]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty array"));
    }
}
