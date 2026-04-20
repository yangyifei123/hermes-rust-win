use crate::tool::{Tool, ToolOutput};
use crate::RuntimeError;
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;

pub struct BrowserTool;

impl Tool for BrowserTool {
    fn name(&self) -> &str {
        "browser"
    }

    fn description(&self) -> &str {
        "Browser automation tool (stub — not yet implemented)"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "description": "Browser action (navigate, click, type, screenshot)" },
                "url": { "type": "string", "description": "URL for navigate action" },
                "selector": { "type": "string", "description": "CSS selector for click/type actions" }
            },
            "required": ["action"]
        })
    }

    fn execute(&self, _params: Value) -> Pin<Box<dyn Future<Output = Result<ToolOutput, RuntimeError>> + Send + '_>> {
        Box::pin(async move {
            Ok(ToolOutput::error(
                "Browser automation not yet implemented. Use terminal tool with curl for HTTP requests.",
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_browser_stub() {
        let tool = BrowserTool;
        let result = tool
            .execute(json!({"action": "navigate", "url": "https://example.com"}))
            .await
            .unwrap();
        assert!(result.is_error);
        assert!(result.content.contains("not yet implemented"));
    }
}