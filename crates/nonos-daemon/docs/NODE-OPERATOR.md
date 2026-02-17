# NONOS Node Operator Guide

This guide covers how to run, configure, and secure a NONOS daemon node.

## Quick Start

### Installation

```bash
# Build from source
cargo build --release -p nonos-daemon

# Binary location
./target/release/nonos-daemon
```

### First Run

```bash
# Start with default configuration
./nonos-daemon

# Start with custom config file
./nonos-daemon --config /path/to/config.toml

# Start with specific data directory
./nonos-daemon --data-dir ~/.nonos
```

## Configuration

### Config File Location

The daemon looks for configuration in this order:

1. Path specified via `--config` flag
2. `./config.toml` in current directory
3. `~/.nonos/config.toml`
4. `/etc/nonos/config.toml`

### Essential Configuration

```toml
# config.toml

[node]
# Node identifier (auto-generated if not set)
# peer_id = "12D3KooW..."

# Data directory for storage
data_dir = "~/.nonos"

[network]
# Node role determines connection limits and responsibilities
# Options: local (25 peers), relay (100 peers), backbone (500 peers)
role = "local"

# P2P listen address
listen_addr = "/ip4/0.0.0.0/tcp/9432"

# Bootstrap mode for peer discovery
# Options: official, custom, none
bootstrap_mode = "official"

# Custom bootstrap nodes (only used when bootstrap_mode = "custom")
# custom_bootstrap_nodes = [
#     "/ip4/1.2.3.4/tcp/9432/p2p/12D3KooW..."
# ]

[api]
# API server listen address
listen = "127.0.0.1:9433"

# Optional authentication token (HIGHLY RECOMMENDED for production)
# auth_token = "your-secure-random-token-here"

# Rate limiting
requests_per_second = 100
burst_size = 200

[rate_limit]
# P2P message rate limiting
enabled = true
p2p_messages_per_second = 100
p2p_burst_size = 200

[metrics]
# Prometheus metrics endpoint
enabled = true
listen = "127.0.0.1:9434"

[storage]
# Database path (relative to data_dir)
db_path = "data.sled"

# Cache size in bytes (default: 1GB)
cache_size = 1073741824

[logging]
# Log level: trace, debug, info, warn, error
level = "info"

# Log format: json, pretty
format = "pretty"
```

## Node Roles

Choose the appropriate role based on your resources and goals:

### Local Node (Default)

Best for: End users, lightweight operation

```toml
[network]
role = "local"
```

- Maximum 25 peer connections
- Minimal bandwidth usage
- Participates in network but doesn't relay heavily

### Relay Node

Best for: Community contributors, moderate resources

```toml
[network]
role = "relay"
```

- Maximum 100 peer connections
- Helps route traffic between nodes
- Recommended: 2+ CPU cores, 4GB+ RAM, 100Mbps+ connection

### Backbone Node

Best for: Infrastructure operators, high availability

```toml
[network]
role = "backbone"
```

- Maximum 500 peer connections
- Critical for network stability
- Recommended: 4+ CPU cores, 8GB+ RAM, 1Gbps+ connection, static IP

## Bootstrap Modes

### Official Bootstrap (Default)

Connects to NONOS-maintained bootstrap nodes:

```toml
[network]
bootstrap_mode = "official"
```

### Custom Bootstrap

Use your own bootstrap nodes (e.g., for private networks):

```toml
[network]
bootstrap_mode = "custom"
custom_bootstrap_nodes = [
    "/ip4/192.168.1.100/tcp/9432/p2p/12D3KooWAbCdEfGhIjKlMnOpQrStUvWxYz...",
    "/ip4/192.168.1.101/tcp/9432/p2p/12D3KooW123456789AbCdEfGhIjKlMnOp..."
]
```

### No Bootstrap (Isolated)

For testing or isolated networks:

```toml
[network]
bootstrap_mode = "none"
```

Note: You'll need to manually connect peers or use mDNS for local discovery.

## Securing the API

### Enable Authentication

**Always enable authentication in production!**

```toml
[api]
auth_token = "generate-a-secure-random-token"
```

Generate a secure token:

```bash
# Linux/macOS
openssl rand -hex 32

# Or use any secure random generator
head -c 32 /dev/urandom | base64
```

