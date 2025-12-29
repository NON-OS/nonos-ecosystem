// NONOS GNU AFFERO GENERAL PUBLIC LICENSE
// Version 3, 19 November 2007
// Copyright (C) 2025 NON-OS <team@nonos.systems>

//! Onion-Routed Mixnet
//! - X25519 for ephemeral key agreement
//! - AES-256-GCM for authenticated encryption per layer
//! - Pool-based mixing with timing jitter
//! - Forward secrecy via ephemeral keys

use nonos_crypto::{blake3_derive_key, random_bytes};
use nonos_types::{Blake3Key, NonosError, NonosResult, NodeId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// X25519 public key size
const X25519_PUBLIC_KEY_SIZE: usize = 32;
/// AES-GCM nonce size
const NONCE_SIZE: usize = 12;
/// AES-GCM tag size
const TAG_SIZE: usize = 16;
/// Maximum payload size per packet
const MAX_PAYLOAD_SIZE: usize = 8192;
/// Routing info size per hop (next_node + flags)
const ROUTING_INFO_SIZE: usize = 40;

/// Node's X25519 keypair for onion decryption
#[derive(Clone)]
pub struct MixnetKeypair {
    secret: [u8; 32],
    public: [u8; 32],
}

impl MixnetKeypair {
    /// Generate a new random keypair
    pub fn generate() -> Self {
        // Generate using x25519-dalek's secure random
        let secret_bytes = random_bytes::<32>();
        let clamped = clamp_scalar(&secret_bytes);
        let public = x25519_base_multiply(&clamped);

        Self {
            secret: clamped,
            public,
        }
    }

    /// Create from existing secret key bytes
    pub fn from_secret(secret: [u8; 32]) -> Self {
        let clamped = clamp_scalar(&secret);
        let public = x25519_base_multiply(&clamped);
        Self {
            secret: clamped,
            public,
        }
    }

    /// Get public key bytes
    pub fn public_key(&self) -> &[u8; 32] {
        &self.public
    }

    /// Perform X25519 key exchange
    pub fn diffie_hellman(&self, their_public: &[u8; 32]) -> [u8; 32] {
        x25519_multiply(&self.secret, their_public)
    }
}

/// Clamp a scalar for X25519 (RFC 7748)
fn clamp_scalar(bytes: &[u8; 32]) -> [u8; 32] {
    let mut clamped = *bytes;
    clamped[0] &= 248;
    clamped[31] &= 127;
    clamped[31] |= 64;
    clamped
}

/// X25519 base point multiplication (compute public key from secret)
fn x25519_base_multiply(secret: &[u8; 32]) -> [u8; 32] {
    use x25519_dalek::{StaticSecret, PublicKey};
    let static_secret = StaticSecret::from(*secret);
    let public = PublicKey::from(&static_secret);
    *public.as_bytes()
}

/// X25519 point multiplication (compute shared secret)
fn x25519_multiply(secret: &[u8; 32], their_public: &[u8; 32]) -> [u8; 32] {
    use x25519_dalek::{StaticSecret, PublicKey};
    let static_secret = StaticSecret::from(*secret);
    let their_public = PublicKey::from(*their_public);
    let shared = static_secret.diffie_hellman(&their_public);
    *shared.as_bytes()
}

/// Registered node with its public key
#[derive(Clone, Debug)]
pub struct MixNode {
    pub node_id: NodeId,
    pub public_key: [u8; 32],
    pub address: String,
}

/// Onion-encrypted packet
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OnionPacket {
    /// Ephemeral public key for this layer
    pub ephemeral_public: [u8; X25519_PUBLIC_KEY_SIZE],
    /// Encrypted routing info + nested packet
    pub encrypted_payload: Vec<u8>,
    /// Unique request ID for tracking
    pub request_id: [u8; 16],
}

impl OnionPacket {
    /// Check if packet structure is valid
    pub fn is_valid(&self) -> bool {
        self.encrypted_payload.len() >= NONCE_SIZE + TAG_SIZE + ROUTING_INFO_SIZE
    }
}

/// Result of decrypting one onion layer
#[derive(Debug)]
pub struct DecryptedLayer {
    /// Next node to forward to (None if we're the exit)
    pub next_hop: Option<NodeId>,
    /// Remaining packet for next hop (or final payload if exit)
    pub payload: Vec<u8>,
    /// Whether this is the final destination
    pub is_exit: bool,
}

/// Routing info header in each layer
#[derive(Clone, Debug, Serialize, Deserialize)]
struct RoutingInfo {
    next_node: [u8; 32],
    flags: u8, // bit 0: is_exit
    reserved: [u8; 7],
}

impl RoutingInfo {
    fn exit_node() -> Self {
        Self {
            next_node: [0u8; 32],
            flags: 0x01,
            reserved: [0u8; 7],
        }
    }

    fn relay(next: &NodeId) -> Self {
        Self {
            next_node: next.0,
            flags: 0x00,
            reserved: [0u8; 7],
        }
    }

    fn is_exit(&self) -> bool {
        self.flags & 0x01 != 0
    }

    fn to_bytes(&self) -> [u8; ROUTING_INFO_SIZE] {
        let mut bytes = [0u8; ROUTING_INFO_SIZE];
        bytes[..32].copy_from_slice(&self.next_node);
        bytes[32] = self.flags;
        bytes[33..40].copy_from_slice(&self.reserved);
        bytes
    }

    fn from_bytes(bytes: &[u8; ROUTING_INFO_SIZE]) -> Self {
        let mut next_node = [0u8; 32];
        next_node.copy_from_slice(&bytes[..32]);
        let mut reserved = [0u8; 7];
        reserved.copy_from_slice(&bytes[33..40]);
        Self {
            next_node,
            flags: bytes[32],
            reserved,
        }
    }
}

/// Derive encryption key from shared secret and context
fn derive_encryption_key(shared_secret: &[u8; 32], context: &[u8]) -> Blake3Key {
    let mut input = Vec::with_capacity(32 + context.len());
    input.extend_from_slice(shared_secret);
    input.extend_from_slice(context);
    blake3_derive_key("nonos-mixnet-layer-key", &input)
}

/// AES-256-GCM encryption
fn aes_gcm_encrypt(key: &Blake3Key, plaintext: &[u8], aad: &[u8]) -> NonosResult<Vec<u8>> {
    use aes_gcm::{
        aead::{Aead, KeyInit, Payload},
        Aes256Gcm, Nonce,
    };

    let cipher = Aes256Gcm::new_from_slice(&key.0)
        .map_err(|e| NonosError::Crypto(e.to_string()))?;

    let nonce_bytes = random_bytes::<NONCE_SIZE>();
    let nonce = Nonce::from_slice(&nonce_bytes);

    let payload = Payload {
        msg: plaintext,
        aad,
    };

    let ciphertext = cipher
        .encrypt(nonce, payload)
        .map_err(|e| NonosError::Crypto(format!("Encryption failed: {}", e)))?;

    // nonce || ciphertext || tag (tag is appended by aes-gcm)
    let mut result = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);
    Ok(result)
}

