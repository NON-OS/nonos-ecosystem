// NONOS GNU AFFERO GENERAL PUBLIC LICENSE
// Version 3, 19 November 2007
// Copyright (C) 2025 NON-OS <team@nonos.systems>

//! Commitment-Based Content Cache
//!
//! Provides content caching with commitment-based indexing:
//! - Content indexed by Poseidon commitment
//! - AES-256-GCM encryption at rest
//! - TTL-based expiration
//!
//! NOTE: This is NOT Private Information Retrieval (PIR).
//! PIR requires homomorphic encryption or multi-server protocols.
//! This module provides commitment-based lookup with encryption.

use nonos_crypto::{blake3_hash, random_bytes};
use nonos_types::{NonosError, NonosResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// AES-GCM nonce size
const NONCE_SIZE: usize = 12;
/// AES-GCM tag size
const TAG_SIZE: usize = 16;

/// Cached content with encryption and metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CachedContent {
    /// Poseidon commitment of plaintext content
    pub commitment: [u8; 32],
    /// Encrypted content (key_header || nonce || ciphertext || tag)
    pub encrypted_data: Vec<u8>,
    /// BLAKE3 hash of plaintext for integrity verification
    pub content_hash: [u8; 32],
    /// Time-to-live in seconds
    pub ttl_secs: u64,
    /// Unix timestamp when stored
    pub created_at: u64,
}

impl CachedContent {
    /// Check if content has expired
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.created_at + self.ttl_secs <= now
    }

    /// Get remaining TTL in seconds (0 if expired)
    pub fn remaining_ttl(&self) -> u64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let expiry = self.created_at + self.ttl_secs;
        if now >= expiry {
            0
        } else {
            expiry - now
        }
    }
}

/// Commitment-based content retrieval system
///
/// Stores content indexed by Poseidon commitment.
/// Content is encrypted at rest with per-item keys.
pub struct PrivateContentRetrieval {
    /// In-memory cache: commitment -> encrypted content
    cache: Arc<RwLock<HashMap<[u8; 32], CachedContent>>>,
    /// Maximum number of items in cache
    max_cache_size: usize,
    /// Master key for key derivation
    master_key: [u8; 32],
}

