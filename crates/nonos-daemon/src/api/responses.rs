use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct StatusResponse {
    pub node_id: String,
    pub status: String,
    pub tier: String,
    pub uptime_secs: u64,
    pub active_connections: usize,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub quality_score: f64,
    pub staked_nox: f64,
    pub pending_rewards: f64,
    pub streak_days: u32,
}

#[derive(Serialize)]
pub struct MetricsResponse {
    pub node_id: String,
    pub quality: QualityMetrics,
    pub requests: RequestMetrics,
    pub network: NetworkMetrics,
    pub rewards: RewardsMetrics,
}

#[derive(Serialize)]
pub struct QualityMetrics {
    pub uptime: f64,
    pub success_rate: f64,
    pub latency_score: f64,
    pub reliability: f64,
    pub total: f64,
}

#[derive(Serialize)]
pub struct RequestMetrics {
    pub total: u64,
    pub successful: u64,
    pub failed: u64,
}

#[derive(Serialize)]
pub struct NetworkMetrics {
    pub active_connections: usize,
    pub peer_count: usize,
}

#[derive(Serialize)]
pub struct RewardsMetrics {
    pub staked_nox: f64,
    pub pending_rewards: f64,
    pub streak_days: u32,
    pub tier: String,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub healthy: bool,
    pub status: String,
    pub uptime_secs: u64,
}

#[derive(Serialize)]
pub struct PeersResponse {
    pub count: usize,
    pub peers: Vec<PeerGeoInfo>,
}

#[derive(Serialize, Clone)]
pub struct PeerGeoInfo {
    pub id: String,
    pub address: String,
    pub lat: f64,
    pub lon: f64,
    pub city: String,
    pub country: String,
    pub country_code: String,
    pub latency_ms: Option<u32>,
    pub connected: bool,
    pub is_bootstrap: bool,
}

#[derive(Serialize)]
pub struct DiagnosticsResponse {
    pub all_passed: bool,
    pub checks: Vec<DiagnosticCheck>,
}

#[derive(Serialize)]
pub struct DiagnosticCheck {
    pub name: String,
    pub status: String,
    pub message: String,
}

#[derive(Serialize)]
pub struct PrivacyStatsResponse {
    pub available: bool,
    pub zk_proofs_issued: u64,
    pub zk_verifications: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_mix_ops: u64,
    pub tracking_blocked: u64,
    pub tracking_total: u64,
    pub tracking_block_rate: f64,
    pub identity_registrations: u64,
    pub identity_verifications_passed: u64,
    pub identity_verifications_failed: u64,
    pub note_deposits: u64,
    pub note_spends: u64,
    pub note_failed_spends: u64,
}

#[derive(Deserialize)]
pub struct TrackingCheckRequest {
    pub domain: String,
}

#[derive(Serialize)]
pub struct TrackingCheckResponse {
    pub blocked: bool,
    pub domain: String,
}

#[derive(Deserialize)]
pub struct TrackingBlockRequest {
    pub domain: String,
}

#[derive(Serialize)]
pub struct TrackingBlockResponse {
    pub success: bool,
    pub domain: String,
}

#[derive(Deserialize)]
pub struct IdentityRegisterRequest {
    pub commitment: String,
}

#[derive(Serialize)]
pub struct IdentityRegisterResponse {
    pub success: bool,
    pub index: usize,
}

#[derive(Serialize)]
pub struct IdentityRootResponse {
    pub root: String,
}

#[derive(Deserialize)]
pub struct ZkIdentityRegisterRequest {
    pub secret: String,
    pub blinding: String,
}

#[derive(Serialize)]
pub struct ZkIdentityRegisterResponse {
    pub success: bool,
    pub commitment: String,
    pub index: usize,
    pub merkle_root: String,
}

