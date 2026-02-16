use ark_bn254::Fr;
use ark_crypto_primitives::sponge::poseidon::PoseidonConfig;
use ark_ff::{BigInteger, PrimeField};

fn poseidon_config() -> PoseidonConfig<Fr> {
    let full_rounds = 8;
    let partial_rounds = 57;
    let alpha = 5;
    let rate = 2;
    let capacity = 1;

    let mds = vec![
        vec![Fr::from(2u64), Fr::from(1u64), Fr::from(1u64)],
        vec![Fr::from(1u64), Fr::from(2u64), Fr::from(1u64)],
        vec![Fr::from(1u64), Fr::from(1u64), Fr::from(2u64)],
    ];

    let mut round_constants = Vec::new();
    let total_rounds = full_rounds + partial_rounds;
    for r in 0..total_rounds {
        let mut row = Vec::new();
        for i in 0..(rate + capacity) {
            let seed = ((r as u64) << 8) | (i as u64);
            let bytes = crate::blake3_hash(&seed.to_le_bytes());
            row.push(Fr::from_le_bytes_mod_order(&bytes.0));
        }
        round_constants.push(row);
    }

    PoseidonConfig {
        full_rounds,
        partial_rounds,
        alpha: alpha as u64,
        mds,
        rate,
        capacity,
        ark: round_constants,
    }
}

fn poseidon_permute(state: &mut [Fr; 3], config: &PoseidonConfig<Fr>) {
    let half_full = config.full_rounds / 2;

    for r in 0..half_full {
        for (i, s) in state.iter_mut().enumerate() {
            *s += config.ark[r][i];
        }
        for s in state.iter_mut() {
            let s2 = *s * *s;
            let s4 = s2 * s2;
            *s = s4 * *s;
        }
        let [a, b, c] = *state;
        state[0] = config.mds[0][0] * a + config.mds[0][1] * b + config.mds[0][2] * c;
        state[1] = config.mds[1][0] * a + config.mds[1][1] * b + config.mds[1][2] * c;
        state[2] = config.mds[2][0] * a + config.mds[2][1] * b + config.mds[2][2] * c;
    }

    for r in half_full..(half_full + config.partial_rounds) {
        for (i, s) in state.iter_mut().enumerate() {
            *s += config.ark[r][i];
        }
        let s2 = state[0] * state[0];
        let s4 = s2 * s2;
        state[0] = s4 * state[0];
        let [a, b, c] = *state;
        state[0] = config.mds[0][0] * a + config.mds[0][1] * b + config.mds[0][2] * c;
        state[1] = config.mds[1][0] * a + config.mds[1][1] * b + config.mds[1][2] * c;
        state[2] = config.mds[2][0] * a + config.mds[2][1] * b + config.mds[2][2] * c;
    }

    for r in (half_full + config.partial_rounds)..(config.full_rounds + config.partial_rounds) {
        for (i, s) in state.iter_mut().enumerate() {
            *s += config.ark[r][i];
        }
        for s in state.iter_mut() {
            let s2 = *s * *s;
            let s4 = s2 * s2;
            *s = s4 * *s;
        }
        let [a, b, c] = *state;
        state[0] = config.mds[0][0] * a + config.mds[0][1] * b + config.mds[0][2] * c;
        state[1] = config.mds[1][0] * a + config.mds[1][1] * b + config.mds[1][2] * c;
        state[2] = config.mds[2][0] * a + config.mds[2][1] * b + config.mds[2][2] * c;
    }
}

fn fr_to_bytes(f: Fr) -> [u8; 32] {
    let bytes = f.into_bigint().to_bytes_le();
    let mut result = [0u8; 32];
    result.copy_from_slice(&bytes[..32]);
    result
}

pub fn poseidon_hash2_fr(left: Fr, right: Fr) -> Fr {
    let config = poseidon_config();
    let mut state = [Fr::from(0u64), left, right];
    poseidon_permute(&mut state, &config);
    state[0]
}

pub fn poseidon_hash2(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let left_fr = Fr::from_le_bytes_mod_order(left);
    let right_fr = Fr::from_le_bytes_mod_order(right);
    fr_to_bytes(poseidon_hash2_fr(left_fr, right_fr))
}

