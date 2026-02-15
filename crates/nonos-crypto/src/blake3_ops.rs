use nonos_types::{Blake3Hash, Blake3Key, NonosResult};

const NONOS_KDF_CONTEXT: &str = "NONOS-v1-key-derivation";

const WALLET_MASTER_CONTEXT: &str = "NONOS-v1-wallet-master";
const _TX_SIGNING_CONTEXT: &str = "NONOS-v1-tx-signing";
const STEALTH_CONTEXT: &str = "NONOS-v1-stealth";

pub fn blake3_hash(data: &[u8]) -> Blake3Hash {
    let hash = blake3::hash(data);
    Blake3Hash::from_bytes(*hash.as_bytes())
}

pub fn blake3_hash_domain(domain: &str, data: &[u8]) -> Blake3Hash {
    let mut hasher = blake3::Hasher::new_derive_key(domain);
    hasher.update(data);
    Blake3Hash::from_bytes(*hasher.finalize().as_bytes())
}

pub fn blake3_derive_key(context: &str, seed: &[u8]) -> Blake3Key {
    let mut hasher = blake3::Hasher::new_derive_key(context);
    hasher.update(seed);
    let output = hasher.finalize();
    Blake3Key::from_bytes(*output.as_bytes())
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
    let child = derive_child_key(master_key, &path);
    child.0
}

pub fn blake3_mac(key: &Blake3Key, data: &[u8]) -> Blake3Hash {
    let hasher = blake3::Hasher::new_keyed(&key.0);
    let mut hasher = hasher;
    hasher.update(data);
    Blake3Hash::from_bytes(*hasher.finalize().as_bytes())
}

pub fn blake3_mac_verify(key: &Blake3Key, data: &[u8], expected_mac: &Blake3Hash) -> bool {
    let computed = blake3_mac(key, data);
    crate::constant_time_eq(&computed.0, &expected_mac.0)
}

pub struct Blake3Hasher {
    inner: blake3::Hasher,
}

impl Blake3Hasher {
    pub fn new() -> Self {
        Self {
            inner: blake3::Hasher::new(),
        }
    }

    pub fn new_derive_key(context: &str) -> Self {
        Self {
            inner: blake3::Hasher::new_derive_key(context),
        }
    }

    pub fn new_keyed(key: &Blake3Key) -> Self {
        Self {
            inner: blake3::Hasher::new_keyed(&key.0),
        }
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

pub fn derive_stealth_shared_secret(
    ephemeral_private: &[u8; 32],
    recipient_public: &[u8; 33],
) -> NonosResult<Blake3Hash> {
    let mut input = Vec::with_capacity(65);
    input.extend_from_slice(ephemeral_private);
    input.extend_from_slice(recipient_public);
    Ok(blake3_hash_domain(STEALTH_CONTEXT, &input))
}

pub fn compute_view_tag(shared_secret: &Blake3Hash) -> [u8; 4] {
    let mut tag = [0u8; 4];
    tag.copy_from_slice(&shared_secret.0[..4]);
    tag
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blake3_hash() {
        let data = b"NONOS test data";
        let hash1 = blake3_hash(data);
        let hash2 = blake3_hash(data);
        assert_eq!(hash1, hash2);

        let different = blake3_hash(b"different data");
        assert_ne!(hash1, different);
    }

    #[test]
    fn test_blake3_domain_separation() {
        let data = b"same data";
        let hash1 = blake3_hash_domain("domain1", data);
        let hash2 = blake3_hash_domain("domain2", data);
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_key_derivation() {
        let seed = b"test mnemonic entropy for NONOS wallet";
        let master = derive_wallet_master_key(seed);

        let master2 = derive_wallet_master_key(seed);
        assert_eq!(master.0, master2.0);

        let different = derive_wallet_master_key(b"different seed");
        assert_ne!(master.0, different.0);
    }

    #[test]
    fn test_child_key_derivation() {
        let master = derive_wallet_master_key(b"test seed");

        let child1 = derive_child_key(&master, &[0, 0]);
        let child2 = derive_child_key(&master, &[0, 1]);
        assert_ne!(child1.0, child2.0);

        let child1_again = derive_child_key(&master, &[0, 0]);
        assert_eq!(child1.0, child1_again.0);
    }

    #[test]
    fn test_blake3_mac() {
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
        let data = b"hello world";

        let hash1 = blake3_hash(data);

        let mut hasher = Blake3Hasher::new();
        hasher.update(b"hello ");
        hasher.update(b"world");
        let hash2 = hasher.finalize();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_view_tag() {
        let secret = Blake3Hash::from_bytes([0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        let tag = compute_view_tag(&secret);
        assert_eq!(tag, [0x12, 0x34, 0x56, 0x78]);
    }
}
