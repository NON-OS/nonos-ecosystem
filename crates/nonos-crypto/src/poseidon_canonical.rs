//! Canonical Poseidon hash implementation for NONOS.
//!
//! This module provides a single, unified Poseidon hash function used throughout
//! the NONOS ecosystem. All commitments, nullifiers, and Merkle tree operations
//! MUST use these functions to ensure cryptographic consistency.
//!
//! ## Parameters (BN254 Scalar Field)
//! - Field: BN254 Fr (scalar field)
//! - Width: 3 (rate=2, capacity=1)
//! - Full rounds: 8
//! - Partial rounds: 57
//! - S-box: x^5
//! - Round constants: Grain LFSR (arkworks standard)
//!
//! ## Output Convention
//! All hash functions output the FIRST element of the sponge state after squeezing.
//! This is the standard convention used by arkworks PoseidonSponge.

use ark_bn254::Fr;
use ark_crypto_primitives::sponge::{
    poseidon::{find_poseidon_ark_and_mds, PoseidonConfig, PoseidonSponge},
    CryptographicSponge,
};
use ark_ff::PrimeField;
use ark_serialize::CanonicalSerialize;
use std::sync::OnceLock;

/// Canonical Poseidon configuration.
/// Uses arkworks standard constants (Grain LFSR).
static CANONICAL_CONFIG: OnceLock<PoseidonConfig<Fr>> = OnceLock::new();

/// Get the canonical Poseidon configuration.
/// Thread-safe singleton initialization.
pub fn canonical_config() -> &'static PoseidonConfig<Fr> {
    CANONICAL_CONFIG.get_or_init(|| {
        let rate = 2;
        let alpha = 5u64;
        let full_rounds = 8;
        let partial_rounds = 57;
        let field_bits = 254;

        let (ark, mds) = find_poseidon_ark_and_mds::<Fr>(
            field_bits,
            rate,
            full_rounds,
            partial_rounds,
            0, // skip_matrices
        );

        PoseidonConfig {
            full_rounds: full_rounds as usize,
            partial_rounds: partial_rounds as usize,
            alpha,
            ark,
            mds,
            rate,
            capacity: 1,
        }
    })
}

/// Hash arbitrary number of field elements using canonical Poseidon.
/// Returns the first squeezed element.
pub fn poseidon_hash_fields(inputs: &[Fr]) -> Fr {
    let config = canonical_config();
    let mut sponge = PoseidonSponge::new(config);
    for input in inputs {
        sponge.absorb(input);
    }
    let output: Vec<Fr> = sponge.squeeze_field_elements(1);
    output[0]
}

/// Hash two field elements. Primary operation for Merkle trees.
pub fn poseidon_hash2_fields(left: Fr, right: Fr) -> Fr {
    poseidon_hash_fields(&[left, right])
}

/// Hash three field elements. Used for nullifier computation.
pub fn poseidon_hash3_fields(a: Fr, b: Fr, c: Fr) -> Fr {
    poseidon_hash_fields(&[a, b, c])
}

/// Hash single field element. Used for leaf hashing.
pub fn poseidon_hash1_field(input: Fr) -> Fr {
    poseidon_hash_fields(&[input])
}

// ============================================================================
// Byte Interface (32-byte arrays)
// ============================================================================

/// Convert field element to 32 bytes (little-endian).
pub fn fr_to_bytes(f: &Fr) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    f.serialize_compressed(&mut bytes[..]).expect("Fr serialization failed");
    bytes
}

/// Convert 32 bytes to field element (mod order).
pub fn bytes_to_fr(bytes: &[u8; 32]) -> Fr {
    Fr::from_le_bytes_mod_order(bytes)
}

/// Hash two 32-byte arrays.
pub fn poseidon_hash2(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let result = poseidon_hash2_fields(bytes_to_fr(left), bytes_to_fr(right));
    fr_to_bytes(&result)
}

