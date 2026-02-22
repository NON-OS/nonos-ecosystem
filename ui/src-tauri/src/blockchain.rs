use k256::ecdsa::SigningKey;
use tiny_keccak::{Hasher, Keccak};

// Network configuration
pub const CHAIN_ID_MAINNET: u64 = 1;
pub const CHAIN_ID_SEPOLIA: u64 = 11155111;

// RPC Endpoints
pub const RPC_ENDPOINTS_MAINNET: &[&str] = &[
    "https://ethereum.publicnode.com",
    "https://1rpc.io/eth",
    "https://eth.merkle.io",
    "https://rpc.payload.de",
];

pub const RPC_ENDPOINTS_SEPOLIA: &[&str] = &[
    "https://ethereum-sepolia-rpc.publicnode.com",
    "https://rpc.sepolia.org",
    "https://sepolia.drpc.org",
];

// Token addresses
pub const NOX_TOKEN_ADDRESS_MAINNET: &str = "0x0a26c80Be4E060e688d7C23aDdB92cBb5D2C9eCA";
pub const NOX_TOKEN_ADDRESS_SEPOLIA: &str = "0xC87799c4517Dcdfc65bfefa3Be64Beb89668c66c";

// Staking contract (Sepolia only for now)
pub const NOX_STAKING_ADDRESS_SEPOLIA: &str = "0x7c34956eb5e92460307846D754dD4d1a2400B652";

// Other Sepolia contracts (reserved for future use)
#[allow(dead_code)]
pub const COLLATERAL_MANAGER_SEPOLIA: &str = "0x7366785f977C7ee9Bce920c91fC851B5F1bF1983";
#[allow(dead_code)]
pub const WORK_REGISTRY_SEPOLIA: &str = "0x7dFFC765475f99714564297C8a016500382BB95D";
#[allow(dead_code)]
pub const FEE_ROUTER_SEPOLIA: &str = "0x95f99a47C7471Ef4db176c45f4d4fE6Fada45A10";
pub const PRIVACY_LIQUIDITY_POOL_SEPOLIA: &str = "0x33221345a0dF2638852dc05D0E51d66cE63C874E";
#[allow(dead_code)]
pub const REWARD_DISTRIBUTOR_SEPOLIA: &str = "0x5b111830208EfdAA0D90e24F6a57EB3491F84724";

pub const BALANCE_OF_SELECTOR: &str = "70a08231";

const SOCKS_PROXY: &str = "socks5h://127.0.0.1:9050";

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Network {
    Mainnet,
    Sepolia,
}

impl Network {
    pub fn rpc_endpoints(&self) -> &'static [&'static str] {
        match self {
            Network::Mainnet => RPC_ENDPOINTS_MAINNET,
            Network::Sepolia => RPC_ENDPOINTS_SEPOLIA,
        }
    }

    pub fn chain_id(&self) -> u64 {
        match self {
            Network::Mainnet => CHAIN_ID_MAINNET,
            Network::Sepolia => CHAIN_ID_SEPOLIA,
        }
    }

    #[allow(dead_code)]
    pub fn nox_token_address(&self) -> &'static str {
        match self {
            Network::Mainnet => NOX_TOKEN_ADDRESS_MAINNET,
            Network::Sepolia => NOX_TOKEN_ADDRESS_SEPOLIA,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Network::Mainnet => "Ethereum Mainnet",
            Network::Sepolia => "Sepolia Testnet",
        }
    }
}

fn build_client() -> Result<reqwest::Client, String> {
    let proxy = reqwest::Proxy::all(SOCKS_PROXY)
        .map_err(|e| format!("Failed to create proxy: {}", e))?;
    reqwest::Client::builder()
        .proxy(proxy)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to build proxied client: {}", e))
}