/// AES-256-GCM decryption
fn aes_gcm_decrypt(key: &Blake3Key, encrypted: &[u8], aad: &[u8]) -> NonosResult<Vec<u8>> {
    use aes_gcm::{
        aead::{Aead, KeyInit, Payload},
        Aes256Gcm, Nonce,
    };

    if encrypted.len() < NONCE_SIZE + TAG_SIZE {
        return Err(NonosError::Crypto("Encrypted data too short".into()));
    }

    let cipher = Aes256Gcm::new_from_slice(&key.0)
        .map_err(|e| NonosError::Crypto(e.to_string()))?;

    let nonce = Nonce::from_slice(&encrypted[..NONCE_SIZE]);
    let ciphertext = &encrypted[NONCE_SIZE..];

    let payload = Payload {
        msg: ciphertext,
        aad,
    };

    cipher
        .decrypt(nonce, payload)
        .map_err(|e| NonosError::Crypto(format!("Decryption failed: {}", e)))
}

/// Build an onion packet for a given path
///
/// The path is a list of (NodeId, public_key) pairs from entry to exit.
/// Payload is encrypted in layers, innermost first (reverse order).
pub fn build_onion_packet(
    payload: &[u8],
    path: &[(NodeId, [u8; 32])],
) -> NonosResult<OnionPacket> {
    if path.is_empty() {
        return Err(NonosError::Crypto("Path cannot be empty".into()));
    }
    if payload.len() > MAX_PAYLOAD_SIZE {
        return Err(NonosError::Crypto("Payload too large".into()));
    }

    let request_id = random_bytes::<16>();

    // Start with the payload for the exit node
    let mut current_payload = payload.to_vec();
    let mut current_ephemeral: Option<[u8; 32]> = None;

    // Build layers from exit to entry (reverse)
    for (i, (_node_id, their_public)) in path.iter().enumerate().rev() {
        // Generate ephemeral keypair for this layer
        let ephemeral = MixnetKeypair::generate();

        // Compute shared secret
        let shared = ephemeral.diffie_hellman(their_public);

        // Derive encryption key
        let layer_key = derive_encryption_key(&shared, &request_id);

        // Build routing info
        let routing = if i == path.len() - 1 {
            // This is the exit node
            RoutingInfo::exit_node()
        } else {
            // This is a relay, next hop is path[i+1]
            RoutingInfo::relay(&path[i + 1].0)
        };

        // Combine routing info + [inner ephemeral key if relay] + inner payload
        let mut layer_plaintext = Vec::with_capacity(
            ROUTING_INFO_SIZE + X25519_PUBLIC_KEY_SIZE + current_payload.len(),
        );
        layer_plaintext.extend_from_slice(&routing.to_bytes());
        // Include the inner layer's ephemeral key (if there is one)
        if let Some(inner_ephemeral) = current_ephemeral {
            layer_plaintext.extend_from_slice(&inner_ephemeral);
        }
        layer_plaintext.extend_from_slice(&current_payload);

        // Encrypt this layer (AAD is the ephemeral public key)
        let encrypted = aes_gcm_encrypt(&layer_key, &layer_plaintext, ephemeral.public_key())?;

        current_payload = encrypted;
        current_ephemeral = Some(*ephemeral.public_key());
    }

    Ok(OnionPacket {
        ephemeral_public: current_ephemeral.expect("path not empty"),
        encrypted_payload: current_payload,
        request_id,
    })
}

