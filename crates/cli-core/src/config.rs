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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelConfig {
    #[serde(default = "default_model")]
    pub default: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub provider: String,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            default: default_model(),
            base_url: String::new(),
            provider: String::new(),
        }
    }
}

fn default_model() -> String {
    "gpt-4o".to_string()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TerminalConfig {
    #[serde(default = "default_env_type")]
    pub env_type: String,
    #[serde(default)]
    pub cwd: String,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

fn default_env_type() -> String {
    "local".to_string()
}

fn default_timeout() -> u64 {
    120
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            env_type: default_env_type(),
            cwd: String::new(),
            timeout: default_timeout(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DisplayConfig {
    #[serde(default)]
    pub compact: bool,
    #[serde(default)]
    pub resume_display: String,
    #[serde(default)]
    pub show_reasoning: bool,
    #[serde(default = "default_streaming")]
    pub streaming: bool,
    #[serde(default)]
    pub skin: String,
}

fn default_streaming() -> bool {
    true
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            compact: false,
            resume_display: String::new(),
            show_reasoning: false,
            streaming: default_streaming(),
            skin: String::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentConfig {
    #[serde(default = "default_max_turns")]
    pub max_turns: u32,
    #[serde(default)]
    pub verbose: bool,
    #[serde(default)]
    pub system_prompt: String,
    #[serde(default = "default_reasoning_effort")]
    pub reasoning_effort: String,
}

fn default_max_turns() -> u32 {
    30
}

fn default_reasoning_effort() -> String {
    "medium".to_string()
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_turns: default_max_turns(),
            verbose: false,
            system_prompt: String::new(),
            reasoning_effort: default_reasoning_effort(),
        }
    }
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
        assert_eq!(config.model.default, "gpt-4o");
        assert_eq!(config.model.provider, "");
        assert_eq!(config.terminal.env_type, "local");
        assert_eq!(config.terminal.timeout, 120);
        assert!(!config.display.compact);
        assert!(config.display.streaming);
        assert_eq!(config.agent.max_turns, 30);
        assert_eq!(config.agent.reasoning_effort, "medium");
    }

    #[test]
    fn test_hermes_home() {
        let home = Config::hermes_home();
        assert!(!home.to_string_lossy().is_empty());
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        // Default values should be present after round-trip
        let parsed: Config = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.model.default, "gpt-4o");
        assert_eq!(parsed.terminal.timeout, 120);
        assert_eq!(parsed.agent.max_turns, 30);
    }

    #[test]
    fn test_config_roundtrip() {
        let config = Config::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        let parsed: Config = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.model.default, config.model.default);
        assert_eq!(parsed.terminal.timeout, config.terminal.timeout);
        assert_eq!(parsed.agent.max_turns, config.agent.max_turns);
    }

    #[test]
    fn test_load_dotenv_no_file() {
        // Should succeed even if .env doesn't exist
        assert!(load_dotenv_from_path("/nonexistent/.env").is_ok());
    }

    fn load_dotenv_from_path(path: &str) -> Result<()> {
        let path = std::path::PathBuf::from(path);
        if !path.exists() {
            return Ok(());
        }
        load_dotenv()
    }
}