#[derive(Deserialize)]
pub struct ZkIdentityVerifyRequest {
    pub proof: String,
    pub merkle_root: String,
    pub nullifier: String,
    pub scope: String,
    pub signal_hash: Option<String>,
}

#[derive(Serialize)]
pub struct ZkIdentityVerifyResponse {
    pub valid: bool,
    pub reason: Option<String>,
    pub nullifier_recorded: bool,
}

#[derive(Deserialize)]
pub struct NoteDepositRequest {
    pub secret: String,
    pub amount: String,
    pub asset: String,
    pub randomness: String,
}

#[derive(Serialize)]
pub struct NoteDepositResponse {
    pub success: bool,
    pub commitment: String,
    pub index: usize,
    pub merkle_root: String,
}

#[derive(Deserialize)]
pub struct NoteSpendRequest {
    pub merkle_root: String,
    pub nullifier: String,
    pub recipient: String,
    pub fee: String,
    pub proof: String,
}

#[derive(Serialize)]
pub struct NoteSpendResponse {
    pub success: bool,
    pub reason: Option<String>,
    pub tx_hash: Option<String>,
}

#[derive(Serialize)]
pub struct MixerStatusResponse {
    pub note_count: usize,
    pub spent_count: usize,
    pub merkle_root: String,
    pub deposits: u64,
    pub spends: u64,
    pub failed_spends: u64,
}

#[derive(Serialize)]
pub struct StakingInfoResponse {
    pub available: bool,
    pub staker_address: String,
    pub staked_amount: String,
    pub balance: String,
    pub tier: String,
    pub tier_index: u8,
}

#[derive(Serialize)]
pub struct BalanceResponse {
    pub balance: String,
    pub balance_raw: String,
}

#[derive(Serialize)]
pub struct TierResponse {
    pub tier: String,
    pub tier_index: u8,
    pub multiplier: f64,
}

#[derive(Deserialize)]
pub struct StakeRequest {
    pub amount: f64,
}

#[derive(Serialize)]
pub struct StakeResponse {
    pub success: bool,
    pub tx_hash: String,
    pub amount: f64,
}

#[derive(Deserialize)]
pub struct UnstakeRequest {
    pub amount: f64,
}

#[derive(Serialize)]
pub struct UnstakeResponse {
    pub success: bool,
    pub tx_hash: String,
    pub amount: f64,
}

#[derive(Deserialize)]
pub struct ApproveRequest {
    pub amount: f64,
}

#[derive(Serialize)]
pub struct ApproveResponse {
    pub success: bool,
    pub tx_hash: String,
    pub amount: f64,
}

#[derive(Deserialize)]
pub struct SetTierRequest {
    pub tier: String,
}

#[derive(Serialize)]
pub struct SetTierResponse {
    pub success: bool,
    pub tx_hash: String,
    pub tier: String,
}

#[derive(Serialize)]
pub struct PendingRewardsResponse {
    pub available: bool,
    pub pending_onchain: String,
    pub pending_local: String,
    pub streak_days: u32,
}

#[derive(Serialize)]
pub struct ClaimResponse {
    pub success: bool,
    pub tx_hash: String,
    pub amount: String,
    pub epoch: u64,
}

#[derive(Serialize)]
pub struct ClaimHistoryItem {
    pub epoch: u64,
    pub amount: String,
    pub tx_hash: String,
    pub claimed_at: String,
}

#[derive(Serialize)]
pub struct ClaimHistoryResponse {
    pub claims: Vec<ClaimHistoryItem>,
    pub total_claimed: String,
}

#[derive(Deserialize)]
pub struct AutoClaimEnableRequest {
    pub threshold: f64,
}

#[derive(Serialize)]
pub struct AutoClaimResponse {
    pub success: bool,
    pub enabled: bool,
    pub threshold: f64,
}

#[derive(Serialize)]
pub struct ApyResponse {
    pub estimated_apy: f64,
    pub stake: String,
    pub tier: String,
    pub daily_emission: f64,
}
