use nonos_wallet::{FileWalletStorage, Wallet, WalletStorage};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct WalletManager {
    storage: Option<FileWalletStorage>,
    active: Option<Wallet>,
}

impl WalletManager {
    pub fn new() -> Self {
        Self {
            storage: None,
            active: None,
        }
    }

    fn storage_dir() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nonos")
            .join("wallets")
    }

    pub fn storage(&mut self) -> Result<&mut FileWalletStorage, String> {
        if self.storage.is_none() {
            let storage = FileWalletStorage::new(Self::storage_dir())
                .map_err(|e| format!("Storage init failed: {}", e))?;
            self.storage = Some(storage);
        }
        self.storage.as_mut().ok_or_else(|| "Storage not available".into())
    }

    pub fn has_wallets(&mut self) -> bool {
        self.storage()
            .ok()
            .and_then(|s| s.list_wallets().ok())
            .map(|w| !w.is_empty())
            .unwrap_or(false)
    }

    pub fn active(&self) -> Option<&Wallet> {
        self.active.as_ref()
    }

    pub fn set_active(&mut self, wallet: Wallet) {
        self.active = Some(wallet);
    }

    pub fn clear_active(&mut self) {
        if let Some(ref mut wallet) = self.active {
            wallet.lock();
        }
        self.active = None;
    }
}

impl Default for WalletManager {
    fn default() -> Self {
        Self::new()
    }
}

lazy_static::lazy_static! {
    pub static ref WALLET_MANAGER: Arc<RwLock<WalletManager>> = Arc::new(RwLock::new(WalletManager::new()));
}
