# NONOS Ecosystem Architecture

## Design Goals

1. **No IP leaks** - Browser never connects directly to internet
2. **No DNS leaks** - All resolution through SOCKS5h at exit nodes
3. **Unlinkable transactions** - Stealth addresses break the transaction graph
4. **Anonymous proofs** - Prove group membership without revealing identity
5. **Local-first** - All sensitive operations happen on user's machine

## System Overview

The ecosystem has five main components:

**Browser** (ui/)
Tauri + Svelte desktop app. Renders web content. All network requests go to daemon at localhost:8420. Never opens sockets to external hosts.

**Daemon** (crates/nonos-daemon)
Background service. Handles P2P networking, wallet API, ZK identity proofs, cache mixing. Routes traffic through Anyone Network. Exposes REST API on localhost.

**Wallet** (crates/nonos-wallet)
HD wallet implementation. BIP39 mnemonics, BIP32 derivation, BIP44 paths. Stealth addresses for unlinkable payments. Encrypted at rest with Argon2id + AES-256-GCM.

**Contracts** (contracts/)
Solidity smart contracts for staking. Node operators lock NOX tokens, earn rewards based on uptime. Foundry for testing and deployment.

**Dashboard** (crates/nonos-dash)
Terminal UI built with ratatui. Shows node status, peer connections, network metrics, reward earnings.

## Data Flow

### Web Browsing

```
User clicks link
    │
    ▼
Browser (Tauri WebView)
    │ HTTP request to localhost:8420/proxy
    ▼
Daemon
    │ Forward through SOCKS5 proxy
    ▼
Anyone Network Client
    │ Build 3-hop circuit
    ▼
Entry Relay → Middle Relay → Exit Relay
    │
    ▼
Destination website
```

DNS never touches the local resolver. Domain names sent to exit relay for resolution (SOCKS5h).

### Wallet Transaction

```
User initiates send
    │
    ▼
Browser → Daemon API: /wallet/send
    │
    ▼
Daemon
    ├── Generate stealth address for recipient
    ├── Build transaction
    ├── Prompt for password
    │
    ▼
Wallet
    ├── Argon2id(password, salt) → encryption key
    ├── AES-GCM decrypt private key
    ├── secp256k1 sign transaction
    ├── Zeroize key material
    │
    ▼
Daemon
    │ Submit via RPC (through Anyone Network)
    ▼
Blockchain
```

### ZK Identity Proof

```
User wants to prove membership in group
    │
    ▼
Daemon: identity prove --scope "voting-2024"
    │
    ▼
Load identity secret from ~/.nonos/identity/
    │
    ▼
Compute:
    ├── commitment = Poseidon(secret, nullifier_key)
    ├── nullifier = Poseidon(nullifier_key, scope)
    ├── merkle_path = path to commitment in tree
    │
    ▼
Generate Groth16 proof:
    ├── Public inputs: merkle_root, nullifier, scope
    ├── Private inputs: secret, nullifier_key, merkle_path
    ├── Circuit verifies: commitment in tree, nullifier correctly derived
    │
    ▼
Output: proof + public inputs
```

The verifier learns: "someone in this set made this claim for this scope" but not which member.

### Cache Mixing

```
Browser makes requests
    │
    ▼
Cache Mixer (daemon)
    ├── Add random delays (decorrelate timing)
    ├── Batch multiple requests
    ├── Add cover traffic
    │
    ▼
Anyone Network
```

Prevents timing analysis that could correlate requests.

## Crate Dependencies

```
nonos-types (base)
    │
    ├──► nonos-crypto
    │       │
    │       ├──► nonos-wallet
    │       │
    │       └──► nonos-daemon
    │               │
    │               └──► nonos-anyone
    │
    └──► nonos-dash
```

## Network Topology

