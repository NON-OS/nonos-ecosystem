// NONOS GNU AFFERO GENERAL PUBLIC LICENSE
// Version 3, 19 November 2007
// Copyright (C) 2025 NON-OS <team@nonos.systems>

//! Privacy Module
//!
//! Comprehensive privacy-enhancing technologies for NONØS:
//! - ZK identity proofs and session management
//! - Cache mixing with Poseidon Merkle trees
//! - Tracking and fingerprint blocking
//! - Stealth address scanning
//! - Private information retrieval (PIR)
//! - Mixnet request processing
//! - Privacy oracle scoring
//! - ZK credential proofs
//! - Distributed cookie vault

mod zk_identity;
mod cache_mixing;
mod tracking_blocker;
mod stealth;
mod manager;

// Advanced privacy modules
mod zk_sessions;
mod mixnet;
mod pir;
mod oracle;
mod stealth_sessions;
mod credentials;
mod fingerprint;
mod cookie_vault;
mod advanced;

// Core privacy services
pub use zk_identity::ZkIdentityService;
pub use cache_mixing::CacheMixingService;
pub use tracking_blocker::{TrackingBlockerService, TrackingBlockerStats};
pub use stealth::StealthScannerService;
pub use manager::{PrivacyServiceManager, PrivacyStats};

// ZK-Authenticated Sessions
pub use zk_sessions::{ZkSessionManager, ZkSessionProof};

// Multi-Node Mixnet
pub use mixnet::{MixnetProcessor, MixnetRequest};

// Private Information Retrieval
pub use pir::{PrivateContentRetrieval, CachedContent};

// Privacy Oracle Network
pub use oracle::{PrivacyOracle, DomainPrivacyScore, CookieBehavior};

// Stealth Browsing Sessions
pub use stealth_sessions::{StealthSession, StealthSessionManager};

// ZK Credentials
pub use credentials::{CredentialProver, CredentialType, ZkCredentialProof};

// Fingerprint Normalization
pub use fingerprint::{FingerprintNormalizer, NormalizedRequest};

// Distributed Cookie Vault
pub use cookie_vault::{DistributedCookieVault, SecretShare};

// Unified Advanced Privacy Manager
pub use advanced::{AdvancedPrivacyManager, AdvancedPrivacyStats};

#[cfg(test)]
mod tests;
