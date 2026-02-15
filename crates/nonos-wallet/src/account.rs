use nonos_types::{EthAddress, NonosError, NonosResult, TokenAmount};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Account {
    pub index: u32,
    pub address: EthAddress,
    pub label: Option<String>,
    pub eth_balance: TokenAmount,
    pub nox_balance: TokenAmount,
    pub is_primary: bool,
}

impl Account {
    pub fn new(index: u32, address: EthAddress) -> Self {
        Self {
            index,
            address,
            label: None,
            eth_balance: TokenAmount::zero(18),
            nox_balance: TokenAmount::zero(18),
            is_primary: index == 0,
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn update_balances(&mut self, eth: TokenAmount, nox: TokenAmount) {
        self.eth_balance = eth;
        self.nox_balance = nox;
    }

    pub fn display_name(&self) -> String {
        self.label.clone().unwrap_or_else(|| format!("Account {}", self.index))
    }
}

pub struct AccountManager {
    accounts: HashMap<u32, Account>,
    next_index: u32,
}

impl AccountManager {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            next_index: 0,
        }
    }

    pub fn add_account(&mut self, account: Account) {
        let index = account.index;
        self.accounts.insert(index, account);
        if index >= self.next_index {
            self.next_index = index + 1;
        }
    }

    pub fn get(&self, index: u32) -> Option<&Account> {
        self.accounts.get(&index)
    }

    pub fn get_mut(&mut self, index: u32) -> Option<&mut Account> {
        self.accounts.get_mut(&index)
    }

    pub fn get_by_address(&self, address: &EthAddress) -> Option<&Account> {
        self.accounts.values().find(|a| &a.address == address)
    }

    pub fn all(&self) -> impl Iterator<Item = &Account> {
        self.accounts.values()
    }

    pub fn primary(&self) -> Option<&Account> {
        self.accounts.values().find(|a| a.is_primary)
    }

    pub fn next_index(&self) -> u32 {
        self.next_index
    }

    pub fn count(&self) -> usize {
        self.accounts.len()
    }

    pub fn remove(&mut self, index: u32) -> Option<Account> {
        if let Some(account) = self.accounts.get(&index) {
            if account.is_primary {
                return None;
            }
        }
        self.accounts.remove(&index)
    }

    pub fn set_primary(&mut self, index: u32) -> NonosResult<()> {
        if !self.accounts.contains_key(&index) {
            return Err(NonosError::Wallet("Account not found".into()));
        }

        for account in self.accounts.values_mut() {
            account.is_primary = account.index == index;
        }

        Ok(())
    }
}

impl Default for AccountManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TokenBalances {
    pub eth: TokenAmount,
    pub nox: TokenAmount,
    pub staked_nox: TokenAmount,
    pub pending_rewards: TokenAmount,
}

impl TokenBalances {
    pub fn zero() -> Self {
        Self {
            eth: TokenAmount::zero(18),
            nox: TokenAmount::zero(18),
            staked_nox: TokenAmount::zero(18),
            pending_rewards: TokenAmount::zero(18),
        }
    }

    pub fn total_nox(&self) -> TokenAmount {
        self.nox.checked_add(&self.staked_nox).unwrap_or(self.nox)
    }

    pub fn net_worth_nox(&self, eth_to_nox_rate: f64) -> f64 {
        let eth_value = (self.eth.raw as f64) * eth_to_nox_rate / 1e18;
        let nox_value = (self.nox.raw as f64) / 1e18;
        let staked_value = (self.staked_nox.raw as f64) / 1e18;
        let rewards_value = (self.pending_rewards.raw as f64) / 1e18;

        eth_value + nox_value + staked_value + rewards_value
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StakingPosition {
    pub amount: TokenAmount,
    pub lock_start: chrono::DateTime<chrono::Utc>,
    pub lock_end: chrono::DateTime<chrono::Utc>,
    pub tier: nonos_types::NodeTier,
    pub accumulated_rewards: TokenAmount,
}

impl StakingPosition {
    pub fn is_locked(&self) -> bool {
        chrono::Utc::now() < self.lock_end
    }

    pub fn days_remaining(&self) -> i64 {
        let duration = self.lock_end.signed_duration_since(chrono::Utc::now());
        duration.num_days().max(0)
    }

    pub fn current_apy(&self) -> f64 {
        let (min_apy, max_apy) = self.tier.apy_range();
        (min_apy as f64 + max_apy as f64) / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_manager() {
        let mut manager = AccountManager::new();

        let account0 = Account::new(0, EthAddress::from_bytes([0xaa; 20]));
        let account1 = Account::new(1, EthAddress::from_bytes([0xbb; 20]));

        manager.add_account(account0);
        manager.add_account(account1);

        assert_eq!(manager.count(), 2);
        assert!(manager.primary().is_some());
        assert_eq!(manager.primary().unwrap().index, 0);
    }

    #[test]
    fn test_set_primary() {
        let mut manager = AccountManager::new();

        manager.add_account(Account::new(0, EthAddress::from_bytes([0xaa; 20])));
        manager.add_account(Account::new(1, EthAddress::from_bytes([0xbb; 20])));

        manager.set_primary(1).unwrap();
        assert_eq!(manager.primary().unwrap().index, 1);
    }

    #[test]
    fn test_token_balances() {
        let mut balances = TokenBalances::zero();

        balances.eth = TokenAmount::from_raw(1_000_000_000_000_000_000, 18);
        balances.nox = TokenAmount::from_raw(100_000_000_000_000_000_000, 18);
        balances.staked_nox = TokenAmount::from_raw(50_000_000_000_000_000_000, 18);

        let total = balances.total_nox();
        assert_eq!(total.raw, 150_000_000_000_000_000_000);
    }
}
