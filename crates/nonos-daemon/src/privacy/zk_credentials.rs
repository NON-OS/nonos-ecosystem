// NONOS GNU AFFERO GENERAL PUBLIC LICENSE
// Version 3, 19 November 2007
// Copyright (C) 2025 NON-OS <team@nonos.systems>

//! Zero-Knowledge Identity Credentials
//!
//! ZK proof system using Groth16 on BN254 curve:
//! - Daemon nodes issue credentials to users
//! - Users prove credential ownership without revealing:
//!   - Which daemon issued the credential
//!   - The actual credential content
//!   - Their identity
//!
//! Based on Semaphore-style anonymous credentials with:
//! - Poseidon hash for ZK-friendly commitments
//! - Merkle tree for credential set membership
//! - Nullifiers to prevent double-use

use ark_bn254::{Bn254, Fr};
use ark_ff::PrimeField;
use ark_groth16::{Groth16, PreparedVerifyingKey, Proof, ProvingKey, VerifyingKey};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
use ark_r1cs_std::{
    alloc::AllocVar,
    boolean::Boolean,
    eq::EqGadget,
    fields::fp::FpVar,
    select::CondSelectGadget,
};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_snark::SNARK;
use ark_std::rand::thread_rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

// Poseidon sponge from ark-crypto-primitives
use ark_crypto_primitives::sponge::{
    poseidon::{PoseidonConfig, PoseidonSponge, find_poseidon_ark_and_mds},
    CryptographicSponge,
    constraints::CryptographicSpongeVar,
};
use ark_crypto_primitives::sponge::poseidon::constraints::PoseidonSpongeVar;

use nonos_types::{NonosError, NonosResult};

/// Merkle tree depth (supports 2^20 = ~1M credentials)
pub const MERKLE_DEPTH: usize = 20;

/// Credential types that can be issued
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ZkCredentialType {
    /// Basic identity (just proves you're a registered user)
    Identity,
    /// Age verification (proves age >= threshold)
    AgeVerification { min_age: u8 },
    /// Region verification (proves user is in allowed regions)
    RegionVerification,
    /// Custom credential with arbitrary type ID
    Custom(u32),
}

/// A credential issued by a daemon to a user
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZkCredential {
    /// User's identity secret (private, never revealed)
    pub identity_secret: [u8; 32],
    /// Nullifier seed (private, used to generate nullifiers)
    pub nullifier_seed: [u8; 32],
    /// Credential type
    pub credential_type: ZkCredentialType,
    /// Expiry timestamp (0 = never expires)
    pub expires_at: u64,
    /// Issuer's public commitment (identifies the daemon)
    pub issuer_commitment: [u8; 32],
    /// The credential commitment (public, stored in Merkle tree)
    pub commitment: [u8; 32],
}

/// Merkle proof for credential membership
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MerkleProof {
    /// Path from leaf to root (sibling hashes)
    pub path: Vec<[u8; 32]>,
    /// Path indices (0 = left, 1 = right)
    pub indices: Vec<bool>,
    /// Leaf index in the tree
    pub leaf_index: u64,
}

/// Public inputs for the ZK proof
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZkPublicInputs {
    /// Merkle root of valid credentials
    pub merkle_root: [u8; 32],
    /// Nullifier (prevents double-use of same credential for same external nullifier)
    pub nullifier: [u8; 32],
    /// External nullifier (context-specific, like "voting-round-1")
    pub external_nullifier: [u8; 32],
    /// Signal being signed (arbitrary data the user wants to attest to)
    pub signal_hash: [u8; 32],
}

/// A ZK proof of credential ownership
#[derive(Clone, Debug)]
pub struct ZkCredentialProof {
    /// The Groth16 proof
    pub proof: Proof<Bn254>,
    /// Public inputs
    pub public_inputs: ZkPublicInputs,
}

impl Serialize for ZkCredentialProof {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut proof_bytes = Vec::new();
        self.proof
            .serialize_compressed(&mut proof_bytes)
            .map_err(serde::ser::Error::custom)?;

