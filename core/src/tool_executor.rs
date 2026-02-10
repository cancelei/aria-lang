// Day 4: Sandboxed Tool Execution
// This module handles real tool execution with timeout enforcement and resource limits

use std::process::{Command, Stdio};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::{Duration, Instant};

use crate::eval::Value;

/// Execute a tool command in a sandboxed subprocess with optional timeout
pub fn execute_tool_command(
    tool_name: &str,
    args: &[Value],
    timeout: Option<f64>,
    max_output_bytes: u64,
) -> Result<Value, String> {
    let timeout = timeout.unwrap_or(30.0); // Default 30s timeout

    // Build the command based on tool name and arguments
    let (program, cmd_args) = build_command(tool_name, args)?;

    println!(
        "[Sandbox] Executing: {} {:?} (timeout: {:.1}s)",
        program, cmd_args, timeout
    );

    let start = Instant::now();
    let killed = Arc::new(AtomicBool::new(false));

    // Spawn the child process
    let child = Command::new(&program)
        .args(&cmd_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            format!(
                "[Sandbox Error] Failed to execute tool '{}': {}",
                tool_name, e
            )
        })?;

    // Spawn a watchdog thread for timeout enforcement
    let killed_clone = killed.clone();
    let child_id = child.id();
    let timeout_duration = Duration::from_secs_f64(timeout);

    let watchdog = thread::spawn(move || {
        thread::sleep(timeout_duration);
        if !killed_clone.load(Ordering::Relaxed) {
            killed_clone.store(true, Ordering::Relaxed);
            // Kill the process on timeout
            unsafe {
                libc::kill(child_id as i32, libc::SIGKILL);
            }
        }
    });

    // Wait for process completion
    let output = child.wait_with_output();
    let elapsed = start.elapsed();

    // Signal watchdog we're done
    killed.store(true, Ordering::Relaxed);

    // Check if we were killed by timeout
    if elapsed >= timeout_duration {
        let _ = watchdog.join();
        return Err(format!(
            "[Timeout] Tool '{}' exceeded timeout of {:.1}s (killed after {:.1}s)",
            tool_name,
            timeout,
            elapsed.as_secs_f64()
        ));
    }

    let _ = watchdog.join();

    match output {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                // Day 5: Enforce output size limit
                if stdout.len() as u64 > max_output_bytes {
                    let truncated = &stdout[..max_output_bytes as usize];
                    println!(
                        "[Sandbox] Output truncated: {} -> {} bytes (limit: {})",
                        stdout.len(),
                        max_output_bytes,
                        max_output_bytes
                    );
                    return Ok(Value::String(format!(
                        "{}... [TRUNCATED: {} bytes exceeded {} byte limit]",
                        truncated,
                        stdout.len(),
                        max_output_bytes
                    )));
                }
                println!(
                    "[Sandbox] Success in {:.2}s: {} bytes output",
                    elapsed.as_secs_f64(),
                    stdout.len()
                );
                Ok(Value::String(stdout))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                Err(format!(
                    "[Tool Error] '{}' failed with exit code {:?}: {}",
                    tool_name,
                    output.status.code(),
                    stderr
                ))
            }
        }
        Err(e) => Err(format!(
            "[Sandbox Error] Failed to execute tool '{}': {}",
            tool_name, e
        )),
    }
}

/// Convert a Value to a string for command arguments
fn value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Null => String::new(),
        Value::Agent(a) => a.clone(),
        Value::Array(items) => {
            let parts: Vec<String> = items.iter().map(value_to_string).collect();
            parts.join(", ")
        }
        Value::Bool(b) => b.to_string(),
    }
}

