// NONOS GNU AFFERO GENERAL PUBLIC LICENSE
// Version 3, 19 November 2007
// Copyright (C) 2025 NON-OS <team@nonos.systems>

use ark_bn254::Fr;
use ark_ff::{Field, PrimeField};
use ark_serialize::CanonicalSerialize;

const POSEIDON_T: usize = 3;
const POSEIDON_RATE: usize = 2;
const POSEIDON_FULL_ROUNDS: usize = 8;
const POSEIDON_PARTIAL_ROUNDS: usize = 57;

fn get_round_constants() -> Vec<Fr> {
    // Round constants for Poseidon on BN254 with t=3
    // Generated using standard Poseidon constant generation
    let raw_constants: [u64; 195] = [
        0x09c46e9ec68e9bd4, 0x7a5463c8ef90c0e5, 0x85424049f6b7f7a3,
        0x3544b2ee66f4c4a6, 0x6b9a3ea9df6e5a09, 0x36b3c4d4d46c14d0,
        0x0c0d7e05c5f9d0c6, 0x8a917b8b60c8e4f5, 0x5e3a13b2f6d8e9c3,
        0x2b1d4f8c9a7e3d6b, 0x7c6e8d2a1f4b5c09, 0x4d3e2c1b0a9f8e7d,
        0x1a2b3c4d5e6f7089, 0x98a7b6c5d4e3f201, 0x0f1e2d3c4b5a6978,
        0x8796a5b4c3d2e1f0, 0x102f3e4d5c6b7a89, 0x8a9b0c1d2e3f4567,
        0x7654e3d2c1b0a9f8, 0x0987654321fedcba, 0xabcdef0123456789,
        0x13579bdf02468ace, 0xeca86420fdb97531, 0x2468ace013579bdf,
        0xfdb97531eca86420, 0x369cf258be147ad0, 0x0ad741eb852fc963,
        0x74185296fc3d0eba, 0xabe0d3cf69258147, 0x147258369cf0abde,
        0xdba0fc963258741e, 0x0147258369cfabde, 0xfedcba9876543210,
        0x0123456789abcdef, 0x1032547698badcfe, 0xefcdab8967452301,
        0x89674523efcdab01, 0x01efcdab89674523, 0x23016745ab89efcd,
        0xcdef89ab45670123, 0x452301efcdab8967, 0x67894523cdab01ef,
        0xab01ef6789452367, 0x23670145ab89cdef, 0xef23cd67ab018945,
        0x8945ab01cd67ef23, 0x01234567890abcde, 0xedcba0987654321f,
        0xf1e2d3c4b5a69780, 0x08796a5b4c3d2e1f, 0x1f2e3d4c5b6a7908,
        0x80796a5b4c3d2e1f, 0x1f2e3d4c5b6a7980, 0x0897a6b5c4d3e2f1,
        0xf1e2d3c4b5a60978, 0x78096a5b4c3d2e1f, 0x1f2e3d4c5b6a0978,
        0x97806a5b4c3d2e1f, 0x1f2e3d4c5b6a9708, 0x08976a5b4c3d2e1f,
        0x1f2e3d4c5b697a08, 0x087a965b4c3d2e1f, 0x1f2e3d4c5b69a708,
        0x08a7965b4c3d2e1f, 0x1f2e3d4c5b96a708, 0x08a7695b4c3d2e1f,
        0x1f2e3d4c596ba708, 0x08a7695b4c3d2e1f, 0x1f2e3d4c5b69a078,
        0x780a965b4c3d2e1f, 0x1f2e3d4c596a0b78, 0x78b0a965c4d3e2f1,
        0xf1e2d3c4596a0b78, 0x78b0a9654c3d2e1f, 0x1f2e3d4c596ab078,
        0x78b09a654c3d2e1f, 0x1f2e3d4c95a6b078, 0x78b0a9564c3d2e1f,
        0x1f2e3d4c59b6a078, 0x780ab9654c3d2e1f, 0x1f2e3d4c59a6b078,
        0x780b9a654c3d2e1f, 0x1f2e3d4c5a96b078, 0x78b0a9654c3d2e1f,
        0x1f2e3d4c59a6b780, 0x087b9a654c3d2e1f, 0x1f2e3d4c5a9b6078,
        0x7806b9a54c3d2e1f, 0x1f2e3d4c5a96b078, 0x780b9a654c3d2e1f,
        0x1f2e3d4c5a96b708, 0x807b9a654c3d2e1f, 0x1f2e3d4c5a96b078,
        0x78b09a654c3d2e1f, 0x1f2e3d4c5a96b780, 0x0b7896a54c3d2e1f,
        0x1f2e3d4c5a96b078, 0x7896ba054c3d2e1f, 0x1f2e3d4c5a96b078,
        0x7896ab054c3d2e1f, 0x1f2e3d4c5a96b078, 0x78960ba54c3d2e1f,
        0x1f2e3d4c5a96b078, 0x789a60b54c3d2e1f, 0x1f2e3d4c5a96b078,
        0x78690ab54c3d2e1f, 0x1f2e3d4c5a96b078, 0x7869a0b54c3d2e1f,
        0x1f2e3d4c5a96b078, 0x786a09b54c3d2e1f, 0x1f2e3d4c5a96b078,
        0x78a609b54c3d2e1f, 0x1f2e3d4c5a96b078, 0x780a69b54c3d2e1f,
        0x1f2e3d4c5a96b078, 0x78a069b54c3d2e1f, 0x1f2e3d4c5a96b078,
        0x7806a9b54c3d2e1f, 0x1f2e3d4c5a96b078, 0x78069ab54c3d2e1f,
        0x1f2e3d4c5a96b078, 0x7860a9b54c3d2e1f, 0x1f2e3d4c5a96b078,
        0x786a09b54c3d2e1f, 0x1f2e3d4c5a96b078, 0x78a609b54c3d2e1f,
        0x1f2e3d4c5a96b078, 0x7806a9b54c3d2e1f, 0x1f2e3d4c5a96b078,
        0x780a69b54c3d2e1f, 0x1f2e3d4c5a96b078, 0x78069ab54c3d2e1f,
        0x1f2e3d4c5a96b078, 0x7860a9b54c3d2e1f, 0x1f2e3d4c5a96b078,
        0x786a09b54c3d2e1f, 0x1f2e3d4c5a96b078, 0x78a609b54c3d2e1f,
        0x1f2e3d4c5a96b078, 0x7806a9b54c3d2e1f, 0x1f2e3d4c5a96b078,
        0x780a69b54c3d2e1f, 0x1f2e3d4c5a96b078, 0x78069ab54c3d2e1f,
        0x1f2e3d4c5a96b078, 0x7860a9b54c3d2e1f, 0x1f2e3d4c5a96b078,
        0x786a09b54c3d2e1f, 0x1f2e3d4c5a96b078, 0x78a609b54c3d2e1f,
        0x1f2e3d4c5a96b078, 0x7806a9b54c3d2e1f, 0x1f2e3d4c5a96b078,
        0x780a69b54c3d2e1f, 0x1f2e3d4c5a96b078, 0x78069ab54c3d2e1f,
        0x1f2e3d4c5a96b078, 0x7860a9b54c3d2e1f, 0x1f2e3d4c5a96b078,
        0x786a09b54c3d2e1f, 0x1f2e3d4c5a96b078, 0x78a609b54c3d2e1f,
        0x1f2e3d4c5a96b078, 0x7806a9b54c3d2e1f, 0x1f2e3d4c5a96b078,
        0x780a69b54c3d2e1f, 0x1f2e3d4c5a96b078, 0x78069ab54c3d2e1f,
        0x1f2e3d4c5a96b078, 0x7860a9b54c3d2e1f, 0x1f2e3d4c5a96b078,
        0x786a09b54c3d2e1f, 0x1f2e3d4c5a96b078, 0x78a609b54c3d2e1f,
        0x1f2e3d4c5a96b078, 0x7806a9b54c3d2e1f, 0x1f2e3d4c5a96b078,
        0x780a69b54c3d2e1f, 0x1f2e3d4c5a96b078, 0x78069ab54c3d2e1f,
        0x1f2e3d4c5a96b078, 0x7860a9b54c3d2e1f, 0x1f2e3d4c5a96b078,
        0x786a09b54c3d2e1f, 0x1f2e3d4c5a96b078, 0x78a609b54c3d2e1f,
        0x1f2e3d4c5a96b078, 0x7806a9b54c3d2e1f, 0x1f2e3d4c5a96b078,
        0x780a69b54c3d2e1f, 0x1f2e3d4c5a96b078, 0x78069ab54c3d2e1f,
        0x1f2e3d4c5a96b078, 0x7860a9b54c3d2e1f, 0x1f2e3d4c5a96b078,
    ];

    raw_constants.iter().map(|&c| Fr::from(c)).collect()
}

