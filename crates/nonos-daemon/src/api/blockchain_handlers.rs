use super::handlers::{send_error_response, send_response};
use crate::services::BlockchainService;
use nonos_types::NonosResult;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::RwLock;

#[derive(Debug, Deserialize)]
pub struct EthCallRequest {
    pub to: String,
    pub data: String,
}

#[derive(Debug, Deserialize)]
pub struct AddressRequest {
    pub address: String,
}

#[derive(Debug, Deserialize)]
pub struct TokenBalanceRequest {
    pub token: String,
    pub address: String,
}

#[derive(Debug, Deserialize)]
pub struct EstimateGasRequest {
    pub from: String,
    pub to: String,
    pub data: String,
}

#[derive(Debug, Deserialize)]
pub struct SendTxRequest {
    pub signed_tx: String,
}

#[derive(Debug, Serialize)]
pub struct BalanceResponse {
    pub balance: String,
    pub balance_wei: String,
}

#[derive(Debug, Serialize)]
pub struct BalancesResponse {
    pub eth: String,
    pub eth_wei: String,
    pub nox: String,
    pub nox_wei: String,
}

#[derive(Debug, Serialize)]
pub struct NonceResponse {
    pub nonce: u64,
}

#[derive(Debug, Serialize)]
pub struct GasPriceResponse {
    pub gas_price: String,
    pub gas_price_wei: String,
}

#[derive(Debug, Serialize)]
pub struct GasEstimateResponse {
    pub gas_limit: u64,
}

#[derive(Debug, Serialize)]
pub struct TxHashResponse {
    pub tx_hash: String,
}

#[derive(Debug, Serialize)]
pub struct CallResultResponse {
    pub result: String,
}

fn wei_to_eth(wei: u128) -> String {
    let eth = wei as f64 / 1e18;
    format!("{:.6}", eth)
}

pub async fn blockchain_eth_call(
    stream: &mut TcpStream,
    blockchain: &Option<Arc<RwLock<BlockchainService>>>,
    body: &str,
) -> NonosResult<()> {
    let Some(bc) = blockchain else {
        return send_error_response(stream, 503, "SERVICE_UNAVAILABLE", "Blockchain service not available").await;
    };

    let request: EthCallRequest = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(e) => {
            return send_error_response(stream, 400, "BAD_REQUEST", &format!("Invalid request: {}", e)).await;
        }
    };

    let bc = bc.read().await;
    match bc.eth_call(&request.to, &request.data).await {
        Ok(result) => {
            let response = CallResultResponse { result };
            let json = serde_json::to_string(&response).unwrap();
            send_response(stream, 200, "application/json", &json).await
        }
        Err(e) => {
            send_error_response(stream, 502, "RPC_ERROR", &format!("{}", e)).await
        }
    }
}

pub async fn blockchain_get_balance(
    stream: &mut TcpStream,
    blockchain: &Option<Arc<RwLock<BlockchainService>>>,
    body: &str,
) -> NonosResult<()> {
    let Some(bc) = blockchain else {
        return send_error_response(stream, 503, "SERVICE_UNAVAILABLE", "Blockchain service not available").await;
    };

    let request: AddressRequest = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(e) => {
            return send_error_response(stream, 400, "BAD_REQUEST", &format!("Invalid request: {}", e)).await;
        }
    };

    let bc = bc.read().await;
    match bc.get_eth_balance(&request.address).await {
        Ok(balance) => {
            let response = BalanceResponse {
                balance: wei_to_eth(balance),
                balance_wei: balance.to_string(),
            };
            let json = serde_json::to_string(&response).unwrap();
            send_response(stream, 200, "application/json", &json).await
        }
        Err(e) => {
            send_error_response(stream, 502, "RPC_ERROR", &format!("{}", e)).await
        }
    }
}

pub async fn blockchain_get_balances(
    stream: &mut TcpStream,
    blockchain: &Option<Arc<RwLock<BlockchainService>>>,
    body: &str,
) -> NonosResult<()> {
    let Some(bc) = blockchain else {
        return send_error_response(stream, 503, "SERVICE_UNAVAILABLE", "Blockchain service not available").await;
    };

    let request: AddressRequest = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(e) => {
            return send_error_response(stream, 400, "BAD_REQUEST", &format!("Invalid request: {}", e)).await;
        }
    };

    let bc = bc.read().await;
    match bc.get_balances(&request.address).await {
        Ok((eth, nox)) => {
            let response = BalancesResponse {
                eth: wei_to_eth(eth),
                eth_wei: eth.to_string(),
                nox: wei_to_eth(nox),
                nox_wei: nox.to_string(),
            };
            let json = serde_json::to_string(&response).unwrap();
            send_response(stream, 200, "application/json", &json).await
        }
        Err(e) => {
            send_error_response(stream, 502, "RPC_ERROR", &format!("{}", e)).await
        }
    }
}

