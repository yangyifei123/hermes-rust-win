use crate::tool::{Tool, ToolOutput};
use crate::RuntimeError;
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;
use std::process::Stdio;
use tokio::process::Command;

pub struct TerminalTool;

impl TerminalTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TerminalTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for TerminalTool {
    fn name(&self) -> &str {
        "terminal"
    }

    fn description(&self) -> &str {
        "Execute terminal commands on the local machine using PowerShell"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The command to execute"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in seconds (default: 120)",
                    "default": 120
                }
            },
            "required": ["command"]
        })
    }

    fn execute(
        &self,
        params: Value,
    ) -> Pin<Box<dyn Future<Output = Result<ToolOutput, RuntimeError>> + Send + '_>> {
        Box::pin(async move {
            let command = params["command"].as_str().ok_or_else(|| {
                RuntimeError::InvalidInput("missing 'command' parameter".to_string())
            })?;

            let timeout_secs = params["timeout"].as_u64().unwrap_or(120);

            let result = tokio::time::timeout(
                std::time::Duration::from_secs(timeout_secs),
                Command::new("powershell")
                    .args(["-NoProfile", "-NonInteractive", "-Command", command])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output(),
            )
            .await
            .map_err(|_| RuntimeError::TimeoutError { duration_secs: timeout_secs })?
            .map_err(|e| RuntimeError::ToolError {
                name: "terminal".to_string(),
                message: e.to_string(),
            })?;

            let stdout = String::from_utf8_lossy(&result.stdout);
            let stderr = String::from_utf8_lossy(&result.stderr);
            let output = if stderr.is_empty() {
                stdout.to_string()
            } else {
                format!("{}\n{}", stdout, stderr)
            };

            if result.status.success() {
                Ok(ToolOutput::success(output))
            } else {
                Ok(ToolOutput::error(format!(
                    "Exit code {}: {}",
                    result.status.code().unwrap_or(-1),
                    output
                )))
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_terminal_echo() {
        let tool = TerminalTool::new();
        let result = tool.execute(json!({"command": "Write-Output HELLO_TOOL"})).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.contains("HELLO_TOOL"));
    }

    #[tokio::test]
    async fn test_terminal_timeout() {
        let tool = TerminalTool::new();
        let result =
            tool.execute(json!({"command": "Start-Sleep -Seconds 30", "timeout": 2})).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            RuntimeError::TimeoutError { .. } => {}
            e => panic!("Expected TimeoutError, got: {:?}", e),
        }
    }
}
