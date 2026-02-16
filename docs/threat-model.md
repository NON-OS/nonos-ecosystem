# NONOS Ecosystem Threat Model

## What We Protect

| Asset | Criticality | Where It Lives |
|-------|-------------|----------------|
| Wallet private keys | Critical | Encrypted in ~/.nonos/wallet_data/ |
| Mnemonic seed phrase | Critical | User's memory or encrypted backup |
| ZK identity secrets | High | ~/.nonos/identity/ |
| Node identity key | Medium | ~/.nonos/p2p_identity.key |
| Browsing activity | High | Memory only, never persisted unencrypted |
| Transaction history | High | On-chain (public but pseudonymous) |
| API tokens | Medium | Environment variable or config |

## Who Attacks Us

### Local Threats

**Malware on your machine**
Can read files, dump memory, intercept system calls, keylog passwords.

Mitigations:
- File permissions 0600 on all sensitive files
- Encrypted storage (Argon2id + AES-GCM)
- Memory zeroization after key use
- Session timeout for unlocked wallets
- Optional: hardware wallet (keys never on machine)

**Physical access**
Someone with your laptop can copy files, install rootkits, rubber-hose you for passwords.

Mitigations:
- Full disk encryption (OS level, not us)
- Hardware wallet (resistant to software attacks)
- Plausible deniability is out of scope

### Network Threats

**ISP / Network observer**
Sees all your traffic. Knows when you're online, how much data, timing patterns.

Mitigations:
- All traffic through Anyone Network (3+ hops)
- No direct connections from browser
- Cover traffic option (constant bandwidth)
- Cache mixing (timing decorrelation)

**DNS provider / resolver**
Sees every domain you visit.

Mitigations:
- SOCKS5h - DNS resolved at exit node, not locally
- No system resolver calls for proxied traffic
- DNS queries never leave your machine unencrypted

**Malicious exit nodes**
Can see unencrypted traffic, inject content, MITM non-HTTPS.

Mitigations:
- HTTPS enforcement where possible
- Certificate validation
- User warning for HTTP sites
- Exit node reputation (future)

### Web Threats

**Tracking / fingerprinting**
Sites identify you across visits via cookies, canvas, WebGL, fonts.

Mitigations:
- Per-site isolation (fresh context per domain)
- Tracking blocker (known tracker domains)
- Canvas noise injection
- WebGL spoofing
- Font enumeration blocking

**XSS / CSRF**
Malicious scripts steal data or perform actions.

Mitigations:
- Content Security Policy headers
- Strict same-origin policy
- No sensitive data in DOM

**WebRTC leaks**
Reveals your real IP via ICE candidates.

Mitigations:
- WebRTC disabled by default
- If enabled: mDNS-only candidates

### Blockchain Threats

**Chain analysis**
Link transactions to real identities via address reuse, timing, amounts.

Mitigations:
- Stealth addresses (EIP-5564) - fresh address per receive
- Transaction timing decorrelation
- Amount obfuscation (future: mixer integration)

**Malicious RPC**
Returns false balances, censors transactions, front-runs.

Mitigations:
- Multiple RPC fallback
- Transaction confirmation verification
- Self-hosted RPC option
- All RPC through Anyone Network (no IP exposure)

### Supply Chain Threats

**Compromised dependencies**
Malicious code in crates.io packages.

Mitigations:
- cargo-audit for known vulnerabilities
- cargo-vet for trusted publishers
- Minimal dependency tree
- Reproducible builds
- Code review for new deps

## Security Goals (Must Hold)

1. Private keys never exposed unencrypted outside process memory
2. Browser makes zero direct internet connections
3. All external traffic routes through Anyone Network
4. DNS resolution never hits local resolver for proxied traffic
5. WebRTC never exposes real IP
6. Secrets never appear in logs
7. ZK proofs are cryptographically sound (Groth16, not simulated)
8. Nullifiers prevent proof replay
9. API accessible only from localhost by default

## Explicit Non-Goals

Things we don't try to protect against:

- Compromised operating system kernel
- Physical device seizure with password coercion
- Nation-state adversaries with unlimited resources
- Quantum computers (future consideration)
- Deanonymization via typing patterns or mouse movements
- Censorship resistance (you can't connect if blocked)

## Attack Surface Summary

| Surface | Exposure | Protection |
|---------|----------|------------|
| P2P port 9432 | Public internet | libp2p validation, rate limiting |
| API port 8420 | Localhost only | Auth token if exposed |
| SOCKS5 port 9050 | Localhost only | Never expose |
| Wallet files | Local disk | Argon2id + AES-256-GCM |
| Identity files | Local disk | File permissions |
| Browser IPC | Local process | Input validation |
| RPC calls | Via Anyone Network | No IP exposure |

## Wallet Security Deep Dive

**At rest:**
```
wallet_file = salt || nonce || ciphertext || tag
ciphertext = AES-256-GCM(key, nonce, plaintext)
key = Argon2id(password, salt, 64MiB, 3 iterations, 4 lanes)
```

**In memory:**
- Key decrypted only when needed
- Zeroized immediately after signing
- Session mode: key cached until timeout or manual lock
- Never serialized to disk unencrypted

**Hardware wallet (planned):**
- Private key never leaves device
- User approves each transaction on hardware
- Immune to malware key extraction
- Ledger and Trezor support via HID
