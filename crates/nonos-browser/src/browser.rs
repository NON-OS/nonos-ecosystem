use crate::{SecurityManager, TabManager};
use nonos_anyone::AnyoneClient;
use nonos_wallet::Wallet;
use nonos_types::{
    ConnectionStatus, NetworkStatus, NonosResult, SecurityLevel, TabId, TabInfo,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserState {
    Initializing,
    Ready,
    ShuttingDown,
    Stopped,
}

pub struct Browser {
    state: Arc<RwLock<BrowserState>>,
    tabs: Arc<TabManager>,
    network: Arc<AnyoneClient>,
    security: Arc<SecurityManager>,
    wallet: Arc<RwLock<Option<Wallet>>>,
}

impl Browser {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(BrowserState::Initializing)),
            tabs: Arc::new(TabManager::new()),
            network: Arc::new(AnyoneClient::new()),
            security: Arc::new(SecurityManager::new()),
            wallet: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn initialize(&self) -> NonosResult<()> {
        info!("Initializing NONOS browser");

        self.network.start().await?;

        let mut attempts = 0;
        while !self.network.is_connected().await && attempts < 30 {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            attempts += 1;
        }

        if !self.network.is_connected().await {
            warn!("Failed to connect to Anyone network");
        }

        *self.state.write().await = BrowserState::Ready;
        info!("NONOS browser initialized");

        Ok(())
    }

    pub async fn shutdown(&self) -> NonosResult<()> {
        info!("Shutting down NONOS browser");
        *self.state.write().await = BrowserState::ShuttingDown;

        self.tabs.close_all().await;

        self.network.stop().await?;

        if let Some(ref mut wallet) = *self.wallet.write().await {
            wallet.lock();
        }

        *self.state.write().await = BrowserState::Stopped;
        info!("NONOS browser shutdown complete");

        Ok(())
    }

    pub async fn state(&self) -> BrowserState {
        *self.state.read().await
    }

    pub async fn is_ready(&self) -> bool {
        *self.state.read().await == BrowserState::Ready
    }

    pub async fn network_status(&self) -> NetworkStatus {
        self.network.network_status().await
    }

    pub async fn connection_status(&self) -> ConnectionStatus {
        self.network.circuits().status().await
    }

    pub async fn new_identity(&self) -> NonosResult<()> {
        self.network.new_identity().await
    }

    pub fn proxy_address(&self) -> String {
        self.network.socks_address()
    }

    pub async fn new_tab(&self, url: Option<&str>) -> NonosResult<TabId> {
        let url = url.unwrap_or("about:blank");
        self.tabs.create_tab(url).await
    }

    pub async fn close_tab(&self, id: TabId) -> NonosResult<()> {
        self.tabs.close_tab(id).await
    }

    pub async fn active_tab(&self) -> Option<TabInfo> {
        self.tabs.active_tab().await
    }

    pub async fn set_active_tab(&self, id: TabId) -> NonosResult<()> {
        self.tabs.set_active(id).await
    }

    pub async fn tabs(&self) -> Vec<TabInfo> {
        self.tabs.all_tabs().await
    }

    pub async fn navigate(&self, id: TabId, url: &str) -> NonosResult<()> {
        self.tabs.navigate(id, url).await
    }

    pub async fn back(&self, id: TabId) -> NonosResult<()> {
        self.tabs.back(id).await
    }

    pub async fn forward(&self, id: TabId) -> NonosResult<()> {
        self.tabs.forward(id).await
    }

    pub async fn reload(&self, id: TabId) -> NonosResult<()> {
        self.tabs.reload(id).await
    }

    pub fn security_level(&self) -> SecurityLevel {
        self.security.level()
    }

    pub fn set_security_level(&self, level: SecurityLevel) {
        self.security.set_level(level);
    }

    pub async fn set_wallet(&self, wallet: Wallet) {
        *self.wallet.write().await = Some(wallet);
    }

    pub async fn with_wallet<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&Wallet) -> R,
    {
        self.wallet.read().await.as_ref().map(f)
    }

    pub async fn with_wallet_mut<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&mut Wallet) -> R,
    {
        self.wallet.write().await.as_mut().map(f)
    }

    pub async fn has_wallet(&self) -> bool {
        self.wallet.read().await.is_some()
    }

    pub async fn lock_wallet(&self) {
        if let Some(ref mut wallet) = *self.wallet.write().await {
            wallet.lock();
        }
    }
}

impl Default for Browser {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct BrowserConfig {
    pub home_page: String,
    pub security_level: SecurityLevel,
    pub javascript_enabled: bool,
    pub block_third_party_cookies: bool,
    pub webrtc_enabled: bool,
    pub user_agent: String,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            home_page: "about:blank".to_string(),
            security_level: SecurityLevel::Safer,
            javascript_enabled: true,
            block_third_party_cookies: true,
            webrtc_enabled: false,
            user_agent: "NONOS/1.0".to_string(),
        }
    }
}

impl BrowserConfig {
    pub fn apply_security_level(&mut self, level: SecurityLevel) {
        self.security_level = level;
        match level {
            SecurityLevel::Standard => {
                self.javascript_enabled = true;
                self.webrtc_enabled = false;
            }
            SecurityLevel::Safer => {
                self.javascript_enabled = true;
                self.webrtc_enabled = false;
            }
            SecurityLevel::Safest => {
                self.javascript_enabled = false;
                self.webrtc_enabled = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_browser_creation() {
        let browser = Browser::new();
        assert_eq!(browser.state().await, BrowserState::Initializing);
    }

    #[test]
    fn test_browser_config() {
        let mut config = BrowserConfig::default();
        assert!(config.javascript_enabled);

        config.apply_security_level(SecurityLevel::Safest);
        assert!(!config.javascript_enabled);
    }
}
