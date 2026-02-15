use super::{NodeStorage, StoredClaim, StoredEpochSummary};
use nonos_types::{EpochNumber, NonosError, NonosResult};
use std::sync::atomic::Ordering;
use tracing::{debug, info};

impl NodeStorage {
    pub fn store_epoch(&self, epoch: EpochNumber, summary: &StoredEpochSummary) -> NonosResult<()> {
        self.metrics.writes.fetch_add(1, Ordering::Relaxed);

        let key = epoch.0.to_be_bytes();
        let value = bincode::serialize(summary)
            .map_err(|e| NonosError::Storage(format!("Failed to serialize epoch: {}", e)))?;

        self.metrics.write_bytes.fetch_add(value.len() as u64, Ordering::Relaxed);

        self.epochs.insert(key, value).map_err(|e| {
            self.metrics.errors.fetch_add(1, Ordering::Relaxed);
            NonosError::Storage(format!("Failed to store epoch: {}", e))
        })?;

        debug!("Stored epoch {}", epoch.0);
        Ok(())
    }

    pub fn load_epoch(&self, epoch: EpochNumber) -> NonosResult<Option<StoredEpochSummary>> {
        self.metrics.reads.fetch_add(1, Ordering::Relaxed);
        let key = epoch.0.to_be_bytes();

        match self.epochs.get(key)
            .map_err(|e| NonosError::Storage(format!("Failed to load epoch: {}", e)))?
        {
            Some(bytes) => {
                self.metrics.read_bytes.fetch_add(bytes.len() as u64, Ordering::Relaxed);
                self.metrics.cache_hits.fetch_add(1, Ordering::Relaxed);

                let summary: StoredEpochSummary = bincode::deserialize(&bytes).map_err(|e| {
                    self.metrics.errors.fetch_add(1, Ordering::Relaxed);
                    NonosError::Storage(format!("Failed to deserialize epoch: {}", e))
                })?;
                Ok(Some(summary))
            }
            None => {
                self.metrics.cache_misses.fetch_add(1, Ordering::Relaxed);
                Ok(None)
            }
        }
    }

    pub fn latest_epoch(&self) -> NonosResult<Option<EpochNumber>> {
        match self.epochs.last()
            .map_err(|e| NonosError::Storage(format!("Failed to get latest epoch: {}", e)))?
        {
            Some((key, _)) => {
                let epoch = u64::from_be_bytes(key.as_ref().try_into()
                    .map_err(|_| NonosError::Storage("Invalid epoch key".into()))?);
                Ok(Some(EpochNumber(epoch)))
            }
            None => Ok(None),
        }
    }

    pub fn load_epoch_range(&self, start: EpochNumber, end: EpochNumber) -> NonosResult<Vec<StoredEpochSummary>> {
        let mut results = Vec::new();
        let start_key = start.0.to_be_bytes();
        let end_key = end.0.to_be_bytes();

        for result in self.epochs.range(start_key..=end_key) {
            let (_, value) = result.map_err(|e| {
                self.metrics.errors.fetch_add(1, Ordering::Relaxed);
                NonosError::Storage(format!("Failed to iterate epochs: {}", e))
            })?;

            self.metrics.reads.fetch_add(1, Ordering::Relaxed);
            self.metrics.read_bytes.fetch_add(value.len() as u64, Ordering::Relaxed);

            let summary: StoredEpochSummary = bincode::deserialize(&value)
                .map_err(|e| NonosError::Storage(format!("Failed to deserialize epoch: {}", e)))?;

            results.push(summary);
        }

        Ok(results)
    }

    pub fn store_claim(&self, epoch: EpochNumber, claim: &StoredClaim) -> NonosResult<()> {
        self.metrics.writes.fetch_add(1, Ordering::Relaxed);

        let key = epoch.0.to_be_bytes();
        let value = bincode::serialize(claim)
            .map_err(|e| NonosError::Storage(format!("Failed to serialize claim: {}", e)))?;

        self.metrics.write_bytes.fetch_add(value.len() as u64, Ordering::Relaxed);

        self.claims.insert(key, value).map_err(|e| {
            self.metrics.errors.fetch_add(1, Ordering::Relaxed);
            NonosError::Storage(format!("Failed to store claim: {}", e))
        })?;

        self.log_audit("claims", "store", Some(&format!("epoch={}", epoch.0)))?;
        info!("Stored claim for epoch {}", epoch.0);
        Ok(())
    }

    pub fn load_claims(&self) -> NonosResult<Vec<(EpochNumber, StoredClaim)>> {
        let mut claims = Vec::new();

        for result in self.claims.iter() {
            let (key, value) = result.map_err(|e| {
                self.metrics.errors.fetch_add(1, Ordering::Relaxed);
                NonosError::Storage(format!("Failed to iterate claims: {}", e))
            })?;

            self.metrics.reads.fetch_add(1, Ordering::Relaxed);
            self.metrics.read_bytes.fetch_add(value.len() as u64, Ordering::Relaxed);

            let epoch = EpochNumber(u64::from_be_bytes(key.as_ref().try_into()
                .map_err(|_| NonosError::Storage("Invalid epoch key".into()))?));

            let claim: StoredClaim = bincode::deserialize(&value)
                .map_err(|e| NonosError::Storage(format!("Failed to deserialize claim: {}", e)))?;

            claims.push((epoch, claim));
        }

        Ok(claims)
    }
}
