use nonos_crypto::{poseidon_hash, random_bytes};
use nonos_types::{NonosError, NonosResult};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZkSessionProof {
    pub proof: Vec<u8>,
    pub public_inputs: Vec<String>,
    pub session_commitment: String,
    pub created_at: u64,
    pub valid_for_secs: u64,
}

pub struct ZkSessionManager {
    identity_root: Arc<RwLock<[u8; 32]>>,
    used_nullifiers: Arc<RwLock<HashSet<String>>>,
    session_duration_secs: u64,
}

impl ZkSessionManager {
    pub fn new() -> Self {
        Self {
            identity_root: Arc::new(RwLock::new([0u8; 32])),
            used_nullifiers: Arc::new(RwLock::new(HashSet::new())),
            session_duration_secs: 3600,
        }
    }

    pub async fn create_session_proof(
        &self,
        identity_secret: &[u8; 32],
        domain: &str,
    ) -> NonosResult<ZkSessionProof> {
        let session_random = random_bytes::<32>();
        let identity_commitment = poseidon_hash(identity_secret);

        let mut nullifier_input = Vec::with_capacity(64);
        nullifier_input.extend_from_slice(identity_secret);
        nullifier_input.extend_from_slice(domain.as_bytes());
        let nullifier = poseidon_hash(&nullifier_input);

        let mut session_input = Vec::with_capacity(64);
        session_input.extend_from_slice(&identity_commitment);
        session_input.extend_from_slice(&session_random);
        let session_commitment = poseidon_hash(&session_input);

        let nullifier_hex = hex::encode(&nullifier);
        {
            let nullifiers = self.used_nullifiers.read().await;
            if nullifiers.contains(&nullifier_hex) {
                return Err(NonosError::Crypto("Nullifier already used".into()));
            }
        }

        let proof = self.generate_groth16_proof(identity_secret).await?;

        {
            let mut nullifiers = self.used_nullifiers.write().await;
            nullifiers.insert(nullifier_hex);
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Ok(ZkSessionProof {
            proof,
            public_inputs: vec![
                hex::encode(self.identity_root.read().await.as_slice()),
                hex::encode(&nullifier),
                hex::encode(&session_commitment),
            ],
            session_commitment: hex::encode(&session_commitment),
            created_at: now,
            valid_for_secs: self.session_duration_secs,
        })
    }

    pub async fn verify_session_proof(&self, proof: &ZkSessionProof) -> NonosResult<bool> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if now > proof.created_at + proof.valid_for_secs {
            return Ok(false);
        }

        self.verify_groth16_proof(proof).await
    }

    async fn generate_groth16_proof(&self, secret: &[u8; 32]) -> NonosResult<Vec<u8>> {
        use nonos_crypto::zk_proofs::generate_identity_proof;
        let blinding = random_bytes::<32>();
        let proof = generate_identity_proof(secret, &blinding)
            .map_err(|e| NonosError::Crypto(format!("Proof generation failed: {}", e)))?;
        Ok(proof.to_bytes())
    }

    async fn verify_groth16_proof(&self, proof: &ZkSessionProof) -> NonosResult<bool> {
        use nonos_crypto::zk_proofs::{verify_identity_proof, ZkIdentityProof};

        let zk_proof = ZkIdentityProof::from_bytes(&proof.proof)
            .map_err(|e| NonosError::Crypto(format!("Invalid proof format: {}", e)))?;

        verify_identity_proof(&zk_proof)
            .map_err(|e| NonosError::Crypto(format!("Proof verification failed: {}", e)))
    }

    pub async fn update_identity_root(&self, new_root: [u8; 32]) {
        *self.identity_root.write().await = new_root;
    }

    pub async fn get_identity_root(&self) -> [u8; 32] {
        *self.identity_root.read().await
    }

    pub async fn nullifier_count(&self) -> usize {
        self.used_nullifiers.read().await.len()
    }
}

impl Default for ZkSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_zk_session_creation() {
        let manager = ZkSessionManager::new();
        let secret = random_bytes::<32>();
        let proof = manager.create_session_proof(&secret, "example.com").await.unwrap();
        assert!(!proof.session_commitment.is_empty());
        assert!(proof.valid_for_secs > 0);
    }

    #[tokio::test]
    async fn test_nullifier_uniqueness() {
        let manager = ZkSessionManager::new();
        let secret = random_bytes::<32>();

        let _ = manager.create_session_proof(&secret, "example.com").await.unwrap();
        let result = manager.create_session_proof(&secret, "example.com").await;

        assert!(result.is_err());
    }
}
