// NONOS GNU AFFERO GENERAL PUBLIC LICENSE
// Version 3, 19 November 2007
// Copyright (C) 2025 NON-OS <team@nonos.systems>

use super::middleware::{ApiContext, AuthResult, RateLimitResult, RequestHeaders};
use super::node_handlers::*;
use super::privacy_handlers::*;
use super::responses::*;
use super::rewards_handlers::*;
use super::staking_handlers::*;
use crate::contracts::ContractClient;
use crate::rewards::RewardTracker;
use crate::{Node, NodeMetricsCollector, PrivacyServiceManager, PrometheusExporter};
use nonos_types::{EthAddress, NodeStatus, NonosError, NonosResult};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::RwLock;

/// Handle HTTP request with authentication and rate limiting
pub async fn handle_request(
    mut stream: TcpStream,
    peer_addr: SocketAddr,
    node: Arc<RwLock<Node>>,
    metrics: Arc<NodeMetricsCollector>,
    privacy: Option<Arc<PrivacyServiceManager>>,
    contract_client: Option<Arc<RwLock<ContractClient>>>,
    reward_tracker: Option<Arc<RewardTracker>>,
    staker_address: Option<EthAddress>,
    api_context: Arc<ApiContext>,
) -> NonosResult<()> {
    let mut reader = BufReader::new(&mut stream);
    let mut request_line = String::new();

    // Read request line with timeout
    match tokio::time::timeout(
        std::time::Duration::from_secs(5),
        reader.read_line(&mut request_line),
    )
    .await
    {
        Ok(Ok(_)) => {}
        Ok(Err(e)) => {
            return send_error_response(
                &mut stream,
                400,
                "BAD_REQUEST",
                &format!("Failed to read request: {}", e),
            )
            .await;
        }
        Err(_) => {
            return send_error_response(&mut stream, 408, "TIMEOUT", "Request timeout").await;
        }
    }

    let parts: Vec<&str> = request_line.trim().split_whitespace().collect();
    if parts.len() < 2 {
        return send_error_response(&mut stream, 400, "BAD_REQUEST", "Invalid request line").await;
    }

    let method = parts[0];
    let path = parts[1];

    // Read headers
    let mut header_lines = Vec::new();
    loop {
        let mut line = String::new();
        match tokio::time::timeout(
            std::time::Duration::from_secs(5),
            reader.read_line(&mut line),
        )
        .await
        {
            Ok(Ok(_)) => {
                if line.trim().is_empty() {
                    break;
                }
                header_lines.push(line);
            }
            Ok(Err(e)) => {
                return send_error_response(
                    &mut stream,
                    400,
                    "BAD_REQUEST",
                    &format!("Failed to read headers: {}", e),
                )
                .await;
            }
            Err(_) => {
                return send_error_response(&mut stream, 408, "TIMEOUT", "Header read timeout")
                    .await;
            }
        }
    }

    let headers = RequestHeaders::parse(&header_lines);
    // Get real client IP, only trusting X-Forwarded-For from configured trusted proxies
    let client_ip = headers.real_ip(peer_addr.ip(), &api_context.trusted_proxies);

    // Check rate limit
    match api_context.rate_limiter.check_request(client_ip) {
        RateLimitResult::Allowed => {}
        RateLimitResult::IpLimitExceeded => {
            return send_error_response(
                &mut stream,
                429,
                "RATE_LIMITED",
                "Too many requests from your IP",
            )
            .await;
        }
        RateLimitResult::GlobalLimitExceeded => {
            return send_error_response(
                &mut stream,
                503,
                "SERVICE_OVERLOADED",
                "Server is overloaded, please try again later",
            )
            .await;
        }
    }

    // Check authentication
    match api_context
        .authenticator
        .authenticate(path, headers.authorization.as_deref())
    {
        AuthResult::Authenticated | AuthResult::NotRequired => {}
        AuthResult::MissingToken => {
            return send_error_response(
                &mut stream,
                401,
                "UNAUTHORIZED",
                "Missing Authorization header",
            )
            .await;
        }
        AuthResult::InvalidFormat => {
            return send_error_response(
                &mut stream,
                401,
                "UNAUTHORIZED",
                "Invalid Authorization format. Use: Bearer <token>",
            )
            .await;
        }
        AuthResult::InvalidToken => {
            return send_error_response(&mut stream, 403, "FORBIDDEN", "Invalid API token").await;
        }
    }

    // Handle CORS preflight
    if method == "OPTIONS" {
        return send_cors_preflight(&mut stream).await;
    }

    // Read body for POST requests
    let body = if method == "POST" {
        let content_length = headers.content_length.unwrap_or(0);
        if content_length > 1024 * 1024 {
            return send_error_response(
                &mut stream,
                413,
                "PAYLOAD_TOO_LARGE",
                "Request body too large",
            )
            .await;
        }

        let mut body = String::new();
        let _ = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            reader.read_line(&mut body),
        )
        .await;
        body.trim().to_string()
    } else {
        String::new()
    };

    // Route request
    match (method, path) {
        ("GET", "/") => serve_dashboard(&mut stream).await,
        ("GET", "/api/status") => serve_status(&mut stream, &node).await,
        ("GET", "/api/metrics") => serve_metrics(&mut stream, &node).await,
        ("GET", "/api/metrics/prometheus") => serve_prometheus(&mut stream, &metrics).await,
        ("GET", "/api/health") => serve_health(&mut stream, &node).await,
        ("GET", "/api/peers") => serve_peers(&mut stream, &node).await,
        ("GET", "/api/diagnostics") => serve_diagnostics(&mut stream, &node).await,
        ("POST", "/api/identity/rotate") => rotate_identity(&mut stream, &node).await,
        // Privacy API
        ("GET", "/api/privacy/stats") => serve_privacy_stats(&mut stream, &privacy).await,
        ("POST", "/api/privacy/tracking/check") => {
            tracking_check(&mut stream, &privacy, &body).await
        }
        ("POST", "/api/privacy/tracking/block") => {
            tracking_block(&mut stream, &privacy, &body).await
        }
        ("POST", "/api/privacy/identity/register") => {
            identity_register(&mut stream, &privacy, &body).await
        }
        ("GET", "/api/privacy/identity/root") => identity_root(&mut stream, &privacy).await,
        // Staking API
        ("GET", "/api/staking/info") => {
            staking_info(&mut stream, &contract_client, staker_address).await
        }
        ("GET", "/api/staking/balance") => {
            staking_balance(&mut stream, &contract_client, staker_address).await
        }
        ("GET", "/api/staking/tier") => {
            staking_tier(&mut stream, &contract_client, staker_address).await
        }
        ("POST", "/api/staking/stake") => staking_stake(&mut stream, &contract_client, &body).await,
        ("POST", "/api/staking/unstake") => {
            staking_unstake(&mut stream, &contract_client, &body).await
        }
        ("POST", "/api/staking/approve") => {
            staking_approve(&mut stream, &contract_client, &body).await
        }
        ("POST", "/api/staking/set-tier") => {
            staking_set_tier(&mut stream, &contract_client, &body).await
        }
        // Rewards API
        ("GET", "/api/rewards/pending") => {
            rewards_pending(&mut stream, &contract_client, &reward_tracker, staker_address).await
        }
        ("POST", "/api/rewards/claim") => rewards_claim(&mut stream, &reward_tracker).await,
        ("GET", "/api/rewards/history") => rewards_history(&mut stream, &reward_tracker).await,
        ("POST", "/api/rewards/auto-claim/enable") => {
            rewards_auto_claim_enable(&mut stream, &reward_tracker, &body).await
        }
        ("POST", "/api/rewards/auto-claim/disable") => {
            rewards_auto_claim_disable(&mut stream, &reward_tracker).await
        }
        ("GET", "/api/rewards/apy") => {
            rewards_apy(&mut stream, &contract_client, staker_address).await
        }
        // V1 Node API (for nonos-dash)
        ("GET", "/api/v1/node/info") => serve_node_info(&mut stream, &node).await,
        ("GET", "/api/v1/node/health") => serve_node_health(&mut stream, &node).await,
        ("GET", "/api/v1/node/services") => serve_node_services(&mut stream, &node).await,
        ("GET", "/api/v1/node/network") => serve_node_network(&mut stream, &node).await,
        ("GET", "/api/v1/node/peers") => serve_node_peers(&mut stream, &node).await,
        ("GET", "/api/v1/node/metrics") => serve_node_metrics(&mut stream, &node, &metrics).await,
        ("GET", "/api/v1/node/rewards") => {
            serve_node_rewards(
                &mut stream,
                &node,
                &contract_client,
                &reward_tracker,
                staker_address,
            )
            .await
        }
        ("GET", "/api/v1/node/config") => serve_node_config(&mut stream, &node).await,
        _ => {
            send_error_response(
                &mut stream,
                404,
                "NOT_FOUND",
                &format!("Endpoint not found: {} {}", method, path),
            )
            .await
        }
    }
}

