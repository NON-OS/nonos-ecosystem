use super::responses::*;

#[test]
fn test_status_response_serialization() {
    let response = StatusResponse {
        node_id: "test".to_string(),
        status: "Running".to_string(),
        tier: "Community".to_string(),
        uptime_secs: 3600,
        active_connections: 10,
        total_requests: 1000,
        successful_requests: 990,
        quality_score: 0.95,
        staked_nox: 100.0,
        pending_rewards: 5.5,
        streak_days: 7,
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"node_id\":\"test\""));
}

#[test]
fn test_privacy_stats_serialization() {
    let response = PrivacyStatsResponse {
        available: true,
        zk_proofs_issued: 100,
        zk_verifications: 95,
        cache_hits: 500,
        cache_misses: 50,
        cache_mix_ops: 200,
        tracking_blocked: 1000,
        tracking_total: 5000,
        tracking_block_rate: 20.0,
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"tracking_block_rate\":20.0"));
}
