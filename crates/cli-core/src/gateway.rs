// Gateway management - PID file, status, lifecycle, Windows service

use crate::config::Config;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use tracing::info;

// =============================================================================
// PID & State File Management
// =============================================================================

/// Gateway PID file path
pub fn gateway_pid_path() -> PathBuf {
    Config::hermes_home().join("gateway.pid")
}

/// Gateway state file path
pub fn gateway_state_path() -> PathBuf {
    Config::hermes_home().join("gateway_state.json")
}

/// Check if gateway is currently running
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
    let content = content.trim();

    // Try JSON format first (newer versions store {pid, kind, argv, ...})
    if content.starts_with('{') {
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(content) {
            if let Some(pid) = data.get("pid").and_then(|p| p.as_u64()) {
                let pid = pid as u32;
                if is_process_alive(pid) {
                    return Some(pid);
                } else {
                    // Stale PID file - clean it up
                    let _ = fs::remove_file(&pid_path);
                    return None;
                }
            }
        }
        return None;
    }

    // Plain number format
    let pid: u32 = content.parse().ok()?;
    if is_process_alive(pid) {
        Some(pid)
    } else {
        // Stale PID file - clean it up
        let _ = fs::remove_file(&pid_path);
        None
    }
}

/// Check if a process with the given PID is alive
fn is_process_alive(pid: u32) -> bool {
    // On Windows, use tasklist to check if process exists
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid), "/NH"])
            .output()
            .map(|output| {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.contains(&pid.to_string())
            })
            .unwrap_or(false)
    }

    #[cfg(not(target_os = "windows"))]
    {
        // On Unix, send signal 0 to check existence
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
}

/// Write PID file for gateway process with metadata
pub fn write_pid_file() -> Result<()> {
    let pid_path = gateway_pid_path();
    if let Some(parent) = pid_path.parent() {
        fs::create_dir_all(parent).context("failed to create hermes home directory")?;
    }
    let pid = std::process::id();
    let data = serde_json::json!({
        "pid": pid,
        "kind": "hermes-gateway",
        "argv": std::env::args().collect::<Vec<_>>(),
        "start_time": chrono::Utc::now().to_rfc3339(),
    });
    let content = serde_json::to_string_pretty(&data).context("failed to serialize PID data")?;
    fs::write(&pid_path, content).context("failed to write gateway PID file")?;
    info!("wrote gateway PID file with pid={}", pid);
    Ok(())
}

/// Remove PID file
pub fn remove_pid_file() -> Result<()> {
    let pid_path = gateway_pid_path();
    if pid_path.exists() {
        fs::remove_file(&pid_path).context("failed to remove gateway PID file")?;
    }
    Ok(())
}

// =============================================================================
// Gateway State
// =============================================================================

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
        fs::create_dir_all(parent).context("failed to create hermes home directory")?;
    }
    let content =
        serde_json::to_string_pretty(state).context("failed to serialize gateway state")?;
    fs::write(&state_path, content).context("failed to write gateway state file")?;
    Ok(())
}

// =============================================================================
// Platform
// =============================================================================

/// Platform enum for gateway
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Cli,
    Telegram,
    Discord,
    Slack,
    WhatsApp,
    Webhook,
}

impl Platform {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "cli" | "local" => Some(Platform::Cli),
            "telegram" | "tg" => Some(Platform::Telegram),
            "discord" | "dc" => Some(Platform::Discord),
            "slack" | "sl" => Some(Platform::Slack),
            "whatsapp" | "wa" => Some(Platform::WhatsApp),
            "webhook" | "http" => Some(Platform::Webhook),
            _ => None,
        }
    }
}

impl std::str::FromStr for Platform {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| format!("Invalid platform: {}", s))
    }
}

