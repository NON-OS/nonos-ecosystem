use serde::Serialize;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
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

pub struct AppState {
    pub network: Arc<RwLock<NetworkState>>,
    pub wallet: Arc<RwLock<WalletState>>,
    pub nodes: Arc<RwLock<NodeState>>,
    pub browser: Arc<RwLock<BrowserState>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            network: Arc::new(RwLock::new(NetworkState::default())),
            wallet: Arc::new(RwLock::new(WalletState::default())),
            nodes: Arc::new(RwLock::new(NodeState::default())),
            browser: Arc::new(RwLock::new(BrowserState::default())),
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
    pub nox_balance: u128,
    pub eth_balance: u128,
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
