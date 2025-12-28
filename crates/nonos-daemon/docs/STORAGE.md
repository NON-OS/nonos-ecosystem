# NONOS Storage System

This document describes the NONOS daemon's persistent storage layer, including schema versioning, data trees, and maintenance operations.

## Overview

The NONOS daemon uses [Sled](https://github.com/spacejam/sled) as its embedded database. Sled is a high-performance, ACID-compliant key-value store optimized for modern SSDs.

Key features:
- **Embedded**: No separate database server required
- **ACID Transactions**: Atomic operations with crash recovery
- **Zero-copy reads**: Memory-mapped for high performance
- **Automatic compaction**: Background maintenance
- **Compression**: Optional LZ4 compression (enabled by default)

## Storage Configuration

Configure storage in `config.toml`:

```toml
[storage]
# Database path (relative to data_dir or absolute)
db_path = "data.sled"

# Memory cache size in bytes (default: 64MB)
# Increase for better read performance on systems with available RAM
cache_capacity_bytes = 67108864

# Flush interval in milliseconds (default: 30000 = 30 seconds)
# Lower values = more durable, higher values = better throughput
flush_every_ms = 30000

# Enable compression (default: true)
# Reduces disk usage at slight CPU cost
compression_enabled = true
```

### StorageConfig API

```rust
pub struct StorageConfig {
    pub path: PathBuf,
    pub cache_capacity_bytes: u64,
    pub flush_every_ms: Option<u64>,
    pub encryption_mode: EncryptionMode,
    pub compression_enabled: bool,
}
```

## Schema Versioning

The storage layer implements automatic schema versioning and migrations.

### Current Schema Version

```
CURRENT_SCHEMA_VERSION = 2
```

### Schema Info Structure

```rust
pub struct SchemaInfo {
    pub version: u32,           // Current schema version
    pub created_at: i64,        // Database creation timestamp
    pub last_migration: Option<i64>,  // Last migration timestamp
    pub migrations_applied: Vec<MigrationRecord>,
}

pub struct MigrationRecord {
    pub from_version: u32,
    pub to_version: u32,
    pub applied_at: i64,
    pub duration_ms: u64,
    pub success: bool,
}
```

### Migration Behavior

On startup, the storage layer:

1. **Reads current schema version** from the `schema` tree
2. **Compares against `CURRENT_SCHEMA_VERSION`**
3. **Takes appropriate action**:

| Scenario | Action |
|----------|--------|
| No version found | Initialize new database at current version |
| Version < Current | Run migrations sequentially |
| Version = Current | No action needed |
| Version > Current | **Error**: Incompatible database (upgrade software) |

### Migration Flow

```
Database Open
    │
    ├─► Read schema version
    │
    ├─► Version missing?
    │   └─► Initialize at CURRENT_SCHEMA_VERSION
    │
    ├─► Version < CURRENT?
    │   └─► Run migrations: v1→v2→v3→...→current
    │       ├─► Record each migration
    │       └─► Flush after each step
    │
    ├─► Version > CURRENT?
    │   └─► ERROR: Database too new
    │
    └─► Ready
```

### Example Migration (v1 to v2)

```rust
fn migrate_v1_to_v2(&mut self) -> NonosResult<()> {
    // Add protocol_version field to all peer entries
    for (key, value) in self.peers.iter() {
        if let Ok(mut peer_info) = deserialize(&value) {
            if peer_info.protocol_version.is_none() {
                peer_info.protocol_version = Some("1.0.0".to_string());
                self.peers.insert(key, serialize(&peer_info)?)?;
            }
        }
    }
    self.flush()?;
    Ok(())
}
```

## Data Trees

The storage is organized into separate "trees" (namespaces) for different data types:

| Tree | Purpose | Key Format | Value Format |
|------|---------|------------|--------------|
| `schema` | Schema version info | `"__schema_version__"` | `SchemaInfo` (bincode) |
| `identity` | Node identity | `"node_id"`, `"encrypted_key"` | Raw bytes / Encrypted |
| `peers` | Peer information | Peer ID string | `StoredPeerInfo` (bincode) |
| `metrics` | Historical metrics | Timestamp (i64 BE) | `StoredMetrics` (bincode) |
| `epochs` | Epoch summaries | Epoch number (u64 BE) | `StoredEpochSummary` (bincode) |
| `config` | Node configuration | Key string | Generic (bincode) |
| `claims` | Reward claims | Epoch number (u64 BE) | `StoredClaim` (bincode) |
| `secrets` | Encrypted secrets | Key string | Encrypted bytes |
| `audit_log` | Audit trail | Timestamp (i64 BE) | `AuditLogEntry` (bincode) |

### Identity Tree

Stores the node's cryptographic identity:

```rust
// Store identity
storage.store_identity(&node_id, &encrypted_private_key)?;

// Load identity
let node_id: Option<NodeId> = storage.load_node_id()?;
let key: Option<Vec<u8>> = storage.load_encrypted_key()?;
```

### Peers Tree

Stores discovered peer information:

```rust
pub struct StoredPeerInfo {
    pub peer_id: String,
    pub addresses: Vec<String>,
    pub last_seen: i64,
    pub avg_latency_ms: Option<u32>,
    pub connection_count: u32,
    pub is_bootstrap: bool,
    pub reputation: u8,           // 0-100
    pub protocol_version: Option<String>,
    pub capabilities: Option<Vec<String>>,
    pub first_seen: Option<i64>,
    pub last_failure: Option<i64>,
    pub failure_count: u32,
}
```

Operations:
```rust
// Single peer operations
storage.store_peer("peer_id", &peer_info)?;
let peer: Option<StoredPeerInfo> = storage.load_peer("peer_id")?;
storage.remove_peer("peer_id")?;

// Batch operations (efficient for bulk updates)
storage.store_peers_batch(&[(peer_id, info), ...])?;

// Queries
let all_peers: Vec<(String, StoredPeerInfo)> = storage.list_peers()?;
let count: usize = storage.count_peers()?;
```

### Metrics Tree

Stores historical performance metrics with timestamp keys:

```rust
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
```

Operations:
```rust
// Store metrics at current time
storage.store_metrics(timestamp, &metrics)?;

// Query by time range
let history: Vec<(i64, StoredMetrics)> = storage.load_metrics_range(start, end)?;

// Get latest N entries
let recent: Vec<(i64, StoredMetrics)> = storage.load_latest_metrics(100)?;

// Prune old data
let removed: usize = storage.prune_metrics(older_than_timestamp)?;
```

### Epochs Tree

Stores summaries of participation epochs:

```rust
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
```

Operations:
```rust
storage.store_epoch(EpochNumber(1), &summary)?;
let summary: Option<StoredEpochSummary> = storage.load_epoch(EpochNumber(1))?;
let latest: Option<EpochNumber> = storage.latest_epoch()?;
let range: Vec<StoredEpochSummary> = storage.load_epoch_range(start, end)?;
```

### Config Tree

Generic key-value configuration storage:

```rust
// Store any serializable type
storage.store_config("bootstrap_nodes", &["addr1", "addr2"])?;
storage.store_config("last_sync", &chrono::Utc::now().timestamp())?;

// Load with type inference
let nodes: Option<Vec<String>> = storage.load_config("bootstrap_nodes")?;
let sync_time: Option<i64> = storage.load_config("last_sync")?;

// Delete
storage.delete_config("old_key")?;
```

### Secrets Tree

Encrypted storage for sensitive data:

```rust
// Secrets are encrypted before storage
storage.store_secret("wallet_key", &private_key_bytes)?;

// Decrypted on load
let key: Option<Vec<u8>> = storage.load_secret("wallet_key")?;

// Secure delete
storage.delete_secret("wallet_key")?;
```

### Audit Log Tree

Automatic logging of sensitive operations:

```rust
// Audit entries are created automatically for:
// - Identity storage
// - Claim storage
// - Secret operations

// Query recent audit entries
let entries: Vec<AuditLogEntry> = storage.load_audit_log(100)?;

pub struct AuditLogEntry {
    pub timestamp: i64,
    pub tree: String,      // e.g., "secrets", "claims"
    pub operation: String, // e.g., "store", "delete"
    pub details: Option<String>,
}
```

## Storage Modes

### Persistent Storage

Default mode for production:

```rust
let config = StorageConfig {
    path: PathBuf::from("~/.nonos/data.sled"),
    cache_capacity_bytes: 64 * 1024 * 1024,
    flush_every_ms: Some(30000),
    ..Default::default()
};

let storage = NodeStorage::open(config)?;
```

### In-Memory Storage

For testing or ephemeral nodes:

```rust
let storage = NodeStorage::in_memory()?;
// or
let storage = NodeStorage::open_memory()?;
```

Data is lost when the process exits.

### Read-Only Mode

For backup/inspection tools:

```rust
let storage = NodeStorage::open_readonly(config)?;
```

## Encryption

### Optional Encryption Feature

Enable encrypted storage with the `encrypted-storage` feature:

```toml
[dependencies]
nonos-daemon = { version = "0.1", features = ["encrypted-storage"] }
```

### Encryption Modes

```rust
pub enum EncryptionMode {
    None,           // No encryption (default)
    SealedBox,      // NaCl sealed box encryption
}
```

### Using Encryption

```rust
// Generate or load encryption keys
let keys = EncryptionKeys::generate();
// or
let keys = EncryptionKeys::from_bytes(&public_key, &secret_key)?;

// Apply to storage
storage.set_encryption_keys(keys);

// Secrets are now encrypted with public-key cryptography
storage.store_secret("key", &sensitive_data)?;
```

## Maintenance Operations

### Flushing

Force data to disk:

```rust
// Synchronous flush
storage.flush()?;

// Async flush (non-blocking)
storage.flush_async().await?;
```

### Compaction

Reclaim disk space:

```rust
storage.compact()?;
```

### Metrics Pruning

Remove old metrics data:

```rust
// Remove entries older than 7 days
let one_week_ago = chrono::Utc::now().timestamp() - (7 * 24 * 3600);
let removed = storage.prune_metrics(one_week_ago)?;
println!("Removed {} old metrics entries", removed);
```

### Integrity Verification

Check database integrity:

```rust
let report = storage.verify_integrity()?;

pub struct IntegrityReport {
    pub checked_at: i64,
    pub total_entries: usize,
    pub valid_entries: usize,
    pub corrupted_entries: usize,
    pub tree_reports: Vec<TreeIntegrityReport>,
}

if report.corrupted_entries > 0 {
    eprintln!("Database corruption detected!");
}
```

### Backup

Export database to backup file:

```rust
let bytes_written = storage.export_backup("/path/to/backup.nonos")?;
println!("Backup exported: {} bytes", bytes_written);
```

## Storage Metrics

The storage layer tracks operational metrics:

```rust
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

// Get current snapshot
let metrics = storage.storage_metrics().snapshot();
println!("Reads: {}, Writes: {}", metrics.reads, metrics.writes);
println!("Cache hit rate: {:.2}%",
    metrics.cache_hits as f64 / (metrics.cache_hits + metrics.cache_misses) as f64 * 100.0);
```

### Tree Sizes

Query entry counts per tree:

```rust
let sizes = storage.tree_sizes()?;

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
```

### Disk Usage

```rust
let bytes = storage.size_on_disk()?;
println!("Database size: {} MB", bytes / 1024 / 1024);
```

### Uptime

```rust
let uptime = storage.uptime();
println!("Storage open for: {:?}", uptime);
```

## Batch Operations

For high-throughput scenarios, use batch operations:

```rust
// Batch peer storage (up to 1000 per batch internally)
let peers: Vec<(String, StoredPeerInfo)> = /* ... */;
let stored = storage.store_peers_batch(&peers)?;
```

Batches are automatically chunked to `MAX_BATCH_SIZE = 1000` entries to avoid memory issues.

## Error Handling

Storage operations return `NonosResult<T>`:

```rust
match storage.load_peer("peer_id") {
    Ok(Some(peer)) => println!("Found: {:?}", peer),
    Ok(None) => println!("Not found"),
    Err(NonosError::Storage(msg)) => eprintln!("Storage error: {}", msg),
    Err(e) => eprintln!("Other error: {}", e),
}
```

Common error scenarios:
- Database file locked by another process
- Disk full
- Corruption detected
- Serialization failure
- Schema version mismatch

## Performance Tuning

### High Write Throughput

```toml
[storage]
cache_capacity_bytes = 134217728  # 128MB
flush_every_ms = 60000            # 1 minute
compression_enabled = false       # Trade space for CPU
```

### Low Memory Usage

```toml
[storage]
cache_capacity_bytes = 16777216   # 16MB
flush_every_ms = 5000             # 5 seconds
compression_enabled = true        # Save disk space
```

### Maximum Durability

```toml
[storage]
cache_capacity_bytes = 67108864   # 64MB
flush_every_ms = 1000             # 1 second
```

## Data Directory Structure

```
~/.nonos/
├── data.sled/           # Main Sled database directory
│   ├── conf             # Sled configuration
│   ├── blobs/           # Large value storage
│   └── db               # Main B-tree data
├── identity.key         # Optional: Exported identity key
└── config.toml          # Node configuration
```

## Troubleshooting

### Database Won't Open

```
Error: "Failed to open database: Resource busy"
```

Another process has the database locked. Only one process can open a Sled database at a time.

### Schema Version Mismatch

```
Error: "Database schema version 3 is newer than supported version 2"
```

The database was created by a newer version of the software. Upgrade your daemon.

### Corruption Detected

```rust
// Run integrity check
let report = storage.verify_integrity()?;
if report.corrupted_entries > 0 {
    // Option 1: Try recovery
    storage.compact()?;

    // Option 2: Restore from backup
    // rm -rf ~/.nonos/data.sled
    // Restore backup file
}
```

### High Disk Usage

```rust
// Prune old metrics
let week_ago = chrono::Utc::now().timestamp() - (7 * 24 * 3600);
storage.prune_metrics(week_ago)?;

// Compact database
storage.compact()?;
```

### Slow Performance

1. Increase cache size
2. Check disk I/O (use SSD)
3. Reduce flush frequency
4. Use batch operations for bulk writes
