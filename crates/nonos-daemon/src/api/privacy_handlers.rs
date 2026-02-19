use super::handlers::send_response;
use super::responses::*;
use crate::privacy::{Note, SpendRequest, ASSET_ETH};
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
            identity_registrations: stats.identity_registrations,
            identity_verifications_passed: stats.identity_verifications_passed,
            identity_verifications_failed: stats.identity_verifications_failed,
            note_deposits: stats.note_deposits,
            note_spends: stats.note_spends,
            note_failed_spends: stats.note_failed_spends,
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
            identity_registrations: 0,
            identity_verifications_passed: 0,
            identity_verifications_failed: 0,
            note_deposits: 0,
            note_spends: 0,
            note_failed_spends: 0,
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

pub async fn zk_identity_register(
    stream: &mut TcpStream,
    privacy: &Option<Arc<PrivacyServiceManager>>,
    body: &str,
) -> NonosResult<()> {
    let Some(p) = privacy else {
        return send_response(stream, 503, "application/json", r#"{"error":"Privacy services not available"}"#).await;
    };

    let req: ZkIdentityRegisterRequest = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(_) => {
            return send_response(stream, 400, "application/json", r#"{"error":"Invalid JSON"}"#).await;
        }
    };

    let secret = match parse_hex_32(&req.secret) {
        Ok(s) => s,
        Err(e) => {
            let err = format!(r#"{{"error":"Invalid secret: {}"}}"#, e);
            return send_response(stream, 400, "application/json", &err).await;
        }
    };

    let blinding = match parse_hex_32(&req.blinding) {
        Ok(b) => b,
        Err(e) => {
            let err = format!(r#"{{"error":"Invalid blinding: {}"}}"#, e);
            return send_response(stream, 400, "application/json", &err).await;
        }
    };

    match p.identity_registry.register_identity(&secret, &blinding).await {
        Ok(identity) => {
            let root = p.identity_registry.current_root().await;
            let response = ZkIdentityRegisterResponse {
                success: true,
                commitment: hex::encode(identity.commitment),
                index: identity.index,
                merkle_root: hex::encode(root),
            };
            let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
            send_response(stream, 200, "application/json", &json).await
        }
        Err(e) => {
            let err = format!(r#"{{"error":"{}"}}"#, e);
            send_response(stream, 500, "application/json", &err).await
        }
    }
}

pub async fn zk_identity_verify(
    stream: &mut TcpStream,
    privacy: &Option<Arc<PrivacyServiceManager>>,
    body: &str,
) -> NonosResult<()> {
    let Some(p) = privacy else {
        return send_response(stream, 503, "application/json", r#"{"error":"Privacy services not available"}"#).await;
    };

    let req: ZkIdentityVerifyRequest = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(_) => {
            return send_response(stream, 400, "application/json", r#"{"error":"Invalid JSON"}"#).await;
        }
    };

    let proof = match hex::decode(req.proof.strip_prefix("0x").unwrap_or(&req.proof)) {
        Ok(p) => p,
        Err(e) => {
            let err = format!(r#"{{"error":"Invalid proof: {}"}}"#, e);
            return send_response(stream, 400, "application/json", &err).await;
        }
    };

    let merkle_root = match parse_hex_32(&req.merkle_root) {
        Ok(r) => r,
        Err(e) => {
            let err = format!(r#"{{"error":"Invalid merkle_root: {}"}}"#, e);
            return send_response(stream, 400, "application/json", &err).await;
        }
    };

    let nullifier = match parse_hex_32(&req.nullifier) {
        Ok(n) => n,
        Err(e) => {
            let err = format!(r#"{{"error":"Invalid nullifier: {}"}}"#, e);
            return send_response(stream, 400, "application/json", &err).await;
        }
    };

    let scope = match parse_hex_32(&req.scope) {
        Ok(s) => s,
        Err(e) => {
            let err = format!(r#"{{"error":"Invalid scope: {}"}}"#, e);
            return send_response(stream, 400, "application/json", &err).await;
        }
    };

    let signal_hash = if let Some(ref sh) = req.signal_hash {
        Some(parse_hex_32(sh).map_err(|e| {
            nonos_types::NonosError::Internal(format!("Invalid signal_hash: {}", e))
        })?)
    } else {
        None
    };

    match p.identity_registry.verify_proof(&proof, &merkle_root, &nullifier, &scope, signal_hash.as_ref()).await {
        Ok(result) => {
            let response = ZkIdentityVerifyResponse {
                valid: result.valid,
                reason: result.reason,
                nullifier_recorded: result.nullifier_recorded,
            };
            let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
            send_response(stream, 200, "application/json", &json).await
        }
        Err(e) => {
            let err = format!(r#"{{"error":"{}"}}"#, e);
            send_response(stream, 500, "application/json", &err).await
        }
    }
}

pub async fn zk_identity_root(
    stream: &mut TcpStream,
    privacy: &Option<Arc<PrivacyServiceManager>>,
) -> NonosResult<()> {
    let Some(p) = privacy else {
        return send_response(stream, 503, "application/json", r#"{"error":"Privacy services not available"}"#).await;
    };

    let root = p.identity_registry.current_root().await;
    let response = IdentityRootResponse {
        root: hex::encode(root),
    };

    let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    send_response(stream, 200, "application/json", &json).await
}

pub async fn mixer_status(
    stream: &mut TcpStream,
    privacy: &Option<Arc<PrivacyServiceManager>>,
) -> NonosResult<()> {
    let Some(p) = privacy else {
        return send_response(stream, 503, "application/json", r#"{"error":"Privacy services not available"}"#).await;
    };

    let (deposits, spends, failed) = p.note_mixer.stats();
    let root = p.note_mixer.root().await;

    let response = MixerStatusResponse {
        note_count: p.note_mixer.note_count().await,
        spent_count: p.note_mixer.spent_count().await,
        merkle_root: hex::encode(root),
        deposits,
        spends,
        failed_spends: failed,
    };

    let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    send_response(stream, 200, "application/json", &json).await
}

pub async fn mixer_deposit(
    stream: &mut TcpStream,
    privacy: &Option<Arc<PrivacyServiceManager>>,
    body: &str,
) -> NonosResult<()> {
    let Some(p) = privacy else {
        return send_response(stream, 503, "application/json", r#"{"error":"Privacy services not available"}"#).await;
    };

    let req: NoteDepositRequest = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(_) => {
            return send_response(stream, 400, "application/json", r#"{"error":"Invalid JSON"}"#).await;
        }
    };

    let secret = match parse_hex_32(&req.secret) {
        Ok(s) => s,
        Err(e) => {
            let err = format!(r#"{{"error":"Invalid secret: {}"}}"#, e);
            return send_response(stream, 400, "application/json", &err).await;
        }
    };

    let randomness = match parse_hex_32(&req.randomness) {
        Ok(r) => r,
        Err(e) => {
            let err = format!(r#"{{"error":"Invalid randomness: {}"}}"#, e);
            return send_response(stream, 400, "application/json", &err).await;
        }
    };

    let amount: u128 = match req.amount.parse() {
        Ok(a) => a,
        Err(_) => {
            return send_response(stream, 400, "application/json", r#"{"error":"Invalid amount"}"#).await;
        }
    };

    let mut note = Note::new(secret, amount, ASSET_ETH, randomness);

    match p.note_mixer.deposit(&mut note).await {
        Ok(index) => {
            let root = p.note_mixer.root().await;
            let response = NoteDepositResponse {
                success: true,
                commitment: hex::encode(note.commitment()),
                index,
                merkle_root: hex::encode(root),
            };
            let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
            send_response(stream, 200, "application/json", &json).await
        }
        Err(e) => {
            let err = format!(r#"{{"error":"{}"}}"#, e);
            send_response(stream, 500, "application/json", &err).await
        }
    }
}

pub async fn mixer_spend(
    stream: &mut TcpStream,
    privacy: &Option<Arc<PrivacyServiceManager>>,
    body: &str,
) -> NonosResult<()> {
    let Some(p) = privacy else {
        return send_response(stream, 503, "application/json", r#"{"error":"Privacy services not available"}"#).await;
    };

    let req: NoteSpendRequest = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(_) => {
            return send_response(stream, 400, "application/json", r#"{"error":"Invalid JSON"}"#).await;
        }
    };

    let merkle_root = match parse_hex_32(&req.merkle_root) {
        Ok(r) => r,
        Err(e) => {
            let err = format!(r#"{{"error":"Invalid merkle_root: {}"}}"#, e);
            return send_response(stream, 400, "application/json", &err).await;
        }
    };

    let nullifier = match parse_hex_32(&req.nullifier) {
        Ok(n) => n,
        Err(e) => {
            let err = format!(r#"{{"error":"Invalid nullifier: {}"}}"#, e);
            return send_response(stream, 400, "application/json", &err).await;
        }
    };

    let recipient = match parse_hex_32(&req.recipient) {
        Ok(r) => r,
        Err(e) => {
            let err = format!(r#"{{"error":"Invalid recipient: {}"}}"#, e);
            return send_response(stream, 400, "application/json", &err).await;
        }
    };

    let fee: u128 = match req.fee.parse() {
        Ok(f) => f,
        Err(_) => {
            return send_response(stream, 400, "application/json", r#"{"error":"Invalid fee"}"#).await;
        }
    };

    let proof = match hex::decode(req.proof.strip_prefix("0x").unwrap_or(&req.proof)) {
        Ok(p) => p,
        Err(e) => {
            let err = format!(r#"{{"error":"Invalid proof: {}"}}"#, e);
            return send_response(stream, 400, "application/json", &err).await;
        }
    };

    let spend_req = SpendRequest {
        merkle_root,
        nullifier,
        recipient,
        fee,
        merkle_path: vec![],
        proof,
    };

    match p.note_mixer.spend(&spend_req).await {
        Ok(result) => {
            let response = NoteSpendResponse {
                success: result.success,
                reason: result.reason,
                tx_hash: result.tx_hash.map(|h| hex::encode(h)),
            };
            let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
            send_response(stream, 200, "application/json", &json).await
        }
        Err(e) => {
            let err = format!(r#"{{"error":"{}"}}"#, e);
            send_response(stream, 500, "application/json", &err).await
        }
    }
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