impl PrivateContentRetrieval {
    /// Create a new content cache with random master key
    pub fn new(max_cache_size: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            max_cache_size,
            master_key: random_bytes::<32>(),
        }
    }

    /// Create with a specific master key (for persistence)
    pub fn with_master_key(max_cache_size: usize, master_key: [u8; 32]) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            max_cache_size,
            master_key,
        }
    }

    /// Store content and return its commitment
    pub async fn store(&self, content: &[u8], ttl_secs: u64) -> NonosResult<[u8; 32]> {
        // Use BLAKE3 for commitment - it's fast and collision-resistant
        let commitment = blake3_hash(content).0;
        let content_hash = commitment; // Same hash, no need to compute twice

        // Derive a unique key for this content from master key + commitment
        let item_key = self.derive_item_key(&commitment);
        let encrypted = encrypt_content(content, &item_key)?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let cached = CachedContent {
            commitment,
            encrypted_data: encrypted,
            content_hash,
            ttl_secs,
            created_at: now,
        };

        let mut cache = self.cache.write().await;

        // Evict if at capacity
        while cache.len() >= self.max_cache_size {
            self.evict_oldest(&mut cache);
        }

        cache.insert(commitment, cached);
        Ok(commitment)
    }

    /// Retrieve content by commitment
    pub async fn retrieve(&self, commitment: &[u8; 32]) -> NonosResult<Option<Vec<u8>>> {
        let cache = self.cache.read().await;

        if let Some(cached) = cache.get(commitment) {
            if cached.is_expired() {
                return Ok(None);
            }

            let item_key = self.derive_item_key(commitment);
            let decrypted = decrypt_content(&cached.encrypted_data, &item_key)?;

            // Verify content hash
            let hash = blake3_hash(&decrypted);
            if hash.0 != cached.content_hash {
                return Err(NonosError::Crypto("Content hash mismatch".into()));
            }

            Ok(Some(decrypted))
        } else {
            Ok(None)
        }
    }

    /// Get metadata without decrypting
    pub async fn get_metadata(&self, commitment: &[u8; 32]) -> Option<ContentMetadata> {
        let cache = self.cache.read().await;
        cache.get(commitment).map(|c| ContentMetadata {
            commitment: c.commitment,
            content_hash: c.content_hash,
            size: c.encrypted_data.len().saturating_sub(NONCE_SIZE + TAG_SIZE),
            ttl_secs: c.ttl_secs,
            created_at: c.created_at,
            remaining_ttl: c.remaining_ttl(),
        })
    }

    /// Check if content exists (without loading)
    pub async fn exists(&self, commitment: &[u8; 32]) -> bool {
        let cache = self.cache.read().await;
        match cache.get(commitment) {
            Some(cached) => !cached.is_expired(),
            None => false,
        }
    }

    /// Remove content by commitment
    pub async fn remove(&self, commitment: &[u8; 32]) -> Option<CachedContent> {
        self.cache.write().await.remove(commitment)
    }

    /// Remove all expired content
    pub async fn cleanup_expired(&self) -> usize {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut cache = self.cache.write().await;
        let before = cache.len();
        cache.retain(|_, content| content.created_at + content.ttl_secs > now);
        before - cache.len()
    }

    /// Get current cache size
    pub async fn cache_size(&self) -> usize {
        self.cache.read().await.len()
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut total_bytes = 0;
        let mut expired_count = 0;

        for content in cache.values() {
            total_bytes += content.encrypted_data.len();
            if content.created_at + content.ttl_secs <= now {
                expired_count += 1;
            }
        }

        CacheStats {
            item_count: cache.len(),
            total_bytes,
            expired_count,
            max_size: self.max_cache_size,
        }
    }

    /// List all commitments in cache
    pub async fn list_commitments(&self) -> Vec<[u8; 32]> {
        self.cache.read().await.keys().copied().collect()
    }

    /// Derive item-specific key from master key and commitment
    fn derive_item_key(&self, commitment: &[u8; 32]) -> [u8; 32] {
        use nonos_crypto::blake3_derive_key;
        let mut input = Vec::with_capacity(64);
        input.extend_from_slice(&self.master_key);
        input.extend_from_slice(commitment);
        blake3_derive_key("nonos-content-cache-item-key", &input).0
    }

    /// Evict oldest item from cache
    fn evict_oldest(&self, cache: &mut HashMap<[u8; 32], CachedContent>) {
        // First try to evict expired items
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let expired: Vec<_> = cache
            .iter()
            .filter(|(_, v)| v.created_at + v.ttl_secs <= now)
            .map(|(k, _)| *k)
            .collect();

        if !expired.is_empty() {
            cache.remove(&expired[0]);
            return;
        }

        // Otherwise evict oldest by creation time
        if let Some(oldest_key) = cache
            .iter()
            .min_by_key(|(_, v)| v.created_at)
            .map(|(k, _)| *k)
        {
            cache.remove(&oldest_key);
        }
    }
}

/// Content metadata (without encrypted payload)
#[derive(Clone, Debug)]
pub struct ContentMetadata {
    pub commitment: [u8; 32],
    pub content_hash: [u8; 32],
    pub size: usize,
    pub ttl_secs: u64,
    pub created_at: u64,
    pub remaining_ttl: u64,
}

/// Cache statistics
#[derive(Clone, Debug)]
pub struct CacheStats {
    pub item_count: usize,
    pub total_bytes: usize,
    pub expired_count: usize,
    pub max_size: usize,
}

/// Encrypt content with AES-256-GCM
fn encrypt_content(content: &[u8], key: &[u8; 32]) -> NonosResult<Vec<u8>> {
    use aes_gcm::{
        aead::{Aead, KeyInit},
        Aes256Gcm, Nonce,
    };

    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|e| NonosError::Crypto(e.to_string()))?;

    let nonce_bytes = random_bytes::<NONCE_SIZE>();
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, content)
        .map_err(|e| NonosError::Crypto(format!("Encryption failed: {}", e)))?;

    // Format: nonce || ciphertext (tag is appended by aes-gcm)
    let mut result = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);
    Ok(result)
}