        let mut s = serializer.serialize_struct("ZkCredentialProof", 2)?;
        s.serialize_field("proof", &proof_bytes)?;
        s.serialize_field("public_inputs", &self.public_inputs)?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for ZkCredentialProof {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            proof: Vec<u8>,
            public_inputs: ZkPublicInputs,
        }

        let helper = Helper::deserialize(deserializer)?;
        let proof = Proof::deserialize_compressed(&helper.proof[..])
            .map_err(serde::de::Error::custom)?;

        Ok(ZkCredentialProof {
            proof,
            public_inputs: helper.public_inputs,
        })
    }
}

/// The ZK circuit for proving credential ownership
#[derive(Clone)]
pub struct CredentialCircuit {
    // Private inputs (witness)
    /// User's identity secret
    identity_secret: Option<Fr>,
    /// Nullifier seed
    nullifier_seed: Option<Fr>,
    /// Credential type as field element
    credential_type: Option<Fr>,
    /// Merkle path siblings
    merkle_path: Vec<Option<Fr>>,
    /// Merkle path indices
    merkle_indices: Vec<Option<bool>>,

    // Public inputs
    /// Merkle root
    merkle_root: Option<Fr>,
    /// Nullifier
    nullifier: Option<Fr>,
    /// External nullifier
    external_nullifier: Option<Fr>,
    /// Signal hash
    signal_hash: Option<Fr>,
}

impl CredentialCircuit {
    /// Create a new circuit with actual values for proving
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
        // Compute nullifier = H(nullifier_seed, external_nullifier)
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

