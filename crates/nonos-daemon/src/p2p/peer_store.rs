// NONOS GNU AFFERO GENERAL PUBLIC LICENSE
// Version 3, 19 November 2007
// Copyright (C) 2025 NON-OS <team@nonos.systems>

//! Peer Store Module
//!
//! Provides a comprehensive peer management abstraction with:
//! - Peer tracking (addresses, last seen, message counts)
//! - Quality scoring (uptime, responsiveness)
//! - Penalty and ban management
//! - Role hints for peer classification
//! - Persistence support for peer data

use crate::config::NodeRole;
use libp2p::{Multiaddr, PeerId};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Maximum penalty score before automatic ban
pub const MAX_PENALTY_SCORE: i32 = 100;

/// Minimum quality score threshold
pub const MIN_QUALITY_THRESHOLD: f64 = 0.3;

/// Default ban duration for protocol violations
pub const DEFAULT_BAN_DURATION: Duration = Duration::from_secs(3600);

/// Extended ban duration for severe violations
pub const SEVERE_BAN_DURATION: Duration = Duration::from_secs(86400);

/// Cooldown period for sidelined peers
pub const SIDELINE_COOLDOWN: Duration = Duration::from_secs(300);

/// Peer connection state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerState {
    /// Peer is connected
    Connected,
    /// Peer is disconnected
    Disconnected,
    /// Peer is being dialed
    Dialing,
    /// Peer is sidelined (in cooldown)
    Sidelined,
    /// Peer is banned
    Banned,
}

impl Default for PeerState {
    fn default() -> Self {
        Self::Disconnected
    }
}

/// Reason for peer penalty
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PenaltyReason {
    /// Protocol violation
    ProtocolViolation,
    /// Excessive message rate
    ExcessiveMessages,
    /// Malformed messages
    MalformedMessage,
    /// Unresponsive (timeout)
    Unresponsive,
    /// Invalid data
    InvalidData,
    /// Spam detection
    Spam,
    /// Connection abuse
    ConnectionAbuse,
}

impl std::fmt::Display for PenaltyReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PenaltyReason::ProtocolViolation => write!(f, "protocol_violation"),
            PenaltyReason::ExcessiveMessages => write!(f, "excessive_messages"),
            PenaltyReason::MalformedMessage => write!(f, "malformed_message"),
            PenaltyReason::Unresponsive => write!(f, "unresponsive"),
            PenaltyReason::InvalidData => write!(f, "invalid_data"),
            PenaltyReason::Spam => write!(f, "spam"),
            PenaltyReason::ConnectionAbuse => write!(f, "connection_abuse"),
        }
    }
}

/// Detailed peer entry in the store
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerEntry {
    /// Peer ID as string
    pub peer_id: String,
    /// Known addresses
    pub addresses: Vec<String>,
    /// Current connection state
    pub state: PeerState,
    /// First seen timestamp
    pub first_seen: i64,
    /// Last seen timestamp
    pub last_seen: i64,
    /// Last successful interaction
    pub last_success: Option<i64>,
    /// Total messages received
    pub messages_received: u64,
    /// Total messages sent
    pub messages_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Error count
    pub error_count: u32,
    /// Penalty score (0-100, higher = worse)
    pub penalty_score: i32,
    /// Quality score (0.0-1.0, higher = better)
    pub quality_score: f64,
    /// Latency in milliseconds
    pub latency_ms: Option<u32>,
    /// Protocol version if known
    pub protocol_version: Option<String>,
    /// Agent version if known
    pub agent_version: Option<String>,
    /// Role hint for this peer
    pub role_hint: Option<NodeRole>,
    /// Is this a bootstrap peer
    pub is_bootstrap: bool,
    /// Ban expiry timestamp (if banned)
    pub ban_expires_at: Option<i64>,
    /// Reason for ban (if banned)
    pub ban_reason: Option<String>,
    /// Sideline expiry timestamp (if sidelined)
    pub sideline_expires_at: Option<i64>,
    /// Total connection count
    pub connection_count: u32,
    /// Consecutive failures
    pub consecutive_failures: u32,
}

impl Default for PeerEntry {
    fn default() -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            peer_id: String::new(),
            addresses: Vec::new(),
            state: PeerState::Disconnected,
            first_seen: now,
            last_seen: now,
            last_success: None,
            messages_received: 0,
            messages_sent: 0,
            bytes_received: 0,
            bytes_sent: 0,
            error_count: 0,
            penalty_score: 0,
            quality_score: 1.0,
            latency_ms: None,
            protocol_version: None,
            agent_version: None,
            role_hint: None,
            is_bootstrap: false,
            ban_expires_at: None,
            ban_reason: None,
            sideline_expires_at: None,
            connection_count: 0,
            consecutive_failures: 0,
        }
    }
}

