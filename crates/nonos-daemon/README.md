# NONOS Daemon

Decentralized node daemon powering the NONOS browser ecosystem.

## Overview

NONOS is a privacy-focused daemon that participates in the decentralized NONOS network, providing:

- **ZK Identity Engine** - Generate and manage zero-knowledge identities using Poseidon commitments
- **Cache Mixer** - Privacy-preserving content caching with Poseidon Merkle trees
- **Tracking Blocker** - Network-level tracking protection
- **P2P Network** - libp2p-based distributed network (Kademlia DHT + GossipSub)
- **Staking & Rewards** - Earn NOX tokens by running a node

## Quick Start

```bash
# Initialize node configuration
nonos init --reward-address 0xYourEthAddress

# Generate a ZK identity
nonos identity generate --label "My Identity"

# Start the daemon
nonos run

# In another terminal, launch the dashboard
nonos dash
```

## Installation

### From Source

```bash
cargo install --path crates/nonos-daemon
```

### Binary Releases

Download pre-built binaries from the [releases page](https://github.com/NON-OS/nonos/releases).

## Commands

### Node Operations

```bash
nonos run                  # Start the daemon
nonos status               # Check if daemon is running
nonos info                 # Show node information
nonos check                # Run diagnostic checks
nonos check --full         # Include network connectivity tests
nonos stop                 # Stop running daemon
nonos reload               # Reload configuration
```

### ZK Identity Management

```bash
nonos identity generate --label "Primary"  # Create new identity
nonos identity list                        # List all identities
nonos identity show <id>                   # Show identity details
nonos identity export <id> -o backup.dat   # Export for backup
nonos identity import backup.dat           # Import from backup
nonos identity prove <id>                  # Generate ZK proof
nonos identity verify <proof>              # Verify a proof
nonos identity register <id>               # Register on-chain
```

### Staking

```bash
nonos stake status           # Show stake and tier
nonos stake deposit <amount> # Stake NOX tokens
nonos stake withdraw <amount># Withdraw staked tokens
nonos stake tier <name>      # Set node tier
nonos stake tiers            # Show tier requirements
```

### Rewards

```bash
nonos rewards status         # Show pending rewards
nonos rewards claim          # Claim pending rewards
nonos rewards history        # Show reward history
nonos rewards auto --threshold 100  # Configure auto-claim
```

### Network

```bash
nonos peers                  # List connected peers
nonos stats                  # Show network statistics
nonos mixer status           # Show cache mixer stats
```

### Configuration

```bash
nonos config                 # Show current configuration
nonos config validate        # Validate config file
```

### System Integration

```bash
nonos systemd --user nonos   # Generate systemd service file
nonos version                # Show version and build info
```

## Configuration

Default configuration is stored at `~/.nonos/config.toml`. Create it with `nonos init` or manually:

```toml
[network]
listen_addr = "/ip4/0.0.0.0/tcp/9000"
bootstrap_nodes = []

[api]
bind_address = "127.0.0.1"
port = 8420

[services]
health_beacon = true
quality_oracle = true
bootstrap = false
cache = true
cache_size_mb = 512

[rewards]
reward_address = "0x0000000000000000000000000000000000000000"
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `NONOS_DATA_DIR` | Data directory path | `~/.nonos` |
| `NONOS_API_PORT` | HTTP API port | `8420` |
| `NONOS_RPC_URL` | Ethereum RPC endpoint | `https://mainnet.base.org` |
| `NONOS_WALLET_KEY` | Wallet private key (for staking) | - |
| `NONOS_CHAIN_ID` | Blockchain chain ID | `8453` |
| `NONOS_LOG_LEVEL` | Log level | `info` |

## Architecture

```
nonos-daemon/
├── src/
│   ├── main.rs           # Entry point
│   ├── lib.rs            # Library exports
│   ├── cli/              # CLI command handlers
│   ├── api/              # HTTP API server
│   ├── p2p/              # P2P networking (libp2p)
│   ├── contracts/        # Blockchain contract clients
│   ├── services/         # Node services
│   └── privacy/          # Privacy services (ZK, mixer)
```

## API Endpoints

The daemon exposes an HTTP API on port 8420:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/status` | GET | Node status |
| `/api/metrics` | GET | Node metrics |
| `/api/peers` | GET | Connected peers |
| `/api/privacy/stats` | GET | Privacy service statistics |
| `/api/identity/generate` | POST | Generate ZK identity |
| `/api/identity/prove` | POST | Generate ZK proof |
| `/api/staking/status` | GET | Staking status |
| `/api/rewards/pending` | GET | Pending rewards |

## Node Tiers

| Tier | Min Stake | Reward Multiplier |
|------|-----------|-------------------|
| Bronze | 100 NOX | 1.0x |
| Silver | 1,000 NOX | 1.5x |
| Gold | 10,000 NOX | 2.0x |
| Platinum | 50,000 NOX | 3.0x |
| Diamond | 100,000 NOX | 5.0x |

## Systemd Integration

Generate and install the systemd service:

```bash
# Generate service file
nonos systemd --user nonos --output-dir /etc/systemd/system

# Enable and start
sudo systemctl daemon-reload
sudo systemctl enable nonos
sudo systemctl start nonos

# Check status
sudo systemctl status nonos
journalctl -u nonos -f
```

## Security

- Private keys are stored in `~/.nonos/identities/`
- Always backup your identity files securely
- Never share your wallet private key
- The daemon only binds to localhost by default

## License

GNU Affero General Public License v3.0 (AGPL-3.0)

Copyright (C) 2025 NON-OS <team@nonos.systems>