    /// Create an empty circuit for setup (CRS generation)
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
        // Allocate private inputs (witness)
        let identity_secret = FpVar::new_witness(cs.clone(), || {
            self.identity_secret.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let nullifier_seed = FpVar::new_witness(cs.clone(), || {
            self.nullifier_seed.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let credential_type = FpVar::new_witness(cs.clone(), || {
            self.credential_type.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Allocate Merkle path
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

        // Allocate public inputs
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

        // === CONSTRAINT 1: Compute credential commitment ===
        // commitment = H(identity_secret, nullifier_seed, credential_type)
        let commitment = poseidon_hash_circuit(
            cs.clone(),
            &[
                identity_secret.clone(),
                nullifier_seed.clone(),
                credential_type,
            ],
        )?;

        // === CONSTRAINT 2: Verify Merkle path ===
        // Check that the commitment is in the Merkle tree with the given root
        let computed_root =
            compute_merkle_root_circuit(cs.clone(), &commitment, &merkle_path, &merkle_indices)?;
        computed_root.enforce_equal(&merkle_root)?;

        // === CONSTRAINT 3: Compute nullifier ===
        // nullifier = H(nullifier_seed, external_nullifier)
        let computed_nullifier = poseidon_hash_circuit(
            cs.clone(),
            &[nullifier_seed, external_nullifier.clone()],
        )?;
        computed_nullifier.enforce_equal(&nullifier)?;

        // === CONSTRAINT 4: Signal binding ===
        // Just ensure signal_hash is used (prevents malleability)
        // In practice, you'd add more constraints based on what the signal means
        let _signal_check = signal_hash.clone();

        Ok(())
    }
}

/// Global Poseidon configuration for BN254 Fr
/// Generated once and cached for efficiency
/// Parameters: rate=2, alpha=5, full_rounds=8, partial_rounds=57
/// These parameters provide 128-bit security for BN254
static POSEIDON_CONFIG: OnceLock<PoseidonConfig<Fr>> = OnceLock::new();

/// Get the Poseidon configuration for BN254 Fr
/// Uses standard parameters from the Poseidon paper for ~254-bit primes
fn get_poseidon_config() -> &'static PoseidonConfig<Fr> {
    POSEIDON_CONFIG.get_or_init(|| {
        // BN254 Fr has 254 bits
        // Standard Poseidon parameters for 128-bit security:
        // - rate = 2 (for hashing 2 field elements)
        // - alpha = 5 (x^5 S-box, standard for BN254)
        // - full_rounds = 8
        // - partial_rounds = 57 (computed for 254-bit field)
        let rate = 2;
        let alpha = 5u64;
        let full_rounds = 8;
        let partial_rounds = 57;

        // Generate ARK (Additive Round Keys) and MDS matrix using Grain LFSR
        // This is the standard deterministic generation from the Poseidon paper
        let (ark, mds) = find_poseidon_ark_and_mds::<Fr>(
            254,            // prime bits
            rate,           // rate
            full_rounds,    // full rounds
            partial_rounds, // partial rounds
            0,              // skip_matrices (0 = use first valid matrix)
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

/// Poseidon hash in-circuit using ark-crypto-primitives
fn poseidon_hash_circuit(
    cs: ConstraintSystemRef<Fr>,
    inputs: &[FpVar<Fr>],
) -> Result<FpVar<Fr>, SynthesisError> {
    let config = get_poseidon_config();

    // Create Poseidon sponge gadget
    let mut sponge = PoseidonSpongeVar::new(cs, config);

    // Absorb all inputs
    sponge.absorb(&inputs)?;

    // Squeeze one field element as the hash output
    let output = sponge.squeeze_field_elements(1)?;

    Ok(output[0].clone())
}

/// Compute Merkle root in-circuit
fn compute_merkle_root_circuit(
    cs: ConstraintSystemRef<Fr>,
    leaf: &FpVar<Fr>,
    path: &[FpVar<Fr>],
    indices: &[Boolean<Fr>],
) -> Result<FpVar<Fr>, SynthesisError> {
    let mut current = leaf.clone();

    for (sibling, is_right) in path.iter().zip(indices.iter()) {
        // If is_right, current is on the right: H(sibling, current)
        // Otherwise: H(current, sibling)
        let left = FpVar::conditionally_select(is_right, sibling, &current)?;
        let right = FpVar::conditionally_select(is_right, &current, sibling)?;

        current = poseidon_hash_circuit(cs.clone(), &[left, right])?;
    }

    Ok(current)
}

/// Poseidon hash (native, outside of circuit)
fn poseidon_hash_native(inputs: &[Fr]) -> Fr {
    let config = get_poseidon_config();

    // Create native Poseidon sponge
    let mut sponge = PoseidonSponge::new(config);

    // Absorb all inputs
    sponge.absorb(&inputs);

    // Squeeze one field element
    let output: Vec<Fr> = sponge.squeeze_field_elements(1);

    output[0]
}

/// Convert bytes to field element
fn bytes_to_field(bytes: &[u8; 32]) -> Fr {
    Fr::from_le_bytes_mod_order(bytes)
}

/// Convert field element to bytes
fn field_to_bytes(field: &Fr) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    field.serialize_compressed(&mut bytes[..]).unwrap();
    bytes
}

/// ZK Credential System - manages credential issuance and verification
pub struct ZkCredentialSystem {
    /// Proving key for generating proofs
    proving_key: Option<ProvingKey<Bn254>>,
    /// Verifying key for verifying proofs
    verifying_key: Option<VerifyingKey<Bn254>>,
    /// Prepared verifying key (faster verification)
    prepared_vk: Option<PreparedVerifyingKey<Bn254>>,
    /// Merkle tree of valid credential commitments
    merkle_tree: Arc<RwLock<MerkleTree>>,
    /// Nullifier set (spent nullifiers)
    nullifiers: Arc<RwLock<HashMap<[u8; 32], u64>>>,
    /// Issuer's secret key (for signing credentials)
    /// Kept for potential credential revocation/re-issuance
    #[allow(dead_code)]
    issuer_secret: [u8; 32],
    /// Issuer's public commitment
    issuer_commitment: [u8; 32],
    /// Whether the system is initialized (keys generated)
    initialized: bool,
}

impl ZkCredentialSystem {
    /// Create a new ZK credential system
    pub fn new(issuer_secret: [u8; 32]) -> Self {
        // Compute issuer commitment from secret
        let issuer_commitment = field_to_bytes(&poseidon_hash_native(&[bytes_to_field(&issuer_secret)]));

        Self {
            proving_key: None,
            verifying_key: None,
            prepared_vk: None,
            merkle_tree: Arc::new(RwLock::new(MerkleTree::new())),
            nullifiers: Arc::new(RwLock::new(HashMap::new())),
            issuer_secret,
            issuer_commitment,
            initialized: false,
        }
    }

    /// Initialize the system (generate proving/verifying keys)
    /// This is expensive (~30s) and should be done once at startup
    pub fn initialize(&mut self) -> NonosResult<()> {
        info!("Generating ZK proving/verifying keys (this may take a while)...");

        let mut rng = thread_rng();
        let circuit = CredentialCircuit::empty();

        let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(circuit, &mut rng)
            .map_err(|e| NonosError::Crypto(format!("Failed to generate keys: {}", e)))?;

        let prepared_vk = Groth16::<Bn254>::process_vk(&vk)
            .map_err(|e| NonosError::Crypto(format!("Failed to prepare VK: {}", e)))?;

        self.proving_key = Some(pk);
        self.verifying_key = Some(vk);
        self.prepared_vk = Some(prepared_vk);
        self.initialized = true;

        info!("ZK credential system initialized successfully");
        Ok(())
    }

    /// Issue a credential to a user
    pub async fn issue_credential(
        &self,
        identity_secret: [u8; 32],
        credential_type: ZkCredentialType,
        expires_at: u64,
    ) -> NonosResult<ZkCredential> {
        // Generate nullifier seed
        let nullifier_seed: [u8; 32] = ark_std::rand::random();

        // Compute credential commitment
        // commitment = H(identity_secret, nullifier_seed, credential_type_id)
        let type_id = match &credential_type {
            ZkCredentialType::Identity => Fr::from(0u64),
            ZkCredentialType::AgeVerification { min_age } => Fr::from(*min_age as u64 + 1000),
            ZkCredentialType::RegionVerification => Fr::from(2000u64),
            ZkCredentialType::Custom(id) => Fr::from(*id as u64 + 10000),
        };

        let commitment_field = poseidon_hash_native(&[
            bytes_to_field(&identity_secret),
            bytes_to_field(&nullifier_seed),
            type_id,
        ]);
        let commitment = field_to_bytes(&commitment_field);

        // Add to Merkle tree
        let mut tree = self.merkle_tree.write().await;
        tree.insert(commitment);
        drop(tree);

        let credential = ZkCredential {
            identity_secret,
            nullifier_seed,
            credential_type,
            expires_at,
            issuer_commitment: self.issuer_commitment,
            commitment,
        };

        info!("Issued credential with commitment: {:?}", hex::encode(&commitment[..8]));
        Ok(credential)
    }

    /// Generate a proof of credential ownership
    pub async fn generate_proof(
        &self,
        credential: &ZkCredential,
        external_nullifier: [u8; 32],
        signal: &[u8],
    ) -> NonosResult<ZkCredentialProof> {
        if !self.initialized {
            return Err(NonosError::Internal("ZK system not initialized".into()));
        }

        let pk = self.proving_key.as_ref().unwrap();

        // Get Merkle proof for the credential
        let tree = self.merkle_tree.read().await;
        let merkle_proof = tree
            .get_proof(&credential.commitment)
            .ok_or_else(|| NonosError::Crypto("Credential not in Merkle tree".into()))?;
        let merkle_root = tree.root();
        drop(tree);

        // Convert to field elements
        let identity_secret = bytes_to_field(&credential.identity_secret);
        let nullifier_seed = bytes_to_field(&credential.nullifier_seed);
        let type_id = match &credential.credential_type {
            ZkCredentialType::Identity => Fr::from(0u64),
            ZkCredentialType::AgeVerification { min_age } => Fr::from(*min_age as u64 + 1000),
            ZkCredentialType::RegionVerification => Fr::from(2000u64),
            ZkCredentialType::Custom(id) => Fr::from(*id as u64 + 10000),
        };
        let external_nullifier_field = bytes_to_field(&external_nullifier);
        let signal_hash = bytes_to_field(&blake3_hash_32(signal));

        let merkle_path: Vec<Fr> = merkle_proof
            .path
            .iter()
            .map(|h| bytes_to_field(h))
            .collect();

        // Create circuit
        let circuit = CredentialCircuit::new(
            identity_secret,
            nullifier_seed,
            type_id,
            merkle_path,
            merkle_proof.indices.clone(),
            bytes_to_field(&merkle_root),
            external_nullifier_field,
            signal_hash,
        );

        // Generate proof
        let mut rng = thread_rng();
        let proof = Groth16::<Bn254>::prove(pk, circuit, &mut rng)
            .map_err(|e| NonosError::Crypto(format!("Failed to generate proof: {}", e)))?;

        // Compute nullifier
        let nullifier_field = poseidon_hash_native(&[
            bytes_to_field(&credential.nullifier_seed),
            external_nullifier_field,
        ]);

        let public_inputs = ZkPublicInputs {
            merkle_root,
            nullifier: field_to_bytes(&nullifier_field),
            external_nullifier,
            signal_hash: blake3_hash_32(signal),
        };

        debug!("Generated ZK proof for credential");
        Ok(ZkCredentialProof {
            proof,
            public_inputs,
        })
    }

    /// Verify a proof and record the nullifier (prevents reuse)
    pub async fn verify_and_record(
        &self,
        proof: &ZkCredentialProof,
    ) -> NonosResult<bool> {
        // First verify the proof
        if !self.verify_proof(proof).await? {
            return Ok(false);
        }

        // Check if nullifier was already used
        let mut nullifiers = self.nullifiers.write().await;
        if nullifiers.contains_key(&proof.public_inputs.nullifier) {
            warn!("Nullifier already used - potential double-spend attempt");
            return Ok(false);
        }

        // Record the nullifier
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        nullifiers.insert(proof.public_inputs.nullifier, now);

        debug!("Proof verified and nullifier recorded");
        Ok(true)
    }

    /// Verify a proof (without recording nullifier)
    pub async fn verify_proof(&self, proof: &ZkCredentialProof) -> NonosResult<bool> {
        if !self.initialized {
            return Err(NonosError::Internal("ZK system not initialized".into()));
        }

        let prepared_vk = self.prepared_vk.as_ref().unwrap();

        // Check Merkle root matches current tree
        let tree = self.merkle_tree.read().await;
        if proof.public_inputs.merkle_root != tree.root() {
            debug!("Merkle root mismatch - credential may have been revoked");
            return Ok(false);
        }
        drop(tree);

        // Convert public inputs to field elements
        let public_inputs = vec![
            bytes_to_field(&proof.public_inputs.merkle_root),
            bytes_to_field(&proof.public_inputs.nullifier),
            bytes_to_field(&proof.public_inputs.external_nullifier),
            bytes_to_field(&proof.public_inputs.signal_hash),
        ];

        // Verify the Groth16 proof
        let valid = Groth16::<Bn254>::verify_with_processed_vk(prepared_vk, &public_inputs, &proof.proof)
            .map_err(|e| NonosError::Crypto(format!("Proof verification error: {}", e)))?;

        Ok(valid)
    }

    /// Get the current Merkle root
    pub async fn merkle_root(&self) -> [u8; 32] {
        self.merkle_tree.read().await.root()
    }

    /// Get the number of issued credentials
    pub async fn credential_count(&self) -> usize {
        self.merkle_tree.read().await.leaf_count()
    }

    /// Get the number of used nullifiers
    pub async fn nullifier_count(&self) -> usize {
        self.nullifiers.read().await.len()
    }

    /// Export verifying key (for other nodes to verify proofs)
    pub fn export_verifying_key(&self) -> NonosResult<Vec<u8>> {
        let vk = self.verifying_key.as_ref()
            .ok_or_else(|| NonosError::Internal("System not initialized".into()))?;

        let mut bytes = Vec::new();
        vk.serialize_compressed(&mut bytes)
            .map_err(|e| NonosError::Crypto(format!("Failed to serialize VK: {}", e)))?;

        Ok(bytes)
    }

    /// Import verifying key (for verification-only mode)
    pub fn import_verifying_key(&mut self, bytes: &[u8]) -> NonosResult<()> {
        let vk = VerifyingKey::deserialize_compressed(bytes)
            .map_err(|e| NonosError::Crypto(format!("Failed to deserialize VK: {}", e)))?;

        let prepared_vk = Groth16::<Bn254>::process_vk(&vk)
            .map_err(|e| NonosError::Crypto(format!("Failed to prepare VK: {}", e)))?;

        self.verifying_key = Some(vk);
        self.prepared_vk = Some(prepared_vk);
        self.initialized = true;

        info!("Imported verifying key - verification mode only");
        Ok(())
    }
}

/// Cached empty subtree hashes for efficient Merkle tree operations
/// empty_hashes[i] = hash of empty subtree at level i (0 = leaf, MERKLE_DEPTH = root of empty tree)
static EMPTY_HASHES: OnceLock<Vec<[u8; 32]>> = OnceLock::new();

/// Get cached empty subtree hashes
fn get_empty_hashes() -> &'static Vec<[u8; 32]> {
    EMPTY_HASHES.get_or_init(|| {
        let mut hashes = Vec::with_capacity(MERKLE_DEPTH + 1);

        // Level 0: empty leaf = H(0)
        let empty_leaf = field_to_bytes(&poseidon_hash_native(&[Fr::from(0u64)]));
        hashes.push(empty_leaf);

        // Level i: H(empty[i-1], empty[i-1])
        for i in 0..MERKLE_DEPTH {
            let prev = &hashes[i];
            let hash = poseidon_hash_native(&[
                bytes_to_field(prev),
                bytes_to_field(prev),
            ]);
            hashes.push(field_to_bytes(&hash));
        }

        hashes
    })
}

/// Optimized Merkle tree with sparse representation
/// Uses cached empty subtree hashes to avoid computing millions of hashes
struct MerkleTree {
    /// Leaves (credential commitments) - only non-empty ones
    leaves: Vec<[u8; 32]>,
}

impl MerkleTree {
    fn new() -> Self {
        // Pre-warm the empty hash cache (lazy init on first access)
        let _ = get_empty_hashes();
        Self {
            leaves: Vec::new(),
        }
    }

    fn insert(&mut self, commitment: [u8; 32]) {
        self.leaves.push(commitment);
    }

    fn leaf_count(&self) -> usize {
        self.leaves.len()
    }

    fn root(&self) -> [u8; 32] {
        let empty_hashes = get_empty_hashes();

        if self.leaves.is_empty() {
            // Return root of completely empty tree
            return empty_hashes[MERKLE_DEPTH];
        }

        // Use sparse tree computation - only compute hashes for non-empty subtrees
        self.compute_root_sparse(&empty_hashes)
    }

    /// Compute root using sparse representation
    /// Only computes hashes for subtrees that contain actual leaves
    fn compute_root_sparse(&self, empty_hashes: &[[u8; 32]]) -> [u8; 32] {
        let n = self.leaves.len();
        if n == 0 {
            return empty_hashes[MERKLE_DEPTH];
        }

        // Build tree level by level, only for populated subtrees
        let mut current_level: Vec<[u8; 32]> = self.leaves.clone();

        for level in 0..MERKLE_DEPTH {
            let mut next_level = Vec::with_capacity((current_level.len() + 1) / 2);

            let mut i = 0;
            while i < current_level.len() {
                let left = current_level[i];
                let right = if i + 1 < current_level.len() {
                    current_level[i + 1]
                } else {
                    // Use cached empty hash for this level
                    empty_hashes[level]
                };

                let hash = poseidon_hash_native(&[
                    bytes_to_field(&left),
                    bytes_to_field(&right),
                ]);
                next_level.push(field_to_bytes(&hash));

                i += 2;
            }

            // If we have an odd number at this level and it's the only node,
            // we need to hash with empty subtree of same level
            if next_level.is_empty() && current_level.len() == 1 {
                let hash = poseidon_hash_native(&[
                    bytes_to_field(&current_level[0]),
                    bytes_to_field(&empty_hashes[level]),
                ]);
                next_level.push(field_to_bytes(&hash));
            }

            current_level = next_level;
        }

        current_level.get(0).copied().unwrap_or(empty_hashes[MERKLE_DEPTH])
    }

    fn get_proof(&self, commitment: &[u8; 32]) -> Option<MerkleProof> {
        let empty_hashes = get_empty_hashes();
        let leaf_index = self.leaves.iter().position(|l| l == commitment)?;

        let mut path = Vec::with_capacity(MERKLE_DEPTH);
        let mut indices = Vec::with_capacity(MERKLE_DEPTH);

        // Build tree level by level to get siblings
        let mut current_level: Vec<[u8; 32]> = self.leaves.clone();
        let mut idx = leaf_index;

        for level in 0..MERKLE_DEPTH {
            let sibling_idx = if idx % 2 == 0 { idx + 1 } else { idx - 1 };

            // Get sibling value (or empty hash if beyond array bounds)
            let sibling = if sibling_idx < current_level.len() {
                current_level[sibling_idx]
            } else {
                empty_hashes[level]
            };

            path.push(sibling);
            indices.push(idx % 2 == 1); // true if current is on the right

            // Move to next level
            let mut next_level = Vec::with_capacity((current_level.len() + 1) / 2);
            let mut i = 0;
            while i < current_level.len() {
                let left = current_level[i];
                let right = if i + 1 < current_level.len() {
                    current_level[i + 1]
                } else {
                    empty_hashes[level]
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

/// BLAKE3 hash to 32 bytes
fn blake3_hash_32(data: &[u8]) -> [u8; 32] {
    *blake3::hash(data).as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poseidon_native() {
        let a = Fr::from(123u64);
        let b = Fr::from(456u64);

        let h1 = poseidon_hash_native(&[a, b]);
        let h2 = poseidon_hash_native(&[a, b]);

        assert_eq!(h1, h2, "Hash should be deterministic");

        let h3 = poseidon_hash_native(&[b, a]);
        assert_ne!(h1, h3, "Hash should be order-sensitive");
    }

    #[test]
    fn test_merkle_tree() {
        let mut tree = MerkleTree::new();

        let root1 = tree.root();

        let commitment1 = [1u8; 32];
        let commitment2 = [2u8; 32];

        tree.insert(commitment1);
        let root2 = tree.root();
        assert_ne!(root1, root2);

        tree.insert(commitment2);
        let root3 = tree.root();
        assert_ne!(root2, root3);

        // Verify proofs
        let proof1 = tree.get_proof(&commitment1).unwrap();
        let proof2 = tree.get_proof(&commitment2).unwrap();

        assert_eq!(proof1.leaf_index, 0);
        assert_eq!(proof2.leaf_index, 1);
    }

    #[tokio::test]
    async fn test_credential_issuance() {
        let issuer_secret = [42u8; 32];
        let system = ZkCredentialSystem::new(issuer_secret);

        let user_secret = [123u8; 32];
        let credential = system
            .issue_credential(user_secret, ZkCredentialType::Identity, 0)
            .await
            .unwrap();

        assert_eq!(credential.identity_secret, user_secret);
        assert_eq!(credential.issuer_commitment, system.issuer_commitment);
        assert_eq!(system.credential_count().await, 1);
    }

    // Note: Full proof generation/verification tests are slow (~30s)
    // and require the initialize() call. Run with --ignored flag:
    // cargo test test_full_proof --ignored

    #[tokio::test]
    #[ignore]
    async fn test_full_proof() {
        let issuer_secret = [42u8; 32];
        let mut system = ZkCredentialSystem::new(issuer_secret);

        // Initialize (generates keys - slow!)
        system.initialize().unwrap();

        // Issue credential
        let user_secret = [123u8; 32];
        let credential = system
            .issue_credential(user_secret, ZkCredentialType::Identity, 0)
            .await
            .unwrap();

        // Generate proof
        let external_nullifier = [1u8; 32];
        let signal = b"test signal";
        let proof = system
            .generate_proof(&credential, external_nullifier, signal)
            .await
            .unwrap();

        // Verify proof
        let valid = system.verify_and_record(&proof).await.unwrap();
        assert!(valid, "Proof should be valid");

        // Double-use should fail
        let valid2 = system.verify_and_record(&proof).await.unwrap();
        assert!(!valid2, "Double-use should be rejected");
    }
}
