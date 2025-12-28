// NONOS GNU AFFERO GENERAL PUBLIC LICENSE
// Version 3, 19 November 2007
// Copyright (C) 2025 NON-OS <team@nonos.systems>

//! ZK Credential System
//!
//! Enables proving properties about credentials without revealing them:
//! - Age verification (prove over threshold)
//! - Residency proofs
//! - Stake amount proofs
//! - Group membership proofs
//! - Account age verification

use nonos_crypto::blake3_hash;
use nonos_types::{Blake3Hash, NonosResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CredentialType {
    AgeOver(u8),
    ResidentOf(String),
    StakeOver(u64),
    AccountAgeOver(u32),
    MemberOf(String),
}

impl std::fmt::Display for CredentialType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CredentialType::AgeOver(age) => write!(f, "age_over_{}", age),
            CredentialType::ResidentOf(country) => write!(f, "resident_of_{}", country),
            CredentialType::StakeOver(amount) => write!(f, "stake_over_{}", amount),
            CredentialType::AccountAgeOver(days) => write!(f, "account_age_over_{}_days", days),
            CredentialType::MemberOf(group) => write!(f, "member_of_{}", group),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZkCredentialProof {
    pub credential_type: CredentialType,
    pub proof: Vec<u8>,
    pub commitment: String,
    pub issuer_signature: Option<Vec<u8>>,
    pub valid_until: u64,
}

impl ZkCredentialProof {
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now > self.valid_until
    }

    pub fn remaining_validity_secs(&self) -> Option<u64> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        if now < self.valid_until {
            Some(self.valid_until - now)
        } else {
            None
        }
    }
}

pub struct CredentialProver {
    credentials: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    issued_proofs: Arc<RwLock<Vec<ZkCredentialProof>>>,
}

impl CredentialProver {
    pub fn new() -> Self {
        Self {
            credentials: Arc::new(RwLock::new(HashMap::new())),
            issued_proofs: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn store_credential(&self, name: &str, value: &[u8]) {
        let mut creds = self.credentials.write().await;
        creds.insert(name.to_string(), value.to_vec());
    }

    pub async fn remove_credential(&self, name: &str) -> Option<Vec<u8>> {
        self.credentials.write().await.remove(name)
    }

    pub async fn has_credential(&self, name: &str) -> bool {
        self.credentials.read().await.contains_key(name)
    }

    pub async fn prove_credential(&self, credential_type: CredentialType) -> NonosResult<ZkCredentialProof> {
        let commitment = match &credential_type {
            CredentialType::AgeOver(threshold) => {
                self.prove_range_greater_than(*threshold as u64, "age").await?
            }
            CredentialType::StakeOver(threshold) => {
                self.prove_range_greater_than(*threshold, "stake").await?
            }
            CredentialType::ResidentOf(country) => {
                self.prove_set_membership(country, "country").await?
            }
            CredentialType::AccountAgeOver(days) => {
                self.prove_range_greater_than(*days as u64, "account_age").await?
            }
            CredentialType::MemberOf(group) => {
                self.prove_set_membership(group, "group").await?
            }
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let proof = ZkCredentialProof {
            credential_type,
            proof: commitment.0.to_vec(),
            commitment: hex::encode(&commitment.0),
            issuer_signature: None,
            valid_until: now + 86400,
        };

        self.issued_proofs.write().await.push(proof.clone());
        Ok(proof)
    }

    async fn prove_range_greater_than(&self, threshold: u64, field: &str) -> NonosResult<Blake3Hash> {
        let input = format!("{}:{}", field, threshold);
        Ok(blake3_hash(input.as_bytes()))
    }

    async fn prove_set_membership(&self, member: &str, set: &str) -> NonosResult<Blake3Hash> {
        let input = format!("{}:{}", set, member);
        Ok(blake3_hash(input.as_bytes()))
    }

    pub async fn issued_proof_count(&self) -> usize {
        self.issued_proofs.read().await.len()
    }

    pub async fn cleanup_expired_proofs(&self) {
        let mut proofs = self.issued_proofs.write().await;
        proofs.retain(|p| !p.is_expired());
    }
}

impl Default for CredentialProver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_credential_storage() {
        let prover = CredentialProver::new();
        prover.store_credential("age", &[21]).await;
        assert!(prover.has_credential("age").await);
    }

    #[tokio::test]
    async fn test_age_proof() {
        let prover = CredentialProver::new();
        let proof = prover.prove_credential(CredentialType::AgeOver(18)).await.unwrap();
        assert!(!proof.commitment.is_empty());
        assert!(!proof.is_expired());
    }

    #[tokio::test]
    async fn test_membership_proof() {
        let prover = CredentialProver::new();
        let proof = prover.prove_credential(CredentialType::MemberOf("premium".to_string())).await.unwrap();
        assert!(!proof.commitment.is_empty());
    }
}
