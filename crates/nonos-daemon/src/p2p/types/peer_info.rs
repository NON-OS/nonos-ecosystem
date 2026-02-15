use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::violations::{MessageViolation, ViolationCounts};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerInfo {
    pub id: String,
    pub addresses: Vec<String>,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub latency_ms: Option<u32>,
    pub protocol_version: Option<String>,
    pub agent_version: Option<String>,
    pub reputation: i32,
    pub message_count: u64,
    pub last_message_at: Option<i64>,
    pub failure_count: u32,
    pub is_banned: bool,
    pub ban_expires_at: Option<i64>,
    #[serde(default)]
    pub violation_counts: ViolationCounts,
    #[serde(default)]
    pub penalty_score: i32,
    #[serde(default)]
    pub last_violation_at: Option<i64>,
}

impl Default for PeerInfo {
    fn default() -> Self {
        Self {
            id: String::new(),
            addresses: Vec::new(),
            connected_at: chrono::Utc::now(),
            bytes_sent: 0,
            bytes_received: 0,
            latency_ms: None,
            protocol_version: None,
            agent_version: None,
            reputation: 50,
            message_count: 0,
            last_message_at: None,
            failure_count: 0,
            is_banned: false,
            ban_expires_at: None,
            violation_counts: ViolationCounts::default(),
            penalty_score: 0,
            last_violation_at: None,
        }
    }
}

impl PeerInfo {
    pub fn new(peer_id: PeerId, addresses: Vec<String>) -> Self {
        Self {
            id: peer_id.to_string(),
            addresses,
            connected_at: chrono::Utc::now(),
            ..Default::default()
        }
    }

    pub fn record_message(&mut self, bytes: u64) {
        self.message_count += 1;
        self.bytes_received += bytes;
        self.last_message_at = Some(chrono::Utc::now().timestamp());
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.reputation = (self.reputation - 5).max(-100);
    }

    pub fn record_success(&mut self) {
        self.failure_count = 0;
        self.reputation = (self.reputation + 1).min(100);
    }

    pub fn is_trustworthy(&self) -> bool {
        self.reputation >= 0 && !self.is_banned
    }

    pub fn record_violation(&mut self, violation: MessageViolation) {
        self.violation_counts.record(violation);
        self.last_violation_at = Some(chrono::Utc::now().timestamp());

        let base_penalty = violation.penalty_score();
        let multiplier = if self.violation_counts.is_repeat_offender() {
            violation.ban_multiplier() as i32
        } else {
            1
        };

        let penalty = base_penalty * multiplier;
        self.penalty_score += penalty;
        self.reputation = (self.reputation - penalty).max(-100);

        self.failure_count += 1;
    }

    pub fn should_ban(&self) -> bool {
        if self.reputation <= -50 {
            return true;
        }

        if self.violation_counts.total() >= 10 {
            return true;
        }

        if self.penalty_score >= 100 {
            return true;
        }

        false
    }

    pub fn recommended_ban_duration(&self) -> Duration {
        let base_seconds = 300u64;

        let multiplier = if self.violation_counts.protocol_violations > 0 {
            20
        } else if self.violation_counts.spam_behavior > 0 {
            12
        } else if self.violation_counts.is_repeat_offender() {
            6
        } else {
            1
        };

        Duration::from_secs(base_seconds * multiplier)
    }

    pub fn reset_violations(&mut self) {
        self.violation_counts = ViolationCounts::default();
        self.penalty_score = 0;
        self.last_violation_at = None;
    }
}
