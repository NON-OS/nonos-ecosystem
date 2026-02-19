use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

const BUILD_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(name = "nonos")]
#[command(version = BUILD_VERSION)]
#[command(author = "NON-OS <team@nonos.systems>")]
#[command(about = "NONOS Daemon - Decentralized privacy infrastructure")]
#[command(long_about = None)]
#[command(propagate_version = true)]
#[command(after_help = "\x1b[38;5;245mDocumentation:\x1b[0m https://docs.nonos.systems\n\x1b[38;5;245mSupport:\x1b[0m       https://github.com/nonos/daemon/issues")]
pub struct Cli {
    #[arg(short, long, global = true, value_name = "FILE", help = "Path to config file")]
    pub config: Option<PathBuf>,

    #[arg(short = 'd', long, global = true, value_name = "DIR", env = "NONOS_DATA_DIR", help = "Data directory path")]
    pub data_dir: Option<PathBuf>,

    #[arg(short, long, action = clap::ArgAction::Count, global = true, help = "Increase verbosity (-v, -vv, -vvv)")]
    pub verbose: u8,

    #[arg(short, long, global = true, help = "Suppress non-error output")]
    pub quiet: bool,

    #[arg(long, global = true, value_name = "FILE", help = "Write logs to file")]
    pub log_file: Option<PathBuf>,

    #[arg(long, global = true, default_value = "text", help = "Output format")]
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
    #[command(about = "Start the daemon")]
    #[command(long_about = "Start the NONOS daemon process.\n\nThe daemon runs ZK identity services, cache mixing, and connects to the P2P network.")]
    Run {
        #[arg(short, long, help = "Run in foreground (default)")]
        foreground: bool,
        #[arg(long, value_name = "FILE", help = "Write PID to file")]
        pid_file: Option<PathBuf>,
        #[arg(long, help = "Notify systemd when ready")]
        systemd: bool,
    },

    #[command(about = "Initialize a new node")]
    #[command(long_about = "Initialize configuration and generate node identity.\n\nThis creates the data directory, generates a P2P keypair, and writes default configuration.")]
    Init {
        #[arg(short, long, help = "Overwrite existing configuration")]
        force: bool,
        #[arg(long, help = "Skip interactive prompts")]
        non_interactive: bool,
        #[arg(long, value_name = "ADDRESS", help = "Ethereum address for rewards (0x...)")]
        reward_address: Option<String>,
        #[arg(long, value_name = "NAME", help = "Node nickname for identification")]
        nickname: Option<String>,
    },

    #[command(about = "Show node information")]
    Info,

    #[command(about = "Show running status")]
    Status,

    #[command(about = "Manage configuration")]
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },

    #[command(about = "Run health checks")]
    Check {
        #[arg(long, help = "Run full diagnostic checks")]
        full: bool,
    },

    #[command(about = "Manage ZK identities")]
    Identity {
        #[command(subcommand)]
        action: IdentityAction,
    },

    #[command(about = "Manage cache mixer")]
    Mixer {
        #[command(subcommand)]
        action: MixerAction,
    },

    #[command(about = "Launch TUI dashboard")]
    Dash {
        #[arg(long, default_value = "matrix", help = "Dashboard theme (matrix, dark, light)")]
        theme: String,
    },

    #[command(about = "Manage staking")]
    Stake {
        #[command(subcommand)]
        action: StakeAction,
    },

    #[command(about = "Manage rewards")]
    Rewards {
        #[command(subcommand)]
        action: RewardsAction,
    },

    #[command(about = "Show statistics")]
    Stats,

    #[command(about = "Manage peers")]
    Peers {
        #[command(subcommand)]
        action: Option<PeersAction>,
    },

    #[command(about = "Generate systemd service files")]
    Systemd {
        #[arg(long, default_value = "/etc/systemd/system", help = "Output directory")]
        output_dir: PathBuf,
        #[arg(long, default_value = "nonos", help = "Service user")]
        user: String,
    },

    #[command(about = "Stop the daemon")]
    Stop {
        #[arg(short, long, help = "Force stop (SIGKILL)")]
        force: bool,
    },

    #[command(about = "Restart the daemon")]
    Restart {
        #[arg(short, long, help = "Force restart")]
        force: bool,
    },

    #[command(about = "Reload configuration")]
    Reload,

    #[command(about = "Show version information")]
    Version,
}

