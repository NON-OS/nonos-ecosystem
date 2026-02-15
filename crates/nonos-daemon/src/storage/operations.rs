use super::{NodeStorage, StoredMetrics, TreeSizes, IntegrityReport, TreeIntegrityReport, AuditLogEntry, MAX_BATCH_SIZE};
use nonos_types::{NonosError, NonosResult};
use serde::{de::DeserializeOwned, Serialize};
use sled::Batch;
use std::path::Path;
use std::sync::atomic::Ordering;
use tracing::{debug, info};

impl NodeStorage {
    pub fn store_metrics(&self, timestamp: i64, metrics: &StoredMetrics) -> NonosResult<()> {
        self.metrics.writes.fetch_add(1, Ordering::Relaxed);

        let key = timestamp.to_be_bytes();
        let value = bincode::serialize(metrics)
            .map_err(|e| NonosError::Storage(format!("Failed to serialize metrics: {}", e)))?;

        self.metrics.write_bytes.fetch_add(value.len() as u64, Ordering::Relaxed);

        self.metrics_tree.insert(key, value).map_err(|e| {
            self.metrics.errors.fetch_add(1, Ordering::Relaxed);
            NonosError::Storage(format!("Failed to store metrics: {}", e))
        })?;

        Ok(())
    }

    pub fn load_metrics_range(&self, start: i64, end: i64) -> NonosResult<Vec<(i64, StoredMetrics)>> {
        let mut results = Vec::new();
        let start_key = start.to_be_bytes();
        let end_key = end.to_be_bytes();

        for result in self.metrics_tree.range(start_key..=end_key) {
            let (key, value) = result.map_err(|e| {
                self.metrics.errors.fetch_add(1, Ordering::Relaxed);
                NonosError::Storage(format!("Failed to iterate metrics: {}", e))
            })?;

            self.metrics.reads.fetch_add(1, Ordering::Relaxed);
            self.metrics.read_bytes.fetch_add(value.len() as u64, Ordering::Relaxed);

            let timestamp = i64::from_be_bytes(key.as_ref().try_into()
                .map_err(|_| NonosError::Storage("Invalid timestamp key".into()))?);

            let m: StoredMetrics = bincode::deserialize(&value)
                .map_err(|e| NonosError::Storage(format!("Failed to deserialize metrics: {}", e)))?;

            results.push((timestamp, m));
        }

        Ok(results)
    }

    pub fn load_latest_metrics(&self, count: usize) -> NonosResult<Vec<(i64, StoredMetrics)>> {
        let mut results = Vec::with_capacity(count);

        for result in self.metrics_tree.iter().rev().take(count) {
            let (key, value) = result.map_err(|e| {
                self.metrics.errors.fetch_add(1, Ordering::Relaxed);
                NonosError::Storage(format!("Failed to iterate metrics: {}", e))
            })?;

            self.metrics.reads.fetch_add(1, Ordering::Relaxed);
            self.metrics.read_bytes.fetch_add(value.len() as u64, Ordering::Relaxed);

            let timestamp = i64::from_be_bytes(key.as_ref().try_into()
                .map_err(|_| NonosError::Storage("Invalid timestamp key".into()))?);

            let m: StoredMetrics = bincode::deserialize(&value)
                .map_err(|e| NonosError::Storage(format!("Failed to deserialize metrics: {}", e)))?;

            results.push((timestamp, m));
        }

        results.reverse();
        Ok(results)
    }

    pub fn prune_metrics(&self, older_than: i64) -> NonosResult<usize> {
        let key = older_than.to_be_bytes();
        let mut count = 0;
        let mut batch = Batch::default();
        let mut batch_size = 0;

        let keys_to_remove: Vec<sled::IVec> = self.metrics_tree
            .range(..key)
            .filter_map(|r| r.ok())
            .map(|(k, _)| k)
            .collect();

        for key in keys_to_remove {
            batch.remove(key);
            batch_size += 1;
            count += 1;

            if batch_size >= MAX_BATCH_SIZE {
                self.metrics_tree.apply_batch(batch).map_err(|e| {
                    self.metrics.errors.fetch_add(batch_size as u64, Ordering::Relaxed);
                    NonosError::Storage(format!("Failed to prune metrics: {}", e))
                })?;
                self.metrics.deletes.fetch_add(batch_size as u64, Ordering::Relaxed);
                batch = Batch::default();
                batch_size = 0;
            }
        }

        if batch_size > 0 {
            self.metrics_tree.apply_batch(batch).map_err(|e| {
                self.metrics.errors.fetch_add(batch_size as u64, Ordering::Relaxed);
                NonosError::Storage(format!("Failed to prune metrics: {}", e))
            })?;
            self.metrics.deletes.fetch_add(batch_size as u64, Ordering::Relaxed);
        }

        if count > 0 {
            info!("Pruned {} old metrics entries", count);
        }

        Ok(count)
    }

