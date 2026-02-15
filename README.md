# NONOS Ecosystem

Privacy-first browser with integrated wallet and ZK services.

Traffic routes through Anyone Protocol. The daemon provides browser features: ZK identity, cache mixing, wallet API.

## What's Inside

| Component | Description |
|-----------|-------------|
| Browser | Desktop app, traffic via Anyone Network |
| Wallet | NOX tokens, stealth addresses |
| Daemon | ZK identity, cache mixer, wallet services |
| Dashboard | Terminal UI for monitoring |

## Quick Start

**Build**
```bash
cargo build --release
```

**Run Browser**
```bash
cd ui && npm install && npm run tauri dev
```

**Run Daemon**
```bash
nonos-daemon init
nonos-daemon run
```

## Project Layout

```
crates/
├── nonos-types      Core type definitions
├── nonos-crypto     Blake3, secp256k1, Ed25519, ZK proofs
├── nonos-wallet     HD wallet, stealth addresses
├── nonos-anyone     Anyone Protocol client (traffic routing)
├── nonos-daemon     ZK identity, cache mixer, wallet API
└── nonos-dash       Terminal dashboard

ui/                  Tauri + Svelte desktop app
contracts/           Solidity staking contracts
```

## Staking (To be finalized - Examples values)

| Tier | Minimum | Lock |
|------|---------|------|
| Bronze | 1,000 NOX | None |
| Silver | 10,000 NOX | 30 days |
| Gold | 50,000 NOX | 90 days |
| Platinum | 200,000 NOX | 180 days |
| Diamond | 1,000,000 NOX | 365 days |

Rewards scale with `sqrt(stake)` to prevent whale dominance.

## Requirements

- Rust 1.75+
- Node.js 18+
- WebKit (Linux/macOS) or WebView2 (Windows)

## License

AGPL-3.0

## Links

- https://nonos.systems
- https://github.com/NON-OS/nonos-ecosystem
