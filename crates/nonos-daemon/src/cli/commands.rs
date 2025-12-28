// NONOS GNU AFFERO GENERAL PUBLIC LICENSE
// Version 3, 19 November 2007
// Copyright (C) 2025 NON-OS <team@nonos.systems>

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

const BUILD_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(name = "nonos")]
#[command(about = "NONOS Daemon - Decentralized node powering the NONOS browser")]
#[command(long_about = "NONOS powers the NONOS browser by participating in the decentralized network,\n\
                        providing ZK identity services, cache mixing, and earning NOX rewards.")]
#[command(version = BUILD_VERSION)]
#[command(author = "NON-OS <team@nonos.systems>")]
#[command(after_help = "For more information, visit: https://nonos.systems")]
pub struct Cli {
    /// Configuration file path
    #[arg(short, long, global = true, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Data directory for node storage
    #[arg(short = 'd', long, global = true, value_name = "DIR", env = "NONOS_DATA_DIR")]
    pub data_dir: Option<PathBuf>,

    /// Increase logging verbosity (-v info, -vv debug, -vvv trace)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Decrease logging verbosity (only show warnings and errors)
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Log to file instead of stdout
    #[arg(long, global = true, value_name = "FILE")]
    pub log_file: Option<PathBuf>,

    /// Output format
    #[arg(long, global = true, default_value = "text")]
    pub format: OutputFormat,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Clone, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start the node daemon
    Run {
        #[arg(short, long)]
        foreground: bool,
        #[arg(long, value_name = "FILE")]
        pid_file: Option<PathBuf>,
        #[arg(long)]
        systemd: bool,
    },
    /// Initialize node configuration and identity
    Init {
        #[arg(short, long)]
        force: bool,
        #[arg(long)]
        non_interactive: bool,
        #[arg(long)]
        reward_address: Option<String>,
        #[arg(long)]
        nickname: Option<String>,
    },
    /// Show node information and identity
    Info,
    /// Check daemon status (connects to running node)
    Status,
    /// Display current configuration
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },
    /// Run diagnostic checks
    Check {
        #[arg(long)]
        full: bool,
    },
    /// Manage ZK identities
    Identity {
        #[command(subcommand)]
        action: IdentityAction,
    },
    /// Manage cache mixer
    Mixer {
        #[command(subcommand)]
        action: MixerAction,
    },
    /// Launch Nyx-style TUI dashboard
    Dash {
        #[arg(long, default_value = "matrix")]
        theme: String,
    },
    /// Manage staking
    Stake {
        #[command(subcommand)]
        action: StakeAction,
    },
    /// Manage rewards
    Rewards {
        #[command(subcommand)]
        action: RewardsAction,
    },
    /// Show network statistics
    Stats,
    /// Manage peers
    Peers {
        #[command(subcommand)]
        action: Option<PeersAction>,
    },
    /// Generate systemd service file
    Systemd {
        #[arg(long, default_value = "/etc/systemd/system")]
        output_dir: PathBuf,
        #[arg(long, default_value = "nonos")]
        user: String,
    },
    /// Stop a running node daemon
    Stop {
        #[arg(short, long)]
        force: bool,
    },
    /// Reload configuration without restart
    Reload,
    /// Show version and build information
    Version,
}

#[derive(Subcommand)]
pub enum ConfigAction {
    Show,
    Validate,
    Set { key: String, value: String },
}

#[derive(Subcommand)]
pub enum IdentityAction {
    Generate {
        #[arg(long)]
        label: Option<String>,
    },
    List,
    Show { id: String },
    Export {
        id: String,
        #[arg(long, short)]
        output: Option<PathBuf>,
    },
    Import { file: PathBuf },
    Prove {
        id: String,
        #[arg(long)]
        challenge: Option<String>,
    },
    Verify { proof: String },
    Register { id: String },
}

#[derive(Subcommand)]
pub enum MixerAction {
    Status,
    Flush {
        #[arg(short, long)]
        force: bool,
    },
    Config {
        #[arg(long)]
        max_entries: Option<usize>,
        #[arg(long)]
        ttl: Option<u64>,
        #[arg(long)]
        enabled: Option<bool>,
    },
    Add { hash: String, data: String },
    Get { hash: String },
}

#[derive(Subcommand)]
pub enum StakeAction {
    Status,
    Deposit { amount: f64 },
    Tier { tier: String },
    Withdraw { amount: f64 },
    Tiers,
}

#[derive(Subcommand)]
pub enum RewardsAction {
    Status,
    Claim,
    Auto {
        #[arg(long, default_value = "100")]
        threshold: f64,
    },
    History {
        #[arg(long, default_value = "10")]
        limit: u32,
    },
    Debug {
        #[arg(long, default_value = "latest")]
        epoch: String,
    },
}

#[derive(Subcommand)]
pub enum PeersAction {
    List,
    Show { peer_id: String },
    Ban { peer_id: String },
    Unban { peer_id: String },
}
