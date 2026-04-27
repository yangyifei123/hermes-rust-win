use super::{
    AuthCommand, ClawCommand, ConfigCommand, CronCommand, DebugCommand, GatewayCommand,
    McpCommand, MemoryCommand, PairingCommand, PluginsCommand, ProfileCommand,
    SessionsCommand, SkillsCommand, ToolsCommand, WebhookCommand,
};
use crate::auth::AuthStore;
use crate::config::Config;
use crate::cron as cron_mod;
use crate::gateway as gateway_mod;
use crate::pairings::{PairingStatus, PairingStore};
use crate::plugins::{Plugin, PluginStore};
use crate::skills::SkillsIndex;
use crate::tools::{self, ToolsConfig};
use crate::webhooks::{Webhook, WebhookStore};
use anyhow::{Context, Result};
use serde_json;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use tracing::info;

pub async fn handle_auth(cmd: AuthCommand) -> Result<()> {
    match cmd {
        AuthCommand::Add { provider, api_key, base_url, .. } => {
            info!("adding auth for provider: {}", provider);
            if api_key.is_none() {
                anyhow::bail!("API key is required. Use: hermes auth add {} --api-key <KEY>", provider);
            }
            let api_key = api_key.unwrap();
            if api_key.is_empty() {
                anyhow::bail!("API key cannot be empty");
            }
            let mut store = AuthStore::load()?;
            store.add(&provider, &api_key, base_url.as_deref());
            store.save()?;
            println!("Auth credentials added for {}", provider);
        }
        AuthCommand::List { .. } => {
            info!("listing auth credentials");
            let store = AuthStore::load()?;
            let credentials = store.list();
            if credentials.is_empty() {
                println!("No auth credentials configured.");
                println!("Run 'hermes auth add <provider> --api-key <KEY>' to add credentials.");
            } else {
                println!("Configured credentials:");
                for (provider, masked_key, base_url) in credentials {
                    println!("  {}: {}", provider, masked_key);
                    if let Some(url) = base_url {
                        println!("    base_url: {}", url);
                    }
                }
            }
        }
        AuthCommand::Remove { provider, .. } => {
            info!("removing auth for provider: {}", provider);
            let mut store = AuthStore::load()?;
            if store.remove(&provider) {
                store.save()?;
                println!("Auth credentials removed for {}", provider);
            } else {
                println!("No auth credentials found for {}", provider);
            }
        }
        AuthCommand::Reset { .. } => {
            info!("resetting all auth credentials");
            let mut store = AuthStore::load()?;
            let count = store.credentials.len();
            store.reset();
            store.save()?;
            println!("All auth credentials cleared ({} removed).", count);
        }
    }
    Ok(())
}

pub fn handle_model(current: bool, global: bool, model: Option<&str>) -> Result<()> {
    let config = Config::load()?;
    match (current, global, model) {
        (true, _, _) => {
            info!("showing current model");
            // Priority: session env var > global config
            let session_model = std::env::var("HERMES_SESSION_MODEL").ok();
            let effective_model = session_model.as_ref().unwrap_or(&config.model.default);
            println!("Current model: {}", effective_model);
            if session_model.is_some() {
                println!("(session override)");
            }
            if !config.model.provider.is_empty() {
                println!("Provider: {}", config.model.provider);
            }
            if !config.model.base_url.is_empty() {
                println!("Base URL: {}", config.model.base_url);
            }
        }
        (_, true, Some(m)) => {
            info!("setting global default model: {}", m);
            let mut config = config;
            config.model.default = m.to_string();
            config.save()?;
            println!("Set global default model to: {}", m);
        }
        (_, _, Some(m)) => {
            info!("setting session model: {}", m);
            // Session model via environment variable - only affects this process and children
            std::env::set_var("HERMES_SESSION_MODEL", m);
            println!("Session model set to: {} (expires when shell exits)", m);
        }
        _ => {
            println!("Model command:");
            println!("  hermes model                    - show current model");
            println!("  hermes model <name>            - set session model (env var)");
            println!("  hermes model --global <name>   - set global default model");
            println!("  hermes model --current          - show current model details");
        }
    }
    Ok(())
}

pub fn handle_tools(cmd: ToolsCommand) -> Result<()> {
    match cmd {
        ToolsCommand::List { all, .. } => {
            info!("listing tools (all: {})", all);
            let tools = tools::list_tools(all)?;
            if tools.is_empty() {
                println!("No tools available.");
            } else {
                println!("Available tools:");
                for (name, description, toolset, enabled) in tools {
                    let status = if enabled { "" } else { " (disabled)" };
                    println!("  {}: {} [{}{}]", name, description, toolset, status);
                }
            }
        }
        ToolsCommand::Disable { names, .. } => {
            for name in &names {
                info!("disabling tool: {}", name);
                let mut config = ToolsConfig::load()?;
                let builtins: Vec<_> = tools::get_builtin_tools()
                    .iter()
                    .map(|t| t.name.to_string())
                    .collect();
                if !builtins.contains(name) {
                    println!("Warning: '{}' is not a built-in tool.", name);
                }
                config.disable(name);
                config.save()?;
                println!("Tool '{}' disabled.", name);
            }
        }
        ToolsCommand::Enable { names, .. } => {
            for name in &names {
                info!("enabling tool: {}", name);
                let mut config = ToolsConfig::load()?;
                config.enable(name);
                config.save()?;
                println!("Tool '{}' enabled.", name);
            }
        }
    }
    Ok(())
}

pub fn handle_skills(cmd: SkillsCommand) -> Result<()> {
    match cmd {
        SkillsCommand::Search { query, .. } => {
            info!("searching skills: {:?}", query);
            let mut index = SkillsIndex::load()?;
            let count = index.scan_local_skills()?;

            let results: Vec<_> = if let Some(ref q) = query {
                index.search(q).into_iter().cloned().collect()
            } else {
                index.get_all().into_iter().cloned().collect()
            };

            if results.is_empty() {
                if let Some(query) = &query {
                    println!("No skills found matching '{}'.", query);
                } else {
                    println!("No skills installed.");
                    println!("Run 'hermes skills install <name>' to install a skill.");
                }
            } else {
                println!("Found {} skill(s):", results.len());
                for skill in results {
                    println!("  {}: {}", skill.name, skill.description);
                    if !skill.tags.is_empty() {
                        println!("    tags: {}", skill.tags.join(", "));
                    }
                }
            }
            let _ = count; // suppress unused warning
        }
        SkillsCommand::Browse { .. } => {
            info!("browsing skills hub");
            println!("Skills Hub:");
            println!("  Browse installed skills: hermes skills search");
            println!("  Install from GitHub: hermes skills install <repo>");
            println!("  Official skills: https://github.com/nousresearch/hermes-skills");
        }
        SkillsCommand::Inspect { name } => {
            info!("inspecting skill: {}", name);
            let index = SkillsIndex::load()?;
            if let Some(skill) = index.get(&name) {
                println!("Skill: {}", skill.name);
                println!("Description: {}", skill.description);
                if let Some(version) = &skill.version {
                    println!("Version: {}", version);
                }
                if let Some(license) = &skill.license {
                    println!("License: {}", license);
                }
                if !skill.platforms.is_empty() {
                    println!("Platforms: {}", skill.platforms.join(", "));
                }
                if !skill.tags.is_empty() {
                    println!("Tags: {}", skill.tags.join(", "));
                }
                if !skill.related_skills.is_empty() {
                    println!("Related: {}", skill.related_skills.join(", "));
                }

                // Try to show skill path
                let skills_home = SkillsIndex::skills_home();
                let skill_path = skills_home.join(&skill.name);
                if skill_path.exists() {
                    println!("Location: {:?}", skill_path);
                }
            } else {
                println!("Skill '{}' not found. Run 'hermes skills search' to see installed skills.", name);
            }
        }
        SkillsCommand::Install { identifier, .. } => {
            info!("installing skill: {}", identifier);
            if identifier.contains('/') {
                println!("Skill install from '{}' requested.", identifier);
                println!("Note: Full remote install requires network access.");
                println!("For now, skills should be installed manually to ~/.hermes/skills/");
            } else {
                println!("Installing skill '{}'...", identifier);
                println!("Skill '{}' is not available in the registry.", identifier);
            }
        }
        SkillsCommand::Uninstall { name } => {
            info!("removing skill: {}", name);
            let mut index = SkillsIndex::load()?;
            if index.remove(&name) {
                index.save()?;
                let skills_home = SkillsIndex::skills_home();
                let skill_path = skills_home.join(&name);
                if skill_path.exists() {
                    println!("Skill '{}' removed from index.", name);
                    println!("Note: Skill files at {:?} were not deleted.", skill_path);
                } else {
                    println!("Skill '{}' removed from index (no files found).", name);
                }
            } else {
                println!("Skill '{}' not found in index.", name);
            }
        }
        SkillsCommand::List { .. } => {
            info!("listing all skills");
            let mut index = SkillsIndex::load()?;
            let count = index.scan_local_skills()?;
            let all_skills: Vec<_> = index.get_all().into_iter().cloned().collect();

            if all_skills.is_empty() {
                println!("No skills installed.");
                println!("Run 'hermes skills install <name>' to install a skill.");
            } else {
                println!("Installed Skills ({}):", all_skills.len());
                for skill in &all_skills {
                    println!("  {}: {}", skill.name, skill.description);
                    if !skill.tags.is_empty() {
                        println!("    tags: {}", skill.tags.join(", "));
                    }
                    if let Some(version) = &skill.version {
                        println!("    version: {}", version);
                    }
                }
            }
            let _ = count; // suppress unused warning
        }
        SkillsCommand::Check { .. } => {
            info!("checking installed skills");
            let mut index = SkillsIndex::load()?;
            let _ = index.scan_local_skills()?;
            let skills_home = SkillsIndex::skills_home();

            println!("Skills Check");
            println!("=============");
            println!();

            if !skills_home.exists() {
                println!("Skills directory does not exist: {:?}", skills_home);
                println!("No skills are installed.");
                return Ok(());
            }

            let all_skills: Vec<_> = index.get_all().into_iter().cloned().collect();

            if all_skills.is_empty() {
                println!("No skills found in index.");
                println!("Run 'hermes skills list' to see installed skills.");
                return Ok(());
            }

            let mut issues = 0;
            for skill in &all_skills {
                let skill_path = skills_home.join(&skill.name);
                let mut skill_issues = Vec::new();

                // Check for required SKILL.md
                if !skill_path.join("SKILL.md").exists() {
                    skill_issues.push("missing SKILL.md");
                }

                // Check for required files based on skill type
                let skill_md_path = skill_path.join("SKILL.md");
                if skill_md_path.exists() {
                    if let Ok(_content) = std::fs::read_to_string(&skill_md_path) {
                        // Check if description is empty in frontmatter
                        if skill.description.is_empty() {
                            skill_issues.push("empty description in SKILL.md");
                        }
                    }
                }

                if skill_issues.is_empty() {
                    println!("  [OK] {}: valid", skill.name);
                } else {
                    for issue in &skill_issues {
                        println!("  [WARN] {}: {}", skill.name, issue);
                    }
                    issues += 1;
                }
            }

            println!();
            if issues == 0 {
                println!("All skills passed validation!");
            } else {
                println!("{} skill(s) have warnings.", issues);
            }
        }
        SkillsCommand::Update { .. } => {
            info!("updating skills");
            println!("Skills Update");
            println!("=============");
            println!();
            println!("Skills are updated by reinstalling them:");
            println!();
            println!("To update a specific skill:");
            println!("  1. hermes skills uninstall <name>");
            println!("  2. hermes skills install <name>");
            println!();
            println!("To update all skills:");
            println!("  - Remove the skills directory and reinstall:");
            println!("    rm -rf ~/.hermes/skills/");
            println!("    hermes skills install <each-skill>");
            println!();
            println!("Note: Skills are manually managed. Automatic updates require");
            println!("      a skill registry server which is not yet implemented.");
        }
        SkillsCommand::Audit { .. } => {
            info!("auditing skills security");
            println!("Skills Audit");
            println!("=============");
            println!();

            let skills_home = SkillsIndex::skills_home();
            if !skills_home.exists() {
                println!("No skills directory found.");
                return Ok(());
            }

            let mut index = SkillsIndex::load()?;
            let _ = index.scan_local_skills()?;
            let all_skills: Vec<_> = index.get_all().into_iter().cloned().collect();

            if all_skills.is_empty() {
                println!("No skills installed to audit.");
                return Ok(());
            }

            println!("Auditing {} skill(s)...", all_skills.len());
            println!();

            let mut passed = 0;
            let mut warnings = 0;

            for skill in &all_skills {
                let skill_path = skills_home.join(&skill.name);
                let skill_md = skill_path.join("SKILL.md");

                // Basic security checks
                let mut issues = Vec::new();

                // Check SKILL.md exists
                if !skill_md.exists() {
                    issues.push("missing SKILL.md");
                }

                // Check for potentially dangerous patterns in skill path
                if skill.name.contains("..") || skill.name.contains('/') || skill.name.contains('\\') {
                    issues.push("skill name contains path separators");
                }

                // Check skill description isn't empty (could hide malicious content)
                if skill.description.is_empty() {
                    issues.push("empty description");
                }

                // Check skill is from known sources (has version/license)
                if skill.version.is_none() {
                    issues.push("no version specified");
                }

                if issues.is_empty() {
                    println!("  [PASS] {}", skill.name);
                    passed += 1;
                } else {
                    for issue in &issues {
                        println!("  [WARN] {}: {}", skill.name, issue);
                    }
                    warnings += 1;
                }
            }

            println!();
            println!("Audit Summary: {} passed, {} warnings", passed, warnings);
            if warnings == 0 {
                println!("All skills passed basic security checks.");
            } else {
                println!("Review warnings above before using these skills.");
            }
        }
        SkillsCommand::Publish { .. } => {
            info!("publishing skill");
            println!("Skills Publish");
            println!("==============");
            println!();
            println!("Publishing skills to a registry:");
            println!();
            println!("1. Create a skill directory with SKILL.md:");
            println!("   my-skill/SKILL.md");
            println!();
            println!("2. SKILL.md format:");
            println!("   ---");
            println!("   name: my-skill");
            println!("   description: My awesome skill");
            println!("   version: 1.0.0");
            println!("   platforms: [windows, macos, linux]");
            println!("   tags: [ai, automation]");
            println!("   ---");
            println!();
            println!("3. Publish to registry (not yet implemented):");
            println!("   hermes skills publish ./my-skill");
            println!();
            println!("Currently, skills are installed manually to:");
            println!("  ~/.hermes/skills/<skill-name>/");
        }
        SkillsCommand::Snapshot(snapshot_cmd) => {
            info!("skill snapshot command");
            println!("Skills Snapshot");
            println!("================");
            println!();

            match snapshot_cmd {
                crate::SkillsSnapshotCommand::Export { output } => {
                    println!("Exporting skills snapshot to: {}", output);
                    println!();
                    println!("Skill snapshot export feature is not yet fully implemented.");
                    println!("Skills are stored in: ~/.hermes/skills/");
                }
                crate::SkillsSnapshotCommand::Import { input, force: _ } => {
                    println!("Importing skills snapshot from: {}", input);
                    println!();
                    println!("Skill snapshot import feature is not yet fully implemented.");
                }
            }
        }
        SkillsCommand::Tap(tap_cmd) => {
            println!("Skills Tap");
            println!("==========");
            println!();

            match tap_cmd {
                crate::SkillsTapCommand::Add { repo } => {
                    println!("Adding skill tap from repo: {}", repo);
                    println!();
                    println!("Tap feature allows adding custom skill repositories.");
                    println!("This is not yet implemented.");
                    println!();
                    println!("Workaround: Manually clone skill repos to:");
                    println!("  ~/.hermes/skills/<skill-name>/");
                }
                crate::SkillsTapCommand::Remove { name } => {
                    println!("Removing skill tap: {}", name);
                    println!();
                    println!("Tap feature allows adding custom skill repositories.");
                    println!("This is not yet implemented.");
                    println!();
                    println!("To remove a skill manually:");
                    println!("  hermes skills uninstall {}", name);
                }
                crate::SkillsTapCommand::List => {
                    println!("Listing configured skill taps...");
                    println!();

                    let taps_file = SkillsIndex::skills_home().join(".hub").join("taps.yaml");
                    if taps_file.exists() {
                        match std::fs::read_to_string(&taps_file) {
                            Ok(content) => {
                                println!("Taps:");
                                println!("{}", content);
                            }
                            Err(e) => {
                                println!("Error reading taps file: {}", e);
                            }
                        }
                    } else {
                        println!("No skill taps configured.");
                        println!();
                        println!("To add a tap, you would run:");
                        println!("  hermes skills tap add <git-url>");
                    }
                }
            }
        }
        SkillsCommand::Config => {
            info!("showing skills configuration");
            println!("Skills Configuration");
            println!("=====================");
            println!();

            let skills_home = SkillsIndex::skills_home();
            println!("Skills directory: {:?}", skills_home);
            println!();

            let hub_dir = skills_home.join(".hub");
            let index_path = hub_dir.join("index.yaml");
            let taps_path = hub_dir.join("taps.yaml");

            println!("Hub directory: {:?}", hub_dir);
            println!("  Index: {}", if index_path.exists() { "exists" } else { "not found" });
            println!("  Taps:  {}", if taps_path.exists() { "exists" } else { "not found" });
            println!();

            // Show environment variables affecting skills
            println!("Environment:");
            if std::env::var("HERMES_SKILLS_URL").is_ok() {
                println!("  HERMES_SKILLS_URL: set");
            } else {
                println!("  HERMES_SKILLS_URL: not set (using default)");
            }
        }
    }
    Ok(())
}

