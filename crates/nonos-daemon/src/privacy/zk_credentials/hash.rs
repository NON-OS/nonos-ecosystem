//! Hash utilities for ZK credentials.
//!
//! Re-exports the canonical Poseidon implementation from nonos-crypto.
//! All hash operations MUST use these functions to ensure cryptographic consistency.

use ark_bn254::Fr;
pub use nonos_crypto::poseidon_canonical::{
    canonical_config as get_poseidon_config,
    bytes_to_fr as bytes_to_field,
    fr_to_bytes as field_to_bytes,
    poseidon_hash_fields as poseidon_hash_native,
};

/// Blake3 hash to 32 bytes.
pub fn blake3_hash_32(data: &[u8]) -> [u8; 32] {
    *blake3::hash(data).as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_consistency() {
        let a = Fr::from(12345u64);
        let b = Fr::from(67890u64);

        let h1 = poseidon_hash_native(&[a, b]);
        let h2 = poseidon_hash_native(&[a, b]);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_field_roundtrip() {
        let original = [0xabu8; 32];
        let field = bytes_to_field(&original);
        let restored = field_to_bytes(&field);
        // Note: may differ due to field reduction, but should be consistent
        let field2 = bytes_to_field(&restored);
        assert_eq!(field, field2);
    }
}
