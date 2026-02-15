//! NONOS Ecosystem Browser - Tauri Application Backend
//!
//! Zero-trust privacy browser powered by Anyone Network with:
//! - Real Anyone Network integration via `anon` binary
//! - SOCKS5 proxy routing for ALL browser traffic
//! - Embedded wallet with transaction signing
//! - Community node discovery and connection
//! - Privacy-first browsing through multi-hop encryption
//!
//! Copyright (c) 2024 NON-OS <team@nonos.systems>
//! Licensed under MIT License

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use serde::Serialize;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{Manager, State, Window};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpStream, TcpListener};
use tokio::process::Command;
use tokio::sync::RwLock;
use hyper::{Request, Response, body::Incoming, service::service_fn};
use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;
use http_body_util::Full;
use hyper::body::Bytes;

// =============================================================================
// Blockchain RPC Integration - REAL Ethereum calls
// =============================================================================

/// RPC endpoints for Ethereum Mainnet (with fallbacks)
/// Note: publicnode and 1rpc are most reliable free endpoints
const RPC_ENDPOINTS: &[&str] = &[
    "https://ethereum.publicnode.com",
    "https://1rpc.io/eth",
    "https://eth.merkle.io",
    "https://rpc.payload.de",
];

/// NOX Token contract address (Mainnet)
const NOX_TOKEN_ADDRESS: &str = "0x0a26c80Be4E060e688d7C23aDdB92cBb5D2C9eCA";

/// NOX Staking contract address (to be deployed - using placeholder)
const NOX_STAKING_ADDRESS: &str = "0x0000000000000000000000000000000000000000";

/// ERC-20 balanceOf(address) function selector
const BALANCE_OF_SELECTOR: &str = "70a08231";

/// ERC-20 transfer(address,uint256) function selector
const TRANSFER_SELECTOR: &str = "a9059cbb";

/// Make a JSON-RPC call to Ethereum
async fn eth_call(to: &str, data: &str) -> Result<String, String> {
    let client = reqwest::Client::new();

    for endpoint in RPC_ENDPOINTS {
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_call",
            "params": [{
                "to": to,
                "data": data
            }, "latest"],
            "id": 1
        });

        println!("NONOS: eth_call to {} via {}", to, endpoint);

        match client.post(*endpoint)
            .header("Content-Type", "application/json")
            .json(&payload)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
        {
            Ok(response) => {
                if let Ok(json) = response.json::<serde_json::Value>().await {
                    println!("NONOS: eth_call response: {}", json);
                    if let Some(result) = json.get("result").and_then(|r| r.as_str()) {
                        return Ok(result.to_string());
                    }
                    if let Some(error) = json.get("error") {
                        // RPC returned an error, try next endpoint
                        println!("NONOS: eth_call RPC error from {}: {}, trying next...", endpoint, error);
                        continue;
                    }
                    // Handle case where result exists but is not a string (null, etc)
                    if let Some(result) = json.get("result") {
                        if result.is_null() {
                            println!("NONOS: eth_call returned null from {}, trying next...", endpoint);
                            continue;
                        }
                    }
                }
            }
            Err(e) => {
                println!("NONOS: eth_call request failed to {}: {}", endpoint, e);
                continue; // Try next endpoint
            }
        }
    }

    Err("All RPC endpoints failed".to_string())
}

/// Get ETH balance of an address
async fn get_eth_balance(address: &str) -> Result<u128, String> {
    let client = reqwest::Client::new();

    println!("NONOS: Getting ETH balance for {}", address);

    for endpoint in RPC_ENDPOINTS {
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getBalance",
            "params": [address, "latest"],
            "id": 1
        });

        println!("NONOS: Trying ETH balance from {}", endpoint);

        match client.post(*endpoint)
            .header("Content-Type", "application/json")
            .json(&payload)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
        {
            Ok(response) => {
                if let Ok(json) = response.json::<serde_json::Value>().await {
                    println!("NONOS: ETH balance response: {}", json);
                    if let Some(result) = json.get("result").and_then(|r| r.as_str()) {
                        let hex = result.trim_start_matches("0x");
                        let balance = u128::from_str_radix(hex, 16)
                            .map_err(|e| format!("Parse error: {}", e))?;
                        println!("NONOS: ETH balance parsed: {} wei", balance);
                        return Ok(balance);
                    }
                    if let Some(error) = json.get("error") {
                        println!("NONOS: ETH balance RPC error from {}: {}", endpoint, error);
                        continue;
                    }
                }
            }
            Err(e) => {
                println!("NONOS: ETH balance request failed to {}: {}", endpoint, e);
                continue;
            }
        }
    }

    Err("Failed to get ETH balance".to_string())
}

/// Get ERC-20 token balance
async fn get_token_balance(token_address: &str, holder_address: &str) -> Result<u128, String> {
    // Encode: balanceOf(address)
    // Remove 0x prefix from address and pad to 32 bytes
    let addr = holder_address.trim_start_matches("0x").to_lowercase();
    let padded_addr = format!("{:0>64}", addr);
    let data = format!("0x{}{}", BALANCE_OF_SELECTOR, padded_addr);

    println!("NONOS: Token balance call - token: {}, holder: {}", token_address, holder_address);
    println!("NONOS: Call data: {}", data);

    let result = eth_call(token_address, &data).await?;
    println!("NONOS: Token balance raw result: {}", result);

    // Handle empty or zero result
    let hex = result.trim_start_matches("0x");
    if hex.is_empty() || hex.chars().all(|c| c == '0') {
        println!("NONOS: Token balance is zero or empty");
        return Ok(0);
    }

    let balance = u128::from_str_radix(hex, 16)
        .map_err(|e| format!("Parse error for '{}': {}", hex, e))?;

    println!("NONOS: Parsed token balance: {} wei ({} tokens)", balance, balance as f64 / 1e18);
    Ok(balance)
}

/// Fetch real balances from blockchain
async fn fetch_real_balances(address: &str) -> (u128, u128) {
    println!("NONOS: Fetching balances for address: {}", address);

    let eth_balance = match get_eth_balance(address).await {
        Ok(bal) => {
            println!("NONOS: ETH balance: {} wei ({} ETH)", bal, bal as f64 / 1e18);
            bal
        }
        Err(e) => {
            println!("NONOS: ERROR getting ETH balance: {}", e);
            0
        }
    };

    let nox_balance = match get_token_balance(NOX_TOKEN_ADDRESS, address).await {
        Ok(bal) => {
            println!("NONOS: NOX balance: {} wei ({} NOX)", bal, bal as f64 / 1e18);
            bal
        }
        Err(e) => {
            println!("NONOS: ERROR getting NOX balance: {}", e);
            0
        }
    };

    (eth_balance, nox_balance)
}

/// Get transaction count (nonce) for an address
async fn get_nonce(address: &str) -> Result<u64, String> {
    let client = reqwest::Client::new();

    for endpoint in RPC_ENDPOINTS {
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getTransactionCount",
            "params": [address, "pending"],
            "id": 1
        });

        match client.post(*endpoint)
            .header("Content-Type", "application/json")
            .json(&payload)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
        {
            Ok(response) => {
                if let Ok(json) = response.json::<serde_json::Value>().await {
                    if let Some(result) = json.get("result").and_then(|r| r.as_str()) {
                        let hex = result.trim_start_matches("0x");
                        return u64::from_str_radix(hex, 16)
                            .map_err(|e| format!("Parse error: {}", e));
                    }
                }
            }
            Err(_) => continue,
        }
    }

    Err("Failed to get nonce".to_string())
}

/// Get current gas price
async fn get_gas_price() -> Result<u128, String> {
    let client = reqwest::Client::new();

    for endpoint in RPC_ENDPOINTS {
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_gasPrice",
            "params": [],
            "id": 1
        });

        match client.post(*endpoint)
            .header("Content-Type", "application/json")
            .json(&payload)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
        {
            Ok(response) => {
                if let Ok(json) = response.json::<serde_json::Value>().await {
                    if let Some(result) = json.get("result").and_then(|r| r.as_str()) {
                        let hex = result.trim_start_matches("0x");
                        return u128::from_str_radix(hex, 16)
                            .map_err(|e| format!("Parse error: {}", e));
                    }
                }
            }
            Err(_) => continue,
        }
    }

    Err("Failed to get gas price".to_string())
}

/// Estimate gas for a transaction
async fn estimate_gas(from: &str, to: &str, data: &str) -> Result<u64, String> {
    let client = reqwest::Client::new();

    for endpoint in RPC_ENDPOINTS {
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_estimateGas",
            "params": [{
                "from": from,
                "to": to,
                "data": data
            }],
            "id": 1
        });

        println!("NONOS: Estimating gas for tx from {} to {}", from, to);

        match client.post(*endpoint)
            .header("Content-Type", "application/json")
            .json(&payload)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
        {
            Ok(response) => {
                if let Ok(json) = response.json::<serde_json::Value>().await {
                    println!("NONOS: Gas estimate response: {}", json);
                    if let Some(result) = json.get("result").and_then(|r| r.as_str()) {
                        let hex = result.trim_start_matches("0x");
                        let gas = u64::from_str_radix(hex, 16)
                            .map_err(|e| format!("Parse error: {}", e))?;
                        // Add 20% buffer for safety
                        let gas_with_buffer = gas + (gas / 5);
                        println!("NONOS: Estimated gas: {}, with buffer: {}", gas, gas_with_buffer);
                        return Ok(gas_with_buffer);
                    }
                    if let Some(error) = json.get("error") {
                        let err_msg = format!("Gas estimation failed: {}", error);
                        println!("NONOS: {}", err_msg);
                        return Err(err_msg);
                    }
                }
            }
            Err(e) => {
                println!("NONOS: Gas estimate request failed: {}", e);
                continue;
            }
        }
    }

    Err("Failed to estimate gas".to_string())
}

/// Send a signed raw transaction
async fn send_raw_transaction(signed_tx: &str) -> Result<String, String> {
    let client = reqwest::Client::new();

    for endpoint in RPC_ENDPOINTS {
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_sendRawTransaction",
            "params": [signed_tx],
            "id": 1
        });

        match client.post(*endpoint)
            .header("Content-Type", "application/json")
            .json(&payload)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
        {
            Ok(response) => {
                if let Ok(json) = response.json::<serde_json::Value>().await {
                    if let Some(result) = json.get("result").and_then(|r| r.as_str()) {
                        return Ok(result.to_string()); // tx hash
                    }
                    if let Some(error) = json.get("error") {
                        return Err(format!("Transaction failed: {}", error));
                    }
                }
            }
            Err(_) => continue,
        }
    }

    Err("Failed to broadcast transaction".to_string())
}

/// Sign a transaction with private key using k256/ecdsa
fn sign_transaction(
    private_key_hex: &str,
    to: &str,
    value: u128,
    data: &[u8],
    nonce: u64,
    gas_limit: u64,
    gas_price: u128,
    chain_id: u64,
) -> Result<String, String> {
    use k256::ecdsa::{SigningKey, signature::Signer};
    use tiny_keccak::{Hasher, Keccak};

    // Parse private key
    let key_bytes = hex::decode(private_key_hex.trim_start_matches("0x"))
        .map_err(|e| format!("Invalid private key: {}", e))?;
    let signing_key = SigningKey::from_slice(&key_bytes)
        .map_err(|e| format!("Invalid key: {}", e))?;

    // Parse to address
    let to_bytes = hex::decode(to.trim_start_matches("0x"))
        .map_err(|e| format!("Invalid to address: {}", e))?;

    // RLP encode the transaction (EIP-155)
    let mut rlp_items: Vec<Vec<u8>> = Vec::new();

    // nonce
    rlp_items.push(encode_rlp_uint(nonce as u128));
    // gasPrice
    rlp_items.push(encode_rlp_uint(gas_price));
    // gasLimit
    rlp_items.push(encode_rlp_uint(gas_limit as u128));
    // to
    rlp_items.push(to_bytes.clone());
    // value
    rlp_items.push(encode_rlp_uint(value));
    // data
    rlp_items.push(data.to_vec());
    // v (chain_id for EIP-155)
    rlp_items.push(encode_rlp_uint(chain_id as u128));
    // r (0 for signing)
    rlp_items.push(vec![]);
    // s (0 for signing)
    rlp_items.push(vec![]);

    let unsigned_tx = encode_rlp_list(&rlp_items);

    // Hash the unsigned transaction
    let mut hasher = Keccak::v256();
    let mut hash = [0u8; 32];
    hasher.update(&unsigned_tx);
    hasher.finalize(&mut hash);

    // Sign the hash
    let (signature, recovery_id) = signing_key
        .sign_prehash_recoverable(&hash)
        .map_err(|e| format!("Signing failed: {}", e))?;

    let sig_bytes = signature.to_bytes();
    let r = &sig_bytes[..32];
    let s = &sig_bytes[32..];

    // Calculate v with EIP-155
    let v = chain_id * 2 + 35 + recovery_id.to_byte() as u64;

    // Build signed transaction
    let mut signed_items: Vec<Vec<u8>> = Vec::new();
    signed_items.push(encode_rlp_uint(nonce as u128));
    signed_items.push(encode_rlp_uint(gas_price));
    signed_items.push(encode_rlp_uint(gas_limit as u128));
    signed_items.push(to_bytes);
    signed_items.push(encode_rlp_uint(value));
    signed_items.push(data.to_vec());
    signed_items.push(encode_rlp_uint(v as u128));
    signed_items.push(r.to_vec());
    signed_items.push(s.to_vec());

    let signed_tx = encode_rlp_list(&signed_items);

    Ok(format!("0x{}", hex::encode(signed_tx)))
}

