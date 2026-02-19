mod identity;
mod mixer;
mod zk_identity;
mod cache_mixing;
mod tracking_blocker;
mod stealth;
mod manager;
mod zk_sessions;
mod mixnet;
mod pir;
mod oracle;
mod stealth_sessions;
mod credentials;
mod fingerprint;
mod cookie_vault;
mod advanced;
mod zk_credentials;

pub use identity::{ZkIdentityRegistry, IdentityCommitment, VerificationResult};
pub use mixer::{NoteMixer, Note, SpendRequest, SpendResult, AssetId, ASSET_ETH, ASSET_NOX};
pub use zk_identity::ZkIdentityService;
pub use cache_mixing::CacheMixingService;
pub use tracking_blocker::{TrackingBlockerService, TrackingBlockerStats};
pub use stealth::StealthScannerService;
pub use manager::{PrivacyServiceManager, PrivacyStats};
pub use zk_sessions::{ZkSessionManager, ZkSessionProof};
pub use mixnet::{MixnetProcessor, MixnetKeypair, MixNode, OnionPacket, DecryptedLayer, PooledRequest, build_onion_packet, decrypt_onion_layer};
pub use pir::{PrivateContentRetrieval, CachedContent, ContentMetadata, CacheStats};
pub use oracle::{PrivacyOracle, DomainPrivacyScore, CookieBehavior};
pub use stealth_sessions::{StealthSession, StealthSessionManager};
pub use credentials::{CredentialManager, CredentialType, CredentialProof, StoredCredential, CredentialInfo};
pub use fingerprint::{FingerprintNormalizer, NormalizedRequest};
pub use cookie_vault::{DistributedCookieVault, SecretShare};
pub use advanced::{AdvancedPrivacyManager, AdvancedPrivacyStats};
pub use zk_credentials::{ZkCredentialSystem, ZkCredential, ZkCredentialType, ZkCredentialProof, ZkPublicInputs, MerkleProof, MERKLE_DEPTH};

#[cfg(test)]
mod tests;
