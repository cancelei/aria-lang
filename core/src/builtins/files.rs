// File I/O operations for Aria standard library
// These wrap the existing tool_executor functions

use crate::eval::Value;
use crate::tool_executor;

/// Read a file's contents
/// Usage: file_read(path: string) -> string
pub fn file_read(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("file_read expects 1 argument, got {}", args.len()));
    }

    // Use the existing tool_executor for read_file
    tool_executor::execute_tool_command("read_file", &args, Some(30.0))
}

/// Write content to a file
/// Usage: file_write(path: string, content: string) -> null
pub fn file_write(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!(
            "file_write expects 2 arguments, got {}",
            args.len()
        ));
    }

    // Use the existing tool_executor for write_file
    tool_executor::execute_tool_command("write_file", &args, Some(30.0))?;

    Ok(Value::Null)
}

/// Check if a file exists
/// Usage: file_exists(path: string) -> number (1 or 0)
pub fn file_exists(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!(
            "file_exists expects 1 argument, got {}",
            args.len()
        ));
    }

    let path = match &args[0] {
        Value::String(s) => s,
        _ => return Err("file_exists expects a string argument".to_string()),
    };

    // Use test -f to check if file exists
    let test_args = vec![Value::String(format!(
        "test -f '{}' && echo '1' || echo '0'",
        path
    ))];

    match tool_executor::execute_tool_command("shell", &test_args, Some(5.0)) {
        Ok(Value::String(output)) => {
            let trimmed = output.trim();
            if trimmed == "1" {
                Ok(Value::Number(1.0))
            } else {
                Ok(Value::Number(0.0))
            }
        }
        _ => Ok(Value::Number(0.0)),
    }
}

/// Append content to a file
/// Usage: file_append(path: string, content: string) -> null
pub fn file_append(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(format!(
            "file_append expects 2 arguments, got {}",
            args.len()
        ));
    }

    let path = match &args[0] {
        Value::String(s) => s,
        _ => return Err("file_append expects string arguments".to_string()),
    };

    let content = match &args[1] {
        Value::String(s) => s,
        _ => return Err("file_append expects string arguments".to_string()),
    };

    // Use shell command to append
    let cmd = format!(
        "echo '{}' >> '{}'",
        content.replace('\'', "'\\''"),
        path.replace('\'', "'\\''")
    );
    let shell_args = vec![Value::String(cmd)];

    tool_executor::execute_tool_command("shell", &shell_args, Some(30.0))?;

    Ok(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_file_read_write() {
        let test_file = "/tmp/aria_test_file.txt";
        let content = "Hello from Aria!";

        // Clean up if exists
        let _ = fs::remove_file(test_file);

        // Write file
        let write_result = file_write(vec![
            Value::String(test_file.to_string()),
            Value::String(content.to_string()),
        ]);
        assert!(write_result.is_ok());

        // Read file
        let read_result = file_read(vec![Value::String(test_file.to_string())]);
        assert!(read_result.is_ok());

        if let Ok(Value::String(read_content)) = read_result {
            assert!(read_content.contains(content));
        }

        // Clean up
        let _ = fs::remove_file(test_file);
    }

    #[test]
    fn test_file_exists() {
        let test_file = "/tmp/aria_exists_test.txt";

        // Clean up if exists
        let _ = fs::remove_file(test_file);

        // Should not exist
        let result = file_exists(vec![Value::String(test_file.to_string())]);
        assert_eq!(result, Ok(Value::Number(0.0)));

        // Create file
        file_write(vec![
            Value::String(test_file.to_string()),
            Value::String("test".to_string()),
        ])
        .unwrap();

        // Should exist now
        let result = file_exists(vec![Value::String(test_file.to_string())]);
        assert_eq!(result, Ok(Value::Number(1.0)));

        // Clean up
        let _ = fs::remove_file(test_file);
    }
}
