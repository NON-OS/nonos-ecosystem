use super::handlers::handle_request;
use super::middleware::ApiContext;
use crate::contracts::ContractClient;
use crate::rewards::RewardTracker;
use crate::{Node, NodeMetricsCollector, PrivacyServiceManager};
use nonos_types::{EthAddress, NonosError, NonosResult};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

pub struct ApiServer {
    addr: SocketAddr,
    running: Arc<RwLock<bool>>,
    node: Arc<RwLock<Node>>,
    metrics: Arc<NodeMetricsCollector>,
    privacy: Option<Arc<PrivacyServiceManager>>,
    contract_client: Option<Arc<RwLock<ContractClient>>>,
    reward_tracker: Option<Arc<RewardTracker>>,
    staker_address: Option<EthAddress>,
    api_context: Arc<ApiContext>,
}

impl ApiServer {
    pub fn new(
        addr: SocketAddr,
        node: Arc<RwLock<Node>>,
        metrics: Arc<NodeMetricsCollector>,
        auth_token: Option<String>,
    ) -> Self {
        let api_context = match auth_token {
            Some(token) if !token.is_empty() => {
                Arc::new(ApiContext::new(Some(token), 100, 200))
            }
            _ => {
                let (ctx, token) = ApiContext::with_generated_token(100, 200);
                warn!("No API auth_token configured. Generated random token: {}", token);
                warn!("Add this to your config.toml: auth_token = \"{}\"", token);
                Arc::new(ctx)
            }
        };

        Self {
            addr,
            running: Arc::new(RwLock::new(false)),
            node,
            metrics,
            privacy: None,
            contract_client: None,
            reward_tracker: None,
            staker_address: None,
            api_context,
        }
    }

    pub fn new_insecure_no_auth(
        addr: SocketAddr,
        node: Arc<RwLock<Node>>,
        metrics: Arc<NodeMetricsCollector>,
    ) -> Self {
        warn!("API server created WITHOUT authentication - this is insecure!");
        Self {
            addr,
            running: Arc::new(RwLock::new(false)),
            node,
            metrics,
            privacy: None,
            contract_client: None,
            reward_tracker: None,
            staker_address: None,
            api_context: Arc::new(ApiContext::insecure_without_auth()),
        }
    }

    pub fn with_config(
        addr: SocketAddr,
        node: Arc<RwLock<Node>>,
        metrics: Arc<NodeMetricsCollector>,
        auth_token: Option<String>,
        requests_per_second: u32,
        burst_size: u32,
    ) -> Self {
        Self {
            addr,
            running: Arc::new(RwLock::new(false)),
            node,
            metrics,
            privacy: None,
            contract_client: None,
            reward_tracker: None,
            staker_address: None,
            api_context: Arc::new(ApiContext::new(auth_token, requests_per_second, burst_size)),
        }
    }

    pub fn with_privacy(
        addr: SocketAddr,
        node: Arc<RwLock<Node>>,
        metrics: Arc<NodeMetricsCollector>,
        privacy: Arc<PrivacyServiceManager>,
        auth_token: Option<String>,
        auth_required: bool,
        requests_per_second: u32,
        burst_size: u32,
    ) -> Self {
        let api_context = if auth_required {
            match auth_token {
                Some(token) if !token.is_empty() => {
                    Arc::new(ApiContext::new(Some(token), requests_per_second, burst_size))
                }
                _ => {
                    let (ctx, token) = ApiContext::with_generated_token(requests_per_second, burst_size);
                    warn!("No API auth_token configured but auth required. Generated token: {}", token);
                    warn!("Set NONOS_API_TOKEN={} or add to config.toml", token);
                    Arc::new(ctx)
                }
            }
        } else {
            warn!("API authentication DISABLED - API is open to all requests!");
            Arc::new(ApiContext::insecure_without_auth())
        };

        Self {
            addr,
            running: Arc::new(RwLock::new(false)),
            node,
            metrics,
            privacy: Some(privacy),
            contract_client: None,
            reward_tracker: None,
            staker_address: None,
            api_context,
        }
    }

