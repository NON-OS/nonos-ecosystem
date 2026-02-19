use nonos_crypto::{decrypt_wallet, encrypt_wallet, EncryptedWallet};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use zeroize::{Zeroize, ZeroizeOnDrop};

const WALLET_FILENAME: &str = "wallet.enc";
const WALLET_PERMS: u32 = 0o600;

#[derive(Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct WalletData {
    pub mnemonic: String,
    pub address: String,
    pub private_key: String,
}

fn wallet_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("nonos")
        .join("wallet")
}

fn wallet_path() -> PathBuf {
    wallet_dir().join(WALLET_FILENAME)
}

pub fn wallet_exists() -> bool {
    wallet_path().exists()
}

pub fn save_wallet(password: &str, data: &WalletData) -> Result<(), String> {
    let dir = wallet_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create wallet dir: {}", e))?;

    let plaintext =
        serde_json::to_vec(data).map_err(|e| format!("Failed to serialize wallet: {}", e))?;

    let encrypted = encrypt_wallet(password.as_bytes(), &plaintext)
        .map_err(|e| format!("Failed to encrypt wallet: {}", e))?;

    let json =
        serde_json::to_string(&encrypted).map_err(|e| format!("Failed to encode wallet: {}", e))?;

    let path = wallet_path();

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut opts = fs::OpenOptions::new();
        opts.write(true).create(true).truncate(true);
        opts.mode(WALLET_PERMS);
        let mut file = opts
            .open(&path)
            .map_err(|e| format!("Failed to create wallet file: {}", e))?;
        use std::io::Write;
        file.write_all(json.as_bytes())
            .map_err(|e| format!("Failed to write wallet: {}", e))?;
    }

    #[cfg(not(unix))]
    {
        fs::write(&path, &json).map_err(|e| format!("Failed to write wallet: {}", e))?;
    }

    Ok(())
}

pub fn load_wallet(password: &str) -> Result<WalletData, String> {
    let path = wallet_path();

    if !path.exists() {
        return Err("No wallet found".into());
    }

    let json = fs::read_to_string(&path).map_err(|e| format!("Failed to read wallet: {}", e))?;

    let encrypted: EncryptedWallet =
        serde_json::from_str(&json).map_err(|e| format!("Invalid wallet format: {}", e))?;

    let plaintext = decrypt_wallet(password.as_bytes(), &encrypted)
        .map_err(|e| format!("Failed to decrypt wallet: {}", e))?;

    let data: WalletData = serde_json::from_slice(&plaintext)
        .map_err(|e| format!("Failed to deserialize wallet: {}", e))?;

    Ok(data)
}

pub fn delete_wallet() -> Result<(), String> {
    let path = wallet_path();
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("Failed to delete wallet: {}", e))?;
    }
    Ok(())
}

pub fn change_password(old_password: &str, new_password: &str) -> Result<(), String> {
    let data = load_wallet(old_password)?;
    save_wallet(new_password, &data)?;
    Ok(())
}
