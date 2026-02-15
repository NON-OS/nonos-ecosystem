use super::network::P2pNetwork;
use crate::p2p::peer_store::{PeerEntry, PeerStoreStats, PenaltyReason};
use libp2p::PeerId;
use std::sync::atomic::Ordering;

impl P2pNetwork {
    pub fn record_peer_success(&self, peer_id: &PeerId) {
        if let Some(info) = self.peers.write().get_mut(peer_id) {
            info.record_success();
        }

        if self.config.enable_circuit_breaker {
            if let Some(cb) = self.circuit_breakers.write().get_mut(peer_id) {
                cb.record_success();
            }
        }
    }

    pub fn record_peer_failure(&self, peer_id: &PeerId) {
        if let Some(info) = self.peers.write().get_mut(peer_id) {
            info.record_failure();
        }

        if self.config.enable_circuit_breaker {
            if let Some(cb) = self.circuit_breakers.write().get_mut(peer_id) {
                cb.record_failure();
            }
        }

        self.stats
            .connection_failures
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn apply_peer_penalty(&self, peer_id: &PeerId, reason: PenaltyReason) {
        if let Some(score) = self.peer_store.apply_penalty(peer_id, reason.clone()) {
            if score >= crate::p2p::peer_store::MAX_PENALTY_SCORE {
                self.stats.banned_peers.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    pub fn record_peer_interaction_success(&self, peer_id: &PeerId) {
        self.peer_store.record_success(peer_id);
        self.record_peer_success(peer_id);
    }

    pub fn record_peer_interaction_failure(&self, peer_id: &PeerId) {
        self.peer_store.record_failure(peer_id);
        self.record_peer_failure(peer_id);
    }

    pub fn trustworthy_peers(&self) -> Vec<PeerEntry> {
        self.peer_store.trustworthy_peers()
    }

    pub fn peer_store_stats(&self) -> PeerStoreStats {
        self.peer_store.stats()
    }

    pub fn cleanup_peer_store(&self) {
        self.peer_store.cleanup_expired();
        self.cleanup_expired_bans();
    }
}