    #[deprecated(note = "Use full_with_auth to ensure authentication is properly configured")]
    pub fn full(
        addr: SocketAddr,
        node: Arc<RwLock<Node>>,
        metrics: Arc<NodeMetricsCollector>,
        privacy: Arc<PrivacyServiceManager>,
        contract_client: Arc<RwLock<ContractClient>>,
        reward_tracker: Arc<RewardTracker>,
        staker_address: EthAddress,
    ) -> Self {
        warn!("Using deprecated full() constructor without authentication - use full_with_auth()");
        Self {
            addr,
            running: Arc::new(RwLock::new(false)),
            node,
            metrics,
            privacy: Some(privacy),
            contract_client: Some(contract_client),
            reward_tracker: Some(reward_tracker),
            staker_address: Some(staker_address),
            api_context: Arc::new(ApiContext::insecure_without_auth()),
        }
    }

    pub fn full_with_auth(
        addr: SocketAddr,
        node: Arc<RwLock<Node>>,
        metrics: Arc<NodeMetricsCollector>,
        privacy: Arc<PrivacyServiceManager>,
        contract_client: Arc<RwLock<ContractClient>>,
        reward_tracker: Arc<RewardTracker>,
        staker_address: EthAddress,
        auth_token: Option<String>,
        requests_per_second: u32,
        burst_size: u32,
    ) -> Self {
        Self {
            addr,
            running: Arc::new(RwLock::new(false)),
            node,
            metrics,
            privacy: Some(privacy),
            contract_client: Some(contract_client),
            reward_tracker: Some(reward_tracker),
            staker_address: Some(staker_address),
            api_context: Arc::new(ApiContext::new(auth_token, requests_per_second, burst_size)),
        }
    }

    pub fn set_contract_client(
        &mut self,
        client: Arc<RwLock<ContractClient>>,
        staker: EthAddress,
    ) {
        self.contract_client = Some(client);
        self.staker_address = Some(staker);
    }

    pub fn set_reward_tracker(&mut self, tracker: Arc<RewardTracker>) {
        self.reward_tracker = Some(tracker);
    }

    pub fn set_api_context(&mut self, context: ApiContext) {
        self.api_context = Arc::new(context);
    }

    pub async fn start(&self) -> NonosResult<()> {
        if *self.running.read().await {
            return Err(NonosError::Internal("API server already running".into()));
        }

        let listener = tokio::net::TcpListener::bind(self.addr)
            .await
            .map_err(|e| NonosError::Network(format!("Failed to bind API server: {}", e)))?;

        info!("API server listening on http://{}", self.addr);

        if self.api_context.authenticator.authenticate("/api/status", None)
            == super::middleware::AuthResult::MissingToken
        {
            info!("API authentication: ENABLED");
        } else {
            warn!("API authentication: DISABLED - API is open to all requests");
        }

        *self.running.write().await = true;

        let running = self.running.clone();
        let node = self.node.clone();
        let metrics = self.metrics.clone();
        let privacy = self.privacy.clone();
        let contract_client = self.contract_client.clone();
        let reward_tracker = self.reward_tracker.clone();
        let staker_address = self.staker_address;
        let api_context = self.api_context.clone();

        let rate_limiter = api_context.rate_limiter.clone();
        let cleanup_running = running.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                if !*cleanup_running.read().await {
                    break;
                }
                rate_limiter.cleanup();
            }
        });

        tokio::spawn(async move {
            loop {
                if !*running.read().await {
                    break;
                }

                match listener.accept().await {
                    Ok((stream, addr)) => {
                        debug!("API request from {}", addr);
                        let node = node.clone();
                        let metrics = metrics.clone();
                        let privacy = privacy.clone();
                        let contract_client = contract_client.clone();
                        let reward_tracker = reward_tracker.clone();
                        let api_context = api_context.clone();

                        tokio::spawn(async move {
                            if let Err(e) = handle_request(
                                stream,
                                addr,
                                node,
                                metrics,
                                privacy,
                                contract_client,
                                reward_tracker,
                                staker_address,
                                api_context,
                            )
                            .await
                            {
                                if !e.to_string().contains("connection reset")
                                    && !e.to_string().contains("broken pipe")
                                {
                                    warn!("API request error from {}: {}", addr, e);
                                }
                            }
                        });
                    }
                    Err(e) => {
                        error!("API accept error: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn stop(&self) {
        *self.running.write().await = false;
        info!("API server stopped");
    }

    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    pub fn rate_limiter_stats(&self) -> super::middleware::RateLimiterStats {
        self.api_context.rate_limiter.stats()
    }
}
