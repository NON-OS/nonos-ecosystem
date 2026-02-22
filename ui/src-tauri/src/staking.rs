use crate::blockchain::{
    send_transaction_sepolia, fetch_sepolia_balances,
    NOX_TOKEN_ADDRESS_SEPOLIA, NOX_STAKING_ADDRESS_SEPOLIA,
};
use crate::helpers::format_wei;
use crate::state::AppState;
use crate::types::{StakingStatusResponse, STAKING_TIERS};
use crate::wallet::state::WALLET_MANAGER;
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
    _state: State<'_, AppState>,
    amount: String,
) -> Result<String, String> {
    let manager = WALLET_MANAGER.read().await;
    let wallet = manager.active().ok_or("Wallet not unlocked")?;

    if !wallet.is_unlocked() {
        return Err("Wallet is locked".into());
    }

    let private_key = wallet.get_account_private_key(0)
        .map_err(|e| format!("Failed to get signing key: {}", e))?;

    let address = wallet.address().to_hex();
    drop(manager);

    let amount_nox: f64 = amount.parse()
        .map_err(|_| "Invalid amount")?;
    let amount_wei = (amount_nox * 1e18) as u128;

    let (_, sepolia_nox) = fetch_sepolia_balances(&address).await;
    if amount_wei > sepolia_nox {
        return Err(format!(
            "Insufficient Sepolia NOX balance. Have {} NOX, staking {} NOX",
            sepolia_nox as f64 / 1e18,
            amount_nox
        ));
    }

    let approve_data = format!(
        "0x095ea7b3{:0>64}{:0>64}",
        NOX_STAKING_ADDRESS_SEPOLIA.trim_start_matches("0x"),
        format!("{:x}", amount_wei)
    );
    let approve_data_bytes = hex::decode(approve_data.trim_start_matches("0x"))
        .map_err(|e| format!("Encode error: {}", e))?;

    let _approve_tx = send_transaction_sepolia(
        &private_key,
        NOX_TOKEN_ADDRESS_SEPOLIA,
        0,
        approve_data_bytes,
        100000
    ).await?;

    let stake_data = format!("0xa694fc3a{:0>64}", format!("{:x}", amount_wei));
    let stake_data_bytes = hex::decode(stake_data.trim_start_matches("0x"))
        .map_err(|e| format!("Encode error: {}", e))?;

    let stake_tx = send_transaction_sepolia(
        &private_key,
        NOX_STAKING_ADDRESS_SEPOLIA,
        0,
        stake_data_bytes,
        150000
    ).await?;

    Ok(format!("Staked {} NOX on Sepolia! Tx: {}", amount_nox, stake_tx))
}

#[tauri::command]
pub async fn staking_unstake(
    _state: State<'_, AppState>,
    amount: String,
) -> Result<String, String> {
    let manager = WALLET_MANAGER.read().await;
    let wallet = manager.active().ok_or("Wallet not unlocked")?;

    if !wallet.is_unlocked() {
        return Err("Wallet is locked".into());
    }

    let private_key = wallet.get_account_private_key(0)
        .map_err(|e| format!("Failed to get signing key: {}", e))?;

    drop(manager);

    let amount_nox: f64 = amount.parse()
        .map_err(|_| "Invalid amount")?;
    let amount_wei = (amount_nox * 1e18) as u128;

    let unstake_data = format!("0x2e17de78{:0>64}", format!("{:x}", amount_wei));
    let unstake_data_bytes = hex::decode(unstake_data.trim_start_matches("0x"))
        .map_err(|e| format!("Encode error: {}", e))?;

    let unstake_tx = send_transaction_sepolia(
        &private_key,
        NOX_STAKING_ADDRESS_SEPOLIA,
        0,
        unstake_data_bytes,
        150000
    ).await?;

    Ok(format!("Unstake initiated for {} NOX (14-day unbonding period). Tx: {}", amount_nox, unstake_tx))
}

#[tauri::command]
pub async fn staking_claim_rewards(
    _state: State<'_, AppState>,
) -> Result<String, String> {
    let manager = WALLET_MANAGER.read().await;
    let wallet = manager.active().ok_or("Wallet not unlocked")?;

    if !wallet.is_unlocked() {
        return Err("Wallet is locked".into());
    }

    let private_key = wallet.get_account_private_key(0)
        .map_err(|e| format!("Failed to get signing key: {}", e))?;

    drop(manager);

    let claim_data = "0x372500ab";
    let claim_data_bytes = hex::decode(claim_data.trim_start_matches("0x"))
        .map_err(|e| format!("Encode error: {}", e))?;

    let claim_tx = send_transaction_sepolia(
        &private_key,
        NOX_STAKING_ADDRESS_SEPOLIA,
        0,
        claim_data_bytes,
        150000
    ).await?;

    Ok(format!("Rewards claimed on Sepolia! Tx: {}", claim_tx))
}

#[tauri::command]
pub async fn staking_withdraw(
    _state: State<'_, AppState>,
) -> Result<String, String> {
    let manager = WALLET_MANAGER.read().await;
    let wallet = manager.active().ok_or("Wallet not unlocked")?;

    if !wallet.is_unlocked() {
        return Err("Wallet is locked".into());
    }

    let private_key = wallet.get_account_private_key(0)
        .map_err(|e| format!("Failed to get signing key: {}", e))?;

    drop(manager);

    let withdraw_data = "0x3ccfd60b";
    let withdraw_data_bytes = hex::decode(withdraw_data.trim_start_matches("0x"))
        .map_err(|e| format!("Encode error: {}", e))?;

    let withdraw_tx = send_transaction_sepolia(
        &private_key,
        NOX_STAKING_ADDRESS_SEPOLIA,
        0,
        withdraw_data_bytes,
        150000
    ).await?;

    Ok(format!("Withdrawal complete on Sepolia! Tx: {}", withdraw_tx))
}
