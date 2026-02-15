use nonos_crypto::random_bytes;
use nonos_types::{NonosError, NonosResult};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::helpers::{compute_commitment, compute_proof_mac};
use super::types::{CredentialInfo, CredentialProof, CredentialType, StoredCredential};

pub struct CredentialManager {
    credentials: Arc<RwLock<HashMap<CredentialType, StoredCredential>>>,
    issued_proofs: Arc<RwLock<Vec<CredentialProof>>>,
    master_secret: [u8; 32],
}

impl CredentialManager {
    pub fn new() -> Self {
        Self {
            credentials: Arc::new(RwLock::new(HashMap::new())),
            issued_proofs: Arc::new(RwLock::new(Vec::new())),
            master_secret: random_bytes::<32>(),
        }
    }

    pub fn with_secret(master_secret: [u8; 32]) -> Self {
        Self {
            credentials: Arc::new(RwLock::new(HashMap::new())),
            issued_proofs: Arc::new(RwLock::new(Vec::new())),
            master_secret,
        }
    }

    pub async fn store_credential(
        &self,
        credential_type: CredentialType,
        value: &[u8],
        expiry_secs: u64,
    ) -> NonosResult<[u8; 32]> {
        let salt = random_bytes::<32>();
        let commitment = compute_commitment(value, &salt);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let credential = StoredCredential {
            credential_type: credential_type.clone(),
            value: value.to_vec(),
            commitment,
            salt,
            issuer: None,
            signature: None,
            created_at: now,
            expires_at: now + expiry_secs,
        };

        self.credentials.write().await.insert(credential_type, credential);
        Ok(commitment)
    }

    pub async fn store_signed_credential(
        &self,
        credential_type: CredentialType,
        value: &[u8],
        issuer: [u8; 32],
        signature: Vec<u8>,
        expiry_secs: u64,
    ) -> NonosResult<[u8; 32]> {
        let salt = random_bytes::<32>();
        let commitment = compute_commitment(value, &salt);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let credential = StoredCredential {
            credential_type: credential_type.clone(),
            value: value.to_vec(),
            commitment,
            salt,
            issuer: Some(issuer),
            signature: Some(signature),
            created_at: now,
            expires_at: now + expiry_secs,
        };

        self.credentials.write().await.insert(credential_type, credential);
        Ok(commitment)
    }

    pub async fn has_credential(&self, credential_type: &CredentialType) -> bool {
        let creds = self.credentials.read().await;
        match creds.get(credential_type) {
            Some(cred) => !cred.is_expired(),
            None => false,
        }
    }

    pub async fn remove_credential(&self, credential_type: &CredentialType) -> Option<StoredCredential> {
        self.credentials.write().await.remove(credential_type)
    }

    pub async fn create_proof(
        &self,
        credential_type: &CredentialType,
        proof_ttl_secs: u64,
    ) -> NonosResult<CredentialProof> {
        let creds = self.credentials.read().await;
        let credential = creds
            .get(credential_type)
            .ok_or_else(|| NonosError::Internal("Credential not found".into()))?;

        if credential.is_expired() {
            return Err(NonosError::Internal("Credential has expired".into()));
        }

        let challenge = random_bytes::<32>();

        let proof_mac = compute_proof_mac(
            &self.master_secret,
            &credential.salt,
            &credential.value,
            &challenge,
        );

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let proof = CredentialProof {
            credential_type: credential_type.clone(),
            commitment: credential.commitment,
            proof_mac,
            challenge,
            created_at: now,
            expires_at: now + proof_ttl_secs,
            issuer_signature: credential.signature.clone(),
        };

        drop(creds);
        self.issued_proofs.write().await.push(proof.clone());

        Ok(proof)
    }

    pub fn verify_proof(
        &self,
        proof: &CredentialProof,
        value: &[u8],
        salt: &[u8; 32],
    ) -> bool {
        if proof.is_expired() {
            return false;
        }

        let expected_commitment = compute_commitment(value, salt);
        if expected_commitment != proof.commitment {
            return false;
        }

        let expected_mac = compute_proof_mac(
            &self.master_secret,
            salt,
            value,
            &proof.challenge,
        );

        expected_mac == proof.proof_mac
    }