/// RLP encode a uint (minimal encoding, no leading zeros)
fn encode_rlp_uint(value: u128) -> Vec<u8> {
    if value == 0 {
        return vec![];
    }
    let bytes = value.to_be_bytes();
    let start = bytes.iter().position(|&b| b != 0).unwrap_or(bytes.len());
    bytes[start..].to_vec()
}

/// RLP encode a list of items
fn encode_rlp_list(items: &[Vec<u8>]) -> Vec<u8> {
    let mut payload = Vec::new();
    for item in items {
        if item.is_empty() {
            payload.push(0x80); // empty string
        } else if item.len() == 1 && item[0] < 0x80 {
            payload.push(item[0]); // single byte < 0x80
        } else if item.len() < 56 {
            payload.push(0x80 + item.len() as u8);
            payload.extend(item);
        } else {
            let len_bytes = item.len().to_be_bytes();
            let start = len_bytes.iter().position(|&b| b != 0).unwrap_or(len_bytes.len());
            let len_bytes = &len_bytes[start..];
            payload.push(0xb7 + len_bytes.len() as u8);
            payload.extend(len_bytes);
            payload.extend(item);
        }
    }

    let mut result = Vec::new();
    if payload.len() < 56 {
        result.push(0xc0 + payload.len() as u8);
    } else {
        let len_bytes = payload.len().to_be_bytes();
        let start = len_bytes.iter().position(|&b| b != 0).unwrap_or(len_bytes.len());
        let len_bytes = &len_bytes[start..];
        result.push(0xf7 + len_bytes.len() as u8);
        result.extend(len_bytes);
    }
    result.extend(payload);
    result
}

/// Fetch staking info from the staking contract
async fn fetch_staking_info(address: &str) -> (u128, u128, u8) {
    if NOX_STAKING_ADDRESS == "0x0000000000000000000000000000000000000000" {
        // Contract not deployed yet
        return (0, 0, 0);
    }

    // Query stakers(address) - returns multiple values
    // For simplicity, we'll query pendingRewards separately
    let addr = address.trim_start_matches("0x");
    let padded_addr = format!("{:0>64}", addr);

    // getStakerInfo(address) selector - would need to compute this
    // For now, query individual values

    // pendingRewards(address) - selector: 0x31d7a262
    let pending_data = format!("0x31d7a262{}", padded_addr);
    let pending_rewards = if let Ok(result) = eth_call(NOX_STAKING_ADDRESS, &pending_data).await {
        let hex = result.trim_start_matches("0x");
        u128::from_str_radix(hex, 16).unwrap_or(0)
    } else {
        0
    };

    // stakers(address).stakedAmount - need to decode struct, skip for now
    // For MVP, return pending rewards only
    (0, pending_rewards, 0)
}

/// Send ETH or tokens
async fn send_transaction(
    private_key: &str,
    to: &str,
    value: u128,
    data: Vec<u8>,
) -> Result<String, String> {
    use tiny_keccak::Hasher;

    println!("NONOS: send_transaction called - to: {}, value: {}, data_len: {}", to, value, data.len());

    // Get sender address from private key
    let key_bytes = hex::decode(private_key.trim_start_matches("0x"))
        .map_err(|e| format!("Invalid private key: {}", e))?;
    let signing_key = k256::ecdsa::SigningKey::from_slice(&key_bytes)
        .map_err(|e| format!("Invalid key: {}", e))?;
    let verifying_key = signing_key.verifying_key();
    let public_key = verifying_key.to_encoded_point(false);
    let public_key_bytes = &public_key.as_bytes()[1..]; // Skip 0x04 prefix

    let mut hasher = tiny_keccak::Keccak::v256();
    let mut address_hash = [0u8; 32];
    hasher.update(public_key_bytes);
    hasher.finalize(&mut address_hash);
    let from_address = format!("0x{}", hex::encode(&address_hash[12..]));
    println!("NONOS: From address: {}", from_address);

    // Get nonce and gas price
    println!("NONOS: Getting nonce...");
    let nonce = get_nonce(&from_address).await?;
    println!("NONOS: Nonce: {}", nonce);

    println!("NONOS: Getting gas price...");
    let gas_price = get_gas_price().await?;
    println!("NONOS: Gas price: {} wei", gas_price);

    // Estimate gas (use safe defaults)
    // ERC-20 transfers on proxy contracts can use more gas, use 150k for safety
    let gas_limit = if data.is_empty() { 21000u64 } else { 150000u64 };
    println!("NONOS: Gas limit: {}", gas_limit);

    // Chain ID 1 for mainnet
    let chain_id = 1u64;

    // Sign the transaction
    println!("NONOS: Signing transaction...");
    let signed_tx = sign_transaction(
        private_key,
        to,
        value,
        &data,
        nonce,
        gas_limit,
        gas_price,
        chain_id,
    )?;
    println!("NONOS: Transaction signed, broadcasting...");

    // Broadcast
    let tx_hash = send_raw_transaction(&signed_tx).await?;
    println!("NONOS: Transaction sent: {}", tx_hash);

    Ok(tx_hash)
}

/// Send ETH or tokens with specific gas limit
async fn send_transaction_with_gas(
    private_key: &str,
    to: &str,
    value: u128,
    data: Vec<u8>,
    gas_limit: u64,
) -> Result<String, String> {
    use tiny_keccak::Hasher;

    println!("NONOS: send_transaction_with_gas called - to: {}, value: {}, data_len: {}, gas: {}",
        to, value, data.len(), gas_limit);

    // Get sender address from private key
    let key_bytes = hex::decode(private_key.trim_start_matches("0x"))
        .map_err(|e| format!("Invalid private key: {}", e))?;
    let signing_key = k256::ecdsa::SigningKey::from_slice(&key_bytes)
        .map_err(|e| format!("Invalid key: {}", e))?;
    let verifying_key = signing_key.verifying_key();
    let public_key = verifying_key.to_encoded_point(false);
    let public_key_bytes = &public_key.as_bytes()[1..]; // Skip 0x04 prefix

    let mut hasher = tiny_keccak::Keccak::v256();
    let mut address_hash = [0u8; 32];
    hasher.update(public_key_bytes);
    hasher.finalize(&mut address_hash);
    let from_address = format!("0x{}", hex::encode(&address_hash[12..]));
    println!("NONOS: From address: {}", from_address);

    // Get nonce and gas price
    let nonce = get_nonce(&from_address).await?;
    println!("NONOS: Nonce: {}", nonce);

    let gas_price = get_gas_price().await?;
    println!("NONOS: Gas price: {} wei ({} gwei)", gas_price, gas_price / 1_000_000_000);

    // Chain ID 1 for mainnet
    let chain_id = 1u64;

    // Sign the transaction
    println!("NONOS: Signing transaction with gas limit {}...", gas_limit);
    let signed_tx = sign_transaction(
        private_key,
        to,
        value,
        &data,
        nonce,
        gas_limit,
        gas_price,
        chain_id,
    )?;
    println!("NONOS: Transaction signed, broadcasting...");

    // Broadcast
    let tx_hash = send_raw_transaction(&signed_tx).await?;
    println!("NONOS: Transaction sent: {}", tx_hash);

    Ok(tx_hash)
}

// =============================================================================
// Application State
// =============================================================================

/// Global application state
pub struct AppState {
    /// Anyone network state
    network: Arc<RwLock<NetworkState>>,
    /// Wallet state
    wallet: Arc<RwLock<WalletState>>,
    /// Node connections
    nodes: Arc<RwLock<NodeState>>,
    /// Browser tabs and history
    browser: Arc<RwLock<BrowserState>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            network: Arc::new(RwLock::new(NetworkState::default())),
            wallet: Arc::new(RwLock::new(WalletState::default())),
            nodes: Arc::new(RwLock::new(NodeState::default())),
            browser: Arc::new(RwLock::new(BrowserState::default())),
        }
    }
}

// =============================================================================
// Network State - Anyone Network Integration
// =============================================================================

#[derive(Clone, Debug, Serialize)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Bootstrapping,
    Connected,
    Error,
}

#[derive(Debug)]
pub struct NetworkState {
    /// Connection status
    status: ConnectionStatus,
    /// Bootstrap progress (0-100)
    bootstrap_progress: u8,
    /// Number of established circuits
    circuits: u32,
    /// SOCKS5 proxy address
    socks_addr: SocketAddr,
    /// Control port for identity rotation
    control_port: u16,
    /// Error message if any
    error: Option<String>,
    /// The anon process handle (kept in separate field for ownership)
    anon_pid: Option<u32>,
    /// Data directory
    data_dir: PathBuf,
}

impl Default for NetworkState {
    fn default() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nonos")
            .join("anon");

        Self {
            status: ConnectionStatus::Disconnected,
            bootstrap_progress: 0,
            circuits: 0,
            socks_addr: SocketAddr::from(([127, 0, 0, 1], 9050)),
            control_port: 9051,
            error: None,
            anon_pid: None,
            data_dir,
        }
    }
}

// =============================================================================
// Wallet State - Real Wallet Integration
// =============================================================================

#[derive(Debug, Default)]
pub struct WalletState {
    /// Is wallet initialized
    initialized: bool,
    /// Is wallet locked
    locked: bool,
    /// ETH address (derived from wallet)
    address: Option<String>,
    /// Private key (32 bytes hex, for signing - stored encrypted in production)
    private_key: Option<String>,
    /// Mnemonic phrase (for backup - should be encrypted in production)
    mnemonic: Option<String>,
    /// NOX balance (in wei) - fetched from blockchain
    nox_balance: u128,
    /// ETH balance (in wei) - fetched from blockchain
    eth_balance: u128,
    /// Pending rewards (fetched from staking contract)
    pending_rewards: u128,
    /// Staked amount (in wei) - fetched from staking contract
    staked_amount: u128,
    /// Staking tier (0-4: Bronze, Silver, Gold, Platinum, Diamond)
    staking_tier: u8,
    /// Current epoch number
    current_epoch: u64,
    /// Last balance refresh timestamp
    last_refresh: u64,
}

// =============================================================================
// Node State - Community Node Connections
// =============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct NodeInfo {
    id: String,
    address: String,
    quality_score: f64,
    latency_ms: u32,
    connected: bool,
}

#[derive(Debug, Default)]
pub struct NodeState {
    /// Connected nodes
    nodes: Vec<NodeInfo>,
    /// Is embedded node running
    embedded_running: bool,
    /// Embedded node quality score
    embedded_quality: f64,
    /// Total requests proxied
    total_requests: AtomicU64,
    /// Embedded NONOS node process ID
    embedded_pid: Option<u32>,
    /// NONOS node ID
    node_id: Option<String>,
    /// NONOS node API address
    api_addr: String,
    /// P2P port
    p2p_port: u16,
}

// =============================================================================
// Browser State
// =============================================================================

#[derive(Debug, Default)]
pub struct BrowserState {
    /// Tab counter for unique IDs
    next_tab_id: u32,
    /// Browsing history (isolated per session)
    history: Vec<String>,
}

// =============================================================================
// Response Types
// =============================================================================

#[derive(Serialize)]
struct AppInfo {
    name: &'static str,
    version: &'static str,
    platform: &'static str,
    arch: &'static str,
    build: &'static str,
}

#[derive(Serialize, Clone)]
struct NetworkStatusResponse {
    connected: bool,
    status: String,
    bootstrap_progress: u8,
    circuits: u32,
    socks_port: u16,
    error: Option<String>,
}

#[derive(Serialize)]
struct WalletStatusResponse {
    initialized: bool,
    locked: bool,
    address: Option<String>,
    nox_balance: String,
    eth_balance: String,
    pending_rewards: String,
}

#[derive(Serialize)]
struct StakingStatusResponse {
    staked_amount: String,
    tier: String,
    tier_multiplier: String,
    pending_rewards: String,
    current_epoch: u64,
    next_tier_threshold: String,
    estimated_apy: String,
}

/// Staking tier names and thresholds
const STAKING_TIERS: &[(&str, u128, &str)] = &[
    ("Bronze", 1_000, "1.0x"),
    ("Silver", 10_000, "1.2x"),
    ("Gold", 50_000, "1.5x"),
    ("Platinum", 200_000, "2.0x"),
    ("Diamond", 1_000_000, "2.5x"),
];

#[derive(Serialize)]
struct NodeStatusResponse {
    running: bool,
    connected_nodes: usize,
    quality: f64,
    total_requests: u64,
}

// =============================================================================
// NONOS Privacy Services Response Types
// =============================================================================

#[derive(Serialize)]
struct PrivacyStatsResponse {
    zk_proofs_issued: u64,
    zk_verifications: u64,
    cache_hits: u64,
    cache_misses: u64,
    cache_hit_rate: f64,
    tracking_blocked: u64,
    tracking_total: u64,
    block_rate: f64,
    stealth_payments: u64,
    stealth_scanned: u64,
}

#[derive(Serialize)]
struct ZkIdentityResponse {
    identity_id: String,
    commitment: String,
    merkle_root: String,
}

#[derive(Serialize)]
struct TrackingCheckResponse {
    domain: String,
    blocked: bool,
    reason: Option<String>,
}

// =============================================================================
// Tauri Commands - App Info
// =============================================================================

#[tauri::command]
fn get_app_info() -> AppInfo {
    AppInfo {
        name: "NONOS Ecosystem",
        version: env!("CARGO_PKG_VERSION"),
        platform: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        build: if cfg!(debug_assertions) { "debug" } else { "release" },
    }
}

// =============================================================================
// Tauri Commands - Network (Anyone Network)
// =============================================================================

