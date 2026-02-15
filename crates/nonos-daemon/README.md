# nonos-daemon

Local service daemon for NONOS browser.

Provides ZK identity management, cache mixing, tracking protection, and wallet API. Traffic routing is handled by Anyone Protocol separately.

## Services

| Service | Description |
|---------|-------------|
| ZK Identity | Zero-knowledge identity generation and proofs |
| Cache Mixer | Privacy-preserving content cache with Merkle proofs |
| Tracking Blocker | Domain-level tracking protection |
| Wallet API | HTTP API for wallet operations |
| Staking | NOX token staking and rewards |

## Quick Start

```bash
nonos-daemon init
nonos-daemon run
```

## Commands

**Daemon**
```bash
nonos-daemon run           # Start daemon
nonos-daemon info          # Show status
nonos-daemon check         # Diagnostics
```

**Identity**
```bash
nonos-daemon identity generate --label "Main"
nonos-daemon identity list
nonos-daemon identity prove <id>
```

**Staking**
```bash
nonos-daemon stake status
nonos-daemon stake deposit <amount>
nonos-daemon rewards claim
```

**Mixer**
```bash
nonos-daemon mixer status
```

## API

HTTP API on `127.0.0.1:8420`:

| Endpoint | Description |
|----------|-------------|
| `GET /api/status` | Daemon status |
| `POST /api/identity/generate` | Create ZK identity |
| `POST /api/identity/prove` | Generate ZK proof |
| `GET /api/privacy/stats` | Privacy service stats |
| `GET /api/staking/status` | Staking info |
| `GET /api/rewards/pending` | Pending rewards |

## Configuration

`~/.nonos/config.toml`

```toml
[api]
bind = "127.0.0.1"
port = 8420

[services]
zk_identity = true
cache_mixer = true
tracking_blocker = true

[rewards]
reward_address = "0x..."
auto_claim = true
```

## Staking Tiers

| Tier | Minimum | Multiplier |
|------|---------|------------|
| Bronze | 1,000 NOX | 1.0x |
| Silver | 10,000 NOX | 1.5x |
| Gold | 50,000 NOX | 2.0x |
| Platinum | 200,000 NOX | 3.0x |
| Diamond | 1,000,000 NOX | 5.0x |

## Architecture

```
src/
├── cli/          Command handlers
├── api/          HTTP API
├── privacy/      ZK identity, cache mixer, tracking blocker
├── services/     Background services
├── storage/      Local persistence
├── contracts/    Staking contract clients
└── config/       Configuration
```

## License

AGPL-3.0