impl PeerEntry {
    /// Create a new peer entry
    pub fn new(peer_id: PeerId) -> Self {
        Self {
            peer_id: peer_id.to_string(),
            ..Default::default()
        }
    }

    /// Create a new peer entry with addresses
    pub fn with_addresses(peer_id: PeerId, addresses: Vec<Multiaddr>) -> Self {
        Self {
            peer_id: peer_id.to_string(),
            addresses: addresses.iter().map(|a| a.to_string()).collect(),
            ..Default::default()
        }
    }

    /// Check if this peer is currently banned
    pub fn is_banned(&self) -> bool {
        if self.state == PeerState::Banned {
            if let Some(expires) = self.ban_expires_at {
                return chrono::Utc::now().timestamp() < expires;
            }
        }
        false
    }

    /// Check if this peer is currently sidelined
    pub fn is_sidelined(&self) -> bool {
        if self.state == PeerState::Sidelined {
            if let Some(expires) = self.sideline_expires_at {
                return chrono::Utc::now().timestamp() < expires;
            }
        }
        false
    }

    /// Check if this peer is trustworthy
    pub fn is_trustworthy(&self) -> bool {
        !self.is_banned() && self.penalty_score < MAX_PENALTY_SCORE / 2 && self.quality_score > MIN_QUALITY_THRESHOLD
    }

    /// Record a successful interaction
    pub fn record_success(&mut self) {
        let now = chrono::Utc::now().timestamp();
        self.last_seen = now;
        self.last_success = Some(now);
        self.consecutive_failures = 0;
        self.penalty_score = (self.penalty_score - 1).max(0);
        self.update_quality_score();
    }

    /// Record a failure
    pub fn record_failure(&mut self) {
        self.last_seen = chrono::Utc::now().timestamp();
        self.consecutive_failures += 1;
        self.error_count += 1;
        self.penalty_score = (self.penalty_score + 5).min(MAX_PENALTY_SCORE);
        self.update_quality_score();
    }

    /// Apply a penalty for a specific reason
    pub fn apply_penalty(&mut self, reason: PenaltyReason) -> i32 {
        let penalty = match reason {
            PenaltyReason::ProtocolViolation => 20,
            PenaltyReason::ExcessiveMessages => 10,
            PenaltyReason::MalformedMessage => 15,
            PenaltyReason::Unresponsive => 5,
            PenaltyReason::InvalidData => 15,
            PenaltyReason::Spam => 25,
            PenaltyReason::ConnectionAbuse => 20,
        };

        self.penalty_score = (self.penalty_score + penalty).min(MAX_PENALTY_SCORE);
        self.update_quality_score();

        debug!(
            "Applied penalty {} to peer {} for {}: new score {}",
            penalty, self.peer_id, reason, self.penalty_score
        );

        self.penalty_score
    }

    /// Record a message
    pub fn record_message(&mut self, bytes: u64, sent: bool) {
        self.last_seen = chrono::Utc::now().timestamp();
        if sent {
            self.messages_sent += 1;
            self.bytes_sent += bytes;
        } else {
            self.messages_received += 1;
            self.bytes_received += bytes;
        }
    }

    /// Update quality score based on current metrics
    fn update_quality_score(&mut self) {
        // Base quality from penalty (inverted)
        let penalty_factor = 1.0 - (self.penalty_score as f64 / MAX_PENALTY_SCORE as f64);

        // Reliability factor based on error rate
        let total_interactions = self.messages_received + self.messages_sent;
        let reliability = if total_interactions > 0 {
            1.0 - (self.error_count as f64 / (total_interactions as f64 + self.error_count as f64))
        } else {
            1.0
        };

        // Latency factor (good latency = higher score)
        let latency_factor = match self.latency_ms {
            Some(ms) if ms < 50 => 1.0,
            Some(ms) if ms < 100 => 0.9,
            Some(ms) if ms < 250 => 0.8,
            Some(ms) if ms < 500 => 0.6,
            Some(ms) if ms < 1000 => 0.4,
            Some(_) => 0.2,
            None => 0.5, // Unknown latency
        };

        // Weighted combination
        self.quality_score = (penalty_factor * 0.5 + reliability * 0.3 + latency_factor * 0.2).clamp(0.0, 1.0);
    }

