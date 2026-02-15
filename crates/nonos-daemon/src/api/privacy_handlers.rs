use super::handlers::send_response;
use super::responses::*;
use crate::PrivacyServiceManager;
use nonos_types::NonosResult;
use std::sync::Arc;
use tokio::net::TcpStream;

pub async fn serve_privacy_stats(
    stream: &mut TcpStream,
    privacy: &Option<Arc<PrivacyServiceManager>>,
) -> NonosResult<()> {
    let response = if let Some(p) = privacy {
        let stats = p.stats();
        let block_rate = if stats.tracking_total > 0 {
            (stats.tracking_blocked as f64 / stats.tracking_total as f64) * 100.0
        } else {
            0.0
        };

        PrivacyStatsResponse {
            available: true,
            zk_proofs_issued: stats.zk_proofs_issued,
            zk_verifications: stats.zk_verifications,
            cache_hits: stats.cache_hits,
            cache_misses: stats.cache_misses,
            cache_mix_ops: stats.cache_mix_ops,
            tracking_blocked: stats.tracking_blocked,
            tracking_total: stats.tracking_total,
            tracking_block_rate: block_rate,
        }
    } else {
        PrivacyStatsResponse {
            available: false,
            zk_proofs_issued: 0,
            zk_verifications: 0,
            cache_hits: 0,
            cache_misses: 0,
            cache_mix_ops: 0,
            tracking_blocked: 0,
            tracking_total: 0,
            tracking_block_rate: 0.0,
        }
    };

    let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    send_response(stream, 200, "application/json", &json).await
}

pub async fn tracking_check(
    stream: &mut TcpStream,
    privacy: &Option<Arc<PrivacyServiceManager>>,
    body: &str,
) -> NonosResult<()> {
    let Some(p) = privacy else {
        return send_response(stream, 503, "application/json", r#"{"error":"Privacy services not available"}"#).await;
    };

    let req: TrackingCheckRequest = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(_) => {
            return send_response(stream, 400, "application/json", r#"{"error":"Invalid JSON"}"#).await;
        }
    };

    let blocked = p.tracking_blocker.should_block_domain(&req.domain).await;

    let response = TrackingCheckResponse {
        blocked,
        domain: req.domain,
    };

    let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    send_response(stream, 200, "application/json", &json).await
}

pub async fn tracking_block(
    stream: &mut TcpStream,
    privacy: &Option<Arc<PrivacyServiceManager>>,
    body: &str,
) -> NonosResult<()> {
    let Some(p) = privacy else {
        return send_response(stream, 503, "application/json", r#"{"error":"Privacy services not available"}"#).await;
    };

    let req: TrackingBlockRequest = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(_) => {
            return send_response(stream, 400, "application/json", r#"{"error":"Invalid JSON"}"#).await;
        }
    };

    p.tracking_blocker.block_domain(&req.domain).await;

    let response = TrackingBlockResponse {
        success: true,
        domain: req.domain,
    };

    let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    send_response(stream, 200, "application/json", &json).await
}

pub async fn identity_register(
    stream: &mut TcpStream,
    privacy: &Option<Arc<PrivacyServiceManager>>,
    body: &str,
) -> NonosResult<()> {
    let Some(p) = privacy else {
        return send_response(stream, 503, "application/json", r#"{"error":"Privacy services not available"}"#).await;
    };

    let req: IdentityRegisterRequest = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(_) => {
            return send_response(stream, 400, "application/json", r#"{"error":"Invalid JSON"}"#).await;
        }
    };

    let commitment = match parse_hex_32(&req.commitment) {
        Ok(c) => c,
        Err(e) => {
            let err = format!(r#"{{"error":"Invalid commitment: {}"}}"#, e);
            return send_response(stream, 400, "application/json", &err).await;
        }
    };

    match p.zk_identity.register_identity(commitment).await {
        Ok(index) => {
            let response = IdentityRegisterResponse { success: true, index };
            let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
            send_response(stream, 200, "application/json", &json).await
        }
        Err(e) => {
            let err = format!(r#"{{"error":"{}"}}"#, e);
            send_response(stream, 500, "application/json", &err).await
        }
    }
}

pub async fn identity_root(
    stream: &mut TcpStream,
    privacy: &Option<Arc<PrivacyServiceManager>>,
) -> NonosResult<()> {
    let Some(p) = privacy else {
        return send_response(stream, 503, "application/json", r#"{"error":"Privacy services not available"}"#).await;
    };

    let root = p.zk_identity.tree_root().await;
    let response = IdentityRootResponse {
        root: hex::encode(root),
    };

    let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    send_response(stream, 200, "application/json", &json).await
}

fn parse_hex_32(s: &str) -> Result<[u8; 32], String> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    let bytes = hex::decode(s).map_err(|e| e.to_string())?;
    if bytes.len() != 32 {
        return Err(format!("Expected 32 bytes, got {}", bytes.len()));
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}
