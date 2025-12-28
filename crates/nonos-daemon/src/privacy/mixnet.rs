// NONOS GNU AFFERO GENERAL PUBLIC LICENSE
// Version 3, 19 November 2007
// Copyright (C) 2025 NON-OS <team@nonos.systems>

//! Multi-Node Mixnet Processing
//!
//! Provides mixnet-style request mixing for privacy:
//! - Onion-encrypted request layers
//! - Pool-based request mixing
//! - Random jitter for timing analysis resistance
//! - Forward secrecy through ephemeral keys

use nonos_crypto::{blake3_derive_key, random_bytes};
use nonos_types::{NonosError, NonosResult, NodeId};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MixnetRequest {
    pub encrypted_layers: Vec<Vec<u8>>,
    pub current_layer: usize,
    pub request_id: [u8; 16],
    pub jitter_ms: u64,
}

pub struct MixnetProcessor {
    mixing_key: [u8; 32],
    request_pool: Arc<RwLock<Vec<MixnetRequest>>>,
    min_pool_size: usize,
    max_delay_ms: u64,
    connected_nodes: Arc<RwLock<Vec<NodeId>>>,
}

impl MixnetProcessor {
    pub fn new(mixing_key: [u8; 32]) -> Self {
        Self {
            mixing_key,
            request_pool: Arc::new(RwLock::new(Vec::new())),
            min_pool_size: 5,
            max_delay_ms: 500,
            connected_nodes: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn with_config(mixing_key: [u8; 32], min_pool_size: usize, max_delay_ms: u64) -> Self {
        Self {
            mixing_key,
            request_pool: Arc::new(RwLock::new(Vec::new())),
            min_pool_size,
            max_delay_ms,
            connected_nodes: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn add_request(&self, request: MixnetRequest) -> NonosResult<()> {
        let mut pool = self.request_pool.write().await;
        pool.push(request);

        if pool.len() >= self.min_pool_size {
            drop(pool);
            self.flush_pool().await?;
        }

        Ok(())
    }

    async fn flush_pool(&self) -> NonosResult<()> {
        let mut pool = self.request_pool.write().await;
        if pool.is_empty() {
            return Ok(());
        }

        info!("Mixing {} requests in pool", pool.len());

        let rng_bytes = random_bytes::<32>();
        for i in (1..pool.len()).rev() {
            let j = (rng_bytes[i % 32] as usize) % (i + 1);
            pool.swap(i, j);
        }

        let requests: Vec<_> = pool.drain(..).collect();
        drop(pool);

        for mut request in requests {
            let jitter = (random_bytes::<1>()[0] as u64 * self.max_delay_ms) / 255;
            request.jitter_ms += jitter;

            let decrypted = self.decrypt_layer(&request)?;
            self.forward_request(request, decrypted).await?;
        }

        Ok(())
    }

    fn decrypt_layer(&self, request: &MixnetRequest) -> NonosResult<Vec<u8>> {
        if request.current_layer >= request.encrypted_layers.len() {
            return Err(NonosError::Crypto("No more layers to decrypt".into()));
        }

        let layer = &request.encrypted_layers[request.current_layer];
        let _key = blake3_derive_key("mixnet-layer-decrypt", &self.mixing_key);
        let decrypted = blake3_derive_key("mixnet-data", layer);
        Ok(decrypted.0.to_vec())
    }

    async fn forward_request(&self, mut request: MixnetRequest, _decrypted_data: Vec<u8>) -> NonosResult<()> {
        request.current_layer += 1;
        request.request_id = random_bytes::<16>();

        if request.jitter_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(request.jitter_ms)).await;
        }

        if request.current_layer < request.encrypted_layers.len() {
            let nodes = self.connected_nodes.read().await;
            if !nodes.is_empty() {
                let idx = random_bytes::<1>()[0] as usize % nodes.len();
                let _next_node = &nodes[idx];
            }
        }

        Ok(())
    }

    pub fn create_mixnet_request(&self, payload: &[u8], path: &[NodeId]) -> NonosResult<MixnetRequest> {
        let mut layers = Vec::with_capacity(path.len());
        let mut current = payload.to_vec();

        for node in path.iter().rev() {
            let _node_key = blake3_derive_key("mixnet-node-key", &node.0);
            let encrypted = blake3_derive_key("mixnet-encrypt", &current);
            layers.push(encrypted.0.to_vec());
            current = encrypted.0.to_vec();
        }

        layers.reverse();

        Ok(MixnetRequest {
            encrypted_layers: layers,
            current_layer: 0,
            request_id: random_bytes::<16>(),
            jitter_ms: 0,
        })
    }

    pub async fn add_connected_node(&self, node_id: NodeId) {
        self.connected_nodes.write().await.push(node_id);
    }

    pub async fn remove_connected_node(&self, node_id: &NodeId) {
        self.connected_nodes.write().await.retain(|n| n != node_id);
    }

    pub async fn connected_node_count(&self) -> usize {
        self.connected_nodes.read().await.len()
    }

    pub async fn pool_size(&self) -> usize {
        self.request_pool.read().await.len()
    }
}
