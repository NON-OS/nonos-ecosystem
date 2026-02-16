use crate::blockchain::{send_transaction, NOX_TOKEN_ADDRESS, NOX_STAKING_ADDRESS};
use crate::helpers::format_wei;
use crate::state::AppState;
use crate::types::{StakingStatusResponse, STAKING_TIERS};
use tauri::State;

fn get_staking_tier(staked_nox: u128) -> (usize, &'static str, &'static str) {
    for (i, (name, threshold, mult)) in STAKING_TIERS.iter().enumerate().rev() {
        if staked_nox >= *threshold {
            return (i, name, mult);
        }
    }
    (0, "None", "0x")
}

fn get_next_tier_threshold(current_tier: usize) -> u128 {
    if current_tier + 1 < STAKING_TIERS.len() {
        STAKING_TIERS[current_tier + 1].1
    } else {
        0
    }
}

#[tauri::command]
pub async fn staking_get_status(state: State<'_, AppState>) -> Result<StakingStatusResponse, String> {
    let wallet = state.wallet.read().await;

    if !wallet.initialized {
        return Err("Wallet not initialized".into());
    }

    let staked_nox = wallet.staked_amount / 10u128.pow(18);
    let (tier_idx, tier_name, tier_mult) = get_staking_tier(staked_nox);
    let next_threshold = get_next_tier_threshold(tier_idx);

    let base_apy = 10.0;
    let multiplier: f64 = tier_mult.trim_end_matches('x').parse().unwrap_or(1.0);
    let estimated_apy = base_apy * multiplier;

    Ok(StakingStatusResponse {
        staked_amount: format_wei(wallet.staked_amount),
        tier: tier_name.to_string(),
        tier_multiplier: tier_mult.to_string(),
        pending_rewards: format_wei(wallet.pending_rewards),
        current_epoch: wallet.current_epoch,
        next_tier_threshold: if next_threshold > 0 {
            format!("{} NOX", next_threshold)
        } else {
            "Max tier".to_string()
        },
        estimated_apy: format!("{:.1}%", estimated_apy),
    })
}

#[tauri::command]
pub async fn staking_stake(
    state: State<'_, AppState>,
    amount: String,
) -> Result<String, String> {
    if NOX_STAKING_ADDRESS == "0x0000000000000000000000000000000000000000" {
        return Err("Staking contract not yet deployed to mainnet. Coming soon!".into());
    }

    let wallet = state.wallet.read().await;

    if !wallet.initialized || wallet.locked {
        return Err("Wallet locked or not initialized".into());
    }

    let private_key = wallet.private_key.clone()
        .ok_or("Private key not available")?;

    let amount_nox: f64 = amount.parse()
        .map_err(|_| "Invalid amount")?;
    let amount_wei = (amount_nox * 1e18) as u128;

    if amount_wei > wallet.nox_balance {
        return Err("Insufficient NOX balance".into());
    }

    drop(wallet);

    let approve_data = format!(
        "0x095ea7b3{:0>64}{:0>64}",
        NOX_STAKING_ADDRESS.trim_start_matches("0x"),
        format!("{:x}", amount_wei)
    );
    let approve_data_bytes = hex::decode(approve_data.trim_start_matches("0x"))
        .map_err(|e| format!("Encode error: {}", e))?;

    let _approve_tx = send_transaction(&private_key, NOX_TOKEN_ADDRESS, 0, approve_data_bytes).await?;

    let stake_data = format!("0xa694fc3a{:0>64}", format!("{:x}", amount_wei));
    let stake_data_bytes = hex::decode(stake_data.trim_start_matches("0x"))
        .map_err(|e| format!("Encode error: {}", e))?;

    let stake_tx = send_transaction(&private_key, NOX_STAKING_ADDRESS, 0, stake_data_bytes).await?;

    Ok(format!("Staked {} NOX! Tx: {}", amount_nox, stake_tx))
}

#[tauri::command]
pub async fn staking_unstake(
    state: State<'_, AppState>,
    amount: String,
) -> Result<String, String> {
    if NOX_STAKING_ADDRESS == "0x0000000000000000000000000000000000000000" {
        return Err("Staking contract not yet deployed to mainnet. Coming soon!".into());
    }

    let wallet = state.wallet.read().await;

    if !wallet.initialized || wallet.locked {
        return Err("Wallet locked or not initialized".into());
    }

    let private_key = wallet.private_key.clone()
        .ok_or("Private key not available")?;

    let amount_nox: f64 = amount.parse()
        .map_err(|_| "Invalid amount")?;
    let amount_wei = (amount_nox * 1e18) as u128;

    drop(wallet);

    let unstake_data = format!("0x2e17de78{:0>64}", format!("{:x}", amount_wei));
    let unstake_data_bytes = hex::decode(unstake_data.trim_start_matches("0x"))
        .map_err(|e| format!("Encode error: {}", e))?;

    let unstake_tx = send_transaction(&private_key, NOX_STAKING_ADDRESS, 0, unstake_data_bytes).await?;

    Ok(format!("Unstake initiated for {} NOX (14-day unbonding period). Tx: {}", amount_nox, unstake_tx))
}

#[tauri::command]
pub async fn staking_claim_rewards(
    state: State<'_, AppState>,
) -> Result<String, String> {
    if NOX_STAKING_ADDRESS == "0x0000000000000000000000000000000000000000" {
        return Err("Staking contract not yet deployed to mainnet. Coming soon!".into());
    }

    let wallet = state.wallet.read().await;

    if !wallet.initialized || wallet.locked {
        return Err("Wallet locked or not initialized".into());
    }

    let private_key = wallet.private_key.clone()
        .ok_or("Private key not available")?;

    drop(wallet);

    let claim_data = "0x372500ab";
    let claim_data_bytes = hex::decode(claim_data.trim_start_matches("0x"))
        .map_err(|e| format!("Encode error: {}", e))?;

    let claim_tx = send_transaction(&private_key, NOX_STAKING_ADDRESS, 0, claim_data_bytes).await?;

    Ok(format!("Rewards claimed! Tx: {}", claim_tx))
}

#[tauri::command]
pub async fn staking_withdraw(
    state: State<'_, AppState>,
) -> Result<String, String> {
    if NOX_STAKING_ADDRESS == "0x0000000000000000000000000000000000000000" {
        return Err("Staking contract not yet deployed to mainnet. Coming soon!".into());
    }

    let wallet = state.wallet.read().await;

    if !wallet.initialized || wallet.locked {
        return Err("Wallet locked or not initialized".into());
    }

    let private_key = wallet.private_key.clone()
        .ok_or("Private key not available")?;

    drop(wallet);

    let withdraw_data = "0x3ccfd60b";
    let withdraw_data_bytes = hex::decode(withdraw_data.trim_start_matches("0x"))
        .map_err(|e| format!("Encode error: {}", e))?;

    let withdraw_tx = send_transaction(&private_key, NOX_STAKING_ADDRESS, 0, withdraw_data_bytes).await?;

    Ok(format!("Withdrawal complete! Tx: {}", withdraw_tx))
}