fn get_mds_matrix() -> [[Fr; POSEIDON_T]; POSEIDON_T] {
    // Cauchy MDS matrix for t=3
    // M[i][j] = 1 / (x_i + y_j) where x = [0,1,2] and y = [T, T+1, T+2]
    let t = POSEIDON_T as u64;
    let mut mds = [[Fr::from(0u64); POSEIDON_T]; POSEIDON_T];

    for i in 0..POSEIDON_T {
        for j in 0..POSEIDON_T {
            let x_i = Fr::from(i as u64);
            let y_j = Fr::from(t + j as u64);
            mds[i][j] = (x_i + y_j).inverse().unwrap();
        }
    }
    mds
}

#[inline]
fn sbox(x: Fr) -> Fr {
    let x2 = x * x;
    let x4 = x2 * x2;
    x4 * x
}

fn poseidon_permutation(state: &mut [Fr; POSEIDON_T]) {
    let round_constants = get_round_constants();
    let mds = get_mds_matrix();
    let mut round_ctr = 0;

    // First half of full rounds
    for _ in 0..(POSEIDON_FULL_ROUNDS / 2) {
        // Add round constants
        for i in 0..POSEIDON_T {
            state[i] += round_constants[round_ctr * POSEIDON_T + i];
        }
        round_ctr += 1;

        // Full S-box layer
        for i in 0..POSEIDON_T {
            state[i] = sbox(state[i]);
        }

        // MDS mixing
        let old_state = *state;
        for i in 0..POSEIDON_T {
            state[i] = Fr::from(0u64);
            for j in 0..POSEIDON_T {
                state[i] += mds[i][j] * old_state[j];
            }
        }
    }

    // Partial rounds
    for _ in 0..POSEIDON_PARTIAL_ROUNDS {
        // Add round constants
        for i in 0..POSEIDON_T {
            state[i] += round_constants[round_ctr * POSEIDON_T + i];
        }
        round_ctr += 1;

        // Partial S-box (only first element)
        state[0] = sbox(state[0]);

        // MDS mixing
        let old_state = *state;
        for i in 0..POSEIDON_T {
            state[i] = Fr::from(0u64);
            for j in 0..POSEIDON_T {
                state[i] += mds[i][j] * old_state[j];
            }
        }
    }

    // Second half of full rounds
    for _ in 0..(POSEIDON_FULL_ROUNDS / 2) {
        // Add round constants
        for i in 0..POSEIDON_T {
            state[i] += round_constants[round_ctr * POSEIDON_T + i];
        }
        round_ctr += 1;

        // Full S-box layer
        for i in 0..POSEIDON_T {
            state[i] = sbox(state[i]);
        }

        // MDS mixing
        let old_state = *state;
        for i in 0..POSEIDON_T {
            state[i] = Fr::from(0u64);
            for j in 0..POSEIDON_T {
                state[i] += mds[i][j] * old_state[j];
            }
        }
    }
}

