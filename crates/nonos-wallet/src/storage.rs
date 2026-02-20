use nonos_crypto::{decrypt_wallet, encrypt_wallet, EncryptedWallet};
use nonos_types::{EthAddress, NonosError, NonosResult, WalletId, WalletMetadata};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const WALLET_FILE_PERMS: u32 = 0o600;

#[derive(Serialize, Deserialize)]
pub struct WalletFile {
    pub version: u32,
    pub id: String,
    pub name: String,
    pub address: String,
    pub encrypted: EncryptedWallet,
    pub created_at: String,
}

#[derive(Serialize, Deserialize)]
struct WalletSecrets {
    master_key: String,
    accounts: Vec<(u32, String)>,
    stealth_count: u32,
}

pub trait WalletStorage: Send + Sync {
    fn list_wallets(&self) -> NonosResult<Vec<WalletId>>;
    fn load_metadata(&self, id: &WalletId) -> NonosResult<WalletMetadata>;
    fn save_wallet(&self, metadata: &WalletMetadata, master_key: &[u8; 32], password: &str) -> NonosResult<()>;
    fn load_secrets(&self, id: &WalletId, password: &str) -> NonosResult<[u8; 32]>;
    fn delete_wallet(&self, id: &WalletId) -> NonosResult<()>;
    fn wallet_exists(&self, id: &WalletId) -> bool;
    fn change_password(&self, id: &WalletId, old_password: &str, new_password: &str) -> NonosResult<()>;
}

pub struct FileWalletStorage {
    base_dir: PathBuf,
}

impl FileWalletStorage {
    pub fn new(base_dir: impl AsRef<Path>) -> NonosResult<Self> {
        let base_dir = base_dir.as_ref().to_path_buf();

        if !base_dir.exists() {
            std::fs::create_dir_all(&base_dir)
                .map_err(|e| NonosError::Storage(e.to_string()))?;
        }

        Ok(Self { base_dir })
    }

    fn wallet_path(&self, id: &WalletId) -> PathBuf {
        self.base_dir.join(format!("{}.wallet", id))
    }

    fn load_wallet_file(&self, id: &WalletId) -> NonosResult<WalletFile> {
        let path = self.wallet_path(id);
        let contents = std::fs::read_to_string(&path)
            .map_err(|e| NonosError::Storage(format!("Failed to read wallet: {}", e)))?;

        serde_json::from_str(&contents)
            .map_err(|e| NonosError::Storage(format!("Failed to parse wallet: {}", e)))
    }

    fn save_wallet_file(&self, wallet: &WalletFile) -> NonosResult<()> {
        let id = WalletId::from_str(&wallet.id)?;
        let path = self.wallet_path(&id);

        let contents = serde_json::to_string_pretty(wallet)
            .map_err(|e| NonosError::Serialization(e.to_string()))?;

        let temp_path = path.with_extension("wallet.tmp");

        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            let mut opts = std::fs::OpenOptions::new();
            opts.write(true).create(true).truncate(true);
            opts.mode(WALLET_FILE_PERMS);
            let mut file = opts.open(&temp_path)
                .map_err(|e| NonosError::Storage(format!("Failed to create wallet file: {}", e)))?;
            use std::io::Write;
            file.write_all(contents.as_bytes())
                .map_err(|e| NonosError::Storage(format!("Failed to write wallet: {}", e)))?;
        }

        #[cfg(not(unix))]
        {
            std::fs::write(&temp_path, &contents)
                .map_err(|e| NonosError::Storage(format!("Failed to write wallet: {}", e)))?;
        }

        std::fs::rename(&temp_path, &path)
            .map_err(|e| NonosError::Storage(format!("Failed to save wallet: {}", e)))?;

        Ok(())
    }
}

impl WalletStorage for FileWalletStorage {
    fn list_wallets(&self) -> NonosResult<Vec<WalletId>> {
        let mut wallets = Vec::new();

        let entries = std::fs::read_dir(&self.base_dir)
            .map_err(|e| NonosError::Storage(e.to_string()))?;

        for entry in entries {
            let entry = entry.map_err(|e| NonosError::Storage(e.to_string()))?;
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "wallet") {
                if let Some(stem) = path.file_stem() {
                    if let Ok(id) = WalletId::from_str(&stem.to_string_lossy()) {
                        wallets.push(id);
                    }
                }
            }
        }