/// Hash single 32-byte array.
pub fn poseidon_hash(data: &[u8; 32]) -> [u8; 32] {
    let result = poseidon_hash1_field(bytes_to_fr(data));
    fr_to_bytes(&result)
}

/// Hash arbitrary bytes (converted to single field element).
pub fn poseidon_hash_bytes(data: &[u8]) -> [u8; 32] {
    let fr = Fr::from_le_bytes_mod_order(data);
    let result = poseidon_hash1_field(fr);
    fr_to_bytes(&result)
}

// ============================================================================
// Commitment Operations
// ============================================================================

/// Compute Pedersen-style commitment: H(value, blinding).
/// Used for hiding values in ZK proofs.
pub fn poseidon_commitment(value: &[u8; 32], blinding: &[u8; 32]) -> [u8; 32] {
    poseidon_hash2(value, blinding)
}

/// Compute commitment with field elements.
pub fn poseidon_commitment_field(value: Fr, blinding: Fr) -> Fr {
    poseidon_hash2_fields(value, blinding)
}

// ============================================================================
// Nullifier Operations
// ============================================================================

/// Compute nullifier: H(spending_key, note_commitment).
/// Used to prevent double-spending.
pub fn compute_nullifier(spending_key: &[u8; 32], note_commitment: &[u8; 32]) -> [u8; 32] {
    poseidon_hash2(spending_key, note_commitment)
}

/// Compute nullifier with field elements.
pub fn compute_nullifier_field(spending_key: Fr, note_commitment: Fr) -> Fr {
    poseidon_hash2_fields(spending_key, note_commitment)
}

/// Compute scoped nullifier: H(secret, commitment, scope).
/// Used for identity proofs with scope separation.
pub fn compute_scoped_nullifier(secret: &[u8; 32], commitment: &[u8; 32], scope: &[u8; 32]) -> [u8; 32] {
    let result = poseidon_hash3_fields(
        bytes_to_fr(secret),
        bytes_to_fr(commitment),
        bytes_to_fr(scope),
    );
    fr_to_bytes(&result)
}

/// Compute scoped nullifier with field elements.
pub fn compute_scoped_nullifier_field(secret: Fr, commitment: Fr, scope: Fr) -> Fr {
    poseidon_hash3_fields(secret, commitment, scope)
}

// ============================================================================
// Merkle Tree
// ============================================================================

/// Canonical Poseidon Merkle tree with configurable depth.
pub struct PoseidonMerkleTree {
    leaves: Vec<Fr>,
    depth: usize,
    zero_values: Vec<Fr>,
}

impl PoseidonMerkleTree {
    /// Create new tree with given depth.
    /// Precomputes zero values for empty nodes.
    pub fn new(depth: usize) -> Self {
        let mut zero_values = Vec::with_capacity(depth + 1);

        // Zero leaf is H(0)
        let zero_leaf = poseidon_hash1_field(Fr::from(0u64));
        zero_values.push(zero_leaf);

        // Compute zero values for each level
        let mut current = zero_leaf;
        for _ in 0..depth {
            current = poseidon_hash2_fields(current, current);
            zero_values.push(current);
        }

        Self {
            leaves: Vec::new(),
            depth,
            zero_values,
        }
    }

    /// Insert leaf (32 bytes).
    pub fn insert(&mut self, leaf: [u8; 32]) -> usize {
        self.insert_field(bytes_to_fr(&leaf))
    }

    /// Insert leaf (field element).
    pub fn insert_field(&mut self, leaf: Fr) -> usize {
        let index = self.leaves.len();
        self.leaves.push(leaf);
        index
    }

    /// Get current root.
    pub fn root(&self) -> [u8; 32] {
        fr_to_bytes(&self.root_field())
    }

