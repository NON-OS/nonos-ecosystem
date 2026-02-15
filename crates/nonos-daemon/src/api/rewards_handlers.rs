use super::handlers::send_response;
use super::responses::*;
use crate::contracts::ContractClient;
use crate::rewards::RewardTracker;
use nonos_types::{EthAddress, NodeTier, NonosResult, TokenAmount, NOX_DECIMALS};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::RwLock;

pub async fn rewards_pending(
    stream: &mut TcpStream,
    contract_client: &Option<Arc<RwLock<ContractClient>>>,
    reward_tracker: &Option<Arc<RewardTracker>>,
    staker_address: Option<EthAddress>,
) -> NonosResult<()> {
    let pending_onchain = if let (Some(client_arc), Some(staker)) = (contract_client, staker_address) {
        let client = client_arc.read().await;
        client.get_pending_rewards(&staker).await.unwrap_or_else(|_| TokenAmount::zero(NOX_DECIMALS))
    } else {
        TokenAmount::zero(NOX_DECIMALS)
    };

    let (pending_local, streak) = if let Some(tracker) = reward_tracker {
        (tracker.pending_rewards().await, tracker.current_streak().await)
    } else {
        (TokenAmount::zero(NOX_DECIMALS), 0)
    };

    let response = PendingRewardsResponse {
        available: contract_client.is_some() || reward_tracker.is_some(),
        pending_onchain: pending_onchain.to_decimal(),
        pending_local: pending_local.to_decimal(),
        streak_days: streak,
    };

    let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    send_response(stream, 200, "application/json", &json).await
}

pub async fn rewards_claim(
    stream: &mut TcpStream,
    reward_tracker: &Option<Arc<RewardTracker>>,
) -> NonosResult<()> {
    let Some(tracker) = reward_tracker else {
        return send_response(stream, 503, "application/json", r#"{"error":"Reward tracker not available"}"#).await;
    };

    let epoch = nonos_types::EpochNumber(chrono::Utc::now().timestamp() as u64 / 86400 / 7);

    match tracker.claim(epoch).await {
        Ok(claim) => {
            let response = ClaimResponse {
                success: true,
                tx_hash: hex::encode(&claim.tx_hash.0),
                amount: claim.amount.to_decimal(),
                epoch: claim.epoch.0,
            };
            let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
            send_response(stream, 200, "application/json", &json).await
        }
        Err(e) => {
            let err = format!(r#"{{"error":"Claim failed: {}"}}"#, e);
            send_response(stream, 500, "application/json", &err).await
        }
    }
}

pub async fn rewards_history(
    stream: &mut TcpStream,
    reward_tracker: &Option<Arc<RewardTracker>>,
) -> NonosResult<()> {
    let Some(tracker) = reward_tracker else {
        return send_response(stream, 503, "application/json", r#"{"error":"Reward tracker not available"}"#).await;
    };

    let history = tracker.claim_history().await;
    let mut total_claimed_raw: u128 = 0;

    let claims: Vec<ClaimHistoryItem> = history.iter().map(|c| {
        total_claimed_raw = total_claimed_raw.saturating_add(c.amount.raw);
        ClaimHistoryItem {
            epoch: c.epoch.0,
            amount: c.amount.to_decimal(),
            tx_hash: hex::encode(&c.tx_hash.0),
            claimed_at: c.claimed_at.to_rfc3339(),
        }
    }).collect();

    let total_claimed = TokenAmount::from_raw(total_claimed_raw, NOX_DECIMALS);

    let response = ClaimHistoryResponse {
        claims,
        total_claimed: total_claimed.to_decimal(),
    };

    let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    send_response(stream, 200, "application/json", &json).await
}

pub async fn rewards_auto_claim_enable(
    stream: &mut TcpStream,
    reward_tracker: &Option<Arc<RewardTracker>>,
    body: &str,
) -> NonosResult<()> {
    let Some(tracker) = reward_tracker else {
        return send_response(stream, 503, "application/json", r#"{"error":"Reward tracker not available"}"#).await;
    };

    let req: AutoClaimEnableRequest = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(_) => {
            return send_response(stream, 400, "application/json", r#"{"error":"Invalid JSON"}"#).await;
        }
    };

    let threshold = TokenAmount::from_raw((req.threshold * 1e18) as u128, NOX_DECIMALS);
    tracker.enable_auto_claim(threshold).await;

    let response = AutoClaimResponse {
        success: true,
        enabled: true,
        threshold: req.threshold,
    };

    let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    send_response(stream, 200, "application/json", &json).await
}

pub async fn rewards_auto_claim_disable(
    stream: &mut TcpStream,
    reward_tracker: &Option<Arc<RewardTracker>>,
) -> NonosResult<()> {
    let Some(tracker) = reward_tracker else {
        return send_response(stream, 503, "application/json", r#"{"error":"Reward tracker not available"}"#).await;
    };

    tracker.disable_auto_claim().await;

    let response = AutoClaimResponse {
        success: true,
        enabled: false,
        threshold: 0.0,
    };

    let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    send_response(stream, 200, "application/json", &json).await
}

pub async fn rewards_apy(
    stream: &mut TcpStream,
    contract_client: &Option<Arc<RwLock<ContractClient>>>,
    staker_address: Option<EthAddress>,
) -> NonosResult<()> {
    let Some(client_arc) = contract_client else {
        return send_response(stream, 503, "application/json", r#"{"error":"Contract client not available"}"#).await;
    };
    let Some(staker) = staker_address else {
        return send_response(stream, 503, "application/json", r#"{"error":"Staker address not configured"}"#).await;
    };

    let client = client_arc.read().await;
    let stake = client.get_stake(&staker).await.unwrap_or_else(|_| TokenAmount::zero(NOX_DECIMALS));
    let tier = client.get_tier(&staker).await.unwrap_or(NodeTier::Bronze);

    let daily_emission: f64 = 43_500.0;
    let estimated_total_staked: f64 = 5_000_000.0;

    let stake_value: f64 = (stake.raw as f64) / 1e18;
    let weight = stake_value.sqrt() * tier.multiplier();
    let total_weight_estimate = estimated_total_staked.sqrt();

    let daily_reward = if total_weight_estimate > 0.0 {
        daily_emission * (weight / total_weight_estimate) * 0.9
    } else {
        0.0
    };

    let annual_reward = daily_reward * 365.0;
    let apy = if stake_value > 0.0 {
        (annual_reward / stake_value) * 100.0
    } else {
        0.0
    };

    let response = ApyResponse {
        estimated_apy: apy,
        stake: stake.to_decimal(),
        tier: format!("{:?}", tier),
        daily_emission,
    };

    let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    send_response(stream, 200, "application/json", &json).await
}
