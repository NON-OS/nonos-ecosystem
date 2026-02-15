use thiserror::Error;

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
