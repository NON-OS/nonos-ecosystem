use aes_gcm::aead::{Aead, KeyInit};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use nonos_types::{Blake3Key, NonosError, NonosResult};
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;
use zeroize::Zeroize;

pub const WALLET_ENCRYPTION_VERSION: u8 = 2;

const XCHACHA_NONCE_SIZE: usize = 24;
const ARGON2_MEMORY: u32 = 64 * 1024;
const ARGON2_TIME: u32 = 3;
const ARGON2_PARALLELISM: u32 = 4;
const SALT_SIZE: usize = 32;

#[derive(Clone, Serialize, Deserialize)]
pub struct EncryptedWallet {
    pub version: u8,
    pub algorithm: String,
    #[serde(with = "hex_serde")]
    pub salt: Vec<u8>,
    #[serde(with = "hex_serde")]
    pub nonce: Vec<u8>,
    #[serde(with = "hex_serde")]
    pub ciphertext: Vec<u8>,
    pub kdf_params: KdfParams,
    #[serde(with = "hex_serde")]
    pub checksum: Vec<u8>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct KdfParams {
    pub algorithm: String,
    pub memory_kib: u32,
    pub iterations: u32,
    pub parallelism: u32,
}

impl Default for KdfParams {
    fn default() -> Self {
        Self {
            algorithm: "argon2id".to_string(),
            memory_kib: ARGON2_MEMORY,
            iterations: ARGON2_TIME,
            parallelism: ARGON2_PARALLELISM,
        }
    }
}

mod hex_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(data: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        hex::encode(data).serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        hex::decode(&s).map_err(serde::de::Error::custom)
    }
}

pub fn derive_wallet_key(password: &[u8], salt: &[u8], params: &KdfParams) -> NonosResult<Blake3Key> {
    use argon2::{Algorithm, Argon2, Params, Version};

    if params.algorithm != "argon2id" {
        return Err(NonosError::Crypto(format!(
            "Unsupported KDF: {}",
            params.algorithm
        )));
    }

    let argon_params = Params::new(
        params.memory_kib,
        params.iterations,
        params.parallelism,
        Some(32),
    )
    .map_err(|e| NonosError::Crypto(format!("Invalid params: {}", e)))?;

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon_params);

    let mut key = [0u8; 32];
    argon2
        .hash_password_into(password, salt, &mut key)
        .map_err(|e| NonosError::Crypto(format!("KDF failed: {}", e)))?;

    let result = Blake3Key::from_bytes(key);
    key.zeroize();
    Ok(result)
}

pub fn encrypt_wallet(password: &[u8], plaintext: &[u8]) -> NonosResult<EncryptedWallet> {
    let salt = crate::random_bytes::<SALT_SIZE>();
    let nonce_bytes = crate::random_bytes::<XCHACHA_NONCE_SIZE>();

    let params = KdfParams::default();
    let key = derive_wallet_key(password, &salt, &params)?;

    let cipher = XChaCha20Poly1305::new_from_slice(&key.0)
        .map_err(|e| NonosError::Crypto(format!("Cipher init: {}", e)))?;

    let nonce = XNonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| NonosError::Crypto(format!("Encrypt: {}", e)))?;

    let mut checksum_input = Vec::with_capacity(salt.len() + nonce_bytes.len() + ciphertext.len());
    checksum_input.extend_from_slice(&salt);
    checksum_input.extend_from_slice(&nonce_bytes);
    checksum_input.extend_from_slice(&ciphertext);
    let checksum = blake3::hash(&checksum_input);

    Ok(EncryptedWallet {
        version: WALLET_ENCRYPTION_VERSION,
        algorithm: "xchacha20-poly1305".to_string(),
        salt: salt.to_vec(),
        nonce: nonce_bytes.to_vec(),
        ciphertext,
        kdf_params: params,
        checksum: checksum.as_bytes().to_vec(),
    })
}