#[derive(Subcommand)]
pub enum ConfigAction {
    #[command(about = "Show current configuration")]
    Show,
    #[command(about = "Validate configuration")]
    Validate,
    #[command(about = "Set configuration value")]
    Set {
        #[arg(help = "Configuration key (e.g., api.port)")]
        key: String,
        #[arg(help = "Value to set")]
        value: String,
    },
}

#[derive(Subcommand)]
pub enum IdentityAction {
    #[command(about = "Generate new ZK identity")]
    Generate {
        #[arg(long, help = "Label for the identity")]
        label: Option<String>,
    },
    #[command(about = "List all identities")]
    List,
    #[command(about = "Show identity details")]
    Show {
        #[arg(help = "Identity ID or label")]
        id: String,
    },
    #[command(about = "Export identity to file")]
    Export {
        #[arg(help = "Identity ID")]
        id: String,
        #[arg(long, short, help = "Output file path")]
        output: Option<PathBuf>,
    },
    #[command(about = "Import identity from file")]
    Import {
        #[arg(help = "Path to identity file")]
        file: PathBuf,
    },
    #[command(about = "Generate ZK proof")]
    Prove {
        #[arg(help = "Identity ID")]
        id: String,
        #[arg(long, help = "Challenge value for the proof")]
        challenge: Option<String>,
    },
    #[command(about = "Verify ZK proof")]
    Verify {
        #[arg(help = "Proof to verify (hex or file path)")]
        proof: String,
    },
    #[command(about = "Register identity on-chain")]
    Register {
        #[arg(help = "Identity ID")]
        id: String,
    },
}

#[derive(Subcommand)]
pub enum MixerAction {
    #[command(about = "Show mixer status")]
    Status,
    #[command(about = "Flush mixer cache")]
    Flush {
        #[arg(short, long, help = "Force flush without confirmation")]
        force: bool,
    },
    #[command(about = "Configure mixer settings")]
    Config {
        #[arg(long, help = "Maximum cache entries")]
        max_entries: Option<usize>,
        #[arg(long, help = "Entry TTL in seconds")]
        ttl: Option<u64>,
        #[arg(long, help = "Enable/disable mixer")]
        enabled: Option<bool>,
    },
    #[command(about = "Add entry to mixer")]
    Add {
        #[arg(help = "Content hash")]
        hash: String,
        #[arg(help = "Data to store")]
        data: String,
    },
    #[command(about = "Get entry from mixer")]
    Get {
        #[arg(help = "Content hash")]
        hash: String,
    },
}

#[derive(Subcommand)]
pub enum StakeAction {
    #[command(about = "Show staking status")]
    Status,
    #[command(about = "Deposit NOX tokens")]
    Deposit {
        #[arg(help = "Amount in NOX")]
        amount: f64,
    },
    #[command(about = "Select staking tier")]
    Tier {
        #[arg(help = "Tier name (bronze, silver, gold, platinum)")]
        tier: String,
    },
    #[command(about = "Withdraw staked tokens")]
    Withdraw {
        #[arg(help = "Amount in NOX")]
        amount: f64,
    },
    #[command(about = "List available tiers")]
    Tiers,
}

#[derive(Subcommand)]
pub enum RewardsAction {
    #[command(about = "Show rewards status")]
    Status,
    #[command(about = "Claim pending rewards")]
    Claim,
    #[command(about = "Configure auto-claim")]
    Auto {
        #[arg(long, default_value = "100", help = "Minimum NOX before auto-claim")]
        threshold: f64,
    },
    #[command(about = "Show rewards history")]
    History {
        #[arg(long, default_value = "10", help = "Number of entries to show")]
        limit: u32,
    },
    #[command(about = "Debug reward calculation")]
    Debug {
        #[arg(long, default_value = "latest", help = "Epoch to debug")]
        epoch: String,
    },
}

#[derive(Subcommand)]
pub enum PeersAction {
    #[command(about = "List connected peers")]
    List,
    #[command(about = "Show peer details")]
    Show {
        #[arg(help = "Peer ID")]
        peer_id: String,
    },
    #[command(about = "Ban a peer")]
    Ban {
        #[arg(help = "Peer ID")]
        peer_id: String,
    },
    #[command(about = "Unban a peer")]
    Unban {
        #[arg(help = "Peer ID")]
        peer_id: String,
    },
}
