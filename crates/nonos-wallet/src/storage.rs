use nonos_crypto::EncryptedData;
use nonos_types::{Blake3Key, EthAddress, NonosError, NonosResult, WalletId, WalletMetadata};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize)]
pub struct WalletFile {
    pub version: u32,
    pub id: String,
    pub name: String,
    pub address: String,
    pub encrypted: EncryptedData,
    pub created_at: String,
}

#[derive(Serialize, Deserialize)]
struct EncryptedWalletData {
    accounts: Vec<(u32, String)>,
    stealth_count: u32,
    tx_hashes: Vec<String>,
}

pub trait WalletStorage: Send + Sync {
    fn list_wallets(&self) -> NonosResult<Vec<WalletId>>;

    fn load_metadata(&self, id: &WalletId) -> NonosResult<WalletMetadata>;

    fn save_wallet(&self, metadata: &WalletMetadata, encryption_key: &Blake3Key) -> NonosResult<()>;

    fn delete_wallet(&self, id: &WalletId) -> NonosResult<()>;

    fn wallet_exists(&self, id: &WalletId) -> bool;
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
        std::fs::write(&temp_path, &contents)
            .map_err(|e| NonosError::Storage(format!("Failed to write wallet: {}", e)))?;

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

    fn save_wallet(&self, metadata: &WalletMetadata, encryption_key: &Blake3Key) -> NonosResult<()> {
        let data = EncryptedWalletData {
            accounts: vec![(0, metadata.address.to_hex())],
            stealth_count: metadata.stealth_count,
            tx_hashes: Vec::new(),
        };

        let data_json = serde_json::to_vec(&data)
            .map_err(|e| NonosError::Serialization(e.to_string()))?;

        let encrypted = EncryptedData::new(encryption_key, &data_json)?;

        let file = WalletFile {
            version: 1,
            id: metadata.id.to_string(),
            name: metadata.name.clone(),
            address: metadata.address.to_hex(),
            encrypted,
            created_at: metadata.created_at.to_rfc3339(),
        };

        self.save_wallet_file(&file)
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
}

pub struct MemoryWalletStorage {
    wallets: std::sync::RwLock<std::collections::HashMap<String, WalletFile>>,
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

        let file = wallets.get(&id.to_string())
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

    fn save_wallet(&self, metadata: &WalletMetadata, encryption_key: &Blake3Key) -> NonosResult<()> {
        let data = EncryptedWalletData {
            accounts: vec![(0, metadata.address.to_hex())],
            stealth_count: metadata.stealth_count,
            tx_hashes: Vec::new(),
        };

        let data_json = serde_json::to_vec(&data)
            .map_err(|e| NonosError::Serialization(e.to_string()))?;

        let encrypted = EncryptedData::new(encryption_key, &data_json)?;

        let file = WalletFile {
            version: 1,
            id: metadata.id.to_string(),
            name: metadata.name.clone(),
            address: metadata.address.to_hex(),
            encrypted,
            created_at: metadata.created_at.to_rfc3339(),
        };

        let mut wallets = self.wallets.write()
            .map_err(|_| NonosError::Storage("Lock poisoned".into()))?;

        wallets.insert(metadata.id.to_string(), file);
        Ok(())
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

        let key = Blake3Key::from_bytes([0xcd; 32]);

        storage.save_wallet(&metadata, &key).unwrap();

        let wallets = storage.list_wallets().unwrap();
        assert_eq!(wallets.len(), 1);

        let loaded = storage.load_metadata(&metadata.id).unwrap();
        assert_eq!(loaded.name, metadata.name);
        assert_eq!(loaded.address, metadata.address);

        storage.delete_wallet(&metadata.id).unwrap();
        assert!(!storage.wallet_exists(&metadata.id));
    }
}