// Network-aware RPC calls
pub async fn eth_call_on_network(network: Network, to: &str, data: &str) -> Result<String, String> {
    let client = build_client()?;

    for endpoint in network.rpc_endpoints() {
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_call",
            "params": [{
                "to": to,
                "data": data
            }, "latest"],
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
                        return Ok(result.to_string());
                    }
                    if json.get("error").is_some() {
                        continue;
                    }
                    if let Some(result) = json.get("result") {
                        if result.is_null() {
                            continue;
                        }
                    }
                }
            }
            Err(_) => continue,
        }
    }

    Err(format!("All {} RPC endpoints failed", network.name()))
}

pub async fn get_eth_balance_on_network(network: Network, address: &str) -> Result<u128, String> {
    let client = build_client()?;

    for endpoint in network.rpc_endpoints() {
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getBalance",
            "params": [address, "latest"],
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
                        let balance = u128::from_str_radix(hex, 16)
                            .map_err(|e| format!("Parse error: {}", e))?;
                        return Ok(balance);
                    }
                    if json.get("error").is_some() {
                        continue;
                    }
                }
            }
            Err(_) => continue,
        }
    }

    Err(format!("Failed to get ETH balance on {}", network.name()))
}

pub async fn get_token_balance_on_network(network: Network, token_address: &str, holder_address: &str) -> Result<u128, String> {
    let addr = holder_address.trim_start_matches("0x").to_lowercase();
    let padded_addr = format!("{:0>64}", addr);
    let data = format!("0x{}{}", BALANCE_OF_SELECTOR, padded_addr);

    let result = eth_call_on_network(network, token_address, &data).await?;

    let hex = result.trim_start_matches("0x");
    if hex.is_empty() || hex.chars().all(|c| c == '0') {
        return Ok(0);
    }

    u128::from_str_radix(hex, 16).map_err(|e| format!("Parse error for '{}': {}", hex, e))
}

pub async fn get_nonce_on_network(network: Network, address: &str) -> Result<u64, String> {
    let client = build_client()?;

    for endpoint in network.rpc_endpoints() {
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

    Err(format!("Failed to get nonce on {}", network.name()))
}

pub async fn get_gas_price_on_network(network: Network) -> Result<u128, String> {
    let client = build_client()?;

    for endpoint in network.rpc_endpoints() {
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

    Err(format!("Failed to get gas price on {}", network.name()))
}

pub async fn send_raw_transaction_on_network(network: Network, signed_tx: &str) -> Result<String, String> {
    let client = build_client()?;

    for endpoint in network.rpc_endpoints() {
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
                        return Ok(result.to_string());
                    }
                    if let Some(error) = json.get("error") {
                        return Err(format!("Transaction failed: {}", error));
                    }
                }
            }
            Err(_) => continue,
        }
    }

    Err(format!("Failed to broadcast transaction on {}", network.name()))
}

// Convenience functions for mainnet (wallet display)
pub async fn fetch_mainnet_balances(address: &str) -> (u128, u128) {
    let eth_balance = get_eth_balance_on_network(Network::Mainnet, address).await.unwrap_or(0);
    let nox_balance = get_token_balance_on_network(Network::Mainnet, NOX_TOKEN_ADDRESS_MAINNET, address).await.unwrap_or(0);
    (eth_balance, nox_balance)
}

// Convenience functions for Sepolia (staking)
pub async fn fetch_sepolia_balances(address: &str) -> (u128, u128) {
    let eth_balance = get_eth_balance_on_network(Network::Sepolia, address).await.unwrap_or(0);
    let nox_balance = get_token_balance_on_network(Network::Sepolia, NOX_TOKEN_ADDRESS_SEPOLIA, address).await.unwrap_or(0);
    (eth_balance, nox_balance)
}