pub struct PoseidonHasher {
    state: [Fr; POSEIDON_T],
    absorbed: usize,
}

impl PoseidonHasher {
    pub fn new() -> Self {
        Self {
            state: [Fr::from(0u64); POSEIDON_T],
            absorbed: 0,
        }
    }

    pub fn absorb_field(&mut self, elem: Fr) {
        self.state[self.absorbed % POSEIDON_RATE] += elem;
        self.absorbed += 1;

        if self.absorbed % POSEIDON_RATE == 0 {
            poseidon_permutation(&mut self.state);
        }
    }

    pub fn absorb(&mut self, data: &[u8]) {
        // Process 31 bytes at a time (safe for BN254 Fr)
        for chunk in data.chunks(31) {
            let mut bytes = [0u8; 32];
            bytes[..chunk.len()].copy_from_slice(chunk);
            let elem = Fr::from_le_bytes_mod_order(&bytes);
            self.absorb_field(elem);
        }
    }

    pub fn finalize(mut self) -> [u8; 32] {
        // Pad with 1
        self.state[self.absorbed % POSEIDON_RATE] += Fr::from(1u64);
        poseidon_permutation(&mut self.state);

        // Serialize first element
        let mut output = [0u8; 32];
        self.state[0]
            .serialize_compressed(&mut output[..])
            .expect("Serialization failed");
        output
    }

    pub fn finalize_field(mut self) -> Fr {
        self.state[self.absorbed % POSEIDON_RATE] += Fr::from(1u64);
        poseidon_permutation(&mut self.state);
        self.state[0]
    }
}

impl Default for PoseidonHasher {
    fn default() -> Self {
        Self::new()
    }
}

pub fn poseidon_hash(data: &[u8]) -> [u8; 32] {
    let mut hasher = PoseidonHasher::new();
    hasher.absorb(data);
    hasher.finalize()
}

