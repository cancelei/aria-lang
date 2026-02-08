// String operations for Aria standard library

use crate::eval::Value;

/// Get the length of a string
pub fn str_len(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("str_len expects 1 argument, got {}", args.len()));
    }
    match &args[0] {
        Value::String(s) => Ok(Value::Number(s.len() as f64)),
        _ => Err("str_len expects a string argument".to_string()),
    }
}

/// Convert string to uppercase
pub fn str_upper(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("str_upper expects 1 argument, got {}", args.len()));
    }
    match &args[0] {
        Value::String(s) => Ok(Value::String(s.to_uppercase())),
        _ => Err("str_upper expects a string argument".to_string()),
    }
}

/// Convert string to lowercase
pub fn str_lower(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("str_lower expects 1 argument, got {}", args.len()));
    }
    match &args[0] {
        Value::String(s) => Ok(Value::String(s.to_lowercase())),
        _ => Err("str_lower expects a string argument".to_string()),
    }
}

/// Trim whitespace from both ends
pub fn str_trim(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("str_trim expects 1 argument, got {}", args.len()));
    }
    match &args[0] {
        Value::String(s) => Ok(Value::String(s.trim().to_string())),
        _ => Err("str_trim expects a string argument".to_string()),
    }
}

/// Check if string contains a substring
pub fn str_contains(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!(
            "str_contains expects 2 arguments, got {}",
            args.len()
        ));
    }
    match (&args[0], &args[1]) {
        (Value::String(haystack), Value::String(needle)) => {
            Ok(Value::Number(if haystack.contains(needle.as_str()) {
                1.0
            } else {
                0.0
            }))
        }
        _ => Err("str_contains expects two string arguments".to_string()),
    }
}

/// Check if string starts with a prefix
pub fn str_starts_with(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!(
            "str_starts_with expects 2 arguments, got {}",
            args.len()
        ));
    }
    match (&args[0], &args[1]) {
        (Value::String(s), Value::String(prefix)) => {
            Ok(Value::Number(if s.starts_with(prefix.as_str()) {
                1.0
            } else {
                0.0
            }))
        }
        _ => Err("str_starts_with expects two string arguments".to_string()),
    }
}

/// Check if string ends with a suffix
pub fn str_ends_with(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!(
            "str_ends_with expects 2 arguments, got {}",
            args.len()
        ));
    }
    match (&args[0], &args[1]) {
        (Value::String(s), Value::String(suffix)) => {
            Ok(Value::Number(if s.ends_with(suffix.as_str()) {
                1.0
            } else {
                0.0
            }))
        }
        _ => Err("str_ends_with expects two string arguments".to_string()),
    }
}

/// Replace occurrences of a substring
pub fn str_replace(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 3 {
        return Err(format!(
            "str_replace expects 3 arguments, got {}",
            args.len()
        ));
    }
    match (&args[0], &args[1], &args[2]) {
        (Value::String(s), Value::String(from), Value::String(to)) => {
            Ok(Value::String(s.replace(from.as_str(), to)))
        }
        _ => Err("str_replace expects three string arguments".to_string()),
    }
}

/// Concatenate two strings
pub fn str_concat(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!(
            "str_concat expects 2 arguments, got {}",
            args.len()
        ));
    }
    let a = match &args[0] {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => format!("{:?}", args[0]),
    };
    let b = match &args[1] {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => format!("{:?}", args[1]),
    };
    Ok(Value::String(format!("{}{}", a, b)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_str_len() {
        let result = str_len(vec![Value::String("hello".to_string())]);
        assert_eq!(result, Ok(Value::Number(5.0)));
    }

    #[test]
    fn test_str_upper() {
        let result = str_upper(vec![Value::String("hello".to_string())]);
        assert_eq!(result, Ok(Value::String("HELLO".to_string())));
    }

    #[test]
    fn test_str_lower() {
        let result = str_lower(vec![Value::String("HELLO".to_string())]);
        assert_eq!(result, Ok(Value::String("hello".to_string())));
    }

    #[test]
    fn test_str_trim() {
        let result = str_trim(vec![Value::String("  hello  ".to_string())]);
        assert_eq!(result, Ok(Value::String("hello".to_string())));
    }

    #[test]
    fn test_str_contains() {
        let result = str_contains(vec![
            Value::String("hello world".to_string()),
            Value::String("world".to_string()),
        ]);
        assert_eq!(result, Ok(Value::Number(1.0)));
    }

    #[test]
    fn test_str_starts_with() {
        let result = str_starts_with(vec![
            Value::String("hello world".to_string()),
            Value::String("hello".to_string()),
        ]);
        assert_eq!(result, Ok(Value::Number(1.0)));
    }

    #[test]
    fn test_str_ends_with() {
        let result = str_ends_with(vec![
            Value::String("hello world".to_string()),
            Value::String("world".to_string()),
        ]);
        assert_eq!(result, Ok(Value::Number(1.0)));
    }

    #[test]
    fn test_str_replace() {
        let result = str_replace(vec![
            Value::String("hello world".to_string()),
            Value::String("world".to_string()),
            Value::String("aria".to_string()),
        ]);
        assert_eq!(result, Ok(Value::String("hello aria".to_string())));
    }

    #[test]
    fn test_str_concat() {
        let result = str_concat(vec![
            Value::String("hello ".to_string()),
            Value::String("world".to_string()),
        ]);
        assert_eq!(result, Ok(Value::String("hello world".to_string())));
    }

    #[test]
    fn test_str_len_wrong_args() {
        let result = str_len(vec![
            Value::String("a".to_string()),
            Value::String("b".to_string()),
        ]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 1 argument"));
    }

    #[test]
    fn test_str_len_wrong_type() {
        let result = str_len(vec![Value::Number(42.0)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("string argument"));
    }
}
