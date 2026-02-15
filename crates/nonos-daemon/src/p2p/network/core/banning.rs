use super::super::helpers::humanize_duration;
use super::network::P2pNetwork;
use crate::p2p::types::{BanEntry, NetworkCommand};
use libp2p::PeerId;
use nonos_types::{NonosError, NonosResult};
use std::sync::atomic::Ordering;
use std::time::Duration;
use tracing::{info, warn};

impl P2pNetwork {
    pub fn is_banned(&self, peer_id: &PeerId) -> bool {
        if let Some(ban) = self.banned_peers.read().get(peer_id) {
            if ban.is_expired() {
                return false;
            }
            return true;
        }
        false
    }

    pub async fn ban_peer(
        &self,
        peer_id: PeerId,
        duration: Duration,
        reason: &str,
    ) -> NonosResult<()> {
        let ban = BanEntry::new(peer_id, duration, reason);

        self.banned_peers.write().insert(peer_id, ban.clone());
        self.stats.banned_peers.fetch_add(1, Ordering::Relaxed);

        if let Some(tx) = &self.command_tx {
            tx.send(NetworkCommand::BanPeer(peer_id, duration))
                .await
                .map_err(|e| NonosError::Network(format!("Failed to send ban command: {}", e)))?;
        }

        self.disconnect(&peer_id.to_string()).await;

        warn!(
            "Banned peer {} for {}: {}",
            peer_id,
            humanize_duration(duration),
            reason
        );
        Ok(())
    }

    pub async fn unban_peer(&self, peer_id: PeerId) -> NonosResult<()> {
        self.banned_peers.write().remove(&peer_id);

        if let Some(tx) = &self.command_tx {
            tx.send(NetworkCommand::UnbanPeer(peer_id))
                .await
                .map_err(|e| NonosError::Network(format!("Failed to send unban command: {}", e)))?;
        }

        info!("Unbanned peer {}", peer_id);
        Ok(())
    }

    pub fn cleanup_expired_bans(&self) {
        let mut banned = self.banned_peers.write();
        banned.retain(|_, ban| !ban.is_expired());
    }

    pub fn banned_peers(&self) -> Vec<BanEntry> {
        self.banned_peers.read().values().cloned().collect()
    }
}