/// Decrypt one layer of an onion packet
pub fn decrypt_onion_layer(
    packet: &OnionPacket,
    our_keypair: &MixnetKeypair,
) -> NonosResult<DecryptedLayer> {
    // Compute shared secret
    let shared = our_keypair.diffie_hellman(&packet.ephemeral_public);

    // Derive decryption key
    let layer_key = derive_encryption_key(&shared, &packet.request_id);

    // Decrypt (AAD is the ephemeral public key)
    let decrypted = aes_gcm_decrypt(&layer_key, &packet.encrypted_payload, &packet.ephemeral_public)?;

    if decrypted.len() < ROUTING_INFO_SIZE {
        return Err(NonosError::Crypto("Decrypted layer too short".into()));
    }

    // Extract routing info
    let mut routing_bytes = [0u8; ROUTING_INFO_SIZE];
    routing_bytes.copy_from_slice(&decrypted[..ROUTING_INFO_SIZE]);
    let routing = RoutingInfo::from_bytes(&routing_bytes);

    let payload = decrypted[ROUTING_INFO_SIZE..].to_vec();

    if routing.is_exit() {
        Ok(DecryptedLayer {
            next_hop: None,
            payload,
            is_exit: true,
        })
    } else {
        Ok(DecryptedLayer {
            next_hop: Some(NodeId::from_bytes(routing.next_node)),
            payload,
            is_exit: false,
        })
    }
}

/// Request waiting in the mixing pool
#[derive(Clone)]
pub struct PooledRequest {
    pub packet: OnionPacket,
    pub arrival_time: std::time::Instant,
    pub jitter_ms: u64,
}

