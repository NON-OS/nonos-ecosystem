# NONOS Daemon

Node daemon for the NONOS network.

## What it does

- P2P networking via libp2p (Kademlia DHT, GossipSub)
- Identity management with Poseidon commitments
- Tracking protection at the network level
- Content caching with Merkle-based verification
- Staking and rewards for node operators

## Install

```bash
cargo install --path crates/nonos-daemon
```

Or build from source:
```bash
cargo build --release -p nonos-daemon
```

## Quick Start

```bash
# Initialize config
nonos init --reward-address 0xYourAddress

# Start the daemon
nonos run

# Check status
nonos status
```

## Commands

### Node
```bash
nonos run            # start daemon
nonos status         # check if running
nonos info           # node information
nonos check          # diagnostics
nonos check --full   # include network tests
nonos stop           # stop daemon
nonos reload         # reload config
```

### Identity
```bash
nonos identity generate --label "main"
nonos identity list
nonos identity show <id>
nonos identity export <id> -o backup.dat
nonos identity import backup.dat
```

### Staking
```bash
nonos stake status
nonos stake deposit <amount>
nonos stake withdraw <amount>
nonos stake tier <name>
nonos stake tiers
```

### Rewards
```bash
nonos rewards status
nonos rewards claim
nonos rewards history
nonos rewards auto --threshold 100
```

### Network
```bash
nonos peers
nonos stats
nonos mixer status
```

### System
```bash
nonos config
nonos config validate
nonos systemd --user nonos
nonos version
```

## Config

Default location: `~/.nonos/config.toml`

```toml
[network]
listen_addr = "/ip4/0.0.0.0/tcp/9000"
bootstrap_nodes = []

[api]
bind_address = "127.0.0.1"
port = 8420
auth_token = "your-token"  # always set this

[services]
health_beacon = true
quality_oracle = true
cache = true
cache_size_mb = 512

[rewards]
reward_address = "0x..."
```

## Environment

| Variable | Description | Default |
|----------|-------------|---------|
| NONOS_DATA_DIR | Data directory | ~/.nonos |
| NONOS_API_PORT | HTTP API port | 8420 |
| NONOS_RPC_URL | Ethereum RPC | Base mainnet |
| NONOS_WALLET_KEY | Wallet key | - |
| NONOS_CHAIN_ID | Chain ID | 8453 |
| NONOS_LOG_LEVEL | Log level | info |

## API

Port 8420 by default.

| Endpoint | Method | Description |
|----------|--------|-------------|
| /api/status | GET | Node status |
| /api/metrics | GET | Metrics |
| /api/peers | GET | Peers |
| /api/privacy/stats | GET | Privacy stats |
| /api/staking/status | GET | Stake info |
| /api/rewards/pending | GET | Pending rewards |

Set auth_token and use:
```bash
curl -H "Authorization: Bearer $TOKEN" http://localhost:8420/api/status
```

## Tiers

| Tier | Stake | Multiplier |
|------|-------|------------|
| Bronze | 100 NOX | 1.0x |
| Silver | 1,000 NOX | 1.5x |
| Gold | 10,000 NOX | 2.0x |
| Platinum | 50,000 NOX | 3.0x |
| Diamond | 100,000 NOX | 5.0x |

## systemd

```bash
nonos systemd --user nonos --output-dir /etc/systemd/system
systemctl daemon-reload
systemctl enable nonos
systemctl start nonos
```

## Architecture

```
src/
├── main.rs         # entry point
├── cli/            # commands
├── api/            # HTTP server
├── p2p/            # networking
├── contracts/      # blockchain
├── services/       # background services
└── privacy/        # privacy features
```

## Security

- Keys stored in ~/.nonos/identities/
- Backup identity files
- Never share wallet keys
- API binds to localhost by default
- Always set auth_token

## Docs

- [Network Architecture](docs/ARCHITECTURE-NETWORK.md)
- [Node Operator Guide](docs/NODE-OPERATOR.md)
- [Storage](docs/STORAGE.md)

## License

GNU Affero General Public License v3.0
