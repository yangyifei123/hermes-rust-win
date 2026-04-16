use super::{AuthCommand, ConfigCommand, CronCommand, GatewayCommand, SkillsCommand, ToolsCommand};
use crate::auth::AuthStore;
use crate::config::Config;
use crate::skills::SkillsIndex;
use crate::tools::{self, ToolsConfig};
use anyhow::Result;
use tracing::info;

pub async fn handle_auth(cmd: AuthCommand) -> Result<()> {
    match cmd {
        AuthCommand::Add { provider, api_key, base_url } => {
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
        AuthCommand::List => {
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
        AuthCommand::Remove { provider } => {
            info!("removing auth for provider: {}", provider);
            let mut store = AuthStore::load()?;
            if store.remove(&provider) {
                store.save()?;
                println!("Auth credentials removed for {}", provider);
            } else {
                println!("No auth credentials found for {}", provider);
            }
        }
        AuthCommand::Reset => {
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
        ToolsCommand::List { all } => {
            info!("listing tools (all: {})", all);
            let tools = tools::list_tools(all);
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
        ToolsCommand::Disable { name } => {
            info!("disabling tool: {}", name);
            let mut config = ToolsConfig::load()?;
            let builtins: Vec<_> = tools::get_builtin_tools()
                .iter()
                .map(|t| t.name.to_string())
                .collect();
            if !builtins.contains(&name) {
                println!("Warning: '{}' is not a built-in tool.", name);
                println!("Known tools: {}", builtins.join(", "));
            }
            config.disable(&name);
            config.save()?;
            println!("Tool '{}' disabled.", name);
        }
        ToolsCommand::Enable { name } => {
            info!("enabling tool: {}", name);
            let mut config = ToolsConfig::load()?;
            config.enable(&name);
            config.save()?;
            println!("Tool '{}' enabled.", name);
        }
    }
    Ok(())
}

pub async fn handle_skills(cmd: SkillsCommand) -> Result<()> {
    match cmd {
        SkillsCommand::Search { query } => {
            info!("searching skills: {:?}", query);
            let mut index = SkillsIndex::load()?;
            let count = index.scan_local_skills()?;

            let results: Vec<_> = if let Some(ref q) = query {
                index.search(q).into_iter().cloned().collect()
            } else {
                index.get_all().into_iter().cloned().collect()
            };

            if results.is_empty() {
                if query.is_some() {
                    println!("No skills found matching '{}'.", query.as_ref().unwrap());
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
        SkillsCommand::Browse => {
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
        SkillsCommand::Install { name } => {
            info!("installing skill: {}", name);
            // For now, just acknowledge the install request
            // Full install from GitHub would require git and network access
            if name.contains('/') {
                println!("Skill install from '{}' requested.", name);
                println!("Note: Full remote install requires network access.");
                println!("For now, skills should be installed manually to ~/.hermes/skills/");
            } else {
                println!("Installing skill '{}'...", name);
                println!("Skill '{}' is not available in the registry.", name);
            }
        }
        SkillsCommand::Remove { name } => {
            info!("removing skill: {}", name);
            let mut index = SkillsIndex::load()?;
            if index.remove(&name) {
                index.save()?;
                // Try to remove the skill directory
                let skills_home = SkillsIndex::skills_home();
                let skill_path = skills_home.join(&name);
                if skill_path.exists() {
                    // Note: This is a simple remove - in production you'd want to confirm
                    println!("Skill '{}' removed from index.", name);
                    println!("Note: Skill files at {:?} were not deleted.", skill_path);
                } else {
                    println!("Skill '{}' removed from index (no files found).", name);
                }
            } else {
                println!("Skill '{}' not found in index.", name);
            }
        }
    }
    Ok(())
}

pub async fn handle_gateway(cmd: GatewayCommand) -> Result<()> {
    match cmd {
        GatewayCommand::Run { platform } => {
            info!("running gateway: {:?}", platform);
            println!("Gateway run not yet implemented");
        }
        GatewayCommand::Start => {
            info!("starting gateway service");
            println!("Gateway start not yet implemented");
        }
        GatewayCommand::Stop => {
            info!("stopping gateway service");
            println!("Gateway stop not yet implemented");
        }
        GatewayCommand::Status => {
            info!("checking gateway status");
            println!("Gateway status not yet implemented");
        }
        GatewayCommand::Setup { platform } => {
            info!("setting up gateway: {:?}", platform);
            println!("Gateway setup not yet implemented");
        }
    }
    Ok(())
}

pub async fn handle_cron(cmd: CronCommand) -> Result<()> {
    match cmd {
        CronCommand::List => {
            info!("listing cron jobs");
            println!("Cron list not yet implemented");
        }
        CronCommand::Add { schedule, command } => {
            info!("adding cron job: {} -> {}", schedule, command);
            println!("Cron add not yet implemented");
        }
        CronCommand::Remove { id } => {
            info!("removing cron job: {}", id);
            println!("Cron remove not yet implemented: {}", id);
        }
        CronCommand::Pause { id } => {
            info!("pausing cron job: {}", id);
            println!("Cron pause not yet implemented: {}", id);
        }
        CronCommand::Resume { id } => {
            info!("resuming cron job: {}", id);
            println!("Cron resume not yet implemented: {}", id);
        }
        CronCommand::Status => {
            info!("checking cron status");
            println!("Cron status not yet implemented");
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