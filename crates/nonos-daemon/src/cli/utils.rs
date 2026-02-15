use super::commands::Cli;
use nonos_daemon::ContractConfig;
use nonos_types::{EthAddress, NonosResult};
use tracing_subscriber::{fmt, prelude::*, EnvFilter, layer::SubscriberExt};

const BUILD_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn init_logging(cli: &Cli) {
    let level = if cli.quiet {
        "warn"
    } else {
        match cli.verbose {
            0 => "info",
            1 => "info,nonos_daemon=debug",
            2 => "debug",
            _ => "trace",
        }
    };

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level));

    let subscriber = tracing_subscriber::registry()
        .with(env_filter);

    if let Some(ref log_file) = cli.log_file {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file)
            .expect("Failed to open log file");
        let file_layer = fmt::layer()
            .with_writer(std::sync::Mutex::new(file))
            .with_ansi(false);
        subscriber.with(file_layer).init();
    } else {
        let stdout_layer = fmt::layer()
            .with_target(cli.verbose >= 2);
        subscriber.with(stdout_layer).init();
    }
}

pub fn print_banner() {
    println!("\x1b[38;5;46m");
    println!(r#"
    ███╗   ██╗ ██████╗ ███╗   ██╗ ██████╗ ███████╗
    ████╗  ██║██╔═══██╗████╗  ██║██╔═══██╗██╔════╝
    ██╔██╗ ██║██║   ██║██╔██╗ ██║██║   ██║███████╗
    ██║╚██╗██║██║   ██║██║╚██╗██║██║   ██║╚════██║
    ██║ ╚████║╚██████╔╝██║ ╚████║╚██████╔╝███████║
    ╚═╝  ╚═══╝ ╚═════╝ ╚═╝  ╚═══╝ ╚═════╝ ╚══════╝"#);
    println!("\x1b[0m");
    println!("        \x1b[38;5;245mDecentralized Node Daemon - v{}\x1b[0m", BUILD_VERSION);
    println!();
}

pub fn load_contract_config() -> NonosResult<ContractConfig> {
    Ok(ContractConfig {
        rpc_url: std::env::var("NONOS_RPC_URL")
            .unwrap_or_else(|_| "https://mainnet.base.org".to_string()),
        staking_address: std::env::var("NONOS_STAKING_CONTRACT")
            .ok()
            .map(|s| parse_eth_address(&s))
            .transpose()?
            .unwrap_or_else(EthAddress::zero),
        token_address: std::env::var("NONOS_TOKEN_CONTRACT")
            .ok()
            .map(|s| parse_eth_address(&s))
            .transpose()?
            .unwrap_or_else(EthAddress::zero),
        chain_id: std::env::var("NONOS_CHAIN_ID")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(8453),
    })
}

pub fn parse_eth_address(s: &str) -> NonosResult<EthAddress> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    if s.len() != 40 {
        return Err(nonos_types::NonosError::Config(
            format!("Invalid address length: {}", s.len())
        ));
    }
    let mut addr = [0u8; 20];
    for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
        let hex_str = std::str::from_utf8(chunk)
            .map_err(|e| nonos_types::NonosError::Config(format!("Invalid UTF-8: {}", e)))?;
        addr[i] = u8::from_str_radix(hex_str, 16)
            .map_err(|e| nonos_types::NonosError::Config(format!("Invalid hex: {}", e)))?;
    }
    Ok(EthAddress(addr))
}
