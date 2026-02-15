#![deny(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod blake3_ops;
pub mod secp256k1_ops;
pub mod ed25519_ops;
pub mod stealth;
pub mod encryption;
pub mod poseidon;
pub mod mnemonic;
pub mod zk_proofs;

pub use blake3_ops::*;
pub use secp256k1_ops::*;
pub use ed25519_ops::*;
pub use stealth::*;
pub use encryption::*;
pub use poseidon::*;
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
