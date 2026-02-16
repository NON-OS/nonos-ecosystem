use crate::state::AppState;
use crate::types::{PrivacyStatsResponse, ZkIdentityResponse, TrackingCheckResponse};
use tauri::State;

const NONOS_API_URL: &str = "http://127.0.0.1:8420/api";

const KNOWN_TRACKERS: &[&str] = &[
    "google-analytics.com",
    "googletagmanager.com",
    "facebook.com",
    "connect.facebook.net",
    "doubleclick.net",
    "googlesyndication.com",
    "googleadservices.com",
    "amazon-adsystem.com",
    "scorecardresearch.com",
    "quantserve.com",
    "adsrvr.org",
    "criteo.com",
    "taboola.com",
    "outbrain.com",
    "chartbeat.com",
    "mixpanel.com",
    "segment.io",
    "amplitude.com",
    "hotjar.com",
    "fullstory.com",
    "clarity.ms",
];

#[tauri::command]
pub async fn privacy_get_stats(state: State<'_, AppState>) -> Result<PrivacyStatsResponse, String> {
    let nodes = state.nodes.read().await;

    if !nodes.embedded_running {
        return Err("NONOS node not running. Start the node first.".into());
    }

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/privacy/stats", NONOS_API_URL))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| format!("Failed to connect to NONOS node: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("NONOS node returned error: {}", response.status()));
    }

    let stats: serde_json::Value = response.json().await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(PrivacyStatsResponse {
        zk_proofs_issued: stats["zk_proofs_issued"].as_u64().unwrap_or(0),
        zk_verifications: stats["zk_verifications"].as_u64().unwrap_or(0),
        cache_hits: stats["cache_hits"].as_u64().unwrap_or(0),
        cache_misses: stats["cache_misses"].as_u64().unwrap_or(0),
        cache_hit_rate: stats["cache_hit_rate"].as_f64().unwrap_or(0.0),
        tracking_blocked: stats["tracking_blocked"].as_u64().unwrap_or(0),
        tracking_total: stats["tracking_total"].as_u64().unwrap_or(0),
        block_rate: stats["block_rate"].as_f64().unwrap_or(0.0),
        stealth_payments: stats["stealth_payments"].as_u64().unwrap_or(0),
        stealth_scanned: stats["stealth_scanned"].as_u64().unwrap_or(0),
    })
}

#[tauri::command]
pub async fn privacy_check_tracking(
    state: State<'_, AppState>,
    domain: String,
) -> Result<TrackingCheckResponse, String> {
    let nodes = state.nodes.read().await;

    if !nodes.embedded_running {
        let blocked = KNOWN_TRACKERS.iter().any(|t| domain.contains(t));
        return Ok(TrackingCheckResponse {
            domain: domain.clone(),
            blocked,
            reason: if blocked { Some("Known tracker domain".into()) } else { None },
        });
    }

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/privacy/tracking/check", NONOS_API_URL))
        .json(&serde_json::json!({ "domain": domain }))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| format!("Failed to connect to NONOS node: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("NONOS node returned error: {}", response.status()));
    }

    let result: serde_json::Value = response.json().await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(TrackingCheckResponse {
        domain: domain.clone(),
        blocked: result["blocked"].as_bool().unwrap_or(false),
        reason: result["reason"].as_str().map(String::from),
    })
}

#[tauri::command]
pub async fn privacy_block_domain(
    state: State<'_, AppState>,
    domain: String,
) -> Result<(), String> {
    let nodes = state.nodes.read().await;

    if !nodes.embedded_running {
        return Err("NONOS node not running. Start the node first.".into());
    }

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/privacy/tracking/block", NONOS_API_URL))
        .json(&serde_json::json!({ "domain": domain }))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| format!("Failed to connect to NONOS node: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Failed to block domain: {}", response.status()));
    }

    Ok(())
}

#[tauri::command]
pub async fn privacy_generate_identity(
    state: State<'_, AppState>,
    name: Option<String>,
) -> Result<ZkIdentityResponse, String> {
    let nodes = state.nodes.read().await;

    if !nodes.embedded_running {
        return Err("NONOS node not running. Start the node first.".into());
    }

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/privacy/identity/register", NONOS_API_URL))
        .json(&serde_json::json!({ "name": name }))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Failed to connect to NONOS node: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Failed to generate identity: {}", response.status()));
    }

    let result: serde_json::Value = response.json().await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(ZkIdentityResponse {
        identity_id: result["identity_id"].as_str().unwrap_or("unknown").to_string(),
        commitment: result["commitment"].as_str().unwrap_or("").to_string(),
        merkle_root: result["merkle_root"].as_str().unwrap_or("").to_string(),
    })
}

#[tauri::command]
pub async fn privacy_get_identity_root(state: State<'_, AppState>) -> Result<String, String> {
    let nodes = state.nodes.read().await;

    if !nodes.embedded_running {
        return Err("NONOS node not running. Start the node first.".into());
    }

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/privacy/identity/root", NONOS_API_URL))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| format!("Failed to connect to NONOS node: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Failed to get identity root: {}", response.status()));
    }

    let result: serde_json::Value = response.json().await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(result["root"].as_str().unwrap_or("").to_string())
}

#[tauri::command]
pub async fn privacy_cache_store(
    state: State<'_, AppState>,
    content: String,
) -> Result<String, String> {
    let nodes = state.nodes.read().await;

    if !nodes.embedded_running {
        return Err("NONOS node not running. Start the node first.".into());
    }

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/privacy/cache/store", NONOS_API_URL))
        .json(&serde_json::json!({ "content": content }))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Failed to connect to NONOS node: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Failed to store in cache: {}", response.status()));
    }

    let result: serde_json::Value = response.json().await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(result["commitment"].as_str().unwrap_or("").to_string())
}
