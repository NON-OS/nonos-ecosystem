# Network Architecture

The NONOS daemon runs a libp2p-based P2P network. This doc covers how it works.

## Stack

| Protocol | Purpose |
|----------|---------|
| TCP | Transport |
| Noise XX | Encryption |
| Yamux | Multiplexing |
| Gossipsub | Pub/sub messaging |
| Kademlia | Peer discovery via DHT |
| Identify | Capability exchange |
| Ping | Liveness |

All connections use Noise encryption. The XX handshake provides mutual authentication.

## Node Roles

```toml
[network]
role = "local"  # local, relay, or backbone
```

| Role | Max Peers | Use Case |
|------|-----------|----------|
| Local | 25 | End users |
| Relay | 100 | Traffic routing, moderate resources |
| Backbone | 500 | Infrastructure, requires static IP |

The role sets connection limits. Relay and backbone nodes help route traffic for NAT'd local nodes.

## Bootstrap

Three modes:

```toml
[network]
bootstrap_mode = "official"  # official, custom, or none

# custom_bootstrap_nodes = [
#     "/ip4/x.x.x.x/tcp/9432/p2p/12D3KooW..."
# ]
```

**official** - NONOS-operated bootstrap nodes. Fast but centralized.
**custom** - Your own bootstrap nodes. Decentralized if you run multiple.
**none** - No bootstrap. For lab networks with mDNS.

For decentralized operation, run custom mode with bootstrap nodes from independent operators.

## Message Handling

Inbound messages go through these checks in order:

1. **Ban check** - drop if peer banned
2. **Size check** - 64KB max, penalty if exceeded
3. **Rate limit** - token bucket per peer
4. **Spam detection** - auto-ban at 50 msg/sec sustained
5. **Handler dispatch**

Messages that fail checks get dropped and the peer takes a penalty.

## Rate Limiting

Token bucket per peer:

```toml
[rate_limit]
enabled = true
p2p_messages_per_second = 100
p2p_burst_size = 200
```

The bucket refills continuously. Burst allows temporary spikes.

## Reputation & Banning

Violations add to a penalty score:

| Violation | Penalty |
|-----------|---------|
| Oversized message | 15 |
| Decode failure | 10 |
| Malformed content | 20 |
| Rate limit hit | 10 |
| Spam behavior | 25 |
| Protocol violation | 30 |

Repeat offenders get multiplied penalties (2x-5x depending on type).

**Ban triggers:**
- Reputation below -50
- 10+ violations
- Penalty score >= 100
- Protocol violations (immediate)

**Ban duration:**
- Normal: 5 minutes
- Repeat: 30 minutes
- Spam: 1 hour
- Protocol: 100 minutes

Bans expire automatically. The peer can reconnect after expiry.

## Peer Store

Each peer has:

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

Quality score weights:
- Latency: 30%
- Success rate: 40%
- Uptime: 30%

Peer selection filters out banned and low-reputation peers, then sorts by quality.

## Topics

Gossipsub topics:

| Topic | Purpose |
|-------|---------|
| `/nonos/announce/1.0.0` | Node announcements |
| `/nonos/health/1.0.0` | Health beacons |
| `/nonos/peers/1.0.0` | Peer exchange |
| `/nonos/privacy/1.0.0` | Privacy service coordination |

Subscribe to topics on connection. Messages propagate across subscribed peers.

## DHT

Kademlia DHT for:
- Finding peers by ID
- Locating service providers
- Bootstrap queries

Queries retry with exponential backoff: 1s, 2s, 4s, 8s, 16s (max 5 attempts).

## Connections

```toml
[network]
max_connections = 100
max_inbound = 50
max_outbound = 50
```

New connections go through:
1. Ban list check
2. Connection limit check
3. Identify exchange
4. Peer store update
5. Topic subscription

Disconnect cleanup:
1. Send disconnect notification
2. Remove rate limiter state
3. Update peer store
4. Remove from active list

## Security

**Message validation:**
- Size limit (64KB)
- Format validation
- Signature check where applicable
- Timestamp freshness

**DoS protection:**
- Per-peer rate limiting
- Connection limits
- Auto-ban on violations
- Circuit breaker for failing peers

**Privacy:**
- Encrypted connections (Noise)
- Optional peer ID rotation
- No IP logging by default

## Metrics

Prometheus metrics on `/api/metrics/prometheus`:

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

Health check at `/api/health`:

```json
{
  "healthy": true,
  "status": "Running",
  "peer_count": 42
}
```

## Troubleshooting

**No peers:**
- Check port 9432 is open
- Verify bootstrap_mode isn't "none"
- Check network connectivity

**High rate limit hits:**
- Review which peers are triggering limits
- Increase limit if traffic is legitimate

**Peers getting banned:**
- Check logs for violation types
- Look for attack patterns

Debug logging:

```bash
RUST_LOG=nonos_daemon::p2p=debug ./nonos-daemon
```
