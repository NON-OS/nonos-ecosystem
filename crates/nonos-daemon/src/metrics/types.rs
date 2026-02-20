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

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WorkMetrics {
    pub traffic_relay: TrafficRelayMetrics,
    pub zk_proofs: ZkProofMetrics,
    pub mixer_ops: MixerOpsMetrics,
    pub entropy: EntropyMetrics,
    pub registry_ops: RegistryOpsMetrics,
    pub epoch: EpochInfo,
    pub total_work_score: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TrafficRelayMetrics {
    pub bytes_relayed: u64,
    pub relay_sessions: u64,
    pub successful_relays: u64,
    pub failed_relays: u64,
    pub avg_latency_ms: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ZkProofMetrics {
    pub proofs_generated: u64,
    pub proofs_verified: u64,
    pub avg_generation_time_ms: f64,
    pub verification_failures: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MixerOpsMetrics {
    pub deposits_processed: u64,
    pub spends_processed: u64,
    pub total_value_mixed: u128,
    pub pool_participations: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EntropyMetrics {
    pub entropy_bytes_contributed: u64,
    pub entropy_requests_served: u64,
    pub quality_score: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RegistryOpsMetrics {
    pub registrations_processed: u64,
    pub lookups_served: u64,
    pub sync_operations: u64,
    pub failed_operations: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EpochInfo {
    pub current_epoch: u64,
    pub epoch_start_timestamp: u64,
    pub epoch_end_timestamp: u64,
    pub submitted_to_oracle: bool,
}

pub const WORK_WEIGHT_TRAFFIC_RELAY: f64 = 0.30;
pub const WORK_WEIGHT_ZK_PROOFS: f64 = 0.25;
pub const WORK_WEIGHT_MIXER_OPS: f64 = 0.20;
pub const WORK_WEIGHT_ENTROPY: f64 = 0.15;
pub const WORK_WEIGHT_REGISTRY_OPS: f64 = 0.10;

pub const EPOCH_DURATION_SECS: u64 = 7 * 24 * 60 * 60;
