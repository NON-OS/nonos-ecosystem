#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::fmt;
use thiserror::Error;

pub const BLAKE3_HASH_SIZE: usize = 32;

pub const BLAKE3_KEY_SIZE: usize = 32;

pub const SECP256K1_PRIVATE_KEY_SIZE: usize = 32;

pub const SECP256K1_PUBLIC_KEY_SIZE: usize = 33;

pub const SECP256K1_PUBLIC_KEY_UNCOMPRESSED_SIZE: usize = 65;

pub const SECP256K1_SIGNATURE_SIZE: usize = 72;

pub const ED25519_PRIVATE_KEY_SIZE: usize = 32;

pub const ED25519_PUBLIC_KEY_SIZE: usize = 32;

pub const ED25519_SIGNATURE_SIZE: usize = 64;

pub const ETH_ADDRESS_SIZE: usize = 20;

pub const MNEMONIC_WORD_COUNT: usize = 24;

pub const NOX_DECIMALS: u8 = 18;

pub const NOX_TOTAL_SUPPLY: u64 = 800_000_000;

pub const NOX_STAKING_POOL: u64 = 32_000_000;

pub const NOX_CONTRIBUTOR_POOL: u64 = 24_000_000;

pub const EPOCH_DURATION_SECS: u64 = 86_400;

pub const EMISSION_DECAY_RATE: f64 = 0.15;

pub const ANYONE_CIRCUIT_LENGTH: usize = 3;

pub const ANYONE_CIRCUIT_ROTATION_SECS: u64 = 600;

#[derive(Error, Debug)]
pub enum NonosError {
    #[error("Cryptographic error: {0}")]
    Crypto(String),

    #[error("Key derivation error: {0}")]
    KeyDerivation(String),

    #[error("Invalid key format: {0}")]
    InvalidKey(String),

    #[error("Invalid signature: {0}")]
    InvalidSignature(String),

    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    #[error("Invalid mnemonic: {0}")]
    InvalidMnemonic(String),

