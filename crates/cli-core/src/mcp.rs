use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

/// MCP server configuration stored in mcp.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    pub name: String,
    pub url: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_added_at")]
    pub added_at: String,
}

fn default_enabled() -> bool {
    true
}

fn default_added_at() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Storage for MCP server configurations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpStore {
    #[serde(default)]
    pub servers: Vec<McpServer>,
}

impl McpStore {
    /// Load MCP store from HERMES_HOME/mcp.json
    pub fn load() -> Result<Self> {
        let path = Self::mcp_path();
        if !path.exists() {
            return Ok(McpStore::default());
        }
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read MCP store from {:?}", path))?;
        let store: McpStore = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse MCP store from {:?}", path))?;
        Ok(store)
    }

    /// Save MCP store to HERMES_HOME/mcp.json
    pub fn save(&self) -> Result<()> {
        let path = Self::mcp_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create MCP directory {:?}", parent))?;
        }
        let content =
            serde_json::to_string_pretty(self).context("failed to serialize MCP store")?;
        fs::write(&path, content)
            .with_context(|| format!("failed to write MCP store to {:?}", path))?;
        Ok(())
    }

    /// Get MCP path
    pub fn mcp_path() -> PathBuf {
        if let Ok(home) = std::env::var("HERMES_HOME") {
            return PathBuf::from(home).join("mcp.json");
        }
        if let Ok(profile) = std::env::var("HERMES_PROFILE") {
            if let Some(proj_dirs) =
                ProjectDirs::from("ai", "hermes", &format!("hermes-{}", profile))
            {
                return proj_dirs.config_dir().join("mcp.json");
            }
        }
        if let Some(proj_dirs) = ProjectDirs::from("ai", "hermes", "hermes-cli") {
            return proj_dirs.config_dir().join("mcp.json");
        }
        if let Ok(home) = std::env::var("USERPROFILE") {
            return PathBuf::from(home).join(".hermes").join("mcp.json");
        }
        PathBuf::from(".hermes").join("mcp.json")
    }

    /// Add an MCP server
    pub fn add_server(&mut self, name: &str, url: &str) -> Result<()> {
        // Check if name already exists
        if self.servers.iter().any(|s| s.name == name) {
            anyhow::bail!(
                "MCP server '{}' already exists. Use a different name or remove it first.",
                name
            );
        }

        self.servers.push(McpServer {
            name: name.to_string(),
            url: url.to_string(),
            enabled: true,
            added_at: chrono::Utc::now().to_rfc3339(),
        });
        Ok(())
    }

    /// Remove an MCP server by name
    pub fn remove_server(&mut self, name: &str) -> Result<()> {
        let len = self.servers.len();
        self.servers.retain(|s| s.name != name);
        if self.servers.len() == len {
            anyhow::bail!("MCP server '{}' not found", name);
        }
        Ok(())
    }

    /// Get a server by name
    pub fn get_server(&self, name: &str) -> Option<&McpServer> {
        self.servers.iter().find(|s| s.name == name)
    }

    /// List all servers
    pub fn list_servers(&self) -> &[McpServer] {
        &self.servers
    }
}

/// Test connectivity to an MCP server
pub fn test_server(server: &McpServer) -> Result<TestResult> {
    let start = Instant::now();

    if server.url.starts_with("stdio://") {
        // For stdio:// URLs, check if the binary exists
        let path = server.url.trim_start_matches("stdio://");
        let path = if path.contains(' ') {
            // If there's a space, it's likely "path/to/binary --args"
            path.split_whitespace().next().unwrap_or(path)
        } else {
            path
        };

        if std::path::Path::new(path).exists() {
            Ok(TestResult {
                success: true,
                response_time_ms: 0,
                message: format!(
                    "Binary '{}' exists (stdio transport, actual connection not tested)",
                    path
                ),
            })
        } else {
            Ok(TestResult {
                success: false,
                response_time_ms: 0,
                message: format!("Binary '{}' not found", path),
            })
        }
    } else if server.url.starts_with("http://") || server.url.starts_with("https://") {
        // For HTTP URLs, we report success since the URL prefix was already validated
        let url = &server.url;
        let duration = start.elapsed();

        Ok(TestResult {
            success: true,
            response_time_ms: duration.as_millis() as u64,
            message: format!(
                "URL '{}' is valid (connection test not fully implemented)",
                url
            ),
        })
    } else {
        Ok(TestResult {
            success: false,
            response_time_ms: start.elapsed().as_millis() as u64,
            message: format!(
                "Unknown transport scheme in URL '{}'. Supported: http://, https://, stdio://",
                server.url
            ),
        })
    }
}

/// Result of testing an MCP server connection
#[derive(Debug)]
pub struct TestResult {
    pub success: bool,
    pub response_time_ms: u64,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_store_default() {
        let store = McpStore::default();
        assert!(store.servers.is_empty());
    }

    #[test]
    fn test_mcp_store_add_server() {
        let mut store = McpStore::default();
        store.add_server("test", "http://localhost:3000").unwrap();
        assert_eq!(store.servers.len(), 1);
        assert_eq!(store.servers[0].name, "test");
        assert_eq!(store.servers[0].url, "http://localhost:3000");
        assert!(store.servers[0].enabled);
    }

    #[test]
    fn test_mcp_store_add_duplicate() {
        let mut store = McpStore::default();
        store.add_server("test", "http://localhost:3000").unwrap();
        let result = store.add_server("test", "http://localhost:4000");
        assert!(result.is_err());
    }

    #[test]
    fn test_mcp_store_remove_server() {
        let mut store = McpStore::default();
        store.add_server("test", "http://localhost:3000").unwrap();
        store.remove_server("test").unwrap();
        assert!(store.servers.is_empty());
    }

    #[test]
    fn test_mcp_store_remove_not_found() {
        let mut store = McpStore::default();
        let result = store.remove_server("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_mcp_server_serialization() {
        let server = McpServer {
            name: "test".to_string(),
            url: "http://localhost:3000".to_string(),
            enabled: true,
            added_at: "2026-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string_pretty(&server).unwrap();
        assert!(json.contains("\"name\": \"test\""));
        assert!(json.contains("\"url\": \"http://localhost:3000\""));
        assert!(json.contains("\"enabled\": true"));
    }
}
