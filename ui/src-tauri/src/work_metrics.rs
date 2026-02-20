use serde::{Deserialize, Serialize};
use tauri::State;
use crate::state::AppState;

const DAEMON_API_URL: &str = "http://127.0.0.1:8080";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TrafficRelayMetrics {
    pub bytes_relayed: u64,
    pub relay_sessions: u64,
    pub successful_relays: u64,
    pub failed_relays: u64,
    pub avg_latency_ms: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ZkProofMetrics {
    pub proofs_generated: u64,
    pub proofs_verified: u64,
    pub avg_generation_time_ms: f64,
    pub verification_failures: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MixerOpsMetrics {
    pub deposits_processed: u64,
    pub spends_processed: u64,
    pub total_value_mixed: u128,
    pub pool_participations: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EntropyMetrics {
    pub entropy_bytes_contributed: u64,
    pub entropy_requests_served: u64,
    pub quality_score: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RegistryOpsMetrics {
    pub registrations_processed: u64,
    pub lookups_served: u64,
    pub sync_operations: u64,
    pub failed_operations: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EpochInfo {
    pub current_epoch: u64,
    pub epoch_start_timestamp: u64,
    pub epoch_end_timestamp: u64,
    pub submitted_to_oracle: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WorkMetrics {
    pub traffic_relay: TrafficRelayMetrics,
    pub zk_proofs: ZkProofMetrics,
    pub mixer_ops: MixerOpsMetrics,
    pub entropy: EntropyMetrics,
    pub registry_ops: RegistryOpsMetrics,
    pub epoch: EpochInfo,
    pub total_work_score: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkMetricsResponse {
    pub success: bool,
    pub data: WorkMetrics,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkCategoryBreakdown {
    pub name: String,
    pub weight: u8,
    pub score: f64,
    pub raw_value: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkDashboard {
    pub metrics: WorkMetrics,
    pub categories: Vec<WorkCategoryBreakdown>,
    pub estimated_epoch_reward: String,
    pub network_rank: Option<u32>,
    pub network_total_nodes: u32,
}

async fn fetch_work_metrics() -> Result<WorkMetrics, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let url = format!("{}/api/v1/work/metrics", DAEMON_API_URL);

    let response = client.get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to daemon: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Daemon returned error: {}", response.status()));
    }

    let data: WorkMetricsResponse = response.json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(data.data)
}

#[tauri::command]
pub async fn work_get_metrics(_state: State<'_, AppState>) -> Result<WorkMetrics, String> {
    fetch_work_metrics().await
}

#[tauri::command]
pub async fn work_get_dashboard(_state: State<'_, AppState>) -> Result<WorkDashboard, String> {
    let metrics = fetch_work_metrics().await?;

    let categories = vec![
        WorkCategoryBreakdown {
            name: "Traffic Relay".into(),
            weight: 30,
            score: calculate_traffic_score(&metrics.traffic_relay),
            raw_value: metrics.traffic_relay.bytes_relayed,
        },
        WorkCategoryBreakdown {
            name: "ZK Proofs".into(),
            weight: 25,
            score: calculate_zk_score(&metrics.zk_proofs),
            raw_value: metrics.zk_proofs.proofs_generated + metrics.zk_proofs.proofs_verified,
        },
        WorkCategoryBreakdown {
            name: "Mixer Operations".into(),
            weight: 20,
            score: calculate_mixer_score(&metrics.mixer_ops),
            raw_value: metrics.mixer_ops.deposits_processed + metrics.mixer_ops.spends_processed,
        },
        WorkCategoryBreakdown {
            name: "Entropy".into(),
            weight: 15,
            score: calculate_entropy_score(&metrics.entropy),
            raw_value: metrics.entropy.entropy_bytes_contributed,
        },
        WorkCategoryBreakdown {
            name: "Registry".into(),
            weight: 10,
            score: calculate_registry_score(&metrics.registry_ops),
            raw_value: metrics.registry_ops.registrations_processed + metrics.registry_ops.lookups_served,
        },
    ];

    let daily_emission: f64 = 54794.52;
    let node_share = 0.70;
    let estimated_reward = (metrics.total_work_score / 100.0) * daily_emission * 7.0 * node_share;

    Ok(WorkDashboard {
        metrics,
        categories,
        estimated_epoch_reward: format!("{:.2}", estimated_reward),
        network_rank: None,
        network_total_nodes: 0,
    })
}

#[tauri::command]
pub async fn work_get_epoch(_state: State<'_, AppState>) -> Result<EpochInfo, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let url = format!("{}/api/v1/work/epoch", DAEMON_API_URL);

    let response = client.get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to daemon: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Daemon returned error: {}", response.status()));
    }

    #[derive(Deserialize)]
    struct EpochResponse {
        epoch: u64,
        epoch_start: u64,
        epoch_end: u64,
        submitted: bool,
    }

    let data: EpochResponse = response.json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(EpochInfo {
        current_epoch: data.epoch,
        epoch_start_timestamp: data.epoch_start,
        epoch_end_timestamp: data.epoch_end,
        submitted_to_oracle: data.submitted,
    })
}

fn calculate_traffic_score(m: &TrafficRelayMetrics) -> f64 {
    const BASELINE: u64 = 1_000_000_000;
    let ratio = m.bytes_relayed as f64 / BASELINE as f64;
    (ratio * 100.0).min(100.0)
}

fn calculate_zk_score(m: &ZkProofMetrics) -> f64 {
    const BASELINE: u64 = 1000;
    let total = m.proofs_generated + m.proofs_verified;
    let ratio = total as f64 / BASELINE as f64;
    (ratio * 100.0).min(100.0)
}

fn calculate_mixer_score(m: &MixerOpsMetrics) -> f64 {
    const BASELINE: u64 = 100;
    let total = m.deposits_processed + m.spends_processed;
    let ratio = total as f64 / BASELINE as f64;
    (ratio * 100.0).min(100.0)
}

fn calculate_entropy_score(m: &EntropyMetrics) -> f64 {
    const BASELINE: u64 = 10_000_000;
    let ratio = m.entropy_bytes_contributed as f64 / BASELINE as f64;
    (ratio * 100.0).min(100.0)
}

fn calculate_registry_score(m: &RegistryOpsMetrics) -> f64 {
    const BASELINE: u64 = 500;
    let total = m.registrations_processed + m.lookups_served;
    let ratio = total as f64 / BASELINE as f64;
    (ratio * 100.0).min(100.0)
}