    /// Get current root as field element.
    pub fn root_field(&self) -> Fr {
        if self.leaves.is_empty() {
            return self.zero_values[self.depth];
        }

        let target_len = 1usize << self.depth;
        let mut level: Vec<Fr> = self.leaves.clone();

        // Pad to full level with zero leaves
        while level.len() < target_len {
            level.push(self.zero_values[0]);
        }

        let mut depth_idx = 0;
        while level.len() > 1 {
            let mut next_level = Vec::with_capacity(level.len() / 2);
            for chunk in level.chunks(2) {
                let left = chunk[0];
                let right = if chunk.len() > 1 {
                    chunk[1]
                } else {
                    self.zero_values[depth_idx]
                };
                next_level.push(poseidon_hash2_fields(left, right));
            }
            level = next_level;
            depth_idx += 1;
        }

        level[0]
    }

    /// Get Merkle proof for leaf at index.
    pub fn proof(&self, index: usize) -> Vec<([u8; 32], bool)> {
        self.proof_field(index)
            .into_iter()
            .map(|(sibling, is_left)| (fr_to_bytes(&sibling), is_left))
            .collect()
    }

    /// Get Merkle proof as field elements.
    pub fn proof_field(&self, index: usize) -> Vec<(Fr, bool)> {
        let mut proof = Vec::new();
        let target_len = 1usize << self.depth;

        let mut level: Vec<Fr> = self.leaves.clone();
        while level.len() < target_len {
            level.push(self.zero_values[0]);
        }

        let mut idx = index;
        let mut depth_idx = 0;

        while level.len() > 1 {
            let sibling_idx = if idx % 2 == 0 { idx + 1 } else { idx - 1 };
            let is_left = idx % 2 == 0;

            let sibling = if sibling_idx < level.len() {
                level[sibling_idx]
            } else {
                self.zero_values[depth_idx]
            };

            proof.push((sibling, is_left));

            let mut next_level = Vec::with_capacity(level.len() / 2);
            for chunk in level.chunks(2) {
                let left = chunk[0];
                let right = if chunk.len() > 1 {
                    chunk[1]
                } else {
                    self.zero_values[depth_idx]
                };
                next_level.push(poseidon_hash2_fields(left, right));
            }
            level = next_level;
            idx /= 2;
            depth_idx += 1;
        }

        proof
    }

    /// Verify a Merkle proof.
    pub fn verify_proof(leaf: &[u8; 32], proof: &[([u8; 32], bool)], root: &[u8; 32]) -> bool {
        let leaf_fr = bytes_to_fr(leaf);
        let root_fr = bytes_to_fr(root);
        let proof_fr: Vec<(Fr, bool)> = proof
            .iter()
            .map(|(sibling, is_left)| (bytes_to_fr(sibling), *is_left))
            .collect();
        Self::verify_proof_field(leaf_fr, &proof_fr, root_fr)
    }

    /// Verify a Merkle proof with field elements.
    pub fn verify_proof_field(leaf: Fr, proof: &[(Fr, bool)], root: Fr) -> bool {
        let mut current = leaf;

        for (sibling, is_left) in proof {
            if *is_left {
                current = poseidon_hash2_fields(current, *sibling);
            } else {
                current = poseidon_hash2_fields(*sibling, current);
            }
        }

        current == root
    }

    /// Number of leaves in tree.
    pub fn len(&self) -> usize {
        self.leaves.len()
    }

