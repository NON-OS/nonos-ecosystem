use crate::metrics::ServiceMetrics;
use crate::p2p::PeerEntry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const API_VERSION: &str = "v1";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BuildInfo {
    pub version: String,
    pub git_commit: Option<String>,
    pub build_time: Option<String>,
    pub rust_version: Option<String>,
    pub target: String,
}

impl Default for BuildInfo {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            git_commit: option_env!("GIT_COMMIT").map(String::from),
            build_time: option_env!("BUILD_TIME").map(String::from),
            rust_version: option_env!("RUSTC_VERSION").map(String::from),
            target: std::env::consts::ARCH.to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeInfoResponse {
    pub api_version: String,
    pub node_id: String,
    pub role: String,
    pub uptime_secs: u64,
    pub started_at: i64,
    pub build: BuildInfo,
    pub capabilities: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub name: String,
    pub running: bool,
    pub state: String,
    pub restart_count: u32,
    pub last_heartbeat: Option<i64>,
    pub last_error: Option<String>,
    pub uptime_secs: u64,
    pub requests: u64,
    pub errors: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeHealthResponse {
    pub status: String,
    pub healthy: bool,
    pub uptime_secs: u64,
    pub last_error: Option<String>,
    pub services: HashMap<String, bool>,
    pub network_healthy: bool,
    pub storage_healthy: bool,
    pub checked_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeServicesResponse {
    pub services: Vec<ServiceStatus>,
    pub total_count: usize,
    pub running_count: usize,
    pub failed_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeNetworkResponse {
    pub peers_connected: u64,
    pub peers_total: u64,
    pub peer_limit: u32,
    pub banned_peers: u64,
    pub quality_score: f64,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub active_topics: u64,
    pub bootstrap_mode: String,
    pub is_bootstrapped: bool,
    pub uptime_secs: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerSummary {
    pub peer_id: String,
    pub state: String,
    pub addresses: Vec<String>,
    pub latency_ms: Option<u32>,
    pub quality_score: f64,
    pub penalty_score: i32,
    pub is_banned: bool,
    pub ban_remaining_secs: Option<u64>,
    pub role_hint: Option<String>,
    pub protocol_version: Option<String>,
    pub last_seen: i64,
    pub messages_received: u64,
    pub is_bootstrap: bool,
}

impl From<&PeerEntry> for PeerSummary {
    fn from(entry: &PeerEntry) -> Self {
        Self {
            peer_id: entry.peer_id.clone(),
            state: format!("{:?}", entry.state),
            addresses: entry.addresses.clone(),
            latency_ms: entry.latency_ms,
            quality_score: entry.quality_score,
            penalty_score: entry.penalty_score,
            is_banned: entry.is_banned(),
            ban_remaining_secs: entry.ban_remaining().map(|d| d.as_secs()),
            role_hint: entry.role_hint.as_ref().map(|r| r.to_string()),
            protocol_version: entry.protocol_version.clone(),
            last_seen: entry.last_seen,
            messages_received: entry.messages_received,
            is_bootstrap: entry.is_bootstrap,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodePeersResponse {
    pub peers: Vec<PeerSummary>,
    pub total_count: usize,
    pub connected_count: usize,
    pub banned_count: usize,
    pub avg_quality_score: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeMetricsResponse {
    pub uptime_secs: u64,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_latency_ms: f64,
    pub active_connections: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub quality_score: f64,
    pub cpu_usage: f64,
    pub memory_bytes: u64,
    pub p2p: P2pMetrics,
    pub services: HashMap<String, ServiceMetricsSummary>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct P2pMetrics {
    pub peers_connected: u64,
    pub peers_total: u64,
    pub messages_published: u64,
    pub messages_received: u64,
    pub messages_dropped: u64,
    pub total_bans: u64,
    pub circuit_breaker_trips: u64,
    pub rate_limit_hits: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceMetricsSummary {
    pub requests: u64,
    pub errors: u64,
    pub running: bool,
    pub restarts: u32,
    pub uptime_secs: u64,
}

impl From<&ServiceMetrics> for ServiceMetricsSummary {
    fn from(m: &ServiceMetrics) -> Self {
        Self {
            requests: m.requests,
            errors: m.errors,
            running: m.running,
            restarts: m.restarts,
            uptime_secs: m.uptime_secs,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeRewardsResponse {
    pub enabled: bool,
    pub staked_nox: f64,
    pub pending_rewards_nox: f64,
    pub total_claimed_nox: f64,
    pub streak_days: u32,
    pub tier: String,
    pub last_claim: Option<i64>,
    pub auto_claim_enabled: bool,
    pub estimated_apy: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeConfigSummaryResponse {
    pub role: String,
    pub p2p_port: u16,
    pub api_port: u16,
    pub api_bind: String,
    pub bootstrap_mode: String,
    pub bootstrap_peer_count: usize,
    pub max_connections: u32,
    pub rate_limiting_enabled: bool,
    pub api_auth_required: bool,
    pub services: HashMap<String, bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiErrorResponse {
    pub error: String,
    pub message: String,
    pub details: Option<String>,
}

impl ApiErrorResponse {
    pub fn new(error: &str, message: &str) -> Self {
        Self {
            error: error.to_string(),
            message: message.to_string(),
            details: None,
        }
    }

    pub fn with_details(error: &str, message: &str, details: &str) -> Self {
        Self {
            error: error.to_string(),
            message: message.to_string(),
            details: Some(details.to_string()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiErrorResponse>,
    pub timestamp: i64,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    pub fn error(error: ApiErrorResponse) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_info() {
        let info = BuildInfo::default();
        assert!(!info.version.is_empty());
    }

    #[test]
    fn test_api_response_success() {
        let response = ApiResponse::success(NodeHealthResponse {
            status: "healthy".to_string(),
            healthy: true,
            uptime_secs: 3600,
            last_error: None,
            services: HashMap::new(),
            network_healthy: true,
            storage_healthy: true,
            checked_at: chrono::Utc::now().timestamp(),
        });

        assert!(response.success);
        assert!(response.data.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_api_response_error() {
        let response: ApiResponse<()> = ApiResponse::error(ApiErrorResponse::new(
            "not_found",
            "Resource not found",
        ));

        assert!(!response.success);
        assert!(response.data.is_none());
        assert!(response.error.is_some());
    }

    #[test]
    fn test_api_error_with_details() {
        let error = ApiErrorResponse::with_details(
            "validation_error",
            "Invalid input",
            "Field 'name' is required",
        );

        assert_eq!(error.error, "validation_error");
        assert!(error.details.is_some());
    }
}
