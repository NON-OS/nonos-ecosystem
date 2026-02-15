mod behaviour;
mod messages;
mod network;
mod peer_store;
mod swarm;
mod types;

pub use behaviour::{NonosBehaviour, NonosBehaviourEvent};
pub use messages::{HealthBeaconData, NodeAnnouncementData, P2pMessage, QualityReportData};
pub use network::{NetworkConfig, P2pNetwork};
pub use peer_store::{
    new_shared_peer_store, PeerEntry, PeerState, PeerStore, PeerStoreStats,
    PenaltyReason, SharedPeerStore, DEFAULT_BAN_DURATION, MAX_PENALTY_SCORE,
    SEVERE_BAN_DURATION, SIDELINE_COOLDOWN,
};
pub use types::{
    BackoffStrategy, BanEntry, CircuitBreaker, CircuitState, ConnectionState,
    ConnectionTracker, NetworkCommand, NetworkEvent, NetworkStats, NetworkStatsSnapshot,
    PeerInfo, RateLimitReason, RateLimiter,
};

pub mod topics {
    pub const HEALTH_BEACON: &str = "nonos/health";
    pub const QUALITY_REPORTS: &str = "nonos/quality";
    pub const PEER_DISCOVERY: &str = "nonos/peers";
    pub const NODE_ANNOUNCEMENTS: &str = "nonos/announcements";
    pub const PRIVACY_COORD: &str = "nonos/privacy";
}

#[cfg(test)]
mod tests;
