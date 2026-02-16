use k256::ecdsa::SigningKey;
use tiny_keccak::{Hasher, Keccak};

pub const RPC_ENDPOINTS: &[&str] = &[
    "https://ethereum.publicnode.com",
    "https://1rpc.io/eth",
    "https://eth.merkle.io",
    "https://rpc.payload.de",
];

pub const NOX_TOKEN_ADDRESS: &str = "0x0a26c80Be4E060e688d7C23aDdB92cBb5D2C9eCA";
pub const NOX_STAKING_ADDRESS: &str = "0x0000000000000000000000000000000000000000";
pub const BALANCE_OF_SELECTOR: &str = "70a08231";

pub async fn eth_call(to: &str, data: &str) -> Result<String, String> {
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

    Err("All RPC endpoints failed".to_string())
}

pub async fn get_eth_balance(address: &str) -> Result<u128, String> {
    let client = reqwest::Client::new();

    for endpoint in RPC_ENDPOINTS {
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

    Err("Failed to get ETH balance".to_string())
}

pub async fn get_token_balance(token_address: &str, holder_address: &str) -> Result<u128, String> {
    let addr = holder_address.trim_start_matches("0x").to_lowercase();
    let padded_addr = format!("{:0>64}", addr);
    let data = format!("0x{}{}", BALANCE_OF_SELECTOR, padded_addr);

    let result = eth_call(token_address, &data).await?;

    let hex = result.trim_start_matches("0x");
    if hex.is_empty() || hex.chars().all(|c| c == '0') {
        return Ok(0);
    }

    u128::from_str_radix(hex, 16).map_err(|e| format!("Parse error for '{}': {}", hex, e))
}

pub async fn fetch_real_balances(address: &str) -> (u128, u128) {
    let eth_balance = get_eth_balance(address).await.unwrap_or(0);
    let nox_balance = get_token_balance(NOX_TOKEN_ADDRESS, address).await.unwrap_or(0);
    (eth_balance, nox_balance)
}

pub async fn get_nonce(address: &str) -> Result<u64, String> {
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

pub async fn get_gas_price() -> Result<u128, String> {
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

pub async fn estimate_gas(from: &str, to: &str, data: &str) -> Result<u64, String> {
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
                        let gas = u64::from_str_radix(hex, 16)
                            .map_err(|e| format!("Parse error: {}", e))?;
                        return Ok(gas + (gas / 5));
                    }
                    if let Some(error) = json.get("error") {
                        return Err(format!("Gas estimation failed: {}", error));
                    }
                }
            }
            Err(_) => continue,
        }
    }

    Err("Failed to estimate gas".to_string())
}

pub async fn send_raw_transaction(signed_tx: &str) -> Result<String, String> {
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

    Err("Failed to broadcast transaction".to_string())
}

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

pub async fn send_transaction(
    private_key: &str,
    to: &str,
    value: u128,
    data: Vec<u8>,
) -> Result<String, String> {
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
    let from_address = format!("0x{}", hex::encode(&address_hash[12..]));

    let nonce = get_nonce(&from_address).await?;
    let gas_price = get_gas_price().await?;
    let gas_limit = if data.is_empty() { 21000u64 } else { 150000u64 };
    let chain_id = 1u64;

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

    send_raw_transaction(&signed_tx).await
}

pub async fn send_transaction_with_gas(
    private_key: &str,
    to: &str,
    value: u128,
    data: Vec<u8>,
    gas_limit: u64,
) -> Result<String, String> {
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
    let from_address = format!("0x{}", hex::encode(&address_hash[12..]));

    let nonce = get_nonce(&from_address).await?;
    let gas_price = get_gas_price().await?;
    let chain_id = 1u64;

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

    send_raw_transaction(&signed_tx).await
}
