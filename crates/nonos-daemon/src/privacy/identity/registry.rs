use super::nullifier::BoundedNullifierSet;
use super::types::{IdentityCommitment, ScopedNullifier, VerificationResult};
use ark_bn254::{Bn254, Fr};
use ark_groth16::{Groth16, PreparedVerifyingKey, Proof};
use ark_serialize::CanonicalDeserialize;
use ark_snark::SNARK;
use nonos_crypto::poseidon_canonical::{
    bytes_to_fr, fr_to_bytes, poseidon_hash2_fields, poseidon_hash3_fields, PoseidonMerkleTree,
};
use nonos_types::{NonosError, NonosResult};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

const MAX_ACCEPTED_ROOTS: usize = 256;
const MAX_IDENTITIES: usize = 1_048_576;

pub struct ZkIdentityRegistry {
    tree: Arc<RwLock<PoseidonMerkleTree>>,
    accepted_roots: Arc<RwLock<VecDeque<[u8; 32]>>>,
    current_root: Arc<RwLock<[u8; 32]>>,
    nullifiers: Arc<RwLock<BoundedNullifierSet>>,
    identities: Arc<RwLock<HashMap<[u8; 32], IdentityCommitment>>>,
    verifying_key: Arc<RwLock<Option<PreparedVerifyingKey<Bn254>>>>,
    production_mode: AtomicBool,
    registrations: AtomicU64,
    verifications_passed: AtomicU64,
    verifications_failed: AtomicU64,
}

impl ZkIdentityRegistry {
    pub fn new() -> Self {
        let tree = PoseidonMerkleTree::new(20);
        let initial_root = tree.root();
        let mut accepted = VecDeque::new();
        accepted.push_back(initial_root);

        Self {
            tree: Arc::new(RwLock::new(tree)),
            accepted_roots: Arc::new(RwLock::new(accepted)),
            current_root: Arc::new(RwLock::new(initial_root)),
            nullifiers: Arc::new(RwLock::new(BoundedNullifierSet::new())),
            identities: Arc::new(RwLock::new(HashMap::new())),
            verifying_key: Arc::new(RwLock::new(None)),
            production_mode: AtomicBool::new(false),
            registrations: AtomicU64::new(0),
            verifications_passed: AtomicU64::new(0),
            verifications_failed: AtomicU64::new(0),
        }
    }

    pub fn set_production_mode(&self, enabled: bool) {
        self.production_mode.store(enabled, Ordering::SeqCst);
        if enabled {
            info!("ZK Identity Registry: production mode enabled");
        }
    }

    pub async fn load_verifying_key(&self, vk_bytes: &[u8]) -> NonosResult<()> {
        let vk = ark_groth16::VerifyingKey::<Bn254>::deserialize_compressed(vk_bytes)
            .map_err(|e| NonosError::Internal(format!("Invalid VK: {}", e)))?;
        let pvk = Groth16::<Bn254>::process_vk(&vk)
            .map_err(|e| NonosError::Internal(format!("VK processing failed: {}", e)))?;

        let mut key = self.verifying_key.write().await;
        *key = Some(pvk);
        info!("Loaded ZK identity verifying key");
        Ok(())
    }

