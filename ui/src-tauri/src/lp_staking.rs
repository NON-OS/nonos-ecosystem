use crate::blockchain::{send_transaction_with_gas, NOX_TOKEN_ADDRESS};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

pub const LP_STAKING_ADDRESS: &str = "0x0000000000000000000000000000000000000000";

#[derive(Clone, Serialize, Deserialize)]
pub struct LockTier {
    pub id: u8,
    pub duration_days: u16,
    pub multiplier: u16,
    pub multiplier_display: String,
}

pub fn get_lock_tiers() -> Vec<LockTier> {
    vec![
        LockTier { id: 0, duration_days: 14, multiplier: 10000, multiplier_display: "1.00x".into() },
        LockTier { id: 1, duration_days: 30, multiplier: 12500, multiplier_display: "1.25x".into() },
        LockTier { id: 2, duration_days: 90, multiplier: 16000, multiplier_display: "1.60x".into() },
        LockTier { id: 3, duration_days: 180, multiplier: 20000, multiplier_display: "2.00x".into() },
        LockTier { id: 4, duration_days: 365, multiplier: 25000, multiplier_display: "2.50x".into() },
    ]
}

#[derive(Serialize)]
pub struct LPLockInfo {
    pub lock_id: u64,
    pub amount: String,
    pub tier: u8,
    pub tier_name: String,
    pub multiplier: String,
    pub lock_start: u64,
    pub lock_end: u64,
    pub is_locked: bool,
    pub pending_rewards: String,
}

#[derive(Serialize)]
pub struct LPStakingStatus {
    pub total_locked: String,
    pub weighted_total: String,
    pub locks: Vec<LPLockInfo>,
    pub total_pending_rewards: String,
    pub available_tiers: Vec<LockTier>,
    pub current_epoch: u64,
    pub epoch_lp_pool: String,
}

#[tauri::command]
pub async fn lp_get_status(state: State<'_, AppState>) -> Result<LPStakingStatus, String> {
    let wallet = state.wallet.read().await;

    if !wallet.initialized {
        return Err("Wallet not initialized".into());
    }

    Ok(LPStakingStatus {
        total_locked: "0".into(),
        weighted_total: "0".into(),
        locks: vec![],
        total_pending_rewards: "0".into(),
        available_tiers: get_lock_tiers(),
        current_epoch: 0,
        epoch_lp_pool: "0".into(),
    })
}

#[tauri::command]
pub async fn lp_lock(
    state: State<'_, AppState>,
    amount: String,
    tier: u8,
) -> Result<String, String> {
    if LP_STAKING_ADDRESS == "0x0000000000000000000000000000000000000000" {
        return Err("LP Staking contract not yet deployed. Coming soon!".into());
    }

    if tier > 4 {
        return Err("Invalid tier. Must be 0-4.".into());
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
        LP_STAKING_ADDRESS.trim_start_matches("0x"),
        format!("{:x}", amount_wei)
    );
    let approve_data_bytes = hex::decode(approve_data.trim_start_matches("0x"))
        .map_err(|e| format!("Encode error: {}", e))?;

    let _ = send_transaction_with_gas(&private_key, NOX_TOKEN_ADDRESS, 0, approve_data_bytes, 100000).await?;

    let lock_data = format!(
        "0x4a4de4a8{:0>64}{:0>64}",
        format!("{:x}", amount_wei),
        format!("{:x}", tier)
    );
    let lock_data_bytes = hex::decode(lock_data.trim_start_matches("0x"))
        .map_err(|e| format!("Encode error: {}", e))?;

    let lock_tx = send_transaction_with_gas(&private_key, LP_STAKING_ADDRESS, 0, lock_data_bytes, 200000).await?;

    let tier_info = get_lock_tiers().get(tier as usize).map(|t| t.duration_days).unwrap_or(0);
    Ok(format!("Locked {} NOX for {} days! Tx: {}", amount_nox, tier_info, lock_tx))
}

#[tauri::command]
pub async fn lp_unlock(
    state: State<'_, AppState>,
    lock_id: u64,
) -> Result<String, String> {
    if LP_STAKING_ADDRESS == "0x0000000000000000000000000000000000000000" {
        return Err("LP Staking contract not yet deployed. Coming soon!".into());
    }

    let wallet = state.wallet.read().await;

    if !wallet.initialized || wallet.locked {
        return Err("Wallet locked or not initialized".into());
    }

    let private_key = wallet.private_key.clone()
        .ok_or("Private key not available")?;

    drop(wallet);

    let unlock_data = format!("0xa69df4b5{:0>64}", format!("{:x}", lock_id));
    let unlock_data_bytes = hex::decode(unlock_data.trim_start_matches("0x"))
        .map_err(|e| format!("Encode error: {}", e))?;

    let unlock_tx = send_transaction_with_gas(&private_key, LP_STAKING_ADDRESS, 0, unlock_data_bytes, 150000).await?;

    Ok(format!("Unlocked position #{}! Tx: {}", lock_id, unlock_tx))
}

