// NONOS GNU AFFERO GENERAL PUBLIC LICENSE
// Version 3, 19 November 2007
// Copyright (C) 2025 NON-OS <team@nonos.systems>

//! Credential Commitment System
//!
//! Provides credential storage with Poseidon commitments:
//! - Commitment-based credential representation
//! - HMAC-based proofs of possession
//! - Issuer signature support
//! - TTL-based expiration
//!
//! NOTE: This is NOT a zero-knowledge proof system.
//! Commitments hide values but proofs reveal the claim being made.
//! For actual ZK proofs, use arkworks circuits or Bulletproofs.

use nonos_crypto::{blake3_derive_key, poseidon_hash, random_bytes};
use nonos_types::{NonosError, NonosResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Types of credentials that can be committed
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CredentialType {
    /// Age threshold (value = birth year, threshold = minimum age)
    AgeOver(u8),
    /// Residency claim (country code)
    ResidentOf(String),
    /// Stake amount threshold (value = amount, threshold = minimum)
    StakeOver(u64),
    /// Account age threshold (days)
    AccountAgeOver(u32),
    /// Group membership claim
    MemberOf(String),
    /// Custom attribute
    Custom(String),
}

impl std::fmt::Display for CredentialType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CredentialType::AgeOver(age) => write!(f, "age_over_{}", age),
            CredentialType::ResidentOf(country) => write!(f, "resident_of_{}", country),
            CredentialType::StakeOver(amount) => write!(f, "stake_over_{}", amount),
            CredentialType::AccountAgeOver(days) => write!(f, "account_age_over_{}_days", days),
            CredentialType::MemberOf(group) => write!(f, "member_of_{}", group),
            CredentialType::Custom(name) => write!(f, "custom_{}", name),
        }
    }
}

/// Stored credential with value and metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredCredential {
    /// Credential type
    pub credential_type: CredentialType,
    /// Actual value (private)
    pub value: Vec<u8>,
    /// Poseidon commitment to the value
    pub commitment: [u8; 32],
    /// Salt used in commitment (for opening proofs)
    pub salt: [u8; 32],
    /// Issuer public key (if signed by authority)
    pub issuer: Option<[u8; 32]>,
    /// Issuer signature (if signed)
    pub signature: Option<Vec<u8>>,
    /// Creation timestamp
    pub created_at: u64,
    /// Expiration timestamp
    pub expires_at: u64,
}

impl StoredCredential {
    /// Check if credential has expired
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now >= self.expires_at
    }

    /// Get remaining validity in seconds
    pub fn remaining_validity(&self) -> u64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if now >= self.expires_at {
            0
        } else {
            self.expires_at - now
        }
    }
}

/// Proof of credential possession
///
/// NOT a zero-knowledge proof - verifier learns the credential type and commitment.
/// The proof demonstrates that the prover knows the preimage of the commitment.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CredentialProof {
    /// Type of credential being proven
    pub credential_type: CredentialType,
    /// Commitment to the value
    pub commitment: [u8; 32],
    /// HMAC proof of knowledge (keyed by challenge)
    pub proof_mac: [u8; 32],
    /// Random challenge used in proof
    pub challenge: [u8; 32],
    /// Proof creation timestamp
    pub created_at: u64,
    /// Proof expiration
    pub expires_at: u64,
    /// Issuer signature (if credential was signed)
    pub issuer_signature: Option<Vec<u8>>,
}

impl CredentialProof {
    /// Check if proof has expired
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now >= self.expires_at
    }
}

/// Credential manager for storing and proving credentials
pub struct CredentialManager {
    /// Stored credentials by type
    credentials: Arc<RwLock<HashMap<CredentialType, StoredCredential>>>,
    /// Issued proofs (for tracking)
    issued_proofs: Arc<RwLock<Vec<CredentialProof>>>,
    /// Master secret for proof generation
    master_secret: [u8; 32],
}

impl CredentialManager {
    /// Create new credential manager with random master secret
    pub fn new() -> Self {
        Self {
            credentials: Arc::new(RwLock::new(HashMap::new())),
            issued_proofs: Arc::new(RwLock::new(Vec::new())),
            master_secret: random_bytes::<32>(),
        }
    }

    /// Create with specific master secret (for persistence)
    pub fn with_secret(master_secret: [u8; 32]) -> Self {
        Self {
            credentials: Arc::new(RwLock::new(HashMap::new())),
            issued_proofs: Arc::new(RwLock::new(Vec::new())),
            master_secret,
        }
    }

