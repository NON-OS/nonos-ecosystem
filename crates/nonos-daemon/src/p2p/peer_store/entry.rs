use crate::config::NodeRole;
use libp2p::{Multiaddr, PeerId};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::debug;

use super::types::{PeerState, PenaltyReason, MAX_PENALTY_SCORE, MIN_QUALITY_THRESHOLD};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerEntry {
    pub peer_id: String,
    pub addresses: Vec<String>,
    pub state: PeerState,
    pub first_seen: i64,
    pub last_seen: i64,
    pub last_success: Option<i64>,
    pub messages_received: u64,
    pub messages_sent: u64,
    pub bytes_received: u64,
    pub bytes_sent: u64,
    pub error_count: u32,
    pub penalty_score: i32,
    pub quality_score: f64,
    pub latency_ms: Option<u32>,
    pub protocol_version: Option<String>,
    pub agent_version: Option<String>,
    pub role_hint: Option<NodeRole>,
    pub is_bootstrap: bool,
    pub ban_expires_at: Option<i64>,
    pub ban_reason: Option<String>,
    pub sideline_expires_at: Option<i64>,
    pub connection_count: u32,
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
    pub fn new(peer_id: PeerId) -> Self {
        Self {
            peer_id: peer_id.to_string(),
            ..Default::default()
        }
    }

    pub fn with_addresses(peer_id: PeerId, addresses: Vec<Multiaddr>) -> Self {
        Self {
            peer_id: peer_id.to_string(),
            addresses: addresses.iter().map(|a| a.to_string()).collect(),
            ..Default::default()
        }
    }

    pub fn is_banned(&self) -> bool {
        if self.state == PeerState::Banned {
            if let Some(expires) = self.ban_expires_at {
                return chrono::Utc::now().timestamp() < expires;
            }
        }
        false
    }

    pub fn is_sidelined(&self) -> bool {
        if self.state == PeerState::Sidelined {
            if let Some(expires) = self.sideline_expires_at {
                return chrono::Utc::now().timestamp() < expires;
            }
        }
        false
    }

    pub fn is_trustworthy(&self) -> bool {
        !self.is_banned() && self.penalty_score < MAX_PENALTY_SCORE / 2 && self.quality_score > MIN_QUALITY_THRESHOLD
    }

    pub fn record_success(&mut self) {
        let now = chrono::Utc::now().timestamp();
        self.last_seen = now;
        self.last_success = Some(now);
        self.consecutive_failures = 0;
        self.penalty_score = (self.penalty_score - 1).max(0);
        self.update_quality_score();
    }

    pub fn record_failure(&mut self) {
        self.last_seen = chrono::Utc::now().timestamp();
        self.consecutive_failures += 1;
        self.error_count += 1;
        self.penalty_score = (self.penalty_score + 5).min(MAX_PENALTY_SCORE);
        self.update_quality_score();
    }

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

    fn update_quality_score(&mut self) {
        let penalty_factor = 1.0 - (self.penalty_score as f64 / MAX_PENALTY_SCORE as f64);

        let total_interactions = self.messages_received + self.messages_sent;
        let reliability = if total_interactions > 0 {
            1.0 - (self.error_count as f64 / (total_interactions as f64 + self.error_count as f64))
        } else {
            1.0
        };

        let latency_factor = match self.latency_ms {
            Some(ms) if ms < 50 => 1.0,
            Some(ms) if ms < 100 => 0.9,
            Some(ms) if ms < 250 => 0.8,
            Some(ms) if ms < 500 => 0.6,
            Some(ms) if ms < 1000 => 0.4,
            Some(_) => 0.2,
            None => 0.5,
        };

        self.quality_score = (penalty_factor * 0.5 + reliability * 0.3 + latency_factor * 0.2).clamp(0.0, 1.0);
    }

    pub fn ban(&mut self, duration: Duration, reason: &str) {
        let expires = chrono::Utc::now().timestamp() + duration.as_secs() as i64;
        self.state = PeerState::Banned;
        self.ban_expires_at = Some(expires);
        self.ban_reason = Some(reason.to_string());
        self.sideline_expires_at = None;
    }

    pub fn unban(&mut self) {
        self.state = PeerState::Disconnected;
        self.ban_expires_at = None;
        self.ban_reason = None;
        self.penalty_score = MAX_PENALTY_SCORE / 2;
    }

    pub fn sideline(&mut self, duration: Duration) {
        let expires = chrono::Utc::now().timestamp() + duration.as_secs() as i64;
        self.state = PeerState::Sidelined;
        self.sideline_expires_at = Some(expires);
    }

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