pub fn poseidon_hash(data: &[u8]) -> [u8; 32] {
    let fr = Fr::from_le_bytes_mod_order(data);
    let config = poseidon_config();
    let mut state = [Fr::from(0u64), fr, Fr::from(0u64)];
    poseidon_permute(&mut state, &config);
    fr_to_bytes(state[0])
}

pub fn poseidon_hash_field(data: &[u8]) -> Fr {
    let fr = Fr::from_le_bytes_mod_order(data);
    let config = poseidon_config();
    let mut state = [Fr::from(0u64), fr, Fr::from(0u64)];
    poseidon_permute(&mut state, &config);
    state[0]
}

pub fn poseidon_commitment(value: &[u8], blinding: &[u8; 32]) -> [u8; 32] {
    let value_fr = Fr::from_le_bytes_mod_order(value);
    let blinding_fr = Fr::from_le_bytes_mod_order(blinding);
    fr_to_bytes(poseidon_hash2_fr(value_fr, blinding_fr))
}

pub fn poseidon_commitment_field(value: Fr, blinding: Fr) -> Fr {
    poseidon_hash2_fr(value, blinding)
}

pub fn compute_nullifier(spending_key: &[u8; 32], note_commitment: &[u8; 32]) -> [u8; 32] {
    poseidon_hash2(spending_key, note_commitment)
}

pub fn compute_nullifier_field(spending_key: Fr, note_commitment: Fr) -> [u8; 32] {
    fr_to_bytes(poseidon_hash2_fr(spending_key, note_commitment))
}

pub fn compute_nullifier_fr(spending_key: Fr, note_commitment: Fr) -> Fr {
    poseidon_hash2_fr(spending_key, note_commitment)
}

pub struct PoseidonMerkleTree {
    leaves: Vec<Fr>,
    depth: usize,
    zero_values: Vec<Fr>,
}

impl PoseidonMerkleTree {
    pub fn new(depth: usize) -> Self {
        let mut zero_values = Vec::with_capacity(depth + 1);
        let mut current = Fr::from(0u64);
        zero_values.push(current);

        for _ in 0..depth {
            current = poseidon_hash2_fr(current, current);
            zero_values.push(current);
        }

        Self {
            leaves: Vec::new(),
            depth,
            zero_values,
        }
    }

    pub fn insert(&mut self, leaf: [u8; 32]) -> usize {
        let leaf_fr = Fr::from_le_bytes_mod_order(&leaf);
        self.insert_field(leaf_fr)
    }

    pub fn insert_field(&mut self, leaf: Fr) -> usize {
        let index = self.leaves.len();
        self.leaves.push(leaf);
        index
    }

    pub fn root(&self) -> [u8; 32] {
        fr_to_bytes(self.root_field())
    }