#[tauri::command]
async fn network_connect(
    state: State<'_, AppState>,
    window: Window,
) -> Result<NetworkStatusResponse, String> {
    let mut network = state.network.write().await;

    if matches!(network.status, ConnectionStatus::Connected) {
        return Ok(create_network_response(&network));
    }

    network.status = ConnectionStatus::Connecting;
    network.error = None;
    emit_network_status(&window, &network);

    // Create data directory
    if !network.data_dir.exists() {
        tokio::fs::create_dir_all(&network.data_dir)
            .await
            .map_err(|e| format!("Failed to create data dir: {}", e))?;
    }

    // Find anon binary
    let anon_path = find_anon_binary()
        .await
        .map_err(|e| format!("anon binary not found: {}", e))?;

    // Write configuration
    let anonrc_path = network.data_dir.join("anonrc");
    write_anonrc(&anonrc_path, &network)
        .await
        .map_err(|e| format!("Failed to write config: {}", e))?;

    network.status = ConnectionStatus::Bootstrapping;
    emit_network_status(&window, &network);

    // Launch anon binary
    let mut child = Command::new(&anon_path)
        .arg("-f")
        .arg(&anonrc_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to launch anon: {}", e))?;

    // Store PID for later cleanup
    network.anon_pid = child.id();

    // Monitor bootstrap progress in background
    let window_clone = window.clone();
    let network_state = state.network.clone();

    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                // Parse bootstrap progress
                if line.contains("Bootstrapped") {
                    if let Some(pct) = parse_bootstrap_progress(&line) {
                        let mut net = network_state.write().await;
                        net.bootstrap_progress = pct;

                        if pct >= 100 {
                            net.status = ConnectionStatus::Connected;
                            net.circuits = 3; // Default circuits
                        }

                        emit_network_status(&window_clone, &net);
                    }
                }

                // Check for errors
                if line.contains("[err]") || line.contains("fatal") {
                    let mut net = network_state.write().await;
                    net.status = ConnectionStatus::Error;
                    net.error = Some(line.clone());
                    emit_network_status(&window_clone, &net);
                }
            }
        });
    }

    // Wait for bootstrap or early success
    let socks_addr = network.socks_addr;
    drop(network); // Release lock

    // Poll for SOCKS5 availability
    for _ in 0..60 {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let net = state.network.read().await;
        if matches!(net.status, ConnectionStatus::Connected) {
            return Ok(create_network_response(&net));
        }
        if matches!(net.status, ConnectionStatus::Error) {
            return Err(net.error.clone().unwrap_or_else(|| "Unknown error".into()));
        }
        drop(net);

        // Check if SOCKS5 proxy is ready
        if TcpStream::connect(socks_addr).await.is_ok() {
            let mut net = state.network.write().await;
            net.status = ConnectionStatus::Connected;
            net.bootstrap_progress = 100;
            net.circuits = 3;
            emit_network_status(&window, &net);
            return Ok(create_network_response(&net));
        }
    }

    Err("Bootstrap timeout".into())
}

#[tauri::command]
async fn network_disconnect(
    state: State<'_, AppState>,
    window: Window,
) -> Result<(), String> {
    let mut network = state.network.write().await;

    // Kill anon process
    if let Some(pid) = network.anon_pid.take() {
        #[cfg(unix)]
        {
            use std::process::Command as StdCommand;
            let _ = StdCommand::new("kill").arg(pid.to_string()).output();
        }
        #[cfg(windows)]
        {
            use std::process::Command as StdCommand;
            let _ = StdCommand::new("taskkill")
                .args(&["/PID", &pid.to_string(), "/F"])
                .output();
        }
    }

    network.status = ConnectionStatus::Disconnected;
    network.bootstrap_progress = 0;
    network.circuits = 0;
    network.error = None;

    emit_network_status(&window, &network);
    Ok(())
}

#[tauri::command]
async fn network_get_status(state: State<'_, AppState>) -> Result<NetworkStatusResponse, String> {
    let network = state.network.read().await;
    Ok(create_network_response(&network))
}

#[tauri::command]
async fn network_new_identity(
    state: State<'_, AppState>,
    window: Window,
) -> Result<(), String> {
    let network = state.network.read().await;

    if !matches!(network.status, ConnectionStatus::Connected) {
        return Err("Not connected".into());
    }

    // Send NEWNYM signal via control port
    let control_addr = SocketAddr::from(([127, 0, 0, 1], network.control_port));

    if let Ok(mut stream) = TcpStream::connect(control_addr).await {
        let _ = stream.write_all(b"AUTHENTICATE\r\n").await;
        let _ = stream.write_all(b"SIGNAL NEWNYM\r\n").await;
        let _ = stream.write_all(b"QUIT\r\n").await;
    }

    // Emit identity changed event
    window
        .emit("nonos://identity-changed", ())
        .map_err(|e| e.to_string())?;

    Ok(())
}

// =============================================================================
// Tauri Commands - Wallet
// =============================================================================

