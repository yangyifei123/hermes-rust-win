// Hermes CLI Core — full command surface matching Python Hermes CLI

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing::info;

pub mod auth;
pub mod commands;
pub mod config;
pub mod credential_pool;
pub mod cron;
pub mod error;
pub mod gateway;
pub mod mcp;
pub mod pairings;
pub mod plugins;
pub mod profiles;
pub mod skills;
pub mod skills_store;
pub mod tools;
pub mod webhooks;

pub use config::Config;
pub use error::CliError;

// ── Top-level CLI ────────────────────────────────────────────────────────────

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
    /// Resume a previous session by ID
    #[arg(long, global = true, value_name = "SESSION_ID")]
    resume: Option<String>,
    /// Resume session by name, or most recent if no name given
    #[arg(short = 'c', long = "continue", global = true, value_name = "SESSION_NAME")]
    continue_last: Option<Option<String>>,
    #[command(subcommand)]
    pub command: Option<Commands>,
}

// ── All Commands ─────────────────────────────────────────────────────────────

#[derive(clap::Subcommand, Debug, Clone)]
pub enum Commands {
    // ── Chat (interactive) ───────────────────────────────────────────────
    Chat {
        model: Option<String>,
        /// Single query (non-interactive mode)
        #[arg(short, long)]
        query: Option<String>,
        /// Optional local image path to attach
        #[arg(long)]
        image: Option<String>,
        /// System prompt override
        #[arg(short, long)]
        system: Option<String>,
        /// Comma-separated toolsets to enable
        #[arg(short, long)]
        toolsets: Option<String>,
        /// Preload one or more skills (repeat flag)
        #[arg(long)]
        skills: Option<Vec<String>>,
        /// Inference provider
        #[arg(long)]
        provider: Option<String>,
        /// Verbose output (chat-specific)
        #[arg(long)]
        chat_verbose: bool,
        /// Quiet mode: suppress banner, spinner, tool previews
        #[arg(short = 'Q', long)]
        quiet: bool,
        /// Resume a previous session by ID
        #[arg(short, long)]
        resume: Option<String>,
        /// Resume session by name, or most recent
        #[arg(short = 'n', long = "continue")]
        continue_last: Option<Option<String>>,
        /// Run in isolated git worktree
        #[arg(short, long)]
        worktree: bool,
        /// Enable filesystem checkpoints before destructive ops
        #[arg(long)]
        checkpoints: bool,
        /// Max tool-calling iterations per turn
        #[arg(long)]
        max_turns: Option<u32>,
        /// Bypass all dangerous command approval prompts
        #[arg(long)]
        yolo: bool,
        /// Include session ID in system prompt
        #[arg(long)]
        pass_session_id: bool,
        /// Session source tag for filtering (default: cli)
        #[arg(long)]
        source: Option<String>,
    },

    // ── Auth ─────────────────────────────────────────────────────────────
    #[command(subcommand)]
    Auth(AuthCommand),

    // ── Model (interactive selection) ─────────────────────────────────────
    Model {
        #[arg(short = 'C', long)]
        current: bool,
        #[arg(long)]
        global: bool,
        model: Option<String>,
        #[arg(long)]
        portal_url: Option<String>,
        #[arg(long)]
        inference_url: Option<String>,
        #[arg(long)]
        client_id: Option<String>,
        #[arg(long)]
        scope: Option<String>,
        #[arg(long)]
        no_browser: bool,
        #[arg(long, default_value = "15.0")]
        timeout: f64,
        #[arg(long)]
        ca_bundle: Option<String>,
        #[arg(long)]
        insecure: bool,
    },

    // ── Tools ────────────────────────────────────────────────────────────
    #[command(subcommand)]
    Tools(ToolsCommand),

    // ── Skills ───────────────────────────────────────────────────────────
    #[command(subcommand)]
    Skills(SkillsCommand),

    // ── Gateway ──────────────────────────────────────────────────────────
    #[command(subcommand)]
    Gateway(GatewayCommand),

    // ── Cron ─────────────────────────────────────────────────────────────
    #[command(subcommand)]
    Cron(CronCommand),

    // ── Config ───────────────────────────────────────────────────────────
    #[command(subcommand)]
    Config(ConfigCommand),

    // ── Setup ────────────────────────────────────────────────────────────
    Setup {
        /// Run setup section: model|tts|terminal|gateway|tools|agent
        section: Option<String>,
        #[arg(long)]
        skip_auth: bool,
        #[arg(long)]
        skip_model: bool,
        #[arg(long)]
        non_interactive: bool,
        #[arg(long)]
        reset: bool,
    },

    // ── Doctor ────────────────────────────────────────────────────────────
    Doctor {
        #[arg(short, long)]
        all: bool,
        /// Check a specific component
        check: Option<String>,
        #[arg(long)]
        fix: bool,
    },

    // ── Status ────────────────────────────────────────────────────────────
    Status {
        #[arg(long)]
        all: bool,
        #[arg(long)]
        deep: bool,
    },

    // ── Sessions ─────────────────────────────────────────────────────────
    #[command(subcommand)]
    Sessions(SessionsCommand),

    // ── Logs ──────────────────────────────────────────────────────────────
    Logs {
        /// Log to view: agent (default), errors, gateway, or 'list'
        log_name: Option<String>,
        /// Number of lines to show
        #[arg(long, default_value = "50")]
        lines: u32,
        /// Follow the log in real time
        #[arg(short, long)]
        follow: bool,
        /// Minimum log level (DEBUG, INFO, WARNING, ERROR)
        #[arg(long)]
        level: Option<String>,
        /// Filter by session ID
        #[arg(long)]
        session: Option<String>,
        /// Show lines since TIME ago (e.g. 1h, 30m, 2d)
        #[arg(long)]
        since: Option<String>,
        /// Filter by component: gateway, agent, tools, cli, cron
        #[arg(long)]
        component: Option<String>,
    },

    // ── Profile ──────────────────────────────────────────────────────────
    #[command(subcommand)]
    Profile(ProfileCommand),

    // ── MCP ───────────────────────────────────────────────────────────────
    #[command(subcommand)]
    Mcp(McpCommand),

    // ── Memory ───────────────────────────────────────────────────────────
    #[command(subcommand)]
    Memory(MemoryCommand),

    // ── Webhook ──────────────────────────────────────────────────────────
    #[command(subcommand)]
    Webhook(WebhookCommand),

    // ── Pairing ─────────────────────────────────────────────────────────
    #[command(subcommand)]
    Pairing(PairingCommand),

    // ── Plugins ──────────────────────────────────────────────────────────
    #[command(subcommand)]
    Plugins(PluginsCommand),

    // ── Backup ───────────────────────────────────────────────────────────
    Backup {
        /// Output path for the zip file
        #[arg(short, long)]
        output: Option<String>,
        /// Quick snapshot of critical state files only
        #[arg(short, long)]
        quick: bool,
        /// Label for the snapshot (with --quick)
        #[arg(short, long)]
        label: Option<String>,
    },

    // ── Import ────────────────────────────────────────────────────────────
    Import {
        /// Path to the backup zip file
        zipfile: String,
        /// Overwrite existing files without confirmation
        #[arg(short, long)]
        force: bool,
    },

    // ── Debug ─────────────────────────────────────────────────────────────
    #[command(subcommand)]
    Debug(DebugCommand),

    // ── Dump ─────────────────────────────────────────────────────────────
    Dump {
        /// Show redacted API key prefixes
        #[arg(long)]
        show_keys: bool,
    },

    // ── Completion ───────────────────────────────────────────────────────
    Completion {
        /// Shell type
        shell: Option<String>,
    },

    // ── Insights ─────────────────────────────────────────────────────────
    Insights {
        /// Number of days to analyze
        #[arg(long, default_value = "30")]
        days: u32,
        /// Filter by platform source
        #[arg(long)]
        source: Option<String>,
    },