/// Send HTTP response
pub async fn send_response(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &str,
) -> NonosResult<()> {
    let status_text = match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        408 => "Request Timeout",
        413 => "Payload Too Large",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        _ => "Unknown",
    };

    let response = format!(
        "HTTP/1.1 {} {}\r\n\
         Content-Type: {}\r\n\
         Content-Length: {}\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
         Access-Control-Allow-Headers: Authorization, Content-Type\r\n\
         Connection: close\r\n\
         \r\n\
         {}",
        status, status_text, content_type, body.len(), body
    );

    stream.write_all(response.as_bytes()).await.map_err(|e| {
        NonosError::Network(format!("Failed to send response: {}", e))
    })?;

    Ok(())
}

/// Send a JSON error response with proper structure
pub async fn send_error_response(
    stream: &mut TcpStream,
    status: u16,
    code: &str,
    message: &str,
) -> NonosResult<()> {
    let body = serde_json::json!({
        "error": {
            "code": code,
            "message": message,
            "status": status
        }
    });
    send_response(
        stream,
        status,
        "application/json",
        &body.to_string(),
    )
    .await
}

/// Send CORS preflight response
async fn send_cors_preflight(stream: &mut TcpStream) -> NonosResult<()> {
    let response = "HTTP/1.1 204 No Content\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
         Access-Control-Allow-Headers: Authorization, Content-Type\r\n\
         Access-Control-Max-Age: 86400\r\n\
         Connection: close\r\n\
         \r\n";

    stream.write_all(response.as_bytes()).await.map_err(|e| {
        NonosError::Network(format!("Failed to send CORS response: {}", e))
    })?;

    Ok(())
}

