use ark_bn254::{Bn254, Fr};
use ark_groth16::{Groth16, PreparedVerifyingKey, ProvingKey, VerifyingKey};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_snark::SNARK;
use ark_std::rand::thread_rng;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use nonos_types::{NonosError, NonosResult};

use super::circuit::CredentialCircuit;
use super::hash::{blake3_hash_32, bytes_to_field, field_to_bytes, poseidon_hash_native};
use super::merkle::MerkleTree;
use super::types::{ZkCredential, ZkCredentialProof, ZkCredentialType, ZkPublicInputs};

pub struct ZkCredentialSystem {
    proving_key: Option<ProvingKey<Bn254>>,
    verifying_key: Option<VerifyingKey<Bn254>>,
    prepared_vk: Option<PreparedVerifyingKey<Bn254>>,
    merkle_tree: Arc<RwLock<MerkleTree>>,
    nullifiers: Arc<RwLock<HashMap<[u8; 32], u64>>>,
    _issuer_secret: [u8; 32],
    issuer_commitment: [u8; 32],
    initialized: bool,
}

impl ZkCredentialSystem {
    pub fn new(issuer_secret: [u8; 32]) -> Self {
        let issuer_commitment = field_to_bytes(&poseidon_hash_native(&[bytes_to_field(&issuer_secret)]));

        Self {
            proving_key: None,
            verifying_key: None,
            prepared_vk: None,
            merkle_tree: Arc::new(RwLock::new(MerkleTree::new())),
            nullifiers: Arc::new(RwLock::new(HashMap::new())),
            _issuer_secret: issuer_secret,
            issuer_commitment,
            initialized: false,
        }
    }

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

    pub async fn issue_credential(
        &self,
        identity_secret: [u8; 32],
        credential_type: ZkCredentialType,
        expires_at: u64,
    ) -> NonosResult<ZkCredential> {
        let nullifier_seed: [u8; 32] = ark_std::rand::random();

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

        let tree = self.merkle_tree.read().await;
        let merkle_proof = tree
            .get_proof(&credential.commitment)
            .ok_or_else(|| NonosError::Crypto("Credential not in Merkle tree".into()))?;
        let merkle_root = tree.root();
        drop(tree);

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
            .map(bytes_to_field)
            .collect();

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

        let mut rng = thread_rng();
        let proof = Groth16::<Bn254>::prove(pk, circuit, &mut rng)
            .map_err(|e| NonosError::Crypto(format!("Failed to generate proof: {}", e)))?;

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

    pub async fn verify_and_record(
        &self,
        proof: &ZkCredentialProof,
    ) -> NonosResult<bool> {
        if !self.verify_proof(proof).await? {
            return Ok(false);
        }

        let mut nullifiers = self.nullifiers.write().await;
        if nullifiers.contains_key(&proof.public_inputs.nullifier) {
            warn!("Nullifier already used - potential double-spend attempt");
            return Ok(false);
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        nullifiers.insert(proof.public_inputs.nullifier, now);

        debug!("Proof verified and nullifier recorded");
        Ok(true)
    }

    pub async fn verify_proof(&self, proof: &ZkCredentialProof) -> NonosResult<bool> {
        if !self.initialized {
            return Err(NonosError::Internal("ZK system not initialized".into()));
        }

        let prepared_vk = self.prepared_vk.as_ref().unwrap();

        let tree = self.merkle_tree.read().await;
        if proof.public_inputs.merkle_root != tree.root() {
            debug!("Merkle root mismatch - credential may have been revoked");
            return Ok(false);
        }
        drop(tree);

        let public_inputs = vec![
            bytes_to_field(&proof.public_inputs.merkle_root),
            bytes_to_field(&proof.public_inputs.nullifier),
            bytes_to_field(&proof.public_inputs.external_nullifier),
            bytes_to_field(&proof.public_inputs.signal_hash),
        ];

        let valid = Groth16::<Bn254>::verify_with_processed_vk(prepared_vk, &public_inputs, &proof.proof)
            .map_err(|e| NonosError::Crypto(format!("Proof verification error: {}", e)))?;

        Ok(valid)
    }

    pub async fn merkle_root(&self) -> [u8; 32] {
        self.merkle_tree.read().await.root()
    }

    pub async fn credential_count(&self) -> usize {
        self.merkle_tree.read().await.leaf_count()
    }

    pub async fn nullifier_count(&self) -> usize {
        self.nullifiers.read().await.len()
    }

    pub fn export_verifying_key(&self) -> NonosResult<Vec<u8>> {
        let vk = self.verifying_key.as_ref()
            .ok_or_else(|| NonosError::Internal("System not initialized".into()))?;

        let mut bytes = Vec::new();
        vk.serialize_compressed(&mut bytes)
            .map_err(|e| NonosError::Crypto(format!("Failed to serialize VK: {}", e)))?;

        Ok(bytes)
    }

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