### Making Authenticated Requests

```bash
# Include the token in the Authorization header
curl -H "Authorization: Bearer your-token-here" \
     http://localhost:9433/api/status
```

### Rate Limiting

The API has built-in rate limiting:

```toml
[api]
requests_per_second = 100  # Sustained rate
burst_size = 200           # Allow bursts up to this
```

Rate-limited requests receive a `429 Too Many Requests` response.

### Network Binding

For security, bind to localhost only:

```toml
[api]
listen = "127.0.0.1:9433"  # Only local access
```

If you need remote access, use a reverse proxy with TLS:

```toml
[api]
listen = "0.0.0.0:9433"    # All interfaces (use with caution)
```

## Exposing Metrics

### Prometheus Integration

Enable the metrics endpoint:

```toml
[metrics]
enabled = true
listen = "127.0.0.1:9434"
```

### Available Metrics

```
# P2P metrics
nonos_p2p_peer_count                  # Current connected peers
nonos_p2p_bytes_sent_total            # Total bytes sent
nonos_p2p_bytes_received_total        # Total bytes received
nonos_p2p_messages_published_total    # Messages published
nonos_p2p_messages_received_total     # Messages received
nonos_p2p_messages_dropped_total      # Messages dropped
nonos_p2p_rate_limit_hits_total       # Rate limit violations
nonos_p2p_banned_peers                # Currently banned peers
nonos_p2p_connection_attempts_total   # Connection attempts
nonos_p2p_connection_failures_total   # Failed connections

# Supervisor metrics
nonos_supervisor_tasks_total          # Total managed tasks
nonos_supervisor_healthy_tasks        # Tasks in healthy state
nonos_supervisor_degraded_tasks       # Tasks in degraded state
nonos_supervisor_critical_tasks       # Tasks in critical state
nonos_supervisor_restarts_total       # Total task restarts

# Storage metrics
nonos_storage_size_bytes              # Database size
nonos_storage_keys_total              # Total stored keys
```

### Prometheus Configuration

Add to `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'nonos'
    static_configs:
      - targets: ['localhost:9434']
    scrape_interval: 15s
```

### Grafana Dashboard

Import the NONOS dashboard or create panels for key metrics:

- Peer count over time
- Message throughput
- Rate limit violations
- Task health status

## Firewall Configuration

### Required Ports

| Port | Protocol | Purpose | Exposure |
|------|----------|---------|----------|
| 9432 | TCP | P2P network | Public (required) |
| 9433 | TCP | API server | Private (localhost) |
| 9434 | TCP | Metrics | Private (monitoring) |

### iptables Example

```bash
# Allow P2P port from anywhere
iptables -A INPUT -p tcp --dport 9432 -j ACCEPT

# Allow API only from localhost
iptables -A INPUT -p tcp --dport 9433 -s 127.0.0.1 -j ACCEPT
iptables -A INPUT -p tcp --dport 9433 -j DROP

# Allow metrics from monitoring network
iptables -A INPUT -p tcp --dport 9434 -s 10.0.0.0/8 -j ACCEPT
iptables -A INPUT -p tcp --dport 9434 -j DROP
```

### UFW Example

```bash
# Allow P2P
ufw allow 9432/tcp

# API and metrics stay blocked (localhost only)
```

## Running as a Service

### systemd Service

Create `/etc/systemd/system/nonos.service`:

```ini
[Unit]
Description=NONOS Daemon
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=nonos
Group=nonos
ExecStart=/usr/local/bin/nonos-daemon --config /etc/nonos/config.toml
Restart=always
RestartSec=5
LimitNOFILE=65535

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true
ReadWritePaths=/var/lib/nonos

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
systemctl daemon-reload
systemctl enable nonos
systemctl start nonos
```

### Docker

```dockerfile
FROM rust:1.75-slim as builder
WORKDIR /app
COPY . .
RUN cargo build --release -p nonos-daemon

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/nonos-daemon /usr/local/bin/
EXPOSE 9432 9433 9434
ENTRYPOINT ["nonos-daemon"]
```

```bash
docker run -d \
  --name nonos \
  -p 9432:9432 \
  -v nonos-data:/data \
  -e RUST_LOG=info \
  nonos-daemon --data-dir /data
```

