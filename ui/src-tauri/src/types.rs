use serde::Serialize;

#[derive(Serialize)]
pub struct AppInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub platform: &'static str,
    pub arch: &'static str,
    pub build: &'static str,
}

#[derive(Serialize, Clone)]
pub struct NetworkStatusResponse {
    pub connected: bool,
    pub status: String,
    pub bootstrap_progress: u8,
    pub circuits: u32,
    pub socks_port: u16,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct WalletStatusResponse {
    pub initialized: bool,
    pub locked: bool,
    pub address: Option<String>,
    // Mainnet balances (real NOX)
    pub nox_balance: String,
    pub eth_balance: String,
    // Sepolia balances (testnet for staking)
    pub sepolia_nox_balance: String,
    pub sepolia_eth_balance: String,
    pub pending_rewards: String,
}

#[derive(Serialize)]
pub struct StakingStatusResponse {
    pub staked_amount: String,
    pub tier: String,
    pub tier_multiplier: String,
    pub pending_rewards: String,
    pub current_epoch: u64,
    pub next_tier_threshold: String,
    pub estimated_apy: String,
}

pub const STAKING_TIERS: &[(&str, u128, &str)] = &[
    ("Bronze", 1_000, "1.0x"),
    ("Silver", 10_000, "1.2x"),
    ("Gold", 50_000, "1.5x"),
    ("Platinum", 200_000, "2.0x"),
    ("Diamond", 1_000_000, "2.5x"),
];

#[derive(Serialize)]
pub struct NodeStatusResponse {
    pub running: bool,
    pub connected_nodes: usize,
    pub quality: f64,
    pub total_requests: u64,
}

#[derive(Serialize)]
pub struct PrivacyStatsResponse {
    pub zk_proofs_issued: u64,
    pub zk_verifications: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_hit_rate: f64,
    pub tracking_blocked: u64,
    pub tracking_total: u64,
    pub block_rate: f64,
    pub stealth_payments: u64,
    pub stealth_scanned: u64,
}

#[derive(Serialize)]
pub struct ZkIdentityResponse {
    pub identity_id: String,
    pub commitment: String,
    pub merkle_root: String,
}

#[derive(Serialize)]
pub struct TrackingCheckResponse {
    pub domain: String,
    pub blocked: bool,
    pub reason: Option<String>,
}

#[derive(Serialize)]
pub struct ProxyFetchResponse {
    pub success: bool,
    pub status_code: u16,
    pub headers: std::collections::HashMap<String, String>,
    pub body: String,
    pub content_type: String,
    pub via_proxy: bool,
    pub circuit_id: Option<String>,
}
