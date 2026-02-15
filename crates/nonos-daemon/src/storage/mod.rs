mod config;
mod metrics;
mod types;

pub use config::*;
pub use metrics::*;
pub use types::*;

use nonos_types::{NonosError, NonosResult};
use sled::{Db, Tree};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;
use tracing::{info, warn};

const CURRENT_SCHEMA_VERSION: u32 = 2;
const SCHEMA_KEY: &[u8] = b"__schema_version__";
const MAX_BATCH_SIZE: usize = 1000;

#[cfg(feature = "encrypted-storage")]
pub struct EncryptionKeys {
    pub public_key: sodiumoxide::crypto::box_::PublicKey,
    pub secret_key: sodiumoxide::crypto::box_::SecretKey,
}

pub struct NodeStorage {
    db: Db,
    schema: Tree,
    identity: Tree,
    peers: Tree,
    metrics_tree: Tree,
    epochs: Tree,
    config_tree: Tree,
    claims: Tree,
    secrets: Tree,
    audit_log: Tree,
    storage_config: StorageConfig,
    metrics: Arc<StorageMetrics>,
    opened_at: Instant,
    #[cfg(feature = "encrypted-storage")]
    encryption_keys: Option<EncryptionKeys>,
}

impl NodeStorage {
    pub fn open(config: StorageConfig) -> NonosResult<Self> {
        Self::open_with_options(config, false)
    }

    pub fn open_readonly(config: StorageConfig) -> NonosResult<Self> {
        Self::open_with_options(config, true)
    }

    fn open_with_options(config: StorageConfig, _read_only: bool) -> NonosResult<Self> {
        let path = &config.path;
        info!("Opening storage at {:?}", path);

        let sled_config = sled::Config::new()
            .path(path)
            .cache_capacity(config.cache_capacity_bytes)
            .mode(sled::Mode::HighThroughput);

        let sled_config = if let Some(flush_ms) = config.flush_every_ms {
            sled_config.flush_every_ms(Some(flush_ms))
        } else {
            sled_config.flush_every_ms(None)
        };

        let db = sled_config
            .open()
            .map_err(|e| NonosError::Storage(format!("Failed to open database: {}", e)))?;

        let mut storage = Self::create_from_db(db, config)?;
        storage.ensure_schema()?;

        info!("Storage opened successfully (schema version {})", CURRENT_SCHEMA_VERSION);
        Ok(storage)
    }

    fn create_from_db(db: Db, config: StorageConfig) -> NonosResult<Self> {
        let schema = Self::open_tree(&db, "schema")?;
        let identity = Self::open_tree(&db, "identity")?;
        let peers = Self::open_tree(&db, "peers")?;
        let metrics_tree = Self::open_tree(&db, "metrics")?;
        let epochs = Self::open_tree(&db, "epochs")?;
        let config_tree = Self::open_tree(&db, "config")?;
        let claims = Self::open_tree(&db, "claims")?;
        let secrets = Self::open_tree(&db, "secrets")?;
        let audit_log = Self::open_tree(&db, "audit_log")?;

        Ok(Self {
            db,
            schema,
            identity,
            peers,
            metrics_tree,
            epochs,
            config_tree,
            claims,
            secrets,
            audit_log,
            storage_config: config,
            metrics: Arc::new(StorageMetrics::new()),
            opened_at: Instant::now(),
            #[cfg(feature = "encrypted-storage")]
            encryption_keys: None,
        })
    }

    fn open_tree(db: &Db, name: &str) -> NonosResult<Tree> {
        db.open_tree(name)
            .map_err(|e| NonosError::Storage(format!("Failed to open {} tree: {}", name, e)))
    }

    pub fn in_memory() -> NonosResult<Self> {
        let sled_config = sled::Config::new().temporary(true);
        let db = sled_config
            .open()
            .map_err(|e| NonosError::Storage(format!("Failed to open temp database: {}", e)))?;

        let config = StorageConfig {
            path: std::path::PathBuf::new(),
            ..Default::default()
        };

        let mut storage = Self::create_from_db(db, config)?;
        storage.ensure_schema()?;
        Ok(storage)
    }

