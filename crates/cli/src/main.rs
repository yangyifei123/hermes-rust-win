//! Hermes CLI - Main entry point
//!
//! Native Windows x64 CLI for Hermes Agent.

use anyhow::Result;
use std::process;

fn main() {
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("PANIC: {}", panic_info);
        if let Some(location) = panic_info.location() {
            eprintln!("  at {}:{}:{}", location.file(), location.line(), location.column());
        }
    }));

    // Spawn with larger stack to accommodate deep CLI dispatch in debug builds
    let builder = std::thread::Builder::new().stack_size(4 * 1024 * 1024);
    let handler = builder.spawn(|| {
        let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
        rt.block_on(async {
            if let Err(e) = run().await {
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        })
    }).expect("failed to spawn main thread");
    handler.join().expect("main thread panicked");
}

async fn run() -> Result<()> {
    hermes_cli_core::config::load_dotenv()?;
    hermes_cli_core::run().await
}