    pub fn store_config<T: Serialize>(&self, key: &str, value: &T) -> NonosResult<()> {
        self.metrics.writes.fetch_add(1, Ordering::Relaxed);

        let bytes = bincode::serialize(value)
            .map_err(|e| NonosError::Storage(format!("Failed to serialize config: {}", e)))?;

        self.metrics.write_bytes.fetch_add(bytes.len() as u64, Ordering::Relaxed);

        self.config_tree.insert(key.as_bytes(), bytes).map_err(|e| {
            self.metrics.errors.fetch_add(1, Ordering::Relaxed);
            NonosError::Storage(format!("Failed to store config: {}", e))
        })?;

        Ok(())
    }

    pub fn load_config<T: DeserializeOwned>(&self, key: &str) -> NonosResult<Option<T>> {
        self.metrics.reads.fetch_add(1, Ordering::Relaxed);

        match self.config_tree.get(key.as_bytes())
            .map_err(|e| NonosError::Storage(format!("Failed to load config: {}", e)))?
        {
            Some(bytes) => {
                self.metrics.read_bytes.fetch_add(bytes.len() as u64, Ordering::Relaxed);
                self.metrics.cache_hits.fetch_add(1, Ordering::Relaxed);

                let value: T = bincode::deserialize(&bytes).map_err(|e| {
                    self.metrics.errors.fetch_add(1, Ordering::Relaxed);
                    NonosError::Storage(format!("Failed to deserialize config: {}", e))
                })?;
                Ok(Some(value))
            }
            None => {
                self.metrics.cache_misses.fetch_add(1, Ordering::Relaxed);
                Ok(None)
            }
        }
    }

    pub fn delete_config(&self, key: &str) -> NonosResult<bool> {
        self.metrics.deletes.fetch_add(1, Ordering::Relaxed);
        let removed = self.config_tree.remove(key.as_bytes())
            .map_err(|e| {
                self.metrics.errors.fetch_add(1, Ordering::Relaxed);
                NonosError::Storage(format!("Failed to delete config: {}", e))
            })?
            .is_some();
        Ok(removed)
    }

    pub fn store_secret(&self, key: &str, value: &[u8]) -> NonosResult<()> {
        self.metrics.writes.fetch_add(1, Ordering::Relaxed);

        let encrypted = self.encrypt(value)?;
        self.metrics.write_bytes.fetch_add(encrypted.len() as u64, Ordering::Relaxed);

        self.secrets.insert(key.as_bytes(), encrypted).map_err(|e| {
            self.metrics.errors.fetch_add(1, Ordering::Relaxed);
            NonosError::Storage(format!("Failed to store secret: {}", e))
        })?;

        self.log_audit("secrets", "store", Some(key))?;
        self.db.flush().map_err(|e| NonosError::Storage(format!("Flush error: {}", e)))?;

        debug!("Stored secret: {}", key);
        Ok(())
    }

    pub fn load_secret(&self, key: &str) -> NonosResult<Option<Vec<u8>>> {
        self.metrics.reads.fetch_add(1, Ordering::Relaxed);

        match self.secrets.get(key.as_bytes())
            .map_err(|e| NonosError::Storage(format!("Failed to load secret: {}", e)))?
        {
            Some(bytes) => {
                self.metrics.read_bytes.fetch_add(bytes.len() as u64, Ordering::Relaxed);
                let decrypted = self.decrypt(&bytes)?;
                Ok(Some(decrypted))
            }
            None => Ok(None),
        }
    }

