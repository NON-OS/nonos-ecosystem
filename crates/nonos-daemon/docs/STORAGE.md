# Storage

The daemon uses sled for persistent storage. Embedded, ACID-compliant, optimized for SSDs.

## Config

```toml
[storage]
db_path = "data.sled"
cache_capacity_bytes = 67108864  # 64MB
flush_every_ms = 30000           # 30 seconds
compression_enabled = true
```

Higher cache = better reads, more RAM.
Lower flush interval = better durability, more writes.

## Schema Versioning

Current version: 2

On startup:
- No version found → initialize at current version
- Version < current → run migrations
- Version > current → error (upgrade your daemon)

Migrations run sequentially. Each step is recorded with timestamp and duration.

## Trees

Sled organizes data into trees (namespaces):

| Tree | Purpose | Key | Value |
|------|---------|-----|-------|
| schema | Version info | `__schema_version__` | SchemaInfo |
| identity | Node keys | `node_id`, `encrypted_key` | bytes |
| peers | Peer data | peer ID | StoredPeerInfo |
| metrics | History | timestamp (BE) | StoredMetrics |
| epochs | Summaries | epoch number (BE) | StoredEpochSummary |
| config | Settings | key string | bincode |
| claims | Rewards | epoch number (BE) | StoredClaim |
| secrets | Encrypted | key string | encrypted bytes |
| audit_log | Trail | timestamp (BE) | AuditLogEntry |

## Data Types

### StoredPeerInfo
```rust
pub struct StoredPeerInfo {
    pub peer_id: String,
    pub addresses: Vec<String>,
    pub last_seen: i64,
    pub avg_latency_ms: Option<u32>,
    pub connection_count: u32,
    pub is_bootstrap: bool,
    pub reputation: u8,
    pub protocol_version: Option<String>,
    pub failure_count: u32,
}
```

### StoredMetrics
```rust
pub struct StoredMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub error_count: u64,
    pub avg_latency_ms: u32,
    pub peer_count: usize,
    pub quality_score: f64,
    pub uptime_secs: u64,
}
```

### StoredEpochSummary
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

## Operations

### Peers
```rust
storage.store_peer("peer_id", &info)?;
let peer = storage.load_peer("peer_id")?;
storage.remove_peer("peer_id")?;
storage.store_peers_batch(&[(id, info), ...])?;  // bulk
let all = storage.list_peers()?;
```

### Metrics
```rust
storage.store_metrics(timestamp, &metrics)?;
let range = storage.load_metrics_range(start, end)?;
let recent = storage.load_latest_metrics(100)?;
storage.prune_metrics(older_than)?;  // cleanup
```

### Config
```rust
storage.store_config("key", &value)?;
let val: Option<T> = storage.load_config("key")?;
storage.delete_config("key")?;
```

### Secrets
```rust
storage.store_secret("key", &data)?;
let data = storage.load_secret("key")?;
storage.delete_secret("key")?;
```

## Maintenance

```rust
// Force flush to disk
storage.flush()?;

// Reclaim disk space
storage.compact()?;

// Check integrity
let report = storage.verify_integrity()?;
if report.corrupted_entries > 0 {
    // handle corruption
}

// Backup
storage.export_backup("/path/to/backup.nonos")?;
```

## Metrics

```rust
let m = storage.storage_metrics().snapshot();
println!("reads: {}, writes: {}", m.reads, m.writes);
println!("cache hit rate: {:.1}%",
    m.cache_hits as f64 / (m.cache_hits + m.cache_misses) as f64 * 100.0);

let sizes = storage.tree_sizes()?;
let disk = storage.size_on_disk()?;
```

## Modes

### Persistent (default)
```rust
let storage = NodeStorage::open(config)?;
```

### In-memory (testing)
```rust
let storage = NodeStorage::in_memory()?;
```

### Read-only (backup tools)
```rust
let storage = NodeStorage::open_readonly(config)?;
```

## Tuning

High write throughput:
```toml
[storage]
cache_capacity_bytes = 134217728  # 128MB
flush_every_ms = 60000            # 1 minute
compression_enabled = false
```

Low memory:
```toml
[storage]
cache_capacity_bytes = 16777216   # 16MB
flush_every_ms = 5000
compression_enabled = true
```

Maximum durability:
```toml
[storage]
flush_every_ms = 1000
```

## Directory Layout

```
~/.nonos/
├── data.sled/
│   ├── conf
│   ├── blobs/
│   └── db
├── identity.key
└── config.toml
```

## Errors

| Error | Cause | Fix |
|-------|-------|-----|
| Resource busy | Another process has the db | Kill other process |
| Schema version X > Y | DB from newer daemon | Upgrade daemon |
| Corruption detected | Disk issue or crash | Restore from backup |

## Troubleshooting

```bash
# Check disk space
df -h ~/.nonos/

# Check permissions
ls -la ~/.nonos/

# Prune old data
# (via API or programmatic call)
```

Recovery from corruption:
1. Stop daemon
2. Backup current data.sled/
3. Delete data.sled/
4. Restore from last good backup
5. Start daemon
