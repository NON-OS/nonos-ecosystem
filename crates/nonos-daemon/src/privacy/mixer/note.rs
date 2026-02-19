use super::types::AssetId;
use ark_bn254::Fr;
use nonos_crypto::poseidon_canonical::{bytes_to_fr, fr_to_bytes, poseidon_hash_fields};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct NoteSecret {
    pub secret: [u8; 32],
    pub randomness: [u8; 32],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotePublic {
    pub amount: u128,
    pub asset: AssetId,
    pub commitment: [u8; 32],
    pub tree_index: Option<usize>,
}

pub struct Note {
    pub private: NoteSecret,
    pub public: NotePublic,
}

impl Note {
    pub fn new(secret: [u8; 32], amount: u128, asset: AssetId, randomness: [u8; 32]) -> Self {
        let commitment = Self::compute_commitment(&secret, amount, &asset, &randomness);
        Self {
            private: NoteSecret { secret, randomness },
            public: NotePublic {
                amount,
                asset,
                commitment,
                tree_index: None,
            },
        }
    }

    pub fn compute_commitment(
        secret: &[u8; 32],
        amount: u128,
        asset: &AssetId,
        randomness: &[u8; 32],
    ) -> [u8; 32] {
        let secret_fr = bytes_to_fr(secret);
        let amount_fr = Fr::from(amount);
        let mut asset_bytes = [0u8; 32];
        asset_bytes[..8].copy_from_slice(asset);
        let asset_fr = bytes_to_fr(&asset_bytes);
        let randomness_fr = bytes_to_fr(randomness);

        let result = poseidon_hash_fields(&[secret_fr, amount_fr, asset_fr, randomness_fr]);
        fr_to_bytes(&result)
    }

    pub fn nullifier(&self) -> [u8; 32] {
        Self::compute_nullifier(&self.private.secret, &self.public.commitment)
    }

    pub fn compute_nullifier(secret: &[u8; 32], commitment: &[u8; 32]) -> [u8; 32] {
        let result = poseidon_hash_fields(&[bytes_to_fr(secret), bytes_to_fr(commitment)]);
        fr_to_bytes(&result)
    }

    pub fn commitment(&self) -> [u8; 32] {
        self.public.commitment
    }

    pub fn set_tree_index(&mut self, index: usize) {
        self.public.tree_index = Some(index);
    }
}

impl Drop for Note {
    fn drop(&mut self) {
        self.private.zeroize();
    }
}
