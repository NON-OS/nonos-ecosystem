use nonos_types::NodeId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum P2pMessage {
    HealthBeacon(HealthBeaconData),
    QualityReport(QualityReportData),
    BootstrapRequest,
    BootstrapResponse(Vec<String>),
    NodeAnnouncement(NodeAnnouncementData),
}

impl P2pMessage {
    pub fn encode(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    pub fn decode(data: &[u8]) -> Option<Self> {
        serde_json::from_slice(data).ok()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthBeaconData {
    pub node_id: NodeId,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub uptime_secs: u64,
    pub version: String,
    pub peer_count: usize,
    pub cpu_usage: Option<f32>,
    pub memory_usage: Option<f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QualityReportData {
    pub node_id: NodeId,
    pub epoch: u64,
    pub success_rate: f64,
    pub avg_latency_ms: u32,
    pub request_count: u64,
    pub uptime_percent: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeAnnouncementData {
    pub node_id: NodeId,
    pub tier: String,
    pub staked_amount: String,
    pub services: Vec<String>,
    pub addresses: Vec<String>,
}