    pub async fn register_identity(
        &self,
        secret: &[u8; 32],
        blinding: &[u8; 32],
    ) -> NonosResult<IdentityCommitment> {
        {
            let identities = self.identities.read().await;
            if identities.len() >= MAX_IDENTITIES {
                return Err(NonosError::Internal("Identity registry full".into()));
            }
        }

        let commitment_fr = poseidon_hash2_fields(bytes_to_fr(secret), bytes_to_fr(blinding));
        let commitment = fr_to_bytes(&commitment_fr);

        {
            let identities = self.identities.read().await;
            if identities.contains_key(&commitment) {
                return Err(NonosError::Internal("Identity already registered".into()));
            }
        }

        let index = {
            let mut tree = self.tree.write().await;
            tree.insert(commitment)
        };

        let new_root = {
            let tree = self.tree.read().await;
            tree.root()
        };

        {
            let mut roots = self.accepted_roots.write().await;
            roots.push_back(new_root);
            if roots.len() > MAX_ACCEPTED_ROOTS {
                roots.pop_front();
            }
        }

        {
            let mut current = self.current_root.write().await;
            *current = new_root;
        }

        let identity = IdentityCommitment {
            commitment,
            index,
            registered_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        {
            let mut identities = self.identities.write().await;
            identities.insert(commitment, identity.clone());
        }

        self.registrations.fetch_add(1, Ordering::Relaxed);
        Ok(identity)
    }

    pub fn compute_nullifier(secret: &[u8; 32], commitment: &[u8; 32], scope: &[u8; 32]) -> [u8; 32] {
        let result = poseidon_hash3_fields(
            bytes_to_fr(secret),
            bytes_to_fr(commitment),
            bytes_to_fr(scope),
        );
        fr_to_bytes(&result)
    }

    pub async fn get_proof(&self, commitment: &[u8; 32]) -> NonosResult<Vec<([u8; 32], bool)>> {
        let identities = self.identities.read().await;
        let identity = identities
            .get(commitment)
            .ok_or_else(|| NonosError::Internal("Identity not found".into()))?;

        let tree = self.tree.read().await;
        Ok(tree.proof(identity.index))
    }

    pub async fn current_root(&self) -> [u8; 32] {
        *self.current_root.read().await
    }

    pub async fn is_root_accepted(&self, root: &[u8; 32]) -> bool {
        let roots = self.accepted_roots.read().await;
        roots.iter().any(|r| r == root)
    }

    pub async fn is_nullifier_used(&self, nullifier: &[u8; 32], scope: &[u8; 32]) -> bool {
        let nullifiers = self.nullifiers.read().await;
        nullifiers.contains(nullifier, scope)
    }

    pub async fn verify_proof(
        &self,
        proof_bytes: &[u8],
        merkle_root: &[u8; 32],
        nullifier: &[u8; 32],
        scope: &[u8; 32],
        signal_hash: Option<&[u8; 32]>,
    ) -> NonosResult<VerificationResult> {
        if !self.is_root_accepted(merkle_root).await {
            self.verifications_failed.fetch_add(1, Ordering::Relaxed);
            warn!("Proof rejected: unknown Merkle root");
            return Ok(VerificationResult {
                valid: false,
                reason: Some("Unknown Merkle root".into()),
                nullifier_recorded: false,
            });
        }

        if self.is_nullifier_used(nullifier, scope).await {
            self.verifications_failed.fetch_add(1, Ordering::Relaxed);
            warn!("Proof rejected: nullifier already used");
            return Ok(VerificationResult {
                valid: false,
                reason: Some("Nullifier already used".into()),
                nullifier_recorded: false,
            });
        }

        let vk = {
            let key = self.verifying_key.read().await;
            match key.as_ref() {
                Some(k) => k.clone(),
                None => {
                    if self.production_mode.load(Ordering::SeqCst) {
                        error!("Production mode: VK not loaded, rejecting proof");
                        self.verifications_failed.fetch_add(1, Ordering::Relaxed);
                        return Ok(VerificationResult {
                            valid: false,
                            reason: Some("Verifying key not loaded".into()),
                            nullifier_recorded: false,
                        });
                    }
                    self.record_nullifier(nullifier, scope).await;
                    self.verifications_passed.fetch_add(1, Ordering::Relaxed);
                    return Ok(VerificationResult {
                        valid: true,
                        reason: Some("VK not loaded (dev mode)".into()),
                        nullifier_recorded: true,
                    });
                }
            }
        };

        let proof = Proof::<Bn254>::deserialize_compressed(proof_bytes)
            .map_err(|e| NonosError::Internal(format!("Invalid proof: {}", e)))?;

        let mut public_inputs = vec![
            bytes_to_fr(merkle_root),
            bytes_to_fr(nullifier),
            bytes_to_fr(scope),
        ];
        if let Some(signal) = signal_hash {
            public_inputs.push(bytes_to_fr(signal));
        }

        let valid = Groth16::<Bn254>::verify_with_processed_vk(&vk, &public_inputs, &proof)
            .map_err(|e| NonosError::Internal(format!("Verification error: {}", e)))?;

        if valid {
            self.record_nullifier(nullifier, scope).await;
            self.verifications_passed.fetch_add(1, Ordering::Relaxed);
            Ok(VerificationResult {
                valid: true,
                reason: None,
                nullifier_recorded: true,
            })
        } else {
            self.verifications_failed.fetch_add(1, Ordering::Relaxed);
            Ok(VerificationResult {
                valid: false,
                reason: Some("Invalid proof".into()),
                nullifier_recorded: false,
            })
        }
    }

    async fn record_nullifier(&self, nullifier: &[u8; 32], scope: &[u8; 32]) {
        let mut nullifiers = self.nullifiers.write().await;
        nullifiers.insert(*nullifier, *scope);
    }

    pub fn stats(&self) -> (u64, u64, u64) {
        (
            self.registrations.load(Ordering::Relaxed),
            self.verifications_passed.load(Ordering::Relaxed),
            self.verifications_failed.load(Ordering::Relaxed),
        )
    }

    pub async fn identity_count(&self) -> usize {
        self.identities.read().await.len()
    }

    pub async fn accepted_roots_count(&self) -> usize {
        self.accepted_roots.read().await.len()
    }
}

impl Default for ZkIdentityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nonos_crypto::poseidon_canonical::PoseidonMerkleTree;