impl Platform {
    pub fn as_str(&self) -> &'static str {
        match self {
            Platform::Cli => "cli",
            Platform::Telegram => "telegram",
            Platform::Discord => "discord",
            Platform::Slack => "slack",
            Platform::WhatsApp => "whatsapp",
            Platform::Webhook => "webhook",
        }
    }

    /// Get all supported platforms
    pub fn all() -> &'static [Platform] {
        &[
            Platform::Cli,
            Platform::Telegram,
            Platform::Discord,
            Platform::Slack,
            Platform::WhatsApp,
            Platform::Webhook,
        ]
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// =============================================================================
// Windows Service Management
// =============================================================================

/// Windows service name for the Hermes gateway
const SERVICE_NAME: &str = "HermesGateway";
const SERVICE_DISPLAY_NAME: &str = "Hermes Agent Gateway";

/// Check if the gateway is installed as a Windows service
pub fn is_service_installed() -> bool {
    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("sc").args(["query", SERVICE_NAME]).output();
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                stdout.contains("RUNNING")
                    || stdout.contains("STOPPED")
                    || stdout.contains("PAUSED")
            }
            Err(_) => false,
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

/// Get the status of the Hermes Windows service
pub fn get_service_status() -> ServiceStatus {
    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("sc").args(["query", SERVICE_NAME]).output();

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                if stdout.contains("RUNNING") {
                    ServiceStatus::Running
                } else if stdout.contains("STOPPED") {
                    ServiceStatus::Stopped
                } else if stdout.contains("PAUSED") {
                    ServiceStatus::Paused
                } else if stdout.contains("START_PENDING") {
                    ServiceStatus::StartPending
                } else if stdout.contains("STOP_PENDING") {
                    ServiceStatus::StopPending
                } else {
                    ServiceStatus::NotFound
                }
            }
            Err(_) => ServiceStatus::NotFound,
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        ServiceStatus::NotApplicable
    }
}

/// Install the gateway as a Windows service using sc.exe
pub fn install_service() -> Result<()> {
    let exe_path = std::env::current_exe().context("failed to get current executable path")?;
    let exe_str = exe_path.to_string_lossy();

    // Create the service bin path with the "gateway run" arguments
    let bin_path = format!("{} gateway run", exe_str);

    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("sc")
            .args([
                "create",
                SERVICE_NAME,
                "binPath=",
                &bin_path,
                "start=",
                "auto",
                "DisplayName=",
                SERVICE_DISPLAY_NAME,
            ])
            .output()
            .context("failed to run sc.exe to create service")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to create service: {}", stderr);
        }

        // Configure failure recovery (restart on failure)
        let _ = std::process::Command::new("sc")
            .args([
                "failure",
                SERVICE_NAME,
                "reset=",
                "86400", // Reset failure count after 24 hours
                "actions=",
                "restart/30000/restart/60000/restart/120000", // Restart with increasing delays
            ])
            .output();

        // Configure service description
        let desc = "Hermes Agent Gateway - Messaging platform integration service";
        let _ = std::process::Command::new("sc").args(["description", SERVICE_NAME, desc]).output();

        info!("installed Hermes gateway service");
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = bin_path; // suppress unused warning
        anyhow::bail!("Windows service installation is only available on Windows");
    }
}

/// Uninstall the gateway Windows service
pub fn uninstall_service() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        // Stop the service first if running
        let _ = stop_service();

        // Delete the service
        let output = std::process::Command::new("sc")
            .args(["delete", SERVICE_NAME])
            .output()
            .context("failed to run sc.exe to delete service")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // If service doesn't exist, that's fine
            if !stderr.contains("does not exist") && !stderr.contains("1060") {
                anyhow::bail!("Failed to delete service: {}", stderr);
            }
        }

        info!("uninstalled Hermes gateway service");
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        anyhow::bail!("Windows service uninstallation is only available on Windows");
    }
}

/// Start the gateway Windows service
pub fn start_service() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("sc")
            .args(["start", SERVICE_NAME])
            .output()
            .context("failed to run sc.exe to start service")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to start service: {}", stderr);
        }
        info!("started Hermes gateway service");
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        anyhow::bail!("Windows service start is only available on Windows");
    }
}