        Ok(wallets)
    }

    fn load_metadata(&self, id: &WalletId) -> NonosResult<WalletMetadata> {
        let file = self.load_wallet_file(id)?;

        let address = EthAddress::from_hex(&file.address)?;
        let created_at = chrono::DateTime::parse_from_rfc3339(&file.created_at)
            .map_err(|e| NonosError::Storage(format!("Invalid timestamp: {}", e)))?
            .with_timezone(&chrono::Utc);

        Ok(WalletMetadata {
            id: id.clone(),
            name: file.name,
            created_at,
            last_accessed: chrono::Utc::now(),
            address,
            stealth_count: 0,
        })
    }

    fn save_wallet(&self, metadata: &WalletMetadata, master_key: &[u8; 32], password: &str) -> NonosResult<()> {
        let secrets = WalletSecrets {
            master_key: hex::encode(master_key),
            accounts: vec![(0, metadata.address.to_hex())],
            stealth_count: metadata.stealth_count,
        };

        let plaintext = serde_json::to_vec(&secrets)
            .map_err(|e| NonosError::Serialization(e.to_string()))?;

        let encrypted = encrypt_wallet(password.as_bytes(), &plaintext)?;

        let file = WalletFile {
            version: 2,
            id: metadata.id.to_string(),
            name: metadata.name.clone(),
            address: metadata.address.to_hex(),
            encrypted,
            created_at: metadata.created_at.to_rfc3339(),
        };

        self.save_wallet_file(&file)
    }

    fn load_secrets(&self, id: &WalletId, password: &str) -> NonosResult<[u8; 32]> {
        let file = self.load_wallet_file(id)?;

        let plaintext = decrypt_wallet(password.as_bytes(), &file.encrypted)?;

        let secrets: WalletSecrets = serde_json::from_slice(&plaintext)
            .map_err(|e| NonosError::Storage(format!("Failed to parse secrets: {}", e)))?;

        let key_bytes = hex::decode(&secrets.master_key)
            .map_err(|e| NonosError::Storage(format!("Invalid key encoding: {}", e)))?;

        if key_bytes.len() != 32 {
            return Err(NonosError::Storage("Invalid master key length".into()));
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(&key_bytes);
        Ok(key)
    }

    fn delete_wallet(&self, id: &WalletId) -> NonosResult<()> {
        let path = self.wallet_path(id);

        if path.exists() {
            std::fs::remove_file(&path)
                .map_err(|e| NonosError::Storage(format!("Failed to delete wallet: {}", e)))?;
        }

        Ok(())
    }

    fn wallet_exists(&self, id: &WalletId) -> bool {
        self.wallet_path(id).exists()
    }

    fn change_password(&self, id: &WalletId, old_password: &str, new_password: &str) -> NonosResult<()> {
        let master_key = self.load_secrets(id, old_password)?;
        let metadata = self.load_metadata(id)?;
        self.save_wallet(&metadata, &master_key, new_password)
    }
}

pub struct MemoryWalletStorage {
    wallets: std::sync::RwLock<std::collections::HashMap<String, (WalletFile, [u8; 32])>>,
}

impl MemoryWalletStorage {
    pub fn new() -> Self {
        Self {
            wallets: std::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }
}

impl Default for MemoryWalletStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl WalletStorage for MemoryWalletStorage {
    fn list_wallets(&self) -> NonosResult<Vec<WalletId>> {
        let wallets = self.wallets.read()
            .map_err(|_| NonosError::Storage("Lock poisoned".into()))?;

        wallets.keys()
            .map(|id| WalletId::from_str(id))
            .collect()
    }