/// Mixnet processor with pool-based mixing
pub struct MixnetProcessor {
    /// Our keypair for decrypting incoming packets
    keypair: MixnetKeypair,
    /// Request pool for timing mixing
    request_pool: Arc<RwLock<Vec<PooledRequest>>>,
    /// Minimum pool size before flushing
    min_pool_size: usize,
    /// Maximum delay in milliseconds
    max_delay_ms: u64,
    /// Known mix nodes (node_id -> MixNode)
    known_nodes: Arc<RwLock<HashMap<NodeId, MixNode>>>,
    /// Callback for forwarding packets
    forward_callback: Arc<RwLock<Option<Box<dyn Fn(NodeId, OnionPacket) + Send + Sync>>>>,
    /// Callback for final payloads
    exit_callback: Arc<RwLock<Option<Box<dyn Fn(Vec<u8>) + Send + Sync>>>>,
}

impl MixnetProcessor {
    /// Create new processor with a fresh keypair
    pub fn new() -> Self {
        Self {
            keypair: MixnetKeypair::generate(),
            request_pool: Arc::new(RwLock::new(Vec::new())),
            min_pool_size: 5,
            max_delay_ms: 500,
            known_nodes: Arc::new(RwLock::new(HashMap::new())),
            forward_callback: Arc::new(RwLock::new(None)),
            exit_callback: Arc::new(RwLock::new(None)),
        }
    }

    /// Create with existing keypair and config
    pub fn with_config(
        keypair: MixnetKeypair,
        min_pool_size: usize,
        max_delay_ms: u64,
    ) -> Self {
        Self {
            keypair,
            request_pool: Arc::new(RwLock::new(Vec::new())),
            min_pool_size,
            max_delay_ms,
            known_nodes: Arc::new(RwLock::new(HashMap::new())),
            forward_callback: Arc::new(RwLock::new(None)),
            exit_callback: Arc::new(RwLock::new(None)),
        }
    }

    /// Get our public key for registration
    pub fn public_key(&self) -> &[u8; 32] {
        self.keypair.public_key()
    }

    /// Set callback for forwarding packets to next hop
    pub async fn set_forward_callback<F>(&self, callback: F)
    where
        F: Fn(NodeId, OnionPacket) + Send + Sync + 'static,
    {
        *self.forward_callback.write().await = Some(Box::new(callback));
    }

    /// Set callback for handling final payloads at exit
    pub async fn set_exit_callback<F>(&self, callback: F)
    where
        F: Fn(Vec<u8>) + Send + Sync + 'static,
    {
        *self.exit_callback.write().await = Some(Box::new(callback));
    }

    /// Register a known mix node
    pub async fn add_node(&self, node: MixNode) {
        self.known_nodes.write().await.insert(node.node_id.clone(), node);
    }

    /// Remove a mix node
    pub async fn remove_node(&self, node_id: &NodeId) {
        self.known_nodes.write().await.remove(node_id);
    }

    /// Get list of known nodes for path building
    pub async fn get_nodes(&self) -> Vec<MixNode> {
        self.known_nodes.read().await.values().cloned().collect()
    }

    /// Process an incoming onion packet
    pub async fn process_packet(&self, packet: OnionPacket) -> NonosResult<()> {
        if !packet.is_valid() {
            return Err(NonosError::Crypto("Invalid packet structure".into()));
        }

        // Add to pool with random jitter
        let jitter = (random_bytes::<2>()[0] as u64 * self.max_delay_ms) / 255;
        let pooled = PooledRequest {
            packet,
            arrival_time: std::time::Instant::now(),
            jitter_ms: jitter,
        };

        let mut pool = self.request_pool.write().await;
        pool.push(pooled);

        // Check if we should flush
        if pool.len() >= self.min_pool_size {
            drop(pool);
            self.flush_pool().await?;
        }

        Ok(())
    }

