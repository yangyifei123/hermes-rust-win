use crate::tool::{Tool, ToolOutput};
use crate::RuntimeError;
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;

pub struct McpTool;

impl Tool for McpTool {
    fn name(&self) -> &str {
        "mcp"
    }

    fn description(&self) -> &str {
        "MCP (Model Context Protocol) tool integration (stub — not yet implemented)"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "server": { "type": "string", "description": "MCP server name" },
                "tool": { "type": "string", "description": "Tool to call on the MCP server" },
                "arguments": { "type": "object", "description": "Arguments for the tool" }
            },
            "required": ["server", "tool"]
        })
    }

    fn execute(&self, _params: Value) -> Pin<Box<dyn Future<Output = Result<ToolOutput, RuntimeError>> + Send + '_>> {
        Box::pin(async move {
            Ok(ToolOutput::error(
                "MCP protocol not yet implemented. Use terminal tool for external integrations.",
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mcp_stub() {
        let tool = McpTool;
        let result = tool
            .execute(json!({"server": "test", "tool": "test"}))
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.content.contains("not yet implemented"));
    }
}