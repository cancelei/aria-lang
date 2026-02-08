// File operations for Aria standard library

use crate::eval::Value;
use std::fs;
use std::path::Path;

/// Read a file's contents
pub fn file_read(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("file_read expects 1 argument, got {}", args.len()));
    }
    let path = match &args[0] {
        Value::String(s) => s,
        _ => return Err("file_read expects a string path argument".to_string()),
    };
    let content =
        fs::read_to_string(path).map_err(|e| format!("Failed to read '{}': {}", path, e))?;
    Ok(Value::String(content))
}

/// Write content to a file
pub fn file_write(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!(
            "file_write expects 2 arguments, got {}",
            args.len()
        ));
    }
    let path = match &args[0] {
        Value::String(s) => s,
        _ => return Err("file_write expects first argument to be a string path".to_string()),
    };
    let content = match &args[1] {
        Value::String(s) => s,
        _ => return Err("file_write expects second argument to be a string".to_string()),
    };
    fs::write(path, content).map_err(|e| format!("Failed to write '{}': {}", path, e))?;
    Ok(Value::String(format!(
        "Wrote {} bytes to {}",
        content.len(),
        path
    )))
}

/// Check if a file exists
pub fn file_exists(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "file_exists expects 1 argument, got {}",
            args.len()
        ));
    }
    let path = match &args[0] {
        Value::String(s) => s,
        _ => return Err("file_exists expects a string path argument".to_string()),
    };
    Ok(Value::Number(if Path::new(path).exists() {
        1.0
    } else {
        0.0
    }))
}

/// Append content to a file
pub fn file_append(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!(
            "file_append expects 2 arguments, got {}",
            args.len()
        ));
    }
    let path = match &args[0] {
        Value::String(s) => s,
        _ => return Err("file_append expects first argument to be a string path".to_string()),
    };
    let content = match &args[1] {
        Value::String(s) => s,
        _ => return Err("file_append expects second argument to be a string".to_string()),
    };

    use std::fs::OpenOptions;
    use std::io::Write;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("Failed to open '{}': {}", path, e))?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("Failed to append to '{}': {}", path, e))?;
    Ok(Value::String(format!(
        "Appended {} bytes to {}",
        content.len(),
        path
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_file_read_write() {
        let path = "/tmp/aria_test_rw.txt";
        let content = "hello from aria";

        // Write
        let result = file_write(vec![
            Value::String(path.to_string()),
            Value::String(content.to_string()),
        ]);
        assert!(result.is_ok());

        // Read
        let result = file_read(vec![Value::String(path.to_string())]);
        assert_eq!(result, Ok(Value::String(content.to_string())));

        // Cleanup
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_file_exists() {
        let result = file_exists(vec![Value::String("/tmp".to_string())]);
        assert_eq!(result, Ok(Value::Number(1.0)));

        let result = file_exists(vec![Value::String("/nonexistent_path_12345".to_string())]);
        assert_eq!(result, Ok(Value::Number(0.0)));
    }

    #[test]
    fn test_file_read_nonexistent() {
        let result = file_read(vec![Value::String(
            "/nonexistent_path_aria_test_12345.txt".to_string(),
        )]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to read"));
    }

    #[test]
    fn test_file_append() {
        let path = "/tmp/aria_test_append.txt";
        // Clean up from any previous test run
        let _ = fs::remove_file(path);

        // Append twice
        let result1 = file_append(vec![
            Value::String(path.to_string()),
            Value::String("hello".to_string()),
        ]);
        assert!(result1.is_ok());

        let result2 = file_append(vec![
            Value::String(path.to_string()),
            Value::String(" world".to_string()),
        ]);
        assert!(result2.is_ok());

        // Read and verify
        let content = fs::read_to_string(path).unwrap();
        assert_eq!(content, "hello world");

        // Cleanup
        let _ = fs::remove_file(path);
    }
}
