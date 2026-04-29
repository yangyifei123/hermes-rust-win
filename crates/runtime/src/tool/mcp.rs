use crate::tool::{Tool, ToolOutput};
use crate::RuntimeError;
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

/// MCP tool implementation using JSON-RPC over stdio transport.
///
/// Each call spawns the MCP server process, performs the handshake
/// (initialize + tools/list), then calls the requested tool.
pub struct McpTool;

impl Tool for McpTool {
    fn name(&self) -> &str {
        "mcp"
    }

    fn description(&self) -> &str {
        "Call a tool on an MCP (Model Context Protocol) server via stdio transport"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "server": {
                    "type": "string",
                    "description": "MCP server binary name or path to execute"
                },
                "tool": {
                    "type": "string",
                    "description": "Tool name to call on the MCP server"
                },
                "arguments": {
                    "type": "object",
                    "description": "Arguments to pass to the tool"
                },
                "args": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Extra CLI arguments to pass to the MCP server process"
                }
            },
            "required": ["server", "tool"]
        })
    }

    fn execute(
        &self,
        params: Value,
    ) -> Pin<Box<dyn Future<Output = Result<ToolOutput, RuntimeError>> + Send + '_>> {
        Box::pin(async move { mcp_call(params).await })
    }
}

/// Perform a full MCP call: spawn server, handshake, invoke tool.
async fn mcp_call(params: Value) -> Result<ToolOutput, RuntimeError> {
    let server = params["server"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();
    let tool_name = params["tool"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    if server.is_empty() {
        return Ok(ToolOutput::error("Missing required parameter: server"));
    }
    if tool_name.is_empty() {
        return Ok(ToolOutput::error("Missing required parameter: tool"));
    }

    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    // Collect extra CLI args for the server process
    let extra_args: Vec<String> = params
        .get("args")
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    // Spawn the MCP server process
    let mut child = Command::new(&server)
        .args(&extra_args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| RuntimeError::ToolError {
            name: "mcp".into(),
            message: format!("Failed to spawn MCP server '{}': {}", server, e),
        })?;

    let child_stdin = child.stdin.as_mut().unwrap();
    let child_stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(child_stdout);

    // -- Step 1: initialize handshake --
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "hermes",
                "version": "1.0.0"
            }
        }
    });

    send_jsonrpc(child_stdin, &init_request).await?;
    let _init_response = read_jsonrpc(&mut reader, 1).await?;

    // Send initialized notification (no id, no response expected)
    let initialized_notification = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    send_jsonrpc(child_stdin, &initialized_notification).await?;

    // -- Step 2: tools/list (optional, for validation) --
    let list_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });

    send_jsonrpc(child_stdin, &list_request).await?;
    let _list_response = read_jsonrpc(&mut reader, 2).await?;

    // -- Step 3: tools/call --
    let call_request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": tool_name,
            "arguments": arguments
        }
    });

    send_jsonrpc(child_stdin, &call_request).await?;
    let call_response = read_jsonrpc(&mut reader, 3).await?;

    // Close stdin to signal we're done
    let _ = child.stdin.take();

    // Wait for the process to exit (best-effort)
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), child.wait()).await;

    // Parse the result
    match call_response {
        JsonRpcResponse::Result { result, .. } => Ok(format_tool_result(&result)),
        JsonRpcResponse::Error { error, .. } => Ok(ToolOutput::error(format!(
            "MCP server error [{}]: {}",
            error.code,
            error.message
        ))),
    }
}

/// Format a tools/call result into a ToolOutput.
fn format_tool_result(result: &Value) -> ToolOutput {
    // MCP spec: result has "content" array with content items
    // Each item has "type" ("text", "image", "resource") and "text"/"data"
    if let Some(content_arr) = result.get("content").and_then(|c| c.as_array()) {
        let texts: Vec<String> = content_arr
            .iter()
            .filter_map(|item| {
                match item.get("type").and_then(|t| t.as_str()) {
                    Some("text") => item.get("text").and_then(|t| t.as_str()).map(String::from),
                    Some("image") => Some("[image content]".to_string()),
                    Some("resource") => Some(
                        item.get("resource")
                            .and_then(|r| r.get("text"))
                            .and_then(|t| t.as_str())
                            .map(String::from)
                            .unwrap_or_else(|| "[resource]".to_string()),
                    ),
                    _ => None,
                }
            })
            .collect();

        if texts.is_empty() {
            ToolOutput::success(serde_json::to_string_pretty(result).unwrap_or_else(|_| result.to_string()))
        } else {
            let is_error = result
                .get("isError")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let combined = texts.join("\n");
            if is_error {
                ToolOutput::error(combined)
            } else {
                ToolOutput::success(combined)
            }
        }
    } else {
        // Fallback: just pretty-print whatever we got
        ToolOutput::success(serde_json::to_string_pretty(result).unwrap_or_else(|_| result.to_string()))
    }
}

// ── JSON-RPC helpers ─────────────────────────────────────────────────

/// A received JSON-RPC response (either success or error).
#[allow(dead_code)]
enum JsonRpcResponse {
    Result { id: u64, result: Value },
    Error { id: u64, error: JsonRpcError },
}

struct JsonRpcError {
    code: i64,
    message: String,
}

