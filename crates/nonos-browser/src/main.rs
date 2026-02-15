//! NONOS Browser - Main entry point
//!
//! Usage: nonos [OPTIONS] [URL]

use clap::Parser;
use nonos_browser::Browser;
use nonos_types::NonosResult;
use tracing::{error, info};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Parser)]
#[command(name = "nonos")]
#[command(about = "NONOS - Privacy-first trustless browser", version)]
#[command(author = "NON-OS <team@nonos.systems>")]
struct Cli {
    /// URL to open
    url: Option<String>,

    /// Start with wallet unlocked (for development)
    #[arg(long)]
    with_wallet: bool,

    /// Security level (standard, safer, safest)
    #[arg(long, default_value = "safer")]
    security: String,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> NonosResult<()> {
    let cli = Cli::parse();

    // Initialize logging
    let filter = if cli.debug {
        "nonos=debug,nonos_browser=debug,nonos_anyone=debug,nonos_wallet=debug"
    } else {
        "nonos=info"
    };

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env().add_directive(filter.parse().unwrap()))
        .init();

    info!("Starting NONOS browser");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));

    // Create browser instance
    let browser = Browser::new();

    // Set security level
    let security_level = match cli.security.as_str() {
        "standard" => nonos_types::SecurityLevel::Standard,
        "safer" => nonos_types::SecurityLevel::Safer,
        "safest" => nonos_types::SecurityLevel::Safest,
        _ => {
            error!("Invalid security level: {}", cli.security);
            nonos_types::SecurityLevel::Safer
        }
    };
    browser.set_security_level(security_level);

    // Initialize browser
    browser.initialize().await?;

    // Open initial URL
    let initial_url = cli.url.as_deref().unwrap_or("about:blank");
    let _tab_id = browser.new_tab(Some(initial_url)).await?;

    info!("Browser ready");

    // In production, this would start the GUI event loop
    // For now, we'll wait for shutdown signal
    info!("Press Ctrl+C to exit");
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install signal handler");

    info!("Shutting down...");
    browser.shutdown().await?;

    Ok(())
}