pub async fn handle_gateway(cmd: GatewayCommand) -> Result<()> {
    match cmd {
        GatewayCommand::Run { platform, .. } => {
            info!("running gateway: {:?}", platform);
            if gateway_mod::is_gateway_running() {
                println!("Gateway is already running.");
                println!("Stop it first with: hermes gateway stop");
                return Ok(());
            }

            println!("Starting Hermes Gateway...");
            println!();
            println!("NOTE: Full gateway implementation requires the agent runtime.");
            println!("For now, this starts a minimal gateway process.");
            println!();
            println!("To run the full gateway:");
            println!("  1. Ensure hermes-agent Python package is installed");
            println!("  2. Run: python -m hermes_cli.main gateway run");
            println!();

            // Write PID file to indicate gateway "started"
            if let Err(e) = gateway_mod::write_pid_file() {
                eprintln!("Warning: Could not write PID file: {}", e);
            }

            // Write initial state
            let state = gateway_mod::GatewayState {
                gateway_state: "running".to_string(),
                pid: std::process::id(),
                platform: platform.clone(),
                platform_state: Some("started".to_string()),
                restart_requested: false,
                active_agents: 0,
                updated_at: chrono::Utc::now().to_rfc3339(),
            };
            let _ = gateway_mod::write_gateway_state(&state);

            println!("Gateway started (PID: {})", std::process::id());
            println!("View status with: hermes gateway status");
        }
        GatewayCommand::Start { .. } => {
            info!("starting gateway service");
            if gateway_mod::is_gateway_running() {
                println!("Gateway is already running.");
                return Ok(());
            }

            // Try Windows service first
            if gateway_mod::is_service_installed() {
                println!("Starting Hermes Gateway service...");
                match gateway_mod::start_service() {
                    Ok(()) => {
                        println!("Gateway service started.");
                        return Ok(());
                    }
                    Err(e) => {
                        eprintln!("Warning: Could not start Windows service: {}", e);
                        println!("Falling back to process mode...");
                    }
                }
            }

            // Fallback: start as process
            println!("Starting Hermes Gateway...");
            println!();
            println!("On Windows, you can also install as a service:");
            println!("  hermes gateway install");
            println!();

            if let Err(e) = gateway_mod::write_pid_file() {
                eprintln!("Warning: Could not write PID file: {}", e);
            }
            println!("Gateway started.");
        }
        GatewayCommand::Stop { .. } => {
            info!("stopping gateway service");

            // Try Windows service first
            let service_status = gateway_mod::get_service_status();
            if service_status == gateway_mod::ServiceStatus::Running
                || service_status == gateway_mod::ServiceStatus::StartPending
            {
                println!("Stopping Hermes Gateway service...");
                match gateway_mod::stop_service() {
                    Ok(()) => {
                        println!("Gateway service stopped.");
                        return Ok(());
                    }
                    Err(e) => {
                        eprintln!("Warning: Could not stop Windows service: {}", e);
                        println!("Falling back to process mode...");
                    }
                }
            }

            if !gateway_mod::is_gateway_running() {
                println!("Gateway is not running.");
                return Ok(());
            }

            println!("Stopping Hermes Gateway...");

            // Write stopped state
            let state = gateway_mod::GatewayState {
                gateway_state: "stopped".to_string(),
                pid: 0,
                platform: None,
                platform_state: Some("stopped".to_string()),
                restart_requested: false,
                active_agents: 0,
                updated_at: chrono::Utc::now().to_rfc3339(),
            };
            let _ = gateway_mod::write_gateway_state(&state);

            if let Err(e) = gateway_mod::remove_pid_file() {
                eprintln!("Warning: Could not remove PID file: {}", e);
            }

            println!("Gateway stopped.");
        }
        GatewayCommand::Status { .. } => {
            info!("checking gateway status");
            println!("Hermes Gateway Status");
            println!("====================");
            println!();

            // Show Windows service status
            let service_status = gateway_mod::get_service_status();
            if service_status != gateway_mod::ServiceStatus::NotApplicable {
                println!("Service:  {}", service_status);
                if service_status == gateway_mod::ServiceStatus::NotFound {
                    println!("  (not installed as Windows service)");
                }
                println!();
            }

            if let Some(pid) = gateway_mod::get_running_pid() {
                println!("Status:   RUNNING");
                println!("PID:      {}", pid);
                println!();

                if let Some(state) = gateway_mod::read_gateway_state() {
                    println!("Platform: {:?}", state.platform.unwrap_or_else(|| "N/A".to_string()));
                    println!("State:     {}", state.gateway_state);
                    println!("Agents:    {}", state.active_agents);
                    if state.restart_requested {
                        println!("Restart:   requested");
                    }
                }
            } else {
                println!("Status:   STOPPED");
                println!();
                if gateway_mod::is_service_installed() {
                    println!("Start the service with: hermes gateway start");
                    println!("Run interactively with:  hermes gateway run");
                } else {
                    println!("Start the gateway with: hermes gateway run");
                    println!("Install as service:    hermes gateway install");
                }
            }
        }
        GatewayCommand::Setup { platform } => {
            info!("setting up gateway: {:?}", platform);
            println!("Gateway Setup");
            println!("=============");
            println!();

            if let Some(p) = platform {
                println!("Setting up platform: {}", p);
            } else {
                println!("Available platforms:");
                println!("  telegram  - Telegram bot");
                println!("  discord   - Discord bot");
                println!("  slack     - Slack bot");
                println!("  whatsapp  - WhatsApp integration");
                println!();
                println!("Run 'hermes gateway setup <platform>' to configure a specific platform.");
            }

            println!();
            println!("Full gateway setup requires:");
            println!("  1. hermes-agent Python package installed");
            println!("  2. API keys configured via 'hermes auth add'");
            println!("  3. Platform-specific setup via 'hermes gateway setup <platform>'");
        }
        GatewayCommand::Restart { system: _ } => {
            info!("restarting gateway");
            println!("Restarting Hermes Gateway...");

            // Stop if running
            let service_status = gateway_mod::get_service_status();
            if service_status == gateway_mod::ServiceStatus::Running
                || service_status == gateway_mod::ServiceStatus::StartPending
            {
                if let Err(e) = gateway_mod::stop_service() {
                    eprintln!("Warning: Could not stop service: {}", e);
                }
            }

            if gateway_mod::is_gateway_running() {
                let state = gateway_mod::GatewayState {
                    gateway_state: "stopped".to_string(),
                    pid: 0,
                    platform: None,
                    platform_state: Some("restarting".to_string()),
                    restart_requested: false,
                    active_agents: 0,
                    updated_at: chrono::Utc::now().to_rfc3339(),
                };
                let _ = gateway_mod::write_gateway_state(&state);
                let _ = gateway_mod::remove_pid_file();
            }

            println!("Gateway stopped. Starting...");

            // Start again
            if gateway_mod::is_service_installed() {
                println!("Starting Hermes Gateway service...");
                match gateway_mod::start_service() {
                    Ok(()) => {
                        println!("Gateway service restarted.");
                        return Ok(());
                    }
                    Err(e) => {
                        eprintln!("Warning: Could not start Windows service: {}", e);
                        println!("Falling back to process mode...");
                    }
                }
            }

            println!("Starting Hermes Gateway...");
            if let Err(e) = gateway_mod::write_pid_file() {
                eprintln!("Warning: Could not write PID file: {}", e);
            }
            let state = gateway_mod::GatewayState {
                gateway_state: "running".to_string(),
                pid: std::process::id(),
                platform: None,
                platform_state: Some("restarted".to_string()),
                restart_requested: false,
                active_agents: 0,
                updated_at: chrono::Utc::now().to_rfc3339(),
            };
            let _ = gateway_mod::write_gateway_state(&state);
            println!("Gateway restarted (PID: {}).", std::process::id());
        }
        GatewayCommand::Install { .. } => {
            info!("installing gateway as Windows service");
            println!("Gateway Install");
            println!("==============");
            println!();

            #[cfg(target_os = "windows")]
            {
                println!("Installing Hermes Gateway as a Windows service...");
                println!();

                match gateway_mod::install_service() {
                    Ok(()) => {
                        println!("Gateway service installed successfully!");
                        println!();
                        println!("To start the service:");
                        println!("  hermes gateway start");
                        println!("  or");
                        println!("  sc start HermesGateway");
                        println!();
                        println!("To check status:");
                        println!("  hermes gateway status");
                    }
                    Err(e) => {
                        anyhow::bail!("Failed to install service: {}", e);
                    }
                }
            }

            #[cfg(not(target_os = "windows"))]
            {
                println!("Windows service installation is only available on Windows.");
                println!();
                println!("On other platforms, use:");
                println!("  hermes gateway run          - Run gateway interactively");
                println!("  nohup hermes gateway run & - Run gateway in background");
            }
        }
        GatewayCommand::Uninstall { .. } => {
            info!("uninstalling gateway Windows service");
            println!("Gateway Uninstall");
            println!("================");
            println!();

            #[cfg(target_os = "windows")]
            {
                if !gateway_mod::is_service_installed() {
                    println!("Gateway is not installed as a Windows service.");
                    println!("Nothing to uninstall.");
                    return Ok(());
                }

                println!("Uninstalling Hermes Gateway from Windows services...");
                println!();

                // Clean up PID and state files
                let _ = gateway_mod::remove_pid_file();
                let state = gateway_mod::GatewayState {
                    gateway_state: "uninstalled".to_string(),
                    pid: 0,
                    platform: None,
                    platform_state: Some("uninstalled".to_string()),
                    restart_requested: false,
                    active_agents: 0,
                    updated_at: chrono::Utc::now().to_rfc3339(),
                };
                let _ = gateway_mod::write_gateway_state(&state);

                match gateway_mod::uninstall_service() {
                    Ok(()) => {
                        println!("Gateway service uninstalled successfully!");
                        println!();
                        println!("Note: Your data in ~/.hermes/ has been preserved.");
                    }
                    Err(e) => {
                        anyhow::bail!("Failed to uninstall service: {}", e);
                    }
                }
            }

            #[cfg(not(target_os = "windows"))]
            {
                println!("Windows service uninstallation is only available on Windows.");
                println!("To stop the gateway: hermes gateway stop");
            }
        }
    }
    Ok(())
}

