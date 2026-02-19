use crate::http_client::ProxiedHttpClient;
use nonos_types::{NonosError, NonosResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, warn};

const RPC_ENDPOINTS: &[&str] = &[
    "https://ethereum.publicnode.com",
    "https://1rpc.io/eth",
    "https://eth.merkle.io",
    "https://rpc.payload.de",
];

pub const NOX_TOKEN_ADDRESS: &str = "0x0a26c80Be4E060e688d7C23aDdB92cBb5D2C9eCA";
pub const BALANCE_OF_SELECTOR: &str = "70a08231";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: serde_json::Value,
    pub id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    pub jsonrpc: String,
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<serde_json::Value>,
}

pub struct BlockchainService {
    http: Arc<ProxiedHttpClient>,
}

impl BlockchainService {
    pub fn new(http: Arc<ProxiedHttpClient>) -> Self {
        Self { http }
    }

    async fn rpc_call(&self, method: &str, params: serde_json::Value) -> NonosResult<String> {
        if !self.http.is_proxy_configured().await {
            return Err(NonosError::Network(
                "Proxy not configured. All RPC calls must route through Anyone transport.".into(),
            ));
        }

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1
        });

        for endpoint in RPC_ENDPOINTS {
            debug!("RPC call to {}: {} {:?}", endpoint, method, params);

            match self
                .http
                .post_raw(endpoint, request.to_string(), "application/json")
                .await
            {
                Ok(response) => {
                    let json: serde_json::Value = response.json().await.map_err(|e| {
                        NonosError::Network(format!("Failed to parse RPC response: {}", e))
                    })?;

                    if let Some(error) = json.get("error") {
                        warn!("RPC error from {}: {}", endpoint, error);
                        continue;
                    }

                    if let Some(result) = json.get("result") {
                        if result.is_null() {
                            continue;
                        }
                        return Ok(result.as_str().unwrap_or("").to_string());
                    }
                }
                Err(e) => {
                    warn!("RPC request failed to {}: {}", endpoint, e);
                    continue;
                }
            }
        }

        Err(NonosError::Network("All RPC endpoints failed".into()))
    }

    pub async fn eth_call(&self, to: &str, data: &str) -> NonosResult<String> {
        let params = serde_json::json!([
            {
                "to": to,
                "data": data
            },
            "latest"
        ]);
        self.rpc_call("eth_call", params).await
    }

    pub async fn get_eth_balance(&self, address: &str) -> NonosResult<u128> {
        let params = serde_json::json!([address, "latest"]);
        let result = self.rpc_call("eth_getBalance", params).await?;

        let hex = result.trim_start_matches("0x");
        u128::from_str_radix(hex, 16)
            .map_err(|e| NonosError::Internal(format!("Failed to parse balance: {}", e)))
    }

    pub async fn get_token_balance(&self, token_address: &str, holder: &str) -> NonosResult<u128> {
        let addr = holder.trim_start_matches("0x").to_lowercase();
        let padded = format!("{:0>64}", addr);
        let data = format!("0x{}{}", BALANCE_OF_SELECTOR, padded);

        let result = self.eth_call(token_address, &data).await?;

        let hex = result.trim_start_matches("0x");
        if hex.is_empty() || hex.chars().all(|c| c == '0') {
            return Ok(0);
        }

        u128::from_str_radix(hex, 16)
            .map_err(|e| NonosError::Internal(format!("Failed to parse token balance: {}", e)))
    }

    pub async fn get_nox_balance(&self, address: &str) -> NonosResult<u128> {
        self.get_token_balance(NOX_TOKEN_ADDRESS, address).await
    }

    pub async fn get_nonce(&self, address: &str) -> NonosResult<u64> {
        let params = serde_json::json!([address, "pending"]);
        let result = self.rpc_call("eth_getTransactionCount", params).await?;

        let hex = result.trim_start_matches("0x");
        u64::from_str_radix(hex, 16)
            .map_err(|e| NonosError::Internal(format!("Failed to parse nonce: {}", e)))
    }

    pub async fn get_gas_price(&self) -> NonosResult<u128> {
        let params = serde_json::json!([]);
        let result = self.rpc_call("eth_gasPrice", params).await?;

        let hex = result.trim_start_matches("0x");
        u128::from_str_radix(hex, 16)
            .map_err(|e| NonosError::Internal(format!("Failed to parse gas price: {}", e)))
    }

    pub async fn estimate_gas(&self, from: &str, to: &str, data: &str) -> NonosResult<u64> {
        let params = serde_json::json!([{
            "from": from,
            "to": to,
            "data": data
        }]);
        let result = self.rpc_call("eth_estimateGas", params).await?;

        let hex = result.trim_start_matches("0x");
        let gas = u64::from_str_radix(hex, 16)
            .map_err(|e| NonosError::Internal(format!("Failed to parse gas estimate: {}", e)))?;

        // Add 20% buffer
        Ok(gas + (gas / 5))
    }

    pub async fn send_raw_transaction(&self, signed_tx: &str) -> NonosResult<String> {
        let params = serde_json::json!([signed_tx]);
        self.rpc_call("eth_sendRawTransaction", params).await
    }

    pub async fn get_balances(&self, address: &str) -> NonosResult<(u128, u128)> {
        let eth = self.get_eth_balance(address).await.unwrap_or(0);
        let nox = self.get_nox_balance(address).await.unwrap_or(0);
        Ok((eth, nox))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_balance_of_selector() {
        assert_eq!(BALANCE_OF_SELECTOR, "70a08231");
    }

    #[test]
    fn test_nox_token_address() {
        assert!(NOX_TOKEN_ADDRESS.starts_with("0x"));
        assert_eq!(NOX_TOKEN_ADDRESS.len(), 42);
    }
}
