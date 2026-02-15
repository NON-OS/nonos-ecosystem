use crate::contracts::{ContractClient, current_epoch as contract_epoch, EPOCH_DURATION_SECS};
use nonos_types::{
    EpochNumber, EpochSummary, EthAddress, NodeTier, NonosError, NonosResult, QualityScore,
    RewardClaim, StakeRecord, TokenAmount, NOX_DECIMALS, EMISSION_DECAY_RATE, Blake3Hash,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

pub struct RewardTracker {
    stake: Arc<RwLock<Option<StakeRecord>>>,
    pending_rewards: Arc<RwLock<TokenAmount>>,
    streak: Arc<RwLock<u32>>,
    claims: Arc<RwLock<Vec<RewardClaim>>>,
    contract_client: Arc<RwLock<Option<Arc<RwLock<ContractClient>>>>>,
    staker_address: Arc<RwLock<Option<EthAddress>>>,
    auto_claim_enabled: Arc<RwLock<bool>>,
    auto_claim_threshold: Arc<RwLock<TokenAmount>>,
}

impl RewardTracker {
    pub fn new() -> Self {
        Self {
            stake: Arc::new(RwLock::new(None)),
            pending_rewards: Arc::new(RwLock::new(TokenAmount::zero(NOX_DECIMALS))),
            streak: Arc::new(RwLock::new(0)),
            claims: Arc::new(RwLock::new(Vec::new())),
            contract_client: Arc::new(RwLock::new(None)),
            staker_address: Arc::new(RwLock::new(None)),
            auto_claim_enabled: Arc::new(RwLock::new(false)),
            auto_claim_threshold: Arc::new(RwLock::new(TokenAmount::from_raw(100 * 10u128.pow(NOX_DECIMALS as u32), NOX_DECIMALS))),
        }
    }

    pub fn with_contract_client(contract_client: Arc<RwLock<ContractClient>>, staker: EthAddress) -> Self {
        Self {
            stake: Arc::new(RwLock::new(None)),
            pending_rewards: Arc::new(RwLock::new(TokenAmount::zero(NOX_DECIMALS))),
            streak: Arc::new(RwLock::new(0)),
            claims: Arc::new(RwLock::new(Vec::new())),
            contract_client: Arc::new(RwLock::new(Some(contract_client))),
            staker_address: Arc::new(RwLock::new(Some(staker))),
            auto_claim_enabled: Arc::new(RwLock::new(false)),
            auto_claim_threshold: Arc::new(RwLock::new(TokenAmount::from_raw(100 * 10u128.pow(NOX_DECIMALS as u32), NOX_DECIMALS))),
        }
    }

    pub async fn set_contract_client(&self, client: Arc<RwLock<ContractClient>>, staker: EthAddress) {
        *self.contract_client.write().await = Some(client);
        *self.staker_address.write().await = Some(staker);
        info!("Contract client configured for staker: {}", staker);
    }

    pub async fn set_stake(&self, stake: StakeRecord) {
        *self.stake.write().await = Some(stake);
    }

    pub async fn staked_amount(&self) -> TokenAmount {
        self.stake
            .read()
            .await
            .as_ref()
            .map(|s| s.amount)
            .unwrap_or_else(|| TokenAmount::zero(NOX_DECIMALS))
    }

    pub async fn pending_rewards(&self) -> TokenAmount {
        *self.pending_rewards.read().await
    }

    pub async fn pending_rewards_onchain(&self) -> NonosResult<TokenAmount> {
        let client_opt = self.contract_client.read().await;
        let client = client_opt.as_ref()
            .ok_or_else(|| NonosError::Network("Contract client not configured".into()))?;

        let staker_opt = self.staker_address.read().await;
        let staker = staker_opt.as_ref()
            .ok_or_else(|| NonosError::Wallet("Staker address not set".into()))?;

        let client = client.read().await;
        client.get_pending_rewards(staker).await
    }

    pub async fn sync_pending_rewards(&self) -> NonosResult<TokenAmount> {
        let onchain = self.pending_rewards_onchain().await?;
        *self.pending_rewards.write().await = onchain;
        info!("Synced pending rewards: {} NOX", onchain.to_decimal());
        Ok(onchain)
    }

    pub async fn current_streak(&self) -> u32 {
        *self.streak.read().await
    }

    pub async fn enable_auto_claim(&self, threshold: TokenAmount) {
        *self.auto_claim_enabled.write().await = true;
        *self.auto_claim_threshold.write().await = threshold;
        info!("Auto-claim enabled with threshold: {} NOX", threshold.to_decimal());
    }

    pub async fn disable_auto_claim(&self) {
        *self.auto_claim_enabled.write().await = false;
        info!("Auto-claim disabled");
    }

    pub async fn check_auto_claim(&self) -> NonosResult<Option<RewardClaim>> {
        let enabled = *self.auto_claim_enabled.read().await;
        if !enabled {
            return Ok(None);
        }

        let threshold = *self.auto_claim_threshold.read().await;
        let pending = self.pending_rewards().await;

        if pending.raw >= threshold.raw {
            info!("Auto-claim triggered: {} NOX >= {} NOX threshold",
                  pending.to_decimal(), threshold.to_decimal());

            let epoch = contract_epoch()
                .map(EpochNumber)
                .unwrap_or_else(|| {
                    warn!("Staking contract not configured, using fallback epoch calculation");
                    EpochNumber(chrono::Utc::now().timestamp() as u64 / EPOCH_DURATION_SECS)
                });
            match self.claim(epoch).await {
                Ok(claim) => Ok(Some(claim)),
                Err(e) => {
                    warn!("Auto-claim failed: {}", e);
                    Err(e)
                }
            }
        } else {
            Ok(None)
        }
    }

    pub async fn calculate_epoch_reward(
        &self,
        epoch: &EpochSummary,
        quality: &QualityScore,
    ) -> TokenAmount {
        let stake = self.stake.read().await;
        let Some(ref stake_record) = *stake else {
            return TokenAmount::zero(NOX_DECIMALS);
        };

        let weight = stake_record.weight();

        let quality_multiplier = quality.total();

        let streak = *self.streak.read().await;
        let streak_bonus = 1.0 + (streak.min(5) as f64 * 0.05);

        let utilization = 1.0;

        let base_reward = epoch.total_emission.raw as f64;
        let weight_ratio = weight / epoch.total_weight.max(1.0);
        let reward = base_reward * weight_ratio * quality_multiplier * utilization * streak_bonus;

        TokenAmount::from_raw(reward as u128, NOX_DECIMALS)
    }

    pub async fn process_epoch(&self, epoch: &EpochSummary, quality: &QualityScore) {
        let reward = self.calculate_epoch_reward(epoch, quality).await;

        let mut pending = self.pending_rewards.write().await;
        if let Some(new_pending) = pending.checked_add(&reward) {
            *pending = new_pending;
        }

        if quality.total() >= 0.8 {
            *self.streak.write().await += 1;
            debug!("Streak incremented, new streak: {}", *self.streak.read().await);
        } else {
            *self.streak.write().await = 0;
            debug!("Streak reset due to low quality score");
        }

        info!(
            "Processed epoch {}: reward={} NOX, streak={}",
            epoch.epoch.0,
            reward.to_decimal(),
            *self.streak.read().await
        );
    }

    pub async fn claim(&self, epoch: EpochNumber) -> NonosResult<RewardClaim> {
        let pending = *self.pending_rewards.read().await;
        if pending.is_zero() {
            return Err(NonosError::Staking("No pending rewards".into()));
        }

        let staker = {
            let staker_opt = self.staker_address.read().await;
            staker_opt.clone().ok_or_else(|| NonosError::Wallet("Staker address not set".into()))?
        };

        let (tx_hash, claimed_amount) = {
            let client_opt = self.contract_client.read().await;

            if let Some(ref client_arc) = *client_opt {
                let client = client_arc.read().await;

                info!("Submitting claim transaction for {} NOX...", pending.to_decimal());

                match client.claim_rewards().await {
                    Ok((hash, amount)) => {
                        info!("Claim transaction confirmed: {:?}", hash);
                        let mut tx_bytes = [0u8; 32];
                        tx_bytes.copy_from_slice(hash.as_bytes());
                        (Blake3Hash(tx_bytes), amount)
                    }
                    Err(e) => {
                        error!("Claim transaction failed: {}", e);
                        return Err(e);
                    }
                }
            } else {
                warn!("No contract client configured - performing local-only claim");
                (Blake3Hash::zero(), pending)
            }
        };

        let claim = RewardClaim {
            epoch,
            claimant: staker,
            amount: claimed_amount,
            claimed_at: chrono::Utc::now(),
            tx_hash,
        };

        *self.pending_rewards.write().await = TokenAmount::zero(NOX_DECIMALS);

        self.claims.write().await.push(claim.clone());

        info!("Successfully claimed {} NOX rewards (tx: {})",
              claimed_amount.to_decimal(),
              hex::encode(&tx_hash.0[..8]));

        Ok(claim)
    }

    pub async fn claim_with_retry(&self, epoch: EpochNumber, max_retries: u32) -> NonosResult<RewardClaim> {
        let mut last_error = None;

        for attempt in 1..=max_retries {
            match self.claim(epoch).await {
                Ok(claim) => return Ok(claim),
                Err(e) => {
                    warn!("Claim attempt {}/{} failed: {}", attempt, max_retries, e);
                    last_error = Some(e);

                    if attempt < max_retries {
                        let delay = std::time::Duration::from_secs(1 << (attempt - 1));
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| NonosError::Staking("Claim failed after retries".into())))
    }

    pub async fn claim_history(&self) -> Vec<RewardClaim> {
        self.claims.read().await.clone()
    }
}

impl Default for RewardTracker {
    fn default() -> Self {
        Self::new()
    }
}

pub fn calculate_epoch_emission(year: u32, daily_emission_year_1: u64) -> u64 {
    let decay_factor = (1.0 - EMISSION_DECAY_RATE).powi(year.saturating_sub(1) as i32);
    (daily_emission_year_1 as f64 * decay_factor) as u64
}

pub fn calculate_stake_weight(stake: u64, tier: NodeTier) -> f64 {
    (stake as f64).sqrt() * tier.multiplier()
}

pub fn calculate_expected_apy(
    stake: u64,
    tier: NodeTier,
    total_staked: u64,
    epoch_emission: u64,
) -> f64 {
    if stake == 0 || total_staked == 0 {
        return 0.0;
    }

    let weight = calculate_stake_weight(stake, tier);

    let quality = 0.9;
    let streak_bonus = 1.0;

    let total_weight = calculate_stake_weight(total_staked, NodeTier::Bronze);
    let epoch_reward = (epoch_emission as f64) * (weight / total_weight) * quality * streak_bonus;

    let annual_reward = epoch_reward * 365.0;

    (annual_reward / stake as f64) * 100.0
}

pub struct SlashingCalculator;

impl SlashingCalculator {
    pub fn calculate_penalty(downtime_hours: u64, stake: u64) -> u64 {
        if downtime_hours <= 72 {
            return 0;
        }

        let slash_periods = (downtime_hours - 72) / 24;
        let slash_rate = 0.05 * slash_periods as f64;
        let slash_amount = (stake as f64 * slash_rate.min(0.50)) as u64;

        slash_amount
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_reward_calculation() {
        let tracker = RewardTracker::new();

        let stake = StakeRecord {
            staker: nonos_types::EthAddress::zero(),
            node_id: None,
            amount: TokenAmount::from_raw(10_000_000_000_000_000_000_000, NOX_DECIMALS),
            tier: NodeTier::Silver,
            lock_start: chrono::Utc::now(),
            lock_end: chrono::Utc::now() + chrono::Duration::days(30),
            is_locked: true,
        };
        tracker.set_stake(stake).await;

        let epoch = EpochSummary {
            epoch: EpochNumber(1),
            start_time: chrono::Utc::now(),
            end_time: chrono::Utc::now() + chrono::Duration::hours(24),
            total_emission: TokenAmount::from_raw(100_000_000_000_000_000_000_000, NOX_DECIMALS),
            total_weight: 1000.0,
            staker_count: 100,
            avg_quality: 0.9,
        };

        let quality = QualityScore::perfect();
        let reward = tracker.calculate_epoch_reward(&epoch, &quality).await;

        assert!(!reward.is_zero());
    }

    #[test]
    fn test_epoch_emission_decay() {
        let year_1 = calculate_epoch_emission(1, 100_000);
        let year_2 = calculate_epoch_emission(2, 100_000);
        let year_3 = calculate_epoch_emission(3, 100_000);

        assert_eq!(year_1, 100_000);
        assert!(year_2 < year_1);
        assert!(year_3 < year_2);

        assert!((year_2 as f64 / year_1 as f64 - 0.85).abs() < 0.01);
    }

    #[test]
    fn test_stake_weight() {
        let weight_bronze = calculate_stake_weight(10_000, NodeTier::Bronze);
        let weight_gold = calculate_stake_weight(10_000, NodeTier::Gold);

        assert!(weight_gold > weight_bronze);
    }

    #[test]
    fn test_slashing() {
        assert_eq!(SlashingCalculator::calculate_penalty(24, 10_000), 0);
        assert_eq!(SlashingCalculator::calculate_penalty(72, 10_000), 0);

        let penalty = SlashingCalculator::calculate_penalty(96, 10_000);
        assert_eq!(penalty, 500);
    }
}
