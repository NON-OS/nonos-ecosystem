use crate::NodeStorage;
use nonos_types::{Blake3Hash, NonosResult};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, info, warn};

struct CacheEntry {
    data: Vec<u8>,
    created: Instant,
    last_accessed: Instant,
    ttl: Duration,
    access_count: u64,
}

impl CacheEntry {
    fn is_expired(&self) -> bool {
        self.created.elapsed() > self.ttl
    }
}

pub struct CacheService {
    _storage: Arc<NodeStorage>,
    max_size_bytes: u64,
    current_size: Arc<RwLock<u64>>,
    entries: Arc<RwLock<HashMap<Blake3Hash, CacheEntry>>>,
}

impl CacheService {
    pub fn new(storage: Arc<NodeStorage>, max_size_mb: u32) -> Self {
        Self {
            _storage: storage,
            max_size_bytes: (max_size_mb as u64) * 1024 * 1024,
            current_size: Arc::new(RwLock::new(0)),
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn run(&self, shutdown: Arc<AtomicBool>) -> NonosResult<()> {
        let mut ticker = interval(Duration::from_secs(60));
        info!("Cache service running with {}MB capacity", self.max_size_bytes / 1024 / 1024);

        loop {
            if shutdown.load(Ordering::SeqCst) {
                info!("Cache service shutting down");
                self.persist_cache().await;
                break;
            }

            ticker.tick().await;
            self.cleanup_expired().await;
            self.evict_if_needed().await;
        }

        Ok(())
    }

    pub async fn get(&self, hash: &Blake3Hash) -> Option<Vec<u8>> {
        let mut entries = self.entries.write().await;
        if let Some(entry) = entries.get_mut(hash) {
            if entry.is_expired() {
                entries.remove(hash);
                return None;
            }
            entry.last_accessed = Instant::now();
            entry.access_count += 1;
            return Some(entry.data.clone());
        }
        None
    }

    pub async fn put(&self, hash: Blake3Hash, data: Vec<u8>, ttl_secs: u64) {
        let size = data.len() as u64;

        if size > self.max_size_bytes {
            warn!("Content too large for cache: {} bytes", size);
            return;
        }

        while *self.current_size.read().await + size > self.max_size_bytes {
            if !self.evict_one().await {
                break;
            }
        }

        let entry = CacheEntry {
            data,
            created: Instant::now(),
            last_accessed: Instant::now(),
            ttl: Duration::from_secs(ttl_secs),
            access_count: 0,
        };

        *self.current_size.write().await += size;
        self.entries.write().await.insert(hash, entry);
        debug!("Cached {} bytes", size);
    }

    async fn cleanup_expired(&self) {
        let mut entries = self.entries.write().await;
        let to_remove: Vec<_> = entries.iter()
            .filter(|(_, e)| e.is_expired())
            .map(|(h, _)| *h)
            .collect();

        let mut freed = 0u64;
        for hash in to_remove {
            if let Some(entry) = entries.remove(&hash) {
                freed += entry.data.len() as u64;
            }
        }

        if freed > 0 {
            *self.current_size.write().await -= freed;
            debug!("Cleaned up {} bytes", freed);
        }
    }

    async fn evict_if_needed(&self) {
        while *self.current_size.read().await > self.max_size_bytes {
            if !self.evict_one().await {
                break;
            }
        }
    }

    async fn evict_one(&self) -> bool {
        let mut entries = self.entries.write().await;
        let lru_hash = entries.iter().min_by_key(|(_, e)| e.last_accessed).map(|(h, _)| *h);

        if let Some(hash) = lru_hash {
            if let Some(entry) = entries.remove(&hash) {
                *self.current_size.write().await -= entry.data.len() as u64;
                debug!("Evicted {} bytes", entry.data.len());
                return true;
            }
        }
        false
    }

    async fn persist_cache(&self) {
        let entries = self.entries.read().await;
        for (hash, entry) in entries.iter() {
            if !entry.is_expired() {
                debug!("Persisting cache entry: {:?}", hash);
            }
        }
    }

    pub async fn stats(&self) -> CacheStats {
        let entries = self.entries.read().await;
        let total_accesses: u64 = entries.values().map(|e| e.access_count).sum();

        CacheStats {
            entry_count: entries.len(),
            total_size: *self.current_size.read().await,
            max_size: self.max_size_bytes,
            total_accesses,
            utilization: *self.current_size.read().await as f64 / self.max_size_bytes as f64,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CacheStats {
    pub entry_count: usize,
    pub total_size: u64,
    pub max_size: u64,
    pub total_accesses: u64,
    pub utilization: f64,
}