    // ── Login ────────────────────────────────────────────────────────────
    Login {
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        portal_url: Option<String>,
        #[arg(long)]
        inference_url: Option<String>,
        #[arg(long)]
        client_id: Option<String>,
        #[arg(long)]
        scope: Option<String>,
        #[arg(long)]
        no_browser: bool,
        #[arg(long, default_value = "15.0")]
        timeout: f64,
        #[arg(long)]
        ca_bundle: Option<String>,
        #[arg(long)]
        insecure: bool,
    },

    // ── Logout ────────────────────────────────────────────────────────────
    Logout {
        #[arg(long)]
        provider: Option<String>,
    },

    // ── WhatsApp ─────────────────────────────────────────────────────────
    Whatsapp,

    // ── ACP ───────────────────────────────────────────────────────────────
    Acp,

    // ── Dashboard ─────────────────────────────────────────────────────────
    Dashboard {
        #[arg(long, default_value = "9119")]
        port: u16,
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long)]
        no_open: bool,
    },

    // ── Claw (OpenClaw migration) ────────────────────────────────────────
    #[command(subcommand)]
    Claw(ClawCommand),

    // ── Version ──────────────────────────────────────────────────────────
    Version,

    // ── Update ───────────────────────────────────────────────────────────
    Update {
        #[arg(long)]
        gateway: bool,
    },

    // ── Uninstall ────────────────────────────────────────────────────────
    Uninstall {
        #[arg(long)]
        full: bool,
        #[arg(short, long)]
        yes: bool,
    },
}

// ── Subcommand Enums ─────────────────────────────────────────────────────────

