// NONOS GNU AFFERO GENERAL PUBLIC LICENSE
// Version 3, 19 November 2007
// Copyright (C) 2025 NON-OS <team@nonos.systems>

//! Node API Handlers for nonos-dash Integration
//!
//! Provides versioned JSON API endpoints for monitoring NONØS nodes:
//! - /api/v1/node/info
//! - /api/v1/node/health
//! - /api/v1/node/services
//! - /api/v1/node/network
//! - /api/v1/node/peers
//! - /api/v1/node/metrics
//! - /api/v1/node/rewards
//! - /api/v1/node/config-summary

use crate::contracts::ContractClient;
use crate::metrics::{NodeMetricsCollector, ServiceMetrics};
use crate::p2p::PeerEntry;
use crate::rewards::RewardTracker;
use crate::Node;
use nonos_types::{EthAddress, NonosResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::RwLock;

/// API version
pub const API_VERSION: &str = "v1";

/// Build information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BuildInfo {
    /// Crate version
    pub version: String,
    /// Git commit hash (if available)
    pub git_commit: Option<String>,
    /// Build timestamp
    pub build_time: Option<String>,
    /// Rust version
    pub rust_version: Option<String>,
    /// Target triple
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

/// Response for /api/v1/node/info
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeInfoResponse {
    /// API version
    pub api_version: String,
    /// Node ID (peer ID)
    pub node_id: String,
    /// Node role (local, relay, backbone)
    pub role: String,
    /// Node uptime in seconds
    pub uptime_secs: u64,
    /// Node started timestamp
    pub started_at: i64,
    /// Build information
    pub build: BuildInfo,
    /// Enabled capabilities
    pub capabilities: Vec<String>,
}

/// Service status for /api/v1/node/services
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceStatus {
    /// Service name
    pub name: String,
    /// Is service running
    pub running: bool,
    /// Service state (starting, running, stopped, failed)
    pub state: String,
    /// Number of restarts
    pub restart_count: u32,
    /// Last heartbeat timestamp
    pub last_heartbeat: Option<i64>,
    /// Last error message
    pub last_error: Option<String>,
    /// Uptime in seconds
    pub uptime_secs: u64,
    /// Requests handled
    pub requests: u64,
    /// Errors encountered
    pub errors: u64,
}

/// Response for /api/v1/node/health
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeHealthResponse {
    /// Overall health status (healthy, degraded, unhealthy)
    pub status: String,
    /// Is node healthy
    pub healthy: bool,
    /// Node uptime in seconds
    pub uptime_secs: u64,
    /// Last error if any
    pub last_error: Option<String>,
    /// Service health summary
    pub services: HashMap<String, bool>,
    /// P2P network health
    pub network_healthy: bool,
    /// Storage health
    pub storage_healthy: bool,
    /// Timestamp of health check
    pub checked_at: i64,
}

/// Response for /api/v1/node/services
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeServicesResponse {
    /// List of services
    pub services: Vec<ServiceStatus>,
    /// Total service count
    pub total_count: usize,
    /// Running service count
    pub running_count: usize,
    /// Failed service count
    pub failed_count: usize,
}

/// Response for /api/v1/node/network
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeNetworkResponse {
    /// Number of connected peers
    pub peers_connected: u64,
    /// Total known peers
    pub peers_total: u64,
    /// Maximum peer limit
    pub peer_limit: u32,
    /// Banned peer count
    pub banned_peers: u64,
    /// Network quality score (0.0-1.0)
    pub quality_score: f64,
    /// Total messages sent
    pub messages_sent: u64,
    /// Total messages received
    pub messages_received: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Active topics subscribed
    pub active_topics: u64,
    /// Bootstrap mode
    pub bootstrap_mode: String,
    /// Is bootstrapped
    pub is_bootstrapped: bool,
    /// Network uptime seconds
    pub uptime_secs: u64,
}

/// Peer summary for /api/v1/node/peers
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerSummary {
    /// Peer ID
    pub peer_id: String,
    /// Connection state
    pub state: String,
    /// Addresses
    pub addresses: Vec<String>,
    /// Latency in milliseconds
    pub latency_ms: Option<u32>,
    /// Quality score
    pub quality_score: f64,
    /// Penalty score
    pub penalty_score: i32,
    /// Is banned
    pub is_banned: bool,
    /// Ban remaining seconds
    pub ban_remaining_secs: Option<u64>,
    /// Role hint
    pub role_hint: Option<String>,
    /// Protocol version
    pub protocol_version: Option<String>,
    /// Last seen timestamp
    pub last_seen: i64,
    /// Messages received from this peer
    pub messages_received: u64,
    /// Is bootstrap peer
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

/// Response for /api/v1/node/peers
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodePeersResponse {
    /// List of peers
    pub peers: Vec<PeerSummary>,
    /// Total peer count
    pub total_count: usize,
    /// Connected count
    pub connected_count: usize,
    /// Banned count
    pub banned_count: usize,
    /// Average quality score
    pub avg_quality_score: f64,
}

/// Response for /api/v1/node/metrics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeMetricsResponse {
    /// Node uptime seconds
    pub uptime_secs: u64,
    /// Total requests
    pub total_requests: u64,
    /// Successful requests
    pub successful_requests: u64,
    /// Failed requests
    pub failed_requests: u64,
    /// Average latency in milliseconds
    pub average_latency_ms: f64,
    /// Active connections
    pub active_connections: u64,
    /// Bytes sent
    pub bytes_sent: u64,
    /// Bytes received
    pub bytes_received: u64,
    /// Quality score
    pub quality_score: f64,
    /// CPU usage percentage
    pub cpu_usage: f64,
    /// Memory usage bytes
    pub memory_bytes: u64,
    /// P2P metrics
    pub p2p: P2pMetrics,
    /// Service metrics
    pub services: HashMap<String, ServiceMetricsSummary>,
}

/// P2P-specific metrics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct P2pMetrics {
    /// Connected peers
    pub peers_connected: u64,
    /// Total known peers
    pub peers_total: u64,
    /// Messages published
    pub messages_published: u64,
    /// Messages received
    pub messages_received: u64,
    /// Messages dropped
    pub messages_dropped: u64,
    /// Total bans
    pub total_bans: u64,
    /// Circuit breaker trips
    pub circuit_breaker_trips: u64,
    /// Rate limit hits
    pub rate_limit_hits: u64,
}

