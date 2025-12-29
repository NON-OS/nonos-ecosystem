# Node Operator Guide

How to run and secure a NONOS node.

## Install

```bash
cargo build --release -p nonos-daemon
./target/release/nonos-daemon
```

## Config

The daemon checks these locations in order:
1. `--config` flag
2. `./config.toml`
3. `~/.nonos/config.toml`
4. `/etc/nonos/config.toml`

### Minimal config

```toml
[network]
role = "local"
listen_addr = "/ip4/0.0.0.0/tcp/9432"
bootstrap_mode = "official"

[api]
listen = "127.0.0.1:9433"
auth_token = "your-token-here"  # generate with: openssl rand -hex 32

[rate_limit]
enabled = true
p2p_messages_per_second = 100
p2p_burst_size = 200

[metrics]
enabled = true
listen = "127.0.0.1:9434"

[storage]
db_path = "data.sled"
cache_size = 1073741824

[logging]
level = "info"
format = "pretty"
```

## Roles

| Role | Max Peers | Hardware |
|------|-----------|----------|
| local | 25 | Minimal |
| relay | 100 | 2 cores, 4GB RAM, 100Mbps |
| backbone | 500 | 4 cores, 8GB RAM, 1Gbps, static IP |

```toml
[network]
role = "relay"
```

## Bootstrap

```toml
[network]
# official - NONOS nodes (fast, centralized)
# custom - your own nodes (decentralized)
# none - manual connections only
bootstrap_mode = "official"

# For custom mode:
# custom_bootstrap_nodes = [
#     "/ip4/x.x.x.x/tcp/9432/p2p/12D3KooW..."
# ]
```

For decentralized operation, use custom mode with bootstrap nodes from independent operators.

## API Security

**Always set auth_token.** Without it, anyone with network access can hit your API.

```toml
[api]
listen = "127.0.0.1:9433"
auth_token = "your-64-char-hex-token"
requests_per_second = 100
burst_size = 200
```

Generate a token:
```bash
openssl rand -hex 32
```

Use it:
```bash
curl -H "Authorization: Bearer your-token" http://localhost:9433/api/status
```

## Ports

| Port | Purpose | Exposure |
|------|---------|----------|
| 9432 | P2P | Public |
| 9433 | API | Localhost |
| 9434 | Metrics | Localhost |

Firewall:
```bash
# P2P open
iptables -A INPUT -p tcp --dport 9432 -j ACCEPT

# API localhost only
iptables -A INPUT -p tcp --dport 9433 -s 127.0.0.1 -j ACCEPT
iptables -A INPUT -p tcp --dport 9433 -j DROP

# Metrics from monitoring network
iptables -A INPUT -p tcp --dport 9434 -s 10.0.0.0/8 -j ACCEPT
iptables -A INPUT -p tcp --dport 9434 -j DROP
```

## systemd

```ini
# /etc/systemd/system/nonos.service
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

NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true
ReadWritePaths=/var/lib/nonos

[Install]
WantedBy=multi-user.target
```

```bash
systemctl daemon-reload
systemctl enable nonos
systemctl start nonos
```

## Docker

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
docker run -d --name nonos -p 9432:9432 -v nonos-data:/data nonos-daemon --data-dir /data
```

## Metrics

Prometheus scrape config:
```yaml
scrape_configs:
  - job_name: 'nonos'
    static_configs:
      - targets: ['localhost:9434']
    scrape_interval: 15s
```

Key metrics:
```
nonos_p2p_peer_count
nonos_p2p_bytes_sent_total
nonos_p2p_bytes_received_total
nonos_p2p_messages_published_total
nonos_p2p_rate_limit_hits_total
nonos_p2p_banned_peers
nonos_supervisor_healthy_tasks
nonos_supervisor_critical_tasks
```

## Health Check

```bash
curl http://localhost:9433/api/health
```

```json
{
  "healthy": true,
  "status": "Running",
  "peer_count": 42
}
```

## Alerts

Watch for:
- `peer_count < 5` - connectivity issues
- `critical_tasks > 0` - supervisor problems
- `rate_limit_hits` climbing - possible attack
- `banned_peers` climbing - network issues

## Troubleshooting

**No peers:**
```bash
nc -zv your-ip 9432            # check port open
grep bootstrap_mode config.toml # check mode
RUST_LOG=nonos_daemon::p2p=debug ./nonos-daemon  # debug logs
```

**High memory:**
```toml
[storage]
cache_size = 536870912  # 512MB
[network]
role = "local"
```

**API issues:**
```bash
netstat -tlnp | grep 9433
curl -v -H "Authorization: Bearer $TOKEN" http://localhost:9433/api/status
```

**Database issues:**
```bash
df -h /path/to/data    # disk space
ls -la ~/.nonos/       # permissions
```

## Tuning

High traffic:
```toml
[network]
role = "backbone"

[rate_limit]
p2p_messages_per_second = 500
p2p_burst_size = 1000

[storage]
cache_size = 4294967296  # 4GB
```

Low resources:
```toml
[network]
role = "local"
max_connections = 10

[rate_limit]
p2p_messages_per_second = 50

[storage]
cache_size = 268435456  # 256MB
```

Backbone file descriptors:
```bash
# /etc/security/limits.conf
nonos soft nofile 65535
nonos hard nofile 65535
```

## Backup

```bash
systemctl stop nonos
tar -czf nonos-backup-$(date +%Y%m%d).tar.gz ~/.nonos/
systemctl start nonos
```

Restore:
```bash
systemctl stop nonos
rm -rf ~/.nonos/
tar -xzf nonos-backup.tar.gz -C ~/
systemctl start nonos
```

Key files:
- `~/.nonos/data.sled/` - database
- `~/.nonos/identity.key` - node identity
- `/etc/nonos/config.toml` - config

## Checklist

- [ ] auth_token set
- [ ] API on localhost or behind proxy
- [ ] Metrics not public
- [ ] Firewall configured
- [ ] Running as non-root
- [ ] systemd hardening enabled
- [ ] Backups scheduled
- [ ] Monitoring in place
- [ ] Logs rotating
