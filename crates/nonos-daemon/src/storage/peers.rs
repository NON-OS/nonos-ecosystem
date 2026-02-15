use super::{NodeStorage, StoredPeerInfo, MAX_BATCH_SIZE};
use nonos_types::{NonosError, NonosResult};
use sled::Batch;
use std::sync::atomic::Ordering;
use tracing::debug;

impl NodeStorage {
    pub fn store_peer(&self, peer_id: &str, info: &StoredPeerInfo) -> NonosResult<()> {
        self.metrics.writes.fetch_add(1, Ordering::Relaxed);

        let value = bincode::serialize(info)
            .map_err(|e| NonosError::Storage(format!("Failed to serialize peer: {}", e)))?;

        self.metrics.write_bytes.fetch_add(value.len() as u64, Ordering::Relaxed);

        self.peers.insert(peer_id.as_bytes(), value).map_err(|e| {
            self.metrics.errors.fetch_add(1, Ordering::Relaxed);
            NonosError::Storage(format!("Failed to store peer: {}", e))
        })?;

        debug!("Stored peer: {}", peer_id);
        Ok(())
    }

    pub fn store_peers_batch(&self, peers: &[(String, StoredPeerInfo)]) -> NonosResult<usize> {
        if peers.is_empty() {
            return Ok(0);
        }

        let mut stored = 0;
        let mut batch = Batch::default();
        let mut batch_size = 0;

        for (peer_id, info) in peers {
            let value = bincode::serialize(info)
                .map_err(|e| NonosError::Storage(format!("Failed to serialize peer: {}", e)))?;

            batch.insert(peer_id.as_bytes(), value.as_slice());
            batch_size += 1;
            stored += 1;

            self.metrics.write_bytes.fetch_add(value.len() as u64, Ordering::Relaxed);

            if batch_size >= MAX_BATCH_SIZE {
                self.peers.apply_batch(batch).map_err(|e| {
                    self.metrics.errors.fetch_add(batch_size as u64, Ordering::Relaxed);
                    NonosError::Storage(format!("Failed to apply batch: {}", e))
                })?;
                self.metrics.writes.fetch_add(batch_size as u64, Ordering::Relaxed);
                batch = Batch::default();
                batch_size = 0;
            }
        }

        if batch_size > 0 {
            self.peers.apply_batch(batch).map_err(|e| {
                self.metrics.errors.fetch_add(batch_size as u64, Ordering::Relaxed);
                NonosError::Storage(format!("Failed to apply batch: {}", e))
            })?;
            self.metrics.writes.fetch_add(batch_size as u64, Ordering::Relaxed);
        }

        debug!("Stored {} peers in batch", stored);
        Ok(stored)
    }

    pub fn load_peer(&self, peer_id: &str) -> NonosResult<Option<StoredPeerInfo>> {
        self.metrics.reads.fetch_add(1, Ordering::Relaxed);

        match self.peers.get(peer_id.as_bytes())
            .map_err(|e| NonosError::Storage(format!("Failed to load peer: {}", e)))?
        {
            Some(bytes) => {
                self.metrics.read_bytes.fetch_add(bytes.len() as u64, Ordering::Relaxed);
                self.metrics.cache_hits.fetch_add(1, Ordering::Relaxed);

                let info: StoredPeerInfo = bincode::deserialize(&bytes).map_err(|e| {
                    self.metrics.errors.fetch_add(1, Ordering::Relaxed);
                    NonosError::Storage(format!("Failed to deserialize peer: {}", e))
                })?;
                Ok(Some(info))
            }
            None => {
                self.metrics.cache_misses.fetch_add(1, Ordering::Relaxed);
                Ok(None)
            }
        }
    }

    pub fn list_peers(&self) -> NonosResult<Vec<(String, StoredPeerInfo)>> {
        let mut peers = Vec::new();

        for result in self.peers.iter() {
            let (key, value) = result.map_err(|e| {
                self.metrics.errors.fetch_add(1, Ordering::Relaxed);
                NonosError::Storage(format!("Failed to iterate peers: {}", e))
            })?;

            self.metrics.reads.fetch_add(1, Ordering::Relaxed);
            self.metrics.read_bytes.fetch_add(value.len() as u64, Ordering::Relaxed);

            let peer_id = String::from_utf8(key.to_vec())
                .map_err(|e| NonosError::Storage(format!("Invalid peer ID: {}", e)))?;

            let info: StoredPeerInfo = bincode::deserialize(&value)
                .map_err(|e| NonosError::Storage(format!("Failed to deserialize peer: {}", e)))?;

            peers.push((peer_id, info));
        }

        Ok(peers)
    }

    pub fn remove_peer(&self, peer_id: &str) -> NonosResult<bool> {
        self.metrics.deletes.fetch_add(1, Ordering::Relaxed);

        let removed = self.peers.remove(peer_id.as_bytes())
            .map_err(|e| {
                self.metrics.errors.fetch_add(1, Ordering::Relaxed);
                NonosError::Storage(format!("Failed to remove peer: {}", e))
            })?
            .is_some();

        Ok(removed)
    }

    pub fn count_peers(&self) -> NonosResult<usize> {
        Ok(self.peers.len())
    }
}
