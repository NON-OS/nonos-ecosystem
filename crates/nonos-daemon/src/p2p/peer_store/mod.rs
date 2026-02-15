mod entry;
mod store;
mod types;

pub use entry::PeerEntry;
pub use store::{new_shared_peer_store, PeerStore, SharedPeerStore};
pub use types::{
    PeerState, PeerStoreStats, PenaltyReason, DEFAULT_BAN_DURATION, MAX_PENALTY_SCORE,
    SEVERE_BAN_DURATION, SIDELINE_COOLDOWN,
};

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::PeerId;
    use std::time::Duration;

    #[test]
    fn test_peer_entry_creation() {
        let peer_id = PeerId::random();
        let entry = PeerEntry::new(peer_id);

        assert_eq!(entry.peer_id, peer_id.to_string());
        assert_eq!(entry.state, PeerState::Disconnected);
        assert_eq!(entry.penalty_score, 0);
        assert!(!entry.is_banned());
    }

    #[test]
    fn test_penalty_application() {
        let peer_id = PeerId::random();
        let mut entry = PeerEntry::new(peer_id);

        entry.apply_penalty(PenaltyReason::ProtocolViolation);
        assert_eq!(entry.penalty_score, 20);

        entry.apply_penalty(PenaltyReason::Spam);
        assert_eq!(entry.penalty_score, 45);
    }

    #[test]
    fn test_quality_score_update() {
        let peer_id = PeerId::random();
        let mut entry = PeerEntry::new(peer_id);

        assert_eq!(entry.quality_score, 1.0);

        entry.apply_penalty(PenaltyReason::ProtocolViolation);
        assert!(entry.quality_score < 1.0);

        entry.latency_ms = Some(25);
        entry.record_success();
    }

    #[test]
    fn test_ban_and_unban() {
        let peer_id = PeerId::random();
        let mut entry = PeerEntry::new(peer_id);

        entry.ban(Duration::from_secs(60), "test");
        assert!(entry.is_banned());
        assert_eq!(entry.state, PeerState::Banned);

        entry.unban();
        assert!(!entry.is_banned());
        assert_eq!(entry.state, PeerState::Disconnected);
    }

    #[test]
    fn test_peer_store_operations() {
        let store = PeerStore::new(100, MAX_PENALTY_SCORE);
        let peer_id = PeerId::random();

        let entry = store.get_or_create(peer_id);
        assert_eq!(entry.peer_id, peer_id.to_string());

        store.update(&peer_id, |e| e.latency_ms = Some(50));
        let updated = store.get(&peer_id).unwrap();
        assert_eq!(updated.latency_ms, Some(50));

        store.mark_connected(&peer_id, vec![]);
        let connected = store.get(&peer_id).unwrap();
        assert_eq!(connected.state, PeerState::Connected);

        let stats = store.stats();
        assert_eq!(stats.total_peers, 1);
        assert_eq!(stats.connected_peers, 1);
    }

    #[test]
    fn test_auto_ban() {
        let store = PeerStore::new(100, 50);
        let peer_id = PeerId::random();

        store.get_or_create(peer_id);

        store.apply_penalty(&peer_id, PenaltyReason::Spam);
        store.apply_penalty(&peer_id, PenaltyReason::Spam);

        assert!(store.is_banned(&peer_id));
    }

    #[test]
    fn test_trustworthy_peers() {
        let store = PeerStore::new(100, MAX_PENALTY_SCORE);
        let good_peer = PeerId::random();
        let bad_peer = PeerId::random();

        store.mark_connected(&good_peer, vec![]);
        store.record_success(&good_peer);

        store.mark_connected(&bad_peer, vec![]);
        for _ in 0..10 {
            store.apply_penalty(&bad_peer, PenaltyReason::ProtocolViolation);
        }

        let trustworthy = store.trustworthy_peers();
        assert_eq!(trustworthy.len(), 1);
        assert_eq!(trustworthy[0].peer_id, good_peer.to_string());
    }
}