pub async fn blockchain_get_token_balance(
    stream: &mut TcpStream,
    blockchain: &Option<Arc<RwLock<BlockchainService>>>,
    body: &str,
) -> NonosResult<()> {
    let Some(bc) = blockchain else {
        return send_error_response(stream, 503, "SERVICE_UNAVAILABLE", "Blockchain service not available").await;
    };

    let request: TokenBalanceRequest = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(e) => {
            return send_error_response(stream, 400, "BAD_REQUEST", &format!("Invalid request: {}", e)).await;
        }
    };

    let bc = bc.read().await;
    match bc.get_token_balance(&request.token, &request.address).await {
        Ok(balance) => {
            let response = BalanceResponse {
                balance: wei_to_eth(balance),
                balance_wei: balance.to_string(),
            };
            let json = serde_json::to_string(&response).unwrap();
            send_response(stream, 200, "application/json", &json).await
        }
        Err(e) => {
            send_error_response(stream, 502, "RPC_ERROR", &format!("{}", e)).await
        }
    }
}

pub async fn blockchain_get_nonce(
    stream: &mut TcpStream,
    blockchain: &Option<Arc<RwLock<BlockchainService>>>,
    body: &str,
) -> NonosResult<()> {
    let Some(bc) = blockchain else {
        return send_error_response(stream, 503, "SERVICE_UNAVAILABLE", "Blockchain service not available").await;
    };

    let request: AddressRequest = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(e) => {
            return send_error_response(stream, 400, "BAD_REQUEST", &format!("Invalid request: {}", e)).await;
        }
    };

    let bc = bc.read().await;
    match bc.get_nonce(&request.address).await {
        Ok(nonce) => {
            let response = NonceResponse { nonce };
            let json = serde_json::to_string(&response).unwrap();
            send_response(stream, 200, "application/json", &json).await
        }
        Err(e) => {
            send_error_response(stream, 502, "RPC_ERROR", &format!("{}", e)).await
        }
    }
}

pub async fn blockchain_get_gas_price(
    stream: &mut TcpStream,
    blockchain: &Option<Arc<RwLock<BlockchainService>>>,
) -> NonosResult<()> {
    let Some(bc) = blockchain else {
        return send_error_response(stream, 503, "SERVICE_UNAVAILABLE", "Blockchain service not available").await;
    };

    let bc = bc.read().await;
    match bc.get_gas_price().await {
        Ok(price) => {
            let gwei = price as f64 / 1e9;
            let response = GasPriceResponse {
                gas_price: format!("{:.2} gwei", gwei),
                gas_price_wei: price.to_string(),
            };
            let json = serde_json::to_string(&response).unwrap();
            send_response(stream, 200, "application/json", &json).await
        }
        Err(e) => {
            send_error_response(stream, 502, "RPC_ERROR", &format!("{}", e)).await
        }
    }
}

pub async fn blockchain_estimate_gas(
    stream: &mut TcpStream,
    blockchain: &Option<Arc<RwLock<BlockchainService>>>,
    body: &str,
) -> NonosResult<()> {
    let Some(bc) = blockchain else {
        return send_error_response(stream, 503, "SERVICE_UNAVAILABLE", "Blockchain service not available").await;
    };

    let request: EstimateGasRequest = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(e) => {
            return send_error_response(stream, 400, "BAD_REQUEST", &format!("Invalid request: {}", e)).await;
        }
    };

    let bc = bc.read().await;
    match bc.estimate_gas(&request.from, &request.to, &request.data).await {
        Ok(gas) => {
            let response = GasEstimateResponse { gas_limit: gas };
            let json = serde_json::to_string(&response).unwrap();
            send_response(stream, 200, "application/json", &json).await
        }
        Err(e) => {
            send_error_response(stream, 502, "RPC_ERROR", &format!("{}", e)).await
        }
    }
}

pub async fn blockchain_send_tx(
    stream: &mut TcpStream,
    blockchain: &Option<Arc<RwLock<BlockchainService>>>,
    body: &str,
) -> NonosResult<()> {
    let Some(bc) = blockchain else {
        return send_error_response(stream, 503, "SERVICE_UNAVAILABLE", "Blockchain service not available").await;
    };

    let request: SendTxRequest = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(e) => {
            return send_error_response(stream, 400, "BAD_REQUEST", &format!("Invalid request: {}", e)).await;
        }
    };

    let bc = bc.read().await;
    match bc.send_raw_transaction(&request.signed_tx).await {
        Ok(tx_hash) => {
            let response = TxHashResponse { tx_hash };
            let json = serde_json::to_string(&response).unwrap();
            send_response(stream, 200, "application/json", &json).await
        }
        Err(e) => {
            send_error_response(stream, 502, "TX_FAILED", &format!("{}", e)).await
        }
    }
}
