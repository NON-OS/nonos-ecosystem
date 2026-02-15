# nonos-crypto

Cryptographic primitives for NONOS.

All operations use audited libraries. No custom cryptography.

## Modules

| Module | Library | Purpose |
|--------|---------|---------|
| `blake3_ops` | blake3 | Hashing, KDF, MAC |
| `secp256k1_ops` | secp256k1 | ECDSA, Ethereum addresses |
| `ed25519_ops` | ed25519-dalek | P2P node identity |
| `encryption` | aes-gcm, chacha20poly1305 | Authenticated encryption |
| `mnemonic` | bip39 | Seed phrase generation |
| `stealth` | secp256k1 | EIP-5564 stealth addresses |
| `poseidon` | ark-bn254 | ZK-friendly hashing |
| `zk_proofs` | ark-groth16 | Identity proofs |

## Usage

```rust
use nonos_crypto::{
    blake3_hash,
    generate_private_key,
    sign_message,
    generate_mnemonic,
    derive_stealth_address,
};
```

## Security

- Keys zeroized on drop
- Constant-time operations
- OS CSPRNG for randomness
- Low-S signature normalization

## License

AGPL-3.0