    pub fn delete_secret(&self, key: &str) -> NonosResult<bool> {
        self.metrics.deletes.fetch_add(1, Ordering::Relaxed);

        let removed = self.secrets.remove(key.as_bytes())
            .map_err(|e| {
                self.metrics.errors.fetch_add(1, Ordering::Relaxed);
                NonosError::Storage(format!("Failed to delete secret: {}", e))
            })?
            .is_some();

        if removed {
            self.log_audit("secrets", "delete", Some(key))?;
        }

        Ok(removed)
    }

    pub fn load_audit_log(&self, limit: usize) -> NonosResult<Vec<AuditLogEntry>> {
        let mut entries = Vec::with_capacity(limit);

        for result in self.audit_log.iter().rev().take(limit) {
            let (_, value) = result
                .map_err(|e| NonosError::Storage(format!("Failed to iterate audit log: {}", e)))?;

            let entry: AuditLogEntry = bincode::deserialize(&value)
                .map_err(|e| NonosError::Storage(format!("Failed to deserialize audit: {}", e)))?;

            entries.push(entry);
        }

        Ok(entries)
    }

    pub async fn record_metrics(&self, total_requests: u64, successful_requests: u64, quality_score: f64) -> NonosResult<()> {
        let now = chrono::Utc::now().timestamp();
        let m = StoredMetrics {
            total_requests,
            successful_requests,
            error_count: total_requests.saturating_sub(successful_requests),
            avg_latency_ms: 0,
            peer_count: 0,
            quality_score,
            uptime_secs: 0,
            cpu_usage: None,
            memory_usage: None,
            network_bytes_sent: None,
            network_bytes_received: None,
        };
        self.store_metrics(now, &m)
    }

    pub fn tree_sizes(&self) -> NonosResult<TreeSizes> {
        Ok(TreeSizes {
            identity: self.identity.len(),
            peers: self.peers.len(),
            metrics: self.metrics_tree.len(),
            epochs: self.epochs.len(),
            config: self.config_tree.len(),
            claims: self.claims.len(),
            secrets: self.secrets.len(),
            audit_log: self.audit_log.len(),
        })
    }

    pub fn export_backup(&self, path: impl AsRef<Path>) -> NonosResult<u64> {
        let backup_path = path.as_ref();
        info!("Exporting backup to {:?}", backup_path);

        self.flush()?;

        let export = self.db.export();
        let mut total_bytes = 0u64;
        let mut backup_data = Vec::new();

        for (tree_name, tree_checksum, entries) in export {
            let entries_vec: Vec<Vec<Vec<u8>>> = entries.collect();
            let collection: (Vec<u8>, Vec<u8>, Vec<Vec<Vec<u8>>>) = (tree_name, tree_checksum, entries_vec);
            let collection_data = bincode::serialize(&collection)
                .map_err(|e| NonosError::Storage(format!("Backup serialize error: {}", e)))?;
            total_bytes += collection_data.len() as u64;
            backup_data.push(collection_data);
        }

        let all_data = bincode::serialize(&backup_data)
            .map_err(|e| NonosError::Storage(format!("Backup data serialize error: {}", e)))?;

        std::fs::write(backup_path, &all_data)
            .map_err(|e| NonosError::Storage(format!("Backup write error: {}", e)))?;

        info!("Backup exported: {} bytes", total_bytes);
        Ok(total_bytes)
    }

    pub fn verify_integrity(&self) -> NonosResult<IntegrityReport> {
        let mut report = IntegrityReport {
            checked_at: chrono::Utc::now().timestamp(),
            total_entries: 0,
            valid_entries: 0,
            corrupted_entries: 0,
            tree_reports: Vec::new(),
        };

        let trees = [
            ("identity", &self.identity),
            ("peers", &self.peers),
            ("metrics", &self.metrics_tree),
            ("epochs", &self.epochs),
            ("config", &self.config_tree),
            ("claims", &self.claims),
            ("secrets", &self.secrets),
        ];

        for (name, tree) in trees {
            let mut tree_report = TreeIntegrityReport {
                name: name.to_string(),
                entries: 0,
                valid: 0,
                corrupted: 0,
            };

            for result in tree.iter() {
                tree_report.entries += 1;
                report.total_entries += 1;

                match result {
                    Ok(_) => {
                        tree_report.valid += 1;
                        report.valid_entries += 1;
                    }
                    Err(_) => {
                        tree_report.corrupted += 1;
                        report.corrupted_entries += 1;
                    }
                }
            }

            report.tree_reports.push(tree_report);
        }

        Ok(report)
    }
}