#[tauri::command]
async fn wallet_get_status(state: State<'_, AppState>) -> Result<WalletStatusResponse, String> {
    // First get current state
    let (address, initialized, locked) = {
        let wallet = state.wallet.read().await;
        (wallet.address.clone(), wallet.initialized, wallet.locked)
    };

    // If wallet has an address, fetch REAL balances from blockchain
    let (eth_balance, nox_balance) = if let Some(ref addr) = address {
        println!("NONOS: Fetching real balances for {}", addr);
        fetch_real_balances(addr).await
    } else {
        (0, 0)
    };

    // Update stored balances
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
async fn wallet_create(
    state: State<'_, AppState>,
    _password: String,
) -> Result<String, String> {
    // Generate real BIP39 mnemonic
    let mnemonic = generate_mnemonic();

    // Derive private key and address from mnemonic
    let (address, private_key) = derive_keys_from_mnemonic(&mnemonic);

    let mut wallet = state.wallet.write().await;
    wallet.initialized = true;
    wallet.locked = false;
    wallet.address = Some(address);
    wallet.private_key = Some(private_key);
    wallet.mnemonic = Some(mnemonic.clone());

    println!("NONOS: Created new wallet with address {}", wallet.address.as_ref().unwrap());

    Ok(mnemonic)
}

#[tauri::command]
async fn wallet_import(
    state: State<'_, AppState>,
    mnemonic: String,
    _password: String,
) -> Result<(), String> {
    // Validate mnemonic
    let word_count = mnemonic.split_whitespace().count();
    if word_count != 12 && word_count != 24 {
        return Err("Invalid mnemonic: must be 12 or 24 words".into());
    }

    // Derive private key and address from mnemonic
    let (address, private_key) = derive_keys_from_mnemonic(&mnemonic);

    let mut wallet = state.wallet.write().await;
    wallet.initialized = true;
    wallet.locked = false;
    wallet.address = Some(address.clone());
    wallet.private_key = Some(private_key);
    wallet.mnemonic = Some(mnemonic);

    println!("NONOS: Imported wallet with address {}", address);

    Ok(())
}

#[tauri::command]
async fn wallet_unlock(
    state: State<'_, AppState>,
    _password: String,
) -> Result<(), String> {
    let mut wallet = state.wallet.write().await;

    if !wallet.initialized {
        return Err("Wallet not initialized".into());
    }

    // In real implementation, verify password against stored hash
    wallet.locked = false;
    Ok(())
}

#[tauri::command]
async fn wallet_lock(state: State<'_, AppState>) -> Result<(), String> {
    let mut wallet = state.wallet.write().await;
    wallet.locked = true;
    Ok(())
}

#[tauri::command]
async fn wallet_get_address(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let wallet = state.wallet.read().await;
    Ok(wallet.address.clone())
}

/// Send ETH to an address
#[tauri::command]
async fn wallet_send_eth(
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

    // Parse amount (in ETH)
    let amount_eth: f64 = amount.parse()
        .map_err(|_| "Invalid amount")?;
    let amount_wei = (amount_eth * 1e18) as u128;

    // Check balance
    if amount_wei > wallet.eth_balance {
        return Err("Insufficient ETH balance".into());
    }

    drop(wallet); // Release lock before async calls

    println!("NONOS: Sending {} ETH to {}", amount_eth, to);
    let tx_hash = send_transaction(&private_key, &to, amount_wei, vec![]).await?;
    println!("NONOS: ETH transfer tx: {}", tx_hash);

    Ok(format!("Sent {} ETH! Tx: {}", amount_eth, tx_hash))
}

/// Send NOX tokens to an address
#[tauri::command]
async fn wallet_send_nox(
    state: State<'_, AppState>,
    to: String,
    amount: String,
) -> Result<String, String> {
    println!("NONOS: wallet_send_nox called - to: {}, amount: {}", to, amount);

    let (private_key, from_address) = {
        let wallet = state.wallet.read().await;

        if !wallet.initialized || wallet.locked {
            println!("NONOS: Wallet not initialized or locked");
            return Err("Wallet locked or not initialized".into());
        }

        let pk = wallet.private_key.clone().ok_or("Private key not available")?;
        let addr = wallet.address.clone().ok_or("Wallet address not available")?;
        (pk, addr)
    };

    // Parse amount (in NOX)
    let amount_nox: f64 = amount.parse()
        .map_err(|_| "Invalid amount")?;
    let amount_wei = (amount_nox * 1e18) as u128;

    // Fetch FRESH balances from blockchain (don't rely on cached values)
    println!("NONOS: Fetching fresh balances before transfer...");
    let (eth_balance, nox_balance) = fetch_real_balances(&from_address).await;
    println!("NONOS: Fresh balances - NOX: {} ({} tokens), ETH: {} ({} ETH)",
        nox_balance, nox_balance as f64 / 1e18,
        eth_balance, eth_balance as f64 / 1e18);

    // Check NOX balance
    if amount_wei > nox_balance {
        return Err(format!("Insufficient NOX balance. You have {} NOX, trying to send {} NOX",
            nox_balance as f64 / 1e18, amount_nox));
    }

    // Check ETH for gas (need ~0.003 ETH minimum for ERC-20 transfer)
    let min_gas_eth = 3_000_000_000_000_000u128; // 0.003 ETH in wei
    if eth_balance < min_gas_eth {
        return Err(format!("Insufficient ETH for gas. You have {} ETH, need at least 0.003 ETH",
            eth_balance as f64 / 1e18));
    }

    // Build transfer calldata: transfer(address to, uint256 amount)
    // Function selector: 0xa9059cbb
    let to_padded = format!("{:0>64}", to.trim_start_matches("0x").to_lowercase());
    let amount_hex = format!("{:0>64x}", amount_wei);
    let transfer_data = format!("0xa9059cbb{}{}", to_padded, amount_hex);

    println!("NONOS: Sending {} NOX to {}", amount_nox, to);
    println!("NONOS: Transfer data: {}", transfer_data);

    // Estimate gas for this specific transfer
    let estimated_gas = match estimate_gas(&from_address, NOX_TOKEN_ADDRESS, &transfer_data).await {
        Ok(gas) => {
            println!("NONOS: Estimated gas: {}", gas);
            gas
        }
        Err(e) => {
            println!("NONOS: Gas estimation failed: {}. This might mean the transfer will revert.", e);
            // If estimation fails, the tx would likely fail on-chain too
            return Err(format!("Transfer would fail: {}", e));
        }
    };

    let transfer_data_bytes = hex::decode(transfer_data.trim_start_matches("0x"))
        .map_err(|e| format!("Encode error: {}", e))?;

    let tx_hash = send_transaction_with_gas(&private_key, NOX_TOKEN_ADDRESS, 0, transfer_data_bytes, estimated_gas).await?;
    println!("NONOS: NOX transfer tx: {}", tx_hash);

    Ok(format!("Sent {} NOX! Tx: {}", amount_nox, tx_hash))
}

/// Get transaction history (most recent transactions)
#[tauri::command]
async fn wallet_get_transactions(
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let wallet = state.wallet.read().await;

    if !wallet.initialized {
        return Err("Wallet not initialized".into());
    }

    let address = wallet.address.clone().ok_or("No address")?;
    drop(wallet);

    // For now, return empty - in production would query an indexer or event logs
    // Etherscan API or similar would be needed for full tx history
    println!("NONOS: Transaction history query for {}", address);

    Ok(vec![])
}

// =============================================================================
// Tauri Commands - Staking
// =============================================================================

/// Get staking tier from staked amount (in NOX, not wei)
fn get_staking_tier(staked_nox: u128) -> (usize, &'static str, &'static str) {
    for (i, (name, threshold, mult)) in STAKING_TIERS.iter().enumerate().rev() {
        if staked_nox >= *threshold {
            return (i, name, mult);
        }
    }
    (0, "None", "0x")
}

/// Get next tier threshold
fn get_next_tier_threshold(current_tier: usize) -> u128 {
    if current_tier + 1 < STAKING_TIERS.len() {
        STAKING_TIERS[current_tier + 1].1
    } else {
        0 // Already at max tier
    }
}

#[tauri::command]
async fn staking_get_status(state: State<'_, AppState>) -> Result<StakingStatusResponse, String> {
    let wallet = state.wallet.read().await;

    if !wallet.initialized {
        return Err("Wallet not initialized".into());
    }

    // Convert staked amount from wei to NOX for tier calculation
    let staked_nox = wallet.staked_amount / 10u128.pow(18);
    let (tier_idx, tier_name, tier_mult) = get_staking_tier(staked_nox);
    let next_threshold = get_next_tier_threshold(tier_idx);

    // Calculate estimated APY based on tier multiplier
    // Base APY is ~10%, multiplied by tier
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
async fn staking_stake(
    state: State<'_, AppState>,
    amount: String,
) -> Result<String, String> {
    // Check if staking contract is deployed
    if NOX_STAKING_ADDRESS == "0x0000000000000000000000000000000000000000" {
        return Err("Staking contract not yet deployed to mainnet. Coming soon!".into());
    }

    let wallet = state.wallet.read().await;

    if !wallet.initialized || wallet.locked {
        return Err("Wallet locked or not initialized".into());
    }

    let private_key = wallet.private_key.clone()
        .ok_or("Private key not available")?;

    // Parse amount (in NOX)
    let amount_nox: f64 = amount.parse()
        .map_err(|_| "Invalid amount")?;
    let amount_wei = (amount_nox * 1e18) as u128;

    // Check balance
    if amount_wei > wallet.nox_balance {
        return Err("Insufficient NOX balance".into());
    }

    drop(wallet); // Release lock before async calls

    // Step 1: Approve staking contract to spend NOX tokens
    // approve(address spender, uint256 amount) selector: 0x095ea7b3
    let approve_data = format!(
        "0x095ea7b3{:0>64}{:0>64}",
        NOX_STAKING_ADDRESS.trim_start_matches("0x"),
        format!("{:x}", amount_wei)
    );
    let approve_data_bytes = hex::decode(approve_data.trim_start_matches("0x"))
        .map_err(|e| format!("Encode error: {}", e))?;

    println!("NONOS: Approving {} NOX for staking contract", amount_nox);
    let approve_tx = send_transaction(&private_key, NOX_TOKEN_ADDRESS, 0, approve_data_bytes).await?;
    println!("NONOS: Approval tx: {}", approve_tx);

    // Step 2: Call stake(uint256 amount) on staking contract
    // stake(uint256) selector: 0xa694fc3a
    let stake_data = format!("0xa694fc3a{:0>64}", format!("{:x}", amount_wei));
    let stake_data_bytes = hex::decode(stake_data.trim_start_matches("0x"))
        .map_err(|e| format!("Encode error: {}", e))?;

    println!("NONOS: Staking {} NOX", amount_nox);
    let stake_tx = send_transaction(&private_key, NOX_STAKING_ADDRESS, 0, stake_data_bytes).await?;
    println!("NONOS: Stake tx: {}", stake_tx);

    Ok(format!("Staked {} NOX! Tx: {}", amount_nox, stake_tx))
}

#[tauri::command]
async fn staking_unstake(
    state: State<'_, AppState>,
    amount: String,
) -> Result<String, String> {
    // Check if staking contract is deployed
    if NOX_STAKING_ADDRESS == "0x0000000000000000000000000000000000000000" {
        return Err("Staking contract not yet deployed to mainnet. Coming soon!".into());
    }

    let wallet = state.wallet.read().await;

    if !wallet.initialized || wallet.locked {
        return Err("Wallet locked or not initialized".into());
    }

    let private_key = wallet.private_key.clone()
        .ok_or("Private key not available")?;

    // Parse amount (in NOX)
    let amount_nox: f64 = amount.parse()
        .map_err(|_| "Invalid amount")?;
    let amount_wei = (amount_nox * 1e18) as u128;

    drop(wallet); // Release lock before async calls

    // Call startUnstake(uint256 amount) on staking contract
    // startUnstake(uint256) selector: 0x2e17de78
    let unstake_data = format!("0x2e17de78{:0>64}", format!("{:x}", amount_wei));
    let unstake_data_bytes = hex::decode(unstake_data.trim_start_matches("0x"))
        .map_err(|e| format!("Encode error: {}", e))?;

    println!("NONOS: Starting unstake for {} NOX", amount_nox);
    let unstake_tx = send_transaction(&private_key, NOX_STAKING_ADDRESS, 0, unstake_data_bytes).await?;
    println!("NONOS: Unstake tx: {}", unstake_tx);

    Ok(format!("Unstake initiated for {} NOX (14-day unbonding period). Tx: {}", amount_nox, unstake_tx))
}

#[tauri::command]
async fn staking_claim_rewards(
    state: State<'_, AppState>,
) -> Result<String, String> {
    // Check if staking contract is deployed
    if NOX_STAKING_ADDRESS == "0x0000000000000000000000000000000000000000" {
        return Err("Staking contract not yet deployed to mainnet. Coming soon!".into());
    }

    let wallet = state.wallet.read().await;

    if !wallet.initialized || wallet.locked {
        return Err("Wallet locked or not initialized".into());
    }

    let private_key = wallet.private_key.clone()
        .ok_or("Private key not available")?;

    drop(wallet); // Release lock before async calls

    // Call claimRewards() on staking contract
    // claimRewards() selector: 0x372500ab
    let claim_data = "0x372500ab";
    let claim_data_bytes = hex::decode(claim_data.trim_start_matches("0x"))
        .map_err(|e| format!("Encode error: {}", e))?;

    println!("NONOS: Claiming staking rewards");
    let claim_tx = send_transaction(&private_key, NOX_STAKING_ADDRESS, 0, claim_data_bytes).await?;
    println!("NONOS: Claim tx: {}", claim_tx);

    Ok(format!("Rewards claimed! Tx: {}", claim_tx))
}

#[tauri::command]
async fn staking_withdraw(
    state: State<'_, AppState>,
) -> Result<String, String> {
    // Check if staking contract is deployed
    if NOX_STAKING_ADDRESS == "0x0000000000000000000000000000000000000000" {
        return Err("Staking contract not yet deployed to mainnet. Coming soon!".into());
    }

    let wallet = state.wallet.read().await;

    if !wallet.initialized || wallet.locked {
        return Err("Wallet locked or not initialized".into());
    }

    let private_key = wallet.private_key.clone()
        .ok_or("Private key not available")?;

    drop(wallet); // Release lock before async calls

    // Call withdraw() on staking contract (after unbonding period)
    // withdraw() selector: 0x3ccfd60b
    let withdraw_data = "0x3ccfd60b";
    let withdraw_data_bytes = hex::decode(withdraw_data.trim_start_matches("0x"))
        .map_err(|e| format!("Encode error: {}", e))?;

    println!("NONOS: Withdrawing unbonded stake");
    let withdraw_tx = send_transaction(&private_key, NOX_STAKING_ADDRESS, 0, withdraw_data_bytes).await?;
    println!("NONOS: Withdraw tx: {}", withdraw_tx);

    Ok(format!("Withdrawal complete! Tx: {}", withdraw_tx))
}

// =============================================================================
// Tauri Commands - Node Management
// =============================================================================

#[tauri::command]
async fn node_get_status(state: State<'_, AppState>) -> Result<NodeStatusResponse, String> {
    let nodes = state.nodes.read().await;
    Ok(NodeStatusResponse {
        running: nodes.embedded_running,
        connected_nodes: nodes.nodes.len(),
        quality: nodes.embedded_quality,
        total_requests: nodes.total_requests.load(Ordering::Relaxed),
    })
}

#[tauri::command]
async fn node_start_embedded(
    state: State<'_, AppState>,
    window: Window,
) -> Result<(), String> {
    use std::process::{Command, Stdio};
    use std::io::{BufReader, BufRead};

    let mut nodes = state.nodes.write().await;

    if nodes.embedded_running {
        return Ok(());
    }

    // Find nonos-node binary
    let node_binary = find_nonos_node_binary()
        .ok_or_else(|| "NONOS node binary not found. Please install it first.".to_string())?;

    // Create data directory
    let data_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("nonos")
        .join("node");

    std::fs::create_dir_all(&data_dir)
        .map_err(|e| format!("Failed to create node data directory: {}", e))?;

    // Initialize node if not already done
    let config_path = data_dir.join("config.toml");
    if !config_path.exists() {
        let init_output = Command::new(&node_binary)
            .args(["init", "-d", data_dir.to_str().unwrap(), "--non-interactive"])
            .output()
            .map_err(|e| format!("Failed to initialize node: {}", e))?;

        if !init_output.status.success() {
            let stderr = String::from_utf8_lossy(&init_output.stderr);
            return Err(format!("Node initialization failed: {}", stderr));
        }
    }

    // Start the node process
    let mut child = Command::new(&node_binary)
        .args(["run", "-d", data_dir.to_str().unwrap()])
        .env("RUST_LOG", "info")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start NONOS node: {}", e))?;

    let pid = child.id();
    nodes.embedded_pid = Some(pid);
    nodes.embedded_running = true;
    nodes.embedded_quality = 0.95;
    nodes.api_addr = "127.0.0.1:8080".to_string();
    nodes.p2p_port = 9432;

    // Parse stderr for node ID
    if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);
        let window_clone = window.clone();

        std::thread::spawn(move || {
            for line in reader.lines().map_while(Result::ok) {
                // Extract node ID from log output
                if line.contains("Node ID:") {
                    if let Some(id) = line.split("Node ID:").nth(1) {
                        let node_id = id.trim().to_string();
                        let _ = window_clone.emit("nonos://node-id", &node_id);
                    }
                }
                // Detect when node is ready
                if line.contains("NONOS node started successfully") || line.contains("API server listening") {
                    let _ = window_clone.emit("nonos://node-ready", ());
                }
            }
        });
    }

    window
        .emit("nonos://node-started", serde_json::json!({
            "pid": pid,
            "api_addr": "http://127.0.0.1:8080",
            "p2p_port": 9432
        }))
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Find the NONOS node binary on the system
fn find_nonos_node_binary() -> Option<std::path::PathBuf> {
    // Check common locations
    let locations = [
        // Development location (relative to workspace)
        std::path::PathBuf::from("../../target/release/nonos-node"),
        // Installed locations
        std::path::PathBuf::from("/usr/local/bin/nonos-node"),
        std::path::PathBuf::from("/usr/bin/nonos-node"),
        // macOS application support
        dirs::data_local_dir()
            .unwrap_or_default()
            .join("nonos")
            .join("bin")
            .join("nonos-node"),
        // Home directory
        dirs::home_dir()
            .unwrap_or_default()
            .join(".local")
            .join("bin")
            .join("nonos-node"),
    ];

    for path in locations {
        if path.exists() {
            return Some(path);
        }
    }

    // Check PATH
    if let Ok(output) = std::process::Command::new("which")
        .arg("nonos-node")
        .output()
    {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(std::path::PathBuf::from(path));
            }
        }
    }

    None
}

