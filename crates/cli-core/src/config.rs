use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tracing::info;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub model: ModelConfig,
    #[serde(default)]
    pub terminal: TerminalConfig,
    #[serde(default)]
    pub display: DisplayConfig,
    #[serde(default)]
    pub agent: AgentConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ModelConfig {
    #[serde(default)]
    pub default: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub provider: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct TerminalConfig {
    #[serde(default)]
    pub env_type: String,
    #[serde(default)]
    pub cwd: String,
    #[serde(default)]
    pub timeout: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct DisplayConfig {
    #[serde(default)]
    pub compact: bool,
    #[serde(default)]
    pub resume_display: String,
    #[serde(default)]
    pub show_reasoning: bool,
    #[serde(default)]
    pub streaming: bool,
    #[serde(default)]
    pub skin: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AgentConfig {
    #[serde(default)]
    pub max_turns: u32,
    #[serde(default)]
    pub verbose: bool,
    #[serde(default)]
    pub system_prompt: String,
    #[serde(default)]
    pub reasoning_effort: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model: ModelConfig::default(),
            terminal: TerminalConfig::default(),
            display: DisplayConfig::default(),
            agent: AgentConfig::default(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();
        if !config_path.exists() {
            info!("config not found at {:?}, using defaults", config_path);
            return Ok(Config::default());
        }
        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("failed to read config from {:?}", config_path))?;
        let config: Config = serde_yaml::from_str(&content)
            .with_context(|| format!("failed to parse config from {:?}", config_path))?;
        info!("loaded config from {:?}", config_path);
        Ok(config)
    }

    pub fn config_path() -> PathBuf {
        Self::hermes_home().join("config.yaml")
    }

    pub fn hermes_home() -> PathBuf {
        if let Ok(home) = std::env::var("HERMES_HOME") {
            return PathBuf::from(home);
        }
        if let Ok(profile) = std::env::var("HERMES_PROFILE") {
            if let Some(proj_dirs) = ProjectDirs::from("ai", "hermes", &format!("hermes-{}", profile)) {
                return proj_dirs.config_dir().to_path_buf();
            }
        }
        if let Some(proj_dirs) = ProjectDirs::from("ai", "hermes", "hermes-cli") {
            return proj_dirs.config_dir().to_path_buf();
        }
        if let Ok(home) = std::env::var("USERPROFILE") {
            return PathBuf::from(home).join(".hermes");
        }
        PathBuf::from(".hermes")
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path();
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create config directory {:?}", parent))?;
        }
        let content = serde_yaml::to_string(self)
            .context("failed to serialize config")?;
        fs::write(&config_path, content)
            .with_context(|| format!("failed to write config to {:?}", config_path))?;
        info!("saved config to {:?}", config_path);
        Ok(())
    }
}

pub fn load_dotenv() -> Result<()> {
    let hermes_home = Config::hermes_home();
    let dotenv_path = hermes_home.join(".env");
    if dotenv_path.exists() {
        info!("loading .env from {:?}", dotenv_path);
        let content = fs::read_to_string(&dotenv_path)?;
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(pos) = line.find('=') {
                let key = line[..pos].trim();
                let value = line[pos + 1..].trim();
                if !key.is_empty() {
                    std::env::set_var(key, value);
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.model.provider, "");
        assert_eq!(config.terminal.env_type, "");
        assert_eq!(config.agent.max_turns, 0);
    }

    #[test]
    fn test_hermes_home() {
        let home = Config::hermes_home();
        assert!(!home.to_string_lossy().is_empty());
    }
}
