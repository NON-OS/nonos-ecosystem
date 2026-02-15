use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BanEntry {
    pub peer_id: String,
    pub banned_at: i64,
    pub expires_at: i64,
    pub reason: String,
}

impl BanEntry {
    pub fn new(peer_id: PeerId, duration: Duration, reason: &str) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            peer_id: peer_id.to_string(),
            banned_at: now,
            expires_at: now + duration.as_secs() as i64,
            reason: reason.to_string(),
        }
    }

    pub fn is_expired(&self) -> bool {
        chrono::Utc::now().timestamp() >= self.expires_at
    }

    pub fn remaining(&self) -> Duration {
        let now = chrono::Utc::now().timestamp();
        if now >= self.expires_at {
            Duration::ZERO
        } else {
            Duration::from_secs((self.expires_at - now) as u64)
        }
    }
}