    /// Store a new credential
    ///
    /// Value is committed using Poseidon hash with random salt.
    /// Expiry is in seconds from now.
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

    /// Store a signed credential from an issuer
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

    /// Check if a credential exists and is valid
    pub async fn has_credential(&self, credential_type: &CredentialType) -> bool {
        let creds = self.credentials.read().await;
        match creds.get(credential_type) {
            Some(cred) => !cred.is_expired(),
            None => false,
        }
    }

    /// Remove a credential
    pub async fn remove_credential(&self, credential_type: &CredentialType) -> Option<StoredCredential> {
        self.credentials.write().await.remove(credential_type)
    }

    /// Generate a proof of credential possession
    ///
    /// Creates an HMAC-based proof showing knowledge of the commitment preimage.
    /// This is NOT a zero-knowledge proof - the verifier learns the credential type.
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

        // Generate random challenge
        let challenge = random_bytes::<32>();

        // Create HMAC proof: HMAC(master_secret || salt || value, challenge)
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

    /// Verify a proof (requires knowing the credential value)
    ///
    /// This verification requires the actual value, so it can only be done
    /// by the credential holder or someone they share the value with.
    pub fn verify_proof(
        &self,
        proof: &CredentialProof,
        value: &[u8],
        salt: &[u8; 32],
    ) -> bool {
        if proof.is_expired() {
            return false;
        }

        // Verify commitment
        let expected_commitment = compute_commitment(value, salt);
        if expected_commitment != proof.commitment {
            return false;
        }

        // Verify HMAC
        let expected_mac = compute_proof_mac(
            &self.master_secret,
            salt,
            value,
            &proof.challenge,
        );

        expected_mac == proof.proof_mac
    }

    /// Get credential metadata (without value)
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

    /// List all credential types held
    pub async fn list_credentials(&self) -> Vec<CredentialType> {
        self.credentials
            .read()
            .await
            .keys()
            .cloned()
            .collect()
    }

    /// Get number of issued proofs
    pub async fn issued_proof_count(&self) -> usize {
        self.issued_proofs.read().await.len()
    }

    /// Remove expired credentials and proofs
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

/// Credential metadata (without private value)
#[derive(Clone, Debug)]
pub struct CredentialInfo {
    pub credential_type: CredentialType,
    pub commitment: [u8; 32],
    pub has_issuer: bool,
    pub created_at: u64,
    pub expires_at: u64,
    pub remaining_validity: u64,
}

/// Compute Poseidon commitment: H(value || salt)
fn compute_commitment(value: &[u8], salt: &[u8; 32]) -> [u8; 32] {
    let mut input = Vec::with_capacity(value.len() + 32);
    input.extend_from_slice(value);
    input.extend_from_slice(salt);
    poseidon_hash(&input)
}

/// Compute HMAC-based proof of knowledge
fn compute_proof_mac(
    master_secret: &[u8; 32],
    salt: &[u8; 32],
    value: &[u8],
    challenge: &[u8; 32],
) -> [u8; 32] {
    // Derive key from master secret and challenge
    let mut key_input = Vec::with_capacity(64);
    key_input.extend_from_slice(master_secret);
    key_input.extend_from_slice(challenge);
    let key = blake3_derive_key("nonos-credential-proof", &key_input);

    // HMAC over salt || value
    let mut mac_input = Vec::with_capacity(32 + value.len());
    mac_input.extend_from_slice(salt);
    mac_input.extend_from_slice(value);

    // Use keyed BLAKE3 for MAC
    blake3_derive_key("nonos-credential-mac", &[&key.0[..], &mac_input[..]].concat()).0
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

        // Store with 0 TTL (immediately expired)
        manager
            .store_credential(CredentialType::AgeOver(21), &[25], 0)
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Should not be considered valid
        assert!(!manager.has_credential(&CredentialType::AgeOver(21)).await);

        // Should fail to create proof
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

    #[test]
    fn test_commitment_determinism() {
        let value = b"test value";
        let salt = [0xab; 32];

        let c1 = compute_commitment(value, &salt);
        let c2 = compute_commitment(value, &salt);

        assert_eq!(c1, c2);
    }

    #[test]
    fn test_different_salt_different_commitment() {
        let value = b"test value";
        let salt1 = [0xab; 32];
        let salt2 = [0xcd; 32];

        let c1 = compute_commitment(value, &salt1);
        let c2 = compute_commitment(value, &salt2);

        assert_ne!(c1, c2);
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let manager = CredentialManager::new();

        // Store expired and valid credentials
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
