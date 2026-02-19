#![deny(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod blake3_ops;
pub mod secp256k1_ops;
pub mod ed25519_ops;
pub mod stealth;
pub mod encryption;
pub mod wallet_encryption;
pub mod poseidon;
pub mod poseidon_canonical;
pub mod mnemonic;
pub mod zk_proofs;

pub use blake3_ops::*;
pub use secp256k1_ops::*;
pub use ed25519_ops::*;
pub use stealth::*;
pub use encryption::*;
pub use wallet_encryption::{
    EncryptedWallet, KdfParams, encrypt_wallet, decrypt_wallet,
    migrate_wallet, derive_wallet_key, WALLET_ENCRYPTION_VERSION,
};
// Legacy poseidon - use poseidon_canonical for new code
pub use poseidon::*;
// Canonical Poseidon implementation
pub use poseidon_canonical::{
    canonical_config,
    poseidon_hash_fields,
    poseidon_hash2_fields,
    poseidon_hash3_fields,
    poseidon_hash1_field,
    fr_to_bytes as canonical_fr_to_bytes,
    bytes_to_fr as canonical_bytes_to_fr,
    poseidon_hash2 as canonical_hash2,
    poseidon_hash as canonical_hash,
    poseidon_commitment as canonical_commitment,
    compute_nullifier as canonical_nullifier,
    compute_scoped_nullifier,
    PoseidonMerkleTree as CanonicalMerkleTree,
};
pub use mnemonic::*;
pub use zk_proofs::*;

pub fn random_bytes<const N: usize>() -> [u8; N] {
    use rand::RngCore;
    let mut bytes = [0u8; N];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes
}

pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    use subtle::ConstantTimeEq;
    if a.len() != b.len() {
        return false;
    }
    a.ct_eq(b).into()
}
