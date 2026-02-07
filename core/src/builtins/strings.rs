// String operations for Aria standard library

use crate::eval::Value;

/// Get the length of a string
/// Usage: str_len(s: string) -> number
pub fn str_len(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("str_len expects 1 argument, got {}", args.len()));
    }

    match &args[0] {
        Value::String(s) => Ok(Value::Number(s.len() as f64)),
        _ => Err("str_len expects a string argument".to_string()),
    }
}

/// Concatenate two strings
/// Usage: str_concat(s1: string, s2: string) -> string
pub fn str_concat(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!(
            "str_concat expects 2 arguments, got {}",
            args.len()
        ));
    }

    let s1 = match &args[0] {
        Value::String(s) => s,
        _ => return Err("str_concat expects string arguments".to_string()),
    };

    let s2 = match &args[1] {
        Value::String(s) => s,
        _ => return Err("str_concat expects string arguments".to_string()),
    };

    Ok(Value::String(format!("{}{}", s1, s2)))
}

/// Convert string to uppercase
/// Usage: str_upper(s: string) -> string
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
/// Usage: str_lower(s: string) -> string
pub fn str_lower(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("str_lower expects 1 argument, got {}", args.len()));
    }

    match &args[0] {
        Value::String(s) => Ok(Value::String(s.to_lowercase())),
        _ => Err("str_lower expects a string argument".to_string()),
    }
}

/// Trim whitespace from both ends of a string
/// Usage: str_trim(s: string) -> string
pub fn str_trim(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("str_trim expects 1 argument, got {}", args.len()));
    }

    match &args[0] {
        Value::String(s) => Ok(Value::String(s.trim().to_string())),
        _ => Err("str_trim expects a string argument".to_string()),
    }
}

/// Check if a string contains a substring
/// Usage: str_contains(haystack: string, needle: string) -> number (1 or 0)
pub fn str_contains(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!(
            "str_contains expects 2 arguments, got {}",
            args.len()
        ));
    }

    let haystack = match &args[0] {
        Value::String(s) => s,
        _ => return Err("str_contains expects string arguments".to_string()),
    };

    let needle = match &args[1] {
        Value::String(s) => s,
        _ => return Err("str_contains expects string arguments".to_string()),
    };

    Ok(Value::Number(if haystack.contains(needle) {
        1.0
    } else {
        0.0
    }))
}

/// Replace all occurrences of a substring
/// Usage: str_replace(s: string, old: string, new: string) -> string
pub fn str_replace(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 3 {
        return Err(format!(
            "str_replace expects 3 arguments, got {}",
            args.len()
        ));
    }

    let s = match &args[0] {
        Value::String(s) => s,
        _ => return Err("str_replace expects string arguments".to_string()),
    };

    let old = match &args[1] {
        Value::String(s) => s,
        _ => return Err("str_replace expects string arguments".to_string()),
    };

    let new = match &args[2] {
        Value::String(s) => s,
        _ => return Err("str_replace expects string arguments".to_string()),
    };

    Ok(Value::String(s.replace(old, new)))
}

/// Split a string by a delimiter
/// Usage: str_split(s: string, delim: string) -> string (JSON array)
pub fn str_split(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!("str_split expects 2 arguments, got {}", args.len()));
    }

    let s = match &args[0] {
        Value::String(s) => s,
        _ => return Err("str_split expects string arguments".to_string()),
    };

    let delim = match &args[1] {
        Value::String(s) => s,
        _ => return Err("str_split expects string arguments".to_string()),
    };

    let parts: Vec<String> = s.split(delim).map(|s| s.to_string()).collect();

    // Return as JSON array string for now (until we have proper array type)
    let json =
        serde_json::to_string(&parts).map_err(|e| format!("Failed to serialize array: {}", e))?;

    Ok(Value::String(json))
}

/// Check if string starts with a prefix
/// Usage: str_starts_with(s: string, prefix: string) -> number (1 or 0)
pub fn str_starts_with(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!(
            "str_starts_with expects 2 arguments, got {}",
            args.len()
        ));
    }

    let s = match &args[0] {
        Value::String(s) => s,
        _ => return Err("str_starts_with expects string arguments".to_string()),
    };

    let prefix = match &args[1] {
        Value::String(s) => s,
        _ => return Err("str_starts_with expects string arguments".to_string()),
    };

    Ok(Value::Number(if s.starts_with(prefix) { 1.0 } else { 0.0 }))
}

/// Check if string ends with a suffix
/// Usage: str_ends_with(s: string, suffix: string) -> number (1 or 0)
pub fn str_ends_with(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!(
            "str_ends_with expects 2 arguments, got {}",
            args.len()
        ));
    }

    let s = match &args[0] {
        Value::String(s) => s,
        _ => return Err("str_ends_with expects string arguments".to_string()),
    };

    let suffix = match &args[1] {
        Value::String(s) => s,
        _ => return Err("str_ends_with expects string arguments".to_string()),
    };

    Ok(Value::Number(if s.ends_with(suffix) { 1.0 } else { 0.0 }))
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
    fn test_str_concat() {
        let result = str_concat(vec![
            Value::String("hello".to_string()),
            Value::String(" world".to_string()),
        ]);
        assert_eq!(result, Ok(Value::String("hello world".to_string())));
    }

    #[test]
    fn test_str_upper() {
        let result = str_upper(vec![Value::String("hello".to_string())]);
        assert_eq!(result, Ok(Value::String("HELLO".to_string())));
    }

    #[test]
    fn test_str_lower() {
        let result = str_lower(vec![Value::String("WORLD".to_string())]);
        assert_eq!(result, Ok(Value::String("world".to_string())));
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

        let result = str_contains(vec![
            Value::String("hello".to_string()),
            Value::String("world".to_string()),
        ]);
        assert_eq!(result, Ok(Value::Number(0.0)));
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
}