    /// Flush and process all requests in the pool
    async fn flush_pool(&self) -> NonosResult<()> {
        let mut pool = self.request_pool.write().await;
        if pool.is_empty() {
            return Ok(());
        }

        debug!("Mixing {} requests in pool", pool.len());

        // Shuffle the pool (Fisher-Yates)
        let rng_bytes = random_bytes::<32>();
        for i in (1..pool.len()).rev() {
            let j = (rng_bytes[i % 32] as usize) % (i + 1);
            pool.swap(i, j);
        }

        let requests: Vec<_> = pool.drain(..).collect();
        drop(pool);

        // Process each request
        for request in requests {
            // Calculate remaining delay based on arrival time and target jitter
            let elapsed = request.arrival_time.elapsed().as_millis() as u64;
            let remaining_delay = request.jitter_ms.saturating_sub(elapsed);
            if remaining_delay > 0 {
                tokio::time::sleep(tokio::time::Duration::from_millis(remaining_delay)).await;
            }

            // Decrypt our layer
            match decrypt_onion_layer(&request.packet, &self.keypair) {
                Ok(layer) => {
                    if layer.is_exit {
                        // We're the exit node - deliver payload
                        if let Some(ref callback) = *self.exit_callback.read().await {
                            callback(layer.payload);
                        }
                    } else if let Some(next_hop) = layer.next_hop {
                        // Forward to next hop
                        // Reconstruct packet from remaining payload
                        if layer.payload.len() >= X25519_PUBLIC_KEY_SIZE {
                            let mut ephemeral = [0u8; X25519_PUBLIC_KEY_SIZE];
                            ephemeral.copy_from_slice(&layer.payload[..X25519_PUBLIC_KEY_SIZE]);
                            let encrypted = layer.payload[X25519_PUBLIC_KEY_SIZE..].to_vec();

                            let next_packet = OnionPacket {
                                ephemeral_public: ephemeral,
                                encrypted_payload: encrypted,
                                request_id: request.packet.request_id,
                            };

                            if let Some(ref callback) = *self.forward_callback.read().await {
                                callback(next_hop, next_packet);
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to decrypt onion layer: {}", e);
                    // Drop packet silently - could be replay attack or corruption
                }
            }
        }

        Ok(())
    }

    /// Build an onion packet for sending through the network
    pub async fn create_packet(&self, payload: &[u8], path: &[NodeId]) -> NonosResult<OnionPacket> {
        if path.is_empty() {
            return Err(NonosError::Crypto("Path cannot be empty".into()));
        }

        let nodes = self.known_nodes.read().await;
        let mut path_with_keys = Vec::with_capacity(path.len());

        for node_id in path {
            let node = nodes.get(node_id)
                .ok_or_else(|| NonosError::Crypto(format!("Unknown node: {:?}", node_id)))?;
            path_with_keys.push((node.node_id.clone(), node.public_key));
        }

        drop(nodes);
        build_onion_packet(payload, &path_with_keys)
    }

    /// Get current pool size
    pub async fn pool_size(&self) -> usize {
        self.request_pool.read().await.len()
    }

    /// Get number of known nodes
    pub async fn known_node_count(&self) -> usize {
        self.known_nodes.read().await.len()
    }

    /// Force flush the pool (for shutdown)
    pub async fn force_flush(&self) -> NonosResult<()> {
        self.flush_pool().await
    }
}

impl Default for MixnetProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let kp1 = MixnetKeypair::generate();
        let kp2 = MixnetKeypair::generate();

        // Keys should be different
        assert_ne!(kp1.public, kp2.public);
        assert_ne!(kp1.secret, kp2.secret);
    }

    #[test]
    fn test_diffie_hellman() {
        let alice = MixnetKeypair::generate();
        let bob = MixnetKeypair::generate();

        let shared_alice = alice.diffie_hellman(&bob.public);
        let shared_bob = bob.diffie_hellman(&alice.public);

        assert_eq!(shared_alice, shared_bob);
    }

    #[test]
    fn test_single_hop_onion() {
        let exit_kp = MixnetKeypair::generate();
        let exit_node = NodeId::from_bytes(random_bytes::<32>());

        let path = vec![(exit_node.clone(), *exit_kp.public_key())];
        let payload = b"secret message";

        let packet = build_onion_packet(payload, &path).unwrap();
        assert!(packet.is_valid());

        let decrypted = decrypt_onion_layer(&packet, &exit_kp).unwrap();
        assert!(decrypted.is_exit);
        assert_eq!(decrypted.payload, payload);
    }

    #[test]
    fn test_three_hop_onion() {
        let entry_kp = MixnetKeypair::generate();
        let middle_kp = MixnetKeypair::generate();
        let exit_kp = MixnetKeypair::generate();

        let entry_node = NodeId::from_bytes(random_bytes::<32>());
        let middle_node = NodeId::from_bytes(random_bytes::<32>());
        let exit_node = NodeId::from_bytes(random_bytes::<32>());

        let path = vec![
            (entry_node.clone(), *entry_kp.public_key()),
            (middle_node.clone(), *middle_kp.public_key()),
            (exit_node.clone(), *exit_kp.public_key()),
        ];

        let payload = b"three-hop secret";
        let packet = build_onion_packet(payload, &path).unwrap();

        // Entry node decrypts
        let layer1 = decrypt_onion_layer(&packet, &entry_kp).unwrap();
        assert!(!layer1.is_exit);
        assert_eq!(layer1.next_hop, Some(middle_node.clone()));

        // Reconstruct packet for middle node
        let ephemeral1: [u8; 32] = layer1.payload[..32].try_into().unwrap();
        let packet2 = OnionPacket {
            ephemeral_public: ephemeral1,
            encrypted_payload: layer1.payload[32..].to_vec(),
            request_id: packet.request_id,
        };

        // Middle node decrypts
        let layer2 = decrypt_onion_layer(&packet2, &middle_kp).unwrap();
        assert!(!layer2.is_exit);
        assert_eq!(layer2.next_hop, Some(exit_node.clone()));

        // Reconstruct packet for exit node
        let ephemeral2: [u8; 32] = layer2.payload[..32].try_into().unwrap();
        let packet3 = OnionPacket {
            ephemeral_public: ephemeral2,
            encrypted_payload: layer2.payload[32..].to_vec(),
            request_id: packet.request_id,
        };

        // Exit node decrypts
        let layer3 = decrypt_onion_layer(&packet3, &exit_kp).unwrap();
        assert!(layer3.is_exit);
        assert_eq!(layer3.payload, payload);
    }

    #[test]
    fn test_wrong_key_fails() {
        let real_kp = MixnetKeypair::generate();
        let wrong_kp = MixnetKeypair::generate();
        let node = NodeId::from_bytes(random_bytes::<32>());

        let path = vec![(node, *real_kp.public_key())];
        let packet = build_onion_packet(b"secret", &path).unwrap();

        // Decrypting with wrong key should fail
        let result = decrypt_onion_layer(&packet, &wrong_kp);
        assert!(result.is_err());
    }

    #[test]
    fn test_aes_gcm_roundtrip() {
        let key = Blake3Key::from_bytes(random_bytes::<32>());
        let plaintext = b"test message for encryption";
        let aad = b"additional data";

        let encrypted = aes_gcm_encrypt(&key, plaintext, aad).unwrap();
        let decrypted = aes_gcm_decrypt(&key, &encrypted, aad).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_aes_gcm_wrong_aad_fails() {
        let key = Blake3Key::from_bytes(random_bytes::<32>());
        let plaintext = b"test message";

        let encrypted = aes_gcm_encrypt(&key, plaintext, b"correct aad").unwrap();
        let result = aes_gcm_decrypt(&key, &encrypted, b"wrong aad");

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mixnet_processor() {
        let processor = MixnetProcessor::new();

        // Register a node
        let node_kp = MixnetKeypair::generate();
        let node = MixNode {
            node_id: NodeId::from_bytes(random_bytes::<32>()),
            public_key: *node_kp.public_key(),
            address: "127.0.0.1:9000".to_string(),
        };
        processor.add_node(node.clone()).await;

        assert_eq!(processor.known_node_count().await, 1);
        assert_eq!(processor.pool_size().await, 0);
    }

    #[test]
    fn test_routing_info_serialization() {
        let node_id = NodeId::from_bytes(random_bytes::<32>());
        let routing = RoutingInfo::relay(&node_id);

        let bytes = routing.to_bytes();
        let restored = RoutingInfo::from_bytes(&bytes);

        assert_eq!(restored.next_node, node_id.0);
        assert!(!restored.is_exit());

        let exit = RoutingInfo::exit_node();
        let exit_bytes = exit.to_bytes();
        let restored_exit = RoutingInfo::from_bytes(&exit_bytes);

        assert!(restored_exit.is_exit());
    }
}