#[tauri::command]
pub async fn lp_early_unlock(
    state: State<'_, AppState>,
    lock_id: u64,
) -> Result<String, String> {
    if LP_STAKING_ADDRESS == "0x0000000000000000000000000000000000000000" {
        return Err("LP Staking contract not yet deployed. Coming soon!".into());
    }

    let wallet = state.wallet.read().await;

    if !wallet.initialized || wallet.locked {
        return Err("Wallet locked or not initialized".into());
    }

    let private_key = wallet.private_key.clone()
        .ok_or("Private key not available")?;

    drop(wallet);

    let unlock_data = format!("0x7b4d0e4c{:0>64}", format!("{:x}", lock_id));
    let unlock_data_bytes = hex::decode(unlock_data.trim_start_matches("0x"))
        .map_err(|e| format!("Encode error: {}", e))?;

    let unlock_tx = send_transaction_with_gas(&private_key, LP_STAKING_ADDRESS, 0, unlock_data_bytes, 150000).await?;

    Ok(format!("Early unlock for position #{}! (Penalty applied) Tx: {}", lock_id, unlock_tx))
}

#[tauri::command]
pub async fn lp_extend_lock(
    state: State<'_, AppState>,
    lock_id: u64,
    new_tier: u8,
) -> Result<String, String> {
    if LP_STAKING_ADDRESS == "0x0000000000000000000000000000000000000000" {
        return Err("LP Staking contract not yet deployed. Coming soon!".into());
    }

    if new_tier > 4 {
        return Err("Invalid tier. Must be 0-4.".into());
    }

    let wallet = state.wallet.read().await;

    if !wallet.initialized || wallet.locked {
        return Err("Wallet locked or not initialized".into());
    }

    let private_key = wallet.private_key.clone()
        .ok_or("Private key not available")?;

    drop(wallet);

    let extend_data = format!(
        "0x9e8c708e{:0>64}{:0>64}",
        format!("{:x}", lock_id),
        format!("{:x}", new_tier)
    );
    let extend_data_bytes = hex::decode(extend_data.trim_start_matches("0x"))
        .map_err(|e| format!("Encode error: {}", e))?;

    let extend_tx = send_transaction_with_gas(&private_key, LP_STAKING_ADDRESS, 0, extend_data_bytes, 150000).await?;

    let tier_info = get_lock_tiers().get(new_tier as usize).map(|t| t.duration_days).unwrap_or(0);
    Ok(format!("Extended lock #{} to {} day tier! Tx: {}", lock_id, tier_info, extend_tx))
}

#[tauri::command]
pub async fn lp_claim_rewards(
    state: State<'_, AppState>,
    lock_id: u64,
) -> Result<String, String> {
    if LP_STAKING_ADDRESS == "0x0000000000000000000000000000000000000000" {
        return Err("LP Staking contract not yet deployed. Coming soon!".into());
    }

    let wallet = state.wallet.read().await;

    if !wallet.initialized || wallet.locked {
        return Err("Wallet locked or not initialized".into());
    }

    let private_key = wallet.private_key.clone()
        .ok_or("Private key not available")?;

    drop(wallet);

    let claim_data = format!("0x0fae75d9{:0>64}", format!("{:x}", lock_id));
    let claim_data_bytes = hex::decode(claim_data.trim_start_matches("0x"))
        .map_err(|e| format!("Encode error: {}", e))?;

    let claim_tx = send_transaction_with_gas(&private_key, LP_STAKING_ADDRESS, 0, claim_data_bytes, 150000).await?;

    Ok(format!("Claimed rewards for lock #{}! Tx: {}", lock_id, claim_tx))
}

#[tauri::command]
pub async fn lp_claim_all_rewards(
    state: State<'_, AppState>,
) -> Result<String, String> {
    if LP_STAKING_ADDRESS == "0x0000000000000000000000000000000000000000" {
        return Err("LP Staking contract not yet deployed. Coming soon!".into());
    }

    let wallet = state.wallet.read().await;

    if !wallet.initialized || wallet.locked {
        return Err("Wallet locked or not initialized".into());
    }

    let private_key = wallet.private_key.clone()
        .ok_or("Private key not available")?;

    drop(wallet);

    let claim_data = "0x4e71d92d";
    let claim_data_bytes = hex::decode(claim_data.trim_start_matches("0x"))
        .map_err(|e| format!("Encode error: {}", e))?;

    let claim_tx = send_transaction_with_gas(&private_key, LP_STAKING_ADDRESS, 0, claim_data_bytes, 300000).await?;

    Ok(format!("Claimed all rewards! Tx: {}", claim_tx))
}

#[tauri::command]
pub async fn lp_compound_rewards(
    state: State<'_, AppState>,
    lock_id: u64,
) -> Result<String, String> {
    if LP_STAKING_ADDRESS == "0x0000000000000000000000000000000000000000" {
        return Err("LP Staking contract not yet deployed. Coming soon!".into());
    }

    let wallet = state.wallet.read().await;

    if !wallet.initialized || wallet.locked {
        return Err("Wallet locked or not initialized".into());
    }

    let private_key = wallet.private_key.clone()
        .ok_or("Private key not available")?;

    drop(wallet);

    let compound_data = format!("0xf69e2046{:0>64}", format!("{:x}", lock_id));
    let compound_data_bytes = hex::decode(compound_data.trim_start_matches("0x"))
        .map_err(|e| format!("Encode error: {}", e))?;

    let compound_tx = send_transaction_with_gas(&private_key, LP_STAKING_ADDRESS, 0, compound_data_bytes, 200000).await?;

    Ok(format!("Compounded rewards for lock #{}! Tx: {}", lock_id, compound_tx))
}

#[tauri::command]
pub fn lp_get_tiers() -> Vec<LockTier> {
    get_lock_tiers()
}
