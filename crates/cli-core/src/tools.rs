use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

/// Built-in tools available in Hermes
#[derive(Debug, Clone)]
pub struct Tool {
    pub name: &'static str,
    pub description: &'static str,
    pub toolset: &'static str,
    pub enabled_by_default: bool,
}

/// List of all built-in tools
pub fn get_builtin_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "web_search",
            description: "Search the web for information",
            toolset: "web",
            enabled_by_default: true,
        },
        Tool {
            name: "web_fetch",
            description: "Fetch content from a URL",
            toolset: "web",
            enabled_by_default: true,
        },
        Tool {
            name: "file_read",
            description: "Read files from the filesystem",
            toolset: "terminal",
            enabled_by_default: true,
        },
        Tool {
            name: "file_write",
            description: "Write files to the filesystem",
            toolset: "terminal",
            enabled_by_default: true,
        },
        Tool {
            name: "bash",
            description: "Execute bash commands",
            toolset: "terminal",
            enabled_by_default: false,
        },
        Tool {
            name: "powershell",
            description: "Execute PowerShell commands",
            toolset: "terminal",
            enabled_by_default: false,
        },
        Tool {
            name: "browser",
            description: "Control a web browser",
            toolset: "browser",
            enabled_by_default: false,
        },
        Tool {
            name: "github",
            description: "Interact with GitHub API",
            toolset: "github",
            enabled_by_default: false,
        },
        Tool {
            name: "jira",
            description: "Interact with Jira",
            toolset: "jira",
            enabled_by_default: false,
        },
        Tool {
            name: "database",
            description: "Execute database queries",
            toolset: "database",
            enabled_by_default: false,
        },
        Tool {
            name: "mcp",
            description: "Use MCP (Model Context Protocol) tools",
            toolset: "mcp",
            enabled_by_default: false,
        },
    ]
}

/// Tools configuration storage
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolsConfig {
    #[serde(default)]
    pub disabled: HashSet<String>,
}

impl ToolsConfig {
    /// Load tools config
    pub fn load() -> Result<Self> {
        let path = Self::tools_config_path();
        if !path.exists() {
            return Ok(ToolsConfig::default());
        }
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read tools config from {:?}", path))?;
        let config: ToolsConfig = serde_yaml::from_str(&content)
            .with_context(|| format!("failed to parse tools config from {:?}", path))?;
        Ok(config)
    }

    /// Save tools config
    pub fn save(&self) -> Result<()> {
        let path = Self::tools_config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create tools config directory {:?}", parent))?;
        }
        let content = serde_yaml::to_string(self)
            .context("failed to serialize tools config")?;
        fs::write(&path, content)
            .with_context(|| format!("failed to write tools config to {:?}", path))?;
        Ok(())
    }

    /// Get tools config path
    fn tools_config_path() -> PathBuf {
        if let Ok(home) = std::env::var("HERMES_HOME") {
            return PathBuf::from(home).join("tools.yaml");
        }
        if let Ok(profile) = std::env::var("HERMES_PROFILE") {
            if let Some(proj_dirs) = ProjectDirs::from("ai", "hermes", &format!("hermes-{}", profile)) {
                return proj_dirs.config_dir().join("tools.yaml");
            }
        }
        if let Some(proj_dirs) = ProjectDirs::from("ai", "hermes", "hermes-cli") {
            return proj_dirs.config_dir().join("tools.yaml");
        }
        if let Ok(home) = std::env::var("USERPROFILE") {
            return PathBuf::from(home).join(".hermes").join("tools.yaml");
        }
        PathBuf::from(".hermes").join("tools.yaml")
    }

    /// Check if a tool is disabled
    pub fn is_disabled(&self, tool_name: &str) -> bool {
        self.disabled.contains(tool_name)
    }

    /// Disable a tool
    pub fn disable(&mut self, tool_name: &str) -> bool {
        self.disabled.insert(tool_name.to_string())
    }

    /// Enable a tool
    pub fn enable(&mut self, tool_name: &str) -> bool {
        self.disabled.remove(tool_name)
    }
}

/// List all tools with their status
pub fn list_tools(all: bool) -> Result<Vec<(String, String, String, bool)>> {
    let config = ToolsConfig::load()
        .map_err(|e| anyhow::anyhow!("failed to load tools config: {}", e))?;
    let tools = get_builtin_tools();

    Ok(tools
        .into_iter()
        .filter(|tool| all || !config.is_disabled(tool.name))
        .map(|tool| {
            let enabled = !config.is_disabled(tool.name);
            (tool.name.to_string(), tool.description.to_string(), tool.toolset.to_string(), enabled)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tools_config_disable_enable() {
        let mut config = ToolsConfig::default();
        assert!(!config.is_disabled("web_search"));
        config.disable("web_search");
        assert!(config.is_disabled("web_search"));
        config.enable("web_search");
        assert!(!config.is_disabled("web_search"));
    }

    #[test]
    fn test_get_builtin_tools() {
        let tools = get_builtin_tools();
        assert!(!tools.is_empty());
        assert!(tools.iter().any(|t| t.name == "web_search"));
        assert!(tools.iter().any(|t| t.name == "file_read"));
    }
}