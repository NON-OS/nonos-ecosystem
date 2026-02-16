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
    #[arg(short, long, global = true, value_name = "FILE")]
    pub config: Option<PathBuf>,

    #[arg(short = 'd', long, global = true, value_name = "DIR", env = "NONOS_DATA_DIR")]
    pub data_dir: Option<PathBuf>,

    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    #[arg(short, long, global = true)]
    pub quiet: bool,

    #[arg(long, global = true, value_name = "FILE")]
    pub log_file: Option<PathBuf>,

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
    Run {
        #[arg(short, long)]
        foreground: bool,
        #[arg(long, value_name = "FILE")]
        pid_file: Option<PathBuf>,
        #[arg(long)]
        systemd: bool,
    },
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
    Info,
    Status,
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },
    Check {
        #[arg(long)]
        full: bool,
    },
    Identity {
        #[command(subcommand)]
        action: IdentityAction,
    },
    Mixer {
        #[command(subcommand)]
        action: MixerAction,
    },
    Dash {
        #[arg(long, default_value = "matrix")]
        theme: String,
    },
    Stake {
        #[command(subcommand)]
        action: StakeAction,
    },
    Rewards {
        #[command(subcommand)]
        action: RewardsAction,
    },
    Stats,
    Peers {
        #[command(subcommand)]
        action: Option<PeersAction>,
    },
    Systemd {
        #[arg(long, default_value = "/etc/systemd/system")]
        output_dir: PathBuf,
        #[arg(long, default_value = "nonos")]
        user: String,
    },
    Stop {
        #[arg(short, long)]
        force: bool,
    },
    Restart {
        #[arg(short, long)]
        force: bool,
    },
    Reload,
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
