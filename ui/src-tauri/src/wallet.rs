use crate::blockchain::{fetch_real_balances, send_transaction, send_transaction_with_gas, estimate_gas, NOX_TOKEN_ADDRESS};
use crate::helpers::{format_wei, generate_mnemonic, derive_keys_from_mnemonic};
use crate::state::AppState;
use crate::types::WalletStatusResponse;
use tauri::State;

#[tauri::command]
pub async fn wallet_get_status(state: State<'_, AppState>) -> Result<WalletStatusResponse, String> {
    let address = {
        let wallet = state.wallet.read().await;
        wallet.address.clone()
    };

    let (eth_balance, nox_balance) = if let Some(ref addr) = address {
        fetch_real_balances(addr).await
    } else {
        (0, 0)
    };

    {
        let mut wallet = state.wallet.write().await;
        wallet.eth_balance = eth_balance;
        wallet.nox_balance = nox_balance;
        wallet.last_refresh = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    let wallet = state.wallet.read().await;
    Ok(WalletStatusResponse {
        initialized: wallet.initialized,
        locked: wallet.locked,
        address: wallet.address.clone(),
        nox_balance: format_wei(nox_balance),
        eth_balance: format_wei(eth_balance),
        pending_rewards: format_wei(wallet.pending_rewards),
    })
}

#[tauri::command]
pub async fn wallet_create(
    state: State<'_, AppState>,
    _password: String,
) -> Result<String, String> {
    let mnemonic = generate_mnemonic();
    let (address, private_key) = derive_keys_from_mnemonic(&mnemonic);

    let mut wallet = state.wallet.write().await;
    wallet.initialized = true;
    wallet.locked = false;
    wallet.address = Some(address);
    wallet.private_key = Some(private_key);
    wallet.mnemonic = Some(mnemonic.clone());

    Ok(mnemonic)
}

#[tauri::command]
pub async fn wallet_import(
    state: State<'_, AppState>,
    mnemonic: String,
    _password: String,
) -> Result<(), String> {
    let word_count = mnemonic.split_whitespace().count();
    if word_count != 12 && word_count != 24 {
        return Err("Invalid mnemonic: must be 12 or 24 words".into());
    }

    let (address, private_key) = derive_keys_from_mnemonic(&mnemonic);

    let mut wallet = state.wallet.write().await;
    wallet.initialized = true;
    wallet.locked = false;
    wallet.address = Some(address);
    wallet.private_key = Some(private_key);
    wallet.mnemonic = Some(mnemonic);

    Ok(())
}

#[tauri::command]
pub async fn wallet_unlock(
    state: State<'_, AppState>,
    _password: String,
) -> Result<(), String> {
    let mut wallet = state.wallet.write().await;

    if !wallet.initialized {
        return Err("Wallet not initialized".into());
    }

    wallet.locked = false;
    Ok(())
}

#[tauri::command]
pub async fn wallet_lock(state: State<'_, AppState>) -> Result<(), String> {
    let mut wallet = state.wallet.write().await;
    wallet.locked = true;
    Ok(())
}

#[tauri::command]
pub async fn wallet_get_address(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let wallet = state.wallet.read().await;
    Ok(wallet.address.clone())
}

#[tauri::command]
pub async fn wallet_send_eth(
    state: State<'_, AppState>,
    to: String,
    amount: String,
) -> Result<String, String> {
    let wallet = state.wallet.read().await;

    if !wallet.initialized || wallet.locked {
        return Err("Wallet locked or not initialized".into());
    }

    let private_key = wallet.private_key.clone()
        .ok_or("Private key not available")?;

    let amount_eth: f64 = amount.parse()
        .map_err(|_| "Invalid amount")?;
    let amount_wei = (amount_eth * 1e18) as u128;

    if amount_wei > wallet.eth_balance {
        return Err("Insufficient ETH balance".into());
    }

    drop(wallet);

    let tx_hash = send_transaction(&private_key, &to, amount_wei, vec![]).await?;

    Ok(format!("Sent {} ETH! Tx: {}", amount_eth, tx_hash))
}

#[tauri::command]
pub async fn wallet_send_nox(
    state: State<'_, AppState>,
    to: String,
    amount: String,
) -> Result<String, String> {
    let (private_key, from_address) = {
        let wallet = state.wallet.read().await;

        if !wallet.initialized || wallet.locked {
            return Err("Wallet locked or not initialized".into());
        }

        let pk = wallet.private_key.clone().ok_or("Private key not available")?;
        let addr = wallet.address.clone().ok_or("Wallet address not available")?;
        (pk, addr)
    };

    let amount_nox: f64 = amount.parse()
        .map_err(|_| "Invalid amount")?;
    let amount_wei = (amount_nox * 1e18) as u128;

    let (eth_balance, nox_balance) = fetch_real_balances(&from_address).await;

    if amount_wei > nox_balance {
        return Err(format!("Insufficient NOX balance. You have {} NOX, trying to send {} NOX",
            nox_balance as f64 / 1e18, amount_nox));
    }

    let min_gas_eth = 3_000_000_000_000_000u128;
    if eth_balance < min_gas_eth {
        return Err(format!("Insufficient ETH for gas. You have {} ETH, need at least 0.003 ETH",
            eth_balance as f64 / 1e18));
    }

    let to_padded = format!("{:0>64}", to.trim_start_matches("0x").to_lowercase());
    let amount_hex = format!("{:0>64x}", amount_wei);
    let transfer_data = format!("0xa9059cbb{}{}", to_padded, amount_hex);

    let estimated_gas = match estimate_gas(&from_address, NOX_TOKEN_ADDRESS, &transfer_data).await {
        Ok(gas) => gas,
        Err(e) => return Err(format!("Transfer would fail: {}", e)),
    };

    let transfer_data_bytes = hex::decode(transfer_data.trim_start_matches("0x"))
        .map_err(|e| format!("Encode error: {}", e))?;

    let tx_hash = send_transaction_with_gas(&private_key, NOX_TOKEN_ADDRESS, 0, transfer_data_bytes, estimated_gas).await?;

    Ok(format!("Sent {} NOX! Tx: {}", amount_nox, tx_hash))
}

#[tauri::command]
pub async fn wallet_get_transactions(
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let wallet = state.wallet.read().await;

    if !wallet.initialized {
        return Err("Wallet not initialized".into());
    }

    Ok(vec![])
}
