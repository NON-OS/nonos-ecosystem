use nonos_crypto::{poseidon_commitment, encrypt, decrypt, PoseidonMerkleTree};
use nonos_types::{Blake3Key, NodeId, NonosResult, NonosError};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, error, info};

#[derive(Clone)]
struct CacheEntry {
    commitment: [u8; 32],
    blinding: [u8; 32],
    encrypted_data: Vec<u8>,
    last_access: Instant,
    access_count: u64,
    expires_at: Instant,
}

pub struct CacheMixingService {
    _node_id: NodeId,
    cache_tree: Arc<RwLock<PoseidonMerkleTree>>,
    content_map: Arc<RwLock<HashMap<[u8; 32], CacheEntry>>>,
    max_entries: usize,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
    mix_ops: AtomicU64,
}

impl CacheMixingService {
    pub fn new(node_id: NodeId, max_entries: usize) -> Self {
        Self {
            _node_id: node_id,
            cache_tree: Arc::new(RwLock::new(PoseidonMerkleTree::new(16))),
            content_map: Arc::new(RwLock::new(HashMap::new())),
            max_entries,
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            mix_ops: AtomicU64::new(0),
        }
    }

    pub async fn store_mixed(&self, content_hash: [u8; 32], data: Vec<u8>) -> NonosResult<[u8; 32]> {
        self.store_mixed_with_ttl(content_hash, data, 3600).await
    }

    pub async fn store_mixed_with_ttl(
        &self,
        content_hash: [u8; 32],
        data: Vec<u8>,
        ttl_secs: u64,
    ) -> NonosResult<[u8; 32]> {
        let blinding: [u8; 32] = rand::random();
        let commitment = poseidon_commitment(&content_hash, &blinding);
        let encryption_key = poseidon_commitment(&commitment, &blinding);
        let key = Blake3Key::from_bytes(encryption_key);

        let encrypted = encrypt(&key, &data)
            .map_err(|e| NonosError::Crypto(format!("Cache encryption failed: {}", e)))?;

        let mut map = self.content_map.write().await;
        let now = Instant::now();
        map.retain(|_, entry| entry.expires_at > now);

        while map.len() >= self.max_entries {
            if let Some((&oldest_hash, _)) = map.iter().min_by_key(|(_, e)| e.last_access) {
                map.remove(&oldest_hash);
            } else {
                break;
            }
        }

        map.insert(content_hash, CacheEntry {
            commitment,
            blinding,
            encrypted_data: encrypted,
            last_access: now,
            access_count: 1,
            expires_at: now + Duration::from_secs(ttl_secs),
        });

        {
            let mut tree = self.cache_tree.write().await;
            tree.insert(commitment);
        }

        self.mix_ops.fetch_add(1, Ordering::Relaxed);
        debug!("Stored mixed cache entry: {:?}", hex::encode(&content_hash[..8]));
        Ok(commitment)
    }

    pub async fn retrieve_mixed(&self, content_hash: &[u8; 32]) -> Option<Vec<u8>> {
        let mut map = self.content_map.write().await;

        if let Some(entry) = map.get_mut(content_hash) {
            if entry.expires_at <= Instant::now() {
                map.remove(content_hash);
                self.cache_misses.fetch_add(1, Ordering::Relaxed);
                return None;
            }

            entry.last_access = Instant::now();
            entry.access_count += 1;

            let encryption_key = poseidon_commitment(&entry.commitment, &entry.blinding);
            let key = Blake3Key::from_bytes(encryption_key);

            match decrypt(&key, &entry.encrypted_data) {
                Ok(decrypted) => {
                    self.cache_hits.fetch_add(1, Ordering::Relaxed);
                    Some(decrypted)
                }
                Err(e) => {
                    error!("Cache decryption failed: {}", e);
                    map.remove(content_hash);
                    self.cache_misses.fetch_add(1, Ordering::Relaxed);
                    None
                }
            }
        } else {
            self.cache_misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    pub async fn contains(&self, content_hash: &[u8; 32]) -> bool {
        let map = self.content_map.read().await;
        map.get(content_hash).map_or(false, |e| e.expires_at > Instant::now())
    }

    pub async fn remove(&self, content_hash: &[u8; 32]) -> bool {
        self.content_map.write().await.remove(content_hash).is_some()
    }

    pub async fn clear(&self) {
        self.content_map.write().await.clear();
        info!("Cache cleared");
    }

    pub async fn size(&self) -> usize {
        self.content_map.read().await.len()
    }

    pub async fn tree_root(&self) -> [u8; 32] {
        self.cache_tree.read().await.root()
    }

    pub fn stats(&self) -> (u64, u64, u64) {
        (
            self.cache_hits.load(Ordering::Relaxed),
            self.cache_misses.load(Ordering::Relaxed),
            self.mix_ops.load(Ordering::Relaxed),
        )
    }

    pub async fn run(self: Arc<Self>, shutdown: Arc<AtomicBool>) -> NonosResult<()> {
        info!("Cache Mixing service started with max {} entries", self.max_entries);
        let mut ticker = interval(Duration::from_secs(300));

        while !shutdown.load(Ordering::Relaxed) {
            ticker.tick().await;

            let (hits, misses, ops) = self.stats();
            let hit_rate = if hits + misses > 0 {
                (hits as f64 / (hits + misses) as f64) * 100.0
            } else {
                0.0
            };
            debug!("Cache mixing: {:.1}% hit rate, {} ops", hit_rate, ops);

            let mut map = self.content_map.write().await;
            let cutoff = Instant::now() - Duration::from_secs(3600);
            map.retain(|_, entry| entry.last_access > cutoff);
        }

        info!("Cache Mixing service stopped");
        Ok(())
    }
}
