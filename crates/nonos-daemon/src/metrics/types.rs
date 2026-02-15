use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default)]
pub struct ServiceMetrics {
    pub name: String,
    pub requests: u64,
    pub errors: u64,
    pub total_latency_us: u64,
    pub running: bool,
    pub restarts: u32,
    pub uptime_secs: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetricsSummary {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_latency_ms: f64,
    pub active_connections: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub uptime_secs: u64,
    pub quality_score: f64,
    pub cpu_usage: f64,
    pub memory_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct P2pMetricsSummary {
    pub peers_connected: u64,
    pub peers_total: u64,
    pub messages_published: u64,
    pub messages_received: u64,
    pub messages_dropped: u64,
    pub peer_bans: u64,
    pub rate_limit_hits: u64,
}