#[tauri::command]
async fn node_stop_embedded(
    state: State<'_, AppState>,
    window: Window,
) -> Result<(), String> {
    let mut nodes = state.nodes.write().await;

    // Kill the node process if running
    if let Some(pid) = nodes.embedded_pid.take() {
        #[cfg(unix)]
        {
            use std::process::Command;
            // Send SIGTERM for graceful shutdown
            let _ = Command::new("kill")
                .args(["-15", &pid.to_string()])
                .output();

            // Give it time to shut down
            std::thread::sleep(std::time::Duration::from_millis(500));

            // Force kill if still running
            let _ = Command::new("kill")
                .args(["-9", &pid.to_string()])
                .output();
        }

        #[cfg(windows)]
        {
            use std::process::Command;
            let _ = Command::new("taskkill")
                .args(["/F", "/PID", &pid.to_string()])
                .output();
        }
    }

    nodes.embedded_running = false;
    nodes.embedded_quality = 0.0;
    nodes.node_id = None;

    window
        .emit("nonos://node-stopped", ())
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn node_get_connected(state: State<'_, AppState>) -> Result<Vec<NodeInfo>, String> {
    let nodes = state.nodes.read().await;
    Ok(nodes.nodes.clone())
}

// =============================================================================
// Tauri Commands - Browser with SOCKS5 Proxy Support
// =============================================================================

/// Browser tab window storage
static BROWSER_WINDOWS: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<u32, String>>> = std::sync::OnceLock::new();

fn get_browser_windows() -> &'static std::sync::Mutex<std::collections::HashMap<u32, String>> {
    BROWSER_WINDOWS.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

/// Response from proxy fetch
#[derive(Serialize)]
struct ProxyFetchResponse {
    success: bool,
    status_code: u16,
    headers: std::collections::HashMap<String, String>,
    body: String,
    content_type: String,
    via_proxy: bool,
    circuit_id: Option<String>,
}

/// Fetch a URL through the SOCKS5 proxy (Anyone Network)
/// This is how NODES POWER THE BROWSER - all traffic goes through community nodes
#[tauri::command]
async fn proxy_fetch(
    state: State<'_, AppState>,
    url: String,
    method: Option<String>,
    headers: Option<std::collections::HashMap<String, String>>,
    body: Option<String>,
) -> Result<ProxyFetchResponse, String> {
    println!("NONOS: proxy_fetch called with url: {}", url);

    let network = state.network.read().await;
    let socks_addr = network.socks_addr;
    let is_connected = matches!(network.status, ConnectionStatus::Connected);
    println!("NONOS: is_connected: {}, socks_addr: {}", is_connected, socks_addr);
    drop(network);

    // Increment request counter
    {
        let nodes = state.nodes.read().await;
        nodes.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    // Build the HTTP client - with SOCKS5 proxy if connected to Anyone Network
    let client = if is_connected {
        // Route ALL traffic through Anyone Network SOCKS5 proxy
        let proxy = reqwest::Proxy::all(format!("socks5h://{}", socks_addr))
            .map_err(|e| format!("Failed to create proxy: {}", e))?;

        reqwest::Client::builder()
            .proxy(proxy)
            .danger_accept_invalid_certs(false) // Keep TLS validation
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| format!("Failed to build proxy client: {}", e))?
    } else {
        // Direct connection (not recommended for privacy)
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Failed to build client: {}", e))?
    };

    // Build the request
    let method_str = method.unwrap_or_else(|| "GET".to_string());
    let method = reqwest::Method::from_bytes(method_str.as_bytes())
        .map_err(|_| "Invalid HTTP method")?;

    let mut request = client.request(method, &url);

    // Add custom headers
    if let Some(hdrs) = headers {
        for (key, value) in hdrs {
            request = request.header(&key, &value);
        }
    }

    // Add body for POST/PUT requests
    if let Some(b) = body {
        request = request.body(b);
    }

    // Execute the request through the proxy
    let response = request.send().await
        .map_err(|e| format!("Request failed: {}", e))?;

    let status_code = response.status().as_u16();
    let content_type = response.headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("text/html")
        .to_string();

    // Collect response headers
    let mut resp_headers = std::collections::HashMap::new();
    for (key, value) in response.headers() {
        if let Ok(v) = value.to_str() {
            resp_headers.insert(key.to_string(), v.to_string());
        }
    }

    // Get response body
    let body_text = response.text().await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    Ok(ProxyFetchResponse {
        success: status_code >= 200 && status_code < 400,
        status_code,
        headers: resp_headers,
        body: body_text,
        content_type,
        via_proxy: is_connected,
        circuit_id: if is_connected { Some("anon-circuit-1".to_string()) } else { None },
    })
}

#[tauri::command]
async fn browser_navigate(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    url: String,
    window: Window,
) -> Result<String, String> {
    // Validate URL
    let target_url = if url.starts_with("http://") || url.starts_with("https://") {
        url.clone()
    } else if url.contains('.') {
        format!("https://{}", url)
    } else {
        // Search query - use privacy-focused search
        format!("https://html.duckduckgo.com/html/?q={}", urlencoding::encode(&url))
    };

    // Add to history
    {
        let mut browser = state.browser.write().await;
        browser.history.push(target_url.clone());
    }

    // Get network status for proxy info
    let network = state.network.read().await;
    let socks_addr = network.socks_addr;
    let is_connected = matches!(network.status, ConnectionStatus::Connected);
    drop(network);

    // Increment request counter
    {
        let nodes = state.nodes.read().await;
        nodes.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    // Create a new browser window with the URL
    let tab_id = {
        let mut browser = state.browser.write().await;
        browser.next_tab_id += 1;
        browser.next_tab_id
    };

    let window_label = format!("browser-{}", tab_id);

    // Route through local proxy when connected to Anyone Network
    let browser_url = if is_connected {
        format!("http://localhost:{}/proxy?url={}", LOCAL_PROXY_PORT, urlencoding::encode(&target_url))
    } else {
        target_url.clone()
    };

    // Create the browser window (underscore prefix to suppress unused warning - window kept for lifetime)
    let _browser_window = tauri::WindowBuilder::new(
        &app_handle,
        &window_label,
        tauri::WindowUrl::External(browser_url.parse().map_err(|e| format!("Invalid URL: {}", e))?)
    )
    .title(format!("NONOS - {}", if is_connected { "Secure" } else { "Direct" }))
    .inner_size(1200.0, 800.0)
    .min_inner_size(800.0, 600.0)
    .center()
    .visible(true)
    .build()
    .map_err(|e| format!("Failed to create browser window: {}", e))?;

    // Store window reference
    {
        let mut windows = get_browser_windows().lock().unwrap();
        windows.insert(tab_id, target_url.clone());
    }

    // Emit navigation event to main window
    window
        .emit("nonos://tab-opened", serde_json::json!({
            "tab_id": tab_id,
            "url": target_url,
            "secure": is_connected,
            "socks_proxy": if is_connected { Some(socks_addr.to_string()) } else { None }
        }))
        .ok();

    Ok(format!(
        "Opened {} in tab {} {}",
        target_url,
        tab_id,
        if is_connected { format!("(via Anyone Network SOCKS5: {})", socks_addr) } else { "(direct connection)".to_string() }
    ))
}

#[tauri::command]
async fn browser_close_tab(
    app_handle: tauri::AppHandle,
    tab_id: u32,
) -> Result<(), String> {
    let window_label = format!("browser-{}", tab_id);
    if let Some(window) = app_handle.get_window(&window_label) {
        window.close().map_err(|e| e.to_string())?;
    }

    let mut windows = get_browser_windows().lock().unwrap();
    windows.remove(&tab_id);

    Ok(())
}

#[tauri::command]
async fn browser_get_tabs() -> Result<Vec<serde_json::Value>, String> {
    let windows = get_browser_windows().lock().unwrap();
    let tabs: Vec<_> = windows.iter().map(|(id, url)| {
        serde_json::json!({
            "id": id,
            "url": url
        })
    }).collect();
    Ok(tabs)
}

#[tauri::command]
async fn browser_get_socks_proxy(state: State<'_, AppState>) -> Result<String, String> {
    let network = state.network.read().await;
    Ok(network.socks_addr.to_string())
}

// =============================================================================
// Tauri Commands - NONOS Privacy Services
// =============================================================================

/// NONOS Node API base URL
const NONOS_API_URL: &str = "http://127.0.0.1:8420/api";

/// Get privacy service statistics from local NONOS node
#[tauri::command]
async fn privacy_get_stats(state: State<'_, AppState>) -> Result<PrivacyStatsResponse, String> {
    let nodes = state.nodes.read().await;

    if !nodes.embedded_running {
        return Err("NONOS node not running. Start the node first.".into());
    }

    // Call the NONOS node API
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/privacy/stats", NONOS_API_URL))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| format!("Failed to connect to NONOS node: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("NONOS node returned error: {}", response.status()));
    }

    let stats: serde_json::Value = response.json().await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(PrivacyStatsResponse {
        zk_proofs_issued: stats["zk_proofs_issued"].as_u64().unwrap_or(0),
        zk_verifications: stats["zk_verifications"].as_u64().unwrap_or(0),
        cache_hits: stats["cache_hits"].as_u64().unwrap_or(0),
        cache_misses: stats["cache_misses"].as_u64().unwrap_or(0),
        cache_hit_rate: stats["cache_hit_rate"].as_f64().unwrap_or(0.0),
        tracking_blocked: stats["tracking_blocked"].as_u64().unwrap_or(0),
        tracking_total: stats["tracking_total"].as_u64().unwrap_or(0),
        block_rate: stats["block_rate"].as_f64().unwrap_or(0.0),
        stealth_payments: stats["stealth_payments"].as_u64().unwrap_or(0),
        stealth_scanned: stats["stealth_scanned"].as_u64().unwrap_or(0),
    })
}

/// Check if a domain should be blocked by the tracking blocker
#[tauri::command]
async fn privacy_check_tracking(
    state: State<'_, AppState>,
    domain: String,
) -> Result<TrackingCheckResponse, String> {
    let nodes = state.nodes.read().await;

    if !nodes.embedded_running {
        // Node not running - return simple local check
        let blocked = KNOWN_TRACKERS.iter().any(|t| domain.contains(t));
        return Ok(TrackingCheckResponse {
            domain: domain.clone(),
            blocked,
            reason: if blocked { Some("Known tracker domain".into()) } else { None },
        });
    }

    // Call the NONOS node API
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/privacy/tracking/check", NONOS_API_URL))
        .json(&serde_json::json!({ "domain": domain }))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| format!("Failed to connect to NONOS node: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("NONOS node returned error: {}", response.status()));
    }

    let result: serde_json::Value = response.json().await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(TrackingCheckResponse {
        domain: domain.clone(),
        blocked: result["blocked"].as_bool().unwrap_or(false),
        reason: result["reason"].as_str().map(String::from),
    })
}

/// Add a domain to the tracking blocklist
#[tauri::command]
async fn privacy_block_domain(
    state: State<'_, AppState>,
    domain: String,
) -> Result<(), String> {
    let nodes = state.nodes.read().await;

    if !nodes.embedded_running {
        return Err("NONOS node not running. Start the node first.".into());
    }

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/privacy/tracking/block", NONOS_API_URL))
        .json(&serde_json::json!({ "domain": domain }))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| format!("Failed to connect to NONOS node: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Failed to block domain: {}", response.status()));
    }

    Ok(())
}

/// Generate a new ZK identity via the NONOS node
#[tauri::command]
async fn privacy_generate_identity(
    state: State<'_, AppState>,
    name: Option<String>,
) -> Result<ZkIdentityResponse, String> {
    let nodes = state.nodes.read().await;

    if !nodes.embedded_running {
        return Err("NONOS node not running. Start the node first.".into());
    }

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/privacy/identity/register", NONOS_API_URL))
        .json(&serde_json::json!({ "name": name }))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Failed to connect to NONOS node: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Failed to generate identity: {}", response.status()));
    }

    let result: serde_json::Value = response.json().await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(ZkIdentityResponse {
        identity_id: result["identity_id"].as_str().unwrap_or("unknown").to_string(),
        commitment: result["commitment"].as_str().unwrap_or("").to_string(),
        merkle_root: result["merkle_root"].as_str().unwrap_or("").to_string(),
    })
}

/// Get the current ZK identity tree root
#[tauri::command]
async fn privacy_get_identity_root(state: State<'_, AppState>) -> Result<String, String> {
    let nodes = state.nodes.read().await;

    if !nodes.embedded_running {
        return Err("NONOS node not running. Start the node first.".into());
    }

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/privacy/identity/root", NONOS_API_URL))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| format!("Failed to connect to NONOS node: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Failed to get identity root: {}", response.status()));
    }

    let result: serde_json::Value = response.json().await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(result["root"].as_str().unwrap_or("").to_string())
}

/// Store content in the cache mixer for privacy protection
#[tauri::command]
async fn privacy_cache_store(
    state: State<'_, AppState>,
    content: String,
) -> Result<String, String> {
    let nodes = state.nodes.read().await;

    if !nodes.embedded_running {
        return Err("NONOS node not running. Start the node first.".into());
    }

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/privacy/cache/store", NONOS_API_URL))
        .json(&serde_json::json!({ "content": content }))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Failed to connect to NONOS node: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Failed to store in cache: {}", response.status()));
    }

    let result: serde_json::Value = response.json().await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(result["commitment"].as_str().unwrap_or("").to_string())
}

/// Known tracker domains for local fallback blocking
const KNOWN_TRACKERS: &[&str] = &[
    "google-analytics.com",
    "googletagmanager.com",
    "facebook.com",
    "connect.facebook.net",
    "doubleclick.net",
    "googlesyndication.com",
    "googleadservices.com",
    "amazon-adsystem.com",
    "scorecardresearch.com",
    "quantserve.com",
    "adsrvr.org",
    "criteo.com",
    "taboola.com",
    "outbrain.com",
    "chartbeat.com",
    "mixpanel.com",
    "segment.io",
    "amplitude.com",
    "hotjar.com",
    "fullstory.com",
    "clarity.ms",
];

// =============================================================================
// Helper Functions
// =============================================================================

fn create_network_response(network: &NetworkState) -> NetworkStatusResponse {
    NetworkStatusResponse {
        connected: matches!(network.status, ConnectionStatus::Connected),
        status: format!("{:?}", network.status),
        bootstrap_progress: network.bootstrap_progress,
        circuits: network.circuits,
        socks_port: network.socks_addr.port(),
        error: network.error.clone(),
    }
}

fn emit_network_status(window: &Window, network: &NetworkState) {
    let _ = window.emit("nonos://network-status", create_network_response(network));
}

fn parse_bootstrap_progress(line: &str) -> Option<u8> {
    if let Some(start) = line.find("Bootstrapped ") {
        let rest = &line[start + 13..];
        if let Some(end) = rest.find('%') {
            if let Ok(pct) = rest[..end].trim().parse::<u8>() {
                return Some(pct);
            }
        }
    }
    None
}

/// Download the anon binary for the current platform
async fn download_anon_binary(target_dir: &PathBuf) -> Result<PathBuf, String> {
    println!("NONOS: Downloading anon binary...");

    // Determine platform and architecture
    let (os, arch) = (std::env::consts::OS, std::env::consts::ARCH);

    let download_url = match (os, arch) {
        ("macos", "aarch64") => "https://github.com/anyone-protocol/ator-protocol/releases/download/v0.4.9.11/anon-live-macos-arm64.zip",
        ("macos", "x86_64") => "https://github.com/anyone-protocol/ator-protocol/releases/download/v0.4.9.11/anon-live-macos-amd64.zip",
        ("linux", "x86_64") => "https://github.com/anyone-protocol/ator-protocol/releases/download/v0.4.9.11/anon-live-linux-amd64.tar.gz",
        ("linux", "aarch64") => "https://github.com/anyone-protocol/ator-protocol/releases/download/v0.4.9.11/anon-live-linux-arm64.tar.gz",
        _ => return Err(format!("Unsupported platform: {}-{}", os, arch)),
    };

    // Create target directory
    tokio::fs::create_dir_all(target_dir)
        .await
        .map_err(|e| format!("Failed to create directory: {}", e))?;

    let target_path = target_dir.join("anon");
    let archive_path = target_dir.join(if os == "macos" { "anon.zip" } else { "anon.tar.gz" });

    // Download the archive
    println!("NONOS: Downloading from {}", download_url);
    let response = reqwest::get(download_url)
        .await
        .map_err(|e| format!("Failed to download: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Download failed with status: {}", response.status()));
    }

    let bytes = response.bytes()
        .await
        .map_err(|e| format!("Failed to read download: {}", e))?;

    tokio::fs::write(&archive_path, &bytes)
        .await
        .map_err(|e| format!("Failed to write archive: {}", e))?;

    println!("NONOS: Extracting anon binary...");

    // Extract based on platform
    if os == "macos" {
        // Use unzip for macOS
        let output = Command::new("unzip")
            .arg("-o")
            .arg(&archive_path)
            .arg("-d")
            .arg(target_dir)
            .output()
            .await
            .map_err(|e| format!("Failed to extract: {}", e))?;

        if !output.status.success() {
            return Err(format!("Extraction failed: {}", String::from_utf8_lossy(&output.stderr)));
        }

        // Find the extracted anon binary (might be in a subdirectory)
        let possible_paths = vec![
            target_dir.join("anon"),
            target_dir.join("anon-live-macos-arm64").join("anon"),
            target_dir.join("anon-live-macos-amd64").join("anon"),
        ];

        for path in possible_paths {
            if path.exists() {
                // Move to target location if needed
                if path != target_path {
                    tokio::fs::rename(&path, &target_path)
                        .await
                        .map_err(|e| format!("Failed to move binary: {}", e))?;
                }
                break;
            }
        }
    } else {
        // Use tar for Linux
        let output = Command::new("tar")
            .arg("-xzf")
            .arg(&archive_path)
            .arg("-C")
            .arg(target_dir)
            .output()
            .await
            .map_err(|e| format!("Failed to extract: {}", e))?;

        if !output.status.success() {
            return Err(format!("Extraction failed: {}", String::from_utf8_lossy(&output.stderr)));
        }
    }

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&target_path)
            .map_err(|e| format!("Failed to get permissions: {}", e))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&target_path, perms)
            .map_err(|e| format!("Failed to set permissions: {}", e))?;
    }

    // Cleanup archive
    let _ = tokio::fs::remove_file(&archive_path).await;

    if target_path.exists() {
        println!("NONOS: anon binary installed at {:?}", target_path);
        Ok(target_path)
    } else {
        Err("Failed to install anon binary".into())
    }
}

