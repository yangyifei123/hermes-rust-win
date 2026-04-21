use crate::tool::{Tool, ToolOutput};
use crate::RuntimeError;
use regex::Regex;
use serde_json::{json, Value};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

/// Validate that a path is within CWD (security sandbox).
fn validate_path(requested: &str) -> Result<PathBuf, RuntimeError> {
    let requested_path = PathBuf::from(requested);
    let cwd = std::env::current_dir().map_err(|e| RuntimeError::ToolError {
        name: "file".to_string(),
        message: format!("Cannot get CWD: {}", e),
    })?;

    let full_path = if requested_path.is_absolute() {
        requested_path
    } else {
        cwd.join(&requested_path)
    };

    let canonical_cwd = cwd.canonicalize().map_err(|e| RuntimeError::ToolError {
        name: "file".to_string(),
        message: format!("Cannot resolve CWD: {}", e),
    })?;

    let canonical_path = if full_path.exists() {
        full_path.canonicalize().map_err(|e| RuntimeError::ToolError {
            name: "file".to_string(),
            message: format!("Cannot resolve path '{}': {}", requested, e),
        })?
    } else if let Some(parent) = full_path.parent() {
        if parent.as_os_str().is_empty() {
            canonical_cwd.clone()
        } else {
            parent.canonicalize().map_err(|e| RuntimeError::ToolError {
                name: "file".to_string(),
                message: format!("Parent dir '{}' not accessible: {}", parent.display(), e),
            })?
        }
    } else {
        canonical_cwd.clone()
    };

    if !canonical_path.starts_with(&canonical_cwd) {
        return Err(RuntimeError::ToolError {
            name: "file".to_string(),
            message: format!("Access denied: '{}' is outside working directory", requested),
        });
    }

    Ok(full_path)
}

// ===== FileReadTool =====

pub struct FileReadTool;

impl Tool for FileReadTool {
    fn name(&self) -> &str { "file_read" }
    fn description(&self) -> &str { "Read file content from the local filesystem" }

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
            let raw = params["path"].as_str()
                .ok_or_else(|| RuntimeError::InvalidInput("missing 'path'".to_string()))?;
            let validated = validate_path(raw)?;
            let display = validated.display().to_string();
            let content = tokio::fs::read_to_string(&validated).await
                .map_err(|e| RuntimeError::ToolError {
                    name: "file_read".to_string(),
                    message: format!("Failed to read '{}': {}", display, e),
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

// ===== FileWriteTool =====

pub struct FileWriteTool;

impl Tool for FileWriteTool {
    fn name(&self) -> &str { "file_write" }
    fn description(&self) -> &str { "Write content to a file on the local filesystem" }

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
            let raw = params["path"].as_str()
                .ok_or_else(|| RuntimeError::InvalidInput("missing 'path'".to_string()))?;
            let content = params["content"].as_str()
                .ok_or_else(|| RuntimeError::InvalidInput("missing 'content'".to_string()))?;
            let validated = validate_path(raw)?;
            let display = validated.display().to_string();

            if let Some(parent) = validated.parent() {
                tokio::fs::create_dir_all(parent).await
                    .map_err(|e| RuntimeError::ToolError {
                        name: "file_write".to_string(),
                        message: format!("Failed to create dir: {}", e),
                    })?;
            }

            tokio::fs::write(&validated, content).await
                .map_err(|e| RuntimeError::ToolError {
                    name: "file_write".to_string(),
                    message: format!("Failed to write '{}': {}", display, e),
                })?;

            Ok(ToolOutput::success(format!("Written {} bytes to {}", content.len(), display)))
        })
    }
}

// ===== FileSearchTool =====

pub struct FileSearchTool;

impl Tool for FileSearchTool {
    fn name(&self) -> &str { "file_search" }
    fn description(&self) -> &str { "Search file content using a regex pattern" }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path to search" },
                "pattern": { "type": "string", "description": "Regex pattern" }
            },
            "required": ["path", "pattern"]
        })
    }

    fn execute(&self, params: Value) -> Pin<Box<dyn Future<Output = Result<ToolOutput, RuntimeError>> + Send + '_>> {
        Box::pin(async move {
            let raw = params["path"].as_str()
                .ok_or_else(|| RuntimeError::InvalidInput("missing 'path'".to_string()))?;
            let pattern = params["pattern"].as_str()
                .ok_or_else(|| RuntimeError::InvalidInput("missing 'pattern'".to_string()))?;
            let validated = validate_path(raw)?;
            let display = validated.display().to_string();

            let content = tokio::fs::read_to_string(&validated).await
                .map_err(|e| RuntimeError::ToolError {
                    name: "file_search".to_string(),
                    message: format!("Failed to read '{}': {}", display, e),
                })?;

            let re = Regex::new(pattern).map_err(|e| RuntimeError::InvalidInput(
                format!("Invalid regex '{}': {}", pattern, e)
            ))?;

            let matches: Vec<String> = content.lines()
                .enumerate()
                .filter(|(_, line)| re.is_match(line))
                .map(|(i, line)| format!("{}: {}", i + 1, line))
                .collect();

            Ok(ToolOutput::success(
                if matches.is_empty() { "No matches found".to_string() } else { matches.join("\n") }
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn make_temp_file(name: &str, content: &str) -> String {
        let cwd = env::current_dir().unwrap();
        let dir = cwd.join("test_temp_files");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join(name);
        std::fs::write(&path, content).unwrap();
        path.to_string_lossy().to_string()
    }

    fn cleanup_temp() {
        let cwd = env::current_dir().unwrap();
        let dir = cwd.join("test_temp_files");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn test_file_round_trip() {
        let path = make_temp_file("test1.txt", "Hello Rust");
        let result = FileReadTool.execute(json!({"path": path})).await.unwrap();
        assert_eq!(result.content, "Hello Rust");
        cleanup_temp();
    }

    #[tokio::test]
    async fn test_file_search() {
        let path = make_temp_file("test2.txt", "needle in haystack\nno match\nanother needle");
        let result = FileSearchTool.execute(json!({"path": path, "pattern": "needle"})).await.unwrap();
        assert!(result.content.contains("needle"));
        cleanup_temp();
    }

    #[tokio::test]
    async fn test_file_path_traversal_blocked() {
        let result = FileReadTool.execute(json!({"path": "../../../etc/passwd"})).await;
        assert!(result.is_err());
    }
}
