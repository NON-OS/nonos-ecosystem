use nonos_types::{NonosError, NonosResult};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, warn, info};

#[derive(Clone, Debug)]
pub struct RpcEndpoint {
    pub url: String,
    pub requires_api_key: bool,
    pub healthy: bool,
    pub failures: u32,
    pub last_success: Option<std::time::Instant>,
}

impl RpcEndpoint {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            requires_api_key: url.contains("{api_key}"),
            healthy: true,
            failures: 0,
            last_success: None,
        }
    }

    pub fn with_api_key(url: &str, api_key: &str) -> Self {
        Self {
            url: url.replace("{api_key}", api_key),
            requires_api_key: false,
            healthy: true,
            failures: 0,
            last_success: None,
        }
    }
}

pub const MAINNET_RPC_ENDPOINTS: &[&str] = &[
    "https://ethereum-rpc.publicnode.com",
    "https://eth.llamarpc.com",
    "https://rpc.ankr.com/eth",
    "https://1rpc.io/eth",
    "https://eth.drpc.org",
    "https://ethereum.blockpi.network/v1/rpc/public",
    "https://rpc.payload.de",
    "https://cloudflare-eth.com",
];

pub const SEPOLIA_RPC_ENDPOINTS: &[&str] = &[
    "https://ethereum-sepolia-rpc.publicnode.com",
    "https://rpc.sepolia.org",
    "https://sepolia.drpc.org",
    "https://rpc2.sepolia.org",
    "https://rpc.ankr.com/eth_sepolia",
];

pub const BASE_RPC_ENDPOINTS: &[&str] = &[
    "https://mainnet.base.org",
    "https://base.llamarpc.com",
    "https://1rpc.io/base",
    "https://base.drpc.org",
];

pub const ARBITRUM_RPC_ENDPOINTS: &[&str] = &[
    "https://arb1.arbitrum.io/rpc",
    "https://arbitrum.llamarpc.com",
    "https://1rpc.io/arb",
    "https://arbitrum.drpc.org",
];

const MAX_FAILURES: u32 = 3;
const _UNHEALTHY_RETRY_SECS: u64 = 60;

pub struct RpcProvider {
    endpoints: Arc<RwLock<Vec<RpcEndpoint>>>,
    current: AtomicUsize,
    chain_id: u64,
    timeout: Duration,
}

impl RpcProvider {
    pub fn mainnet() -> Self {
        let endpoints: Vec<RpcEndpoint> = MAINNET_RPC_ENDPOINTS
            .iter()
            .map(|url| RpcEndpoint::new(url))
            .collect();

        Self {
            endpoints: Arc::new(RwLock::new(endpoints)),
            current: AtomicUsize::new(0),
            chain_id: 1,
            timeout: Duration::from_secs(10),
        }
    }

    pub fn sepolia() -> Self {
        let endpoints: Vec<RpcEndpoint> = SEPOLIA_RPC_ENDPOINTS
            .iter()
            .map(|url| RpcEndpoint::new(url))
            .collect();

        Self {
            endpoints: Arc::new(RwLock::new(endpoints)),
            current: AtomicUsize::new(0),
            chain_id: 11155111,
            timeout: Duration::from_secs(10),
        }
    }

    pub fn base() -> Self {
        let endpoints: Vec<RpcEndpoint> = BASE_RPC_ENDPOINTS
            .iter()
            .map(|url| RpcEndpoint::new(url))
            .collect();

        Self {
            endpoints: Arc::new(RwLock::new(endpoints)),
            current: AtomicUsize::new(0),
            chain_id: 8453,
            timeout: Duration::from_secs(10),
        }
    }

    pub fn arbitrum() -> Self {
        let endpoints: Vec<RpcEndpoint> = ARBITRUM_RPC_ENDPOINTS
            .iter()
            .map(|url| RpcEndpoint::new(url))
            .collect();

        Self {
            endpoints: Arc::new(RwLock::new(endpoints)),
            current: AtomicUsize::new(0),
            chain_id: 42161,
            timeout: Duration::from_secs(10),
        }
    }

    pub fn custom(endpoints: Vec<String>, chain_id: u64) -> Self {
        let endpoints: Vec<RpcEndpoint> = endpoints
            .iter()
            .map(|url| RpcEndpoint::new(url))
            .collect();

        Self {
            endpoints: Arc::new(RwLock::new(endpoints)),
            current: AtomicUsize::new(0),
            chain_id,
            timeout: Duration::from_secs(10),
        }
    }

    pub async fn add_endpoint(&self, url: &str) {
        let mut endpoints = self.endpoints.write().await;
        endpoints.insert(0, RpcEndpoint::new(url));
        info!("Added custom RPC endpoint: {}", url);
    }