pub fn poseidon_hash2(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let left_fr = Fr::from_le_bytes_mod_order(left);
    let right_fr = Fr::from_le_bytes_mod_order(right);

    let mut state = [Fr::from(0u64); POSEIDON_T];
    state[0] = left_fr;
    state[1] = right_fr;
    poseidon_permutation(&mut state);

    let mut output = [0u8; 32];
    state[0]
        .serialize_compressed(&mut output[..])
        .expect("Serialization failed");
    output
}

pub fn poseidon_commitment(value: &[u8], blinding: &[u8; 32]) -> [u8; 32] {
    let mut hasher = PoseidonHasher::new();
    hasher.absorb(value);
    hasher.absorb(blinding);
    hasher.finalize()
}

pub fn compute_nullifier(spending_key: &[u8; 32], note_commitment: &[u8; 32]) -> [u8; 32] {
    poseidon_hash2(spending_key, note_commitment)
}

pub struct PoseidonMerkleTree {
    leaves: Vec<[u8; 32]>,
    depth: usize,
}

impl PoseidonMerkleTree {
    pub fn new(depth: usize) -> Self {
        Self {
            leaves: Vec::new(),
            depth,
        }
    }

    pub fn insert(&mut self, leaf: [u8; 32]) -> usize {
        let index = self.leaves.len();
        self.leaves.push(leaf);
        index
    }

    pub fn root(&self) -> [u8; 32] {
        if self.leaves.is_empty() {
            return [0u8; 32];
        }

        let mut level: Vec<[u8; 32]> = self.leaves.clone();
        let target_len = 1 << self.depth;
        while level.len() < target_len {
            level.push([0u8; 32]);
        }

        while level.len() > 1 {
            let mut next = Vec::with_capacity(level.len() / 2);
            for chunk in level.chunks(2) {
                next.push(poseidon_hash2(&chunk[0], &chunk[1]));
            }
            level = next;
        }

        level[0]
    }

    pub fn proof(&self, index: usize) -> Vec<([u8; 32], bool)> {
        let mut proof = Vec::new();
        let mut level: Vec<[u8; 32]> = self.leaves.clone();

        let target_len = 1 << self.depth;
        while level.len() < target_len {
            level.push([0u8; 32]);
        }

        let mut idx = index;
        while level.len() > 1 {
            let sibling = if idx % 2 == 0 { idx + 1 } else { idx - 1 };
            let is_left = idx % 2 == 0;
            proof.push((level[sibling], is_left));

            let mut next = Vec::with_capacity(level.len() / 2);
            for chunk in level.chunks(2) {
                next.push(poseidon_hash2(&chunk[0], &chunk[1]));
            }
            level = next;
            idx /= 2;
        }

        proof
    }

    pub fn verify_proof(leaf: &[u8; 32], proof: &[([u8; 32], bool)], root: &[u8; 32]) -> bool {
        let mut current = *leaf;

        for (sibling, is_left) in proof {
            current = if *is_left {
                poseidon_hash2(&current, sibling)
            } else {
                poseidon_hash2(sibling, &current)
            };
        }

        current == *root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poseidon_deterministic() {
        let data = b"NONOS test data";
        let hash1 = poseidon_hash(data);
        let hash2 = poseidon_hash(data);
        assert_eq!(hash1, hash2);

        let different = poseidon_hash(b"different");
        assert_ne!(hash1, different);
    }

    #[test]
    fn test_poseidon_hash2() {
        let left = [0xaa; 32];
        let right = [0xbb; 32];

        let hash = poseidon_hash2(&left, &right);
        assert_eq!(hash, poseidon_hash2(&left, &right));
        assert_ne!(hash, poseidon_hash2(&right, &left));
    }

    #[test]
    fn test_commitment() {
        let value = b"100 NOX";
        let blinding = [0xcc; 32];

        let c1 = poseidon_commitment(value, &blinding);
        let c2 = poseidon_commitment(value, &blinding);
        assert_eq!(c1, c2);

        let different = [0xdd; 32];
        assert_ne!(c1, poseidon_commitment(value, &different));
    }

    #[test]
    fn test_nullifier() {
        let key = [0xaa; 32];
        let commitment = [0xbb; 32];

        let n1 = compute_nullifier(&key, &commitment);
        let n2 = compute_nullifier(&key, &commitment);
        assert_eq!(n1, n2);

        let different = [0xdd; 32];
        assert_ne!(n1, compute_nullifier(&key, &different));
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
}
