// Hermes CLI Core

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing::info;

pub mod commands;
pub mod config;
pub mod error;

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
        #[arg(short, long)]
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
        Commands::Skills(ref cmd) => commands::handle_skills(cmd.clone()).await?,
        Commands::Gateway(ref cmd) => commands::handle_gateway(cmd.clone()).await?,
        Commands::Cron(ref cmd) => commands::handle_cron(cmd.clone()).await?,
        Commands::Config(ref cmd) => commands::handle_config(cmd.clone())?,
        Commands::Setup { .. } => { info!("setup not implemented"); }
        Commands::Doctor { .. } => { info!("doctor not implemented"); }
        Commands::Status => commands::handle_status()?,
        Commands::Version => { println!("hermes {}", env!("CARGO_PKG_VERSION")); }
        Commands::Update => { info!("update not implemented"); }
        Commands::Uninstall => { info!("uninstall not implemented"); }
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
}