use nonos_types::{CircuitId, NonosError, NonosResult, TabId, TabInfo};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

pub struct TabManager {
    tabs: Arc<RwLock<HashMap<TabId, Tab>>>,
    active: Arc<RwLock<Option<TabId>>>,
    order: Arc<RwLock<Vec<TabId>>>,
}

struct Tab {
    info: TabInfo,
    history: Vec<String>,
    history_position: usize,
    can_go_back: bool,
    can_go_forward: bool,
}

impl TabManager {
    pub fn new() -> Self {
        Self {
            tabs: Arc::new(RwLock::new(HashMap::new())),
            active: Arc::new(RwLock::new(None)),
            order: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn create_tab(&self, url: &str) -> NonosResult<TabId> {
        let id = TabId::new();

        let info = TabInfo {
            id,
            url: url.to_string(),
            title: "New Tab".to_string(),
            favicon: None,
            loading: true,
            secure: url.starts_with("https://"),
            circuit_id: None,
        };

        let tab = Tab {
            info,
            history: vec![url.to_string()],
            history_position: 0,
            can_go_back: false,
            can_go_forward: false,
        };

        self.tabs.write().await.insert(id, tab);
        self.order.write().await.push(id);

        if self.active.read().await.is_none() {
            *self.active.write().await = Some(id);
        }

        debug!("Created tab {} with URL: {}", id, url);
        Ok(id)
    }

    pub async fn close_tab(&self, id: TabId) -> NonosResult<()> {
        self.tabs.write().await.remove(&id);
        self.order.write().await.retain(|&t| t != id);

        let mut active = self.active.write().await;
        if *active == Some(id) {
            *active = self.order.read().await.first().copied();
        }

        debug!("Closed tab {}", id);
        Ok(())
    }

    pub async fn close_all(&self) {
        self.tabs.write().await.clear();
        self.order.write().await.clear();
        *self.active.write().await = None;
        debug!("Closed all tabs");
    }

    pub async fn get_tab(&self, id: TabId) -> Option<TabInfo> {
        self.tabs.read().await.get(&id).map(|t| t.info.clone())
    }

    pub async fn active_tab(&self) -> Option<TabInfo> {
        let active = *self.active.read().await;
        if let Some(id) = active {
            self.get_tab(id).await
        } else {
            None
        }
    }

    pub async fn set_active(&self, id: TabId) -> NonosResult<()> {
        if !self.tabs.read().await.contains_key(&id) {
            return Err(NonosError::Internal("Tab not found".into()));
        }
        *self.active.write().await = Some(id);
        debug!("Set active tab to {}", id);
        Ok(())
    }

    pub async fn all_tabs(&self) -> Vec<TabInfo> {
        let tabs = self.tabs.read().await;
        let order = self.order.read().await;

        order
            .iter()
            .filter_map(|id| tabs.get(id).map(|t| t.info.clone()))
            .collect()
    }

    pub async fn count(&self) -> usize {
        self.tabs.read().await.len()
    }

    pub async fn navigate(&self, id: TabId, url: &str) -> NonosResult<()> {
        let mut tabs = self.tabs.write().await;
        let tab = tabs
            .get_mut(&id)
            .ok_or_else(|| NonosError::Internal("Tab not found".into()))?;

        tab.info.url = url.to_string();
        tab.info.loading = true;
        tab.info.secure = url.starts_with("https://");

        if tab.history_position < tab.history.len() - 1 {
            tab.history.truncate(tab.history_position + 1);
        }
        tab.history.push(url.to_string());
        tab.history_position = tab.history.len() - 1;

        tab.can_go_back = tab.history_position > 0;
        tab.can_go_forward = false;

        debug!("Tab {} navigated to: {}", id, url);
        Ok(())
    }

    pub async fn back(&self, id: TabId) -> NonosResult<()> {
        let mut tabs = self.tabs.write().await;
        let tab = tabs
            .get_mut(&id)
            .ok_or_else(|| NonosError::Internal("Tab not found".into()))?;

        if !tab.can_go_back {
            return Err(NonosError::Internal("Cannot go back".into()));
        }

        tab.history_position -= 1;
        tab.info.url = tab.history[tab.history_position].clone();
        tab.info.loading = true;
        tab.can_go_back = tab.history_position > 0;
        tab.can_go_forward = true;

        debug!("Tab {} went back to: {}", id, tab.info.url);
        Ok(())
    }

    pub async fn forward(&self, id: TabId) -> NonosResult<()> {
        let mut tabs = self.tabs.write().await;
        let tab = tabs
            .get_mut(&id)
            .ok_or_else(|| NonosError::Internal("Tab not found".into()))?;

        if !tab.can_go_forward {
            return Err(NonosError::Internal("Cannot go forward".into()));
        }

        tab.history_position += 1;
        tab.info.url = tab.history[tab.history_position].clone();
        tab.info.loading = true;
        tab.can_go_back = true;
        tab.can_go_forward = tab.history_position < tab.history.len() - 1;

        debug!("Tab {} went forward to: {}", id, tab.info.url);
        Ok(())
    }

    pub async fn reload(&self, id: TabId) -> NonosResult<()> {
        let mut tabs = self.tabs.write().await;
        let tab = tabs
            .get_mut(&id)
            .ok_or_else(|| NonosError::Internal("Tab not found".into()))?;

        tab.info.loading = true;
        debug!("Tab {} reloading", id);
        Ok(())
    }

    pub async fn update_tab(&self, id: TabId, title: Option<String>, favicon: Option<String>) {
        let mut tabs = self.tabs.write().await;
        if let Some(tab) = tabs.get_mut(&id) {
            if let Some(title) = title {
                tab.info.title = title;
            }
            if let Some(favicon) = favicon {
                tab.info.favicon = Some(favicon);
            }
        }
    }

    pub async fn set_loading(&self, id: TabId, loading: bool) {
        let mut tabs = self.tabs.write().await;
        if let Some(tab) = tabs.get_mut(&id) {
            tab.info.loading = loading;
        }
    }

    pub async fn set_circuit(&self, id: TabId, circuit_id: CircuitId) {
        let mut tabs = self.tabs.write().await;
        if let Some(tab) = tabs.get_mut(&id) {
            tab.info.circuit_id = Some(circuit_id);
        }
    }
}

impl Default for TabManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tab_creation() {
        let manager = TabManager::new();

        let id = manager.create_tab("https://example.com").await.unwrap();
        let tab = manager.get_tab(id).await.unwrap();

        assert_eq!(tab.url, "https://example.com");
        assert!(tab.secure);
    }

    #[tokio::test]
    async fn test_tab_navigation() {
        let manager = TabManager::new();

        let id = manager.create_tab("https://page1.com").await.unwrap();
        manager.navigate(id, "https://page2.com").await.unwrap();
        manager.navigate(id, "https://page3.com").await.unwrap();

        let tab = manager.get_tab(id).await.unwrap();
        assert_eq!(tab.url, "https://page3.com");

        manager.back(id).await.unwrap();
        let tab = manager.get_tab(id).await.unwrap();
        assert_eq!(tab.url, "https://page2.com");

        manager.forward(id).await.unwrap();
        let tab = manager.get_tab(id).await.unwrap();
        assert_eq!(tab.url, "https://page3.com");
    }

    #[tokio::test]
    async fn test_active_tab() {
        let manager = TabManager::new();

        let id1 = manager.create_tab("https://page1.com").await.unwrap();
        let id2 = manager.create_tab("https://page2.com").await.unwrap();

        let active = manager.active_tab().await.unwrap();
        assert_eq!(active.id, id1);

        manager.set_active(id2).await.unwrap();
        let active = manager.active_tab().await.unwrap();
        assert_eq!(active.id, id2);
    }

    #[tokio::test]
    async fn test_close_tab() {
        let manager = TabManager::new();

        let id1 = manager.create_tab("https://page1.com").await.unwrap();
        let id2 = manager.create_tab("https://page2.com").await.unwrap();

        assert_eq!(manager.count().await, 2);

        manager.close_tab(id1).await.unwrap();
        assert_eq!(manager.count().await, 1);

        let active = manager.active_tab().await.unwrap();
        assert_eq!(active.id, id2);
    }
}
