use crate::config::NodeRole;
use libp2p::{Multiaddr, PeerId};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

use super::entry::PeerEntry;
use super::types::{
    PeerState, PeerStoreStats, PenaltyReason, DEFAULT_BAN_DURATION, MAX_PENALTY_SCORE,
    SEVERE_BAN_DURATION,
};

pub struct PeerStore {
    peers: RwLock<HashMap<PeerId, PeerEntry>>,
    max_peers: u32,
    ban_threshold: i32,
    total_bans: AtomicU64,
    total_penalties: AtomicU64,
    created_at: Instant,
}

impl PeerStore {
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

    pub fn with_defaults() -> Self {
        Self::new(100, MAX_PENALTY_SCORE)
    }

    pub fn get_or_create(&self, peer_id: PeerId) -> PeerEntry {
        let mut peers = self.peers.write();
        peers
            .entry(peer_id)
            .or_insert_with(|| PeerEntry::new(peer_id))
            .clone()
    }

    pub fn get(&self, peer_id: &PeerId) -> Option<PeerEntry> {
        self.peers.read().get(peer_id).cloned()
    }

    pub fn update<F>(&self, peer_id: &PeerId, f: F)
    where
        F: FnOnce(&mut PeerEntry),
    {
        if let Some(entry) = self.peers.write().get_mut(peer_id) {
            f(entry);
        }
    }

    pub fn upsert(&self, peer_id: PeerId, entry: PeerEntry) {
        self.peers.write().insert(peer_id, entry);
    }

    pub fn remove(&self, peer_id: &PeerId) -> Option<PeerEntry> {
        self.peers.write().remove(peer_id)
    }

    pub fn contains(&self, peer_id: &PeerId) -> bool {
        self.peers.read().contains_key(peer_id)
    }

    pub fn is_banned(&self, peer_id: &PeerId) -> bool {
        self.peers.read().get(peer_id).map(|p| p.is_banned()).unwrap_or(false)
    }

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

    pub fn unban(&self, peer_id: &PeerId) {
        if let Some(entry) = self.peers.write().get_mut(peer_id) {
            entry.unban();
            info!("Unbanned peer {}", peer_id);
        }
    }

    pub fn apply_penalty(&self, peer_id: &PeerId, reason: PenaltyReason) -> Option<i32> {
        let mut peers = self.peers.write();
        if let Some(entry) = peers.get_mut(peer_id) {
            let new_score = entry.apply_penalty(reason.clone());
            self.total_penalties.fetch_add(1, Ordering::Relaxed);

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

    pub fn record_success(&self, peer_id: &PeerId) {
        self.update(peer_id, |entry| entry.record_success());
    }

    pub fn record_failure(&self, peer_id: &PeerId) {
        self.update(peer_id, |entry| entry.record_failure());
    }

    pub fn record_message(&self, peer_id: &PeerId, bytes: u64, sent: bool) {
        self.update(peer_id, |entry| entry.record_message(bytes, sent));
    }

    pub fn mark_connected(&self, peer_id: &PeerId, addresses: Vec<Multiaddr>) {
        let mut peers = self.peers.write();
        let entry = peers.entry(*peer_id).or_insert_with(|| PeerEntry::new(*peer_id));
        entry.state = PeerState::Connected;
        entry.last_seen = chrono::Utc::now().timestamp();
        entry.connection_count += 1;
        entry.addresses = addresses.iter().map(|a| a.to_string()).collect();
    }

    pub fn mark_disconnected(&self, peer_id: &PeerId) {
        self.update(peer_id, |entry| {
            if entry.state == PeerState::Connected {
                entry.state = PeerState::Disconnected;
            }
        });
    }

    pub fn set_latency(&self, peer_id: &PeerId, latency_ms: u32) {
        self.update(peer_id, |entry| {
            entry.latency_ms = Some(latency_ms);
            entry.record_success();
        });
    }

    pub fn set_protocol_info(&self, peer_id: &PeerId, protocol: String, agent: String) {
        self.update(peer_id, |entry| {
            entry.protocol_version = Some(protocol);
            entry.agent_version = Some(agent);
        });
    }

    pub fn connected_peers(&self) -> Vec<PeerEntry> {
        self.peers
            .read()
            .values()
            .filter(|p| p.state == PeerState::Connected)
            .cloned()
            .collect()
    }

    pub fn banned_peers(&self) -> Vec<PeerEntry> {
        self.peers
            .read()
            .values()
            .filter(|p| p.is_banned())
            .cloned()
            .collect()
    }

    pub fn all_peers(&self) -> Vec<PeerEntry> {
        self.peers.read().values().cloned().collect()
    }

    pub fn peer_count(&self) -> usize {
        self.peers.read().len()
    }

    pub fn connected_count(&self) -> usize {
        self.peers
            .read()
            .values()
            .filter(|p| p.state == PeerState::Connected)
            .count()
    }

    pub fn has_capacity(&self) -> bool {
        self.connected_count() < self.max_peers as usize
    }

    pub fn trustworthy_peers(&self) -> Vec<PeerEntry> {
        self.peers
            .read()
            .values()
            .filter(|p| p.is_trustworthy() && p.state == PeerState::Connected)
            .cloned()
            .collect()
    }

    pub fn peers_by_role(&self, role: NodeRole) -> Vec<PeerEntry> {
        self.peers
            .read()
            .values()
            .filter(|p| p.role_hint == Some(role))
            .cloned()
            .collect()
    }

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

    pub fn total_bans(&self) -> u64 {
        self.total_bans.load(Ordering::Relaxed)
    }

    pub fn total_penalties(&self) -> u64 {
        self.total_penalties.load(Ordering::Relaxed)
    }

    pub fn uptime(&self) -> Duration {
        self.created_at.elapsed()
    }
}

impl Default for PeerStore {
    fn default() -> Self {
        Self::with_defaults()
    }
}

pub type SharedPeerStore = Arc<PeerStore>;

pub fn new_shared_peer_store(max_peers: u32, ban_threshold: i32) -> SharedPeerStore {
    Arc::new(PeerStore::new(max_peers, ban_threshold))
}
