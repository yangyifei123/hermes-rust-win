use serde_json::Value;
use crate::RuntimeError;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Clone)]
pub struct ToolOutput {
    pub content: String,
    pub is_error: bool,
}

impl ToolOutput {
    pub fn success(content: impl Into<String>) -> Self {
        Self { content: content.into(), is_error: false }
    }

    pub fn error(content: impl Into<String>) -> Self {
        Self { content: content.into(), is_error: true }
    }
}

/// Tool trait using native async fn pattern.
/// Returns a pinned boxed future to allow `dyn Tool` usage.
/// This avoids the async-trait crate while supporting trait objects.
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;
    fn execute(&self, params: Value) -> Pin<Box<dyn Future<Output = Result<ToolOutput, RuntimeError>> + Send + '_>>;
}

pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: HashMap::new() }
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        let name = tool.name().to_string();
        self.tools.insert(name, tool);
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    pub fn list(&self) -> Vec<(&str, &str)> {
        self.tools.values().map(|t| (t.name(), t.description())).collect()
    }

    pub fn tool_definitions(&self) -> Vec<Value> {
        self.tools.values().map(|t| {
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": t.name(),
                    "description": t.description(),
                    "parameters": t.parameters_schema()
                }
            })
        }).collect()
    }

    pub async fn dispatch(&self, name: &str, params: Value) -> Result<ToolOutput, RuntimeError> {
        match self.get(name) {
            Some(tool) => tool.execute(params).await,
            None => Err(RuntimeError::NotFound(format!("tool '{}' not found", name))),
        }
    }
}

impl Default for ToolRegistry {
    fn default() -> Self { Self::new() }
}

pub mod terminal;
pub mod file;
pub mod web;
pub mod mcp;
pub mod browser;
