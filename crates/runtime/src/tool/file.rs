use crate::tool::{Tool, ToolOutput};
use crate::RuntimeError;
use regex::Regex;
use serde_json::{json, Value};
use std::future::Future;
use std::path::Path;
use std::pin::Pin;

pub struct FileReadTool;

impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "file_read"
    }

    fn description(&self) -> &str {
        "Read file content from the local filesystem"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path to read" },
                "offset": { "type": "integer", "description": "Start line (0-based)" },
                "limit": { "type": "integer", "description": "Max lines to read" }
            },
            "required": ["path"]
        })
    }

    fn execute(&self, params: Value) -> Pin<Box<dyn Future<Output = Result<ToolOutput, RuntimeError>> + Send + '_>> {
        Box::pin(async move {
            let path = params["path"]
                .as_str()
                .ok_or_else(|| RuntimeError::InvalidInput("missing 'path'".to_string()))?;
            let content = tokio::fs::read_to_string(path)
                .await
                .map_err(|e| RuntimeError::ToolError {
                    name: "file_read".to_string(),
                    message: format!("Failed to read '{}': {}", path, e),
                })?;

            let lines: Vec<&str> = content.lines().collect();
            let offset = params["offset"].as_u64().unwrap_or(0) as usize;
            let limit = params["limit"].as_u64().map(|l| l as usize);

            let slice = if let Some(limit) = limit {
                &lines[offset..(offset + limit).min(lines.len())]
            } else {
                &lines[offset..]
            };

            Ok(ToolOutput::success(slice.join("\n")))
        })
    }
}

pub struct FileWriteTool;

impl Tool for FileWriteTool {
    fn name(&self) -> &str {
        "file_write"
    }

    fn description(&self) -> &str {
        "Write content to a file on the local filesystem"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path to write" },
                "content": { "type": "string", "description": "Content to write" }
            },
            "required": ["path", "content"]
        })
    }

    fn execute(&self, params: Value) -> Pin<Box<dyn Future<Output = Result<ToolOutput, RuntimeError>> + Send + '_>> {
        Box::pin(async move {
            let path = params["path"]
                .as_str()
                .ok_or_else(|| RuntimeError::InvalidInput("missing 'path'".to_string()))?;
            let content = params["content"]
                .as_str()
                .ok_or_else(|| RuntimeError::InvalidInput("missing 'content'".to_string()))?;

            if let Some(parent) = Path::new(path).parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| RuntimeError::ToolError {
                        name: "file_write".to_string(),
                        message: format!("Failed to create dir: {}", e),
                    })?;
            }

            tokio::fs::write(path, content)
                .await
                .map_err(|e| RuntimeError::ToolError {
                    name: "file_write".to_string(),
                    message: format!("Failed to write '{}': {}", path, e),
                })?;

            Ok(ToolOutput::success(format!("Written {} bytes to {}", content.len(), path)))
        })
    }
}

pub struct FileSearchTool;

impl Tool for FileSearchTool {
    fn name(&self) -> &str {
        "file_search"
    }

    fn description(&self) -> &str {
        "Search file content using a regex pattern"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path to search" },
                "pattern": { "type": "string", "description": "Regex pattern to search for" }
            },
            "required": ["path", "pattern"]
        })
    }

    fn execute(&self, params: Value) -> Pin<Box<dyn Future<Output = Result<ToolOutput, RuntimeError>> + Send + '_>> {
        Box::pin(async move {
            let path = params["path"]
                .as_str()
                .ok_or_else(|| RuntimeError::InvalidInput("missing 'path'".to_string()))?;
            let pattern = params["pattern"]
                .as_str()
                .ok_or_else(|| RuntimeError::InvalidInput("missing 'pattern'".to_string()))?;

            let content = tokio::fs::read_to_string(path)
                .await
                .map_err(|e| RuntimeError::ToolError {
                    name: "file_search".to_string(),
                    message: format!("Failed to read '{}': {}", path, e),
                })?;

            let re = Regex::new(pattern).map_err(|e| RuntimeError::InvalidInput(format!(
                "Invalid regex '{}': {}", pattern, e
            )))?;

            let matches: Vec<String> = content
                .lines()
                .enumerate()
                .filter(|(_, line)| re.is_match(line))
                .map(|(i, line)| format!("{}: {}", i + 1, line))
                .collect();

            if matches.is_empty() {
                Ok(ToolOutput::success("No matches found"))
            } else {
                Ok(ToolOutput::success(matches.join("\n")))
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_round_trip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        let path_str = path.to_str().unwrap();

        let write_tool = FileWriteTool;
        write_tool
            .execute(json!({"path": path_str, "content": "Hello Rust"}))
            .await
            .unwrap();

        let read_tool = FileReadTool;
        let result = read_tool
            .execute(json!({"path": path_str}))
            .await
            .unwrap();
        assert_eq!(result.content, "Hello Rust");
    }

    #[tokio::test]
    async fn test_file_search() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        let path_str = path.to_str().unwrap();

        let write_tool = FileWriteTool;
        write_tool
            .execute(json!({"path": path_str, "content": "needle in haystack\nno match here\nanother needle found"}))
            .await
            .unwrap();

        let search_tool = FileSearchTool;
        let result = search_tool
            .execute(json!({"path": path_str, "pattern": "needle"}))
            .await
            .unwrap();
        assert!(result.content.contains("needle"));
        assert!(!result.is_error);
    }
}