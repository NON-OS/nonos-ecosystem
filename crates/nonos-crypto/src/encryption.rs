use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use nonos_types::{Blake3Key, NonosError, NonosResult};

const NONCE_SIZE: usize = 12;

const TAG_SIZE: usize = 16;

pub fn encrypt(key: &Blake3Key, plaintext: &[u8]) -> NonosResult<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(&key.0)
        .map_err(|e| NonosError::Crypto(e.to_string()))?;

    let nonce_bytes = crate::random_bytes::<NONCE_SIZE>();
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| NonosError::Crypto(e.to_string()))?;

    let mut result = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);

    Ok(result)
}

pub fn decrypt(key: &Blake3Key, encrypted: &[u8]) -> NonosResult<Vec<u8>> {
    if encrypted.len() < NONCE_SIZE + TAG_SIZE {
        return Err(NonosError::Crypto("Encrypted data too short".into()));
    }

    let cipher = Aes256Gcm::new_from_slice(&key.0)
        .map_err(|e| NonosError::Crypto(e.to_string()))?;

    let nonce = Nonce::from_slice(&encrypted[..NONCE_SIZE]);
    let ciphertext = &encrypted[NONCE_SIZE..];

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| NonosError::Crypto(format!("Decryption failed: {}", e)))
}

pub fn encrypt_with_aad(
    key: &Blake3Key,
    plaintext: &[u8],
    aad: &[u8],
) -> NonosResult<Vec<u8>> {
    use aes_gcm::aead::Payload;

    let cipher = Aes256Gcm::new_from_slice(&key.0)
        .map_err(|e| NonosError::Crypto(e.to_string()))?;

    let nonce_bytes = crate::random_bytes::<NONCE_SIZE>();
    let nonce = Nonce::from_slice(&nonce_bytes);

    let payload = Payload {
        msg: plaintext,
        aad,
    };

    let ciphertext = cipher
        .encrypt(nonce, payload)
        .map_err(|e| NonosError::Crypto(e.to_string()))?;

    let mut result = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);

    Ok(result)
}

pub fn decrypt_with_aad(
    key: &Blake3Key,
    encrypted: &[u8],
    aad: &[u8],
) -> NonosResult<Vec<u8>> {
    use aes_gcm::aead::Payload;

    if encrypted.len() < NONCE_SIZE + TAG_SIZE {
        return Err(NonosError::Crypto("Encrypted data too short".into()));
    }

    let cipher = Aes256Gcm::new_from_slice(&key.0)
        .map_err(|e| NonosError::Crypto(e.to_string()))?;

    let nonce = Nonce::from_slice(&encrypted[..NONCE_SIZE]);
    let ciphertext = &encrypted[NONCE_SIZE..];

    let payload = Payload {
        msg: ciphertext,
        aad,
    };

    cipher
        .decrypt(nonce, payload)
        .map_err(|e| NonosError::Crypto(format!("Decryption failed: {}", e)))
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct EncryptedData {
    pub version: u8,
    pub algorithm: String,
    pub payload: Vec<u8>,
}

impl EncryptedData {
    pub fn new(key: &Blake3Key, plaintext: &[u8]) -> NonosResult<Self> {
        let payload = encrypt(key, plaintext)?;
        Ok(Self {
            version: 1,
            algorithm: "AES-256-GCM".to_string(),
            payload,
        })
    }

    pub fn decrypt(&self, key: &Blake3Key) -> NonosResult<Vec<u8>> {
        if self.version != 1 {
            return Err(NonosError::Crypto(format!(
                "Unsupported encryption version: {}",
                self.version
            )));
        }
        if self.algorithm != "AES-256-GCM" {
            return Err(NonosError::Crypto(format!(
                "Unsupported algorithm: {}",
                self.algorithm
            )));
        }
        decrypt(key, &self.payload)
    }
}

pub fn derive_key_from_password(password: &[u8], salt: &[u8], iterations: u32) -> Blake3Key {
    let mut key = crate::blake3_hash(password);

    for _ in 0..iterations {
        let mut input = Vec::with_capacity(64);
        input.extend_from_slice(&key.0);
        input.extend_from_slice(salt);
        key = crate::blake3_hash(&input);
    }

    Blake3Key::from_bytes(key.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let key = Blake3Key::from_bytes([0xab; 32]);
        let plaintext = b"NONOS secret wallet data";

        let encrypted = encrypt(&key, plaintext).unwrap();
        assert!(encrypted.len() > plaintext.len());

        let decrypted = decrypt(&key, &encrypted).unwrap();
        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn test_wrong_key_fails() {
        let key1 = Blake3Key::from_bytes([0xab; 32]);
        let key2 = Blake3Key::from_bytes([0xcd; 32]);
        let plaintext = b"secret data";

        let encrypted = encrypt(&key1, plaintext).unwrap();
        let result = decrypt(&key2, &encrypted);

        assert!(result.is_err());
    }

    #[test]
    fn test_tampered_data_fails() {
        let key = Blake3Key::from_bytes([0xab; 32]);
        let plaintext = b"secret data";

        let mut encrypted = encrypt(&key, plaintext).unwrap();

        let last_idx = encrypted.len() - 1;
        encrypted[last_idx] ^= 0xff;

        let result = decrypt(&key, &encrypted);
        assert!(result.is_err());
    }

    #[test]
    fn test_encrypt_with_aad() {
        let key = Blake3Key::from_bytes([0xab; 32]);
        let plaintext = b"secret data";
        let aad = b"associated data";

        let encrypted = encrypt_with_aad(&key, plaintext, aad).unwrap();
        let decrypted = decrypt_with_aad(&key, &encrypted, aad).unwrap();

        assert_eq!(plaintext.as_slice(), decrypted.as_slice());

        let result = decrypt_with_aad(&key, &encrypted, b"wrong aad");
        assert!(result.is_err());
    }

    #[test]
    fn test_encrypted_data_wrapper() {
        let key = Blake3Key::from_bytes([0xab; 32]);
        let plaintext = b"wallet private key data";

        let encrypted = EncryptedData::new(&key, plaintext).unwrap();
        assert_eq!(encrypted.version, 1);
        assert_eq!(encrypted.algorithm, "AES-256-GCM");

        let decrypted = encrypted.decrypt(&key).unwrap();
        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn test_password_key_derivation() {
        let password = b"user password";
        let salt = b"random salt";

        let key1 = derive_key_from_password(password, salt, 100);
        let key2 = derive_key_from_password(password, salt, 100);

        assert_eq!(key1.0, key2.0);

        let key3 = derive_key_from_password(b"different", salt, 100);
        assert_ne!(key1.0, key3.0);

        let key4 = derive_key_from_password(password, salt, 200);
        assert_ne!(key1.0, key4.0);
    }
}