    /// Ban this peer
    pub fn ban(&mut self, duration: Duration, reason: &str) {
        let expires = chrono::Utc::now().timestamp() + duration.as_secs() as i64;
        self.state = PeerState::Banned;
        self.ban_expires_at = Some(expires);
        self.ban_reason = Some(reason.to_string());
        self.sideline_expires_at = None;
    }

    /// Unban this peer
    pub fn unban(&mut self) {
        self.state = PeerState::Disconnected;
        self.ban_expires_at = None;
        self.ban_reason = None;
        self.penalty_score = MAX_PENALTY_SCORE / 2; // Reset to half
    }

    /// Sideline this peer (temporary cooldown)
    pub fn sideline(&mut self, duration: Duration) {
        let expires = chrono::Utc::now().timestamp() + duration.as_secs() as i64;
        self.state = PeerState::Sidelined;
        self.sideline_expires_at = Some(expires);
    }

    /// Get remaining ban time
    pub fn ban_remaining(&self) -> Option<Duration> {
        self.ban_expires_at.and_then(|expires| {
            let remaining = expires - chrono::Utc::now().timestamp();
            if remaining > 0 {
                Some(Duration::from_secs(remaining as u64))
            } else {
                None
            }
        })
    }
}

/// Peer store statistics
#[derive(Clone, Debug, Default)]
pub struct PeerStoreStats {
    pub total_peers: u64,
    pub connected_peers: u64,
    pub banned_peers: u64,
    pub sidelined_peers: u64,
    pub bootstrap_peers: u64,
    pub avg_quality_score: f64,
    pub total_messages: u64,
    pub total_bytes: u64,
}

/// Thread-safe peer store
pub struct PeerStore {
    /// All known peers
    peers: RwLock<HashMap<PeerId, PeerEntry>>,
    /// Maximum peer capacity
    max_peers: u32,
    /// Ban threshold
    ban_threshold: i32,
    /// Stats counters
    total_bans: AtomicU64,
    total_penalties: AtomicU64,
    created_at: Instant,
}

impl PeerStore {
    /// Create a new peer store
    pub fn new(max_peers: u32, ban_threshold: i32) -> Self {
        Self {
            peers: RwLock::new(HashMap::new()),
            max_peers,
            ban_threshold,
            total_bans: AtomicU64::new(0),
            total_penalties: AtomicU64::new(0),
            created_at: Instant::now(),
        }
    }

    /// Create a peer store with default settings
    pub fn with_defaults() -> Self {
        Self::new(100, MAX_PENALTY_SCORE)
    }

    /// Get or create a peer entry
    pub fn get_or_create(&self, peer_id: PeerId) -> PeerEntry {
        let mut peers = self.peers.write();
        peers
            .entry(peer_id)
            .or_insert_with(|| PeerEntry::new(peer_id))
            .clone()
    }

    /// Get a peer entry
    pub fn get(&self, peer_id: &PeerId) -> Option<PeerEntry> {
        self.peers.read().get(peer_id).cloned()
    }

    /// Update a peer entry
    pub fn update<F>(&self, peer_id: &PeerId, f: F)
    where
        F: FnOnce(&mut PeerEntry),
    {
        if let Some(entry) = self.peers.write().get_mut(peer_id) {
            f(entry);
        }
    }

    /// Insert or update a peer
    pub fn upsert(&self, peer_id: PeerId, entry: PeerEntry) {
        self.peers.write().insert(peer_id, entry);
    }

    /// Remove a peer
    pub fn remove(&self, peer_id: &PeerId) -> Option<PeerEntry> {
        self.peers.write().remove(peer_id)
    }

    /// Check if a peer exists
    pub fn contains(&self, peer_id: &PeerId) -> bool {
        self.peers.read().contains_key(peer_id)
    }

    /// Check if a peer is banned
    pub fn is_banned(&self, peer_id: &PeerId) -> bool {
        self.peers.read().get(peer_id).map(|p| p.is_banned()).unwrap_or(false)
    }

    /// Ban a peer
    pub fn ban(&self, peer_id: &PeerId, duration: Duration, reason: &str) {
        if let Some(entry) = self.peers.write().get_mut(peer_id) {
            entry.ban(duration, reason);
            self.total_bans.fetch_add(1, Ordering::Relaxed);
            warn!(
                "Banned peer {} for {:?}: {}",
                peer_id, duration, reason
            );
        }
    }