/// Build a shell command from tool name and arguments
pub fn build_command(tool_name: &str, args: &[Value]) -> Result<(String, Vec<String>), String> {
    match tool_name {
        "echo" => {
            let text = if args.is_empty() {
                String::new()
            } else {
                value_to_string(&args[0])
            };
            Ok(("echo".to_string(), vec![text]))
        }
        "shell" => {
            // shell(command) - execute arbitrary shell command
            if args.is_empty() {
                return Err("[Tool Error] shell() requires a command argument".to_string());
            }
            let cmd = value_to_string(&args[0]);
            Ok(("sh".to_string(), vec!["-c".to_string(), cmd]))
        }
        "read_file" => {
            // read_file(path) - read a file
            if args.is_empty() {
                return Err("[Tool Error] read_file() requires a path argument".to_string());
            }
            let path = value_to_string(&args[0]);
            Ok(("cat".to_string(), vec![path]))
        }
        "write_file" => {
            // write_file(path, content) - write content to file
            if args.len() < 2 {
                return Err(
                    "[Tool Error] write_file() requires path and content arguments".to_string(),
                );
            }
            let path = value_to_string(&args[0]);
            let content = value_to_string(&args[1]);
            Ok((
                "sh".to_string(),
                vec![
                    "-c".to_string(),
                    format!(
                        "echo '{}' > '{}'",
                        escape_shell_arg(&content),
                        escape_shell_arg(&path)
                    ),
                ],
            ))
        }
        _ => {
            // Unknown tool - try to execute as command with args
            let cmd_args: Vec<String> = args.iter().map(value_to_string).collect();
            Ok((tool_name.to_string(), cmd_args))
        }
    }
}

/// Escape single quotes in shell arguments
pub fn escape_shell_arg(s: &str) -> String {
    s.replace('\'', "'\\''")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_command_echo() {
        let (prog, args) = build_command("echo", &[Value::String("hello".to_string())]).unwrap();
        assert_eq!(prog, "echo");
        assert_eq!(args, vec!["hello"]);
    }

    #[test]
    fn test_build_command_shell() {
        let (prog, args) = build_command("shell", &[Value::String("ls -la".to_string())]).unwrap();
        assert_eq!(prog, "sh");
        assert_eq!(args, vec!["-c", "ls -la"]);
    }

    #[test]
    fn test_escape_shell_arg() {
        assert_eq!(escape_shell_arg("hello"), "hello");
        assert_eq!(escape_shell_arg("it's"), "it'\\''s");
    }

    const DEFAULT_MAX_OUTPUT: u64 = 1_048_576;

    #[test]
    fn test_real_tool_execution() {
        let result = execute_tool_command(
            "echo",
            &[Value::String("test_output".to_string())],
            Some(5.0),
            DEFAULT_MAX_OUTPUT,
        );
        assert!(result.is_ok());
        if let Ok(Value::String(s)) = result {
            assert!(s.contains("test_output"));
        }
    }

    #[test]
    fn test_timeout_enforcement() {
        let result = execute_tool_command(
            "shell",
            &[Value::String("sleep 10".to_string())],
            Some(1.0),
            DEFAULT_MAX_OUTPUT,
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Timeout"));
    }

    #[test]
    fn test_output_truncation() {
        // Generate output larger than the limit
        let result = execute_tool_command(
            "shell",
            &[Value::String("printf 'A%.0s' {1..100}".to_string())],
            Some(5.0),
            10, // Very small limit: 10 bytes
        );
        assert!(result.is_ok());
        if let Ok(Value::String(s)) = result {
            assert!(s.contains("TRUNCATED"));
        }
    }

    #[test]
    fn test_build_command_read_file() {
        let (prog, args) =
            build_command("read_file", &[Value::String("/tmp/test.txt".to_string())]).unwrap();
        assert_eq!(prog, "cat");
        assert_eq!(args, vec!["/tmp/test.txt"]);
    }

    #[test]
    fn test_build_command_write_file() {
        let (prog, args) = build_command(
            "write_file",
            &[
                Value::String("/tmp/out.txt".to_string()),
                Value::String("content".to_string()),
            ],
        )
        .unwrap();
        assert_eq!(prog, "sh");
        assert_eq!(args.len(), 2);
        assert_eq!(args[0], "-c");
        assert!(args[1].contains("content"));
        assert!(args[1].contains("/tmp/out.txt"));
    }

    #[test]
    fn test_build_command_unknown_tool() {
        let (prog, args) =
            build_command("my_custom_tool", &[Value::String("arg1".to_string())]).unwrap();
        assert_eq!(prog, "my_custom_tool");
        assert_eq!(args, vec!["arg1"]);
    }

    #[test]
    fn test_execute_nonexistent_command() {
        let result = execute_tool_command(
            "nonexistent_binary_xyz_12345",
            &[],
            Some(5.0),
            DEFAULT_MAX_OUTPUT,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Sandbox Error"));
    }
}