    fn load_metadata(&self, id: &WalletId) -> NonosResult<WalletMetadata> {
        let wallets = self.wallets.read()
            .map_err(|_| NonosError::Storage("Lock poisoned".into()))?;

        let (file, _) = wallets.get(&id.to_string())
            .ok_or_else(|| NonosError::Storage("Wallet not found".into()))?;

        let address = EthAddress::from_hex(&file.address)?;
        let created_at = chrono::DateTime::parse_from_rfc3339(&file.created_at)
            .map_err(|e| NonosError::Storage(format!("Invalid timestamp: {}", e)))?
            .with_timezone(&chrono::Utc);

        Ok(WalletMetadata {
            id: id.clone(),
            name: file.name.clone(),
            created_at,
            last_accessed: chrono::Utc::now(),
            address,
            stealth_count: 0,
        })
    }

    fn save_wallet(&self, metadata: &WalletMetadata, master_key: &[u8; 32], password: &str) -> NonosResult<()> {
        let secrets = WalletSecrets {
            master_key: hex::encode(master_key),
            accounts: vec![(0, metadata.address.to_hex())],
            stealth_count: metadata.stealth_count,
        };

        let plaintext = serde_json::to_vec(&secrets)
            .map_err(|e| NonosError::Serialization(e.to_string()))?;

        let encrypted = encrypt_wallet(password.as_bytes(), &plaintext)?;

        let file = WalletFile {
            version: 2,
            id: metadata.id.to_string(),
            name: metadata.name.clone(),
            address: metadata.address.to_hex(),
            encrypted,
            created_at: metadata.created_at.to_rfc3339(),
        };

        let mut wallets = self.wallets.write()
            .map_err(|_| NonosError::Storage("Lock poisoned".into()))?;

        wallets.insert(metadata.id.to_string(), (file, *master_key));
        Ok(())
    }

    fn load_secrets(&self, id: &WalletId, _password: &str) -> NonosResult<[u8; 32]> {
        let wallets = self.wallets.read()
            .map_err(|_| NonosError::Storage("Lock poisoned".into()))?;

        let (_, key) = wallets.get(&id.to_string())
            .ok_or_else(|| NonosError::Storage("Wallet not found".into()))?;

        Ok(*key)
    }

    fn delete_wallet(&self, id: &WalletId) -> NonosResult<()> {
        let mut wallets = self.wallets.write()
            .map_err(|_| NonosError::Storage("Lock poisoned".into()))?;

        wallets.remove(&id.to_string());
        Ok(())
    }

    fn wallet_exists(&self, id: &WalletId) -> bool {
        self.wallets.read()
            .map(|w| w.contains_key(&id.to_string()))
            .unwrap_or(false)
    }

    fn change_password(&self, id: &WalletId, old_password: &str, new_password: &str) -> NonosResult<()> {
        let master_key = self.load_secrets(id, old_password)?;
        let metadata = self.load_metadata(id)?;
        self.save_wallet(&metadata, &master_key, new_password)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_storage() {
        let storage = MemoryWalletStorage::new();

        let metadata = WalletMetadata::new(
            "Test Wallet".to_string(),
            EthAddress::from_bytes([0xab; 20]),
        );

        let master_key = [0xcd; 32];
        let password = "testpassword123";

        storage.save_wallet(&metadata, &master_key, password).unwrap();

        let wallets = storage.list_wallets().unwrap();
        assert_eq!(wallets.len(), 1);

        let loaded = storage.load_metadata(&metadata.id).unwrap();
        assert_eq!(loaded.name, metadata.name);
        assert_eq!(loaded.address, metadata.address);

        let loaded_key = storage.load_secrets(&metadata.id, password).unwrap();
        assert_eq!(loaded_key, master_key);

        storage.delete_wallet(&metadata.id).unwrap();
        assert!(!storage.wallet_exists(&metadata.id));
    }

    #[test]
    fn test_password_change() {
        let storage = MemoryWalletStorage::new();

        let metadata = WalletMetadata::new(
            "Test Wallet".to_string(),
            EthAddress::from_bytes([0xab; 20]),
        );

        let master_key = [0xcd; 32];
        let old_password = "oldpass123";
        let new_password = "newpass456";

        storage.save_wallet(&metadata, &master_key, old_password).unwrap();
        storage.change_password(&metadata.id, old_password, new_password).unwrap();

        let loaded_key = storage.load_secrets(&metadata.id, new_password).unwrap();
        assert_eq!(loaded_key, master_key);
    }
}
