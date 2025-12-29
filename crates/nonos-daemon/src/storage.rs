// NONOS GNU AFFERO GENERAL PUBLIC LICENSE
// Version 3, 19 November 2007
//
// Copyright (C) 2024 NON-OS <team@nonos.systems>
// https://nonos.systems
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

use nonos_types::{EpochNumber, NodeId, NonosError, NonosResult};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sled::{Batch, Db, Tree};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, error, info, warn};

const CURRENT_SCHEMA_VERSION: u32 = 2;
const SCHEMA_KEY: &[u8] = b"__schema_version__";
const MAX_BATCH_SIZE: usize = 1000;
const FLUSH_INTERVAL_SECS: u64 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncryptionMode {
    None,
    #[cfg(feature = "encrypted-storage")]
    SealedBox,
}

impl Default for EncryptionMode {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub path: PathBuf,
    pub cache_capacity_bytes: u64,
    pub flush_every_ms: Option<u64>,
    pub encryption_mode: EncryptionMode,
    pub compression_enabled: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from("data"),
            cache_capacity_bytes: 64 * 1024 * 1024,
            flush_every_ms: Some(FLUSH_INTERVAL_SECS * 1000),
            encryption_mode: EncryptionMode::None,
            compression_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaInfo {
    pub version: u32,
    pub created_at: i64,
    pub last_migration: Option<i64>,
    pub migrations_applied: Vec<MigrationRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationRecord {
    pub from_version: u32,
    pub to_version: u32,
    pub applied_at: i64,
    pub duration_ms: u64,
    pub success: bool,
}

pub struct StorageMetrics {
    pub reads: AtomicU64,
    pub writes: AtomicU64,
    pub deletes: AtomicU64,
    pub flushes: AtomicU64,
    pub read_bytes: AtomicU64,
    pub write_bytes: AtomicU64,
    pub errors: AtomicU64,
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
}

impl StorageMetrics {
    fn new() -> Self {
        Self {
            reads: AtomicU64::new(0),
            writes: AtomicU64::new(0),
            deletes: AtomicU64::new(0),
            flushes: AtomicU64::new(0),
            read_bytes: AtomicU64::new(0),
            write_bytes: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
        }
    }

    pub fn snapshot(&self) -> StorageMetricsSnapshot {
        StorageMetricsSnapshot {
            reads: self.reads.load(Ordering::Relaxed),
            writes: self.writes.load(Ordering::Relaxed),
            deletes: self.deletes.load(Ordering::Relaxed),
            flushes: self.flushes.load(Ordering::Relaxed),
            read_bytes: self.read_bytes.load(Ordering::Relaxed),
            write_bytes: self.write_bytes.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.cache_misses.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageMetricsSnapshot {
    pub reads: u64,
    pub writes: u64,
    pub deletes: u64,
    pub flushes: u64,
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub errors: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

#[cfg(feature = "encrypted-storage")]
pub struct EncryptionKeys {
    pub public_key: sodiumoxide::crypto::box_::PublicKey,
    pub secret_key: sodiumoxide::crypto::box_::SecretKey,
}

#[cfg(feature = "encrypted-storage")]
impl EncryptionKeys {
    pub fn generate() -> Self {
        let (public_key, secret_key) = sodiumoxide::crypto::box_::gen_keypair();
        Self {
            public_key,
            secret_key,
        }
    }

    pub fn from_bytes(public: &[u8], secret: &[u8]) -> NonosResult<Self> {
        let public_key = sodiumoxide::crypto::box_::PublicKey::from_slice(public)
            .ok_or_else(|| NonosError::Storage("Invalid public key".into()))?;
        let secret_key = sodiumoxide::crypto::box_::SecretKey::from_slice(secret)
            .ok_or_else(|| NonosError::Storage("Invalid secret key".into()))?;
        Ok(Self {
            public_key,
            secret_key,
        })
    }
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

        let schema = Self::open_tree(&db, "schema")?;
        let identity = Self::open_tree(&db, "identity")?;
        let peers = Self::open_tree(&db, "peers")?;
        let metrics_tree = Self::open_tree(&db, "metrics")?;
        let epochs = Self::open_tree(&db, "epochs")?;
        let config_tree = Self::open_tree(&db, "config")?;
        let claims = Self::open_tree(&db, "claims")?;
        let secrets = Self::open_tree(&db, "secrets")?;
        let audit_log = Self::open_tree(&db, "audit_log")?;

        let mut storage = Self {
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
        };

        storage.ensure_schema()?;

        info!(
            "Storage opened successfully (schema version {})",
            CURRENT_SCHEMA_VERSION
        );

        Ok(storage)
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

        let schema = Self::open_tree(&db, "schema")?;
        let identity = Self::open_tree(&db, "identity")?;
        let peers = Self::open_tree(&db, "peers")?;
        let metrics_tree = Self::open_tree(&db, "metrics")?;
        let epochs = Self::open_tree(&db, "epochs")?;
        let config_tree = Self::open_tree(&db, "config")?;
        let claims = Self::open_tree(&db, "claims")?;
        let secrets = Self::open_tree(&db, "secrets")?;
        let audit_log = Self::open_tree(&db, "audit_log")?;

        let config = StorageConfig {
            path: PathBuf::new(),
            ..Default::default()
        };

        let mut storage = Self {
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
        };

        storage.ensure_schema()?;

        Ok(storage)
    }

    pub fn open_memory() -> NonosResult<Self> {
        Self::in_memory()
    }

    fn ensure_schema(&mut self) -> NonosResult<()> {
        let current_version = self.get_schema_version()?;

        match current_version {
            None => {
                self.initialize_schema()?;
            }
            Some(version) if version < CURRENT_SCHEMA_VERSION => {
                self.run_migrations(version)?;
            }
            Some(version) if version > CURRENT_SCHEMA_VERSION => {
                return Err(NonosError::Storage(format!(
                    "Database schema version {} is newer than supported version {}. \
                     Please upgrade the software.",
                    version, CURRENT_SCHEMA_VERSION
                )));
            }
            Some(_) => {}
        }

        Ok(())
    }

    fn get_schema_version(&self) -> NonosResult<Option<u32>> {
        match self
            .schema
            .get(SCHEMA_KEY)
            .map_err(|e| NonosError::Storage(format!("Failed to read schema version: {}", e)))?
        {
            Some(bytes) => {
                let info: SchemaInfo = bincode::deserialize(&bytes).map_err(|e| {
                    NonosError::Storage(format!("Failed to deserialize schema info: {}", e))
                })?;
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
            .map_err(|e| NonosError::Storage(format!("Failed to serialize schema info: {}", e)))?;

        self.schema
            .insert(SCHEMA_KEY, bytes)
            .map_err(|e| NonosError::Storage(format!("Failed to store schema info: {}", e)))?;

        self.flush()?;

        Ok(())
    }

    fn run_migrations(&mut self, from_version: u32) -> NonosResult<()> {
        info!(
            "Running migrations from version {} to {}",
            from_version, CURRENT_SCHEMA_VERSION
        );

        let mut current = from_version;

        while current < CURRENT_SCHEMA_VERSION {
            let start = Instant::now();
            let next = current + 1;

            info!("Applying migration {} -> {}", current, next);

            match self.apply_migration(current, next) {
                Ok(()) => {
                    let duration = start.elapsed();
                    self.record_migration(current, next, duration.as_millis() as u64, true)?;
                    current = next;
                    info!("Migration {} -> {} completed in {:?}", current - 1, current, duration);
                }
                Err(e) => {
                    self.record_migration(current, next, start.elapsed().as_millis() as u64, false)?;
                    error!("Migration {} -> {} failed: {}", current, next, e);
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
                warn!("No migration path defined for {} -> {}", from, to);
                Ok(())
            }
        }
    }

    fn migrate_v1_to_v2(&mut self) -> NonosResult<()> {
        let peers_to_update: Vec<(sled::IVec, sled::IVec)> = self
            .peers
            .iter()
            .filter_map(|r| r.ok())
            .collect();

        for (key, value) in peers_to_update {
            if let Ok(mut peer_info) = serde_json::from_slice::<StoredPeerInfo>(&value) {
                if peer_info.protocol_version.is_none() {
                    peer_info.protocol_version = Some("1.0.0".to_string());

                    let updated = serde_json::to_vec(&peer_info).map_err(|e| {
                        NonosError::Storage(format!("Failed to serialize peer during migration: {}", e))
                    })?;

                    self.peers.insert(key, updated).map_err(|e| {
                        NonosError::Storage(format!("Failed to update peer during migration: {}", e))
                    })?;
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
            .map_err(|e| NonosError::Storage(format!("Failed to serialize schema info: {}", e)))?;

        self.schema
            .insert(SCHEMA_KEY, bytes)
            .map_err(|e| NonosError::Storage(format!("Failed to update schema info: {}", e)))?;

        Ok(())
    }

    fn get_schema_info(&self) -> NonosResult<Option<SchemaInfo>> {
        match self
            .schema
            .get(SCHEMA_KEY)
            .map_err(|e| NonosError::Storage(format!("Failed to read schema info: {}", e)))?
        {
            Some(bytes) => {
                let info: SchemaInfo = bincode::deserialize(&bytes).map_err(|e| {
                    NonosError::Storage(format!("Failed to deserialize schema info: {}", e))
                })?;
                Ok(Some(info))
            }
            None => Ok(None),
        }
    }

    pub fn schema_version(&self) -> NonosResult<u32> {
        self.get_schema_version().map(|v| v.unwrap_or(0))
    }

    #[cfg(feature = "encrypted-storage")]
    pub fn set_encryption_keys(&mut self, keys: EncryptionKeys) {
        self.encryption_keys = Some(keys);
    }

    #[cfg(feature = "encrypted-storage")]
    fn encrypt(&self, data: &[u8]) -> NonosResult<Vec<u8>> {
        match &self.encryption_keys {
            Some(keys) => {
                let nonce = sodiumoxide::crypto::box_::gen_nonce();
                let ciphertext = sodiumoxide::crypto::box_::seal(
                    data,
                    &nonce,
                    &keys.public_key,
                    &keys.secret_key,
                );
                let mut result = nonce.as_ref().to_vec();
                result.extend_from_slice(&ciphertext);
                Ok(result)
            }
            None => Ok(data.to_vec()),
        }
    }

    #[cfg(feature = "encrypted-storage")]
    fn decrypt(&self, data: &[u8]) -> NonosResult<Vec<u8>> {
        match &self.encryption_keys {
            Some(keys) => {
                if data.len() < sodiumoxide::crypto::box_::NONCEBYTES {
                    return Err(NonosError::Storage("Encrypted data too short".into()));
                }

                let nonce = sodiumoxide::crypto::box_::Nonce::from_slice(
                    &data[..sodiumoxide::crypto::box_::NONCEBYTES],
                )
                .ok_or_else(|| NonosError::Storage("Invalid nonce".into()))?;

                let ciphertext = &data[sodiumoxide::crypto::box_::NONCEBYTES..];

                sodiumoxide::crypto::box_::open(ciphertext, &nonce, &keys.public_key, &keys.secret_key)
                    .map_err(|_| NonosError::Storage("Decryption failed".into()))
            }
            None => Ok(data.to_vec()),
        }
    }

    #[cfg(not(feature = "encrypted-storage"))]
    fn encrypt(&self, data: &[u8]) -> NonosResult<Vec<u8>> {
        Ok(data.to_vec())
    }

    #[cfg(not(feature = "encrypted-storage"))]
    fn decrypt(&self, data: &[u8]) -> NonosResult<Vec<u8>> {
        Ok(data.to_vec())
    }

    pub fn store_identity(&self, node_id: &NodeId, encrypted_key: &[u8]) -> NonosResult<()> {
        self.metrics.writes.fetch_add(1, Ordering::Relaxed);

        self.identity
            .insert("node_id", node_id.0.as_slice())
            .map_err(|e| {
                self.metrics.errors.fetch_add(1, Ordering::Relaxed);
                NonosError::Storage(format!("Failed to store node ID: {}", e))
            })?;

        let encrypted = self.encrypt(encrypted_key)?;
        self.identity
            .insert("encrypted_key", encrypted)
            .map_err(|e| {
                self.metrics.errors.fetch_add(1, Ordering::Relaxed);
                NonosError::Storage(format!("Failed to store key: {}", e))
            })?;

        self.metrics.write_bytes.fetch_add(
            (node_id.0.len() + encrypted_key.len()) as u64,
            Ordering::Relaxed,
        );

        self.log_audit("identity", "store", Some(&format!("node_id={}", hex::encode(&node_id.0[..8]))))?;

        self.db
            .flush()
            .map_err(|e| NonosError::Storage(format!("Failed to flush: {}", e)))?;

        debug!("Stored node identity");
        Ok(())
    }

    pub fn load_node_id(&self) -> NonosResult<Option<NodeId>> {
        self.metrics.reads.fetch_add(1, Ordering::Relaxed);

        match self
            .identity
            .get("node_id")
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

        match self
            .identity
            .get("encrypted_key")
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

    pub fn store_peer(&self, peer_id: &str, info: &StoredPeerInfo) -> NonosResult<()> {
        self.metrics.writes.fetch_add(1, Ordering::Relaxed);

        let value = bincode::serialize(info)
            .map_err(|e| NonosError::Storage(format!("Failed to serialize peer: {}", e)))?;

        self.metrics.write_bytes.fetch_add(value.len() as u64, Ordering::Relaxed);

        self.peers
            .insert(peer_id.as_bytes(), value)
            .map_err(|e| {
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
                    NonosError::Storage(format!("Failed to apply peer batch: {}", e))
                })?;
                self.metrics.writes.fetch_add(batch_size as u64, Ordering::Relaxed);
                batch = Batch::default();
                batch_size = 0;
            }
        }

        if batch_size > 0 {
            self.peers.apply_batch(batch).map_err(|e| {
                self.metrics.errors.fetch_add(batch_size as u64, Ordering::Relaxed);
                NonosError::Storage(format!("Failed to apply peer batch: {}", e))
            })?;
            self.metrics.writes.fetch_add(batch_size as u64, Ordering::Relaxed);
        }

        debug!("Stored {} peers in batch", stored);
        Ok(stored)
    }

    pub fn load_peer(&self, peer_id: &str) -> NonosResult<Option<StoredPeerInfo>> {
        self.metrics.reads.fetch_add(1, Ordering::Relaxed);

        match self
            .peers
            .get(peer_id.as_bytes())
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

            let info: StoredPeerInfo = bincode::deserialize(&value).map_err(|e| {
                NonosError::Storage(format!("Failed to deserialize peer: {}", e))
            })?;

            peers.push((peer_id, info));
        }

        Ok(peers)
    }

    pub fn remove_peer(&self, peer_id: &str) -> NonosResult<bool> {
        self.metrics.deletes.fetch_add(1, Ordering::Relaxed);

        let removed = self
            .peers
            .remove(peer_id.as_bytes())
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

    pub fn store_metrics(&self, timestamp: i64, metrics: &StoredMetrics) -> NonosResult<()> {
        self.metrics.writes.fetch_add(1, Ordering::Relaxed);

        let key = timestamp.to_be_bytes();
        let value = bincode::serialize(metrics)
            .map_err(|e| NonosError::Storage(format!("Failed to serialize metrics: {}", e)))?;

        self.metrics.write_bytes.fetch_add(value.len() as u64, Ordering::Relaxed);

        self.metrics_tree
            .insert(key, value)
            .map_err(|e| {
                self.metrics.errors.fetch_add(1, Ordering::Relaxed);
                NonosError::Storage(format!("Failed to store metrics: {}", e))
            })?;

        Ok(())
    }

    pub fn load_metrics_range(
        &self,
        start: i64,
        end: i64,
    ) -> NonosResult<Vec<(i64, StoredMetrics)>> {
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

            let timestamp = i64::from_be_bytes(
                key.as_ref()
                    .try_into()
                    .map_err(|_| NonosError::Storage("Invalid timestamp key".into()))?,
            );

            let metrics: StoredMetrics = bincode::deserialize(&value).map_err(|e| {
                NonosError::Storage(format!("Failed to deserialize metrics: {}", e))
            })?;

            results.push((timestamp, metrics));
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

            let timestamp = i64::from_be_bytes(
                key.as_ref()
                    .try_into()
                    .map_err(|_| NonosError::Storage("Invalid timestamp key".into()))?,
            );

            let metrics: StoredMetrics = bincode::deserialize(&value).map_err(|e| {
                NonosError::Storage(format!("Failed to deserialize metrics: {}", e))
            })?;

            results.push((timestamp, metrics));
        }

        results.reverse();
        Ok(results)
    }

    pub fn prune_metrics(&self, older_than: i64) -> NonosResult<usize> {
        let key = older_than.to_be_bytes();
        let mut count = 0;
        let mut batch = Batch::default();
        let mut batch_size = 0;

        let keys_to_remove: Vec<sled::IVec> = self
            .metrics_tree
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

    pub fn store_epoch(&self, epoch: EpochNumber, summary: &StoredEpochSummary) -> NonosResult<()> {
        self.metrics.writes.fetch_add(1, Ordering::Relaxed);

        let key = epoch.0.to_be_bytes();
        let value = bincode::serialize(summary)
            .map_err(|e| NonosError::Storage(format!("Failed to serialize epoch: {}", e)))?;

        self.metrics.write_bytes.fetch_add(value.len() as u64, Ordering::Relaxed);

        self.epochs
            .insert(key, value)
            .map_err(|e| {
                self.metrics.errors.fetch_add(1, Ordering::Relaxed);
                NonosError::Storage(format!("Failed to store epoch: {}", e))
            })?;

        debug!("Stored epoch {}", epoch.0);
        Ok(())
    }

    pub fn load_epoch(&self, epoch: EpochNumber) -> NonosResult<Option<StoredEpochSummary>> {
        self.metrics.reads.fetch_add(1, Ordering::Relaxed);

        let key = epoch.0.to_be_bytes();

        match self
            .epochs
            .get(key)
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
        match self
            .epochs
            .last()
            .map_err(|e| NonosError::Storage(format!("Failed to get latest epoch: {}", e)))?
        {
            Some((key, _)) => {
                let epoch = u64::from_be_bytes(
                    key.as_ref()
                        .try_into()
                        .map_err(|_| NonosError::Storage("Invalid epoch key".into()))?,
                );
                Ok(Some(EpochNumber(epoch)))
            }
            None => Ok(None),
        }
    }

    pub fn load_epoch_range(
        &self,
        start: EpochNumber,
        end: EpochNumber,
    ) -> NonosResult<Vec<StoredEpochSummary>> {
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

            let summary: StoredEpochSummary = bincode::deserialize(&value).map_err(|e| {
                NonosError::Storage(format!("Failed to deserialize epoch: {}", e))
            })?;

            results.push(summary);
        }

        Ok(results)
    }

    pub fn store_config<T: Serialize>(&self, key: &str, value: &T) -> NonosResult<()> {
        self.metrics.writes.fetch_add(1, Ordering::Relaxed);

        let bytes = bincode::serialize(value)
            .map_err(|e| NonosError::Storage(format!("Failed to serialize config: {}", e)))?;

        self.metrics.write_bytes.fetch_add(bytes.len() as u64, Ordering::Relaxed);

        self.config_tree
            .insert(key.as_bytes(), bytes)
            .map_err(|e| {
                self.metrics.errors.fetch_add(1, Ordering::Relaxed);
                NonosError::Storage(format!("Failed to store config: {}", e))
            })?;

        Ok(())
    }

    pub fn load_config<T: DeserializeOwned>(&self, key: &str) -> NonosResult<Option<T>> {
        self.metrics.reads.fetch_add(1, Ordering::Relaxed);

        match self
            .config_tree
            .get(key.as_bytes())
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

        let removed = self
            .config_tree
            .remove(key.as_bytes())
            .map_err(|e| {
                self.metrics.errors.fetch_add(1, Ordering::Relaxed);
                NonosError::Storage(format!("Failed to delete config: {}", e))
            })?
            .is_some();

        Ok(removed)
    }

    pub fn store_claim(&self, epoch: EpochNumber, claim: &StoredClaim) -> NonosResult<()> {
        self.metrics.writes.fetch_add(1, Ordering::Relaxed);

        let key = epoch.0.to_be_bytes();
        let value = bincode::serialize(claim)
            .map_err(|e| NonosError::Storage(format!("Failed to serialize claim: {}", e)))?;

        self.metrics.write_bytes.fetch_add(value.len() as u64, Ordering::Relaxed);

        self.claims
            .insert(key, value)
            .map_err(|e| {
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

            let epoch = EpochNumber(u64::from_be_bytes(
                key.as_ref()
                    .try_into()
                    .map_err(|_| NonosError::Storage("Invalid epoch key".into()))?,
            ));

            let claim: StoredClaim = bincode::deserialize(&value).map_err(|e| {
                NonosError::Storage(format!("Failed to deserialize claim: {}", e))
            })?;

            claims.push((epoch, claim));
        }

        Ok(claims)
    }

    pub fn store_secret(&self, key: &str, value: &[u8]) -> NonosResult<()> {
        self.metrics.writes.fetch_add(1, Ordering::Relaxed);

        let encrypted = self.encrypt(value)?;
        self.metrics.write_bytes.fetch_add(encrypted.len() as u64, Ordering::Relaxed);

        self.secrets
            .insert(key.as_bytes(), encrypted)
            .map_err(|e| {
                self.metrics.errors.fetch_add(1, Ordering::Relaxed);
                NonosError::Storage(format!("Failed to store secret: {}", e))
            })?;

        self.log_audit("secrets", "store", Some(key))?;

        self.db
            .flush()
            .map_err(|e| NonosError::Storage(format!("Failed to flush: {}", e)))?;

        debug!("Stored secret: {}", key);
        Ok(())
    }

    pub fn load_secret(&self, key: &str) -> NonosResult<Option<Vec<u8>>> {
        self.metrics.reads.fetch_add(1, Ordering::Relaxed);

        match self
            .secrets
            .get(key.as_bytes())
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

        let removed = self
            .secrets
            .remove(key.as_bytes())
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

    fn log_audit(&self, tree: &str, operation: &str, details: Option<&str>) -> NonosResult<()> {
        let entry = AuditLogEntry {
            timestamp: chrono::Utc::now().timestamp_millis(),
            tree: tree.to_string(),
            operation: operation.to_string(),
            details: details.map(String::from),
        };

        let key = entry.timestamp.to_be_bytes();
        let value = bincode::serialize(&entry)
            .map_err(|e| NonosError::Storage(format!("Failed to serialize audit entry: {}", e)))?;

        self.audit_log
            .insert(key, value)
            .map_err(|e| NonosError::Storage(format!("Failed to write audit log: {}", e)))?;

        Ok(())
    }

    pub fn load_audit_log(&self, limit: usize) -> NonosResult<Vec<AuditLogEntry>> {
        let mut entries = Vec::with_capacity(limit);

        for result in self.audit_log.iter().rev().take(limit) {
            let (_, value) = result.map_err(|e| {
                NonosError::Storage(format!("Failed to iterate audit log: {}", e))
            })?;

            let entry: AuditLogEntry = bincode::deserialize(&value).map_err(|e| {
                NonosError::Storage(format!("Failed to deserialize audit entry: {}", e))
            })?;

            entries.push(entry);
        }

        Ok(entries)
    }

    pub async fn record_metrics(
        &self,
        total_requests: u64,
        successful_requests: u64,
        quality_score: f64,
    ) -> NonosResult<()> {
        let now = chrono::Utc::now().timestamp();
        let metrics = StoredMetrics {
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
        self.store_metrics(now, &metrics)
    }

    pub fn flush(&self) -> NonosResult<()> {
        self.metrics.flushes.fetch_add(1, Ordering::Relaxed);

        self.db
            .flush()
            .map_err(|e| NonosError::Storage(format!("Failed to flush: {}", e)))?;

        Ok(())
    }

    pub async fn flush_async(&self) -> NonosResult<()> {
        self.metrics.flushes.fetch_add(1, Ordering::Relaxed);

        self.db
            .flush_async()
            .await
            .map_err(|e| NonosError::Storage(format!("Failed to flush: {}", e)))?;

        Ok(())
    }

    pub fn size_on_disk(&self) -> NonosResult<u64> {
        self.db
            .size_on_disk()
            .map_err(|e| NonosError::Storage(format!("Failed to get size: {}", e)))
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

        let export = self
            .db
            .export();

        let mut total_bytes = 0u64;
        let mut backup_data = Vec::new();

        for (tree_name, tree_checksum, entries) in export {
            // Collect the entries iterator into a Vec for serialization
            let entries_vec: Vec<Vec<Vec<u8>>> = entries.collect();
            let collection: (Vec<u8>, Vec<u8>, Vec<Vec<Vec<u8>>>) = (tree_name, tree_checksum, entries_vec);
            let collection_data = bincode::serialize(&collection).map_err(|e| {
                NonosError::Storage(format!("Failed to serialize backup: {}", e))
            })?;
            total_bytes += collection_data.len() as u64;
            backup_data.push(collection_data);
        }

        let all_data = bincode::serialize(&backup_data).map_err(|e| {
            NonosError::Storage(format!("Failed to serialize backup data: {}", e))
        })?;

        std::fs::write(backup_path, &all_data).map_err(|e| {
            NonosError::Storage(format!("Failed to write backup file: {}", e))
        })?;

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

    /// Get the storage configuration
    pub fn config(&self) -> &StorageConfig {
        &self.storage_config
    }

    /// Get the data directory path
    pub fn data_path(&self) -> &std::path::Path {
        &self.storage_config.path
    }

    /// Check if storage is using in-memory mode
    pub fn is_in_memory(&self) -> bool {
        self.storage_config.path.as_os_str().is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeSizes {
    pub identity: usize,
    pub peers: usize,
    pub metrics: usize,
    pub epochs: usize,
    pub config: usize,
    pub claims: usize,
    pub secrets: usize,
    pub audit_log: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub timestamp: i64,
    pub tree: String,
    pub operation: String,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityReport {
    pub checked_at: i64,
    pub total_entries: usize,
    pub valid_entries: usize,
    pub corrupted_entries: usize,
    pub tree_reports: Vec<TreeIntegrityReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeIntegrityReport {
    pub name: String,
    pub entries: usize,
    pub valid: usize,
    pub corrupted: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredPeerInfo {
    pub peer_id: String,
    pub addresses: Vec<String>,
    pub last_seen: i64,
    pub avg_latency_ms: Option<u32>,
    pub connection_count: u32,
    pub is_bootstrap: bool,
    pub reputation: u8,
    pub protocol_version: Option<String>,
    pub capabilities: Option<Vec<String>>,
    pub first_seen: Option<i64>,
    pub last_failure: Option<i64>,
    pub failure_count: u32,
}

impl Default for StoredPeerInfo {
    fn default() -> Self {
        Self {
            peer_id: String::new(),
            addresses: Vec::new(),
            last_seen: 0,
            avg_latency_ms: None,
            connection_count: 0,
            is_bootstrap: false,
            reputation: 50,
            protocol_version: Some("1.0.0".to_string()),
            capabilities: Some(Vec::new()),
            first_seen: None,
            last_failure: None,
            failure_count: 0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub error_count: u64,
    pub avg_latency_ms: u32,
    pub peer_count: usize,
    pub quality_score: f64,
    pub uptime_secs: u64,
    pub cpu_usage: Option<f32>,
    pub memory_usage: Option<f32>,
    pub network_bytes_sent: Option<u64>,
    pub network_bytes_received: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredEpochSummary {
    pub epoch: u64,
    pub start_time: i64,
    pub end_time: i64,
    pub total_emission: u128,
    pub our_reward: u128,
    pub quality_score: f64,
    pub participated: bool,
    pub streak: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredClaim {
    pub epoch: u64,
    pub amount: u128,
    pub claimed_at: i64,
    pub tx_hash: Option<String>,
    pub claimant: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_in_memory() {
        let storage = NodeStorage::in_memory().expect("Failed to create in-memory storage");

        let node_id = NodeId::from_bytes([1u8; 32]);
        storage
            .store_identity(&node_id, b"encrypted_key_data")
            .expect("Failed to store identity");

        let loaded = storage.load_node_id().expect("Failed to load node ID");
        assert_eq!(loaded, Some(node_id));

        let key = storage
            .load_encrypted_key()
            .expect("Failed to load encrypted key");
        assert_eq!(key, Some(b"encrypted_key_data".to_vec()));
    }

    #[test]
    fn test_peer_storage() {
        let storage = NodeStorage::in_memory().expect("Failed to create storage");

        let peer_info = StoredPeerInfo {
            peer_id: "12D3KooW...".to_string(),
            addresses: vec!["/ip4/127.0.0.1/tcp/9432".to_string()],
            last_seen: chrono::Utc::now().timestamp(),
            avg_latency_ms: Some(50),
            connection_count: 5,
            is_bootstrap: false,
            reputation: 80,
            protocol_version: Some("1.0.0".to_string()),
            capabilities: Some(vec!["relay".to_string()]),
            first_seen: Some(chrono::Utc::now().timestamp() - 3600),
            last_failure: None,
            failure_count: 0,
        };

        storage
            .store_peer("peer1", &peer_info)
            .expect("Failed to store peer");

        let loaded = storage.load_peer("peer1").expect("Failed to load peer");
        assert!(loaded.is_some());
        assert_eq!(loaded.as_ref().unwrap().peer_id, "12D3KooW...");

        let peers = storage.list_peers().expect("Failed to list peers");
        assert_eq!(peers.len(), 1);

        let count = storage.count_peers().expect("Failed to count peers");
        assert_eq!(count, 1);
    }

    #[test]
    fn test_batch_peer_storage() {
        let storage = NodeStorage::in_memory().expect("Failed to create storage");

        let peers: Vec<(String, StoredPeerInfo)> = (0..100)
            .map(|i| {
                (
                    format!("peer{}", i),
                    StoredPeerInfo {
                        peer_id: format!("12D3KooW...{}", i),
                        ..Default::default()
                    },
                )
            })
            .collect();

        let stored = storage
            .store_peers_batch(&peers)
            .expect("Failed to batch store peers");
        assert_eq!(stored, 100);

        let count = storage.count_peers().expect("Failed to count peers");
        assert_eq!(count, 100);
    }

    #[test]
    fn test_metrics_storage() {
        let storage = NodeStorage::in_memory().expect("Failed to create storage");

        let metrics = StoredMetrics {
            total_requests: 100,
            successful_requests: 95,
            error_count: 5,
            avg_latency_ms: 50,
            peer_count: 10,
            quality_score: 0.95,
            uptime_secs: 3600,
            cpu_usage: Some(25.0),
            memory_usage: Some(50.0),
            network_bytes_sent: Some(1024 * 1024),
            network_bytes_received: Some(2048 * 1024),
        };

        let now = chrono::Utc::now().timestamp();
        storage
            .store_metrics(now, &metrics)
            .expect("Failed to store metrics");

        let loaded = storage
            .load_metrics_range(now - 100, now + 100)
            .expect("Failed to load metrics range");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].1.total_requests, 100);
    }

    #[test]
    fn test_metrics_pruning() {
        let storage = NodeStorage::in_memory().expect("Failed to create storage");

        let now = chrono::Utc::now().timestamp();

        for i in 0..10 {
            let metrics = StoredMetrics {
                total_requests: i as u64,
                successful_requests: i as u64,
                error_count: 0,
                avg_latency_ms: 0,
                peer_count: 0,
                quality_score: 1.0,
                uptime_secs: 0,
                cpu_usage: None,
                memory_usage: None,
                network_bytes_sent: None,
                network_bytes_received: None,
            };
            storage
                .store_metrics(now - (i * 3600), &metrics)
                .expect("Failed to store metrics");
        }

        let pruned = storage
            .prune_metrics(now - 18000)
            .expect("Failed to prune");
        assert!(pruned > 0);

        let remaining = storage
            .load_metrics_range(now - 100000, now + 100)
            .expect("Failed to load remaining");
        assert!(remaining.len() < 10);
    }

    #[test]
    fn test_epoch_storage() {
        let storage = NodeStorage::in_memory().expect("Failed to create storage");

        let summary = StoredEpochSummary {
            epoch: 1,
            start_time: 0,
            end_time: 86400,
            total_emission: 100_000_000_000_000_000_000_000,
            our_reward: 1_000_000_000_000_000_000_000,
            quality_score: 0.95,
            participated: true,
            streak: 5,
        };

        storage
            .store_epoch(EpochNumber(1), &summary)
            .expect("Failed to store epoch");

        let loaded = storage
            .load_epoch(EpochNumber(1))
            .expect("Failed to load epoch");
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().epoch, 1);

        let latest = storage.latest_epoch().expect("Failed to get latest");
        assert_eq!(latest, Some(EpochNumber(1)));
    }

    #[test]
    fn test_config_storage() {
        let storage = NodeStorage::in_memory().expect("Failed to create storage");

        storage
            .store_config("test_key", &"test_value".to_string())
            .expect("Failed to store config");

        let loaded: Option<String> = storage
            .load_config("test_key")
            .expect("Failed to load config");
        assert_eq!(loaded, Some("test_value".to_string()));

        let deleted = storage
            .delete_config("test_key")
            .expect("Failed to delete config");
        assert!(deleted);

        let gone: Option<String> = storage.load_config("test_key").expect("Failed to check");
        assert!(gone.is_none());
    }

    #[test]
    fn test_secret_storage() {
        let storage = NodeStorage::in_memory().expect("Failed to create storage");

        let secret_data = b"super_secret_key_material";
        storage
            .store_secret("my_secret", secret_data)
            .expect("Failed to store secret");

        let loaded = storage
            .load_secret("my_secret")
            .expect("Failed to load secret");
        assert_eq!(loaded, Some(secret_data.to_vec()));

        let deleted = storage
            .delete_secret("my_secret")
            .expect("Failed to delete secret");
        assert!(deleted);
    }

    #[test]
    fn test_audit_log() {
        let storage = NodeStorage::in_memory().expect("Failed to create storage");

        storage
            .store_secret("test", b"data")
            .expect("Failed to store secret");

        let entries = storage
            .load_audit_log(10)
            .expect("Failed to load audit log");
        assert!(!entries.is_empty());
        assert_eq!(entries[0].tree, "secrets");
        assert_eq!(entries[0].operation, "store");
    }

    #[test]
    fn test_storage_metrics() {
        let storage = NodeStorage::in_memory().expect("Failed to create storage");

        storage
            .store_config("key1", &"value1")
            .expect("Failed to store");
        let _: Option<String> = storage.load_config("key1").expect("Failed to load");

        let metrics = storage.storage_metrics().snapshot();
        assert!(metrics.writes > 0);
        assert!(metrics.reads > 0);
    }

    #[test]
    fn test_tree_sizes() {
        let storage = NodeStorage::in_memory().expect("Failed to create storage");

        storage
            .store_peer("peer1", &StoredPeerInfo::default())
            .expect("Failed to store peer");

        let sizes = storage.tree_sizes().expect("Failed to get tree sizes");
        assert_eq!(sizes.peers, 1);
    }

    #[test]
    fn test_integrity_check() {
        let storage = NodeStorage::in_memory().expect("Failed to create storage");

        storage
            .store_peer("peer1", &StoredPeerInfo::default())
            .expect("Failed to store peer");
        storage
            .store_config("key", &"value")
            .expect("Failed to store config");

        let report = storage
            .verify_integrity()
            .expect("Failed to verify integrity");
        assert!(report.total_entries >= 2);
        assert_eq!(report.corrupted_entries, 0);
    }

    #[test]
    fn test_schema_version() {
        let storage = NodeStorage::in_memory().expect("Failed to create storage");
        let version = storage.schema_version().expect("Failed to get version");
        assert_eq!(version, CURRENT_SCHEMA_VERSION);
    }
}