    /// Unban a peer
    pub fn unban(&self, peer_id: &PeerId) {
        if let Some(entry) = self.peers.write().get_mut(peer_id) {
            entry.unban();
            info!("Unbanned peer {}", peer_id);
        }
    }

    /// Apply a penalty to a peer
    pub fn apply_penalty(&self, peer_id: &PeerId, reason: PenaltyReason) -> Option<i32> {
        let mut peers = self.peers.write();
        if let Some(entry) = peers.get_mut(peer_id) {
            let new_score = entry.apply_penalty(reason.clone());
            self.total_penalties.fetch_add(1, Ordering::Relaxed);

            // Auto-ban if threshold exceeded
            if new_score >= self.ban_threshold {
                let duration = match reason {
                    PenaltyReason::Spam | PenaltyReason::ProtocolViolation => SEVERE_BAN_DURATION,
                    _ => DEFAULT_BAN_DURATION,
                };
                entry.ban(duration, &format!("Penalty threshold exceeded: {}", reason));
                self.total_bans.fetch_add(1, Ordering::Relaxed);
                warn!(
                    "Auto-banned peer {} due to penalty threshold: score={}",
                    peer_id, new_score
                );
            }

            return Some(new_score);
        }
        None
    }

    /// Record a successful interaction
    pub fn record_success(&self, peer_id: &PeerId) {
        self.update(peer_id, |entry| entry.record_success());
    }

    /// Record a failure
    pub fn record_failure(&self, peer_id: &PeerId) {
        self.update(peer_id, |entry| entry.record_failure());
    }

    /// Record a message
    pub fn record_message(&self, peer_id: &PeerId, bytes: u64, sent: bool) {
        self.update(peer_id, |entry| entry.record_message(bytes, sent));
    }

    /// Mark peer as connected
    pub fn mark_connected(&self, peer_id: &PeerId, addresses: Vec<Multiaddr>) {
        let mut peers = self.peers.write();
        let entry = peers.entry(*peer_id).or_insert_with(|| PeerEntry::new(*peer_id));
        entry.state = PeerState::Connected;
        entry.last_seen = chrono::Utc::now().timestamp();
        entry.connection_count += 1;
        entry.addresses = addresses.iter().map(|a| a.to_string()).collect();
    }

    /// Mark peer as disconnected
    pub fn mark_disconnected(&self, peer_id: &PeerId) {
        self.update(peer_id, |entry| {
            if entry.state == PeerState::Connected {
                entry.state = PeerState::Disconnected;
            }
        });
    }

    /// Set peer latency
    pub fn set_latency(&self, peer_id: &PeerId, latency_ms: u32) {
        self.update(peer_id, |entry| {
            entry.latency_ms = Some(latency_ms);
            entry.record_success();
        });
    }

    /// Set peer protocol info
    pub fn set_protocol_info(&self, peer_id: &PeerId, protocol: String, agent: String) {
        self.update(peer_id, |entry| {
            entry.protocol_version = Some(protocol);
            entry.agent_version = Some(agent);
        });
    }

    /// Get all connected peers
    pub fn connected_peers(&self) -> Vec<PeerEntry> {
        self.peers
            .read()
            .values()
            .filter(|p| p.state == PeerState::Connected)
            .cloned()
            .collect()
    }

    /// Get all banned peers
    pub fn banned_peers(&self) -> Vec<PeerEntry> {
        self.peers
            .read()
            .values()
            .filter(|p| p.is_banned())
            .cloned()
            .collect()
    }

    /// Get all peers
    pub fn all_peers(&self) -> Vec<PeerEntry> {
        self.peers.read().values().cloned().collect()
    }

    /// Get peer count
    pub fn peer_count(&self) -> usize {
        self.peers.read().len()
    }

    /// Get connected peer count
    pub fn connected_count(&self) -> usize {
        self.peers
            .read()
            .values()
            .filter(|p| p.state == PeerState::Connected)
            .count()
    }

    /// Check if we have capacity for more peers
    pub fn has_capacity(&self) -> bool {
        self.connected_count() < self.max_peers as usize
    }

    /// Get trustworthy peers (for relaying messages, etc.)
    pub fn trustworthy_peers(&self) -> Vec<PeerEntry> {
        self.peers
            .read()
            .values()
            .filter(|p| p.is_trustworthy() && p.state == PeerState::Connected)
            .cloned()
            .collect()
    }

