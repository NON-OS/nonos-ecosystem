use nonos_types::{NodeTier, TokenAmount, NOX_DECIMALS};
use serde::{Deserialize, Serialize};

pub const NODE_INCENTIVES_POOL: u128 = 200_000_000;

pub const STAKING_REWARDS_POOL: u128 = 32_000_000;

pub const TOTAL_EMISSION_POOL: u128 = NODE_INCENTIVES_POOL + STAKING_REWARDS_POOL;

pub const EMISSION_DURATION_DAYS: u64 = 1095;

pub const YEAR_1_DAILY_EMISSION: u128 = 150_000;

pub const YEARLY_DECAY_RATE: f64 = 0.30;

pub const DECAY_MULTIPLIER: f64 = 0.70;

pub const MIN_DAILY_EMISSION: u128 = 10_000;

pub const EPOCH_DURATION_DAYS: u64 = 7;

pub const EPOCHS_PER_YEAR: u64 = 52;

pub const MAX_STREAK_BONUS: f64 = 0.25;

pub const STREAK_BONUS_PER_EPOCH: f64 = 0.05;

pub const GOOD_EPOCH_QUALITY_THRESHOLD: f64 = 0.80;

pub fn calculate_daily_emission(day_since_genesis: u64) -> u128 {
    let years_elapsed = day_since_genesis as f64 / 365.0;
    let decay_factor = DECAY_MULTIPLIER.powf(years_elapsed);
    let emission = (YEAR_1_DAILY_EMISSION as f64 * decay_factor) as u128;
    emission.max(MIN_DAILY_EMISSION)
}

pub fn calculate_epoch_emission(epoch_number: u64) -> u128 {
    let start_day = epoch_number * EPOCH_DURATION_DAYS;
    let mut total = 0u128;

    for day in start_day..(start_day + EPOCH_DURATION_DAYS) {
        total += calculate_daily_emission(day);
    }

    total
}

pub fn calculate_yearly_emission(year: u32) -> u128 {
    let start_day = (year as u64 - 1) * 365;
    let mut total = 0u128;

    for day in start_day..(start_day + 365) {
        total += calculate_daily_emission(day);
    }

    total
}

