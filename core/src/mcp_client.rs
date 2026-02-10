// MCP (Model Context Protocol) Client Implementation
// Implements JSON-RPC 2.0 over stdio for communicating with MCP servers
//
// Architecture:
//   Aria program -> eval_call() -> mcp_client -> subprocess (MCP server) -> JSON-RPC response
//   All calls go through Aria's permission system before reaching the MCP server.
#![allow(dead_code)]

use crate::eval::Value;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static REQUEST_ID: AtomicU64 = AtomicU64::new(1);

/// Represents an active connection to an MCP server process.
/// The BufReader is stored persistently to avoid losing buffered data
/// when reading multi-line or partial responses.
pub struct McpConnection {
    pub server_name: String,
    pub process: Child,
    pub reader: BufReader<ChildStdout>,
    pub capabilities: Vec<String>,
    pub initialized: bool,
}

/// MCP JSON-RPC request
#[derive(Debug)]
struct JsonRpcRequest {
    id: u64,
    method: String,
    params: serde_json::Value,
}

impl JsonRpcRequest {
    fn to_json(&self) -> String {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": self.id,
            "method": self.method,
            "params": self.params
        })
        .to_string()
    }
}

/// Spawn an MCP server process and establish a stdio connection.
/// stderr is inherited (passed to parent) to avoid deadlocks from unconsumed piped stderr.
pub fn connect_to_server(server_command: &str) -> Result<McpConnection, String> {
    // Parse the server command using shell-words to handle quoted paths correctly
    // e.g., '"/path with spaces/server" arg1 arg2'
    let parts = shell_words::split(server_command)
        .map_err(|e| format!("[MCP Error] Failed to parse server command '{}': {}", server_command, e))?;
    if parts.is_empty() {
        return Err("[MCP Error] Empty server command".to_string());
    }

    let program = &parts[0];
    let args = &parts[1..];

    let mut process = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit()) // Inherit stderr to prevent deadlocks
        .spawn()
        .map_err(|e| {
            format!(
                "[MCP Error] Failed to start server '{}': {}",
                server_command, e
            )
        })?;

    // Take stdout and wrap in persistent BufReader to avoid losing buffered data
    let stdout = process.stdout.take().ok_or_else(|| {
        "[MCP Error] Failed to capture server stdout".to_string()
    })?;
    let reader = BufReader::new(stdout);

    Ok(McpConnection {
        server_name: server_command.to_string(),
        process,
        reader,
        capabilities: Vec::new(),
        initialized: false,
    })
}

/// Send the MCP initialize handshake
pub fn initialize(conn: &mut McpConnection) -> Result<serde_json::Value, String> {
    let request = JsonRpcRequest {
        id: REQUEST_ID.fetch_add(1, Ordering::SeqCst),
        method: "initialize".to_string(),
        params: serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "aria-lang",
                "version": "1.0.0"
            }
        }),
    };

    let response = send_request(conn, &request)?;
    conn.initialized = true;

    // Extract server capabilities from response
    if let Some(caps) = response.get("capabilities") {
        if let Some(tools) = caps.get("tools") {
            if tools.is_object() {
                conn.capabilities.push("tools".to_string());
            }
        }
        if let Some(resources) = caps.get("resources") {
            if resources.is_object() {
                conn.capabilities.push("resources".to_string());
            }
        }
    }

    // Send initialized notification
    let notification = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    write_message(conn, &notification.to_string())?;

    Ok(response)
}

