use ark_bn254::{Bn254, Fr};
use ark_crypto_primitives::sponge::poseidon::PoseidonConfig;
use ark_ff::{BigInteger, PrimeField};
use ark_groth16::{Groth16, PreparedVerifyingKey, Proof, ProvingKey};
use ark_r1cs_std::{
    alloc::AllocVar,
    boolean::Boolean,
    eq::EqGadget,
    fields::{fp::FpVar, FieldVar},
    select::CondSelectGadget,
};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_snark::SNARK;
use ark_std::rand::thread_rng;
use std::sync::OnceLock;

const MERKLE_DEPTH: usize = 20;

static IDENTITY_PK: OnceLock<ProvingKey<Bn254>> = OnceLock::new();
static IDENTITY_VK: OnceLock<PreparedVerifyingKey<Bn254>> = OnceLock::new();

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

fn poseidon_permute_native(state: &mut [Fr; 3], config: &PoseidonConfig<Fr>) {
    let half_full = config.full_rounds / 2;

    // First half of full rounds
    for r in 0..half_full {
        for (i, s) in state.iter_mut().enumerate() {
            *s += config.ark[r][i];
        }
        for s in state.iter_mut() {
            let s2 = *s * *s;
            let s4 = s2 * s2;
            *s = s4 * *s;
        }
        let old = *state;
        for (i, row) in config.mds.iter().enumerate() {
            state[i] = row[0] * old[0] + row[1] * old[1] + row[2] * old[2];
        }
    }

    // Partial rounds
    for r in half_full..(half_full + config.partial_rounds) {
        for (i, s) in state.iter_mut().enumerate() {
            *s += config.ark[r][i];
        }
        let s = &mut state[0];
        let s2 = *s * *s;
        let s4 = s2 * s2;
        *s = s4 * *s;
        let old = *state;
        for (i, row) in config.mds.iter().enumerate() {
            state[i] = row[0] * old[0] + row[1] * old[1] + row[2] * old[2];
        }
    }

    // Second half of full rounds
    for r in (half_full + config.partial_rounds)..(config.full_rounds + config.partial_rounds) {
        for (i, s) in state.iter_mut().enumerate() {
            *s += config.ark[r][i];
        }
        for s in state.iter_mut() {
            let s2 = *s * *s;
            let s4 = s2 * s2;
            *s = s4 * *s;
        }
        let old = *state;
        for (i, row) in config.mds.iter().enumerate() {
            state[i] = row[0] * old[0] + row[1] * old[1] + row[2] * old[2];
        }
    }
}

fn poseidon_hash_native(left: Fr, right: Fr) -> Fr {
    let config = poseidon_config();
    let mut state = [Fr::from(0u64), left, right];
    poseidon_permute_native(&mut state, &config);
    state[1]
}

fn poseidon_hash_three(a: Fr, b: Fr, c: Fr) -> Fr {
    // Hash(a, b) then Hash(result, c)
    let h1 = poseidon_hash_native(a, b);
    poseidon_hash_native(h1, c)
}

fn poseidon_hash_gadget(
    _cs: ConstraintSystemRef<Fr>,
    left: &FpVar<Fr>,
    right: &FpVar<Fr>,
) -> Result<FpVar<Fr>, SynthesisError> {
    let config = poseidon_config();

    let mut state = vec![
        FpVar::constant(Fr::from(0u64)),
        left.clone(),
        right.clone(),
    ];

    let half_full = config.full_rounds / 2;

    for r in 0..half_full {
        for (i, s) in state.iter_mut().enumerate() {
            let c = FpVar::constant(config.ark[r][i]);
            *s = s.clone() + c;
        }
        for s in state.iter_mut() {
            let s2 = s.clone() * s.clone();
            let s4 = s2.clone() * &s2;
            *s = s4 * s.clone();
        }
        let new_state = apply_mds_gadget(&state, &config.mds)?;
        state = new_state;
    }

    for r in half_full..(half_full + config.partial_rounds) {
        for (i, s) in state.iter_mut().enumerate() {
            let c = FpVar::constant(config.ark[r][i]);
            *s = s.clone() + c;
        }
        let s = &mut state[0];
        let s2 = s.clone() * s.clone();
        let s4 = s2.clone() * &s2;
        *s = s4 * s.clone();
        let new_state = apply_mds_gadget(&state, &config.mds)?;
        state = new_state;
    }

    for r in (half_full + config.partial_rounds)..(config.full_rounds + config.partial_rounds) {
        for (i, s) in state.iter_mut().enumerate() {
            let c = FpVar::constant(config.ark[r][i]);
            *s = s.clone() + c;
        }
        for s in state.iter_mut() {
            let s2 = s.clone() * s.clone();
            let s4 = s2.clone() * &s2;
            *s = s4 * s.clone();
        }
        let new_state = apply_mds_gadget(&state, &config.mds)?;
        state = new_state;
    }

    Ok(state[1].clone())
}