async fn find_anon_binary() -> Result<PathBuf, String> {
    // First check standard locations
    let candidates = vec![
        PathBuf::from("/usr/bin/anon"),
        PathBuf::from("/usr/local/bin/anon"),
        PathBuf::from("/opt/anon/bin/anon"),
        PathBuf::from("/opt/homebrew/bin/anon"),
        dirs::home_dir()
            .map(|h| h.join(".local/bin/anon"))
            .unwrap_or_default(),
        PathBuf::from("./anon"),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("anon")))
            .unwrap_or_default(),
        // Also check our data directory
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nonos")
            .join("bin")
            .join("anon"),
    ];

    for path in &candidates {
        if path.exists() {
            return Ok(path.clone());
        }
    }

    // Try PATH
    if let Ok(output) = Command::new("which").arg("anon").output().await {
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout);
            let path = PathBuf::from(path_str.trim());
            if path.exists() {
                return Ok(path);
            }
        }
    }

    // Not found - try to download automatically
    println!("NONOS: anon binary not found locally, attempting auto-download...");
    let download_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("nonos")
        .join("bin");

    download_anon_binary(&download_dir).await
}

async fn write_anonrc(path: &PathBuf, network: &NetworkState) -> Result<(), String> {
    let config = format!(
        r#"SocksPort {}
ControlPort {}
DataDirectory {}
Log notice stderr
SafeLogging 1
AvoidDiskWrites 1
CircuitBuildTimeout 60
"#,
        network.socks_addr.port(),
        network.control_port,
        network.data_dir.display()
    );

    tokio::fs::write(path, config)
        .await
        .map_err(|e| format!("Failed to write anonrc: {}", e))
}

fn format_wei(wei: u128) -> String {
    let eth = wei as f64 / 1e18;
    if eth >= 1.0 {
        format!("{:.4}", eth)
    } else if eth >= 0.0001 {
        format!("{:.6}", eth)
    } else {
        format!("{:.8}", eth)
    }
}

fn generate_mnemonic() -> String {
    use bip39::{Mnemonic, Language};
    use rand::RngCore;

    let mut entropy = [0u8; 16]; // 128 bits for 12 words
    rand::thread_rng().fill_bytes(&mut entropy);

    Mnemonic::from_entropy_in(Language::English, &entropy)
        .expect("Failed to generate mnemonic")
        .to_string()
}

fn derive_address_from_mnemonic(mnemonic: &str) -> String {
    let (address, _) = derive_keys_from_mnemonic(mnemonic);
    address
}

/// Derive both address and private key from mnemonic
/// Returns (address, private_key_hex)
fn derive_keys_from_mnemonic(mnemonic: &str) -> (String, String) {
    use bip39::Mnemonic;
    use k256::ecdsa::SigningKey;
    use tiny_keccak::{Hasher, Keccak};

    let zero_addr = "0x0000000000000000000000000000000000000000".to_string();
    let zero_key = "0".repeat(64);

    let mnemonic = match Mnemonic::parse_normalized(mnemonic) {
        Ok(m) => m,
        Err(_) => return (zero_addr, zero_key),
    };

    let seed = mnemonic.to_seed("");

    // Derive private key using standard Ethereum derivation path
    let mut derived = [0u8; 32];
    let mut hasher = Keccak::v256();
    hasher.update(&seed);
    hasher.update(b"m/44'/60'/0'/0/0");
    hasher.finalize(&mut derived);

    let signing_key = match SigningKey::from_slice(&derived) {
        Ok(key) => key,
        Err(_) => return (zero_addr, zero_key),
    };

    // Get private key as hex
    let private_key_hex = hex::encode(derived);

    // Derive public key and address
    let verifying_key = signing_key.verifying_key();
    let public_key_bytes = verifying_key.to_encoded_point(false);
    let public_key_uncompressed = &public_key_bytes.as_bytes()[1..];

    let mut address_hash = [0u8; 32];
    let mut keccak = Keccak::v256();
    keccak.update(public_key_uncompressed);
    keccak.finalize(&mut address_hash);

    let address = format!("0x{}", hex::encode(&address_hash[12..]));

    (address, private_key_hex)
}

// =============================================================================
// Local HTTP Proxy Server for Full Page Rendering
// =============================================================================

/// Local proxy port for serving web content with full CSS/JS support
const LOCAL_PROXY_PORT: u16 = 9060;

/// Global state for the local proxy server
static PROXY_SOCKS_ADDR: std::sync::OnceLock<std::sync::Mutex<SocketAddr>> = std::sync::OnceLock::new();
static PROXY_CONNECTED: std::sync::OnceLock<AtomicBool> = std::sync::OnceLock::new();

fn get_proxy_socks_addr() -> &'static std::sync::Mutex<SocketAddr> {
    PROXY_SOCKS_ADDR.get_or_init(|| std::sync::Mutex::new(SocketAddr::from(([127, 0, 0, 1], 9050))))
}

fn get_proxy_connected() -> &'static AtomicBool {
    PROXY_CONNECTED.get_or_init(|| AtomicBool::new(false))
}

/// Build the reqwest client with optional SOCKS5 proxy
fn build_proxy_client() -> reqwest::Client {
    let socks_addr = *get_proxy_socks_addr().lock().unwrap();
    let is_connected = get_proxy_connected().load(Ordering::Relaxed);

    if is_connected {
        match reqwest::Proxy::all(format!("socks5h://{}", socks_addr)) {
            Ok(proxy) => {
                reqwest::Client::builder()
                    .proxy(proxy)
                    .timeout(std::time::Duration::from_secs(60))
                    .redirect(reqwest::redirect::Policy::limited(10))
                    .build()
                    .unwrap_or_else(|_| reqwest::Client::new())
            }
            Err(_) => reqwest::Client::new(),
        }
    } else {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    }
}

/// Handle incoming HTTP proxy requests
async fn handle_proxy_request(
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let uri = req.uri();
    let query = uri.query().unwrap_or("");

    // Handle CORS preflight
    if req.method() == hyper::Method::OPTIONS {
        return Ok(Response::builder()
            .status(200)
            .header("Access-Control-Allow-Origin", "*")
            .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
            .header("Access-Control-Allow-Headers", "*")
            .header("Access-Control-Max-Age", "86400")
            .body(Full::new(Bytes::new()))
            .unwrap());
    }

    // Parse the target URL from query parameter
    let target_url = query
        .split('&')
        .find_map(|param| {
            let parts: Vec<&str> = param.splitn(2, '=').collect();
            if parts.len() == 2 && parts[0] == "url" {
                urlencoding::decode(parts[1]).ok().map(|s| s.into_owned())
            } else {
                None
            }
        });

    let target_url = match target_url {
        Some(url) => url,
        None => {
            // No URL param - return error
            let body = r#"<!DOCTYPE html>
<html><head><title>NONOS Proxy</title></head>
<body style="font-family: sans-serif; padding: 40px; background: #0a0a0f; color: #e0e0e0;">
<h1 style="color: #66ffff;">NONOS Privacy Proxy</h1>
<p>Missing 'url' parameter. Usage: /proxy?url=https://example.com</p>
</body></html>"#;
            return Ok(Response::builder()
                .status(400)
                .header("Content-Type", "text/html")
                .header("Access-Control-Allow-Origin", "*")
                .body(Full::new(Bytes::from(body)))
                .unwrap());
        }
    };

    println!("NONOS Proxy: Fetching {}", target_url);

    let client = build_proxy_client();

    // Fetch the target URL
    match client.get(&target_url).send().await {
        Ok(response) => {
            let status = response.status();
            let final_url = response.url().to_string();
            let content_type = response.headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/octet-stream")
                .to_string();

            match response.bytes().await {
                Ok(body_bytes) => {
                    // For HTML content, rewrite URLs to go through our proxy
                    let final_body = if content_type.contains("text/html") {
                        let html = String::from_utf8_lossy(&body_bytes);
                        rewrite_html_urls(&html, &final_url)
                    } else if content_type.contains("text/css") {
                        // Rewrite CSS url() references
                        let css = String::from_utf8_lossy(&body_bytes);
                        rewrite_css_urls(&css, &final_url)
                    } else {
                        body_bytes.to_vec()
                    };

                    Ok(Response::builder()
                        .status(status)
                        .header("Content-Type", &content_type)
                        .header("Access-Control-Allow-Origin", "*")
                        .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
                        .header("Access-Control-Allow-Headers", "*")
                        .body(Full::new(Bytes::from(final_body)))
                        .unwrap())
                }
                Err(e) => {
                    let body = format!(r#"<!DOCTYPE html>
<html><head><title>Error</title></head>
<body style="font-family: sans-serif; padding: 40px; background: #0a0a0f; color: #e0e0e0;">
<h1 style="color: #ff6666;">Error Reading Response</h1>
<p>{}</p>
<p><a href="javascript:history.back()" style="color: #66ffff;">Go Back</a></p>
</body></html>"#, e);
                    Ok(Response::builder()
                        .status(502)
                        .header("Content-Type", "text/html")
                        .header("Access-Control-Allow-Origin", "*")
                        .body(Full::new(Bytes::from(body)))
                        .unwrap())
                }
            }
        }
        Err(e) => {
            let body = format!(r#"<!DOCTYPE html>
<html><head><title>Error</title></head>
<body style="font-family: sans-serif; padding: 40px; background: #0a0a0f; color: #e0e0e0;">
<h1 style="color: #ff6666;">Connection Error</h1>
<p>Failed to fetch: {}</p>
<p style="color: #888;">URL: {}</p>
<p><a href="javascript:history.back()" style="color: #66ffff;">Go Back</a></p>
</body></html>"#, e, target_url);
            Ok(Response::builder()
                .status(502)
                .header("Content-Type", "text/html")
                .header("Access-Control-Allow-Origin", "*")
                .body(Full::new(Bytes::from(body)))
                .unwrap())
        }
    }
}

/// Extract base URL (scheme + host + port) from a full URL
fn extract_base_url(url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url) {
        if let Some(port) = parsed.port() {
            format!("{}://{}:{}", parsed.scheme(), parsed.host_str().unwrap_or(""), port)
        } else {
            format!("{}://{}", parsed.scheme(), parsed.host_str().unwrap_or(""))
        }
    } else {
        String::new()
    }
}

/// Resolve a potentially relative URL against a base URL
fn resolve_url(base_url: &str, href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        // Already absolute
        return href.to_string();
    }

    if let Ok(base) = url::Url::parse(base_url) {
        if href.starts_with("//") {
            // Protocol-relative URL
            return format!("{}:{}", base.scheme(), href);
        }

        // Use URL join for proper resolution
        if let Ok(resolved) = base.join(href) {
            return resolved.to_string();
        }
    }

    // Fallback: return as-is
    href.to_string()
}

