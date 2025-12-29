// NONOS GNU AFFERO GENERAL PUBLIC LICENSE
// Version 3, 19 November 2007
// Copyright (C) 2025 NON-OS <team@nonos.systems>
//
//! BLAKE3 is the primary hash function for NONOS, providing:
//! Fast content hashing, secure key derivation (KDF), keyed message 
//! authentication (MAC) and domain-separated hashing

use nonos_types::{Blake3Hash, Blake3Key};

const NONOS_KDF_CONTEXT: &str = "NONOS-v1-key-derivation";
const WALLET_MASTER_CONTEXT: &str = "NONOS-v1-wallet-master";

pub fn blake3_hash(data: &[u8]) -> Blake3Hash {
    Blake3Hash::from_bytes(*blake3::hash(data).as_bytes())
}

pub fn blake3_hash_domain(domain: &str, data: &[u8]) -> Blake3Hash {
    let mut hasher = blake3::Hasher::new_derive_key(domain);
    hasher.update(data);
    Blake3Hash::from_bytes(*hasher.finalize().as_bytes())
}

pub fn blake3_derive_key(context: &str, seed: &[u8]) -> Blake3Key {
    let mut hasher = blake3::Hasher::new_derive_key(context);
    hasher.update(seed);
    Blake3Key::from_bytes(*hasher.finalize().as_bytes())
}

pub fn derive_wallet_master_key(mnemonic_entropy: &[u8]) -> Blake3Key {
    blake3_derive_key(WALLET_MASTER_CONTEXT, mnemonic_entropy)
}

pub fn derive_child_key(master_key: &Blake3Key, path: &[u32]) -> Blake3Key {
    let mut current = master_key.0;

    for &index in path {
        let mut input = Vec::with_capacity(36);
        input.extend_from_slice(&current);
        input.extend_from_slice(&index.to_be_bytes());

        let mut hasher = blake3::Hasher::new_derive_key(NONOS_KDF_CONTEXT);
        hasher.update(&input);
        current = *hasher.finalize().as_bytes();
    }

    Blake3Key::from_bytes(current)
}

pub fn derive_secp256k1_key(master_key: &Blake3Key, account: u32, index: u32) -> [u8; 32] {
    let path = [44, 60, account, 0, index];
    derive_child_key(master_key, &path).0
}

pub fn blake3_mac(key: &Blake3Key, data: &[u8]) -> Blake3Hash {
    let mut hasher = blake3::Hasher::new_keyed(&key.0);
    hasher.update(data);
    Blake3Hash::from_bytes(*hasher.finalize().as_bytes())
}

pub fn blake3_mac_verify(key: &Blake3Key, data: &[u8], expected: &Blake3Hash) -> bool {
    let computed = blake3_mac(key, data);
    crate::constant_time_eq(&computed.0, &expected.0)
}

pub struct Blake3Hasher {
    inner: blake3::Hasher,
}

impl Blake3Hasher {
    pub fn new() -> Self {
        Self { inner: blake3::Hasher::new() }
    }

    pub fn new_derive_key(context: &str) -> Self {
        Self { inner: blake3::Hasher::new_derive_key(context) }
    }

    pub fn new_keyed(key: &Blake3Key) -> Self {
        Self { inner: blake3::Hasher::new_keyed(&key.0) }
    }

    pub fn update(&mut self, data: &[u8]) {
        self.inner.update(data);
    }

    pub fn finalize(self) -> Blake3Hash {
        Blake3Hash::from_bytes(*self.inner.finalize().as_bytes())
    }

    pub fn reset(&mut self) {
        self.inner.reset();
    }
}

impl Default for Blake3Hasher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blake3_hash_deterministic() {
        let data = b"NONOS test data";
        assert_eq!(blake3_hash(data), blake3_hash(data));
        assert_ne!(blake3_hash(data), blake3_hash(b"different"));
    }

    #[test]
    fn test_domain_separation() {
        let data = b"same data";
        assert_ne!(
            blake3_hash_domain("domain1", data),
            blake3_hash_domain("domain2", data)
        );
    }

    #[test]
    fn test_key_derivation() {
        let seed = b"test mnemonic entropy";
        let master1 = derive_wallet_master_key(seed);
        let master2 = derive_wallet_master_key(seed);
        assert_eq!(master1.0, master2.0);

        let different = derive_wallet_master_key(b"different seed");
        assert_ne!(master1.0, different.0);
    }

    #[test]
    fn test_child_key_derivation() {
        let master = derive_wallet_master_key(b"test seed");
        let child1 = derive_child_key(&master, &[0, 0]);
        let child2 = derive_child_key(&master, &[0, 1]);
        assert_ne!(child1.0, child2.0);
        assert_eq!(child1.0, derive_child_key(&master, &[0, 0]).0);
    }

    #[test]
    fn test_mac_verification() {
        let key = Blake3Key::from_bytes([0xab; 32]);
        let data = b"message to authenticate";
        let mac = blake3_mac(&key, data);

        assert!(blake3_mac_verify(&key, data, &mac));
        assert!(!blake3_mac_verify(&key, b"wrong data", &mac));

        let wrong_key = Blake3Key::from_bytes([0xcd; 32]);
        assert!(!blake3_mac_verify(&wrong_key, data, &mac));
    }

    #[test]
    fn test_incremental_hasher() {
        let hash1 = blake3_hash(b"hello world");

        let mut hasher = Blake3Hasher::new();
        hasher.update(b"hello ");
        hasher.update(b"world");
        let hash2 = hasher.finalize();

        assert_eq!(hash1, hash2);
    }
}
