use super::network::P2pNetwork;
use crate::p2p::types::NetworkCommand;
use nonos_types::{NonosError, NonosResult};

impl P2pNetwork {
    pub async fn set_rate_limit(
        &self,
        messages_per_sec: u32,
        bytes_per_sec: u64,
    ) -> NonosResult<()> {
        if let Some(tx) = &self.command_tx {
            tx.send(NetworkCommand::SetRateLimit {
                messages_per_sec,
                bytes_per_sec,
            })
            .await
            .map_err(|e| NonosError::Network(format!("Failed to set rate limit: {}", e)))?;
        }
        Ok(())
    }
}
