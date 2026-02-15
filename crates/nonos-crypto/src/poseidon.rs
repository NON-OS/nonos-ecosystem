const POSEIDON_ROUNDS_F: usize = 8;
const POSEIDON_ROUNDS_P: usize = 57;
const POSEIDON_T: usize = 3;
const POSEIDON_RATE: usize = 2;

pub struct PoseidonHasher {
    state: [u64; POSEIDON_T],
    absorbed: usize,
}

impl PoseidonHasher {
    pub fn new() -> Self {
        Self {
            state: [0u64; POSEIDON_T],
            absorbed: 0,
        }
    }

    pub fn absorb(&mut self, data: &[u8]) {
        for chunk in data.chunks(8) {
            let mut bytes = [0u8; 8];
            let len = chunk.len().min(8);
            bytes[..len].copy_from_slice(&chunk[..len]);

            let elem = u64::from_le_bytes(bytes);
            self.state[self.absorbed % POSEIDON_RATE] ^= elem;
            self.absorbed += 1;

            if self.absorbed % POSEIDON_RATE == 0 {
                self.permute();
            }
        }
    }

    fn permute(&mut self) {
        for round in 0..(POSEIDON_ROUNDS_F / 2) {
            for (i, s) in self.state.iter_mut().enumerate() {
                let round_const = (round as u64 + 1)
                    .wrapping_mul(i as u64 + 1)
                    .wrapping_mul(0x9e3779b97f4a7c15);
                *s = s.wrapping_add(round_const);
            }

            for s in self.state.iter_mut() {
                let x = *s;
                *s = x.wrapping_mul(x).wrapping_mul(x).wrapping_mul(x).wrapping_mul(x);
            }

            let [a, b, c] = self.state;
            self.state[0] = a.wrapping_add(b).wrapping_add(c);
            self.state[1] = a.wrapping_add(b.wrapping_mul(2)).wrapping_add(c.wrapping_mul(4));
            self.state[2] = a.wrapping_add(b.wrapping_mul(4)).wrapping_add(c.wrapping_mul(16));
        }

        for round in 0..POSEIDON_ROUNDS_P {
            let round_const = (round as u64 + 1).wrapping_mul(0x517cc1b727220a95);
            self.state[0] = self.state[0].wrapping_add(round_const);

            let x = self.state[0];
            self.state[0] = x.wrapping_mul(x).wrapping_mul(x).wrapping_mul(x).wrapping_mul(x);

            let [a, b, c] = self.state;
            self.state[0] = a.wrapping_add(b).wrapping_add(c);
            self.state[1] = a.wrapping_add(b.wrapping_mul(2)).wrapping_add(c.wrapping_mul(4));
            self.state[2] = a.wrapping_add(b.wrapping_mul(4)).wrapping_add(c.wrapping_mul(16));
        }

        for round in (POSEIDON_ROUNDS_F / 2)..POSEIDON_ROUNDS_F {
            for (i, s) in self.state.iter_mut().enumerate() {
                let round_const = (round as u64 + 1)
                    .wrapping_mul(i as u64 + 1)
                    .wrapping_mul(0x9e3779b97f4a7c15);
                *s = s.wrapping_add(round_const);
            }

            for s in self.state.iter_mut() {
                let x = *s;
                *s = x.wrapping_mul(x).wrapping_mul(x).wrapping_mul(x).wrapping_mul(x);
            }

            let [a, b, c] = self.state;
            self.state[0] = a.wrapping_add(b).wrapping_add(c);
            self.state[1] = a.wrapping_add(b.wrapping_mul(2)).wrapping_add(c.wrapping_mul(4));
            self.state[2] = a.wrapping_add(b.wrapping_mul(4)).wrapping_add(c.wrapping_mul(16));
        }
    }

    pub fn finalize(mut self) -> [u8; 32] {
        self.state[self.absorbed % POSEIDON_RATE] ^= 1u64;
        self.permute();

        let mut output = [0u8; 32];
        output[..8].copy_from_slice(&self.state[0].to_le_bytes());
        output[8..16].copy_from_slice(&self.state[1].to_le_bytes());
        output[16..24].copy_from_slice(&self.state[2].to_le_bytes());

        self.permute();
        output[24..32].copy_from_slice(&self.state[0].to_le_bytes());

        output
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
    let mut hasher = PoseidonHasher::new();
    hasher.absorb(left);
    hasher.absorb(right);
    hasher.finalize()
}

pub fn poseidon_commitment(value: &[u8], blinding: &[u8; 32]) -> [u8; 32] {
    let mut hasher = PoseidonHasher::new();
    hasher.absorb(value);
    hasher.absorb(blinding);
    hasher.finalize()
}

pub fn compute_nullifier(spending_key: &[u8; 32], note_commitment: &[u8; 32]) -> [u8; 32] {
    let mut hasher = PoseidonHasher::new();
    hasher.absorb(spending_key);
    hasher.absorb(note_commitment);
    hasher.finalize()
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
            let mut next_level = Vec::with_capacity(level.len() / 2);
            for chunk in level.chunks(2) {
                next_level.push(poseidon_hash2(&chunk[0], &chunk[1]));
            }
            level = next_level;
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
            let sibling_idx = if idx % 2 == 0 { idx + 1 } else { idx - 1 };
            let is_left = idx % 2 == 0;
            proof.push((level[sibling_idx], is_left));

            let mut next_level = Vec::with_capacity(level.len() / 2);
            for chunk in level.chunks(2) {
                next_level.push(poseidon_hash2(&chunk[0], &chunk[1]));
            }
            level = next_level;
            idx /= 2;
        }

        proof
    }

    pub fn verify_proof(leaf: &[u8; 32], proof: &[([u8; 32], bool)], root: &[u8; 32]) -> bool {
        let mut current = *leaf;

        for (sibling, is_left) in proof {
            if *is_left {
                current = poseidon_hash2(&current, sibling);
            } else {
                current = poseidon_hash2(sibling, &current);
            }
        }

        current == *root
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
    fn test_commitment() {
        let value = b"100 NOX";
        let blinding = [0xcc; 32];

        let commitment = poseidon_commitment(value, &blinding);

        assert_eq!(commitment, poseidon_commitment(value, &blinding));

        let different_blinding = [0xdd; 32];
        assert_ne!(commitment, poseidon_commitment(value, &different_blinding));
    }

    #[test]
    fn test_nullifier() {
        let spending_key = [0xaa; 32];
        let note_commitment = [0xbb; 32];

        let nullifier = compute_nullifier(&spending_key, &note_commitment);

        assert_eq!(nullifier, compute_nullifier(&spending_key, &note_commitment));

        let different_commitment = [0xdd; 32];
        assert_ne!(nullifier, compute_nullifier(&spending_key, &different_commitment));
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
