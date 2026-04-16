// Gateway management - PID file, status, lifecycle

use crate::config::Config;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

/// Gateway PID file path
pub fn gateway_pid_path() -> PathBuf {
    Config::hermes_home().join("gateway.pid")
}

/// Gateway state file path
pub fn gateway_state_path() -> PathBuf {
    Config::hermes_home().join("gateway_state.json")
}

/// Check if gateway is currently running by checking PID file
pub fn is_gateway_running() -> bool {
    get_running_pid().is_some()
}

/// Get the PID of running gateway instance, if any
pub fn get_running_pid() -> Option<u32> {
    let pid_path = gateway_pid_path();
    if !pid_path.exists() {
        return None;
    }

    let content = fs::read_to_string(&pid_path).ok()?;
    let pid: u32 = content.trim().parse().ok()?;
    
    // On Windows, we can't easily signal a process to check if it's alive
    // So we just check if the file exists and contains a PID
    Some(pid)
}

/// Write PID file for gateway process
pub fn write_pid_file() -> Result<()> {
    let pid_path = gateway_pid_path();
    if let Some(parent) = pid_path.parent() {
        fs::create_dir_all(parent)
            .context("failed to create hermes home directory")?;
    }
    let pid = std::process::id();
    fs::write(&pid_path, pid.to_string())
        .context("failed to write gateway PID file")?;
    Ok(())
}

/// Remove PID file
pub fn remove_pid_file() -> Result<()> {
    let pid_path = gateway_pid_path();
    if pid_path.exists() {
        fs::remove_file(&pid_path)
            .context("failed to remove gateway PID file")?;
    }
    Ok(())
}

/// Gateway state structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GatewayState {
    pub gateway_state: String,
    pub pid: u32,
    pub platform: Option<String>,
    pub platform_state: Option<String>,
    pub restart_requested: bool,
    pub active_agents: u32,
    pub updated_at: String,
}

impl Default for GatewayState {
    fn default() -> Self {
        Self {
            gateway_state: "stopped".to_string(),
            pid: std::process::id(),
            platform: None,
            platform_state: None,
            restart_requested: false,
            active_agents: 0,
            updated_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Read gateway state from file
pub fn read_gateway_state() -> Option<GatewayState> {
    let state_path = gateway_state_path();
    if !state_path.exists() {
        return None;
    }
    let content = fs::read_to_string(&state_path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Write gateway state to file
pub fn write_gateway_state(state: &GatewayState) -> Result<()> {
    let state_path = gateway_state_path();
    if let Some(parent) = state_path.parent() {
        fs::create_dir_all(parent)
            .context("failed to create hermes home directory")?;
    }
    let content = serde_json::to_string_pretty(state)
        .context("failed to serialize gateway state")?;
    fs::write(&state_path, content)
        .context("failed to write gateway state file")?;
    Ok(())
}

/// Platform enum for gateway
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
   Cli,
    Telegram,
    Discord,
    Slack,
}

impl Platform {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "cli" | "local" => Some(Platform::Cli),
            "telegram" | "tg" => Some(Platform::Telegram),
            "discord" | "dc" => Some(Platform::Discord),
            "slack" | "sl" => Some(Platform::Slack),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Platform::Cli => "cli",
            Platform::Telegram => "telegram",
            Platform::Discord => "discord",
            Platform::Slack => "slack",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_from_str() {
        assert_eq!(Platform::from_str("cli"), Some(Platform::Cli));
        assert_eq!(Platform::from_str("telegram"), Some(Platform::Telegram));
        assert_eq!(Platform::from_str("TG"), Some(Platform::Telegram));
        assert_eq!(Platform::from_str("unknown"), None);
    }

    #[test]
    fn test_platform_as_str() {
        assert_eq!(Platform::Telegram.as_str(), "telegram");
    }
}