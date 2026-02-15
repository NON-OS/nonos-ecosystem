use super::types::*;
use crate::contracts::ContractClient;
use crate::metrics::NodeMetricsCollector;
use crate::rewards::RewardTracker;
use crate::services::{ServiceState, ServiceType};
use crate::Node;
use nonos_types::{EthAddress, NonosResult};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::RwLock;

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

    let body = serde_json::to_string(data)
        .unwrap_or_else(|_| r#"{"error":"Serialization failed"}"#.to_string());

    let response = format!(
        "HTTP/1.1 {} {}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Connection: close\r\n\
         \r\n\
         {}",
        status,
        status_text,
        body.len(),
        body
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
        status: if is_healthy {
            "healthy".to_string()
        } else {
            "degraded".to_string()
        },
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
    node: &Arc<RwLock<Node>>,
) -> NonosResult<()> {
    let node = node.read().await;
    let uptime = node.uptime_secs();

    let mut services = Vec::new();

    if let Some(ref service_manager) = node.services() {
        let sm = service_manager.read().await;

        let service_types = [
            (ServiceType::HealthBeacon, "health_beacon"),
            (ServiceType::QualityOracle, "quality_oracle"),
            (ServiceType::Bootstrap, "bootstrap"),
            (ServiceType::Cache, "cache"),
        ];

        for (service_type, name) in service_types {
            let state = sm.get_state(service_type).await;
            let (running, state_str) = match state {
                ServiceState::Running => (true, "running"),
                ServiceState::Starting => (true, "starting"),
                ServiceState::Stopped => (false, "stopped"),
                ServiceState::Failed => (false, "failed"),
            };

            services.push(ServiceStatus {
                name: name.to_string(),
                running,
                state: state_str.to_string(),
                restart_count: 0,
                last_heartbeat: if running { Some(chrono::Utc::now().timestamp()) } else { None },
                last_error: None,
                uptime_secs: if running { uptime } else { 0 },
                requests: 0,
                errors: 0,
            });
        }
    }

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

    let (peers_connected, peer_limit, banned_peers, messages_sent, messages_received, bytes_sent, bytes_received, active_topics, bootstrap_mode, is_running) =
        if let Some(ref network) = node.network() {
            let net = network.read().await;
            let stats = net.stats();
            let config = net.config();
            (
                net.peer_count() as u64,
                config.max_connections,
                stats.banned_peers,
                stats.messages_published,
                stats.messages_received,
                stats.bytes_sent,
                stats.bytes_received,
                stats.active_topics,
                format!("{:?}", net.bootstrap_mode()),
                net.is_running(),
            )
        } else {
            (0, 25, 0, 0, 0, 0, 0, 0, "official".to_string(), false)
        };

    let response = ApiResponse::success(NodeNetworkResponse {
        peers_connected,
        peers_total: peers_connected,
        peer_limit,
        banned_peers,
        quality_score: 1.0,
        messages_sent,
        messages_received,
        bytes_sent,
        bytes_received,
        active_topics,
        bootstrap_mode,
        is_bootstrapped: is_running && peers_connected > 0,
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
        net.peers()
            .iter()
            .map(|p| PeerSummary {
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
            })
            .collect()
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

    let (peers_connected, messages_published, messages_received, bytes_sent, bytes_received, total_bans, circuit_breaker_trips, rate_limit_hits) =
        if let Some(ref network) = node.network() {
            let net = network.read().await;
            let stats = net.stats();
            (
                net.peer_count() as u64,
                stats.messages_published,
                stats.messages_received,
                stats.bytes_sent,
                stats.bytes_received,
                stats.banned_peers,
                stats.circuit_breaker_trips,
                stats.rate_limit_hits,
            )
        } else {
            (0, 0, 0, 0, 0, 0, 0, 0)
        };

    let response = ApiResponse::success(NodeMetricsResponse {
        uptime_secs: metrics.uptime_secs,
        total_requests: metrics.total_requests,
        successful_requests: metrics.successful_requests,
        failed_requests: metrics.total_requests.saturating_sub(metrics.successful_requests),
        average_latency_ms: 0.0,
        active_connections: peers_connected,
        bytes_sent,
        bytes_received,
        quality_score: metrics.quality.total(),
        cpu_usage: 0.0,
        memory_bytes: 0,
        p2p: P2pMetrics {
            peers_connected,
            peers_total: peers_connected,
            messages_published,
            messages_received,
            messages_dropped: 0,
            total_bans,
            circuit_breaker_trips,
            rate_limit_hits,
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