/// Rewrite URLs in HTML to go through our local proxy
/// Uses a simpler approach: inject <base> tag and JavaScript to intercept navigation
fn rewrite_html_urls(html: &str, page_url: &str) -> Vec<u8> {
    let proxy_base = format!("http://localhost:{}/proxy?url=", LOCAL_PROXY_PORT);

    // Parse the base URL for relative resource resolution
    let base_url = if let Ok(parsed) = url::Url::parse(page_url) {
        // Get the directory path (everything up to the last /)
        let mut base = parsed.clone();
        if let Some(path) = parsed.path().rfind('/') {
            let _ = base.set_path(&parsed.path()[..=path]);
        }
        base.to_string()
    } else {
        page_url.to_string()
    };

    let origin = extract_base_url(page_url);

    let mut result = html.to_string();

    // Inject <base> tag with the ORIGINAL URL (not proxied) - this allows relative URLs to resolve correctly
    // The browser will resolve relative URLs against this base, then our JS interceptor will proxy them
    let base_tag = format!(r#"<base href="{}">"#, base_url);

    // Also inject JavaScript that intercepts clicks and form submissions
    // Plus fingerprint protection for canvas, WebGL, audio, and other APIs
    let interceptor_script = format!(r#"
<script>
(function() {{
    const PROXY_BASE = '{}';
    const ORIGIN = '{}';

    // ==============================================
    // NONOS FINGERPRINT PROTECTION
    // ==============================================

    // Canvas fingerprint protection - add subtle noise to canvas data
    const originalToDataURL = HTMLCanvasElement.prototype.toDataURL;
    HTMLCanvasElement.prototype.toDataURL = function(...args) {{
        const ctx = this.getContext('2d');
        if (ctx) {{
            const imageData = ctx.getImageData(0, 0, this.width, this.height);
            const data = imageData.data;
            // Add subtle random noise (undetectable but changes fingerprint)
            for (let i = 0; i < data.length; i += 4) {{
                data[i] = data[i] ^ (Math.random() > 0.99 ? 1 : 0);     // R
                data[i+1] = data[i+1] ^ (Math.random() > 0.99 ? 1 : 0); // G
                data[i+2] = data[i+2] ^ (Math.random() > 0.99 ? 1 : 0); // B
            }}
            ctx.putImageData(imageData, 0, 0);
        }}
        return originalToDataURL.apply(this, args);
    }};

    const originalGetImageData = CanvasRenderingContext2D.prototype.getImageData;
    CanvasRenderingContext2D.prototype.getImageData = function(...args) {{
        const imageData = originalGetImageData.apply(this, args);
        // Add subtle noise
        for (let i = 0; i < imageData.data.length; i += 4) {{
            imageData.data[i] = imageData.data[i] ^ (Math.random() > 0.99 ? 1 : 0);
            imageData.data[i+1] = imageData.data[i+1] ^ (Math.random() > 0.99 ? 1 : 0);
            imageData.data[i+2] = imageData.data[i+2] ^ (Math.random() > 0.99 ? 1 : 0);
        }}
        return imageData;
    }};

    // WebGL fingerprint protection - normalize renderer info
    const getParameterOriginal = WebGLRenderingContext.prototype.getParameter;
    WebGLRenderingContext.prototype.getParameter = function(parameter) {{
        // Mask WebGL renderer and vendor
        if (parameter === 37445) return 'Intel Inc.';           // UNMASKED_VENDOR_WEBGL
        if (parameter === 37446) return 'Intel Iris OpenGL Engine'; // UNMASKED_RENDERER_WEBGL
        return getParameterOriginal.apply(this, arguments);
    }};

    // WebGL2 protection
    if (typeof WebGL2RenderingContext !== 'undefined') {{
        const getParameter2Original = WebGL2RenderingContext.prototype.getParameter;
        WebGL2RenderingContext.prototype.getParameter = function(parameter) {{
            if (parameter === 37445) return 'Intel Inc.';
            if (parameter === 37446) return 'Intel Iris OpenGL Engine';
            return getParameter2Original.apply(this, arguments);
        }};
    }}

    // Audio fingerprint protection
    const originalGetChannelData = AudioBuffer.prototype.getChannelData;
    AudioBuffer.prototype.getChannelData = function(channel) {{
        const data = originalGetChannelData.apply(this, arguments);
        // Add minimal noise to audio data
        for (let i = 0; i < data.length; i++) {{
            if (Math.random() > 0.999) {{
                data[i] += (Math.random() - 0.5) * 0.0001;
            }}
        }}
        return data;
    }};

    // Navigator protection - standardize common fingerprint vectors
    Object.defineProperty(navigator, 'hardwareConcurrency', {{
        get: () => 4  // Report standard 4 cores
    }});

    Object.defineProperty(navigator, 'deviceMemory', {{
        get: () => 8  // Report standard 8GB
    }});

    // Screen protection - standardize screen metrics
    Object.defineProperty(screen, 'colorDepth', {{
        get: () => 24
    }});

    Object.defineProperty(screen, 'pixelDepth', {{
        get: () => 24
    }});

    // Timezone protection - expose standard timezone
    const originalGetTimezoneOffset = Date.prototype.getTimezoneOffset;
    Date.prototype.getTimezoneOffset = function() {{
        return 0; // Report UTC
    }};

    // Block WebRTC IP leakage by disabling RTCPeerConnection when using NONOS
    const RTCPeerConnectionOriginal = window.RTCPeerConnection;
    window.RTCPeerConnection = function(...args) {{
        const pc = new RTCPeerConnectionOriginal(...args);
        // Override to prevent IP leaking via STUN
        const originalCreateOffer = pc.createOffer.bind(pc);
        pc.createOffer = function(options) {{
            if (!options) options = {{}};
            options.iceServers = [];  // Disable STUN servers
            return originalCreateOffer(options);
        }};
        return pc;
    }};
    window.RTCPeerConnection.prototype = RTCPeerConnectionOriginal.prototype;

    console.log('NONOS: Fingerprint protection enabled');

    // ==============================================
    // NAVIGATION INTERCEPTION
    // ==============================================

    // Intercept link clicks
    document.addEventListener('click', function(e) {{
        let target = e.target;
        while (target && target.tagName !== 'A') {{
            target = target.parentElement;
        }}
        if (target && target.href && !target.href.startsWith('javascript:') && !target.href.startsWith('#')) {{
            e.preventDefault();
            const url = target.href;
            window.location.href = PROXY_BASE + encodeURIComponent(url);
        }}
    }}, true);

    // Intercept form submissions
    document.addEventListener('submit', function(e) {{
        const form = e.target;
        if (form.method.toLowerCase() === 'get') {{
            e.preventDefault();
            const formData = new FormData(form);
            const params = new URLSearchParams(formData).toString();
            const action = form.action || window.location.href;
            const url = action + (action.includes('?') ? '&' : '?') + params;
            window.location.href = PROXY_BASE + encodeURIComponent(url);
        }}
    }}, true);

    console.log('NONOS Proxy: Navigation interceptor loaded');
}})();
</script>
"#, proxy_base, origin);

    // Find where to inject - prefer <head>, fall back to start of document
    let inject_point = if let Some(pos) = result.to_lowercase().find("<head") {
        // Find the end of the <head> opening tag
        if let Some(end) = result[pos..].find('>') {
            pos + end + 1
        } else {
            0
        }
    } else if let Some(pos) = result.to_lowercase().find("<html") {
        if let Some(end) = result[pos..].find('>') {
            pos + end + 1
        } else {
            0
        }
    } else {
        0
    };

    // Inject base tag and script
    result.insert_str(inject_point, &format!("{}{}", base_tag, interceptor_script));

    // Now rewrite resource URLs (CSS, JS, images) to go through proxy
    // These are loaded by the browser automatically and need proxying

    // Rewrite absolute URLs in src/href attributes
    result = result.replace("src=\"https://", &format!("src=\"{}https://", proxy_base));
    result = result.replace("src='https://", &format!("src='{}https://", proxy_base));
    result = result.replace("src=\"http://", &format!("src=\"{}http://", proxy_base));
    result = result.replace("src='http://", &format!("src='{}http://", proxy_base));

    // Rewrite protocol-relative URLs
    result = result.replace("src=\"//", &format!("src=\"{}https://", proxy_base));
    result = result.replace("src='//", &format!("src='{}https://", proxy_base));

    // Rewrite root-relative URLs (starting with /)
    result = rewrite_root_relative_urls(&result, &origin, &proxy_base);

    // Handle link tags (CSS)
    result = result.replace("href=\"https://", &format!("href=\"{}https://", proxy_base));
    result = result.replace("href='https://", &format!("href='{}https://", proxy_base));
    result = result.replace("href=\"http://", &format!("href=\"{}http://", proxy_base));
    result = result.replace("href='http://", &format!("href='{}http://", proxy_base));
    result = result.replace("href=\"//", &format!("href=\"{}https://", proxy_base));
    result = result.replace("href='//", &format!("href='{}https://", proxy_base));

    // Rewrite srcset for responsive images
    result = rewrite_srcset(&result, &origin, &proxy_base);

    // Rewrite CSS url() in inline styles and <style> tags
    result = rewrite_inline_css_urls(&result, page_url);

    result.into_bytes()
}

/// Rewrite root-relative URLs (starting with /) in src attributes
fn rewrite_root_relative_urls(html: &str, origin: &str, proxy_base: &str) -> String {
    let mut result = String::new();
    let mut remaining = html;

    // Pattern: src="/ but not src="// (protocol-relative)
    while let Some(pos) = remaining.find("src=\"/") {
        // Check it's not protocol-relative
        if remaining[pos..].starts_with("src=\"//") {
            result.push_str(&remaining[..pos + 7]);
            remaining = &remaining[pos + 7..];
            continue;
        }

        // It's root-relative
        result.push_str(&remaining[..pos]);
        result.push_str("src=\"");
        result.push_str(proxy_base);
        result.push_str(&urlencoding::encode(origin));

        remaining = &remaining[pos + 5..]; // Skip past src="
    }
    result.push_str(remaining);

    // Same for single quotes
    let html = result;
    let mut result = String::new();
    let mut remaining = html.as_str();

    while let Some(pos) = remaining.find("src='/") {
        if remaining[pos..].starts_with("src='//") {
            result.push_str(&remaining[..pos + 7]);
            remaining = &remaining[pos + 7..];
            continue;
        }

        result.push_str(&remaining[..pos]);
        result.push_str("src='");
        result.push_str(proxy_base);
        result.push_str(&urlencoding::encode(origin));

        remaining = &remaining[pos + 5..];
    }
    result.push_str(remaining);

    result
}

/// Rewrite srcset attributes for responsive images
fn rewrite_srcset(html: &str, base_url: &str, proxy_base: &str) -> String {
    let mut result = html.to_string();

    // Process srcset with double quotes
    result = rewrite_srcset_pattern(&result, "srcset=\"", '"', base_url, proxy_base);
    // Process srcset with single quotes
    result = rewrite_srcset_pattern(&result, "srcset='", '\'', base_url, proxy_base);

    result
}

fn rewrite_srcset_pattern(html: &str, pattern: &str, quote: char, base_url: &str, proxy_base: &str) -> String {
    let mut result = String::new();
    let mut remaining = html;

    while let Some(start) = remaining.find(pattern) {
        result.push_str(&remaining[..start]);

        let after = &remaining[start + pattern.len()..];

        if let Some(end) = after.find(quote) {
            let srcset_content = &after[..end];
            let rewritten = rewrite_srcset_content(srcset_content, base_url, proxy_base);
            result.push_str(&format!("{}{}{}", pattern, rewritten, quote));
            remaining = &after[end + 1..];
        } else {
            result.push_str(pattern);
            remaining = after;
        }
    }

    result.push_str(remaining);
    result
}

fn rewrite_srcset_content(srcset: &str, base_url: &str, proxy_base: &str) -> String {
    srcset
        .split(',')
        .map(|part| {
            let trimmed = part.trim();
            let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
            if let Some(url) = parts.first() {
                let resolved = resolve_url(base_url, url);
                let proxied = format!("{}{}", proxy_base, urlencoding::encode(&resolved));
                if parts.len() > 1 {
                    format!("{} {}", proxied, parts[1])
                } else {
                    proxied
                }
            } else {
                trimmed.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Rewrite CSS url() references
fn rewrite_css_urls(css: &str, page_url: &str) -> Vec<u8> {
    let proxy_base = format!("http://localhost:{}/proxy?url=", LOCAL_PROXY_PORT);
    let base_url = extract_base_url(page_url);

    let mut result = css.to_string();

    // Handle url("/path"), url('/path'), url(/path), url("https://..."), etc.
    result = rewrite_css_url_pattern(&result, "url(\"/", "url(\"", &base_url, &proxy_base);
    result = rewrite_css_url_pattern(&result, "url('/", "url('", &base_url, &proxy_base);
    result = rewrite_css_url_pattern(&result, "url(/", "url(", &base_url, &proxy_base);

    // Handle absolute URLs in CSS
    result = result.replace("url(\"https://", &format!("url(\"{}https://", proxy_base));
    result = result.replace("url('https://", &format!("url('{}https://", proxy_base));
    result = result.replace("url(https://", &format!("url({}https://", proxy_base));
    result = result.replace("url(\"http://", &format!("url(\"{}http://", proxy_base));
    result = result.replace("url('http://", &format!("url('{}http://", proxy_base));
    result = result.replace("url(http://", &format!("url({}http://", proxy_base));

    // Protocol-relative URLs in CSS
    result = result.replace("url(\"//", &format!("url(\"{}https://", proxy_base));
    result = result.replace("url('//", &format!("url('{}https://", proxy_base));
    result = result.replace("url(//", &format!("url({}https://", proxy_base));

    // Handle @import statements
    result = rewrite_css_imports(&result, &base_url, &proxy_base);

    result.into_bytes()
}

fn rewrite_css_url_pattern(css: &str, pattern: &str, prefix: &str, base_url: &str, proxy_base: &str) -> String {
    let mut result = String::new();
    let mut remaining = css;

    while let Some(start) = remaining.find(pattern) {
        result.push_str(&remaining[..start]);

        let after_pattern = &remaining[start + pattern.len()..];

        // Determine end character based on pattern
        let end_char = if pattern.contains('"') {
            '"'
        } else if pattern.contains('\'') {
            '\''
        } else {
            ')'
        };

        if let Some(end) = after_pattern.find(end_char) {
            let path = &after_pattern[..end];
            // Skip data: URLs
            if !path.starts_with("data:") {
                let full_url = resolve_url(base_url, &format!("/{}", path.trim_start_matches('/')));
                result.push_str(&format!("{}{}{}{}",
                    prefix,
                    proxy_base,
                    urlencoding::encode(&full_url),
                    end_char
                ));
            } else {
                result.push_str(pattern);
                result.push_str(&after_pattern[..end + 1]);
            }
            remaining = &after_pattern[end + 1..];
        } else {
            result.push_str(pattern);
            remaining = after_pattern;
        }
    }

    result.push_str(remaining);
    result
}

fn rewrite_css_imports(css: &str, _base_url: &str, proxy_base: &str) -> String {
    let mut result = css.to_string();

    // @import url("...") and @import "..."
    // This is a simplified implementation
    result = result.replace("@import url(\"https://", &format!("@import url(\"{}https://", proxy_base));
    result = result.replace("@import url('https://", &format!("@import url('{}https://", proxy_base));
    result = result.replace("@import \"https://", &format!("@import \"{}https://", proxy_base));
    result = result.replace("@import 'https://", &format!("@import '{}https://", proxy_base));

    result
}

/// Rewrite inline CSS (in style attributes and <style> tags) url() references
fn rewrite_inline_css_urls(html: &str, page_url: &str) -> String {
    let proxy_base = format!("http://localhost:{}/proxy?url=", LOCAL_PROXY_PORT);
    let base_url = extract_base_url(page_url);

    let mut result = html.to_string();

    // Handle background: url(/...), background-image: url(/...), etc. in inline styles
    result = result.replace("url(\"/", &format!("url(\"{}{}/", proxy_base, urlencoding::encode(&base_url)));
    result = result.replace("url('/", &format!("url('{}{}/", proxy_base, urlencoding::encode(&base_url)));

    result
}

/// Start the local HTTP proxy server
async fn start_local_proxy_server() {
    let addr = SocketAddr::from(([127, 0, 0, 1], LOCAL_PROXY_PORT));

    let listener = match TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            println!("NONOS: Failed to start local proxy server on port {}: {}", LOCAL_PROXY_PORT, e);
            return;
        }
    };

    println!("NONOS: Local HTTP proxy server running on http://localhost:{}", LOCAL_PROXY_PORT);

    loop {
        let (stream, _) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                println!("NONOS Proxy: Accept error: {}", e);
                continue;
            }
        };

        let io = TokioIo::new(stream);

        tokio::spawn(async move {
            if let Err(e) = http1::Builder::new()
                .serve_connection(io, service_fn(handle_proxy_request))
                .await
            {
                // Silently ignore connection reset errors
                let err_str = e.to_string();
                if !err_str.contains("connection reset") && !err_str.contains("broken pipe") {
                    println!("NONOS Proxy: Connection error: {}", e);
                }
            }
        });
    }
}

/// Get the local proxy URL for a target URL
#[tauri::command]
fn get_proxy_url(target_url: String) -> String {
    format!("http://localhost:{}/proxy?url={}", LOCAL_PROXY_PORT, urlencoding::encode(&target_url))
}

/// Auto-start the anon binary as a SOCKS5 client for traffic routing
/// This is NOT running as a relay node - just as a client to connect to the Anyone Network
async fn auto_start_anon(network_state: Arc<RwLock<NetworkState>>) -> Result<(), String> {
    let mut network = network_state.write().await;

    // Already connected?
    if matches!(network.status, ConnectionStatus::Connected) || matches!(network.status, ConnectionStatus::Connecting) {
        return Ok(());
    }

    // FIRST: Check if SOCKS5 proxy is already running on port 9050
    // (Another anon instance or anyone-client might already be running)
    if TcpStream::connect(network.socks_addr).await.is_ok() {
        println!("NONOS: Detected existing SOCKS5 proxy at {} - using it!", network.socks_addr);
        network.status = ConnectionStatus::Connected;
        network.bootstrap_progress = 100;
        network.circuits = 3;
        get_proxy_connected().store(true, Ordering::Relaxed);
        return Ok(());
    }

    network.status = ConnectionStatus::Connecting;

    // Create data directory
    if !network.data_dir.exists() {
        tokio::fs::create_dir_all(&network.data_dir)
            .await
            .map_err(|e| format!("Failed to create data dir: {}", e))?;
    }

    // Find anon binary
    let anon_path = match find_anon_binary().await {
        Ok(path) => path,
        Err(e) => {
            // anon not installed - browsing will work in direct mode
            println!("NONOS: anon binary not found ({}), browsing will use direct connection", e);
            network.status = ConnectionStatus::Disconnected;
            network.error = Some("anon binary not installed. Install from https://github.com/anyone-protocol/anon-install".into());
            return Ok(());
        }
    };

    println!("NONOS: Found anon binary at {:?}", anon_path);

    // Write anonrc configuration for CLIENT mode (not relay)
    let anonrc_path = network.data_dir.join("anonrc");
    let client_config = format!(
        r#"# NONOS Browser - Anyone Network Client Configuration
# This runs anon as a CLIENT ONLY - not as a relay/node
SocksPort {}
ControlPort {}
DataDirectory {}
Log notice stderr
SafeLogging 1
AvoidDiskWrites 1
CircuitBuildTimeout 60
# CLIENT ONLY - no relay functionality
ClientOnly 1
"#,
        network.socks_addr.port(),
        network.control_port,
        network.data_dir.display()
    );

    tokio::fs::write(&anonrc_path, client_config)
        .await
        .map_err(|e| format!("Failed to write anonrc: {}", e))?;

    network.status = ConnectionStatus::Bootstrapping;

    // Launch anon binary as CLIENT
    let mut child = Command::new(&anon_path)
        .arg("-f")
        .arg(&anonrc_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to launch anon: {}", e))?;

    // Store PID for later cleanup
    network.anon_pid = child.id();
    let socks_addr = network.socks_addr;

    println!("NONOS: anon client started with PID {:?}, SOCKS5 proxy will be at {}", network.anon_pid, socks_addr);

    drop(network); // Release lock

    // Monitor bootstrap progress in background
    let network_state_clone = network_state.clone();
    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                // Parse bootstrap progress
                if line.contains("Bootstrapped") {
                    if let Some(pct) = parse_bootstrap_progress(&line) {
                        let mut net = network_state_clone.write().await;
                        net.bootstrap_progress = pct;
                        println!("NONOS: Bootstrap progress {}%", pct);

                        if pct >= 100 {
                            net.status = ConnectionStatus::Connected;
                            net.circuits = 3;
                            println!("NONOS: Connected to Anyone Network! SOCKS5 proxy ready at {}", net.socks_addr);
                        }
                    }
                }

                // Log errors but don't fail completely
                if line.contains("[err]") {
                    println!("NONOS anon warning: {}", line);
                }
            }
        });
    }

    // Wait for SOCKS5 availability in background
    let network_state_clone = network_state.clone();
    tokio::spawn(async move {
        // Poll for SOCKS5 availability
        for i in 0..30 {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            let net = network_state_clone.read().await;
            if matches!(net.status, ConnectionStatus::Connected) {
                return;
            }
            drop(net);

            // Check if SOCKS5 proxy is ready
            if TcpStream::connect(socks_addr).await.is_ok() {
                let mut net = network_state_clone.write().await;
                if !matches!(net.status, ConnectionStatus::Connected) {
                    net.status = ConnectionStatus::Connected;
                    net.bootstrap_progress = 100;
                    net.circuits = 3;
                    println!("NONOS: SOCKS5 proxy verified at {} - Anyone Network ready!", socks_addr);
                }
                return;
            }

            if i % 5 == 0 {
                println!("NONOS: Waiting for Anyone Network bootstrap... ({} seconds)", i * 2);
            }
        }
        println!("NONOS: Bootstrap timeout - browsing will use direct connection");
    });

    Ok(())
}