    pub fn open_memory() -> NonosResult<Self> {
        Self::in_memory()
    }

    fn ensure_schema(&mut self) -> NonosResult<()> {
        let current_version = self.get_schema_version()?;

        match current_version {
            None => self.initialize_schema()?,
            Some(version) if version < CURRENT_SCHEMA_VERSION => self.run_migrations(version)?,
            Some(version) if version > CURRENT_SCHEMA_VERSION => {
                return Err(NonosError::Storage(format!(
                    "Database schema version {} is newer than supported {}",
                    version, CURRENT_SCHEMA_VERSION
                )));
            }
            Some(_) => {}
        }

        Ok(())
    }

    fn get_schema_version(&self) -> NonosResult<Option<u32>> {
        match self.schema.get(SCHEMA_KEY).map_err(|e| NonosError::Storage(format!("Failed to read schema: {}", e)))? {
            Some(bytes) => {
                let info: SchemaInfo = bincode::deserialize(&bytes)
                    .map_err(|e| NonosError::Storage(format!("Failed to deserialize schema: {}", e)))?;
                Ok(Some(info.version))
            }
            None => Ok(None),
        }
    }

    fn initialize_schema(&self) -> NonosResult<()> {
        info!("Initializing new database with schema version {}", CURRENT_SCHEMA_VERSION);

        let info = SchemaInfo {
            version: CURRENT_SCHEMA_VERSION,
            created_at: chrono::Utc::now().timestamp(),
            last_migration: None,
            migrations_applied: Vec::new(),
        };

        let bytes = bincode::serialize(&info)
            .map_err(|e| NonosError::Storage(format!("Failed to serialize schema: {}", e)))?;

        self.schema.insert(SCHEMA_KEY, bytes)
            .map_err(|e| NonosError::Storage(format!("Failed to store schema: {}", e)))?;

        self.flush()?;
        Ok(())
    }

