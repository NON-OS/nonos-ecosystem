// NONOS GNU AFFERO GENERAL PUBLIC LICENSE
// Version 3, 19 November 2007
// Copyright (C) 2025 NON-OS <team@nonos.systems>

//! P2P Network Module
//!
//! Provides decentralized peer-to-peer networking for NONØS nodes:
//! - libp2p-based networking with gossipsub and Kademlia
//! - Peer discovery and management
//! - Rate limiting and circuit breakers
//! - Penalty and ban management
//! - Bootstrap node support

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

/// P2P topic names for gossipsub
pub mod topics {
    /// Health beacon announcements
    pub const HEALTH_BEACON: &str = "nonos/health";
    /// Quality report submissions
    pub const QUALITY_REPORTS: &str = "nonos/quality";
    /// Peer discovery messages
    pub const PEER_DISCOVERY: &str = "nonos/peers";
    /// Node announcements (role changes, capabilities)
    pub const NODE_ANNOUNCEMENTS: &str = "nonos/announcements";
    /// Privacy service coordination
    pub const PRIVACY_COORD: &str = "nonos/privacy";
}

#[cfg(test)]
mod tests;