    #[tokio::test]
    async fn test_register_identity() {
        let registry = ZkIdentityRegistry::new();
        let secret = [0x11; 32];
        let blinding = [0x22; 32];

        let identity = registry.register_identity(&secret, &blinding).await.unwrap();
        assert_eq!(identity.index, 0);
        assert!(registry.accepted_roots_count().await >= 2);
    }

    #[tokio::test]
    async fn test_duplicate_registration_fails() {
        let registry = ZkIdentityRegistry::new();
        let secret = [0x11; 32];
        let blinding = [0x22; 32];

        registry.register_identity(&secret, &blinding).await.unwrap();
        assert!(registry.register_identity(&secret, &blinding).await.is_err());
    }

    #[tokio::test]
    async fn test_nullifier_scope_isolation() {
        let registry = ZkIdentityRegistry::new();
        let nullifier = [0xaa; 32];
        let scope1 = [0x01; 32];
        let scope2 = [0x02; 32];

        registry.record_nullifier(&nullifier, &scope1).await;
        assert!(registry.is_nullifier_used(&nullifier, &scope1).await);
        assert!(!registry.is_nullifier_used(&nullifier, &scope2).await);
    }

    #[tokio::test]
    async fn test_unknown_root_rejected() {
        let registry = ZkIdentityRegistry::new();
        let fake_root = [0xff; 32];

        let result = registry
            .verify_proof(&[], &fake_root, &[0xaa; 32], &[0x01; 32], None)
            .await
            .unwrap();

        assert!(!result.valid);
        assert!(result.reason.unwrap().contains("root"));
    }

    #[tokio::test]
    async fn test_replay_rejected() {
        let registry = ZkIdentityRegistry::new();
        let root = registry.current_root().await;
        let nullifier = [0xaa; 32];
        let scope = [0x01; 32];

        let r1 = registry.verify_proof(&[], &root, &nullifier, &scope, None).await.unwrap();
        assert!(r1.valid);

        let r2 = registry.verify_proof(&[], &root, &nullifier, &scope, None).await.unwrap();
        assert!(!r2.valid);
    }

    #[tokio::test]
    async fn test_merkle_proof_generation() {
        let registry = ZkIdentityRegistry::new();
        let secret = [0x11; 32];
        let blinding = [0x22; 32];

        let identity = registry.register_identity(&secret, &blinding).await.unwrap();
        let proof = registry.get_proof(&identity.commitment).await.unwrap();

        assert_eq!(proof.len(), 20);
        let root = registry.current_root().await;
        assert!(PoseidonMerkleTree::verify_proof(&identity.commitment, &proof, &root));
    }

    #[tokio::test]
    async fn test_wrong_commitment_fails() {
        let registry = ZkIdentityRegistry::new();
        registry.register_identity(&[0x11; 32], &[0x22; 32]).await.unwrap();
        assert!(registry.get_proof(&[0xff; 32]).await.is_err());
    }
}