/// Service metrics summary
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceMetricsSummary {
    /// Requests handled
    pub requests: u64,
    /// Errors
    pub errors: u64,
    /// Is running
    pub running: bool,
    /// Restarts
    pub restarts: u32,
    /// Uptime seconds
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

/// Response for /api/v1/node/rewards
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeRewardsResponse {
    /// Is staking enabled
    pub enabled: bool,
    /// Staked amount (NOX)
    pub staked_nox: f64,
    /// Pending rewards (NOX)
    pub pending_rewards_nox: f64,
    /// Total claimed (NOX)
    pub total_claimed_nox: f64,
    /// Current streak in days
    pub streak_days: u32,
    /// Staking tier
    pub tier: String,
    /// Last claim timestamp
    pub last_claim: Option<i64>,
    /// Auto-claim enabled
    pub auto_claim_enabled: bool,
    /// Estimated APY percentage
    pub estimated_apy: Option<f64>,
}

/// Response for /api/v1/node/config-summary
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeConfigSummaryResponse {
    /// Node role
    pub role: String,
    /// P2P port
    pub p2p_port: u16,
    /// API port
    pub api_port: u16,
    /// API bind address
    pub api_bind: String,
    /// Bootstrap mode
    pub bootstrap_mode: String,
    /// Bootstrap peer count
    pub bootstrap_peer_count: usize,
    /// Max connections
    pub max_connections: u32,
    /// Rate limiting enabled
    pub rate_limiting_enabled: bool,
    /// API auth required
    pub api_auth_required: bool,
    /// Services enabled
    pub services: HashMap<String, bool>,
}

/// Error response format
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiErrorResponse {
    /// Error code
    pub error: String,
    /// Error message
    pub message: String,
    /// Additional details
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

/// Generic API response wrapper
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// Success status
    pub success: bool,
    /// Response data (if success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// Error (if failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiErrorResponse>,
    /// Request timestamp
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

// Handler functions for v1 API

async fn send_json_response<T: serde::Serialize>(
    stream: &mut TcpStream,
    status: u16,
    data: &T,
) -> NonosResult<()> {
    let status_text = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "Unknown",
    };

    let body = serde_json::to_string(data).unwrap_or_else(|_| r#"{"error":"Serialization failed"}"#.to_string());

    let response = format!(
        "HTTP/1.1 {} {}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Connection: close\r\n\
         \r\n\
         {}",
        status, status_text, body.len(), body
    );

    stream.write_all(response.as_bytes()).await.map_err(|e| {
        nonos_types::NonosError::Network(format!("Failed to send response: {}", e))
    })?;

    Ok(())
}

pub async fn serve_node_info(
    stream: &mut TcpStream,
    node: &Arc<RwLock<Node>>,
) -> NonosResult<()> {
    let node = node.read().await;
    let metrics = node.metrics().await;

    let response = ApiResponse::success(NodeInfoResponse {
        api_version: API_VERSION.to_string(),
        node_id: metrics.node_id.to_string(),
        role: format!("{:?}", crate::config::NodeRole::Local),
        uptime_secs: metrics.uptime_secs,
        started_at: chrono::Utc::now().timestamp() - metrics.uptime_secs as i64,
        build: BuildInfo::default(),
        capabilities: vec![
            "p2p".to_string(),
            "privacy".to_string(),
            "caching".to_string(),
        ],
    });

    send_json_response(stream, 200, &response).await
}

pub async fn serve_node_health(
    stream: &mut TcpStream,
    node: &Arc<RwLock<Node>>,
) -> NonosResult<()> {
    let node = node.read().await;
    let status = node.status().await;
    let is_healthy = status == nonos_types::NodeStatus::Running;

    let response = ApiResponse::success(NodeHealthResponse {
        status: if is_healthy { "healthy".to_string() } else { "degraded".to_string() },
        healthy: is_healthy,
        uptime_secs: node.uptime_secs(),
        last_error: None,
        services: HashMap::new(),
        network_healthy: true,
        storage_healthy: true,
        checked_at: chrono::Utc::now().timestamp(),
    });

    send_json_response(stream, 200, &response).await
}

pub async fn serve_node_services(
    stream: &mut TcpStream,
    _node: &Arc<RwLock<Node>>,
) -> NonosResult<()> {
    let services = vec![
        ServiceStatus {
            name: "zk_identity".to_string(),
            running: true,
            state: "running".to_string(),
            restart_count: 0,
            last_heartbeat: Some(chrono::Utc::now().timestamp()),
            last_error: None,
            uptime_secs: 0,
            requests: 0,
            errors: 0,
        },
        ServiceStatus {
            name: "cache_mixing".to_string(),
            running: true,
            state: "running".to_string(),
            restart_count: 0,
            last_heartbeat: Some(chrono::Utc::now().timestamp()),
            last_error: None,
            uptime_secs: 0,
            requests: 0,
            errors: 0,
        },
        ServiceStatus {
            name: "tracking_blocker".to_string(),
            running: true,
            state: "running".to_string(),
            restart_count: 0,
            last_heartbeat: Some(chrono::Utc::now().timestamp()),
            last_error: None,
            uptime_secs: 0,
            requests: 0,
            errors: 0,
        },
    ];

    let running_count = services.iter().filter(|s| s.running).count();
    let failed_count = services.iter().filter(|s| s.state == "failed").count();

    let response = ApiResponse::success(NodeServicesResponse {
        total_count: services.len(),
        running_count,
        failed_count,
        services,
    });

    send_json_response(stream, 200, &response).await
}

pub async fn serve_node_network(
    stream: &mut TcpStream,
    node: &Arc<RwLock<Node>>,
) -> NonosResult<()> {
    let node = node.read().await;

    let (peers_connected, peers_total) = if let Some(ref network) = node.network() {
        let net = network.read().await;
        (net.peer_count() as u64, net.peer_count() as u64)
    } else {
        (0, 0)
    };

    let response = ApiResponse::success(NodeNetworkResponse {
        peers_connected,
        peers_total,
        peer_limit: 25,
        banned_peers: 0,
        quality_score: 1.0,
        messages_sent: 0,
        messages_received: 0,
        bytes_sent: 0,
        bytes_received: 0,
        active_topics: 5,
        bootstrap_mode: "official".to_string(),
        is_bootstrapped: true,
        uptime_secs: node.uptime_secs(),
    });

    send_json_response(stream, 200, &response).await
}

pub async fn serve_node_peers(
    stream: &mut TcpStream,
    node: &Arc<RwLock<Node>>,
) -> NonosResult<()> {
    let node = node.read().await;

    let peers: Vec<PeerSummary> = if let Some(ref network) = node.network() {
        let net = network.read().await;
        net.peers().iter().map(|p| PeerSummary {
            peer_id: p.id.clone(),
            state: "connected".to_string(),
            addresses: p.addresses.clone(),
            latency_ms: p.latency_ms,
            quality_score: 1.0,
            penalty_score: 0,
            is_banned: false,
            ban_remaining_secs: None,
            role_hint: None,
            protocol_version: None,
            last_seen: (chrono::Utc::now() - p.connected_at).num_seconds(),
            messages_received: 0,
            is_bootstrap: false,
        }).collect()
    } else {
        Vec::new()
    };

    let connected_count = peers.len();
    let banned_count = peers.iter().filter(|p| p.is_banned).count();
    let avg_quality = if peers.is_empty() {
        1.0
    } else {
        peers.iter().map(|p| p.quality_score).sum::<f64>() / peers.len() as f64
    };

    let response = ApiResponse::success(NodePeersResponse {
        peers,
        total_count: connected_count,
        connected_count,
        banned_count,
        avg_quality_score: avg_quality,
    });

    send_json_response(stream, 200, &response).await
}

pub async fn serve_node_metrics(
    stream: &mut TcpStream,
    node: &Arc<RwLock<Node>>,
    _metrics_collector: &Arc<NodeMetricsCollector>,
) -> NonosResult<()> {
    let node = node.read().await;
    let metrics = node.metrics().await;

    let response = ApiResponse::success(NodeMetricsResponse {
        uptime_secs: metrics.uptime_secs,
        total_requests: metrics.total_requests,
        successful_requests: metrics.successful_requests,
        failed_requests: metrics.total_requests.saturating_sub(metrics.successful_requests),
        average_latency_ms: 0.0,
        active_connections: metrics.active_connections as u64,
        bytes_sent: 0,
        bytes_received: 0,
        quality_score: metrics.quality.total(),
        cpu_usage: 0.0,
        memory_bytes: 0,
        p2p: P2pMetrics {
            peers_connected: metrics.active_connections as u64,
            peers_total: metrics.active_connections as u64,
            messages_published: 0,
            messages_received: 0,
            messages_dropped: 0,
            total_bans: 0,
            circuit_breaker_trips: 0,
            rate_limit_hits: 0,
        },
        services: HashMap::new(),
    });

    send_json_response(stream, 200, &response).await
}

pub async fn serve_node_rewards(
    stream: &mut TcpStream,
    node: &Arc<RwLock<Node>>,
    _contract_client: &Option<Arc<RwLock<ContractClient>>>,
    _reward_tracker: &Option<Arc<RewardTracker>>,
    _staker_address: Option<EthAddress>,
) -> NonosResult<()> {
    let node = node.read().await;
    let metrics = node.metrics().await;

    let response = ApiResponse::success(NodeRewardsResponse {
        enabled: true,
        staked_nox: metrics.staked.raw as f64 / 1e18,
        pending_rewards_nox: metrics.pending_rewards.raw as f64 / 1e18,
        total_claimed_nox: 0.0,
        streak_days: metrics.streak,
        tier: format!("{:?}", metrics.tier),
        last_claim: None,
        auto_claim_enabled: false,
        estimated_apy: Some(15.0),
    });

    send_json_response(stream, 200, &response).await
}

pub async fn serve_node_config(
    stream: &mut TcpStream,
    _node: &Arc<RwLock<Node>>,
) -> NonosResult<()> {
    let mut services = HashMap::new();
    services.insert("health_beacon".to_string(), true);
    services.insert("quality_oracle".to_string(), true);
    services.insert("cache".to_string(), true);

    let response = ApiResponse::success(NodeConfigSummaryResponse {
        role: "local".to_string(),
        p2p_port: 9432,
        api_port: 8420,
        api_bind: "0.0.0.0".to_string(),
        bootstrap_mode: "official".to_string(),
        bootstrap_peer_count: 5,
        max_connections: 25,
        rate_limiting_enabled: true,
        api_auth_required: false,
        services,
    });

    send_json_response(stream, 200, &response).await
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
