use crate::blockchain::{
    fetch_real_balances, get_gas_price, send_transaction, send_transaction_with_gas,
    NOX_TOKEN_ADDRESS,
};
use crate::helpers::format_wei;
use crate::state::AppState;
use crate::types::WalletStatusResponse;
use super::state::WALLET_MANAGER;
use nonos_wallet::{Wallet, WalletStorage};
use tauri::State;

#[tauri::command]
pub async fn wallet_get_status(state: State<'_, AppState>) -> Result<WalletStatusResponse, String> {
    let mut manager = WALLET_MANAGER.write().await;

    let (initialized, locked, address) = if let Some(wallet) = manager.active() {
        (true, !wallet.is_unlocked(), Some(wallet.address().to_hex()))
    } else {
        let has_wallets = manager.has_wallets();
        (has_wallets, true, None)
    };

    drop(manager);

    let (eth_balance, nox_balance) = if let Some(ref addr) = address {
        fetch_real_balances(addr).await
    } else {
        (0, 0)
    };

    {
        let mut app_wallet = state.wallet.write().await;
        app_wallet.eth_balance = eth_balance;
        app_wallet.nox_balance = nox_balance;
        app_wallet.initialized = initialized;
        app_wallet.locked = locked;
        app_wallet.address = address.clone();
        app_wallet.last_refresh = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    let app_wallet = state.wallet.read().await;
    Ok(WalletStatusResponse {
        initialized,
        locked,
        address,
        nox_balance: format_wei(nox_balance),
        eth_balance: format_wei(eth_balance),
        pending_rewards: format_wei(app_wallet.pending_rewards),
    })
}

#[tauri::command]
pub async fn wallet_create(
    state: State<'_, AppState>,
    password: String,
) -> Result<String, String> {
    if password.len() < 8 {
        return Err("Password must be at least 8 characters".into());
    }

    let (wallet, mnemonic, _blake3_key) = Wallet::create("Default Wallet".to_string())
        .map_err(|e| format!("Failed to create wallet: {}", e))?;

    let address = wallet.address().to_hex();

    let master_key = nonos_crypto::derive_blake3_key_from_mnemonic(&mnemonic)
        .map_err(|e| format!("Key derivation failed: {}", e))?;

    {
        let mut manager = WALLET_MANAGER.write().await;
        let storage = manager.storage()?;
        storage.save_wallet(wallet.metadata(), &master_key.0, &password)
            .map_err(|e| format!("Failed to save wallet: {}", e))?;
        manager.set_active(wallet);
    }

    {
        let mut app_wallet = state.wallet.write().await;
        app_wallet.initialized = true;
        app_wallet.locked = false;
        app_wallet.address = Some(address);
        app_wallet.mnemonic = Some(mnemonic.clone());
    }

    Ok(mnemonic)
}

#[tauri::command]
pub async fn wallet_import(
    state: State<'_, AppState>,
    mnemonic: String,
    password: String,
) -> Result<(), String> {
    if password.len() < 8 {
        return Err("Password must be at least 8 characters".into());
    }

    let word_count = mnemonic.split_whitespace().count();
    if word_count != 12 && word_count != 24 {
        return Err("Invalid mnemonic: must be 12 or 24 words".into());
    }

    let wallet = Wallet::import_from_mnemonic("Imported Wallet".to_string(), &mnemonic)
        .map_err(|e| format!("Failed to import wallet: {}", e))?;

    let address = wallet.address().to_hex();

    let master_key = nonos_crypto::derive_blake3_key_from_mnemonic(&mnemonic)
        .map_err(|e| format!("Key derivation failed: {}", e))?;

    {
        let mut manager = WALLET_MANAGER.write().await;
        let storage = manager.storage()?;
        storage.save_wallet(wallet.metadata(), &master_key.0, &password)
            .map_err(|e| format!("Failed to save wallet: {}", e))?;
        manager.set_active(wallet);
    }

    {
        let mut app_wallet = state.wallet.write().await;
        app_wallet.initialized = true;
        app_wallet.locked = false;
        app_wallet.address = Some(address);
        app_wallet.mnemonic = Some(mnemonic);
    }

    Ok(())
}

#[tauri::command]
pub async fn wallet_unlock(state: State<'_, AppState>, password: String) -> Result<(), String> {
    let mut manager = WALLET_MANAGER.write().await;
    let storage = manager.storage()?;

    let wallets = storage.list_wallets()
        .map_err(|e| format!("Failed to list wallets: {}", e))?;

    if wallets.is_empty() {
        return Err("No wallet found".into());
    }

    let wallet_id = &wallets[0];
    let metadata = storage.load_metadata(wallet_id)
        .map_err(|e| format!("Failed to load wallet: {}", e))?;

    let master_key = storage.load_secrets(wallet_id, &password)
        .map_err(|_| "Wrong password".to_string())?;

    let key_hex = hex::encode(master_key);
    let wallet = Wallet::import_from_blake3_key(metadata.name.clone(), &key_hex)
        .map_err(|e| format!("Failed to unlock wallet: {}", e))?;

    let address = wallet.address().to_hex();
    manager.set_active(wallet);
    drop(manager);

    {
        let mut app_wallet = state.wallet.write().await;
        app_wallet.initialized = true;
        app_wallet.locked = false;
        app_wallet.address = Some(address);
    }

    Ok(())
}

#[tauri::command]
pub async fn wallet_lock(state: State<'_, AppState>) -> Result<(), String> {
    {
        let mut manager = WALLET_MANAGER.write().await;
        manager.clear_active();
    }

    {
        let mut app_wallet = state.wallet.write().await;
        app_wallet.locked = true;
        app_wallet.private_key = None;
        app_wallet.mnemonic = None;
    }

    Ok(())
}

#[tauri::command]
pub async fn wallet_get_address(_state: State<'_, AppState>) -> Result<Option<String>, String> {
    let manager = WALLET_MANAGER.read().await;
    Ok(manager.active().map(|w| w.address().to_hex()))
}

#[tauri::command]
pub async fn wallet_send_eth(
    _state: State<'_, AppState>,
    to: String,
    amount: String,
) -> Result<String, String> {
    let manager = WALLET_MANAGER.read().await;
    let wallet = manager.active().ok_or("Wallet not unlocked")?;

    if !wallet.is_unlocked() {
        return Err("Wallet is locked".into());
    }

    let private_key = wallet.get_account_private_key(0)
        .map_err(|e| format!("Failed to get signing key: {}", e))?;

    let from_address = wallet.address().to_hex();
    drop(manager);

    let amount_eth: f64 = amount.parse().map_err(|_| "Invalid amount")?;
    let amount_wei = (amount_eth * 1e18) as u128;

    let (eth_balance, _) = fetch_real_balances(&from_address).await;
    if amount_wei > eth_balance {
        return Err(format!(
            "Insufficient ETH balance. Have {} ETH, sending {} ETH",
            eth_balance as f64 / 1e18,
            amount_eth
        ));
    }

    let gas_price = get_gas_price().await?;
    let gas_limit = 21000u64;
    let gas_cost = gas_price * gas_limit as u128;

    if amount_wei + gas_cost > eth_balance {
        return Err(format!(
            "Insufficient ETH for amount + gas. Have {} ETH, need {} ETH",
            eth_balance as f64 / 1e18,
            (amount_wei + gas_cost) as f64 / 1e18
        ));
    }

    send_transaction(&private_key, &to, amount_wei, vec![]).await
}

#[tauri::command]
pub async fn wallet_send_nox(
    _state: State<'_, AppState>,
    to: String,
    amount: String,
) -> Result<String, String> {
    let manager = WALLET_MANAGER.read().await;
    let wallet = manager.active().ok_or("Wallet not unlocked")?;

    if !wallet.is_unlocked() {
        return Err("Wallet is locked".into());
    }

    let private_key = wallet.get_account_private_key(0)
        .map_err(|e| format!("Failed to get signing key: {}", e))?;

    let from_address = wallet.address().to_hex();
    drop(manager);

    let amount_nox: f64 = amount.parse().map_err(|_| "Invalid amount")?;
    let amount_wei = (amount_nox * 1e18) as u128;

    let (eth_balance, nox_balance) = fetch_real_balances(&from_address).await;

    if amount_wei > nox_balance {
        return Err(format!(
            "Insufficient NOX balance. Have {} NOX, sending {} NOX",
            nox_balance as f64 / 1e18,
            amount_nox
        ));
    }

    let min_gas_eth = 3_000_000_000_000_000u128;
    if eth_balance < min_gas_eth {
        return Err(format!(
            "Insufficient ETH for gas. Have {} ETH, need at least 0.003 ETH",
            eth_balance as f64 / 1e18
        ));
    }

    let to_addr = to.trim_start_matches("0x").to_lowercase();
    let padded_to = format!("{:0>64}", to_addr);
    let padded_amount = format!("{:0>64}", format!("{:x}", amount_wei));
    let data = hex::decode(format!("a9059cbb{}{}", padded_to, padded_amount))
        .map_err(|_| "Failed to encode transfer data")?;

    send_transaction_with_gas(&private_key, NOX_TOKEN_ADDRESS, 0, data, 100000).await
}

#[tauri::command]
pub async fn wallet_get_transactions(
    _state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let manager = WALLET_MANAGER.read().await;

    let wallet = manager.active().ok_or("Wallet not initialized")?;

    let txs: Vec<serde_json::Value> = wallet
        .transactions()
        .iter()
        .map(|tx| {
            serde_json::json!({
                "hash": tx.hash.to_hex(),
                "status": format!("{:?}", tx.status),
            })
        })
        .collect();

    Ok(txs)
}

#[tauri::command]
pub async fn wallet_check_exists() -> Result<bool, String> {
    let mut manager = WALLET_MANAGER.write().await;
    Ok(manager.has_wallets())
}

#[tauri::command]
pub async fn wallet_get_stealth_address() -> Result<String, String> {
    let manager = WALLET_MANAGER.read().await;
    let wallet = manager.active().ok_or("Wallet not unlocked")?;

    wallet
        .generate_receive_stealth_address()
        .map_err(|e| format!("Failed to generate stealth address: {}", e))
}

#[tauri::command]
pub async fn wallet_change_password(
    old_password: String,
    new_password: String,
) -> Result<(), String> {
    if new_password.len() < 8 {
        return Err("New password must be at least 8 characters".into());
    }

    let mut manager = WALLET_MANAGER.write().await;
    let storage = manager.storage()?;

    let wallets = storage.list_wallets()
        .map_err(|e| format!("Failed to list wallets: {}", e))?;

    if wallets.is_empty() {
        return Err("No wallet found".into());
    }

    let wallet_id = &wallets[0];
    storage.change_password(wallet_id, &old_password, &new_password)
        .map_err(|_| "Wrong password".to_string())?;

    Ok(())
}
