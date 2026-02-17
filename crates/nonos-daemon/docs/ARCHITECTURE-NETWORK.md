# NONOS Network Architecture

This document describes how the NONOS daemon operates as a decentralized P2P network node.

## Overview

The NONOS daemon implements a decentralized peer-to-peer network using libp2p. Each node can operate in one of three roles with configurable bootstrap modes, forming a resilient mesh network for privacy-preserving operations.

## Network Stack

### Transport Layer

The network uses libp2p with the following protocols:

- **TCP Transport**: Primary transport for peer connections
- **Noise Protocol**: Encrypted communication using the XX handshake pattern
- **Yamux**: Stream multiplexing for concurrent connections
- **Gossipsub**: Pub/sub messaging for network-wide announcements
- **Kademlia DHT**: Distributed hash table for peer discovery
- **Identify**: Protocol for peer identification and capability exchange
- **Ping**: Liveness detection for connected peers

### Node Roles

Nodes operate in one of three roles with different connection limits:

| Role | Max Peers | Description |
|------|-----------|-------------|
| **Local** | 25 | End-user nodes with minimal resource requirements |
| **Relay** | 100 | Intermediate nodes that help route traffic |
| **Backbone** | 500 | Infrastructure nodes that maintain network stability |

Configure the role in `config.toml`:

```toml
[network]
role = "local"  # Options: local, relay, backbone
```

### Bootstrap Modes

Three bootstrap modes control how nodes discover the network:

| Mode | Description |
|------|-------------|
| **Official** | Connect to NONOS official bootstrap nodes (default) |
| **Custom** | Use only user-specified bootstrap peers |
| **None** | No bootstrap - for isolated/lab networks |

```toml
[network]
bootstrap_mode = "official"

# For custom mode:
# bootstrap_mode = "custom"
# custom_bootstrap_nodes = [
#     "/ip4/192.168.1.100/tcp/9432/p2p/12D3KooW..."
# ]
```

## Message Flow

### Inbound Message Processing

All inbound messages pass through multiple safety layers:

```
Receive Message
    │
    ├─► Ban Check
    │   └─► Drop if peer is banned
    │
    ├─► Size Check (64KB max)
    │   └─► Penalty + potential ban if oversized
    │
    ├─► Rate Limit Check
    │   └─► Token bucket per peer (100 msg/sec default)
    │   └─► Penalty for violations
    │
    ├─► Spam Detection
    │   └─► Auto-ban for >50 msg/sec sustained
    │
    └─► Forward to Handler
```

### Rate Limiting

Per-peer rate limiting uses a token bucket algorithm:

- **Default rate**: 100 messages/second
- **Burst capacity**: 2x rate (200 messages)
- **Byte limit**: 1 MB/second
- **Refill rate**: Continuous based on elapsed time

Configure in `config.toml`:

```toml
[rate_limit]
enabled = true
p2p_messages_per_second = 100
p2p_burst_size = 200
```

### Penalty System

Violations accumulate penalty scores that affect peer reputation:

| Violation | Base Penalty | Repeat Multiplier |
|-----------|-------------|-------------------|
| Oversized Message | 15 | 2x |
| Decode Failure | 10 | 1x |
| Malformed Content | 20 | 3x |
| Unexpected Type | 5 | 1x |
| Rate Limit Exceeded | 10 | 2x |
| Spam Behavior | 25 | 4x |
| Protocol Violation | 30 | 5x |

**Auto-ban triggers:**
- Reputation drops below -50
- 10+ total violations
- Penalty score reaches 100
- Immediate for protocol violations or spam

**Ban durations:**
- First offense: 5 minutes
- Repeat offender: 30 minutes
- Spam behavior: 1 hour
- Protocol violation: 100 minutes

## Peer Store

The `PeerStore` maintains peer metadata and reputation:

```rust
PeerEntry {
    peer_id: PeerId,
    addresses: Vec<Multiaddr>,
    last_seen: Timestamp,
    reputation: i32,      // -100 to +100
    quality_score: f64,   // 0.0 to 1.0
    is_bootstrap: bool,
    banned_until: Option<Timestamp>,
}
```

