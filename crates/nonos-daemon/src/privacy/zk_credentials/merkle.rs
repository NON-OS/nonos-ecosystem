use ark_bn254::Fr;
use std::sync::OnceLock;

use super::hash::{bytes_to_field, field_to_bytes, poseidon_hash_native};
use super::types::{MerkleProof, MERKLE_DEPTH};

static EMPTY_HASHES: OnceLock<Vec<[u8; 32]>> = OnceLock::new();

fn get_empty_hashes() -> &'static Vec<[u8; 32]> {
    EMPTY_HASHES.get_or_init(|| {
        let mut hashes = Vec::with_capacity(MERKLE_DEPTH + 1);

        let empty_leaf = field_to_bytes(&poseidon_hash_native(&[Fr::from(0u64)]));
        hashes.push(empty_leaf);

        for _ in 0..MERKLE_DEPTH {
            let prev = hashes.last().unwrap();
            let hash = poseidon_hash_native(&[
                bytes_to_field(prev),
                bytes_to_field(prev),
            ]);
            hashes.push(field_to_bytes(&hash));
        }

        hashes
    })
}

pub struct MerkleTree {
    leaves: Vec<[u8; 32]>,
}

impl MerkleTree {
    pub fn new() -> Self {
        let _ = get_empty_hashes();
        Self {
            leaves: Vec::new(),
        }
    }

    pub fn insert(&mut self, commitment: [u8; 32]) {
        self.leaves.push(commitment);
    }

    pub fn leaf_count(&self) -> usize {
        self.leaves.len()
    }

    pub fn root(&self) -> [u8; 32] {
        let empty_hashes = get_empty_hashes();

        if self.leaves.is_empty() {
            return empty_hashes[MERKLE_DEPTH];
        }

        self.compute_root_sparse(empty_hashes)
    }

    fn compute_root_sparse(&self, empty_hashes: &[[u8; 32]]) -> [u8; 32] {
        let n = self.leaves.len();
        if n == 0 {
            return empty_hashes[MERKLE_DEPTH];
        }

        let mut current_level: Vec<[u8; 32]> = self.leaves.clone();

        for empty_hash in empty_hashes.iter().take(MERKLE_DEPTH) {
            let mut next_level = Vec::with_capacity(current_level.len().div_ceil(2));

            let mut i = 0;
            while i < current_level.len() {
                let left = current_level[i];
                let right = if i + 1 < current_level.len() {
                    current_level[i + 1]
                } else {
                    *empty_hash
                };

                let hash = poseidon_hash_native(&[
                    bytes_to_field(&left),
                    bytes_to_field(&right),
                ]);
                next_level.push(field_to_bytes(&hash));

                i += 2;
            }

            if next_level.is_empty() && current_level.len() == 1 {
                let hash = poseidon_hash_native(&[
                    bytes_to_field(&current_level[0]),
                    bytes_to_field(empty_hash),
                ]);
                next_level.push(field_to_bytes(&hash));
            }

            current_level = next_level;
        }

        current_level.first().copied().unwrap_or(empty_hashes[MERKLE_DEPTH])
    }

    pub fn get_proof(&self, commitment: &[u8; 32]) -> Option<MerkleProof> {
        let empty_hashes = get_empty_hashes();
        let leaf_index = self.leaves.iter().position(|l| l == commitment)?;

        let mut path = Vec::with_capacity(MERKLE_DEPTH);
        let mut indices = Vec::with_capacity(MERKLE_DEPTH);

        let mut current_level: Vec<[u8; 32]> = self.leaves.clone();
        let mut idx = leaf_index;

        for empty_hash in empty_hashes.iter().take(MERKLE_DEPTH) {
            let sibling_idx = if idx % 2 == 0 { idx + 1 } else { idx - 1 };

            let sibling = if sibling_idx < current_level.len() {
                current_level[sibling_idx]
            } else {
                *empty_hash
            };

            path.push(sibling);
            indices.push(idx % 2 == 1);

            let mut next_level = Vec::with_capacity(current_level.len().div_ceil(2));
            let mut i = 0;
            while i < current_level.len() {
                let left = current_level[i];
                let right = if i + 1 < current_level.len() {
                    current_level[i + 1]
                } else {
                    *empty_hash
                };

                let hash = poseidon_hash_native(&[
                    bytes_to_field(&left),
                    bytes_to_field(&right),
                ]);
                next_level.push(field_to_bytes(&hash));

                i += 2;
            }

            current_level = next_level;
            idx /= 2;
        }

        Some(MerkleProof {
            path,
            indices,
            leaf_index: leaf_index as u64,
        })
    }
}

impl Default for MerkleTree {
    fn default() -> Self {
        Self::new()
    }
}