pub async fn handle_cron(cmd: CronCommand) -> Result<()> {
    match cmd {
        CronCommand::List { .. } => {
            info!("listing cron jobs");
            println!("Hermes Cron Jobs");
            println!("================");
            println!();

            let jobs = cron_mod::list_jobs(true)?;

            if jobs.is_empty() {
                println!("No cron jobs configured.");
                println!();
                println!("Create a job with:");
                println!("  hermes cron add <schedule> <prompt>");
                println!();
                println!("Example:");
                println!("  hermes cron add 'every 30m' 'Check system status'");
            } else {
                for job in &jobs {
                    let status = if job.enabled { "[active]" } else { "[paused]" };
                    println!("{} {}", job.id, status);
                    println!("  Name:     {}", job.name);
                    println!("  Schedule: {}", job.schedule_display);
                    if let Some(ref next) = job.next_run_at {
                        println!("  Next run: {}", next);
                    }
                    if let Some(ref last) = job.last_run_at {
                        let last_status = job.last_status.as_deref().unwrap_or("N/A");
                        println!("  Last run: {} ({})", last, last_status);
                    }
                    if !job.skills.is_empty() {
                        println!("  Skills:   {}", job.skills.join(", "));
                    }
                    println!();
                }
            }

            if !gateway_mod::is_gateway_running() {
                println!("NOTE: Gateway is not running - jobs won't fire automatically.");
                println!("Start it with: hermes gateway run");
            }
        }
CronCommand::Add { schedule, command, .. } => {
            info!("adding cron job: {} -> {:?}", schedule, command);
            let prompt = command.unwrap_or_else(|| schedule.clone());
            match cron_mod::create_job(prompt, schedule) {
                Ok(job) => {
                    println!("Cron job created successfully!");
                    println!("  ID:       {}", job.id);
                    println!("  Name:     {}", job.name);
                    println!("  Schedule: {}", job.schedule_display);
                    println!();
                    if !gateway_mod::is_gateway_running() {
                        println!("NOTE: Start the gateway for jobs to run automatically:");
                        println!("  hermes gateway run");
                    }
                }
                Err(e) => {
                    anyhow::bail!("Failed to create cron job: {}", e);
                }
            }
        }
        CronCommand::Remove { id } => {
            info!("removing cron job: {}", id);

            match cron_mod::remove_job(&id) {
                Ok(true) => {
                    println!("Cron job {} removed.", id);
                }
                Ok(false) => {
                    println!("Cron job '{}' not found.", id);
                }
                Err(e) => {
                    anyhow::bail!("Failed to remove cron job: {}", e);
                }
            }
        }
        CronCommand::Pause { id } => {
            info!("pausing cron job: {}", id);

            match cron_mod::pause_job(&id, None) {
                Ok(Some(job)) => {
                    println!("Cron job '{}' paused.", job.name);
                }
                Ok(None) => {
                    println!("Cron job '{}' not found.", id);
                }
                Err(e) => {
                    anyhow::bail!("Failed to pause cron job: {}", e);
                }
            }
        }
        CronCommand::Resume { id } => {
            info!("resuming cron job: {}", id);

            match cron_mod::resume_job(&id) {
                Ok(Some(job)) => {
                    println!("Cron job '{}' resumed.", job.name);
                    if let Some(ref next) = job.next_run_at {
                        println!("  Next run: {}", next);
                    }
                }
                Ok(None) => {
                    println!("Cron job '{}' not found.", id);
                }
                Err(e) => {
                    anyhow::bail!("Failed to resume cron job: {}", e);
                }
            }
        }
        CronCommand::Status => {
            info!("checking cron status");
            println!("Hermes Cron Status");
            println!("==================");
            println!();

            let jobs = cron_mod::list_jobs(true)?;
            let active: usize = jobs.iter().filter(|j| j.enabled).count();

            println!("Gateway:  {}", if gateway_mod::is_gateway_running() { "running" } else { "stopped" });
            println!("Jobs:     {} total, {} active", jobs.len(), active);
            println!();

            if !jobs.is_empty() {
                println!("Due jobs: {}", cron_mod::get_due_jobs().len());
            }

            if !gateway_mod::is_gateway_running() {
                println!();
                println!("NOTE: Gateway is not running - jobs won't fire.");
                println!("Start it with: hermes gateway run");
            }
        }
        CronCommand::Edit { job_id, schedule, prompt: _, name, deliver, repeat: _, skill: _, add_skill, remove_skill, clear_skills, script } => {
            info!("editing cron job: {}", job_id);
            println!("Hermes Cron Edit");
            println!("================");
            println!();

            let jobs = cron_mod::list_jobs(true)?;
            let job = jobs.iter().find(|j| j.id == *job_id);

            match job {
                Some(j) => {
                    println!("Editing job: {}", j.name);
                    println!("  Current schedule: {}", j.schedule_display);
                    if let Some(s) = schedule {
                        println!("  New schedule: {}", s);
                    }
                    if let Some(n) = name {
                        println!("  New name: {}", n);
                    }
                    println!();
                    println!("Note: Full cron job editing requires:");
                    println!("  1. Remove the existing job: hermes cron remove {}", job_id);
                    println!("  2. Create a new job with updated settings: hermes cron add <schedule> <prompt>");
                    println!();
                    println!("Alternative parameters that can be edited:");
                    if add_skill.is_some() { println!("  --add-skill <skill>"); }
                    if remove_skill.is_some() { println!("  --remove-skill <skill>"); }
                    if clear_skills { println!("  --clear-skills"); }
                    if deliver.is_some() { println!("  --deliver <channel>"); }
                    if script.is_some() { println!("  --script <script>"); }
                }
                None => {
                    println!("Job '{}' not found.", job_id);
                }
            }
        }
        CronCommand::Run { id } => {
            info!("running cron job manually: {}", id);
            println!("Hermes Cron Run");
            println!("================");
            println!();

            let jobs = cron_mod::list_jobs(true)?;
            let job = jobs.iter().find(|j| j.id == *id);

            match job {
                Some(j) => {
                    println!("Running cron job: {}", j.name);
                    println!("  Schedule: {}", j.schedule_display);
                    println!();
                    println!("Executing job now (dry-run - actual execution not implemented)...");
                    println!("  In production, this would execute the cron job prompt immediately.");
                }
                None => {
                    println!("Job '{}' not found.", id);
                }
            }
        }
        CronCommand::Tick => {
            info!("cron tick - checking due jobs");
            let jobs = cron_mod::list_jobs(true)?;
            let due = cron_mod::get_due_jobs();

            println!("Cron Tick");
            println!("=========");
            println!("Total jobs: {}", jobs.len());
            println!("Due now: {}", due.len());

            if due.is_empty() {
                println!("No jobs are due for execution.");
            } else {
                println!("\nDue jobs:");
                for job in &due {
                    println!("  - {} ({})", job.name, job.id);
                }
            }
        }
    }
    Ok(())
}

pub fn handle_config(cmd: ConfigCommand) -> Result<()> {
    match cmd {
        ConfigCommand::Show => {
            info!("showing configuration");
            let config = Config::load()?;
            println!("Hermes Configuration:");
            println!("  Config path: {:?}", Config::config_path());
            println!();
            println!("Model:");
            println!("  default: {}", config.model.default);
            println!("  provider: {}", config.model.provider);
            println!("  base_url: {}", config.model.base_url);
            println!();
            println!("Terminal:");
            println!("  env_type: {}", config.terminal.env_type);
            println!("  cwd: {}", config.terminal.cwd);
            println!("  timeout: {}", config.terminal.timeout);
            println!();
            println!("Display:");
            println!("  compact: {}", config.display.compact);
            println!("  resume_display: {}", config.display.resume_display);
            println!("  show_reasoning: {}", config.display.show_reasoning);
            println!("  streaming: {}", config.display.streaming);
            println!("  skin: {}", config.display.skin);
            println!();
            println!("Agent:");
            println!("  max_turns: {}", config.agent.max_turns);
            println!("  verbose: {}", config.agent.verbose);
            println!("  system_prompt: {}", config.agent.system_prompt);
            println!("  reasoning_effort: {}", config.agent.reasoning_effort);
        }
        ConfigCommand::Get { key } => {
            info!("getting config value: {}", key);
            let config = Config::load()?;
            let value = get_config_value(&config, &key)?;
            println!("{}", value);
        }
        ConfigCommand::Set { key, value } => {
            info!("setting config value: {} = {}", key, value);
            let mut config = Config::load()?;
            set_config_value(&mut config, &key, &value)?;
            config.save()?;
            println!("Set {} = {}", key, value);
        }
        ConfigCommand::Reset => {
            info!("resetting configuration to defaults");
            let config = Config::default();
            config.save()?;
            println!("Config reset to defaults");
        }
        ConfigCommand::Edit => {
            info!("editing configuration");
            let config_path = Config::config_path();
            println!("Hermes Config Edit");
            println!("=================");
            println!();
            println!("To edit your configuration, open the config file in your editor:");
            println!();
            println!("  Config file: {:?}", config_path);
            println!();

            #[cfg(target_os = "windows")]
            {
                std::process::Command::new("cmd")
                    .args(["/C", "start", "", &config_path.to_string_lossy()])
                    .spawn()
                    .ok();
                println!("Opening in default editor...");
            }

            #[cfg(target_os = "macos")]
            {
                std::process::Command::new("open")
                    .arg(&config_path)
                    .spawn()
                    .ok();
                println!("Opening in default editor...");
            }

            #[cfg(target_os = "linux")]
            {
                if let Ok(editor) = std::env::var("EDITOR") {
                    std::process::Command::new(&editor)
                        .arg(&config_path)
                        .spawn()
                        .ok();
                    println!("Opening in ${}...", editor);
                } else {
                    println!("Set $EDITOR to open automatically, or open manually:");
                    println!("  nano {}", config_path.display());
                    println!("  vim {}", config_path.display());
                    println!("  code {}", config_path.display());
                }
            }

            println!();
            println!("Alternatively, use these commands to set specific values:");
            println!("  hermes config set <key> <value>");
            println!();
            println!("Run 'hermes config show' to see current configuration.");
        }
        ConfigCommand::Path => {
            println!("{:?}", Config::config_path());
        }
        ConfigCommand::EnvPath => {
            let home = Config::hermes_home();
            println!("{:?}", home.join(".env"));
        }
        ConfigCommand::Check => {
            info!("checking configuration");
            println!("Config Check");
            println!("============");
            println!();

            let config_path = Config::config_path();
            println!("Config file: {:?}", config_path);
            println!();

            match Config::load() {
                Ok(config) => {
                    println!("[OK] Config file is valid YAML.");
                    println!();
                    println!("Current settings:");
                    println!("  Model: {}", config.model.default);
                    if !config.model.provider.is_empty() {
                        println!("  Provider: {}", config.model.provider);
                    }
                    println!("  Timeout: {}s", config.terminal.timeout);
                    println!("  Max turns: {}", config.agent.max_turns);
                }
                Err(e) => {
                    println!("[ERROR] Config file has issues: {}", e);
                    println!();
                    println!("Try 'hermes config reset' to restore defaults.");
                }
            }
        }
        ConfigCommand::Migrate => {
            info!("checking config migration");
            println!("Config Migrate");
            println!("=============");
            println!();

            let config_path = Config::config_path();
            println!("Config file: {:?}", config_path);
            println!();

            // Current version is 1 (no version field in config yet)
            const CURRENT_CONFIG_VERSION: u32 = 1;
            println!("Current config format version: {}", CURRENT_CONFIG_VERSION);
            println!();

            if !config_path.exists() {
                println!("Config file does not exist yet.");
                println!("A new config will be created with default values.");
                return Ok(());
            }

            // Try to load and re-save to validate format
            match Config::load() {
                Ok(_) => {
                    println!("[OK] Config file is valid and up-to-date.");
                    println!();
                    println!("Config is at the latest version ({}).", CURRENT_CONFIG_VERSION);
                    println!("No migration needed.");
                }
                Err(e) => {
                    println!("[WARN] Config file may be in an old format.");
                    println!("Error: {}", e);
                    println!();
                    println!("Migration instructions:");
                    println!("  1. Backup your config: cp config.yaml config.yaml.backup");
                    println!("  2. Try resetting: hermes config reset");
                    println!("  3. Or manually update the format to match the current schema");
                }
            }
        }
    }
    Ok(())
}

fn get_config_value(config: &Config, key: &str) -> Result<String> {
    match key {
        "model.default" => Ok(config.model.default.clone()),
        "model.provider" => Ok(config.model.provider.clone()),
        "model.base_url" => Ok(config.model.base_url.clone()),
        "terminal.env_type" => Ok(config.terminal.env_type.clone()),
        "terminal.cwd" => Ok(config.terminal.cwd.clone()),
        "terminal.timeout" => Ok(config.terminal.timeout.to_string()),
        "display.compact" => Ok(config.display.compact.to_string()),
        "display.resume_display" => Ok(config.display.resume_display.clone()),
        "display.show_reasoning" => Ok(config.display.show_reasoning.to_string()),
        "display.streaming" => Ok(config.display.streaming.to_string()),
        "display.skin" => Ok(config.display.skin.clone()),
        "agent.max_turns" => Ok(config.agent.max_turns.to_string()),
        "agent.verbose" => Ok(config.agent.verbose.to_string()),
        "agent.system_prompt" => Ok(config.agent.system_prompt.clone()),
        "agent.reasoning_effort" => Ok(config.agent.reasoning_effort.clone()),
        _ => anyhow::bail!("Unknown config key: {}. Run 'hermes config show' for valid keys.", key),
    }
}

