use crate::address::EthAddress;
use crate::crypto::Blake3Hash;
use crate::node::{NodeId, NodeTier};
use crate::wallet::TokenAmount;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EpochNumber(pub u64);

impl EpochNumber {
    pub fn next(&self) -> Self {
        Self(self.0 + 1)
    }

    pub fn prev(&self) -> Self {
        Self(self.0.saturating_sub(1))
    }
}

impl fmt::Display for EpochNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Epoch#{}", self.0)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StakeRecord {
    pub staker: EthAddress,
    pub node_id: Option<NodeId>,
    pub amount: TokenAmount,
    pub tier: NodeTier,
    pub lock_start: chrono::DateTime<chrono::Utc>,
    pub lock_end: chrono::DateTime<chrono::Utc>,
    pub is_locked: bool,
}

impl StakeRecord {
    pub fn is_unlockable(&self) -> bool {
        !self.is_locked || chrono::Utc::now() >= self.lock_end
    }

    pub fn weight(&self) -> f64 {
        let stake_value = self.amount.raw as f64 / 10f64.powi(self.amount.decimals as i32);
        stake_value.sqrt() * self.tier.multiplier()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RewardClaim {
    pub epoch: EpochNumber,
    pub claimant: EthAddress,
    pub amount: TokenAmount,
    pub claimed_at: chrono::DateTime<chrono::Utc>,
    pub tx_hash: Blake3Hash,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EpochSummary {
    pub epoch: EpochNumber,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: chrono::DateTime<chrono::Utc>,
    pub total_emission: TokenAmount,
    pub total_weight: f64,
    pub staker_count: u32,
    pub avg_quality: f64,
}
