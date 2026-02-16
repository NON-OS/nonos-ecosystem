# NONOS Security Invariants

Ten properties that must always hold. Each has enforcement in code and automated tests.

## 1. Browser Never Connects Directly

The Tauri app makes zero outbound connections to non-localhost addresses.

**Why it matters:** If the browser connects directly, your IP is exposed to every site you visit. The entire privacy model breaks.

**Enforcement:**
- All fetch/XHR abstracted through daemon proxy
- Tauri network permissions restricted
- No hardcoded external URLs in browser code

**Test:** `tests/integration/browser_no_direct.rs`
Spawns browser process, monitors network syscalls, fails if any connect() to non-127.0.0.1.

## 2. DNS Resolution Through Exit Nodes Only

Domain names never queried through local system resolver for proxied traffic.

**Why it matters:** DNS leaks reveal every site you visit to your ISP, even if traffic is routed through Tor/Anyone.

**Enforcement:**
- SOCKS5h addressing mode (domain sent to proxy)
- No gethostbyname/getaddrinfo in proxy path
- Anyone client handles DNS at exit

**Test:** `tests/integration/dns_leak.rs`
Captures DNS queries during browsing session, fails if any hit local resolver.

## 3. Localhost Binding by Default

API server (8420) and SOCKS5 proxy (9050) bind to 127.0.0.1.

**Why it matters:** Binding to 0.0.0.0 exposes these services to the network. Anyone could connect to your wallet API.

**Enforcement:**
- Default config hardcodes 127.0.0.1
- Non-local binding requires explicit config + auth enabled
- Startup validation rejects 0.0.0.0 without auth

**Test:** `tests/integration/localhost_only.rs`
Starts daemon with default config, attempts connection from non-localhost, verifies rejection.

## 4. WebRTC Disabled or Filtered

Local IP addresses never leaked via WebRTC ICE candidates.

**Why it matters:** WebRTC can reveal your real IP even through VPN/Tor by enumerating local network interfaces.

**Enforcement:**
- WebRTC disabled by default in Tauri config
- If enabled: ICE candidates filtered to mDNS only
- No STUN server connections

**Test:** `tests/integration/webrtc_leak.rs`
Enables WebRTC, loads test page, captures ICE candidates, fails if any contain IP addresses.

## 5. Secrets Never Logged

Private keys, mnemonics, passwords never appear in log output.

**Why it matters:** Logs get stored, backed up, maybe sent to monitoring services. Secrets in logs = secrets everywhere.

**Enforcement:**
- Secret types implement Debug as `[REDACTED]`
- No `format!("{:?}", secret)` patterns
- Log sanitizer strips hex patterns matching key lengths

**Test:** `tests/unit/secret_redaction.rs`
Creates secrets, logs at all levels, greps output for patterns, fails if found.

## 6. Wallet Encrypted at Rest

Wallet files use memory-hard KDF and authenticated encryption.

**Why it matters:** If someone copies your wallet file, they shouldn't be able to brute-force it on GPUs.

**Enforcement:**
- Argon2id with 64 MiB memory requirement
- AES-256-GCM authenticated encryption
- 16-byte random salt per wallet
- Version header for future algorithm changes

**Test:** `tests/unit/wallet_encryption.rs`
Creates wallet, verifies file is not plaintext, verifies decrypt with correct password, verifies reject with wrong password.

## 7. ZK Proofs Are Cryptographically Sound

Identity proofs use real SNARK verification, not mock checks.

**Why it matters:** If proofs can be forged, the entire identity system is worthless. Anyone could claim membership.

**Enforcement:**
- Groth16 proving system via arkworks
- BN254 curve with ~128-bit security
- Poseidon hash matches circuit constraints
- Verifying key validated against trusted setup

**Test:** `tests/unit/zk_soundness.rs`
- Generate valid proof → verify passes
- Flip bit in proof → verify fails
- Wrong merkle root → verify fails
- Reuse nullifier → application rejects

## 8. Nullifiers Prevent Replay

Each ZK proof has a nullifier that can only be used once per scope.

**Why it matters:** Without nullifiers, someone could reuse the same proof to vote twice, claim rewards twice, etc.

**Enforcement:**
- nullifier = Poseidon(nullifier_key, scope)
- Nullifier stored in database after use
- Duplicate check before accepting proof

**Test:** `tests/unit/nullifier_replay.rs`
Generate proof, submit, verify acceptance, submit same nullifier, verify rejection.

## 9. File Permissions Enforced

Sensitive files created with owner-only permissions.

**Why it matters:** Other users on same system shouldn't read your wallet.

**Enforcement:**
- chmod 0600 for wallet files, keys, config
- chmod 0700 for data directory
- Umask set before file creation

**Test:** `tests/integration/file_permissions.rs`
Create files, stat permissions, fail if group/other can read.

## 10. Atomic Storage Writes

Writes use temp file + rename pattern.

**Why it matters:** Crash during write can corrupt data. Rename is atomic on all filesystems.

**Enforcement:**
- Write to `.tmp` file
- fsync()
- Rename to target
- sled handles its own atomicity

**Test:** `tests/integration/atomic_write.rs`
Simulates crash during write, verifies original file intact or new file complete, never partial.

## Running Tests

```bash
# All security invariant tests
cargo test --features security-tests

# Individual invariant
cargo test dns_leak
cargo test wallet_encryption
cargo test zk_soundness

# With output
RUST_LOG=debug cargo test --features security-tests -- --nocapture
```

## Invariant Checklist

| # | Invariant | Enforced | Tested |
|---|-----------|----------|--------|
| 1 | No direct browser connections | Yes | browser_no_direct.rs |
| 2 | DNS through exit only | Yes | dns_leak.rs |
| 3 | Localhost binding default | Yes | localhost_only.rs |
| 4 | WebRTC disabled/filtered | Yes | webrtc_leak.rs |
| 5 | Secrets not logged | Yes | secret_redaction.rs |
| 6 | Wallet encrypted | Yes | wallet_encryption.rs |
| 7 | ZK proofs sound | Yes | zk_soundness.rs |
| 8 | Nullifiers prevent replay | Yes | nullifier_replay.rs |
| 9 | File permissions | Yes | file_permissions.rs |
| 10 | Atomic writes | Yes | atomic_write.rs |

## Adding New Invariants

When adding security-critical features:

1. Define the invariant clearly
2. Implement enforcement in code
3. Write automated test
4. Add to this document
5. Add to CI pipeline