pub fn calculate_cumulative_emission(day: u64) -> u128 {
    let mut total = 0u128;
    for d in 0..day {
        total += calculate_daily_emission(d);
    }
    total
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmissionSchedule {
    pub year_1_total: u128,
    pub year_2_total: u128,
    pub year_3_total: u128,
    pub three_year_total: u128,
    pub remaining_after_3_years: u128,
    pub estimated_years_until_exhaustion: f64,
}

impl EmissionSchedule {
    pub fn generate() -> Self {
        let year_1 = calculate_yearly_emission(1);
        let year_2 = calculate_yearly_emission(2);
        let year_3 = calculate_yearly_emission(3);
        let three_year = year_1 + year_2 + year_3;

        let pool_with_decimals = TOTAL_EMISSION_POOL * 10u128.pow(NOX_DECIMALS as u32);
        let three_year_with_decimals = three_year * 10u128.pow(NOX_DECIMALS as u32);

        let remaining = pool_with_decimals.saturating_sub(three_year_with_decimals);

        let year_3_daily = calculate_daily_emission(730);
        let days_remaining = if year_3_daily > 0 {
            remaining / (year_3_daily * 10u128.pow(NOX_DECIMALS as u32))
        } else {
            u128::MAX
        };
        let years_remaining = days_remaining as f64 / 365.0;

        Self {
            year_1_total: year_1,
            year_2_total: year_2,
            year_3_total: year_3,
            three_year_total: three_year,
            remaining_after_3_years: remaining / 10u128.pow(NOX_DECIMALS as u32),
            estimated_years_until_exhaustion: 3.0 + years_remaining,
        }
    }

    pub fn to_summary(&self) -> String {
        format!(
            "NOX Emission Schedule (3-Year Plan)\n\
             =====================================\n\
             Year 1: {:>12} NOX ({:.1}% of pool)\n\
             Year 2: {:>12} NOX ({:.1}% of pool)\n\
             Year 3: {:>12} NOX ({:.1}% of pool)\n\
             ------------------------------------\n\
             3-Year Total: {:>8} NOX\n\
             Remaining:    {:>8} NOX\n\
             Est. Duration: {:.1} years\n",
            format_with_commas(self.year_1_total),
            (self.year_1_total as f64 / TOTAL_EMISSION_POOL as f64) * 100.0,
            format_with_commas(self.year_2_total),
            (self.year_2_total as f64 / TOTAL_EMISSION_POOL as f64) * 100.0,
            format_with_commas(self.year_3_total),
            (self.year_3_total as f64 / TOTAL_EMISSION_POOL as f64) * 100.0,
            format_with_commas(self.three_year_total),
            format_with_commas(self.remaining_after_3_years),
            self.estimated_years_until_exhaustion,
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RewardParams {
    pub staked_amount: u64,
    pub tier: NodeTier,
    pub quality_score: f64,
    pub streak: u32,
    pub total_network_stake: u64,
    pub total_network_weight: f64,
    pub current_epoch: u64,
}

pub fn calculate_staker_reward(params: &RewardParams) -> TokenAmount {
    if params.staked_amount == 0 || params.total_network_weight == 0.0 {
        return TokenAmount::zero(NOX_DECIMALS);
    }

    let effective_stake = (params.staked_amount as f64).sqrt() * params.tier.multiplier();

    let stake_share = effective_stake / params.total_network_weight;

    let quality_factor = params.quality_score.clamp(0.0, 1.0);

    let streak_bonus = 1.0 + (params.streak.min(5) as f64 * STREAK_BONUS_PER_EPOCH);

    let epoch_emission = calculate_epoch_emission(params.current_epoch);

    let reward = epoch_emission as f64 * stake_share * quality_factor * streak_bonus;

    let raw_reward = (reward * 10f64.powi(NOX_DECIMALS as i32)) as u128;
    TokenAmount::from_raw(raw_reward, NOX_DECIMALS)
}

pub fn calculate_effective_stake(staked_amount: u64, tier: NodeTier) -> f64 {
    (staked_amount as f64).sqrt() * tier.multiplier()
}

pub fn calculate_expected_apy(params: &RewardParams) -> f64 {
    if params.staked_amount == 0 {
        return 0.0;
    }

    let epoch_reward = calculate_staker_reward(params);
    let epoch_reward_nox = epoch_reward.raw as f64 / 10f64.powi(NOX_DECIMALS as i32);

    let annual_reward = epoch_reward_nox * EPOCHS_PER_YEAR as f64;

    (annual_reward / params.staked_amount as f64) * 100.0
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TierBenefits {
    pub name: &'static str,
    pub min_stake: u64,
    pub lock_days: u32,
    pub multiplier: f64,
    pub apy_range: (f64, f64),
    pub priority: u8,
    pub max_connections: u32,
}

impl TierBenefits {
    pub fn for_tier(tier: NodeTier) -> Self {
        match tier {
            NodeTier::Bronze => Self {
                name: "Bronze",
                min_stake: 1_000,
                lock_days: 0,
                multiplier: 1.0,
                apy_range: (5.0, 10.0),
                priority: 1,
                max_connections: 100,
            },
            NodeTier::Silver => Self {
                name: "Silver",
                min_stake: 10_000,
                lock_days: 30,
                multiplier: 1.5,
                apy_range: (10.0, 15.0),
                priority: 2,
                max_connections: 500,
            },
            NodeTier::Gold => Self {
                name: "Gold",
                min_stake: 50_000,
                lock_days: 90,
                multiplier: 2.0,
                apy_range: (15.0, 22.0),
                priority: 3,
                max_connections: 1_000,
            },
            NodeTier::Platinum => Self {
                name: "Platinum",
                min_stake: 200_000,
                lock_days: 180,
                multiplier: 2.5,
                apy_range: (22.0, 30.0),
                priority: 4,
                max_connections: 2_500,
            },
            NodeTier::Diamond => Self {
                name: "Diamond",
                min_stake: 1_000_000,
                lock_days: 365,
                multiplier: 3.0,
                apy_range: (30.0, 40.0),
                priority: 5,
                max_connections: 10_000,
            },
        }
    }

    pub fn all_tiers() -> Vec<Self> {
        vec![
            Self::for_tier(NodeTier::Bronze),
            Self::for_tier(NodeTier::Silver),
            Self::for_tier(NodeTier::Gold),
            Self::for_tier(NodeTier::Platinum),
            Self::for_tier(NodeTier::Diamond),
        ]
    }

    pub fn to_markdown_table() -> String {
        let mut table = String::from(
            "| Tier | Min Stake | Lock Period | Multiplier | Est. APY | Priority |\n\
             |------|-----------|-------------|------------|----------|----------|\n"
        );

        for t in Self::all_tiers() {
            let min_stake_formatted = format_with_commas(t.min_stake as u128);
            table.push_str(&format!(
                "| {} | {} NOX | {} days | {}x | {:.0}-{:.0}% | {} |\n",
                t.name, min_stake_formatted, t.lock_days, t.multiplier,
                t.apy_range.0, t.apy_range.1, t.priority
            ));
        }

        table
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NetworkEmissionState {
    pub total_staked: u128,
    pub active_nodes: u32,
    pub total_effective_weight: f64,
    pub avg_quality_score: f64,
    pub utilization: f64,
    pub current_epoch: u64,
    pub total_distributed: u128,
    pub pool_remaining: u128,
}

impl NetworkEmissionState {
    pub fn new() -> Self {
        Self {
            total_staked: 0,
            active_nodes: 0,
            total_effective_weight: 0.0,
            avg_quality_score: 0.0,
            utilization: 0.0,
            current_epoch: 0,
            total_distributed: 0,
            pool_remaining: TOTAL_EMISSION_POOL * 10u128.pow(NOX_DECIMALS as u32),
        }
    }

    pub fn adjusted_epoch_emission(&self) -> u128 {
        let base_emission = calculate_epoch_emission(self.current_epoch);

        let utilization_factor = if self.utilization < 0.5 {
            0.5 + self.utilization
        } else {
            1.0
        };

        let adjusted = (base_emission as f64 * utilization_factor) as u128;

        let max_emission = self.pool_remaining / (EPOCH_DURATION_DAYS as u128);
        adjusted.min(max_emission / 10u128.pow(NOX_DECIMALS as u32))
    }

    pub fn record_distribution(&mut self, amount: u128) {
        self.total_distributed += amount;
        self.pool_remaining = self.pool_remaining.saturating_sub(amount);
    }

    pub fn is_pool_exhausted(&self) -> bool {
        self.pool_remaining < MIN_DAILY_EMISSION * 10u128.pow(NOX_DECIMALS as u32)
    }

    pub fn estimated_days_remaining(&self) -> u64 {
        if self.current_epoch == 0 {
            return EMISSION_DURATION_DAYS * 2;
        }

        let avg_daily_distribution = self.total_distributed / (self.current_epoch * EPOCH_DURATION_DAYS) as u128;
        if avg_daily_distribution == 0 {
            return u64::MAX;
        }

        (self.pool_remaining / avg_daily_distribution) as u64
    }
}

fn format_with_commas(n: u128) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let mut count = 0;

    for c in s.chars().rev() {
        if count > 0 && count % 3 == 0 {
            result.insert(0, ',');
        }
        result.insert(0, c);
        count += 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daily_emission_decay() {
        let day_0 = calculate_daily_emission(0);
        let day_365 = calculate_daily_emission(365);
        let day_730 = calculate_daily_emission(730);

        assert!((day_0 as i128 - YEAR_1_DAILY_EMISSION as i128).abs() < 1000);

        let ratio_1 = day_365 as f64 / day_0 as f64;
        assert!((ratio_1 - 0.70).abs() < 0.01);

        let ratio_2 = day_730 as f64 / day_0 as f64;
        assert!((ratio_2 - 0.49).abs() < 0.02);
    }

    #[test]
    fn test_3_year_schedule() {
        let schedule = EmissionSchedule::generate();

        assert!(schedule.year_1_total > schedule.year_2_total);
        assert!(schedule.year_2_total > schedule.year_3_total);

        assert!(schedule.three_year_total < TOTAL_EMISSION_POOL);

        assert!(schedule.remaining_after_3_years > 0);

        assert!(schedule.estimated_years_until_exhaustion > 3.0);

        println!("{}", schedule.to_summary());
    }

    #[test]
    fn test_effective_stake_calculation() {
        let bronze = calculate_effective_stake(10_000, NodeTier::Bronze);
        let gold = calculate_effective_stake(10_000, NodeTier::Gold);
        let diamond = calculate_effective_stake(10_000, NodeTier::Diamond);

        assert!(diamond > gold);
        assert!(gold > bronze);

        assert!((bronze - 100.0).abs() < 0.1);
        assert!((gold - 200.0).abs() < 0.1);
        assert!((diamond - 300.0).abs() < 0.1);
    }

    #[test]
    fn test_anti_whale_mechanism() {
        let small_stake = calculate_effective_stake(10_000, NodeTier::Bronze);
        let large_stake = calculate_effective_stake(100_000, NodeTier::Bronze);

        let ratio = large_stake / small_stake;
        assert!((ratio - 3.16).abs() < 0.1);
    }

    #[test]
    fn test_reward_calculation() {
        let params = RewardParams {
            staked_amount: 10_000,
            tier: NodeTier::Silver,
            quality_score: 0.95,
            streak: 3,
            total_network_stake: 1_000_000,
            total_network_weight: 10_000.0,
            current_epoch: 0,
        };

        let reward = calculate_staker_reward(&params);
        assert!(!reward.is_zero());

        let mut params_low_quality = params.clone();
        params_low_quality.quality_score = 0.5;
        let reward_low = calculate_staker_reward(&params_low_quality);
        assert!(reward.raw > reward_low.raw);
    }

    #[test]
    fn test_tier_benefits_table() {
        let table = TierBenefits::to_markdown_table();
        assert!(table.contains("Bronze"));
        assert!(table.contains("Diamond"));
        assert!(table.contains("1,000,000 NOX"));

        println!("{}", table);
    }

    #[test]
    fn test_minimum_emission_floor() {
        let far_future = calculate_daily_emission(3650);
        assert!(far_future >= MIN_DAILY_EMISSION);
    }

    #[test]
    fn test_network_state_adjustments() {
        let mut state = NetworkEmissionState::new();
        state.current_epoch = 52;
        state.utilization = 0.3;

        let base_emission = calculate_epoch_emission(52);
        let adjusted = state.adjusted_epoch_emission();

        assert!(adjusted < base_emission);
    }
}
