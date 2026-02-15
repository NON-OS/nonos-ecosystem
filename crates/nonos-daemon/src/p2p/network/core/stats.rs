use super::network::P2pNetwork;
use crate::p2p::types::{NetworkStats, NetworkStatsSnapshot};
use std::sync::Arc;
use std::time::Duration;

impl P2pNetwork {
    pub fn stats(&self) -> NetworkStatsSnapshot {
        let mut snapshot = self.stats.snapshot();
        snapshot.peer_count = self.peers.read().len() as u64;
        snapshot.active_topics = self.subscribed_topics.read().len() as u64;
        snapshot
    }

    pub fn stats_ref(&self) -> Arc<NetworkStats> {
        Arc::clone(&self.stats)
    }

    pub fn uptime(&self) -> Option<Duration> {
        self.started_at.map(|t| t.elapsed())
    }
}