    pub async fn get_url(&self) -> NonosResult<String> {
        let endpoints = self.endpoints.read().await;
        let len = endpoints.len();

        if len == 0 {
            return Err(NonosError::Network("No RPC endpoints configured".into()));
        }

        let start = self.current.load(Ordering::Relaxed);

        for i in 0..len {
            let idx = (start + i) % len;
            let endpoint = &endpoints[idx];

            if endpoint.healthy {
                if i > 0 {
                    self.current.store(idx, Ordering::Relaxed);
                    debug!("Switched to RPC endpoint: {}", endpoint.url);
                }
                return Ok(endpoint.url.clone());
            }
        }

        warn!("All RPC endpoints marked unhealthy, trying first endpoint");
        Ok(endpoints[0].url.clone())
    }

    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }

    pub async fn report_success(&self) {
        let idx = self.current.load(Ordering::Relaxed);
        let mut endpoints = self.endpoints.write().await;

        if let Some(endpoint) = endpoints.get_mut(idx) {
            endpoint.healthy = true;
            endpoint.failures = 0;
            endpoint.last_success = Some(std::time::Instant::now());
        }
    }

    pub async fn report_failure(&self) {
        let idx = self.current.load(Ordering::Relaxed);
        let mut endpoints = self.endpoints.write().await;
        let len = endpoints.len();

        if let Some(endpoint) = endpoints.get_mut(idx) {
            endpoint.failures += 1;
            warn!(
                "RPC endpoint {} failed ({} consecutive failures)",
                endpoint.url, endpoint.failures
            );

            if endpoint.failures >= MAX_FAILURES {
                endpoint.healthy = false;
                warn!("Marking RPC endpoint {} as unhealthy", endpoint.url);

                for i in 1..len {
                    let next_idx = (idx + i) % len;
                    if endpoints[next_idx].healthy {
                        self.current.store(next_idx, Ordering::Relaxed);
                        info!("Failover to RPC endpoint: {}", endpoints[next_idx].url);
                        break;
                    }
                }
            }
        }
    }

    pub async fn health_check(&self) -> Vec<(String, bool)> {
        let endpoints = self.endpoints.read().await;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        let mut results = Vec::new();

        for endpoint in endpoints.iter() {
            let healthy = Self::check_endpoint(&client, &endpoint.url).await;
            results.push((endpoint.url.clone(), healthy));
        }

        drop(endpoints);
        let mut endpoints = self.endpoints.write().await;

        for (url, healthy) in &results {
            if let Some(endpoint) = endpoints.iter_mut().find(|e| &e.url == url) {
                if *healthy && !endpoint.healthy {
                    info!("RPC endpoint {} is now healthy", url);
                    endpoint.healthy = true;
                    endpoint.failures = 0;
                }
            }
        }

        results
    }

    async fn check_endpoint(client: &reqwest::Client, url: &str) -> bool {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_chainId",
            "params": [],
            "id": 1
        });

        match client.post(url).json(&request).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    pub async fn healthy_count(&self) -> usize {
        let endpoints = self.endpoints.read().await;
        endpoints.iter().filter(|e| e.healthy).count()
    }

    pub async fn all_urls(&self) -> Vec<String> {
        let endpoints = self.endpoints.read().await;
        endpoints.iter().map(|e| e.url.clone()).collect()
    }
}

impl Clone for RpcProvider {
    fn clone(&self) -> Self {
        Self {
            endpoints: self.endpoints.clone(),
            current: AtomicUsize::new(self.current.load(Ordering::Relaxed)),
            chain_id: self.chain_id,
            timeout: self.timeout,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mainnet_endpoints() {
        assert!(!MAINNET_RPC_ENDPOINTS.is_empty());
        assert!(MAINNET_RPC_ENDPOINTS.len() >= 5);
    }

    #[test]
    fn test_provider_creation() {
        let provider = RpcProvider::mainnet();
        assert_eq!(provider.chain_id(), 1);
    }

    #[tokio::test]
    async fn test_get_url() {
        let provider = RpcProvider::mainnet();
        let url = provider.get_url().await.unwrap();
        assert!(url.starts_with("https://"));
    }

    #[tokio::test]
    async fn test_custom_provider() {
        let endpoints = vec![
            "https://my-node.example.com".to_string(),
            "https://backup.example.com".to_string(),
        ];
        let provider = RpcProvider::custom(endpoints, 1);
        let url = provider.get_url().await.unwrap();
        assert_eq!(url, "https://my-node.example.com");
    }
}
