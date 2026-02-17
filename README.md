# NONOS Ecosystem

Privacy browser, wallet, and decentralized node network. All traffic routes through the Anyone Network. Zero-knowledge proofs for identity without revealing who you are.

## The Problem

Browsers leak everything. Your IP, your history, your identity. Even "private" modes leave traces. Wallets expose your transaction graph. There's no way to prove you belong to a group without revealing which member you are.

## The Solution

NONOS is an ecosystem of tools that work together:

- **Browser** - Desktop app that routes all traffic through onion relays. No direct connections. DNS resolved at exit nodes, not locally.
- **Daemon** - Background service handling wallet operations, ZK identity proofs, cache mixing, and P2P networking.
- **Wallet** - HD wallet with stealth addresses (EIP-5564). Transactions can't be linked back to you.
- **Staking Contracts** - On-chain staking for node operators. Rewards based on uptime and stake.
- **Dashboard** - Terminal UI for monitoring your node.


## Project Structure

```
nonos-ecosystem/
│
├── crates/
│   ├── nonos-types       Shared types, errors, constants
│   ├── nonos-crypto      Poseidon hash, Groth16 proofs, secp256k1, Ed25519, AES-GCM
│   ├── nonos-wallet      HD derivation (BIP39/32/44), stealth addresses, encrypted storage
│   ├── nonos-anyone      Anyone Network client, SOCKS5 proxy management
│   ├── nonos-daemon      P2P networking, REST API, privacy services, node operations
│   └── nonos-dash        Terminal dashboard for node monitoring
│
├── ui/                   Desktop browser (Tauri + Svelte)
├── contracts/            Solidity staking contracts (Foundry)
└── docs/                 Architecture, threat model, security docs
```

## Cryptography

**Wallet**
- Key derivation: Argon2id (64 MiB memory, 3 iterations, 4 parallel lanes)
- Encryption: AES-256-GCM authenticated encryption
- Signatures: secp256k1 ECDSA
- HD paths: BIP44 (m/44'/60'/0'/0/x)

**Identity Proofs**
- Proving system: Groth16 SNARKs
- Curve: BN254 (alt_bn128)
- Hash: Poseidon over scalar field
- Merkle tree: depth 20, Poseidon hash
- Nullifiers: scope-bound to prevent cross-context replay

**Network**
- P2P identity: Ed25519
- Transport: libp2p with Noise protocol encryption
- Discovery: Kademlia DHT
- Messaging: Gossipsub

## Staking

Node operators stake NOX tokens to participate in the network.

| Tier | Stake | Lock Period | Multiplier |
|------|-------|-------------|------------|
| Bronze | 1,000 NOX | None | 1.0x |
| Silver | 10,000 NOX | 30 days | 1.2x |
| Gold | 50,000 NOX | 90 days | 1.5x |
| Platinum | 200,000 NOX | 180 days | 2.0x |
| Diamond | 1,000,000 NOX | 365 days | 3.0x |

Rewards scale with `sqrt(stake)` to limit whale dominance. A node with 4x the stake gets 2x the rewards, not 4x.

## Network Ports

| Port | Binding | Purpose |
|------|---------|---------|
| 9432 | 0.0.0.0 | P2P node communication |
| 8420 | 127.0.0.1 | REST API (local only) |
| 9050 | 127.0.0.1 | SOCKS5 proxy (Anyone Network) |
| 9051 | 127.0.0.1 | Anyone control port |

## Quick Start

**Build everything**
```bash
cargo build --release
```

**Initialize node**
```bash
./target/release/nonos init --nickname mynode --reward-address 0x...
```

**Start daemon**
```bash
./target/release/nonos run
```

**Start browser**
```bash
cd ui && npm install && npm run tauri dev
```

## Commands

```bash
# Node lifecycle
nonos init                  Create config and keys
nonos run                   Start daemon
nonos stop                  Stop daemon
nonos restart               Restart daemon
nonos status                Check if running

# Identity
nonos identity generate     Create ZK identity
nonos identity prove        Generate membership proof
nonos identity show         Display identity info

# Network
nonos peers list            Show connected peers
nonos peers add <addr>      Add peer manually
nonos stats                 Network statistics

# Diagnostics
nonos check                 Basic health check
nonos check --full          Extended diagnostics
nonos dash                  Launch terminal UI
```

## Configuration

File: `~/.nonos/config.toml`

```toml
[node]
nickname = "mynode"
reward_address = "0x..."

[p2p]
listen_port = 9432
bootstrap_peers = [
    "/ip4/102.211.56.24/tcp/9432/p2p/12D3KooW..."
]
max_peers = 50

[api]
bind_address = "127.0.0.1"
port = 8420
auth_enabled = true

[privacy]
tracking_blocker = true
cache_mixing = true
zk_sessions = true

[logging]
level = "info"
file = "~/.nonos/logs/daemon.log"
```

## Requirements

- Rust 1.75+
- Node.js 18+
- 8 GB RAM minimum (16 GB recommended for ZK proofs)
- WebKit (Linux/macOS) or WebView2 (Windows)

## Documentation

| Document | Description |
|----------|-------------|
| [Architecture](docs/architecture.md) | System design, data flows, component interaction |
| [Threat Model](docs/threat-model.md) | Adversaries, attack surfaces, mitigations |
| [Security Invariants](docs/security-invariants.md) | Properties that must hold, test coverage |
| [Release Guide](docs/release-beta.md) | Build verification, beta checklist |

## Security

Report vulnerabilities to **team@nonos.systems**

Do not open public issues for security bugs. We have a bug bounty program for responsible disclosure.

## License

AGPL-3.0

## Links

- Website: https://nonos.systems
- GitHub: https://github.com/NON-OS/nonos-ecosystem
- Docs: https://docs.nonos.systems
