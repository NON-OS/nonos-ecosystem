use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone, Debug, Serialize)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Bootstrapping,
    Connected,
    Error,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SelectedNetwork {
    #[default]
    Mainnet,
    Sepolia,
}

impl From<u8> for SelectedNetwork {
    fn from(v: u8) -> Self {
        match v {
            1 => SelectedNetwork::Sepolia,
            _ => SelectedNetwork::Mainnet,
        }
    }
}

impl From<SelectedNetwork> for u8 {
    fn from(n: SelectedNetwork) -> Self {
        match n {
            SelectedNetwork::Mainnet => 0,
            SelectedNetwork::Sepolia => 1,
        }
    }
}

pub struct AppState {
    pub network: Arc<RwLock<NetworkState>>,
    pub wallet: Arc<RwLock<WalletState>>,
    pub nodes: Arc<RwLock<NodeState>>,
    pub browser: Arc<RwLock<BrowserState>>,
    pub selected_network: AtomicU8, // 0 = Mainnet, 1 = Sepolia
}

impl AppState {
    pub fn get_selected_network(&self) -> SelectedNetwork {
        SelectedNetwork::from(self.selected_network.load(Ordering::Relaxed))
    }

    pub fn set_selected_network(&self, network: SelectedNetwork) {
        self.selected_network.store(network.into(), Ordering::Relaxed);
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            network: Arc::new(RwLock::new(NetworkState::default())),
            wallet: Arc::new(RwLock::new(WalletState::default())),
            nodes: Arc::new(RwLock::new(NodeState::default())),
            browser: Arc::new(RwLock::new(BrowserState::default())),
            selected_network: AtomicU8::new(0), // Default to Mainnet
        }
    }
}

#[derive(Debug)]
pub struct NetworkState {
    pub status: ConnectionStatus,
    pub bootstrap_progress: u8,
    pub circuits: u32,
    pub socks_addr: SocketAddr,
    pub control_port: u16,
    pub error: Option<String>,
    pub anon_pid: Option<u32>,
    pub data_dir: PathBuf,
}

impl Default for NetworkState {
    fn default() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nonos")
            .join("anon");

        Self {
            status: ConnectionStatus::Disconnected,
            bootstrap_progress: 0,
            circuits: 0,
            socks_addr: SocketAddr::from(([127, 0, 0, 1], 9050)),
            control_port: 9051,
            error: None,
            anon_pid: None,
            data_dir,
        }
    }
}

#[derive(Debug, Default)]
pub struct WalletState {
    pub initialized: bool,
    pub locked: bool,
    pub address: Option<String>,
    pub private_key: Option<String>,
    pub mnemonic: Option<String>,
    // Mainnet balances (real NOX)
    pub nox_balance: u128,
    pub eth_balance: u128,
    // Sepolia balances (testnet for staking)
    pub sepolia_nox_balance: u128,
    pub sepolia_eth_balance: u128,
    // Staking info (on Sepolia)
    pub pending_rewards: u128,
    pub staked_amount: u128,
    pub current_epoch: u64,
    pub last_refresh: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct NodeInfo {
    pub id: String,
    pub address: String,
    pub quality_score: f64,
    pub latency_ms: u32,
    pub connected: bool,
}

#[derive(Debug, Default)]
pub struct NodeState {
    pub nodes: Vec<NodeInfo>,
    pub embedded_running: bool,
    pub embedded_quality: f64,
    pub total_requests: AtomicU64,
    pub embedded_pid: Option<u32>,
    pub node_id: Option<String>,
    pub api_addr: String,
    pub p2p_port: u16,
}

#[derive(Debug, Default)]
pub struct BrowserState {
    pub next_tab_id: u32,
    pub history: Vec<String>,
}