/// Send a JSON-RPC message as a single newline-terminated line.
async fn send_jsonrpc(
    stdin: &mut tokio::process::ChildStdin,
    msg: &Value,
) -> Result<(), RuntimeError> {
    let mut line = serde_json::to_string(msg).map_err(|e| RuntimeError::ToolError {
        name: "mcp".into(),
        message: format!("Failed to serialize JSON-RPC message: {}", e),
    })?;
    line.push('\n');
    stdin
        .write_all(line.as_bytes())
        .await
        .map_err(|e| RuntimeError::ToolError {
            name: "mcp".into(),
            message: format!("Failed to write to MCP server stdin: {}", e),
        })?;
    stdin.flush().await.map_err(|e| RuntimeError::ToolError {
        name: "mcp".into(),
        message: format!("Failed to flush MCP server stdin: {}", e),
    })?;
    Ok(())
}

/// Read a JSON-RPC response line, parse it, and verify the id matches.
async fn read_jsonrpc(
    reader: &mut BufReader<tokio::process::ChildStdout>,
    expected_id: u64,
) -> Result<JsonRpcResponse, RuntimeError> {
    let mut line = String::new();
    // Read with a timeout so we don't hang forever
    let read_result = tokio::time::timeout(std::time::Duration::from_secs(30), reader.read_line(&mut line))
        .await
        .map_err(|_| RuntimeError::ToolError {
            name: "mcp".into(),
            message: format!(
                "Timeout waiting for MCP server response (id={})",
                expected_id
            ),
        })?;

    match read_result {
        Ok(0) => {
            return Err(RuntimeError::ToolError {
                name: "mcp".into(),
                message: "MCP server closed stdout unexpectedly".into(),
            })
        }
        Ok(_) => {}
        Err(e) => {
            return Err(RuntimeError::ToolError {
                name: "mcp".into(),
                message: format!("Failed to read from MCP server stdout: {}", e),
            })
        }
    }

    let msg: Value = serde_json::from_str(line.trim()).map_err(|e| RuntimeError::ToolError {
        name: "mcp".into(),
        message: format!("Invalid JSON-RPC response from MCP server: {}", e),
    })?;

    // Verify id
    let id = msg
        .get("id")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| RuntimeError::ToolError {
            name: "mcp".into(),
            message: format!(
                "JSON-RPC response missing 'id' field: {}",
                line.trim()
            ),
        })?;

    if id != expected_id {
        return Err(RuntimeError::ToolError {
            name: "mcp".into(),
            message: format!(
                "JSON-RPC response id mismatch: expected {}, got {}",
                expected_id, id
            ),
        });
    }

    // Check for error vs result
    if let Some(error) = msg.get("error") {
        Ok(JsonRpcResponse::Error {
            id,
            error: JsonRpcError {
                code: error
                    .get("code")
                    .and_then(|c| c.as_i64())
                    .unwrap_or(-1),
                message: error
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("unknown error")
                    .to_string(),
            },
        })
    } else {
        let result = msg.get("result").cloned().unwrap_or(Value::Null);
        Ok(JsonRpcResponse::Result { id, result })
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mcp_missing_server_param() {
        let tool = McpTool;
        let result = tool
            .execute(json!({"tool": "some_tool"}))
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(
            result.content.contains("Missing required parameter: server"),
            "unexpected: {}",
            result.content
        );
    }

    #[tokio::test]
    async fn test_mcp_missing_tool_param() {
        let tool = McpTool;
        let result = tool
            .execute(json!({"server": "some_server"}))
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(
            result.content.contains("Missing required parameter: tool"),
            "unexpected: {}",
            result.content
        );
    }

    #[tokio::test]
    async fn test_mcp_nonexistent_server() {
        let tool = McpTool;
        let result = tool
            .execute(json!({"server": "definitely_not_a_real_mcp_server_binary_xyz", "tool": "test"}))
            .await;
        // Should fail with a ToolError about spawning
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            RuntimeError::ToolError { name, message } => {
                assert_eq!(name, "mcp");
                assert!(message.contains("Failed to spawn MCP server"));
            }
            other => panic!("Expected ToolError, got: {:?}", other),
        }
    }

    #[test]
    fn test_format_tool_result_text_content() {
        let result = json!({
            "content": [
                { "type": "text", "text": "hello world" }
            ]
        });
        let output = format_tool_result(&result);
        assert!(!output.is_error);
        assert_eq!(output.content, "hello world");
    }

    #[test]
    fn test_format_tool_result_multiple_texts() {
        let result = json!({
            "content": [
                { "type": "text", "text": "line 1" },
                { "type": "text", "text": "line 2" }
            ]
        });
        let output = format_tool_result(&result);
        assert!(!output.is_error);
        assert_eq!(output.content, "line 1\nline 2");
    }

    #[test]
    fn test_format_tool_result_error_flag() {
        let result = json!({
            "isError": true,
            "content": [
                { "type": "text", "text": "something went wrong" }
            ]
        });
        let output = format_tool_result(&result);
        assert!(output.is_error);
        assert_eq!(output.content, "something went wrong");
    }

    #[test]
    fn test_format_tool_result_fallback() {
        let result = json!({
            "someField": "someValue",
            "number": 42
        });
        let output = format_tool_result(&result);
        assert!(!output.is_error);
        assert!(output.content.contains("someField"));
    }
}
