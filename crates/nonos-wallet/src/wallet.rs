use nonos_crypto::{
    derive_blake3_key_from_mnemonic, derive_eth_address_from_private,
    derive_secp256k1_key, sign_message,
    sign_personal_message, SecureMnemonic, StealthKeyPair, StealthMetaAddress,
    derive_stealth_private_key, check_stealth_address,
};
use nonos_types::{
    Blake3Hash, Blake3Key, EcdsaSignature, EthAddress, NonosError, NonosResult,
    Secp256k1PrivateKey, Secp256k1PublicKey, TransactionRecord, TransactionStatus,
    WalletId, WalletMetadata,
};
use std::collections::HashMap;
use tracing::{debug, info};
use zeroize::Zeroize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalletState {
    Locked,
    Unlocked,
}

pub struct Wallet {
    metadata: WalletMetadata,
    state: WalletState,
    master_key: Option<Blake3Key>,
    accounts: HashMap<u32, EthAddress>,
    stealth_keypair: Option<StealthKeyPair>,
    transactions: Vec<TransactionRecord>,
}

impl Wallet {
    pub fn create(name: String) -> NonosResult<(Self, String, String)> {
        info!("Creating new wallet: {}", name);

        let mnemonic = SecureMnemonic::new()?;
        let phrase = mnemonic.phrase().to_string();

        let master_key = derive_blake3_key_from_mnemonic(&phrase)?;
        let blake3_hex = master_key.to_hex();

        let account_key = derive_secp256k1_key(&master_key, 0, 0);
        let private_key = Secp256k1PrivateKey::from_bytes(account_key);
        let address = derive_eth_address_from_private(&private_key)?;

        let stealth_keypair = StealthKeyPair::derive_from_master(&master_key)?;

        let metadata = WalletMetadata::new(name, address);

        let mut accounts = HashMap::new();
        accounts.insert(0, address);

        let wallet = Self {
            metadata,
            state: WalletState::Unlocked,
            master_key: Some(master_key),
            accounts,
            stealth_keypair: Some(stealth_keypair),
            transactions: Vec::new(),
        };

        Ok((wallet, phrase, blake3_hex))
    }

    pub fn import_from_mnemonic(name: String, phrase: &str) -> NonosResult<Self> {
        info!("Importing wallet from mnemonic: {}", name);

        let _mnemonic = SecureMnemonic::from_phrase(phrase.to_string())?;
        let master_key = derive_blake3_key_from_mnemonic(phrase)?;

        let account_key = derive_secp256k1_key(&master_key, 0, 0);
        let private_key = Secp256k1PrivateKey::from_bytes(account_key);
        let address = derive_eth_address_from_private(&private_key)?;

        let stealth_keypair = StealthKeyPair::derive_from_master(&master_key)?;

        let metadata = WalletMetadata::new(name, address);

        let mut accounts = HashMap::new();
        accounts.insert(0, address);

        Ok(Self {
            metadata,
            state: WalletState::Unlocked,
            master_key: Some(master_key),
            accounts,
            stealth_keypair: Some(stealth_keypair),
            transactions: Vec::new(),
        })
    }

    pub fn import_from_blake3_key(name: String, key_hex: &str) -> NonosResult<Self> {
        info!("Importing wallet from BLAKE3 key: {}", name);

        let master_key = Blake3Key::from_hex(key_hex)?;

        let account_key = derive_secp256k1_key(&master_key, 0, 0);
        let private_key = Secp256k1PrivateKey::from_bytes(account_key);
        let address = derive_eth_address_from_private(&private_key)?;

        let stealth_keypair = StealthKeyPair::derive_from_master(&master_key)?;

        let metadata = WalletMetadata::new(name, address);

        let mut accounts = HashMap::new();
        accounts.insert(0, address);

        Ok(Self {
            metadata,
            state: WalletState::Unlocked,
            master_key: Some(master_key),
            accounts,
            stealth_keypair: Some(stealth_keypair),
            transactions: Vec::new(),
        })
    }

    pub fn id(&self) -> &WalletId {
        &self.metadata.id
    }

    pub fn name(&self) -> &str {
        &self.metadata.name
    }

    pub fn state(&self) -> WalletState {
        self.state
    }

    pub fn is_unlocked(&self) -> bool {
        self.state == WalletState::Unlocked
    }

    pub fn address(&self) -> &EthAddress {
        &self.metadata.address
    }

    pub fn metadata(&self) -> &WalletMetadata {
        &self.metadata
    }

    pub fn accounts(&self) -> &HashMap<u32, EthAddress> {
        &self.accounts
    }

    pub fn lock(&mut self) {
        info!("Locking wallet: {}", self.metadata.name);

        if let Some(ref mut key) = self.master_key {
            key.0.zeroize();
        }
        self.master_key = None;
        self.stealth_keypair = None;
        self.state = WalletState::Locked;
    }

