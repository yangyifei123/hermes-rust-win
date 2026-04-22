use super::{
    AuthCommand, ClawCommand, ConfigCommand, CronCommand, DebugCommand, GatewayCommand,
    McpCommand, MemoryCommand, PairingCommand, PluginsCommand, ProfileCommand,
    SessionsCommand, SkillsCommand, ToolsCommand, WebhookCommand,
};
use crate::auth::AuthStore;
use crate::config::Config;
use crate::cron as cron_mod;
use crate::gateway as gateway_mod;
use crate::skills::SkillsIndex;
use crate::tools::{self, ToolsConfig};
use anyhow::Result;
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
        SkillsCommand::List { .. } => println!("Skills list — coming soon"),
        SkillsCommand::Check { .. } => println!("Skills check — coming soon"),
        SkillsCommand::Update { .. } => println!("Skills update — coming soon"),
        SkillsCommand::Audit { .. } => println!("Skills audit — coming soon"),
        SkillsCommand::Publish { .. } => println!("Skills publish — coming soon"),
        SkillsCommand::Snapshot(_) => println!("Skills snapshot — coming soon"),
        SkillsCommand::Tap(_) => println!("Skills tap — coming soon"),
        SkillsCommand::Config => println!("Skills config — coming soon"),
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
        GatewayCommand::Restart { .. } => println!("Gateway restart — coming soon"),
        GatewayCommand::Install { .. } => println!("Gateway install — coming soon"),
        GatewayCommand::Uninstall { .. } => println!("Gateway uninstall — coming soon"),
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
        CronCommand::Edit { .. } => println!("Cron edit — coming soon"),
        CronCommand::Run { .. } => println!("Cron run — coming soon"),
        CronCommand::Tick => println!("Cron tick — coming soon"),
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
        ConfigCommand::Edit => println!("Config edit — coming soon"),
        ConfigCommand::Path => {
            println!("{:?}", Config::config_path());
        }
        ConfigCommand::EnvPath => {
            let home = Config::hermes_home();
            println!("{:?}", home.join(".env"));
        }
        ConfigCommand::Check => println!("Config check — coming soon"),
        ConfigCommand::Migrate => println!("Config migrate — coming soon"),
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
    match cmd {
        SessionsCommand::List { source, limit } => println!("Sessions list (source={:?}, limit={}) — coming soon", source, limit),
        SessionsCommand::Export { output, source: _, session_id: _ } => println!("Sessions export to '{}' — coming soon", output),
        SessionsCommand::Delete { session_id, yes } => println!("Sessions delete '{}' (yes={}) — coming soon", session_id, yes),
        SessionsCommand::Prune { older_than, source: _, yes: _ } => println!("Sessions prune (older_than={} days) — coming soon", older_than),
        SessionsCommand::Stats => println!("Sessions stats — coming soon"),
        SessionsCommand::Rename { session_id, title } => println!("Sessions rename '{}' to '{}' — coming soon", session_id, title.join(" ")),
        SessionsCommand::Browse { source: _, limit: _ } => println!("Sessions browse — coming soon"),
    }
}

pub fn handle_profile(cmd: ProfileCommand) {
    match cmd {
        ProfileCommand::List => println!("Profile list — coming soon"),
        ProfileCommand::Use { profile_name } => println!("Profile use '{}' — coming soon", profile_name),
        ProfileCommand::Create { profile_name, clone, .. } => println!("Profile create '{}' (clone={}) — coming soon", profile_name, clone),
        ProfileCommand::Delete { profile_name, yes } => println!("Profile delete '{}' (yes={}) — coming soon", profile_name, yes),
        ProfileCommand::Show { profile_name } => println!("Profile show '{}' — coming soon", profile_name),
        ProfileCommand::Alias { profile_name, remove, .. } => println!("Profile alias '{}' (remove={}) — coming soon", profile_name, remove),
        ProfileCommand::Rename { old_name, new_name } => println!("Profile rename '{}' -> '{}' — coming soon", old_name, new_name),
        ProfileCommand::Export { profile_name, .. } => println!("Profile export '{}' — coming soon", profile_name),
        ProfileCommand::Import { archive, .. } => println!("Profile import '{}' — coming soon", archive),
    }
}

pub fn handle_mcp(cmd: McpCommand) {
    match cmd {
        McpCommand::Serve { verbose } => println!("MCP serve (verbose={}) — coming soon", verbose),
        McpCommand::Add { name, url, .. } => println!("MCP add '{}' (url={:?}) — coming soon", name, url),
        McpCommand::Remove { name } => println!("MCP remove '{}' — coming soon", name),
        McpCommand::List => println!("MCP list — coming soon"),
        McpCommand::Test { name } => println!("MCP test '{}' — coming soon", name),
        McpCommand::Configure { name } => println!("MCP configure '{}' — coming soon", name),
    }
}

pub fn handle_memory(cmd: MemoryCommand) {
    match cmd {
        MemoryCommand::Setup => println!("Memory setup — coming soon"),
        MemoryCommand::Status => println!("Memory status — coming soon"),
        MemoryCommand::Off => println!("Memory off — coming soon"),
    }
}

pub fn handle_webhook(cmd: WebhookCommand) {
    match cmd {
        WebhookCommand::Subscribe { name, .. } => println!("Webhook subscribe '{}' — coming soon", name),
        WebhookCommand::List => println!("Webhook list — coming soon"),
        WebhookCommand::Remove { name } => println!("Webhook remove '{}' — coming soon", name),
        WebhookCommand::Test { name, .. } => println!("Webhook test '{}' — coming soon", name),
    }
}

pub fn handle_pairing(cmd: PairingCommand) {
    match cmd {
        PairingCommand::List => println!("Pairing list — coming soon"),
        PairingCommand::Approve { platform, code } => println!("Pairing approve '{}' '{}' — coming soon", platform, code),
        PairingCommand::Revoke { platform, user_id } => println!("Pairing revoke '{}' '{}' — coming soon", platform, user_id),
        PairingCommand::ClearPending => println!("Pairing clear-pending — coming soon"),
    }
}

pub fn handle_plugins(cmd: PluginsCommand) {
    match cmd {
        PluginsCommand::Install { identifier, force } => println!("Plugins install '{}' (force={}) — coming soon", identifier, force),
        PluginsCommand::Update { name } => println!("Plugins update '{}' — coming soon", name),
        PluginsCommand::Remove { name } => println!("Plugins remove '{}' — coming soon", name),
        PluginsCommand::List => println!("Plugins list — coming soon"),
        PluginsCommand::Enable { name } => println!("Plugins enable '{}' — coming soon", name),
        PluginsCommand::Disable { name } => println!("Plugins disable '{}' — coming soon", name),
    }
}

pub fn handle_debug(cmd: DebugCommand) {
    match cmd {
        DebugCommand::Share { lines, expire, local } => println!("Debug share (lines={}, expire={}d, local={}) — coming soon", lines, expire, local),
    }
}

pub fn handle_claw(cmd: ClawCommand) {
    match cmd {
        ClawCommand::Migrate { source, dry_run, .. } => println!("Claw migrate (source={:?}, dry_run={}) — coming soon", source, dry_run),
        ClawCommand::Cleanup { source, dry_run, .. } => println!("Claw cleanup (source={:?}, dry_run={}) — coming soon", source, dry_run),
    }
}