    /// Check if tree is empty.
    pub fn is_empty(&self) -> bool {
        self.leaves.is_empty()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_deterministic() {
        let a = Fr::from(12345u64);
        let b = Fr::from(67890u64);

        let h1 = poseidon_hash2_fields(a, b);
        let h2 = poseidon_hash2_fields(a, b);
        assert_eq!(h1, h2);

        // Order matters
        let h3 = poseidon_hash2_fields(b, a);
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_hash_bytes() {
        let left = [0xaa; 32];
        let right = [0xbb; 32];

        let hash1 = poseidon_hash2(&left, &right);
        let hash2 = poseidon_hash2(&left, &right);
        assert_eq!(hash1, hash2);

        // Order matters
        let hash3 = poseidon_hash2(&right, &left);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_commitment() {
        let value = [0x11; 32];
        let blinding = [0x22; 32];

        let c1 = poseidon_commitment(&value, &blinding);
        let c2 = poseidon_commitment(&value, &blinding);
        assert_eq!(c1, c2);

        // Different blinding -> different commitment
        let c3 = poseidon_commitment(&value, &[0x33; 32]);
        assert_ne!(c1, c3);
    }

    #[test]
    fn test_nullifier() {
        let sk = [0xaa; 32];
        let nc = [0xbb; 32];

        let n1 = compute_nullifier(&sk, &nc);
        let n2 = compute_nullifier(&sk, &nc);
        assert_eq!(n1, n2);

        // Different commitment -> different nullifier
        let n3 = compute_nullifier(&sk, &[0xcc; 32]);
        assert_ne!(n1, n3);
    }

    #[test]
    fn test_scoped_nullifier() {
        let secret = [0x11; 32];
        let commitment = [0x22; 32];
        let scope1 = [0x33; 32];
        let scope2 = [0x44; 32];

        let n1 = compute_scoped_nullifier(&secret, &commitment, &scope1);
        let n2 = compute_scoped_nullifier(&secret, &commitment, &scope2);

        // Different scopes -> different nullifiers
        assert_ne!(n1, n2);

        // Same inputs -> same nullifier
        let n3 = compute_scoped_nullifier(&secret, &commitment, &scope1);
        assert_eq!(n1, n3);
    }

    #[test]
    fn test_merkle_tree() {
        let mut tree = PoseidonMerkleTree::new(4);

        let leaf1 = [0x11; 32];
        let leaf2 = [0x22; 32];
        let leaf3 = [0x33; 32];

        let idx1 = tree.insert(leaf1);
        let idx2 = tree.insert(leaf2);
        let idx3 = tree.insert(leaf3);

        let root = tree.root();

        // Verify all proofs
        let proof1 = tree.proof(idx1);
        assert!(PoseidonMerkleTree::verify_proof(&leaf1, &proof1, &root));

        let proof2 = tree.proof(idx2);
        assert!(PoseidonMerkleTree::verify_proof(&leaf2, &proof2, &root));

        let proof3 = tree.proof(idx3);
        assert!(PoseidonMerkleTree::verify_proof(&leaf3, &proof3, &root));

        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(idx3, 2);
    }

    #[test]
    fn test_merkle_proof_invalid() {
        let mut tree = PoseidonMerkleTree::new(4);

        let leaf1 = [0x11; 32];
        let leaf2 = [0x22; 32];

        tree.insert(leaf1);
        tree.insert(leaf2);

        let root = tree.root();
        let proof1 = tree.proof(0);

        // Wrong leaf fails
        assert!(!PoseidonMerkleTree::verify_proof(&leaf2, &proof1, &root));

        // Wrong root fails
        let wrong_root = [0xff; 32];
        assert!(!PoseidonMerkleTree::verify_proof(&leaf1, &proof1, &wrong_root));
    }

    #[test]
    fn test_empty_tree() {
        let tree = PoseidonMerkleTree::new(4);
        let root = tree.root();
        // Empty tree has deterministic root
        assert_ne!(root, [0u8; 32]);
    }

    #[test]
    fn test_field_roundtrip() {
        let original = Fr::from(0xdeadbeefu64);
        let bytes = fr_to_bytes(&original);
        let restored = bytes_to_fr(&bytes);
        assert_eq!(original, restored);
    }

    #[test]
    fn test_hash_multiple_inputs() {
        let inputs = vec![
            Fr::from(1u64),
            Fr::from(2u64),
            Fr::from(3u64),
            Fr::from(4u64),
        ];

        let h1 = poseidon_hash_fields(&inputs);
        let h2 = poseidon_hash_fields(&inputs);
        assert_eq!(h1, h2);

        // Different order -> different hash
        let inputs_rev: Vec<Fr> = inputs.iter().rev().cloned().collect();
        let h3 = poseidon_hash_fields(&inputs_rev);
        assert_ne!(h1, h3);
    }
}
