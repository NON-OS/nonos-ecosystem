use super::NodeStorage;
use nonos_types::{NodeId, NonosError, NonosResult};
use std::sync::atomic::Ordering;
use tracing::debug;

impl NodeStorage {
    pub fn store_identity(&self, node_id: &NodeId, encrypted_key: &[u8]) -> NonosResult<()> {
        self.metrics.writes.fetch_add(1, Ordering::Relaxed);

        self.identity.insert("node_id", node_id.0.as_slice()).map_err(|e| {
            self.metrics.errors.fetch_add(1, Ordering::Relaxed);
            NonosError::Storage(format!("Failed to store node ID: {}", e))
        })?;

        let encrypted = self.encrypt(encrypted_key)?;
        self.identity.insert("encrypted_key", encrypted).map_err(|e| {
            self.metrics.errors.fetch_add(1, Ordering::Relaxed);
            NonosError::Storage(format!("Failed to store key: {}", e))
        })?;

        self.metrics.write_bytes.fetch_add(
            (node_id.0.len() + encrypted_key.len()) as u64,
            Ordering::Relaxed,
        );

        self.log_audit("identity", "store", Some(&format!("node_id={}", hex::encode(&node_id.0[..8]))))?;
        self.db.flush().map_err(|e| NonosError::Storage(format!("Flush error: {}", e)))?;

        debug!("Stored node identity");
        Ok(())
    }

    pub fn load_node_id(&self) -> NonosResult<Option<NodeId>> {
        self.metrics.reads.fetch_add(1, Ordering::Relaxed);

        match self.identity.get("node_id")
            .map_err(|e| NonosError::Storage(format!("Failed to load node ID: {}", e)))?
        {
            Some(bytes) => {
                self.metrics.read_bytes.fetch_add(bytes.len() as u64, Ordering::Relaxed);
                self.metrics.cache_hits.fetch_add(1, Ordering::Relaxed);

                if bytes.len() == 32 {
                    let mut id = [0u8; 32];
                    id.copy_from_slice(&bytes);
                    Ok(Some(NodeId(id)))
                } else {
                    self.metrics.errors.fetch_add(1, Ordering::Relaxed);
                    Err(NonosError::Storage("Invalid node ID length".into()))
                }
            }
            None => {
                self.metrics.cache_misses.fetch_add(1, Ordering::Relaxed);
                Ok(None)
            }
        }
    }

    pub fn load_encrypted_key(&self) -> NonosResult<Option<Vec<u8>>> {
        self.metrics.reads.fetch_add(1, Ordering::Relaxed);

        match self.identity.get("encrypted_key")
            .map_err(|e| NonosError::Storage(format!("Failed to load key: {}", e)))?
        {
            Some(bytes) => {
                self.metrics.read_bytes.fetch_add(bytes.len() as u64, Ordering::Relaxed);
                let decrypted = self.decrypt(&bytes)?;
                Ok(Some(decrypted))
            }
            None => Ok(None),
        }
    }
}