    pub fn unlock_with_mnemonic(&mut self, phrase: &str) -> NonosResult<()> {
        info!("Unlocking wallet with mnemonic: {}", self.metadata.name);

        let _mnemonic = SecureMnemonic::from_phrase(phrase.to_string())?;
        let master_key = derive_blake3_key_from_mnemonic(phrase)?;

        let account_key = derive_secp256k1_key(&master_key, 0, 0);
        let private_key = Secp256k1PrivateKey::from_bytes(account_key);
        let address = derive_eth_address_from_private(&private_key)?;

        if address != self.metadata.address {
            return Err(NonosError::Wallet("Mnemonic does not match wallet".into()));
        }

        let stealth_keypair = StealthKeyPair::derive_from_master(&master_key)?;

        self.master_key = Some(master_key);
        self.stealth_keypair = Some(stealth_keypair);
        self.state = WalletState::Unlocked;

        Ok(())
    }

    pub fn unlock_with_blake3_key(&mut self, key_hex: &str) -> NonosResult<()> {
        info!("Unlocking wallet with BLAKE3 key: {}", self.metadata.name);

        let master_key = Blake3Key::from_hex(key_hex)?;

        let account_key = derive_secp256k1_key(&master_key, 0, 0);
        let private_key = Secp256k1PrivateKey::from_bytes(account_key);
        let address = derive_eth_address_from_private(&private_key)?;

        if address != self.metadata.address {
            return Err(NonosError::Wallet("BLAKE3 key does not match wallet".into()));
        }

        let stealth_keypair = StealthKeyPair::derive_from_master(&master_key)?;

        self.master_key = Some(master_key);
        self.stealth_keypair = Some(stealth_keypair);
        self.state = WalletState::Unlocked;

        Ok(())
    }

    pub fn derive_account(&mut self, index: u32) -> NonosResult<EthAddress> {
        let master_key = self.master_key.as_ref()
            .ok_or_else(|| NonosError::Wallet("Wallet is locked".into()))?;

        let account_key = derive_secp256k1_key(master_key, 0, index);
        let private_key = Secp256k1PrivateKey::from_bytes(account_key);
        let address = derive_eth_address_from_private(&private_key)?;

        self.accounts.insert(index, address);
        debug!("Derived account {}: {}", index, address);

        Ok(address)
    }

    pub fn sign_hash(&self, account_index: u32, hash: &[u8; 32]) -> NonosResult<EcdsaSignature> {
        let master_key = self.master_key.as_ref()
            .ok_or_else(|| NonosError::Wallet("Wallet is locked".into()))?;

        let account_key = derive_secp256k1_key(master_key, 0, account_index);
        let private_key = Secp256k1PrivateKey::from_bytes(account_key);

        sign_message(&private_key, hash)
    }

    pub fn get_account_private_key(&self, account_index: u32) -> NonosResult<String> {
        let master_key = self.master_key.as_ref()
            .ok_or_else(|| NonosError::Wallet("Wallet is locked".into()))?;

        let account_key = derive_secp256k1_key(master_key, 0, account_index);
        Ok(hex::encode(account_key))
    }

    pub fn sign_personal(&self, account_index: u32, message: &[u8]) -> NonosResult<EcdsaSignature> {
        let master_key = self.master_key.as_ref()
            .ok_or_else(|| NonosError::Wallet("Wallet is locked".into()))?;

        let account_key = derive_secp256k1_key(master_key, 0, account_index);
        let private_key = Secp256k1PrivateKey::from_bytes(account_key);

        sign_personal_message(&private_key, message)
    }

    pub fn stealth_meta_address(&self) -> NonosResult<StealthMetaAddress> {
        let stealth = self.stealth_keypair.as_ref()
            .ok_or_else(|| NonosError::Wallet("Wallet is locked".into()))?;

        Ok(stealth.meta_address())
    }

    pub fn generate_receive_stealth_address(&self) -> NonosResult<String> {
        let meta = self.stealth_meta_address()?;
        Ok(meta.encode())
    }

    pub fn scan_stealth_payment(
        &self,
        ephemeral_pubkey: &Secp256k1PublicKey,
        view_tag: &[u8; 4],
    ) -> NonosResult<Option<EthAddress>> {
        let stealth = self.stealth_keypair.as_ref()
            .ok_or_else(|| NonosError::Wallet("Wallet is locked".into()))?;

        if !check_stealth_address(stealth, ephemeral_pubkey, view_tag)? {
            return Ok(None);
        }

        let stealth_private = derive_stealth_private_key(stealth, ephemeral_pubkey)?;
        let stealth_address = derive_eth_address_from_private(&stealth_private)?;

        Ok(Some(stealth_address))
    }