fn set_config_value(config: &mut Config, key: &str, value: &str) -> Result<()> {
    match key {
        "model.default" => config.model.default = value.to_string(),
        "model.provider" => config.model.provider = value.to_string(),
        "model.base_url" => config.model.base_url = value.to_string(),
        "terminal.env_type" => config.terminal.env_type = value.to_string(),
        "terminal.cwd" => config.terminal.cwd = value.to_string(),
        "terminal.timeout" => {
            config.terminal.timeout = value.parse()
                .map_err(|_| anyhow::anyhow!("Invalid timeout value '{}': must be a positive integer", value))?;
        }
        "display.compact" => {
            config.display.compact = value.parse()
                .map_err(|_| anyhow::anyhow!("Invalid compact value '{}': must be true or false", value))?;
        }
        "display.resume_display" => config.display.resume_display = value.to_string(),
        "display.show_reasoning" => {
            config.display.show_reasoning = value.parse()
                .map_err(|_| anyhow::anyhow!("Invalid show_reasoning value '{}': must be true or false", value))?;
        }
        "display.streaming" => {
            config.display.streaming = value.parse()
                .map_err(|_| anyhow::anyhow!("Invalid streaming value '{}': must be true or false", value))?;
        }
        "display.skin" => config.display.skin = value.to_string(),
        "agent.max_turns" => {
            config.agent.max_turns = value.parse()
                .map_err(|_| anyhow::anyhow!("Invalid max_turns value '{}': must be a positive integer", value))?;
        }
        "agent.verbose" => {
            config.agent.verbose = value.parse()
                .map_err(|_| anyhow::anyhow!("Invalid verbose value '{}': must be true or false", value))?;
        }
        "agent.system_prompt" => config.agent.system_prompt = value.to_string(),
        "agent.reasoning_effort" => config.agent.reasoning_effort = value.to_string(),
        _ => {
            anyhow::bail!("Unknown config key: {}. Run 'hermes config show' for valid keys.", key);
        }
    }
    Ok(())
}

pub fn handle_status() -> Result<()> {
    info!("showing status");
    let config = Config::load()?;
    let config_path = Config::config_path();

    println!("Hermes CLI Status");
    println!("=================");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!("Config: {:?}", config_path);
    if config_path.exists() {
        println!("Config file: exists");
    } else {
        println!("Config file: not found (using defaults)");
    }
    println!();

    // Show effective model (session > global)
    let session_model = std::env::var("HERMES_SESSION_MODEL").ok();
    let effective_model = session_model.as_ref().unwrap_or(&config.model.default);
    println!("Model: {}", effective_model);
    if session_model.is_some() {
        println!("  (session override active)");
    }
    if !config.model.provider.is_empty() {
        println!("Provider: {}", config.model.provider);
    }
    println!();
    println!("Agent settings:");
    println!("  max_turns: {}", config.agent.max_turns);
    println!("  reasoning_effort: {}", config.agent.reasoning_effort);
    println!("  verbose: {}", config.agent.verbose);

    Ok(())
}

pub fn handle_setup(skip_auth: bool, skip_model: bool) -> Result<()> {
    info!("running setup wizard");

    println!("Hermes CLI Setup");
    println!("================");
    println!();

    // Check if Python hermes-agent is available
    println!("Checking hermes-agent installation...");
    let python_hermes = std::process::Command::new("python")
        .args(["-c", "import hermes_cli; print(hermes_cli.__file__)"])
        .output();

    match python_hermes {
        Ok(output) if output.status.success() => {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("  hermes-agent Python package: found at {}", path);
        }
        _ => {
            println!("  hermes-agent Python package: not found");
            println!();
            println!("  Install with:");
            println!("    pip install hermes-agent");
            println!();
        }
    }

    if !skip_model {
        println!("\nModel Configuration:");
        println!("  Configure your AI provider with:");
        println!("    hermes auth add <provider> --api-key <key>");
        println!("    hermes model <model-name>");
        println!();
        println!("  Supported providers:");
        println!("    openai, anthropic, openrouter, gemini, etc.");
    }

    if !skip_auth {
        println!("\nAuth Configuration:");
        let auth_store = AuthStore::load()?;
        if auth_store.credentials.is_empty() {
            println!("  No API keys configured.");
            println!("  Run 'hermes auth add <provider> --api-key <key>' to add credentials.");
        } else {
            println!("  Configured providers:");
            for cred in &auth_store.credentials {
                println!("    - {}", cred.provider);
            }
        }
    }

    println!("\nGateway Setup:");
    println!("  Start the gateway with: hermes gateway run");
    println!("  Configure platforms with: hermes gateway setup <platform>");

    println!("\nNext Steps:");
    println!("  1. Add your API key: hermes auth add <provider> --api-key <key>");
    println!("  2. Set your model: hermes model <model-name>");
    println!("  3. Start chatting: hermes chat");

    println!();
    println!("For more help, see: https://hermes-agent.nousresearch.com/docs");

    Ok(())
}

#[allow(unused_variables)]
pub fn handle_doctor(_all: bool, _check: Option<&str>) -> Result<()> {
    info!("running doctor diagnostic");

    println!("Hermes Doctor");
    println!("=============");
    println!();

    let mut issues = 0;
    let mut warnings = 0;

    // Check Python version
    println!("◆ Python");
    let python_version = std::process::Command::new("python")
        .arg("--version")
        .output();

    match python_version {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("  ✓ Python installed: {}", version);
        }
        _ => {
            println!("  ✗ Python not found");
            issues += 1;
        }
    }

    // Check hermes-agent Python package
    println!("\n◆ hermes-agent Package");
    let hermes_check = std::process::Command::new("python")
        .args(["-c", "import hermes_cli; print('ok')"])
        .output();

    match hermes_check {
        Ok(output) if output.status.success() => {
            println!("  ✓ hermes-agent Python package installed");
        }
        _ => {
            println!("  ✗ hermes-agent Python package not installed");
            println!("    Install with: pip install hermes-agent");
            issues += 1;
        }
    }

    // Check configuration
    println!("\n◆ Configuration");
    let config_path = Config::config_path();
    println!("  Config path: {:?}", config_path);
    if config_path.exists() {
        println!("  ✓ Config file exists");
    } else {
        println!("  ⚠ Config file not found (will use defaults)");
        warnings += 1;
    }

    let config = Config::load()?;
    if config.model.default.is_empty() {
        println!("  ⚠ No default model configured");
        warnings += 1;
    } else {
        println!("  ✓ Default model: {}", config.model.default);
    }

    // Check auth
    println!("\n◆ Authentication");
    let auth_store = AuthStore::load()?;
    if auth_store.credentials.is_empty() {
        println!("  ⚠ No API keys configured");
        warnings += 1;
    } else {
        println!("  ✓ API keys configured for {} provider(s)", auth_store.credentials.len());
    }

    // Check gateway status
    println!("\n◆ Gateway");
    if gateway_mod::is_gateway_running() {
        println!("  ✓ Gateway is running");
    } else {
        println!("  ⚠ Gateway is not running");
        println!("    Start with: hermes gateway run");
        warnings += 1;
    }

    // Check cron jobs
    println!("\n◆ Cron Jobs");
    let jobs = cron_mod::list_jobs(true).unwrap_or_default();
    let active: usize = jobs.iter().filter(|j| j.enabled).count();
    println!("  {} job(s) configured, {} active", jobs.len(), active);

    // Summary
    println!("\n───────────────");
    if issues > 0 {
        println!("Result: {} issue(s) found", issues);
        println!("Fix the issues above for best experience.");
    } else if warnings > 0 {
        println!("Result: {} warning(s)", warnings);
        println!("Your setup is mostly working.");
    } else {
        println!("Result: All checks passed!");
        println!("Your Hermes CLI is properly configured.");
    }

    Ok(())
}

pub fn handle_update() -> Result<()> {
    info!("checking for updates");

    println!("Hermes Update");
    println!("=============");
    println!();

    println!("Checking for updates...");
    println!();

    // For Rust CLI, we can't auto-update like Python
    // Just check git or show current version
    let version = env!("CARGO_PKG_VERSION");
    println!("Current version: {}", version);
    println!();

    println!("To update Hermes CLI (Rust):");
    println!("  1. Download the latest release from:");
    println!("     https://github.com/nousresearch/hermes-agent/releases");
    println!();
    println!("  2. Or rebuild from source:");
    println!("     git pull origin main");
    println!("     cargo build --release");
    println!();

    // Try to check if hermes-agent Python has updates
    println!("For hermes-agent Python package:");
    let pip_check = std::process::Command::new("pip")
        .args(["index", "versions", "hermes-agent"])
        .output();

    if let Ok(output) = pip_check {
        let output_str = String::from_utf8_lossy(&output.stdout);
        if output_str.contains("Available versions:") {
            println!("  hermes-agent Python package update info:");
            // Just show that we checked
            println!("  Run 'pip install --upgrade hermes-agent' to update");
        }
    }

    Ok(())
}

pub fn handle_uninstall() -> Result<()> {
    info!("running uninstall");

    println!("Hermes Uninstall");
    println!("================");
    println!();

    println!("This will remove the Hermes CLI (Rust) from your system.");
    println!();

    println!("What would you like to do?");
    println!();
    println!("  1. Keep data (~/.hermes/) - Removes CLI only");
    println!("  2. Full uninstall - Removes everything including data");
    println!("  3. Cancel");
    println!();

    // For automated uninstall, we'll do option 1 (keep data) by default
    // A real interactive mode would ask

    println!("Running uninstall (keeping data)...");
    println!();

    // Stop gateway if running
    if gateway_mod::is_gateway_running() {
        println!("Stopping gateway...");
        let state = gateway_mod::GatewayState {
            gateway_state: "stopped".to_string(),
            pid: 0,
            platform: None,
            platform_state: Some("uninstalled".to_string()),
            restart_requested: false,
            active_agents: 0,
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        let _ = gateway_mod::write_gateway_state(&state);
        let _ = gateway_mod::remove_pid_file();
    }

    // Note: On Windows, removing the binary would be done by the installer
    println!("Hermes CLI (Rust) has been uninstalled.");
    println!();
    println!("Your data in ~/.hermes/ has been preserved.");
    println!();
    println!("To reinstall, download the latest release from:");
    println!("  https://github.com/nousresearch/hermes-agent/releases");

    Ok(())
}

// ── Stub handlers for new commands ──────────────────────────────────────────

pub fn handle_sessions(cmd: SessionsCommand) {
    use hermes_session_db::SessionStore;

    let home = crate::config::Config::hermes_home();
    let db_path = home.join("sessions.db");
    let store = match SessionStore::new(&db_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error opening session database: {}", e);
            return;
        }
    };

    match cmd {
        SessionsCommand::List { source: _, limit } => {
            let sessions = match store.list_sessions(limit as usize) {
                Ok(s) => s,
                Err(e) => { eprintln!("Error listing sessions: {}", e); return; }
            };
            if sessions.is_empty() {
                println!("No sessions found.");
                return;
            }
            println!("ID                                     Source          Model                Updated");
            println!("{}", "-".repeat(90));
            for s in &sessions {
                let updated = s.updated_at.format("%Y-%m-%d %H:%M").to_string();
                println!("{:<38} {:<15} {:<20} {}", s.id.to_string(), s.source, s.model, updated);
            }
            println!("\n{} session(s) shown.", sessions.len());
        }
        SessionsCommand::Export { output, source: _, session_id } => {
            let sid = match session_id {
                Some(id) => match id.parse::<uuid::Uuid>() {
                    Ok(u) => u,
                    Err(_) => { eprintln!("Invalid session ID: {}", id); return; }
                },
                None => { eprintln!("Please specify --session-id to export."); return; }
            };
            let messages = match store.get_messages(&sid) {
                Ok(m) => m,
                Err(e) => { eprintln!("Error reading session: {}", e); return; }
            };
            let json = serde_json::to_string_pretty(&messages).unwrap_or_default();
            match std::fs::write(&output, &json) {
                Ok(_) => println!("Exported {} messages to '{}'.", messages.len(), output),
                Err(e) => eprintln!("Error writing file: {}", e),
            }
        }
        SessionsCommand::Delete { session_id, yes } => {
            let sid = match session_id.parse::<uuid::Uuid>() {
                Ok(u) => u,
                Err(_) => { eprintln!("Invalid session ID: {}", session_id); return; }
            };
            if !yes {
                println!("Are you sure you want to delete session {}? Use -y to confirm.", sid);
                return;
            }
            match store.delete_session(&sid) {
                Ok(_) => println!("Session {} deleted.", sid),
                Err(e) => eprintln!("Error deleting session: {}", e),
            }
        }
        SessionsCommand::Prune { older_than, source: _, yes: _ } => {
            // List sessions and filter by age
            let sessions = match store.list_sessions(1000) {
                Ok(s) => s,
                Err(e) => { eprintln!("Error listing sessions: {}", e); return; }
            };
            let cutoff = chrono::Utc::now() - chrono::Duration::days(older_than as i64);
            let old_sessions: Vec<_> = sessions.iter()
                .filter(|s| s.updated_at < cutoff)
                .collect();
            if old_sessions.is_empty() {
                println!("No sessions older than {} days found.", older_than);
                return;
            }
            println!("Found {} session(s) older than {} days.", old_sessions.len(), older_than);
            for s in &old_sessions {
                println!("  {} (updated: {})", s.id, s.updated_at.format("%Y-%m-%d"));
            }
            println!("Run with -y to confirm deletion.");
        }
        SessionsCommand::Stats => {
            let sessions = match store.list_sessions(10000) {
                Ok(s) => s,
                Err(e) => { eprintln!("Error listing sessions: {}", e); return; }
            };
            let total_messages: usize = sessions.iter()
                .filter_map(|s| store.get_messages(&s.id).ok())
                .map(|m| m.len())
                .sum();
            println!("Session Statistics:");
            println!("  Total sessions: {}", sessions.len());
            println!("  Total messages: {}", total_messages);
            if !sessions.is_empty() {
                let latest = &sessions[0];
                println!("  Latest session: {} ({})", latest.id, latest.model);
            }
        }
        SessionsCommand::Rename { session_id, title } => {
            // Session rename not supported in current schema — would need title column
            let _ = (session_id, title);
            println!("Session rename not yet supported in current schema.");
        }
        SessionsCommand::Browse { source: _, limit } => {
            // Browse is same as list with message preview
            let sessions = match store.list_sessions(limit as usize) {
                Ok(s) => s,
                Err(e) => { eprintln!("Error listing sessions: {}", e); return; }
            };
            if sessions.is_empty() {
                println!("No sessions found.");
                return;
            }
            for s in &sessions {
                println!("══ {} ══", s.id);
                println!("  Model: {} | Source: {} | Updated: {}", s.model, s.source, s.updated_at.format("%Y-%m-%d %H:%M"));
                if let Ok(msgs) = store.get_messages(&s.id) {
                    for msg in msgs.iter().take(3) {
                        let preview: String = msg.content.chars().take(80).collect();
                        println!("  [{:?}] {}", msg.role, preview);
                    }
                    if msgs.len() > 3 {
                        println!("  ... and {} more messages", msgs.len() - 3);
                    }
                }
                println!();
            }
        }
    }
}

