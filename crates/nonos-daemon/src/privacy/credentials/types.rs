use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CredentialType {
    AgeOver(u8),
    ResidentOf(String),
    StakeOver(u64),
    AccountAgeOver(u32),
    MemberOf(String),
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredCredential {
    pub credential_type: CredentialType,
    pub value: Vec<u8>,
    pub commitment: [u8; 32],
    pub salt: [u8; 32],
    pub issuer: Option<[u8; 32]>,
    pub signature: Option<Vec<u8>>,
    pub created_at: u64,
    pub expires_at: u64,
}

impl StoredCredential {
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now >= self.expires_at
    }

    pub fn remaining_validity(&self) -> u64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.expires_at.saturating_sub(now)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CredentialProof {
    pub credential_type: CredentialType,
    pub commitment: [u8; 32],
    pub proof_mac: [u8; 32],
    pub challenge: [u8; 32],
    pub created_at: u64,
    pub expires_at: u64,
    pub issuer_signature: Option<Vec<u8>>,
}

impl CredentialProof {
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now >= self.expires_at
    }
}

#[derive(Clone, Debug)]
pub struct CredentialInfo {
    pub credential_type: CredentialType,
    pub commitment: [u8; 32],
    pub has_issuer: bool,
    pub created_at: u64,
    pub expires_at: u64,
    pub remaining_validity: u64,
}