// Transaction signing
pub fn sign_transaction(
    private_key_hex: &str,
    to: &str,
    value: u128,
    data: &[u8],
    nonce: u64,
    gas_limit: u64,
    gas_price: u128,
    chain_id: u64,
) -> Result<String, String> {
    let key_bytes = hex::decode(private_key_hex.trim_start_matches("0x"))
        .map_err(|e| format!("Invalid private key: {}", e))?;
    let signing_key = SigningKey::from_slice(&key_bytes)
        .map_err(|e| format!("Invalid key: {}", e))?;

    let to_bytes = hex::decode(to.trim_start_matches("0x"))
        .map_err(|e| format!("Invalid to address: {}", e))?;

    let mut rlp_items: Vec<Vec<u8>> = Vec::new();
    rlp_items.push(encode_rlp_uint(nonce as u128));
    rlp_items.push(encode_rlp_uint(gas_price));
    rlp_items.push(encode_rlp_uint(gas_limit as u128));
    rlp_items.push(to_bytes.clone());
    rlp_items.push(encode_rlp_uint(value));
    rlp_items.push(data.to_vec());
    rlp_items.push(encode_rlp_uint(chain_id as u128));
    rlp_items.push(vec![]);
    rlp_items.push(vec![]);

    let unsigned_tx = encode_rlp_list(&rlp_items);

    let mut hasher = Keccak::v256();
    let mut hash = [0u8; 32];
    hasher.update(&unsigned_tx);
    hasher.finalize(&mut hash);

    let (signature, recovery_id) = signing_key
        .sign_prehash_recoverable(&hash)
        .map_err(|e| format!("Signing failed: {}", e))?;

    let sig_bytes = signature.to_bytes();
    let r = &sig_bytes[..32];
    let s = &sig_bytes[32..];

    let v = chain_id * 2 + 35 + recovery_id.to_byte() as u64;

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

fn encode_rlp_uint(value: u128) -> Vec<u8> {
    if value == 0 {
        return vec![];
    }
    let bytes = value.to_be_bytes();
    let start = bytes.iter().position(|&b| b != 0).unwrap_or(bytes.len());
    bytes[start..].to_vec()
}

fn encode_rlp_list(items: &[Vec<u8>]) -> Vec<u8> {
    let mut payload = Vec::new();
    for item in items {
        if item.is_empty() {
            payload.push(0x80);
        } else if item.len() == 1 && item[0] < 0x80 {
            payload.push(item[0]);
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

fn get_address_from_private_key(private_key: &str) -> Result<String, String> {
    let key_bytes = hex::decode(private_key.trim_start_matches("0x"))
        .map_err(|e| format!("Invalid private key: {}", e))?;
    let signing_key = SigningKey::from_slice(&key_bytes)
        .map_err(|e| format!("Invalid key: {}", e))?;
    let verifying_key = signing_key.verifying_key();
    let public_key = verifying_key.to_encoded_point(false);
    let public_key_bytes = &public_key.as_bytes()[1..];

    let mut hasher = Keccak::v256();
    let mut address_hash = [0u8; 32];
    hasher.update(public_key_bytes);
    hasher.finalize(&mut address_hash);
    Ok(format!("0x{}", hex::encode(&address_hash[12..])))
}

// Send transaction on specific network
pub async fn send_transaction_on_network(
    network: Network,
    private_key: &str,
    to: &str,
    value: u128,
    data: Vec<u8>,
    gas_limit: u64,
) -> Result<String, String> {
    let from_address = get_address_from_private_key(private_key)?;

    let nonce = get_nonce_on_network(network, &from_address).await?;
    let gas_price = get_gas_price_on_network(network).await?;
    let chain_id = network.chain_id();

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

    send_raw_transaction_on_network(network, &signed_tx).await
}

pub async fn fetch_real_balances(address: &str) -> (u128, u128) {
    fetch_mainnet_balances(address).await
}

pub async fn get_gas_price() -> Result<u128, String> {
    get_gas_price_on_network(Network::Mainnet).await
}

pub async fn send_transaction(
    private_key: &str,
    to: &str,
    value: u128,
    data: Vec<u8>,
) -> Result<String, String> {
    send_transaction_on_network(Network::Mainnet, private_key, to, value, data, 150000).await
}

// Sepolia-specific transaction helper for staking
pub async fn send_transaction_sepolia(
    private_key: &str,
    to: &str,
    value: u128,
    data: Vec<u8>,
    gas_limit: u64,
) -> Result<String, String> {
    send_transaction_on_network(Network::Sepolia, private_key, to, value, data, gas_limit).await
}
