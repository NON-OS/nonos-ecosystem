# nonos-wallet

HD wallet with stealth address support.

## Features

- BIP39 mnemonic generation (24 words)
- BIP44 hierarchical derivation
- Multiple account support
- EIP-1559 transaction signing
- Stealth addresses for private transfers
- Encrypted local storage

## Modules

| Module | Purpose |
|--------|---------|
| `wallet` | HD wallet, key derivation |
| `account` | Multi-account management |
| `transaction` | TX building and signing |
| `storage` | Encrypted persistence |

## Usage

```rust
use nonos_wallet::{HDWallet, TransactionRequest};

let wallet = HDWallet::from_mnemonic(&phrase, "")?;
let account = wallet.derive_account(0)?;
let signed_tx = account.sign_transaction(&tx_request)?;
```

## Storage

Wallet data encrypted with Argon2id + ChaCha20-Poly1305.

Default location: `~/.nonos/wallet.db`

## License

AGPL-3.0
