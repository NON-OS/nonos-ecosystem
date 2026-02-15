use super::network::P2pNetwork;
use crate::p2p::types::NetworkCommand;
use nonos_types::{NonosError, NonosResult};
use std::sync::atomic::Ordering;

impl P2pNetwork {
    pub async fn broadcast(&self, topic: &str, message: &[u8]) -> NonosResult<()> {
        if message.len() > self.config.max_message_size {
            return Err(NonosError::Network(format!(
                "Message too large: {} bytes (max: {})",
                message.len(),
                self.config.max_message_size
            )));
        }

        if let Some(tx) = &self.command_tx {
            tx.send(NetworkCommand::Publish {
                topic: topic.to_string(),
                data: message.to_vec(),
            })
            .await
            .map_err(|e| NonosError::Network(format!("Failed to broadcast: {}", e)))?;

            self.stats
                .messages_published
                .fetch_add(1, Ordering::Relaxed);
            self.stats
                .bytes_sent
                .fetch_add(message.len() as u64, Ordering::Relaxed);
        }
        Ok(())
    }

    pub async fn publish(&self, topic: &str, message: &[u8]) -> NonosResult<()> {
        self.broadcast(topic, message).await
    }
}
