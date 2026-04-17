// Hermes CLI Core

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing::info;

pub mod auth;
pub mod commands;
pub mod config;
pub mod cron;
pub mod error;
pub mod gateway;
pub mod skills;
pub mod tools;

pub use config::Config;
pub use error::CliError;

#[derive(Parser, Debug)]
#[command(name = "hermes", about = "Hermes Agent CLI", version, author)]
pub struct Cli {
    #[arg(short, long, global = true)]
    verbose: bool,
    #[arg(short, long, global = true)]
    debug: bool,
    #[arg(short = 'p', long, global = true, value_name = "NAME")]
    profile: Option<String>,
    #[arg(long, global = true, value_name = "PATH")]
    directory: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum Commands {
    Chat {
        model: Option<String>,
        #[arg(short, long)]
        system: Option<String>,
        #[arg(long)]
        resume: Option<String>,
    },
    #[command(subcommand)]
    Auth(AuthCommand),
    Model {
        #[arg(short, long)]
        current: bool,
        #[arg(long)]
        global: bool,
        model: Option<String>,
    },
    #[command(subcommand)]
    Tools(ToolsCommand),
    #[command(subcommand)]
    Skills(SkillsCommand),
    #[command(subcommand)]
    Gateway(GatewayCommand),
    #[command(subcommand)]
    Cron(CronCommand),
    #[command(subcommand)]
    Config(ConfigCommand),
    Setup {
        #[arg(long)]
        skip_auth: bool,
        #[arg(long)]
        skip_model: bool,
    },
    Doctor {
        #[arg(short, long)]
        all: bool,
        check: Option<String>,
    },
    Status,
    Version,
    Update,
    Uninstall,
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum AuthCommand {
    Add {
        provider: String,
        #[arg(short, long)]
        api_key: Option<String>,
        #[arg(long)]
        base_url: Option<String>,
    },
    List,
    Remove { provider: String },
    Reset,
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum ToolsCommand {
    List {
        #[arg(short, long)]
        all: bool,
    },
    Disable { name: String },
    Enable { name: String },
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum SkillsCommand {
    Search { query: Option<String> },
    Browse,
    Inspect { name: String },
    Install { name: String },
    Remove { name: String },
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum GatewayCommand {
    Run {
        #[arg(short = 'P', long)]
        platform: Option<String>,
    },
    Start,
    Stop,
    Status,
    Setup {
        platform: Option<String>,
    },
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum CronCommand {
    List,
    Add { schedule: String, command: String },
    Remove { id: String },
    Pause { id: String },
    Resume { id: String },
    Status,
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum ConfigCommand {
    Show,
    Get { key: String },
    Set { key: String, value: String },
    Reset,
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    init_logging(cli.verbose, cli.debug);
    info!("hermes-cli starting...");
    let _config = Config::load()?;
    match &cli.command {
        Commands::Chat { .. } => { info!("chat mode not yet implemented"); println!("Chat mode coming soon!"); }
        Commands::Auth(ref cmd) => commands::handle_auth(cmd.clone()).await?,
        Commands::Model { current, global, model } => commands::handle_model(*current, *global, model.as_deref())?,
        Commands::Tools(ref cmd) => commands::handle_tools(cmd.clone())?,
        Commands::Skills(ref cmd) => commands::handle_skills(cmd.clone())?,
        Commands::Gateway(ref cmd) => commands::handle_gateway(cmd.clone()).await?,
        Commands::Cron(ref cmd) => commands::handle_cron(cmd.clone()).await?,
        Commands::Config(ref cmd) => commands::handle_config(cmd.clone())?,
        Commands::Setup { skip_auth, skip_model } => commands::handle_setup(*skip_auth, *skip_model)?,
        Commands::Doctor { all, check } => commands::handle_doctor(*all, check.as_deref())?,
        Commands::Status => commands::handle_status()?,
        Commands::Version => { println!("hermes {}", env!("CARGO_PKG_VERSION")); }
        Commands::Update => commands::handle_update()?,
        Commands::Uninstall => commands::handle_uninstall()?,
    }
    Ok(())
}

fn init_logging(verbose: bool, debug: bool) {
    use tracing_subscriber::EnvFilter;
    let level = if debug { tracing::Level::DEBUG } else if verbose { tracing::Level::INFO } else { tracing::Level::WARN };
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(level.into()))
        .with_target(false)
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Chat ===
    #[test]
    fn test_cli_parse_chat() {
        let cli = Cli::parse_from(vec!["hermes", "chat", "gpt-4"]);
        if let Commands::Chat { model, .. } = cli.command {
            assert_eq!(model, Some("gpt-4".to_string()));
        } else { panic!("expected Chat"); }
    }

    #[test]
    fn test_cli_parse_chat_with_system() {
        let cli = Cli::parse_from(vec!["hermes", "chat", "gpt-4", "--system", "You are helpful"]);
        if let Commands::Chat { model, system, .. } = cli.command {
            assert_eq!(model, Some("gpt-4".to_string()));
            assert_eq!(system, Some("You are helpful".to_string()));
        } else { panic!("expected Chat"); }
    }

    // === Status / Version ===
    #[test]
    fn test_cli_parse_status() {
        let cli = Cli::parse_from(vec!["hermes", "status"]);
        assert!(matches!(cli.command, Commands::Status));
    }

    #[test]
    fn test_cli_parse_version() {
        let cli = Cli::parse_from(vec!["hermes", "version"]);
        assert!(matches!(cli.command, Commands::Version));
    }

    // === Config ===
    #[test]
    fn test_cli_parse_config_show() {
        let cli = Cli::parse_from(vec!["hermes", "config", "show"]);
        assert!(matches!(cli.command, Commands::Config(ConfigCommand::Show)));
    }

    #[test]
    fn test_cli_parse_config_get() {
        let cli = Cli::parse_from(vec!["hermes", "config", "get", "model.default"]);
        if let Commands::Config(ConfigCommand::Get { key }) = cli.command {
            assert_eq!(key, "model.default");
        } else { panic!("expected Config::Get"); }
    }

    #[test]
    fn test_cli_parse_config_set() {
        let cli = Cli::parse_from(vec!["hermes", "config", "set", "model.default", "gpt-4"]);
        if let Commands::Config(ConfigCommand::Set { key, value }) = cli.command {
            assert_eq!(key, "model.default");
            assert_eq!(value, "gpt-4");
        } else { panic!("expected Config::Set"); }
    }

    #[test]
    fn test_cli_parse_config_reset() {
        let cli = Cli::parse_from(vec!["hermes", "config", "reset"]);
        assert!(matches!(cli.command, Commands::Config(ConfigCommand::Reset)));
    }

    // === Model ===
    #[test]
    fn test_cli_parse_model_current() {
        let cli = Cli::parse_from(vec!["hermes", "model", "--current"]);
        if let Commands::Model { current, global, model } = cli.command {
            assert!(current);
            assert!(!global);
            assert_eq!(model, None);
        } else { panic!("expected Model"); }
    }

    #[test]
    fn test_cli_parse_model_global() {
        let cli = Cli::parse_from(vec!["hermes", "model", "--global", "claude-3"]);
        if let Commands::Model { current, global, model } = cli.command {
            assert!(!current);
            assert!(global);
            assert_eq!(model, Some("claude-3".to_string()));
        } else { panic!("expected Model"); }
    }

    #[test]
    fn test_cli_parse_model_session() {
        let cli = Cli::parse_from(vec!["hermes", "model", "gpt-4o"]);
        if let Commands::Model { current, global, model } = cli.command {
            assert!(!current);
            assert!(!global);
            assert_eq!(model, Some("gpt-4o".to_string()));
        } else { panic!("expected Model"); }
    }

    // === Auth ===
    #[test]
    fn test_cli_parse_auth_add() {
        let cli = Cli::parse_from(vec!["hermes", "auth", "add", "openai", "--api-key", "sk-test123"]);
        if let Commands::Auth(AuthCommand::Add { provider, api_key, base_url }) = cli.command {
            assert_eq!(provider, "openai");
            assert_eq!(api_key, Some("sk-test123".to_string()));
            assert_eq!(base_url, None);
        } else { panic!("expected Auth::Add"); }
    }

    #[test]
    fn test_cli_parse_auth_add_with_base_url() {
        let cli = Cli::parse_from(vec!["hermes", "auth", "add", "custom", "--api-key", "key123", "--base-url", "https://api.example.com"]);
        if let Commands::Auth(AuthCommand::Add { provider, api_key, base_url }) = cli.command {
            assert_eq!(provider, "custom");
            assert_eq!(api_key, Some("key123".to_string()));
            assert_eq!(base_url, Some("https://api.example.com".to_string()));
        } else { panic!("expected Auth::Add"); }
    }

    #[test]
    fn test_cli_parse_auth_list() {
        let cli = Cli::parse_from(vec!["hermes", "auth", "list"]);
        assert!(matches!(cli.command, Commands::Auth(AuthCommand::List)));
    }

    #[test]
    fn test_cli_parse_auth_remove() {
        let cli = Cli::parse_from(vec!["hermes", "auth", "remove", "openai"]);
        if let Commands::Auth(AuthCommand::Remove { provider }) = cli.command {
            assert_eq!(provider, "openai");
        } else { panic!("expected Auth::Remove"); }
    }

    #[test]
    fn test_cli_parse_auth_reset() {
        let cli = Cli::parse_from(vec!["hermes", "auth", "reset"]);
        assert!(matches!(cli.command, Commands::Auth(AuthCommand::Reset)));
    }

    // === Tools ===
    #[test]
    fn test_cli_parse_tools_list() {
        let cli = Cli::parse_from(vec!["hermes", "tools", "list"]);
        if let Commands::Tools(ToolsCommand::List { all }) = cli.command {
            assert!(!all);
        } else { panic!("expected Tools::List"); }
    }

    #[test]
    fn test_cli_parse_tools_list_all() {
        let cli = Cli::parse_from(vec!["hermes", "tools", "list", "--all"]);
        if let Commands::Tools(ToolsCommand::List { all }) = cli.command {
            assert!(all);
        } else { panic!("expected Tools::List"); }
    }

    #[test]
    fn test_cli_parse_tools_disable() {
        let cli = Cli::parse_from(vec!["hermes", "tools", "disable", "web_search"]);
        if let Commands::Tools(ToolsCommand::Disable { name }) = cli.command {
            assert_eq!(name, "web_search");
        } else { panic!("expected Tools::Disable"); }
    }

    #[test]
    fn test_cli_parse_tools_enable() {
        let cli = Cli::parse_from(vec!["hermes", "tools", "enable", "web_search"]);
        if let Commands::Tools(ToolsCommand::Enable { name }) = cli.command {
            assert_eq!(name, "web_search");
        } else { panic!("expected Tools::Enable"); }
    }

    // === Skills ===
    #[test]
    fn test_cli_parse_skills_search() {
        let cli = Cli::parse_from(vec!["hermes", "skills", "search", "web"]);
        if let Commands::Skills(SkillsCommand::Search { query }) = cli.command {
            assert_eq!(query, Some("web".to_string()));
        } else { panic!("expected Skills::Search"); }
    }

    #[test]
    fn test_cli_parse_skills_browse() {
        let cli = Cli::parse_from(vec!["hermes", "skills", "browse"]);
        assert!(matches!(cli.command, Commands::Skills(SkillsCommand::Browse)));
    }

    #[test]
    fn test_cli_parse_skills_inspect() {
        let cli = Cli::parse_from(vec!["hermes", "skills", "inspect", "web-search"]);
        if let Commands::Skills(SkillsCommand::Inspect { name }) = cli.command {
            assert_eq!(name, "web-search");
        } else { panic!("expected Skills::Inspect"); }
    }

    // === Gateway ===
    #[test]
    fn test_cli_parse_gateway_run() {
        let cli = Cli::parse_from(vec!["hermes", "gateway", "run"]);
        if let Commands::Gateway(GatewayCommand::Run { platform }) = cli.command {
            assert_eq!(platform, None);
        } else { panic!("expected Gateway::Run"); }
    }

    #[test]
    fn test_cli_parse_gateway_run_with_platform() {
        let cli = Cli::parse_from(vec!["hermes", "gateway", "run", "-P", "telegram"]);
        if let Commands::Gateway(GatewayCommand::Run { platform }) = cli.command {
            assert_eq!(platform, Some("telegram".to_string()));
        } else { panic!("expected Gateway::Run"); }
    }

    #[test]
    fn test_cli_parse_gateway_start() {
        let cli = Cli::parse_from(vec!["hermes", "gateway", "start"]);
        assert!(matches!(cli.command, Commands::Gateway(GatewayCommand::Start)));
    }

    #[test]
    fn test_cli_parse_gateway_stop() {
        let cli = Cli::parse_from(vec!["hermes", "gateway", "stop"]);
        assert!(matches!(cli.command, Commands::Gateway(GatewayCommand::Stop)));
    }

    #[test]
    fn test_cli_parse_gateway_status() {
        let cli = Cli::parse_from(vec!["hermes", "gateway", "status"]);
        assert!(matches!(cli.command, Commands::Gateway(GatewayCommand::Status)));
    }

    #[test]
    fn test_cli_parse_gateway_setup() {
        let cli = Cli::parse_from(vec!["hermes", "gateway", "setup", "telegram"]);
        if let Commands::Gateway(GatewayCommand::Setup { platform }) = cli.command {
            assert_eq!(platform, Some("telegram".to_string()));
        } else { panic!("expected Gateway::Setup"); }
    }

    // === Cron ===
    #[test]
    fn test_cli_parse_cron_list() {
        let cli = Cli::parse_from(vec!["hermes", "cron", "list"]);
        assert!(matches!(cli.command, Commands::Cron(CronCommand::List)));
    }

    #[test]
    fn test_cli_parse_cron_add() {
        let cli = Cli::parse_from(vec!["hermes", "cron", "add", "every 30m", "check status"]);
        if let Commands::Cron(CronCommand::Add { schedule, command }) = cli.command {
            assert_eq!(schedule, "every 30m");
            assert_eq!(command, "check status");
        } else { panic!("expected Cron::Add"); }
    }

    #[test]
    fn test_cli_parse_cron_remove() {
        let cli = Cli::parse_from(vec!["hermes", "cron", "remove", "abc123"]);
        if let Commands::Cron(CronCommand::Remove { id }) = cli.command {
            assert_eq!(id, "abc123");
        } else { panic!("expected Cron::Remove"); }
    }

    #[test]
    fn test_cli_parse_cron_pause() {
        let cli = Cli::parse_from(vec!["hermes", "cron", "pause", "abc123"]);
        if let Commands::Cron(CronCommand::Pause { id }) = cli.command {
            assert_eq!(id, "abc123");
        } else { panic!("expected Cron::Pause"); }
    }

    #[test]
    fn test_cli_parse_cron_resume() {
        let cli = Cli::parse_from(vec!["hermes", "cron", "resume", "abc123"]);
        if let Commands::Cron(CronCommand::Resume { id }) = cli.command {
            assert_eq!(id, "abc123");
        } else { panic!("expected Cron::Resume"); }
    }

    #[test]
    fn test_cli_parse_cron_status() {
        let cli = Cli::parse_from(vec!["hermes", "cron", "status"]);
        assert!(matches!(cli.command, Commands::Cron(CronCommand::Status)));
    }

    // === Setup / Doctor ===
    #[test]
    fn test_cli_parse_setup() {
        let cli = Cli::parse_from(vec!["hermes", "setup"]);
        if let Commands::Setup { skip_auth, skip_model } = cli.command {
            assert!(!skip_auth);
            assert!(!skip_model);
        } else { panic!("expected Setup"); }
    }

    #[test]
    fn test_cli_parse_setup_skip_auth() {
        let cli = Cli::parse_from(vec!["hermes", "setup", "--skip-auth"]);
        if let Commands::Setup { skip_auth, skip_model } = cli.command {
            assert!(skip_auth);
            assert!(!skip_model);
        } else { panic!("expected Setup"); }
    }

    #[test]
    fn test_cli_parse_setup_skip_model() {
        let cli = Cli::parse_from(vec!["hermes", "setup", "--skip-model"]);
        if let Commands::Setup { skip_auth, skip_model } = cli.command {
            assert!(!skip_auth);
            assert!(skip_model);
        } else { panic!("expected Setup"); }
    }

    #[test]
    fn test_cli_parse_doctor() {
        let cli = Cli::parse_from(vec!["hermes", "doctor"]);
        if let Commands::Doctor { all, check } = cli.command {
            assert!(!all);
            assert_eq!(check, None);
        } else { panic!("expected Doctor"); }
    }

    #[test]
    fn test_cli_parse_doctor_all() {
        let cli = Cli::parse_from(vec!["hermes", "doctor", "--all"]);
        if let Commands::Doctor { all, check } = cli.command {
            assert!(all);
            assert_eq!(check, None);
        } else { panic!("expected Doctor"); }
    }

    #[test]
    fn test_cli_parse_doctor_check() {
        let cli = Cli::parse_from(vec!["hermes", "doctor", "python"]);
        if let Commands::Doctor { all, check } = cli.command {
            assert!(!all);
            assert_eq!(check, Some("python".to_string()));
        } else { panic!("expected Doctor"); }
    }

    // === Update / Uninstall ===
    #[test]
    fn test_cli_parse_update() {
        let cli = Cli::parse_from(vec!["hermes", "update"]);
        assert!(matches!(cli.command, Commands::Update));
    }

    #[test]
    fn test_cli_parse_uninstall() {
        let cli = Cli::parse_from(vec!["hermes", "uninstall"]);
        assert!(matches!(cli.command, Commands::Uninstall));
    }

    // === Global flags ===
    #[test]
    fn test_cli_parse_verbose() {
        let cli = Cli::parse_from(vec!["hermes", "-v", "status"]);
        assert!(cli.verbose);
    }

    #[test]
    fn test_cli_parse_debug() {
        let cli = Cli::parse_from(vec!["hermes", "-d", "status"]);
        assert!(cli.debug);
    }

    #[test]
    fn test_cli_parse_profile() {
        let cli = Cli::parse_from(vec!["hermes", "-p", "work", "status"]);
        assert_eq!(cli.profile, Some("work".to_string()));
    }
}