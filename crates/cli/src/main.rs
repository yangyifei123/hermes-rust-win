//! Hermes CLI - Main entry point
//!
//! Native Windows x64 CLI for Hermes Agent.

use anyhow::Result;
use std::process;

#[tokio::main]
async fn main() {
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("PANIC: {}", panic_info);
        if let Some(location) = panic_info.location() {
            eprintln!("  at {}:{}:{}", location.file(), location.line(), location.column());
        }
    }));

    if let Err(e) = run().await {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

async fn run() -> Result<()> {
    hermes_cli_core::config::load_dotenv()?;
    hermes_cli_core::run().await
}