fn apply_mds_gadget(
    state: &[FpVar<Fr>],
    mds: &[Vec<Fr>],
) -> Result<Vec<FpVar<Fr>>, SynthesisError> {
    let mut new_state = Vec::with_capacity(state.len());
    for row in mds {
        let mut acc = FpVar::constant(Fr::from(0u64));
        for (j, s) in state.iter().enumerate() {
            let coeff = FpVar::constant(row[j]);
            acc = acc + (coeff * s);
        }
        new_state.push(acc);
    }
    Ok(new_state)
}

fn poseidon_hash_three_gadget(
    cs: ConstraintSystemRef<Fr>,
    a: &FpVar<Fr>,
    b: &FpVar<Fr>,
    c: &FpVar<Fr>,
) -> Result<FpVar<Fr>, SynthesisError> {
    let h1 = poseidon_hash_gadget(cs.clone(), a, b)?;
    poseidon_hash_gadget(cs, &h1, c)
}

#[derive(Clone)]
pub struct IdentityCircuit {
    pub secret: Option<Fr>,
    pub blinding: Option<Fr>,
    pub leaf_index: Option<u64>,
    pub merkle_path: Option<Vec<Fr>>,
    pub merkle_root: Option<Fr>,
    pub nullifier: Option<Fr>,
    pub scope: Option<Fr>,
}

