use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const FLUSH_INTERVAL_SECS: u64 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EncryptionMode {
    #[default]
    None,
    #[cfg(feature = "encrypted-storage")]
    SealedBox,
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