    pub async fn get_credential_info(&self, credential_type: &CredentialType) -> Option<CredentialInfo> {
        let creds = self.credentials.read().await;
        creds.get(credential_type).map(|c| CredentialInfo {
            credential_type: c.credential_type.clone(),
            commitment: c.commitment,
            has_issuer: c.issuer.is_some(),
            created_at: c.created_at,
            expires_at: c.expires_at,
            remaining_validity: c.remaining_validity(),
        })
    }

    pub async fn list_credentials(&self) -> Vec<CredentialType> {
        self.credentials
            .read()
            .await
            .keys()
            .cloned()
            .collect()
    }

    pub async fn issued_proof_count(&self) -> usize {
        self.issued_proofs.read().await.len()
    }

    pub async fn cleanup_expired(&self) -> (usize, usize) {
        let mut creds = self.credentials.write().await;
        let cred_before = creds.len();
        creds.retain(|_, c| !c.is_expired());
        let creds_removed = cred_before - creds.len();
        drop(creds);

        let mut proofs = self.issued_proofs.write().await;
        let proof_before = proofs.len();
        proofs.retain(|p| !p.is_expired());
        let proofs_removed = proof_before - proofs.len();

        (creds_removed, proofs_removed)
    }
}

impl Default for CredentialManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_store_and_check_credential() {
        let manager = CredentialManager::new();

        let commitment = manager
            .store_credential(CredentialType::AgeOver(18), &[25], 3600)
            .await
            .unwrap();

        assert!(manager.has_credential(&CredentialType::AgeOver(18)).await);
        assert!(!commitment.iter().all(|&b| b == 0));
    }

    #[tokio::test]
    async fn test_create_proof() {
        let manager = CredentialManager::new();

        manager
            .store_credential(CredentialType::StakeOver(1000), &1500u64.to_le_bytes(), 3600)
            .await
            .unwrap();

        let proof = manager
            .create_proof(&CredentialType::StakeOver(1000), 300)
            .await
            .unwrap();

        assert!(!proof.is_expired());
        assert!(!proof.commitment.iter().all(|&b| b == 0));
        assert!(!proof.proof_mac.iter().all(|&b| b == 0));
    }

    #[tokio::test]
    async fn test_missing_credential() {
        let manager = CredentialManager::new();

        let result = manager
            .create_proof(&CredentialType::MemberOf("premium".into()), 300)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_expired_credential() {
        let manager = CredentialManager::new();

        manager
            .store_credential(CredentialType::AgeOver(21), &[25], 0)
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        assert!(!manager.has_credential(&CredentialType::AgeOver(21)).await);

        let result = manager
            .create_proof(&CredentialType::AgeOver(21), 300)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_credentials() {
        let manager = CredentialManager::new();

        manager
            .store_credential(CredentialType::AgeOver(18), &[25], 3600)
            .await
            .unwrap();
        manager
            .store_credential(CredentialType::ResidentOf("US".into()), b"verified", 3600)
            .await
            .unwrap();

        let list = manager.list_credentials().await;
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_remove_credential() {
        let manager = CredentialManager::new();

        manager
            .store_credential(CredentialType::AgeOver(18), &[25], 3600)
            .await
            .unwrap();

        assert!(manager.has_credential(&CredentialType::AgeOver(18)).await);

        manager.remove_credential(&CredentialType::AgeOver(18)).await;

        assert!(!manager.has_credential(&CredentialType::AgeOver(18)).await);
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let manager = CredentialManager::new();

        manager
            .store_credential(CredentialType::AgeOver(18), &[25], 0)
            .await
            .unwrap();
        manager
            .store_credential(CredentialType::AgeOver(21), &[25], 3600)
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let (creds_removed, _) = manager.cleanup_expired().await;
        assert_eq!(creds_removed, 1);

        let list = manager.list_credentials().await;
        assert_eq!(list.len(), 1);
    }

    #[tokio::test]
    async fn test_signed_credential() {
        let manager = CredentialManager::new();
        let issuer = [0x11; 32];
        let signature = vec![0x22; 64];

        manager
            .store_signed_credential(
                CredentialType::MemberOf("verified".into()),
                b"member123",
                issuer,
                signature.clone(),
                3600,
            )
            .await
            .unwrap();

        let proof = manager
            .create_proof(&CredentialType::MemberOf("verified".into()), 300)
            .await
            .unwrap();

        assert_eq!(proof.issuer_signature, Some(signature));
    }
}
