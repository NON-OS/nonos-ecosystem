use super::network::P2pNetwork;
use crate::p2p::types::NetworkCommand;
use nonos_types::{NonosError, NonosResult};
use std::sync::atomic::Ordering;
use tracing::debug;

impl P2pNetwork {
    pub async fn subscribe(&self, topic: &str) -> NonosResult<()> {
        if let Some(tx) = &self.command_tx {
            tx.send(NetworkCommand::Subscribe(topic.to_string()))
                .await
                .map_err(|e| NonosError::Network(format!("Failed to subscribe: {}", e)))?;

            self.subscribed_topics.write().push(topic.to_string());
            self.stats.active_topics.fetch_add(1, Ordering::Relaxed);
            debug!("Subscribed to topic: {}", topic);
        }
        Ok(())
    }

    pub async fn unsubscribe(&self, topic: &str) -> NonosResult<()> {
        if let Some(tx) = &self.command_tx {
            tx.send(NetworkCommand::Unsubscribe(topic.to_string()))
                .await
                .map_err(|e| NonosError::Network(format!("Failed to unsubscribe: {}", e)))?;

            self.subscribed_topics.write().retain(|t| t != topic);
            self.stats.active_topics.fetch_sub(1, Ordering::Relaxed);
        }
        Ok(())
    }

    pub fn subscribed_topics(&self) -> Vec<String> {
        self.subscribed_topics.read().clone()
    }
}