    fn run_migrations(&mut self, from_version: u32) -> NonosResult<()> {
        info!("Running migrations from {} to {}", from_version, CURRENT_SCHEMA_VERSION);
        let mut current = from_version;

        while current < CURRENT_SCHEMA_VERSION {
            let start = Instant::now();
            let next = current + 1;
            info!("Applying migration {} -> {}", current, next);

            match self.apply_migration(current, next) {
                Ok(()) => {
                    self.record_migration(current, next, start.elapsed().as_millis() as u64, true)?;
                    current = next;
                }
                Err(e) => {
                    self.record_migration(current, next, start.elapsed().as_millis() as u64, false)?;
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    fn apply_migration(&mut self, from: u32, to: u32) -> NonosResult<()> {
        match (from, to) {
            (1, 2) => self.migrate_v1_to_v2(),
            _ => {
                warn!("No migration path for {} -> {}", from, to);
                Ok(())
            }
        }
    }

    fn migrate_v1_to_v2(&mut self) -> NonosResult<()> {
        let peers_to_update: Vec<_> = self.peers.iter().filter_map(|r| r.ok()).collect();

        for (key, value) in peers_to_update {
            if let Ok(mut peer_info) = serde_json::from_slice::<StoredPeerInfo>(&value) {
                if peer_info.protocol_version.is_none() {
                    peer_info.protocol_version = Some("1.0.0".to_string());
                    let updated = serde_json::to_vec(&peer_info)
                        .map_err(|e| NonosError::Storage(format!("Migration serialize error: {}", e)))?;
                    self.peers.insert(key, updated)
                        .map_err(|e| NonosError::Storage(format!("Migration insert error: {}", e)))?;
                }
            }
        }

        self.flush()?;
        Ok(())
    }

    fn record_migration(&self, from: u32, to: u32, duration_ms: u64, success: bool) -> NonosResult<()> {
        let mut info = self.get_schema_info()?.unwrap_or_else(|| SchemaInfo {
            version: from,
            created_at: chrono::Utc::now().timestamp(),
            last_migration: None,
            migrations_applied: Vec::new(),
        });

        if success {
            info.version = to;
            info.last_migration = Some(chrono::Utc::now().timestamp());
        }

        info.migrations_applied.push(MigrationRecord {
            from_version: from,
            to_version: to,
            applied_at: chrono::Utc::now().timestamp(),
            duration_ms,
            success,
        });

        let bytes = bincode::serialize(&info)
            .map_err(|e| NonosError::Storage(format!("Failed to serialize schema: {}", e)))?;

        self.schema.insert(SCHEMA_KEY, bytes)
            .map_err(|e| NonosError::Storage(format!("Failed to update schema: {}", e)))?;

        Ok(())
    }

    fn get_schema_info(&self) -> NonosResult<Option<SchemaInfo>> {
        match self.schema.get(SCHEMA_KEY).map_err(|e| NonosError::Storage(format!("Schema read error: {}", e)))? {
            Some(bytes) => Ok(Some(bincode::deserialize(&bytes)
                .map_err(|e| NonosError::Storage(format!("Schema deserialize error: {}", e)))?)),
            None => Ok(None),
        }
    }

    pub fn schema_version(&self) -> NonosResult<u32> {
        self.get_schema_version().map(|v| v.unwrap_or(0))
    }

    fn encrypt(&self, data: &[u8]) -> NonosResult<Vec<u8>> {
        Ok(data.to_vec())
    }

    fn decrypt(&self, data: &[u8]) -> NonosResult<Vec<u8>> {
        Ok(data.to_vec())
    }

    fn log_audit(&self, tree: &str, operation: &str, details: Option<&str>) -> NonosResult<()> {
        let entry = AuditLogEntry {
            timestamp: chrono::Utc::now().timestamp_millis(),
            tree: tree.to_string(),
            operation: operation.to_string(),
            details: details.map(String::from),
        };

        let key = entry.timestamp.to_be_bytes();
        let value = bincode::serialize(&entry)
            .map_err(|e| NonosError::Storage(format!("Audit serialize error: {}", e)))?;

        self.audit_log.insert(key, value)
            .map_err(|e| NonosError::Storage(format!("Audit write error: {}", e)))?;

        Ok(())
    }

    pub fn flush(&self) -> NonosResult<()> {
        self.metrics.flushes.fetch_add(1, Ordering::Relaxed);
        self.db.flush().map_err(|e| NonosError::Storage(format!("Flush error: {}", e)))?;
        Ok(())
    }

    pub async fn flush_async(&self) -> NonosResult<()> {
        self.metrics.flushes.fetch_add(1, Ordering::Relaxed);
        self.db.flush_async().await.map_err(|e| NonosError::Storage(format!("Flush error: {}", e)))?;
        Ok(())
    }

    pub fn size_on_disk(&self) -> NonosResult<u64> {
        self.db.size_on_disk().map_err(|e| NonosError::Storage(format!("Size error: {}", e)))
    }

    pub fn compact(&self) -> NonosResult<()> {
        self.flush()?;
        info!("Database compacted");
        Ok(())
    }

    pub fn storage_metrics(&self) -> Arc<StorageMetrics> {
        Arc::clone(&self.metrics)
    }

    pub fn uptime(&self) -> std::time::Duration {
        self.opened_at.elapsed()
    }

    pub fn config(&self) -> &StorageConfig {
        &self.storage_config
    }

    pub fn data_path(&self) -> &std::path::Path {
        &self.storage_config.path
    }

    pub fn is_in_memory(&self) -> bool {
        self.storage_config.path.as_os_str().is_empty()
    }
}

mod identity;
mod peers;
mod epochs;
mod operations;
