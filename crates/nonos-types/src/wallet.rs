use crate::address::EthAddress;
use crate::constants::NOX_DECIMALS;
use crate::crypto::Blake3Hash;
use crate::error::{NonosError, NonosResult};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WalletId(pub uuid::Uuid);

impl WalletId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl std::str::FromStr for WalletId {
    type Err = NonosError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let uuid = uuid::Uuid::parse_str(s).map_err(|e| NonosError::Wallet(e.to_string()))?;
        Ok(Self(uuid))
    }
}

impl Default for WalletId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for WalletId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WalletId({})", self.0)
    }
}

impl fmt::Display for WalletId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WalletMetadata {
    pub id: WalletId,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_accessed: chrono::DateTime<chrono::Utc>,
    pub address: EthAddress,
    pub stealth_count: u32,
}

impl WalletMetadata {
    pub fn new(name: String, address: EthAddress) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: WalletId::new(),
            name,
            created_at: now,
            last_accessed: now,
            address,
            stealth_count: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TokenAmount {
    pub raw: u128,
    pub decimals: u8,
}

impl Default for TokenAmount {
    fn default() -> Self {
        Self::zero(NOX_DECIMALS)
    }
}

impl TokenAmount {
    pub fn from_raw(raw: u128, decimals: u8) -> Self {
        Self { raw, decimals }
    }

    pub fn from_decimal(s: &str, decimals: u8) -> NonosResult<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() > 2 {
            return Err(NonosError::Transaction("Invalid decimal format".into()));
        }

        let whole: u128 = parts[0]
            .parse()
            .map_err(|_| NonosError::Transaction("Invalid number".into()))?;

        let frac = if parts.len() == 2 {
            let frac_str = parts[1];
            if frac_str.len() > decimals as usize {
                return Err(NonosError::Transaction("Too many decimal places".into()));
            }
            let padded = format!("{:0<width$}", frac_str, width = decimals as usize);
            padded
                .parse::<u128>()
                .map_err(|_| NonosError::Transaction("Invalid fraction".into()))?
        } else {
            0
        };

        let multiplier = 10u128.pow(decimals as u32);
        let raw = whole
            .checked_mul(multiplier)
            .and_then(|w| w.checked_add(frac))
            .ok_or_else(|| NonosError::Transaction("Amount overflow".into()))?;

        Ok(Self { raw, decimals })
    }

    pub fn to_decimal(&self) -> String {
        let multiplier = 10u128.pow(self.decimals as u32);
        let whole = self.raw / multiplier;
        let frac = self.raw % multiplier;

        if frac == 0 {
            whole.to_string()
        } else {
            let frac_str = format!("{:0>width$}", frac, width = self.decimals as usize);
            let trimmed = frac_str.trim_end_matches('0');
            format!("{}.{}", whole, trimmed)
        }
    }

    pub fn nox(amount: &str) -> NonosResult<Self> {
        Self::from_decimal(amount, NOX_DECIMALS)
    }

    pub fn zero(decimals: u8) -> Self {
        Self { raw: 0, decimals }
    }

    pub fn is_zero(&self) -> bool {
        self.raw == 0
    }

    pub fn checked_add(&self, other: &Self) -> Option<Self> {
        if self.decimals != other.decimals {
            return None;
        }
        self.raw.checked_add(other.raw).map(|raw| Self {
            raw,
            decimals: self.decimals,
        })
    }

    pub fn checked_sub(&self, other: &Self) -> Option<Self> {
        if self.decimals != other.decimals {
            return None;
        }
        self.raw.checked_sub(other.raw).map(|raw| Self {
            raw,
            decimals: self.decimals,
        })
    }
}

impl fmt::Display for TokenAmount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_decimal())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Failed,
    Dropped,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionRecord {
    pub hash: Blake3Hash,
    pub from: EthAddress,
    pub to: EthAddress,
    pub amount: TokenAmount,
    pub gas_price: u128,
    pub gas_limit: u64,
    pub nonce: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub status: TransactionStatus,
    pub block_number: Option<u64>,
    pub is_private: bool,
}