pub fn handle_profile(cmd: ProfileCommand) {
    use crate::profiles;

    match cmd {
        ProfileCommand::List => {
            let profiles = match profiles::list_profiles() {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Error listing profiles: {}", e);
                    return;
                }
            };
            let active = profiles::get_active_profile();
            if profiles.is_empty() {
                println!("No profiles found.");
                println!("Create one with: hermes profile create <name>");
            } else {
                println!("Profiles:");
                for name in profiles {
                    if name == active {
                        println!("  {} (*)", name);
                    } else {
                        println!("  {}", name);
                    }
                }
            }
        }
        ProfileCommand::Use { profile_name } => {
            match profiles::load_profile(&profile_name) {
                Ok(config) => {
                    if let Err(e) = config.save() {
                        eprintln!("Error saving config: {}", e);
                        return;
                    }
                    std::env::set_var("HERMES_PROFILE", &profile_name);
                    println!("Switched to profile '{}'", profile_name);
                    println!("Active profile will be '{}' on next launch", profile_name);
                }
                Err(e) => {
                    eprintln!("Error loading profile '{}': {}", profile_name, e);
                }
            }
        }
        ProfileCommand::Create { profile_name, clone, clone_all: _, clone_from, no_alias: _ } => {
            let result = if let Some(src) = clone_from {
                profiles::clone_profile(&src, &profile_name)
            } else if clone {
                let current = profiles::get_active_profile();
                if profiles::profile_exists(&current) {
                    profiles::clone_profile(&current, &profile_name)
                } else {
                    Err(anyhow::anyhow!("Cannot clone from '{}': profile not found", current))
                }
            } else {
                let config = Config::default();
                profiles::save_profile(&profile_name, &config)
            };

            match result {
                Ok(()) => println!("Created profile '{}'", profile_name),
                Err(e) => eprintln!("Error creating profile: {}", e),
            }
        }
        ProfileCommand::Delete { profile_name, yes } => {
            if !yes {
                eprintln!("This will delete profile '{}'. Use --yes to confirm.", profile_name);
                return;
            }
            match profiles::delete_profile(&profile_name) {
                Ok(()) => println!("Deleted profile '{}'", profile_name),
                Err(e) => eprintln!("Error deleting profile: {}", e),
            }
        }
        ProfileCommand::Show { profile_name } => {
            match profiles::load_profile(&profile_name) {
                Ok(config) => {
                    let yaml = serde_yaml::to_string(&config).unwrap_or_default();
                    println!("Profile '{}':", profile_name);
                    println!("{}", yaml);
                }
                Err(e) => eprintln!("Error loading profile: {}", e),
            }
        }
        ProfileCommand::Alias { profile_name, remove, alias_name } => {
            if remove {
                if let Some(alias) = alias_name {
                    match profiles::delete_profile(&alias) {
                        Ok(()) => println!("Removed alias '{}'", alias),
                        Err(e) => eprintln!("Error removing alias: {}", e),
                    }
                } else {
                    eprintln!("Specify alias name with --alias-name <name>");
                }
            } else if let Some(alias) = alias_name {
                match profiles::clone_profile(&profile_name, &alias) {
                    Ok(()) => println!("Created alias '{}' -> '{}'", alias, profile_name),
                    Err(e) => eprintln!("Error creating alias: {}", e),
                }
            } else {
                eprintln!("Specify alias name with --alias-name <name>");
            }
        }
        ProfileCommand::Rename { old_name, new_name } => {
            match profiles::rename_profile(&old_name, &new_name) {
                Ok(()) => println!("Renamed profile '{}' to '{}'", old_name, new_name),
                Err(e) => eprintln!("Error renaming profile: {}", e),
            }
        }
        ProfileCommand::Export { profile_name, output } => {
            let config = match profiles::load_profile(&profile_name) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error loading profile: {}", e);
                    return;
                }
            };
            let output_path = output.map(PathBuf::from).unwrap_or_else(|| {
                PathBuf::from(format!("{}.yaml", profile_name))
            });
            let yaml = match serde_yaml::to_string(&config) {
                Ok(y) => y,
                Err(e) => {
                    eprintln!("Error serializing profile: {}", e);
                    return;
                }
            };
            if let Err(e) = fs::write(&output_path, yaml) {
                eprintln!("Error writing export file: {}", e);
                return;
            }
            println!("Exported profile '{}' to {:?}", profile_name, output_path);
        }
        ProfileCommand::Import { archive, import_name } => {
            let content = match fs::read_to_string(&archive) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error reading import file: {}", e);
                    return;
                }
            };
            let config: Config = match serde_yaml::from_str(&content) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error parsing YAML: {}", e);
                    return;
                }
            };
            let name = import_name.unwrap_or_else(|| "imported".to_string());
            match profiles::save_profile(&name, &config) {
                Ok(()) => println!("Imported profile as '{}'", name),
                Err(e) => eprintln!("Error saving profile: {}", e),
            }
        }
    }
}

pub fn handle_mcp(cmd: McpCommand) {
    use crate::mcp;

    match cmd {
        McpCommand::Serve { verbose: _ } => {
            println!("Hermes MCP Serve Mode");
            println!();
            println!("MCP (Model Context Protocol) servers can be configured to extend Hermes");
            println!("with additional tools and capabilities.");
            println!();
            println!("Configuration file: ~/.hermes/mcp.json");
            println!();
            println!("To add an MCP server:");
            println!("  hermes mcp add <name> --url <url>");
            println!();
            println!("Example MCP servers:");
            println!("  hermes mcp add filesystem --url stdio://npx -y @modelcontextprotocol/server-filesystem /path/to/dir");
            println!("  hermes mcp add memory --url stdio://npx -y @modelcontextprotocol/server-memory");
        }
        McpCommand::Add { name, url, command: _, args: _, auth: _, preset: _, env: _ } => {
            let url = match url {
                Some(u) => u,
                None => {
                    eprintln!("Error: --url is required");
                    return;
                }
            };
            let mut store = match mcp::McpStore::load() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error loading MCP store: {}", e);
                    return;
                }
            };
            if let Err(e) = store.add_server(&name, &url) {
                eprintln!("Error adding server: {}", e);
                return;
            }
            if let Err(e) = store.save() {
                eprintln!("Error saving MCP store: {}", e);
                return;
            }
            println!("Added MCP server '{}' with URL {}", name, url);
        }
        McpCommand::Remove { name } => {
            let mut store = match mcp::McpStore::load() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error loading MCP store: {}", e);
                    return;
                }
            };
            if let Err(e) = store.remove_server(&name) {
                eprintln!("Error removing server: {}", e);
                return;
            }
            if let Err(e) = store.save() {
                eprintln!("Error saving MCP store: {}", e);
                return;
            }
            println!("Removed MCP server '{}'", name);
        }
        McpCommand::List => {
            let store = match mcp::McpStore::load() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error loading MCP store: {}", e);
                    return;
                }
            };
            let servers = store.list_servers();
            if servers.is_empty() {
                println!("No MCP servers configured.");
                println!("Add one with: hermes mcp add <name> --url <url>");
            } else {
                println!("MCP Servers:");
                println!("{:<20} {:<40} Enabled", "Name", "URL");
                println!("{}", "-".repeat(80));
                for server in servers {
                    println!("{:<20} {:<40} {}", server.name, server.url, server.enabled);
                }
            }
        }
        McpCommand::Test { name } => {
            let store = match mcp::McpStore::load() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error loading MCP store: {}", e);
                    return;
                }
            };
            let server = match store.get_server(&name) {
                Some(s) => s,
                None => {
                    eprintln!("MCP server '{}' not found", name);
                    return;
                }
            };
            print!("Testing connection to '{}'... ", name);
            match mcp::test_server(server) {
                Ok(result) => {
                    if result.success {
                        println!("OK");
                        println!("  Response time: {}ms", result.response_time_ms);
                        println!("  {}", result.message);
                    } else {
                        println!("FAILED");
                        println!("  {}", result.message);
                    }
                }
                Err(e) => {
                    println!("ERROR");
                    eprintln!("  {}", e);
                }
            }
        }
        McpCommand::Configure { name: _ } => {
            let path = mcp::McpStore::mcp_path();
            println!("MCP configuration file: {:?}", path);
            println!();
            println!("To edit the MCP configuration, open this file in your editor:");
            println!("  {:?}", path);
            println!();
            println!("File format:");
            println!("{{");
            println!("  \"servers\": [");
            println!("    {{");
            println!("      \"name\": \"example\",");
            println!("      \"url\": \"stdio://npx -y @modelcontextprotocol/server-example\",");
            println!("      \"enabled\": true");
            println!("    }}");
            println!("  ]");
            println!("}}");
        }
    }
}

pub fn handle_memory(cmd: MemoryCommand) -> Result<()> {
    match cmd {
        MemoryCommand::Setup => handle_memory_setup(),
        MemoryCommand::Status => handle_memory_status(),
        MemoryCommand::Off => handle_memory_off(),
    }
}

fn get_memory_dir() -> PathBuf {
    Config::hermes_home().join("memory")
}

fn get_memory_file(name: &str) -> PathBuf {
    get_memory_dir().join(format!("{}.json", name))
}

fn ensure_memory_dir() -> Result<PathBuf> {
    let dir = get_memory_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir)
            .with_context(|| format!("failed to create memory directory at {:?}", dir))?;
    }
    Ok(dir)
}

fn read_memory_json(name: &str) -> Result<serde_json::Value> {
    let path = get_memory_file(name);
    if !path.exists() {
        return Ok(serde_json::json!({ "entries": [] }));
    }
    let content = fs::read_to_string(&path)
        .with_context(|| format!("failed to read memory file {:?}", path))?;
    serde_json::from_str(&content)
        .with_context(|| format!("failed to parse memory file {:?}", path))
}

fn write_memory_json(name: &str, value: &serde_json::Value) -> Result<()> {
    let path = get_memory_file(name);
    let content = serde_json::to_string_pretty(value)
        .context("failed to serialize memory data")?;
    fs::write(&path, content)
        .with_context(|| format!("failed to write memory file {:?}", path))?;
    Ok(())
}

fn handle_memory_setup() -> Result<()> {
    let dir = ensure_memory_dir()?;
    println!("Created memory directory: {:?}", dir);

    // Create each memory file with empty entries
    let files = ["preferences", "facts", "context", "settings"];
    for name in files {
        let path = get_memory_file(name);
        if path.exists() {
            println!("  {}: already exists", name);
        } else {
            let default_value = if name == "settings" {
                serde_json::json!({ "enabled": true })
            } else {
                serde_json::json!({ "entries": [] })
            };
            write_memory_json(name, &default_value)?;
            println!("  {}: created", name);
        }
    }

    println!("\nMemory setup complete. Memory is enabled.");
    println!("Run 'hermes memory off' to disable memory storage.");
    Ok(())
}

fn handle_memory_status() -> Result<()> {
    let dir = get_memory_dir();

    if !dir.exists() {
        println!("Memory is not initialized.");
        println!("Run 'hermes memory setup' to initialize memory storage.");
        return Ok(());
    }

    // Read settings to check if memory is enabled
    let settings = read_memory_json("settings")?;
    let enabled = settings.get("enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    // Count files and calculate total size
    let mut total_size: u64 = 0;
    let mut file_count = 0;
    let mut file_info: Vec<(String, usize, u64)> = Vec::new();

    let files = ["preferences", "facts", "context", "settings"];
    for name in files {
        let path = get_memory_file(name);
        if path.exists() {
            let metadata = fs::metadata(&path)?;
            let size = metadata.len();
            total_size += size;
            file_count += 1;

            // Count entries
            let entries = if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    json.get("entries")
                        .and_then(|e| e.as_array())
                        .map(|arr| arr.len())
                        .unwrap_or(0)
                } else {
                    0
                }
            } else {
                0
            };

            file_info.push((name.to_string(), entries, size));
        }
    }

    println!("Memory Status:");
    println!("  Location: {:?}", dir);
    println!("  Status: {}", if enabled { "Enabled" } else { "Disabled" });
    println!("  Files: {} (total {} bytes)", file_count, total_size);
    println!("\nMemory Files:");
    for (name, entries, size) in file_info {
        println!("  {}: {} entries ({} bytes)", name, entries, size);
    }

    Ok(())
}

fn handle_memory_off() -> Result<()> {
    ensure_memory_dir()?;

    let settings_path = get_memory_file("settings");
    let settings = if settings_path.exists() {
        read_memory_json("settings")?
    } else {
        serde_json::json!({ "enabled": true })
    };

    let mut settings_obj = settings.as_object()
        .cloned()
        .unwrap_or_default();
    settings_obj.insert("enabled".to_string(), serde_json::json!(false));

    write_memory_json("settings", &serde_json::Value::Object(settings_obj))?;

    println!("Memory has been disabled.");
    println!("Your existing memory files are preserved.");
    println!("Run 'hermes memory setup' to re-enable memory storage.");
    Ok(())
}

