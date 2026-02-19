use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IdentityCommitment {
    pub commitment: [u8; 32],
    pub index: usize,
    pub registered_at: u64,
}

#[derive(Clone, Debug)]
pub struct VerificationResult {
    pub valid: bool,
    pub reason: Option<String>,
    pub nullifier_recorded: bool,
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct ScopedNullifier {
    pub nullifier: [u8; 32],
    pub scope: [u8; 32],
}
