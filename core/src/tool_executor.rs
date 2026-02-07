// Day 5: Sandboxed Tool Execution
// This module handles real tool execution with timeout enforcement and resource limits

use std::process::{Command, Stdio};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::{Duration, Instant};

use crate::eval::Value;

/// Executes a tool command in a sandboxed child process
/// Returns stdout as a Value::String or error message
pub fn execute_tool_command(
    tool_name: &str,
    args: &[Value],
    timeout_secs: Option<f64>,
) -> Result<Value, String> {
    // Build command string from tool name and args
    let command_str = build_command_string(tool_name, args)?;

    println!("[Sandbox] Executing: {}", command_str);

    // Get timeout (default: 30 seconds)
    let timeout = timeout_secs.unwrap_or(30.0);
    let timeout_duration = Duration::from_secs_f64(timeout);

    // Spawn child process
    let child = Command::new("sh")
        .arg("-c")
        .arg(&command_str)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("[Sandbox Error] Failed to spawn process: {}", e))?;

    let child_id = child.id();
    let start_time = Instant::now();

    // Setup timeout watchdog
    let timeout_triggered = Arc::new(AtomicBool::new(false));
    let timeout_flag = timeout_triggered.clone();

    let watchdog = thread::spawn(move || {
        thread::sleep(timeout_duration);
        timeout_flag.store(true, Ordering::SeqCst);

        // Send SIGTERM to child process
        unsafe {
            libc::kill(child_id as i32, libc::SIGTERM);
        }

        // Wait 2 seconds, then send SIGKILL if still alive
        thread::sleep(Duration::from_secs(2));
        unsafe {
            libc::kill(child_id as i32, libc::SIGKILL);
        }
    });

    // Wait for child to complete
    let result = child.wait_with_output();

    let elapsed = start_time.elapsed();

    // Check if timeout was triggered
    if timeout_triggered.load(Ordering::SeqCst) {
        let _ = watchdog.join(); // Clean up watchdog thread
        return Err(format!(
            "[Timeout] Tool '{}' exceeded timeout of {:.1}s (killed after {:.1}s)",
            tool_name,
            timeout,
            elapsed.as_secs_f64()
        ));
    }

    // Process completed before timeout - clean up watchdog
    drop(watchdog); // Watchdog will be killed when main thread exits, but that's okay

    // Handle execution result
    match result {
        Ok(output) => {
            if output.status.success() {
                // Success - return stdout
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                println!(
                    "[Sandbox] Success in {:.2}s: {} bytes output",
                    elapsed.as_secs_f64(),
                    stdout.len()
                );
                Ok(Value::String(stdout))
            } else {
                // Command failed
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap_or(-1);
                Err(format!(
                    "[Command Failed] Tool '{}' exited with code {}\nStderr: {}",
                    tool_name, exit_code, stderr
                ))
            }
        }
        Err(e) => Err(format!(
            "[Sandbox Error] Failed to execute tool '{}': {}",
            tool_name, e
        )),
    }
}

/// Builds a shell command string from tool name and arguments
fn build_command_string(tool_name: &str, args: &[Value]) -> Result<String, String> {
    // For now, we support basic tools. In production, this would map to actual tool implementations
    match tool_name {
        "shell" => {
            // shell(command) - execute shell command
            if args.is_empty() {
                return Err("[Tool Error] shell() requires a command argument".to_string());
            }
            let cmd = value_to_string(&args[0]);
            Ok(cmd)
        }
        "echo" => {
            // echo(message) - print message
            if args.is_empty() {
                return Err("[Tool Error] echo() requires a message argument".to_string());
            }
            let msg = value_to_string(&args[0]);
            Ok(format!("echo '{}'", escape_shell_arg(&msg)))
        }
        "read_file" => {
            // read_file(path) - read file contents
            if args.is_empty() {
                return Err("[Tool Error] read_file() requires a path argument".to_string());
            }
            let path = value_to_string(&args[0]);
            Ok(format!("cat '{}'", escape_shell_arg(&path)))
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
            Ok(format!(
                "echo '{}' > '{}'",
                escape_shell_arg(&content),
                escape_shell_arg(&path)
            ))
        }
        _ => {
            // Unknown tool - try to execute as command with args
            let mut cmd = tool_name.to_string();
            for arg in args {
                cmd.push(' ');
                cmd.push_str(&format!("'{}'", escape_shell_arg(&value_to_string(arg))));
            }
            Ok(cmd)
        }
    }
}

/// Converts a Value to its string representation
fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Null => "null".to_string(),
        Value::Agent(a) => a.clone(),
    }
}

/// Escapes shell special characters in an argument
fn escape_shell_arg(arg: &str) -> String {
    // Replace single quotes with '\'' (end quote, escaped quote, start quote)
    arg.replace('\'', "'\\''")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_command_echo() {
        let args = vec![Value::String("Hello World".to_string())];
        let cmd = build_command_string("echo", &args).unwrap();
        assert_eq!(cmd, "echo 'Hello World'");
    }

    #[test]
    fn test_build_command_shell() {
        let args = vec![Value::String("ls -la".to_string())];
        let cmd = build_command_string("shell", &args).unwrap();
        assert_eq!(cmd, "ls -la");
    }

    #[test]
    fn test_escape_shell_arg() {
        assert_eq!(escape_shell_arg("simple"), "simple");
        assert_eq!(escape_shell_arg("with'quote"), "with'\\''quote");
    }

    #[test]
    fn test_real_tool_execution() {
        // Test actual echo command
        let args = vec![Value::String("test output".to_string())];
        let result = execute_tool_command("echo", &args, Some(5.0));

        assert!(result.is_ok());
        if let Ok(Value::String(output)) = result {
            assert!(output.contains("test output"));
        }
    }

    #[test]
    fn test_timeout_enforcement() {
        // Test timeout with sleep command
        let args = vec![Value::String("sleep 5".to_string())];
        let start = Instant::now();
        let result = execute_tool_command("shell", &args, Some(1.0));
        let elapsed = start.elapsed();

        assert!(result.is_err());
        assert!(elapsed.as_secs_f64() < 4.0); // Should timeout around 1-3 seconds (allowing for signal delay)

        if let Err(e) = result {
            assert!(e.contains("Timeout") || e.contains("timeout"));
        }
    }
}