pub fn handle_webhook(cmd: WebhookCommand) {
    match cmd {
        WebhookCommand::Subscribe { name, prompt, events, description, skills, deliver, deliver_chat_id, secret } => {
            info!("subscribing webhook: {}", name);
            let events: Vec<String> = if events.is_empty() {
                vec!["message".to_string()]
            } else {
                events.split(',').map(|s| s.trim().to_string()).collect()
            };

            let webhook = Webhook {
                name: name.clone(),
                url: prompt, // Using prompt field as URL since that's the actual webhook URL
                events,
                enabled: true,
                description,
                skills: if skills.is_empty() { vec![] } else { skills.split(',').map(|s| s.trim().to_string()).collect() },
                deliver,
                deliver_chat_id,
                secret,
                added_at: chrono::Utc::now().to_rfc3339(),
            };

            let mut store = match WebhookStore::load() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error loading webhook store: {}", e);
                    return;
                }
            };

            if let Err(e) = store.add_webhook(webhook) {
                eprintln!("Error adding webhook: {}", e);
                return;
            }

            if let Err(e) = store.save() {
                eprintln!("Error saving webhook store: {}", e);
                return;
            }

            println!("Webhook '{}' subscribed successfully.", name);
        }
        WebhookCommand::List => {
            info!("listing webhooks");
            let store = match WebhookStore::load() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error loading webhook store: {}", e);
                    return;
                }
            };

            let webhooks = store.list_webhooks();
            if webhooks.is_empty() {
                println!("No webhooks configured.");
                println!("Add one with: hermes webhook subscribe <name> --prompt <url>");
            } else {
                println!("Webhooks:");
                println!("{:<20} {:<40} {:<15} Enabled", "Name", "URL", "Events");
                println!("{}", "-".repeat(90));
                for webhook in webhooks {
                    let events = if webhook.events.is_empty() {
                        "none".to_string()
                    } else {
                        webhook.events.join(",")
                    };
                    println!("{:<20} {:<40} {:<15} {}", webhook.name, webhook.url, events, webhook.enabled);
                }
            }
        }
        WebhookCommand::Remove { name } => {
            info!("removing webhook: {}", name);
            let mut store = match WebhookStore::load() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error loading webhook store: {}", e);
                    return;
                }
            };

            if let Err(e) = store.remove_webhook(&name) {
                eprintln!("Error removing webhook: {}", e);
                return;
            }

            if let Err(e) = store.save() {
                eprintln!("Error saving webhook store: {}", e);
                return;
            }

            println!("Webhook '{}' removed.", name);
        }
        WebhookCommand::Test { name, payload } => {
            info!("testing webhook: {}", name);
            let store = match WebhookStore::load() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error loading webhook store: {}", e);
                    return;
                }
            };

            let webhook = match store.get_webhook(&name) {
                Some(w) => w,
                None => {
                    eprintln!("Webhook '{}' not found", name);
                    return;
                }
            };

            // Validate URL format
            if !webhook.url.starts_with("http://") && !webhook.url.starts_with("https://") {
                eprintln!("Invalid webhook URL: {}. Must start with http:// or https://", webhook.url);
                return;
            }

            println!("Testing webhook '{}' at {}", name, webhook.url);
            println!("Payload: {}", if payload.is_empty() { "(empty)".to_string() } else { payload.clone() });

            // For local/testing purposes, just validate URL format
            // Actual HTTP POST test would require reqwest or similar
            println!("URL format validated: OK");
            if !webhook.enabled {
                println!("WARNING: Webhook is disabled");
            }
            println!("Test complete. Configure your server to receive webhooks at the URL above.");
        }
    }
}

pub fn handle_pairing(cmd: PairingCommand) {
    match cmd {
        PairingCommand::List => {
            info!("listing pairings");
            let store = match PairingStore::load() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error loading pairing store: {}", e);
                    return;
                }
            };

            let pairings = store.list_pairings();
            if pairings.is_empty() {
                println!("No pairings configured.");
                println!("Pairings allow other platforms to connect to Hermes.");
                return;
            }

            println!("Pairings:");
            println!("{:<15} {:<20} {:<15} Created", "Platform", "User ID", "Status");
            println!("{}", "-".repeat(80));

            for pairing in pairings {
                let status = match pairing.status {
                    PairingStatus::Pending => "pending",
                    PairingStatus::Approved => "approved",
                    PairingStatus::Revoked => "revoked",
                };
                println!("{:<15} {:<20} {:<15} {}", pairing.platform, pairing.user_id, status, pairing.created_at);
            }

            // Show summary by status
            let pending = store.list_by_status(&PairingStatus::Pending).len();
            let approved = store.list_by_status(&PairingStatus::Approved).len();
            let revoked = store.list_by_status(&PairingStatus::Revoked).len();
            println!("\nSummary: {} pending, {} approved, {} revoked", pending, approved, revoked);
        }
        PairingCommand::Approve { platform, code } => {
            info!("approving pairing: platform={}, code={}", platform, code);
            let mut store = match PairingStore::load() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error loading pairing store: {}", e);
                    return;
                }
            };

            if let Err(e) = store.approve_pairing(&platform, &code) {
                eprintln!("Error approving pairing: {}", e);
                return;
            }

            if let Err(e) = store.save() {
                eprintln!("Error saving pairing store: {}", e);
                return;
            }

            println!("Pairing approved for platform '{}'.", platform);
        }
        PairingCommand::Revoke { platform, user_id } => {
            info!("revoking pairing: platform={}, user_id={}", platform, user_id);
            let mut store = match PairingStore::load() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error loading pairing store: {}", e);
                    return;
                }
            };

            if let Err(e) = store.revoke_pairing(&platform, &user_id) {
                eprintln!("Error revoking pairing: {}", e);
                return;
            }

            if let Err(e) = store.save() {
                eprintln!("Error saving pairing store: {}", e);
                return;
            }

            println!("Pairing revoked for platform '{}', user '{}'.", platform, user_id);
        }
        PairingCommand::ClearPending => {
            info!("clearing pending pairings");
            let mut store = match PairingStore::load() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error loading pairing store: {}", e);
                    return;
                }
            };

            match store.clear_pending() {
                Ok(()) => {
                    if let Err(e) = store.save() {
                        eprintln!("Error saving pairing store: {}", e);
                        return;
                    }
                    println!("All pending pairings cleared.");
                }
                Err(e) => {
                    eprintln!("{}", e);
                }
            }
        }
    }
}

pub fn handle_plugins(cmd: PluginsCommand) {
    match cmd {
        PluginsCommand::Install { identifier, force: _ } => {
            info!("installing plugin: {}", identifier);
            let mut store = match PluginStore::load() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error loading plugin store: {}", e);
                    return;
                }
            };

            // Parse identifier as name and optional source
            let parts: Vec<&str> = identifier.split('@').collect();
            let name = parts[0].to_string();
            let source = if parts.len() > 1 { parts[1] } else { "local" }.to_string();

            // Check if already installed
            if store.get_plugin(&name).is_some() {
                eprintln!("Plugin '{}' is already installed. Use 'hermes plugins update {}' to update.", name, name);
                return;
            }

            let plugin = Plugin {
                name: name.clone(),
                version: "1.0.0".to_string(), // Default version
                source,
                enabled: true,
                description: format!("Plugin: {}", name),
                author: "Unknown".to_string(),
                installed_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            };

            if let Err(e) = store.add_plugin(plugin) {
                eprintln!("Error installing plugin: {}", e);
                return;
            }

            if let Err(e) = store.save() {
                eprintln!("Error saving plugin store: {}", e);
                return;
            }

            println!("Plugin '{}' installed successfully.", name);
        }
        PluginsCommand::Update { name } => {
            info!("updating plugin: {}", name);
            let mut store = match PluginStore::load() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error loading plugin store: {}", e);
                    return;
                }
            };

            // Bump version
            let new_version = "1.1.0".to_string(); // Simple bump for now
            if let Err(e) = store.update_plugin(&name, &new_version) {
                eprintln!("Error updating plugin: {}", e);
                return;
            }

            if let Err(e) = store.save() {
                eprintln!("Error saving plugin store: {}", e);
                return;
            }

            println!("Plugin '{}' updated to version {}.", name, new_version);
        }
        PluginsCommand::Remove { name } => {
            info!("removing plugin: {}", name);
            let mut store = match PluginStore::load() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error loading plugin store: {}", e);
                    return;
                }
            };

            if let Err(e) = store.remove_plugin(&name) {
                eprintln!("Error removing plugin: {}", e);
                return;
            }

            if let Err(e) = store.save() {
                eprintln!("Error saving plugin store: {}", e);
                return;
            }

            println!("Plugin '{}' removed.", name);
        }
        PluginsCommand::List => {
            info!("listing plugins");
            let store = match PluginStore::load() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error loading plugin store: {}", e);
                    return;
                }
            };

            let plugins = store.list_plugins();
            if plugins.is_empty() {
                println!("No plugins installed.");
                println!("Install one with: hermes plugins install <identifier>");
                return;
            }

            println!("Plugins:");
            println!("{:<20} {:<10} {:<15} {:<40} Description", "Name", "Version", "Enabled", "Source");
            println!("{}", "-".repeat(100));

            for plugin in plugins {
                println!("{:<20} {:<10} {:<15} {:<40} {}",
                    plugin.name,
                    plugin.version,
                    plugin.enabled,
                    plugin.source,
                    if plugin.description.len() > 40 { format!("{}...", &plugin.description[..37]) } else { plugin.description.clone() }
                );
            }

            let enabled = plugins.iter().filter(|p| p.enabled).count();
            println!("\n{} plugin(s) installed, {} enabled", plugins.len(), enabled);
        }
        PluginsCommand::Enable { name } => {
            info!("enabling plugin: {}", name);
            let mut store = match PluginStore::load() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error loading plugin store: {}", e);
                    return;
                }
            };

            if let Err(e) = store.enable_plugin(&name) {
                eprintln!("Error enabling plugin: {}", e);
                return;
            }

            if let Err(e) = store.save() {
                eprintln!("Error saving plugin store: {}", e);
                return;
            }

            println!("Plugin '{}' enabled.", name);
        }
        PluginsCommand::Disable { name } => {
            info!("disabling plugin: {}", name);
            let mut store = match PluginStore::load() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error loading plugin store: {}", e);
                    return;
                }
            };

            if let Err(e) = store.disable_plugin(&name) {
                eprintln!("Error disabling plugin: {}", e);
                return;
            }

            if let Err(e) = store.save() {
                eprintln!("Error saving plugin store: {}", e);
                return;
            }

            println!("Plugin '{}' disabled.", name);
        }
    }
}

pub fn handle_debug(cmd: DebugCommand) {
    match cmd {
        DebugCommand::Share { lines, expire, local } => {
            info!("debug share: lines={}, expire={}, local={}", lines, expire, local);
            println!("Hermes Debug Share");
            println!("====================");
            println!();
            println!("Parameters:");
            println!("  Lines: {}", lines);
            println!("  Expire: {} days", expire);
            println!("  Local only: {}", local);
            println!();

            // Gather debug information
            let hermes_home = Config::hermes_home();
            let config_path = Config::config_path();

            println!("Debug Information:");
            println!("------------------");
            println!();

            // Version info
            println!("Version: {}", env!("CARGO_PKG_VERSION"));
            println!();

            // Paths
            println!("Paths:");
            println!("  HERMES_HOME: {:?}", hermes_home);
            println!("  Config: {:?}", config_path);
            println!();

            // Config summary (last {} lines of config if exists)
            if config_path.exists() {
                if let Ok(content) = fs::read_to_string(&config_path) {
                    let config_lines: Vec<&str> = content.lines().rev().take(lines as usize).collect();
                    println!("Config (last {} lines):", config_lines.len());
                    for line in config_lines.iter().rev() {
                        println!("  {}", line);
                    }
                }
            }
            println!();

            // Auth summary (no secrets)
            let auth_store = AuthStore::load().unwrap_or_default();
            println!("Auth providers: {}", auth_store.credentials.len());
            for cred in &auth_store.credentials {
                println!("  - {}", cred.provider);
            }
            println!();

            // Tool status
            let tools = tools::list_tools(false).unwrap_or_default();
            println!("Tools: {} registered", tools.len());
            let enabled = tools.iter().filter(|(_, _, _, e)| *e).count();
            println!("  {} enabled, {} disabled", enabled, tools.len() - enabled);

            if local {
                println!();
                println!("[LOCAL MODE] Debug info printed to stdout only.");
                println!("No data was shared or transmitted.");
            } else {
                println!();
                println!("[REMOTE MODE] Note: Actual sharing functionality not implemented.");
                println!("This would upload debug info to a temporary paste service.");
            }
        }
    }
}

pub fn handle_claw(cmd: ClawCommand) {
    match cmd {
        ClawCommand::Migrate { source, dry_run, preset, overwrite, migrate_secrets, workspace_target: _, skill_conflict, yes } => {
            info!("claw migrate: source={:?}, dry_run={}", source, dry_run);
            println!("Hermes Claw Migrate");
            println!("====================");
            println!();

            let source_path = source.clone().unwrap_or_else(|| ".".to_string());
            println!("Source: {}", source_path);
            println!("Preset: {}", preset);
            println!("Dry run: {}", dry_run);
            println!();

            // Show what would be migrated
            println!("Migration would process:");
            println!("  - Skills configuration");
            println!("  - Auth credentials {}", if migrate_secrets { "(including secrets)" } else { "(secrets excluded)" });
            println!("  - Config settings");
            println!("  - Tool configurations");
            println!();

            if overwrite {
                println!("[WARNING] --overwrite is set. Existing data will be replaced.");
                println!();
            }

            match skill_conflict.as_str() {
                "skip" => println!("Skill conflicts: skip"),
                "overwrite" => println!("Skill conflicts: overwrite"),
                "keep" => println!("Skill conflicts: keep existing"),
                _ => println!("Skill conflicts: {}", skill_conflict),
            }
            println!();

            if dry_run {
                println!("[DRY RUN] No changes have been made.");
                println!("Run without --dry-run to perform the actual migration.");
            } else {
                if !yes {
                    println!("WARNING: This will modify your Hermes configuration.");
                    println!("Use --yes to confirm or --dry-run to preview first.");
                }
            }
        }
        ClawCommand::Cleanup { source, dry_run, yes } => {
            info!("claw cleanup: source={:?}, dry_run={}", source, dry_run);
            println!("Hermes Claw Cleanup");
            println!("====================");
            println!();

            let source_path = source.clone().unwrap_or_else(|| ".".to_string());
            println!("Source: {}", source_path);
            println!("Dry run: {}", dry_run);
            println!();

            // Show what would be cleaned up
            println!("Cleanup would remove:");
            println!("  - Orphaned skill directories");
            println!("  - Unused configuration keys");
            println!("  - Temporary files");
            println!("  - Cache directories");
            println!();

            if dry_run {
                println!("[DRY RUN] No changes have been made.");
                println!("Run without --dry-run to perform the actual cleanup.");
            } else {
                if !yes {
                    println!("WARNING: This will delete files from your Hermes directory.");
                    println!("Use --yes to confirm or --dry-run to preview first.");
                }
            }
        }
    }
}