/// List available tools from the MCP server
pub fn list_tools(conn: &mut McpConnection) -> Result<Vec<McpToolInfo>, String> {
    if !conn.initialized {
        return Err("[MCP Error] Connection not initialized. Call initialize() first.".to_string());
    }

    let request = JsonRpcRequest {
        id: REQUEST_ID.fetch_add(1, Ordering::SeqCst),
        method: "tools/list".to_string(),
        params: serde_json::json!({}),
    };

    let response = send_request(conn, &request)?;

    let tools = response
        .get("tools")
        .and_then(|t| t.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    Some(McpToolInfo {
                        name: t.get("name")?.as_str()?.to_string(),
                        description: t
                            .get("description")
                            .and_then(|d| d.as_str())
                            .unwrap_or("")
                            .to_string(),
                        input_schema: t.get("inputSchema").cloned(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(tools)
}

/// Call a tool on the MCP server
pub fn call_tool(
    conn: &mut McpConnection,
    tool_name: &str,
    arguments: HashMap<String, Value>,
    timeout: Option<f64>,
) -> Result<Value, String> {
    if !conn.initialized {
        return Err("[MCP Error] Connection not initialized".to_string());
    }

    // Convert Aria Values to JSON for the MCP call
    let json_args: serde_json::Map<String, serde_json::Value> = arguments
        .into_iter()
        .map(|(k, v)| (k, value_to_json(&v)))
        .collect();

    let request = JsonRpcRequest {
        id: REQUEST_ID.fetch_add(1, Ordering::SeqCst),
        method: "tools/call".to_string(),
        params: serde_json::json!({
            "name": tool_name,
            "arguments": json_args
        }),
    };

    let response = send_request_with_timeout(conn, &request, timeout)?;

    // Check for error flag FIRST — isError takes priority over content
    if let Some(is_error) = response.get("isError") {
        if is_error.as_bool() == Some(true) {
            let error_text = response
                .get("content")
                .and_then(|c| c.as_array())
                .and_then(|a| a.first())
                .and_then(|i| i.get("text"))
                .and_then(|t| t.as_str())
                .unwrap_or("Unknown MCP error");
            return Err(format!("[MCP Tool Error] {}: {}", tool_name, error_text));
        }
    }

    // Parse MCP tool response content
    if let Some(content) = response.get("content") {
        if let Some(arr) = content.as_array() {
            let mut results = Vec::new();
            for item in arr {
                if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                    results.push(text.to_string());
                }
            }
            if results.len() == 1 {
                return Ok(Value::String(results.into_iter().next().unwrap()));
            }
            return Ok(Value::Array(
                results.into_iter().map(Value::String).collect(),
            ));
        }
    }

    Ok(Value::String(response.to_string()))
}

/// Shut down an MCP connection gracefully
pub fn shutdown(conn: &mut McpConnection) -> Result<(), String> {
    // Best-effort: try to kill the process
    let _ = conn.process.kill();
    let _ = conn.process.wait();
    Ok(())
}

// ============================================================================
// Internal helpers
// ============================================================================

fn send_request(
    conn: &mut McpConnection,
    request: &JsonRpcRequest,
) -> Result<serde_json::Value, String> {
    let message = request.to_json();
    write_message(conn, &message)?;
    read_response(conn, request.id, None)
}

fn send_request_with_timeout(
    conn: &mut McpConnection,
    request: &JsonRpcRequest,
    timeout: Option<f64>,
) -> Result<serde_json::Value, String> {
    let message = request.to_json();
    write_message(conn, &message)?;

    let deadline = timeout.map(|secs| Instant::now() + Duration::from_secs_f64(secs));
    read_response(conn, request.id, deadline)
}

/// Write a JSON-RPC message to the server's stdin using newline-delimited JSON
fn write_message(conn: &mut McpConnection, message: &str) -> Result<(), String> {
    let stdin = conn
        .process
        .stdin
        .as_mut()
        .ok_or("[MCP Error] Server stdin not available")?;

    // MCP uses newline-delimited JSON (one message per line)
    writeln!(stdin, "{}", message).map_err(|e| {
        format!(
            "[MCP Error] Failed to write to server '{}': {}",
            conn.server_name, e
        )
    })?;
    stdin.flush().map_err(|e| {
        format!(
            "[MCP Error] Failed to flush stdin for '{}': {}",
            conn.server_name, e
        )
    })?;

    Ok(())
}

/// Read a JSON-RPC response from the server's stdout using the persistent BufReader.
/// Optionally enforces a deadline for timeout.
fn read_response(
    conn: &mut McpConnection,
    expected_id: u64,
    deadline: Option<Instant>,
) -> Result<serde_json::Value, String> {
    let mut line = String::new();

    // Read lines until we get a JSON-RPC response with matching id
    // Skip notification messages (no id field)
    loop {
        // Check timeout deadline before each read
        if let Some(dl) = deadline {
            if Instant::now() >= dl {
                return Err(format!(
                    "[MCP Error] Timeout waiting for response from server '{}'",
                    conn.server_name
                ));
            }
        }

        line.clear();
        let bytes_read = conn.reader.read_line(&mut line).map_err(|e| {
            format!(
                "[MCP Error] Failed to read from server '{}': {}",
                conn.server_name, e
            )
        })?;

        if bytes_read == 0 {
            return Err(format!(
                "[MCP Error] Server '{}' closed connection unexpectedly",
                conn.server_name
            ));
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Parse JSON
        let parsed: serde_json::Value = serde_json::from_str(trimmed).map_err(|e| {
            format!(
                "[MCP Error] Invalid JSON from server '{}': {} (line: '{}')",
                conn.server_name, e, trimmed
            )
        })?;

        // Check if this is a response (has id field)
        if let Some(id) = parsed.get("id") {
            if id.as_u64() == Some(expected_id) {
                // Check for JSON-RPC error
                if let Some(error) = parsed.get("error") {
                    let code = error.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
                    let message = error
                        .get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("Unknown error");
                    return Err(format!(
                        "[MCP Error] Server '{}' returned error {}: {}",
                        conn.server_name, code, message
                    ));
                }

                // Return the result
                return parsed
                    .get("result")
                    .cloned()
                    .ok_or(format!(
                        "[MCP Error] Response from '{}' missing 'result' field",
                        conn.server_name
                    ));
            }
        }
        // If it's a notification (no id), skip it and keep reading
    }
}

/// Convert an Aria Value to a serde_json Value for MCP transport
fn value_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::Number(n) => {
            serde_json::Number::from_f64(*n)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null)
        }
        Value::Null => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Agent(a) => serde_json::Value::String(format!("[Agent: {}]", a)),
        Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(value_to_json).collect())
        }
    }
}