## Health Monitoring

### API Health Check

```bash
curl http://localhost:9433/api/health
```

Response:

```json
{
  "healthy": true,
  "status": "Running",
  "peer_count": 42,
  "connection_rate": 0.95
}
```

### Supervisor Health

The supervisor tracks task health with three classifications:

- **Healthy**: Task running normally
- **Degraded**: Task has restarted recently but is stable
- **Critical**: Task is restarting too frequently (>5 times in 60 seconds)

Check via API:

```bash
curl -H "Authorization: Bearer $TOKEN" \
     http://localhost:9433/api/supervisor/stats
```

### Alerting Recommendations

Set up alerts for:

1. `peer_count < 5` - Network connectivity issues
2. `critical_tasks > 0` - Supervisor problems
3. `rate_limit_hits` increasing - Possible attack
4. `banned_peers` increasing - Network issues or attacks

## Troubleshooting

### No Peers Connecting

1. Check firewall allows port 9432:
   ```bash
   nc -zv your-ip 9432
   ```

2. Verify bootstrap mode:
   ```bash
   grep bootstrap_mode config.toml
   ```

3. Check logs for connection errors:
   ```bash
   RUST_LOG=nonos_daemon::p2p=debug ./nonos-daemon
   ```

### High Memory Usage

1. Reduce cache size:
   ```toml
   [storage]
   cache_size = 536870912  # 512MB
   ```

2. Lower peer limits:
   ```toml
   [network]
   role = "local"
   ```

### Peers Getting Banned

Check for violations in logs:

```bash
RUST_LOG=nonos_daemon::p2p=debug ./nonos-daemon 2>&1 | grep -i violation
```

Common causes:
- Network issues causing message corruption
- Misconfigured clients sending oversized messages
- Actual attack traffic

### API Not Responding

1. Check binding address:
   ```bash
   netstat -tlnp | grep 9433
   ```

2. Verify authentication:
   ```bash
   curl -v -H "Authorization: Bearer $TOKEN" http://localhost:9433/api/status
   ```

3. Check rate limiting:
   ```bash
   curl http://localhost:9433/api/rate-limit/stats
   ```

### Database Issues

1. Check disk space:
   ```bash
   df -h /path/to/data
   ```

2. Verify permissions:
   ```bash
   ls -la ~/.nonos/
   ```

3. Check for corruption (backup first!):
   ```bash
   # The daemon will attempt recovery on startup
   ./nonos-daemon --data-dir ~/.nonos
   ```

## Performance Tuning

### High-Traffic Nodes

```toml
[network]
role = "backbone"
max_connections = 500

[rate_limit]
p2p_messages_per_second = 500
p2p_burst_size = 1000

[storage]
cache_size = 4294967296  # 4GB
```

### Low-Resource Environments

```toml
[network]
role = "local"
max_connections = 10

[rate_limit]
p2p_messages_per_second = 50
p2p_burst_size = 100

[storage]
cache_size = 268435456  # 256MB
```

### File Descriptor Limits

For backbone nodes, increase limits:

```bash
# /etc/security/limits.conf
nonos soft nofile 65535
nonos hard nofile 65535
```

## Backup and Recovery

### Data Backup

```bash
# Stop the daemon first
systemctl stop nonos

# Backup data directory
tar -czf nonos-backup-$(date +%Y%m%d).tar.gz ~/.nonos/

# Restart
systemctl start nonos
```

### Recovery

```bash
# Stop daemon
systemctl stop nonos

# Restore from backup
rm -rf ~/.nonos/
tar -xzf nonos-backup-20240101.tar.gz -C ~/

# Start daemon
systemctl start nonos
```

### Key Files to Backup

- `~/.nonos/data.sled/` - Main database
- `~/.nonos/identity.key` - Node identity (if persisted)
- `/etc/nonos/config.toml` - Configuration

## Security Checklist

- [ ] API authentication enabled
- [ ] API bound to localhost or behind reverse proxy
- [ ] Metrics endpoint not publicly accessible
- [ ] Firewall configured correctly
- [ ] Running as non-root user
- [ ] systemd security options enabled
- [ ] Regular backups configured
- [ ] Monitoring and alerting set up
- [ ] Log rotation configured
- [ ] Updates applied regularly