    /// Get peers by role hint
    pub fn peers_by_role(&self, role: NodeRole) -> Vec<PeerEntry> {
        self.peers
            .read()
            .values()
            .filter(|p| p.role_hint == Some(role))
            .cloned()
            .collect()
    }

    /// Cleanup expired bans and sidelines
    pub fn cleanup_expired(&self) {
        let now = chrono::Utc::now().timestamp();
        let mut peers = self.peers.write();

        for entry in peers.values_mut() {
            if entry.state == PeerState::Banned {
                if let Some(expires) = entry.ban_expires_at {
                    if now >= expires {
                        entry.state = PeerState::Disconnected;
                        entry.ban_expires_at = None;
                        entry.ban_reason = None;
                        debug!("Ban expired for peer {}", entry.peer_id);
                    }
                }
            }

            if entry.state == PeerState::Sidelined {
                if let Some(expires) = entry.sideline_expires_at {
                    if now >= expires {
                        entry.state = PeerState::Disconnected;
                        entry.sideline_expires_at = None;
                        debug!("Sideline expired for peer {}", entry.peer_id);
                    }
                }
            }
        }
    }

    /// Prune old disconnected peers to stay within capacity
    pub fn prune_old_peers(&self, max_age_secs: i64) {
        let now = chrono::Utc::now().timestamp();
        let mut peers = self.peers.write();

        let to_remove: Vec<PeerId> = peers
            .iter()
            .filter(|(_, entry)| {
                entry.state == PeerState::Disconnected
                    && (now - entry.last_seen) > max_age_secs
                    && !entry.is_bootstrap
            })
            .map(|(id, _)| *id)
            .collect();

        for peer_id in to_remove {
            peers.remove(&peer_id);
        }
    }

    /// Get store statistics
    pub fn stats(&self) -> PeerStoreStats {
        let peers = self.peers.read();
        let total = peers.len() as u64;

        let connected = peers.values().filter(|p| p.state == PeerState::Connected).count() as u64;
        let banned = peers.values().filter(|p| p.is_banned()).count() as u64;
        let sidelined = peers.values().filter(|p| p.is_sidelined()).count() as u64;
        let bootstrap = peers.values().filter(|p| p.is_bootstrap).count() as u64;

        let avg_quality = if total > 0 {
            peers.values().map(|p| p.quality_score).sum::<f64>() / total as f64
        } else {
            1.0
        };

        let total_messages: u64 = peers.values().map(|p| p.messages_received + p.messages_sent).sum();
        let total_bytes: u64 = peers.values().map(|p| p.bytes_received + p.bytes_sent).sum();

        PeerStoreStats {
            total_peers: total,
            connected_peers: connected,
            banned_peers: banned,
            sidelined_peers: sidelined,
            bootstrap_peers: bootstrap,
            avg_quality_score: avg_quality,
            total_messages,
            total_bytes,
        }
    }

    /// Get total bans count
    pub fn total_bans(&self) -> u64 {
        self.total_bans.load(Ordering::Relaxed)
    }

    /// Get total penalties count
    pub fn total_penalties(&self) -> u64 {
        self.total_penalties.load(Ordering::Relaxed)
    }

    /// Get store uptime
    pub fn uptime(&self) -> Duration {
        self.created_at.elapsed()
    }
}

impl Default for PeerStore {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Thread-safe peer store wrapper
pub type SharedPeerStore = Arc<PeerStore>;

/// Create a new shared peer store
pub fn new_shared_peer_store(max_peers: u32, ban_threshold: i32) -> SharedPeerStore {
    Arc::new(PeerStore::new(max_peers, ban_threshold))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // Quality should improve slightly
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

        // Get or create
        let entry = store.get_or_create(peer_id);
        assert_eq!(entry.peer_id, peer_id.to_string());

        // Update
        store.update(&peer_id, |e| e.latency_ms = Some(50));
        let updated = store.get(&peer_id).unwrap();
        assert_eq!(updated.latency_ms, Some(50));

        // Mark connected
        store.mark_connected(&peer_id, vec![]);
        let connected = store.get(&peer_id).unwrap();
        assert_eq!(connected.state, PeerState::Connected);

        // Stats
        let stats = store.stats();
        assert_eq!(stats.total_peers, 1);
        assert_eq!(stats.connected_peers, 1);
    }

    #[test]
    fn test_auto_ban() {
        let store = PeerStore::new(100, 50);
        let peer_id = PeerId::random();

        store.get_or_create(peer_id);

        // Apply enough penalties to trigger auto-ban
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