impl ConstraintSynthesizer<Fr> for IdentityCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        let secret_var = FpVar::new_witness(cs.clone(), || {
            self.secret.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let blinding_var = FpVar::new_witness(cs.clone(), || {
            self.blinding.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let _leaf_index_var = FpVar::new_witness(cs.clone(), || {
            self.leaf_index
                .map(Fr::from)
                .ok_or(SynthesisError::AssignmentMissing)
        })?;

        let merkle_path = self.merkle_path.unwrap_or_else(|| vec![Fr::from(0u64); MERKLE_DEPTH]);
        let mut path_vars = Vec::with_capacity(MERKLE_DEPTH);
        for sibling in merkle_path.iter() {
            let sibling_var = FpVar::new_witness(cs.clone(), || Ok(*sibling))?;
            path_vars.push(sibling_var);
        }

        let merkle_root_var = FpVar::new_input(cs.clone(), || {
            self.merkle_root.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let nullifier_var = FpVar::new_input(cs.clone(), || {
            self.nullifier.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let scope_var = FpVar::new_input(cs.clone(), || {
            self.scope.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let commitment_var = poseidon_hash_gadget(cs.clone(), &secret_var, &blinding_var)?;

        let mut current = commitment_var.clone();
        let mut index = self.leaf_index.unwrap_or(0);

        for sibling_var in path_vars.iter() {
            let is_right = (index & 1) == 1;
            let is_right_var = Boolean::constant(is_right);

            let left = FpVar::conditionally_select(&is_right_var, sibling_var, &current)?;
            let right = FpVar::conditionally_select(&is_right_var, &current, sibling_var)?;

            current = poseidon_hash_gadget(cs.clone(), &left, &right)?;
            index >>= 1;
        }

        current.enforce_equal(&merkle_root_var)?;

        let computed_nullifier =
            poseidon_hash_three_gadget(cs.clone(), &secret_var, &commitment_var, &scope_var)?;
        computed_nullifier.enforce_equal(&nullifier_var)?;

        Ok(())
    }
}

fn initialize_identity_keys() {
    if IDENTITY_PK.get().is_some() {
        return;
    }

    let mut rng = thread_rng();

    let dummy_circuit = IdentityCircuit {
        secret: Some(Fr::from(0u64)),
        blinding: Some(Fr::from(0u64)),
        leaf_index: Some(0),
        merkle_path: Some(vec![Fr::from(0u64); MERKLE_DEPTH]),
        merkle_root: Some(Fr::from(0u64)),
        nullifier: Some(Fr::from(0u64)),
        scope: Some(Fr::from(0u64)),
    };

    let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(dummy_circuit, &mut rng)
        .expect("Failed to generate ZK keys");

    let pvk = Groth16::<Bn254>::process_vk(&vk).expect("Failed to process verifying key");

    let _ = IDENTITY_PK.set(pk);
    let _ = IDENTITY_VK.set(pvk);
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ZkIdentityProof {
    #[serde(with = "proof_serde")]
    pub proof: Proof<Bn254>,
    pub merkle_root: [u8; 32],
    pub nullifier: [u8; 32],
    pub scope: [u8; 32],
}

mod proof_serde {
    use super::*;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(proof: &Proof<Bn254>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut bytes = Vec::new();
        proof
            .serialize_compressed(&mut bytes)
            .map_err(serde::ser::Error::custom)?;
        bytes.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Proof<Bn254>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes = Vec::<u8>::deserialize(deserializer)?;
        Proof::deserialize_compressed(&bytes[..]).map_err(serde::de::Error::custom)
    }
}

impl ZkIdentityProof {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.proof
            .serialize_compressed(&mut bytes)
            .expect("Serialization failed");
        bytes.extend_from_slice(&self.merkle_root);
        bytes.extend_from_slice(&self.nullifier);
        bytes.extend_from_slice(&self.scope);
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() < 96 {
            return Err("Invalid proof bytes");
        }

        let proof_len = bytes.len() - 96;
        let proof = Proof::<Bn254>::deserialize_compressed(&bytes[..proof_len])
            .map_err(|_| "Failed to deserialize proof")?;

        let mut merkle_root = [0u8; 32];
        let mut nullifier = [0u8; 32];
        let mut scope = [0u8; 32];

        merkle_root.copy_from_slice(&bytes[proof_len..proof_len + 32]);
        nullifier.copy_from_slice(&bytes[proof_len + 32..proof_len + 64]);
        scope.copy_from_slice(&bytes[proof_len + 64..proof_len + 96]);

        Ok(Self {
            proof,
            merkle_root,
            nullifier,
            scope,
        })
    }

    pub fn to_base64(&self) -> String {
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, self.to_bytes())
    }

    pub fn from_base64(s: &str) -> Result<Self, &'static str> {
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, s)
            .map_err(|_| "Invalid base64")?;
        Self::from_bytes(&bytes)
    }
}

pub struct IdentityProofInput {
    pub secret: [u8; 32],
    pub blinding: [u8; 32],
    pub leaf_index: u64,
    pub merkle_path: Vec<[u8; 32]>,
    pub merkle_root: [u8; 32],
    pub scope: [u8; 32],
}

fn fr_to_bytes(f: Fr) -> [u8; 32] {
    let bytes = f.into_bigint().to_bytes_le();
    let mut result = [0u8; 32];
    result.copy_from_slice(&bytes[..32]);
    result
}

pub fn generate_identity_proof(input: &IdentityProofInput) -> Result<ZkIdentityProof, &'static str> {
    initialize_identity_keys();

    let pk = IDENTITY_PK.get().ok_or("Proving key not initialized")?;

    let secret_fr = Fr::from_le_bytes_mod_order(&input.secret);
    let blinding_fr = Fr::from_le_bytes_mod_order(&input.blinding);
    let scope_fr = Fr::from_le_bytes_mod_order(&input.scope);
    let merkle_root_fr = Fr::from_le_bytes_mod_order(&input.merkle_root);

    let commitment_fr = poseidon_hash_native(secret_fr, blinding_fr);
    let nullifier_fr = poseidon_hash_three(secret_fr, commitment_fr, scope_fr);

    let merkle_path_fr: Vec<Fr> = input
        .merkle_path
        .iter()
        .map(|b| Fr::from_le_bytes_mod_order(b))
        .collect();

    let circuit = IdentityCircuit {
        secret: Some(secret_fr),
        blinding: Some(blinding_fr),
        leaf_index: Some(input.leaf_index),
        merkle_path: Some(merkle_path_fr),
        merkle_root: Some(merkle_root_fr),
        nullifier: Some(nullifier_fr),
        scope: Some(scope_fr),
    };

    let mut rng = thread_rng();
    let proof =
        Groth16::<Bn254>::prove(pk, circuit, &mut rng).map_err(|_| "Proof generation failed")?;

    Ok(ZkIdentityProof {
        proof,
        merkle_root: input.merkle_root,
        nullifier: fr_to_bytes(nullifier_fr),
        scope: input.scope,
    })
}

pub fn verify_identity_proof(proof: &ZkIdentityProof) -> Result<bool, &'static str> {
    initialize_identity_keys();

    let pvk = IDENTITY_VK.get().ok_or("Verifying key not initialized")?;

    let merkle_root_fr = Fr::from_le_bytes_mod_order(&proof.merkle_root);
    let nullifier_fr = Fr::from_le_bytes_mod_order(&proof.nullifier);
    let scope_fr = Fr::from_le_bytes_mod_order(&proof.scope);

    let public_inputs = vec![merkle_root_fr, nullifier_fr, scope_fr];

    let valid = Groth16::<Bn254>::verify_with_processed_vk(pvk, &public_inputs, &proof.proof)
        .map_err(|_| "Verification failed")?;

    Ok(valid)
}

pub fn compute_identity_commitment(secret: &[u8; 32], blinding: &[u8; 32]) -> [u8; 32] {
    let secret_fr = Fr::from_le_bytes_mod_order(secret);
    let blinding_fr = Fr::from_le_bytes_mod_order(blinding);
    let commitment_fr = poseidon_hash_native(secret_fr, blinding_fr);
    fr_to_bytes(commitment_fr)
}

pub fn compute_identity_nullifier(
    secret: &[u8; 32],
    blinding: &[u8; 32],
    scope: &[u8; 32],
) -> [u8; 32] {
    let secret_fr = Fr::from_le_bytes_mod_order(secret);
    let blinding_fr = Fr::from_le_bytes_mod_order(blinding);
    let scope_fr = Fr::from_le_bytes_mod_order(scope);
    let commitment_fr = poseidon_hash_native(secret_fr, blinding_fr);
    let nullifier_fr = poseidon_hash_three(secret_fr, commitment_fr, scope_fr);
    fr_to_bytes(nullifier_fr)
}

pub fn compute_merkle_root(leaf: &[u8; 32], index: u64, path: &[[u8; 32]]) -> [u8; 32] {
    let mut current = Fr::from_le_bytes_mod_order(leaf);
    let mut idx = index;

    for sibling in path {
        let sibling_fr = Fr::from_le_bytes_mod_order(sibling);
        let is_right = (idx & 1) == 1;

        current = if is_right {
            poseidon_hash_native(sibling_fr, current)
        } else {
            poseidon_hash_native(current, sibling_fr)
        };

        idx >>= 1;
    }

    fr_to_bytes(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngCore;

    fn setup_test_tree() -> (Vec<[u8; 32]>, [u8; 32], u64, Vec<[u8; 32]>) {
        let mut rng = thread_rng();

        let mut leaves = Vec::new();
        for _ in 0..4 {
            let mut leaf = [0u8; 32];
            rng.fill_bytes(&mut leaf);
            leaves.push(Fr::from_le_bytes_mod_order(&leaf));
        }

        let mut level = leaves.clone();
        let mut all_levels = vec![level.clone()];

        while level.len() > 1 {
            let mut next_level = Vec::new();
            for chunk in level.chunks(2) {
                let left = chunk[0];
                let right = if chunk.len() > 1 {
                    chunk[1]
                } else {
                    Fr::from(0u64)
                };
                next_level.push(poseidon_hash_native(left, right));
            }
            level = next_level;
            all_levels.push(level.clone());
        }

        let mut path = Vec::new();
        let leaf_index = 0u64;
        let mut idx = leaf_index as usize;

        for l in 0..(all_levels.len() - 1) {
            let sibling_idx = if idx % 2 == 0 { idx + 1 } else { idx - 1 };
            let sibling = if sibling_idx < all_levels[l].len() {
                all_levels[l][sibling_idx]
            } else {
                Fr::from(0u64)
            };
            path.push(fr_to_bytes(sibling));
            idx /= 2;
        }

        while path.len() < MERKLE_DEPTH {
            path.push([0u8; 32]);
        }

        let root = fr_to_bytes(all_levels.last().unwrap()[0]);
        let leaf_bytes = fr_to_bytes(leaves[0]);

        (vec![leaf_bytes], root, leaf_index, path)
    }

    #[test]
    fn test_identity_proof_generation_and_verification() {
        let mut rng = thread_rng();

        let mut secret = [0u8; 32];
        let mut blinding = [0u8; 32];
        let mut scope = [0u8; 32];
        rng.fill_bytes(&mut secret);
        rng.fill_bytes(&mut blinding);
        rng.fill_bytes(&mut scope);

        let commitment = compute_identity_commitment(&secret, &blinding);

        let mut path = vec![[0u8; 32]; MERKLE_DEPTH];
        let leaf_index = 0u64;

        let mut current = Fr::from_le_bytes_mod_order(&commitment);
        for i in 0..MERKLE_DEPTH {
            let sibling = Fr::from(0u64);
            path[i] = fr_to_bytes(sibling);
            current = poseidon_hash_native(current, sibling);
        }
        let merkle_root = fr_to_bytes(current);

        let input = IdentityProofInput {
            secret,
            blinding,
            leaf_index,
            merkle_path: path,
            merkle_root,
            scope,
        };

        let proof = generate_identity_proof(&input).unwrap();

        let valid = verify_identity_proof(&proof).unwrap();
        assert!(valid, "Proof should be valid");
    }

    #[test]
    fn test_nullifier_uniqueness() {
        let mut rng = thread_rng();

        let mut secret = [0u8; 32];
        let mut blinding = [0u8; 32];
        let mut scope1 = [0u8; 32];
        let mut scope2 = [0u8; 32];

        rng.fill_bytes(&mut secret);
        rng.fill_bytes(&mut blinding);
        rng.fill_bytes(&mut scope1);
        rng.fill_bytes(&mut scope2);

        let nullifier1 = compute_identity_nullifier(&secret, &blinding, &scope1);
        let nullifier2 = compute_identity_nullifier(&secret, &blinding, &scope2);

        assert_ne!(
            nullifier1, nullifier2,
            "Different scopes should produce different nullifiers"
        );

        let nullifier1_again = compute_identity_nullifier(&secret, &blinding, &scope1);
        assert_eq!(
            nullifier1, nullifier1_again,
            "Same inputs should produce same nullifier"
        );
    }

    #[test]
    fn test_commitment_uniqueness() {
        let secret1 = [1u8; 32];
        let secret2 = [2u8; 32];
        let blinding = [0u8; 32];

        let commitment1 = compute_identity_commitment(&secret1, &blinding);
        let commitment2 = compute_identity_commitment(&secret2, &blinding);

        assert_ne!(
            commitment1, commitment2,
            "Different secrets should produce different commitments"
        );
    }

    #[test]
    fn test_proof_serialization() {
        let mut rng = thread_rng();

        let mut secret = [0u8; 32];
        let mut blinding = [0u8; 32];
        let mut scope = [0u8; 32];
        rng.fill_bytes(&mut secret);
        rng.fill_bytes(&mut blinding);
        rng.fill_bytes(&mut scope);

        let commitment = compute_identity_commitment(&secret, &blinding);

        let mut path = vec![[0u8; 32]; MERKLE_DEPTH];
        let leaf_index = 0u64;

        let mut current = Fr::from_le_bytes_mod_order(&commitment);
        for i in 0..MERKLE_DEPTH {
            let sibling = Fr::from(0u64);
            path[i] = fr_to_bytes(sibling);
            current = poseidon_hash_native(current, sibling);
        }
        let merkle_root = fr_to_bytes(current);

        let input = IdentityProofInput {
            secret,
            blinding,
            leaf_index,
            merkle_path: path,
            merkle_root,
            scope,
        };

        let proof = generate_identity_proof(&input).unwrap();

        let bytes = proof.to_bytes();
        let restored = ZkIdentityProof::from_bytes(&bytes).unwrap();

        let valid = verify_identity_proof(&restored).unwrap();
        assert!(valid, "Restored proof should be valid");

        let base64 = proof.to_base64();
        let restored2 = ZkIdentityProof::from_base64(&base64).unwrap();

        let valid2 = verify_identity_proof(&restored2).unwrap();
        assert!(valid2, "Base64 restored proof should be valid");
    }

    #[test]
    fn test_tampered_proof_rejected() {
        let mut rng = thread_rng();

        let mut secret = [0u8; 32];
        let mut blinding = [0u8; 32];
        let mut scope = [0u8; 32];
        rng.fill_bytes(&mut secret);
        rng.fill_bytes(&mut blinding);
        rng.fill_bytes(&mut scope);

        let commitment = compute_identity_commitment(&secret, &blinding);

        let mut path = vec![[0u8; 32]; MERKLE_DEPTH];
        let leaf_index = 0u64;

        let mut current = Fr::from_le_bytes_mod_order(&commitment);
        for i in 0..MERKLE_DEPTH {
            let sibling = Fr::from(0u64);
            path[i] = fr_to_bytes(sibling);
            current = poseidon_hash_native(current, sibling);
        }
        let merkle_root = fr_to_bytes(current);

        let input = IdentityProofInput {
            secret,
            blinding,
            leaf_index,
            merkle_path: path,
            merkle_root,
            scope,
        };

        let mut proof = generate_identity_proof(&input).unwrap();

        proof.nullifier[0] ^= 0xFF;

        let valid = verify_identity_proof(&proof).unwrap();
        assert!(!valid, "Tampered proof should be invalid");
    }

    #[test]
    fn test_wrong_merkle_root_rejected() {
        let mut rng = thread_rng();

        let mut secret = [0u8; 32];
        let mut blinding = [0u8; 32];
        let mut scope = [0u8; 32];
        rng.fill_bytes(&mut secret);
        rng.fill_bytes(&mut blinding);
        rng.fill_bytes(&mut scope);

        let commitment = compute_identity_commitment(&secret, &blinding);

        let mut path = vec![[0u8; 32]; MERKLE_DEPTH];
        let leaf_index = 0u64;

        let mut current = Fr::from_le_bytes_mod_order(&commitment);
        for i in 0..MERKLE_DEPTH {
            let sibling = Fr::from(0u64);
            path[i] = fr_to_bytes(sibling);
            current = poseidon_hash_native(current, sibling);
        }
        let merkle_root = fr_to_bytes(current);

        let input = IdentityProofInput {
            secret,
            blinding,
            leaf_index,
            merkle_path: path,
            merkle_root,
            scope,
        };

        let mut proof = generate_identity_proof(&input).unwrap();

        proof.merkle_root[0] ^= 0xFF;

        let valid = verify_identity_proof(&proof).unwrap();
        assert!(!valid, "Wrong merkle root should be rejected");
    }

    #[test]
    fn test_merkle_root_computation() {
        let leaf = [0x11; 32];
        let mut path = vec![[0u8; 32]; 3];

        path[0] = [0x22; 32];
        path[1] = [0x33; 32];
        path[2] = [0x44; 32];

        let root = compute_merkle_root(&leaf, 0, &path);

        let leaf_fr = Fr::from_le_bytes_mod_order(&leaf);
        let s0 = Fr::from_le_bytes_mod_order(&path[0]);
        let s1 = Fr::from_le_bytes_mod_order(&path[1]);
        let s2 = Fr::from_le_bytes_mod_order(&path[2]);

        let h1 = poseidon_hash_native(leaf_fr, s0);
        let h2 = poseidon_hash_native(h1, s1);
        let h3 = poseidon_hash_native(h2, s2);

        let expected = fr_to_bytes(h3);

        assert_eq!(root, expected, "Merkle root should match manual computation");
    }
}
