use crate::wallet::TokenAmount;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub [u8; 32]);

impl NodeId {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn to_string_id(&self) -> String {
        format!("nxnd_{}", &self.to_hex()[..16])
    }
}

impl fmt::Debug for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeId({})", self.to_string_id())
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string_id())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeStatus {
    Starting,
    Running,
    Syncing,
    Stopped,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum NodeTier {
    Bronze,
    Silver,
    Gold,
    Platinum,
    Diamond,
}

impl NodeTier {
    pub fn min_stake(&self) -> u64 {
        match self {
            NodeTier::Bronze => 1_000,
            NodeTier::Silver => 10_000,
            NodeTier::Gold => 50_000,
            NodeTier::Platinum => 200_000,
            NodeTier::Diamond => 1_000_000,
        }
    }

    pub fn lock_days(&self) -> u32 {
        match self {
            NodeTier::Bronze => 0,
            NodeTier::Silver => 30,
            NodeTier::Gold => 90,
            NodeTier::Platinum => 180,
            NodeTier::Diamond => 365,
        }
    }

    pub fn apy_range(&self) -> (u8, u8) {
        match self {
            NodeTier::Bronze => (5, 8),
            NodeTier::Silver => (8, 12),
            NodeTier::Gold => (12, 18),
            NodeTier::Platinum => (18, 25),
            NodeTier::Diamond => (25, 35),
        }
    }

    pub fn multiplier(&self) -> f64 {
        match self {
            NodeTier::Bronze => 1.0,
            NodeTier::Silver => 1.5,
            NodeTier::Gold => 2.0,
            NodeTier::Platinum => 2.5,
            NodeTier::Diamond => 3.0,
        }
    }

    pub fn from_stake(stake: u64) -> Self {
        if stake >= 1_000_000 {
            NodeTier::Diamond
        } else if stake >= 200_000 {
            NodeTier::Platinum
        } else if stake >= 50_000 {
            NodeTier::Gold
        } else if stake >= 10_000 {
            NodeTier::Silver
        } else {
            NodeTier::Bronze
        }
    }

    pub fn to_index(&self) -> u8 {
        match self {
            NodeTier::Bronze => 0,
            NodeTier::Silver => 1,
            NodeTier::Gold => 2,
            NodeTier::Platinum => 3,
            NodeTier::Diamond => 4,
        }
    }

    pub fn from_index(index: u8) -> Self {
        match index {
            1 => NodeTier::Silver,
            2 => NodeTier::Gold,
            3 => NodeTier::Platinum,
            4 => NodeTier::Diamond,
            _ => NodeTier::Bronze,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QualityScore {
    pub uptime: f64,
    pub success_rate: f64,
    pub latency_score: f64,
    pub reliability: f64,
}

impl QualityScore {
    pub fn total(&self) -> f64 {
        (self.uptime * 0.30)
            + (self.success_rate * 0.35)
            + (self.latency_score * 0.20)
            + (self.reliability * 0.15)
    }

    pub fn perfect() -> Self {
        Self {
            uptime: 1.0,
            success_rate: 1.0,
            latency_score: 1.0,
            reliability: 1.0,
        }
    }

    pub fn zero() -> Self {
        Self {
            uptime: 0.0,
            success_rate: 0.0,
            latency_score: 0.0,
            reliability: 0.0,
        }
    }
}

impl Default for QualityScore {
    fn default() -> Self {
        Self::zero()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeMetrics {
    pub node_id: NodeId,
    pub status: NodeStatus,
    pub tier: NodeTier,
    pub quality: QualityScore,
    pub staked: TokenAmount,
    pub pending_rewards: TokenAmount,
    pub streak: u32,
    pub uptime_secs: u64,
    pub active_connections: u32,
    pub total_requests: u64,
    pub successful_requests: u64,
}