/// Stop the gateway Windows service
pub fn stop_service() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("sc")
            .args(["stop", SERVICE_NAME])
            .output()
            .context("failed to run sc.exe to stop service")?;

        // SC returns error 1062 if service is already stopped - that's OK
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !output.status.success() && !stderr.contains("1062") && !stderr.contains("not started") {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if !stdout.contains("not started") {
                // Don't fail if service is already stopped
            }
        }
        info!("stopped Hermes gateway service");
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        anyhow::bail!("Windows service stop is only available on Windows");
    }
}

/// Service status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceStatus {
    Running,
    Stopped,
    Paused,
    StartPending,
    StopPending,
    NotFound,
    NotApplicable,
}

impl std::fmt::Display for ServiceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceStatus::Running => write!(f, "running"),
            ServiceStatus::Stopped => write!(f, "stopped"),
            ServiceStatus::Paused => write!(f, "paused"),
            ServiceStatus::StartPending => write!(f, "start_pending"),
            ServiceStatus::StopPending => write!(f, "stop_pending"),
            ServiceStatus::NotFound => write!(f, "not_found"),
            ServiceStatus::NotApplicable => write!(f, "not_applicable"),
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_platform_parse() {
        assert_eq!(Platform::parse("cli"), Some(Platform::Cli));
        assert_eq!(Platform::parse("telegram"), Some(Platform::Telegram));
        assert_eq!(Platform::parse("TG"), Some(Platform::Telegram));
        assert_eq!(Platform::parse("discord"), Some(Platform::Discord));
        assert_eq!(Platform::parse("slack"), Some(Platform::Slack));
        assert_eq!(Platform::parse("whatsapp"), Some(Platform::WhatsApp));
        assert_eq!(Platform::parse("webhook"), Some(Platform::Webhook));
        assert_eq!(Platform::parse("unknown"), None);
    }

    #[test]
    fn test_platform_from_str() {
        assert!(Platform::from_str("cli").is_ok());
        assert!(Platform::from_str("unknown").is_err());
    }

    #[test]
    fn test_platform_as_str() {
        assert_eq!(Platform::Telegram.as_str(), "telegram");
        assert_eq!(Platform::Discord.as_str(), "discord");
        assert_eq!(Platform::WhatsApp.as_str(), "whatsapp");
        assert_eq!(Platform::Webhook.as_str(), "webhook");
    }

    #[test]
    fn test_platform_display() {
        assert_eq!(format!("{}", Platform::Cli), "cli");
        assert_eq!(format!("{}", Platform::Telegram), "telegram");
    }

    #[test]
    fn test_platform_all() {
        let platforms = Platform::all();
        assert!(platforms.len() >= 5);
        assert!(platforms.contains(&Platform::Cli));
        assert!(platforms.contains(&Platform::Telegram));
    }

    #[test]
    fn test_gateway_state_default() {
        let state = GatewayState::default();
        assert_eq!(state.gateway_state, "stopped");
        assert_eq!(state.active_agents, 0);
        assert!(!state.restart_requested);
    }

    #[test]
    fn test_gateway_state_serialization() {
        let state = GatewayState::default();
        let json = serde_json::to_string_pretty(&state).unwrap();
        let parsed: GatewayState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.gateway_state, "stopped");
        assert_eq!(parsed.active_agents, 0);
    }

    #[test]
    fn test_service_status_display() {
        assert_eq!(format!("{}", ServiceStatus::Running), "running");
        assert_eq!(format!("{}", ServiceStatus::Stopped), "stopped");
        assert_eq!(format!("{}", ServiceStatus::NotFound), "not_found");
        assert_eq!(format!("{}", ServiceStatus::NotApplicable), "not_applicable");
    }

    #[test]
    fn test_pid_path() {
        let path = gateway_pid_path();
        assert!(path.to_string_lossy().contains("gateway.pid"));
    }

    #[test]
    fn test_state_path() {
        let path = gateway_state_path();
        assert!(path.to_string_lossy().contains("gateway_state.json"));
    }
}
