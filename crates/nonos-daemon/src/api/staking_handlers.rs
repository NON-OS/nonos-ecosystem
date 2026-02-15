use super::handlers::send_response;
use super::responses::*;
use crate::contracts::ContractClient;
use nonos_types::{EthAddress, NodeTier, NonosResult, TokenAmount, NOX_DECIMALS};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::RwLock;

pub async fn staking_info(
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

    let staked = client.get_stake(&staker).await.unwrap_or_else(|_| TokenAmount::zero(NOX_DECIMALS));
    let balance = client.get_balance(&staker).await.unwrap_or_else(|_| TokenAmount::zero(NOX_DECIMALS));
    let tier = client.get_tier(&staker).await.unwrap_or(NodeTier::Bronze);

    let response = StakingInfoResponse {
        available: true,
        staker_address: format!("0x{}", hex::encode(&staker.0)),
        staked_amount: staked.to_decimal(),
        balance: balance.to_decimal(),
        tier: format!("{:?}", tier),
        tier_index: tier.to_index(),
    };

    let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    send_response(stream, 200, "application/json", &json).await
}

pub async fn staking_balance(
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
    let balance = match client.get_balance(&staker).await {
        Ok(b) => b,
        Err(e) => {
            let err = format!(r#"{{"error":"{}"}}"#, e);
            return send_response(stream, 500, "application/json", &err).await;
        }
    };

    let response = BalanceResponse {
        balance: balance.to_decimal(),
        balance_raw: balance.raw.to_string(),
    };

    let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    send_response(stream, 200, "application/json", &json).await
}

pub async fn staking_tier(
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
    let tier = match client.get_tier(&staker).await {
        Ok(t) => t,
        Err(e) => {
            let err = format!(r#"{{"error":"{}"}}"#, e);
            return send_response(stream, 500, "application/json", &err).await;
        }
    };

    let response = TierResponse {
        tier: format!("{:?}", tier),
        tier_index: tier.to_index(),
        multiplier: tier.multiplier(),
    };

    let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    send_response(stream, 200, "application/json", &json).await
}

pub async fn staking_stake(
    stream: &mut TcpStream,
    contract_client: &Option<Arc<RwLock<ContractClient>>>,
    body: &str,
) -> NonosResult<()> {
    let Some(client_arc) = contract_client else {
        return send_response(stream, 503, "application/json", r#"{"error":"Contract client not available"}"#).await;
    };

    let req: StakeRequest = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(_) => {
            return send_response(stream, 400, "application/json", r#"{"error":"Invalid JSON"}"#).await;
        }
    };

    let amount = TokenAmount::from_raw((req.amount * 1e18) as u128, NOX_DECIMALS);
    let client = client_arc.read().await;

    match client.stake(&amount).await {
        Ok(tx_hash) => {
            let response = StakeResponse {
                success: true,
                tx_hash: format!("{:?}", tx_hash),
                amount: req.amount,
            };
            let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
            send_response(stream, 200, "application/json", &json).await
        }
        Err(e) => {
            let err = format!(r#"{{"error":"Stake failed: {}"}}"#, e);
            send_response(stream, 500, "application/json", &err).await
        }
    }
}

pub async fn staking_unstake(
    stream: &mut TcpStream,
    contract_client: &Option<Arc<RwLock<ContractClient>>>,
    body: &str,
) -> NonosResult<()> {
    let Some(client_arc) = contract_client else {
        return send_response(stream, 503, "application/json", r#"{"error":"Contract client not available"}"#).await;
    };

    let req: UnstakeRequest = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(_) => {
            return send_response(stream, 400, "application/json", r#"{"error":"Invalid JSON"}"#).await;
        }
    };

    let amount = TokenAmount::from_raw((req.amount * 1e18) as u128, NOX_DECIMALS);
    let client = client_arc.read().await;

    match client.unstake(&amount).await {
        Ok(tx_hash) => {
            let response = UnstakeResponse {
                success: true,
                tx_hash: format!("{:?}", tx_hash),
                amount: req.amount,
            };
            let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
            send_response(stream, 200, "application/json", &json).await
        }
        Err(e) => {
            let err = format!(r#"{{"error":"Unstake failed: {}"}}"#, e);
            send_response(stream, 500, "application/json", &err).await
        }
    }
}

pub async fn staking_approve(
    stream: &mut TcpStream,
    contract_client: &Option<Arc<RwLock<ContractClient>>>,
    body: &str,
) -> NonosResult<()> {
    let Some(client_arc) = contract_client else {
        return send_response(stream, 503, "application/json", r#"{"error":"Contract client not available"}"#).await;
    };

    let req: ApproveRequest = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(_) => {
            return send_response(stream, 400, "application/json", r#"{"error":"Invalid JSON"}"#).await;
        }
    };

    let amount = TokenAmount::from_raw((req.amount * 1e18) as u128, NOX_DECIMALS);
    let client = client_arc.read().await;

    match client.approve(&amount).await {
        Ok(tx_hash) => {
            let response = ApproveResponse {
                success: true,
                tx_hash: format!("{:?}", tx_hash),
                amount: req.amount,
            };
            let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
            send_response(stream, 200, "application/json", &json).await
        }
        Err(e) => {
            let err = format!(r#"{{"error":"Approve failed: {}"}}"#, e);
            send_response(stream, 500, "application/json", &err).await
        }
    }
}

pub async fn staking_set_tier(
    stream: &mut TcpStream,
    contract_client: &Option<Arc<RwLock<ContractClient>>>,
    body: &str,
) -> NonosResult<()> {
    let Some(client_arc) = contract_client else {
        return send_response(stream, 503, "application/json", r#"{"error":"Contract client not available"}"#).await;
    };

    let req: SetTierRequest = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(_) => {
            return send_response(stream, 400, "application/json", r#"{"error":"Invalid JSON"}"#).await;
        }
    };

    let tier = match req.tier.to_lowercase().as_str() {
        "bronze" => NodeTier::Bronze,
        "silver" => NodeTier::Silver,
        "gold" => NodeTier::Gold,
        "platinum" => NodeTier::Platinum,
        "diamond" => NodeTier::Diamond,
        _ => {
            return send_response(stream, 400, "application/json",
                r#"{"error":"Invalid tier. Use: Bronze, Silver, Gold, Platinum, Diamond"}"#).await;
        }
    };

    let client = client_arc.read().await;

    match client.set_tier(tier).await {
        Ok(tx_hash) => {
            let response = SetTierResponse {
                success: true,
                tx_hash: format!("{:?}", tx_hash),
                tier: format!("{:?}", tier),
            };
            let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
            send_response(stream, 200, "application/json", &json).await
        }
        Err(e) => {
            let err = format!(r#"{{"error":"Set tier failed: {}"}}"#, e);
            send_response(stream, 500, "application/json", &err).await
        }
    }
}
