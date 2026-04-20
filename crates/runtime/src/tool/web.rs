use crate::tool::{Tool, ToolOutput};
use crate::RuntimeError;
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;

pub struct WebSearchTool;

impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web for information (stub — not yet implemented)"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Search query" }
            },
            "required": ["query"]
        })
    }

    fn execute(&self, _params: Value) -> Pin<Box<dyn Future<Output = Result<ToolOutput, RuntimeError>> + Send + '_>> {
        Box::pin(async move {
            Ok(ToolOutput::error(
                "Web search not yet implemented. Please use terminal tool with curl.",
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_web_search_stub() {
        let tool = WebSearchTool;
        let result = tool.execute(json!({"query": "test"})).await.unwrap();
        assert!(result.is_error);
        assert!(result.content.contains("not yet implemented"));
    }
}