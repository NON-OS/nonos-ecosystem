use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const MAX_PENALTY_SCORE: i32 = 100;
pub const MIN_QUALITY_THRESHOLD: f64 = 0.3;
pub const DEFAULT_BAN_DURATION: Duration = Duration::from_secs(3600);
pub const SEVERE_BAN_DURATION: Duration = Duration::from_secs(86400);
pub const SIDELINE_COOLDOWN: Duration = Duration::from_secs(300);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerState {
    Connected,
    #[default]
    Disconnected,
    Dialing,
    Sidelined,
    Banned,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PenaltyReason {
    ProtocolViolation,
    ExcessiveMessages,
    MalformedMessage,
    Unresponsive,
    InvalidData,
    Spam,
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
