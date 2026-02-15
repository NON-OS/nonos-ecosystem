use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredPeerInfo {
    pub peer_id: String,
    pub addresses: Vec<String>,
    pub last_seen: i64,
    pub avg_latency_ms: Option<u32>,
    pub connection_count: u32,
    pub is_bootstrap: bool,
    pub reputation: u8,
    pub protocol_version: Option<String>,
    pub capabilities: Option<Vec<String>>,
    pub first_seen: Option<i64>,
    pub last_failure: Option<i64>,
    pub failure_count: u32,
}

impl Default for StoredPeerInfo {
    fn default() -> Self {
        Self {
            peer_id: String::new(),
            addresses: Vec::new(),
            last_seen: 0,
            avg_latency_ms: None,
            connection_count: 0,
            is_bootstrap: false,
            reputation: 50,
            protocol_version: Some("1.0.0".to_string()),
            capabilities: Some(Vec::new()),
            first_seen: None,
            last_failure: None,
            failure_count: 0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub error_count: u64,
    pub avg_latency_ms: u32,
    pub peer_count: usize,
    pub quality_score: f64,
    pub uptime_secs: u64,
    pub cpu_usage: Option<f32>,
    pub memory_usage: Option<f32>,
    pub network_bytes_sent: Option<u64>,
    pub network_bytes_received: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredEpochSummary {
    pub epoch: u64,
    pub start_time: i64,
    pub end_time: i64,
    pub total_emission: u128,
    pub our_reward: u128,
    pub quality_score: f64,
    pub participated: bool,
    pub streak: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredClaim {
    pub epoch: u64,
    pub amount: u128,
    pub claimed_at: i64,
    pub tx_hash: Option<String>,
    pub claimant: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeSizes {
    pub identity: usize,
    pub peers: usize,
    pub metrics: usize,
    pub epochs: usize,
    pub config: usize,
    pub claims: usize,
    pub secrets: usize,
    pub audit_log: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub timestamp: i64,
    pub tree: String,
    pub operation: String,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityReport {
    pub checked_at: i64,
    pub total_entries: usize,
    pub valid_entries: usize,
    pub corrupted_entries: usize,
    pub tree_reports: Vec<TreeIntegrityReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeIntegrityReport {
    pub name: String,
    pub entries: usize,
    pub valid: usize,
    pub corrupted: usize,
}