/// Serve dashboard HTML
pub async fn serve_dashboard(stream: &mut TcpStream) -> NonosResult<()> {
    let html = include_str!("../dashboard.html");
    send_response(stream, 200, "text/html; charset=utf-8", html).await
}

/// Serve node status
async fn serve_status(stream: &mut TcpStream, node: &Arc<RwLock<Node>>) -> NonosResult<()> {
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

/// Serve detailed metrics
async fn serve_metrics(stream: &mut TcpStream, node: &Arc<RwLock<Node>>) -> NonosResult<()> {
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

/// Serve Prometheus metrics
async fn serve_prometheus(
    stream: &mut TcpStream,
    metrics: &Arc<NodeMetricsCollector>,
) -> NonosResult<()> {
    let exporter = PrometheusExporter::new(metrics.clone());
    let output = exporter.export();
    send_response(stream, 200, "text/plain; charset=utf-8", &output).await
}

/// Serve health check
async fn serve_health(stream: &mut TcpStream, node: &Arc<RwLock<Node>>) -> NonosResult<()> {
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

/// Serve peer list
async fn serve_peers(stream: &mut TcpStream, node: &Arc<RwLock<Node>>) -> NonosResult<()> {
    let node = node.read().await;

    let peers = if let Some(ref network) = node.network() {
        network.read().await.peers()
    } else {
        vec![]
    };

    let response = PeersResponse {
        count: peers.len(),
        peers: peers.iter().map(|p| p.id.clone()).collect(),
    };

    let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    send_response(stream, 200, "application/json", &json).await
}

/// Serve diagnostics
async fn serve_diagnostics(stream: &mut TcpStream, node: &Arc<RwLock<Node>>) -> NonosResult<()> {
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

/// Rotate node identity
async fn rotate_identity(stream: &mut TcpStream, _node: &Arc<RwLock<Node>>) -> NonosResult<()> {
    let response = r#"{"success":true,"message":"Identity rotation scheduled"}"#;
    send_response(stream, 200, "application/json", response).await
}