#[derive(clap::Subcommand, Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum AuthCommand {
    Add {
        provider: String,
        /// API key value (otherwise prompted)
        #[arg(long)]
        api_key: Option<String>,
        /// Credential type: oauth, api-key
        #[arg(long = "type", value_name = "AUTH_TYPE")]
        auth_type: Option<String>,
        /// Optional display label
        #[arg(long)]
        label: Option<String>,
        /// Base URL override
        #[arg(long)]
        base_url: Option<String>,
        /// Nous portal base URL
        #[arg(long)]
        portal_url: Option<String>,
        /// Nous inference base URL
        #[arg(long)]
        inference_url: Option<String>,
        /// OAuth client id
        #[arg(long)]
        client_id: Option<String>,
        /// OAuth scope override
        #[arg(long)]
        scope: Option<String>,
        /// Do not auto-open browser for OAuth
        #[arg(long)]
        no_browser: bool,
        /// OAuth/network timeout in seconds
        #[arg(long)]
        timeout: Option<f64>,
        /// Disable TLS verification
        #[arg(long)]
        insecure: bool,
        /// Custom CA bundle path
        #[arg(long)]
        ca_bundle: Option<String>,
    },
    List {
        /// Optional provider filter
        provider: Option<String>,
    },
    Remove {
        provider: String,
        /// Credential index, entry id, or exact label
        target: Option<String>,
    },
    Reset {
        /// Provider to reset exhaustion for
        provider: Option<String>,
    },
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum ToolsCommand {
    List {
        #[arg(short, long)]
        all: bool,
        /// Platform to show (default: cli)
        #[arg(long, default_value = "cli")]
        platform: String,
    },
    Disable {
        /// Toolset names to disable
        names: Vec<String>,
        #[arg(long, default_value = "cli")]
        platform: String,
    },
    Enable {
        /// Toolset names to enable
        names: Vec<String>,
        #[arg(long, default_value = "cli")]
        platform: String,
    },
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum SkillsCommand {
    Search {
        query: Option<String>,
        #[arg(long, default_value = "all")]
        source: String,
        #[arg(long, default_value = "10")]
        limit: u32,
    },
    Browse {
        #[arg(long, default_value = "1")]
        page: u32,
        #[arg(long, default_value = "20")]
        size: u32,
        #[arg(long, default_value = "all")]
        source: String,
    },
    Inspect {
        name: String,
    },
    Install {
        identifier: String,
        #[arg(long, default_value = "")]
        category: String,
        #[arg(long)]
        force: bool,
        #[arg(short, long)]
        yes: bool,
    },
    List {
        #[arg(long, default_value = "all")]
        source: String,
    },
    Check {
        /// Specific skill to check (default: all)
        name: Option<String>,
    },
    Update {
        /// Specific skill to update (default: all outdated)
        name: Option<String>,
    },
    Audit {
        /// Specific skill to audit (default: all)
        name: Option<String>,
    },
    Uninstall {
        name: String,
    },
    Publish {
        skill_path: String,
        #[arg(long, default_value = "github")]
        to: String,
        #[arg(long, default_value = "")]
        repo: String,
    },
    #[command(subcommand)]
    Snapshot(SkillsSnapshotCommand),
    #[command(subcommand)]
    Tap(SkillsTapCommand),
    Config,
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum SkillsSnapshotCommand {
    Export {
        /// Output JSON file path (use - for stdout)
        output: String,
    },
    Import {
        /// Input JSON file path
        input: String,
        #[arg(long)]
        force: bool,
    },
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum SkillsTapCommand {
    List,
    Add {
        /// GitHub repo (e.g. owner/repo)
        repo: String,
    },
    Remove {
        name: String,
    },
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum GatewayCommand {
    Run {
        #[arg(short = 'P', long)]
        platform: Option<String>,
        /// Increase stderr log verbosity
        #[arg(long)]
        verbose: bool,
        /// Suppress all stderr log output
        #[arg(short, long)]
        quiet: bool,
        /// Replace any existing gateway instance
        #[arg(long)]
        replace: bool,
    },
    Start {
        #[arg(long)]
        system: bool,
    },
    Stop {
        #[arg(long)]
        system: bool,
        #[arg(long)]
        all: bool,
    },
    Restart {
        #[arg(long)]
        system: bool,
    },
    Status {
        #[arg(long)]
        deep: bool,
        #[arg(long)]
        system: bool,
    },
    Setup {
        platform: Option<String>,
    },
    Install {
        #[arg(long)]
        force: bool,
        #[arg(long)]
        system: bool,
        #[arg(long)]
        run_as_user: Option<String>,
    },
    Uninstall {
        #[arg(long)]
        system: bool,
    },
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum CronCommand {
    List {
        #[arg(long)]
        all: bool,
    },
    Add {
        schedule: String,
        /// Optional self-contained prompt
        command: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        deliver: Option<String>,
        #[arg(long)]
        repeat: Option<u32>,
        #[arg(long)]
        skill: Option<Vec<String>>,
        #[arg(long)]
        script: Option<String>,
    },
    Edit {
        job_id: String,
        #[arg(long)]
        schedule: Option<String>,
        #[arg(long)]
        prompt: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        deliver: Option<String>,
        #[arg(long)]
        repeat: Option<u32>,
        #[arg(long)]
        skill: Option<Vec<String>>,
        #[arg(long)]
        add_skill: Option<Vec<String>>,
        #[arg(long)]
        remove_skill: Option<Vec<String>>,
        #[arg(long)]
        clear_skills: bool,
        #[arg(long)]
        script: Option<String>,
    },
    Remove {
        id: String,
    },
    Pause {
        id: String,
    },
    Resume {
        id: String,
    },
    Run {
        id: String,
    },
    Status,
    Tick,
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum ConfigCommand {
    Show,
    Edit,
    Get { key: String },
    Set { key: String, value: String },
    Reset,
    Path,
    EnvPath,
    Check,
    Migrate,
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum SessionsCommand {
    List {
        #[arg(long)]
        source: Option<String>,
        #[arg(long, default_value = "20")]
        limit: u32,
    },
    Export {
        output: String,
        #[arg(long)]
        source: Option<String>,
        #[arg(long)]
        session_id: Option<String>,
    },
    Delete {
        session_id: String,
        #[arg(short, long)]
        yes: bool,
    },
    Prune {
        #[arg(long, default_value = "90")]
        older_than: u32,
        #[arg(long)]
        source: Option<String>,
        #[arg(short, long)]
        yes: bool,
    },
    Stats,
    Rename {
        session_id: String,
        title: Vec<String>,
    },
    Browse {
        #[arg(long)]
        source: Option<String>,
        #[arg(long, default_value = "50")]
        limit: u32,
    },
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum ProfileCommand {
    List,
    Use {
        profile_name: String,
    },
    Create {
        profile_name: String,
        #[arg(long)]
        clone: bool,
        #[arg(long)]
        clone_all: bool,
        #[arg(long)]
        clone_from: Option<String>,
        #[arg(long)]
        no_alias: bool,
    },
    Delete {
        profile_name: String,
        #[arg(short, long)]
        yes: bool,
    },
    Show {
        profile_name: String,
    },
    Alias {
        profile_name: String,
        #[arg(long)]
        remove: bool,
        #[arg(long)]
        alias_name: Option<String>,
    },
    Rename {
        old_name: String,
        new_name: String,
    },
    Export {
        profile_name: String,
        #[arg(short, long)]
        output: Option<String>,
    },
    Import {
        archive: String,
        #[arg(long)]
        import_name: Option<String>,
    },
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum McpCommand {
    Serve {
        #[arg(short, long)]
        verbose: bool,
    },
    Add {
        name: String,
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        command: Option<String>,
        #[arg(long)]
        args: Option<Vec<String>>,
        #[arg(long)]
        auth: Option<String>,
        #[arg(long)]
        preset: Option<String>,
        #[arg(long)]
        env: Option<Vec<String>>,
    },
    Remove {
        name: String,
    },
    List,
    Test {
        name: String,
    },
    Configure {
        name: String,
    },
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum MemoryCommand {
    Setup,
    Status,
    Off,
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum WebhookCommand {
    Subscribe {
        /// Route name (used in URL)
        name: String,
        #[arg(long, default_value = "")]
        prompt: String,
        #[arg(long, default_value = "")]
        events: String,
        #[arg(long, default_value = "")]
        description: String,
        #[arg(long, default_value = "")]
        skills: String,
        #[arg(long, default_value = "log")]
        deliver: String,
        #[arg(long, default_value = "")]
        deliver_chat_id: String,
        #[arg(long, default_value = "")]
        secret: String,
    },
    List,
    Remove {
        name: String,
    },
    Test {
        name: String,
        #[arg(long, default_value = "")]
        payload: String,
    },
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum PairingCommand {
    List,
    Approve {
        platform: String,
        code: String,
    },
    Revoke {
        platform: String,
        user_id: String,
    },
    ClearPending,
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum PluginsCommand {
    Install {
        identifier: String,
        #[arg(short, long)]
        force: bool,
    },
    Update {
        name: String,
    },
    Remove {
        name: String,
    },
    List,
    Enable {
        name: String,
    },
    Disable {
        name: String,
    },
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum DebugCommand {
    Share {
        #[arg(long, default_value = "200")]
        lines: u32,
        #[arg(long, default_value = "7")]
        expire: u32,
        #[arg(long)]
        local: bool,
    },
}

#[derive(clap::Subcommand, Debug, Clone)]
pub enum ClawCommand {
    Migrate {
        #[arg(long)]
        source: Option<String>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, default_value = "full")]
        preset: String,
        #[arg(long)]
        overwrite: bool,
        #[arg(long)]
        migrate_secrets: bool,
        #[arg(long)]
        workspace_target: Option<String>,
        #[arg(long, default_value = "skip")]
        skill_conflict: String,
        #[arg(short, long)]
        yes: bool,
    },
    Cleanup {
        #[arg(long)]
        source: Option<String>,
        #[arg(long)]
        dry_run: bool,
        #[arg(short, long)]
        yes: bool,
    },
}

// ── Entry point ──────────────────────────────────────────────────────────────

/// Check C drive free space and auto-clean when below threshold.
/// Returns the free GB after any cleanup.
fn ensure_disk_space(threshold_gb: f64) -> f64 {
    #[cfg(target_os = "windows")]
    {
        // Use PowerShell to check disk space
        let output = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", "(Get-PSDrive C).Free / 1GB"])
            .output();

        let free_gb = match output {
            Ok(o) if o.status.success() => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                stdout.trim().parse::<f64>().unwrap_or(100.0)
            }
            _ => 100.0, // Assume OK if can't check
        };

        if free_gb < threshold_gb {
            info!("C drive low: {:.2}GB free (threshold: {:.2}GB), auto-cleaning...", free_gb, threshold_gb);

            // Clean cargo target on E: drive
            let _ = std::fs::remove_dir_all("E:\\AI_field\\hermes-rust-win\\target");

            // Clean C: caches
            let home = std::env::var("USERPROFILE").unwrap_or("C:\\Users\\Default".to_string());
            let dirs_to_clean = [
                format!("{}\\.cargo\\registry\\cache", home),
                format!("{}\\.cargo\\registry\\src", home),
                format!("{}\\.cache", home),
                format!("{}\\AppData\\Local\\npm-cache", home),
            ];
            for dir in &dirs_to_clean {
                let _ = std::fs::remove_dir_all(dir);
            }

            // Re-check after cleanup
            let output2 = std::process::Command::new("powershell")
                .args(["-NoProfile", "-Command", "(Get-PSDrive C).Free / 1GB"])
                .output();

            let free_gb_after = match output2 {
                Ok(o) if o.status.success() => {
                    let stdout = String::from_utf8_lossy(&o.stdout);
                    stdout.trim().parse::<f64>().unwrap_or(free_gb)
                }
                _ => free_gb,
            };

            info!("After cleanup: {:.2}GB free", free_gb_after);

            if free_gb_after < threshold_gb / 2.0 {
                eprintln!("⚠ WARNING: C drive critically low ({:.2}GB free). Consider manual cleanup.", free_gb_after);
            }

            free_gb_after
        } else {
            free_gb
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        threshold_gb + 1.0 // Non-Windows: no-op
    }
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    init_logging(cli.verbose, cli.debug);
    info!("hermes-cli starting...");

    // Auto-check C drive space on Windows
    ensure_disk_space(2.0);

    let _config = Config::load()?;

    // Default to chat if no command specified
    let command = cli.command.unwrap_or(Commands::Chat {
        model: None,
        query: None,
        image: None,
        system: None,
        toolsets: None,
        skills: None,
        provider: None,
        chat_verbose: false,
        quiet: false,
        resume: cli.resume,
        continue_last: cli.continue_last,
        worktree: false,
        checkpoints: false,
        max_turns: None,
        yolo: false,
        pass_session_id: false,
        source: None,
    });

    match &command {
        Commands::Chat { model, query, system, provider, max_turns, yolo, quiet, .. } => {
            handle_chat(model.clone(), query.clone(), system.clone(), provider.clone(), *max_turns, *yolo, *quiet).await?;
        }
        Commands::Auth(ref cmd) => commands::handle_auth(cmd.clone()).await?,
        Commands::Model { current, global, model, portal_url: _, inference_url: _, client_id: _, scope: _, no_browser: _, timeout: _, ca_bundle: _, insecure: _ } =>
            commands::handle_model(*current, *global, model.as_deref())?,
        Commands::Tools(ref cmd) => commands::handle_tools(cmd.clone())?,
        Commands::Skills(ref cmd) => commands::handle_skills(cmd.clone())?,
        Commands::Gateway(ref cmd) => commands::handle_gateway(cmd.clone()).await?,
        Commands::Cron(ref cmd) => commands::handle_cron(cmd.clone()).await?,
        Commands::Config(ref cmd) => commands::handle_config(cmd.clone())?,
        Commands::Setup { section: _, skip_auth, skip_model, non_interactive: _, reset: _ } =>
            commands::handle_setup(*skip_auth, *skip_model)?,
        Commands::Doctor { all, check, fix: _ } => commands::handle_doctor(*all, check.as_deref())?,
        Commands::Status { all: _, deep: _ } => commands::handle_status()?,
        Commands::Version => { println!("hermes {}", env!("CARGO_PKG_VERSION")); }
        Commands::Update { gateway: _ } => commands::handle_update()?,
        Commands::Uninstall { full: _, yes: _ } => commands::handle_uninstall()?,
        // Stub handlers for new commands
        Commands::Sessions(cmd) => commands::handle_sessions(cmd.clone()),
        Commands::Logs { log_name, lines, follow, level, session, since, component } =>
            commands::handle_logs(log_name.as_deref(), *lines, *follow, level.as_deref(), session.as_deref(), since.as_deref(), component.as_deref())?,
        Commands::Profile(cmd) => commands::handle_profile(cmd.clone()),
        Commands::Mcp(cmd) => commands::handle_mcp(cmd.clone()),
        Commands::Memory(cmd) => commands::handle_memory(cmd.clone())?,
        Commands::Webhook(cmd) => commands::handle_webhook(cmd.clone()),
        Commands::Pairing(cmd) => commands::handle_pairing(cmd.clone()),
        Commands::Plugins(cmd) => commands::handle_plugins(cmd.clone()),
        Commands::Backup { output, quick, label } => commands::handle_backup(output.clone(), *quick, label.clone())?,
        Commands::Import { zipfile, force } => commands::handle_import(zipfile.clone(), *force)?,
        Commands::Debug(cmd) => commands::handle_debug(cmd.clone()),
        Commands::Dump { show_keys } => commands::handle_dump(*show_keys)?,
        Commands::Completion { shell } => commands::handle_completion(shell.as_deref()),
        Commands::Insights { days, source } => commands::handle_insights(*days, source.as_deref())?,
        Commands::Login { provider, portal_url, inference_url, client_id, scope, no_browser, timeout, ca_bundle, insecure } =>
            commands::handle_login(provider.as_deref(), portal_url.as_deref(), inference_url.as_deref(), client_id.as_deref(), scope.as_deref(), *no_browser, *timeout, ca_bundle.as_deref(), *insecure)?,
        Commands::Logout { provider } => commands::handle_logout(provider.as_deref())?,
        Commands::Whatsapp => commands::handle_whatsapp()?,
        Commands::Acp => commands::handle_acp()?,
        Commands::Dashboard { port, host, no_open } => commands::handle_dashboard(*port, host.to_string(), *no_open)?,
        Commands::Claw(cmd) => commands::handle_claw(cmd.clone()),
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

/// Handle the `hermes chat` command — wire CLI to runtime
async fn handle_chat(
    model: Option<String>,
    query: Option<String>,
    system: Option<String>,
    provider: Option<String>,
    max_turns: Option<u32>,
    yolo: bool,
    quiet: bool,
) -> anyhow::Result<()> {
    use hermes_runtime::{Agent, AgentConfig, ChatRepl};
    use hermes_runtime::provider::create_provider;
    use hermes_runtime::tool::{ToolRegistry, terminal::TerminalTool, file::{FileReadTool, FileWriteTool, FileSearchTool}, web::WebSearchTool, mcp::McpTool, browser::BrowserTool};
    use hermes_session_db::SessionStore;

    // Load user config (config.yaml)
    let user_config = crate::config::Config::load().unwrap_or_default();

    // Resolve provider: CLI flag > config > default
    let provider_str = provider.as_deref()
        .or_else(|| if user_config.model.provider.is_empty() { None } else { Some(&user_config.model.provider) })
        .unwrap_or("openai");
    let provider_type = hermes_common::Provider::from_str(provider_str)
        .unwrap_or(hermes_common::Provider::OpenAI);

    // Resolve API key: credential pool (round-robin) > env var
    let auth_store = crate::auth::AuthStore::load().unwrap_or_default();
    let cred_pool = crate::credential_pool::CredentialPool::from_auth_store(&auth_store);
    let api_key = cred_pool.get(provider_str)
        .map(|c| c.api_key)
        .or_else(|| std::env::var(format!("{}_API_KEY", provider_str.to_uppercase())).ok())
        .or_else(|| std::env::var("OPENAI_API_KEY").ok());

    let api_key = match api_key {
        Some(key) if !key.is_empty() => key,
        _ => {
            anyhow::bail!(
                "No API key configured for '{}'. Run: hermes auth add {} --api-key <KEY>",
                provider_str, provider_str
            );
        }
    };

    // Resolve base_url: pool entry > config > provider default
    let base_url_owned = auth_store.get(provider_str)
        .and_then(|c| c.base_url.clone())
        .or_else(|| if user_config.model.base_url.is_empty() { None } else { Some(user_config.model.base_url.clone()) });
    let base_url = base_url_owned.as_deref();

    // Resolve model: CLI flag > config > provider default
    let model = model.unwrap_or_else(|| {
        if !user_config.model.default.is_empty() { user_config.model.default.clone() }
        else { create_provider(&provider_type, &api_key, base_url).default_model().to_string() }
    });

    let provider_box = create_provider(&provider_type, &api_key, base_url);

    // Create tool registry
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(TerminalTool::new()));
    registry.register(Box::new(FileReadTool));
    registry.register(Box::new(FileWriteTool));
    registry.register(Box::new(FileSearchTool));
    registry.register(Box::new(WebSearchTool::new()));
    registry.register(Box::new(McpTool));
    registry.register(Box::new(BrowserTool));

    // Create session store using HERMES_HOME
    let home = crate::config::Config::hermes_home();
    let db_path = home.join("sessions.db");
    let session_store = SessionStore::new(&db_path)
        .map_err(|e| anyhow::anyhow!("Failed to open session DB: {}", e))?;

    // Create agent config: CLI flags override config.yaml
    let agent_config = AgentConfig {
        max_turns: max_turns.unwrap_or(user_config.agent.max_turns),
        system_prompt: system.unwrap_or_else(|| user_config.agent.system_prompt.clone()),
        timeout_secs: user_config.terminal.timeout,
        yolo,
        max_context_tokens: 128_000,
        streaming: user_config.display.streaming,
    };

    // Create agent
    let agent = Agent::new(provider_box, registry, session_store, agent_config, model.clone());

    if let Some(q) = query {
        // Single-shot mode
        let response = ChatRepl::run_query(agent, &q).await
            .map_err(|e| anyhow::anyhow!("Query failed: {}", e))?;
        println!("{}", response);
    } else {
        // Interactive REPL mode
        if !quiet {
            println!("Hermes Agent v{} — model: {}", env!("CARGO_PKG_VERSION"), model);
            println!("Type /help for commands, /quit to exit");
            println!();
        }

        let mut repl = ChatRepl::new(agent)
            .map_err(|e| anyhow::anyhow!("Failed to create REPL: {}", e))?;

        // Async REPL loop with Ctrl+C handling
        use std::io::Write;
        use tokio::io::{AsyncBufReadExt, BufReader};

        let stdin = BufReader::new(tokio::io::stdin());
        let mut lines = stdin.lines();

        loop {
            if !quiet {
                print!("> ");
                let _ = std::io::stdout().flush();
            }

            // Race between next input line and Ctrl+C
            let line = tokio::select! {
                result = lines.next_line() => {
                    match result {
                        Ok(Some(line)) => line,
                        Ok(None) => break, // EOF
                        Err(e) => anyhow::bail!("Input error: {}", e),
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    println!("\nInterrupted. Saving session...");
                    let session_id = repl.graceful_shutdown();
                    println!("Session {} saved. Goodbye!", session_id);
                    std::process::exit(0);
                }
            };

            let input = line.trim();
            if input.is_empty() {
                continue;
            }

            // Run the turn, but allow Ctrl+C to cancel long-running LLM calls
            let turn_result = tokio::select! {
                result = repl.run_turn(input) => result,
                _ = tokio::signal::ctrl_c() => {
                    println!("\nInterrupted. Saving session...");
                    let session_id = repl.graceful_shutdown();
                    println!("Session {} saved. Goodbye!", session_id);
                    std::process::exit(0);
                }
            };

            match turn_result {
                Ok(response) => println!("{}", response.content),
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("REPL exited") {
                        let session_id = repl.graceful_shutdown();
                        println!("Session {} saved. Goodbye!", session_id);
                        break;
                    }
                    eprintln!("Error: {}", msg);
                }
            }
        }
    }

    Ok(())
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // === Chat ===
    #[test]
    fn test_cli_parse_chat() {
        let cli = Cli::parse_from(vec!["hermes", "chat", "gpt-4"]);
        if let Commands::Chat { model, .. } = cli.command.unwrap() {
            assert_eq!(model, Some("gpt-4".to_string()));
        } else { panic!("expected Chat"); }
    }

    #[test]
    fn test_cli_parse_chat_with_system() {
        let cli = Cli::parse_from(vec!["hermes", "chat", "gpt-4", "--system", "You are helpful"]);
        if let Commands::Chat { model, system, .. } = cli.command.unwrap() {
            assert_eq!(model, Some("gpt-4".to_string()));
            assert_eq!(system, Some("You are helpful".to_string()));
        } else { panic!("expected Chat"); }
    }

    #[test]
    fn test_cli_parse_chat_with_query() {
        let cli = Cli::parse_from(vec!["hermes", "chat", "-q", "hello world"]);
        if let Commands::Chat { query, .. } = cli.command.unwrap() {
            assert_eq!(query, Some("hello world".to_string()));
        } else { panic!("expected Chat"); }
    }

    #[test]
    fn test_cli_parse_chat_with_provider() {
        let cli = Cli::parse_from(vec!["hermes", "chat", "--provider", "anthropic"]);
        if let Commands::Chat { provider, .. } = cli.command.unwrap() {
            assert_eq!(provider, Some("anthropic".to_string()));
        } else { panic!("expected Chat"); }
    }

    #[test]
    fn test_cli_parse_chat_with_toolsets() {
        let cli = Cli::parse_from(vec!["hermes", "chat", "--toolsets", "web,memory"]);
        if let Commands::Chat { toolsets, .. } = cli.command.unwrap() {
            assert_eq!(toolsets, Some("web,memory".to_string()));
        } else { panic!("expected Chat"); }
    }

    #[test]
    fn test_cli_parse_chat_yolo() {
        let cli = Cli::parse_from(vec!["hermes", "chat", "--yolo"]);
        if let Commands::Chat { yolo, .. } = cli.command.unwrap() {
            assert!(yolo);
        } else { panic!("expected Chat"); }
}

    #[test]
    fn test_cli_parse_auth_add() {
        let cli = Cli::parse_from(vec!["hermes", "auth", "add", "openai", "--api-key", "sk-test123"]);
        if let Commands::Auth(AuthCommand::Add { provider, api_key, .. }) = cli.command.unwrap() {
            assert_eq!(provider, "openai");
            assert_eq!(api_key, Some("sk-test123".to_string()));
        } else { panic!("expected Auth::Add"); }
    }

    #[test]
    fn test_cli_parse_auth_add_with_base_url() {
        let cli = Cli::parse_from(vec!["hermes", "auth", "add", "custom", "--api-key", "key123", "--base-url", "https://api.example.com"]);
        if let Commands::Auth(AuthCommand::Add { provider, api_key, base_url, .. }) = cli.command.unwrap() {
            assert_eq!(provider, "custom");
            assert_eq!(api_key, Some("key123".to_string()));
            assert_eq!(base_url, Some("https://api.example.com".to_string()));
        } else { panic!("expected Auth::Add"); }
    }

    #[test]
    fn test_cli_parse_auth_add_with_type() {
        let cli = Cli::parse_from(vec!["hermes", "auth", "add", "nous", "--type", "oauth"]);
        if let Commands::Auth(AuthCommand::Add { provider, auth_type, .. }) = cli.command.unwrap() {
            assert_eq!(provider, "nous");
            assert_eq!(auth_type, Some("oauth".to_string()));
        } else { panic!("expected Auth::Add"); }
    }

    #[test]
    fn test_cli_parse_auth_list() {
        let cli = Cli::parse_from(vec!["hermes", "auth", "list"]);
        if let Commands::Auth(AuthCommand::List { provider }) = cli.command.unwrap() {
            assert!(provider.is_none());
        } else { panic!("expected Auth::List"); }
    }

    #[test]
    fn test_cli_parse_auth_list_with_provider() {
        let cli = Cli::parse_from(vec!["hermes", "auth", "list", "openai"]);
        if let Commands::Auth(AuthCommand::List { provider }) = cli.command.unwrap() {
            assert_eq!(provider, Some("openai".to_string()));
        } else { panic!("expected Auth::List"); }
    }

    #[test]
    fn test_cli_parse_auth_remove() {
        let cli = Cli::parse_from(vec!["hermes", "auth", "remove", "openai"]);
        if let Commands::Auth(AuthCommand::Remove { provider, .. }) = cli.command.unwrap() {
            assert_eq!(provider, "openai");
        } else { panic!("expected Auth::Remove"); }
    }

    #[test]
    fn test_cli_parse_auth_reset() {
        let cli = Cli::parse_from(vec!["hermes", "auth", "reset"]);
        assert!(matches!(cli.command.unwrap(), Commands::Auth(AuthCommand::Reset { provider: None })));
    }

    // === Model ===
    #[test]
    fn test_cli_parse_model_current() {
        let cli = Cli::parse_from(vec!["hermes", "model", "--current"]);
        if let Commands::Model { current, global, model, .. } = cli.command.unwrap() {
            assert!(current);
            assert!(!global);
            assert_eq!(model, None);
        } else { panic!("expected Model"); }
    }

    #[test]
    fn test_cli_parse_model_global() {
        let cli = Cli::parse_from(vec!["hermes", "model", "--global", "claude-3"]);
        if let Commands::Model { current, global, model, .. } = cli.command.unwrap() {
            assert!(!current);
            assert!(global);
            assert_eq!(model, Some("claude-3".to_string()));
        } else { panic!("expected Model"); }
    }

    #[test]
    fn test_cli_parse_model_session() {
        let cli = Cli::parse_from(vec!["hermes", "model", "gpt-4o"]);
        if let Commands::Model { current, global, model, .. } = cli.command.unwrap() {
            assert!(!current);
            assert!(!global);
            assert_eq!(model, Some("gpt-4o".to_string()));
        } else { panic!("expected Model"); }
    }

    // === Tools ===
    #[test]
    fn test_cli_parse_tools_list() {
        let cli = Cli::parse_from(vec!["hermes", "tools", "list"]);
        if let Commands::Tools(ToolsCommand::List { all, platform }) = cli.command.unwrap() {
            assert!(!all);
            assert_eq!(platform, "cli");
        } else { panic!("expected Tools::List"); }
    }

    #[test]
    fn test_cli_parse_tools_list_all() {
        let cli = Cli::parse_from(vec!["hermes", "tools", "list", "--all", "--platform", "telegram"]);
        if let Commands::Tools(ToolsCommand::List { all, platform }) = cli.command.unwrap() {
            assert!(all);
            assert_eq!(platform, "telegram");
        } else { panic!("expected Tools::List"); }
    }

    #[test]
    fn test_cli_parse_tools_disable() {
        let cli = Cli::parse_from(vec!["hermes", "tools", "disable", "web_search", "memory"]);
        if let Commands::Tools(ToolsCommand::Disable { names, platform }) = cli.command.unwrap() {
            assert_eq!(names, vec!["web_search", "memory"]);
            assert_eq!(platform, "cli");
        } else { panic!("expected Tools::Disable"); }
    }

    #[test]
    fn test_cli_parse_tools_enable() {
        let cli = Cli::parse_from(vec!["hermes", "tools", "enable", "web_search", "--platform", "discord"]);
        if let Commands::Tools(ToolsCommand::Enable { names, platform }) = cli.command.unwrap() {
            assert_eq!(names, vec!["web_search"]);
            assert_eq!(platform, "discord");
        } else { panic!("expected Tools::Enable"); }
    }

    // === Skills ===
    #[test]
    fn test_cli_parse_skills_search() {
        let cli = Cli::parse_from(vec!["hermes", "skills", "search", "web"]);
        if let Commands::Skills(SkillsCommand::Search { query, source, limit, .. }) = cli.command.unwrap() {
            assert_eq!(query, Some("web".to_string()));
            assert_eq!(source, "all");
            assert_eq!(limit, 10);
        } else { panic!("expected Skills::Search"); }
    }

    #[test]
    fn test_cli_parse_skills_browse() {
        let cli = Cli::parse_from(vec!["hermes", "skills", "browse"]);
        assert!(matches!(cli.command.unwrap(), Commands::Skills(SkillsCommand::Browse { .. })));
    }

    #[test]
    fn test_cli_parse_skills_inspect() {
        let cli = Cli::parse_from(vec!["hermes", "skills", "inspect", "web-search"]);
        if let Commands::Skills(SkillsCommand::Inspect { name }) = cli.command.unwrap() {
            assert_eq!(name, "web-search");
        } else { panic!("expected Skills::Inspect"); }
    }

    #[test]
    fn test_cli_parse_skills_install() {
        let cli = Cli::parse_from(vec!["hermes", "skills", "install", "openai/skills/skill-creator", "--force"]);
        if let Commands::Skills(SkillsCommand::Install { identifier, force, .. }) = cli.command.unwrap() {
            assert_eq!(identifier, "openai/skills/skill-creator");
            assert!(force);
        } else { panic!("expected Skills::Install"); }
    }

    #[test]
    fn test_cli_parse_skills_list() {
        let cli = Cli::parse_from(vec!["hermes", "skills", "list", "--source", "hub"]);
        if let Commands::Skills(SkillsCommand::List { source }) = cli.command.unwrap() {
            assert_eq!(source, "hub");
        } else { panic!("expected Skills::List"); }
    }

    #[test]
    fn test_cli_parse_skills_check() { assert!(matches!(Cli::parse_from(vec!["hermes", "skills", "check"]).command.unwrap(), Commands::Skills(SkillsCommand::Check { .. }))); }
    #[test]
    fn test_cli_parse_skills_update() { assert!(matches!(Cli::parse_from(vec!["hermes", "skills", "update"]).command.unwrap(), Commands::Skills(SkillsCommand::Update { .. }))); }
    #[test]
    fn test_cli_parse_skills_audit() { assert!(matches!(Cli::parse_from(vec!["hermes", "skills", "audit"]).command.unwrap(), Commands::Skills(SkillsCommand::Audit { .. }))); }
    #[test]
    fn test_cli_parse_skills_uninstall() { assert!(matches!(Cli::parse_from(vec!["hermes", "skills", "uninstall", "foo"]).command.unwrap(), Commands::Skills(SkillsCommand::Uninstall { .. }))); }

    // === Gateway ===
    #[test]
    fn test_cli_parse_gateway_run() {
        let cli = Cli::parse_from(vec!["hermes", "gateway", "run"]);
        if let Commands::Gateway(GatewayCommand::Run { platform, .. }) = cli.command.unwrap() {
            assert_eq!(platform, None);
        } else { panic!("expected Gateway::Run"); }
    }

    #[test]
    fn test_cli_parse_gateway_run_with_platform() {
        let cli = Cli::parse_from(vec!["hermes", "gateway", "run", "-P", "telegram"]);
        if let Commands::Gateway(GatewayCommand::Run { platform, .. }) = cli.command.unwrap() {
            assert_eq!(platform, Some("telegram".to_string()));
        } else { panic!("expected Gateway::Run"); }
    }

    #[test]
    fn test_cli_parse_gateway_start() { assert!(matches!(Cli::parse_from(vec!["hermes", "gateway", "start"]).command.unwrap(), Commands::Gateway(GatewayCommand::Start { .. }))); }
    #[test]
    fn test_cli_parse_gateway_stop() { assert!(matches!(Cli::parse_from(vec!["hermes", "gateway", "stop"]).command.unwrap(), Commands::Gateway(GatewayCommand::Stop { .. }))); }
    #[test]
    fn test_cli_parse_gateway_restart() { assert!(matches!(Cli::parse_from(vec!["hermes", "gateway", "restart"]).command.unwrap(), Commands::Gateway(GatewayCommand::Restart { .. }))); }
    #[test]
    fn test_cli_parse_gateway_status() { assert!(matches!(Cli::parse_from(vec!["hermes", "gateway", "status"]).command.unwrap(), Commands::Gateway(GatewayCommand::Status { .. }))); }
    #[test]
    fn test_cli_parse_gateway_setup() {
        let cli = Cli::parse_from(vec!["hermes", "gateway", "setup", "telegram"]);
        if let Commands::Gateway(GatewayCommand::Setup { platform }) = cli.command.unwrap() {
            assert_eq!(platform, Some("telegram".to_string()));
        } else { panic!("expected Gateway::Setup"); }
    }
    #[test]
    fn test_cli_parse_gateway_install() { assert!(matches!(Cli::parse_from(vec!["hermes", "gateway", "install"]).command.unwrap(), Commands::Gateway(GatewayCommand::Install { .. }))); }
    #[test]
    fn test_cli_parse_gateway_uninstall() { assert!(matches!(Cli::parse_from(vec!["hermes", "gateway", "uninstall"]).command.unwrap(), Commands::Gateway(GatewayCommand::Uninstall { .. }))); }

    // === Cron ===
    #[test]
    fn test_cli_parse_cron_list() { assert!(matches!(Cli::parse_from(vec!["hermes", "cron", "list"]).command.unwrap(), Commands::Cron(CronCommand::List { .. }))); }
    #[test]
    fn test_cli_parse_cron_add() {
        let cli = Cli::parse_from(vec!["hermes", "cron", "add", "every 30m", "check status"]);
        if let Commands::Cron(CronCommand::Add { schedule, command, .. }) = cli.command.unwrap() {
            assert_eq!(schedule, "every 30m");
            assert_eq!(command, Some("check status".to_string()));
        } else { panic!("expected Cron::Add"); }
    }
    #[test]
    fn test_cli_parse_cron_remove() { assert!(matches!(Cli::parse_from(vec!["hermes", "cron", "remove", "abc123"]).command.unwrap(), Commands::Cron(CronCommand::Remove { .. }))); }
    #[test]
    fn test_cli_parse_cron_pause() { assert!(matches!(Cli::parse_from(vec!["hermes", "cron", "pause", "abc123"]).command.unwrap(), Commands::Cron(CronCommand::Pause { .. }))); }
    #[test]
    fn test_cli_parse_cron_resume() { assert!(matches!(Cli::parse_from(vec!["hermes", "cron", "resume", "abc123"]).command.unwrap(), Commands::Cron(CronCommand::Resume { .. }))); }
    #[test]
    fn test_cli_parse_cron_run() { assert!(matches!(Cli::parse_from(vec!["hermes", "cron", "run", "abc123"]).command.unwrap(), Commands::Cron(CronCommand::Run { .. }))); }
    #[test]
    fn test_cli_parse_cron_status() { assert!(matches!(Cli::parse_from(vec!["hermes", "cron", "status"]).command.unwrap(), Commands::Cron(CronCommand::Status))); }
    #[test]
    fn test_cli_parse_cron_tick() { assert!(matches!(Cli::parse_from(vec!["hermes", "cron", "tick"]).command.unwrap(), Commands::Cron(CronCommand::Tick))); }
    #[test]
    fn test_cli_parse_cron_edit() { assert!(matches!(Cli::parse_from(vec!["hermes", "cron", "edit", "abc123"]).command.unwrap(), Commands::Cron(CronCommand::Edit { .. }))); }

    // === Config ===
    #[test]
    fn test_cli_parse_config_show() { assert!(matches!(Cli::parse_from(vec!["hermes", "config", "show"]).command.unwrap(), Commands::Config(ConfigCommand::Show))); }
    #[test]
    fn test_cli_parse_config_get() {
        let cli = Cli::parse_from(vec!["hermes", "config", "get", "model.default"]);
        if let Commands::Config(ConfigCommand::Get { key }) = cli.command.unwrap() {
            assert_eq!(key, "model.default");
        } else { panic!("expected Config::Get"); }
    }
    #[test]
    fn test_cli_parse_config_set() {
        let cli = Cli::parse_from(vec!["hermes", "config", "set", "model.default", "gpt-4"]);
        if let Commands::Config(ConfigCommand::Set { key, value }) = cli.command.unwrap() {
            assert_eq!(key, "model.default"); assert_eq!(value, "gpt-4");
        } else { panic!("expected Config::Set"); }
    }
    #[test]
    fn test_cli_parse_config_reset() { assert!(matches!(Cli::parse_from(vec!["hermes", "config", "reset"]).command.unwrap(), Commands::Config(ConfigCommand::Reset))); }
    #[test]
    fn test_cli_parse_config_edit() { assert!(matches!(Cli::parse_from(vec!["hermes", "config", "edit"]).command.unwrap(), Commands::Config(ConfigCommand::Edit))); }
    #[test]
    fn test_cli_parse_config_path() { assert!(matches!(Cli::parse_from(vec!["hermes", "config", "path"]).command.unwrap(), Commands::Config(ConfigCommand::Path))); }
    #[test]
    fn test_cli_parse_config_env_path() { assert!(matches!(Cli::parse_from(vec!["hermes", "config", "env-path"]).command.unwrap(), Commands::Config(ConfigCommand::EnvPath))); }
    #[test]
    fn test_cli_parse_config_check() { assert!(matches!(Cli::parse_from(vec!["hermes", "config", "check"]).command.unwrap(), Commands::Config(ConfigCommand::Check))); }
    #[test]
    fn test_cli_parse_config_migrate() { assert!(matches!(Cli::parse_from(vec!["hermes", "config", "migrate"]).command.unwrap(), Commands::Config(ConfigCommand::Migrate))); }

    // === Setup / Doctor ===
    #[test]
    fn test_cli_parse_setup() {
        let cli = Cli::parse_from(vec!["hermes", "setup"]);
        if let Commands::Setup { skip_auth, skip_model, .. } = cli.command.unwrap() {
            assert!(!skip_auth); assert!(!skip_model);
        } else { panic!("expected Setup"); }
    }
    #[test]
    fn test_cli_parse_setup_skip_auth() {
        let cli = Cli::parse_from(vec!["hermes", "setup", "--skip-auth"]);
        if let Commands::Setup { skip_auth, skip_model, .. } = cli.command.unwrap() {
            assert!(skip_auth); assert!(!skip_model);
        } else { panic!("expected Setup"); }
    }
    #[test]
    fn test_cli_parse_setup_section() {
        let cli = Cli::parse_from(vec!["hermes", "setup", "gateway"]);
        if let Commands::Setup { section, .. } = cli.command.unwrap() {
            assert_eq!(section, Some("gateway".to_string()));
        } else { panic!("expected Setup"); }
    }

    #[test]
    fn test_cli_parse_doctor() {
        let cli = Cli::parse_from(vec!["hermes", "doctor"]);
        if let Commands::Doctor { all, check, .. } = cli.command.unwrap() {
            assert!(!all); assert_eq!(check, None);
        } else { panic!("expected Doctor"); }
    }
    #[test]
    fn test_cli_parse_doctor_fix() {
        let cli = Cli::parse_from(vec!["hermes", "doctor", "--fix"]);
        if let Commands::Doctor { fix, .. } = cli.command.unwrap() {
            assert!(fix);
        } else { panic!("expected Doctor"); }
    }

    // === Status ===
    #[test]
    fn test_cli_parse_status() {
        let cli = Cli::parse_from(vec!["hermes", "status"]);
        if let Commands::Status { all, deep } = cli.command.unwrap() {
            assert!(!all); assert!(!deep);
        } else { panic!("expected Status"); }
    }
    #[test]
    fn test_cli_parse_status_all() {
        let cli = Cli::parse_from(vec!["hermes", "status", "--all", "--deep"]);
        if let Commands::Status { all, deep } = cli.command.unwrap() {
            assert!(all); assert!(deep);
        } else { panic!("expected Status"); }
    }

    // === Sessions ===
    #[test]
    fn test_cli_parse_sessions_list() { assert!(matches!(Cli::parse_from(vec!["hermes", "sessions", "list"]).command.unwrap(), Commands::Sessions(SessionsCommand::List { .. }))); }
    #[test]
    fn test_cli_parse_sessions_export() { assert!(matches!(Cli::parse_from(vec!["hermes", "sessions", "export", "out.json"]).command.unwrap(), Commands::Sessions(SessionsCommand::Export { .. }))); }
    #[test]
    fn test_cli_parse_sessions_delete() { assert!(matches!(Cli::parse_from(vec!["hermes", "sessions", "delete", "abc123"]).command.unwrap(), Commands::Sessions(SessionsCommand::Delete { .. }))); }
    #[test]
    fn test_cli_parse_sessions_prune() { assert!(matches!(Cli::parse_from(vec!["hermes", "sessions", "prune"]).command.unwrap(), Commands::Sessions(SessionsCommand::Prune { .. }))); }
    #[test]
    fn test_cli_parse_sessions_stats() { assert!(matches!(Cli::parse_from(vec!["hermes", "sessions", "stats"]).command.unwrap(), Commands::Sessions(SessionsCommand::Stats))); }
    #[test]
    fn test_cli_parse_sessions_rename() { assert!(matches!(Cli::parse_from(vec!["hermes", "sessions", "rename", "abc123", "My", "Session"]).command.unwrap(), Commands::Sessions(SessionsCommand::Rename { .. }))); }
    #[test]
    fn test_cli_parse_sessions_browse() { assert!(matches!(Cli::parse_from(vec!["hermes", "sessions", "browse"]).command.unwrap(), Commands::Sessions(SessionsCommand::Browse { .. }))); }

    // === Logs ===
    #[test]
    fn test_cli_parse_logs() {
        let cli = Cli::parse_from(vec!["hermes", "logs"]);
        if let Commands::Logs { log_name, lines, .. } = cli.command.unwrap() {
            assert_eq!(log_name, None); assert_eq!(lines, 50);
        } else { panic!("expected Logs"); }
    }
    #[test]
    fn test_cli_parse_logs_with_options() {
        let cli = Cli::parse_from(vec!["hermes", "logs", "errors", "--lines", "100", "-f", "--level", "WARNING"]);
        if let Commands::Logs { log_name, lines, follow, level, .. } = cli.command.unwrap() {
            assert_eq!(log_name, Some("errors".to_string()));
            assert_eq!(lines, 100);
            assert!(follow);
            assert_eq!(level, Some("WARNING".to_string()));
        } else { panic!("expected Logs"); }
    }

    // === Profile ===
    #[test]
    fn test_cli_parse_profile_list() { assert!(matches!(Cli::parse_from(vec!["hermes", "profile", "list"]).command.unwrap(), Commands::Profile(ProfileCommand::List))); }
    #[test]
    fn test_cli_parse_profile_use() { assert!(matches!(Cli::parse_from(vec!["hermes", "profile", "use", "work"]).command.unwrap(), Commands::Profile(ProfileCommand::Use { .. }))); }
    #[test]
    fn test_cli_parse_profile_create() { assert!(matches!(Cli::parse_from(vec!["hermes", "profile", "create", "test"]).command.unwrap(), Commands::Profile(ProfileCommand::Create { .. }))); }
    #[test]
    fn test_cli_parse_profile_delete() { assert!(matches!(Cli::parse_from(vec!["hermes", "profile", "delete", "test"]).command.unwrap(), Commands::Profile(ProfileCommand::Delete { .. }))); }

    // === MCP ===
    #[test]
    fn test_cli_parse_mcp_serve() { assert!(matches!(Cli::parse_from(vec!["hermes", "mcp", "serve"]).command.unwrap(), Commands::Mcp(McpCommand::Serve { .. }))); }
    #[test]
    fn test_cli_parse_mcp_add() { assert!(matches!(Cli::parse_from(vec!["hermes", "mcp", "add", "github"]).command.unwrap(), Commands::Mcp(McpCommand::Add { .. }))); }
    #[test]
    fn test_cli_parse_mcp_remove() { assert!(matches!(Cli::parse_from(vec!["hermes", "mcp", "remove", "github"]).command.unwrap(), Commands::Mcp(McpCommand::Remove { .. }))); }
    #[test]
    fn test_cli_parse_mcp_list() { assert!(matches!(Cli::parse_from(vec!["hermes", "mcp", "list"]).command.unwrap(), Commands::Mcp(McpCommand::List))); }

    // === Memory ===
    #[test]
    fn test_cli_parse_memory_setup() { assert!(matches!(Cli::parse_from(vec!["hermes", "memory", "setup"]).command.unwrap(), Commands::Memory(MemoryCommand::Setup))); }
    #[test]
    fn test_cli_parse_memory_status() { assert!(matches!(Cli::parse_from(vec!["hermes", "memory", "status"]).command.unwrap(), Commands::Memory(MemoryCommand::Status))); }
    #[test]
    fn test_cli_parse_memory_off() { assert!(matches!(Cli::parse_from(vec!["hermes", "memory", "off"]).command.unwrap(), Commands::Memory(MemoryCommand::Off))); }

    // === Webhook ===
    #[test]
    fn test_cli_parse_webhook_subscribe() { assert!(matches!(Cli::parse_from(vec!["hermes", "webhook", "subscribe", "test"]).command.unwrap(), Commands::Webhook(WebhookCommand::Subscribe { .. }))); }
    #[test]
    fn test_cli_parse_webhook_list() { assert!(matches!(Cli::parse_from(vec!["hermes", "webhook", "list"]).command.unwrap(), Commands::Webhook(WebhookCommand::List))); }

    // === Pairing ===
    #[test]
    fn test_cli_parse_pairing_list() { assert!(matches!(Cli::parse_from(vec!["hermes", "pairing", "list"]).command.unwrap(), Commands::Pairing(PairingCommand::List))); }
    #[test]
    fn test_cli_parse_pairing_approve() { assert!(matches!(Cli::parse_from(vec!["hermes", "pairing", "approve", "telegram", "ABC123"]).command.unwrap(), Commands::Pairing(PairingCommand::Approve { .. }))); }

    // === Plugins ===
    #[test]
    fn test_cli_parse_plugins_install() { assert!(matches!(Cli::parse_from(vec!["hermes", "plugins", "install", "foo/bar"]).command.unwrap(), Commands::Plugins(PluginsCommand::Install { .. }))); }
    #[test]
    fn test_cli_parse_plugins_list() { assert!(matches!(Cli::parse_from(vec!["hermes", "plugins", "list"]).command.unwrap(), Commands::Plugins(PluginsCommand::List))); }

    // === Backup / Import ===
    #[test]
    fn test_cli_parse_backup() {
        let cli = Cli::parse_from(vec!["hermes", "backup", "--quick"]);
        if let Commands::Backup { quick, .. } = cli.command.unwrap() { assert!(quick); } else { panic!("expected Backup"); }
    }
    #[test]
    fn test_cli_parse_import() {
        let cli = Cli::parse_from(vec!["hermes", "import", "backup.zip", "--force"]);
        if let Commands::Import { zipfile, force } = cli.command.unwrap() {
            assert_eq!(zipfile, "backup.zip"); assert!(force);
        } else { panic!("expected Import"); }
    }

    // === Debug / Dump ===
    #[test]
    fn test_cli_parse_debug_share() { assert!(matches!(Cli::parse_from(vec!["hermes", "debug", "share"]).command.unwrap(), Commands::Debug(DebugCommand::Share { .. }))); }
    #[test]
    fn test_cli_parse_dump() { assert!(matches!(Cli::parse_from(vec!["hermes", "dump"]).command.unwrap(), Commands::Dump { .. })); }

    // === Completion / Insights ===
    #[test]
    fn test_cli_parse_completion() { assert!(matches!(Cli::parse_from(vec!["hermes", "completion", "bash"]).command.unwrap(), Commands::Completion { .. })); }
    #[test]
    fn test_cli_parse_insights() { assert!(matches!(Cli::parse_from(vec!["hermes", "insights"]).command.unwrap(), Commands::Insights { .. })); }

    // === Login / Logout ===
    #[test]
    fn test_cli_parse_login() { assert!(matches!(Cli::parse_from(vec!["hermes", "login"]).command.unwrap(), Commands::Login { .. })); }
    #[test]
    fn test_cli_parse_logout() { assert!(matches!(Cli::parse_from(vec!["hermes", "logout"]).command.unwrap(), Commands::Logout { .. })); }

    // === WhatsApp / ACP / Dashboard ===
    #[test]
    fn test_cli_parse_whatsapp() { assert!(matches!(Cli::parse_from(vec!["hermes", "whatsapp"]).command.unwrap(), Commands::Whatsapp)); }
    #[test]
    fn test_cli_parse_acp() { assert!(matches!(Cli::parse_from(vec!["hermes", "acp"]).command.unwrap(), Commands::Acp)); }
    #[test]
    fn test_cli_parse_dashboard() { assert!(matches!(Cli::parse_from(vec!["hermes", "dashboard"]).command.unwrap(), Commands::Dashboard { .. })); }

    // === Claw ===
    #[test]
    fn test_cli_parse_claw_migrate() { assert!(matches!(Cli::parse_from(vec!["hermes", "claw", "migrate"]).command.unwrap(), Commands::Claw(ClawCommand::Migrate { .. }))); }
    #[test]
    fn test_cli_parse_claw_cleanup() { assert!(matches!(Cli::parse_from(vec!["hermes", "claw", "cleanup"]).command.unwrap(), Commands::Claw(ClawCommand::Cleanup { .. }))); }

    // === Version / Update / Uninstall ===
    #[test]
    fn test_cli_parse_version() { assert!(matches!(Cli::parse_from(vec!["hermes", "version"]).command.unwrap(), Commands::Version)); }
    #[test]
    fn test_cli_parse_update() { assert!(matches!(Cli::parse_from(vec!["hermes", "update"]).command.unwrap(), Commands::Update { .. })); }
    #[test]
    fn test_cli_parse_update_gateway() {
        let cli = Cli::parse_from(vec!["hermes", "update", "--gateway"]);
        if let Commands::Update { gateway } = cli.command.unwrap() { assert!(gateway); } else { panic!("expected Update"); }
    }
    #[test]
    fn test_cli_parse_uninstall() { assert!(matches!(Cli::parse_from(vec!["hermes", "uninstall"]).command.unwrap(), Commands::Uninstall { .. })); }
    #[test]
    fn test_cli_parse_uninstall_full() {
        let cli = Cli::parse_from(vec!["hermes", "uninstall", "--full", "--yes"]);
        if let Commands::Uninstall { full, yes } = cli.command.unwrap() { assert!(full); assert!(yes); } else { panic!("expected Uninstall"); }
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
    #[test]
    fn test_cli_parse_resume_global() {
        let cli = Cli::parse_from(vec!["hermes", "--resume", "abc123"]);
        assert_eq!(cli.resume, Some("abc123".to_string()));
    }
}