/// Decrypt content with AES-256-GCM
fn decrypt_content(encrypted: &[u8], key: &[u8; 32]) -> NonosResult<Vec<u8>> {
    use aes_gcm::{
        aead::{Aead, KeyInit},
        Aes256Gcm, Nonce,
    };

    if encrypted.len() < NONCE_SIZE + TAG_SIZE {
        return Err(NonosError::Crypto("Encrypted data too short".into()));
    }

    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|e| NonosError::Crypto(e.to_string()))?;

    let nonce = Nonce::from_slice(&encrypted[..NONCE_SIZE]);
    let ciphertext = &encrypted[NONCE_SIZE..];

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| NonosError::Crypto(format!("Decryption failed: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let cache = PrivateContentRetrieval::new(100);
        let content = b"test content for caching";

        let commitment = cache.store(content, 3600).await.unwrap();
        let retrieved = cache.retrieve(&commitment).await.unwrap();

        assert_eq!(retrieved.as_deref(), Some(content.as_slice()));
    }

    #[tokio::test]
    async fn test_missing_content() {
        let cache = PrivateContentRetrieval::new(100);
        let fake_commitment = [0xab; 32];

        let result = cache.retrieve(&fake_commitment).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_expiration() {
        let cache = PrivateContentRetrieval::new(100);
        let content = b"expiring content";

        // Store with 0 TTL (immediately expired)
        let commitment = cache.store(content, 0).await.unwrap();

        // Wait a moment to ensure expiration
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let result = cache.retrieve(&commitment).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cache_eviction() {
        let cache = PrivateContentRetrieval::new(3);

        // Fill cache
        let c1 = cache.store(b"content1", 3600).await.unwrap();
        let c2 = cache.store(b"content2", 3600).await.unwrap();
        let c3 = cache.store(b"content3", 3600).await.unwrap();

        assert_eq!(cache.cache_size().await, 3);

        // Add one more - should evict one item to make room
        let c4 = cache.store(b"content4", 3600).await.unwrap();

        assert_eq!(cache.cache_size().await, 3);

        // c4 should definitely exist (just added)
        assert!(cache.retrieve(&c4).await.unwrap().is_some());

        // Count how many of c1, c2, c3 still exist - should be 2
        let remaining = [
            cache.retrieve(&c1).await.unwrap().is_some(),
            cache.retrieve(&c2).await.unwrap().is_some(),
            cache.retrieve(&c3).await.unwrap().is_some(),
        ].iter().filter(|&&x| x).count();
        assert_eq!(remaining, 2, "Expected 2 of the original 3 items to remain");
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let cache = PrivateContentRetrieval::new(100);

        // Store some with 0 TTL
        cache.store(b"expired1", 0).await.unwrap();
        cache.store(b"expired2", 0).await.unwrap();
        cache.store(b"valid", 3600).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let removed = cache.cleanup_expired().await;
        assert_eq!(removed, 2);
        assert_eq!(cache.cache_size().await, 1);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let content = b"secret content to encrypt";
        let key = random_bytes::<32>();

        let encrypted = encrypt_content(content, &key).unwrap();
        assert!(encrypted.len() > content.len()); // Should be larger due to nonce + tag

        let decrypted = decrypt_content(&encrypted, &key).unwrap();
        assert_eq!(decrypted, content);
    }

    #[test]
    fn test_wrong_key_fails() {
        let content = b"secret content";
        let key1 = random_bytes::<32>();
        let key2 = random_bytes::<32>();

        let encrypted = encrypt_content(content, &key1).unwrap();
        let result = decrypt_content(&encrypted, &key2);

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_exists() {
        let cache = PrivateContentRetrieval::new(100);
        let content = b"test content";

        let commitment = cache.store(content, 3600).await.unwrap();
        assert!(cache.exists(&commitment).await);

        let fake = [0xff; 32];
        assert!(!cache.exists(&fake).await);
    }

    #[tokio::test]
    async fn test_stats() {
        let cache = PrivateContentRetrieval::new(100);

        cache.store(b"content1", 3600).await.unwrap();
        cache.store(b"content2", 3600).await.unwrap();

        let stats = cache.stats().await;
        assert_eq!(stats.item_count, 2);
        assert!(stats.total_bytes > 0);
        assert_eq!(stats.expired_count, 0);
        assert_eq!(stats.max_size, 100);
    }

    #[tokio::test]
    async fn test_deterministic_commitment() {
        let cache = PrivateContentRetrieval::new(100);
        let content = b"deterministic content";

        let c1 = cache.store(content, 3600).await.unwrap();
        cache.remove(&c1).await;
        let c2 = cache.store(content, 3600).await.unwrap();

        // Same content should produce same commitment
        assert_eq!(c1, c2);
    }
}
