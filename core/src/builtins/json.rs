// JSON operations for Aria standard library

use crate::eval::Value;

/// Parse a JSON string (validates it)
pub fn json_parse(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("json_parse expects 1 argument, got {}", args.len()));
    }
    let json_str = match &args[0] {
        Value::String(s) => s,
        _ => return Err("json_parse expects a string argument".to_string()),
    };

    // Parse to validate, then return as string
    let _parsed: serde_json::Value =
        serde_json::from_str(json_str).map_err(|e| format!("Invalid JSON: {}", e))?;
    Ok(Value::String(json_str.clone()))
}

/// Convert a value to JSON string
pub fn json_stringify(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "json_stringify expects 1 argument, got {}",
            args.len()
        ));
    }
    let json_value = value_to_json(&args[0])?;
    let json_str =
        serde_json::to_string(&json_value).map_err(|e| format!("Failed to stringify: {}", e))?;
    Ok(Value::String(json_str))
}

/// Get a value from a JSON object by key
pub fn json_get(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!("json_get expects 2 arguments, got {}", args.len()));
    }
    let json_str = match &args[0] {
        Value::String(s) => s,
        _ => return Err("json_get expects first argument to be a string".to_string()),
    };
    let key = match &args[1] {
        Value::String(s) => s,
        _ => return Err("json_get expects second argument to be a string".to_string()),
    };

    let parsed: serde_json::Value =
        serde_json::from_str(json_str).map_err(|e| format!("Invalid JSON: {}", e))?;

    match &parsed {
        serde_json::Value::Object(map) => match map.get(key.as_str()) {
            Some(value) => {
                let result_str = serde_json::to_string(value)
                    .map_err(|e| format!("Failed to serialize value: {}", e))?;
                Ok(Value::String(result_str))
            }
            None => Err(format!("Key '{}' not found in JSON object", key)),
        },
        _ => Err("json_get expects a JSON object".to_string()),
    }
}

fn value_to_json(value: &Value) -> Result<serde_json::Value, String> {
    match value {
        Value::String(s) => Ok(serde_json::Value::String(s.clone())),
        Value::Number(n) => serde_json::Number::from_f64(*n)
            .map(serde_json::Value::Number)
            .ok_or("Invalid number for JSON".to_string()),
        Value::Null => Ok(serde_json::Value::Null),
        Value::Agent(a) => Ok(serde_json::Value::String(format!("Agent({})", a))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_parse_valid() {
        let json = r#"{"name":"aria","version":1}"#;
        let result = json_parse(vec![Value::String(json.to_string())]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_json_parse_invalid() {
        let json = r#"{"name":"aria""#;
        let result = json_parse(vec![Value::String(json.to_string())]);
        assert!(result.is_err());
    }

    #[test]
    fn test_json_stringify() {
        let result = json_stringify(vec![Value::String("hello".to_string())]);
        assert_eq!(result, Ok(Value::String("\"hello\"".to_string())));
    }

    #[test]
    fn test_json_get() {
        let json = r#"{"name":"aria","version":1}"#;
        let result = json_get(vec![
            Value::String(json.to_string()),
            Value::String("name".to_string()),
        ]);
        if let Ok(Value::String(s)) = result {
            assert_eq!(s, "\"aria\"");
        } else {
            panic!("Expected Ok(String)");
        }
    }

    #[test]
    fn test_json_get_missing_key() {
        let json = r#"{"name":"aria"}"#;
        let result = json_get(vec![
            Value::String(json.to_string()),
            Value::String("missing".to_string()),
        ]);
        assert!(result.is_err());
    }
}
