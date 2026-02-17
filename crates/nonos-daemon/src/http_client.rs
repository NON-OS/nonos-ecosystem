use nonos_types::{NonosError, NonosResult};
use reqwest::{Client, Proxy};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info};

pub struct ProxiedHttpClient {
    client: Arc<RwLock<Option<Client>>>,
    socks_addr: Arc<RwLock<Option<String>>>,
    direct_client: Client,
}

impl ProxiedHttpClient {
    pub fn new() -> Self {
        let direct_client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build direct HTTP client");

        Self {
            client: Arc::new(RwLock::new(None)),
            socks_addr: Arc::new(RwLock::new(None)),
            direct_client,
        }
    }

    pub async fn configure_proxy(&self, socks_port: u16) -> NonosResult<()> {
        let proxy_addr = format!("socks5h://127.0.0.1:{}", socks_port);
        info!("Configuring HTTP client with SOCKS5 proxy: {}", proxy_addr);

        let proxy = Proxy::all(&proxy_addr)
            .map_err(|e| NonosError::Network(format!("Failed to create proxy: {}", e)))?;

        let client = Client::builder()
            .proxy(proxy)
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| NonosError::Network(format!("Failed to build proxied client: {}", e)))?;

        *self.client.write().await = Some(client);
        *self.socks_addr.write().await = Some(proxy_addr);

        info!("HTTP client configured with SOCKS5 proxy on port {}", socks_port);
        Ok(())
    }

    pub async fn disable_proxy(&self) {
        *self.client.write().await = None;
        *self.socks_addr.write().await = None;
        info!("HTTP client proxy disabled");
    }

    pub async fn is_proxy_configured(&self) -> bool {
        self.client.read().await.is_some()
    }

    pub async fn get(&self, url: &str) -> NonosResult<reqwest::Response> {
        let client = self.get_client().await;
        debug!("HTTP GET: {}", url);

        client
            .get(url)
            .send()
            .await
            .map_err(|e| NonosError::Network(format!("HTTP GET failed: {}", e)))
    }

    pub async fn post<T: serde::Serialize + ?Sized>(
        &self,
        url: &str,
        body: &T,
    ) -> NonosResult<reqwest::Response> {
        let client = self.get_client().await;
        debug!("HTTP POST: {}", url);

        client
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(|e| NonosError::Network(format!("HTTP POST failed: {}", e)))
    }

    pub async fn post_raw(
        &self,
        url: &str,
        body: String,
        content_type: &str,
    ) -> NonosResult<reqwest::Response> {
        let client = self.get_client().await;
        debug!("HTTP POST (raw): {}", url);

        client
            .post(url)
            .header("Content-Type", content_type)
            .body(body)
            .send()
            .await
            .map_err(|e| NonosError::Network(format!("HTTP POST failed: {}", e)))
    }

    async fn get_client(&self) -> Client {
        if let Some(ref client) = *self.client.read().await {
            client.clone()
        } else {
            self.direct_client.clone()
        }
    }

    pub fn direct_client(&self) -> &Client {
        &self.direct_client
    }
}

impl Default for ProxiedHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ProxiedHttpClient {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            socks_addr: self.socks_addr.clone(),
            direct_client: self.direct_client.clone(),
        }
    }
}

pub struct RpcClient {
    http: ProxiedHttpClient,
    url: String,
}

impl RpcClient {
    pub fn new(http: ProxiedHttpClient, url: String) -> Self {
        Self { http, url }
    }

    pub async fn call<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> NonosResult<T> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1
        });

        let response = self
            .http
            .post_raw(&self.url, request.to_string(), "application/json")
            .await?;

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| NonosError::Network(format!("Failed to parse RPC response: {}", e)))?;

        if let Some(error) = result.get("error") {
            return Err(NonosError::Network(format!("RPC error: {}", error)));
        }

        serde_json::from_value(result["result"].clone())
            .map_err(|e| NonosError::Network(format!("Failed to parse RPC result: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = ProxiedHttpClient::new();
        assert!(!client.is_proxy_configured().await);
    }

    #[tokio::test]
    async fn test_proxy_configuration() {
        let client = ProxiedHttpClient::new();
        assert!(!client.is_proxy_configured().await);
    }
}