/// Information about an MCP tool as reported by the server
#[derive(Debug, Clone)]
pub struct McpToolInfo {
    pub name: String,
    pub description: String,
    pub input_schema: Option<serde_json::Value>,
}

// ============================================================================
// Simulated MCP for testing (no real server needed)
// ============================================================================

/// Execute an MCP tool call in simulation mode (when no real server is available).
/// This allows Aria programs to define and test MCP tool flows without running
/// actual MCP server processes.
pub fn execute_mcp_simulated(
    server_name: &str,
    tool_name: &str,
    args: &[Value],
    _timeout: Option<f64>,
) -> Result<Value, String> {
    // In simulation mode, return structured mock responses that demonstrate
    // the MCP tool call went through the correct path
    let arg_strs: Vec<String> = args
        .iter()
        .map(|v| format!("{}", v))
        .collect();

    Ok(Value::String(format!(
        "[MCP:{}] {}({}) -> simulated result",
        server_name,
        tool_name,
        arg_strs.join(", ")
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_rpc_request_format() {
        let req = JsonRpcRequest {
            id: 1,
            method: "tools/call".to_string(),
            params: serde_json::json!({"name": "test", "arguments": {}}),
        };
        let json = req.to_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["id"], 1);
        assert_eq!(parsed["method"], "tools/call");
    }

    #[test]
    fn test_value_to_json_conversions() {
        assert_eq!(
            value_to_json(&Value::String("hello".to_string())),
            serde_json::Value::String("hello".to_string())
        );
        assert_eq!(
            value_to_json(&Value::Number(42.0)),
            serde_json::json!(42.0)
        );
        assert_eq!(value_to_json(&Value::Null), serde_json::Value::Null);
        assert_eq!(
            value_to_json(&Value::Bool(true)),
            serde_json::Value::Bool(true)
        );
    }

    #[test]
    fn test_mcp_simulated_execution() {
        let result = execute_mcp_simulated(
            "github-server",
            "search_code",
            &[Value::String("query".to_string())],
            Some(10.0),
        );
        assert!(result.is_ok());
        let val = result.unwrap();
        if let Value::String(s) = val {
            assert!(s.contains("[MCP:github-server]"));
            assert!(s.contains("search_code"));
            assert!(s.contains("query"));
        } else {
            panic!("Expected String value");
        }
    }

    #[test]
    fn test_mcp_simulated_multiple_args() {
        let result = execute_mcp_simulated(
            "db-server",
            "query",
            &[
                Value::String("SELECT *".to_string()),
                Value::Number(10.0),
            ],
            None,
        );
        assert!(result.is_ok());
        let val = result.unwrap();
        if let Value::String(s) = val {
            assert!(s.contains("[MCP:db-server]"));
            assert!(s.contains("SELECT *"));
        } else {
            panic!("Expected String value");
        }
    }
}