### Quality Scoring

Quality score affects peer selection priority:

- **Latency weight**: 30% - Lower latency = higher score
- **Success rate weight**: 40% - Successful interactions
- **Uptime weight**: 30% - Connection stability

### Peer Selection

When selecting peers for operations:

1. Filter out banned peers
2. Filter peers with reputation < 0
3. Sort by quality score
4. Apply role-based limits

## Network Events

The daemon emits events for network state changes:

| Event | Description |
|-------|-------------|
| `PeerConnected` | New peer connection established |
| `PeerDisconnected` | Peer connection closed |
| `Message` | Message received on a topic |
| `PeerDiscovered` | New peer found via DHT |
| `PingResult` | Latency measurement completed |
| `RateLimited` | Peer hit rate limit |
| `PeerBanned` | Peer was banned |

## Gossipsub Topics

The network uses topic-based pub/sub for different message types:

| Topic | Purpose |
|-------|---------|
| `/nonos/announce/1.0.0` | Node announcements |
| `/nonos/health/1.0.0` | Health beacons |
| `/nonos/peers/1.0.0` | Peer exchange |
| `/nonos/privacy/1.0.0` | Privacy service coordination |

## DHT Operations

Kademlia DHT is used for:

1. **Peer Discovery**: Find nodes by ID
2. **Content Routing**: Locate service providers
3. **Bootstrap**: Initial network join

DHT queries have built-in retry with exponential backoff:

- Initial delay: 1 second
- Max delay: 60 seconds
- Max attempts: 5

## Connection Management

### Connection Limits

Configurable connection limits per role:

```toml
[network]
max_connections = 100
max_inbound = 50
max_outbound = 50
```

### Connection Lifecycle

```
New Connection
    │
    ├─► Check ban list
    ├─► Check connection limits
    ├─► Identify exchange
    ├─► Add to peer store
    └─► Subscribe to topics
```

### Graceful Disconnect

When disconnecting:

1. Send disconnect notification
2. Clean up rate limiters
3. Update peer store
4. Remove from active peers

## Security Considerations

### Message Validation

All messages are validated before processing:

- **Size check**: Reject > 64KB
- **Format check**: Valid protobuf/JSON
- **Signature check**: For signed messages
- **Timestamp check**: Reject stale messages

### DDoS Protection

Built-in protections against denial of service:

1. Per-peer rate limiting
2. Connection limits
3. Auto-ban for violations
4. Circuit breaker for failing peers

### Privacy

Network-level privacy measures:

- No IP logging by default
- Encrypted connections (Noise)
- Peer ID rotation support
- Mixnet integration for traffic analysis resistance

## Monitoring

### Metrics

Network metrics exposed via Prometheus:

```
nonos_p2p_peer_count
nonos_p2p_bytes_sent_total
nonos_p2p_bytes_received_total
nonos_p2p_messages_published_total
nonos_p2p_messages_received_total
nonos_p2p_messages_dropped_total
nonos_p2p_rate_limit_hits_total
nonos_p2p_banned_peers
nonos_p2p_connection_attempts_total
nonos_p2p_connection_failures_total
```

### Health Checks

Network health available at `/api/health`:

```json
{
  "healthy": true,
  "status": "Running",
  "peer_count": 42,
  "connection_rate": 0.95
}
```

## Troubleshooting

### Common Issues

**No peers connecting:**
- Check firewall allows port 9432 (default)
- Verify bootstrap mode is not "none"
- Check network connectivity

**High rate limit hits:**
- Check for misbehaving peers
- Increase `p2p_messages_per_second` if legitimate

**Peers getting banned:**
- Review logs for violation types
- Check for network attacks
- Verify message sizes are within limits

### Debug Logging

Enable debug logging for network:

```bash
RUST_LOG=nonos_daemon::p2p=debug ./nonos-daemon
```