    pub fn sign_stealth(
        &self,
        ephemeral_pubkey: &Secp256k1PublicKey,
        hash: &[u8; 32],
    ) -> NonosResult<EcdsaSignature> {
        let stealth = self.stealth_keypair.as_ref()
            .ok_or_else(|| NonosError::Wallet("Wallet is locked".into()))?;

        let stealth_private = derive_stealth_private_key(stealth, ephemeral_pubkey)?;
        sign_message(&stealth_private, hash)
    }

    pub fn add_transaction(&mut self, tx: TransactionRecord) {
        self.transactions.push(tx);
    }

    pub fn transactions(&self) -> &[TransactionRecord] {
        &self.transactions
    }

    pub fn update_transaction_status(&mut self, hash: &Blake3Hash, status: TransactionStatus) {
        if let Some(tx) = self.transactions.iter_mut().find(|t| &t.hash == hash) {
            tx.status = status;
        }
    }
}

impl Drop for Wallet {
    fn drop(&mut self) {
        self.lock();
    }
}

pub struct WalletBuilder {
    name: Option<String>,
    mnemonic: Option<String>,
    blake3_key: Option<String>,
}

impl WalletBuilder {
    pub fn new() -> Self {
        Self {
            name: None,
            mnemonic: None,
            blake3_key: None,
        }
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn mnemonic(mut self, phrase: impl Into<String>) -> Self {
        self.mnemonic = Some(phrase.into());
        self
    }

    pub fn blake3_key(mut self, key: impl Into<String>) -> Self {
        self.blake3_key = Some(key.into());
        self
    }

    pub fn build(self) -> NonosResult<(Wallet, Option<String>, Option<String>)> {
        let name = self.name.unwrap_or_else(|| "Default Wallet".to_string());

        if let Some(mnemonic) = self.mnemonic {
            let wallet = Wallet::import_from_mnemonic(name, &mnemonic)?;
            Ok((wallet, None, None))
        } else if let Some(key) = self.blake3_key {
            let wallet = Wallet::import_from_blake3_key(name, &key)?;
            Ok((wallet, None, None))
        } else {
            let (wallet, mnemonic, key) = Wallet::create(name)?;
            Ok((wallet, Some(mnemonic), Some(key)))
        }
    }
}

impl Default for WalletBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_creation() {
        let (wallet, mnemonic, key) = Wallet::create("Test Wallet".to_string()).unwrap();

        assert!(wallet.is_unlocked());
        assert!(!mnemonic.is_empty());
        assert!(!key.is_empty());

        let words: Vec<&str> = mnemonic.split_whitespace().collect();
        assert_eq!(words.len(), 24);
    }

    #[test]
    fn test_wallet_lock_unlock() {
        let (mut wallet, mnemonic, key) = Wallet::create("Test Wallet".to_string()).unwrap();
        let address = *wallet.address();

        assert!(wallet.is_unlocked());

        wallet.lock();
        assert!(!wallet.is_unlocked());
        assert!(wallet.sign_hash(0, &[0; 32]).is_err());

        wallet.unlock_with_mnemonic(&mnemonic).unwrap();
        assert!(wallet.is_unlocked());
        assert_eq!(*wallet.address(), address);

        wallet.lock();

        wallet.unlock_with_blake3_key(&key).unwrap();
        assert!(wallet.is_unlocked());
        assert_eq!(*wallet.address(), address);
    }

    #[test]
    fn test_wallet_signing() {
        let (wallet, _, _) = Wallet::create("Test Wallet".to_string()).unwrap();

        let message = b"Test message";
        let signature = wallet.sign_personal(0, message).unwrap();

        assert!(signature.v == 27 || signature.v == 28);
    }

    #[test]
    fn test_stealth_address() {
        let (wallet, _, _) = Wallet::create("Test Wallet".to_string()).unwrap();

        let meta = wallet.stealth_meta_address().unwrap();
        let encoded = meta.encode();

        assert!(encoded.starts_with("st:eth:0x"));
    }

    #[test]
    fn test_account_derivation() {
        let (mut wallet, _, _) = Wallet::create("Test Wallet".to_string()).unwrap();

        let addr0 = *wallet.address();
        let addr1 = wallet.derive_account(1).unwrap();
        let addr2 = wallet.derive_account(2).unwrap();

        assert_ne!(addr0, addr1);
        assert_ne!(addr1, addr2);
        assert_ne!(addr0, addr2);
    }

    #[test]
    fn test_wallet_builder() {
        let (wallet, mnemonic, key) = WalletBuilder::new()
            .name("Builder Wallet")
            .build()
            .unwrap();

        assert!(wallet.is_unlocked());
        assert!(mnemonic.is_some());
        assert!(key.is_some());
    }

    #[test]
    fn test_wrong_key_fails() {
        let (mut wallet, _, _) = Wallet::create("Test Wallet".to_string()).unwrap();
        wallet.lock();

        let wrong_key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let result = wallet.unlock_with_blake3_key(wrong_key);

        assert!(result.is_err());
    }
}
