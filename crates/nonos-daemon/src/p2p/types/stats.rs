use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

pub struct NetworkStats {
    pub peer_count: AtomicU64,
    pub bytes_sent: AtomicU64,
    pub bytes_received: AtomicU64,
    pub messages_published: AtomicU64,
    pub messages_received: AtomicU64,
    pub messages_dropped: AtomicU64,
    pub active_topics: AtomicU64,
    pub connection_attempts: AtomicU64,
    pub connection_failures: AtomicU64,
    pub rate_limit_hits: AtomicU64,
    pub banned_peers: AtomicU64,
    pub circuit_breaker_trips: AtomicU64,
    started_at: Instant,
}

impl Default for NetworkStats {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkStats {
    pub fn new() -> Self {
        Self {
            peer_count: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            messages_published: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            messages_dropped: AtomicU64::new(0),
            active_topics: AtomicU64::new(0),
            connection_attempts: AtomicU64::new(0),
            connection_failures: AtomicU64::new(0),
            rate_limit_hits: AtomicU64::new(0),
            banned_peers: AtomicU64::new(0),
            circuit_breaker_trips: AtomicU64::new(0),
            started_at: Instant::now(),
        }
    }

    pub fn uptime(&self) -> std::time::Duration {
        self.started_at.elapsed()
    }

    pub fn snapshot(&self) -> NetworkStatsSnapshot {
        NetworkStatsSnapshot {
            peer_count: self.peer_count.load(Ordering::Relaxed),
            bytes_sent: self.bytes_sent.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
            messages_published: self.messages_published.load(Ordering::Relaxed),
            messages_received: self.messages_received.load(Ordering::Relaxed),
            messages_dropped: self.messages_dropped.load(Ordering::Relaxed),
            active_topics: self.active_topics.load(Ordering::Relaxed),
            connection_attempts: self.connection_attempts.load(Ordering::Relaxed),
            connection_failures: self.connection_failures.load(Ordering::Relaxed),
            rate_limit_hits: self.rate_limit_hits.load(Ordering::Relaxed),
            banned_peers: self.banned_peers.load(Ordering::Relaxed),
            circuit_breaker_trips: self.circuit_breaker_trips.load(Ordering::Relaxed),
            uptime_secs: self.started_at.elapsed().as_secs(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkStatsSnapshot {
    pub peer_count: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub messages_published: u64,
    pub messages_received: u64,
    pub messages_dropped: u64,
    pub active_topics: u64,
    pub connection_attempts: u64,
    pub connection_failures: u64,
    pub rate_limit_hits: u64,
    pub banned_peers: u64,
    pub circuit_breaker_trips: u64,
    pub uptime_secs: u64,
}
