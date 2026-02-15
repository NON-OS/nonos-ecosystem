use super::super::config::get_bootstrap_nodes_for_mode;
use super::network::P2pNetwork;
use crate::p2p::types::NetworkCommand;
use nonos_types::{NonosError, NonosResult};
use tracing::warn;

impl P2pNetwork {
    pub(crate) async fn bootstrap(&self) -> NonosResult<()> {
        if let Some(tx) = &self.command_tx {
            tx.send(NetworkCommand::Bootstrap)
                .await
                .map_err(|e| NonosError::Network(format!("Failed to send bootstrap command: {}", e)))?;
        }
        Ok(())
    }

    pub fn get_bootstrap_nodes(&self) -> Vec<String> {
        get_bootstrap_nodes_for_mode(&self.bootstrap_mode, &self.custom_bootstrap_peers)
    }

    pub async fn bootstrap_with_retry(&self) -> NonosResult<()> {
        loop {
            match self.bootstrap().await {
                Ok(_) => {
                    self.bootstrap_backoff.write().reset();
                    return Ok(());
                }
                Err(e) => {
                    let delay = {
                        let mut backoff = self.bootstrap_backoff.write();
                        backoff.next_delay()
                    };
                    if let Some(delay) = delay {
                        warn!("Bootstrap failed: {}. Retrying in {:?}", e, delay);
                        tokio::time::sleep(delay).await;
                    } else {
                        let attempts = self.bootstrap_backoff.read().attempts();
                        return Err(NonosError::Network(format!(
                            "Bootstrap failed after {} attempts: {}",
                            attempts, e
                        )));
                    }
                }
            }
        }
    }

    pub fn bootstrap_backoff_state(&self) -> (u32, bool) {
        let backoff = self.bootstrap_backoff.read();
        (backoff.attempts(), backoff.is_exhausted())
    }
}