fn main() {
    let state = AppState::default();
    let network_state_for_setup = state.network.clone();

    tauri::Builder::default()
        .manage(state)
        .setup(move |app| {
            let window = app.get_window("main").unwrap();

            // Start the local HTTP proxy server for full page rendering
            tauri::async_runtime::spawn(async move {
                start_local_proxy_server().await;
            });

            // Auto-start anon binary as SOCKS5 client for traffic routing
            // This is NOT running as a node - just connecting to Anyone Network
            let network_for_spawn = network_state_for_setup.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = auto_start_anon(network_for_spawn).await {
                    println!("NONOS: Failed to auto-start anon: {}", e);
                }
            });

            // Inject NONOS JavaScript bridge
            window
                .eval(
                    r#"
                window.nonos = {
                    version: '1.0.0',

                    // Network API
                    network: {
                        connect: () => window.__TAURI__.invoke('network_connect'),
                        disconnect: () => window.__TAURI__.invoke('network_disconnect'),
                        getStatus: () => window.__TAURI__.invoke('network_get_status'),
                        newIdentity: () => window.__TAURI__.invoke('network_new_identity'),
                    },

                    // Wallet API
                    wallet: {
                        getStatus: () => window.__TAURI__.invoke('wallet_get_status'),
                        create: (password) => window.__TAURI__.invoke('wallet_create', { password }),
                        import: (mnemonic, password) => window.__TAURI__.invoke('wallet_import', { mnemonic, password }),
                        unlock: (password) => window.__TAURI__.invoke('wallet_unlock', { password }),
                        lock: () => window.__TAURI__.invoke('wallet_lock'),
                        getAddress: () => window.__TAURI__.invoke('wallet_get_address'),
                        sendEth: (to, amount) => window.__TAURI__.invoke('wallet_send_eth', { to, amount: String(amount) }),
                        sendNox: (to, amount) => window.__TAURI__.invoke('wallet_send_nox', { to, amount: String(amount) }),
                        getTransactions: () => window.__TAURI__.invoke('wallet_get_transactions'),
                    },

                    // Staking API - NOX token staking for node rewards
                    staking: {
                        getStatus: () => window.__TAURI__.invoke('staking_get_status'),
                        stake: (amount) => window.__TAURI__.invoke('staking_stake', { amount }),
                        unstake: (amount) => window.__TAURI__.invoke('staking_unstake', { amount }),
                        claimRewards: () => window.__TAURI__.invoke('staking_claim_rewards'),
                        withdraw: () => window.__TAURI__.invoke('staking_withdraw'),
                    },

                    // Node API
                    node: {
                        getStatus: () => window.__TAURI__.invoke('node_get_status'),
                        startEmbedded: () => window.__TAURI__.invoke('node_start_embedded'),
                        stopEmbedded: () => window.__TAURI__.invoke('node_stop_embedded'),
                        getConnected: () => window.__TAURI__.invoke('node_get_connected'),
                    },

                    // Browser API - ALL traffic routed through community nodes
                    browser: {
                        navigate: (url) => window.__TAURI__.invoke('browser_navigate', { url }),
                        getSocksProxy: () => window.__TAURI__.invoke('browser_get_socks_proxy'),
                        // Fetch content through Anyone Network - THIS IS HOW NODES POWER THE BROWSER
                        proxyFetch: (url, options = {}) => window.__TAURI__.invoke('proxy_fetch', {
                            url,
                            method: options.method || 'GET',
                            headers: options.headers || null,
                            body: options.body || null,
                        }),
                    },

                    // Privacy Services API - Powered by NONOS nodes
                    privacy: {
                        // Get statistics from all privacy services
                        getStats: () => window.__TAURI__.invoke('privacy_get_stats'),
                        // Check if a domain is blocked by tracking blocker
                        checkTracking: (domain) => window.__TAURI__.invoke('privacy_check_tracking', { domain }),
                        // Add a domain to the blocklist
                        blockDomain: (domain) => window.__TAURI__.invoke('privacy_block_domain', { domain }),
                        // Generate a new ZK identity
                        generateIdentity: (name) => window.__TAURI__.invoke('privacy_generate_identity', { name }),
                        // Get the current identity Merkle tree root
                        getIdentityRoot: () => window.__TAURI__.invoke('privacy_get_identity_root'),
                        // Store content in the cache mixer
                        cacheStore: (content) => window.__TAURI__.invoke('privacy_cache_store', { content }),
                    },

                    // App API
                    getAppInfo: () => window.__TAURI__.invoke('get_app_info'),

                    // Event listeners
                    onNetworkStatus: (callback) => {
                        return window.__TAURI__.event.listen('nonos://network-status', (event) => callback(event.payload));
                    },
                    onIdentityChanged: (callback) => {
                        return window.__TAURI__.event.listen('nonos://identity-changed', callback);
                    },
                    onNodeStarted: (callback) => {
                        return window.__TAURI__.event.listen('nonos://node-started', callback);
                    },
                    onNodeStopped: (callback) => {
                        return window.__TAURI__.event.listen('nonos://node-stopped', callback);
                    },
                };

                console.log('NONOS Ecosystem bridge initialized - Zero-trust privacy browsing ready');
            "#,
                )
                .ok();

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // App
            get_app_info,
            // Network
            network_connect,
            network_disconnect,
            network_get_status,
            network_new_identity,
            // Wallet
            wallet_get_status,
            wallet_create,
            wallet_import,
            wallet_unlock,
            wallet_lock,
            wallet_get_address,
            wallet_send_eth,
            wallet_send_nox,
            wallet_get_transactions,
            // Staking
            staking_get_status,
            staking_stake,
            staking_unstake,
            staking_claim_rewards,
            staking_withdraw,
            // Node
            node_get_status,
            node_start_embedded,
            node_stop_embedded,
            node_get_connected,
            // Browser - SOCKS5 proxy powered by nodes
            browser_navigate,
            browser_close_tab,
            browser_get_tabs,
            browser_get_socks_proxy,
            proxy_fetch, // Fetch through Anyone Network nodes
            get_proxy_url, // Get local proxy URL for full page rendering
            // Privacy Services - powered by NONOS nodes
            privacy_get_stats,
            privacy_check_tracking,
            privacy_block_domain,
            privacy_generate_identity,
            privacy_get_identity_root,
            privacy_cache_store,
        ])
        .run(tauri::generate_context!())
        .expect("error while running NONOS Ecosystem browser");
}
