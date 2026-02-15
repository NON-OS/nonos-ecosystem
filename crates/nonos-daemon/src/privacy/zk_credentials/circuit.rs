use ark_bn254::Fr;
use ark_crypto_primitives::sponge::constraints::CryptographicSpongeVar;
use ark_crypto_primitives::sponge::poseidon::constraints::PoseidonSpongeVar;
use ark_r1cs_std::{
    alloc::AllocVar,
    boolean::Boolean,
    eq::EqGadget,
    fields::fp::FpVar,
    select::CondSelectGadget,
};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};

use super::hash::{get_poseidon_config, poseidon_hash_native};
use super::types::MERKLE_DEPTH;

#[derive(Clone)]
pub struct CredentialCircuit {
    identity_secret: Option<Fr>,
    nullifier_seed: Option<Fr>,
    credential_type: Option<Fr>,
    merkle_path: Vec<Option<Fr>>,
    merkle_indices: Vec<Option<bool>>,
    merkle_root: Option<Fr>,
    nullifier: Option<Fr>,
    external_nullifier: Option<Fr>,
    signal_hash: Option<Fr>,
}

impl CredentialCircuit {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        identity_secret: Fr,
        nullifier_seed: Fr,
        credential_type: Fr,
        merkle_path: Vec<Fr>,
        merkle_indices: Vec<bool>,
        merkle_root: Fr,
        external_nullifier: Fr,
        signal_hash: Fr,
    ) -> Self {
        let nullifier = poseidon_hash_native(&[nullifier_seed, external_nullifier]);

        Self {
            identity_secret: Some(identity_secret),
            nullifier_seed: Some(nullifier_seed),
            credential_type: Some(credential_type),
            merkle_path: merkle_path.into_iter().map(Some).collect(),
            merkle_indices: merkle_indices.into_iter().map(Some).collect(),
            merkle_root: Some(merkle_root),
            nullifier: Some(nullifier),
            external_nullifier: Some(external_nullifier),
            signal_hash: Some(signal_hash),
        }
    }

    pub fn empty() -> Self {
        Self {
            identity_secret: None,
            nullifier_seed: None,
            credential_type: None,
            merkle_path: vec![None; MERKLE_DEPTH],
            merkle_indices: vec![None; MERKLE_DEPTH],
            merkle_root: None,
            nullifier: None,
            external_nullifier: None,
            signal_hash: None,
        }
    }
}

impl ConstraintSynthesizer<Fr> for CredentialCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        let identity_secret = FpVar::new_witness(cs.clone(), || {
            self.identity_secret.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let nullifier_seed = FpVar::new_witness(cs.clone(), || {
            self.nullifier_seed.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let credential_type = FpVar::new_witness(cs.clone(), || {
            self.credential_type.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let mut merkle_path = Vec::with_capacity(MERKLE_DEPTH);
        for sibling in &self.merkle_path {
            merkle_path.push(FpVar::new_witness(cs.clone(), || {
                sibling.ok_or(SynthesisError::AssignmentMissing)
            })?);
        }

        let mut merkle_indices = Vec::with_capacity(MERKLE_DEPTH);
        for idx in &self.merkle_indices {
            merkle_indices.push(Boolean::new_witness(cs.clone(), || {
                idx.ok_or(SynthesisError::AssignmentMissing)
            })?);
        }

        let merkle_root = FpVar::new_input(cs.clone(), || {
            self.merkle_root.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let nullifier = FpVar::new_input(cs.clone(), || {
            self.nullifier.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let external_nullifier = FpVar::new_input(cs.clone(), || {
            self.external_nullifier.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let signal_hash = FpVar::new_input(cs.clone(), || {
            self.signal_hash.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let commitment = poseidon_hash_circuit(
            cs.clone(),
            &[
                identity_secret.clone(),
                nullifier_seed.clone(),
                credential_type,
            ],
        )?;

        let computed_root =
            compute_merkle_root_circuit(cs.clone(), &commitment, &merkle_path, &merkle_indices)?;
        computed_root.enforce_equal(&merkle_root)?;

        let computed_nullifier = poseidon_hash_circuit(
            cs.clone(),
            &[nullifier_seed, external_nullifier.clone()],
        )?;
        computed_nullifier.enforce_equal(&nullifier)?;

        let _signal_check = signal_hash.clone();

        Ok(())
    }
}

pub fn poseidon_hash_circuit(
    cs: ConstraintSystemRef<Fr>,
    inputs: &[FpVar<Fr>],
) -> Result<FpVar<Fr>, SynthesisError> {
    let config = get_poseidon_config();

    let mut sponge = PoseidonSpongeVar::new(cs, config);
    sponge.absorb(&inputs)?;

    let output = sponge.squeeze_field_elements(1)?;
    Ok(output[0].clone())
}

pub fn compute_merkle_root_circuit(
    cs: ConstraintSystemRef<Fr>,
    leaf: &FpVar<Fr>,
    path: &[FpVar<Fr>],
    indices: &[Boolean<Fr>],
) -> Result<FpVar<Fr>, SynthesisError> {
    let mut current = leaf.clone();

    for (sibling, is_right) in path.iter().zip(indices.iter()) {
        let left = FpVar::conditionally_select(is_right, sibling, &current)?;
        let right = FpVar::conditionally_select(is_right, &current, sibling)?;

        current = poseidon_hash_circuit(cs.clone(), &[left, right])?;
    }

    Ok(current)
}