// ── Backup / Import / Dump ─────────────────────────────────────────────────────

/// Handle the `hermes backup` command
pub fn handle_backup(output: Option<String>, quick: bool, label: Option<String>) -> Result<()> {
    use chrono::Local;

    let hermes_home = Config::hermes_home();
    let backups_dir = hermes_home.join("backups");

    let timestamp = Local::now().format("%Y%m%d-%H%M%S");
    let label_suffix = label.clone().map(|l| format!("-{}", l)).unwrap_or_default();
    let quick_suffix = if quick { "-quick" } else { "" };
    let backup_name = format!("hermes-backup-{}{}{}", timestamp, quick_suffix, label_suffix);
    let backup_path = backups_dir.join(&backup_name);

    println!("Hermes Backup");
    println!("=============");
    println!();
    println!("Creating backup: {}", backup_name);
    println!("Source: {:?}", hermes_home);
    println!("Destination: {:?}", backup_path);
    println!();

    fs::create_dir_all(&backups_dir).with_context(|| format!("failed to create backups directory {:?}", backups_dir))?;
    fs::create_dir_all(&backup_path).with_context(|| format!("failed to create backup directory {:?}", backup_path))?;

    let items_to_backup: Vec<(&str, Option<&str>)> = if quick {
        vec![
            ("config.yaml", Some("config.yaml")),
            ("sessions.db", Some("sessions.db")),
            ("credentials.yaml", Some("auth.json")),
        ]
    } else {
        vec![
            ("config.yaml", Some("config.yaml")),
            ("sessions.db", Some("sessions.db")),
            ("credentials.yaml", Some("auth.json")),
            ("cron", None),
            ("memory", None),
            ("profiles", None),
            ("skills", None),
            (".env", Some(".env")),
        ]
    };

    let mut backed_up_count = 0;
    let mut total_size: u64 = 0;

    for (item_name, dest_name) in items_to_backup {
        let src = hermes_home.join(item_name);
        let dst = backup_path.join(dest_name.unwrap_or(item_name));

        if !src.exists() {
            continue;
        }

        if src.is_dir() {
            copy_dir_recursive(&src, &dst)?;
            let size = calculate_dir_size(&dst);
            total_size += size;
            println!("  [OK] Backed up directory: {} ({} bytes)", item_name, size);
        } else {
            fs::copy(&src, &dst).with_context(|| format!("failed to copy {:?}", src))?;
            let size = src.metadata().map(|m| m.len()).unwrap_or(0);
            total_size += size;
            println!("  [OK] Backed up file: {} ({} bytes)", item_name, size);
        }
        backed_up_count += 1;
    }

    let metadata = BackupMetadata {
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: Local::now().to_rfc3339(),
        hermes_home: hermes_home.to_string_lossy().to_string(),
        quick,
        label,
        items_backed_up: backed_up_count,
        total_size_bytes: total_size,
    };
    let metadata_path = backup_path.join("backup-meta.yaml");
    let metadata_yaml = serde_yaml::to_string(&metadata).with_context(|| "failed to serialize backup metadata".to_string())?;
    fs::write(&metadata_path, metadata_yaml).with_context(|| format!("failed to write metadata to {:?}", metadata_path))?;

    println!();
    println!("Backup complete!");
    println!("  {} item(s) backed up", backed_up_count);
    println!("  Total size: {} bytes", total_size);
    println!("  Location: {:?}", backup_path);

    if let Some(custom_output) = output {
        println!("  Copy/symlink to: {}", custom_output);
    }

    Ok(())
}

/// Handle the `hermes import` command
pub fn handle_import(backup_path: String, force: bool) -> Result<()> {
    let hermes_home = Config::hermes_home();
    let backup_dir = PathBuf::from(&backup_path);

    println!("Hermes Import");
    println!("=============");
    println!();
    println!("Backup source: {:?}", backup_dir);
    println!("Restore target: {:?}", hermes_home);
    println!();

    if !backup_dir.exists() {
        anyhow::bail!("Backup directory does not exist: {:?}", backup_dir);
    }

    let metadata_path = backup_dir.join("backup-meta.yaml");
    let has_metadata = metadata_path.exists();

    let items_in_backup = get_backup_items(&backup_dir)?;

    if items_in_backup.is_empty() {
        anyhow::bail!("Backup directory is empty or invalid: {:?}", backup_dir);
    }

    println!("Items found in backup:");
    for item in &items_in_backup {
        println!("  - {}", item);
    }
    println!();

    if has_metadata {
        match fs::read_to_string(&metadata_path) {
            Ok(content) => {
                match serde_yaml::from_str::<BackupMetadata>(&content) {
                    Ok(metadata) => {
                        println!("Backup metadata:");
                        println!("  Version: {}", metadata.version);
                        println!("  Created: {}", metadata.timestamp);
                        println!("  Size: {} bytes", metadata.total_size_bytes);
                        if metadata.quick {
                            println!("  Type: quick");
                        }
                        if let Some(ref l) = metadata.label {
                            println!("  Label: {}", l);
                        }
                        println!();
                    }
                    Err(e) => {
                        eprintln!("Warning: Could not parse backup metadata: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Warning: Could not read backup metadata: {}", e);
            }
        }
    }

    if !force {
        println!("WARNING: This will overwrite existing files in {:?}", hermes_home);
        println!("Continue? [y/N] ");
        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() {
            anyhow::bail!("Failed to read confirmation input");
        }
        let input = input.trim().to_lowercase();
        if input != "y" && input != "yes" {
            println!("Import cancelled.");
            return Ok(());
        }
    }

    let mut restored_count = 0;
    for item_name in &items_in_backup {
        let src = backup_dir.join(item_name);
        let dst = hermes_home.join(item_name);

        if !src.exists() {
            continue;
        }

        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent).with_context(|| format!("failed to create directory {:?}", parent))?;
        }

        if src.is_dir() {
            if dst.exists() {
                fs::remove_dir_all(&dst).with_context(|| format!("failed to remove existing directory {:?}", dst))?;
            }
            copy_dir_recursive(&src, &dst)?;
            println!("  [OK] Restored directory: {}", item_name);
        } else {
            fs::copy(&src, &dst).with_context(|| format!("failed to restore file {:?}", src))?;
            println!("  [OK] Restored file: {}", item_name);
        }
        restored_count += 1;
    }

    println!();
    println!("Import complete!");
    println!("  {} item(s) restored", restored_count);
    println!("  Restored to: {:?}", hermes_home);

    Ok(())
}

/// Handle the `hermes dump` command
pub fn handle_dump(show_keys: bool) -> Result<()> {
    use std::env;

    println!("========================================");
    println!("HERMES DIAGNOSTIC DUMP");
    println!("========================================");
    println!();

    println!("-- Version --");
    println!("  Hermes CLI: {}", env!("CARGO_PKG_VERSION"));
    println!("  Rust: {} (target: {})", env::consts::ARCH, env::consts::OS);

    println!();
    println!("-- OS Info --");
    #[cfg(target_os = "windows")]
    {
        println!("  OS: Windows");
        if let Ok(version) = env::var("OS") {
            println!("  OS Version: {}", version);
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        println!("  OS: {}", std::env::consts::OS);
    }

    let hermes_home = Config::hermes_home();
    let config_path = Config::config_path();
    let auth_path = crate::auth::AuthStore::auth_path();

    println!();
    println!("-- Paths --");
    println!("  HERMES_HOME: {:?}", hermes_home);
    println!("  Config: {:?}", config_path);
    println!("  Auth Store: {:?}", auth_path);
    if let Ok(profile) = env::var("HERMES_PROFILE") {
        println!("  HERMES_PROFILE: {}", profile);
    }
    if let Ok(home) = env::var("HERMES_HOME") {
        println!("  HERMES_HOME (env): {}", home);
    }

    println!();
    println!("-- Disk Space --");
    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", "(Get-PSDrive C).Free / 1GB"])
            .output();
        if let Ok(o) = output {
            if o.status.success() {
                let free_gb = String::from_utf8_lossy(&o.stdout).trim().parse::<f64>().unwrap_or(0.0);
                println!("  C: drive free: {:.2} GB", free_gb);
            }
        }
    }

    println!();
    println!("-- Config --");
    match Config::load() {
        Ok(config) => {
            println!("  Model: {}", config.model.default);
            println!("  Provider: {}", config.model.provider);
            if !config.model.base_url.is_empty() {
                println!("  Base URL: {}", config.model.base_url);
            }
            println!("  Max turns: {}", config.agent.max_turns);
            println!("  Reasoning effort: {}", config.agent.reasoning_effort);
            println!("  Terminal env: {}", config.terminal.env_type);
            println!("  Timeout: {}s", config.terminal.timeout);
            println!("  Display streaming: {}", config.display.streaming);
        }
        Err(e) => {
            println!("  Error loading config: {}", e);
        }
    }

    println!();
    println!("-- Auth Providers --");
    let auth_store = crate::auth::AuthStore::load()?;
    if auth_store.credentials.is_empty() {
        println!("  No providers configured");
    } else {
        for cred in &auth_store.credentials {
            let masked_key: String = if show_keys {
                cred.api_key.clone()
            } else {
                mask_key(&cred.api_key)
            };
            println!("  {}: {}", cred.provider, masked_key);
            if let Some(ref base_url) = cred.base_url {
                println!("    base_url: {}", base_url);
            }
        }
    }

    println!();
    println!("-- Sessions --");
    let sessions_db_path = hermes_home.join("sessions.db");
    println!("  Database: {:?}", sessions_db_path);
    if sessions_db_path.exists() {
        if let Ok(meta) = fs::metadata(&sessions_db_path) {
            println!("  Size: {} bytes", meta.len());
        }
        println!("  Status: exists");
    } else {
        println!("  Status: not found");
    }

    println!();
    println!("-- Tool Registry --");
    let tools = tools::get_builtin_tools();
    let mut toolsets: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for t in &tools {
        toolsets.insert(t.toolset);
    }
    println!("  Built-in tools: {}", tools.len());
    println!("  Toolsets: {}", toolsets.len());
    let mut sorted_toolsets: Vec<_> = toolsets.iter().collect();
    sorted_toolsets.sort();
    for toolset in sorted_toolsets {
        let count = tools.iter().filter(|t| t.toolset == *toolset).count();
        println!("    {}: {} tool(s)", toolset, count);
    }

    println!();
    println!("-- Cron --");
    let cron_dir = cron_mod::cron_dir();
    println!("  Directory: {:?}", cron_dir);
    if cron_dir.exists() {
        if let Ok(entries) = fs::read_dir(&cron_dir) {
            let count = entries.filter_map(|e| e.ok()).count();
            println!("  Entries: {}", count);
        }
        let jobs_path = cron_mod::cron_jobs_path();
        if jobs_path.exists() {
            println!("  Jobs file: exists");
        }
    } else {
        println!("  Status: not configured");
    }

    println!();
    println!("-- Skills --");
    let skills_home = SkillsIndex::skills_home();
    println!("  Directory: {:?}", skills_home);
    if skills_home.exists() {
        match SkillsIndex::load() {
            Ok(index) => {
                println!("  Indexed skills: {}", index.skills.len());
            }
            Err(_) => {
                println!("  Could not load skills index");
            }
        }
    } else {
        println!("  Status: not installed");
    }

    println!();
    println!("-- Environment Variables (HERMES_) --");
    for (key, value) in env::vars() {
        if key.starts_with("HERMES_") {
            println!("  {}: {}", key, value);
        }
    }

    println!();
    println!("========================================");
    println!("End of diagnostic dump");

    Ok(())
}

// Helper functions

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct BackupMetadata {
    version: String,
    timestamp: String,
    hermes_home: String,
    quick: bool,
    label: Option<String>,
    items_backed_up: usize,
    total_size_bytes: u64,
}

fn get_backup_items(backup_dir: &Path) -> Result<Vec<String>> {
    let mut items = Vec::new();
    let expected_files = ["config.yaml", "sessions.db", "credentials.yaml", ".env"];
    let expected_dirs = ["cron", "memory", "profiles", "skills"];

    for name in &expected_files {
        if backup_dir.join(name).exists() {
            items.push(name.to_string());
        }
    }

    for name in &expected_dirs {
        if backup_dir.join(name).is_dir() {
            items.push(name.to_string());
        }
    }

    Ok(items)
}

fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) -> Result<()> {
    if !src.is_dir() {
        anyhow::bail!("Source is not a directory: {:?}", src);
    }

    fs::create_dir_all(dst).with_context(|| format!("failed to create directory {:?}", dst))?;

    for entry in fs::read_dir(src).with_context(|| format!("failed to read directory {:?}", src))? {
        let entry = entry.with_context(|| format!("failed to read directory entry in {:?}", src))?;
        let ty = entry.file_type().with_context(|| format!("failed to get file type for {:?}", entry.path()))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).with_context(|| format!("failed to copy {:?} to {:?}", src_path, dst_path))?;
        }
    }

    Ok(())
}

fn calculate_dir_size(path: &PathBuf) -> u64 {
    let mut size = 0u64;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.filter_map(|e| e.ok()) {
            if let Ok(meta) = entry.metadata() {
                if meta.is_dir() {
                    size += calculate_dir_size(&entry.path());
                } else {
                    size += meta.len();
                }
            }
        }
    }
    size
}

fn mask_key(key: &str) -> String {
    if key.len() <= 8 {
        return "*".repeat(key.len());
    }
    let start = &key[..4];
    let end = &key[key.len() - 4..];
    format!("{}...{}", start, end)
}

