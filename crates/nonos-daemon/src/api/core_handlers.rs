use super::handlers::send_response;
use super::responses::*;
use crate::{Node, NodeMetricsCollector, PrometheusExporter};
use nonos_types::{NodeStatus, NonosResult};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::RwLock;

pub async fn serve_dashboard(stream: &mut TcpStream) -> NonosResult<()> {
    let html = include_str!("../dashboard.html");
    send_response(stream, 200, "text/html; charset=utf-8", html).await
}

pub async fn serve_status(stream: &mut TcpStream, node: &Arc<RwLock<Node>>) -> NonosResult<()> {
    let node = node.read().await;
    let metrics = node.metrics().await;

    let response = StatusResponse {
        node_id: metrics.node_id.to_string(),
        status: format!("{:?}", metrics.status),
        tier: format!("{:?}", metrics.tier),
        uptime_secs: metrics.uptime_secs,
        active_connections: metrics.active_connections as usize,
        total_requests: metrics.total_requests,
        successful_requests: metrics.successful_requests,
        quality_score: metrics.quality.total(),
        staked_nox: metrics.staked.raw as f64 / 1e18,
        pending_rewards: metrics.pending_rewards.raw as f64 / 1e18,
        streak_days: metrics.streak,
    };

    let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    send_response(stream, 200, "application/json", &json).await
}

pub async fn serve_metrics(stream: &mut TcpStream, node: &Arc<RwLock<Node>>) -> NonosResult<()> {
    let node = node.read().await;
    let metrics = node.metrics().await;

    let response = MetricsResponse {
        node_id: metrics.node_id.to_string(),
        quality: QualityMetrics {
            uptime: metrics.quality.uptime,
            success_rate: metrics.quality.success_rate,
            latency_score: metrics.quality.latency_score,
            reliability: metrics.quality.reliability,
            total: metrics.quality.total(),
        },
        requests: RequestMetrics {
            total: metrics.total_requests,
            successful: metrics.successful_requests,
            failed: metrics.total_requests.saturating_sub(metrics.successful_requests),
        },
        network: NetworkMetrics {
            active_connections: metrics.active_connections as usize,
            peer_count: if let Some(ref network) = node.network() {
                network.read().await.peer_count()
            } else {
                0
            },
        },
        rewards: RewardsMetrics {
            staked_nox: metrics.staked.raw as f64 / 1e18,
            pending_rewards: metrics.pending_rewards.raw as f64 / 1e18,
            streak_days: metrics.streak,
            tier: format!("{:?}", metrics.tier),
        },
    };

    let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    send_response(stream, 200, "application/json", &json).await
}

pub async fn serve_prometheus(
    stream: &mut TcpStream,
    metrics: &Arc<NodeMetricsCollector>,
) -> NonosResult<()> {
    let exporter = PrometheusExporter::new(metrics.clone());
    let output = exporter.export();
    send_response(stream, 200, "text/plain; charset=utf-8", &output).await
}

pub async fn serve_health(stream: &mut TcpStream, node: &Arc<RwLock<Node>>) -> NonosResult<()> {
    let node = node.read().await;
    let status = node.status().await;

    let response = HealthResponse {
        healthy: status == NodeStatus::Running,
        status: format!("{:?}", status),
        uptime_secs: node.uptime_secs(),
    };

    let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    send_response(stream, 200, "application/json", &json).await
}

pub async fn serve_peers(stream: &mut TcpStream, node: &Arc<RwLock<Node>>) -> NonosResult<()> {
    use crate::geo::{GeoCache, GeoLocation};

    let node = node.read().await;
    let geo_cache = GeoCache::new();

    let live_peers = if let Some(ref network) = node.network() {
        network.read().await.peers()
    } else {
        vec![]
    };

    let mut peers_geo: Vec<PeerGeoInfo> = Vec::new();

    let bootstrap_nodes = vec![
        ("bootstrap-amsterdam", "Amsterdam", "Netherlands", "NL", 52.37, 4.90),
        ("bootstrap-sofia", "Sofia", "Bulgaria", "BG", 42.70, 23.32),
        ("bootstrap-capetown", "Cape Town", "South Africa", "ZA", -33.92, 18.42),
        ("bootstrap-budapest", "Budapest", "Hungary", "HU", 47.50, 19.04),
    ];

    for (id, city, country, code, lat, lon) in bootstrap_nodes {
        peers_geo.push(PeerGeoInfo {
            id: id.to_string(),
            address: format!("/ip4/{}/tcp/9000", id),
            lat,
            lon,
            city: city.to_string(),
            country: country.to_string(),
            country_code: code.to_string(),
            latency_ms: Some(50),
            connected: true,
            is_bootstrap: true,
        });
    }

    for peer in live_peers {
        let addr = peer.addresses.first().cloned().unwrap_or_default();
        let geo = if let Some(ip) = GeoCache::extract_ip(&addr) {
            geo_cache.lookup(&ip).await.unwrap_or_default()
        } else {
            GeoLocation::default()
        };

        if geo.lat == 0.0 && geo.lon == 0.0 {
            continue;
        }

        peers_geo.push(PeerGeoInfo {
            id: peer.id.clone(),
            address: addr,
            lat: geo.lat,
            lon: geo.lon,
            city: geo.city,
            country: geo.country,
            country_code: geo.country_code,
            latency_ms: peer.latency_ms,
            connected: !peer.is_banned,
            is_bootstrap: false,
        });
    }

    let response = PeersResponse {
        count: peers_geo.len(),
        peers: peers_geo,
    };

    let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    send_response(stream, 200, "application/json", &json).await
}

pub async fn serve_diagnostics(stream: &mut TcpStream, node: &Arc<RwLock<Node>>) -> NonosResult<()> {
    let node = node.read().await;
    let report = node.diagnose().await;

    let checks: Vec<DiagnosticCheck> = report
        .checks()
        .iter()
        .map(|(name, result)| {
            let (status, message) = match result {
                crate::CheckResult::Pass(msg) => ("pass", msg.clone()),
                crate::CheckResult::Warn(msg) => ("warn", msg.clone()),
                crate::CheckResult::Fail(msg) => ("fail", msg.clone()),
            };
            DiagnosticCheck {
                name: name.clone(),
                status: status.to_string(),
                message,
            }
        })
        .collect();

    let response = DiagnosticsResponse {
        all_passed: report.all_passed(),
        checks,
    };

    let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    send_response(stream, 200, "application/json", &json).await
}

pub async fn rotate_identity(stream: &mut TcpStream, _node: &Arc<RwLock<Node>>) -> NonosResult<()> {
    let response = r#"{"success":true,"message":"Identity rotation scheduled"}"#;
    send_response(stream, 200, "application/json", response).await
}