pub fn decrypt_wallet(password: &[u8], wallet: &EncryptedWallet) -> NonosResult<Vec<u8>> {
    if wallet.version != WALLET_ENCRYPTION_VERSION && wallet.version != 1 {
        return Err(NonosError::Crypto(format!("Unsupported version: {}", wallet.version)));
    }

    if wallet.version == 1 {
        return decrypt_wallet_v1(password, wallet);
    }

    let mut checksum_input = Vec::with_capacity(wallet.salt.len() + wallet.nonce.len() + wallet.ciphertext.len());
    checksum_input.extend_from_slice(&wallet.salt);
    checksum_input.extend_from_slice(&wallet.nonce);
    checksum_input.extend_from_slice(&wallet.ciphertext);
    let expected = blake3::hash(&checksum_input);

    if wallet.checksum.len() != 32 || !bool::from(wallet.checksum.ct_eq(expected.as_bytes())) {
        return Err(NonosError::Crypto("Checksum mismatch".into()));
    }

    let key = derive_wallet_key(password, &wallet.salt, &wallet.kdf_params)?;

    if wallet.algorithm != "xchacha20-poly1305" {
        return Err(NonosError::Crypto(format!("Unsupported algo: {}", wallet.algorithm)));
    }

    let cipher = XChaCha20Poly1305::new_from_slice(&key.0)
        .map_err(|e| NonosError::Crypto(format!("Cipher init: {}", e)))?;

    let nonce = XNonce::from_slice(&wallet.nonce);
    cipher
        .decrypt(nonce, wallet.ciphertext.as_ref())
        .map_err(|e| NonosError::Crypto(format!("Decrypt: {}", e)))
}

fn decrypt_wallet_v1(password: &[u8], wallet: &EncryptedWallet) -> NonosResult<Vec<u8>> {
    if wallet.algorithm != "AES-256-GCM" && wallet.algorithm != "aes-256-gcm" {
        return Err(NonosError::Crypto(format!("Unsupported v1 algo: {}", wallet.algorithm)));
    }

    let key = derive_wallet_key(password, &wallet.salt, &wallet.kdf_params)?;

    let cipher = aes_gcm::Aes256Gcm::new_from_slice(&key.0)
        .map_err(|e| NonosError::Crypto(format!("Cipher init: {}", e)))?;

    let nonce = aes_gcm::Nonce::from_slice(&wallet.nonce);
    cipher
        .decrypt(nonce, wallet.ciphertext.as_ref())
        .map_err(|e| NonosError::Crypto(format!("Decrypt: {}", e)))
}

pub fn migrate_wallet(password: &[u8], v1_wallet: &EncryptedWallet) -> NonosResult<EncryptedWallet> {
    if v1_wallet.version != 1 {
        return Err(NonosError::Crypto("Only v1 migration supported".into()));
    }
    let plaintext = decrypt_wallet_v1(password, v1_wallet)?;
    encrypt_wallet(password, &plaintext)
}

pub fn is_wallet_supported(wallet: &EncryptedWallet) -> bool {
    wallet.version == 1 || wallet.version == 2
}

pub fn encryption_info(wallet: &EncryptedWallet) -> String {
    format!(
        "v{}: {}, KDF: {} ({}KiB, {}it, {}p)",
        wallet.version,
        wallet.algorithm,
        wallet.kdf_params.algorithm,
        wallet.kdf_params.memory_kib,
        wallet.kdf_params.iterations,
        wallet.kdf_params.parallelism
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let password = b"strong password 123!";
        let plaintext = b"wallet secret data";

        let encrypted = encrypt_wallet(password, plaintext).unwrap();
        assert_eq!(encrypted.version, 2);

        let decrypted = decrypt_wallet(password, &encrypted).unwrap();
        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn test_wrong_password() {
        let encrypted = encrypt_wallet(b"correct", b"secret").unwrap();
        assert!(decrypt_wallet(b"wrong", &encrypted).is_err());
    }

    #[test]
    fn test_tampered_ciphertext() {
        let mut encrypted = encrypt_wallet(b"pass", b"data").unwrap();
        if !encrypted.ciphertext.is_empty() {
            encrypted.ciphertext[0] ^= 0xff;
        }
        assert!(decrypt_wallet(b"pass", &encrypted).is_err());
    }

    #[test]
    fn test_tampered_checksum() {
        let mut encrypted = encrypt_wallet(b"pass", b"data").unwrap();
        encrypted.checksum[0] ^= 0xff;
        let result = decrypt_wallet(b"pass", &encrypted);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Checksum"));
    }

    #[test]
    fn test_key_deterministic() {
        let salt = [0xab; 32];
        let params = KdfParams::default();
        let k1 = derive_wallet_key(b"test", &salt, &params).unwrap();
        let k2 = derive_wallet_key(b"test", &salt, &params).unwrap();
        assert_eq!(k1.0, k2.0);
    }

    #[test]
    fn test_different_salt() {
        let params = KdfParams::default();
        let k1 = derive_wallet_key(b"test", &[0xab; 32], &params).unwrap();
        let k2 = derive_wallet_key(b"test", &[0xcd; 32], &params).unwrap();
        assert_ne!(k1.0, k2.0);
    }
}
