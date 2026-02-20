use super::core_handlers::*;
use super::middleware::{ApiContext, AuthResult, RateLimitResult, RequestHeaders};
use super::node_handlers::*;
use super::privacy_handlers::*;
use super::rewards_handlers::*;
use super::staking_handlers::*;
use super::work_handlers::*;
use crate::contracts::ContractClient;
use crate::rewards::RewardTracker;
use crate::{Node, NodeMetricsCollector, PrivacyServiceManager};
use nonos_types::{EthAddress, NonosError, NonosResult};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::RwLock;

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
    let client_ip = headers.real_ip(peer_addr.ip(), &api_context.trusted_proxies);

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

    if method == "OPTIONS" {
        return send_cors_preflight(&mut stream).await;
    }

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

    route_request(&mut stream, method, path, &body, &node, &metrics, &privacy, &contract_client, &reward_tracker, staker_address).await
}

async fn route_request(
    stream: &mut TcpStream,
    method: &str,
    path: &str,
    body: &str,
    node: &Arc<RwLock<Node>>,
    metrics: &Arc<NodeMetricsCollector>,
    privacy: &Option<Arc<PrivacyServiceManager>>,
    contract_client: &Option<Arc<RwLock<ContractClient>>>,
    reward_tracker: &Option<Arc<RewardTracker>>,
    staker_address: Option<EthAddress>,
) -> NonosResult<()> {
    match (method, path) {
        ("GET", "/") => serve_dashboard(stream).await,
        ("GET", "/api/status") => serve_status(stream, node).await,
        ("GET", "/api/metrics") => serve_metrics(stream, node).await,
        ("GET", "/api/metrics/prometheus") => serve_prometheus(stream, metrics).await,
        ("GET", "/api/health") => serve_health(stream, node).await,
        ("GET", "/api/peers") => serve_peers(stream, node).await,
        ("GET", "/api/diagnostics") => serve_diagnostics(stream, node).await,
        ("POST", "/api/identity/rotate") => rotate_identity(stream, node).await,
        ("GET", "/api/privacy/stats") => serve_privacy_stats(stream, privacy).await,
        ("POST", "/api/privacy/tracking/check") => tracking_check(stream, privacy, body).await,
        ("POST", "/api/privacy/tracking/block") => tracking_block(stream, privacy, body).await,
        ("POST", "/api/privacy/identity/register") => identity_register(stream, privacy, body).await,
        ("GET", "/api/privacy/identity/root") => identity_root(stream, privacy).await,
        ("POST", "/api/privacy/zk/register") => zk_identity_register(stream, privacy, body).await,
        ("POST", "/api/privacy/zk/verify") => zk_identity_verify(stream, privacy, body).await,
        ("GET", "/api/privacy/zk/root") => zk_identity_root(stream, privacy).await,
        ("GET", "/api/privacy/mixer/status") => mixer_status(stream, privacy).await,
        ("POST", "/api/privacy/mixer/deposit") => mixer_deposit(stream, privacy, body).await,
        ("POST", "/api/privacy/mixer/spend") => mixer_spend(stream, privacy, body).await,
        ("GET", "/api/staking/info") => staking_info(stream, contract_client, staker_address).await,
        ("GET", "/api/staking/balance") => staking_balance(stream, contract_client, staker_address).await,
        ("GET", "/api/staking/tier") => staking_tier(stream, contract_client, staker_address).await,
        ("POST", "/api/staking/stake") => staking_stake(stream, contract_client, body).await,
        ("POST", "/api/staking/unstake") => staking_unstake(stream, contract_client, body).await,
        ("POST", "/api/staking/approve") => staking_approve(stream, contract_client, body).await,
        ("POST", "/api/staking/set-tier") => staking_set_tier(stream, contract_client, body).await,
        ("GET", "/api/rewards/pending") => rewards_pending(stream, contract_client, reward_tracker, staker_address).await,
        ("POST", "/api/rewards/claim") => rewards_claim(stream, reward_tracker).await,
        ("GET", "/api/rewards/history") => rewards_history(stream, reward_tracker).await,
        ("POST", "/api/rewards/auto-claim/enable") => rewards_auto_claim_enable(stream, reward_tracker, body).await,
        ("POST", "/api/rewards/auto-claim/disable") => rewards_auto_claim_disable(stream, reward_tracker).await,
        ("GET", "/api/rewards/apy") => rewards_apy(stream, contract_client, staker_address).await,
        ("GET", "/api/v1/node/info") => serve_node_info(stream, node).await,
        ("GET", "/api/v1/node/health") => serve_node_health(stream, node).await,
        ("GET", "/api/v1/node/services") => serve_node_services(stream, node).await,
        ("GET", "/api/v1/node/network") => serve_node_network(stream, node).await,
        ("GET", "/api/v1/node/peers") => serve_node_peers(stream, node).await,
        ("GET", "/api/v1/node/metrics") => serve_node_metrics(stream, node, metrics).await,
        ("GET", "/api/v1/node/rewards") => serve_node_rewards(stream, node, contract_client, reward_tracker, staker_address).await,
        ("GET", "/api/v1/node/config") => serve_node_config(stream, node).await,
        ("GET", "/api/v1/work/metrics") => serve_work_metrics(stream, metrics).await,
        ("GET", "/api/v1/work/epoch") => serve_epoch_info(stream, metrics).await,
        ("POST", "/api/v1/work/epoch/advance") => check_epoch_advance(stream, metrics).await,
        ("POST", "/api/v1/work/epoch/submit") => mark_epoch_submitted(stream, metrics).await,
        _ => {
            send_error_response(
                stream,
                404,
                "NOT_FOUND",
                &format!("Endpoint not found: {} {}", method, path),
            )
            .await
        }
    }
}

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
    send_response(stream, status, "application/json", &body.to_string()).await
}

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