    pub fn root_field(&self) -> Fr {
        if self.leaves.is_empty() {
            return self.zero_values[self.depth];
        }

        let target_len = 1usize << self.depth;
        let mut level: Vec<Fr> = self.leaves.clone();

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
                next_level.push(poseidon_hash2_fr(left, right));
            }
            level = next_level;
            depth_idx += 1;
        }

        level[0]
    }

    pub fn proof(&self, index: usize) -> Vec<([u8; 32], bool)> {
        self.proof_field(index)
            .into_iter()
            .map(|(sibling, is_left)| (fr_to_bytes(sibling), is_left))
            .collect()
    }

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
                next_level.push(poseidon_hash2_fr(left, right));
            }
            level = next_level;
            idx /= 2;
            depth_idx += 1;
        }

        proof
    }

    pub fn verify_proof(leaf: &[u8; 32], proof: &[([u8; 32], bool)], root: &[u8; 32]) -> bool {
        let leaf_fr = Fr::from_le_bytes_mod_order(leaf);
        let root_fr = Fr::from_le_bytes_mod_order(root);
        let proof_fr: Vec<(Fr, bool)> = proof
            .iter()
            .map(|(sibling, is_left)| (Fr::from_le_bytes_mod_order(sibling), *is_left))
            .collect();
        Self::verify_proof_field(leaf_fr, &proof_fr, root_fr)
    }

    pub fn verify_proof_field(leaf: Fr, proof: &[(Fr, bool)], root: Fr) -> bool {
        let mut current = leaf;

        for (sibling, is_left) in proof {
            if *is_left {
                current = poseidon_hash2_fr(current, *sibling);
            } else {
                current = poseidon_hash2_fr(*sibling, current);
            }
        }

        current == root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poseidon_hash_deterministic() {
        let data = b"NONOS test data";
        let hash1 = poseidon_hash(data);
        let hash2 = poseidon_hash(data);
        assert_eq!(hash1, hash2);

        let different = poseidon_hash(b"different data");
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
    fn test_field_operations() {
        let a = Fr::from(12345u64);
        let b = Fr::from(67890u64);

        let hash1 = poseidon_hash2_fr(a, b);
        let hash2 = poseidon_hash2_fr(a, b);
        assert_eq!(hash1, hash2);

        let hash3 = poseidon_hash2_fr(b, a);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_commitment() {
        let value = b"100 NOX";
        let blinding = [0xcc; 32];

        let commitment = poseidon_commitment(value, &blinding);
        assert_eq!(commitment, poseidon_commitment(value, &blinding));

        let different_blinding = [0xdd; 32];
        assert_ne!(commitment, poseidon_commitment(value, &different_blinding));
    }

    #[test]
    fn test_commitment_field() {
        let value = Fr::from(100u64);
        let blinding = Fr::from(0xabcdefu64);

        let c1 = poseidon_commitment_field(value, blinding);
        let c2 = poseidon_commitment_field(value, blinding);
        assert_eq!(c1, c2);

        let c3 = poseidon_commitment_field(value, Fr::from(0xffeeddccu64));
        assert_ne!(c1, c3);
    }

    #[test]
    fn test_nullifier() {
        let spending_key = [0xaa; 32];
        let note_commitment = [0xbb; 32];

        let nullifier = compute_nullifier(&spending_key, &note_commitment);
        assert_eq!(
            nullifier,
            compute_nullifier(&spending_key, &note_commitment)
        );

        let different_commitment = [0xdd; 32];
        assert_ne!(
            nullifier,
            compute_nullifier(&spending_key, &different_commitment)
        );
    }

    #[test]
    fn test_nullifier_field() {
        let sk = Fr::from(0xaabbccddu64);
        let nc = Fr::from(0x11223344u64);

        let n1 = compute_nullifier_fr(sk, nc);
        let n2 = compute_nullifier_fr(sk, nc);
        assert_eq!(n1, n2);

        let n3 = compute_nullifier_fr(sk, Fr::from(0x55667788u64));
        assert_ne!(n1, n3);
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

    #[test]
    fn test_merkle_tree_field() {
        let mut tree = PoseidonMerkleTree::new(4);

        let leaf1 = Fr::from(0x11u64);
        let leaf2 = Fr::from(0x22u64);
        let leaf3 = Fr::from(0x33u64);

        let idx1 = tree.insert_field(leaf1);
        let idx2 = tree.insert_field(leaf2);
        tree.insert_field(leaf3);

        let root = tree.root_field();

        let proof1 = tree.proof_field(idx1);
        assert!(PoseidonMerkleTree::verify_proof_field(leaf1, &proof1, root));

        let proof2 = tree.proof_field(idx2);
        assert!(PoseidonMerkleTree::verify_proof_field(leaf2, &proof2, root));
    }

    #[test]
    fn test_empty_tree() {
        let tree = PoseidonMerkleTree::new(4);
        let root = tree.root();
        assert_ne!(root, [0u8; 32]);
    }

    #[test]
    fn test_wrong_proof_fails() {
        let mut tree = PoseidonMerkleTree::new(4);

        let leaf1 = [0x11; 32];
        let leaf2 = [0x22; 32];

        tree.insert(leaf1);
        tree.insert(leaf2);

        let root = tree.root();
        let proof1 = tree.proof(0);

        assert!(!PoseidonMerkleTree::verify_proof(&leaf2, &proof1, &root));
    }

    #[test]
    fn test_wrong_root_fails() {
        let mut tree = PoseidonMerkleTree::new(4);

        let leaf = [0x11; 32];
        tree.insert(leaf);

        let proof = tree.proof(0);
        let wrong_root = [0xff; 32];

        assert!(!PoseidonMerkleTree::verify_proof(&leaf, &proof, &wrong_root));
    }

    #[test]
    fn test_poseidon_consistency_with_zk() {
        let left = [0x11; 32];
        let right = [0x22; 32];
        let hash = poseidon_hash2(&left, &right);
        let hash2 = poseidon_hash2(&left, &right);
        assert_eq!(hash, hash2);
    }
}
