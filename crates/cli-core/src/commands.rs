use super::{AuthCommand, ConfigCommand, CronCommand, GatewayCommand, SkillsCommand, ToolsCommand};
use crate::config::Config;
use anyhow::Result;
use tracing::info;

pub async fn handle_auth(cmd: AuthCommand) -> Result<()> {
    match cmd {
        AuthCommand::Add { provider, api_key, base_url } => {
            info!("adding auth for provider: {}", provider);
            println!("Auth add for {} (api_key: {:?}, base_url: {:?})", provider, api_key.is_some(), base_url.is_some());
        }
        AuthCommand::List => {
            info!("listing auth credentials");
            println!("Auth list not yet implemented");
        }
        AuthCommand::Remove { provider } => {
            info!("removing auth for provider: {}", provider);
            println!("Auth remove not yet implemented for: {}", provider);
        }
        AuthCommand::Reset => {
            info!("resetting all auth credentials");
            println!("Auth reset not yet implemented");
        }
    }
    Ok(())
}

pub fn handle_model(current: bool, global: bool, model: Option<&str>) -> Result<()> {
    let config = Config::load()?;
    match (current, global, model) {
        (true, _, _) => {
            info!("showing current model");
            println!("Current model: {}", config.model.default);
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
            println!("Session model set to: {} (this session only)", m);
        }
        _ => {
            println!("Model command:");
            println!("  hermes model                    - show current model");
            println!("  hermes model <name>            - set session model");
            println!("  hermes model --global <name>   - set global default model");
            println!("  hermes model --current         - show current model details");
        }
    }
    Ok(())
}

pub fn handle_tools(cmd: ToolsCommand) -> Result<()> {
    match cmd {
        ToolsCommand::List { all } => {
            info!("listing tools (all: {})", all);
            println!("Tools list not yet implemented");
        }
        ToolsCommand::Disable { name } => {
            info!("disabling tool: {}", name);
            println!("Tool disable not yet implemented: {}", name);
        }
        ToolsCommand::Enable { name } => {
            info!("enabling tool: {}", name);
            println!("Tool enable not yet implemented: {}", name);
        }
    }
    Ok(())
}

pub async fn handle_skills(cmd: SkillsCommand) -> Result<()> {
    match cmd {
        SkillsCommand::Search { query } => {
            info!("searching skills: {:?}", query);
            println!("Skills search not yet implemented");
        }
        SkillsCommand::Browse => {
            info!("browsing skills hub");
            println!("Skills browse not yet implemented");
        }
        SkillsCommand::Inspect { name } => {
            info!("inspecting skill: {}", name);
            println!("Skill inspect not yet implemented: {}", name);
        }
        SkillsCommand::Install { name } => {
            info!("installing skill: {}", name);
            println!("Skill install not yet implemented: {}", name);
        }
        SkillsCommand::Remove { name } => {
            info!("removing skill: {}", name);
            println!("Skill remove not yet implemented: {}", name);
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
            let value = get_config_value(&config, &key);
            match value {
                Some(v) => println!("{}", v),
                None => {
                    eprintln!("Config key not found: {}", key);
                    std::process::exit(1);
                }
            }
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

fn get_config_value(config: &Config, key: &str) -> Option<String> {
    match key {
        "model.default" => Some(config.model.default.clone()),
        "model.provider" => Some(config.model.provider.clone()),
        "model.base_url" => Some(config.model.base_url.clone()),
        "terminal.env_type" => Some(config.terminal.env_type.clone()),
        "terminal.cwd" => Some(config.terminal.cwd.clone()),
        "terminal.timeout" => Some(config.terminal.timeout.to_string()),
        "display.compact" => Some(config.display.compact.to_string()),
        "display.resume_display" => Some(config.display.resume_display.clone()),
        "display.show_reasoning" => Some(config.display.show_reasoning.to_string()),
        "display.streaming" => Some(config.display.streaming.to_string()),
        "display.skin" => Some(config.display.skin.clone()),
        "agent.max_turns" => Some(config.agent.max_turns.to_string()),
        "agent.verbose" => Some(config.agent.verbose.to_string()),
        "agent.system_prompt" => Some(config.agent.system_prompt.clone()),
        "agent.reasoning_effort" => Some(config.agent.reasoning_effort.clone()),
        _ => None,
    }
}

fn set_config_value(config: &mut Config, key: &str, value: &str) -> Result<()> {
    match key {
        "model.default" => config.model.default = value.to_string(),
        "model.provider" => config.model.provider = value.to_string(),
        "model.base_url" => config.model.base_url = value.to_string(),
        "terminal.env_type" => config.terminal.env_type = value.to_string(),
        "terminal.cwd" => config.terminal.cwd = value.to_string(),
        "terminal.timeout" => config.terminal.timeout = value.parse().unwrap_or(60),
        "display.compact" => config.display.compact = value.parse().unwrap_or(false),
        "display.resume_display" => config.display.resume_display = value.to_string(),
        "display.show_reasoning" => config.display.show_reasoning = value.parse().unwrap_or(false),
        "display.streaming" => config.display.streaming = value.parse().unwrap_or(true),
        "display.skin" => config.display.skin = value.to_string(),
        "agent.max_turns" => config.agent.max_turns = value.parse().unwrap_or(10),
        "agent.verbose" => config.agent.verbose = value.parse().unwrap_or(false),
        "agent.system_prompt" => config.agent.system_prompt = value.to_string(),
        "agent.reasoning_effort" => config.agent.reasoning_effort = value.to_string(),
        _ => {
            anyhow::bail!("Unknown config key: {}", key);
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
    println!("Model: {}", config.model.default);
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