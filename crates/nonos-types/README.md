# nonos-types

Core type definitions shared across all NONOS crates.

No dependencies on other workspace crates. This is the foundation layer.

## Modules

| Module | Contents |
|--------|----------|
| `crypto` | Blake3Hash, Secp256k1/Ed25519 keys, signatures, StealthKeys |
| `address` | EthAddress, StealthAddress |
| `wallet` | WalletId, TokenAmount, TransactionRequest |
| `node` | NodeId, NodeTier, NodeStatus, PeerInfo |
| `circuit` | CircuitId, CircuitHop, CircuitStatus |
| `epoch` | EpochId, EpochStats, RewardAmount |
| `error` | NonosError, NonosResult |
| `constants` | Protocol parameters |

## Usage

```rust
use nonos_types::{EthAddress, NodeTier, TokenAmount, NonosResult};
```

## License

AGPL-3.0