    #[error("Wallet error: {0}")]
    Wallet(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Circuit error: {0}")]
    Circuit(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Staking error: {0}")]
    Staking(String),

    #[error("Transaction error: {0}")]
    Transaction(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Contract error: {0}")]
    Contract(String),
}

pub type NonosResult<T> = Result<T, NonosError>;

#[deprecated(since = "1.0.0", note = "Use NonosError instead")]
pub type NoxoneError = NonosError;

#[deprecated(since = "1.0.0", note = "Use NonosResult instead")]
pub type NoxoneResult<T> = NonosResult<T>;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Blake3Hash(pub [u8; BLAKE3_HASH_SIZE]);

impl Blake3Hash {
    pub fn from_bytes(bytes: [u8; BLAKE3_HASH_SIZE]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; BLAKE3_HASH_SIZE] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(s: &str) -> NonosResult<Self> {
        let bytes = hex::decode(s).map_err(|e| NonosError::Crypto(e.to_string()))?;
        if bytes.len() != BLAKE3_HASH_SIZE {
            return Err(NonosError::Crypto("Invalid hash length".into()));
        }
        let mut arr = [0u8; BLAKE3_HASH_SIZE];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }

    pub fn zero() -> Self {
        Self([0u8; BLAKE3_HASH_SIZE])
    }
}

impl fmt::Debug for Blake3Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Blake3Hash({})", self.to_hex())
    }
}

impl fmt::Display for Blake3Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl Default for Blake3Hash {
    fn default() -> Self {
        Self::zero()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Blake3Key(pub [u8; BLAKE3_KEY_SIZE]);

impl Blake3Key {
    pub fn from_bytes(bytes: [u8; BLAKE3_KEY_SIZE]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; BLAKE3_KEY_SIZE] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(s: &str) -> NonosResult<Self> {
        let bytes = hex::decode(s).map_err(|e| NonosError::Crypto(e.to_string()))?;
        if bytes.len() != BLAKE3_KEY_SIZE {
            return Err(NonosError::Crypto("Invalid key length".into()));
        }
        let mut arr = [0u8; BLAKE3_KEY_SIZE];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

impl fmt::Debug for Blake3Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Blake3Key({}...)", &self.to_hex()[..8])
    }
}

impl Drop for Blake3Key {
    fn drop(&mut self) {
        self.0.iter_mut().for_each(|b| *b = 0);
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Secp256k1PrivateKey(pub [u8; SECP256K1_PRIVATE_KEY_SIZE]);

impl Secp256k1PrivateKey {
    pub fn from_bytes(bytes: [u8; SECP256K1_PRIVATE_KEY_SIZE]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; SECP256K1_PRIVATE_KEY_SIZE] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(s: &str) -> NonosResult<Self> {
        let bytes = hex::decode(s).map_err(|e| NonosError::InvalidKey(e.to_string()))?;
        if bytes.len() != SECP256K1_PRIVATE_KEY_SIZE {
            return Err(NonosError::InvalidKey("Invalid private key length".into()));
        }
        let mut arr = [0u8; SECP256K1_PRIVATE_KEY_SIZE];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

impl fmt::Debug for Secp256k1PrivateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Secp256k1PrivateKey([REDACTED])")
    }
}

impl Drop for Secp256k1PrivateKey {
    fn drop(&mut self) {
        self.0.iter_mut().for_each(|b| *b = 0);
    }
}

#[serde_as]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Secp256k1PublicKey(#[serde_as(as = "serde_with::Bytes")] pub [u8; SECP256K1_PUBLIC_KEY_SIZE]);

impl Secp256k1PublicKey {
    pub fn from_bytes(bytes: [u8; SECP256K1_PUBLIC_KEY_SIZE]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; SECP256K1_PUBLIC_KEY_SIZE] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(s: &str) -> NonosResult<Self> {
        let bytes = hex::decode(s).map_err(|e| NonosError::InvalidKey(e.to_string()))?;
        if bytes.len() != SECP256K1_PUBLIC_KEY_SIZE {
            return Err(NonosError::InvalidKey("Invalid public key length".into()));
        }
        let mut arr = [0u8; SECP256K1_PUBLIC_KEY_SIZE];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

impl fmt::Debug for Secp256k1PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Secp256k1PublicKey({})", self.to_hex())
    }
}

impl fmt::Display for Secp256k1PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Ed25519PrivateKey(pub [u8; ED25519_PRIVATE_KEY_SIZE]);

impl Ed25519PrivateKey {
    pub fn from_bytes(bytes: [u8; ED25519_PRIVATE_KEY_SIZE]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; ED25519_PRIVATE_KEY_SIZE] {
        &self.0
    }
}

impl fmt::Debug for Ed25519PrivateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ed25519PrivateKey([REDACTED])")
    }
}

impl Drop for Ed25519PrivateKey {
    fn drop(&mut self) {
        self.0.iter_mut().for_each(|b| *b = 0);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Ed25519PublicKey(pub [u8; ED25519_PUBLIC_KEY_SIZE]);

impl Ed25519PublicKey {
    pub fn from_bytes(bytes: [u8; ED25519_PUBLIC_KEY_SIZE]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; ED25519_PUBLIC_KEY_SIZE] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl fmt::Debug for Ed25519PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ed25519PublicKey({})", self.to_hex())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct EcdsaSignature {
    pub r: [u8; 32],
    pub s: [u8; 32],
    pub v: u8,
}

impl EcdsaSignature {
    pub fn new(r: [u8; 32], s: [u8; 32], v: u8) -> Self {
        Self { r, s, v }
    }

    pub fn to_bytes(&self) -> [u8; 65] {
        let mut bytes = [0u8; 65];
        bytes[..32].copy_from_slice(&self.r);
        bytes[32..64].copy_from_slice(&self.s);
        bytes[64] = self.v;
        bytes
    }

    pub fn from_bytes(bytes: &[u8; 65]) -> Self {
        let mut r = [0u8; 32];
        let mut s = [0u8; 32];
        r.copy_from_slice(&bytes[..32]);
        s.copy_from_slice(&bytes[32..64]);
        Self {
            r,
            s,
            v: bytes[64],
        }
    }
}

impl fmt::Debug for EcdsaSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EcdsaSignature(v={})", self.v)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EthAddress(pub [u8; ETH_ADDRESS_SIZE]);

impl EthAddress {
    pub fn from_bytes(bytes: [u8; ETH_ADDRESS_SIZE]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; ETH_ADDRESS_SIZE] {
        &self.0
    }

    pub fn to_checksum(&self) -> String {
        let hex_addr = hex::encode(self.0);
        let hash = blake3::hash(hex_addr.as_bytes());
        let hash_hex = hex::encode(hash.as_bytes());

        let mut checksummed = String::with_capacity(42);
        checksummed.push_str("0x");

        for (i, c) in hex_addr.chars().enumerate() {
            if c.is_ascii_alphabetic() {
                let hash_char = hash_hex.chars().nth(i).unwrap_or('0');
                if hash_char >= '8' {
                    checksummed.push(c.to_ascii_uppercase());
                } else {
                    checksummed.push(c.to_ascii_lowercase());
                }
            } else {
                checksummed.push(c);
            }
        }
        checksummed
    }

    pub fn to_hex(&self) -> String {
        format!("0x{}", hex::encode(self.0))
    }

    pub fn from_hex(s: &str) -> NonosResult<Self> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        let bytes = hex::decode(s).map_err(|e| NonosError::InvalidAddress(e.to_string()))?;
        if bytes.len() != ETH_ADDRESS_SIZE {
            return Err(NonosError::InvalidAddress("Invalid address length".into()));
        }
        let mut arr = [0u8; ETH_ADDRESS_SIZE];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }

    pub fn zero() -> Self {
        Self([0u8; ETH_ADDRESS_SIZE])
    }
}

impl fmt::Debug for EthAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EthAddress({})", self.to_checksum())
    }
}

impl fmt::Display for EthAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_checksum())
    }
}