```
                    ┌─────────────────┐
                    │  Bootstrap Node │
                    │  102.211.56.24  │
                    └────────┬────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
        ▼                    ▼                    ▼
   ┌─────────┐          ┌─────────┐          ┌─────────┐
   │ Node A  │◄────────►│ Node B  │◄────────►│ Node C  │
   │ (you)   │          │         │          │         │
   └────┬────┘          └─────────┘          └─────────┘
        │
        │ P2P: libp2p/noise
        │ DHT: Kademlia
        │ Pub/Sub: Gossipsub
        │
        ▼
   ┌─────────────────────────────────────┐
   │           Anyone Network            │
   │  Entry ──► Middle ──► Exit ──► Web  │
   └─────────────────────────────────────┘
```

Nodes discover each other via Kademlia DHT. Messages propagate via Gossipsub. All node-to-node traffic encrypted with Noise protocol.

## Storage Layout

```
~/.nonos/
├── config.toml              Node configuration
├── nonos.pid                PID file (when running)
├── p2p_identity.key         Ed25519 node identity (never leaves machine)
│
├── identity/
│   ├── default.json         Primary ZK identity
│   └── *.json               Additional identities
│
├── wallet_data/
│   ├── default.wallet       Primary wallet (encrypted)
│   └── *.wallet             Additional wallets
│
├── db/                      sled embedded database
│   ├── peers                Known peer addresses and reputation
│   ├── metrics              Node performance metrics
│   ├── epochs               Staking epoch data
│   ├── claims               Submitted ZK claims
│   └── nullifiers           Used nullifiers (prevent replay)
│
└── logs/
    └── daemon.log           Rotating log files
```

## Security Boundaries

**Trust boundary 1: Browser ↔ Daemon**
Browser trusts daemon completely. Daemon validates all requests. Communication over localhost HTTP with optional auth token.

**Trust boundary 2: Daemon ↔ Network**
Daemon trusts no external entity. All P2P messages validated. RPC responses verified where possible. Traffic analysis mitigated by Anyone Network.

**Trust boundary 3: User ↔ Wallet**
Wallet operations require user password. Keys encrypted at rest. Memory zeroized after use. No key material in logs.

## API Endpoints

| Endpoint | Method | Purpose |
|----------|--------|---------|
| /status | GET | Node health check |
| /wallet/balance | GET | Get wallet balance |
| /wallet/send | POST | Send transaction |
| /wallet/receive | GET | Generate receive address |
| /identity/prove | POST | Generate ZK proof |
| /proxy/* | * | Forward to Anyone Network |
| /peers | GET | List connected peers |
| /metrics | GET | Prometheus metrics |

## Configuration Reference

```toml
[node]
nickname = "string"           # Display name
reward_address = "0x..."      # Where to receive staking rewards
data_dir = "~/.nonos"         # Override data directory

[p2p]
listen_port = 9432            # P2P listen port
listen_address = "0.0.0.0"    # P2P bind address
bootstrap_peers = []          # Initial peers to connect
max_peers = 50                # Maximum peer connections
min_peers = 5                 # Minimum before seeking more

[api]
bind_address = "127.0.0.1"    # API bind (keep localhost!)
port = 8420                   # API port
auth_enabled = true           # Require auth token
rate_limit = 100              # Requests per second

[privacy]
tracking_blocker = true       # Block known trackers
cache_mixing = true           # Enable timing decorrelation
zk_sessions = true            # Enable ZK session proofs
cover_traffic = false         # Generate fake traffic (bandwidth cost)

[anyone]
socks_port = 9050             # SOCKS5 proxy port
control_port = 9051           # Circuit control port
circuit_timeout = 60          # Seconds before circuit rotation

[staking]
auto_claim = false            # Auto-claim rewards
claim_threshold = 100         # Minimum NOX before claim

[logging]
level = "info"                # trace, debug, info, warn, error
file = "daemon.log"           # Log file name
max_size = "100MB"            # Rotate at this size
max_files = 5                 # Keep this many rotated files
```