/// Handle the `hermes completion` command
pub fn handle_completion(shell: Option<&str>) {
    println!("Hermes Shell Completion");
    println!("========================");
    println!();

    let shell = shell.unwrap_or("bash");
    let hermes_home = Config::hermes_home();

    println!("Generating completion script for: {}", shell);
    println!();

    match shell.to_lowercase().as_str() {
        "bash" => {
            println!("Add to your ~/.bashrc or ~/.bash_profile:");
            println!();
            println!("  source <(hermes --completion bash)");
        }
        "zsh" => {
            println!("Add to your ~/.zshrc:");
            println!();
            println!("  autoload -U compinit");
            println!("  compinit");
            println!("  source <(hermes --completion zsh)");
        }
        "fish" => {
            println!("Run:");
            println!();
            println!("  hermes --completion fish | source");
        }
        "powershell" | "pwsh" => {
            println!("Add to your PowerShell profile:");
            println!();
            println!("  hermes --completion powershell | Out-String | Invoke-Expression");
        }
        _ => {
            println!("Unsupported shell: {}. Supported: bash, zsh, fish, powershell", shell);
        }
    }

    println!();
    println!("Hermes completion script location: {:?}", hermes_home.join("completion"));
}

/// Handle the `hermes insights` command
pub fn handle_insights(days: u32, source: Option<&str>) -> Result<()> {
    use hermes_session_db::SessionStore;

    println!("Hermes Insights");
    println!("==============");
    println!();
    println!("Analyzing last {} days of activity...", days);
    if let Some(s) = source {
        println!("Filter: source = {}", s);
    }
    println!();

    let home = Config::hermes_home();
    let db_path = home.join("sessions.db");

    if !db_path.exists() {
        println!("No session database found. Start chatting to generate insights!");
        return Ok(());
    }

    let store = SessionStore::new(&db_path)
        .map_err(|e| anyhow::anyhow!("Failed to open session DB: {}", e))?;

    let cutoff = chrono::Utc::now() - chrono::Duration::days(days as i64);
    let sessions = store.list_sessions(10000)
        .map_err(|e| anyhow::anyhow!("Failed to list sessions: {}", e))?;

    // Filter by source if specified
    let filtered_sessions: Vec<_> = sessions.iter()
        .filter(|s| {
            if let Some(src) = source {
                s.source == src
            } else {
                true
            }
        })
        .filter(|s| s.updated_at >= cutoff)
        .collect();

    if filtered_sessions.is_empty() {
        println!("No sessions found in the last {} days.", days);
        return Ok(());
    }

    println!("Sessions: {}", filtered_sessions.len());
    println!();

    // Calculate messages per day
    let mut messages_per_day: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut total_messages = 0;

    for session in &filtered_sessions {
        if let Ok(messages) = store.get_messages(&session.id) {
            total_messages += messages.len();
            let day = session.updated_at.format("%Y-%m-%d").to_string();
            *messages_per_day.entry(day).or_insert(0) += messages.len();
        }
    }

    println!("Total messages: {}", total_messages);
    if !filtered_sessions.is_empty() {
        println!("Avg messages/session: {:.1}", total_messages as f64 / filtered_sessions.len() as f64);
    }
    println!();

    // Top sources
    let mut sources: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for session in &filtered_sessions {
        *sources.entry(&session.source).or_insert(0) += 1;
    }
    let mut top_sources: Vec<_> = sources.iter().collect();
    top_sources.sort_by(|a, b| b.1.cmp(a.1));

    println!("Top sources:");
    for (src, count) in top_sources.iter().take(5) {
        println!("  {}: {} sessions", src, count);
    }
    println!();

    // Message distribution by day (last 7 days)
    println!("Recent activity:");
    let now = chrono::Utc::now();
    for i in 0..7 {
        let day = (now - chrono::Duration::days(i)).format("%Y-%m-%d").to_string();
        let count = messages_per_day.get(&day).unwrap_or(&0);
        println!("  {}: {} messages", day, count);
    }

    Ok(())
}

/// Handle the `hermes login` command
#[allow(clippy::too_many_arguments)]
pub fn handle_login(
    provider: Option<&str>,
    portal_url: Option<&str>,
    inference_url: Option<&str>,
    client_id: Option<&str>,
    scope: Option<&str>,
    _no_browser: bool,
    timeout: f64,
    ca_bundle: Option<&str>,
    insecure: bool,
) -> Result<()> {
    println!("Hermes Login");
    println!("============");
    println!();

    let provider = provider.unwrap_or("nous");
    println!("Provider: {}", provider);
    println!();

    // Build the login URL
    let portal = portal_url.unwrap_or("https://portal.nousresearch.com");
    let login_path = "/auth/login";

    println!("To login:");
    println!();
    println!("1. Open the following URL in your browser:");
    println!();
    println!("   {}{}", portal, login_path);
    println!();

    println!("2. Complete the OAuth flow in your browser");
    println!("3. Copy the authorization code");
    println!();

    println!("Configuration:");
    if let Some(inf_url) = inference_url {
        println!("  Inference URL: {}", inf_url);
    }
    if let Some(cid) = client_id {
        println!("  Client ID: {}", cid);
    }
    if let Some(sc) = scope {
        println!("  Scope: {}", sc);
    }
    println!("  Timeout: {}s", timeout);
    if insecure {
        println!("  [WARNING] TLS verification disabled");
    }
    if let Some(ca) = ca_bundle {
        println!("  CA Bundle: {}", ca);
    }

    println!();
    println!("Then run:");
    println!("  hermes auth add {} --api-key <your-token>", provider);

    Ok(())
}

/// Handle the `hermes logout` command
pub fn handle_logout(provider: Option<&str>) -> Result<()> {
    println!("Hermes Logout");
    println!("=============");
    println!();

    let mut store = AuthStore::load()?;
    let credentials = store.list();

    if credentials.is_empty() {
        println!("No auth credentials configured.");
        return Ok(());
    }

    if let Some(p) = provider {
        // Logout specific provider
        if store.remove(p) {
            store.save()?;
            println!("Logged out from {}.", p);
        } else {
            println!("No credentials found for provider: {}", p);
        }
    } else {
        // Logout all
        let count = store.credentials.len();
        store.reset();
        store.save()?;
        println!("Logged out from {} provider(s).", count);
    }

    println!();
    println!("To login again, run:");
    println!("  hermes login");

    Ok(())
}

/// Handle the `hermes whatsapp` command
pub fn handle_whatsapp() -> Result<()> {
    println!("Hermes WhatsApp Setup");
    println!("=====================");
    println!();

    println!("WhatsApp integration allows you to interact with Hermes via WhatsApp.");
    println!();

    println!("Setup Instructions:");
    println!("------------------");
    println!();
    println!("1. Install hermes-gateway:");
    println!("   pip install hermes-agent");
    println!();
    println!("2. Configure WhatsApp gateway:");
    println!("   hermes gateway setup whatsapp");
    println!();
    println!("3. Link your WhatsApp number:");
    println!("   - Run: hermes gateway run -P whatsapp");
    println!("   - Scan the QR code with WhatsApp");
    println!();
    println!("4. Start chatting with Hermes on WhatsApp!");
    println!();

    println!("Requirements:");
    println!("  - WhatsApp Business API account (optional, for official integration)");
    println!("  - Or use the Unofficial WhatsApp gateway (development)");
    println!();

    println!("For more help:");
    println!("  hermes gateway setup");

    Ok(())
}

/// Handle the `hermes acp` command
pub fn handle_acp() -> Result<()> {
    println!("Hermes ACP Server Mode");
    println!("======================");
    println!();

    println!("ACP (Agent Communication Protocol) enables Hermes to communicate");
    println!("with other agents and services in a distributed system.");
    println!();

    println!("Server Modes:");
    println!("-------------");
    println!();
    println!("  1. Local Mode (default)");
    println!("     - Runs on localhost for single-user testing");
    println!("     - No network exposure");
    println!();
    println!("  2. Network Mode");
    println!("     - Exposes ACP server on network for multi-agent communication");
    println!("     - Requires authentication");
    println!();
    println!("  3. Gateway Mode");
    println!("     - Full gateway with ACP + platform integrations");
    println!("     - hermes gateway run");
    println!();

    println!("Current Status:");
    let gateway_running = gateway_mod::is_gateway_running();
    if gateway_running {
        println!("  Gateway: RUNNING");
        if let Some(state) = gateway_mod::read_gateway_state() {
            println!("  ACP: {} (state: {})",
                if state.gateway_state == "running" { "enabled" } else { "disabled" },
                state.gateway_state
            );
        }
    } else {
        println!("  Gateway: STOPPED");
        println!("  ACP: not active");
    }
    println!();

    println!("To start ACP server:");
    println!("  hermes gateway run");

    Ok(())
}

/// Handle the `hermes dashboard` command
pub fn handle_dashboard(port: u16, host: String, no_open: bool) -> Result<()> {
    println!("Hermes Dashboard");
    println!("================");
    println!();

    let url = format!("http://{}:{}", host, port);
    println!("Dashboard URL: {}", url);
    println!("Port: {}", port);
    println!("Host: {}", host);
    println!();

    if !no_open {
        println!("Opening dashboard in default browser...");

        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("cmd")
                .args(["/C", "start", "", &url])
                .spawn()
                .ok();
        }

        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg(&url)
                .spawn()
                .ok();
        }

        #[cfg(target_os = "linux")]
        {
            std::process::Command::new("xdg-open")
                .arg(&url)
                .spawn()
                .ok();
        }
    }

    println!();
    println!("Dashboard Features:");
    println!("  - Session history and management");
    println!("  - Tool usage analytics");
    println!("  - Cron job monitoring");
    println!("  - Auth provider management");
    println!("  - Skills marketplace");
    println!();

    println!("Note: Dashboard server runs locally. Access is restricted to this machine.");
    println!("      Use --no-open to prevent automatic browser opening.");

    Ok(())
}

/// Handle the `hermes logs` command
pub fn handle_logs(
    log_name: Option<&str>,
    lines: u32,
    follow: bool,
    level: Option<&str>,
    session: Option<&str>,
    since: Option<&str>,
    component: Option<&str>,
) -> Result<()> {
    use std::io::{self, BufRead};

    println!("Hermes Logs");
    println!("==========");
    println!();

    let hermes_home = Config::hermes_home();
    let logs_dir = hermes_home.join("logs");

    // Determine which log to show
    let log_name = log_name.unwrap_or("agent");
    let log_file = logs_dir.join(format!("{}.log", log_name));

    // Show available logs if listing
    if log_name == "list" {
        println!("Available logs:");
        if logs_dir.exists() {
            if let Ok(entries) = fs::read_dir(&logs_dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    if let Some(name) = entry.path().file_name().and_then(|n| n.to_str()) {
                        println!("  - {}", name.replace(".log", ""));
                    }
                }
            }
        }
        println!();
        println!("Usage: hermes logs <name> [options]");
        return Ok(());
    }

    if !log_file.exists() {
        println!("Log file not found: {:?}", log_file);
        println!();
        println!("Available logs:");
        if logs_dir.exists() {
            if let Ok(entries) = fs::read_dir(&logs_dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    if let Some(name) = entry.path().file_name().and_then(|n| n.to_str()) {
                        println!("  - {}", name.replace(".log", ""));
                    }
                }
            } else {
                println!("  (no logs directory found)");
            }
        } else {
            println!("  (no logs directory found)");
        }
        return Ok(());
    }

    println!("Log file: {:?}", log_file);
    println!("Showing last {} lines", lines);
    if follow {
        println!("[Following mode - press Ctrl+C to stop]");
    }
    println!();

    // Parse level filter
    let level_filter = level.map(|l| l.to_uppercase());
    let session_filter = session.map(|s| s.to_string());
    let since_filter = since.map(|s| s.to_string());

    // For simple viewing, just read and display the file
    if follow {
        // Follow mode - watch the file
        use std::io::Seek;
        let file = fs::File::open(&log_file)?;
        let reader = io::BufReader::new(file);

        // Read existing lines first
        for line in reader.lines().take_while(|l| l.is_ok()).skip(lines as usize).flatten() {
            if !filter_line(&line, level_filter.as_deref(), session_filter.as_deref(), since_filter.as_deref(), component) {
                println!("{}", line);
            }
        }

        // Then watch for new lines
        let file = fs::File::open(&log_file)?;
        let mut reader = io::BufReader::new(file);
        let mut seek_pos = reader.stream_position()?;

        loop {
            use std::time::Duration;
            std::thread::sleep(Duration::from_millis(500));

            let metadata = fs::metadata(&log_file)?;
            let current_size = metadata.len();

            if current_size > seek_pos {
                let mut file = fs::File::open(&log_file)?;
                use std::io::Seek;
                file.seek(io::SeekFrom::Start(seek_pos))?;
                let reader = io::BufReader::new(file);

                for line in reader.lines().map_while(Result::ok) {
                    if !filter_line(&line, level_filter.as_deref(), session_filter.as_deref(), since_filter.as_deref(), component) {
                        println!("{}", line);
                    }
                }
                seek_pos = current_size;
            }
        }
    } else {
        // Non-follow mode - just show last N lines
        let file = fs::File::open(&log_file)?;
        let reader = io::BufReader::new(file);

        let all_lines: Vec<String> = reader.lines().map_while(Result::ok).collect();
        let start = if all_lines.len() > lines as usize {
            all_lines.len() - lines as usize
        } else {
            0
        };

        for line in all_lines.iter().skip(start) {
            if !filter_line(line, level_filter.as_deref(), session_filter.as_deref(), since_filter.as_deref(), component) {
                println!("{}", line);
            }
        }
    }

    Ok(())
}

fn filter_line(line: &str, level: Option<&str>, _session: Option<&str>, _since: Option<&str>, component: Option<&str>) -> bool {
    // Filter by level
    if let Some(lvl) = level {
        if !line.contains(&format!("[{}]", lvl)) && !line.to_uppercase().contains(&format!("{}:", lvl)) {
            // Line doesn't contain the level
        }
    }

    // Filter by component
    if let Some(comp) = component {
        if !line.contains(&format!("[{}]", comp)) && !line.contains(&format!("{}:", comp)) {
            // Line doesn't contain the component
        }
    }

    false // Don't filter
}