impl Default for EthAddress {
    fn default() -> Self {
        Self::zero()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct StealthAddress {
    pub ephemeral_pubkey: Secp256k1PublicKey,
    pub view_tag: [u8; 4],
    pub meta_hash: Blake3Hash,
}

impl StealthAddress {
    pub fn new(
        ephemeral_pubkey: Secp256k1PublicKey,
        view_tag: [u8; 4],
        meta_hash: Blake3Hash,
    ) -> Self {
        Self {
            ephemeral_pubkey,
            view_tag,
            meta_hash,
        }
    }

    pub fn encode(&self) -> String {
        format!(
            "st:{}:{}:{}",
            self.ephemeral_pubkey.to_hex(),
            hex::encode(self.view_tag),
            self.meta_hash.to_hex()
        )
    }

    pub fn decode(s: &str) -> NonosResult<Self> {
        let s = s.strip_prefix("st:").ok_or_else(|| {
            NonosError::InvalidAddress("Invalid stealth address prefix".into())
        })?;

        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 3 {
            return Err(NonosError::InvalidAddress(
                "Invalid stealth address format".into(),
            ));
        }

        let ephemeral_pubkey = Secp256k1PublicKey::from_hex(parts[0])?;
        let view_tag_bytes =
            hex::decode(parts[1]).map_err(|e| NonosError::InvalidAddress(e.to_string()))?;
        if view_tag_bytes.len() != 4 {
            return Err(NonosError::InvalidAddress("Invalid view tag length".into()));
        }
        let mut view_tag = [0u8; 4];
        view_tag.copy_from_slice(&view_tag_bytes);
        let meta_hash = Blake3Hash::from_hex(parts[2])?;

        Ok(Self {
            ephemeral_pubkey,
            view_tag,
            meta_hash,
        })
    }
}

impl fmt::Debug for StealthAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StealthAddress({}...)", &self.encode()[..20])
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WalletId(pub uuid::Uuid);

impl WalletId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    pub fn from_str(s: &str) -> NonosResult<Self> {
        let uuid =
            uuid::Uuid::parse_str(s).map_err(|e| NonosError::Wallet(e.to_string()))?;
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

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CircuitId(pub [u8; 16]);

impl CircuitId {
    pub fn new() -> Self {
        let mut bytes = [0u8; 16];
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        bytes[..8].copy_from_slice(&nanos.to_le_bytes()[..8]);
        let ptr = &bytes as *const _ as usize;
        bytes[8..16].copy_from_slice(&ptr.to_le_bytes());
        Self(bytes)
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl Default for CircuitId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for CircuitId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CircuitId({})", self.to_hex())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelayInfo {
    pub fingerprint: [u8; 20],
    pub nickname: String,
    pub country: Option<String>,
    pub bandwidth: u64,
    pub is_exit: bool,
    pub is_guard: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitStatus {
    Building,
    Ready,
    Active,
    Closing,
    Closed,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CircuitInfo {
    pub id: CircuitId,
    pub path: Vec<RelayInfo>,
    pub status: CircuitStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionStatus {
    #[default]
    Disconnected,
    Connecting,
    Bootstrapping,
    Connected,
    Error,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkStatus {
    pub status: ConnectionStatus,
    pub bootstrap_progress: u8,
    pub active_circuits: u32,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub socks_port: Option<u16>,
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecurityLevel {
    Standard,
    Safer,
    Safest,
}

impl Default for SecurityLevel {
    fn default() -> Self {
        Self::Safer
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TabId(pub u64);

impl TabId {
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for TabId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for TabId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tab#{}", self.0)
    }
}

impl fmt::Display for TabId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TabInfo {
    pub id: TabId,
    pub url: String,
    pub title: String,
    pub favicon: Option<String>,
    pub loading: bool,
    pub secure: bool,
    pub circuit_id: Option<CircuitId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blake3_hash() {
        let hash = Blake3Hash::from_bytes([0xab; 32]);
        assert_eq!(hash.to_hex().len(), 64);

        let parsed = Blake3Hash::from_hex(&hash.to_hex()).unwrap();
        assert_eq!(hash, parsed);
    }

    #[test]
    fn test_eth_address() {
        let addr = EthAddress::from_bytes([0xde, 0xad, 0xbe, 0xef, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);
        let hex = addr.to_hex();
        assert!(hex.starts_with("0x"));
        assert_eq!(hex.len(), 42);

        let parsed = EthAddress::from_hex(&hex).unwrap();
        assert_eq!(addr, parsed);
    }

    #[test]
    fn test_token_amount() {
        let amount = TokenAmount::nox("100.5").unwrap();
        assert_eq!(amount.to_decimal(), "100.5");

        let zero = TokenAmount::zero(18);
        assert!(zero.is_zero());

        let sum = amount.checked_add(&TokenAmount::nox("0.5").unwrap()).unwrap();
        assert_eq!(sum.to_decimal(), "101");
    }

    #[test]
    fn test_node_tier() {
        assert_eq!(NodeTier::from_stake(500), NodeTier::Bronze);
        assert_eq!(NodeTier::from_stake(10_000), NodeTier::Silver);
        assert_eq!(NodeTier::from_stake(50_000), NodeTier::Gold);
        assert_eq!(NodeTier::from_stake(200_000), NodeTier::Platinum);
        assert_eq!(NodeTier::from_stake(1_000_000), NodeTier::Diamond);
    }

    #[test]
    fn test_quality_score() {
        let perfect = QualityScore::perfect();
        assert!((perfect.total() - 1.0).abs() < 0.001);

        let score = QualityScore {
            uptime: 0.95,
            success_rate: 0.98,
            latency_score: 0.90,
            reliability: 0.92,
        };
        let total = score.total();
        assert!(total > 0.9 && total < 1.0);
    }

    #[test]
    fn test_stealth_address() {
        let pubkey = Secp256k1PublicKey::from_bytes([0x02; 33]);
        let view_tag = [0x12, 0x34, 0x56, 0x78];
        let meta_hash = Blake3Hash::from_bytes([0xab; 32]);

        let stealth = StealthAddress::new(pubkey, view_tag, meta_hash);
        let encoded = stealth.encode();
        assert!(encoded.starts_with("st:"));

        let decoded = StealthAddress::decode(&encoded).unwrap();
        assert_eq!(decoded.view_tag, view_tag);
    }
}
